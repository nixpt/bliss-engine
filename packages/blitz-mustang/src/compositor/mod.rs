//! Compositor module for Blitz
//!
//! Copyright (c) 2026 The Exosphere Authors
//!
//! Dual-licensed under MIT or Apache-2.0.
//!
//! Provides post-processing effects for CSS features that cannot be rendered natively.
//! This includes backdrop-filter blur, transforms, color adjustments, and security clipping.

pub mod effect;
pub mod element_tracker;
pub mod integration;
pub mod region;

// Re-export main types from effect (which re-exports from mustang)
pub use effect::{Effect, EffectType, TransformParams, BlurParams, BlurQuality, ColorAdjustParams};
// Re-export Region from local region module
pub use region::Region;
// Re-export element tracker types
pub use element_tracker::{SharedElementTracker, TrackedElement};

/// Convert CSS features to compositor effects
pub fn features_to_effects(
    features: &[SyntheticFeature],
    viewport_width: u32,
    viewport_height: u32,
) -> Vec<Effect> {
    let mut effects = Vec::new();

    for feature in features {
        if let Some(effect) = effect_from_feature(feature, viewport_width, viewport_height) {
            effects.push(effect);
        }
    }

    effects
}

/// Synthetic feature from CSS normalization
#[derive(Debug, Clone)]
pub struct SyntheticFeature {
    pub feature_type: FeatureType,
    pub selector: String,
    pub original_value: String,
}

/// Types of synthetic CSS features
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureType {
    BackdropFilter,
    Transform,
    ColorAdjust,
    Clip,
}

fn effect_from_feature(
    feature: &SyntheticFeature,
    viewport_width: u32,
    viewport_height: u32,
) -> Option<Effect> {
    match feature.feature_type {
        FeatureType::BackdropFilter => {
            // Parse backdrop-filter: blur(10px)
            let blur_amount = parse_blur_amount(&feature.original_value);
            Some(Effect::blur(
                &feature.selector,
                blur_amount,
                viewport_width,
                viewport_height,
            ))
        }
        FeatureType::Transform => {
            // Parse transform: scale(1.1), translate(10px, 20px), etc.
            let transform = parse_transform(&feature.original_value);
            Some(Effect::transform(
                &feature.selector,
                transform,
                viewport_width,
                viewport_height,
            ))
        }
        FeatureType::ColorAdjust => {
            // Parse color-adjust: brightness(1.2) etc.
            let color_params = parse_color_adjust(&feature.original_value);
            Some(Effect::color_adjust(&feature.selector, color_params))
        }
        FeatureType::Clip => {
            // Parse clip-path or security clipping
            let region = parse_clip_region(&feature.original_value, viewport_width, viewport_height);
            Some(Effect::clip(region))
        }
    }
}

fn parse_blur_amount(value: &str) -> f32 {
    // Extract blur amount from "backdrop-filter: blur(10px)" or similar
    if let Some(start) = value.find("blur(") {
        let after = &value[start + 5..];
        if let Some(end) = after.find(')') {
            let num_str = &after[..end];
            // Remove 'px' suffix if present
            let num = num_str.trim().trim_end_matches("px").trim();
            return num.parse::<f32>().unwrap_or(10.0);
        }
    }
    10.0 // Default blur amount
}

fn parse_transform(value: &str) -> TransformParams {
    let mut params = TransformParams::default();

    // Parse scale(x), translate(x, y), rotate(deg)
    if let Some(start) = value.find("scale(") {
        let after = &value[start + 6..];
        if let Some(end) = after.find(')') {
            let scale_str = &after[..end];
            if let Ok(scale) = scale_str.parse::<f32>() {
                params.scale_x = scale;
                params.scale_y = scale;
            }
        }
    }

    if let Some(start) = value.find("translate(") {
        let after = &value[start + 10..];
        if let Some(end) = after.find(')') {
            let parts: Vec<&str> = after[..end].split(',').collect();
            if parts.len() >= 1 {
                let x = parts[0]
                    .trim()
                    .trim_end_matches("px")
                    .parse::<f32>()
                    .unwrap_or(0.0);
                params.translate_x = x;
            }
            if parts.len() >= 2 {
                let y = parts[1]
                    .trim()
                    .trim_end_matches("px")
                    .parse::<f32>()
                    .unwrap_or(0.0);
                params.translate_y = y;
            }
        }
    }

    if let Some(start) = value.find("rotate(") {
        let after = &value[start + 7..];
        if let Some(end) = after.find(')') {
            let rot_str = &after[..end];
            let rot = rot_str
                .trim()
                .trim_end_matches("deg")
                .parse::<f32>()
                .unwrap_or(0.0);
            params.rotate_degrees = rot;
        }
    }

    params
}

fn parse_color_adjust(value: &str) -> ColorAdjustParams {
    let mut params = ColorAdjustParams::default();

    // Parse brightness(1.2), contrast(0.8), etc.
    if let Some(start) = value.find("brightness(") {
        let after = &value[start + 11..];
        if let Some(end) = after.find(')') {
            let brightness_str = &after[..end];
            if let Ok(brightness) = brightness_str.parse::<f32>() {
                params.red_multiplier = brightness;
                params.green_multiplier = brightness;
                params.blue_multiplier = brightness;
            }
        }
    }

    params
}

fn parse_clip_region(_value: &str, viewport_width: u32, viewport_height: u32) -> Region {
    // Default to full viewport if parsing fails
    Region::new(0.0, 0.0, viewport_width as f32, viewport_height as f32)
}