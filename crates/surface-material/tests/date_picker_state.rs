use surface_material::{
    shape::LayoutDirection, CivilDate, DateBounds, DateInputError, DateInputFormat, DateInputOrder,
    DateInputSeparator, DatePickerConstraints, DatePickerEffect, DatePickerEvent, DatePickerLocale,
    DatePickerMode, DatePickerSession, DatePickerStep, StepDirection, Weekday, YearMonth,
};

fn date(year: u16, month: u8, day: u8) -> CivilDate {
    CivilDate::new(year, month, day).unwrap()
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
        first_weekday: Weekday::Monday,
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

#[test]
fn gregorian_arithmetic_validates_centuries_days_and_year_limits() {
    for year in [1900, 2100] {
        assert!(CivilDate::new(year, 2, 29).is_none());
        assert_eq!(
            date(year, 3, 1).checked_add_days(-1),
            Some(date(year, 2, 28))
        );
    }
    assert_eq!(
        date(2000, 3, 1).checked_add_days(-1),
        Some(date(2000, 2, 29))
    );
    assert_eq!(date(2000, 2, 29).weekday(), Weekday::Tuesday);
    assert_eq!(date(1970, 1, 1).weekday(), Weekday::Thursday);
    assert_eq!(date(1, 1, 1).weekday(), Weekday::Monday);
    assert_eq!(
        date(2025, 12, 31).checked_add_days(1),
        Some(date(2026, 1, 1))
    );
    assert_eq!(
        date(2026, 1, 1).checked_add_days(-1),
        Some(date(2025, 12, 31))
    );
    assert_eq!(date(1, 1, 1).checked_add_days(-1), None);
    assert_eq!(date(9999, 12, 31).checked_add_days(1), None);
    assert_eq!(date(2026, 1, 1).checked_add_days(i32::MAX), None);
    assert_eq!(date(2026, 1, 1).checked_add_days(i32::MIN), None);
    assert_eq!(date(1, 2, 3).to_string(), "0001-02-03");

    assert!(CivilDate::new(0, 1, 1).is_none());
    assert!(CivilDate::new(2026, 1, 0).is_none());
    assert!(CivilDate::new(2026, 4, 31).is_none());
    assert!(YearMonth::new(10000, 1).is_none());
    assert!(YearMonth::new(2026, 13).is_none());
    assert_eq!(date(1, 1, 1).year_month().step(-1), None);
    assert_eq!(date(9999, 12, 31).year_month().step(1), None);
    assert_eq!(date(2026, 1, 1).year_month().step(i32::MAX), None);
    let february = date(2026, 1, 31).year_month().step(1).unwrap();
    assert_eq!(february.day_clamped(31), date(2026, 2, 28));
    assert_eq!(february.day_clamped(0), date(2026, 2, 1));
    assert_eq!(february.step(-2), YearMonth::new(2025, 12));
}

#[test]
fn calendar_slots_rotate_the_first_weekday_without_leaking_adjacent_months() {
    assert_eq!(Weekday::Monday.index(), 0);
    assert_eq!(Weekday::Sunday.index(), 6);
    let month = YearMonth::new(2026, 2).unwrap();
    let sunday = month.calendar_days(Weekday::Sunday);
    let monday = month.calendar_days(Weekday::Monday);
    assert_eq!(sunday[0], Some(date(2026, 2, 1)));
    assert_eq!(sunday[27], Some(date(2026, 2, 28)));
    assert!(sunday[28..].iter().all(Option::is_none));
    assert!(monday[..6].iter().all(Option::is_none));
    assert_eq!(monday[6], Some(date(2026, 2, 1)));
    assert_eq!(monday[33], Some(date(2026, 2, 28)));
    assert!(monday[34..].iter().all(Option::is_none));
    let six_weeks = YearMonth::new(2025, 3)
        .unwrap()
        .calendar_days(Weekday::Sunday);
    assert_eq!(six_weeks[36], Some(date(2025, 3, 31)));
}

#[test]
fn input_parses_declared_order_and_formats_only_with_the_canonical_separator() {
    let ymd = locale().input_format;
    let us = DateInputFormat {
        order: DateInputOrder::MonthDayYear,
        separator: DateInputSeparator::Dash,
    };
    let day_first = DateInputFormat {
        order: DateInputOrder::DayMonthYear,
        separator: DateInputSeparator::Dot,
    };
    let target = date(2026, 4, 5);
    assert_eq!(ymd.parse(" ２０２６／４．５ "), Ok(target));
    assert_eq!(ymd.parse("2026 - 4 5"), Ok(target));
    assert_eq!(day_first.parse("5/4/2026"), Ok(target));
    assert_eq!(us.parse("4 5 2026"), Ok(target));
    assert_eq!(ymd.format(target), "2026/04/05");
    assert_eq!(us.format(target), "04-05-2026");
    assert_eq!(day_first.format(target), "05.04.2026");
    assert_eq!(
        DateInputFormat {
            separator: DateInputSeparator::Space,
            ..ymd
        }
        .format(target),
        "2026 04 05"
    );

    for (text, error) in [
        ("", DateInputError::Required),
        ("2026/04/", DateInputError::Incomplete),
        ("2026/04", DateInputError::Incomplete),
        ("26/4/5", DateInputError::InvalidFormat),
        ("2026/4/005", DateInputError::InvalidFormat),
        ("2026--4-5", DateInputError::InvalidFormat),
        ("2026/4/5/6", DateInputError::InvalidFormat),
        ("2026/February/5", DateInputError::InvalidFormat),
        ("2026/2/29", DateInputError::InvalidDate),
        ("0000/1/1", DateInputError::InvalidDate),
    ] {
        assert_eq!(ymd.parse(text), Err(error), "{text:?}");
    }
}

#[test]
fn initial_focus_uses_the_saved_date_today_or_a_legal_in_month_fallback() {
    let lower = date(2026, 2, 12);
    let upper = date(2026, 3, 1);
    let allowed = |date: CivilDate| date.day() % 2 == 1;
    let policy = DatePickerConstraints {
        bounds: DateBounds::new(lower, upper).unwrap(),
        allowed_date: Some(&allowed),
    };
    let locale = locale();
    let committed = date(2026, 2, 19);
    let saved = DatePickerSession::new(
        Some(committed),
        date(2026, 2, 17),
        policy,
        &locale,
        DatePickerMode::Calendar,
    );
    assert_eq!(saved.active(), Some(committed));
    assert_eq!(saved.draft(), Some(committed));
    assert_eq!(saved.raw_input(), "2026/02/19");
    let today = DatePickerSession::new(
        Some(date(2026, 2, 20)),
        date(2026, 2, 17),
        policy,
        &locale,
        DatePickerMode::Calendar,
    );
    assert_eq!(today.active(), Some(date(2026, 2, 17)));
    assert_eq!(today.draft(), None);
    let clamped = DatePickerSession::new(
        None,
        date(2026, 1, 17),
        policy,
        &locale,
        DatePickerMode::Calendar,
    );
    assert_eq!(clamped.month(), lower.year_month());
    assert_eq!(clamped.active(), Some(date(2026, 2, 13)));
    assert_eq!(policy.validate(lower), Err(DateInputError::Unavailable));
    assert_eq!(policy.validate(upper), Ok(upper));
    assert_eq!(
        policy.validate(date(2026, 3, 2)),
        Err(DateInputError::OutsideBounds)
    );
    assert!(DateBounds::new(upper, lower).is_none());
}

#[test]
fn month_and_year_paging_keep_the_preferred_day_without_selecting_it() {
    let locale = locale();
    let policy = all_dates();
    let committed = date(2026, 1, 31);
    let mut session = DatePickerSession::new(
        Some(committed),
        committed,
        policy,
        &locale,
        DatePickerMode::Calendar,
    );
    let forward = DatePickerStep::Month(StepDirection::Forward);
    for expected in [date(2026, 2, 28), date(2026, 3, 31)] {
        assert_eq!(
            session.navigation_target(forward, policy, &locale),
            Some(expected)
        );
        assert_eq!(
            session.apply(DatePickerEvent::Navigate(forward), policy, &locale),
            DatePickerEffect::None
        );
        assert_eq!(session.active(), Some(expected));
        assert_eq!(session.draft(), Some(committed));
    }
    let year = DatePickerStep::Year(StepDirection::Backward);
    assert_eq!(
        session.navigation_target(year, policy, &locale),
        Some(date(2025, 3, 31))
    );

    let leap = DatePickerSession::new(
        Some(date(2024, 2, 29)),
        committed,
        policy,
        &locale,
        DatePickerMode::Calendar,
    );
    assert_eq!(
        leap.navigation_target(
            DatePickerStep::Year(StepDirection::Forward),
            policy,
            &locale
        ),
        Some(date(2025, 2, 28))
    );
}

#[test]
fn navigation_after_direct_day_focus_uses_that_origin_without_selecting() {
    let locale = locale();
    let policy = all_dates();
    let selected = date(2026, 1, 31);
    let actual_focus = date(2026, 1, 15);
    let mut session = DatePickerSession::new(
        Some(selected),
        selected,
        policy,
        &locale,
        DatePickerMode::Calendar,
    );
    assert_eq!(
        session.navigation_target_from(
            actual_focus,
            DatePickerStep::Day(StepDirection::Forward),
            policy,
            &locale,
        ),
        Some(date(2026, 1, 16)),
    );
    let page = DatePickerStep::Month(StepDirection::Forward);
    assert_eq!(
        session.navigation_target_from(actual_focus, page, policy, &locale),
        Some(date(2026, 2, 15)),
    );
    assert_eq!(
        session.apply(
            DatePickerEvent::NavigateFrom(actual_focus, page),
            policy,
            &locale
        ),
        DatePickerEffect::None,
    );
    assert_eq!(session.active(), Some(date(2026, 2, 15)));
    assert_eq!(session.draft(), Some(selected));
    assert_eq!(
        session.navigation_target(page, policy, &locale),
        Some(date(2026, 3, 15)),
    );
    assert_eq!(
        session.navigation_target_from(actual_focus, page, policy, &locale),
        None,
    );
}

#[test]
fn navigation_skips_disabled_days_using_the_requested_week_or_month() {
    let locale = locale();
    let blocked = date(2025, 12, 30);
    let allow = |date| date != blocked;
    let policy = DatePickerConstraints {
        allowed_date: Some(&allow),
        ..all_dates()
    };
    let forward = DatePickerSession::new(
        Some(date(2025, 12, 23)),
        blocked,
        policy,
        &locale,
        DatePickerMode::Calendar,
    );
    assert_eq!(
        forward.navigation_target(
            DatePickerStep::Week(StepDirection::Forward),
            policy,
            &locale
        ),
        Some(date(2025, 12, 31))
    );
    let backward = DatePickerSession::new(
        Some(date(2026, 1, 6)),
        blocked,
        policy,
        &locale,
        DatePickerMode::Calendar,
    );
    assert_eq!(
        backward.navigation_target(
            DatePickerStep::Week(StepDirection::Backward),
            policy,
            &locale
        ),
        Some(date(2025, 12, 29))
    );

    let policy = constraints(date(2026, 2, 25), date(2026, 3, 3));
    let march = date(2026, 3, 1);
    let session = DatePickerSession::new(
        Some(march),
        march,
        policy,
        &locale,
        DatePickerMode::Calendar,
    );
    assert_eq!(
        session.navigation_target(DatePickerStep::WeekStart, policy, &locale),
        Some(date(2026, 2, 25))
    );
    assert_eq!(
        session.navigation_target(DatePickerStep::WeekEnd, policy, &locale),
        Some(march)
    );
    assert_eq!(
        session.navigation_target(
            DatePickerStep::WeekStart,
            policy,
            &DatePickerLocale {
                first_weekday: Weekday::Sunday,
                ..locale
            },
        ),
        Some(march)
    );

    let unavailable_first = |date: CivilDate| date.day() != 1;
    let policy = DatePickerConstraints {
        allowed_date: Some(&unavailable_first),
        ..all_dates()
    };
    let end = date(2026, 2, 28);
    let session = DatePickerSession::new(Some(end), end, policy, &locale, DatePickerMode::Calendar);
    assert_eq!(
        session.navigation_target(DatePickerStep::Day(StepDirection::Forward), policy, &locale),
        Some(date(2026, 3, 2))
    );
}

#[test]
fn empty_months_and_year_pages_preserve_the_draft_and_respect_bounds() {
    let first = date(2025, 12, 16);
    let last = date(2027, 2, 5);
    let locale = locale();
    let unavailable_month = |date: CivilDate| date.year_month() != YearMonth::new(2026, 1).unwrap();
    let policy = DatePickerConstraints {
        allowed_date: Some(&unavailable_month),
        ..constraints(first, last)
    };
    let mut session = DatePickerSession::new(
        Some(first),
        first,
        policy,
        &locale,
        DatePickerMode::Calendar,
    );
    assert_eq!(
        session.navigation_target(
            DatePickerStep::Month(StepDirection::Forward),
            policy,
            &locale
        ),
        None
    );
    session.apply(
        DatePickerEvent::BrowseMonth(StepDirection::Forward),
        policy,
        &locale,
    );
    assert_eq!(session.month(), YearMonth::new(2026, 1).unwrap());
    assert_eq!(session.active(), None);
    assert_eq!(session.draft(), Some(first));

    session.apply(
        DatePickerEvent::SetMode(DatePickerMode::Year),
        policy,
        &locale,
    );
    session.apply(DatePickerEvent::BrowseYears(2027), policy, &locale);
    assert_eq!(session.mode(), DatePickerMode::Year);
    assert_eq!(session.month(), YearMonth::new(2027, 1).unwrap());
    session.apply(DatePickerEvent::ChooseYear(2028), policy, &locale);
    assert_eq!(session.mode(), DatePickerMode::Year);
    session.apply(DatePickerEvent::ChooseYear(2025), policy, &locale);
    assert_eq!(session.mode(), DatePickerMode::Calendar);
    assert_eq!(session.month(), first.year_month());
    assert_eq!(session.active(), Some(first));
    assert_eq!(session.draft(), Some(first));
}

#[test]
fn input_and_calendar_share_draft_and_confirm_revalidates_current_constraints() {
    let locale = locale();
    let policy = all_dates();
    let saved = date(2026, 2, 28);
    let typed = date(2026, 3, 9);
    let mut session =
        DatePickerSession::new(Some(saved), saved, policy, &locale, DatePickerMode::Input);
    assert_eq!(
        session.apply(DatePickerEvent::Edit("2026-3-9".into()), policy, &locale),
        DatePickerEffect::None
    );
    assert_eq!(session.draft(), Some(typed));
    assert_eq!(session.raw_input(), "2026-3-9");
    session.apply(
        DatePickerEvent::SetMode(DatePickerMode::Calendar),
        policy,
        &locale,
    );
    assert_eq!(session.active(), Some(typed));
    assert_eq!(session.raw_input(), "2026-3-9");
    session.apply(
        DatePickerEvent::SetMode(DatePickerMode::Input),
        policy,
        &locale,
    );

    let blocked = |date| date != typed;
    let new_policy = DatePickerConstraints {
        allowed_date: Some(&blocked),
        ..policy
    };
    assert_eq!(
        session.apply(DatePickerEvent::Confirm, new_policy, &locale),
        DatePickerEffect::None
    );
    assert_eq!(session.error(), Some(DateInputError::Unavailable));
    assert_eq!(session.draft(), None);
    assert_eq!(session.raw_input(), "2026-3-9");

    session.apply(DatePickerEvent::Edit("2026/2/30".into()), policy, &locale);
    assert_eq!(session.error(), None);
    assert_eq!(session.draft(), None);
    session.apply(
        DatePickerEvent::SetMode(DatePickerMode::Calendar),
        policy,
        &locale,
    );
    assert_eq!(session.raw_input(), "2026/2/30");
    assert_eq!(
        session.apply(DatePickerEvent::Confirm, policy, &locale),
        DatePickerEffect::None
    );
    assert_eq!(session.error(), Some(DateInputError::InvalidDate));

    session.apply(DatePickerEvent::Select(saved), policy, &locale);
    assert_eq!(session.raw_input(), "2026/02/28");
    assert_eq!(session.error(), None);
    assert_eq!(
        session.apply(DatePickerEvent::Confirm, policy, &locale),
        DatePickerEffect::Commit(saved)
    );
    assert_eq!(
        session.apply(DatePickerEvent::Cancel, policy, &locale),
        DatePickerEffect::Dismiss
    );
}

#[test]
fn input_focus_loss_formats_or_validates_once_and_mode_switch_retires_the_observation() {
    let locale = locale();
    let policy = all_dates();
    let today = date(2026, 11, 20);
    let mut session = DatePickerSession::new(None, today, policy, &locale, DatePickerMode::Input);
    assert!(!session.observe_input_focus(false, policy, &locale));
    assert_eq!(session.error(), None);
    assert!(!session.observe_input_focus(true, policy, &locale));
    session.apply(DatePickerEvent::Edit("2026-2-3".into()), policy, &locale);
    assert_eq!(session.raw_input(), "2026-2-3");
    assert!(session.observe_input_focus(false, policy, &locale));
    assert_eq!(session.raw_input(), "2026/02/03");
    assert!(!session.observe_input_focus(false, policy, &locale));

    assert!(!session.observe_input_focus(true, policy, &locale));
    session.apply(DatePickerEvent::Edit("2026/2/30".into()), policy, &locale);
    assert!(session.observe_input_focus(false, policy, &locale));
    assert_eq!(session.error(), Some(DateInputError::InvalidDate));
    assert!(!session.observe_input_focus(false, policy, &locale));

    assert!(!session.observe_input_focus(true, policy, &locale));
    session.apply(DatePickerEvent::Edit("2026.4.7".into()), policy, &locale);
    session.apply(
        DatePickerEvent::SetMode(DatePickerMode::Calendar),
        policy,
        &locale,
    );
    assert!(!session.observe_input_focus(false, policy, &locale));
    assert_eq!(session.raw_input(), "2026.4.7");
    assert_eq!(session.active(), Some(date(2026, 4, 7)));
}
