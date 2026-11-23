//! Pure hour/minute input for an inline form or modal dialog.

use std::fmt;

const ASCII_DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

/// One wall-clock time at minute precision. No date or offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
}

impl TimeOfDay {
    pub const fn new(hour: u8, minute: u8) -> Option<Self> {
        if hour < 24 && minute < 60 {
            Some(Self { hour, minute })
        } else {
            None
        }
    }

    pub const fn from_twelve_hour(hour: u8, minute: u8, period: TimePeriod) -> Option<Self> {
        if hour < 1 || hour > 12 {
            return None;
        }
        let offset = match period {
            TimePeriod::Am => 0,
            TimePeriod::Pm => 12,
        };
        Self::new(hour % 12 + offset, minute)
    }

    pub const fn hour(self) -> u8 {
        self.hour
    }

    pub const fn minute(self) -> u8 {
        self.minute
    }

    pub const fn display_hour(self, clock: ClockFormat) -> u8 {
        match clock {
            ClockFormat::TwelveHour => (self.hour + 11) % 12 + 1,
            ClockFormat::TwentyFourHour => self.hour,
        }
    }

    pub const fn period(self) -> TimePeriod {
        if self.hour < 12 {
            TimePeriod::Am
        } else {
            TimePeriod::Pm
        }
    }
}

impl fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockFormat {
    TwelveHour,
    TwentyFourHour,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimePeriod {
    Am,
    Pm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeField {
    Hour,
    Minute,
}

impl TimeField {
    const ALL: [Self; 2] = [Self::Hour, Self::Minute];

    fn index(self) -> usize {
        match self {
            Self::Hour => 0,
            Self::Minute => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PeriodPosition {
    Before,
    #[default]
    After,
}

/// Caller supplies labels and display order. Parsing also accepts regular
/// ASCII and full-width digits independent of the chosen display digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeInputLocale<'a> {
    digits: [char; 10],
    am: &'a str,
    pm: &'a str,
    separator: &'a str,
    period_position: PeriodPosition,
    period_gap: &'a str,
}

impl<'a> TimeInputLocale<'a> {
    pub const fn new(am: &'a str, pm: &'a str, separator: &'a str) -> Self {
        Self {
            digits: ASCII_DIGITS,
            am,
            pm,
            separator,
            period_position: PeriodPosition::After,
            period_gap: " ",
        }
    }

    /// Select one distinct display character per digit.
    pub fn with_digits(mut self, digits: [char; 10]) -> Option<Self> {
        for (index, digit) in digits.iter().copied().enumerate() {
            if digit.is_whitespace()
                || digit.is_control()
                || digits[..index].contains(&digit)
                || standard_digit(digit).is_some_and(|value| value as usize != index)
            {
                return None;
            }
        }
        self.digits = digits;
        Some(self)
    }

    pub fn with_period_position(mut self, position: PeriodPosition, gap: &'a str) -> Self {
        self.period_position = position;
        self.period_gap = gap;
        self
    }

    pub fn period_label(self, period: TimePeriod) -> &'a str {
        match period {
            TimePeriod::Am => self.am,
            TimePeriod::Pm => self.pm,
        }
    }

    pub fn format(self, value: TimeOfDay, clock: ClockFormat) -> String {
        let hour = field_text(
            &self.digits,
            TimeField::Hour,
            value.display_hour(clock),
            clock,
        );
        let minute = field_text(&self.digits, TimeField::Minute, value.minute(), clock);
        let separator = self.separator;
        if clock == ClockFormat::TwentyFourHour {
            return format!("{hour}{separator}{minute}");
        }
        let period = self.period_label(value.period());
        let gap = self.period_gap;
        match self.period_position {
            PeriodPosition::Before => format!("{period}{gap}{hour}{separator}{minute}"),
            PeriodPosition::After => format!("{hour}{separator}{minute}{gap}{period}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeInputError {
    Required,
    ExpectedOneOrTwoDigits,
    OutOfRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeInputValidation {
    hour: Result<u8, TimeInputError>,
    minute: Result<u8, TimeInputError>,
    candidate: Option<TimeOfDay>,
}

impl TimeInputValidation {
    /// The hour field uses the displayed clock's numbering.
    pub fn field(self, field: TimeField) -> Result<u8, TimeInputError> {
        match field {
            TimeField::Hour => self.hour,
            TimeField::Minute => self.minute,
        }
    }

    pub const fn candidate(self) -> Option<TimeOfDay> {
        self.candidate
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeInputEvent {
    Edit(TimeField, String),
    FinishEditing(TimeField),
    SelectPeriod(TimePeriod),
    Confirm,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeInputEffect {
    None,
    Commit(TimeOfDay),
    Dismiss,
}

/// The app owns the stored value. Keep this draft with the inline form or
/// while the modal is open. The clock format and locale stay fixed per draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeInputSession {
    raw: [String; 2],
    show_error: [bool; 2],
    focused: Option<TimeField>,
    clock: ClockFormat,
    digits: [char; 10],
    period: TimePeriod,
}

impl TimeInputSession {
    pub fn new(
        committed: Option<TimeOfDay>,
        clock: ClockFormat,
        locale: TimeInputLocale<'_>,
    ) -> Self {
        let raw = committed
            .map(|time| {
                [
                    field_text(
                        &locale.digits,
                        TimeField::Hour,
                        time.display_hour(clock),
                        clock,
                    ),
                    field_text(&locale.digits, TimeField::Minute, time.minute(), clock),
                ]
            })
            .unwrap_or_default();
        Self {
            raw,
            show_error: [false; 2],
            focused: None,
            clock,
            digits: locale.digits,
            period: committed.map_or(TimePeriod::Am, TimeOfDay::period),
        }
    }

    pub const fn clock(&self) -> ClockFormat {
        self.clock
    }

    pub fn period(&self) -> Option<TimePeriod> {
        (self.clock == ClockFormat::TwelveHour).then_some(self.period)
    }

    pub fn raw(&self, field: TimeField) -> &str {
        &self.raw[field.index()]
    }

    pub fn validation(&self) -> TimeInputValidation {
        let hour = self.parse(TimeField::Hour);
        let minute = self.parse(TimeField::Minute);
        let candidate = match (hour, minute, self.clock) {
            (Ok(hour), Ok(minute), ClockFormat::TwelveHour) => {
                TimeOfDay::from_twelve_hour(hour, minute, self.period)
            }
            (Ok(hour), Ok(minute), ClockFormat::TwentyFourHour) => TimeOfDay::new(hour, minute),
            _ => None,
        };
        TimeInputValidation {
            hour,
            minute,
            candidate,
        }
    }

    pub fn displayed_error(&self, field: TimeField) -> Option<TimeInputError> {
        if self.show_error[field.index()] {
            self.parse(field).err()
        } else {
            None
        }
    }

    /// Validate once when a previously observed field loses focus. Return
    /// whether the visible error or normalized text changed.
    pub fn observe_input_focus(&mut self, focused: Option<TimeField>) -> bool {
        let previous = std::mem::replace(&mut self.focused, focused);
        if let Some(field) = previous.filter(|field| Some(*field) != focused) {
            self.finish(field)
        } else {
            false
        }
    }

    /// Track accepted editor focus for a Baseline time picker. An entry, exit
    /// or move between fields needs a rebuild to update its fill and outline.
    /// Normal editor blur still validates and normalizes the previous field.
    pub fn observe_baseline_input_focus(&mut self, focused: Option<TimeField>) -> bool {
        let changed = self.focused != focused;
        self.observe_input_focus(focused) || changed
    }

    pub(crate) const fn focused_input_field(&self) -> Option<TimeField> {
        self.focused
    }

    pub fn apply(&mut self, event: TimeInputEvent) -> TimeInputEffect {
        match event {
            TimeInputEvent::Edit(field, raw) => {
                let index = field.index();
                if self.raw[index] != raw {
                    self.raw[index] = raw;
                    self.show_error[index] = false;
                }
            }
            TimeInputEvent::FinishEditing(field) => {
                self.finish(field);
            }
            TimeInputEvent::SelectPeriod(period) if self.clock == ClockFormat::TwelveHour => {
                self.period = period;
            }
            TimeInputEvent::Confirm => {
                self.show_error = [true; 2];
                if let Some(value) = self.validation().candidate() {
                    for field in TimeField::ALL {
                        self.finish(field);
                    }
                    self.focused = None;
                    return TimeInputEffect::Commit(value);
                }
            }
            TimeInputEvent::Cancel => {
                self.focused = None;
                return TimeInputEffect::Dismiss;
            }
            TimeInputEvent::SelectPeriod(_) => {}
        }
        TimeInputEffect::None
    }

    fn finish(&mut self, field: TimeField) -> bool {
        let index = field.index();
        let previous = self.show_error[index];
        self.show_error[index] = true;
        let Ok(value) = self.parse(field) else {
            return !previous;
        };
        let text = field_text(&self.digits, field, value, self.clock);
        let changed = self.raw[index] != text;
        self.raw[index] = text;
        changed
    }

    pub(crate) fn retire_input_focus(&mut self) {
        self.focused = None;
    }

    /// Accept one dial choice through the draft's number format.
    pub(crate) fn write_picked_value(&mut self, field: TimeField, value: u8) -> bool {
        if !self.accepts_value(field, value) {
            return false;
        }
        self.raw[field.index()] = self.digit_text(field, value);
        self.show_error[field.index()] = false;
        true
    }

    pub(crate) fn digit_text(&self, field: TimeField, value: u8) -> String {
        field_text(&self.digits, field, value, self.clock)
    }

    pub(crate) fn accepts_value(&self, field: TimeField, value: u8) -> bool {
        match (field, self.clock) {
            (TimeField::Hour, ClockFormat::TwelveHour) => (1..=12).contains(&value),
            (TimeField::Hour, ClockFormat::TwentyFourHour) => value < 24,
            (TimeField::Minute, _) => value < 60,
        }
    }

    fn parse(&self, field: TimeField) -> Result<u8, TimeInputError> {
        let text = self.raw(field).trim();
        if text.is_empty() {
            return Err(TimeInputError::Required);
        }
        if text.chars().take(3).count() > 2 {
            return Err(TimeInputError::ExpectedOneOrTwoDigits);
        }
        let number = text
            .chars()
            .try_fold(0, |value, digit| {
                read_digit(&self.digits, digit).map(|digit| value * 10 + digit)
            })
            .ok_or(TimeInputError::ExpectedOneOrTwoDigits)?;
        if self.accepts_value(field, number) {
            Ok(number)
        } else {
            Err(TimeInputError::OutOfRange)
        }
    }
}

fn field_text(digits: &[char; 10], field: TimeField, value: u8, clock: ClockFormat) -> String {
    let mut output = String::new();
    if value >= 10 || field == TimeField::Minute || clock == ClockFormat::TwentyFourHour {
        output.push(digits[usize::from(value / 10)]);
    }
    output.push(digits[usize::from(value % 10)]);
    output
}

fn read_digit(digits: &[char; 10], digit: char) -> Option<u8> {
    standard_digit(digit).or_else(|| {
        digits
            .iter()
            .position(|candidate| *candidate == digit)
            .map(|n| n as u8)
    })
}

fn standard_digit(digit: char) -> Option<u8> {
    match digit {
        '0'..='9' => Some((digit as u32 - '0' as u32) as u8),
        '０'..='９' => Some((digit as u32 - '０' as u32) as u8),
        _ => None,
    }
}
