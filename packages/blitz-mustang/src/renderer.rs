//! Vello renderer integration for Mustang
//!
//! Copyright (c) 2026 The Exosphere Authors
//!
//! Dual-licensed under MIT or Apache-2.0.
//!
//! This module provides the glue between Mustang effects and 
//! the Vello rendering engine.

use anyrender::PaintScene;
use super::effect::{Effect, ApplyEffect};
use vello::Scene;

/// Extension trait for PaintScene to add Mustang effect support
///
/// This allows any PaintScene implementation (including VelloScenePainter)
/// to apply Mustang effects directly.
pub trait EffectScene: PaintScene + Sized {
    /// Apply a single effect to the scene
    fn apply_effect(&mut self, effect: &Effect, viewport: (u32, u32)) {
        effect.apply_to_scene(self, viewport);
    }
    
    /// Apply multiple effects to the scene
    fn apply_effects(&mut self, effects: &[Effect], viewport: (u32, u32)) {
        for effect in effects {
            self.apply_effect(effect, viewport);
        }
    }
}

// Implement EffectScene for any type that implements PaintScene
impl<S: PaintScene> EffectScene for S {}

/// 🐎 Mustang Scene Bundle for Zero-Copy Rendering
///
/// Stores a complete scene with its effects metadata for hardware-accelerated composition.
/// This enables efficient scene reuse without CPU pixel copying.
pub struct MustangSceneBundle {
    /// The Vello scene containing all rendered content
    pub scene: Scene,
    /// Effects to apply during composition
    pub effects: Vec<Effect>,
    /// Viewport dimensions (width, height)
    pub viewport: (u32, u32),
    /// Tab/capsule identifier
    pub tab_id: String,
    /// Timestamp for cache eviction
    pub created_at: std::time::Instant,
}

impl std::fmt::Debug for MustangSceneBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MustangSceneBundle")
            .field("effects", &self.effects)
            .field("viewport", &self.viewport)
            .field("tab_id", &self.tab_id)
            .field("created_at", &self.created_at)
            .finish_non_exhaustive()
    }
}

impl MustangSceneBundle {
    /// Create a new scene bundle
    pub fn new(
        scene: Scene,
        effects: Vec<Effect>,
        viewport: (u32, u32),
        tab_id: String,
    ) -> Self {
        Self {
            scene,
            effects,
            viewport,
            tab_id,
            created_at: std::time::Instant::now(),
        }
    }

    /// Check if the bundle is stale (older than timeout)
    pub fn is_stale(&self, timeout: std::time::Duration) -> bool {
        self.created_at.elapsed() > timeout
    }

    /// Get the age of the bundle
    pub fn age(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }
}

// Re-export anyrender_vello types for convenience
pub use anyrender_vello::{VelloWindowRenderer, VelloScenePainter, VelloRendererOptions};
