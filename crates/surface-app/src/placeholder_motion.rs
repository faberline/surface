//! The retained Material placeholder opacity. The caller owns draw eligibility.

use crate::motion::ScalarSpring;

// AndroidX 26065e0db6b4c61d1779c89ad388ea75ce98235a, StandardMotionTokens.kt.
const FAST_EFFECTS: f32 = 3800.;
const SLOW_EFFECTS: f32 = 800.;

/// The current editor phase. Focus has priority over committed text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlaceholderPhase {
    Focused,
    UnfocusedEmpty,
    UnfocusedNotEmpty,
}

impl PlaceholderPhase {
    fn opacity(self, expands: bool) -> f64 {
        match self {
            Self::Focused => 1.,
            Self::UnfocusedEmpty if !expands => 1.,
            _ => 0.,
        }
    }

    // TextFieldImpl.kt, Transition<InputPhase>.placeholderOpacity, at the
    // pinned AndroidX revision. The preceding target phase supplies the
    // transition rule, including when it did not change the opacity target.
    fn stiffness_to(self, next: Self) -> f32 {
        match (self, next) {
            (Self::UnfocusedEmpty, Self::Focused)
            | (Self::UnfocusedNotEmpty, Self::UnfocusedEmpty) => SLOW_EFFECTS,
            _ => FAST_EFFECTS,
        }
    }
}

pub(crate) struct PlaceholderMotion {
    phase: PlaceholderPhase,
    target: f64,
    spring: Option<ScalarSpring>,
    last_time_ms: Option<f64>,
}

impl PlaceholderMotion {
    /// Start settled. An expanded label hides the unfocused empty hint.
    pub(crate) fn new(phase: PlaceholderPhase, expands: bool) -> Self {
        Self {
            phase,
            target: phase.opacity(expands),
            spring: None,
            last_time_ms: None,
        }
    }

    /// Return a clamped alpha and whether another frame is needed.
    ///
    /// Text, active composition and label overlap affect draw eligibility
    /// separately. They do not select a different alpha or restart its spring.
    /// A snap or missing, nonfinite, or backwards time settles immediately.
    pub(crate) fn sample(
        &mut self,
        phase: PlaceholderPhase,
        expands: bool,
        now_ms: Option<f64>,
        snap: bool,
    ) -> (f32, bool) {
        let stiffness = self.phase.stiffness_to(phase);
        self.phase = phase;
        let target = phase.opacity(expands);
        let Some(now_ms) = now_ms.filter(|time| {
            time.is_finite() && !snap && self.last_time_ms.is_none_or(|last| *time >= last)
        }) else {
            self.target = target;
            self.spring = None;
            self.last_time_ms = None;
            return (target as f32, false);
        };
        self.last_time_ms = Some(now_ms);

        if target != self.target {
            let [position, velocity] = self.pose(now_ms).unwrap_or([self.target, 0.]);
            self.spring = Some(ScalarSpring::new(
                stiffness,
                1.,
                [1. + (position - target), velocity],
                now_ms,
            ));
            self.target = target;
        }

        if let Some([position, _]) = self.pose(now_ms) {
            (position.clamp(0., 1.) as f32, true)
        } else {
            self.spring = None;
            (self.target as f32, false)
        }
    }

    fn pose(&self, now_ms: f64) -> Option<[f64; 2]> {
        let [position, velocity] = self.spring?.sample(now_ms)?;
        Some([self.target + (position - 1.), velocity])
    }
}

#[cfg(test)]
mod tests {
    use super::{PlaceholderMotion, PlaceholderPhase::*};

    fn near(actual: (f32, bool), expected: f32) {
        assert!(actual.1);
        assert!((actual.0 - expected).abs() < 1e-6, "{actual:?}");
    }

    #[test]
    fn expanding_label_uses_slow_focus_and_fast_blur() {
        let mut motion = PlaceholderMotion::new(UnfocusedEmpty, true);
        assert_eq!(motion.sample(Focused, true, Some(100.), false), (0., true));
        // Direct critically damped response, 1 - (1 + sqrt(k)*t)*exp(-sqrt(k)*t).
        near(
            motion.sample(Focused, true, Some(114.), false),
            0.060_477_268,
        );
        near(
            motion.sample(Focused, true, Some(144.), false),
            0.353_396_27,
        );
        assert_eq!(motion.sample(Focused, true, Some(800.), false), (1., false));
        assert_eq!(
            motion.sample(UnfocusedEmpty, true, Some(850.), false),
            (1., true)
        );
        near(
            motion.sample(UnfocusedEmpty, true, Some(894.), false),
            0.246_429_53,
        );
        assert_eq!(
            motion.sample(UnfocusedEmpty, true, Some(1500.), false),
            (0., false)
        );
    }

    #[test]
    fn above_label_reveals_an_unfocused_clear_slowly() {
        let mut motion = PlaceholderMotion::new(UnfocusedNotEmpty, false);
        assert_eq!(
            motion.sample(UnfocusedEmpty, false, Some(100.), false),
            (0., true)
        );
        near(
            motion.sample(UnfocusedEmpty, false, Some(144.), false),
            0.353_396_27,
        );
        assert_eq!(
            motion.sample(UnfocusedEmpty, false, Some(800.), false),
            (1., false)
        );
        // An above-label field keeps the same opacity across focus changes.
        assert_eq!(
            motion.sample(Focused, false, Some(810.), false),
            (1., false)
        );
        assert_eq!(
            motion.sample(UnfocusedEmpty, false, Some(820.), false),
            (1., false)
        );
    }

    #[test]
    fn same_target_phase_changes_count_and_do_not_restart_the_track() {
        let mut motion = PlaceholderMotion::new(UnfocusedEmpty, true);
        // Both unfocused phases target zero for the expanding-label field.
        assert_eq!(
            motion.sample(UnfocusedNotEmpty, true, Some(100.), false),
            (0., false)
        );
        assert_eq!(motion.sample(Focused, true, Some(110.), false), (0., true));
        // UnfocusedNotEmpty -> Focused uses Fast Effects.
        near(
            motion.sample(Focused, true, Some(154.), false),
            0.753_570_44,
        );
        motion.sample(Focused, true, Some(800.), false);
        assert_eq!(
            motion.sample(UnfocusedEmpty, true, Some(900.), false),
            (1., true)
        );
        near(
            motion.sample(UnfocusedNotEmpty, true, Some(914.), false),
            0.785_982_9,
        );
        near(
            motion.sample(UnfocusedEmpty, true, Some(944.), false),
            0.246_429_53,
        );
    }

    #[test]
    fn reversal_reuses_position_and_velocity() {
        let mut motion = PlaceholderMotion::new(UnfocusedEmpty, true);
        motion.sample(Focused, true, Some(100.), false);
        let original = motion.sample(Focused, true, Some(144.), false);
        let before = motion.pose(144.).unwrap();
        assert!(before[1] > 0.);
        assert_eq!(
            motion.sample(UnfocusedEmpty, true, Some(144.), false),
            original
        );
        let after = motion.pose(144.).unwrap();
        assert!((before[0] - after[0]).abs() < 1e-12);
        assert_eq!(before[1], after[1]);
        assert!(motion.sample(UnfocusedEmpty, true, Some(145.), false).0 > original.0);
        assert!(motion.sample(UnfocusedEmpty, true, Some(190.), false).0 < original.0);
    }

    #[test]
    fn layout_changes_update_the_same_unfocused_empty_target() {
        let mut motion = PlaceholderMotion::new(UnfocusedEmpty, false);
        assert_eq!(
            motion.sample(UnfocusedEmpty, true, Some(100.), false),
            (1., true)
        );
        near(
            motion.sample(UnfocusedEmpty, true, Some(144.), false),
            0.246_429_53,
        );
        assert_eq!(
            motion.sample(UnfocusedEmpty, false, Some(150.), true),
            (1., false)
        );
    }

    #[test]
    fn reduce_motion_and_unusable_clock_settle_the_exact_target() {
        let mut motion = PlaceholderMotion::new(UnfocusedEmpty, true);
        motion.sample(Focused, true, Some(100.), false);
        near(
            motion.sample(Focused, true, Some(144.), false),
            0.353_396_27,
        );
        assert_eq!(motion.sample(Focused, true, Some(144.), true), (1., false));
        assert_eq!(
            motion.sample(UnfocusedEmpty, true, None, false),
            (0., false)
        );
        assert_eq!(
            motion.sample(UnfocusedEmpty, false, Some(f64::NAN), false),
            (1., false)
        );
        assert_eq!(
            motion.sample(UnfocusedNotEmpty, false, Some(200.), false),
            (1., true)
        );
        assert_eq!(
            motion.sample(UnfocusedNotEmpty, false, Some(199.), false),
            (0., false)
        );
    }
}
