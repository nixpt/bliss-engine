//! Mustang configuration
//!
//! Copyright (c) 2026 The Exosphere Authors
//!
//! Dual-licensed under MIT or Apache-2.0.

/// Mustang processing mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MustangMode {
    /// CPU-only processing (fallback)
    CpuOnly,
    /// GPU-accelerated processing
    GpuAccelerated,
    /// Hybrid mode (GPU with CPU fallback)
    Hybrid,
}

impl Default for MustangMode {
    fn default() -> Self {
        MustangMode::GpuAccelerated
    }
}

/// Configuration for Mustang compositor
#[derive(Debug, Clone)]
pub struct MustangConfig {
    /// Processing mode
    pub mode: MustangMode,
    /// Enable effect caching
    pub enable_caching: bool,
    /// Maximum cache size
    pub max_cache_size: usize,
    /// Enable debug visualization
    pub enable_debug: bool,
    /// GPU device selection
    pub gpu_device: Option<String>,
}

impl Default for MustangConfig {
    fn default() -> Self {
        Self {
            mode: MustangMode::GpuAccelerated,
            enable_caching: true,
            max_cache_size: 1000,
            enable_debug: false,
            gpu_device: None,
        }
    }
}

impl MustangConfig {
    /// Create a new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set processing mode
    pub fn mode(mut self, mode: MustangMode) -> Self {
        self.mode = mode;
        self
    }

    /// Enable/disable caching
    pub fn enable_caching(mut self, enable: bool) -> Self {
        self.enable_caching = enable;
        self
    }

    /// Set maximum cache size
    pub fn max_cache_size(mut self, size: usize) -> Self {
        self.max_cache_size = size;
        self
    }

    /// Enable/disable debug visualization
    pub fn enable_debug(mut self, enable: bool) -> Self {
        self.enable_debug = enable;
        self
    }

    /// Set GPU device
    pub fn gpu_device(mut self, device: &str) -> Self {
        self.gpu_device = Some(device.to_string());
        self
    }

    /// Create CPU-only configuration
    pub fn cpu_only() -> Self {
        Self::default().mode(MustangMode::CpuOnly)
    }

    /// Create GPU-accelerated configuration
    pub fn gpu_accelerated() -> Self {
        Self::default().mode(MustangMode::GpuAccelerated)
    }

    /// Create hybrid configuration
    pub fn hybrid() -> Self {
        Self::default().mode(MustangMode::Hybrid)
    }
}

/// Color scheme for themes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    Light,
    Dark,
    HighContrast,
}

impl Default for ColorScheme {
    fn default() -> Self {
        ColorScheme::Light
    }
}

/// Configuration for application themes
#[derive(Debug, Clone)]
pub struct ThemeConfig {
    /// Theme name (e.g., "glass", "default")
    pub name: String,
    /// Color scheme
    pub color_scheme: ColorScheme,
    /// Enable transparency
    pub transparent: bool,
    /// Border radius
    pub border_radius: f32,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            color_scheme: ColorScheme::default(),
            transparent: false,
            border_radius: 8.0,
        }
    }
}

impl ThemeConfig {
    /// Create a new theme configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set theme name
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }
}
