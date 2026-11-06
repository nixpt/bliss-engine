# BLISS-MOTION-1 — spring timing + velocity-preserving retarget for transitions

**Status:** Backlog
**Filed:** 2026-11-06, from arniko `docs/RUNE_ENGINE_BORROW.md` §A (Rune concept borrow; arniko PR #29). Also GitHub issue #16.

## Problem

Rune's motion is **analytic springs**: `Spring(duration:bounce:)`, closed form for every damping
regime, with retargeting that keeps position **and** velocity, so interrupting a running animation
never jumps (`crates/rune-motion`).

`bliss-dom` already animates CSS transitions and animations via stylo (`packages/bliss-dom/src/stylo.rs`:
`AnimationState`, `has_css_transitions`, `has_css_animations`; `is_animating` in `document.rs`;
`resolve(time)` in `resolve.rs`). But the timing is CSS `cubic-bezier` / `steps` — there is **no
spring timing function** and **no velocity-preserving retarget**.

## Done when

- [ ] A **spring** usable as a CSS `linear()`-equivalent timing for `transition-*` (and available
      programmatically), with closed-form sampling across under/critical/over-damped regimes.
- [ ] **Retargeting keeps position and velocity** (sample-and-restart from the current
      `(value, velocity)`), so a mid-flight target change has no discontinuity.
- [ ] A reduced-motion policy and a time scale hook for the animation clock.

## Notes

- A working reference for the math (`Spring` / `Curve` / `Track`) is arniko `crates/arniko/src/motion`;
  it is a candidate for extraction into a small shared crate that both bliss-engine and
  `nixpt/mustang` depend on (mustang's animation is tween-only — see `nixpt/mustang` issue #10).
- arniko's `css::normalize` used to strip `transition`/`animation`/`@keyframes` before the document
  saw them; that is fixed in arniko PR #29, so the engine now receives them and this ticket makes the
  timing worth having.
