const HOVER_DELAY_MS: f64 = 500.;
const LONG_PRESS_MS: f64 = 500.;
const TOUCH_VISIBLE_MS: f64 = 1500.;

/// The current tooltip owners and activity for one application frame.
///
/// Supply only enabled anchors in the active scope. A held touch is an
/// eligible anchor while it stays within the caller's movement tolerance.
/// `popup_active` covers pointer or focus in the visible popup. Callers can
/// include the gap between its bounds and the owner in the pointer region.
#[derive(Clone, Copy, Debug, Default)]
pub struct TooltipInput<'a> {
    pub now_ms: Option<f64>,
    pub hovered_anchor: Option<&'a str>,
    pub focused_anchor: Option<&'a str>,
    pub popup_active: bool,
    pub touching: bool,
    pub touch_anchor: Option<&'a str>,
    pub touch_started_ms: Option<f64>,
    pub touch_cancelled: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TooltipUpdate {
    pub changed: bool,
    pub request_next: bool,
    pub suppress_touch_activation: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Trigger {
    Owner,
    Touch { expires_ms: Option<f64> },
}

struct Visible {
    anchor: String,
    trigger: Trigger,
}

/// Controls one visible tooltip among the application's stable anchor IDs.
///
/// Focus and deliberate activation open immediately. Mouse hover waits
/// 500 ms and stays open while the owner or popup is active. A touch
/// long press uses 500 ms and consumes the corresponding release.
/// The touch popup stays for 1500 ms after release.
#[derive(Default)]
pub struct TooltipState {
    visible: Option<Visible>,
    pending_hover: Option<(String, f64)>,
    dismissed_anchor: Option<String>,
    touch_focus: Option<String>,
    held_anchor: Option<String>,
    was_touching: bool,
    opened_during_touch: bool,
}

impl TooltipState {
    pub fn active(&self) -> Option<&str> {
        self.visible.as_ref().map(|visible| visible.anchor.as_str())
    }

    /// Open an interactive tooltip after the user activates its anchor.
    pub fn activate(&mut self, anchor: &str) {
        self.dismissed_anchor = None;
        self.show(anchor, Trigger::Owner);
    }

    /// Dismiss the popup and wait for that anchor to be left before
    /// allowing automatic opening again.
    pub fn dismiss(&mut self) -> bool {
        let pending = self.pending_hover.take();
        if let Some(visible) = self.visible.take() {
            self.dismissed_anchor = Some(visible.anchor);
            true
        } else if let Some((anchor, _)) = pending {
            self.dismissed_anchor = Some(anchor);
            true
        } else {
            false
        }
    }

    /// Advance hover, focus and touch. Propagate `suppress_touch_activation`
    /// to the runtime for the contact that opened a tooltip.
    pub fn update(&mut self, input: TooltipInput<'_>) -> TooltipUpdate {
        let before = self.active().map(str::to_owned);
        let now = input.now_ms.filter(|now| now.is_finite());
        if self.dismissed_anchor.as_deref().is_some_and(|dismissed| {
            [
                input.hovered_anchor,
                input.focused_anchor,
                input.touch_anchor,
            ]
            .into_iter()
            .all(|anchor| anchor != Some(dismissed))
        }) {
            self.dismissed_anchor = None;
        }
        let (request_next, suppress_touch_activation) = if input.touch_cancelled {
            self.held_anchor = None;
            self.was_touching = false;
            if self.is_touch_popup() {
                self.dismiss();
            }
            (false, false)
        } else if input.touching {
            self.advance_touch(input, now)
        } else {
            self.held_anchor = None;
            self.was_touching = false;
            (self.advance_idle(input, now), false)
        };
        TooltipUpdate {
            changed: before.as_deref() != self.active(),
            request_next,
            suppress_touch_activation,
        }
    }

    fn advance_touch(&mut self, input: TooltipInput<'_>, now: Option<f64>) -> (bool, bool) {
        if !self.was_touching {
            self.was_touching = true;
            self.opened_during_touch = false;
            self.pending_hover = None;
            self.held_anchor = input
                .touch_anchor
                .filter(|anchor| Some(*anchor) != self.dismissed_anchor.as_deref())
                .map(str::to_owned);
            self.touch_focus = input.touch_anchor.map(str::to_owned);
            if !input.popup_active {
                self.visible = None;
            }
        }
        let opened_touch = self.opened_during_touch;
        if input.touch_anchor != self.held_anchor.as_deref() {
            self.held_anchor = None;
        }
        let Some(anchor) = input.touch_anchor.filter(|_| self.held_anchor.is_some()) else {
            return (false, opened_touch);
        };
        if opened_touch {
            return (false, true);
        }
        let Some((now, start)) = now
            .zip(input.touch_started_ms)
            .filter(|(_, start)| start.is_finite())
        else {
            return (false, false);
        };
        if now - start < LONG_PRESS_MS {
            return (true, false);
        }
        self.show(anchor, Trigger::Touch { expires_ms: None });
        self.opened_during_touch = true;
        (false, true)
    }

    fn advance_idle(&mut self, input: TooltipInput<'_>, now: Option<f64>) -> bool {
        if self.touch_focus.as_deref() != input.focused_anchor {
            self.touch_focus = None;
        }
        if let Some(Visible {
            trigger: Trigger::Touch { expires_ms },
            ..
        }) = &mut self.visible
        {
            if let Some(now) = now {
                let expires = *expires_ms.get_or_insert(now + TOUCH_VISIBLE_MS);
                if now < expires || input.popup_active {
                    return now < expires;
                }
            }
            self.dismiss();
            return false;
        }
        let focus = input
            .focused_anchor
            .filter(|anchor| Some(*anchor) != self.touch_focus.as_deref());
        let candidate = focus.or(input.hovered_anchor);
        if self
            .visible
            .as_ref()
            .is_some_and(|visible| input.popup_active || candidate == Some(visible.anchor.as_str()))
        {
            return false;
        }
        self.visible = None;
        let Some(anchor) =
            candidate.filter(|anchor| Some(*anchor) != self.dismissed_anchor.as_deref())
        else {
            self.pending_hover = None;
            return false;
        };
        if let Some(now) = now.filter(|_| focus.is_none()) {
            let deadline = match &mut self.pending_hover {
                Some((pending, deadline)) if pending == anchor => *deadline,
                pending => {
                    let deadline = now + HOVER_DELAY_MS;
                    *pending = Some((anchor.to_owned(), deadline));
                    deadline
                }
            };
            if now < deadline {
                return true;
            }
        }
        self.show(anchor, Trigger::Owner);
        false
    }

    fn is_touch_popup(&self) -> bool {
        self.visible
            .as_ref()
            .is_some_and(|visible| matches!(visible.trigger, Trigger::Touch { .. }))
    }

    fn show(&mut self, anchor: &str, trigger: Trigger) {
        self.visible = Some(Visible {
            anchor: anchor.to_owned(),
            trigger,
        });
        self.pending_hover = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hover(anchor: &str, ms: f64) -> TooltipInput<'_> {
        TooltipInput {
            hovered_anchor: Some(anchor),
            now_ms: Some(ms),
            ..Default::default()
        }
    }

    fn touch(ms: f64, anchor: Option<&str>) -> TooltipInput<'_> {
        TooltipInput {
            touching: true,
            touch_anchor: anchor,
            touch_started_ms: Some(0.),
            now_ms: Some(ms),
            focused_anchor: Some("first"),
            ..Default::default()
        }
    }

    #[test]
    fn hover_waits_stays_while_popup_is_active_and_rearms_after_dismissal() {
        let mut state = TooltipState::default();
        assert!(state.update(hover("first", 0.)).request_next);
        assert!(state.update(hover("first", 499.)).request_next);
        assert!(state.update(hover("first", 500.)).changed);
        assert!(
            !state
                .update(TooltipInput {
                    now_ms: Some(5000.),
                    popup_active: true,
                    ..Default::default()
                })
                .changed
        );
        assert!(state.dismiss());
        assert!(!state.update(hover("first", 6000.)).request_next);
        state.update(TooltipInput::default());
        assert!(state.update(hover("first", 7000.)).request_next);
    }

    #[test]
    fn focus_and_explicit_activation_open_without_a_clock() {
        let mut state = TooltipState::default();
        state.update(TooltipInput {
            focused_anchor: Some("first"),
            ..Default::default()
        });
        assert_eq!(state.active(), Some("first"));
        state.activate("second");
        state.update(TooltipInput {
            focused_anchor: Some("second"),
            ..Default::default()
        });
        assert_eq!(state.active(), Some("second"));
        assert!(state.update(TooltipInput::default()).changed);
    }

    #[test]
    fn a_short_tap_and_a_moved_touch_stay_closed() {
        let mut state = TooltipState::default();
        assert!(
            !state
                .update(touch(0., Some("first")))
                .suppress_touch_activation
        );
        assert!(!state.update(touch(350., None)).suppress_touch_activation);
        assert!(
            !state
                .update(touch(800., Some("first")))
                .suppress_touch_activation
        );
        state.update(TooltipInput {
            focused_anchor: Some("first"),
            ..Default::default()
        });
        assert_eq!(state.active(), None);
    }

    #[test]
    fn long_press_consumes_release_and_lingers_until_touch_timeout() {
        let mut state = TooltipState::default();
        assert!(state.update(touch(0., Some("first"))).request_next);
        assert!(
            state
                .update(touch(500., Some("first")))
                .suppress_touch_activation
        );
        assert!(
            state
                .update(TooltipInput {
                    now_ms: Some(600.),
                    ..Default::default()
                })
                .request_next
        );
        assert_eq!(state.active(), Some("first"));
        state.update(TooltipInput {
            now_ms: Some(2100.),
            ..Default::default()
        });
        assert_eq!(state.active(), None);
    }

    #[test]
    fn cancelled_contact_dismisses_an_already_shown_tooltip() {
        let mut state = TooltipState::default();
        state.update(touch(0., Some("first")));
        state.update(touch(500., Some("first")));
        assert!(
            state
                .update(TooltipInput {
                    touch_cancelled: true,
                    ..Default::default()
                })
                .changed
        );
        assert_eq!(state.active(), None);
    }
}
