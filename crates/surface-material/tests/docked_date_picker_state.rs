use surface::Rect;
use surface_material::{
    place_docked_date_picker, shape::LayoutDirection, CivilDate, DateBounds, DateInputError,
    DateInputFormat, DateInputOrder, DateInputSeparator, DatePickerConstraints, DatePickerEffect,
    DatePickerEvent, DatePickerLocale, DatePickerMode, DockedDatePickerEvent as Event,
    DockedDatePickerMode as Mode, DockedDatePickerState as State, StepDirection, Weekday,
    YearMonth,
};

fn date(year: u16, month: u8, day: u8) -> CivilDate {
    CivilDate::new(year, month, day).unwrap()
}

fn today() -> CivilDate {
    date(2026, 2, 18)
}

fn constraints(min: CivilDate, max: CivilDate) -> DatePickerConstraints<'static> {
    DatePickerConstraints {
        bounds: DateBounds::new(min, max).unwrap(),
        allowed_date: None,
    }
}

fn all_dates() -> DatePickerConstraints<'static> {
    constraints(date(1, 1, 1), date(9999, 12, 31))
}

fn locale() -> DatePickerLocale<'static> {
    DatePickerLocale {
        direction: LayoutDirection::LeftToRight,
        first_weekday: Weekday::Sunday,
        input_format: DateInputFormat {
            order: DateInputOrder::YearMonthDay,
            separator: DateInputSeparator::Slash,
        },
        format_hint: "年/月/日",
        weekday_short: ["一", "二", "三", "四", "五", "六", "日"],
        weekday_full: [
            "星期一",
            "星期二",
            "星期三",
            "星期四",
            "星期五",
            "星期六",
            "星期日",
        ],
        month_title: &|month| format!("{}年{}月", month.year(), month.month()),
        date_headline: &|date| format!("{}月{}日", date.month(), date.day()),
        date_name: &|date| date.to_string(),
        error_text: &|error| format!("{error:?}"),
    }
}

fn apply(state: &mut State, event: Event, policy: DatePickerConstraints<'_>) -> DatePickerEffect {
    state.apply(event, today(), policy, &locale())
}

fn observe(
    state: &mut State,
    focused: bool,
    policy: DatePickerConstraints<'_>,
) -> (bool, DatePickerEffect) {
    state.observe_input_focus(focused, policy, &locale())
}

#[test]
fn direct_input_commits_full_width_leap_date_only_at_the_enter_boundary() {
    let policy = all_dates();
    let mut state = State::new(Some(today()), today(), policy, &locale());
    let leap_day = date(2000, 2, 29);
    assert_eq!(
        apply(&mut state, Event::Edit("２０００／２／２９".into()), policy),
        DatePickerEffect::None
    );
    assert_eq!(state.raw_input(), "２０００／２／２９");
    assert_eq!(state.committed(), Some(today()));
    assert_eq!(state.mode(), None);
    assert_eq!(
        apply(&mut state, Event::FinishEditing, policy),
        DatePickerEffect::Commit(leap_day)
    );
    assert_eq!(state.raw_input(), "2000/02/29");
    assert_eq!(state.committed(), Some(leap_day));
    assert_eq!(state.error(), None);
    assert_eq!(state.mode(), None);
    assert_eq!(
        apply(&mut state, Event::FinishEditing, policy),
        DatePickerEffect::None
    );

    apply(&mut state, Event::Edit("2100/2/29".into()), policy);
    assert_eq!(state.error(), None);
    assert_eq!(
        apply(&mut state, Event::FinishEditing, policy),
        DatePickerEffect::None
    );
    assert_eq!(state.error(), Some(DateInputError::InvalidDate));
    assert_eq!(state.committed(), Some(leap_day));
    assert_eq!(state.raw_input(), "2100/2/29");
}

#[test]
fn ordinary_field_exit_commits_a_changed_canonical_value_once() {
    let policy = all_dates();
    let mut state = State::new(None, today(), policy, &locale());
    assert_eq!(
        observe(&mut state, true, policy),
        (false, DatePickerEffect::None)
    );
    assert_eq!(
        observe(&mut state, false, policy),
        (false, DatePickerEffect::None)
    );
    assert_eq!(state.error(), None);

    observe(&mut state, true, policy);
    apply(&mut state, Event::Edit("2026/03/09".into()), policy);
    let march = date(2026, 3, 9);
    assert_eq!(
        observe(&mut state, false, policy),
        (true, DatePickerEffect::Commit(march))
    );
    assert_eq!(state.raw_input(), "2026/03/09");
    assert_eq!(
        observe(&mut state, false, policy),
        (false, DatePickerEffect::None)
    );

    observe(&mut state, true, policy);
    apply(&mut state, Event::Edit("2026-3-9".into()), policy);
    assert_eq!(
        observe(&mut state, false, policy),
        (true, DatePickerEffect::None)
    );
    assert_eq!(state.raw_input(), "2026/03/09");

    observe(&mut state, true, policy);
    apply(&mut state, Event::Edit("2026/03".into()), policy);
    assert_eq!(
        observe(&mut state, false, policy),
        (true, DatePickerEffect::None)
    );
    assert_eq!(state.error(), Some(DateInputError::Incomplete));
    assert_eq!(state.committed(), Some(march));
    assert_eq!(state.raw_input(), "2026/03");
}

#[test]
fn popup_open_close_and_dismissal_disarm_in_flight_field_blur() {
    let policy = all_dates();
    let old = today();
    let mut state = State::new(Some(old), today(), policy, &locale());
    observe(&mut state, true, policy);
    apply(&mut state, Event::Edit("2000-2-29".into()), policy);
    assert_eq!(
        apply(&mut state, Event::ToggleCalendar, policy),
        DatePickerEffect::None
    );
    assert_eq!(state.mode(), Some(Mode::Calendar));
    assert_eq!(state.calendar().active(), Some(old));
    // The anchor may need a frame before the popup moves focus. Seeing the
    // field still focused must not re-arm blur during that interval.
    assert_eq!(
        observe(&mut state, true, policy),
        (false, DatePickerEffect::None)
    );
    assert_eq!(
        observe(&mut state, false, policy),
        (false, DatePickerEffect::None)
    );
    assert_eq!(state.raw_input(), "2000-2-29");
    assert_eq!(state.committed(), Some(old));

    observe(&mut state, true, policy);
    apply(&mut state, Event::Edit("2000-3-1".into()), policy);
    assert_eq!(
        apply(&mut state, Event::Dismiss, policy),
        DatePickerEffect::Dismiss
    );
    assert_eq!(
        observe(&mut state, false, policy),
        (false, DatePickerEffect::None)
    );
    assert_eq!(state.mode(), None);
    assert_eq!(state.committed(), Some(old));

    apply(&mut state, Event::ToggleCalendar, policy);
    observe(&mut state, true, policy);
    apply(&mut state, Event::Edit("2000-3-2".into()), policy);
    assert_eq!(
        apply(&mut state, Event::ToggleCalendar, policy),
        DatePickerEffect::Dismiss
    );
    assert_eq!(
        observe(&mut state, false, policy),
        (false, DatePickerEffect::None)
    );
    assert_eq!(state.committed(), Some(old));
    assert_eq!(state.raw_input(), "2000-3-2");

    // A new focus visit is an ordinary edit boundary again.
    observe(&mut state, true, policy);
    assert_eq!(
        observe(&mut state, false, policy),
        (true, DatePickerEffect::Commit(date(2000, 3, 2)))
    );
}

#[test]
fn calendar_selection_commits_once_and_can_repair_invalid_input() {
    let blocked = date(2026, 2, 21);
    let allows = |date| date != blocked;
    let policy = DatePickerConstraints {
        allowed_date: Some(&allows),
        ..all_dates()
    };
    let old = today();
    let mut state = State::new(Some(old), today(), policy, &locale());
    apply(&mut state, Event::Edit("2026/02/30".into()), policy);
    apply(&mut state, Event::FinishEditing, policy);
    apply(&mut state, Event::ToggleCalendar, policy);
    assert_eq!(state.calendar().active(), Some(old));
    assert_eq!(
        apply(
            &mut state,
            Event::Calendar(DatePickerEvent::Select(blocked)),
            policy
        ),
        DatePickerEffect::None
    );
    assert_eq!(
        apply(
            &mut state,
            Event::Calendar(DatePickerEvent::Select(date(2026, 3, 1))),
            policy
        ),
        DatePickerEffect::None
    );
    assert_eq!(state.committed(), Some(old));
    assert_eq!(state.mode(), Some(Mode::Calendar));
    assert_eq!(state.error(), Some(DateInputError::InvalidDate));

    let target = date(2026, 2, 20);
    let event = Event::Calendar(DatePickerEvent::Select(target));
    assert_eq!(
        apply(&mut state, event.clone(), policy),
        DatePickerEffect::Commit(target)
    );
    assert_eq!(state.committed(), Some(target));
    assert_eq!(state.raw_input(), "2026/02/20");
    assert_eq!(state.error(), None);
    assert_eq!(state.mode(), None);
    assert_eq!(
        apply(&mut state, event.clone(), policy),
        DatePickerEffect::None
    );
    apply(&mut state, Event::ToggleCalendar, policy);
    assert_eq!(apply(&mut state, event, policy), DatePickerEffect::Dismiss);
    assert_eq!(state.mode(), None);
}

#[test]
fn input_commit_in_an_open_picker_keeps_it_open_and_updates_selection() {
    let policy = all_dates();
    let mut state = State::new(Some(today()), today(), policy, &locale());
    apply(&mut state, Event::ToggleCalendar, policy);
    observe(&mut state, true, policy);
    apply(&mut state, Event::Edit("2024-2-29".into()), policy);
    let typed = date(2024, 2, 29);
    assert_eq!(
        apply(&mut state, Event::FinishEditing, policy),
        DatePickerEffect::Commit(typed)
    );
    assert_eq!(state.mode(), Some(Mode::Calendar));
    assert_eq!(state.calendar().draft(), Some(typed));
    assert_eq!(state.calendar().month(), typed.year_month());
    assert_eq!(state.raw_input(), "2024/02/29");
    assert_eq!(
        observe(&mut state, false, policy),
        (false, DatePickerEffect::None)
    );
}

#[test]
fn open_picker_focus_movement_preserves_typed_draft_until_a_day_is_chosen() {
    let policy = all_dates();
    let old = today();
    let mut state = State::new(Some(old), old, policy, &locale());
    apply(&mut state, Event::ToggleCalendar, policy);
    observe(&mut state, true, policy);
    apply(&mut state, Event::Edit("2000-2-29".into()), policy);
    assert_eq!(
        observe(&mut state, false, policy),
        (false, DatePickerEffect::None)
    );
    assert_eq!(state.mode(), Some(Mode::Calendar));
    assert_eq!(state.committed(), Some(old));
    assert_eq!(state.raw_input(), "2000-2-29");

    let target = date(2026, 2, 20);
    assert_eq!(
        apply(
            &mut state,
            Event::Calendar(DatePickerEvent::Select(target)),
            policy
        ),
        DatePickerEffect::Commit(target)
    );
    assert_eq!(state.raw_input(), "2026/02/20");
    assert_eq!(state.mode(), None);
}

#[test]
fn month_and_year_browsing_do_not_commit_and_dismiss_unwinds_the_list_first() {
    let first = date(2025, 12, 16);
    let last = date(2027, 2, 5);
    let policy = constraints(first, last);
    let mut state = State::new(Some(today()), today(), policy, &locale());
    apply(&mut state, Event::SetMode(Mode::Month), policy);
    assert_eq!(state.mode(), None);
    apply(&mut state, Event::ToggleCalendar, policy);
    apply(&mut state, Event::SetMode(Mode::Month), policy);
    let june = YearMonth::new(2026, 6).unwrap();
    assert_eq!(
        apply(&mut state, Event::BrowseTo(june), policy),
        DatePickerEffect::None
    );
    assert_eq!(state.calendar().month(), june);
    assert_eq!(state.mode(), Some(Mode::Calendar));
    assert_eq!(state.committed(), Some(today()));

    apply(&mut state, Event::SetMode(Mode::Year), policy);
    apply(
        &mut state,
        Event::Calendar(DatePickerEvent::BrowseYears(2027)),
        policy,
    );
    assert_eq!(state.calendar().month(), last.year_month());
    assert_eq!(state.mode(), Some(Mode::Year));
    assert_eq!(
        apply(
            &mut state,
            Event::BrowseTo(YearMonth::new(2028, 1).unwrap()),
            policy
        ),
        DatePickerEffect::None
    );
    assert_eq!(state.mode(), Some(Mode::Year));
    apply(
        &mut state,
        Event::Calendar(DatePickerEvent::ChooseYear(2025)),
        policy,
    );
    assert_eq!(state.calendar().month(), first.year_month());
    assert_eq!(state.mode(), Some(Mode::Calendar));
    assert_eq!(state.calendar().mode(), DatePickerMode::Calendar);
    assert_eq!(state.committed(), Some(today()));

    apply(&mut state, Event::SetMode(Mode::Year), policy);
    assert_eq!(
        apply(&mut state, Event::Dismiss, policy),
        DatePickerEffect::None
    );
    assert_eq!(state.mode(), Some(Mode::Calendar));
    assert_eq!(
        apply(&mut state, Event::Dismiss, policy),
        DatePickerEffect::Dismiss
    );
    assert_eq!(state.mode(), None);
    assert_eq!(state.committed(), Some(today()));
}

#[test]
fn incompatible_calendar_events_leave_field_and_popup_state_alone() {
    let policy = all_dates();
    let mut state = State::new(Some(today()), today(), policy, &locale());
    apply(&mut state, Event::ToggleCalendar, policy);
    for event in [
        DatePickerEvent::SetMode(DatePickerMode::Input),
        DatePickerEvent::Edit("2026/3/1".into()),
        DatePickerEvent::FinishEditing,
        DatePickerEvent::Confirm,
        DatePickerEvent::Cancel,
        DatePickerEvent::ChooseYear(2000),
    ] {
        let before = state.clone();
        assert_eq!(
            apply(&mut state, Event::Calendar(event), policy),
            DatePickerEffect::None
        );
        assert_eq!(state, before);
    }
    apply(
        &mut state,
        Event::Calendar(DatePickerEvent::BrowseMonth(StepDirection::Forward)),
        policy,
    );
    assert_eq!(state.calendar().month(), YearMonth::new(2026, 3).unwrap());
    assert_eq!(state.committed(), Some(today()));
}

#[test]
fn placement_prefers_below_flips_above_and_mirrors_logical_start() {
    let viewport = Rect::new(0., 0., 960., 800.);
    let ltr = LayoutDirection::LeftToRight;
    let rtl = LayoutDirection::RightToLeft;
    assert_eq!(
        place_docked_date_picker(Rect::new(100., 64., 260., 56.), viewport, ltr),
        Some(Rect::new(100., 124., 360., 456.))
    );
    assert_eq!(
        place_docked_date_picker(Rect::new(600., 708., 200., 56.), viewport, ltr),
        Some(Rect::new(600., 248., 360., 456.))
    );
    assert_eq!(
        place_docked_date_picker(
            Rect::new(620., 60., 96., 56.),
            Rect::new(10., 20., 720., 700.),
            rtl,
        ),
        Some(Rect::new(356., 120., 360., 456.))
    );
}

#[test]
fn short_or_clipped_placement_never_covers_the_editable_field() {
    let ltr = LayoutDirection::LeftToRight;
    let short = Rect::new(0., 0., 320., 278.);
    let field = Rect::new(16., 140., 288., 56.);
    let popup = place_docked_date_picker(field, short, ltr).unwrap();
    assert_eq!(popup, Rect::new(0., 0., 320., 136.));
    assert_eq!(popup.intersect(field).height, 0.);
    assert_eq!(
        place_docked_date_picker(
            Rect::new(16., 140., 288., 56.),
            Rect::new(0., 0., 320., 336.),
            ltr,
        ),
        Some(Rect::new(0., 200., 320., 136.))
    );
    let clipped = Rect::new(-20., 290., 100., 56.);
    assert_eq!(
        place_docked_date_picker(clipped, Rect::new(0., 0., 320., 320.), ltr),
        Some(Rect::new(0., 0., 320., 286.))
    );
    assert_eq!(
        place_docked_date_picker(
            Rect::new(16., 44., 288., 56.),
            Rect::new(0., 0., 320., 144.),
            ltr,
        ),
        None
    );
    for anchor in [
        Rect::new(80., -56., 100., 56.),
        Rect::new(320., 30., 100., 56.),
        Rect::new(10., 30., 0., 56.),
        Rect::new(10., f32::NAN, 100., 56.),
    ] {
        assert_eq!(place_docked_date_picker(anchor, short, ltr), None);
    }
}

#[test]
fn placement_requires_room_for_the_header_and_a_complete_date_target() {
    let ltr = LayoutDirection::LeftToRight;
    let field = Rect::new(0., 0., 48., 56.);
    assert_eq!(
        place_docked_date_picker(field, Rect::new(0., 0., 48., 171.), ltr),
        None
    );
    assert_eq!(
        place_docked_date_picker(field, Rect::new(0., 0., 48., 172.), ltr),
        Some(Rect::new(0., 60., 48., 112.))
    );
    assert_eq!(
        place_docked_date_picker(field, Rect::new(0., 0., 47., 172.), ltr),
        None
    );

    let bottom = Rect::new(0., 116., 320., 56.);
    assert_eq!(
        place_docked_date_picker(bottom, Rect::new(0., 0., 320., 172.), ltr),
        Some(Rect::new(0., 0., 320., 112.))
    );
    let bottom = Rect::new(0., 115., 320., 56.);
    assert_eq!(
        place_docked_date_picker(bottom, Rect::new(0., 0., 320., 172.), ltr),
        None
    );
}
