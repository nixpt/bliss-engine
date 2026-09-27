//! SHELL-SPLIT-0-1 Phase 1 coexistence proof — a [`SceneOverlay`] (the existing,
//! unchanged path) and a [`TextureOverlay`] (the new Phase 1 path) composited on
//! top of the same shell scene in a single `render()` call, proving they don't
//! interfere with each other.
//!
//! This talks to `VelloWindowRenderer` directly via a minimal winit
//! `ApplicationHandler`, bypassing bliss-shell/arniko entirely: the high-level
//! `arniko::reactive::launch_reactive_configured` helper (used by the Phase 0
//! spike, `crates/arniko/examples/texture_overlay_spike.rs`) only exposes a
//! renderer-configuration hook that runs BEFORE the renderer resumes (i.e.
//! before a `DeviceHandle`/GPU device exists), so it can't be used to create the
//! GPU resources this proof needs. Going straight to `anyrender_vello` avoids
//! that constraint and keeps this test scoped to the crate it's actually
//! testing.
//!
//! Run with:
//!   CARGO_TARGET_DIR=/build/target-arniko cargo run -p anyrender_vello --example overlay_coexistence
//!
//! Expected result (see SHELL-SPLIT-0-1 report for the captured screenshot):
//!   - A grey chrome background (the "shell" scene — left empty on purpose).
//!   - A green rectangle at (40,40)..(190,140) — the `SceneOverlay` (vector scene,
//!     merged via `VelloScene::append`, exactly like the existing web-content-overlay
//!     mechanism this crate already ships).
//!   - A blue rectangle at (240,40)..(390,140) — the `TextureOverlay` (an offscreen
//!     `wgpu::Texture`, registered + drawn as a single image fill via the new
//!     `set_overlay_texture` added in this phase).
//!   - Both visible simultaneously, at their correct independent positions.

use std::sync::Arc;

use anyrender::WindowRenderer;
use anyrender_vello::{VelloWindowRenderer, vello, wgpu};
use kurbo::{Affine, Rect};
use peniko::{Color, Fill};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

const TEX_W: u32 = 150;
const TEX_H: u32 = 100;

struct App {
    renderer: VelloWindowRenderer,
    window: Option<Arc<dyn Window>>,
    /// Kept alive for the process lifetime — dropping it would invalidate the
    /// texture registered with the window's renderer.
    _offscreen_renderer: Option<vello::Renderer>,
    frame_count: u32,
}

impl App {
    fn new() -> Self {
        Self {
            renderer: VelloWindowRenderer::new(),
            window: None,
            _offscreen_renderer: None,
            frame_count: 0,
        }
    }
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let window: Arc<dyn Window> = Arc::from(
            event_loop
                .create_window(WindowAttributes::default().with_title("overlay_coexistence"))
                .unwrap(),
        );
        let size = window.surface_size();
        self.renderer
            .resume(Arc::new(window.clone()), size.width, size.height);
        self.window = Some(window.clone());

        if !self.renderer.is_active() {
            eprintln!("overlay_coexistence: renderer failed to resume");
            return;
        }

        let device_handle = self
            .renderer
            .current_device_handle()
            .expect("just resumed")
            .clone();

        // ── SceneOverlay: a green rect, positioned via Affine translate ────────────
        let mut scene = vello::Scene::new();
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(34, 197, 94),
            None,
            &Rect::new(0.0, 0.0, 150.0, 100.0),
        );
        self.renderer
            .set_overlay_scene(Arc::new(scene), Affine::translate((40.0, 40.0)), None);

        // ── TextureOverlay: a blue rect rendered to an offscreen texture ───────────
        let texture = device_handle.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("overlay-coexistence-blue"),
            size: wgpu::Extent3d {
                width: TEX_W,
                height: TEX_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            // STORAGE_BINDING: required by render_to_texture (compute rasterizer).
            // COPY_SRC: required by register_texture (copied into the image atlas).
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let mut offscreen_renderer = vello::Renderer::new(
            &device_handle.device,
            vello::RendererOptions {
                use_cpu: false,
                antialiasing_support: vello::AaSupport::area_only(),
                num_init_threads: None,
                pipeline_cache: None,
            },
        )
        .expect("failed to create offscreen vello::Renderer");

        let mut tex_scene = vello::Scene::new();
        tex_scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(59, 130, 246),
            None,
            &Rect::new(0.0, 0.0, TEX_W as f64, TEX_H as f64),
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        offscreen_renderer
            .render_to_texture(
                &device_handle.device,
                &device_handle.queue,
                &tex_scene,
                &view,
                &vello::RenderParams {
                    base_color: Color::from_rgb8(59, 130, 246),
                    width: TEX_W,
                    height: TEX_H,
                    antialiasing_method: vello::AaConfig::Area,
                },
            )
            .expect("offscreen render_to_texture failed");

        self.renderer
            .set_overlay_texture(texture, Affine::translate((240.0, 40.0)), None);
        self._offscreen_renderer = Some(offscreen_renderer);

        window.request_redraw();
    }

    fn destroy_surfaces(&mut self, _event_loop: &dyn ActiveEventLoop) {
        self.renderer.suspend();
    }

    fn resumed(&mut self, _event_loop: &dyn ActiveEventLoop) {}
    fn suspended(&mut self, _event_loop: &dyn ActiveEventLoop) {}
    fn proxy_wake_up(&mut self, _event_loop: &dyn ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                // Empty chrome scene — this proof only cares about the two overlays.
                self.renderer.render(|_scene_painter| {});
                self.frame_count += 1;
                eprintln!("[overlay-coexistence] rendered frame {}", self.frame_count);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::SurfaceResized(size) => {
                self.renderer.set_size(size.width, size.height);
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::builder().build().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    event_loop.run_app(App::new()).unwrap();
}
