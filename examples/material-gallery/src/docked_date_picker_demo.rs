use surface::{Align, Insets, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, FrameInput, View};
use surface_material::{
    place_docked_date_picker, ButtonKind, CivilDate, DatePickerEffect, DockedDatePickerEvent,
    DockedDatePickerLabels, DockedDatePickerOptions, DockedDatePickerState, Theme, YearMonth,
};

use crate::{
    date_picker_demo::{constraints, date, locale, today},
    Message,
};

pub(crate) const SECTION: usize = 2;
const PICKER: &str = "docked-date-picker-demo-picker";
const ANCHOR: &str = "docked-date-picker-demo-picker-field";
const INPUT: &str = "docked-date-picker-demo-picker-input";
const SUMMARY: &str = "docked-date-picker-demo-summary";

#[derive(Clone)]
pub enum DockedDateDemoEvent {
    Show,
    Back,
    Direction,
    LargeText,
    Bottom,
    Outside,
    Picker(DockedDatePickerEvent),
}

pub(crate) struct DockedDateDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    near_bottom: bool,
    saved: Option<CivilDate>,
    commits: usize,
    outside_actions: usize,
    viewport: Rect,
    anchor: Option<Rect>,
    anchor_top: f32,
    picker: DockedDatePickerState,
}

impl Default for DockedDateDemo {
    fn default() -> Self {
        let saved = Some(date(2024, 2, 28));
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            near_bottom: false,
            saved,
            commits: 0,
            outside_actions: 0,
            viewport: Rect::new(0., 0., 960., 760.),
            anchor: None,
            anchor_top: 192.,
            picker: DockedDatePickerState::new(
                saved,
                today(),
                constraints(),
                &locale(false, false),
            ),
        }
    }
}

impl DockedDateDemo {
    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme.button_with(
            "docked-date-picker-demo-show",
            "錨定日期選擇器",
            ButtonKind::Outlined,
            false,
            Message::DockedDate(DockedDateDemoEvent::Show),
        )
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.anchor = None;
        self.picker = DockedDatePickerState::new(
            self.saved,
            today(),
            constraints(),
            &locale(false, self.rtl),
        );
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
        let changed = viewport != self.viewport;
        self.viewport = viewport;
        changed && self.visible
    }

    fn accept(&mut self, effect: DatePickerEffect) {
        if let DatePickerEffect::Commit(date) = effect {
            self.saved = Some(date);
            self.commits += 1;
        }
    }

    pub(crate) fn observe(&mut self, frame: &AnimationFrame<'_>, input: FrameInput<'_>) -> bool {
        if !self.visible {
            return false;
        }
        let Some(layout) = frame.layout else {
            return false;
        };
        let bounds = |key| {
            frame
                .dom
                .by_id(key)
                .and_then(|id| layout.nodes.get(id.index()))
                .map(|node| node.bounds)
        };
        let new_anchor = bounds(ANCHOR);
        let anchor_changed = self.anchor != new_anchor;
        self.anchor = new_anchor;
        let next_top = bounds(SUMMARY).map_or(self.anchor_top, |row| row.y + row.height + 24.);
        let moved = next_top != self.anchor_top;
        self.anchor_top = next_top;
        let locale = locale(false, self.rtl);
        let lost_popup = self.picker.mode().is_some()
            && new_anchor
                .and_then(|anchor| {
                    place_docked_date_picker(anchor, self.viewport, locale.direction)
                })
                .is_none();
        if lost_popup {
            while self.picker.mode().is_some() {
                self.picker.apply(
                    DockedDatePickerEvent::Dismiss,
                    today(),
                    constraints(),
                    &locale,
                );
            }
            return true;
        }
        let (changed, effect) =
            self.picker
                .observe_input_focus(input.focused == Some(INPUT), constraints(), &locale);
        self.accept(effect);
        anchor_changed || moved || changed || matches!(effect, DatePickerEffect::Commit(_))
    }

    pub(crate) fn update(&mut self, event: DockedDateDemoEvent) {
        match event {
            DockedDateDemoEvent::Show => self.visible = true,
            DockedDateDemoEvent::Back => self.leave(),
            DockedDateDemoEvent::Direction => self.rtl = !self.rtl,
            DockedDateDemoEvent::LargeText => self.large_text = !self.large_text,
            DockedDateDemoEvent::Bottom => self.near_bottom = !self.near_bottom,
            DockedDateDemoEvent::Outside => self.outside_actions += 1,
            DockedDateDemoEvent::Picker(event) => {
                let effect =
                    self.picker
                        .apply(event, today(), constraints(), &locale(false, self.rtl));
                self.accept(effect);
            }
        }
    }

    pub(crate) fn view(&self, theme: &Theme) -> View<Message> {
        let action = |id, text, event| theme.button(id, text, Message::DockedDate(event));
        let controls = theme.row([
            action(
                "docked-date-picker-demo-back",
                "返回",
                DockedDateDemoEvent::Back,
            ),
            theme.button("docked-date-picker-demo-theme", "切換配色", Message::Theme),
            action(
                "docked-date-picker-demo-rtl",
                if self.rtl { "左起" } else { "右起" },
                DockedDateDemoEvent::Direction,
            ),
            action(
                "docked-date-picker-demo-large-text",
                if self.large_text {
                    "一般文字"
                } else {
                    "大字"
                },
                DockedDateDemoEvent::LargeText,
            ),
            action(
                "docked-date-picker-demo-bottom",
                if self.near_bottom {
                    "移到上方"
                } else {
                    "移到下方"
                },
                DockedDateDemoEvent::Bottom,
            ),
        ]);
        let outside = action(
            "docked-date-picker-demo-outside",
            "其他操作",
            DockedDateDemoEvent::Outside,
        );
        let locale = locale(false, self.rtl);
        let value = self
            .saved
            .map(|date| locale.input_format.format(date))
            .unwrap_or_default();
        let status = theme.text(
            "docked-date-picker-demo-status",
            format!("已選日期 {value}"),
        );
        let labels = DockedDatePickerLabels {
            input_name: "日期",
            calendar: "開啟日曆",
            previous_month: "上個月",
            next_month: "下個月",
            choose_month: "選擇月份",
            choose_year: "選擇年份",
            month_name: &month_label,
        };
        let picker = theme.docked_date_picker(
            DockedDatePickerOptions {
                id: PICKER,
                today: today(),
                viewport: self.viewport,
                anchor: self.anchor,
                constraints: constraints(),
                locale: &locale,
                labels: &labels,
                restore_focus_to: Some("docked-date-picker-demo-outside"),
            },
            &self.picker,
            |event| Message::DockedDate(DockedDateDemoEvent::Picker(event)),
        );
        let mut position = Insets {
            left: Length::Px(20.),
            ..Insets::default()
        };
        if self.near_bottom {
            position.bottom = Length::Px(24.);
        } else {
            position.top = Length::Px(self.anchor_top);
        }
        let anchor = View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Px((self.viewport.width - 40.).clamp(48., 360.)))
                    .fixed(position),
            ),
        )
        .children([picker]);
        let summary = View::new(
            Node::new(Role::Group, "")
                .id(SUMMARY)
                .styled(Style::row().align(Align::Center).gap(12.).wrap(true)),
        )
        .children([status, outside]);
        theme.page([controls, anchor, summary])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        let calendar = self.picker.calendar();
        serde_json::json!({
            "visible": self.visible,
            "committed": self.saved.map(|value| value.to_string()),
            "commits": self.commits,
            "outside_actions": self.outside_actions,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "bottom": self.near_bottom,
            "raw": self.picker.raw_input(),
            "error": self.picker.error().map(|error| format!("{error:?}")),
            "mode": self.picker.mode().map(|mode| format!("{mode:?}")),
            "month": [calendar.month().year(), u16::from(calendar.month().month())],
            "active": calendar.active().map(|value| value.to_string()),
            "anchor": self.anchor,
        })
    }
}

fn month_label(month: YearMonth) -> String {
    format!("{}月", month.month())
}
