//! Retained state for the Expressive Dial and its shared Input form.

use crate::{TimeField, TimeInputEffect, TimeInputEvent, TimeInputSession};
use surface_app::PointDragSession;

/// Which editor is visible inside the same dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialMode {
    Dial,
    Input,
}

/// A point or captured pointer preview resolved to a value on the dial face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialPoint {
    Move(Option<u8>),
    Tap(Option<u8>),
    End(Option<u8>),
    Cancel,
}

/// Events emitted by the Dial and its shared Input form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialEvent {
    Show(DialMode),
    SetDialFits(bool),
    SetField(TimeField),
    Choose {
        field: TimeField,
        value: u8,
    },
    /// Set a value with the keyboard while staying in the current field.
    Adjust {
        field: TimeField,
        value: u8,
    },
    Point {
        field: TimeField,
        phase: DialPoint,
    },
    Input(TimeInputEvent),
}

/// The temporary hand position while a captured pointer is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialPreview {
    pub field: TimeField,
    pub value: u8,
}

/// Expressive Dial and Input share the same caller-owned draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialState {
    input: TimeInputSession,
    mode: DialMode,
    field: TimeField,
    fits: bool,
    preview: Option<DialPreview>,
    pub(crate) point_session: PointDragSession,
}

impl DialState {
    pub fn new(input: TimeInputSession, mode: DialMode, dial_fits: bool) -> Self {
        Self {
            input,
            mode: if dial_fits { mode } else { DialMode::Input },
            field: TimeField::Hour,
            fits: dial_fits,
            preview: None,
            point_session: PointDragSession::new(),
        }
    }

    pub const fn input(&self) -> &TimeInputSession {
        &self.input
    }

    pub const fn mode(&self) -> DialMode {
        self.mode
    }

    pub const fn field(&self) -> TimeField {
        self.field
    }

    pub const fn dial_fits(&self) -> bool {
        self.fits
    }

    pub const fn preview(&self) -> Option<DialPreview> {
        self.preview
    }

    /// An unfit layout switches to Input once. When space returns the
    /// user can choose Dial again while the same input draft stays open.
    pub fn apply(&mut self, event: DialEvent) -> TimeInputEffect {
        match event {
            DialEvent::Show(mode) if mode == DialMode::Input || self.fits => {
                self.set_mode(mode);
            }
            DialEvent::SetDialFits(fits) => {
                self.fits = fits;
                if !fits {
                    self.set_mode(DialMode::Input);
                }
            }
            DialEvent::SetField(field) if self.mode == DialMode::Dial => {
                self.preview = None;
                self.set_field(field);
            }
            DialEvent::Choose { field, value } if self.on_active_dial(field) => {
                self.preview = None;
                self.choose(field, value);
            }
            DialEvent::Adjust { field, value } if self.on_active_dial(field) => {
                self.preview = None;
                self.input.write_picked_value(field, value);
            }
            DialEvent::Point { field, phase } if self.on_active_dial(field) => match phase {
                DialPoint::Move(Some(value)) if self.input.accepts_value(field, value) => {
                    self.preview = Some(DialPreview { field, value });
                }
                DialPoint::Tap(value) | DialPoint::End(value) => {
                    self.preview = None;
                    if let Some(value) = value {
                        self.choose(field, value);
                    }
                }
                DialPoint::Cancel => self.preview = None,
                DialPoint::Move(_) => {}
            },
            DialEvent::Input(event) => return self.input_event(event),
            _ => {}
        }
        TimeInputEffect::None
    }

    pub fn observe_input_focus(&mut self, focused: Option<TimeField>) -> bool {
        if self.mode == DialMode::Input {
            self.input.observe_input_focus(focused)
        } else {
            false
        }
    }

    /// Update the focused field when a Baseline Dial shows its Input mode.
    /// Use the returned flag to refresh the editor's focused fill and outline.
    pub fn observe_baseline_input_focus(&mut self, focused: Option<TimeField>) -> bool {
        self.mode == DialMode::Input && self.input.observe_baseline_input_focus(focused)
    }

    fn set_mode(&mut self, mode: DialMode) {
        if mode != self.mode {
            self.preview = None;
            self.point_session = PointDragSession::new();
            self.input.retire_input_focus();
            self.mode = mode;
        }
    }

    fn on_active_dial(&self, field: TimeField) -> bool {
        self.mode == DialMode::Dial && self.field == field
    }

    fn choose(&mut self, field: TimeField, value: u8) {
        if self.input.write_picked_value(field, value) && field == TimeField::Hour {
            self.set_field(TimeField::Minute);
        }
    }

    fn set_field(&mut self, field: TimeField) {
        if self.field != field {
            self.field = field;
            self.point_session = PointDragSession::new();
        }
    }

    fn input_event(&mut self, event: TimeInputEvent) -> TimeInputEffect {
        if matches!(
            event,
            TimeInputEvent::Edit(..) | TimeInputEvent::FinishEditing(_)
        ) && self.mode != DialMode::Input
        {
            return TimeInputEffect::None;
        }
        self.preview = None;
        self.input.apply(event)
    }
}
