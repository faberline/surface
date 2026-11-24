use surface::{Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityAdjustment, AccessibilityAdjustmentResult, Application, EditKey, Event, ImeEvent,
    Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    scroll_field_pair_layout, shape::LayoutDirection, ScrollChoiceEvent, ScrollChoiceFieldOptions,
    ScrollChoiceItem, ScrollChoiceState, ScrollFieldEvent, ScrollFieldOptions,
    ScrollFieldPairLayout, ScrollFieldPairOptions, ScrollFieldRange, ScrollFieldSize,
    ScrollFieldState, Theme,
};

const QUANTITY_WHEEL: &str = "quantity-adjustable";
const UNIT_WHEEL: &str = "unit-adjustable";
const UNIT_EDITOR: &str = "unit-editor";
const PAIR: &str = "water";

#[derive(Clone)]
enum Message {
    Quantity(ScrollFieldEvent),
    Unit(ScrollChoiceEvent),
}

struct Measurement {
    quantity: ScrollFieldState,
    unit: ScrollChoiceState,
    quantity_commits: Vec<i32>,
    unit_commits: Vec<String>,
    available: (f32, f32),
    scale: f32,
    direction: LayoutDirection,
}

impl Measurement {
    fn new(scale: f32, direction: LayoutDirection) -> Self {
        let units = [
            ScrollChoiceItem {
                id: "milliliter",
                label: "毫升",
                detail: None,
                aliases: &["mL"],
            },
            ScrollChoiceItem {
                id: "liter",
                label: "公升",
                detail: Some("L"),
                aliases: &["L"],
            },
        ];
        Self {
            quantity: ScrollFieldState::new(ScrollFieldRange::new(0, 30, 2).unwrap(), 14).unwrap(),
            unit: ScrollChoiceState::new(units, "milliliter").unwrap(),
            quantity_commits: Vec::new(),
            unit_commits: Vec::new(),
            available: (960., 760.),
            scale,
            direction,
        }
    }

    fn options(&self) -> ScrollFieldPairOptions<'_> {
        ScrollFieldPairOptions {
            id: PAIR,
            label: "飲水量",
            available_width: self.available.0,
            available_height: self.available.1,
            direction: self.direction,
            quantity: ScrollFieldOptions {
                id: "quantity",
                label: "數量",
                size: ScrollFieldSize::Small,
                height: 120.,
                available_width: 1.,
                available_height: 1.,
                text_scale: self.scale,
                direction: LayoutDirection::LeftToRight,
                disabled: false,
                hint: Some("每次增減 2"),
                use_input: "輸入數量",
                use_scroll: "捲動數量",
                error_text: &|_| "請輸入數量".into(),
                value_text: &|value| value.to_string(),
            },
            unit: ScrollChoiceFieldOptions {
                id: "unit",
                label: "單位",
                size: ScrollFieldSize::Medium,
                height: 200.,
                available_width: 1.,
                available_height: 1.,
                text_scale: self.scale,
                direction: LayoutDirection::LeftToRight,
                disabled: false,
                hint: Some("可輸入單位名稱"),
                use_input: "輸入單位",
                use_scroll: "捲動單位",
                error_text: &|_| "請輸入單位".into(),
                choice_text: &|label, detail| {
                    detail.map_or_else(|| label.to_owned(), |detail| format!("{label} {detail}"))
                },
                suggestion_text: &|suggestion| format!("建議 {suggestion}"),
            },
        }
    }

    fn plan(&self) -> ScrollFieldPairLayout {
        scroll_field_pair_layout(self.options(), &self.quantity, &self.unit)
    }
}

impl Application for Measurement {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Quantity(event) => {
                self.quantity_commits
                    .extend(self.quantity.apply(event, true));
            }
            Message::Unit(event) => {
                self.unit_commits.extend(self.unit.apply(event, true));
            }
        }
    }

    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        self.available = (width, height);
        let layout = self.plan();
        self.quantity.invalidate_layout();
        self.unit.invalidate_layout();
        self.quantity.update_fit(layout.quantity_fits);
        self.unit.update_fit(layout.unit_fits);
        true
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn view(&self) -> View<Message> {
        let pair = Theme::light().scroll_field_pair(
            self.options(),
            &self.quantity,
            &self.unit,
            Message::Quantity,
            Message::Unit,
        );
        View::new(Node::new(Role::Window, "").styled(Style::column())).children([View::new(
            Node::new(Role::Group, "測量內容").id("page").styled(
                Style::column()
                    .width(Length::Percent(1.))
                    .height(Length::Percent(1.))
                    .clip(true),
            ),
        )
        .children([pair])
        .accessibility_scroll_pages()])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, limit: Option<f32>) -> (f32, f32) {
        let natural = text
            .chars()
            .map(|ch| if ch.is_ascii() { size * 0.58 } else { size })
            .sum::<f32>();
        let width = limit.map_or(natural, |width| natural.min(width.max(1.)));
        let lines = (natural / width.max(1.)).ceil().max(1.);
        (width, size * 1.5 * lines)
    }
}

fn render(runtime: &mut Runtime<Measurement>, width: f32, height: f32) {
    runtime
        .render_extended(width, height, &mut Metrics)
        .unwrap();
}

fn rect(runtime: &Runtime<Measurement>, name: &str) -> Rect {
    let id = runtime.dom().by_id(name).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn increment(runtime: &mut Runtime<Measurement>, name: &str) {
    let id = runtime.dom().by_id(name).unwrap();
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .adjustment_target(id)
        .unwrap();
    assert_eq!(
        runtime
            .accessibility_adjust(&target, AccessibilityAdjustment::Increment)
            .unwrap(),
        AccessibilityAdjustmentResult::Adjusted,
    );
}

#[test]
fn rtl_pair_mirrors_geometry_and_keeps_authored_focus_and_separate_actions() {
    let mut runtime = Runtime::new(Measurement::new(1., LayoutDirection::RightToLeft)).unwrap();
    render(&mut runtime, 960., 760.);
    assert_eq!(
        runtime.app().plan(),
        ScrollFieldPairLayout {
            stacked: false,
            quantity_fits: true,
            unit_fits: true,
        },
    );
    let quantity = rect(&runtime, "quantity");
    let unit = rect(&runtime, "unit");
    assert_eq!(quantity.x - unit.x - unit.width, 16.);
    assert_eq!(
        rect(&runtime, PAIR).width,
        quantity.width + 16. + unit.width
    );
    assert_eq!(quantity.y, unit.y);
    assert_eq!(rect(&runtime, "quantity-face").height, 120.);
    assert_eq!(rect(&runtime, "unit-face").height, 200.);
    assert!(rect(&runtime, QUANTITY_WHEEL).x > rect(&runtime, "quantity-use-input").x);
    assert!(rect(&runtime, UNIT_WHEEL).x > rect(&runtime, "unit-use-input").x);

    let dom = runtime.dom();
    let frame = runtime.accessibility_frame().unwrap();
    let group_id = dom.by_id(PAIR).unwrap();
    let group = frame.node(group_id).unwrap().node;
    assert_eq!(group.role(), Role::Group);
    assert_eq!(group.name(), "飲水量");
    assert!(frame.focus_target(group_id).is_none());
    assert_eq!(
        frame.children(group_id).collect::<Vec<_>>(),
        [dom.by_id("quantity").unwrap(), dom.by_id("unit").unwrap()],
    );
    for (id, label, value, text) in [
        (QUANTITY_WHEEL, "數量", 14., "14"),
        (UNIT_WHEEL, "單位", 1., "毫升"),
    ] {
        let id = dom.by_id(id).unwrap();
        let control = frame.node(id).unwrap().node;
        let range = control.accessibility_adjustable().unwrap();
        assert_eq!(control.name(), label);
        assert_eq!(range.value, Some(value));
        assert_eq!(range.value_text.as_deref(), Some(text));
        assert!(frame.adjustment_target(id).is_some());
    }

    for expected in [
        QUANTITY_WHEEL,
        "quantity-use-input",
        UNIT_WHEEL,
        "unit-use-input",
    ] {
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some(expected));
    }
    render(&mut runtime, 960., 760.);
    increment(&mut runtime, QUANTITY_WHEEL);
    assert_eq!(runtime.app().quantity_commits, [16]);
    assert_eq!(runtime.app().unit.choice().id, "milliliter");
    assert!(runtime.app().unit_commits.is_empty());

    render(&mut runtime, 960., 760.);
    increment(&mut runtime, UNIT_WHEEL);
    assert_eq!(runtime.app().unit_commits, ["liter"]);
    assert_eq!(runtime.app().quantity_commits, [16]);
    assert_eq!(runtime.app().quantity.value(), 16);
    assert_eq!(runtime.focused(), Some("unit-use-input"));
}

#[test]
fn pair_breakpoint_includes_both_mode_controls_and_stacks_at_logical_start() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let mut runtime = Runtime::new(Measurement::new(1., direction)).unwrap();
        render(&mut runtime, 960., 760.);
        let breakpoint = rect(&runtime, PAIR).width;
        assert!(breakpoint < 960.);

        render(&mut runtime, breakpoint, 760.);
        assert!(!runtime.app().plan().stacked);
        assert_eq!(rect(&runtime, PAIR).width, breakpoint);

        render(&mut runtime, breakpoint - 1., 760.);
        assert_eq!(
            runtime.app().plan(),
            ScrollFieldPairLayout {
                stacked: true,
                quantity_fits: true,
                unit_fits: true,
            },
        );
        let group = rect(&runtime, PAIR);
        let quantity = rect(&runtime, "quantity");
        let unit = rect(&runtime, "unit");
        assert_eq!(unit.y - quantity.y - quantity.height, 16.);
        for child in [quantity, unit] {
            if direction == LayoutDirection::RightToLeft {
                assert_eq!(child.x + child.width, group.x + group.width);
            } else {
                assert_eq!(child.x, group.x);
            }
        }
    }
}

#[test]
fn compact_pair_keeps_the_unit_composition_through_both_width_and_height_changes() {
    let mut runtime = Runtime::new(Measurement::new(2., LayoutDirection::RightToLeft)).unwrap();
    render(&mut runtime, 320., 760.);
    assert_eq!(
        runtime.app().plan(),
        ScrollFieldPairLayout {
            stacked: true,
            quantity_fits: true,
            unit_fits: false,
        },
    );
    assert!(runtime.dom().by_id(UNIT_WHEEL).is_none());
    let editor = rect(&runtime, UNIT_EDITOR);
    assert!(editor.width >= 48. && editor.height >= 48.);
    assert!(editor.x >= 0. && editor.x + editor.width <= 320.);

    assert!(runtime.activate(UNIT_EDITOR).unwrap());
    runtime
        .dispatch(Event::Edit {
            key: EditKey::SelectAll,
            extend: false,
            by_word: false,
        })
        .unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "公".into(),
            selection: None,
        }))
        .unwrap();

    for (height, fits) in [(240., false), (760., true)] {
        render(&mut runtime, 960., height);
        assert_eq!(
            runtime.app().plan(),
            ScrollFieldPairLayout {
                stacked: false,
                quantity_fits: fits,
                unit_fits: fits,
            },
        );
        assert_eq!(runtime.focused(), Some(UNIT_EDITOR));
        assert!(runtime.input_snapshot().unwrap().composing);
        assert_eq!(
            runtime
                .editor(UNIT_EDITOR)
                .unwrap()
                .composition()
                .unwrap()
                .text,
            "公"
        );
        assert_eq!(runtime.app().unit.raw(), "毫升");
        assert!(runtime.app().unit_commits.is_empty());
        assert!(runtime.app().quantity_commits.is_empty());
    }

    let enter = KeyPress::new(Key::Enter, Modifiers::default());
    assert!(!runtime.key(&enter).unwrap());
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("公".into())))
        .unwrap();
    assert_eq!(runtime.app().unit.raw(), "公");
    assert!(runtime.app().unit_commits.is_empty());
    assert!(runtime.key(&enter).unwrap());
    render(&mut runtime, 960., 760.);
    assert_eq!(runtime.focused(), Some(UNIT_WHEEL));
    assert_eq!(runtime.app().unit_commits, ["liter"]);
    assert_eq!(runtime.app().quantity.value(), 14);
    assert!(runtime.app().quantity_commits.is_empty());
}
