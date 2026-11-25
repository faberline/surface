use std::num::NonZeroU16;

use surface::{RangeValue, Rect};

use crate::EditKey;

/// How a slider converts a position to a controlled value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderScale {
    /// Pointer and absolute adjustment keep the requested value. Keyboard
    /// and relative adjustment use the range's step.
    Continuous,
    /// The range contains this many equal intervals, including both endpoints.
    Discrete(NonZeroU16),
}

/// Optional pointer, key, and accessible adjustment rules for a horizontal
/// slider. Without this policy, a slider uses its original step behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SliderInputOptions {
    pub scale: SliderScale,
    /// The physical right edge is the minimum. Only Left and Right reverse.
    pub reverse_horizontal: bool,
}

impl SliderInputOptions {
    /// Clamp a finite request to the range and apply the authored stop grid.
    pub fn resolve_value(self, range: RangeValue, value: f64) -> Option<f64> {
        if !range.is_valid() || !value.is_finite() {
            return None;
        }
        let value = value.clamp(range.min, range.max);
        Some(match self.scale {
            SliderScale::Continuous => value,
            SliderScale::Discrete(intervals) => {
                stop_value(range, intervals, stop_index(range, intervals, value))
            }
        })
    }

    /// Position of the controlled value between the logical minimum and maximum.
    pub fn value_fraction(self, range: RangeValue) -> Option<f64> {
        if !range.is_valid() {
            return None;
        }
        Some(match self.scale {
            SliderScale::Continuous => (range.value - range.min) / (range.max - range.min),
            SliderScale::Discrete(intervals) => {
                stop_index(range, intervals, range.value) / f64::from(intervals.get())
            }
        })
    }

    pub(crate) fn relative(self, range: RangeValue, steps: f64) -> f64 {
        match self.scale {
            SliderScale::Continuous => {
                (range.value + steps * range.step).clamp(range.min, range.max)
            }
            SliderScale::Discrete(intervals) => {
                let index = stop_index(range, intervals, range.value);
                stop_value(range, intervals, index + steps)
            }
        }
    }

    pub(crate) fn key_value(self, range: RangeValue, key: EditKey) -> Option<f64> {
        let sign = if self.reverse_horizontal { -1. } else { 1. };
        Some(match key {
            EditKey::Left => self.relative(range, -sign),
            EditKey::Right => self.relative(range, sign),
            EditKey::Down => self.relative(range, -1.),
            EditKey::Up => self.relative(range, 1.),
            EditKey::Home => range.min,
            EditKey::End => range.max,
            EditKey::PageUp | EditKey::PageDown => {
                let sign = if key == EditKey::PageUp { 1. } else { -1. };
                match self.scale {
                    SliderScale::Continuous => {
                        let distance = ((range.max - range.min) / 10.).max(range.step);
                        (range.value + sign * distance).clamp(range.min, range.max)
                    }
                    SliderScale::Discrete(intervals) => {
                        self.relative(range, sign * (f64::from(intervals.get()) / 10.).ceil())
                    }
                }
            }
            _ => return None,
        })
    }

    pub(crate) fn pointer_value(self, range: RangeValue, rail: Rect, x: f32) -> f64 {
        let fraction = if x <= rail.x {
            0.
        } else if x >= rail.x + rail.width {
            1.
        } else {
            (f64::from(x) - f64::from(rail.x)) / f64::from(rail.width)
        };
        let fraction = if self.reverse_horizontal {
            1. - fraction
        } else {
            fraction
        };
        if fraction == 1. {
            range.max
        } else {
            range.min + fraction * (range.max - range.min)
        }
    }
}

fn stop_index(range: RangeValue, intervals: NonZeroU16, value: f64) -> f64 {
    let span = range.max - range.min;
    (((value - range.min) / span) * f64::from(intervals.get())).round()
}

fn stop_value(range: RangeValue, intervals: NonZeroU16, index: f64) -> f64 {
    let count = f64::from(intervals.get());
    if index <= 0. {
        range.min
    } else if index >= count {
        range.max
    } else {
        range.min + (range.max - range.min) * (index / count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discrete_arrows_and_page_keys_cross_a_sub_ulp_boundary_by_index() {
        let scale = SliderScale::Discrete(NonZeroU16::new(3).unwrap());
        let input = SliderInputOptions {
            scale,
            reverse_horizontal: false,
        };
        let eps = f64::EPSILON;
        let range = RangeValue {
            min: 1.,
            max: 1. + 4. * eps,
            value: 1. + 3. * eps,
            step: (4. * eps) / 3.,
        };
        assert_eq!(input.key_value(range, EditKey::Left), Some(1. + eps));
        assert_eq!(input.key_value(range, EditKey::PageDown), Some(1. + eps));
        assert_eq!(input.value_fraction(range), Some(2. / 3.));
    }

    #[test]
    fn pointer_uses_the_visible_f32_edge_as_the_exact_endpoint() {
        let input = SliderInputOptions {
            scale: SliderScale::Continuous,
            reverse_horizontal: false,
        };
        let range = RangeValue {
            min: 0.,
            max: 100.,
            value: 0.,
            step: 1.,
        };
        let rail = Rect::new(0.2, 0., 0.6, 48.);
        assert_eq!(
            input.pointer_value(range, rail, rail.x + rail.width),
            range.max
        );
    }
}
