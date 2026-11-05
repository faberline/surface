//! Transition timing values. They hold no clock and no platform handle.
use serde::{Deserialize, Serialize};

/// Maps linear progress in `[0, 1]` to eased progress.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Easing {
    #[default]
    Linear,
    /// `(x1, y1, x2, y2)`, like CSS `cubic-bezier()`.
    CubicBezier(f32, f32, f32, f32),
}

impl Easing {
    /// Eased progress for linear progress `t`.
    ///
    /// A `NaN` input counts as 0. The input is clamped to `[0, 1]`, and
    /// `sample(0)` and `sample(1)` return exactly 0 and 1. An easing that is
    /// not valid samples as linear, so the result is never `NaN`.
    pub fn sample(&self, t: f32) -> f32 {
        let t = if t.is_nan() { 0. } else { t.clamp(0., 1.) };
        match *self {
            Self::Linear => t,
            Self::CubicBezier(..) if !self.is_valid() => t,
            Self::CubicBezier(..) if t == 0. || t == 1. => t,
            Self::CubicBezier(x1, y1, x2, y2) => {
                let x = Curve::new(f64::from(x1), f64::from(x2));
                let y = Curve::new(f64::from(y1), f64::from(y2));
                y.at(x.solve(f64::from(t))) as f32
            }
        }
    }

    /// True when every parameter is finite and `x1` and `x2` are in `[0, 1]`.
    ///
    /// Only the x axis maps time, so the y axis may overshoot.
    pub fn is_valid(&self) -> bool {
        match *self {
            Self::Linear => true,
            Self::CubicBezier(x1, y1, x2, y2) => {
                [x1, y1, x2, y2].into_iter().all(f32::is_finite)
                    && (0. ..=1.).contains(&x1)
                    && (0. ..=1.).contains(&x2)
            }
        }
    }
}

/// One cubic Bezier axis with end points 0 and 1.
struct Curve {
    a: f64,
    b: f64,
    c: f64,
}

impl Curve {
    fn new(p1: f64, p2: f64) -> Self {
        let c = 3. * p1;
        let b = 3. * (p2 - p1) - c;
        Self {
            a: 1. - c - b,
            b,
            c,
        }
    }
    fn at(&self, s: f64) -> f64 {
        ((self.a * s + self.b) * s + self.c) * s
    }
    fn slope(&self, s: f64) -> f64 {
        (3. * self.a * s + 2. * self.b) * s + self.c
    }
    /// The parameter `s` in `[0, 1]` where this axis equals `t`.
    /// Newton's method first, then bisection when it does not converge.
    fn solve(&self, t: f64) -> f64 {
        const EPSILON: f64 = 1e-9;
        let mut s = t;
        for _ in 0..8 {
            let error = self.at(s) - t;
            if error.abs() < EPSILON {
                return s;
            }
            let slope = self.slope(s);
            if slope.abs() < 1e-6 {
                break;
            }
            s = (s - error / slope).clamp(0., 1.);
        }
        let (mut low, mut high) = (0., 1.);
        s = t;
        for _ in 0..64 {
            let error = self.at(s) - t;
            if error.abs() < EPSILON {
                break;
            }
            if error > 0. {
                high = s;
            } else {
                low = s;
            }
            s = (low + high) / 2.;
        }
        s
    }
}

/// How long a style change takes to show, and how it eases.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub duration_ms: f32,
    pub easing: Easing,
}

impl Transition {
    pub const fn new(duration_ms: f32, easing: Easing) -> Self {
        Self {
            duration_ms,
            easing,
        }
    }

    /// True when the duration is finite and not negative, and the easing is valid.
    pub fn is_valid(&self) -> bool {
        self.duration_ms.is_finite() && self.duration_ms >= 0. && self.easing.is_valid()
    }
}
