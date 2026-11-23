use surface::Rect;
use surface_app::{AnimationFrame, AnimationUpdate, Queue, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, CivilDate, DateBounds, DateInputError, DateInputFormat,
    DateInputOrder, DateInputSeparator, DatePickerConstraints, DatePickerEffect, DatePickerLabels,
    DatePickerLayout, DatePickerLocale, DatePickerMode, DatePickerOptions, DatePickerSession,
    DatePickerSwipe, DatePickerSwipeInput, Theme, Weekday, YearMonth,
};

use crate::Message;

pub(crate) const SECTION: usize = 2;
const ID: &str = "date-picker-demo-modal";
const INPUT_ID: &str = "date-picker-demo-modal-input";

#[derive(Clone)]
pub enum DatePickerDemoEvent {
    Show,
    Back,
    Direction,
    LargeText,
    Locale,
    Open(DatePickerMode),
    Picker(DatePickerSwipeInput),
}

pub(crate) struct DatePickerDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    english: bool,
    viewport: Rect,
    committed: CivilDate,
    session: Option<DatePickerSession>,
    swipe: DatePickerSwipe,
    commits: usize,
}

impl Default for DatePickerDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            english: false,
            viewport: Rect::new(0., 0., 960., 720.),
            committed: date(2024, 2, 28),
            session: None,
            swipe: DatePickerSwipe::default(),
            commits: 0,
        }
    }
}

impl DatePickerDemo {
    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme.button_with(
            "date-picker-demo-show",
            "日期選擇器",
            ButtonKind::Outlined,
            false,
            Message::DatePicker(DatePickerDemoEvent::Show),
        )
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.session = None;
        self.swipe = DatePickerSwipe::default();
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

    pub(crate) fn observes_input(&self) -> bool {
        self.session.is_some()
    }

    pub(crate) fn observe(
        &mut self,
        frame: &AnimationFrame<'_>,
        focused: Option<&str>,
        queue: Option<&Queue<Message>>,
    ) -> AnimationUpdate {
        let Some(session) = &mut self.session else {
            return AnimationUpdate::default();
        };
        let locale = locale(self.english, self.rtl);
        let mut update = queue.map_or_else(AnimationUpdate::default, |queue| {
            self.swipe
                .observe(ID, frame, session, constraints(), &locale, queue)
        });
        update.rebuild |=
            session.observe_input_focus(focused == Some(INPUT_ID), constraints(), &locale);
        update
    }

    pub(crate) fn update(&mut self, event: DatePickerDemoEvent) {
        match event {
            DatePickerDemoEvent::Show => self.visible = true,
            DatePickerDemoEvent::Back => self.leave(),
            DatePickerDemoEvent::Direction => self.rtl = !self.rtl,
            DatePickerDemoEvent::LargeText => self.large_text = !self.large_text,
            DatePickerDemoEvent::Locale => self.english = !self.english,
            DatePickerDemoEvent::Open(mode) => {
                self.swipe = DatePickerSwipe::default();
                self.session = Some(DatePickerSession::new(
                    Some(self.committed),
                    today(),
                    constraints(),
                    &locale(self.english, self.rtl),
                    mode,
                ));
            }
            DatePickerDemoEvent::Picker(event) => {
                let locale = locale(self.english, self.rtl);
                if let Some(session) = &mut self.session {
                    match self.swipe.apply(event, session, constraints(), &locale) {
                        DatePickerEffect::None => {}
                        DatePickerEffect::Commit(date) => {
                            self.committed = date;
                            self.commits += 1;
                            self.session = None;
                        }
                        DatePickerEffect::Dismiss => self.session = None,
                    }
                }
            }
        }
    }

    pub(crate) fn view(&self, theme: &Theme) -> View<Message> {
        let locale = locale(self.english, self.rtl);
        let actions = theme.row([
            theme.button_with(
                "date-picker-demo-back",
                "返回",
                ButtonKind::Tonal,
                false,
                Message::DatePicker(DatePickerDemoEvent::Back),
            ),
            theme.button("date-picker-demo-theme", "切換配色", Message::Theme),
            theme.button(
                "date-picker-demo-rtl",
                if self.rtl { "左起" } else { "右起" },
                Message::DatePicker(DatePickerDemoEvent::Direction),
            ),
            theme.button(
                "date-picker-demo-large-text",
                if self.large_text {
                    "一般文字"
                } else {
                    "大字"
                },
                Message::DatePicker(DatePickerDemoEvent::LargeText),
            ),
            theme.button(
                "date-picker-demo-locale",
                if self.english { "中文" } else { "English" },
                Message::DatePicker(DatePickerDemoEvent::Locale),
            ),
        ]);
        let choices = theme.row([
            theme.button_with(
                "date-picker-demo-open",
                "選擇日期",
                ButtonKind::Filled,
                false,
                Message::DatePicker(DatePickerDemoEvent::Open(DatePickerMode::Calendar)),
            ),
            theme.button_with(
                "date-picker-demo-input-open",
                "輸入日期",
                ButtonKind::Outlined,
                false,
                Message::DatePicker(DatePickerDemoEvent::Open(DatePickerMode::Input)),
            ),
        ]);
        let status = theme.text(
            "date-picker-demo-status",
            format!("已選日期 {}", locale.input_format.format(self.committed)),
        );
        let picker = self.session.as_ref().map(|session| {
            let labels = labels(self.english);
            theme.swipeable_date_picker(
                DatePickerOptions {
                    id: ID,
                    today: today(),
                    constraints: constraints(),
                    locale: &locale,
                    labels: &labels,
                    layout: DatePickerLayout::Adaptive,
                    viewport: self.viewport,
                    restore_focus_to: Some("date-picker-demo-open"),
                },
                session,
                &self.swipe,
                |event| Message::DatePicker(DatePickerDemoEvent::Picker(event)),
            )
        });
        theme.page([actions, choices, status].into_iter().chain(picker))
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        serde_json::json!({
            "visible": self.visible,
            "committed": self.committed.to_string(),
            "commits": self.commits,
            "rtl": self.rtl,
            "english": self.english,
            "text_scale": self.text_scale(),
            "swipe": { "offset": self.swipe.preview_offset() },
            "session": self.session.as_ref().map(|session| serde_json::json!({
                "mode": format!("{:?}", session.mode()),
                "draft": session.draft().map(|value| value.to_string()),
                "month": [session.month().year(), u16::from(session.month().month())],
                "active": session.active().map(|value| value.to_string()),
                "raw": session.raw_input(),
                "error": session.error().map(|error| format!("{error:?}")),
            })),
        })
    }
}

pub(crate) fn date(year: u16, month: u8, day: u8) -> CivilDate {
    CivilDate::new(year, month, day).expect("valid Gallery date")
}

pub(crate) fn today() -> CivilDate {
    date(2024, 2, 29)
}

pub(crate) fn constraints() -> DatePickerConstraints<'static> {
    DatePickerConstraints {
        bounds: DateBounds::new(date(1900, 1, 1), date(2100, 12, 31)).expect("ordered dates"),
        allowed_date: Some(&available),
    }
}

fn available(value: CivilDate) -> bool {
    !matches!(
        (value.year(), value.month(), value.day()),
        (2024, 2, 27) | (2024, 3, 1)
    )
}

const ZH_SHORT: [&str; 7] = ["一", "二", "三", "四", "五", "六", "日"];
const ZH_WEEKDAYS: [&str; 7] = [
    "星期一",
    "星期二",
    "星期三",
    "星期四",
    "星期五",
    "星期六",
    "星期日",
];
const EN_SHORT: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];
const EN_WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const EN_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn zh_month(month: YearMonth) -> String {
    format!("{}年{}月", month.year(), month.month())
}

fn en_month(month: YearMonth) -> String {
    format!(
        "{} {}",
        EN_MONTHS[usize::from(month.month() - 1)],
        month.year()
    )
}

fn zh_date(value: CivilDate) -> String {
    format!(
        "{} {}",
        zh_headline(value),
        ZH_WEEKDAYS[value.weekday().index()],
    )
}

fn zh_headline(value: CivilDate) -> String {
    format!("{}{}日", zh_month(value.year_month()), value.day())
}

fn en_date(value: CivilDate) -> String {
    format!(
        "{}, {}",
        EN_WEEKDAYS[value.weekday().index()],
        en_headline(value),
    )
}

fn en_headline(value: CivilDate) -> String {
    format!(
        "{} {} {}",
        value.day(),
        EN_MONTHS[usize::from(value.month() - 1)],
        value.year(),
    )
}

fn zh_error(error: DateInputError) -> String {
    match error {
        DateInputError::Required => "請輸入日期",
        DateInputError::Incomplete | DateInputError::InvalidFormat => "請依年／月／日格式輸入",
        DateInputError::InvalidDate => "這個日期不存在",
        DateInputError::OutsideBounds => "日期超出可選範圍",
        DateInputError::Unavailable => "這個日期無法選取",
    }
    .into()
}

fn en_error(error: DateInputError) -> String {
    match error {
        DateInputError::Required => "Enter a date",
        DateInputError::Incomplete | DateInputError::InvalidFormat => "Use DD/MM/YYYY",
        DateInputError::InvalidDate => "Enter a real date",
        DateInputError::OutsideBounds => "Date is outside the allowed range",
        DateInputError::Unavailable => "This date is unavailable",
    }
    .into()
}

pub(crate) fn locale(english: bool, rtl: bool) -> DatePickerLocale<'static> {
    let direction = if rtl {
        LayoutDirection::RightToLeft
    } else {
        LayoutDirection::LeftToRight
    };
    if english {
        DatePickerLocale {
            direction,
            first_weekday: Weekday::Monday,
            input_format: DateInputFormat {
                order: DateInputOrder::DayMonthYear,
                separator: DateInputSeparator::Slash,
            },
            format_hint: "DD/MM/YYYY",
            weekday_short: EN_SHORT,
            weekday_full: EN_WEEKDAYS,
            month_title: &en_month,
            date_headline: &en_headline,
            date_name: &en_date,
            error_text: &en_error,
        }
    } else {
        DatePickerLocale {
            direction,
            first_weekday: Weekday::Sunday,
            input_format: DateInputFormat {
                order: DateInputOrder::YearMonthDay,
                separator: DateInputSeparator::Slash,
            },
            format_hint: "年／月／日（YYYY/MM/DD）",
            weekday_short: ZH_SHORT,
            weekday_full: ZH_WEEKDAYS,
            month_title: &zh_month,
            date_headline: &zh_headline,
            date_name: &zh_date,
            error_text: &zh_error,
        }
    }
}

fn labels(english: bool) -> DatePickerLabels<'static> {
    if english {
        DatePickerLabels {
            title: "Select date",
            input_name: "Date",
            previous_month: "Previous month",
            next_month: "Next month",
            choose_year: "Choose year",
            edit_mode: "Enter date",
            calendar_mode: "Show calendar",
            confirm: "OK",
            cancel: "Cancel",
            save: "Save",
            close: "Close",
        }
    } else {
        DatePickerLabels {
            title: "選擇日期",
            input_name: "日期",
            previous_month: "上個月",
            next_month: "下個月",
            choose_year: "選擇年份",
            edit_mode: "輸入日期",
            calendar_mode: "顯示行事曆",
            confirm: "確定",
            cancel: "取消",
            save: "儲存",
            close: "關閉",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surface::TextMeasurer;
    use surface_app::{Event, Runtime};

    struct Metrics;

    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
            (text.chars().count() as f32 * size * 0.6, size * 1.5)
        }
    }

    fn draw(runtime: &mut Runtime<crate::Gallery>, time: f64) {
        runtime.set_time(time);
        runtime.render(960., 760., &mut Metrics).unwrap();
    }

    #[test]
    fn gallery_touch_swipe_cancels_or_browses_without_committing_the_draft() {
        let mut runtime = Runtime::new(crate::Gallery::default()).unwrap();
        let active = "date-picker-demo-modal-day-2024-02-28";
        let preview = "date-picker-demo-modal-swipe-preview";
        runtime.apply(Message::Section(SECTION)).unwrap();
        runtime.activate("date-picker-demo-show").unwrap();
        runtime.activate("date-picker-demo-open").unwrap();
        draw(&mut runtime, 0.);
        draw(&mut runtime, 17.);

        let id = runtime.dom().by_id(active).unwrap();
        let rect = runtime.computed().unwrap().nodes[id.index()].bounds;
        let (x, y) = (rect.x + rect.width / 2., rect.y + rect.height / 2.);
        runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
        runtime
            .dispatch_touch(Event::PointerMove { x: x - 40., y })
            .unwrap();
        draw(&mut runtime, 50.);
        let preview_node = runtime.dom().by_id(preview).expect("held swipe preview");
        let ax = runtime.accessibility_frame().unwrap();
        assert!(ax.node(preview_node).is_none());
        assert_eq!(runtime.focused(), Some(active));
        let session = runtime.app().date_picker_demo.session.as_ref().unwrap();
        assert_eq!(session.month(), date(2024, 2, 1).year_month());
        assert_eq!(session.draft(), Some(date(2024, 2, 28)));

        runtime.cancel_pointer().unwrap();
        draw(&mut runtime, 70.);
        draw(&mut runtime, 470.);
        assert!(runtime.dom().by_id(preview).is_none());
        assert_eq!(runtime.focused(), Some(active));

        runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
        draw(&mut runtime, 500.);
        runtime
            .dispatch_touch(Event::PointerMove { x: x - 152., y })
            .unwrap();
        draw(&mut runtime, 630.);
        runtime
            .dispatch_touch(Event::PointerUp { x: x - 152., y })
            .unwrap();
        draw(&mut runtime, 640.);
        draw(&mut runtime, 1080.);

        let state = &runtime.app().date_picker_demo;
        let session = state.session.as_ref().unwrap();
        assert_eq!(session.month(), date(2024, 3, 1).year_month());
        assert_eq!(session.draft(), Some(date(2024, 2, 28)));
        assert_eq!(state.commits, 0);
        assert_eq!(
            runtime.focused(),
            Some("date-picker-demo-modal-day-2024-03-28")
        );
        assert!(runtime.dom().by_id(preview).is_none());
    }
}
