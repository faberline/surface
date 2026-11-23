//! Local coordinates on the proposed 256 dp Expressive dial face.
use std::f64::consts::TAU;

use crate::{ClockFormat, TimeField};

pub const FACE_RADIUS: f64 = 128.;
pub const INNER_RADIUS: f64 = 64.;
pub const OUTER_RADIUS: f64 = 100.;
const RING_DIVIDER: f64 = (INNER_RADIUS + OUTER_RADIUS) / 2.;
const DEAD_ZONE: f64 = 24.;

pub fn hit(field: TimeField, clock: ClockFormat, x: f64, y: f64) -> Option<u8> {
    let dx = x - FACE_RADIUS;
    let dy = y - FACE_RADIUS;
    let radius = dx.hypot(dy);
    if !(DEAD_ZONE..=FACE_RADIUS).contains(&radius) {
        return None;
    }
    let count = if field == TimeField::Hour { 12 } else { 60 };
    let turns = (dx.atan2(-dy) / TAU).rem_euclid(1.);
    let slot = ((turns * f64::from(count)).round() as u8) % count;
    Some(match (field, clock) {
        (TimeField::Minute, _) => slot,
        (TimeField::Hour, ClockFormat::TwentyFourHour) if radius < RING_DIVIDER => slot + 12,
        (TimeField::Hour, ClockFormat::TwentyFourHour) => slot,
        (TimeField::Hour, ClockFormat::TwelveHour) if slot == 0 => 12,
        (TimeField::Hour, ClockFormat::TwelveHour) => slot,
    })
}

/// Reject a contact that began outside the usable circle, even if it later
/// reaches a valid slot inside the captured rectangle.
pub fn captured_hit(
    field: TimeField,
    clock: ClockFormat,
    origin: (f64, f64),
    position: (f64, f64),
) -> Option<u8> {
    hit(field, clock, origin.0, origin.1)?;
    hit(field, clock, position.0, position.1)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct HandPosition {
    pub angle: f64,
    pub radius: f64,
}

impl HandPosition {
    pub fn for_value(field: TimeField, clock: ClockFormat, value: u8) -> Self {
        let (slots, radius) = match (field, clock) {
            (TimeField::Minute, _) => (60, OUTER_RADIUS),
            (TimeField::Hour, ClockFormat::TwentyFourHour) if value >= 12 => (12, INNER_RADIUS),
            (TimeField::Hour, _) => (12, OUTER_RADIUS),
        };
        Self {
            angle: TAU * f64::from(value % slots) / f64::from(slots),
            radius,
        }
    }

    pub fn center(self) -> (f32, f32) {
        let (sin, cos) = self.angle.sin_cos();
        (
            (FACE_RADIUS + self.radius * sin) as f32,
            (FACE_RADIUS - self.radius * cos) as f32,
        )
    }
}

pub fn center(field: TimeField, clock: ClockFormat, value: u8) -> (f32, f32) {
    HandPosition::for_value(field, clock, value).center()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ClockFormat::{TwelveHour, TwentyFourHour};
    use TimeField::{Hour, Minute};

    fn point(slot: f64, slots: f64, radius: f64) -> (f64, f64) {
        let (sin, cos) = (slot / slots * TAU).sin_cos();
        (FACE_RADIUS + radius * sin, FACE_RADIUS - radius * cos)
    }

    #[test]
    fn all_painted_hour_and_minute_positions_resolve_to_their_own_values() {
        for (field, clock, values) in [
            (Hour, TwelveHour, 1..13),
            (Hour, TwentyFourHour, 0..24),
            (Minute, TwentyFourHour, 0..60),
        ] {
            for value in values {
                let (x, y) = center(field, clock, value);
                assert_eq!(hit(field, clock, f64::from(x), f64::from(y)), Some(value));
            }
        }
    }

    #[test]
    fn minute_half_step_and_hour_ring_split_are_unambiguous() {
        for (offset, slot) in [(-0.51, 59), (-0.49, 0), (0.49, 0), (0.51, 1)] {
            let (x, y) = point(offset, 60., OUTER_RADIUS);
            assert_eq!(hit(Minute, TwelveHour, x, y), Some(slot));
        }
        for (radius, expected) in [(81.9, 12), (82., 0), (82.1, 0)] {
            let (x, y) = point(0., 12., radius);
            assert_eq!(hit(Hour, TwentyFourHour, x, y), Some(expected));
        }
    }

    #[test]
    fn capture_preserves_rejection_of_a_contact_outside_the_usable_circle() {
        let target = point(6., 12., OUTER_RADIUS);
        for origin in [(0., 0.), (128., 128.), (f64::NAN, 0.)] {
            assert_eq!(captured_hit(Hour, TwelveHour, origin, target), None);
        }
        let origin = point(3., 12., OUTER_RADIUS);
        assert_eq!(captured_hit(Hour, TwelveHour, origin, target), Some(6));
    }
}
