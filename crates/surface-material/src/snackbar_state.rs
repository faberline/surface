//! Caller-owned state for a single-visible-item snackbar queue.

use std::{collections::VecDeque, time::Duration};

/// The time a passive snackbar stays visible once it reaches the front.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnackbarDuration {
    /// Four seconds.
    Short,
    /// Ten seconds.
    Long,
    /// Requires explicit dismissal.
    Indefinite,
}

impl SnackbarDuration {
    fn timeout(self) -> Option<Duration> {
        match self {
            Self::Short => Some(Duration::from_secs(4)),
            Self::Long => Some(Duration::from_secs(10)),
            Self::Indefinite => None,
        }
    }
}

/// Identifies one notification within its queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnackbarId(u64);

impl SnackbarId {
    /// The queue-local number, suitable for stable IDs within a host view.
    pub const fn get(self) -> u64 {
        self.0
    }
}

struct Entry<T> {
    id: SnackbarId,
    value: T,
    duration: SnackbarDuration,
    has_action: bool,
}

struct Active<T> {
    entry: Entry<T>,
    deadline: Option<Duration>,
}

/// Keep at most one visible snackbar. Waiting items appear in insertion order.
///
/// Supply elapsed time from the same monotonic origin to every operation.
/// The queue ignores older timestamps and does not install a timer or move
/// focus. Schedule a wake for [`Self::deadline`] when the host needs one.
pub struct SnackbarQueue<T> {
    active: Option<Active<T>>,
    pending: VecDeque<Entry<T>>,
    last_time: Duration,
    last_id: u64,
}

impl<T> Default for SnackbarQueue<T> {
    fn default() -> Self {
        Self {
            active: None,
            pending: VecDeque::new(),
            last_time: Duration::ZERO,
            last_id: 0,
        }
    }
}

impl<T> SnackbarQueue<T> {
    /// Enqueue a snackbar. Its timeout starts when it becomes visible.
    /// An actionable snackbar stays visible until a user explicitly acts or
    /// dismisses it. Returns `None` for an older timestamp or exhausted IDs.
    pub fn push(
        &mut self,
        value: T,
        duration: SnackbarDuration,
        has_action: bool,
        now: Duration,
    ) -> Option<SnackbarId> {
        let id = SnackbarId(self.last_id.checked_add(1)?);
        if !self.accept_time(now) {
            return None;
        }
        self.last_id = id.0;
        self.pending.push_back(Entry {
            id,
            value,
            duration,
            has_action,
        });
        self.activate_next(now);
        Some(id)
    }

    /// The visible notification and the ID to put in its interaction messages.
    pub fn current(&self) -> Option<(SnackbarId, &T)> {
        self.active
            .as_ref()
            .map(|active| (active.entry.id, &active.entry.value))
    }

    /// The number of items waiting behind the visible notification.
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// The visible item's deadline, if it has a finite timeout.
    pub fn deadline(&self) -> Option<Duration> {
        self.active.as_ref().and_then(|active| active.deadline)
    }

    /// Return the value for the action on the currently visible snackbar.
    /// Run the application action only when this method returns a value.
    pub fn act(&mut self, id: SnackbarId, now: Duration) -> Option<T> {
        if !self.active.as_ref()?.entry.has_action {
            return None;
        }
        self.dismiss(id, now)
    }

    /// Manually remove the matching visible snackbar without taking its action.
    pub fn dismiss(&mut self, id: SnackbarId, now: Duration) -> Option<T> {
        if self.current()?.0 != id || !self.accept_time(now) {
            return None;
        }
        self.finish(now)
    }

    /// Remove a timed notification at or after its deadline, if due.
    /// The next notification gets its full duration even after a late wakeup.
    pub fn expire(&mut self, now: Duration) -> Option<T> {
        if !self.accept_time(now) || self.deadline().is_none_or(|deadline| now < deadline) {
            return None;
        }
        self.finish(now)
    }

    /// Give a visible timed snackbar at least `minimum_remaining` from `now`.
    /// The host can supply a longer reading time for accessibility. Returns
    /// true only when the deadline changes. The timeout can only increase.
    pub fn extend_timeout(
        &mut self,
        id: SnackbarId,
        minimum_remaining: Duration,
        now: Duration,
    ) -> bool {
        if self.current().is_none_or(|(current, _)| current != id) || !self.accept_time(now) {
            return false;
        }
        let Some(deadline) = self
            .active
            .as_mut()
            .and_then(|active| active.deadline.as_mut())
        else {
            return false;
        };
        let extended = now.saturating_add(minimum_remaining);
        if now >= *deadline || extended <= *deadline {
            return false;
        }
        *deadline = extended;
        true
    }

    fn accept_time(&mut self, now: Duration) -> bool {
        if now < self.last_time {
            return false;
        }
        self.last_time = now;
        true
    }

    fn finish(&mut self, now: Duration) -> Option<T> {
        let completed = self.active.take()?;
        self.activate_next(now);
        Some(completed.entry.value)
    }

    fn activate_next(&mut self, now: Duration) {
        if self.active.is_none() {
            self.active = self.pending.pop_front().map(|entry| Active {
                deadline: if entry.has_action {
                    None
                } else {
                    // Keep a manual dismissal path if the deadline overflows.
                    entry
                        .duration
                        .timeout()
                        .and_then(|timeout| now.checked_add(timeout))
                },
                entry,
            });
        }
    }
}
