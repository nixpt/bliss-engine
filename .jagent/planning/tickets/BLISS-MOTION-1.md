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

- [x] A **spring** with closed-form sampling across under/critical/over-damped regimes —
      `bliss_traits::motion::Spring` (`new` / `sample` / `settling_duration`), in the shared leaf
      crate every `bliss-*` crate, `mustang` and `arniko` already depend on. *(Programmatic
      primitive landed; the CSS-timing and retarget API over it are the remaining items.)*
- [x] A **programmatic animation API on the document** — `BaseDocument::animate(node, property,
      target, spring)` + `tick_animations()` (samples the springs, writes changed values) +
      `animation_value` / `cancel_animation`; `is_animating` reports running springs. Independent
      of stylo; composes with the virtual clock. *(The CSS `transition-*` spring-timing
      alternative is not viable upstream — see the design note.)*
- [x] **Retargeting keeps position and velocity** — a later `animate` sample-and-restarts from the
      current `(value, velocity)`; test: `retarget_keeps_position`.
- [x] A **reduced-motion policy and a time scale hook** — `MotionSettings` (shared, in
      `bliss_traits::motion`) on the document: `set_reduced_motion(true)` snaps spatial animation;
      `set_time_scale(t)` multiplies the animation clock (`0.0` pauses, `2.0` doubles). Tests:
      `reduced_motion_snaps`, `time_scale_pauses_and_resumes`.

## Note

`bliss_traits::motion::Spring` supersedes a per-consumer copy (arniko already has an equivalent
`crates/arniko/src/motion`; it can re-export from here to dedupe).

## Notes

- A working reference for the math (`Spring` / `Curve` / `Track`) is arniko `crates/arniko/src/motion`;
  it is a candidate for extraction into a small shared crate that both bliss-engine and
  `nixpt/mustang` depend on (mustang's animation is tween-only — see `nixpt/mustang` issue #10).
- arniko's `css::normalize` used to strip `transition`/`animation`/`@keyframes` before the document
  saw them; that is fixed in arniko PR #29, so the engine now receives them and this ticket makes the
  timing worth having.

## Design (using the spring) — programmatic, not stylo

`bliss_traits::motion::Spring` is in place. To *use* it:

- **A · Programmatic document animation (recommended)** — `BaseDocument::animate(node, property,
  target, spring)` plus `tick_animations(now)`, which the shell calls each frame: sample the spring,
  write the style property, set `has_active_animations`, re-resolve. Retarget = call `animate` again
  (sample-and-restart from `(value, velocity)`). No stylo change, composes with the virtual clock,
  and gives every consumer (surfer, arniko) springs.
- **B · CSS spring timing** — make `transition-timing-function` accept a spring. That reaches into
  stylo's `TimingFunction` (upstream) — not viable without patching stylo.

**Recommendation:** A. The engine already resolves + repaints per frame while `has_active_animations`
is set, so a per-frame `tick_animations` slots in beside the existing fling stepping.
