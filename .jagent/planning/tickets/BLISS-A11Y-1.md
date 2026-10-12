# BLISS-A11Y-1 — `bliss-shell/accessibility` compiles a no-op adapter on Linux

**Status:** Backlog
**Filed:** 2026-10-12, from surfer-browser #22 (super-surfer accessibility)

## Problem

`bliss-shell`'s `accessibility` feature enables `dep:accesskit_xplat`, but the dependency is declared
`default-features = false` (`packages/bliss-shell/Cargo.toml`, `accesskit_xplat` → `bliss-accesskit-xplat`).
`bliss-accesskit-xplat`'s defaults are what enable `accesskit_unix` + `async-io`, so on Linux the adapter
compiles without a backend and never reaches AT-SPI. super-surfer works around it by depending on
`bliss-accesskit-xplat` with those features directly.

Separately (environment, not a bliss bug, but worth recording): accesskit_unix 0.20/0.21 activates only on
`org.a11y.Status.ScreenReaderEnabled`; an at-spi2-core 2.62 bus exposing only `IsEnabled` never activates it
unless a screen reader provides the property.

## Done when

- [ ] `accessibility = ["dep:accesskit", "dep:accesskit_xplat", "accesskit_xplat/accesskit_unix",
      "accesskit_xplat/async-io", "bliss-dom/accessibility"]` (or a `tokio` variant selected by the app).
- [ ] A smoke test or example documenting how to verify the tree with AT-SPI (incl. the ScreenReaderEnabled note).
- [ ] super-surfer drops its workaround after the release.
