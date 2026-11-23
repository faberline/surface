use surface::{GridRole, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AnimationFrame, AnimationUpdate, Application, EditKey, Event, FrameInput, InputAnimationUpdate,
    Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, CivilDate, DateBounds, DateInputFormat, DateInputOrder,
    DateInputSeparator, DatePickerConstraints, DatePickerEffect, DatePickerLocale,
    DockedDatePickerEvent, DockedDatePickerLabels, DockedDatePickerMode, DockedDatePickerOptions,
    DockedDatePickerState, Theme, Weekday,
};

const WIDTH: f32 = 320.;
const HEIGHT: f32 = 640.;
const INPUT: &str = "docked-input";
const FIELD: &str = "docked-field";
const TRIGGER: &str = "docked-calendar-trigger";
const POPUP: &str = "docked-popup";
const LABELS: DockedDatePickerLabels<'static> = DockedDatePickerLabels {
    input_name: "日期",
    calendar: "開啟行事曆",
    previous_month: "上個月",
    next_month: "下個月",
    choose_month: "選擇月份",
    choose_year: "選擇年份",
    month_name: &|month| format!("{}月", month.month()),
};

fn date(year: u16, month: u8, day: u8) -> CivilDate {
    CivilDate::new(year, month, day).unwrap()
}

fn constraints() -> DatePickerConstraints<'static> {
    DatePickerConstraints {
        bounds: DateBounds::new(date(2024, 1, 1), date(2030, 12, 31)).unwrap(),
        allowed_date: None,
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

struct Demo {
    picker: DockedDatePickerState,
    anchor: Option<Rect>,
    commits: usize,
}

impl Demo {
    fn record(&mut self, result: DatePickerEffect) {
        self.commits += usize::from(matches!(result, DatePickerEffect::Commit(_)));
    }
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            picker: DockedDatePickerState::new(
                Some(date(2024, 2, 28)),
                date(2024, 2, 29),
                constraints(),
                &locale(),
            ),
            anchor: None,
            commits: 0,
        }
    }
}

impl Application for Demo {
    type Message = DockedDatePickerEvent;

    fn update(&mut self, message: Self::Message) {
        let effect = self
            .picker
            .apply(message, date(2024, 2, 29), constraints(), &locale());
        self.record(effect);
    }

    fn observes_frame_input(&self) -> bool {
        true
    }

    fn animation_frame_with_input(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
    ) -> InputAnimationUpdate {
        let mut moved = false;
        if let Some(layout) = frame.layout {
            let anchor = frame
                .dom
                .by_id(FIELD)
                .map(|field| layout.nodes[field.index()].bounds);
            moved = anchor != self.anchor;
            self.anchor = anchor;
        }
        let (edited, effect) =
            self.picker
                .observe_input_focus(input.focused == Some(INPUT), constraints(), &locale());
        self.record(effect);
        InputAnimationUpdate {
            animation: AnimationUpdate {
                rebuild: moved || edited,
                request_next: false,
            },
            suppress_touch_activation: false,
        }
    }

    fn text_scale(&self) -> f32 {
        2.
    }

    fn view(&self) -> View<Self::Message> {
        let previous = View::new(
            Node::new(Role::Button, "工作區")
                .id("previous")
                .styled(Style::row().height(Length::Px(48.))),
        )
        .on_press(DockedDatePickerEvent::Dismiss);
        let docked = Theme::dark().docked_date_picker(
            DockedDatePickerOptions {
                id: "docked",
                today: date(2024, 2, 29),
                viewport: Rect::new(0., 0., WIDTH, HEIGHT),
                anchor: self.anchor,
                constraints: constraints(),
                locale: &locale(),
                labels: &LABELS,
                restore_focus_to: Some("previous"),
            },
            &self.picker,
            |event| event,
        );
        View::new(Node::new(Role::Window, "Test")).children([previous, docked])
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

fn render(runtime: &mut Runtime<Demo>) {
    runtime.render(WIDTH, HEIGHT, &mut Metrics).unwrap();
}

fn rect(runtime: &Runtime<Demo>, name: &str) -> Rect {
    let id = runtime.dom().by_id(name).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

#[test]
fn shift_shortcuts_focus_the_calendar_selectors_without_opening_lists() {
    let mut runtime = Runtime::new(Demo::default()).unwrap();
    render(&mut runtime);
    render(&mut runtime);
    let shifted = |letter: &str| {
        KeyPress::new(
            Key::Character(letter.into()),
            Modifiers {
                shift: true,
                ..Modifiers::default()
            },
        )
    };
    runtime.activate(INPUT).unwrap();
    assert!(!runtime.key(&shifted("M")).unwrap());
    assert_eq!(runtime.focused(), Some(INPUT));
    assert!(runtime.dom().by_id(POPUP).is_none());

    runtime.activate(TRIGGER).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("docked-day-2024-02-28"));
    for (letter, selector, choice, mode) in [
        (
            "M",
            "docked-month",
            "docked-month-02",
            DockedDatePickerMode::Month,
        ),
        (
            "y",
            "docked-year",
            "docked-year-2024",
            DockedDatePickerMode::Year,
        ),
    ] {
        assert!(runtime.key(&shifted(letter)).unwrap());
        assert_eq!(runtime.focused(), Some(selector));
        assert_eq!(
            runtime.app().picker.mode(),
            Some(DockedDatePickerMode::Calendar)
        );
        render(&mut runtime);
        let id = runtime.dom().by_id(selector).unwrap();
        let layout = &runtime.computed().unwrap().nodes[id.index()];
        assert_eq!(layout.bounds.intersect(layout.clip), layout.bounds);
        runtime.dispatch(Event::Activate).unwrap();
        render(&mut runtime);
        assert_eq!(runtime.app().picker.mode(), Some(mode));
        assert_eq!(runtime.focused(), Some(choice));
        assert!(!runtime.key(&shifted(letter)).unwrap());
        runtime.dispatch(Event::Cancel).unwrap();
        render(&mut runtime);
        assert_eq!(runtime.focused(), Some(selector));
    }
    assert_eq!(runtime.app().picker.raw_input(), "2024/02/28");
    assert_eq!(runtime.app().commits, 0);
}

#[test]
fn docked_pointer_open_and_month_list_keep_the_unsubmitted_editor_live() {
    let mut runtime = Runtime::new(Demo::default()).unwrap();
    render(&mut runtime);
    render(&mut runtime);
    assert!(runtime.dom().by_id(POPUP).is_none());
    for target in ["previous", INPUT, TRIGGER] {
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some(target));
    }
    runtime
        .dispatch(Event::FocusNext { reverse: true })
        .unwrap();
    assert_eq!(runtime.focused(), Some(INPUT));
    runtime
        .dispatch(Event::Edit {
            key: EditKey::SelectAll,
            by_word: false,
            extend: false,
        })
        .unwrap();
    runtime.dispatch(Event::Text("2024/3/2".into())).unwrap();
    render(&mut runtime);

    let icon = rect(&runtime, TRIGGER);
    let (x, y) = (icon.x + icon.width / 2., icon.y + icon.height / 2.);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some(INPUT));
    assert_eq!(runtime.app().commits, 0);
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    render(&mut runtime);
    assert_eq!(
        runtime.app().picker.mode(),
        Some(DockedDatePickerMode::Calendar)
    );
    assert_eq!(runtime.focused(), Some("docked-day-2024-02-28"));
    assert_eq!(runtime.app().picker.raw_input(), "2024/3/2");
    assert_eq!(runtime.app().commits, 0);
    let field = rect(&runtime, FIELD);
    let popup = rect(&runtime, POPUP);
    assert!(popup.y > field.y + field.height);
    assert_eq!(popup.width, WIDTH);
    let adjacent = runtime
        .dom()
        .get(runtime.dom().by_id("docked-adjacent-2024-01-28").unwrap())
        .unwrap();
    assert_eq!(adjacent.accessibility_grid_role(), Some(GridRole::GridCell));
    assert!(adjacent.is_disabled());
    assert!(!adjacent.is_focusable());
    for id in ["docked-prev", "docked-next", "docked-month", "docked-year"] {
        let control = rect(&runtime, id);
        assert!(
            control.width >= 48. && control.height >= 48.,
            "{id}: {control:?}"
        );
    }

    let month = rect(&runtime, "docked-month");
    let (x, y) = (month.x + month.width / 2., month.y + month.height / 2.);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("docked-month-02"));
    runtime
        .dispatch(Event::Edit {
            key: EditKey::Down,
            by_word: false,
            extend: false,
        })
        .unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("docked-month-03"));
    assert!(runtime.activate("docked-month-03").unwrap());
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("docked-month"));
    assert_eq!(
        runtime.app().picker.mode(),
        Some(DockedDatePickerMode::Calendar)
    );
    assert_eq!(runtime.app().picker.calendar().month().month(), 3);
    assert_eq!(runtime.app().picker.committed(), Some(date(2024, 2, 28)));
    assert_eq!(runtime.app().commits, 0);

    assert!(runtime.activate("docked-day-2024-03-02").unwrap());
    render(&mut runtime);
    assert!(runtime.dom().by_id(POPUP).is_none());
    assert_eq!(runtime.focused(), Some(TRIGGER));
    assert_eq!(runtime.app().commits, 1);
    assert_eq!(runtime.app().picker.committed(), Some(date(2024, 3, 2)));
    assert_eq!(runtime.app().picker.raw_input(), "2024/03/02");
}
