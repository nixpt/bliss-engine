# BLISS-HEADLESS-1 — agent headless tooling: element-tree JSON dump + virtual clock + scripted input

**Status:** Backlog
**Filed:** 2026-11-06, from arniko `docs/RUNE_ENGINE_BORROW.md` §C (Rune concept borrow; arniko PR #29). Also GitHub issue #15.

## Problem

[Rune](https://github.com/anomalyco/rune) makes every app runnable **headless**: render to PNG,
dump the element tree as JSON (`--tree`), and drive it with a scripted input file (click /
double-click / type / key / paste / scroll / fling) on a fixed **virtual clock**, so a scenario is
deterministic and an agent can verify a UI with no display (`crates/rune/src/tooling`).

bliss has only stdout debug: `packages/bliss-dom/src/debug.rs` (`debug_log_node`,
`print_taffy_tree`). `bliss-shell` has no headless script or virtual clock. An agent editing
markup/CSS blind therefore cannot see what it built or check it.

## Done when

- [x] A **JSON serialization of the resolved DOM + layout tree** — `BaseDocument::document_tree_json()`
      in `packages/bliss-dom/src/debug.rs` (compact, dependency-free; per node: `tag` / `text` /
      `comment` / `shadow_root`, optional `attrs` and `anonymous`, `rect` `[x,y,w,h]`, `children`).
      Block-layout rects only — inline content reports zero. Reference: arniko
      `crates/arniko/src/inspect.rs` (`document_tree`).
- [x] A **deterministic virtual clock** — `BaseDocument::set_clock_ms(Option<f64>)` / `now_ms()`.
      The fling (pointer pan → pointer up → `resolve`) now reads the document clock instead of
      `SystemTime`, so scrolling reproduces frame-for-frame when the clock is set. (`resolve(time)`
      already took a time for styles; the wall-clock leak was the fling.)
- [x] **Synthetic input** — `EventDriver::{synthesize_pointer_move, synthesize_pointer_down,
      synthesize_pointer_up, synthesize_click, synthesize_wheel, synthesize_key_down,
      synthesize_key_up}`, built as the same `UiEvent`s the shell feeds and run through the
      handler chain, sink and default actions. Tests in `src/tests/events.rs` (hover, click, wheel).
- [x] A headless smoke — `headless_input_then_dump`: a synthetic click runs through to a handler
      that mutates the DOM, and `document_tree_json` reflects it. (Delivered as a test; bliss-dom
      has no HTML parser, so an HTML-driven example belongs in a crate that does.)

## Note

- Double-click detection still uses `Instant` (`last_mousedown_time`), outside the virtual
  clock; folding it in is a follow-up.

## Notes

- `bliss-paint` already paints a resolved document; a headless `render_to_png`-style path exists in
  consumers (e.g. surfer-page's `VisualBackend`), so the missing piece is the **dump + clock + input**,
  not the paint.
