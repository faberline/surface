use surface::Rect;
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, Queue, View};
use surface_material::{
    ButtonKind, CivilDate, CivilDateRange, DateRangeConstraints, DateRangeEffect, DateRangeMode,
    DateRangePairError, DateRangePickerInput, DateRangePickerLabels, DateRangePickerOptions,
    DateRangePickerSession, DateRangePosition, Theme,
};

use crate::{
    date_picker_demo::{constraints, date, locale, today},
    Message,
};

pub(crate) const SECTION: usize = 2;
const PICKER: &str = "date-range-picker-demo-modal";
const OPEN: &str = "date-range-picker-demo-open";

#[derive(Clone)]
pub enum DateRangeDemoEvent {
    Show,
    Back,
    Direction,
    LargeText,
    Outside,
    Open(DateRangeMode),
    Picker(DateRangePickerInput),
}

pub(crate) struct DateRangeDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    viewport: Rect,
    committed: CivilDateRange,
    commits: usize,
    outside_actions: usize,
    session: Option<DateRangePickerSession>,
}

impl Default for DateRangeDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            viewport: Rect::new(0., 0., 960., 800.),
            committed: CivilDateRange::new(date(2024, 2, 24), date(2024, 3, 5))
                .expect("ordered Gallery range"),
            commits: 0,
            outside_actions: 0,
            session: None,
        }
    }
}

impl DateRangeDemo {
    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme.button_with(
            "date-range-picker-demo-show",
            "日期範圍選擇器",
            ButtonKind::Outlined,
            false,
            Message::DateRange(DateRangeDemoEvent::Show),
        )
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.session = None;
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
        self.visible && changed
    }

    pub(crate) fn observes_input(&self) -> bool {
        self.session.is_some()
    }

    pub(crate) fn observe(
        &mut self,
        frame: &AnimationFrame<'_>,
        input: FrameInput<'_>,
    ) -> AnimationUpdate {
        self.session
            .as_mut()
            .map_or_else(AnimationUpdate::default, |session| {
                session.observe_frame(
                    PICKER,
                    frame,
                    input,
                    range_constraints(),
                    &locale(false, self.rtl),
                )
            })
    }

    pub(crate) fn flush_scroll_requests(&mut self, queue: &Queue<Message>) {
        if let Some(session) = &mut self.session {
            session.flush_scroll_requests(PICKER, queue);
        }
    }

    pub(crate) fn update(&mut self, event: DateRangeDemoEvent) {
        match event {
            DateRangeDemoEvent::Show => self.visible = true,
            DateRangeDemoEvent::Back => self.leave(),
            DateRangeDemoEvent::Direction => self.rtl = !self.rtl,
            DateRangeDemoEvent::LargeText => self.large_text = !self.large_text,
            DateRangeDemoEvent::Outside => self.outside_actions += 1,
            DateRangeDemoEvent::Open(mode) => {
                self.session = Some(DateRangePickerSession::new(
                    Some(self.committed),
                    today(),
                    range_constraints(),
                    &locale(false, self.rtl),
                    mode,
                ));
            }
            DateRangeDemoEvent::Picker(input) => {
                if let Some(session) = &mut self.session {
                    match session.apply(input, range_constraints(), &locale(false, self.rtl)) {
                        DateRangeEffect::None => {}
                        DateRangeEffect::Commit(range) => {
                            self.committed = range;
                            self.commits += 1;
                            self.session = None;
                        }
                        DateRangeEffect::Dismiss => self.session = None,
                    }
                }
            }
        }
    }

    pub(crate) fn view(&self, theme: &Theme) -> View<Message> {
        let action = |id, text, event| theme.button(id, text, Message::DateRange(event));
        let controls = theme.row([
            action(
                "date-range-picker-demo-back",
                "返回",
                DateRangeDemoEvent::Back,
            ),
            theme.button("date-range-picker-demo-theme", "切換配色", Message::Theme),
            action(
                "date-range-picker-demo-rtl",
                if self.rtl { "左起" } else { "右起" },
                DateRangeDemoEvent::Direction,
            ),
            action(
                "date-range-picker-demo-large-text",
                if self.large_text {
                    "一般文字"
                } else {
                    "大字"
                },
                DateRangeDemoEvent::LargeText,
            ),
        ]);
        let choices = theme.row([
            theme.button_with(
                OPEN,
                "選擇日期範圍",
                ButtonKind::Filled,
                false,
                Message::DateRange(DateRangeDemoEvent::Open(DateRangeMode::Calendar)),
            ),
            theme.button_with(
                "date-range-picker-demo-input-open",
                "輸入日期範圍",
                ButtonKind::Outlined,
                false,
                Message::DateRange(DateRangeDemoEvent::Open(DateRangeMode::Input)),
            ),
            action(
                "date-range-picker-demo-outside",
                "其他操作",
                DateRangeDemoEvent::Outside,
            ),
        ]);
        let locale = locale(false, self.rtl);
        let start = locale.input_format.format(self.committed.start());
        let end = locale.input_format.format(self.committed.end());
        let status = theme.text(
            "date-range-picker-demo-status",
            format!("已儲存 {start} 至 {end}"),
        );
        let picker = self.session.as_ref().map(|session| {
            theme.date_range_picker(
                DateRangePickerOptions {
                    id: PICKER,
                    today: today(),
                    constraints: range_constraints(),
                    locale: &locale,
                    labels: &labels(),
                    viewport: self.viewport,
                    restore_focus_to: Some(OPEN),
                },
                session,
                |input| Message::DateRange(DateRangeDemoEvent::Picker(input)),
            )
        });
        theme.page([controls, choices, status].into_iter().chain(picker))
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        use surface_material::{DateRangeEndpoint, YearMonth};

        let month_pair = |month: YearMonth| [month.year(), u16::from(month.month())];
        let pair = |range: CivilDateRange| [range.start().to_string(), range.end().to_string()];
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "committed": pair(self.committed),
            "commits": self.commits,
            "outside_actions": self.outside_actions,
            "session": self.session.as_ref().map(|session| {
                let state = session.state();
                let validation = state.validation(range_constraints(), &locale(false, self.rtl));
                let first = DateRangeEndpoint::Start;
                let last = DateRangeEndpoint::End;
                serde_json::json!({
                    "mode": format!("{:?}", state.mode()),
                    "month": month_pair(state.month()),
                    "mounted_months": session.mounted_months().iter().copied().map(month_pair).collect::<Vec<_>>(),
                    "active": state.active().map(|value| value.to_string()),
                    "candidate": validation.candidate().map(pair),
                    "raw_start": state.raw_input(first),
                    "raw_end": state.raw_input(last),
                    "start_error": state.displayed_field_error(first, validation).map(|error| format!("{error:?}")),
                    "end_error": state.displayed_field_error(last, validation).map(|error| format!("{error:?}")),
                    "pair_error": state.displayed_pair_error(validation).map(|error| format!("{error:?}")),
                })
            }),
        })
    }
}

fn range_constraints() -> DateRangeConstraints<'static> {
    DateRangeConstraints {
        dates: constraints(),
        allowed_span: None,
    }
}

fn labels() -> DateRangePickerLabels<'static> {
    DateRangePickerLabels {
        title: "選擇日期範圍",
        start: "開始日期",
        end: "結束日期",
        edit_mode: "輸入日期範圍",
        calendar_mode: "顯示行事曆",
        save: "儲存",
        close: "關閉",
        previous_month: "上個月",
        next_month: "下個月",
        calendar: "日期範圍行事曆",
        summary: &range_summary,
        day_name: &range_day_name,
        pair_error: &range_error,
    }
}

fn range_summary(start: Option<CivilDate>, end: Option<CivilDate>) -> String {
    let headline = locale(false, false).date_headline;
    let display = |value: Option<CivilDate>| value.map_or_else(|| "尚未選擇".into(), headline);
    format!("開始日期 {}\n結束日期 {}", display(start), display(end))
}

fn range_day_name(date: CivilDate, position: DateRangePosition) -> String {
    let date = (locale(false, false).date_name)(date);
    match position {
        DateRangePosition::None => date,
        DateRangePosition::Start => format!("開始日期 {date}"),
        DateRangePosition::End => format!("結束日期 {date}"),
        DateRangePosition::Interior => format!("範圍內 {date}"),
        DateRangePosition::OnlyDay => format!("開始與結束日期 {date}"),
    }
}

fn range_error(error: DateRangePairError) -> String {
    match error {
        DateRangePairError::Reversed => "結束日期不可早於開始日期".into(),
        DateRangePairError::SpanUnavailable => "這段日期範圍無法選取".into(),
    }
}
