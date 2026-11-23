use surface::Rect;
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, time_dial_baseline_layout, time_dial_layout, ButtonKind, ClockFormat,
    DialEvent, DialHandMotion, DialMode, DialState, Theme, TimeDialLabels, TimeDialOptions,
    TimeField, TimeInputEffect, TimeInputOptions, TimeInputPresentation, TimeInputSession,
    TimeOfDay,
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
    ShowBaseline,
    Back,
    Direction,
    LargeText,
    Locale,
    Format,
    Open,
    Outside,
    Dial(DialEvent),
}

struct OpenDial {
    state: DialState,
    hand: DialHandMotion,
}

impl OpenDial {
    fn apply(&mut self, event: DialEvent, baseline: bool) -> TimeInputEffect {
        if baseline {
            self.state.apply(event)
        } else {
            self.hand.apply(&mut self.state, event)
        }
    }
}

pub(crate) struct TimeDialDemo {
    visible: bool,
    baseline: bool,
    rtl: bool,
    large_text: bool,
    japanese: bool,
    clock: ClockFormat,
    viewport: Rect,
    draft: Option<OpenDial>,
    saved: TimeOfDay,
    commits: usize,
    outside_actions: usize,
}

impl Default for TimeDialDemo {
    fn default() -> Self {
        Self {
            visible: false,
            baseline: false,
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

    pub(crate) fn show_baseline_button(theme: &Theme) -> View<Message> {
        theme.button_with(
            "time-dial-baseline-demo-show",
            "Baseline 錶盤",
            ButtonKind::Outlined,
            false,
            Message::TimeDial(TimeDialDemoEvent::ShowBaseline),
        )
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.baseline = false;
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
        let plan = if self.baseline {
            time_dial_baseline_layout
        } else {
            time_dial_layout
        };
        plan(
            self.viewport.width,
            self.viewport.height,
            self.text_scale(),
            self.clock,
        )
    }

    fn update_fit(&mut self) {
        let fits = self.layout().is_some();
        if let Some(draft) = &mut self.draft {
            draft.apply(DialEvent::SetDialFits(fits), self.baseline);
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
        self.draft.as_mut().is_some_and(|draft| {
            let field = field(focused, DIAL);
            if self.baseline {
                draft.state.observe_baseline_input_focus(field)
            } else {
                draft.state.observe_input_focus(field)
            }
        })
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if self.baseline {
            return AnimationUpdate::default();
        }
        self.draft
            .as_mut()
            .map_or_else(AnimationUpdate::default, |draft| {
                draft.hand.advance(frame.now_ms, frame.reduced_motion)
            })
    }

    pub(crate) fn update(&mut self, event: TimeDialDemoEvent) {
        match event {
            TimeDialDemoEvent::Show | TimeDialDemoEvent::ShowBaseline => {
                self.visible = true;
                self.baseline = matches!(event, TimeDialDemoEvent::ShowBaseline);
            }
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
                let input =
                    TimeInputSession::new(Some(self.saved), self.clock, locale(self.japanese));
                let state = DialState::new(input, DialMode::Dial, self.layout().is_some());
                let hand = DialHandMotion::new(&state);
                self.draft = Some(OpenDial { state, hand });
            }
            TimeDialDemoEvent::Outside => self.outside_actions += 1,
            TimeDialDemoEvent::Dial(event) => {
                if let Some(draft) = &mut self.draft {
                    match draft.apply(event, self.baseline) {
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
            let options = TimeDialOptions {
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
            };
            let emit = |event| Message::TimeDial(TimeDialDemoEvent::Dial(event));
            if self.baseline {
                theme.time_dial_baseline(options, &draft.state, emit)
            } else {
                theme.time_dial_with_motion(options, &draft.state, &draft.hand, emit)
            }
        });
        let saved = theme.text("time-dial-demo-saved", format!("已儲存 {}", self.saved));
        theme.page([actions, controls, saved].into_iter().chain(modal))
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        let draft = self.draft.as_ref().map(|draft| {
            let draft = &draft.state;
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
        let mut value = serde_json::json!({
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
        });
        if self.baseline {
            value["variant"] = serde_json::json!("Baseline");
        }
        value
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallery;
    use surface::TextMeasurer;
    use surface_app::{EditKey, Event, Key, KeyPress, Runtime};

    struct Metrics;

    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
            (text.chars().count() as f32 * size * 0.6, size * 1.5)
        }
    }

    fn draw(runtime: &mut Runtime<Gallery>, time: f64) {
        runtime.set_time(time);
        runtime.render(960., 820., &mut Metrics).unwrap();
    }

    fn press(runtime: &mut Runtime<Gallery>, key: Key) {
        let press = KeyPress::new(key, Default::default());
        assert!(runtime.key(&press).unwrap(), "{press:?}");
    }

    fn center(runtime: &Runtime<Gallery>, id: &str) -> (f32, f32) {
        let node = runtime.dom().by_id(id).unwrap();
        let bounds = runtime.computed().unwrap().nodes[node.index()].bounds;
        (bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.)
    }

    #[test]
    fn gallery_animates_arrow_selection_without_delaying_draft_or_focus() {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        runtime.apply(Message::Section(SECTION)).unwrap();
        runtime.activate("time-dial-demo-show").unwrap();
        draw(&mut runtime, 0.);
        runtime.activate(OPEN).unwrap();
        draw(&mut runtime, 0.);
        press(&mut runtime, Key::Enter);
        assert_eq!(runtime.focused(), Some("time-dial-dial-hour-12"));
        runtime.activate("time-dial-minute-selector").unwrap();
        draw(&mut runtime, 0.);
        draw(&mut runtime, 1000.);

        press(&mut runtime, Key::Enter);
        assert_eq!(runtime.focused(), Some("time-dial-dial-minute-0"));
        draw(&mut runtime, 1000.);
        let hand = "time-dial-dial-handle";
        let before = center(&runtime, hand);
        assert!(runtime
            .dispatch(Event::Edit {
                key: EditKey::Left,
                extend: false,
                by_word: false,
            })
            .unwrap());
        let selected = "time-dial-dial-minute-59";
        assert_eq!(runtime.focused(), Some(selected));
        let gallery = &runtime.app().time_dial_demo;
        assert_eq!(
            gallery
                .draft
                .as_ref()
                .unwrap()
                .state
                .input()
                .raw(TimeField::Minute),
            "59"
        );
        assert_eq!(gallery.saved, TimeOfDay::new(9, 7).unwrap());
        assert_eq!(gallery.commits, 0);
        draw(&mut runtime, 1000.);
        draw(&mut runtime, 1020.);
        let target = center(&runtime, selected);
        let mid = center(&runtime, hand);
        assert!(
            target.0 < mid.0 && mid.0 < before.0,
            "{target:?} {mid:?} {before:?}"
        );

        runtime.set_reduced_motion(true);
        draw(&mut runtime, 1021.);
        let reduced = center(&runtime, hand);
        assert!((reduced.0 - target.0).abs() <= 0.5 && (reduced.1 - target.1).abs() <= 0.5);
        runtime.set_reduced_motion(false);
        draw(&mut runtime, 1100.);
        assert_eq!(center(&runtime, hand), reduced);
        assert!(runtime.dispatch(Event::Cancel).unwrap());
        assert_eq!(runtime.focused(), Some(OPEN));
    }
}
