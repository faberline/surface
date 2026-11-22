use surface::{GridRole, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{Application, EditKey, Event, Runtime, View};
use surface_material::{
    shape::LayoutDirection, CivilDate, DateBounds, DateInputFormat, DateInputOrder,
    DateInputSeparator, DatePickerConstraints, DatePickerEffect, DatePickerEvent, DatePickerLabels,
    DatePickerLayout, DatePickerLocale, DatePickerMode, DatePickerOptions, DatePickerSession,
    Theme, Weekday,
};

const PICKER: &str = "picker";
const SELECTED: &str = "picker-day-2024-02-28";
const TODAY: &str = "picker-day-2024-02-29";
const VIEWPORT: Rect = Rect {
    x: 0.,
    y: 0.,
    width: 320.,
    height: 400.,
};

const LABELS: DatePickerLabels<'static> = DatePickerLabels {
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
};

fn date(year: u16, month: u8, day: u8) -> CivilDate {
    CivilDate::new(year, month, day).unwrap()
}

fn constraints() -> DatePickerConstraints<'static> {
    DatePickerConstraints {
        bounds: DateBounds::new(date(1, 1, 1), date(9999, 12, 31)).unwrap(),
        allowed_date: Some(&|value| value != date(2024, 2, 27)),
    }
}

fn locale() -> DatePickerLocale<'static> {
    DatePickerLocale {
        direction: LayoutDirection::RightToLeft,
        first_weekday: Weekday::Sunday,
        input_format: DateInputFormat {
            order: DateInputOrder::YearMonthDay,
            separator: DateInputSeparator::Slash,
        },
        format_hint: "YYYY/MM/DD",
        weekday_short: ["一", "二", "三", "四", "五", "六", "日"],
        weekday_full: [
            "星期一",
            "星期二",
            "星期三",
            "星期四",
            "星期五",
            "星期六",
            "星期日",
        ],
        month_title: &|month| format!("{}年{}月", month.year(), month.month()),
        date_headline: &|value| value.to_string(),
        date_name: &|value| value.to_string(),
        error_text: &|error| format!("{error:?}"),
    }
}

#[derive(Clone)]
enum Message {
    Open,
    Picker(DatePickerEvent),
}

#[derive(Default)]
struct Demo {
    session: Option<DatePickerSession>,
}

impl Application for Demo {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => {
                self.session = Some(DatePickerSession::new(
                    Some(date(2024, 2, 28)),
                    date(2024, 2, 29),
                    constraints(),
                    &locale(),
                    DatePickerMode::Calendar,
                ));
            }
            Message::Picker(event) => {
                if let Some(session) = &mut self.session {
                    if session.apply(event, constraints(), &locale()) != DatePickerEffect::None {
                        self.session = None;
                    }
                }
            }
        }
    }

    fn text_scale(&self) -> f32 {
        2.
    }

    fn view(&self) -> View<Message> {
        let opener = View::new(
            Node::new(Role::Button, "開啟日期")
                .id("open")
                .styled(Style::row().height(Length::Px(48.))),
        )
        .on_press(Message::Open);
        let picker = self.session.as_ref().map(|session| {
            Theme::dark().date_picker(
                DatePickerOptions {
                    id: PICKER,
                    today: date(2024, 2, 29),
                    constraints: constraints(),
                    locale: &locale(),
                    labels: &LABELS,
                    layout: DatePickerLayout::Adaptive,
                    viewport: VIEWPORT,
                    restore_focus_to: Some("open"),
                },
                session,
                Message::Picker,
            )
        });
        View::new(Node::new(Role::Window, "Test")).children(std::iter::once(opener).chain(picker))
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, content: &str, size: f32, max_width: Option<f32>) -> (f32, f32) {
        let width = content.chars().count() as f32 * size * 0.82;
        let fitted = width.min(max_width.unwrap_or(width));
        let lines = (width / fitted.max(1.)).ceil().max(1.);
        (fitted, size * 1.5 * lines)
    }
}

fn node<'a>(runtime: &'a Runtime<Demo>, name: &str) -> &'a Node {
    runtime
        .dom()
        .get(runtime.dom().by_id(name).unwrap())
        .unwrap()
}

fn rect(runtime: &Runtime<Demo>, name: &str) -> Rect {
    let id = runtime.dom().by_id(name).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn render(runtime: &mut Runtime<Demo>) {
    runtime
        .render(VIEWPORT.width, VIEWPORT.height, &mut Metrics)
        .unwrap();
}

#[test]
fn compact_rtl_keeps_grid_geometry_focus_and_draft_independent() {
    let mut runtime = Runtime::new(Demo::default()).unwrap();
    render(&mut runtime);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("open"));
    runtime.activate("open").unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some(SELECTED));
    assert_eq!(rect(&runtime, PICKER), VIEWPORT);
    assert_eq!(rect(&runtime, "picker-grid-scroll").width, 320.);
    assert_eq!(rect(&runtime, "picker-grid").width, 336.);
    assert_eq!(rect(&runtime, SELECTED).width, 48.);
    assert_eq!(rect(&runtime, SELECTED).height, 48.);
    assert_eq!(rect(&runtime, "picker-day-2024-02-28-face").width, 40.);
    assert!(rect(&runtime, "picker-confirm").height >= 48.);
    assert_eq!(node(&runtime, SELECTED).is_selected(), Some(true));
    assert_eq!(node(&runtime, TODAY).is_selected(), Some(false));
    assert!(node(&runtime, TODAY).is_current_date());
    assert!(node(&runtime, "picker-day-2024-02-27").is_disabled());
    assert_eq!(node(&runtime, "picker-weekday-0").name(), "星期日");
    let parent = runtime
        .dom()
        .get(node(&runtime, SELECTED).parent().unwrap())
        .unwrap();
    assert_eq!(parent.accessibility_grid_role(), Some(GridRole::Row));

    runtime
        .dispatch(Event::Edit {
            key: EditKey::Left,
            extend: false,
            by_word: false,
        })
        .unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some(TODAY));
    assert_eq!(
        runtime.app().session.as_ref().unwrap().draft(),
        Some(date(2024, 2, 28))
    );
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert!(!runtime.focused().unwrap().starts_with("picker-day-"));

    runtime.activate("picker-month").unwrap();
    render(&mut runtime);
    let years = runtime
        .dom()
        .iter()
        .filter(|(_, node)| {
            node.stable_id().is_some_and(|id| {
                id.strip_prefix("picker-year-")
                    .is_some_and(|name| name.parse::<u16>().is_ok())
            })
        })
        .count();
    assert_eq!(years, 72);
    assert_eq!(
        runtime.app().session.as_ref().unwrap().mode(),
        DatePickerMode::Year
    );
    for _ in 0..12 {
        if runtime.focused() == Some("picker-year-2024") {
            break;
        }
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
    }
    assert_eq!(runtime.focused(), Some("picker-year-2024"));
    for (key, target) in [
        (EditKey::Down, "picker-year-2027"),
        (EditKey::PageDown, "picker-year-2099"),
    ] {
        runtime
            .dispatch(Event::Edit {
                key,
                extend: false,
                by_word: false,
            })
            .unwrap();
        render(&mut runtime);
        assert_eq!(runtime.focused(), Some(target));
    }
    runtime.activate("picker-mode").unwrap();
    render(&mut runtime);
    assert_eq!(
        node(&runtime, "picker-input").text_value(),
        Some("2024/02/28")
    );
    assert_eq!(
        node(&runtime, "picker-input").description_text_target(),
        Some("picker-hint")
    );
    runtime.activate("picker-cancel").unwrap();
    assert!(runtime.app().session.is_none());
    assert_eq!(runtime.focused(), Some("open"));
}
