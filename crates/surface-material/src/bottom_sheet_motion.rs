use surface_app::{AnimationUpdate, VerticalDrag, VerticalDragSession, View};

use crate::{
    motion::{duration, easing},
    BottomSheetContent, BottomSheetHeights, BottomSheetOptions, BottomSheetPresentation,
    BottomSheetState, Theme,
};

const FLING_SPEED: f32 = 500.;

struct Settle {
    from: f32,
    to: f32,
    intent: Option<BottomSheetState>,
    start_ms: Option<f64>,
}

/// Tracks visible height and pointer motion for one bottom sheet.
///
/// The application owns the sheet state. Call [Self::set_state] when it
/// changes, pass handle drag events to [Self::apply], and commit the state
/// returned by [Self::advance] in the same update. Start again from the
/// committed state when the presentation or viewport heights change.
pub struct BottomSheetMotion {
    state: BottomSheetState,
    heights: BottomSheetHeights,
    presentation: BottomSheetPresentation,
    height: f32,
    session: VerticalDragSession,
    dragging: bool,
    settle: Option<Settle>,
}

impl BottomSheetMotion {
    pub fn new(
        state: BottomSheetState,
        heights: BottomSheetHeights,
        presentation: BottomSheetPresentation,
    ) -> Self {
        Self {
            state,
            heights,
            presentation,
            height: heights.height(state),
            session: VerticalDragSession::new(),
            dragging: false,
            settle: None,
        }
    }

    /// Animate to a new application state from the current visible height.
    pub fn set_state(&mut self, state: BottomSheetState) {
        if self.state != state {
            self.state = state;
            self.settle_to(state, None);
        }
    }

    pub fn visible_height(&self) -> f32 {
        self.height
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn is_settling(&self) -> bool {
        self.settle.is_some()
    }

    /// Track a vertical drag on the handle. Positive displacement means down.
    pub fn apply(&mut self, event: VerticalDrag) {
        if self.state == BottomSheetState::Closed || self.is_settling() {
            return;
        }
        match event {
            VerticalDrag::Move { delta_y } if delta_y.is_finite() => {
                self.dragging = true;
                self.height = self.preview_height(delta_y);
            }
            VerticalDrag::End {
                delta_y,
                velocity_y,
            } if delta_y.is_finite() => {
                self.height = self.preview_height(delta_y);
                let target = self.drag_target(velocity_y);
                self.settle_to(target, (target != self.state).then_some(target));
            }
            VerticalDrag::Cancel | VerticalDrag::End { .. } if self.dragging => {
                self.settle_to(self.state, None);
            }
            _ => {}
        }
    }

    /// Advance the animation. Reduce Motion settles on the next frame.
    pub fn advance(
        &mut self,
        now_ms: Option<f64>,
        reduced_motion: bool,
    ) -> (AnimationUpdate, Option<BottomSheetState>) {
        let Some(settle) = &mut self.settle else {
            return (AnimationUpdate::default(), None);
        };
        if let Some(now) = now_ms.filter(|time| time.is_finite() && !reduced_motion) {
            let start = *settle.start_ms.get_or_insert(now);
            let opening = settle.to > settle.from;
            // The MDC v1.14.0 bottom sheet uses medium4 for entering and
            // medium3 for exiting, with the emphasized easing curve.
            let duration = if opening {
                duration::MEDIUM4
            } else {
                duration::MEDIUM3
            };
            let elapsed = (now - start).max(0.);
            if elapsed < f64::from(duration) && settle.from != settle.to {
                let fraction = easing::EMPHASIZED.sample((elapsed / f64::from(duration)) as f32);
                let sample = settle.from + (settle.to - settle.from) * fraction;
                let height = if opening {
                    self.height.max(sample)
                } else {
                    self.height.min(sample)
                };
                let rebuild = self.height != height;
                self.height = height;
                return (
                    AnimationUpdate {
                        rebuild,
                        request_next: true,
                    },
                    None,
                );
            }
        }
        self.height = settle.to;
        let intent = settle.intent;
        self.settle = None;
        if let Some(state) = intent {
            self.state = state;
        }
        (
            AnimationUpdate {
                rebuild: true,
                request_next: false,
            },
            intent,
        )
    }

    fn preview_height(&self, delta_y: f32) -> f32 {
        use BottomSheetState::{Expanded, Peek};
        let minimum = if self.presentation == BottomSheetPresentation::Standard {
            self.heights.height(Peek)
        } else {
            0.
        };
        (self.heights.height(self.state) - delta_y).clamp(minimum, self.heights.height(Expanded))
    }

    fn drag_target(&self, velocity_y: f32) -> BottomSheetState {
        use BottomSheetState::{Closed, Expanded, Peek};
        let modal = self.presentation == BottomSheetPresentation::Modal;
        let peek = self.heights.height(Peek) < self.heights.height(Expanded);
        let anchors: &[BottomSheetState] = match (modal, peek) {
            (true, true) => &[Closed, Peek, Expanded],
            (true, false) => &[Closed, Expanded],
            (false, true) => &[Peek, Expanded],
            (false, false) => &[Expanded],
        };
        let velocity_y = if velocity_y.is_finite() {
            velocity_y
        } else {
            0.
        };
        let height = |state| self.heights.height(state);
        if velocity_y > FLING_SPEED {
            return anchors
                .iter()
                .rev()
                .copied()
                .find(|state| height(*state) <= self.height)
                .unwrap_or(anchors[0]);
        }
        if velocity_y < -FLING_SPEED {
            return anchors
                .iter()
                .copied()
                .find(|state| height(*state) >= self.height)
                .unwrap_or(Expanded);
        }
        anchors
            .iter()
            .copied()
            .min_by(|left, right| {
                (height(*left) - self.height)
                    .abs()
                    .total_cmp(&(height(*right) - self.height).abs())
                    .then_with(|| (*right == self.state).cmp(&(*left == self.state)))
            })
            .unwrap_or(self.state)
    }

    fn settle_to(&mut self, target: BottomSheetState, intent: Option<BottomSheetState>) {
        self.session = VerticalDragSession::new();
        self.dragging = false;
        let to = self.heights.height(target);
        self.settle = (self.height != to || intent.is_some()).then_some(Settle {
            from: self.height,
            to,
            intent,
            start_ms: None,
        });
    }
}

impl Theme {
    /// Show the caller's bottom sheet motion and bind vertical pointer drag
    /// to its handle. Keep the supplied heights fitted to the layout and
    /// shared with the motion state. The content scrolls independently.
    pub fn bottom_sheet_with_motion<M: Clone>(
        &self,
        options: BottomSheetOptions<'_>,
        state: BottomSheetState,
        motion: &BottomSheetMotion,
        content: BottomSheetContent<M>,
        on_drag: impl Fn(VerticalDrag) -> M + 'static,
    ) -> View<M> {
        self.bottom_sheet_at_height(options, state, motion.height, content, |handle| {
            if state != BottomSheetState::Closed && !motion.is_settling() {
                handle.on_vertical_drag(motion.session.clone(), on_drag)
            } else {
                handle
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use BottomSheetPresentation::{Modal, Standard};
    use BottomSheetState::{Closed, Expanded, Peek};

    fn motion(state: BottomSheetState, presentation: BottomSheetPresentation) -> BottomSheetMotion {
        BottomSheetMotion::new(state, BottomSheetHeights::new(200., 500.), presentation)
    }

    fn movement(delta_y: f32) -> VerticalDrag {
        VerticalDrag::Move { delta_y }
    }

    fn release(delta_y: f32, velocity_y: f32) -> VerticalDrag {
        VerticalDrag::End {
            delta_y,
            velocity_y,
        }
    }

    #[test]
    fn upward_drag_previews_then_commits_expansion() {
        let mut sheet = motion(Peek, Standard);
        sheet.apply(movement(-175.));
        assert_eq!(sheet.visible_height(), 375.);
        assert!(sheet.is_dragging());
        sheet.apply(release(-175., 0.));
        assert!(sheet.is_settling());
        assert!(!sheet.is_dragging());
        let (update, state) = sheet.advance(Some(10.), true);
        assert!(update.rebuild);
        assert!(!update.request_next);
        assert_eq!(state, Some(Expanded));
        assert_eq!(sheet.visible_height(), 500.);
        assert_eq!(sheet.advance(Some(20.), true).1, None);
    }

    #[test]
    fn standard_stops_at_peek_while_modal_can_dismiss() {
        for (presentation, expected_height, expected_state) in
            [(Standard, 200., Peek), (Modal, 0., Closed)]
        {
            let mut sheet = motion(Expanded, presentation);
            sheet.apply(movement(600.));
            assert_eq!(sheet.visible_height(), expected_height);
            sheet.apply(release(600., 20.));
            assert_eq!(sheet.advance(None, false).1, Some(expected_state));
        }
    }

    #[test]
    fn cancelled_drag_restores_the_initial_anchor() {
        let mut sheet = motion(Peek, Modal);
        let session = sheet.session.clone();
        sheet.apply(movement(65.));
        sheet.apply(VerticalDrag::Cancel);
        assert_ne!(sheet.session, session);
        assert_eq!(sheet.visible_height(), 135.);
        assert_eq!(sheet.advance(None, false).1, None);
        assert_eq!(sheet.visible_height(), 200.);
    }

    #[test]
    fn release_uses_distance_or_velocity_and_preserves_a_distance_tie() {
        for (delta, velocity, expected) in [
            (-150., 0., None),
            (-75., -600., Some(Expanded)),
            (25., 600., Some(Closed)),
            (25., f32::INFINITY, None),
        ] {
            let mut sheet = motion(Peek, Modal);
            sheet.apply(release(delta, velocity));
            assert_eq!(sheet.advance(None, false).1, expected);
        }
    }

    #[test]
    fn equal_heights_have_no_peek_snap() {
        let heights = BottomSheetHeights::new(500., 500.);
        let mut standard = BottomSheetMotion::new(Expanded, heights, Standard);
        let mut modal = BottomSheetMotion::new(Expanded, heights, Modal);
        standard.apply(release(300., 20.));
        assert_eq!(standard.visible_height(), 500.);
        assert_eq!(standard.advance(None, false).1, None);
        modal.apply(release(300., 20.));
        assert_eq!(modal.advance(None, false).1, Some(Closed));
    }

    #[test]
    fn interruption_uses_current_height_and_backward_clocks_do_not_rewind() {
        let mut sheet = motion(Closed, Modal);
        sheet.set_state(Expanded);
        sheet.advance(Some(10.), false);
        sheet.advance(Some(110.), false);
        let visible = sheet.visible_height();
        assert!(visible > 0. && visible < 500.);
        sheet.advance(Some(60.), false);
        assert_eq!(sheet.visible_height(), visible);
        sheet.set_state(Closed);
        sheet.advance(Some(110.), false);
        assert_eq!(sheet.visible_height(), visible);
        assert!(sheet.advance(Some(459.), false).0.request_next);
        assert!(!sheet.advance(Some(460.), false).0.request_next);
        assert_eq!(sheet.visible_height(), 0.);
    }
}
