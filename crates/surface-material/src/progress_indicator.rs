//! Material 3 progress semantics shared by linear and circular indicators.

use surface::{Node, RangeValue, Role, Style};
use surface_app::View;

mod circular;
mod linear;

/// The application's current progress.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProgressState {
    /// The fraction completed. Values outside 0–1 are clamped.
    /// Non-finite values are treated as zero.
    Determinate(f64),
    /// Work is in progress with no known fraction completed.
    Indeterminate,
}

/// The appearance of the active indicator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProgressShape {
    #[default]
    Flat,
    Wavy,
}

/// The label, value and animation time for a progress indicator.
///
/// Keep the ID stable while progress changes. The indicator is read-only.
/// It exposes a numeric range only when the fraction is known.
#[derive(Clone, Copy, Debug)]
pub struct ProgressIndicatorOptions<'a> {
    pub id: &'a str,
    /// Localized name of the operation.
    pub name: &'a str,
    pub state: ProgressState,
    pub shape: ProgressShape,
    /// Time since the animation started, in milliseconds. A fixed time
    /// gives a fixed frame. Pass zero before the first timed frame.
    pub elapsed_ms: f64,
    /// Use the static frame when the user prefers reduced motion.
    pub reduced_motion: bool,
}

impl ProgressIndicatorOptions<'_> {
    fn value(self) -> Option<f64> {
        match self.state {
            ProgressState::Determinate(value) if value.is_finite() => Some(value.clamp(0., 1.)),
            ProgressState::Determinate(_) => Some(0.),
            ProgressState::Indeterminate => None,
        }
    }

    fn fraction(self) -> Option<f32> {
        self.value().map(|value| value as f32)
    }

    fn elapsed(self) -> f64 {
        if self.reduced_motion || !self.elapsed_ms.is_finite() {
            0.
        } else {
            self.elapsed_ms.max(0.)
        }
    }

    fn view<M>(self, style: Style) -> View<M> {
        let mut node = Node::new(Role::Progress, self.name)
            .id(self.id)
            .styled(style);
        if let Some(value) = self.value() {
            node = node.range(RangeValue {
                min: 0.,
                max: 1.,
                value,
                step: 0.01,
            });
        }
        View::new(node)
    }
}
