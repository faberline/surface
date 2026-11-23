//! Controlled field, calendar state and anchor placement for the docked date picker.

use surface::Rect;

use crate::{
    shape::LayoutDirection, CivilDate, DateInputError, DatePickerConstraints, DatePickerEffect,
    DatePickerEvent, DatePickerLocale, DatePickerMode, DatePickerSession, YearMonth,
};

const PANEL_WIDTH: f32 = 360.;
const PANEL_HEIGHT: f32 = 456.;
const FIELD_GAP: f32 = 4.;
const MIN_TARGET: f32 = 48.;
pub(crate) const DOCKED_HEADER_HEIGHT: f32 = 64.;
const MIN_PANEL_HEIGHT: f32 = DOCKED_HEADER_HEIGHT + MIN_TARGET;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockedDatePickerMode {
    Calendar,
    Month,
    Year,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DockedDatePickerEvent {
    Edit(String),
    FinishEditing,
    ToggleCalendar,
    Dismiss,
    Calendar(DatePickerEvent),
    SetMode(DockedDatePickerMode),
    BrowseTo(YearMonth),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputFocus {
    Unfocused,
    Focused,
    SuppressBlur,
}

/// The app owns this state for as long as the editable field is mounted.
/// The field draft and the popup's browse position have separate lifetimes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockedDatePickerState {
    committed: Option<CivilDate>,
    raw_input: String,
    error: Option<DateInputError>,
    calendar: DatePickerSession,
    mode: Option<DockedDatePickerMode>,
    input_focus: InputFocus,
    input_edited: bool,
}

impl DockedDatePickerState {
    pub fn new(
        committed: Option<CivilDate>,
        today: CivilDate,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> Self {
        Self {
            committed,
            raw_input: committed
                .map(|date| locale.input_format.format(date))
                .unwrap_or_default(),
            error: None,
            calendar: DatePickerSession::new(
                committed,
                today,
                constraints,
                locale,
                DatePickerMode::Calendar,
            ),
            mode: None,
            input_focus: InputFocus::Unfocused,
            input_edited: false,
        }
    }

    pub const fn committed(&self) -> Option<CivilDate> {
        self.committed
    }

    pub fn raw_input(&self) -> &str {
        &self.raw_input
    }

    pub const fn error(&self) -> Option<DateInputError> {
        self.error
    }

    pub const fn mode(&self) -> Option<DockedDatePickerMode> {
        self.mode
    }

    pub const fn calendar(&self) -> &DatePickerSession {
        &self.calendar
    }

    /// Feed the editor's stable focus ID from the app's frame hook. An
    /// ordinary focus exit validates an edited value while the popup is
    /// closed. Focus changes inside the open picker preserve the draft.
    /// Popup toggles and dismissal disarm blur before moving focus.
    /// The boolean reports a change that needs a view rebuild.
    pub fn observe_input_focus(
        &mut self,
        focused: bool,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> (bool, DatePickerEffect) {
        if focused {
            if self.input_focus == InputFocus::Unfocused {
                self.input_focus = InputFocus::Focused;
            }
            return (false, DatePickerEffect::None);
        }
        let previous = std::mem::replace(&mut self.input_focus, InputFocus::Unfocused);
        if previous == InputFocus::Focused && self.input_edited && self.mode.is_none() {
            self.finish_input(constraints, locale)
        } else {
            (false, DatePickerEffect::None)
        }
    }

    /// Apply one editor or popup event. A Commit effect reports the new
    /// stored value. A Dismiss effect reports a closed popup. Both leave
    /// this retained field state available to the caller.
    pub fn apply(
        &mut self,
        event: DockedDatePickerEvent,
        today: CivilDate,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> DatePickerEffect {
        match event {
            DockedDatePickerEvent::Edit(raw) => {
                if raw != self.raw_input {
                    self.raw_input = raw;
                    self.error = None;
                    self.input_edited = true;
                }
            }
            DockedDatePickerEvent::FinishEditing => {
                return self.finish_input(constraints, locale).1;
            }
            DockedDatePickerEvent::ToggleCalendar => {
                self.disarm_blur();
                if self.mode.take().is_some() {
                    return DatePickerEffect::Dismiss;
                }
                self.calendar = DatePickerSession::new(
                    self.committed,
                    today,
                    constraints,
                    locale,
                    DatePickerMode::Calendar,
                );
                self.mode = Some(DockedDatePickerMode::Calendar);
            }
            DockedDatePickerEvent::Dismiss => {
                self.disarm_blur();
                match self.mode {
                    Some(DockedDatePickerMode::Calendar) => {
                        self.mode = None;
                        return DatePickerEffect::Dismiss;
                    }
                    Some(DockedDatePickerMode::Month | DockedDatePickerMode::Year) => {
                        self.mode = Some(DockedDatePickerMode::Calendar);
                    }
                    None => {}
                }
            }
            DockedDatePickerEvent::Calendar(event) => {
                return self.apply_calendar(event, constraints, locale);
            }
            DockedDatePickerEvent::SetMode(mode) => {
                if self.mode.is_some() {
                    self.mode = Some(mode);
                }
            }
            DockedDatePickerEvent::BrowseTo(month) => {
                if matches!(
                    self.mode,
                    Some(DockedDatePickerMode::Month | DockedDatePickerMode::Year)
                ) && self.calendar.browse_to(month, constraints)
                {
                    self.mode = Some(DockedDatePickerMode::Calendar);
                }
            }
        }
        DatePickerEffect::None
    }

    fn apply_calendar(
        &mut self,
        event: DatePickerEvent,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> DatePickerEffect {
        use DatePickerEvent::{
            BrowseMonth, BrowseYears, ChooseYear, Navigate, NavigateFrom, Select,
        };
        use DockedDatePickerMode::{Calendar, Year};

        match (self.mode, event) {
            (Some(Calendar), Select(date))
                if date.year_month() == self.calendar.month() && constraints.allows(date) =>
            {
                self.disarm_blur();
                self.mode = None;
                let (_, effect) = self.accept_date(date, constraints, locale);
                if effect != DatePickerEffect::None {
                    return effect;
                }
                return DatePickerEffect::Dismiss;
            }
            (Some(Calendar), event @ (BrowseMonth(_) | Navigate(_) | NavigateFrom(_, _)))
            | (Some(Year), event @ BrowseYears(_)) => {
                self.calendar.apply(event, constraints, locale);
            }
            (Some(Year), ChooseYear(year))
                if (constraints.bounds.min().year()..=constraints.bounds.max().year())
                    .contains(&year) =>
            {
                self.calendar.apply(ChooseYear(year), constraints, locale);
                self.mode = Some(Calendar);
            }
            _ => {}
        }
        DatePickerEffect::None
    }

    fn disarm_blur(&mut self) {
        if self.input_focus == InputFocus::Focused {
            self.input_focus = InputFocus::SuppressBlur;
        }
    }

    fn finish_input(
        &mut self,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> (bool, DatePickerEffect) {
        let parsed = locale
            .input_format
            .parse(&self.raw_input)
            .and_then(|date| constraints.validate(date));
        match parsed {
            Ok(date) => self.accept_date(date, constraints, locale),
            Err(error) => {
                let changed = self.error != Some(error);
                self.error = Some(error);
                (changed, DatePickerEffect::None)
            }
        }
    }

    fn accept_date(
        &mut self,
        date: CivilDate,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> (bool, DatePickerEffect) {
        let formatted = locale.input_format.format(date);
        let committed = self.committed != Some(date);
        let changed = committed || self.raw_input != formatted || self.error.is_some();
        self.raw_input = formatted;
        self.error = None;
        self.input_edited = false;
        if committed {
            self.committed = Some(date);
            self.calendar
                .apply(DatePickerEvent::Select(date), constraints, locale);
            (changed, DatePickerEffect::Commit(date))
        } else {
            (changed, DatePickerEffect::None)
        }
    }
}

/// Place a 360 × 456 dp popup at the field's logical start edge. Keep a 4 dp
/// gap, prefer a full-height popup below, and use the roomier side when
/// neither side fits. Return no placement when the anchor is offscreen or
/// the available space on either side cannot fit the fixed header and a
/// complete 48 dp date target.
pub fn place_docked_date_picker(
    anchor: Rect,
    viewport: Rect,
    direction: LayoutDirection,
) -> Option<Rect> {
    if !valid_rect(anchor)
        || !valid_rect(viewport)
        || !valid_rect(anchor.intersect(viewport))
        || viewport.width < MIN_TARGET
    {
        return None;
    }
    let width = PANEL_WIDTH.min(viewport.width);
    let x = match direction {
        LayoutDirection::LeftToRight => anchor.x,
        LayoutDirection::RightToLeft => anchor.x + anchor.width - width,
    }
    .clamp(
        viewport.x,
        (viewport.x + viewport.width - width).max(viewport.x),
    );
    let below = anchor.y + anchor.height + FIELD_GAP;
    let above = anchor.y - FIELD_GAP;
    let room_below = (viewport.y + viewport.height - below).max(0.);
    let room_above = (above - viewport.y).max(0.);
    if room_below >= PANEL_HEIGHT || room_below >= room_above {
        let height = PANEL_HEIGHT.min(room_below);
        (height >= MIN_PANEL_HEIGHT).then_some(Rect::new(x, below, width, height))
    } else {
        let height = PANEL_HEIGHT.min(room_above);
        (height >= MIN_PANEL_HEIGHT).then_some(Rect::new(x, above - height, width, height))
    }
}

fn valid_rect(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width > 0.
        && rect.height > 0.
        && (rect.x + rect.width).is_finite()
        && (rect.y + rect.height).is_finite()
}
