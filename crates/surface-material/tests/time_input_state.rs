use surface_material::{
    ClockFormat::{TwelveHour, TwentyFourHour},
    PeriodPosition, TimeField,
    TimeField::{Hour, Minute},
    TimeInputEffect as Effect, TimeInputError as Error, TimeInputEvent as Event, TimeInputLocale,
    TimeInputSession, TimeOfDay,
    TimePeriod::{Am, Pm},
};

fn time(hour: u8, minute: u8) -> TimeOfDay {
    TimeOfDay::new(hour, minute).unwrap()
}

fn locale() -> TimeInputLocale<'static> {
    TimeInputLocale::new("AM", "PM", ":")
}

fn edit(session: &mut TimeInputSession, field: TimeField, raw: &str) {
    assert_eq!(
        session.apply(Event::Edit(field, raw.to_owned())),
        Effect::None
    );
}

#[test]
fn every_minute_round_trips_through_twelve_hour_notation() {
    for hour in 0..24 {
        for minute in 0..60 {
            let value = time(hour, minute);
            assert_eq!(value.hour(), hour);
            assert_eq!(value.minute(), minute);
            assert_eq!(value.display_hour(TwentyFourHour), hour);
            assert_eq!(
                TimeOfDay::from_twelve_hour(value.display_hour(TwelveHour), minute, value.period()),
                Some(value)
            );
        }
    }
    assert_eq!(TimeOfDay::from_twelve_hour(12, 0, Am), Some(time(0, 0)));
    assert_eq!(TimeOfDay::from_twelve_hour(12, 0, Pm), Some(time(12, 0)));
    assert_eq!(time(23, 59).to_string(), "23:59");
}

#[test]
fn invalid_hours_and_minutes_never_construct_a_value() {
    for hour in [24, 25, u8::MAX] {
        assert_eq!(TimeOfDay::new(hour, 0), None);
    }
    for minute in [60, 61, u8::MAX] {
        assert_eq!(TimeOfDay::new(0, minute), None);
        assert_eq!(TimeOfDay::from_twelve_hour(12, minute, Pm), None);
    }
    for hour in [0, 13, u8::MAX] {
        assert_eq!(TimeOfDay::from_twelve_hour(hour, 0, Am), None);
    }
}

#[test]
fn localized_summary_uses_custom_digits_labels_and_period_order() {
    let chinese = TimeInputLocale::new("上午", "下午", "：")
        .with_digits(['０', '１', '２', '３', '４', '５', '６', '７', '８', '９'])
        .unwrap()
        .with_period_position(PeriodPosition::Before, "");
    assert_eq!(chinese.format(time(0, 8), TwelveHour), "上午１２：０８");
    assert_eq!(chinese.format(time(14, 8), TwelveHour), "下午２：０８");
    assert_eq!(chinese.format(time(14, 8), TwentyFourHour), "１４：０８");
    assert_eq!(locale().format(time(14, 8), TwelveHour), "2:08 PM");
    assert_eq!(locale().format(time(7, 1), TwentyFourHour), "07:01");
}

#[test]
fn locale_rejects_duplicate_or_ambiguous_digits() {
    assert!(locale().with_digits(['0'; 10]).is_none());
    assert!(locale()
        .with_digits(['０', '２', '１', '３', '４', '５', '６', '７', '８', '９'])
        .is_none());
    assert!(locale()
        .with_digits([' ', '1', '2', '3', '4', '5', '6', '7', '8', '9'])
        .is_none());
}

#[test]
fn both_input_formats_show_the_same_committed_time() {
    let committed = time(21, 6);
    let mut twelve = TimeInputSession::new(Some(committed), TwelveHour, locale());
    let mut twenty_four = TimeInputSession::new(Some(committed), TwentyFourHour, locale());
    assert_eq!(twelve.clock(), TwelveHour);
    assert_eq!(twelve.raw(Hour), "9");
    assert_eq!(twelve.raw(Minute), "06");
    assert_eq!(twelve.period(), Some(Pm));
    assert_eq!(twelve.validation().candidate(), Some(committed));
    assert_eq!(twenty_four.raw(Hour), "21");
    assert_eq!(twenty_four.raw(Minute), "06");
    assert_eq!(twenty_four.period(), None);
    assert_eq!(twenty_four.validation().candidate(), Some(committed));
    assert_eq!(twelve.apply(Event::Confirm), Effect::Commit(committed));
    assert_eq!(twenty_four.apply(Event::Confirm), Effect::Commit(committed));
}

#[test]
fn accepted_fullwidth_input_is_preserved_until_that_field_finishes() {
    let mut session = TimeInputSession::new(None, TwentyFourHour, locale());
    edit(&mut session, Minute, "　５９　");
    edit(&mut session, Hour, "２3");
    assert_eq!(session.validation().candidate(), Some(time(23, 59)));
    assert_eq!(session.raw(Hour), "２3");
    assert_eq!(session.raw(Minute), "　５９　");
    assert_eq!(session.apply(Event::FinishEditing(Hour)), Effect::None);
    assert_eq!(session.raw(Hour), "23");
    assert_eq!(session.raw(Minute), "　５９　");
    assert_eq!(session.apply(Event::Confirm), Effect::Commit(time(23, 59)));
    assert_eq!(session.raw(Minute), "59");
}

#[test]
fn validation_rejects_in_field_separators_signs_and_overlong_input() {
    let mut session = TimeInputSession::new(None, TwentyFourHour, locale());
    for (raw, error) in [
        ("　", Error::Required),
        ("2:5", Error::ExpectedOneOrTwoDigits),
        ("+2", Error::ExpectedOneOrTwoDigits),
        ("２－", Error::ExpectedOneOrTwoDigits),
        ("23x", Error::ExpectedOneOrTwoDigits),
        ("003", Error::ExpectedOneOrTwoDigits),
        ("24", Error::OutOfRange),
    ] {
        edit(&mut session, Hour, raw);
        assert_eq!(session.validation().field(Hour), Err(error), "{raw}");
        assert_eq!(session.apply(Event::FinishEditing(Hour)), Effect::None);
        assert_eq!(session.raw(Hour), raw);
    }
}

#[test]
fn localized_digits_parse_and_normalize_without_borrowing_labels() {
    let label = String::from("am");
    let mut session = {
        let local = TimeInputLocale::new(&label, "pm", ":")
            .with_digits(['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩'])
            .unwrap();
        TimeInputSession::new(None, TwelveHour, local)
    };
    drop(label);
    edit(&mut session, Hour, "٠9");
    edit(&mut session, Minute, "٤");
    assert_eq!(session.validation().candidate(), Some(time(9, 4)));
    assert_eq!(session.apply(Event::FinishEditing(Hour)), Effect::None);
    assert_eq!(session.raw(Hour), "٩");
    assert_eq!(session.raw(Minute), "٤");
    assert_eq!(session.apply(Event::Confirm), Effect::Commit(time(9, 4)));
    assert_eq!(session.raw(Minute), "٠٤");
}

#[test]
fn field_errors_follow_blur_and_edit_independently() {
    let mut session = TimeInputSession::new(None, TwelveHour, locale());
    edit(&mut session, Hour, "１３");
    edit(&mut session, Minute, "６０");
    assert_eq!(session.displayed_error(Hour), None);
    assert_eq!(session.displayed_error(Minute), None);
    assert!(!session.observe_input_focus(Some(Hour)));
    assert!(!session.observe_input_focus(Some(Hour)));
    assert!(session.observe_input_focus(Some(Minute)));
    assert_eq!(session.displayed_error(Hour), Some(Error::OutOfRange));
    assert_eq!(session.displayed_error(Minute), None);
    assert!(session.observe_input_focus(None));
    assert_eq!(session.displayed_error(Minute), Some(Error::OutOfRange));
    assert!(!session.observe_input_focus(None));

    edit(&mut session, Hour, "０");
    assert_eq!(session.validation().field(Hour), Err(Error::OutOfRange));
    edit(&mut session, Hour, "１２");
    assert_eq!(session.displayed_error(Hour), None);
    assert_eq!(session.displayed_error(Minute), Some(Error::OutOfRange));
    assert_eq!(session.apply(Event::Confirm), Effect::None);
    assert_eq!(session.raw(Hour), "１２");
    assert_eq!(session.raw(Minute), "６０");
}

#[test]
fn period_changes_the_preview_without_changing_raw_text_or_saving() {
    let mut session = TimeInputSession::new(None, TwelveHour, locale());
    edit(&mut session, Hour, "１２");
    edit(&mut session, Minute, "０");
    assert_eq!(session.period(), Some(Am));
    assert_eq!(session.validation().candidate(), Some(time(0, 0)));
    assert_eq!(session.apply(Event::SelectPeriod(Pm)), Effect::None);
    assert_eq!(session.validation().candidate(), Some(time(12, 0)));
    assert_eq!(session.raw(Hour), "１２");
    assert_eq!(session.raw(Minute), "０");

    let mut other = TimeInputSession::new(Some(time(20, 0)), TwentyFourHour, locale());
    assert_eq!(other.apply(Event::SelectPeriod(Am)), Effect::None);
    assert_eq!(other.period(), None);
    assert_eq!(other.validation().candidate(), Some(time(20, 0)));
}

#[test]
fn caller_can_discard_and_reopen_or_consume_one_explicit_commit() {
    let initial = time(14, 26);
    let mut saved = Some(initial);
    let mut session = Some(TimeInputSession::new(saved, TwentyFourHour, locale()));
    let draft = session.as_mut().unwrap();
    edit(draft, Minute, "３");
    assert_eq!(draft.validation().candidate(), Some(time(14, 3)));
    assert_eq!(draft.apply(Event::FinishEditing(Minute)), Effect::None);
    assert_eq!(draft.raw(Minute), "03");
    assert_eq!(saved, Some(initial));
    assert_eq!(draft.apply(Event::Cancel), Effect::Dismiss);
    session.take();
    assert_eq!(saved, Some(initial));

    session = Some(TimeInputSession::new(saved, TwelveHour, locale()));
    edit(session.as_mut().unwrap(), Minute, "３");
    let mut commits = 0;
    for _ in 0..2 {
        if let Some(open) = &mut session {
            if let Effect::Commit(value) = open.apply(Event::Confirm) {
                saved = Some(value);
                commits += 1;
                session = None;
            }
        }
    }
    assert_eq!(commits, 1);
    assert_eq!(saved, Some(time(14, 3)));
}

#[test]
fn confirm_reveals_both_required_errors() {
    let mut session = TimeInputSession::new(None, TwelveHour, locale());
    assert_eq!(session.apply(Event::Confirm), Effect::None);
    assert_eq!(session.displayed_error(Hour), Some(Error::Required));
    assert_eq!(session.displayed_error(Minute), Some(Error::Required));
}
