//! Programmatic spring animation of style properties (BLISS-MOTION-1, route A).
//!
//! [`BaseDocument::animate`] registers (or retargets) a spring on a numeric CSS
//! property; the shell calls [`BaseDocument::tick_animations`] each frame, which
//! samples the springs and writes changed values. This is independent of stylo's
//! CSS timing (a spring is not a CSS easing), and it composes with the virtual
//! clock ([`BaseDocument::set_clock_ms`]) for deterministic frames.
//!
//! A [`MotionSettings`] policy applies: `reduced_motion` makes spatial animation
//! snap, and `time_scale` multiplies the animation clock (`0.0` pauses).
//!
//! As elsewhere in bliss, **nothing animates on the first frame**: the first
//! `animate` for a `(node, property)` sets the value instantly, and later calls
//! spring. Retargeting keeps position and velocity, so interrupting never jumps.

use bliss_traits::motion::{MotionSettings, Spring};

use crate::BaseDocument;

/// One spring-animated `(node, property)` lane.
pub(crate) struct Animated {
    spring: Spring,
    /// Start of the current segment.
    from: f32,
    from_velocity: f32,
    target: f32,
    /// Segment start time, seconds (wall clock).
    start: f64,
    /// The spring's settling time, in animation seconds.
    settle_secs: f32,
    value: f32,
    velocity: f32,
    /// Last value written to the DOM.
    written: Option<f32>,
    active: bool,
}

impl Animated {
    /// A lane settled at `value` (the first set is instant).
    fn settled(value: f32, spring: Spring) -> Animated {
        Animated {
            spring,
            from: value,
            from_velocity: 0.0,
            target: value,
            start: 0.0,
            settle_secs: 0.0,
            value,
            velocity: 0.0,
            written: None,
            active: false,
        }
    }

    /// Retarget from the current `(value, velocity)` toward `target`.
    fn retarget(&mut self, target: f32, spring: Spring, now: f64, time_scale: f32) {
        self.from = self.value;
        self.from_velocity = self.velocity;
        self.target = target;
        self.spring = spring;
        self.start = now;
        self.settle_secs = spring.settling_duration(1e-3);
        self.active = true;
        self.advance(now, time_scale);
    }

    /// Jump to `target` with no motion (reduced motion).
    fn snap(&mut self, target: f32) {
        self.from = target;
        self.from_velocity = 0.0;
        self.target = target;
        self.settle_secs = 0.0;
        self.value = target;
        self.velocity = 0.0;
        self.active = false;
    }

    /// Sample at `now` (wall seconds), scaled by `time_scale`; snap to the target
    /// once the segment is over.
    fn advance(&mut self, now: f64, time_scale: f32) {
        if !self.active {
            return;
        }
        let elapsed = ((now - self.start).max(0.0) as f32) * time_scale;
        let (x, v) = self.spring.sample(self.from, self.from_velocity, self.target, elapsed);
        self.value = x;
        self.velocity = v;
        if elapsed >= self.settle_secs {
            self.value = self.target;
            self.velocity = 0.0;
            self.active = false;
        }
    }
}

impl BaseDocument {
    /// Animate the numeric style `property` of `node` toward `target` on `spring`.
    ///
    /// The first call for a `(node, property)` sets the value instantly; a later
    /// call retargets the running spring from the current `(value, velocity)`.
    /// Under `reduced_motion` the value snaps instead. Values reach the DOM on
    /// the next [`tick_animations`](BaseDocument::tick_animations).
    pub fn animate(&mut self, node: usize, property: &str, target: f32, spring: Spring) {
        let key = (node, property.to_string());
        if self.motion.reduced_motion {
            match self.spring_animations.get_mut(&key) {
                Some(anim) => anim.snap(target),
                None => {
                    self.spring_animations.insert(key, Animated::settled(target, spring));
                }
            }
            return;
        }
        let now = self.now_ms() / 1000.0;
        let scale = self.motion.time_scale;
        match self.spring_animations.get_mut(&key) {
            Some(anim) => anim.retarget(target, spring, now, scale),
            None => {
                self.spring_animations.insert(key, Animated::settled(target, spring));
            }
        }
    }

    /// Advance every programmatic animation on the document clock, writing changed
    /// values. Returns `true` if the DOM changed (a redraw is due). Call each
    /// frame while [`is_animating`](BaseDocument::is_animating).
    pub fn tick_animations(&mut self) -> bool {
        if self.spring_animations.is_empty() {
            return false;
        }
        let now = self.now_ms() / 1000.0;
        let scale = self.motion.time_scale;

        let mut updates: Vec<(usize, String, f32)> = Vec::new();
        for ((node, property), anim) in self.spring_animations.iter_mut() {
            anim.advance(now, scale);
            if anim.written != Some(anim.value) {
                anim.written = Some(anim.value);
                updates.push((*node, property.clone(), anim.value));
            }
        }
        if updates.is_empty() {
            return false;
        }

        let mut mutator = self.mutate();
        for (node, property, value) in &updates {
            mutator.set_style_property(*node, property, &value.to_string());
        }
        true
    }

    /// The current presented value of a programmatic animation, if any.
    pub fn animation_value(&self, node: usize, property: &str) -> Option<f32> {
        self.spring_animations
            .get(&(node, property.to_string()))
            .map(|a| a.value)
    }

    /// Stop animating `node`'s `property` (the value stays as last written).
    pub fn cancel_animation(&mut self, node: usize, property: &str) {
        self.spring_animations.remove(&(node, property.to_string()));
    }

    /// The document's motion policy.
    pub fn motion_settings(&self) -> MotionSettings {
        self.motion
    }

    /// Set the animation clock multiplier: `1.0` normal, `0.0` paused, `2.0`
    /// double speed.
    pub fn set_time_scale(&mut self, scale: f32) {
        self.motion.time_scale = scale;
    }

    /// Enable or disable reduced motion: when set, spatial animation snaps.
    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.motion.reduced_motion = reduced;
    }

    /// Whether any programmatic spring is still running.
    pub(crate) fn programmatic_animating(&self) -> bool {
        self.spring_animations.values().any(|a| a.active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentConfig, qual_name};

    fn doc_with_div() -> (BaseDocument, usize) {
        let mut doc = BaseDocument::new(DocumentConfig::default());
        let mut mutator = doc.mutate();
        let node = mutator.create_element(qual_name!("div"), vec![]);
        mutator.append_children(0, &[node]);
        drop(mutator);
        (doc, node)
    }

    #[test]
    fn first_animate_is_instant_then_springs() {
        let (mut doc, node) = doc_with_div();
        let prop = "opacity";

        doc.set_clock_ms(Some(0.0));
        doc.animate(node, prop, 0.0, Spring::new(0.3, 0.0));
        assert!(doc.tick_animations(), "first tick writes the value");
        assert_eq!(doc.animation_value(node, prop), Some(0.0));
        assert!(!doc.is_animating());

        doc.animate(node, prop, 1.0, Spring::new(0.3, 0.0));
        doc.set_clock_ms(Some(50.0));
        assert!(doc.tick_animations());
        let mid = doc.animation_value(node, prop).expect("value");
        assert!(mid > 0.0 && mid < 1.0, "mid={mid}");
        assert!(doc.is_animating());

        doc.set_clock_ms(Some(2000.0));
        doc.tick_animations();
        assert_eq!(doc.animation_value(node, prop), Some(1.0));
        assert!(!doc.is_animating());
    }

    #[test]
    fn retarget_keeps_position() {
        let (mut doc, node) = doc_with_div();
        let prop = "width";

        doc.set_clock_ms(Some(0.0));
        doc.animate(node, prop, 0.0, Spring::new(0.3, 0.0));
        doc.tick_animations();
        doc.animate(node, prop, 100.0, Spring::new(0.3, 0.0));
        doc.set_clock_ms(Some(100.0));
        doc.tick_animations();
        let before = doc.animation_value(node, prop).expect("value");

        doc.animate(node, prop, 40.0, Spring::new(0.3, 0.0));
        assert_eq!(doc.animation_value(node, prop), Some(before), "no jump on retarget");
    }

    #[test]
    fn reduced_motion_snaps() {
        let (mut doc, node) = doc_with_div();
        doc.set_reduced_motion(true);
        doc.animate(node, "opacity", 1.0, Spring::new(0.3, 0.4));
        assert_eq!(doc.animation_value(node, "opacity"), Some(1.0));
        assert!(!doc.is_animating(), "reduced motion does not animate");
    }

    #[test]
    fn time_scale_pauses_and_resumes() {
        let (mut doc, node) = doc_with_div();
        let prop = "width";

        doc.set_clock_ms(Some(0.0));
        doc.animate(node, prop, 0.0, Spring::new(0.3, 0.0));
        doc.tick_animations();
        doc.animate(node, prop, 100.0, Spring::new(0.3, 0.0));

        // Paused: the clock advances but the animation does not.
        doc.set_time_scale(0.0);
        doc.set_clock_ms(Some(500.0));
        doc.tick_animations();
        assert_eq!(doc.animation_value(node, prop), Some(0.0), "paused");

        // Resumed: it settles.
        doc.set_time_scale(1.0);
        doc.set_clock_ms(Some(2000.0));
        doc.tick_animations();
        assert_eq!(doc.animation_value(node, prop), Some(100.0));
    }
}
