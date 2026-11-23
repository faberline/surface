//! Gregorian civil dates and the caller-owned single-date picker session.

use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

use crate::shape::LayoutDirection;

const MIN_YEAR: u16 = 1;
const MAX_YEAR: u16 = 9999;
const MONTH_STARTS: [i32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

/// A date in the proleptic Gregorian calendar, with no time or time zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilDate {
    year: u16,
    month: u8,
    day: u8,
}

impl CivilDate {
    pub const fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        match YearMonth::new(year, month) {
            Some(month) if day > 0 && day <= month.days_in_month() => Some(month.date(day)),
            _ => None,
        }
    }

    pub const fn year(self) -> u16 {
        self.year
    }

    pub const fn month(self) -> u8 {
        self.month
    }

    pub const fn day(self) -> u8 {
        self.day
    }

    pub const fn year_month(self) -> YearMonth {
        YearMonth {
            year: self.year,
            month: self.month,
        }
    }

    pub fn weekday(self) -> Weekday {
        // 0001-01-01 is Monday in the proleptic Gregorian calendar.
        Weekday::ALL[(self.day_number() % 7) as usize]
    }

    pub fn checked_add_days(self, days: i32) -> Option<Self> {
        Self::from_day_number(self.day_number().checked_add(days)?)
    }

    fn day_number(self) -> i32 {
        self.year_month().first_day_number() + i32::from(self.day) - 1
    }

    fn from_day_number(day: i32) -> Option<Self> {
        if !(0..days_before_year(MAX_YEAR + 1)).contains(&day) {
            return None;
        }
        let mut first = MIN_YEAR;
        let mut next = MAX_YEAR + 1;
        while next - first > 1 {
            let middle = first + (next - first) / 2;
            if days_before_year(middle) <= day {
                first = middle;
            } else {
                next = middle;
            }
        }
        let mut month = YearMonth {
            year: first,
            month: 1,
        };
        let mut remaining = day - days_before_year(first);
        while remaining >= i32::from(month.days_in_month()) {
            remaining -= i32::from(month.days_in_month());
            month.month += 1;
        }
        Some(month.date((remaining + 1) as u8))
    }
}

impl fmt::Display for CivilDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    year: u16,
    month: u8,
}

impl YearMonth {
    pub const fn new(year: u16, month: u8) -> Option<Self> {
        if year < MIN_YEAR || year > MAX_YEAR || month < 1 || month > 12 {
            None
        } else {
            Some(Self { year, month })
        }
    }

    pub const fn year(self) -> u16 {
        self.year
    }

    pub const fn month(self) -> u8 {
        self.month
    }

    pub fn step(self, months: i32) -> Option<Self> {
        let index = (i32::from(self.year) - 1) * 12 + i32::from(self.month) - 1;
        let target = index.checked_add(months)?;
        if !(0..i32::from(MAX_YEAR) * 12).contains(&target) {
            return None;
        }
        Self::new((target / 12 + 1) as u16, (target % 12 + 1) as u8)
    }

    pub const fn days_in_month(self) -> u8 {
        match self.month {
            2 if leap_year(self.year) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    /// Use the closest valid day in this month.
    pub fn day_clamped(self, day: u8) -> CivilDate {
        self.date(day.clamp(1, self.days_in_month()))
    }

    /// Six weeks of cells in chronological order. Adjacent months are blank.
    /// Rotate the weekday labels with the same first weekday.
    pub fn calendar_days(self, first_weekday: Weekday) -> [Option<CivilDate>; 42] {
        let start = weekday_offset(self.date(1).weekday(), first_weekday);
        let end = start + usize::from(self.days_in_month());
        std::array::from_fn(|index| {
            (start..end)
                .contains(&index)
                .then(|| self.date((index - start + 1) as u8))
        })
    }

    const fn date(self, day: u8) -> CivilDate {
        CivilDate {
            year: self.year,
            month: self.month,
            day,
        }
    }

    fn first_day_number(self) -> i32 {
        let leap_day = i32::from(self.month > 2 && leap_year(self.year));
        days_before_year(self.year) + MONTH_STARTS[usize::from(self.month - 1)] + leap_day
    }

    fn last_day_number(self) -> i32 {
        self.first_day_number() + i32::from(self.days_in_month()) - 1
    }
}

const fn leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn days_before_year(year: u16) -> i32 {
    let completed = i32::from(year) - 1;
    completed * 365 + completed / 4 - completed / 100 + completed / 400
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub const ALL: [Self; 7] = [
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
        Self::Saturday,
        Self::Sunday,
    ];

    /// Monday is zero and Sunday is six.
    pub const fn index(self) -> usize {
        self as usize
    }
}

fn weekday_offset(day: Weekday, first_weekday: Weekday) -> usize {
    (day.index() + 7 - first_weekday.index()) % 7
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateBounds {
    min: CivilDate,
    max: CivilDate,
}

impl DateBounds {
    pub fn new(min: CivilDate, max: CivilDate) -> Option<Self> {
        (min <= max).then_some(Self { min, max })
    }

    pub const fn min(self) -> CivilDate {
        self.min
    }

    pub const fn max(self) -> CivilDate {
        self.max
    }

    pub fn contains(self, date: CivilDate) -> bool {
        (self.min..=self.max).contains(&date)
    }

    fn contains_month(self, month: YearMonth) -> bool {
        (self.min.year_month()..=self.max.year_month()).contains(&month)
    }
}

/// Inclusive bounds and an optional date rule. The rule should give the
/// same result for a date throughout one view and its corresponding update.
#[derive(Clone, Copy)]
pub struct DatePickerConstraints<'a> {
    pub bounds: DateBounds,
    pub allowed_date: Option<&'a dyn Fn(CivilDate) -> bool>,
}

impl<'a> DatePickerConstraints<'a> {
    pub fn allows(self, date: CivilDate) -> bool {
        self.validate(date).is_ok()
    }

    pub fn validate(self, date: CivilDate) -> Result<CivilDate, DateInputError> {
        if !self.bounds.contains(date) {
            return Err(DateInputError::OutsideBounds);
        }
        if self.allowed_date.is_some_and(|allows| !allows(date)) {
            return Err(DateInputError::Unavailable);
        }
        Ok(date)
    }

    fn allowed_days(self, start: i32, end: i32) -> impl DoubleEndedIterator<Item = CivilDate> + 'a {
        let start = start.max(self.bounds.min.day_number());
        let end = end.min(self.bounds.max.day_number());
        (start..=end)
            .filter_map(CivilDate::from_day_number)
            .filter(move |date| self.allows(*date))
    }

    fn nearest(
        self,
        target: CivilDate,
        start: i32,
        end: i32,
        direction: StepDirection,
    ) -> Option<CivilDate> {
        let target = target.day_number();
        self.allowed_days(start, end).min_by_key(|date| {
            let offset = date.day_number() - target;
            let before_target = offset < 0;
            let prefer_before = direction == StepDirection::Backward;
            (offset.abs(), before_target != prefer_before)
        })
    }

    fn nearest_in_month(
        self,
        month: YearMonth,
        preferred_day: u8,
        direction: StepDirection,
    ) -> Option<CivilDate> {
        self.nearest(
            month.day_clamped(preferred_day),
            month.first_day_number(),
            month.last_day_number(),
            direction,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateInputOrder {
    YearMonthDay,
    MonthDayYear,
    DayMonthYear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateInputSeparator {
    Slash,
    Dash,
    Dot,
    Space,
}

impl DateInputSeparator {
    fn as_char(self) -> char {
        match self {
            Self::Slash => '/',
            Self::Dash => '-',
            Self::Dot => '.',
            Self::Space => ' ',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateInputError {
    Required,
    Incomplete,
    InvalidFormat,
    InvalidDate,
    OutsideBounds,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateInputFormat {
    pub order: DateInputOrder,
    pub separator: DateInputSeparator,
}

impl DateInputFormat {
    /// Parse the declared component order. Accept ASCII and full-width
    /// decimal digits and common date delimiters. Policy is validated
    /// separately through [DatePickerConstraints::validate].
    pub fn parse(self, input: &str) -> Result<CivilDate, DateInputError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(DateInputError::Required);
        }
        let mut chars = input.chars().peekable();
        let first = take_number(&mut chars)?;
        take_delimiter(&mut chars)?;
        let second = take_number(&mut chars)?;
        take_delimiter(&mut chars)?;
        let third = take_number(&mut chars)?;
        if chars.peek().is_some() {
            return Err(DateInputError::InvalidFormat);
        }
        let ((year, year_digits), (month, month_digits), (day, day_digits)) = match self.order {
            DateInputOrder::YearMonthDay => (first, second, third),
            DateInputOrder::MonthDayYear => (third, first, second),
            DateInputOrder::DayMonthYear => (third, second, first),
        };
        if year_digits != 4 || month_digits > 2 || day_digits > 2 {
            return Err(DateInputError::InvalidFormat);
        }
        CivilDate::new(year, month as u8, day as u8).ok_or(DateInputError::InvalidDate)
    }

    /// Format using the declared order and canonical separator.
    pub fn format(self, date: CivilDate) -> String {
        let CivilDate { year, month, day } = date;
        let sep = self.separator.as_char();
        match self.order {
            DateInputOrder::YearMonthDay => format!("{year:04}{sep}{month:02}{sep}{day:02}"),
            DateInputOrder::MonthDayYear => format!("{month:02}{sep}{day:02}{sep}{year:04}"),
            DateInputOrder::DayMonthYear => format!("{day:02}{sep}{month:02}{sep}{year:04}"),
        }
    }
}

fn digit_value(ch: char) -> Option<u16> {
    match ch {
        '0'..='9' => Some((ch as u16) - ('0' as u16)),
        '０'..='９' => Some((ch as u16) - ('０' as u16)),
        _ => None,
    }
}

fn take_number(chars: &mut Peekable<Chars<'_>>) -> Result<(u16, u8), DateInputError> {
    let mut value = 0;
    let mut count = 0;
    while let Some(digit) = chars.peek().copied().and_then(digit_value) {
        if count == 4 {
            return Err(DateInputError::InvalidFormat);
        }
        chars.next();
        value = value * 10 + digit;
        count += 1;
    }
    if count > 0 {
        Ok((value, count))
    } else if chars.peek().is_none() {
        Err(DateInputError::Incomplete)
    } else {
        Err(DateInputError::InvalidFormat)
    }
}

fn take_delimiter(chars: &mut Peekable<Chars<'_>>) -> Result<(), DateInputError> {
    let mut whitespace = false;
    while chars.next_if(|ch| ch.is_whitespace()).is_some() {
        whitespace = true;
    }
    if chars
        .next_if(|ch| matches!(*ch, '/' | '-' | '.' | '／' | '－' | '．'))
        .is_some()
    {
        while chars.next_if(|ch| ch.is_whitespace()).is_some() {}
        Ok(())
    } else if whitespace {
        Ok(())
    } else if chars.peek().is_none() {
        Err(DateInputError::Incomplete)
    } else {
        Err(DateInputError::InvalidFormat)
    }
}

/// Caller-supplied locale rules and text. The weekday arrays use Monday
/// first regardless of the displayed week's start.
#[derive(Clone, Copy)]
pub struct DatePickerLocale<'a> {
    pub direction: LayoutDirection,
    pub first_weekday: Weekday,
    pub input_format: DateInputFormat,
    pub format_hint: &'a str,
    pub weekday_short: [&'a str; 7],
    pub weekday_full: [&'a str; 7],
    pub month_title: &'a dyn Fn(YearMonth) -> String,
    /// Displayed text for the date headline.
    pub date_headline: &'a dyn Fn(CivilDate) -> String,
    /// Full accessible name for a day in the grid.
    pub date_name: &'a dyn Fn(CivilDate) -> String,
    pub error_text: &'a dyn Fn(DateInputError) -> String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DatePickerMode {
    #[default]
    Calendar,
    Year,
    Input,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepDirection {
    Backward,
    Forward,
}

impl StepDirection {
    fn sign(self) -> i32 {
        match self {
            Self::Backward => -1,
            Self::Forward => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePickerStep {
    Day(StepDirection),
    Week(StepDirection),
    WeekStart,
    WeekEnd,
    Month(StepDirection),
    Year(StepDirection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DatePickerEvent {
    BrowseMonth(StepDirection),
    /// Move the visible year list without choosing a year or changing mode.
    BrowseYears(u16),
    ChooseYear(u16),
    Navigate(DatePickerStep),
    /// Navigate from the actual focused day, which may differ from the
    /// session's active date after an accessibility focus action.
    NavigateFrom(CivilDate, DatePickerStep),
    Select(CivilDate),
    SetMode(DatePickerMode),
    Edit(String),
    FinishEditing,
    Confirm,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePickerEffect {
    None,
    Commit(CivilDate),
    Dismiss,
}

/// The caller owns a session while the picker is open. Effects request a
/// commit or dismissal. The session never changes the caller's saved date.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatePickerSession {
    draft: Option<CivilDate>,
    raw_input: String,
    error: Option<DateInputError>,
    mode: DatePickerMode,
    month: YearMonth,
    active: Option<CivilDate>,
    preferred_day: u8,
    input_focused: bool,
}

impl DatePickerSession {
    pub fn new(
        committed: Option<CivilDate>,
        today: CivilDate,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
        initial_mode: DatePickerMode,
    ) -> Self {
        let draft = committed.filter(|date| constraints.allows(*date));
        let start =
            draft.unwrap_or_else(|| today.clamp(constraints.bounds.min, constraints.bounds.max));
        let month = start.year_month();
        let today = (today.year_month() == month && constraints.allows(today)).then_some(today);
        let active = draft.or(today).or_else(|| {
            constraints
                .allowed_days(month.first_day_number(), month.last_day_number())
                .next()
        });
        Self {
            draft,
            raw_input: draft
                .map(|date| locale.input_format.format(date))
                .unwrap_or_default(),
            error: None,
            mode: initial_mode,
            month,
            active,
            preferred_day: active.unwrap_or(start).day,
            input_focused: false,
        }
    }

    pub const fn draft(&self) -> Option<CivilDate> {
        self.draft
    }

    pub fn raw_input(&self) -> &str {
        &self.raw_input
    }

    pub const fn error(&self) -> Option<DateInputError> {
        self.error
    }

    pub const fn mode(&self) -> DatePickerMode {
        self.mode
    }

    pub const fn month(&self) -> YearMonth {
        self.month
    }

    pub const fn active(&self) -> Option<CivilDate> {
        self.active
    }

    /// Observe the editor's stable focus target from the application's frame
    /// hook. Validate on the transition out of a focused Input field. Return
    /// whether the draft, visible error or displayed input changed.
    pub fn observe_input_focus(
        &mut self,
        focused: bool,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> bool {
        let focused = focused && self.mode == DatePickerMode::Input;
        let was_focused = std::mem::replace(&mut self.input_focused, focused);
        was_focused && !focused && self.finish_editing(constraints, locale)
    }

    /// Resolve the same day that [Self::apply] will move to for a Navigate
    /// event. A view can use this to name the focus target mounted by the update.
    pub fn navigation_target(
        &self,
        step: DatePickerStep,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> Option<CivilDate> {
        self.navigation_target_from(self.active?, step, constraints, locale)
    }

    /// Resolve a move from a specific day in the displayed month. Use this
    /// for a control focused directly by an accessibility action. A page
    /// move preserves the preferred day only for the current active date.
    pub fn navigation_target_from(
        &self,
        origin: CivilDate,
        step: DatePickerStep,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> Option<CivilDate> {
        if origin.year_month() != self.month || !constraints.bounds.contains(origin) {
            return None;
        }
        match step {
            DatePickerStep::Day(direction) => {
                let first = origin.checked_add_days(direction.sign())?;
                let month = first.year_month();
                match direction {
                    StepDirection::Backward => constraints
                        .allowed_days(month.first_day_number(), first.day_number())
                        .next_back(),
                    StepDirection::Forward => constraints
                        .allowed_days(first.day_number(), month.last_day_number())
                        .next(),
                }
            }
            DatePickerStep::Week(direction) => {
                let target = origin.checked_add_days(direction.sign() * 7)?;
                let start = week_start(target, locale.first_weekday);
                constraints.nearest(target, start, start + 6, direction)
            }
            DatePickerStep::WeekStart => {
                let start = week_start(origin, locale.first_weekday);
                constraints.allowed_days(start, start + 6).next()
            }
            DatePickerStep::WeekEnd => {
                let start = week_start(origin, locale.first_weekday);
                constraints.allowed_days(start, start + 6).next_back()
            }
            DatePickerStep::Month(direction) | DatePickerStep::Year(direction) => {
                let count = if matches!(step, DatePickerStep::Year(_)) {
                    12
                } else {
                    1
                };
                let month = self.month.step(direction.sign() * count)?;
                if !constraints.bounds.contains_month(month) {
                    return None;
                }
                constraints.nearest_in_month(month, self.preferred_day_from(origin), direction)
            }
        }
    }

    pub fn apply(
        &mut self,
        event: DatePickerEvent,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> DatePickerEffect {
        match event {
            DatePickerEvent::BrowseMonth(direction) => {
                if let Some(month) = self.month.step(direction.sign()) {
                    self.browse(month, direction, constraints);
                }
            }
            DatePickerEvent::ChooseYear(year) => {
                if self.browse_year(year, constraints) {
                    self.set_mode(DatePickerMode::Calendar);
                }
            }
            DatePickerEvent::BrowseYears(year) => {
                self.browse_year(year, constraints);
            }
            DatePickerEvent::Navigate(step) => {
                if let Some(origin) = self.active {
                    self.navigate_from(origin, step, constraints, locale);
                }
            }
            DatePickerEvent::NavigateFrom(origin, step) => {
                self.navigate_from(origin, step, constraints, locale);
            }
            DatePickerEvent::Select(date) => {
                if constraints.allows(date) {
                    self.set_active(date);
                    self.draft = Some(date);
                    self.raw_input = locale.input_format.format(date);
                    self.error = None;
                }
            }
            DatePickerEvent::SetMode(mode) => {
                if self.mode == DatePickerMode::Input && mode != DatePickerMode::Input {
                    if let Some(date) = self.draft.filter(|date| constraints.allows(*date)) {
                        self.set_active(date);
                    }
                }
                self.set_mode(mode);
            }
            DatePickerEvent::Edit(input) => {
                self.raw_input = input;
                self.draft = self.parse(constraints, locale).ok();
                self.error = None;
            }
            DatePickerEvent::FinishEditing => {
                self.finish_editing(constraints, locale);
            }
            DatePickerEvent::Confirm => {
                self.finish_editing(constraints, locale);
                if let Some(date) = self.draft {
                    return DatePickerEffect::Commit(date);
                }
            }
            DatePickerEvent::Cancel => return DatePickerEffect::Dismiss,
        }
        DatePickerEffect::None
    }

    fn browse(
        &mut self,
        month: YearMonth,
        direction: StepDirection,
        constraints: DatePickerConstraints<'_>,
    ) -> bool {
        if !constraints.bounds.contains_month(month) {
            return false;
        }
        self.month = month;
        self.active = constraints.nearest_in_month(month, self.preferred_day, direction);
        true
    }

    pub(super) fn browse_to(
        &mut self,
        month: YearMonth,
        constraints: DatePickerConstraints<'_>,
    ) -> bool {
        let direction = if month < self.month {
            StepDirection::Backward
        } else {
            StepDirection::Forward
        };
        self.browse(month, direction, constraints)
    }

    fn browse_year(&mut self, year: u16, constraints: DatePickerConstraints<'_>) -> bool {
        let first = constraints.bounds.min.year_month();
        let last = constraints.bounds.max.year_month();
        if !(first.year..=last.year).contains(&year) {
            return false;
        }
        let month = YearMonth {
            year,
            month: self.month.month,
        }
        .clamp(first, last);
        self.browse_to(month, constraints)
    }

    fn set_mode(&mut self, mode: DatePickerMode) {
        if self.mode != mode {
            self.input_focused = false;
            self.mode = mode;
        }
    }

    fn preferred_day_from(&self, origin: CivilDate) -> u8 {
        if self.active == Some(origin) {
            self.preferred_day
        } else {
            origin.day
        }
    }

    fn navigate_from(
        &mut self,
        origin: CivilDate,
        step: DatePickerStep,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) {
        if let Some(date) = self.navigation_target_from(origin, step, constraints, locale) {
            let preferred_day = self.preferred_day_from(origin);
            self.set_active(date);
            if matches!(step, DatePickerStep::Month(_) | DatePickerStep::Year(_)) {
                self.preferred_day = preferred_day;
            }
        }
    }

    fn set_active(&mut self, date: CivilDate) {
        self.month = date.year_month();
        self.active = Some(date);
        self.preferred_day = date.day;
    }

    fn parse(
        &self,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> Result<CivilDate, DateInputError> {
        constraints.validate(locale.input_format.parse(&self.raw_input)?)
    }

    fn finish_editing(
        &mut self,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> bool {
        match self.parse(constraints, locale) {
            Ok(date) => {
                let formatted = locale.input_format.format(date);
                let changed =
                    self.draft != Some(date) || self.error.is_some() || self.raw_input != formatted;
                self.draft = Some(date);
                self.raw_input = formatted;
                self.error = None;
                changed
            }
            Err(error) => {
                let changed = self.draft.is_some() || self.error != Some(error);
                self.draft = None;
                self.error = Some(error);
                changed
            }
        }
    }
}

fn week_start(date: CivilDate, start: Weekday) -> i32 {
    date.day_number() - weekday_offset(date.weekday(), start) as i32
}
