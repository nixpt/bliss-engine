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
- [ ] **Scroll-snap**.
- [ ] Sticky headers carry a stuck-shadow once pinned.
- [ ] (larger, later) **Virtualized rows** and **cached subtrees** — realize/paint only what is on
      screen; rasterize a settled subtree and composite it.

## Notes

- The first two are small and high-visibility; virtualization and cached subtrees are the real
  performance work and can be split into follow-ups.
