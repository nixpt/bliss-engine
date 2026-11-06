# BLISS-DEPS-1 — Dependency depth: what's load-bearing, and what "homegrown" would cost

**Status:** Backlog (analysis)
**Filed:** 2026-11-06, from a dependency/utilization audit of bliss-engine.

## What's ours vs borrowed

**In-tree (homegrown):**

| Package | src LOC |
| --- | --- |
| `bliss-dom` (DOM + layout pipeline + Stylo bridge + events) | 18,964 |
| `bliss-paint` (anyrender command generator: boxes, text, shadows, gradients) | 3,749 |
| `bliss-traits` (shared traits) | 1,938 |
| `bliss-shell` (winit window + driver) | 1,849 |
| `stylo_taffy` (the Stylo ↔ Taffy bridge) | 1,846 |
| `bliss` (umbrella) | 947 |
| `bliss-html` (parser wiring) | 369 |
| `bliss-net` | 327 |

**In-tree forks, published under `bliss-*` names:** `accesskit_xplat` (509) + `anyrender_vello` (769)
= **1,018 LOC** total — small, boundary-level forks (graceful UIKit/WinRT fallback; vello scene
overlays). These are the right size to own.

**Borrowed (crates.io, NOT vendored — no `vendor/`, no `[patch]` in the root):** the Servo **CSS
stack** (`stylo`, `stylo_dom`, `stylo_traits`, `stylo_config`, `stylo_atoms`, `selectors`,
`cssparser`), **`taffy`** (layout), **`parley`** (+ `fontique`/`swash`/`skrifa`; text), **`euclid`**
/`app_units` (geometry/units), **`anyrender`** (+ vello), **`accesskit`**, `image`, `url`, `winit`,
`wgpu`.

`bliss-dom` alone pulls **192** unique normal-transitive crates.

## How deep is the coupling?

Touchpoint counts in `bliss-dom`:

| Surface | occurrences |
| --- | --- |
| `style::` (stylo internals) | 161 |
| `ServoArc` | 33 |
| `selectors::` | 22 |
| `ComputedValues` | 19 |
| `Stylist` / `PropertyId` / `cssparser` | 4 / 3 / 1 |
| `taffy::` | 126 |
| `parley` | 121 |

Stylo is **driven** (parse the stylesheet → cascade → computed styles), and `bliss-dom` then
**reads** `ComputedValues` to populate Taffy styles and Parley text. The touchpoints live mostly in
`stylo.rs`, the node's `stylo_element_data`, and `layout/*`. So the Servo stack is load-bearing but
reached through a **bounded surface** (one style driver + one layout bridge).

## What a homegrown version would take

- **stylo** — the full CSS engine: parsing, selector matching, the cascade, computed values for
  every CSS property, animations. This is Servo's crown jewel: hundreds of thousands of LOC,
  spec-complete, years of work. Reimplementing is **rebuilding a browser's style engine**. Not
  viable, and not where bliss's value is.
- **taffy** — flexbox + grid. Rust, ~tens of thousands of LOC. Reimplementing is *feasible but
  large*, and would only reproduce a solved problem.
- **parley** (+ fontique/swash/skrifa) — shaping + text layout + font management. Huge.
- **anyrender/vello** — GPU 2D. Huge.

**Conclusion:** homegrown replacements for these are **out of scope**. "Buy, don't build" is the
correct posture for a browser engine; the value is in the integration, not in re-deriving CSS.

## Where homegrown *does* pay

- **The integration / IP** — bliss-dom's layout pipeline, the `stylo_taffy` bridge, `bliss-paint`'s
  painter. Already homegrown.
- **Boundary forks** — `accesskit_xplat`, `anyrender_vello`. Right-sized.
- **Our own engine layers** — motion (`Spring` in `bliss-traits`), headless tooling
  (dump/clock/synthetic input), mutation ops. These are bliss-native and should stay so.

## De-risking (not replacing)

1. **Pin deliberately** (the workspace lock already does) and track upstream majors (BLISS-PIPE-1
   covers the release/publish hygiene around this).
2. **Keep the surface bounded**: one `stylo_taffy` bridge + one `stylo.rs` driver; don't let
   `style::` / `taffy::` leak into more modules than necessary.
3. **Do NOT abstract stylo/taffy/parley behind traits** — a wrap-for-wrap's-sake layer costs more
   than it saves (these are not going to be swapped).
