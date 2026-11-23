use surface::Rect;
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, time_dial_layout, ButtonKind, ClockFormat, DialEvent, DialMode,
    DialState, Theme, TimeDialLabels, TimeDialOptions, TimeField, TimeInputEffect,
    TimeInputOptions, TimeInputPresentation, TimeInputSession, TimeOfDay,
};

use crate::{
    time_input_demo::{field, labels, locale},
    Message,
};

pub(crate) const SECTION: usize = 2;
const DIAL: &str = "time-dial";
const OPEN: &str = "time-dial-demo-open";

#[derive(Clone)]
pub enum TimeDialDemoEvent {
    Show,
    Back,
    Direction,
    LargeText,
    Locale,
    Format,
    Open,
    Outside,
    Dial(DialEvent),
}

pub(crate) struct TimeDialDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    japanese: bool,
    clock: ClockFormat,
    viewport: Rect,
    draft: Option<DialState>,
    saved: TimeOfDay,
    commits: usize,
    outside_actions: usize,
}

impl Default for TimeDialDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            japanese: false,
            clock: ClockFormat::TwelveHour,
            viewport: Rect::new(0., 0., 960., 760.),
            draft: None,
            saved: TimeOfDay::new(9, 7).unwrap(),
            commits: 0,
            outside_actions: 0,
        }
    }
}

impl TimeDialDemo {
    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme.button_with(
            "time-dial-demo-show",
            "時間錶盤",
            ButtonKind::Outlined,
            false,
            Message::TimeDial(TimeDialDemoEvent::Show),
        )
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.draft = None;
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    fn layout(&self) -> Option<surface_material::TimeDialLayout> {
        time_dial_layout(
            self.viewport.width,
            self.viewport.height,
            self.text_scale(),
            self.clock,
        )
    }

    fn update_fit(&mut self) {
        let fits = self.layout().is_some();
        if let Some(draft) = &mut self.draft {
            draft.apply(DialEvent::SetDialFits(fits));
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let viewport = Rect::new(0., 0., width, height);
        if self.viewport == viewport {
            return false;
        }
        self.viewport = viewport;
        self.update_fit();
        self.visible
    }

    pub(crate) fn observe_focus(&mut self, focused: Option<&str>) -> bool {
        self.draft
            .as_mut()
            .is_some_and(|draft| draft.observe_input_focus(field(focused, DIAL)))
    }

    pub(crate) fn update(&mut self, event: TimeDialDemoEvent) {
        match event {
            TimeDialDemoEvent::Show => self.visible = true,
            TimeDialDemoEvent::Back => self.leave(),
            TimeDialDemoEvent::Direction => self.rtl = !self.rtl,
            TimeDialDemoEvent::LargeText => {
                self.large_text = !self.large_text;
                self.update_fit();
            }
            TimeDialDemoEvent::Locale => self.japanese = !self.japanese,
            TimeDialDemoEvent::Format => {
                self.clock = match self.clock {
                    ClockFormat::TwelveHour => ClockFormat::TwentyFourHour,
                    ClockFormat::TwentyFourHour => ClockFormat::TwelveHour,
                };
            }
            TimeDialDemoEvent::Open => {
                let draft =
                    TimeInputSession::new(Some(self.saved), self.clock, locale(self.japanese));
                self.draft = Some(DialState::new(
                    draft,
                    DialMode::Dial,
                    self.layout().is_some(),
                ));
            }
            TimeDialDemoEvent::Outside => self.outside_actions += 1,
            TimeDialDemoEvent::Dial(event) => {
                if let Some(draft) = &mut self.draft {
                    match draft.apply(event) {
                        TimeInputEffect::Commit(value) => {
                            self.saved = value;
                            self.commits += 1;
                            self.draft = None;
                        }
                        TimeInputEffect::Dismiss => self.draft = None,
                        TimeInputEffect::None => {}
                    }
                }
            }
        }
    }

    pub(crate) fn view(&self, theme: &Theme) -> View<Message> {
        let action = |id, label, event| theme.button(id, label, Message::TimeDial(event));
        let controls = theme.row([
            action("time-dial-demo-back", "返回", TimeDialDemoEvent::Back),
            theme.button("time-dial-demo-theme", "切換配色", Message::Theme),
            action(
                "time-dial-demo-rtl",
                if self.rtl { "左起" } else { "右起" },
                TimeDialDemoEvent::Direction,
            ),
            action(
                "time-dial-demo-large-text",
                if self.large_text {
                    "一般文字"
                } else {
                    "大字"
                },
                TimeDialDemoEvent::LargeText,
            ),
            action(
                "time-dial-demo-locale",
                if self.japanese { "中文" } else { "日本語" },
                TimeDialDemoEvent::Locale,
            ),
            action(
                "time-dial-demo-format",
                if self.clock == ClockFormat::TwelveHour {
                    "24 小時"
                } else {
                    "12 小時"
                },
                TimeDialDemoEvent::Format,
            ),
        ]);
        let actions = theme.row([
            action(OPEN, "開啟錶盤", TimeDialDemoEvent::Open),
            action(
                "time-dial-demo-outside",
                "其他操作",
                TimeDialDemoEvent::Outside,
            ),
        ]);
        let modal = self.draft.as_ref().map(|draft| {
            let (title, dial_labels) = if self.japanese {
                (
                    "時刻を入力",
                    TimeDialLabels {
                        use_input: "文字で入力",
                        use_dial: "文字盤を使う",
                        unfit: "文字盤を表示できる幅と高さが必要です",
                        value_name: &japanese_value_name,
                    },
                )
            } else {
                (
                    "選擇時間",
                    TimeDialLabels {
                        use_input: "文字輸入",
                        use_dial: "使用錶盤",
                        unfit: "空間不足，請用文字輸入",
                        value_name: &chinese_value_name,
                    },
                )
            };
            theme.time_dial(
                TimeDialOptions {
                    input: TimeInputOptions {
                        id: DIAL,
                        title,
                        labels: &labels(self.japanese, self.clock),
                        locale: locale(self.japanese),
                        direction: if self.rtl {
                            LayoutDirection::RightToLeft
                        } else {
                            LayoutDirection::LeftToRight
                        },
                        presentation: TimeInputPresentation::Modal,
                        viewport: self.viewport,
                        disabled: false,
                        restore_focus_to: Some(OPEN),
                    },
                    labels: &dial_labels,
                    text_scale: self.text_scale(),
                },
                draft,
                |event| Message::TimeDial(TimeDialDemoEvent::Dial(event)),
            )
        });
        let saved = theme.text("time-dial-demo-saved", format!("已儲存 {}", self.saved));
        theme.page([actions, controls, saved].into_iter().chain(modal))
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        let draft = self.draft.as_ref().map(|draft| {
            let input = draft.input();
            serde_json::json!({
                "mode": format!("{:?}", draft.mode()),
                "stage": format!("{:?}", draft.field()),
                "fits": draft.dial_fits(),
                "preview": draft.preview().map(|value| serde_json::json!({
                    "field": format!("{:?}", value.field),
                    "value": value.value,
                })),
                "raw_hour": input.raw(TimeField::Hour),
                "raw_minute": input.raw(TimeField::Minute),
                "hour_error": input.displayed_error(TimeField::Hour).map(|e| format!("{e:?}")),
                "minute_error": input.displayed_error(TimeField::Minute).map(|e| format!("{e:?}")),
                "period": input.period().map(|period| format!("{period:?}")),
                "candidate": input.validation().candidate().map(|time| time.to_string()),
            })
        });
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "locale": if self.japanese { "ja" } else { "zh-Hant" },
            "clock": format!("{:?}", self.clock),
            "orientation": self.layout().map(|layout| format!("{layout:?}")),
            "text_scale": self.text_scale(),
            "saved": self.saved.to_string(),
            "commits": self.commits,
            "outside_actions": self.outside_actions,
            "draft": draft,
        })
    }
}

fn chinese_value_name(field: TimeField, value: &str, total: u8) -> String {
    let unit = match field {
        TimeField::Hour => "小時",
        TimeField::Minute => "分鐘",
    };
    format!("{unit} {value}，共 {total} 個選項")
}

fn japanese_value_name(field: TimeField, value: &str, total: u8) -> String {
    let unit = match field {
        TimeField::Hour => "時",
        TimeField::Minute => "分",
    };
    format!("{total} のうち {value} {unit}")
}
