# bliss-anyrender-vello

Vello rendering backend for [`anyrender`](https://github.com/DioxusLabs/anyrender), forked
for the [Bliss](https://github.com/nixpt/bliss-engine) rendering stack.

Forked from [DioxusLabs/anyrender](https://github.com/DioxusLabs/anyrender)'s `anyrender_vello`
crate (published as `anyrender_vello` on crates.io). This fork adds `SceneOverlay`/
`set_scene_effects` hooks (see `src/window_renderer.rs`) used by Bliss's `mustang`
GPU post-processor (blur/transform CSS effects). See [NOTICE](../../NOTICE) for the full
upstream attribution.

Published under the crates.io name `bliss-anyrender-vello` (the bare `anyrender_vello` name
belongs to the upstream DioxusLabs crate); the in-repo library name stays `anyrender_vello`
via `[lib] name`.

Licensed under MIT OR Apache-2.0, same as upstream.
