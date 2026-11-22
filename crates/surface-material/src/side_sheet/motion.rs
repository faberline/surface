use surface_app::{AnimationUpdate, HorizontalDrag, HorizontalDragSession};

use super::SideSheetEdge;
use crate::{motion::easing, shape::LayoutDirection};

// At Android Material Components 66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad,
// the side sheet animation resources use emphasized easing and 275 ms.
// SideSheetBehavior and its edge delegates use 500 px/s and the midpoint
// between the expanded and hidden anchors.
const SETTLE_MS: f64 = 275.;
const FLING_SPEED: f32 = 500.;

#[derive(Clone, Copy)]
struct Drag {
    travel: f32,
    closing_sign: f32,
}

impl Drag {
    fn new(travel: f32, edge: SideSheetEdge, direction: LayoutDirection) -> Option<Self> {
        if !travel.is_finite() || travel <= 0. {
            return None;
        }
        let closing_sign = match (edge, direction) {
            (SideSheetEdge::Start, LayoutDirection::LeftToRight)
            | (SideSheetEdge::End, LayoutDirection::RightToLeft) => -1.,
            (SideSheetEdge::End, LayoutDirection::LeftToRight)
            | (SideSheetEdge::Start, LayoutDirection::RightToLeft) => 1.,
        };
        Some(Self {
            travel,
            closing_sign,
        })
    }

    fn reveal(self, delta_x: f32) -> f32 {
        (1. - self.closing_sign * delta_x / self.travel).clamp(0., 1.)
    }

    fn dismiss(self, reveal: f32, velocity_x: f32) -> bool {
        let closing_velocity = if velocity_x.is_finite() {
            self.closing_sign * velocity_x
        } else {
            0.
        };
        // The host already claims a mostly horizontal drag. A release toward
        // the open anchor takes priority over distance.
        closing_velocity >= 0. && (closing_velocity > FLING_SPEED || reveal < 0.5)
    }
}

struct Settle {
    from: f32,
    target: f32,
    dismiss_on_finish: bool,
    start_ms: Option<f64>,
}

/// Animation and horizontal dismissal state for one side sheet.
///
/// The application owns whether the sheet is open. Call [Self::set_open]
/// when that flag changes. Feed the gesture messages from the sheet builder
/// to [Self::apply]. Call [Self::advance] on the host animation frame and
/// apply its returned open flag when present.
///
/// When the viewport or sheet configuration changes during a drag, create
/// new state from the application's current flag. This discards the old
/// travel and releases its pointer ownership.
pub struct SideSheetMotion {
    open: bool,
    reveal: f32,
    session: HorizontalDragSession,
    drag: Option<Drag>,
    settle: Option<Settle>,
}

impl SideSheetMotion {
    /// Start at the current open or closed anchor.
    pub fn new(open: bool) -> Self {
        Self {
            open,
            reveal: endpoint(open),
            session: HorizontalDragSession::new(),
            drag: None,
            settle: None,
        }
    }

    /// Animate an application change from the current displayed position.
    pub fn set_open(&mut self, open: bool) {
        if self.open != open {
            self.open = open;
            self.settle_to(endpoint(open), false);
        }
    }

    /// The visible fraction of the sheet, from zero to one.
    pub fn reveal(&self) -> f32 {
        self.reveal
    }

    /// Whether the sheet is animating toward an anchor.
    pub fn is_settling(&self) -> bool {
        self.settle.is_some()
    }

    /// Whether a held gesture controls the reveal fraction.
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub(super) fn session(&self) -> HorizontalDragSession {
        self.session.clone()
    }

    /// Apply a physical horizontal drag on a fully open sheet.
    ///
    /// Pass the signed event from the runtime and the positive distance from
    /// the open sheet position to the hidden position in logical pixels. The
    /// first event captures this distance and the physical edge.
    pub fn apply(
        &mut self,
        event: HorizontalDrag,
        travel: f32,
        edge: SideSheetEdge,
        direction: LayoutDirection,
    ) {
        if !self.open || self.is_settling() {
            return;
        }
        let Some(drag) = self.drag.or_else(|| Drag::new(travel, edge, direction)) else {
            return;
        };
        match event {
            HorizontalDrag::Move { delta_x } if delta_x.is_finite() => {
                self.drag = Some(drag);
                self.reveal = drag.reveal(delta_x);
            }
            HorizontalDrag::End {
                delta_x,
                velocity_x,
            } if delta_x.is_finite() => {
                self.reveal = drag.reveal(delta_x);
                let dismiss = drag.dismiss(self.reveal, velocity_x);
                self.settle_to(endpoint(!dismiss), dismiss);
            }
            HorizontalDrag::Cancel | HorizontalDrag::End { .. } if self.is_dragging() => {
                self.settle_to(1., false);
            }
            _ => {}
        }
    }

    /// Advance motion on the host animation clock.
    ///
    /// A completed swipe returns `Some(false)` once. Commit it to the
    /// application's open flag in the same update. Reduce Motion and missing
    /// or non-finite clocks settle at the target immediately.
    pub fn advance(
        &mut self,
        now_ms: Option<f64>,
        reduced_motion: bool,
    ) -> (AnimationUpdate, Option<bool>) {
        let Some(settle) = &mut self.settle else {
            return (AnimationUpdate::default(), None);
        };
        if let Some(now) = now_ms.filter(|now| now.is_finite() && !reduced_motion) {
            let start = *settle.start_ms.get_or_insert(now);
            let elapsed = (now - start).max(0.);
            if elapsed < SETTLE_MS && settle.from != settle.target {
                let eased = easing::EMPHASIZED.sample((elapsed / SETTLE_MS) as f32);
                let sampled = settle.from + (settle.target - settle.from) * eased;
                // A host clock that briefly steps backward cannot undo an
                // earlier displayed frame.
                let reveal = if settle.target > settle.from {
                    self.reveal.max(sampled)
                } else {
                    self.reveal.min(sampled)
                };
                let rebuild = self.reveal != reveal;
                self.reveal = reveal;
                return (
                    AnimationUpdate {
                        rebuild,
                        request_next: true,
                    },
                    None,
                );
            }
        }
        let dismiss = settle.dismiss_on_finish;
        self.reveal = settle.target;
        self.settle = None;
        if dismiss {
            self.open = false;
        }
        (
            AnimationUpdate {
                rebuild: true,
                request_next: false,
            },
            dismiss.then_some(false),
        )
    }

    fn settle_to(&mut self, target: f32, dismiss_on_finish: bool) {
        self.drag = None;
        self.session = HorizontalDragSession::new();
        self.settle = (self.reveal != target || dismiss_on_finish).then_some(Settle {
            from: self.reveal,
            target,
            dismiss_on_finish,
            start_ms: None,
        });
    }
}

fn endpoint(open: bool) -> f32 {
    if open {
        1.
    } else {
        0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LayoutDirection::{LeftToRight, RightToLeft};
    use SideSheetEdge::{End, Start};

    fn movement(delta_x: f32) -> HorizontalDrag {
        HorizontalDrag::Move { delta_x }
    }

    fn release(delta_x: f32, velocity_x: f32) -> HorizontalDrag {
        HorizontalDrag::End {
            delta_x,
            velocity_x,
        }
    }

    #[test]
    fn closing_direction_mirrors_both_logical_edges() {
        for (edge, direction, dx) in [
            (End, LeftToRight, 64.),
            (Start, LeftToRight, -64.),
            (End, RightToLeft, -64.),
            (Start, RightToLeft, 64.),
        ] {
            let mut sheet = SideSheetMotion::new(true);
            sheet.apply(movement(-dx), 256., edge, direction);
            assert_eq!(sheet.reveal(), 1.);
            sheet.apply(movement(dx), 256., edge, direction);
            assert_eq!(sheet.reveal(), 0.75);
        }
    }

    #[test]
    fn held_gesture_keeps_its_travel_edge_and_session() {
        let mut sheet = SideSheetMotion::new(true);
        let session = sheet.session();
        sheet.apply(movement(64.), 256., End, LeftToRight);
        sheet.set_open(true);
        assert!(sheet.is_dragging());
        assert_eq!(sheet.session(), session);
        sheet.apply(movement(128.), 64., Start, LeftToRight);
        assert_eq!(sheet.reveal(), 0.5);
        sheet.apply(HorizontalDrag::Cancel, f32::NAN, Start, RightToLeft);
        assert!(!sheet.is_dragging());
        assert!(sheet.is_settling());
        assert_ne!(sheet.session(), session);
        assert!(sheet.advance(Some(0.), false).0.request_next);
        assert!(sheet.advance(Some(100.), false).0.rebuild);
        assert!(sheet.reveal() > 0.5);
        assert_eq!(sheet.advance(Some(275.), false).1, None);
        assert_eq!(sheet.reveal(), 1.);
    }

    #[test]
    fn release_uses_midpoint_significant_speed_and_opening_direction() {
        for (distance, velocity, dismissal) in [
            (128., 0., None),
            (129., 0., Some(false)),
            (0., 500., None),
            (0., 501., Some(false)),
            (192., -15., None),
            (192., f32::NAN, Some(false)),
        ] {
            for (edge, sign) in [(End, 1.), (Start, -1.)] {
                let mut sheet = SideSheetMotion::new(true);
                sheet.apply(
                    release(sign * distance, sign * velocity),
                    256.,
                    edge,
                    LeftToRight,
                );
                assert_eq!(sheet.advance(None, false).1, dismissal);
                assert_eq!(sheet.reveal(), endpoint(dismissal.is_none()));
                assert_eq!(sheet.advance(None, false).1, None);
            }
        }
    }

    #[test]
    fn enter_exit_reverse_from_the_displayed_position_with_a_275_ms_clock() {
        let mut sheet = SideSheetMotion::new(false);
        sheet.set_open(true);
        assert_eq!(sheet.reveal(), 0.);
        assert!(sheet.advance(Some(100.), false).0.request_next);
        assert!(sheet.advance(Some(200.), false).0.rebuild);
        let entering = sheet.reveal();
        assert!(entering > 0. && entering < 1.);
        sheet.advance(Some(150.), false);
        assert_eq!(sheet.reveal(), entering);
        sheet.set_open(false);
        assert_eq!(sheet.reveal(), entering);
        assert!(sheet.advance(Some(200.), false).0.request_next);
        assert!(sheet.advance(Some(474.), false).0.request_next);
        assert_eq!(
            sheet.advance(Some(475.), false),
            (
                AnimationUpdate {
                    rebuild: true,
                    request_next: false,
                },
                None
            )
        );
        assert_eq!(sheet.reveal(), 0.);
    }

    #[test]
    fn swipe_dismissal_commits_once_at_the_hidden_anchor() {
        let mut sheet = SideSheetMotion::new(true);
        sheet.apply(release(176., 0.), 256., End, LeftToRight);
        assert!(sheet.is_settling());
        assert_eq!(sheet.advance(Some(100.), false).1, None);
        assert_eq!(sheet.advance(Some(374.), false).1, None);
        assert_eq!(sheet.advance(Some(375.), false).1, Some(false));
        assert_eq!(sheet.reveal(), 0.);
        sheet.set_open(false);
        assert!(!sheet.is_settling());
        assert_eq!(sheet.advance(Some(376.), false).1, None);
    }

    #[test]
    fn reduced_motion_or_missing_clock_finishes_without_a_followup_frame() {
        for (clock, reduced_motion) in [(None, false), (Some(f64::NAN), false), (Some(0.), true)] {
            let mut sheet = SideSheetMotion::new(false);
            sheet.set_open(true);
            assert!(!sheet.advance(clock, reduced_motion).0.request_next);
            assert_eq!(sheet.reveal(), 1.);
            sheet.apply(release(256., 0.), 256., End, LeftToRight);
            assert_eq!(sheet.advance(clock, reduced_motion).1, Some(false));
            assert_eq!(sheet.reveal(), 0.);
        }
    }

    #[test]
    fn ignored_gestures_and_application_close_never_emit_a_swipe_commit() {
        let mut sheet = SideSheetMotion::new(false);
        sheet.apply(release(192., 600.), 256., End, LeftToRight);
        assert_eq!(sheet.reveal(), 0.);
        sheet.set_open(true);
        sheet.apply(release(192., 600.), 256., End, LeftToRight);
        sheet.advance(None, false);
        sheet.apply(release(192., 600.), 0., End, LeftToRight);
        assert_eq!(sheet.reveal(), 1.);

        sheet.apply(release(192., 600.), 256., End, LeftToRight);
        let held_reveal = sheet.reveal();
        assert!(held_reveal < 1.);
        sheet.set_open(false);
        assert_eq!(sheet.reveal(), held_reveal);
        assert_eq!(sheet.advance(None, false).1, None);
        assert_eq!(sheet.reveal(), 0.);
    }
}
