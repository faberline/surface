//! Decide how a claimed horizontal drag moves a modal date calendar.

use crate::{shape::LayoutDirection, DateBounds, StepDirection, YearMonth};

const FLING_DISTANCE: f32 = 24.;
const FLING_SPEED: f32 = 500.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SwipeViewport {
    pub width: f32,
    pub offset_x: f32,
    pub max_offset_x: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum SwipeInput {
    /// Signed physical displacement from the original contact point.
    Move {
        delta_x: f32,
    },
    End {
        delta_x: f32,
        velocity_x: f32,
    },
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum SwipeDecision {
    None,
    PanTo(f32),
    Preview {
        month: YearMonth,
        translation_x: f32,
        sign: f32,
    },
    Restore,
    Browse {
        from: YearMonth,
        to: YearMonth,
        direction: StepDirection,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Owner {
    Pan,
    Page {
        month: YearMonth,
        direction: StepDirection,
        sign: f32,
    },
    Blocked,
}

/// One gesture from the first accepted horizontal move through release.
/// Start a new value for the next contact. The Runtime owns the drag slop.
pub(super) struct SwipeGesture {
    viewport: SwipeViewport,
    direction: LayoutDirection,
    month: YearMonth,
    bounds: DateBounds,
    preview: bool,
    owner: Option<Owner>,
    finished: bool,
}

impl SwipeGesture {
    pub(super) fn new(
        mut viewport: SwipeViewport,
        direction: LayoutDirection,
        month: YearMonth,
        bounds: DateBounds,
        preview: bool,
    ) -> Option<Self> {
        if !viewport.width.is_finite()
            || viewport.width <= 0.
            || !viewport.offset_x.is_finite()
            || !viewport.max_offset_x.is_finite()
            || viewport.max_offset_x < 0.
            || !bounds.contains_month(month)
        {
            return None;
        }
        viewport.offset_x = viewport.offset_x.clamp(0., viewport.max_offset_x);
        Some(Self {
            viewport,
            direction,
            month,
            bounds,
            preview,
            owner: None,
            finished: false,
        })
    }

    pub(super) fn apply(&mut self, input: SwipeInput) -> SwipeDecision {
        if self.finished {
            return SwipeDecision::None;
        }
        let (delta_x, velocity) = match input {
            SwipeInput::Move { delta_x } => (delta_x, None),
            SwipeInput::End {
                delta_x,
                velocity_x,
            } => (delta_x, Some(velocity_x)),
            SwipeInput::Cancel => return self.cancel(),
        };
        if !delta_x.is_finite() {
            return self.cancel();
        }
        if velocity.is_some() {
            self.finished = true;
        }
        let owner = match self.owner {
            Some(owner) => owner,
            None if delta_x == 0. => return SwipeDecision::None,
            None => {
                let owner = self.owner_for(delta_x);
                self.owner = Some(owner);
                owner
            }
        };

        match owner {
            Owner::Pan => SwipeDecision::PanTo(
                (self.viewport.offset_x - delta_x).clamp(0., self.viewport.max_offset_x),
            ),
            Owner::Page {
                month,
                direction,
                sign,
            } => {
                let distance = (delta_x * sign).max(0.);
                if let Some(velocity_x) = velocity {
                    let far_enough = distance >= self.viewport.width / 3.;
                    let fast_enough = velocity_x.is_finite()
                        && distance >= FLING_DISTANCE
                        && velocity_x * sign >= FLING_SPEED;
                    if far_enough || fast_enough {
                        SwipeDecision::Browse {
                            from: self.month,
                            to: month,
                            direction,
                        }
                    } else {
                        self.restore()
                    }
                } else if self.preview {
                    SwipeDecision::Preview {
                        month,
                        translation_x: distance.min(self.viewport.width) * sign,
                        sign,
                    }
                } else {
                    SwipeDecision::None
                }
            }
            Owner::Blocked => SwipeDecision::None,
        }
    }

    fn owner_for(&self, delta_x: f32) -> Owner {
        let can_pan = if delta_x < 0. {
            self.viewport.offset_x < self.viewport.max_offset_x
        } else {
            self.viewport.offset_x > 0.
        };
        if can_pan {
            return Owner::Pan;
        }

        let rtl = self.direction == LayoutDirection::RightToLeft;
        let (direction, step) = if (delta_x > 0.) == rtl {
            (StepDirection::Forward, 1)
        } else {
            (StepDirection::Backward, -1)
        };
        let Some(month) = self
            .month
            .step(step)
            .filter(|month| self.bounds.contains_month(*month))
        else {
            return Owner::Blocked;
        };
        Owner::Page {
            month,
            direction,
            sign: delta_x.signum(),
        }
    }

    fn cancel(&mut self) -> SwipeDecision {
        self.finished = true;
        if matches!(self.owner, Some(Owner::Page { .. })) {
            self.restore()
        } else {
            SwipeDecision::None
        }
    }

    fn restore(&self) -> SwipeDecision {
        if self.preview {
            SwipeDecision::Restore
        } else {
            SwipeDecision::None
        }
    }
}
