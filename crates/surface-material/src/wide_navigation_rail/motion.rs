use surface_app::{AnimationUpdate, ToggleMotion, ToggleSpring};

/// Choose the Material motion tracks for a wide navigation rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WideNavigationRailMotionKind {
    /// Animate the inline rail between its collapsed and expanded widths.
    Inline,
    /// Animate the modal rail width, standalone slide, and scrim.
    Modal,
}

/// The current visual fractions of a wide navigation rail.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WideNavigationRailMotionFrame {
    /// Width fraction from collapsed (0) to expanded (1).
    pub rail: f32,
    /// Standalone modal position from offscreen (0) to open (1).
    pub slide: f32,
    /// Modal scrim opacity as a fraction of its full opacity.
    pub scrim: f32,
}

struct ModalMotion {
    slide: ToggleMotion,
    scrim: ToggleMotion,
}

/// Drive wide navigation rail motion from the application's expanded state.
///
/// Keep one controller per rail and call [Self::advance] from
/// `Application::animation_frame`. Targets can change in either direction
/// while moving. Each track starts from its sampled position and velocity.
/// Keep painting the modal while the target is expanded, any fraction is
/// nonzero, or [Self::is_running] is true. Use the app's target to close the
/// focus scope. A modal with a retained inline rail uses the width and scrim
/// tracks. The slide track is for the standalone modal.
///
/// [AndroidX WideNavigationRail motion](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/WideNavigationRail.kt)
pub struct WideNavigationRailMotion {
    rail: ToggleMotion,
    modal: Option<ModalMotion>,
    frame: WideNavigationRailMotionFrame,
    running: bool,
}

impl WideNavigationRailMotion {
    /// Start settled at the application's initial endpoint.
    pub fn new(expanded: bool, kind: WideNavigationRailMotionKind) -> Self {
        let fraction = if expanded { 1. } else { 0. };
        let is_modal = kind == WideNavigationRailMotionKind::Modal;
        let effects = if is_modal { fraction } else { 0. };
        Self {
            rail: ToggleMotion::new(expanded),
            modal: is_modal.then(|| ModalMotion {
                slide: ToggleMotion::new(expanded),
                scrim: ToggleMotion::new(expanded),
            }),
            frame: WideNavigationRailMotionFrame {
                rail: fraction,
                slide: effects,
                scrim: effects,
            },
            running: false,
        }
    }

    /// Return the fractions last sampled. The slide and scrim are zero inline.
    pub fn frame(&self) -> WideNavigationRailMotionFrame {
        self.frame
    }

    /// Return whether any track still needs an animation frame.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Move directly to an endpoint and discard the current motion.
    pub fn snap(&mut self, expanded: bool) {
        let kind = if self.modal.is_some() {
            WideNavigationRailMotionKind::Modal
        } else {
            WideNavigationRailMotionKind::Inline
        };
        *self = Self::new(expanded, kind);
    }

    /// Sample all tracks at the host's current animation time in milliseconds.
    /// Pass a nondecreasing clock. Missing or non-finite times and Reduce Motion
    /// settle immediately.
    pub fn advance(
        &mut self,
        expanded: bool,
        now_ms: Option<f64>,
        reduced_motion: bool,
    ) -> AnimationUpdate {
        let rail_spring = if self.modal.is_some() {
            ToggleSpring::StandardFastSpatial
        } else {
            ToggleSpring::StandardDefaultSpatial
        };
        let (rail, rail_running) = self
            .rail
            .sample(expanded, now_ms, reduced_motion, rail_spring);
        let mut frame = WideNavigationRailMotionFrame {
            rail,
            ..WideNavigationRailMotionFrame::default()
        };
        let mut running = rail_running;
        if let Some(modal) = &mut self.modal {
            let (slide, slide_running) = modal.slide.sample(
                expanded,
                now_ms,
                reduced_motion,
                ToggleSpring::StandardDefaultSpatial,
            );
            let (scrim, scrim_running) = modal.scrim.sample(
                expanded,
                now_ms,
                reduced_motion,
                ToggleSpring::StandardDefaultEffects,
            );
            frame.slide = slide;
            frame.scrim = scrim;
            running |= slide_running || scrim_running;
        }
        // The caller may remove the closing layer once the motion settles,
        // even when the last frame already showed the clamped endpoint.
        let rebuild = self.frame != frame || self.running != running;
        self.frame = frame;
        self.running = running;
        AnimationUpdate {
            rebuild,
            request_next: running,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use WideNavigationRailMotionKind::{Inline, Modal};

    const EXPANDED_MODAL: WideNavigationRailMotionFrame = WideNavigationRailMotionFrame {
        rail: 1.,
        slide: 1.,
        scrim: 1.,
    };

    #[test]
    fn inline_uses_standard_default_spatial_and_settles_at_both_endpoints() {
        let mut motion = WideNavigationRailMotion::new(false, Inline);
        assert_eq!(motion.frame(), WideNavigationRailMotionFrame::default());
        assert_eq!(
            motion.advance(false, Some(0.), false),
            AnimationUpdate::default()
        );

        assert!(motion.advance(true, Some(100.), false).request_next);
        assert!(motion.is_running());
        assert_eq!(motion.frame(), WideNavigationRailMotionFrame::default());
        let mid = motion.advance(true, Some(140.), false);
        assert!(mid.rebuild && mid.request_next);
        assert!((motion.frame().rail - 0.29998).abs() < 0.0001);
        assert_eq!((motion.frame().slide, motion.frame().scrim), (0., 0.));
        let repeated = motion.advance(true, Some(140.), false);
        assert!(!repeated.rebuild && repeated.request_next);
        let end = motion.advance(true, Some(600.), false);
        assert!(end.rebuild && !end.request_next);
        assert_eq!(
            motion.frame(),
            WideNavigationRailMotionFrame {
                rail: 1.,
                slide: 0.,
                scrim: 0.
            }
        );

        assert!(motion.advance(false, Some(900.), false).request_next);
        assert!(motion.advance(false, Some(940.), false).rebuild);
        assert!((motion.frame().rail - 0.70002).abs() < 0.0001);
        assert!(!motion.advance(false, Some(1400.), false).request_next);
        assert_eq!(motion.frame(), WideNavigationRailMotionFrame::default());
    }

    #[test]
    fn modal_samples_width_slide_and_scrim_with_their_material_springs() {
        let mut motion = WideNavigationRailMotion::new(false, Modal);
        motion.advance(true, Some(0.), false);
        motion.advance(true, Some(40.), false);
        let frame = motion.frame();
        assert!((frame.rail - 0.46745).abs() < 0.0001);
        assert!((frame.slide - 0.29998).abs() < 0.0001);
        assert!((frame.scrim - 0.47507).abs() < 0.0001);
        assert!(motion.is_running());

        let end = motion.advance(true, Some(500.), false);
        assert!(end.rebuild && !end.request_next);
        assert_eq!(motion.frame(), EXPANDED_MODAL);
        assert_eq!(
            motion.advance(true, Some(520.), false),
            AnimationUpdate::default()
        );
    }

    #[test]
    fn reversal_reuses_the_current_position_and_velocity_of_each_track() {
        let mut reversal = WideNavigationRailMotion::new(false, Modal);
        let mut continuous = WideNavigationRailMotion::new(false, Modal);
        for motion in [&mut reversal, &mut continuous] {
            motion.advance(true, Some(0.), false);
            motion.advance(true, Some(30.), false);
        }
        continuous.advance(true, Some(60.), false);
        assert!(reversal.advance(false, Some(60.), false).request_next);
        let turn = continuous.frame();
        let reversed = reversal.frame();
        assert!((reversed.rail - turn.rail).abs() < f32::EPSILON);
        assert!((reversed.slide - turn.slide).abs() < f32::EPSILON);
        assert!((reversed.scrim - turn.scrim).abs() < f32::EPSILON);

        reversal.advance(false, Some(61.), false);
        let next = reversal.frame();
        assert!(next.rail > turn.rail && next.slide > turn.slide && next.scrim > turn.scrim);
        reversal.advance(false, Some(150.), false);
        let closing = reversal.frame();
        assert!(
            closing.rail < turn.rail && closing.slide < turn.slide && closing.scrim < turn.scrim
        );
        assert!(reversal.is_running());
        assert!(!reversal.advance(false, Some(550.), false).request_next);
        assert_eq!(reversal.frame(), WideNavigationRailMotionFrame::default());
    }

    #[test]
    fn reduce_motion_missing_clock_and_explicit_snap_finish_without_queued_frames() {
        let mut motion = WideNavigationRailMotion::new(true, Modal);
        assert_eq!(motion.frame(), EXPANDED_MODAL);
        motion.advance(false, Some(0.), false);
        motion.advance(false, Some(40.), false);

        let reduced = motion.advance(false, Some(40.), true);
        assert!(reduced.rebuild && !reduced.request_next);
        assert_eq!(motion.frame(), WideNavigationRailMotionFrame::default());
        assert!(!motion.is_running());
        assert!(!motion.advance(true, None, false).request_next);
        assert_eq!(motion.frame(), EXPANDED_MODAL);
        assert!(!motion.advance(false, Some(f64::NAN), false).request_next);
        assert_eq!(motion.frame(), WideNavigationRailMotionFrame::default());

        motion.advance(true, Some(100.), false);
        assert!(motion.is_running());
        motion.snap(true);
        assert_eq!(motion.frame(), EXPANDED_MODAL);
        assert!(!motion.is_running());
        assert_eq!(
            motion.advance(true, Some(101.), false),
            AnimationUpdate::default()
        );
    }
}
