# 🐎 Mustang

**GPU-Accelerated Effect Compositor for the Blitz DOM**

![Mustang Logo](../../../Projects/Exosphere/vendor/crushed-book/src/assets/mustang_logo.png)

Mustang is a high-performance rendering extension for the **Blitz** web engine. It transforms expensive CSS synthetic features—such as `backdrop-filter: blur()` and complex `transform` operations—into hardware-accelerated GPU commands using **Vello** and **WGPU**.

## 🚀 Features

- **Backdrop Blur**: Real-time Gaussian blur effects that bypass CPU pixel processing.
- **2D Transforms**: Sub-pixel accurate scaling, rotation, and translation.
- **Zero-Copy Composition**: Direct scene merging into the window renderer.
- **Security Clipping**: Hardware-enforced regions for multi-tenant isolation.
- **Effect Caching**: Intelligent caching of expensive effect parameters.

## 🏗️ Architecture

Mustang fits into the Blitz rendering pipeline as a "Synthetic Feature" handler.

1. **Feature Extraction**: `blitz-dom` identifies features the base engine shouldn't handle.
2. **Normalized Translation**: Parameters are converted into engine-agnostic `Effect` variants.
3. **GPU Execution**: Effects are applied directly to the Vello scene graph.

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
blitz-mustang = { path = "packages/blitz-mustang" }
```

## 🛠️ Usage

```rust
use blitz_mustang::{MustangCompositor, Effect, BlurParams};

let mut compositor = MustangCompositor::default();

// Create a blur effect for a glass card
let blur = Effect::blur(".glass-card", 15.0, 1920, 1080);

// Apply to your scene
compositor.apply_scene_effects(&mut scene, &[blur], (1920, 1080));
```

## ⚖️ License

Dual-licensed under **MIT** or **Apache-2.0**.

Created and maintained by **The Exosphere Authors**.
