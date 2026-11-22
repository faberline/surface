use std::time::Duration;
use surface::{Align, Insets, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    motion, shape::LayoutDirection, SnackbarDuration, SnackbarId, SnackbarOptions, SnackbarQueue,
    Theme,
};

use crate::Message;

#[derive(Clone, Copy)]
pub enum SnackbarEvent {
    ShowText,
    ShowUndo,
    ShowLong,
    Direction,
    TextScale,
    Act(SnackbarId),
    Dismiss(SnackbarId),
}

#[derive(Clone, Copy)]
enum Sample {
    Text,
    Undo,
    Long,
}

impl Sample {
    #[cfg(target_arch = "wasm32")]
    fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Undo => "undo",
            Self::Long => "long",
        }
    }

    fn trigger(self) -> &'static str {
        match self {
            Self::Text => "snackbar-show-text",
            Self::Undo => "snackbar-show-undo",
            Self::Long => "snackbar-show-long",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Text => "變更已儲存",
            Self::Undo => "專案已移到封存",
            Self::Long => "已將跨部門協作專案移到另一個工作區。你可以復原這項變更。",
        }
    }

    fn action(self) -> Option<&'static str> {
        match self {
            Self::Text => None,
            Self::Undo => Some("復原"),
            Self::Long => Some("復原工作區移動"),
        }
    }
}

#[derive(Default)]
pub(crate) struct SnackbarDemo {
    queue: SnackbarQueue<Sample>,
    now: Duration,
    appeared: Duration,
    progress: f32,
    rtl: bool,
    large_text: bool,
    acted: u32,
    dismissed: u32,
    expired: u32,
}

impl SnackbarDemo {
    fn current(&self) -> Option<(SnackbarId, Sample)> {
        self.queue.current().map(|(id, sample)| (id, *sample))
    }

    fn view_id(id: SnackbarId) -> String {
        format!("gallery-snackbar-{}", id.get())
    }

    fn show(&mut self, sample: Sample) {
        self.queue.push(
            sample,
            SnackbarDuration::Short,
            sample.action().is_some(),
            self.now,
        );
    }

    fn restart_if_changed(&mut self, previous: Option<SnackbarId>) -> bool {
        let next = self.current().map(|(id, _)| id);
        if previous == next {
            return false;
        }
        self.appeared = self.now;
        self.progress = 0.;
        true
    }

    pub(crate) fn update(&mut self, event: SnackbarEvent) {
        let previous = self.current().map(|(id, _)| id);
        match event {
            SnackbarEvent::ShowText => self.show(Sample::Text),
            SnackbarEvent::ShowUndo => self.show(Sample::Undo),
            SnackbarEvent::ShowLong => self.show(Sample::Long),
            SnackbarEvent::Direction => self.rtl = !self.rtl,
            SnackbarEvent::TextScale => self.large_text = !self.large_text,
            SnackbarEvent::Act(id) => {
                if self.queue.act(id, self.now).is_some() {
                    self.acted = self.acted.saturating_add(1);
                }
            }
            SnackbarEvent::Dismiss(id) => {
                if self.queue.dismiss(id, self.now).is_some() {
                    self.dismissed = self.dismissed.saturating_add(1);
                }
            }
        }
        self.restart_if_changed(previous);
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        let time = frame
            .now_ms
            .and_then(|ms| Duration::try_from_secs_f64(ms / 1000.).ok());
        if let Some(now) = time {
            self.now = self.now.max(now);
        }
        let previous = self.current().map(|(id, _)| id);
        if self.queue.expire(self.now).is_some() {
            self.expired = self.expired.saturating_add(1);
        }
        let changed = self.restart_if_changed(previous);
        if self.current().is_none() {
            return AnimationUpdate {
                rebuild: changed,
                request_next: false,
            };
        }
        let elapsed = self.now.saturating_sub(self.appeared).as_secs_f32() * 1000.;
        let progress = if frame.reduced_motion || time.is_none() {
            1.
        } else {
            (elapsed / motion::duration::SHORT3).min(1.)
        };
        let moved = progress != self.progress;
        self.progress = progress;
        AnimationUpdate {
            rebuild: changed || moved,
            request_next: time.is_some() && (progress < 1. || self.queue.deadline().is_some()),
        }
    }

    pub(crate) fn large_text(&self) -> bool {
        self.large_text
    }

    pub(crate) fn card(&self, theme: &Theme) -> View<Message> {
        let show = |sample: Sample, label, event| {
            theme
                .tonal_button(sample.trigger(), label, Message::Snackbar(event))
                .accessible_activation()
        };
        let toggle = |id, label, event| {
            theme
                .button(id, label, Message::Snackbar(event))
                .accessible_activation()
        };
        theme.card(
            "snackbar-card",
            [
                theme.text("snackbar-heading", "短訊息"),
                theme.row([
                    show(Sample::Text, "顯示通知", SnackbarEvent::ShowText),
                    show(Sample::Undo, "可復原通知", SnackbarEvent::ShowUndo),
                    show(Sample::Long, "顯示長通知", SnackbarEvent::ShowLong),
                ]),
                theme.row([
                    toggle(
                        "snackbar-direction",
                        if self.rtl {
                            "由左至右"
                        } else {
                            "由右至左"
                        },
                        SnackbarEvent::Direction,
                    ),
                    toggle(
                        "snackbar-scale",
                        if self.large_text {
                            "一般文字"
                        } else {
                            "放大文字"
                        },
                        SnackbarEvent::TextScale,
                    ),
                ]),
            ],
        )
    }

    pub(crate) fn layer(&self, theme: &Theme) -> Option<View<Message>> {
        let (id, sample) = self.current()?;
        let action = sample
            .action()
            .map(|label| (label, Message::Snackbar(SnackbarEvent::Act(id))));
        let dismiss = action
            .as_ref()
            .map(|_| ("關閉通知", Message::Snackbar(SnackbarEvent::Dismiss(id))));
        let snackbar = theme.snackbar_with(
            &Self::view_id(id),
            sample.message(),
            action,
            dismiss,
            SnackbarOptions {
                direction: if self.rtl {
                    LayoutDirection::RightToLeft
                } else {
                    LayoutDirection::LeftToRight
                },
                action_on_new_line: false,
                restore_focus_to: Some(sample.trigger()),
            },
        );
        let progress = motion::easing::STANDARD_DECELERATE.sample(self.progress);
        let style = Style::column().align(Align::Center).fixed(Insets {
            left: Length::Px(16.),
            right: Length::Px(16.),
            top: Length::Auto,
            bottom: Length::Px(4. + 12. * progress),
        });
        Some(
            View::new(Node::new(Role::Group, "").id("snackbar-host").styled(style))
                .children([snackbar]),
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, text_scale: f32) -> serde_json::Value {
        let current = self.current();
        serde_json::json!({
            "id": current.map(|(id, _)| id.get()),
            "view_id": current.map(|(id, _)| Self::view_id(id)),
            "visible": current.map(|(_, sample)| sample.name()),
            "pending": self.queue.pending_len(),
            "acted": self.acted,
            "dismissed": self.dismissed,
            "expired": self.expired,
            "rtl": self.rtl,
            "text_scale": text_scale,
        })
    }
}
