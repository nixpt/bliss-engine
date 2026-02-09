//! Effect types for Mustang GPU compositor
//!
//! Copyright (c) 2026 The Exosphere Authors
//!
//! Dual-licensed under MIT or Apache-2.0.
//!
//! Effects can be applied to any PaintScene implementation,
//! including VelloScenePainter from anyrender_vello.

use anyrender::PaintScene;
use crate::compositor::region::Region;

/// Types of Mustang effects
#[derive(Debug, Clone, PartialEq)]
pub enum EffectType {
    /// Gaussian blur effect for backdrop-filter
    BackdropBlur,
    /// 2D transform (scale, translate, rotate)
    Transform2D,
    /// Color adjustment (multipliers and offsets)
    ColorAdjust,
    /// Clip/mask effect for security gating
    Clip,
}

/// Quality levels for blur effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlurQuality {
    Low,    // Fast, good for previews
    Medium, // Balanced
    High,   // Best quality, slower
    Ultra,  // Maximum quality
}

impl Default for BlurQuality {
    fn default() -> Self {
        BlurQuality::High
    }
}

/// Parameters for color adjustment
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorAdjustParams {
    pub red_multiplier: f32,
    pub green_multiplier: f32,
    pub blue_multiplier: f32,
    pub red_offset: f32,
    pub green_offset: f32,
    pub blue_offset: f32,
}

impl Default for ColorAdjustParams {
    fn default() -> Self {
        Self {
            red_multiplier: 1.0,
            green_multiplier: 1.0,
            blue_multiplier: 1.0,
            red_offset: 0.0,
            green_offset: 0.0,
            blue_offset: 0.0,
        }
    }
}

/// Parameters for blur effect
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlurParams {
    /// Blur radius in pixels
    pub radius: f32,
    /// Number of blur passes (more = smoother but slower)
    pub passes: u32,
    /// Quality level for the blur
    pub quality: BlurQuality,
}

impl Default for BlurParams {
    fn default() -> Self {
        Self {
            radius: 10.0,
            passes: 2,
            quality: BlurQuality::High,
        }
    }
}

/// Parameters for 2D transform
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformParams {
    /// Scale on X axis
    pub scale_x: f32,
    /// Scale on Y axis
    pub scale_y: f32,
    /// Translation on X axis in pixels
    pub translate_x: f32,
    /// Translation on Y axis in pixels
    pub translate_y: f32,
    /// Rotation in degrees
    pub rotate_degrees: f32,
    /// Pivot point X (0.0 = left, 0.5 = center, 1.0 = right)
    pub pivot_x: f32,
    /// Pivot point Y (0.0 = top, 0.5 = center, 1.0 = bottom)
    pub pivot_y: f32,
}

impl Default for TransformParams {
    fn default() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
            rotate_degrees: 0.0,
            pivot_x: 0.5,
            pivot_y: 0.5,
        }
    }
}

/// A Mustang effect to be applied to a region
#[derive(Debug, Clone)]
pub struct Effect {
    /// Type of effect
    pub effect_type: EffectType,
    /// CSS selector that identifies the target element
    pub selector: String,
    /// Region bounds (x, y, width, height) in screen coordinates
    pub region: Region,
    /// Blur-specific parameters
    pub blur_params: Option<BlurParams>,
    /// Transform-specific parameters
    pub transform_params: Option<TransformParams>,
    /// Color-specific parameters
    pub color_params: Option<ColorAdjustParams>,
    /// Z-order for layering (higher = on top)
    pub z_index: i32,
}

impl Effect {
    /// Create a blur effect
    pub fn blur(selector: &str, radius: f32, viewport_width: u32, viewport_height: u32) -> Self {
        Self {
            effect_type: EffectType::BackdropBlur,
            selector: selector.to_string(),
            region: Region::new(0.0, 0.0, viewport_width as f32, viewport_height as f32),
            blur_params: Some(BlurParams {
                radius,
                passes: 2,
                quality: BlurQuality::High,
            }),
            transform_params: None,
            color_params: None,
            z_index: 0,
        }
    }

    /// Create a transform effect
    pub fn transform(
        selector: &str,
        params: TransformParams,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Self {
        Self {
            effect_type: EffectType::Transform2D,
            selector: selector.to_string(),
            region: Region::new(0.0, 0.0, viewport_width as f32, viewport_height as f32),
            blur_params: None,
            transform_params: Some(params),
            color_params: None,
            z_index: 0,
        }
    }

    /// Create a color adjustment effect
    pub fn color_adjust(selector: &str, params: ColorAdjustParams) -> Self {
        Self {
            effect_type: EffectType::ColorAdjust,
            selector: selector.to_string(),
            region: Region::new(0.0, 0.0, 0.0, 0.0),
            blur_params: None,
            transform_params: None,
            color_params: Some(params),
            z_index: 0,
        }
    }

    /// Create a clip effect for security gating
    pub fn clip(region: Region) -> Self {
        Self {
            effect_type: EffectType::Clip,
            selector: String::new(),
            region,
            blur_params: None,
            transform_params: None,
            color_params: None,
            z_index: 9999, // Clips are always top
        }
    }

    /// Update the region for this effect
    pub fn with_region(mut self, region: Region) -> Self {
        self.region = region;
        self
    }

    /// Set z-index for layering
    pub fn with_z_index(mut self, z_index: i32) -> Self {
        self.z_index = z_index;
        self
    }

    /// Returns true if this effect can be applied scene-natively
    pub fn is_native(&self) -> bool {
        matches!(
            self.effect_type,
            EffectType::BackdropBlur | EffectType::Transform2D | EffectType::Clip
        )
    }

    /// Returns true if this effect requires GPU compute
    pub fn requires_gpu_compute(&self) -> bool {
        matches!(self.effect_type, EffectType::ColorAdjust)
    }
}

/// Trait for applying effects to scenes
/// 
/// Implemented for Effect to apply itself to any PaintScene
/// 
/// Note: This uses generics instead of dyn PaintScene because PaintScene
/// has methods that make it not object-safe.
pub trait ApplyEffect<S: PaintScene> {
    /// Apply this effect to a scene
    fn apply_to_scene(&self, scene: &mut S, viewport: (u32, u32));
}

impl<S: PaintScene> ApplyEffect<S> for Effect {
    fn apply_to_scene(&self, scene: &mut S, _viewport: (u32, u32)) {
        use kurbo::Rect;
        use peniko::{Fill, BlendMode};

        match self.effect_type {
            EffectType::BackdropBlur => {
                if let Some(ref _params) = self.blur_params {
                    // Use draw_box_shadow for blur effect
                    let rect = Rect::new(
                        self.region.x as f64,
                        self.region.y as f64,
                        (self.region.x + self.region.width) as f64,
                        (self.region.y + self.region.height) as f64,
                    );
                    // Note: In full implementation, this would use 
                    // scene.draw_box_shadow() or similar blur operation
                    // For now, place a semi-transparent rect as placeholder
                    // Use a simple color with alpha
                    let color = peniko::color::palette::css::BLUE.with_alpha(0.2);
                    scene.fill(
                        Fill::NonZero,
                        kurbo::Affine::IDENTITY,
                        color,
                        None,
                        &rect,
                    );
                }
            }
            EffectType::Transform2D => {
                if let Some(ref params) = self.transform_params {
                    // Apply transform using push_layer with transform
                    let rect = Rect::new(
                        self.region.x as f64,
                        self.region.y as f64,
                        (self.region.x + self.region.width) as f64,
                        (self.region.y + self.region.height) as f64,
                    );
                    
                    // Build affine transform
                    let transform = kurbo::Affine::translate((
                        (self.region.x + self.region.width * params.pivot_x) as f64,
                        (self.region.y + self.region.height * params.pivot_y) as f64,
                    )) * kurbo::Affine::rotate(params.rotate_degrees.to_radians() as f64)
                        * kurbo::Affine::scale_non_uniform(params.scale_x as f64, params.scale_y as f64)
                        * kurbo::Affine::translate((
                            -(self.region.x + self.region.width * params.pivot_x) as f64,
                            -(self.region.y + self.region.height * params.pivot_y) as f64,
                        ))
                        * kurbo::Affine::translate((params.translate_x as f64, params.translate_y as f64));

                    // Push transform layer
                    scene.push_layer(
                        BlendMode::default(),
                        1.0,
                        transform,
                        &rect,
                    );
                    // Note: Caller must pop_layer after rendering content
                }
            }
            EffectType::Clip => {
                // Push clip layer
                let rect = Rect::new(
                    self.region.x as f64,
                    self.region.y as f64,
                    (self.region.x + self.region.width) as f64,
                    (self.region.y + self.region.height) as f64,
                );
                scene.push_clip_layer(kurbo::Affine::IDENTITY, &rect);
                // Note: Caller must pop_layer after rendering content
            }
            EffectType::ColorAdjust => {
                // Requires GPU compute - handled by CustomPaintSource
                // This is a no-op in scene-native rendering
            }
        }
    }
}

// For backwards compatibility, keep SceneEffect as an alias
pub use ApplyEffect as SceneEffect;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_blur() {
        let effect = Effect::blur(".test", 10.0, 1280, 800);
        assert!(matches!(effect.effect_type, EffectType::BackdropBlur));
        assert!(effect.is_native());
    }

    #[test]
    fn test_effect_transform() {
        let params = TransformParams::default();
        let effect = Effect::transform(".test", params, 1280, 800);
        assert!(matches!(effect.effect_type, EffectType::Transform2D));
        assert!(effect.is_native());
    }

    #[test]
    fn test_effect_color_adjust_requires_gpu() {
        let params = ColorAdjustParams::default();
        let effect = Effect::color_adjust(".test", params);
        assert!(effect.requires_gpu_compute());
        assert!(!effect.is_native());
    }

    #[test]
    fn test_effect_builder_pattern() {
        let effect = Effect::blur(".glass", 15.0)
            .with_z_index(10)
            .with_region(Region::new(10.0, 10.0, 200.0, 100.0));

        assert_eq!(effect.z_index, 10);
        assert_eq!(effect.region.x, 10.0);
        assert_eq!(effect.region.y, 10.0);
        assert_eq!(effect.region.width, 200.0);
        assert_eq!(effect.region.height, 100.0);
    }
}
