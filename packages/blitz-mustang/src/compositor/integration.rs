//! Integration helpers for connecting compositor with Blitz applications
//!
//! Copyright (c) 2026 The Exosphere Authors
//!
//! Dual-licensed under MIT or Apache-2.0.
//!
//! This module provides utilities to integrate the compositor
//! with the component system and rendering pipeline.


use crate::{MustangCompositor, MustangConfig};
use crate::config::ThemeConfig;

/// Theme-aware compositor that applies effects based on theme metadata
pub struct ThemeCompositor {
    compositor: MustangCompositor,
    current_theme: String,
}

impl ThemeCompositor {
    /// Create a new theme compositor
    pub fn new() -> Self {
        Self {
            compositor: MustangCompositor::new(MustangConfig::default()),
            current_theme: "default".to_string(),
        }
    }

    /// Create a new theme compositor with custom configuration
    pub fn with_config(config: MustangConfig) -> Self {
        Self {
            compositor: MustangCompositor::new(config),
            current_theme: "default".to_string(),
        }
    }

    /// Update the current theme
    pub fn set_theme(&mut self, theme: &str) {
        self.current_theme = theme.to_string();
    }

    /// Apply compositor effects for the current theme to a frame buffer
    ///
    /// This would be called after rendering the base content but before final display
    pub fn composite_frame(
        &mut self,
        buffer: &[u8],
        _width: u32,
        _height: u32,
        theme_config: &ThemeConfig,
    ) -> anyhow::Result<Vec<u8>> {
        // Get synthetic features from theme metadata
        let _effects = self.extract_theme_effects(theme_config, _width, _height);

        // TODO: Port to Vello scene composition
        // MustangCompositor now works on PaintScene, not raw buffers.
        Ok(buffer.to_vec())
    }

    /// Get the underlying compositor for advanced usage
    pub fn compositor(&mut self) -> &mut MustangCompositor {
        &mut self.compositor
    }

    /// Extract effects from theme configuration
    fn extract_theme_effects(
        &self,
        theme_config: &ThemeConfig,
        width: u32,
        height: u32,
    ) -> Vec<crate::compositor::Effect> {
        let mut effects = Vec::new();

        // Check if theme has glass morphism effects
        if theme_config.name.contains("glass") || theme_config.name.contains("frosted") {
            effects.push(crate::compositor::Effect::blur(
                ".glass-panel",
                10.0,
                width,
                height,
            ));
        }

        // Check if theme has cyberpunk effects
        if theme_config.name.contains("cyberpunk") {
            effects.push(crate::compositor::Effect::color_adjust(
                ".cyberpunk-element",
                crate::compositor::ColorAdjustParams {
                    red_multiplier: 1.2,
                    green_multiplier: 0.8,
                    blue_multiplier: 1.5,
                    red_offset: 0.1,
                    green_offset: -0.05,
                    blue_offset: 0.2,
                },
            ));
        }

        effects
    }
}

impl Default for ThemeCompositor {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to check if a theme has effects that need compositing
pub fn theme_has_effects(theme_config: &ThemeConfig) -> bool {
    theme_config.name.contains("glass")
        || theme_config.name.contains("frosted")
        || theme_config.name.contains("cyberpunk")
        || theme_config.name.contains("aurora")
}

/// Component effect integration
pub struct ComponentEffects {
    effects: Vec<crate::compositor::Effect>,
}

impl ComponentEffects {
    /// Create a new component effects collection
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    /// Add a blur effect to a component
    pub fn blur(
        mut self,
        selector: &str,
        radius: f32,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Self {
        self.effects.push(crate::compositor::Effect::blur(
            selector,
            radius,
            viewport_width,
            viewport_height,
        ));
        self
    }

    /// Add a transform effect to a component
    pub fn transform(
        mut self,
        selector: &str,
        params: crate::compositor::TransformParams,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Self {
        self.effects.push(crate::compositor::Effect::transform(
            selector,
            params,
            viewport_width,
            viewport_height,
        ));
        self
    }

    /// Add a color adjustment effect to a component
    pub fn color_adjust(
        mut self,
        selector: &str,
        params: crate::compositor::ColorAdjustParams,
    ) -> Self {
        self.effects
            .push(crate::compositor::Effect::color_adjust(selector, params));
        self
    }

    /// Add a clip effect to a component
    pub fn clip(mut self, region: crate::compositor::Region) -> Self {
        self.effects.push(crate::compositor::Effect::clip(region));
        self
    }

    /// Get all effects
    pub fn effects(&self) -> &[crate::compositor::Effect] {
        &self.effects
    }

    /// Consume into effects vector
    pub fn into_effects(self) -> Vec<crate::compositor::Effect> {
        self.effects
    }
}

impl Default for ComponentEffects {
    fn default() -> Self {
        Self::new()
    }
}

/// Security gating integration
pub struct SecurityGating {
    enabled: bool,
    security_regions: Vec<crate::compositor::Region>,
}

impl SecurityGating {
    /// Create a new security gating system
    pub fn new() -> Self {
        Self {
            enabled: true,
            security_regions: Vec::new(),
        }
    }

    /// Enable/disable security gating
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a security region
    pub fn add_security_region(mut self, region: crate::compositor::Region) -> Self {
        self.security_regions.push(region);
        self
    }

    /// Apply security clipping to a frame buffer
    pub fn apply_security_clipping(
        &self,
        buffer: &[u8],
        _width: u32,
        _height: u32,
    ) -> anyhow::Result<Vec<u8>> {
        if !self.enabled || self.security_regions.is_empty() {
            return Ok(buffer.to_vec());
        }

        // For now, return original buffer
        // In a real implementation, we would apply clipping masks
        Ok(buffer.to_vec())
    }

    /// Check if a point is within any security region
    pub fn is_point_secured(&self, x: f32, y: f32) -> bool {
        self.security_regions
            .iter()
            .any(|region| region.contains(x, y))
    }

    /// Get security regions that intersect with a given area
    pub fn get_intersecting_regions(
        &self,
        area: &crate::compositor::Region,
    ) -> Vec<&crate::compositor::Region> {
        self.security_regions
            .iter()
            .filter(|region| region.intersects(area))
            .collect()
    }
}

impl Default for SecurityGating {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ColorScheme, ThemeConfig};

    #[test]
    fn test_theme_compositor_new() {
        let compositor = ThemeCompositor::new();
        assert_eq!(compositor.current_theme, "default");
    }

    #[test]
    fn test_theme_compositor_set_theme() {
        let mut compositor = ThemeCompositor::new();
        compositor.set_theme("dark");
        assert_eq!(compositor.current_theme, "dark");
    }

    #[test]
    fn test_theme_has_effects() {
        let glass_theme = ThemeConfig::new().name("glass-morphism");
        assert!(theme_has_effects(&glass_theme));

        let plain_theme = ThemeConfig::new().name("plain");
        assert!(!theme_has_effects(&plain_theme));
    }

    #[test]
    fn test_component_effects() {
        let effects = ComponentEffects::new()
            .blur(".glass", 10.0, 800, 600)
            .transform(
                ".card",
                crate::compositor::TransformParams::default(),
                800,
                600,
            );

        assert_eq!(effects.effects().len(), 2);
    }

    #[test]
    fn test_security_gating() {
        let gating = SecurityGating::new()
            .add_security_region(crate::compositor::Region::new(0.0, 0.0, 100.0, 100.0));

        assert!(gating.is_point_secured(50.0, 50.0));
        assert!(!gating.is_point_secured(150.0, 50.0));
    }

    #[test]
    fn test_extract_theme_effects() {
        let compositor = ThemeCompositor::new();
        let glass_theme = ThemeConfig::new().name("glass-panel");
        let effects = compositor.extract_theme_effects(&glass_theme, 800, 600);

        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].effect_type,
            crate::compositor::EffectType::BackdropBlur
        ));
    }
}
