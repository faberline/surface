use surface::{Node, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityAdjustment, AccessibilityAdjustmentResult, AnimationFrame, AnimationUpdate,
    Application, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    scroll_choice_field_layout, shape::LayoutDirection, ScrollChoiceEvent,
    ScrollChoiceFieldOptions, ScrollChoiceItem, ScrollChoiceQueryError, ScrollChoiceState,
    ScrollFieldSize, Theme,
};

const WHEEL: &str = "place-adjustable";
const EDITOR: &str = "place-editor";

fn places(reverse: bool) -> [ScrollChoiceItem<'static>; 4] {
    let mut choices = [
        ScrollChoiceItem {
            id: "banqiao",
            label: "板橋",
            detail: None,
            aliases: &[],
        },
        ScrollChoiceItem {
            id: "north",
            label: "台北",
            detail: Some("北門"),
            aliases: &["台北北門"],
        },
        ScrollChoiceItem {
            id: "station",
            label: "台北",
            detail: Some("車站"),
            aliases: &["台北車站"],
        },
        ScrollChoiceItem {
            id: "taoyuan",
            label: "桃園",
            detail: None,
            aliases: &[],
        },
    ];
    if reverse {
        choices.reverse();
    }
    choices
}

#[derive(Clone)]
enum Message {
    Field(ScrollChoiceEvent),
    Reverse,
    Relabel,
}

struct Form {
    state: ScrollChoiceState,
    commits: Vec<String>,
    available: (f32, f32),
    scale: f32,
}

impl Form {
    fn new(scale: f32) -> Self {
        Self {
            state: ScrollChoiceState::new(places(false), "banqiao").unwrap(),
            commits: Vec::new(),
            available: (960., 760.),
            scale,
        }
    }

    fn options(&self) -> ScrollChoiceFieldOptions<'_> {
        ScrollChoiceFieldOptions {
            id: "place",
            label: "地點",
            size: ScrollFieldSize::Medium,
            height: 120.,
            available_width: self.available.0,
            available_height: self.available.1,
            text_scale: self.scale,
            direction: LayoutDirection::RightToLeft,
            disabled: false,
            hint: Some("可輸入部分地名"),
            use_input: "輸入地點",
            use_scroll: "捲動選取",
            error_text: &|error| match error {
                ScrollChoiceQueryError::Ambiguous { .. } => "請輸入完整名稱".to_owned(),
                _ => "找不到地點".to_owned(),
            },
            choice_text: &|label, detail| {
                detail.map_or_else(|| label.to_owned(), |detail| format!("{label} {detail}"))
            },
            suggestion_text: &|suggestion| format!("建議 {suggestion}"),
        }
    }
}

impl Application for Form {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Field(event) => {
                if let Some(id) = self.state.apply(event, false) {
                    self.commits.push(id);
                }
            }
            Message::Reverse => {
                let selected = self.state.choice().id.to_owned();
                self.state.replace_items(places(true), &selected).unwrap();
            }
            Message::Relabel => {
                let mut choices = places(false);
                choices[0].label = "板橋站";
                let selected = self.state.choice().id.to_owned();
                self.state.replace_items(choices, &selected).unwrap();
            }
        }
    }

    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        self.available = (width, height);
        let fits = scroll_choice_field_layout(self.options(), &self.state).is_some();
        self.state.invalidate_layout();
        self.state.update_fit(fits);
        true
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        self.state.advance(frame.now_ms, frame.reduced_motion)
    }

    fn view(&self) -> View<Message> {
        View::new(Node::new(Role::Window, "").styled(Style::column())).children([
            Theme::dark().scroll_choice_field(self.options(), &self.state, Message::Field)
        ])
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
        let lines = if width > 0. {
            (natural / width).ceil().max(1.)
        } else {
            1.
        };
        (width, size * 1.5 * lines)
    }
}

fn render(runtime: &mut Runtime<Form>, width: f32) {
    runtime.render_extended(width, 760., &mut Metrics).unwrap();
}

fn press(runtime: &mut Runtime<Form>, key: Key) {
    assert!(runtime
        .key(&KeyPress::new(key, Modifiers::default()))
        .unwrap());
}

fn input(runtime: &mut Runtime<Form>, value: &str) {
    runtime
        .dispatch(Event::InputValue {
            value: value.to_owned(),
            anchor: value.len(),
            cursor: value.len(),
        })
        .unwrap();
}

#[test]
fn projected_ordinal_selects_current_ids_and_rejects_targets_from_old_order() {
    let mut runtime = Runtime::new(Form::new(1.)).unwrap();
    render(&mut runtime, 960.);
    let id = runtime.dom().by_id(WHEEL).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.children(id).count(), 0);
    let range = frame
        .node(id)
        .unwrap()
        .node
        .accessibility_adjustable()
        .unwrap();
    assert_eq!(
        (range.min, range.max, range.step, range.value),
        (1., 4., 1., Some(1.))
    );
    assert_eq!(range.value_text.as_deref(), Some("板橋"));
    let old = frame.adjustment_target(id).unwrap();

    runtime.apply(Message::Reverse).unwrap();
    render(&mut runtime, 960.);
    assert_eq!(runtime.app().state.choice().id, "banqiao");
    assert!(runtime.app().commits.is_empty());
    assert_eq!(
        runtime
            .accessibility_adjust(&old, AccessibilityAdjustment::SetValue(2.))
            .unwrap(),
        AccessibilityAdjustmentResult::Rejected,
    );
    let id = runtime.dom().by_id(WHEEL).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(
        frame
            .node(id)
            .unwrap()
            .node
            .accessibility_adjustable()
            .unwrap()
            .value,
        Some(4.)
    );
    let current = frame.adjustment_target(id).unwrap();
    assert_eq!(
        runtime
            .accessibility_adjust(&current, AccessibilityAdjustment::Decrement)
            .unwrap(),
        AccessibilityAdjustmentResult::Adjusted,
    );
    render(&mut runtime, 960.);
    assert_eq!(runtime.app().commits, ["north"]);
    let id = runtime.dom().by_id(WHEEL).unwrap();
    assert_eq!(
        runtime
            .dom()
            .get(id)
            .unwrap()
            .accessibility_adjustable()
            .unwrap()
            .value_text
            .as_deref(),
        Some("台北 北門")
    );
}

#[test]
fn editor_reports_the_duplicate_error_and_commits_only_an_explicit_choice() {
    let mut runtime = Runtime::new(Form::new(1.)).unwrap();
    render(&mut runtime, 960.);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(WHEEL));
    press(&mut runtime, Key::Enter);
    render(&mut runtime, 960.);
    assert_eq!(runtime.focused(), Some(EDITOR));
    input(&mut runtime, "台北");
    press(&mut runtime, Key::Enter);
    render(&mut runtime, 960.);
    assert_eq!(runtime.focused(), Some(EDITOR));
    assert!(runtime.app().commits.is_empty());
    let field = runtime.dom().by_id(EDITOR).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.node(field).unwrap().node.is_invalid());
    assert_eq!(
        frame.description_target(field),
        runtime.dom().by_id("place-error")
    );

    input(&mut runtime, "桃");
    render(&mut runtime, 960.);
    let field = runtime.dom().by_id(EDITOR).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(!frame.node(field).unwrap().node.is_invalid());
    let suggestion = runtime.dom().by_id("place-suggestion").unwrap();
    assert_eq!(frame.description_target(field), Some(suggestion));
    assert_eq!(
        runtime.dom().get(suggestion).unwrap().text_value(),
        Some("建議 桃園")
    );
    assert_eq!(runtime.input_snapshot().unwrap().value, "桃");
    assert!(runtime.app().commits.is_empty());

    press(&mut runtime, Key::Enter);
    render(&mut runtime, 960.);
    assert_eq!(runtime.focused(), Some(WHEEL));
    assert_eq!(runtime.app().commits, ["taoyuan"]);
}

#[test]
fn resize_and_catalog_refresh_keep_the_fallback_editor_and_its_active_composition() {
    let mut runtime = Runtime::new(Form::new(2.)).unwrap();
    render(&mut runtime, 264.);
    assert!(runtime.dom().by_id(WHEEL).is_none());
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(EDITOR));
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "tao".into(),
            selection: None,
        }))
        .unwrap();

    render(&mut runtime, 960.);
    assert_eq!(runtime.focused(), Some(EDITOR));
    assert!(runtime.input_snapshot().unwrap().composing);
    assert_eq!(runtime.app().state.raw(), "板橋");
    assert!(runtime.app().commits.is_empty());

    runtime.apply(Message::Relabel).unwrap();
    render(&mut runtime, 960.);
    assert_eq!(runtime.focused(), Some(EDITOR));
    assert!(runtime.input_snapshot().unwrap().composing);
    assert_eq!(runtime.app().state.raw(), "板橋");
    assert_eq!(runtime.app().state.choice().label, "板橋站");
    assert!(runtime.app().commits.is_empty());

    runtime.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();
    input(&mut runtime, "台北車站");
    press(&mut runtime, Key::Enter);
    render(&mut runtime, 960.);
    assert_eq!(runtime.focused(), Some(WHEEL));
    assert_eq!(runtime.app().commits, ["station"]);
}
