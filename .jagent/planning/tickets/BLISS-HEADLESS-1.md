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
- [ ] **Synthetic input** (click / type / key / scroll / fling) so a scripted scenario renders
      deterministically.
- [ ] A smoke script/example that drives a page headlessly and diffs two frames.

## Note

- Double-click detection still uses `Instant` (`last_mousedown_time`), outside the virtual
  clock; folding it in is a follow-up.

## Notes

- `bliss-paint` already paints a resolved document; a headless `render_to_png`-style path exists in
  consumers (e.g. surfer-page's `VisualBackend`), so the missing piece is the **dump + clock + input**,
  not the paint.
