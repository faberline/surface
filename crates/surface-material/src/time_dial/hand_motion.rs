//! Retained paint for the Material 3 dial selector.

use std::f64::consts::{PI, TAU};

use surface_app::{AnimationUpdate, SpatialMotion};

use crate::{DialEvent, DialMode, DialPoint, DialState, TimeInputEffect};

use super::polar::{HandPosition, INNER_RADIUS, OUTER_RADIUS};

struct MovingHand {
    angle: SpatialMotion,
    radius: SpatialMotion,
    goal: HandPosition,
}

impl MovingHand {
    fn new(origin: HandPosition) -> Self {
        Self {
            angle: SpatialMotion::new(origin.angle * OUTER_RADIUS),
            radius: SpatialMotion::new(origin.radius),
            goal: origin,
        }
    }

    fn advance(&mut self, goal: HandPosition, now_ms: f64) -> (HandPosition, bool) {
        if goal.angle != self.goal.angle {
            // Use arc length at the outer radius so both tracks settle to
            // the same distance tolerance. Keep angles unwrapped in flight.
            let current = self.angle.position(now_ms) / OUTER_RADIUS;
            let clockwise = (goal.angle - current).rem_euclid(TAU);
            let rotation = if clockwise > PI {
                clockwise - TAU
            } else {
                clockwise
            };
            self.angle.aim((current + rotation) * OUTER_RADIUS, now_ms);
        }
        self.radius.aim(goal.radius, now_ms);
        self.goal = goal;
        let (arc, moving_angle) = self.angle.advance(now_ms);
        let (radius, moving_radius) = self.radius.advance(now_ms);
        (
            HandPosition {
                angle: arc / OUTER_RADIUS,
                radius: radius.clamp(INNER_RADIUS, OUTER_RADIUS),
            },
            moving_angle || moving_radius,
        )
    }
}

/// Drive the painted selector of an open [DialState]. The draft and effects
/// stay independent of the animation clock.
pub struct DialHandMotion {
    shown: Option<HandPosition>,
    target: Option<HandPosition>,
    moving: Option<MovingHand>,
}

impl DialHandMotion {
    /// Start at the draft's current hand position. Keep one driver per draft.
    pub fn new(state: &DialState) -> Self {
        let position = hand_position(state);
        Self {
            shown: position,
            target: position,
            moving: None,
        }
    }

    /// Apply events to the draft and update the hand. Forward every dial
    /// event here, including layout changes. Keep the usual commit and
    /// dismissal handling for the returned effect.
    pub fn apply(&mut self, state: &mut DialState, event: DialEvent) -> TimeInputEffect {
        let snap = match &event {
            DialEvent::Point { field, phase }
                if state.mode() == DialMode::Dial && state.field() == *field =>
            {
                match phase {
                    DialPoint::Move(Some(value)) => state.input().accepts_value(*field, *value),
                    DialPoint::End(_) | DialPoint::Cancel => true,
                    _ => false,
                }
            }
            DialEvent::SetField(_) => state.preview().is_some(),
            _ => false,
        };
        let effect = state.apply(event);
        self.target = (effect == TimeInputEffect::None)
            .then(|| hand_position(state))
            .flatten();
        if snap || self.target.is_none() || self.shown.is_none() {
            self.snap();
        }
        effect
    }

    /// Call from the application frame and use both returned flags. Reduce
    /// Motion and frames without a finite clock settle to the target.
    pub fn advance(&mut self, now_ms: Option<f64>, reduced_motion: bool) -> AnimationUpdate {
        let previous = self.shown;
        let mut request_next = false;
        if let (Some(target), Some(shown), Some(now_ms)) = (
            self.target,
            self.shown,
            now_ms.filter(|time| time.is_finite() && !reduced_motion),
        ) {
            if self.moving.is_some() || target != shown {
                let moving = self.moving.get_or_insert_with(|| MovingHand::new(shown));
                let (position, in_motion) = moving.advance(target, now_ms);
                if in_motion {
                    self.shown = Some(position);
                    request_next = true;
                } else {
                    self.snap();
                }
            }
        } else {
            self.snap();
        }
        AnimationUpdate {
            rebuild: self.shown != previous,
            request_next,
        }
    }

    /// The painted center in logical pixels from the top left of the face.
    pub fn center(&self) -> Option<(f32, f32)> {
        self.shown.map(HandPosition::center)
    }

    pub(super) fn is_settled(&self) -> bool {
        self.shown == self.target && self.moving.is_none()
    }

    fn snap(&mut self) {
        self.shown = self.target;
        self.moving = None;
    }
}

fn hand_position(state: &DialState) -> Option<HandPosition> {
    if state.mode() != DialMode::Dial {
        return None;
    }
    let field = state.field();
    let input = state.input();
    let value = state
        .preview()
        .map(|preview| preview.value)
        .or_else(|| input.validation().field(field).ok())?;
    Some(HandPosition::for_value(field, input.clock(), value))
}
