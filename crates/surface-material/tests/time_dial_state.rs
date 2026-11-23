use surface_material::{
    ClockFormat::{self, TwelveHour, TwentyFourHour},
    DialEvent, DialMode, DialPoint, DialPreview, DialState,
    TimeField::{self, Hour, Minute},
    TimeInputEffect::{self, Commit, Dismiss, None as NoEffect},
    TimeInputError::{OutOfRange, Required},
    TimeInputEvent::{self, Cancel, Confirm, Edit, FinishEditing, SelectPeriod},
    TimeInputLocale, TimeInputSession, TimeOfDay,
    TimePeriod::{Am, Pm},
};

fn point(state: &mut DialState, field: TimeField, phase: DialPoint) -> TimeInputEffect {
    state.apply(DialEvent::Point { field, phase })
}

fn time(hour: u8, minute: u8) -> TimeOfDay {
    TimeOfDay::new(hour, minute).unwrap()
}

fn session(value: Option<TimeOfDay>, clock: ClockFormat, mode: DialMode) -> DialState {
    let locale = TimeInputLocale::new("上午", "下午", "：");
    DialState::new(TimeInputSession::new(value, clock, locale), mode, true)
}

fn input(state: &mut DialState, event: TimeInputEvent) -> TimeInputEffect {
    state.apply(DialEvent::Input(event))
}

fn edit(state: &mut DialState, field: TimeField, raw: &str) {
    assert_eq!(input(state, Edit(field, raw.to_owned())), NoEffect);
}

fn choose(state: &mut DialState, field: TimeField, value: u8) {
    assert_eq!(state.apply(DialEvent::Choose { field, value }), NoEffect);
}

#[test]
fn empty_dial_has_no_default_time_and_requires_explicit_save() {
    let mut state = session(None, TwelveHour, DialMode::Dial);
    assert_eq!(state.field(), Hour);
    assert_eq!(state.input().validation().candidate(), None);
    assert_eq!(input(&mut state, Confirm), NoEffect);
    assert_eq!(state.input().displayed_error(Hour), Some(Required));
    assert_eq!(state.input().displayed_error(Minute), Some(Required));

    choose(&mut state, Hour, 12);
    assert_eq!(state.field(), Minute);
    assert_eq!(state.input().raw(Hour), "12");
    assert_eq!(state.input().raw(Minute), "");
    assert_eq!(state.input().displayed_error(Hour), None);
    assert_eq!(state.input().displayed_error(Minute), Some(Required));
    assert_eq!(state.input().validation().candidate(), None);
    choose(&mut state, Minute, 0);
    assert_eq!(state.input().validation().candidate(), Some(time(0, 0)));
    assert_eq!(input(&mut state, Confirm), Commit(time(0, 0)));
}

#[test]
fn valid_hour_and_invalid_minute_keep_their_own_text_and_error_on_toggle() {
    let mut state = session(None, TwelveHour, DialMode::Input);
    edit(&mut state, Hour, "０９");
    edit(&mut state, Minute, "６０");
    input(&mut state, FinishEditing(Minute));
    input(&mut state, SelectPeriod(Pm));
    assert!(!state.observe_input_focus(Some(Hour)));
    state.apply(DialEvent::Show(DialMode::Dial));
    assert_eq!(state.input().raw(Hour), "０９");
    assert_eq!(state.input().raw(Minute), "６０");
    assert_eq!(state.input().displayed_error(Minute), Some(OutOfRange));
    assert_eq!(state.input().period(), Some(Pm));

    choose(&mut state, Hour, 11);
    assert_eq!(state.input().raw(Hour), "11");
    assert_eq!(state.field(), Minute);
    assert_eq!(state.input().raw(Minute), "６０");
    assert_eq!(state.input().displayed_error(Minute), Some(OutOfRange));
    assert_eq!(input(&mut state, Confirm), NoEffect);
    state.apply(DialEvent::Show(DialMode::Input));
    assert!(!state.observe_input_focus(None));
    assert_eq!(state.input().raw(Minute), "６０");
    assert_eq!(state.input().displayed_error(Minute), Some(OutOfRange));
    assert_eq!(state.input().period(), Some(Pm));
}

#[test]
fn repairing_one_field_keeps_the_valid_opposite_literal() {
    let mut state = session(None, TwentyFourHour, DialMode::Input);
    edit(&mut state, Hour, "２４");
    edit(&mut state, Minute, "　０５　");
    assert_eq!(input(&mut state, Confirm), NoEffect);
    state.apply(DialEvent::Show(DialMode::Dial));
    assert_eq!(state.input().validation().field(Minute), Ok(5));
    assert_eq!(state.input().validation().field(Hour), Err(OutOfRange));

    assert_eq!(point(&mut state, Hour, DialPoint::End(Some(23))), NoEffect);
    assert_eq!(state.input().raw(Hour), "23");
    assert_eq!(state.input().raw(Minute), "　０５　");
    assert_eq!(state.input().displayed_error(Hour), None);
    assert_eq!(state.input().period(), None);
    assert_eq!(state.input().validation().candidate(), Some(time(23, 5)));
    assert_eq!(state.field(), Minute);
    assert_eq!(input(&mut state, Confirm), Commit(time(23, 5)));
}

#[test]
fn stale_editor_events_and_invalid_numeric_choices_have_no_effect_in_dial() {
    let mut state = session(Some(time(9, 5)), TwelveHour, DialMode::Dial);
    let before = state.clone();
    choose(&mut state, Hour, 0);
    choose(&mut state, Hour, 13);
    choose(&mut state, Minute, 24);
    input(&mut state, Edit(Hour, "１１".into()));
    input(&mut state, FinishEditing(Hour));
    assert_eq!(state, before);
}

#[test]
fn intentional_mode_change_keeps_raw_but_normal_input_blur_normalizes_it() {
    let mut state = session(None, TwentyFourHour, DialMode::Input);
    edit(&mut state, Hour, "９");
    assert!(!state.observe_input_focus(Some(Hour)));
    state.apply(DialEvent::Show(DialMode::Dial));
    assert!(!state.observe_input_focus(None));
    state.apply(DialEvent::Show(DialMode::Input));
    assert!(!state.observe_input_focus(None));
    assert_eq!(state.input().raw(Hour), "９");

    assert!(!state.observe_input_focus(Some(Hour)));
    assert!(state.observe_input_focus(Some(Minute)));
    assert_eq!(state.input().raw(Hour), "09");
}

#[test]
fn compact_fallback_clears_preview_and_requires_explicit_return() {
    let mut state = session(None, TwelveHour, DialMode::Input);
    edit(&mut state, Minute, "３４");
    state.apply(DialEvent::Show(DialMode::Dial));
    assert_eq!(point(&mut state, Hour, DialPoint::Move(Some(12))), NoEffect);
    assert_eq!(
        state.preview(),
        Some(DialPreview {
            field: Hour,
            value: 12
        })
    );
    state.apply(DialEvent::SetDialFits(false));
    assert!(!state.dial_fits());
    assert_eq!(state.mode(), DialMode::Input);
    assert_eq!(state.preview(), None);
    assert_eq!(point(&mut state, Hour, DialPoint::End(Some(12))), NoEffect);
    assert_eq!(state.input().raw(Hour), "");
    assert_eq!(state.input().raw(Minute), "３４");
    state.apply(DialEvent::Show(DialMode::Dial));
    assert_eq!(state.mode(), DialMode::Input);
    state.apply(DialEvent::SetDialFits(true));
    assert_eq!(state.mode(), DialMode::Input);
    state.apply(DialEvent::Show(DialMode::Dial));
    assert_eq!(state.mode(), DialMode::Dial);
    assert_eq!(state.field(), Hour);
}

#[test]
fn cancelled_or_unaccepted_point_samples_leave_the_draft_unchanged() {
    let mut state = session(Some(time(14, 6)), TwelveHour, DialMode::Dial);
    assert_eq!(point(&mut state, Hour, DialPoint::Move(Some(3))), NoEffect);
    assert_eq!(
        state.preview(),
        Some(DialPreview {
            field: Hour,
            value: 3
        })
    );
    assert_eq!(point(&mut state, Hour, DialPoint::Move(None)), NoEffect);
    assert_eq!(state.preview().unwrap().value, 3);
    assert_eq!(point(&mut state, Hour, DialPoint::End(None)), NoEffect);
    assert_eq!(state.preview(), None);

    point(&mut state, Hour, DialPoint::Move(Some(9)));
    point(&mut state, Hour, DialPoint::Cancel);
    assert_eq!(state.preview(), None);
    assert_eq!(state.field(), Hour);
    assert_eq!(state.input().validation().candidate(), Some(time(14, 6)));
    assert_eq!(input(&mut state, Cancel), Dismiss);
}

#[test]
fn hour_release_advances_once_but_an_arrow_adjustment_stays_in_field() {
    let mut state = session(Some(time(13, 42)), TwelveHour, DialMode::Dial);
    assert_eq!(
        state.apply(DialEvent::Adjust {
            field: Hour,
            value: 12
        }),
        NoEffect
    );
    assert_eq!(state.field(), Hour);
    assert_eq!(state.input().validation().candidate(), Some(time(12, 42)));
    assert_eq!(point(&mut state, Hour, DialPoint::Tap(Some(12))), NoEffect);
    assert_eq!(state.field(), Minute);
    assert_eq!(point(&mut state, Hour, DialPoint::End(Some(1))), NoEffect);
    assert_eq!(state.input().raw(Hour), "12");
    assert_eq!(state.input().period(), Some(Pm));

    assert_eq!(
        point(&mut state, Minute, DialPoint::Move(Some(59))),
        NoEffect
    );
    assert_eq!(state.preview().unwrap().value, 59);
    assert_eq!(state.input().raw(Minute), "42");
    assert_eq!(
        point(&mut state, Minute, DialPoint::End(Some(59))),
        NoEffect
    );
    assert_eq!(state.preview(), None);
    assert_eq!(state.input().validation().candidate(), Some(time(12, 59)));
    assert_eq!(input(&mut state, SelectPeriod(Am)), NoEffect);
    assert_eq!(input(&mut state, Confirm), Commit(time(0, 59)));
}
