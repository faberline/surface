use surface::Rect;
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, ButtonKind, CivilDate, DateBounds, DateInputError, DateInputFormat,
    DateInputOrder, DateInputSeparator, DatePickerConstraints, DatePickerEffect, DatePickerEvent,
    DatePickerLabels, DatePickerLayout, DatePickerLocale, DatePickerMode, DatePickerOptions,
    DatePickerSession, Theme, Weekday, YearMonth,
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
    Picker(DatePickerEvent),
}

pub(crate) struct DatePickerDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    english: bool,
    viewport: Rect,
    committed: CivilDate,
    session: Option<DatePickerSession>,
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

    pub(crate) fn observe_focus(&mut self, focused: Option<&str>) -> bool {
        let locale = locale(self.english, self.rtl);
        self.session.as_mut().is_some_and(|session| {
            session.observe_input_focus(focused == Some(INPUT_ID), constraints(), &locale)
        })
    }

    pub(crate) fn update(&mut self, event: DatePickerDemoEvent) {
        match event {
            DatePickerDemoEvent::Show => self.visible = true,
            DatePickerDemoEvent::Back => self.leave(),
            DatePickerDemoEvent::Direction => self.rtl = !self.rtl,
            DatePickerDemoEvent::LargeText => self.large_text = !self.large_text,
            DatePickerDemoEvent::Locale => self.english = !self.english,
            DatePickerDemoEvent::Open(mode) => {
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
                    match session.apply(event, constraints(), &locale) {
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
            theme.date_picker(
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

fn date(year: u16, month: u8, day: u8) -> CivilDate {
    CivilDate::new(year, month, day).expect("valid Gallery date")
}

fn today() -> CivilDate {
    date(2024, 2, 29)
}

fn constraints() -> DatePickerConstraints<'static> {
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

fn locale(english: bool, rtl: bool) -> DatePickerLocale<'static> {
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
            today: "Today",
            selected: "Selected",
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
            today: "今天",
            selected: "已選取",
        }
    }
}
