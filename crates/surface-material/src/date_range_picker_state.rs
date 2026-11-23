//! Pure range draft and selection rules. The app owns the saved value.

use crate::date_picker_state::resolve_navigation_target;
use crate::{
    CivilDate, DateInputError, DatePickerConstraints, DatePickerLocale, DatePickerMode,
    DatePickerSession, DatePickerStep, StepDirection, YearMonth,
};

/// An ordered inclusive Gregorian date range. A same-day range is valid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilDateRange {
    start: CivilDate,
    end: CivilDate,
}

impl CivilDateRange {
    pub fn new(start: CivilDate, end: CivilDate) -> Option<Self> {
        (start <= end).then_some(Self { start, end })
    }

    pub const fn start(self) -> CivilDate {
        self.start
    }

    pub const fn end(self) -> CivilDate {
        self.end
    }

    pub fn contains(self, date: CivilDate) -> bool {
        (self.start..=self.end).contains(&date)
    }
}

/// Check both endpoints with existing date policy. An optional span rule
/// can reject an otherwise valid ordered range without scanning dates here.
#[derive(Clone, Copy)]
pub struct DateRangeConstraints<'a> {
    pub dates: DatePickerConstraints<'a>,
    /// Use a consistent result for a range in one view and its update.
    pub allowed_span: Option<&'a dyn Fn(CivilDateRange) -> bool>,
}

impl DateRangeConstraints<'_> {
    fn allows_span(self, range: CivilDateRange) -> bool {
        self.allowed_span.is_none_or(|allows| allows(range))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateRangeEndpoint {
    Start,
    End,
}

impl DateRangeEndpoint {
    fn index(self) -> usize {
        match self {
            Self::Start => 0,
            Self::End => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateRangeMode {
    Calendar,
    Input,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateRangePairError {
    Reversed,
    SpanUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateRangePosition {
    None,
    Start,
    Interior,
    End,
    OnlyDay,
}

/// A current-policy snapshot. Both field errors are independently retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateRangeValidation {
    start: Result<CivilDate, DateInputError>,
    end: Result<CivilDate, DateInputError>,
    pair_error: Option<DateRangePairError>,
}

impl DateRangeValidation {
    pub fn endpoint(self, endpoint: DateRangeEndpoint) -> Result<CivilDate, DateInputError> {
        match endpoint {
            DateRangeEndpoint::Start => self.start,
            DateRangeEndpoint::End => self.end,
        }
    }

    pub const fn pair_error(self) -> Option<DateRangePairError> {
        self.pair_error
    }

    pub fn candidate(self) -> Option<CivilDateRange> {
        if self.pair_error.is_some() {
            return None;
        }
        CivilDateRange::new(self.start.ok()?, self.end.ok()?)
    }

    /// Use one validation snapshot for every visible date in the view.
    pub fn position(self, date: CivilDate) -> DateRangePosition {
        match self.candidate() {
            Some(range) if range.start() == date && range.end() == date => {
                DateRangePosition::OnlyDay
            }
            Some(range) if range.start() == date => DateRangePosition::Start,
            Some(range) if range.end() == date => DateRangePosition::End,
            Some(range) if range.contains(date) => DateRangePosition::Interior,
            None if self.start == Ok(date) => DateRangePosition::Start,
            _ => DateRangePosition::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DateRangeEvent {
    SelectDay(CivilDate),
    Edit(DateRangeEndpoint, String),
    FinishEditing(DateRangeEndpoint),
    SetMode(DateRangeMode),
    /// Jump to a month and set its keyboard entry date.
    BrowseTo(YearMonth),
    /// Report the month at the viewport anchor after passive scrolling.
    SetVisibleMonth(YearMonth),
    FocusDay(CivilDate),
    /// Move from the actual focused day, which may be in another mounted month.
    NavigateFrom(CivilDate, DatePickerStep),
    Confirm,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateRangeEffect {
    None,
    Commit(CivilDateRange),
    Dismiss,
}

/// Own the two raw endpoint buffers for the duration of a picker session.
/// The caller consumes this state when handling Commit or Dismiss.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateRangePickerState {
    raw: [String; 2],
    show_field_error: [bool; 2],
    show_pair_error: bool,
    rejected_span: bool,
    mode: DateRangeMode,
    month: YearMonth,
    active: Option<CivilDate>,
    preferred_day: u8,
    input_focus: Option<DateRangeEndpoint>,
}

impl DateRangePickerState {
    pub fn new(
        committed: Option<CivilDateRange>,
        today: CivilDate,
        constraints: DateRangeConstraints<'_>,
        locale: &DatePickerLocale<'_>,
        mode: DateRangeMode,
    ) -> Self {
        let calendar = DatePickerSession::new(
            committed.map(CivilDateRange::start),
            today,
            constraints.dates,
            locale,
            DatePickerMode::Calendar,
        );
        let raw = committed
            .map(|range| [range.start(), range.end()].map(|date| locale.input_format.format(date)))
            .unwrap_or_default();
        Self {
            raw,
            show_field_error: [false; 2],
            show_pair_error: false,
            rejected_span: false,
            mode,
            month: calendar.month(),
            active: calendar.active(),
            preferred_day: calendar.active().map_or(today.day(), CivilDate::day),
            input_focus: None,
        }
    }

    pub fn raw_input(&self, endpoint: DateRangeEndpoint) -> &str {
        &self.raw[endpoint.index()]
    }

    pub const fn mode(&self) -> DateRangeMode {
        self.mode
    }

    /// The month at the calendar cursor. Passive scrolling may move it
    /// away from the month of [Self::active].
    pub const fn month(&self) -> YearMonth {
        self.month
    }

    pub const fn active(&self) -> Option<CivilDate> {
        self.active
    }

    /// Resolve the day mounted by a matching [DateRangeEvent::NavigateFrom].
    /// The caller supplies the actual focused day. It can be in any mounted month.
    pub fn navigation_target_from(
        &self,
        origin: CivilDate,
        step: DatePickerStep,
        constraints: DateRangeConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> Option<CivilDate> {
        if self.mode != DateRangeMode::Calendar {
            return None;
        }
        let target = resolve_navigation_target(
            origin,
            self.preferred_day_from(origin),
            step,
            constraints.dates,
            locale.first_weekday,
        );
        target.or_else(|| {
            let DatePickerStep::Day(direction) = step else {
                return None;
            };
            // Cross a month edge after unavailable dates in the continuous
            // feed. Search at most one adjoining month.
            if !constraints.dates.bounds.contains(origin) {
                return None;
            }
            let month = origin.year_month();
            let boundary = month.day_clamped(match direction {
                StepDirection::Backward => 1,
                StepDirection::Forward => month.days_in_month(),
            });
            if boundary == origin {
                return None;
            }
            resolve_navigation_target(
                boundary,
                boundary.day(),
                step,
                constraints.dates,
                locale.first_weekday,
            )
        })
    }

    pub fn validation(
        &self,
        constraints: DateRangeConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> DateRangeValidation {
        let start = self.parse(DateRangeEndpoint::Start, constraints, locale);
        let end = self.parse(DateRangeEndpoint::End, constraints, locale);
        let pair_error = match (start, end) {
            (Ok(start), Ok(end)) => match CivilDateRange::new(start, end) {
                None => Some(DateRangePairError::Reversed),
                Some(range) if !constraints.allows_span(range) => {
                    Some(DateRangePairError::SpanUnavailable)
                }
                Some(_) => None,
            },
            _ => None,
        };
        DateRangeValidation {
            start,
            end,
            pair_error,
        }
    }

    pub fn displayed_field_error(
        &self,
        endpoint: DateRangeEndpoint,
        validation: DateRangeValidation,
    ) -> Option<DateInputError> {
        if self.show_field_error[endpoint.index()] {
            validation.endpoint(endpoint).err()
        } else {
            None
        }
    }

    /// Report a rejected calendar span without replacing the existing draft.
    pub fn displayed_pair_error(
        &self,
        validation: DateRangeValidation,
    ) -> Option<DateRangePairError> {
        if self.rejected_span {
            Some(DateRangePairError::SpanUnavailable)
        } else if self.show_pair_error {
            validation.pair_error()
        } else {
            None
        }
    }

    /// Reconcile calendar position after the caller changes availability.
    /// Validation uses the new policy while both raw buffers stay intact.
    pub fn reconcile_constraints(&mut self, constraints: DateRangeConstraints<'_>) {
        let bounds = constraints.dates.bounds;
        self.month = self
            .month
            .clamp(bounds.min().year_month(), bounds.max().year_month());
        if !self
            .active
            .is_some_and(|date| constraints.dates.allows(date))
        {
            self.reset_active_in_visible_month(constraints);
        }
        self.rejected_span = false;
    }

    /// Normalize only the field that loses focus. Return whether a raw
    /// buffer or its visible validation changed.
    pub fn observe_input_focus(
        &mut self,
        focused: Option<DateRangeEndpoint>,
        constraints: DateRangeConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> bool {
        let focused = focused.filter(|_| self.mode == DateRangeMode::Input);
        let previous = std::mem::replace(&mut self.input_focus, focused);
        if let Some(previous) = previous.filter(|previous| Some(*previous) != focused) {
            self.finish_editing(previous, constraints, locale)
        } else {
            false
        }
    }

    pub fn apply(
        &mut self,
        event: DateRangeEvent,
        constraints: DateRangeConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> DateRangeEffect {
        match event {
            DateRangeEvent::SelectDay(date)
                if self.mode == DateRangeMode::Calendar && constraints.dates.allows(date) =>
            {
                self.select_day(date, constraints, locale);
            }
            DateRangeEvent::Edit(endpoint, raw) if self.mode == DateRangeMode::Input => {
                let index = endpoint.index();
                if self.raw[index] != raw {
                    self.raw[index] = raw;
                    self.show_field_error[index] = false;
                    self.show_pair_error = false;
                    self.rejected_span = false;
                }
            }
            DateRangeEvent::FinishEditing(endpoint) if self.mode == DateRangeMode::Input => {
                self.finish_editing(endpoint, constraints, locale);
            }
            DateRangeEvent::SetMode(mode) if self.mode != mode => {
                self.input_focus = None;
                self.mode = mode;
                if mode == DateRangeMode::Calendar {
                    self.show_pair_error = true;
                    if let Ok(start) = self.parse(DateRangeEndpoint::Start, constraints, locale) {
                        self.focus_day(start);
                    }
                }
            }
            DateRangeEvent::BrowseTo(month) if self.mode == DateRangeMode::Calendar => {
                self.browse_to(month, constraints);
            }
            DateRangeEvent::SetVisibleMonth(month) if self.mode == DateRangeMode::Calendar => {
                self.set_visible_month(month, constraints);
            }
            DateRangeEvent::FocusDay(date)
                if self.mode == DateRangeMode::Calendar
                    && self.active != Some(date)
                    && constraints.dates.allows(date) =>
            {
                self.focus_day(date);
            }
            DateRangeEvent::NavigateFrom(origin, step) => {
                if let Some(date) = self.navigation_target_from(origin, step, constraints, locale) {
                    let preferred_day = self.preferred_day_from(origin);
                    self.focus_day(date);
                    if step.preserves_preferred_day() {
                        self.preferred_day = preferred_day;
                    }
                }
            }
            DateRangeEvent::Confirm => {
                self.show_field_error = [true; 2];
                self.show_pair_error = true;
                self.rejected_span = false;
                if let Some(range) = self.validation(constraints, locale).candidate() {
                    self.raw =
                        [range.start(), range.end()].map(|date| locale.input_format.format(date));
                    self.input_focus = None;
                    return DateRangeEffect::Commit(range);
                }
            }
            DateRangeEvent::Cancel => {
                self.input_focus = None;
                return DateRangeEffect::Dismiss;
            }
            _ => {}
        }
        DateRangeEffect::None
    }

    fn parse(
        &self,
        endpoint: DateRangeEndpoint,
        constraints: DateRangeConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> Result<CivilDate, DateInputError> {
        locale
            .input_format
            .parse(self.raw_input(endpoint))
            .and_then(|date| constraints.dates.validate(date))
    }

    fn finish_editing(
        &mut self,
        endpoint: DateRangeEndpoint,
        constraints: DateRangeConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> bool {
        let index = endpoint.index();
        let validation = self.validation(constraints, locale);
        let mut changed = self.rejected_span
            || (!self.show_field_error[index] && validation.endpoint(endpoint).is_err())
            || (!self.show_pair_error && validation.pair_error().is_some());
        if let Ok(date) = validation.endpoint(endpoint) {
            let formatted = locale.input_format.format(date);
            changed |= self.raw[index] != formatted;
            self.raw[index] = formatted;
        }
        self.show_field_error[index] = true;
        self.show_pair_error = true;
        self.rejected_span = false;
        changed
    }

    fn select_day(
        &mut self,
        date: CivilDate,
        constraints: DateRangeConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) {
        let draft = self.validation(constraints, locale);
        self.focus_day(date);
        let provisional = draft.start.ok().filter(|_| draft.candidate().is_none());
        if let Some(range) = provisional.and_then(|start| CivilDateRange::new(start, date)) {
            if !constraints.allows_span(range) {
                self.rejected_span = true;
                return;
            }
            self.raw[DateRangeEndpoint::End.index()] = locale.input_format.format(date);
        } else {
            self.raw = [locale.input_format.format(date), String::new()];
        }
        self.show_field_error = [false; 2];
        self.show_pair_error = false;
        self.rejected_span = false;
    }

    fn preferred_day_from(&self, origin: CivilDate) -> u8 {
        if self.active == Some(origin) {
            self.preferred_day
        } else {
            origin.day()
        }
    }

    fn focus_day(&mut self, date: CivilDate) {
        self.active = Some(date);
        self.month = date.year_month();
        self.preferred_day = date.day();
    }

    fn browse_to(&mut self, month: YearMonth, constraints: DateRangeConstraints<'_>) {
        if self.set_visible_month(month, constraints)
            && !self
                .active
                .is_some_and(|date| date.year_month() == month && constraints.dates.allows(date))
        {
            self.reset_active_in_visible_month(constraints);
        }
    }

    fn set_visible_month(
        &mut self,
        month: YearMonth,
        constraints: DateRangeConstraints<'_>,
    ) -> bool {
        let bounds = constraints.dates.bounds;
        if !(bounds.min().year_month()..=bounds.max().year_month()).contains(&month) {
            return false;
        }
        self.month = month;
        true
    }

    fn reset_active_in_visible_month(&mut self, constraints: DateRangeConstraints<'_>) {
        let month = self.month;
        self.active = (1..=month.days_in_month())
            .map(|day| month.day_clamped(day))
            .find(|date| constraints.dates.allows(*date));
        if let Some(active) = self.active {
            self.preferred_day = active.day();
        }
    }
}
