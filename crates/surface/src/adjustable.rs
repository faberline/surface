//! Opt-in semantics for a discrete, possibly unset, adjustable control.

use crate::RangeValue;
use serde::{Deserialize, Serialize};

/// The accepted value of a named adjustable control, independently of its
/// rendered preview. Unset is distinct from selecting the minimum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdjustableValue {
    pub min: f64,
    pub max: f64,
    pub step: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_text: Option<String>,
}

impl AdjustableValue {
    pub fn is_valid(&self) -> bool {
        self.range().is_valid()
    }

    /// Clamp and snap a finite absolute request to the authored range.
    /// Return None when the value is unchanged or the input is invalid.
    pub fn resolve(&self, proposed: f64) -> Option<f64> {
        if !proposed.is_finite() || !self.is_valid() {
            return None;
        }
        let value = self.range().snap(proposed);
        (self.value != Some(value) && value.is_finite()).then_some(value)
    }

    /// Return the next valid step. Either first step selects the minimum
    /// when the control has no accepted value.
    pub fn step_value(&self, increment: bool) -> Option<f64> {
        if !self.is_valid() {
            return None;
        }
        let value = if let Some(value) = self.value {
            let delta = if increment { self.step } else { -self.step };
            (value + delta).clamp(self.min, self.max)
        } else {
            self.min
        };
        self.resolve(value)
    }

    fn range(&self) -> RangeValue {
        RangeValue {
            min: self.min,
            max: self.max,
            step: self.step,
            value: self.value.unwrap_or(self.min),
        }
    }
}
