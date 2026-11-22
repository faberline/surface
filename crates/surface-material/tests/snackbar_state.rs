use std::time::Duration;

use surface_material::{SnackbarDuration, SnackbarQueue};

const fn secs(value: u64) -> Duration {
    Duration::from_secs(value)
}

#[test]
fn waiting_items_get_their_full_duration_even_after_a_late_wake() {
    let mut queue = SnackbarQueue::default();
    let first = queue
        .push("first", SnackbarDuration::Short, false, secs(1))
        .unwrap();
    let second = queue
        .push("second", SnackbarDuration::Long, false, secs(2))
        .unwrap();
    let third = queue
        .push("third", SnackbarDuration::Indefinite, false, secs(2))
        .unwrap();

    assert_eq!(queue.current(), Some((first, &"first")));
    assert_eq!(queue.pending_len(), 2);
    assert_eq!(queue.deadline(), Some(secs(5)));
    assert_eq!(queue.expire(secs(4)), None);

    assert_eq!(queue.expire(secs(8)), Some("first"));
    assert_eq!(queue.expire(secs(8)), None);
    assert_eq!(queue.current(), Some((second, &"second")));
    assert_eq!(queue.pending_len(), 1);
    assert_eq!(queue.deadline(), Some(secs(18)));

    assert_eq!(queue.expire(secs(18)), Some("second"));
    assert_eq!(queue.current(), Some((third, &"third")));
    assert_eq!(queue.pending_len(), 0);
    assert_eq!(queue.deadline(), None);
    assert_eq!(queue.expire(secs(99)), None);
    assert_eq!(queue.dismiss(third, secs(99)), Some("third"));
    assert_eq!(queue.current(), None);
}

#[test]
fn actionable_items_wait_for_explicit_action_or_dismissal() {
    let mut queue = SnackbarQueue::default();
    let action = queue
        .push("undo edit", SnackbarDuration::Short, true, secs(1))
        .unwrap();
    let dismiss = queue
        .push("undo move", SnackbarDuration::Long, true, secs(1))
        .unwrap();
    let passive = queue
        .push("saved", SnackbarDuration::Short, false, secs(2))
        .unwrap();

    assert_eq!(queue.deadline(), None);
    assert_eq!(queue.expire(secs(20)), None);
    assert!(!queue.extend_timeout(action, secs(40), secs(20)));
    assert_eq!(queue.act(dismiss, secs(20)), None);
    assert_eq!(queue.act(action, secs(20)), Some("undo edit"));
    assert_eq!(queue.act(action, secs(20)), None);
    assert_eq!(queue.current(), Some((dismiss, &"undo move")));
    assert_eq!(queue.dismiss(dismiss, secs(20)), Some("undo move"));
    assert_eq!(queue.dismiss(dismiss, secs(20)), None);
    assert_eq!(queue.act(passive, secs(20)), None);
    assert_eq!(queue.current(), Some((passive, &"saved")));
    assert_eq!(queue.deadline(), Some(secs(24)));
}

#[test]
fn older_timestamps_and_old_ids_do_not_change_current_item() {
    let mut queue = SnackbarQueue::default();
    let old = queue
        .push(1, SnackbarDuration::Long, false, secs(1))
        .unwrap();
    assert_eq!(queue.dismiss(old, secs(2)), Some(1));
    let current = queue
        .push(2, SnackbarDuration::Short, false, secs(5))
        .unwrap();
    assert!(old.get() < current.get());

    assert_eq!(queue.dismiss(old, secs(200)), None);
    assert!(!queue.extend_timeout(old, secs(200), secs(200)));
    assert_eq!(queue.dismiss(current, secs(4)), None);
    assert!(!queue.extend_timeout(current, secs(20), secs(4)));
    assert_eq!(queue.expire(secs(4)), None);
    assert_eq!(queue.push(3, SnackbarDuration::Short, false, secs(4)), None);
    assert_eq!(queue.current(), Some((current, &2)));
    assert_eq!(queue.pending_len(), 0);
    assert_eq!(queue.deadline(), Some(secs(9)));
    assert_eq!(queue.expire(secs(9)), Some(2));
}

#[test]
fn timeout_extensions_are_monotonic_and_same_time_repeats_are_harmless() {
    let mut queue = SnackbarQueue::default();
    let id = queue
        .push("notification", SnackbarDuration::Short, false, secs(2))
        .unwrap();
    assert_eq!(queue.deadline(), Some(secs(6)));
    assert!(queue.extend_timeout(id, secs(9), secs(3)));
    assert_eq!(queue.deadline(), Some(secs(12)));
    assert!(!queue.extend_timeout(id, secs(9), secs(3)));
    assert!(!queue.extend_timeout(id, secs(1), secs(4)));
    assert_eq!(queue.expire(secs(6)), None);
    assert!(!queue.extend_timeout(id, secs(20), secs(12)));
    assert_eq!(queue.expire(secs(12)), Some("notification"));
    assert_eq!(queue.expire(secs(12)), None);
}

#[test]
fn an_unrepresentable_deadline_keeps_a_manual_dismissal_path() {
    let mut queue = SnackbarQueue::default();
    let then = Duration::MAX - secs(1);
    let id = queue
        .push("timed", SnackbarDuration::Short, false, then)
        .unwrap();
    assert_eq!(queue.deadline(), None);
    assert_eq!(queue.expire(Duration::MAX), None);
    assert_eq!(queue.dismiss(id, Duration::MAX), Some("timed"));
}
