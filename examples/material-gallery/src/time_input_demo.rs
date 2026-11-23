use surface::Rect;
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, ButtonKind, ClockFormat, PeriodPosition, Theme, TimeField,
    TimeInputEffect, TimeInputError, TimeInputEvent, TimeInputLabels, TimeInputLocale,
    TimeInputOptions, TimeInputPresentation, TimeInputSession, TimeOfDay,
};

use crate::Message;

pub(crate) const SECTION: usize = 2;
const INLINE: &str = "time-input-inline";
const MODAL: &str = "time-input-modal";
const OPEN: &str = "time-input-demo-modal-open";

#[derive(Clone)]
pub enum TimeInputDemoEvent {
    Show,
    Back,
    Direction,
    LargeText,
    Locale,
    Format,
    Open,
    Outside,
    Inline(TimeInputEvent),
    Modal(TimeInputEvent),
}

pub(crate) struct TimeInputDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    japanese: bool,
    clock: ClockFormat,
    viewport: Rect,
    inline: TimeInputSession,
    inline_committed: TimeOfDay,
    inline_commits: usize,
    modal: Option<TimeInputSession>,
    modal_committed: TimeOfDay,
    modal_commits: usize,
    outside_actions: usize,
}

impl Default for TimeInputDemo {
    fn default() -> Self {
        let clock = ClockFormat::TwelveHour;
        let midnight = TimeOfDay::new(0, 5).unwrap();
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            japanese: false,
            clock,
            viewport: Rect::new(0., 0., 960., 760.),
            inline: TimeInputSession::new(Some(midnight), clock, locale(false)),
            inline_committed: midnight,
            inline_commits: 0,
            modal: None,
            modal_committed: TimeOfDay::new(9, 7).unwrap(),
            modal_commits: 0,
            outside_actions: 0,
        }
    }
}

impl TimeInputDemo {
    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme.button_with(
            "time-input-demo-show",
            "時間輸入",
            ButtonKind::Outlined,
            false,
            Message::TimeInput(TimeInputDemoEvent::Show),
        )
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        if self.visible {
            self.visible = false;
            self.modal = None;
            self.reset_inline();
        }
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let viewport = Rect::new(0., 0., width, height);
        let changed = self.viewport != viewport;
        self.viewport = viewport;
        changed && self.visible
    }

    pub(crate) fn observe_focus(&mut self, focused: Option<&str>) -> bool {
        if !self.visible {
            return false;
        }
        let inline = self.inline.observe_input_focus(field(focused, INLINE));
        let modal = self
            .modal
            .as_mut()
            .is_some_and(|session| session.observe_input_focus(field(focused, MODAL)));
        inline || modal
    }

    fn reset_inline(&mut self) {
        self.inline = TimeInputSession::new(
            Some(self.inline_committed),
            self.clock,
            locale(self.japanese),
        );
    }

    pub(crate) fn update(&mut self, event: TimeInputDemoEvent) {
        match event {
            TimeInputDemoEvent::Show => self.visible = true,
            TimeInputDemoEvent::Back => self.leave(),
            TimeInputDemoEvent::Direction => self.rtl = !self.rtl,
            TimeInputDemoEvent::LargeText => self.large_text = !self.large_text,
            TimeInputDemoEvent::Locale => {
                self.japanese = !self.japanese;
                self.reset_inline();
            }
            TimeInputDemoEvent::Format => {
                self.clock = match self.clock {
                    ClockFormat::TwelveHour => ClockFormat::TwentyFourHour,
                    ClockFormat::TwentyFourHour => ClockFormat::TwelveHour,
                };
                self.reset_inline();
            }
            TimeInputDemoEvent::Open => {
                self.modal = Some(TimeInputSession::new(
                    Some(self.modal_committed),
                    self.clock,
                    locale(self.japanese),
                ));
            }
            TimeInputDemoEvent::Outside => self.outside_actions += 1,
            TimeInputDemoEvent::Inline(event) => match self.inline.apply(event) {
                TimeInputEffect::Commit(value) => {
                    self.inline_committed = value;
                    self.inline_commits += 1;
                }
                TimeInputEffect::Dismiss => self.reset_inline(),
                TimeInputEffect::None => {}
            },
            TimeInputDemoEvent::Modal(event) => {
                if let Some(session) = &mut self.modal {
                    match session.apply(event) {
                        TimeInputEffect::Commit(value) => {
                            self.modal_committed = value;
                            self.modal_commits += 1;
                            self.modal = None;
                        }
                        TimeInputEffect::Dismiss => self.modal = None,
                        TimeInputEffect::None => {}
                    }
                }
            }
        }
    }

    pub(crate) fn view(&self, theme: &Theme) -> View<Message> {
        let action = |id, label, event| theme.button(id, label, Message::TimeInput(event));
        let controls = theme.row([
            action("time-input-demo-back", "返回", TimeInputDemoEvent::Back),
            theme.button("time-input-demo-theme", "切換配色", Message::Theme),
            action(
                "time-input-demo-rtl",
                if self.rtl { "左起" } else { "右起" },
                TimeInputDemoEvent::Direction,
            ),
            action(
                "time-input-demo-large-text",
                if self.large_text {
                    "一般文字"
                } else {
                    "大字"
                },
                TimeInputDemoEvent::LargeText,
            ),
            action(
                "time-input-demo-locale",
                if self.japanese { "中文" } else { "日本語" },
                TimeInputDemoEvent::Locale,
            ),
            action(
                "time-input-demo-format",
                if self.clock == ClockFormat::TwelveHour {
                    "24 小時"
                } else {
                    "12 小時"
                },
                TimeInputDemoEvent::Format,
            ),
        ]);
        let actions = theme.row([
            action(OPEN, "開啟時間選擇器", TimeInputDemoEvent::Open),
            action(
                "time-input-demo-outside",
                "其他操作",
                TimeInputDemoEvent::Outside,
            ),
        ]);
        let locale = locale(self.japanese);
        let labels = labels(self.japanese, self.clock);
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let title = if self.japanese {
            "時刻を入力"
        } else {
            "選擇時間"
        };
        let options = |id, presentation, viewport, restore_focus_to| TimeInputOptions {
            id,
            title,
            labels: &labels,
            locale,
            direction,
            presentation,
            viewport,
            disabled: false,
            restore_focus_to,
        };
        let inline_width = (self.viewport.width - 56.).max(0.);
        let inline = theme.time_input(
            options(
                INLINE,
                TimeInputPresentation::Inline,
                Rect::new(0., 0., inline_width, self.viewport.height),
                None,
            ),
            &self.inline,
            |event| Message::TimeInput(TimeInputDemoEvent::Inline(event)),
        );
        let submit = theme
            .button_with(
                "time-input-demo-inline-submit",
                "儲存行內時間",
                ButtonKind::Filled,
                false,
                Message::TimeInput(TimeInputDemoEvent::Inline(TimeInputEvent::Confirm)),
            )
            .protect_composition_on_press();
        let modal = self.modal.as_ref().map(|session| {
            theme.time_input(
                options(
                    MODAL,
                    TimeInputPresentation::Modal,
                    self.viewport,
                    Some(OPEN),
                ),
                session,
                |event| Message::TimeInput(TimeInputDemoEvent::Modal(event)),
            )
        });
        let inline_saved = theme.text(
            "time-input-demo-inline-value",
            format!("已儲存 {}", self.inline_committed),
        );
        let modal_saved = theme.text(
            "time-input-demo-modal-value",
            format!("已儲存 {}", self.modal_committed),
        );
        theme.page(
            [
                controls,
                actions,
                modal_saved,
                inline,
                theme.row([submit, inline_saved]),
            ]
            .into_iter()
            .chain(modal),
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        let draft = |session: &TimeInputSession| {
            let hour = TimeField::Hour;
            let minute = TimeField::Minute;
            serde_json::json!({
                "raw_hour": session.raw(hour),
                "raw_minute": session.raw(minute),
                "period": session.period().map(|p| format!("{p:?}")),
                "candidate": session.validation().candidate().map(|value| value.to_string()),
                "hour_error": session.displayed_error(hour).map(|e| format!("{e:?}")),
                "minute_error": session.displayed_error(minute).map(|e| format!("{e:?}")),
            })
        };
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "locale": if self.japanese { "ja" } else { "zh-Hant" },
            "format": format!("{:?}", self.clock),
            "outside_actions": self.outside_actions,
            "inline_committed": self.inline_committed.to_string(),
            "inline_commits": self.inline_commits,
            "modal_committed": self.modal_committed.to_string(),
            "modal_commits": self.modal_commits,
            "inline": draft(&self.inline),
            "modal": self.modal.as_ref().map(draft),
        })
    }
}

fn field(focused: Option<&str>, prefix: &str) -> Option<TimeField> {
    match focused?.strip_prefix(prefix)? {
        "-hour-input" => Some(TimeField::Hour),
        "-minute-input" => Some(TimeField::Minute),
        _ => None,
    }
}

fn locale(japanese: bool) -> TimeInputLocale<'static> {
    if japanese {
        TimeInputLocale::new("午前", "午後", ":")
            .with_period_position(PeriodPosition::Before, " ")
            .with_digits(['０', '１', '２', '３', '４', '５', '６', '７', '８', '９'])
            .expect("distinct full-width digits")
    } else {
        TimeInputLocale::new("上午", "下午", ":").with_period_position(PeriodPosition::Before, " ")
    }
}

fn labels(japanese: bool, clock: ClockFormat) -> TimeInputLabels<'static> {
    let twelve = clock == ClockFormat::TwelveHour;
    if japanese {
        TimeInputLabels {
            hour: "時",
            minute: "分",
            period: "午前と午後",
            hour_hint: if twelve {
                "１–１２"
            } else {
                "０–２３"
            },
            minute_hint: "０–５９",
            confirm: "保存",
            cancel: "取り消し",
            error_text: &japanese_error,
        }
    } else {
        TimeInputLabels {
            hour: "小時",
            minute: "分鐘",
            period: "時段",
            hour_hint: if twelve { "1–12" } else { "0–23" },
            minute_hint: "0–59",
            confirm: "儲存",
            cancel: "取消",
            error_text: &chinese_error,
        }
    }
}

fn chinese_error(field: TimeField, error: TimeInputError, clock: ClockFormat) -> String {
    match error {
        TimeInputError::Required => match field {
            TimeField::Hour => "請輸入小時",
            TimeField::Minute => "請輸入分鐘",
        },
        TimeInputError::ExpectedOneOrTwoDigits => "請輸入一至兩位數字",
        TimeInputError::OutOfRange => match (field, clock) {
            (TimeField::Minute, _) => "分鐘須介於 0 與 59",
            (TimeField::Hour, ClockFormat::TwelveHour) => "小時須介於 1 與 12",
            (TimeField::Hour, ClockFormat::TwentyFourHour) => "小時須介於 0 與 23",
        },
    }
    .into()
}

fn japanese_error(field: TimeField, error: TimeInputError, clock: ClockFormat) -> String {
    match error {
        TimeInputError::Required => match field {
            TimeField::Hour => "時を入力してください",
            TimeField::Minute => "分を入力してください",
        },
        TimeInputError::ExpectedOneOrTwoDigits => "１けたか２けたで入力してください",
        TimeInputError::OutOfRange => match (field, clock) {
            (TimeField::Minute, _) => "分は０から５９までです",
            (TimeField::Hour, ClockFormat::TwelveHour) => "時は１から１２までです",
            (TimeField::Hour, ClockFormat::TwentyFourHour) => "時は０から２３までです",
        },
    }
    .into()
}
