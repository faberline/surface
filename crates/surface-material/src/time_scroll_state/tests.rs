use super::*;
use crate::{
    TimeField::{Hour, Minute},
    TimeInputEffect::{self as Effect, Commit, Dismiss, None as NoEffect},
    TimeInputError, TimeInputLocale, TimeOfDay, TimePeriod,
};
use TimeScrollEvent as Event;

fn time(hour: u8, minute: u8) -> TimeOfDay {
    TimeOfDay::new(hour, minute).unwrap()
}

fn picker(time: Option<TimeOfDay>, clock: ClockFormat, mode: TimeScrollMode) -> TimeScrollState {
    let locale = TimeInputLocale::new("上午", "下午", "：");
    TimeScrollState::new(TimeInputSession::new(time, clock, locale), mode, true)
}

fn input(state: &mut TimeScrollState, event: TimeInputEvent) -> Effect {
    state.apply(Event::Input(event), true)
}

fn drag(
    state: &mut TimeScrollState,
    field: TimeField,
    sample: TimeScrollDrag,
    height: f32,
    reduced: bool,
) {
    let session = state.drag_session(field);
    drag_with_session(state, field, session, sample, height, reduced);
}

fn drag_with_session(
    state: &mut TimeScrollState,
    field: TimeField,
    session: PointDragSession,
    sample: TimeScrollDrag,
    height: f32,
    reduced: bool,
) {
    assert_eq!(
        state.apply(
            Event::Drag {
                field,
                session,
                sample,
                row_height: height,
            },
            reduced,
        ),
        NoEffect
    );
}

fn wheel(state: &mut TimeScrollState, field: TimeField, delta_y: f32, now_ms: Option<f64>) {
    assert_eq!(
        state.apply(
            Event::Wheel {
                field,
                delta_y,
                row_height: 48.,
                now_ms
            },
            false
        ),
        NoEffect
    );
}

#[test]
fn independent_held_offsets_do_not_change_draft_and_one_release_preserves_the_other() {
    let mut state = picker(
        Some(time(9, 7)),
        ClockFormat::TwentyFourHour,
        TimeScrollMode::Scroll,
    );
    drag(
        &mut state,
        Hour,
        TimeScrollDrag::Move { delta_y: -72. },
        48.,
        false,
    );
    drag(
        &mut state,
        Minute,
        TimeScrollDrag::Move { delta_y: 72. },
        48.,
        false,
    );
    assert_eq!(state.column(Hour).offset, 10.5);
    assert_eq!(state.column(Hour).preview, Some(11));
    assert_eq!(state.column(Minute).offset, 5.5);
    assert_eq!(state.column(Minute).preview, Some(6));
    assert_eq!(state.input().validation().candidate(), Some(time(9, 7)));

    // Changed terminal measurement does not redefine the held gesture.
    drag(
        &mut state,
        Hour,
        TimeScrollDrag::End { delta_y: -74. },
        120.,
        true,
    );
    assert_eq!(state.input().validation().candidate(), Some(time(11, 7)));
    assert_eq!(state.column(Hour).offset, 11.);
    assert_eq!(state.column(Minute).preview, Some(6));
    drag(&mut state, Minute, TimeScrollDrag::Cancel, 48., true);
    assert_eq!(state.column(Minute).offset, 7.);
    assert_eq!(state.column(Minute).preview, None);
    assert_eq!(
        input(&mut state, TimeInputEvent::Confirm),
        Commit(time(11, 7))
    );
}

#[test]
fn release_sets_a_draft_then_a_bounded_tween_snaps_the_paint() {
    let mut state = picker(
        Some(time(9, 7)),
        ClockFormat::TwentyFourHour,
        TimeScrollMode::Scroll,
    );
    drag(
        &mut state,
        Minute,
        TimeScrollDrag::Move { delta_y: -59. },
        48.,
        false,
    );
    drag(
        &mut state,
        Minute,
        TimeScrollDrag::End { delta_y: -59. },
        48.,
        false,
    );
    assert_eq!(state.input().raw(Minute), "08");
    assert!(state.advance(Some(200.), false).request_next);
    let start = state.column(Minute).offset;
    assert!(start > 8.);
    assert!(state.advance(Some(260.), false).rebuild);
    assert!((8. ..start).contains(&state.column(Minute).offset));
    assert!(!state.advance(Some(320.), false).request_next);
    assert_eq!(state.column(Minute).offset, 8.);
    assert!(!state.advance(Some(340.), false).rebuild);
}

#[test]
fn cancelled_or_nonfinite_terminal_restores_draft_without_a_value_change() {
    let mut state = picker(
        Some(time(3, 18)),
        ClockFormat::TwentyFourHour,
        TimeScrollMode::Scroll,
    );
    drag_with_session(
        &mut state,
        Minute,
        PointDragSession::new(),
        TimeScrollDrag::End { delta_y: -130. },
        48.,
        false,
    );
    assert_eq!(state.input().raw(Minute), "18");
    drag(
        &mut state,
        Minute,
        TimeScrollDrag::Move { delta_y: -80. },
        f32::NAN,
        false,
    );
    assert_eq!(state.column(Minute).preview, None);
    drag(
        &mut state,
        Minute,
        TimeScrollDrag::Move { delta_y: -130. },
        48.,
        false,
    );
    drag(
        &mut state,
        Minute,
        TimeScrollDrag::End { delta_y: f32::NAN },
        48.,
        false,
    );
    assert_eq!(state.column(Minute).preview, None);
    assert_eq!(state.column(Minute).selected, Some(18));
    assert!(state.advance(Some(300.), false).request_next);
    assert!(!state.advance(Some(260.), false).request_next);
    assert_eq!(state.column(Minute).offset, 18.);
}

#[test]
fn release_without_move_accepts_once_and_rearms_only_its_wheel() {
    let mut state = picker(
        Some(time(9, 7)),
        ClockFormat::TwentyFourHour,
        TimeScrollMode::Scroll,
    );
    let hour = state.drag_session(Hour);
    let minute = state.drag_session(Minute);
    let release = TimeScrollDrag::End { delta_y: -60. };
    drag_with_session(&mut state, Hour, minute.clone(), release, 48., true);
    assert_eq!(state.input().validation().candidate(), Some(time(9, 7)));

    drag_with_session(&mut state, Hour, hour.clone(), release, 48., true);
    assert_eq!(state.input().validation().candidate(), Some(time(10, 7)));
    assert_ne!(state.drag_session(Hour), hour);
    assert_eq!(state.drag_session(Minute), minute);
    drag_with_session(&mut state, Hour, hour, release, 48., true);
    assert_eq!(state.input().validation().candidate(), Some(time(10, 7)));

    drag(&mut state, Hour, release, 48., true);
    assert_eq!(state.input().validation().candidate(), Some(time(11, 7)));
}

#[test]
fn same_mode_resize_discards_preview_and_rejects_contacts_from_old_geometry() {
    let mut state = picker(
        Some(time(9, 7)),
        ClockFormat::TwentyFourHour,
        TimeScrollMode::Scroll,
    );
    let old_minute = state.drag_session(Minute);
    let old_hour = state.drag_session(Hour);
    drag(
        &mut state,
        Minute,
        TimeScrollDrag::Move { delta_y: -60. },
        48.,
        false,
    );
    wheel(&mut state, Hour, 80., None);
    assert_eq!(state.column(Minute).preview, Some(8));
    assert_eq!(state.column(Hour).preview, Some(11));

    state.apply(Event::SetScrollFits(true), false);
    assert_eq!(state.mode(), TimeScrollMode::Scroll);
    assert_ne!(state.drag_session(Minute), old_minute);
    assert_ne!(state.drag_session(Hour), old_hour);
    assert_eq!(state.column(Minute).offset, 7.);
    assert_eq!(state.column(Hour).offset, 9.);
    assert_eq!(state.advance(Some(500.), false), AnimationUpdate::default());
    drag_with_session(
        &mut state,
        Minute,
        old_minute,
        TimeScrollDrag::End { delta_y: -60. },
        48.,
        true,
    );
    drag_with_session(
        &mut state,
        Hour,
        old_hour,
        TimeScrollDrag::Tap { value: 10 },
        48.,
        true,
    );
    assert_eq!(state.input().validation().candidate(), Some(time(9, 7)));

    drag(
        &mut state,
        Minute,
        TimeScrollDrag::Tap { value: 8 },
        48.,
        true,
    );
    assert_eq!(state.input().validation().candidate(), Some(time(9, 8)));
}

#[test]
fn empty_center_cancel_and_direct_value_preserve_field_independence() {
    let mut state = picker(None, ClockFormat::TwelveHour, TimeScrollMode::Scroll);
    let old_hour = state.drag_session(Hour);
    let old_minute = state.drag_session(Minute);
    drag(&mut state, Hour, TimeScrollDrag::Cancel, 48., false);
    assert_eq!(state.column(Hour).selected, None);
    assert_eq!(state.column(Hour).offset, 0.);
    assert_ne!(state.drag_session(Hour), old_hour);
    assert_eq!(state.drag_session(Minute), old_minute);

    state.apply(
        Event::Pick {
            field: Minute,
            value: 60,
        },
        true,
    );
    assert_eq!(state.column(Minute).selected, None);
    assert_eq!(state.drag_session(Minute), old_minute);
    state.apply(
        Event::Pick {
            field: Minute,
            value: 17,
        },
        true,
    );
    assert_eq!(state.column(Minute).selected, Some(17));
    assert_ne!(state.drag_session(Minute), old_minute);
    drag_with_session(
        &mut state,
        Minute,
        old_minute,
        TimeScrollDrag::Tap { value: 18 },
        48.,
        true,
    );
    assert_eq!(state.column(Minute).selected, Some(17));
    assert_eq!(state.column(Hour).selected, None);
}

#[test]
fn finite_bounds_keep_twelve_hour_choices_and_period_independent() {
    let mut state = picker(
        Some(time(23, 59)),
        ClockFormat::TwelveHour,
        TimeScrollMode::Scroll,
    );
    let hour = state.column(Hour);
    assert_eq!(
        (hour.minimum, hour.maximum, hour.selected),
        (1, 12, Some(11))
    );
    for (field, step) in [
        (Hour, TimeScrollStep::Next),
        (Hour, TimeScrollStep::Next),
        (Minute, TimeScrollStep::Next),
    ] {
        assert_eq!(state.apply(Event::Adjust { field, step }, true), NoEffect);
    }
    assert_eq!(state.input().validation().candidate(), Some(time(12, 59)));
    assert_eq!(state.input().period(), Some(TimePeriod::Pm));
    state.apply(
        Event::Adjust {
            field: Minute,
            step: TimeScrollStep::First,
        },
        true,
    );
    assert_eq!(state.input().validation().candidate(), Some(time(12, 0)));
    state.apply(
        Event::Adjust {
            field: Hour,
            step: TimeScrollStep::First,
        },
        true,
    );
    state.apply(
        Event::Adjust {
            field: Hour,
            step: TimeScrollStep::Previous,
        },
        true,
    );
    assert_eq!(state.input().validation().candidate(), Some(time(13, 0)));
    assert_eq!(
        input(&mut state, TimeInputEvent::SelectPeriod(TimePeriod::Am)),
        NoEffect
    );
    assert_eq!(state.input().validation().candidate(), Some(time(1, 0)));
}

#[test]
fn wheel_small_deltas_accumulate_independently_and_wait_for_quiet() {
    let mut state = picker(
        Some(time(9, 7)),
        ClockFormat::TwentyFourHour,
        TimeScrollMode::Scroll,
    );
    wheel(&mut state, Minute, 8., Some(100.));
    assert!(state.advance(Some(115.), true).request_next);
    wheel(&mut state, Minute, 8., Some(120.));
    wheel(&mut state, Minute, 10., Some(140.));
    wheel(&mut state, Hour, -52., Some(145.));
    assert_eq!(state.column(Minute).preview, Some(8));
    assert_eq!(state.column(Hour).preview, Some(8));
    assert_eq!(state.input().validation().candidate(), Some(time(9, 7)));
    assert!(state.advance(Some(249.), true).request_next);
    let state_update = state.advance(Some(250.), true);
    assert!(state_update.rebuild && state_update.request_next);
    assert_eq!(state.column(Minute).offset, 8.);
    assert_eq!(state.column(Hour).selected, Some(9));
    assert!(!state.advance(Some(255.), true).request_next);
    assert_eq!(state.column(Hour).offset, 8.);
    assert_eq!(state.input().validation().candidate(), Some(time(8, 8)));
}

#[test]
fn wheel_without_dispatch_clock_debounces_from_host_frame_or_steps_without_a_host_clock() {
    let mut state = picker(
        Some(time(9, 7)),
        ClockFormat::TwentyFourHour,
        TimeScrollMode::Scroll,
    );
    wheel(&mut state, Minute, 8., None);
    assert!(state.advance(Some(100.), false).request_next);
    wheel(&mut state, Minute, 38., None);
    assert!(state.advance(Some(160.), false).request_next);
    assert_eq!(state.column(Minute).selected, Some(7));
    assert!(state.advance(Some(260.), false).request_next);
    assert!(!state.advance(Some(270.), true).request_next);
    assert_eq!(state.column(Minute).selected, Some(8));
    wheel(&mut state, Minute, -2., None);
    assert!(!state.advance(None, false).request_next);
    assert_eq!(state.column(Minute).selected, Some(7));
    wheel(&mut state, Minute, -51., Some(500.));
    assert!(state.advance(Some(501.), true).request_next);
    assert!(!state.advance(Some(25.), true).request_next);
    assert_eq!(state.column(Minute).selected, Some(6));
}

#[test]
fn wheel_at_bound_does_not_start_a_preview_or_request_a_frame() {
    let mut state = picker(
        Some(time(0, 59)),
        ClockFormat::TwentyFourHour,
        TimeScrollMode::Scroll,
    );
    wheel(&mut state, Hour, -48., Some(10.));
    wheel(&mut state, Minute, 48., Some(10.));
    for field in FIELDS {
        assert_eq!(state.column(field).preview, None);
        assert!(!state.column(field).settling);
    }
    assert_eq!(state.advance(Some(10.), false), AnimationUpdate::default());
}

#[test]
fn mode_switch_retires_focus_and_keeps_the_unresolved_peer_raw_and_touched() {
    let mut state = picker(None, ClockFormat::TwelveHour, TimeScrollMode::Input);
    assert_eq!(
        input(&mut state, TimeInputEvent::Edit(Hour, "０９".into())),
        NoEffect
    );
    assert_eq!(
        input(&mut state, TimeInputEvent::Edit(Minute, "６０".into())),
        NoEffect
    );
    assert_eq!(
        input(&mut state, TimeInputEvent::FinishEditing(Minute)),
        NoEffect
    );
    assert!(!state.observe_input_focus(Some(Hour)));
    state.apply(Event::Show(TimeScrollMode::Scroll), true);
    assert_eq!(state.column(Hour).selected, Some(9));
    assert_eq!(state.column(Minute).selected, None);
    assert_eq!(state.input().raw(Hour), "０９");
    assert_eq!(state.input().raw(Minute), "６０");
    assert_eq!(
        state.input().displayed_error(Minute),
        Some(TimeInputError::OutOfRange)
    );
    state.apply(
        Event::Pick {
            field: Hour,
            value: 0,
        },
        true,
    );
    assert_eq!(state.input().raw(Hour), "０９");
    state.apply(
        Event::Pick {
            field: Hour,
            value: 11,
        },
        true,
    );
    assert_eq!(state.input().raw(Hour), "11");
    assert_eq!(state.input().raw(Minute), "６０");
    assert_eq!(input(&mut state, TimeInputEvent::Confirm), NoEffect);
    state.apply(Event::Show(TimeScrollMode::Input), true);
    assert!(!state.observe_input_focus(None));
    assert_eq!(state.input().raw(Minute), "６０");
    assert_eq!(input(&mut state, TimeInputEvent::Cancel), Dismiss);
}

#[test]
fn fallback_cancels_previews_and_returns_only_after_explicit_switch() {
    let mut state = picker(None, ClockFormat::TwentyFourHour, TimeScrollMode::Scroll);
    wheel(&mut state, Minute, 75., Some(0.));
    drag(
        &mut state,
        Hour,
        TimeScrollDrag::Move { delta_y: -70. },
        48.,
        true,
    );
    assert_eq!(state.input().validation().candidate(), None);
    state.apply(Event::SetScrollFits(false), true);
    assert_eq!(state.mode(), TimeScrollMode::Input);
    assert!(!state.scroll_fits());
    assert_eq!(state.column(Hour).selected, None);
    assert_eq!(state.column(Minute).selected, None);
    assert_eq!(state.column(Hour).preview, None);
    assert_eq!(state.column(Minute).preview, None);
    state.apply(Event::Show(TimeScrollMode::Scroll), true);
    assert_eq!(state.mode(), TimeScrollMode::Input);
    state.apply(Event::SetScrollFits(true), true);
    assert_eq!(state.mode(), TimeScrollMode::Input);
    state.apply(Event::Show(TimeScrollMode::Scroll), true);
    assert_eq!(state.mode(), TimeScrollMode::Scroll);
    assert!(!state.advance(Some(320.), false).request_next);
}

#[test]
fn equivalent_editor_focus_is_retained_across_modes_and_toolbar_focus() {
    let mut state = picker(
        Some(time(8, 37)),
        ClockFormat::TwelveHour,
        TimeScrollMode::Scroll,
    );
    assert_eq!(state.active_field(), Hour);
    assert!(state.observe_scroll_focus(Some(Minute)));
    assert!(!state.observe_scroll_focus(Some(Minute)));
    assert!(!state.observe_scroll_focus(None));
    assert!(!state.observe_input_focus(Some(Hour)));
    assert_eq!(state.active_field(), Minute);

    let session = state.drag_session(Hour);
    drag(
        &mut state,
        Hour,
        TimeScrollDrag::Move { delta_y: -60. },
        48.,
        false,
    );
    let preview = state.column(Hour);
    state.apply(Event::Focus(Hour), false);
    assert_eq!(state.active_field(), Hour);
    assert_eq!(state.column(Hour), preview);
    assert_eq!(state.drag_session(Hour), session);
    state.apply(Event::Focus(Minute), false);
    state.apply(Event::Show(TimeScrollMode::Input), false);
    assert_eq!(state.active_field(), Minute);
    state.apply(Event::Focus(Hour), false);
    assert_eq!(state.active_field(), Minute);
    assert!(!state.observe_scroll_focus(Some(Hour)));
    state.apply(
        Event::Input(TimeInputEvent::Edit(Hour, "０８".into())),
        true,
    );
    assert!(state.observe_input_focus(Some(Hour)));
    assert!(state.observe_input_focus(None));
    assert_eq!(state.active_field(), Hour);
    assert_eq!(state.input().raw(Hour), "8");
    state.apply(Event::Show(TimeScrollMode::Scroll), false);
    assert_eq!(state.active_field(), Hour);
    assert_eq!(state.input().validation().candidate(), Some(time(8, 37)));
}

#[test]
fn save_flushes_pending_wheel_but_never_accepts_an_unreleased_drag() {
    let mut state = picker(
        Some(time(0, 9)),
        ClockFormat::TwentyFourHour,
        TimeScrollMode::Scroll,
    );
    wheel(&mut state, Hour, 52., Some(10.));
    let held = state.drag_session(Minute);
    drag(
        &mut state,
        Minute,
        TimeScrollDrag::Move { delta_y: 95. },
        48.,
        true,
    );
    assert_eq!(state.input().validation().candidate(), Some(time(0, 9)));
    assert_eq!(
        input(&mut state, TimeInputEvent::Confirm),
        Commit(time(1, 9))
    );
    assert_eq!(state.column(Minute).preview, None);
    drag_with_session(
        &mut state,
        Minute,
        held,
        TimeScrollDrag::End { delta_y: 95. },
        48.,
        true,
    );
    assert_eq!(state.input().validation().candidate(), Some(time(1, 9)));
    assert_eq!(input(&mut state, TimeInputEvent::Cancel), Dismiss);
    assert_eq!(state.column(Minute).offset, 9.);
}
