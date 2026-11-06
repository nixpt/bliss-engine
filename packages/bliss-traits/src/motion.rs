//! Spring motion math, shared by every `bliss-*` crate and by consumers
//! (`mustang`, `arniko`). Perceptual duration + bounce, closed-form sampling
//! across every damping regime, so an animation can be sampled at a frame
//! timestamp and retargeted from the current `(position, velocity)` with no
//! discontinuity. See the `BLISS-MOTION-1` ticket.
//!
//! ```text
//! ω = 2π / duration          stiffness k = ω²   (mass 1)
//! ζ = 1 − bounce             (bounce ≥ 0)       damping   c = 2ζω
//! ζ = 1 / (1 + bounce)       (bounce < 0, overdamped)
//! ```

/// A damped spring with unit mass, described by perceptual duration and bounce.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    /// Perceptual duration in seconds (the undamped period, `2π/ω`).
    pub duration: f32,
    /// Damping character in `(-1, 1)`: `0` critically damped, `> 0` overshoots,
    /// `< 0` overdamped.
    pub bounce: f32,
}

/// Upper bound on any reported settling time, in seconds.
const MAX_SETTLE: f32 = 120.0;

/// Motion policy for a document or app (BLISS-MOTION-1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionSettings {
    /// Clock multiplier for animations: `1.0` normal, `0.0` paused, `2.0` double
    /// speed.
    pub time_scale: f32,
    /// When set, spatial animation snaps instead of travelling (accessibility).
    pub reduced_motion: bool,
}

impl Default for MotionSettings {
    fn default() -> Self {
        MotionSettings {
            time_scale: 1.0,
            reduced_motion: false,
        }
    }
}

impl Spring {
    /// `Spring(duration:bounce:)`. Panics on a non-positive duration or a bounce
    /// outside `(-1, 1)`.
    pub const fn new(duration: f32, bounce: f32) -> Spring {
        assert!(duration > 0.0 && duration < 1e4, "spring duration must be positive seconds");
        assert!(bounce > -1.0 && bounce < 1.0, "spring bounce must be in (-1, 1)");
        Spring { duration, bounce }
    }

    /// Angular frequency `ω = 2π / duration` (rad/s).
    #[inline]
    pub fn omega(self) -> f32 {
        core::f32::consts::TAU / self.duration
    }

    /// Damping ratio `ζ`.
    #[inline]
    pub fn zeta(self) -> f32 {
        if self.bounce >= 0.0 {
            1.0 - self.bounce
        } else {
            1.0 / (1.0 + self.bounce)
        }
    }

    /// Position and velocity `t` seconds after leaving `x0` at velocity `v0`
    /// toward `target`. Closed form for every damping regime and exact at any
    /// `t` (including `t < 0`, which returns the start state).
    pub fn sample(self, x0: f32, v0: f32, target: f32, t: f32) -> (f32, f32) {
        let a = x0 - target;
        let t = t.max(0.0);
        if a == 0.0 && v0 == 0.0 {
            return (target, 0.0);
        }
        let omega = self.omega();
        let zeta = self.zeta();

        if (zeta - 1.0).abs() < 1e-4 {
            // Critically damped: (a + b·t)·e^{−ωt}.
            let b = v0 + omega * a;
            let e = (-omega * t).exp();
            (target + (a + b * t) * e, (b - omega * (a + b * t)) * e)
        } else if zeta < 1.0 {
            // Under-damped: e^{−σt}(a·cos(ωd·t) + b·sin(ωd·t)).
            let sigma = zeta * omega;
            let wd = omega * (1.0 - zeta * zeta).sqrt();
            let b = (v0 + sigma * a) / wd;
            let e = (-sigma * t).exp();
            let (s, c) = (wd * t).sin_cos();
            (
                target + e * (a * c + b * s),
                e * ((b * wd - sigma * a) * c - (a * wd + sigma * b) * s),
            )
        } else {
            // Over-damped: c1·e^{r1·t} + c2·e^{r2·t}, both roots negative.
            let root = (zeta * zeta - 1.0).sqrt();
            let r1 = -omega * (zeta - root);
            let r2 = -omega * (zeta + root);
            let c1 = (v0 - r2 * a) / (r1 - r2);
            let c2 = a - c1;
            let (e1, e2) = ((r1 * t).exp(), (r2 * t).exp());
            (target + c1 * e1 + c2 * e2, c1 * r1 * e1 + c2 * r2 * e2)
        }
    }

    /// Seconds until a unit step (`0 → 1` from rest) stays within `epsilon` of
    /// the target in position and within `10·epsilon` in velocity. Numeric and
    /// regime-agnostic; capped at 120 s.
    pub fn settling_duration(self, epsilon: f32) -> f32 {
        assert!(epsilon.is_finite() && epsilon > 0.0, "epsilon must be positive");
        let vel_eps = epsilon * 10.0;
        let settled = |t: f32| {
            let (x, v) = self.sample(0.0, 0.0, 1.0, t);
            (x - 1.0).abs() <= epsilon && v.abs() <= vel_eps
        };
        if settled(0.0) {
            return 0.0;
        }
        let mut hi = self.duration.max(1e-3);
        while !settled(hi) {
            hi *= 2.0;
            if hi > MAX_SETTLE {
                return MAX_SETTLE;
            }
        }
        let mut lo = 0.0;
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            if settled(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        hi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_is_exact_at_start() {
        let s = Spring::new(0.3, 0.25);
        assert_eq!(s.sample(10.0, 2.0, 0.0, 0.0), (10.0, 2.0));
        assert_eq!(s.sample(10.0, 2.0, 0.0, -1.0), (10.0, 2.0));
    }

    #[test]
    fn critical_does_not_overshoot() {
        let s = Spring::new(0.4, 0.0);
        let mut prev = 0.0;
        for i in 0..=200 {
            let x = s.sample(0.0, 0.0, 1.0, i as f32 * 0.01).0;
            assert!(x >= prev - 1e-4 && x <= 1.0 + 1e-4, "x={x}");
            prev = x;
        }
    }

    #[test]
    fn underdamped_overshoots_then_settles() {
        let s = Spring::new(0.4, 0.4);
        let peak = (0..400)
            .map(|i| s.sample(0.0, 0.0, 1.0, i as f32 * 0.005).0)
            .fold(f32::MIN, f32::max);
        assert!(peak > 1.05, "peak={peak}");
        assert!((s.sample(0.0, 0.0, 1.0, 8.0).0 - 1.0).abs() < 1e-3);
    }

    #[test]
    fn settling_duration_bounds_the_curve() {
        for (d, b) in [(0.2, 0.0), (0.3, 0.3), (0.5, -0.4)] {
            let s = Spring::new(d, b);
            let eps = 1e-3;
            let t = s.settling_duration(eps);
            let (x, v) = s.sample(0.0, 0.0, 1.0, t);
            assert!((x - 1.0).abs() <= eps && v.abs() <= eps * 10.0, "t={t} x={x} v={v}");
        }
    }
}
