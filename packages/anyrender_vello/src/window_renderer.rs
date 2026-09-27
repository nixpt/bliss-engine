use anyrender::{WindowHandle, WindowRenderer};
use debug_timer::debug_timer;
use kurbo::{Affine, Rect};
use peniko::{Color, Fill, ImageBrush, ImageData};
use rustc_hash::FxHashMap;
use std::sync::{
    atomic::{self, AtomicU64},
    Arc,
};
use vello::{
    AaConfig, AaSupport, RenderParams, Renderer as VelloRenderer, RendererOptions,
    Scene as VelloScene,
};
use wgpu::{Features, Limits, PresentMode, Texture, TextureFormat, TextureUsages};
use wgpu_context::{
    DeviceHandle, SurfaceRenderer, SurfaceRendererConfiguration, TextureConfiguration, WGPUContext,
};

use crate::{CustomPaintSource, VelloScenePainter, DEFAULT_THREADS};

/// An overlay scene to composite on top of the shell scene during rendering.
/// Used for zero-copy rendering of web content into the browser chrome.
pub struct SceneOverlay {
    /// The Vello scene to overlay (Arc for zero-copy sharing)
    pub scene: Arc<VelloScene>,
    /// Transform to position the overlay in the shell's coordinate space
    pub transform: Affine,
    /// Optional clip rectangle (in shell coordinates) to constrain the overlay
    pub clip: Option<Rect>,
}

/// A texture-backed overlay to composite on top of the shell scene during rendering.
///
/// Unlike [`SceneOverlay`] (which merges vector paint ops into the shell's own
/// scene via [`VelloScene::append`]), this composites an already-rasterized
/// `wgpu::Texture` — e.g. content rendered to its own GPU texture on a separate
/// render pipeline — as a single image fill. That's a real blit, not a scene
/// merge: it decouples the cost of the overlay's own rendering from the shell's
/// frame time, at the expense of only being able to sample a flat rectangle of
/// pixels (no further vector compositing of the overlay's contents).
pub struct TextureOverlay {
    /// The already-rendered texture to sample as an image fill. Must be
    /// [`TextureFormat::Rgba8Unorm`] with [`TextureUsages::COPY_SRC`] set
    /// (required by [`vello::Renderer::register_texture`]).
    pub texture: Texture,
    /// Transform to position the overlay in the shell's coordinate space
    pub transform: Affine,
    /// Optional clip rectangle (in shell coordinates) to constrain the overlay
    pub clip: Option<Rect>,
}

static PAINT_SOURCE_ID: AtomicU64 = AtomicU64::new(0);

// Simple struct to hold the state of the renderer
struct ActiveRenderState {
    renderer: VelloRenderer,
    render_surface: SurfaceRenderer<'static>,
}

#[allow(clippy::large_enum_variant)]
enum RenderState {
    Active(ActiveRenderState),
    Suspended,
}

impl RenderState {
    fn current_device_handle(&self) -> Option<&DeviceHandle> {
        let RenderState::Active(state) = self else {
            return None;
        };
        Some(&state.render_surface.device_handle)
    }
}

#[derive(Clone)]
pub struct VelloRendererOptions {
    pub features: Option<Features>,
    pub limits: Option<Limits>,
    pub base_color: Color,
    pub antialiasing_method: AaConfig,
}

impl Default for VelloRendererOptions {
    fn default() -> Self {
        Self {
            features: None,
            limits: None,
            base_color: Color::WHITE,
            antialiasing_method: AaConfig::Msaa16,
        }
    }
}

pub struct VelloWindowRenderer {
    // The fields MUST be in this order, so that the surface is dropped before the window
    // Window is cached even when suspended so that it can be reused when the app is resumed after being suspended
    render_state: RenderState,
    window_handle: Option<Arc<dyn WindowHandle>>,

    // Vello
    wgpu_context: WGPUContext,
    scene: VelloScene,
    config: VelloRendererOptions,

    custom_paint_sources: FxHashMap<u64, Box<dyn CustomPaintSource>>,

    /// Overlay scenes to composite on top of the main scene (e.g. web content)
    overlay_scenes: Vec<SceneOverlay>,

    /// Texture overlays to composite on top of the main scene (e.g. content rendered
    /// to its own GPU texture on a separate pipeline — see [`TextureOverlay`]).
    overlay_textures: Vec<TextureOverlay>,

    /// Cache of the last texture registered with the active `vello::Renderer`'s image
    /// atlas via [`VelloRenderer::register_texture`], keyed by the `wgpu::Texture`
    /// handle it was registered from. `register_texture` allocates a fresh atlas slot
    /// on every call (there is no way to look one up by texture identity), so without
    /// this cache re-registering the same unchanged texture every frame would leak an
    /// `image_overrides` entry per frame. Cleared on suspend/resume, since a new
    /// `vello::Renderer` is created on resume and `ImageData` handles are only valid
    /// for the renderer that produced them.
    registered_overlay_texture: Option<(Texture, ImageData)>,

    /// Optional post-paint effect hook — called after all draw commands and overlays are
    /// composited, immediately before the scene is submitted to wgpu. Use this to apply
    /// GPU effects (e.g. blur via mustang) without coupling this crate to a specific
    /// effect compositor.
    scene_effects: Option<Box<dyn FnMut(&mut VelloScene, u32, u32) + Send>>,
}
impl VelloWindowRenderer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::with_options(VelloRendererOptions::default())
    }

    pub fn with_options(config: VelloRendererOptions) -> Self {
        let mut features = config.features.unwrap_or_default();
        // Request optional features that improve performance when available.
        // wgpu will ignore unsupported features during adapter selection.
        features |= Features::CLEAR_TEXTURE;
        features |= Features::PIPELINE_CACHE;
        Self {
            wgpu_context: WGPUContext::with_features_and_limits(
                Some(features),
                config.limits.clone(),
            ),
            config,
            render_state: RenderState::Suspended,
            window_handle: None,
            scene: VelloScene::new(),
            custom_paint_sources: FxHashMap::default(),
            overlay_scenes: Vec::new(),
            overlay_textures: Vec::new(),
            registered_overlay_texture: None,
            scene_effects: None,
        }
    }

    /// Register a post-paint effect hook.
    ///
    /// The closure is called every frame after all draw commands and overlay scenes
    /// have been composited into the Vello scene, and before the scene is handed off
    /// to wgpu for GPU rendering. Use it to apply GPU effects (blur, transforms, …)
    /// by wrapping the `VelloScene` in a `VelloScenePainter` and calling your effect
    /// compositor of choice.
    ///
    /// Only one hook is active at a time; calling this again replaces the previous one.
    pub fn set_scene_effects<F>(&mut self, f: F)
    where
        F: FnMut(&mut VelloScene, u32, u32) + Send + 'static,
    {
        self.scene_effects = Some(Box::new(f));
    }

    /// Remove any registered effect hook.
    pub fn clear_scene_effects(&mut self) {
        self.scene_effects = None;
    }

    pub fn current_device_handle(&self) -> Option<&DeviceHandle> {
        self.render_state.current_device_handle()
    }

    pub fn register_custom_paint_source(&mut self, mut source: Box<dyn CustomPaintSource>) -> u64 {
        if let Some(device_handle) = self.render_state.current_device_handle() {
            source.resume(device_handle);
        }
        let id = PAINT_SOURCE_ID.fetch_add(1, atomic::Ordering::Relaxed);
        self.custom_paint_sources.insert(id, source);

        id
    }

    pub fn unregister_custom_paint_source(&mut self, id: u64) {
        if let Some(mut source) = self.custom_paint_sources.remove(&id) {
            source.suspend();
            drop(source);
        }
    }

    /// Set a single overlay scene to composite on top of the shell scene.
    /// Replaces any existing overlays.
    pub fn set_overlay_scene(
        &mut self,
        scene: Arc<VelloScene>,
        transform: Affine,
        clip: Option<Rect>,
    ) {
        self.overlay_scenes.clear();
        self.overlay_scenes.push(SceneOverlay {
            scene,
            transform,
            clip,
        });
    }

    /// Remove all overlay scenes.
    pub fn clear_overlay_scenes(&mut self) {
        self.overlay_scenes.clear();
    }

    /// Set a single texture overlay to composite on top of the shell scene.
    /// Replaces any existing texture overlays. Independent of [`Self::set_overlay_scene`]
    /// — both kinds of overlay can be active at the same time.
    pub fn set_overlay_texture(&mut self, texture: Texture, transform: Affine, clip: Option<Rect>) {
        self.overlay_textures.clear();
        self.overlay_textures.push(TextureOverlay {
            texture,
            transform,
            clip,
        });
    }

    /// Remove all texture overlays.
    pub fn clear_overlay_textures(&mut self) {
        self.overlay_textures.clear();
    }
}

impl WindowRenderer for VelloWindowRenderer {
    type ScenePainter<'a>
        = VelloScenePainter<'a, 'a>
    where
        Self: 'a;

    fn is_active(&self) -> bool {
        matches!(self.render_state, RenderState::Active(_))
    }

    fn resume(&mut self, window_handle: Arc<dyn WindowHandle>, width: u32, height: u32) {
        // Create wgpu_context::SurfaceRenderer
        let render_surface = pollster::block_on(self.wgpu_context.create_surface(
            window_handle.clone(),
            SurfaceRendererConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                formats: vec![TextureFormat::Rgba8Unorm, TextureFormat::Bgra8Unorm],
                width,
                height,
                present_mode: PresentMode::AutoVsync,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
            },
            Some(TextureConfiguration {
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            }),
        ))
        .expect("Error creating surface");

        // Create vello::Renderer
        let renderer = VelloRenderer::new(
            render_surface.device(),
            RendererOptions {
                antialiasing_support: AaSupport::all(),
                use_cpu: false,
                num_init_threads: DEFAULT_THREADS,
                // TODO: add pipeline cache
                pipeline_cache: None,
            },
        )
        .unwrap();

        // Resume custom paint sources
        let device_handle = &render_surface.device_handle;
        for source in self.custom_paint_sources.values_mut() {
            source.resume(device_handle)
        }

        // A fresh `vello::Renderer` was just created above — any `ImageData` cached
        // from a previous renderer's `register_texture` call is no longer valid
        // (`ImageData` handles only work with the renderer that produced them).
        self.registered_overlay_texture = None;

        // Set state to Active
        self.window_handle = Some(window_handle);
        self.render_state = RenderState::Active(ActiveRenderState {
            renderer,
            render_surface,
        });
    }

    fn suspend(&mut self) {
        // Suspend custom paint sources
        for source in self.custom_paint_sources.values_mut() {
            source.suspend()
        }

        // The renderer that produced this cached `ImageData` is about to be dropped.
        self.registered_overlay_texture = None;

        // Set state to Suspended
        self.render_state = RenderState::Suspended;
    }

    fn set_size(&mut self, width: u32, height: u32) {
        if let RenderState::Active(state) = &mut self.render_state {
            state.render_surface.resize(width, height);
        };
    }

    fn render<F: FnOnce(&mut Self::ScenePainter<'_>)>(&mut self, draw_fn: F) {
        let RenderState::Active(state) = &mut self.render_state else {
            return;
        };

        let render_surface = &mut state.render_surface;

        debug_timer!(timer, feature = "log_frame_times");

        // Regenerate the vello scene
        draw_fn(&mut VelloScenePainter {
            inner: &mut self.scene,
            renderer: Some(&mut state.renderer),
            custom_paint_sources: Some(&mut self.custom_paint_sources),
        });

        // Composite overlay scenes (e.g. web content) on top of the shell scene
        for overlay in &self.overlay_scenes {
            if let Some(clip_rect) = &overlay.clip {
                self.scene.push_layer(
                    Fill::NonZero,
                    peniko::BlendMode::default(),
                    1.0,
                    Affine::IDENTITY,
                    clip_rect,
                );
                self.scene.append(&overlay.scene, Some(overlay.transform));
                self.scene.pop_layer();
            } else {
                self.scene.append(&overlay.scene, Some(overlay.transform));
            }
        }
        // Composite texture overlays (e.g. content rendered to its own GPU texture on
        // a separate pipeline) on top of the shell scene. Unlike scene overlays, this
        // registers the already-rasterized texture with Vello's image atlas and draws
        // it as a single image fill — a real blit, not a scene merge. `overlay_textures`
        // holds at most one entry (mirroring `set_overlay_scene`'s single-overlay-
        // replace behaviour), so the single-slot registration cache below is sufficient.
        for overlay in &self.overlay_textures {
            let image_data = match &self.registered_overlay_texture {
                Some((cached_texture, cached_image)) if *cached_texture == overlay.texture => {
                    cached_image.clone()
                }
                _ => {
                    if let Some((_, old_image)) = self.registered_overlay_texture.take() {
                        state.renderer.unregister_texture(old_image);
                    }
                    let image_data = state.renderer.register_texture(overlay.texture.clone());
                    self.registered_overlay_texture =
                        Some((overlay.texture.clone(), image_data.clone()));
                    image_data
                }
            };

            let image_brush = ImageBrush::new(image_data);
            let bounds = Rect::from_origin_size(
                (0.0, 0.0),
                (
                    overlay.texture.width() as f64,
                    overlay.texture.height() as f64,
                ),
            );

            if let Some(clip_rect) = &overlay.clip {
                self.scene.push_layer(
                    Fill::NonZero,
                    peniko::BlendMode::default(),
                    1.0,
                    Affine::IDENTITY,
                    clip_rect,
                );
                self.scene
                    .fill(Fill::NonZero, overlay.transform, &image_brush, None, &bounds);
                self.scene.pop_layer();
            } else {
                self.scene
                    .fill(Fill::NonZero, overlay.transform, &image_brush, None, &bounds);
            }
        }
        timer.record_time("cmd");

        // Apply GPU effects (blur, transforms, …) registered via set_scene_effects.
        // Runs after all paint commands and overlays are composited, before wgpu submission.
        let effect_viewport = (render_surface.config.width, render_surface.config.height);
        if let Some(hook) = &mut self.scene_effects {
            hook(&mut self.scene, effect_viewport.0, effect_viewport.1);
        }

        let texture_view = render_surface.target_texture_view();
        state
            .renderer
            .render_to_texture(
                render_surface.device(),
                render_surface.queue(),
                &self.scene,
                &texture_view,
                &RenderParams {
                    base_color: self.config.base_color,
                    width: render_surface.config.width,
                    height: render_surface.config.height,
                    antialiasing_method: self.config.antialiasing_method,
                },
            )
            .expect("failed to render to texture");
        timer.record_time("render");

        drop(texture_view);

        render_surface.maybe_blit_and_present();
        timer.record_time("present");

        // Poll without blocking — GPU work is pipelined with presentation
        let _ = render_surface
            .device()
            .poll(wgpu::PollType::Poll);

        timer.record_time("wait");
        timer.print_times("vello: ");

        // static COUNTER: AtomicU64 = AtomicU64::new(0);
        // println!("FRAME {}", COUNTER.fetch_add(1, atomic::Ordering::Relaxed));

        // Empty the Vello scene (memory optimisation)
        self.scene.reset();
    }
}
