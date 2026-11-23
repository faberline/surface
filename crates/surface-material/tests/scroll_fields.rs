use surface::{Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityAdjustment, AccessibilityAdjustmentResult, AnimationFrame, AnimationUpdate,
    Application, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    scroll_field_layout, shape::LayoutDirection, ScrollFieldEvent, ScrollFieldMode,
    ScrollFieldOptions, ScrollFieldRange, ScrollFieldSize, ScrollFieldState, Theme,
};

const FIELD: &str = "quantity-adjustable";
const EDITOR: &str = "quantity-editor";

struct Form {
    field: ScrollFieldState,
    commits: Vec<i32>,
    width: f32,
    height: f32,
    scale: f32,
    disabled: bool,
}

impl Form {
    fn new(range: (i32, i32), value: i32) -> Self {
        Self {
            field: ScrollFieldState::new(
                ScrollFieldRange::new(range.0, range.1, 2).unwrap(),
                value,
            )
            .unwrap(),
            commits: Vec::new(),
            width: 800.,
            height: 640.,
            scale: 1.,
            disabled: false,
        }
    }

    fn options(&self) -> ScrollFieldOptions<'_> {
        ScrollFieldOptions {
            id: "quantity",
            label: "數量",
            size: ScrollFieldSize::Medium,
            height: 120.,
            available_width: self.width,
            available_height: self.height,
            text_scale: self.scale,
            direction: LayoutDirection::RightToLeft,
            disabled: self.disabled,
            hint: Some("請選數量"),
            use_input: "輸入數量",
            use_scroll: "捲動選取",
            error_text: &|error| format!("{error:?}"),
            value_text: &|value| format!("{value} 件"),
        }
    }
}

impl Application for Form {
    type Message = ScrollFieldEvent;

    fn update(&mut self, message: Self::Message) {
        if let Some(value) = self.field.apply(message, false) {
            self.commits.push(value);
        }
    }

    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        self.width = width;
        self.height = height;
        let fits = scroll_field_layout(self.options(), &self.field).is_some();
        self.field.invalidate_layout();
        self.field.update_fit(fits);
        true
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        self.field.advance(frame.now_ms, frame.reduced_motion)
    }

    fn view(&self) -> View<Self::Message> {
        View::new(Node::new(Role::Window, "").styled(Style::column()))
            .children([Theme::light().scroll_field(self.options(), &self.field, |event| event)])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, available_width: Option<f32>) -> (f32, f32) {
        let natural = text
            .chars()
            .map(|ch| if ch.is_ascii() { size * 0.58 } else { size })
            .sum::<f32>();
        let width = available_width.map_or(natural, |width| natural.min(width.max(1.)));
        let lines = if width > 0. {
            (natural / width).ceil().max(1.)
        } else {
            1.
        };
        (width, size * 1.5 * lines)
    }
}

fn render(runtime: &mut Runtime<Form>, width: f32) {
    runtime.render_extended(width, 640., &mut Metrics).unwrap();
}

fn rect(runtime: &Runtime<Form>, id: &str) -> Rect {
    let id = runtime.dom().by_id(id).expect("scroll field part");
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn send_text(runtime: &mut Runtime<Form>, value: &str) {
    runtime
        .dispatch(Event::InputValue {
            value: value.to_owned(),
            anchor: value.len(),
            cursor: value.len(),
        })
        .unwrap();
}

#[test]
fn cropped_neighbor_target_accepts_a_pointer_tap_outside_the_face() {
    let mut runtime = Runtime::new(Form::new((0, 30), 14)).unwrap();
    runtime.set_reduced_motion(true);
    render(&mut runtime, 800.);
    let face = rect(&runtime, "quantity-face");
    let target = rect(&runtime, "quantity-above-target");
    assert_eq!(face.height, 120.);
    assert!(face.width >= 120.);
    assert!(target.height >= 48.);
    assert!(target.y < face.y && target.y + target.height > face.y);
    let x = target.x + target.width / 2.;
    let y = (face.y + target.y) / 2.;
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    render(&mut runtime, 800.);
    assert_eq!(runtime.app().field.value(), 16);
    assert_eq!(runtime.app().commits, [16]);
    assert!(!runtime.needs_frame());

    runtime.dispatch(Event::Blur).unwrap();
    render(&mut runtime, 800.);
    let face = rect(&runtime, "quantity-face");
    let x = face.x + face.width / 2.;
    let y = face.y + face.height / 2.;
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    render(&mut runtime, 800.);
    assert_eq!(runtime.focused(), Some(EDITOR));
    assert_eq!(runtime.app().commits, [16]);
}

#[test]
fn current_accessible_adjustment_commits_once_and_retires_its_target() {
    let mut runtime = Runtime::new(Form::new((0, 30), 14)).unwrap();
    render(&mut runtime, 800.);
    let id = runtime.dom().by_id(FIELD).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    let source = frame.node(id).unwrap();
    assert_eq!(source.node.name(), "數量");
    assert_eq!(frame.children(id).count(), 0);
    let current = source.node.accessibility_adjustable().unwrap();
    assert_eq!((current.min, current.max, current.step), (0., 30., 2.));
    assert_eq!(current.value_text.as_deref(), Some("14 件"));
    let target = frame.adjustment_target(id).unwrap();
    assert_eq!(
        runtime
            .accessibility_adjust(&target, AccessibilityAdjustment::Increment)
            .unwrap(),
        AccessibilityAdjustmentResult::Adjusted,
    );
    assert_eq!(
        runtime
            .accessibility_adjust(&target, AccessibilityAdjustment::Increment)
            .unwrap(),
        AccessibilityAdjustmentResult::Rejected,
    );
    assert_eq!(runtime.app().commits, [16]);
    render(&mut runtime, 800.);
    let id = runtime.dom().by_id(FIELD).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(
        frame
            .node(id)
            .unwrap()
            .node
            .accessibility_adjustable()
            .unwrap()
            .value,
        Some(16.),
    );
}

#[test]
fn compact_edit_keeps_focus_and_draft_when_space_returns() {
    let mut app = Form::new((-1800, 1800), 1200);
    app.scale = 2.;
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime, 264.);
    assert!(runtime.dom().by_id(FIELD).is_none());
    for _ in 0..3 {
        if runtime.focused() == Some(EDITOR) {
            break;
        }
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
    }
    assert_eq!(runtime.focused(), Some(EDITOR));
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "１３００".to_owned(),
            selection: None,
        }))
        .unwrap();
    render(&mut runtime, 800.);
    assert_eq!(runtime.focused(), Some(EDITOR));
    assert!(runtime.input_snapshot().unwrap().composing);
    assert_eq!(runtime.app().field.raw(), "1200");
    render(&mut runtime, 264.);
    runtime.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();
    send_text(&mut runtime, "１３０１");
    assert!(runtime
        .key(&KeyPress::new(Key::Enter, Modifiers::default()))
        .unwrap());
    render(&mut runtime, 264.);
    assert_eq!(runtime.focused(), Some(EDITOR));
    assert_eq!(runtime.app().field.raw(), "１３０１");
    assert!(runtime.dom().by_id("quantity-error").is_some());
    let editor = runtime
        .dom()
        .get(runtime.dom().by_id(EDITOR).unwrap())
        .unwrap();
    assert!(editor.is_invalid());
    assert!(runtime.app().commits.is_empty());
    render(&mut runtime, 800.);
    assert_eq!(runtime.focused(), Some(EDITOR));
    assert_eq!(runtime.app().field.mode(), ScrollFieldMode::Input);
    assert_eq!(runtime.app().field.raw(), "１３０１");
    send_text(&mut runtime, "１３００");
    assert!(runtime
        .key(&KeyPress::new(Key::Enter, Modifiers::default()))
        .unwrap());
    render(&mut runtime, 800.);
    assert_eq!(runtime.focused(), Some(FIELD));
    assert_eq!(runtime.app().commits, [1300]);
    assert_eq!(runtime.app().field.mode(), ScrollFieldMode::Scroll);
}
