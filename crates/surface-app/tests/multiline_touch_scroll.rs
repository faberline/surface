use surface::{
    DisplayList, Insets, Length, Node, PaintOp, Role, Style, TextAlignment, TextMeasurer,
};
use surface_app::{Application, Event, Runtime, TextOptions, VerticalScrollResponse, View};

const VALUE: &str = "a\nb\nc\nd\ne\nf";
const FIELD: &str = "editor";
const OUTER: &str = "form";

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size, size * 1.5)
    }
}

#[derive(Clone)]
enum Message {
    Edit,
    Wheel,
    RemoveOptIn,
}

struct Form {
    value: &'static str,
    opted_in: bool,
    outer: bool,
    custom_outer: bool,
    edits: usize,
    wheel_calls: usize,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            value: VALUE,
            opted_in: true,
            outer: true,
            custom_outer: false,
            edits: 0,
            wheel_calls: 0,
        }
    }
}

impl Application for Form {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Edit => self.edits += 1,
            Message::Wheel => self.wheel_calls += 1,
            Message::RemoveOptIn => self.opted_in = false,
        }
    }

    fn view(&self) -> View<Message> {
        let editor = View::new(
            Node::new(Role::TextInput, "Notes")
                .id(FIELD)
                .text(self.value)
                .styled(
                    Style::column()
                        .height(Length::Px(72.))
                        .font_size(16.)
                        .line_height(24.),
                ),
        )
        .text_options(TextOptions {
            multiline: true,
            ..Default::default()
        })
        .on_input(|_| Message::Edit);
        let editor = if self.opted_in {
            editor.multiline_text_viewport_from("text-viewport", TextAlignment::Left)
        } else {
            editor
        };
        let viewport = View::new(
            Node::new(Role::Group, "").id("text-viewport").styled(
                Style::column()
                    .width(Length::Px(100.))
                    .height(Length::Px(48.))
                    .absolute(Insets {
                        left: Length::Px(20.),
                        top: Length::Px(12.),
                        ..Default::default()
                    }),
            ),
        )
        .without_interaction(None);
        let section =
            View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(72.))))
                .children([editor, viewport]);
        let space =
            View::new(
                Node::new(Role::Group, "")
                    .styled(Style::column().height(Length::Px(if self.outer { 200. } else { 0. }))),
            );
        let form = View::new(
            Node::new(Role::Group, "Form")
                .id(OUTER)
                .styled(Style::column().height(Length::Px(100.)).clip(true)),
        )
        .children([section, space]);
        if self.custom_outer {
            form.on_vertical_scroll(|scroll| {
                Some(VerticalScrollResponse {
                    consumed: scroll.delta,
                    message: Message::Wheel,
                })
            })
        } else {
            form
        }
    }
}

fn render(runtime: &mut Runtime<Form>) -> DisplayList {
    runtime.render(180., 160., &mut Metrics).unwrap()
}

fn offsets(runtime: &mut Runtime<Form>) -> (f32, f32) {
    let inner = render(runtime)
        .ops
        .into_iter()
        .find_map(|op| match op {
            PaintOp::StyledText { text, scroll, .. } if text == runtime.app().value => {
                Some(scroll.1)
            }
            _ => None,
        })
        .expect("the editor paints its text");
    (inner, runtime.scroll_metrics(OUTER).unwrap().offset.1)
}

fn down(runtime: &mut Runtime<Form>, y: f32) {
    runtime
        .dispatch_touch(Event::PointerDown { x: 21., y })
        .unwrap();
}

fn moved(runtime: &mut Runtime<Form>, y: f32) {
    runtime
        .dispatch_touch(Event::PointerMove { x: 21., y })
        .unwrap();
}

fn up(runtime: &mut Runtime<Form>, y: f32) {
    runtime
        .dispatch_touch(Event::PointerUp { x: 21., y })
        .unwrap();
}

#[test]
fn deferred_tap_places_the_caret_on_release_without_mutating_text() {
    let mut runtime = Runtime::new(Form::default()).unwrap();
    render(&mut runtime);
    let prior = runtime.editor(FIELD).unwrap().selection().clone();

    down(&mut runtime, 38.);
    assert!(runtime.has_touch_scroll_capture());
    assert_eq!(runtime.focused(), None);
    moved(&mut runtime, 40.);
    assert_eq!(runtime.editor(FIELD).unwrap().selection(), &prior);
    up(&mut runtime, 40.);

    assert!(!runtime.has_touch_scroll_capture());
    assert_eq!(runtime.focused(), Some(FIELD));
    let selection = runtime.editor(FIELD).unwrap().selection();
    assert_eq!((selection.anchor, selection.cursor), (2, 2));
    assert_eq!(runtime.editor(FIELD).unwrap().value(), VALUE);
    assert_eq!(runtime.app().edits, 0);
    assert_eq!(offsets(&mut runtime), (0., 0.));
}

#[test]
fn panning_consumes_inner_then_outer_and_reverses_from_contact_origin() {
    let mut runtime = Runtime::new(Form::default()).unwrap();
    runtime.activate(FIELD).unwrap();
    runtime
        .dispatch(Event::Select {
            anchor: 0,
            cursor: 2,
        })
        .unwrap();
    let selection = runtime.editor(FIELD).unwrap().selection().clone();
    assert_eq!(offsets(&mut runtime), (0., 0.));

    down(&mut runtime, 56.);
    moved(&mut runtime, 32.);
    assert_eq!(offsets(&mut runtime), (24., 0.));
    moved(&mut runtime, -64.);
    assert_eq!(offsets(&mut runtime), (96., 24.));
    moved(&mut runtime, -64.);
    assert_eq!(offsets(&mut runtime), (96., 24.));
    moved(&mut runtime, -44.);
    assert_eq!(offsets(&mut runtime), (96., 4.));
    moved(&mut runtime, -36.);
    up(&mut runtime, -16.);
    assert_eq!(offsets(&mut runtime), (72., 0.));

    assert!(!runtime.has_touch_scroll_capture());
    assert_eq!(runtime.focused(), Some(FIELD));
    assert_eq!(runtime.editor(FIELD).unwrap().selection(), &selection);
    assert_eq!(runtime.editor(FIELD).unwrap().value(), VALUE);
    assert_eq!(runtime.app().edits, 0);
}

#[test]
fn release_can_take_the_axis_and_capture_loss_restores_both_previews() {
    let mut runtime = Runtime::new(Form::default()).unwrap();
    assert_eq!(offsets(&mut runtime), (0., 0.));

    down(&mut runtime, 56.);
    up(&mut runtime, 32.);
    assert_eq!(offsets(&mut runtime), (24., 0.));
    assert_eq!(runtime.focused(), None);

    down(&mut runtime, 56.);
    moved(&mut runtime, -46.);
    assert_eq!(offsets(&mut runtime), (96., 30.));
    assert!(runtime.cancel_touch_scroll_capture().unwrap());
    assert_eq!(offsets(&mut runtime), (24., 0.));

    up(&mut runtime, 54.);
    assert!(!runtime.has_touch_scroll_capture());
    assert_eq!(runtime.focused(), None);
    assert_eq!(offsets(&mut runtime), (24., 0.));
}

#[test]
fn resize_and_changed_editor_opt_in_reject_stale_releases() {
    let mut runtime = Runtime::new(Form::default()).unwrap();
    assert_eq!(offsets(&mut runtime), (0., 0.));

    down(&mut runtime, 56.);
    moved(&mut runtime, -64.);
    assert_eq!(offsets(&mut runtime), (96., 24.));
    assert!(runtime.pointer_geometry_changed().unwrap());
    render(&mut runtime);
    up(&mut runtime, 56.);
    assert_eq!(offsets(&mut runtime), (0., 0.));
    assert_eq!(runtime.focused(), None);

    down(&mut runtime, 56.);
    runtime.apply(Message::RemoveOptIn).unwrap();
    render(&mut runtime);
    assert!(!runtime.has_touch_scroll_capture());
    up(&mut runtime, 56.);
    assert_eq!(runtime.focused(), None);
    down(&mut runtime, 32.);
    assert!(!runtime.has_touch_scroll_capture());
    assert_eq!(runtime.focused(), Some(FIELD));
}

#[test]
fn a_fitting_editor_hands_motion_to_the_outer_scroll_area() {
    let mut runtime = Runtime::new(Form {
        value: "a",
        ..Default::default()
    })
    .unwrap();
    assert_eq!(offsets(&mut runtime), (0., 0.));

    down(&mut runtime, 40.);
    assert!(runtime.has_touch_scroll_capture());
    up(&mut runtime, 20.);
    assert_eq!(offsets(&mut runtime), (0., 20.));
    assert_eq!(runtime.focused(), None);

    let mut fitted = Runtime::new(Form {
        value: "a",
        outer: false,
        ..Default::default()
    })
    .unwrap();
    render(&mut fitted);
    down(&mut fitted, 40.);
    assert!(!fitted.has_touch_scroll_capture());
    assert_eq!(fitted.focused(), Some(FIELD));
}

#[test]
fn touch_never_changes_a_custom_vertical_owner_or_triggers_its_wheel_hook() {
    let mut runtime = Runtime::new(Form {
        custom_outer: true,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(offsets(&mut runtime), (0., 0.));
    down(&mut runtime, 56.);
    up(&mut runtime, -64.);
    assert_eq!(offsets(&mut runtime), (96., 0.));
    assert_eq!(runtime.app().wheel_calls, 0);
}
