//! Retained draft and independent wheel motion for the Expressive Scroll picker.

use surface_app::{AnimationUpdate, PointDragSession};

use crate::{ClockFormat, TimeField, TimeInputEffect, TimeInputEvent, TimeInputSession};

const WHEEL_IDLE_MS: f64 = 110.;
const SNAP_MS: f64 = 120.;
const FIELDS: [TimeField; 2] = [TimeField::Hour, TimeField::Minute];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeScrollMode {
    Scroll,
    Input,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeScrollDrag {
    /// Signed logical pixels from the initial press. Positive is down.
    Move {
        delta_y: f32,
    },
    /// Accept the final position. A captured release may arrive without a move.
    End {
        delta_y: f32,
    },
    /// Accept a tapped, visible row of the same wheel.
    Tap {
        value: u8,
    },
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeScrollStep {
    Previous,
    Next,
    First,
    Last,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimeScrollEvent {
    Show(TimeScrollMode),
    /// Report a layout change. Retires old contacts even if the wheel still fits.
    SetScrollFits(bool),
    /// Move the active field with keyboard focus. Leaves each wheel's value
    /// and motion intact.
    Focus(TimeField),
    /// Accept a discrete value from an input or accessibility action.
    Pick {
        field: TimeField,
        value: u8,
    },
    Adjust {
        field: TimeField,
        step: TimeScrollStep,
    },
    Drag {
        field: TimeField,
        session: PointDragSession,
        sample: TimeScrollDrag,
        row_height: f32,
    },
    Wheel {
        field: TimeField,
        delta_y: f32,
        row_height: f32,
        /// When absent, measure the quiet period from the next frame.
        now_ms: Option<f64>,
    },
    Input(TimeInputEvent),
}

/// Presentation and accepted value for one wheel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeScrollColumn {
    pub minimum: u8,
    pub maximum: u8,
    /// The draft's value. An unresolved field has no selected value.
    pub selected: Option<u8>,
    /// Fractional rows from the minimum. Increases toward larger values.
    pub offset: f32,
    /// A held or scrolling preview. It does not change the accepted value.
    pub preview: Option<u8>,
    /// The wheel is autonomously moving to a resting position.
    pub settling: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Idle,
    Drag {
        origin: f32,
        row_height: f32,
    },
    Wheel {
        origin: f32,
        last_input_ms: Option<f64>,
        last_frame_ms: Option<f64>,
    },
    Snap {
        origin: f32,
        target: f32,
        started_ms: Option<f64>,
        last_ms: Option<f64>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Track {
    offset: f32,
    phase: Phase,
}

#[derive(Clone, Debug, PartialEq)]
struct Wheel {
    session: PointDragSession,
    track: Track,
}

impl Wheel {
    fn new(offset: f32) -> Self {
        Self {
            session: PointDragSession::new(),
            track: Track::rest(offset),
        }
    }

    fn retire_contact(&mut self) {
        self.session = PointDragSession::new();
    }
}

impl Track {
    fn rest(offset: f32) -> Self {
        Self {
            offset,
            phase: Phase::Idle,
        }
    }

    fn snap_to(&mut self, target: f32, reduced_motion: bool) {
        if reduced_motion || self.offset == target {
            *self = Self::rest(target);
        } else {
            self.phase = Phase::Snap {
                origin: self.offset,
                target,
                started_ms: None,
                last_ms: None,
            };
        }
    }
}

#[derive(Clone, Copy)]
struct Bounds {
    min: u8,
    max: u8,
}

impl Bounds {
    fn new(field: TimeField, clock: ClockFormat) -> Self {
        let (min, max) = match (field, clock) {
            (TimeField::Hour, ClockFormat::TwelveHour) => (1, 12),
            (TimeField::Hour, ClockFormat::TwentyFourHour) => (0, 23),
            (TimeField::Minute, _) => (0, 59),
        };
        Self { min, max }
    }

    fn clamp_offset(self, offset: f32) -> f32 {
        offset.clamp(0., f32::from(self.max - self.min))
    }

    fn closest(self, offset: f32) -> u8 {
        self.min + self.clamp_offset(offset).round() as u8
    }

    fn offset(self, value: u8) -> f32 {
        f32::from(value.clamp(self.min, self.max) - self.min)
    }
}

/// Controlled Scroll and Input views share one time session. Events accept
/// draft changes only. The caller consumes the Commit/Dismiss effect.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeScrollState {
    input: TimeInputSession,
    mode: TimeScrollMode,
    scroll_fits: bool,
    active_field: TimeField,
    hour: Wheel,
    minute: Wheel,
}

impl TimeScrollState {
    pub fn new(input: TimeInputSession, mode: TimeScrollMode, scroll_fits: bool) -> Self {
        let at = |field| {
            let bounds = Bounds::new(field, input.clock());
            Wheel::new(bounds.offset(input.validation().field(field).unwrap_or(bounds.min)))
        };
        let (hour, minute) = (at(TimeField::Hour), at(TimeField::Minute));
        Self {
            input,
            mode: if scroll_fits {
                mode
            } else {
                TimeScrollMode::Input
            },
            scroll_fits,
            active_field: TimeField::Hour,
            hour,
            minute,
        }
    }

    pub const fn input(&self) -> &TimeInputSession {
        &self.input
    }

    pub const fn mode(&self) -> TimeScrollMode {
        self.mode
    }

    pub const fn scroll_fits(&self) -> bool {
        self.scroll_fits
    }

    /// The last focused hour or minute control, shared across both modes.
    pub const fn active_field(&self) -> TimeField {
        self.active_field
    }

    pub fn column(&self, field: TimeField) -> TimeScrollColumn {
        let bounds = self.bounds(field);
        let track = self.track(field);
        TimeScrollColumn {
            minimum: bounds.min,
            maximum: bounds.max,
            offset: track.offset,
            selected: self.selected(field),
            preview: matches!(track.phase, Phase::Drag { .. } | Phase::Wheel { .. })
                .then(|| bounds.closest(track.offset)),
            settling: matches!(track.phase, Phase::Snap { .. }),
        }
    }

    pub(crate) fn drag_session(&self, field: TimeField) -> PointDragSession {
        self.wheel(field).session.clone()
    }

    pub fn observe_input_focus(&mut self, focused: Option<TimeField>) -> bool {
        if self.mode != TimeScrollMode::Input {
            return false;
        }
        let changed_field = self.observe_field(focused);
        self.input.observe_input_focus(focused) || changed_field
    }

    /// Remember the focused wheel. Toolbar or outside focus keeps the last field.
    pub fn observe_scroll_focus(&mut self, focused: Option<TimeField>) -> bool {
        self.mode == TimeScrollMode::Scroll && self.observe_field(focused)
    }

    fn observe_field(&mut self, focused: Option<TimeField>) -> bool {
        let Some(field) = focused.filter(|&field| field != self.active_field) else {
            return false;
        };
        self.active_field = field;
        true
    }

    pub fn apply(&mut self, event: TimeScrollEvent, reduced_motion: bool) -> TimeInputEffect {
        match event {
            TimeScrollEvent::Show(mode) if mode == TimeScrollMode::Input || self.scroll_fits => {
                self.set_mode(mode);
            }
            TimeScrollEvent::SetScrollFits(fits) => {
                self.scroll_fits = fits;
                if !fits && self.mode == TimeScrollMode::Scroll {
                    self.set_mode(TimeScrollMode::Input);
                } else {
                    self.reset_wheels();
                }
            }
            TimeScrollEvent::Input(event) => {
                return self.apply_input(event, reduced_motion);
            }
            event if self.mode == TimeScrollMode::Scroll => match event {
                TimeScrollEvent::Focus(field) => {
                    self.observe_field(Some(field));
                }
                TimeScrollEvent::Pick { field, value } => {
                    if self.pick(field, value, reduced_motion) {
                        self.wheel_mut(field).retire_contact();
                    }
                }
                TimeScrollEvent::Adjust { field, step } => {
                    self.adjust(field, step, reduced_motion);
                }
                TimeScrollEvent::Drag {
                    field,
                    session,
                    sample,
                    row_height,
                } => {
                    self.drag(field, session, sample, row_height, reduced_motion);
                }
                TimeScrollEvent::Wheel {
                    field,
                    delta_y,
                    row_height,
                    now_ms,
                } => {
                    self.scroll_wheel(field, delta_y, row_height, now_ms);
                }
                _ => {}
            },
            _ => {}
        }
        TimeInputEffect::None
    }

    pub fn advance(&mut self, now_ms: Option<f64>, reduced_motion: bool) -> AnimationUpdate {
        let now_ms = now_ms.filter(|now| now.is_finite());
        let mut update = AnimationUpdate::default();
        for field in FIELDS {
            let old = self.column(field);
            self.tick(field, now_ms, reduced_motion);
            update.rebuild |= self.column(field) != old;
            update.request_next |= matches!(
                self.track(field).phase,
                Phase::Wheel { .. } | Phase::Snap { .. }
            );
        }
        update
    }

    fn selected(&self, field: TimeField) -> Option<u8> {
        self.input.validation().field(field).ok()
    }

    fn bounds(&self, field: TimeField) -> Bounds {
        Bounds::new(field, self.input.clock())
    }

    fn wheel(&self, field: TimeField) -> &Wheel {
        match field {
            TimeField::Hour => &self.hour,
            TimeField::Minute => &self.minute,
        }
    }

    fn wheel_mut(&mut self, field: TimeField) -> &mut Wheel {
        match field {
            TimeField::Hour => &mut self.hour,
            TimeField::Minute => &mut self.minute,
        }
    }

    fn track(&self, field: TimeField) -> Track {
        self.wheel(field).track
    }

    fn track_mut(&mut self, field: TimeField) -> &mut Track {
        &mut self.wheel_mut(field).track
    }

    fn rest_offset(&self, field: TimeField) -> f32 {
        self.selected(field)
            .map_or(0., |value| self.bounds(field).offset(value))
    }

    fn set_mode(&mut self, mode: TimeScrollMode) {
        if mode != self.mode {
            self.input.retire_input_focus();
            self.mode = mode;
            self.reset_wheels();
        }
    }

    fn reset_wheels(&mut self) {
        self.hour = Wheel::new(self.rest_offset(TimeField::Hour));
        self.minute = Wheel::new(self.rest_offset(TimeField::Minute));
    }

    fn apply_input(&mut self, event: TimeInputEvent, reduced_motion: bool) -> TimeInputEffect {
        if self.mode == TimeScrollMode::Scroll {
            match event {
                TimeInputEvent::Edit(..) | TimeInputEvent::FinishEditing(_) => {
                    return TimeInputEffect::None
                }
                TimeInputEvent::Confirm => {
                    for field in FIELDS {
                        self.wheel_mut(field).retire_contact();
                        match self.track(field).phase {
                            // Finish a scroll burst before Save. A held pointer
                            // can leave without contributing its preview.
                            Phase::Wheel { origin, .. } => {
                                self.finish_preview(field, origin, false, reduced_motion);
                            }
                            Phase::Drag { .. } => self.return_to_selection(field, reduced_motion),
                            _ => {}
                        }
                    }
                }
                TimeInputEvent::Cancel => self.reset_wheels(),
                _ => {}
            }
        }
        self.input.apply(event)
    }

    fn pick(&mut self, field: TimeField, value: u8, reduced_motion: bool) -> bool {
        if !self.input.write_picked_value(field, value) {
            return false;
        }
        let target = self.bounds(field).offset(value);
        self.track_mut(field).snap_to(target, reduced_motion);
        true
    }

    fn adjust(&mut self, field: TimeField, step: TimeScrollStep, reduced_motion: bool) {
        self.wheel_mut(field).retire_contact();
        let bounds = self.bounds(field);
        let selected = self.selected(field);
        let start = selected.unwrap_or(bounds.min);
        let target = match step {
            TimeScrollStep::Previous => start.saturating_sub(1).max(bounds.min),
            TimeScrollStep::Next => start.saturating_add(1).min(bounds.max),
            TimeScrollStep::First => bounds.min,
            TimeScrollStep::Last => bounds.max,
        };
        if selected != Some(target) || !matches!(self.track(field).phase, Phase::Idle) {
            self.pick(field, target, reduced_motion);
        }
    }

    fn drag(
        &mut self,
        field: TimeField,
        session: PointDragSession,
        sample: TimeScrollDrag,
        row_height: f32,
        reduced_motion: bool,
    ) {
        if self.wheel(field).session != session {
            return;
        }
        if !matches!(sample, TimeScrollDrag::Move { .. }) {
            self.wheel_mut(field).retire_contact();
        }
        let track = self.track(field);
        let bounds = self.bounds(field);
        let baseline = match track.phase {
            Phase::Drag { origin, row_height } => Some((origin, row_height)),
            _ if row_height.is_finite() && row_height > 0. => Some((track.offset, row_height)),
            _ => None,
        };
        match sample {
            TimeScrollDrag::Move { delta_y } => {
                if !delta_y.is_finite() {
                    return;
                }
                let Some((origin, row_height)) = baseline else {
                    return;
                };
                *self.track_mut(field) = Track {
                    offset: bounds.clamp_offset(origin - delta_y / row_height),
                    phase: Phase::Drag { origin, row_height },
                };
            }
            TimeScrollDrag::End { delta_y } => {
                if let Some((origin, row_height)) = baseline.filter(|_| delta_y.is_finite()) {
                    self.track_mut(field).offset =
                        bounds.clamp_offset(origin - delta_y / row_height);
                    self.finish_preview(field, origin, false, reduced_motion);
                } else if matches!(track.phase, Phase::Drag { .. }) {
                    self.return_to_selection(field, reduced_motion);
                }
            }
            TimeScrollDrag::Tap { value } if !matches!(track.phase, Phase::Drag { .. }) => {
                self.pick(field, value, reduced_motion);
            }
            TimeScrollDrag::Tap { .. } => self.return_to_selection(field, reduced_motion),
            TimeScrollDrag::Cancel if matches!(track.phase, Phase::Drag { .. }) => {
                self.return_to_selection(field, reduced_motion);
            }
            TimeScrollDrag::Cancel => {}
        }
    }

    fn scroll_wheel(&mut self, field: TimeField, delta: f32, row_height: f32, now_ms: Option<f64>) {
        if !delta.is_finite() || delta == 0. || !row_height.is_finite() || row_height <= 0. {
            return;
        }
        let track = self.track(field);
        let (origin, last_frame_ms) = match track.phase {
            Phase::Drag { .. } => return,
            Phase::Wheel {
                origin,
                last_frame_ms,
                ..
            } => (origin, last_frame_ms),
            _ => (track.offset, None),
        };
        let offset = self
            .bounds(field)
            .clamp_offset(track.offset + delta / row_height);
        if offset == track.offset {
            return;
        }
        *self.track_mut(field) = Track {
            offset,
            phase: Phase::Wheel {
                origin,
                last_input_ms: now_ms.filter(|now| now.is_finite()),
                last_frame_ms,
            },
        };
    }

    fn finish_preview(
        &mut self,
        field: TimeField,
        origin: f32,
        no_clock: bool,
        reduced_motion: bool,
    ) {
        let bounds = self.bounds(field);
        let offset = self.track(field).offset;
        let target = bounds.closest(offset);
        if target != bounds.closest(origin) {
            self.pick(field, target, reduced_motion);
        } else if no_clock && (offset - origin).abs() > f32::EPSILON {
            let step = if offset > origin {
                TimeScrollStep::Next
            } else {
                TimeScrollStep::Previous
            };
            self.adjust(field, step, true);
            self.return_to_selection(field, true);
        } else {
            self.return_to_selection(field, reduced_motion);
        }
    }

    fn return_to_selection(&mut self, field: TimeField, reduced_motion: bool) {
        let target = self.rest_offset(field);
        self.track_mut(field).snap_to(target, reduced_motion);
    }

    fn tick(&mut self, field: TimeField, now: Option<f64>, reduced_motion: bool) {
        match self.track(field).phase {
            Phase::Wheel {
                origin,
                last_input_ms,
                last_frame_ms,
            } => {
                if let Some(now) = now {
                    let input = last_input_ms.unwrap_or(now);
                    if last_frame_ms.is_some_and(|last| now < last)
                        || now < input
                        || now - input >= WHEEL_IDLE_MS
                    {
                        self.finish_preview(field, origin, false, reduced_motion);
                    } else {
                        self.track_mut(field).phase = Phase::Wheel {
                            origin,
                            last_input_ms: Some(input),
                            last_frame_ms: Some(now),
                        };
                    }
                } else {
                    self.finish_preview(field, origin, true, true);
                }
            }
            Phase::Snap {
                origin,
                target,
                started_ms,
                last_ms,
            } => {
                let Some(now) =
                    now.filter(|now| !reduced_motion && last_ms.is_none_or(|last| *now >= last))
                else {
                    *self.track_mut(field) = Track::rest(target);
                    return;
                };
                let started = started_ms.unwrap_or(now);
                let fraction = ((now - started) / SNAP_MS) as f32;
                if fraction >= 1. {
                    *self.track_mut(field) = Track::rest(target);
                } else {
                    let ease = 1. - (1. - fraction).powi(3);
                    *self.track_mut(field) = Track {
                        offset: origin + (target - origin) * ease,
                        phase: Phase::Snap {
                            origin,
                            target,
                            started_ms: Some(started),
                            last_ms: Some(now),
                        },
                    };
                }
            }
            Phase::Idle | Phase::Drag { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests;
