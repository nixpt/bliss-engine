# BLISS-SCROLL-1 — scroll polish: edge fade + snap, then virtualization

**Status:** Backlog
**Filed:** 2026-11-06, from arniko `docs/RUNE_ENGINE_BORROW.md` §D (Rune concept borrow; arniko PR #29). Also GitHub issue #17.

## Problem

Rune's scroll surfaces add: virtual rows (only an overscanned window is realized), scroll **edge
fade** (a viewport fades where content is hidden under its edge), sticky headers with a stuck-shadow,
tuned fling decay, scroll-snap / overscroll, and cached subtrees (rasterize a settled subtree).

`bliss-dom` has `position: sticky` in layout and a fling
(`packages/bliss-dom/src/events/pointer.rs::generate_fling`,
`ScrollAnimationState::Fling` in `resolve.rs`). It has **no** scroll edge fade, scroll-snap,
overscroll, virtualized rows, or cached-subtree rasterization.

## Done when

- [ ] **Scroll edge fade** — fade a viewport's top/bottom (and left/right) only while content is
      hidden under them.
- [x] **Scroll-snap** (programmatic) — `SnapPoints` + `nearest_snap` in `src/scroll.rs`, and
      `BaseDocument::set_scroll_snap` / `scroll_snap_target`, applied when a pan/fling settles
      (`resolve_scroll_animation`). Remaining: wire CSS `scroll-snap-type` / `scroll-snap-align`
      to these points (needs per-child layout geometry).
- [ ] Sticky headers carry a stuck-shadow once pinned.
- [ ] (larger, later) **Virtualized rows** and **cached subtrees** — realize/paint only what is on
      screen; rasterize a settled subtree and composite it.

## Notes

- The first two are small and high-visibility; virtualization and cached subtrees are the real
  performance work and can be split into follow-ups.

## Design (edge fade) — standard vs bespoke, decide first

`bliss-paint` draws no scroll-edge fade today, and there is no CSS trigger: `mask-image` is only
treated as a stacking-context flag (`node.rs`), never painted. Two routes:

- **A · Standard (`mask-image`)** — support a linear-gradient mask so a container fades at its edges
  (`mask-image: linear-gradient(transparent, #000 15px, #000 calc(100% - 15px), transparent)`).
  General (fades, marquees, image masks), the way the web does it. Cost: gradient **mask
  compositing** in `bliss-paint` (a new render op/pass). The right long-term home.
- **B · Bespoke auto-fade** — `bliss-paint` draws a top/bottom gradient overlay on a scroll
  container, strength = hidden extent / 15 pt, opt-in per node (mirrors `set_scroll_snap`).
  Contained and quick, but non-standard and a second mechanism to maintain.

**Recommendation:** A long-term; B only as a contained first slice if the visual is wanted now.
Needs a maintainer call on standard-vs-bespoke before coding.
