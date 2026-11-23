use std::cell::Cell;

use surface_material::{
    shape::LayoutDirection,
    CivilDate, CivilDateRange, DateBounds, DateInputError, DateInputFormat, DateInputOrder,
    DateInputSeparator, DatePickerConstraints, DatePickerLocale, DatePickerStep as Step,
    DateRangeConstraints as Policy, DateRangeEffect as Effect, DateRangeEndpoint as Endpoint,
    DateRangeEvent as Event, DateRangeMode as Mode, DateRangePairError as PairError,
    DateRangePickerState as State, DateRangePosition as Position,
    StepDirection::{Backward, Forward},
    Weekday,
};

fn day(year: u16, month: u8, day: u8) -> CivilDate {
    CivilDate::new(year, month, day).unwrap()
}

fn range(start: CivilDate, end: CivilDate) -> CivilDateRange {
    CivilDateRange::new(start, end).unwrap()
}

fn policy(first: CivilDate, last: CivilDate) -> Policy<'static> {
    Policy {
        dates: DatePickerConstraints {
            bounds: DateBounds::new(first, last).unwrap(),
            allowed_date: None,
        },
        allowed_span: None,
    }
}

fn all() -> Policy<'static> {
    policy(day(1, 1, 1), day(9999, 12, 31))
}

fn locale() -> DatePickerLocale<'static> {
    DatePickerLocale {
        direction: LayoutDirection::LeftToRight,
        first_weekday: Weekday::Sunday,
        input_format: DateInputFormat {
            order: DateInputOrder::YearMonthDay,
            separator: DateInputSeparator::Slash,
        },
        format_hint: "YYYY/MM/DD",
        weekday_short: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
        weekday_full: [
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
        ],
        month_title: &|month| format!("{}-{:02}", month.year(), month.month()),
        date_headline: &|date| format!("{:02}/{:02}", date.month(), date.day()),
        date_name: &|date| date.to_string(),
        error_text: &|error| format!("{error:?}"),
    }
}

fn fixture(mode: Mode) -> State {
    State::new(None, day(2026, 2, 18), all(), &locale(), mode)
}

fn edit(endpoint: Endpoint, text: &str) -> Event {
    Event::Edit(endpoint, text.to_owned())
}

#[test]
fn calendar_range_is_inclusive_and_restarts_on_earlier_or_completed_pick() {
    let mut state = fixture(Mode::Calendar);
    let policy = all();
    let locale = locale();
    let late = day(2025, 12, 31);
    let early = day(2025, 12, 30);
    let end = day(2026, 1, 2);
    let inside = day(2026, 1, 1);
    assert!(CivilDateRange::new(end, early).is_none());
    assert_eq!(
        state.apply(Event::SelectDay(late), policy, &locale),
        Effect::None
    );
    assert_eq!(
        state.validation(policy, &locale).position(late),
        Position::Start
    );
    assert_eq!(
        state.apply(Event::SelectDay(early), policy, &locale),
        Effect::None
    );
    assert_eq!(state.raw_input(Endpoint::Start), "2025/12/30");
    assert_eq!(state.raw_input(Endpoint::End), "");
    assert_eq!(
        state.apply(Event::SelectDay(end), policy, &locale),
        Effect::None
    );
    let complete = state.validation(policy, &locale);
    assert_eq!(complete.candidate(), Some(range(early, end)));
    assert_eq!(complete.position(early), Position::Start);
    assert_eq!(complete.position(inside), Position::Interior);
    assert_eq!(complete.position(end), Position::End);
    assert_eq!(complete.position(day(2026, 1, 3)), Position::None);

    assert_eq!(
        state.apply(Event::SelectDay(late), policy, &locale),
        Effect::None
    );
    assert_eq!(state.validation(policy, &locale).candidate(), None);
    assert_eq!(state.raw_input(Endpoint::End), "");
    assert_eq!(
        state.apply(Event::SelectDay(late), policy, &locale),
        Effect::None
    );
    let single = state.validation(policy, &locale);
    assert_eq!(single.candidate(), Some(range(late, late)));
    assert_eq!(single.position(late), Position::OnlyDay);
}

#[test]
fn calendar_allows_an_interior_block_by_default_and_a_span_rule_can_reject_it() {
    let blocked = day(2026, 2, 19);
    let first = day(2026, 2, 18);
    let last = day(2026, 2, 20);
    let allowed_date = |date| date != blocked;
    let allowed_span = |span: CivilDateRange| !span.contains(blocked);
    let endpoints_only = Policy {
        dates: DatePickerConstraints {
            allowed_date: Some(&allowed_date),
            ..all().dates
        },
        ..all()
    };
    let strict = Policy {
        allowed_span: Some(&allowed_span),
        ..endpoints_only
    };
    let mut state = fixture(Mode::Calendar);
    let locale = locale();
    state.apply(Event::SelectDay(first), endpoints_only, &locale);
    let before = state.clone();
    assert_eq!(
        state.apply(Event::SelectDay(blocked), endpoints_only, &locale),
        Effect::None
    );
    assert_eq!(state, before);
    state.apply(Event::SelectDay(last), endpoints_only, &locale);
    assert_eq!(
        state.validation(endpoints_only, &locale).position(blocked),
        Position::Interior
    );
    assert_eq!(
        state.validation(strict, &locale).pair_error(),
        Some(PairError::SpanUnavailable)
    );

    let mut state = fixture(Mode::Calendar);
    state.apply(Event::SelectDay(first), strict, &locale);
    state.apply(Event::SetMode(Mode::Input), strict, &locale);
    state.apply(edit(Endpoint::End, "pending end"), strict, &locale);
    state.apply(Event::SetMode(Mode::Calendar), strict, &locale);
    assert_eq!(
        state.apply(Event::SelectDay(last), strict, &locale),
        Effect::None
    );
    assert_eq!(state.active(), Some(last));
    assert_eq!(state.raw_input(Endpoint::Start), "2026/02/18");
    assert_eq!(state.raw_input(Endpoint::End), "pending end");
    assert_eq!(
        state.displayed_pair_error(state.validation(strict, &locale)),
        Some(PairError::SpanUnavailable)
    );
    assert_eq!(
        state.apply(Event::SelectDay(first), strict, &locale),
        Effect::None
    );
    assert_eq!(
        state.validation(strict, &locale).candidate(),
        Some(range(first, first))
    );
    assert_eq!(
        state.displayed_pair_error(state.validation(strict, &locale)),
        None
    );
}

#[test]
fn end_first_and_reverse_text_are_preserved_in_both_modes() {
    let locale = DatePickerLocale {
        input_format: DateInputFormat {
            order: DateInputOrder::DayMonthYear,
            separator: DateInputSeparator::Dot,
        },
        ..locale()
    };
    let mut state = State::new(None, day(2026, 2, 18), all(), &locale, Mode::Input);
    state.apply(edit(Endpoint::End, "２９／２／２０００"), all(), &locale);
    assert_eq!(state.raw_input(Endpoint::End), "２９／２／２０００");
    let end = day(2000, 2, 29);
    let pending = state.validation(all(), &locale);
    assert_eq!(pending.endpoint(Endpoint::End), Ok(end));
    assert_eq!(pending.position(end), Position::None);
    assert_eq!(
        state.apply(Event::FinishEditing(Endpoint::End), all(), &locale),
        Effect::None
    );
    assert_eq!(state.raw_input(Endpoint::End), "29.02.2000");
    state.apply(edit(Endpoint::Start, "1/3/2000"), all(), &locale);
    assert_eq!(state.apply(Event::Confirm, all(), &locale), Effect::None);
    assert_eq!(
        state.displayed_pair_error(state.validation(all(), &locale)),
        Some(PairError::Reversed)
    );

    for mode in [Mode::Calendar, Mode::Input] {
        state.apply(Event::SetMode(mode), all(), &locale);
        assert_eq!(state.mode(), mode);
        assert_eq!(state.raw_input(Endpoint::Start), "1/3/2000");
        assert_eq!(state.raw_input(Endpoint::End), "29.02.2000");
        assert_eq!(state.validation(all(), &locale).candidate(), None);
    }
    state.apply(edit(Endpoint::Start, "2/2/2000"), all(), &locale);
    assert_eq!(
        state.apply(Event::Confirm, all(), &locale),
        Effect::Commit(range(day(2000, 2, 2), end))
    );
    assert_eq!(state.raw_input(Endpoint::Start), "02.02.2000");
}

#[test]
fn field_errors_are_independent_and_only_one_field_is_formatted_on_blur() {
    let mut state = fixture(Mode::Input);
    let locale = locale();
    state.apply(edit(Endpoint::Start, "2100/2/29"), all(), &locale);
    state.apply(edit(Endpoint::End, "2026/2"), all(), &locale);
    let validation = state.validation(all(), &locale);
    assert_eq!(
        validation.endpoint(Endpoint::Start),
        Err(DateInputError::InvalidDate)
    );
    assert_eq!(
        validation.endpoint(Endpoint::End),
        Err(DateInputError::Incomplete)
    );
    assert_eq!(
        state.displayed_field_error(Endpoint::Start, validation),
        None
    );
    assert!(!state.observe_input_focus(Some(Endpoint::Start), all(), &locale));
    assert!(state.observe_input_focus(Some(Endpoint::End), all(), &locale));
    assert_eq!(
        state.displayed_field_error(Endpoint::Start, validation),
        Some(DateInputError::InvalidDate)
    );
    assert_eq!(state.displayed_field_error(Endpoint::End, validation), None);
    assert!(state.observe_input_focus(None, all(), &locale));
    assert_eq!(
        state.displayed_field_error(Endpoint::End, validation),
        Some(DateInputError::Incomplete)
    );
    assert!(!state.observe_input_focus(None, all(), &locale));

    state.apply(edit(Endpoint::Start, "2026-2-1"), all(), &locale);
    state.apply(edit(Endpoint::End, "2026-2-2"), all(), &locale);
    state.observe_input_focus(Some(Endpoint::Start), all(), &locale);
    assert!(state.observe_input_focus(Some(Endpoint::End), all(), &locale));
    assert_eq!(state.raw_input(Endpoint::Start), "2026/02/01");
    assert_eq!(state.raw_input(Endpoint::End), "2026-2-2");
    assert!(state.observe_input_focus(None, all(), &locale));
    assert_eq!(state.raw_input(Endpoint::End), "2026/02/02");
}

#[test]
fn only_valid_ordered_pairs_reach_span_policy() {
    let endpoints = Cell::new(0);
    let spans = Cell::new(0);
    let allow_date = |_| {
        endpoints.set(endpoints.get() + 1);
        true
    };
    let allow_span = |_| {
        spans.set(spans.get() + 1);
        true
    };
    let policy = Policy {
        dates: DatePickerConstraints {
            allowed_date: Some(&allow_date),
            ..policy(day(2026, 2, 1), day(2026, 2, 28)).dates
        },
        allowed_span: Some(&allow_span),
    };
    let locale = locale();
    let mut state = fixture(Mode::Input);
    state.apply(edit(Endpoint::Start, "2026/1/31"), policy, &locale);
    state.apply(edit(Endpoint::End, "2026/2/1"), policy, &locale);
    assert_eq!(state.validation(policy, &locale).candidate(), None);
    assert_eq!((endpoints.get(), spans.get()), (1, 0));

    state.apply(edit(Endpoint::Start, "2026/2/2"), policy, &locale);
    assert_eq!(
        state.validation(policy, &locale).pair_error(),
        Some(PairError::Reversed)
    );
    assert_eq!((endpoints.get(), spans.get()), (3, 0));
    state.apply(edit(Endpoint::End, "2026/2/3"), policy, &locale);
    assert_eq!(
        state.validation(policy, &locale).candidate(),
        Some(range(day(2026, 2, 2), day(2026, 2, 3)))
    );
    assert_eq!((endpoints.get(), spans.get()), (5, 1));
}

#[test]
fn confirm_rechecks_live_constraints_and_never_rewrites_invalid_endpoints() {
    let reject_span = Cell::new(false);
    let allowed_span = |_| !reject_span.get();
    let original = range(day(2026, 2, 12), day(2026, 2, 28));
    let initial = Policy {
        allowed_span: Some(&allowed_span),
        ..all()
    };
    let locale = locale();
    let mut state = State::new(
        Some(original),
        day(2026, 3, 1),
        initial,
        &locale,
        Mode::Calendar,
    );
    assert_eq!(
        state.validation(initial, &locale).candidate(),
        Some(original)
    );
    reject_span.set(true);
    assert_eq!(state.apply(Event::Confirm, initial, &locale), Effect::None);
    assert_eq!(
        state.displayed_pair_error(state.validation(initial, &locale)),
        Some(PairError::SpanUnavailable)
    );
    reject_span.set(false);

    let lower = day(2026, 3, 5);
    let tightened = policy(lower, day(2026, 3, 31));
    state.reconcile_constraints(tightened);
    assert_eq!(state.month(), lower.year_month());
    assert_eq!(state.active(), Some(lower));
    assert_eq!(state.raw_input(Endpoint::Start), "2026/02/12");
    assert_eq!(state.raw_input(Endpoint::End), "2026/02/28");
    assert_eq!(
        state.apply(Event::Confirm, tightened, &locale),
        Effect::None
    );
    let invalid = state.validation(tightened, &locale);
    for endpoint in [Endpoint::Start, Endpoint::End] {
        assert_eq!(
            state.displayed_field_error(endpoint, invalid),
            Some(DateInputError::OutsideBounds)
        );
    }
    state.reconcile_constraints(initial);
    assert_eq!(
        state.apply(Event::Confirm, initial, &locale),
        Effect::Commit(original)
    );
}

#[test]
fn owner_consumes_the_session_only_after_explicit_confirmation_or_cancel() {
    fn dispatch(
        session: &mut Option<State>,
        stored: &mut Option<CivilDateRange>,
        event: Event,
    ) -> Effect {
        let Some(state) = session else {
            return Effect::None;
        };
        let effect = state.apply(event, all(), &locale());
        match effect {
            Effect::None => {}
            Effect::Commit(range) => {
                *stored = Some(range);
                session.take();
            }
            Effect::Dismiss => {
                session.take();
            }
        }
        effect
    }

    let mut session = Some(fixture(Mode::Input));
    let mut stored = None;
    let mut send = |event| dispatch(&mut session, &mut stored, event);
    assert_eq!(send(Event::Confirm), Effect::None);
    assert_eq!(send(edit(Endpoint::Start, "2026-1-5")), Effect::None);
    assert_eq!(send(edit(Endpoint::End, "2026-1-6")), Effect::None);
    assert_eq!(send(Event::FinishEditing(Endpoint::Start)), Effect::None);
    assert_eq!(send(Event::FinishEditing(Endpoint::End)), Effect::None);
    let saved = range(day(2026, 1, 5), day(2026, 1, 6));
    assert_eq!(send(Event::Confirm), Effect::Commit(saved));
    assert_eq!(send(Event::Confirm), Effect::None);
    assert_eq!(stored, Some(saved));

    session = Some(State::new(
        stored,
        day(2026, 1, 1),
        all(),
        &locale(),
        Mode::Calendar,
    ));
    assert_eq!(
        dispatch(&mut session, &mut stored, Event::SelectDay(day(2026, 1, 8))),
        Effect::None
    );
    assert_eq!(
        dispatch(&mut session, &mut stored, Event::Cancel),
        Effect::Dismiss
    );
    assert_eq!(stored, Some(saved));
    assert!(session.is_none());
}

#[test]
fn bounded_focus_browse_and_exclusive_checkout_do_not_change_the_draft() {
    let first = day(1, 1, 1);
    let last = day(9999, 12, 31);
    let nights = |range: CivilDateRange| range.start() < range.end();
    let policy = Policy {
        allowed_span: Some(&nights),
        ..policy(first, last)
    };
    let locale = locale();
    let mut state = State::new(None, first, policy, &locale, Mode::Calendar);
    assert_eq!(state.active(), Some(first));
    state.apply(Event::SelectDay(first), policy, &locale);
    state.apply(Event::SelectDay(first), policy, &locale);
    assert_eq!(state.validation(policy, &locale).candidate(), None);
    state.apply(Event::BrowseTo(last.year_month()), policy, &locale);
    assert_eq!(state.active(), Some(day(9999, 12, 1)));
    state.apply(Event::FocusDay(last), policy, &locale);
    assert_eq!(state.active(), Some(last));
    assert_eq!(state.raw_input(Endpoint::Start), "0001/01/01");
    assert_eq!(state.raw_input(Endpoint::End), "");
    state.apply(Event::SelectDay(last), policy, &locale);
    assert_eq!(
        state.apply(Event::Confirm, policy, &locale),
        Effect::Commit(range(first, last))
    );
}

#[test]
fn mode_transitions_retire_editor_focus_and_ignore_stale_control_events() {
    let locale = locale();
    let mut state = fixture(Mode::Input);
    state.apply(edit(Endpoint::Start, "2026-2-18"), all(), &locale);
    state.observe_input_focus(Some(Endpoint::Start), all(), &locale);
    let snapshot = state.clone();
    state.apply(Event::SelectDay(day(2026, 2, 21)), all(), &locale);
    assert_eq!(state, snapshot);
    state.apply(Event::SetMode(Mode::Calendar), all(), &locale);
    assert!(!state.observe_input_focus(None, all(), &locale));
    let snapshot = state.clone();
    state.apply(edit(Endpoint::End, "2026-2-19"), all(), &locale);
    state.apply(Event::FinishEditing(Endpoint::Start), all(), &locale);
    assert_eq!(state, snapshot);
    assert_eq!(state.raw_input(Endpoint::Start), "2026-2-18");
}

#[test]
fn an_empty_legal_month_keeps_no_active_day_and_retains_the_saved_text() {
    let saved = range(day(2026, 2, 18), day(2026, 2, 20));
    let bounds = policy(day(2026, 1, 1), day(2026, 1, 31));
    let empty = |_| false;
    let disabled = Policy {
        dates: DatePickerConstraints {
            allowed_date: Some(&empty),
            ..bounds.dates
        },
        ..bounds
    };
    let locale = locale();
    let mut state = State::new(
        Some(saved),
        day(2026, 2, 18),
        disabled,
        &locale,
        Mode::Calendar,
    );
    assert_eq!(state.month(), day(2026, 1, 1).year_month());
    assert_eq!(state.active(), None);
    assert_eq!(state.validation(disabled, &locale).candidate(), None);
    assert_eq!(state.raw_input(Endpoint::Start), "2026/02/18");
    assert_eq!(state.raw_input(Endpoint::End), "2026/02/20");
    state.reconcile_constraints(bounds);
    assert_eq!(state.active(), Some(day(2026, 1, 1)));
    assert_eq!(state.raw_input(Endpoint::Start), "2026/02/18");
    assert_eq!(state.raw_input(Endpoint::End), "2026/02/20");
}

fn move_focus(
    state: &mut State,
    origin: CivilDate,
    step: Step,
    expected: CivilDate,
    policy: Policy<'_>,
    locale: &DatePickerLocale<'_>,
) {
    assert_eq!(
        state.navigation_target_from(origin, step, policy, locale),
        Some(expected)
    );
    assert_eq!(
        state.apply(Event::NavigateFrom(origin, step), policy, locale),
        Effect::None
    );
    assert_eq!(state.active(), Some(expected));
    assert_eq!(state.month(), expected.year_month());
}

#[test]
fn adjacent_month_focus_supplies_the_paging_origin_without_editing_the_draft() {
    let locale = locale();
    let policy = all();
    let start = day(2026, 1, 31);
    let end = day(2026, 2, 11);
    let saved = range(start, end);
    let mut state = State::new(Some(saved), start, policy, &locale, Mode::Calendar);
    let february = day(2026, 2, 28);
    let march = day(2026, 3, 31);
    let page = Step::Month(Forward);

    move_focus(&mut state, start, page, february, policy, &locale);
    state.apply(Event::FocusDay(february), policy, &locale);
    move_focus(&mut state, february, page, march, policy, &locale);

    // A direct AX focus can name a mounted date outside the model month.
    let focus = day(2026, 2, 14);
    let next = day(2026, 3, 14);
    move_focus(&mut state, focus, page, next, policy, &locale);
    move_focus(&mut state, next, page, day(2026, 4, 14), policy, &locale);
    assert_eq!(state.raw_input(Endpoint::Start), "2026/01/31");
    assert_eq!(state.raw_input(Endpoint::End), "2026/02/11");
    assert_eq!(state.validation(policy, &locale).candidate(), Some(saved));
}

#[test]
fn year_paging_recovers_leap_day_and_a_day_step_resets_the_preference() {
    let locale = locale();
    let policy = all();
    let start = day(2024, 2, 29);
    let mut state = State::new(None, start, policy, &locale, Mode::Calendar);
    let next = day(2025, 2, 28);
    move_focus(
        &mut state,
        start,
        Step::Year(Forward),
        next,
        policy,
        &locale,
    );
    move_focus(
        &mut state,
        next,
        Step::Year(Backward),
        start,
        policy,
        &locale,
    );

    let previous = day(2024, 2, 28);
    move_focus(
        &mut state,
        start,
        Step::Day(Backward),
        previous,
        policy,
        &locale,
    );
    let page = Step::Month(Forward);
    assert_eq!(
        state.navigation_target_from(previous, page, policy, &locale),
        Some(day(2024, 3, 28))
    );
}

#[test]
fn week_edges_follow_the_locale_even_from_a_neighbor_month() {
    let mut locale = locale();
    let policy = all();
    let current = day(2026, 1, 31);
    let origin = day(2026, 2, 1);
    let mut state = State::new(None, current, policy, &locale, Mode::Calendar);

    locale.first_weekday = Weekday::Monday;
    let monday = day(2026, 1, 26);
    move_focus(&mut state, origin, Step::WeekStart, monday, policy, &locale);
    assert_eq!(
        state.navigation_target_from(origin, Step::WeekEnd, policy, &locale),
        Some(origin)
    );
    locale.first_weekday = Weekday::Sunday;
    assert_eq!(
        state.navigation_target_from(origin, Step::WeekEnd, policy, &locale),
        Some(day(2026, 2, 7))
    );
}

#[test]
fn day_navigation_crosses_unavailable_dates_on_both_sides_of_a_month_edge() {
    let locale = locale();
    let february = day(2024, 2, 28);
    let march = day(2024, 3, 2);
    let unavailable = [day(2024, 2, 29), day(2024, 3, 1)];
    let eligible = |date| !unavailable.contains(&date);
    let mut policy = all();
    policy.dates.allowed_date = Some(&eligible);
    let saved = range(february, march);
    let mut state = State::new(Some(saved), february, policy, &locale, Mode::Calendar);

    move_focus(
        &mut state,
        february,
        Step::Day(Forward),
        march,
        policy,
        &locale,
    );
    move_focus(
        &mut state,
        march,
        Step::Day(Backward),
        february,
        policy,
        &locale,
    );
    assert_eq!(state.validation(policy, &locale).candidate(), Some(saved));
}

#[test]
fn unavailable_origin_can_recover_without_scanning_across_an_empty_month() {
    let locale = locale();
    let bounds = policy(day(2026, 1, 20), day(2026, 3, 10));
    let origin = day(2026, 1, 29);
    let allowed = |date: CivilDate| date != origin && date.month() != 2;
    let no_span_check = |_| panic!("navigation only checks date availability");
    let policy = Policy {
        dates: DatePickerConstraints {
            allowed_date: Some(&allowed),
            ..bounds.dates
        },
        allowed_span: Some(&no_span_check),
    };
    let mut state = State::new(None, origin, bounds, &locale, Mode::Calendar);
    let next = day(2026, 1, 30);
    move_focus(
        &mut state,
        origin,
        Step::Day(Forward),
        next,
        policy,
        &locale,
    );

    let snapshot = state.clone();
    let last_in_january = day(2026, 1, 31);
    for step in [Step::Day(Forward), Step::Month(Forward)] {
        assert_eq!(
            state.navigation_target_from(last_in_january, step, policy, &locale),
            None
        );
        assert_eq!(
            state.apply(Event::NavigateFrom(last_in_january, step), policy, &locale),
            Effect::None
        );
        assert_eq!(state, snapshot);
    }

    let outside = day(2026, 1, 19);
    assert_eq!(
        state.navigation_target_from(outside, Step::Day(Forward), policy, &locale),
        None
    );
}

#[test]
fn stale_navigation_does_not_mutate_input_mode_or_reach_beyond_supported_years() {
    let locale = locale();
    let policy = all();
    let first = day(1, 1, 1);
    let last = day(9999, 12, 31);
    let mut state = State::new(None, first, policy, &locale, Mode::Input);
    let snapshot = state.clone();
    let forward = Step::Day(Forward);
    assert_eq!(
        state.navigation_target_from(first, forward, policy, &locale),
        None
    );
    assert_eq!(
        state.apply(Event::NavigateFrom(first, forward), policy, &locale),
        Effect::None
    );
    state.apply(Event::SetVisibleMonth(last.year_month()), policy, &locale);
    assert_eq!(state, snapshot);

    state.apply(Event::SetMode(Mode::Calendar), policy, &locale);
    for (origin, direction) in [(first, Backward), (last, Forward)] {
        for step in [
            Step::Day(direction),
            Step::Week(direction),
            Step::Month(direction),
            Step::Year(direction),
        ] {
            assert_eq!(
                state.navigation_target_from(origin, step, policy, &locale),
                None
            );
        }
    }
}

#[test]
fn passive_scroll_keeps_focus_and_raw_draft_while_explicit_browse_changes_focus() {
    let locale = locale();
    let start = day(2026, 1, 31);
    let saved = range(start, day(2026, 2, 11));
    let bounds = policy(start, day(2026, 4, 30));
    let mut state = State::new(Some(saved), start, bounds, &locale, Mode::Calendar);
    let february = day(2026, 2, 28);
    let march = day(2026, 3, 31);
    let april = day(2026, 4, 1);
    let page = Step::Month(Forward);

    move_focus(&mut state, start, page, february, bounds, &locale);
    assert_eq!(
        state.apply(Event::SetVisibleMonth(march.year_month()), bounds, &locale),
        Effect::None
    );
    state.apply(Event::FocusDay(february), bounds, &locale);
    state.reconcile_constraints(bounds);
    assert_eq!(state.month(), march.year_month());
    assert_eq!(state.active(), Some(february));
    assert_eq!(
        state.navigation_target_from(february, page, bounds, &locale),
        Some(march)
    );

    let snapshot = state.clone();
    let outside = day(2026, 5, 1).year_month();
    state.apply(Event::SetVisibleMonth(outside), bounds, &locale);
    assert_eq!(state, snapshot);
    state.apply(Event::BrowseTo(april.year_month()), bounds, &locale);
    assert_eq!(state.month(), april.year_month());
    assert_eq!(state.active(), Some(april));
    assert_eq!(state.raw_input(Endpoint::Start), "2026/01/31");
    assert_eq!(state.raw_input(Endpoint::End), "2026/02/11");
    assert_eq!(state.validation(bounds, &locale).candidate(), Some(saved));
}

#[test]
fn calendar_mode_entry_reveals_a_pair_error_without_rewriting_unfinished_fields() {
    let locale = locale();
    let mut state = fixture(Mode::Input);
    state.apply(edit(Endpoint::Start, "2026-2-20"), all(), &locale);
    state.apply(edit(Endpoint::End, "2026-2-18"), all(), &locale);
    let invalid = state.validation(all(), &locale);
    assert_eq!(state.displayed_pair_error(invalid), None);

    assert_eq!(
        state.apply(Event::SetMode(Mode::Calendar), all(), &locale),
        Effect::None
    );
    assert_eq!(state.active(), Some(day(2026, 2, 20)));
    assert_eq!(
        state.displayed_pair_error(invalid),
        Some(PairError::Reversed)
    );
    assert_eq!(state.raw_input(Endpoint::Start), "2026-2-20");
    assert_eq!(state.raw_input(Endpoint::End), "2026-2-18");
}
