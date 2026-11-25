use std::{cell::Cell, rc::Rc};

use surface::{
    Color, ExtendedDisplayList, ExtendedPaintOp, FillContour, Length, Node, PaintOp, Rect, Role,
    Style, TextMeasurer,
};
use surface_app::{Application, Event, ImeEvent, Runtime, ScrollTarget, TextInputPaintState, View};

const EDITOR: &str = "field";
const INLINE: &str = "inline";
const FLOATING: &str = "floating";
const OUTLINE: &str = "outline";
const HINT: Color = Color::rgb(22, 35, 61);
const IDLE: Color = Color::rgb(40, 56, 71);
const HOVER: Color = Color::rgb(40, 92, 118);
const FOCUSED: Color = Color::rgb(70, 31, 144);
const MARKER: Color = Color::rgb(33, 44, 55);

type Observation = Rc<Cell<Option<(TextInputPaintState, Option<Rect>, Rect)>>>;

#[derive(Clone)]
enum Message {
    Input(String),
    Modal,
    Refresh,
}

#[derive(Default)]
struct Model {
    value: String,
    disabled: bool,
    modal: bool,
    foreign_reference: bool,
    input_count: usize,
    views: Cell<usize>,
    observation: Observation,
}

impl Application for Model {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Input(value) => {
                self.value = value;
                self.input_count += 1;
            }
            Message::Modal => self.modal = !self.modal,
            Message::Refresh => {}
        }
    }

    fn view(&self) -> View<Message> {
        self.views.set(self.views.get() + 1);
        let editor = View::new(
            Node::new(Role::TextInput, "Editor name")
                .id(EDITOR)
                .text(&self.value)
                .disabled(self.disabled)
                .styled(
                    Style::column()
                        .height(Length::Px(56.))
                        .font_size(16.)
                        .line_height(24.),
                ),
        )
        .center_single_line_text()
        .focused_text_input_placeholder("Hint", HINT)
        .on_input(Message::Input);
        let inline = View::new(Node::new(Role::Text, "").id(INLINE).text("Resting"))
            .paint_text_input_foreground(EDITOR, |state| (!floats(state)).then_some(ink(state)));
        let floating = View::new(Node::new(Role::Text, "").id(FLOATING).text("Floating"))
            .paint_text_input_foreground(EDITOR, |state| floats(state).then_some(ink(state)));
        let observation = self.observation.clone();
        let reference = if self.foreign_reference {
            "remote"
        } else {
            FLOATING
        };
        let outline = View::new(
            Node::new(Role::Group, "")
                .id(OUTLINE)
                .styled(Style::column().height(Length::Px(12.))),
        )
        .paint_text_input_contour_with_bounds(
            EDITOR,
            Some(reference),
            move |state, outline, label| {
                observation.set(Some((state, label, outline)));
                FillContour::new([[0., 0.], [12., 0.], [0., 8.]], ink(state))
            },
        );
        let conditional = View::new(Node::new(Role::Group, "").id("conditionally-painted"))
            .children([View::new(
                Node::new(Role::Text, "")
                    .id("conditional-text")
                    .text("Conditional")
                    .styled(
                        Style::column()
                            .foreground(MARKER)
                            .fixed(surface::Insets::all(12.)),
                    ),
            )])
            .paint_when_text_input(EDITOR, |state| state.is_focused);
        let foreign = View::new(Node::new(Role::Text, "Remote").id("remote").text("Remote"));
        let wrong_owner = View::new(
            Node::new(Role::Text, "Unbound")
                .id("unbound")
                .text("Unbound"),
        )
        .paint_when_text_input("after", |_| panic!("non-editor must not resolve"));
        let shell = View::new(
            Node::new(Role::Group, "Shell")
                .id("shell")
                .styled(Style::column().width(Length::Px(200.)).gap(2.)),
        )
        .children([editor, inline, floating, outline, conditional, wrong_owner]);
        let after = View::new(
            Node::new(Role::Button, "Next")
                .id("after")
                .styled(Style::column().height(Length::Px(36.))),
        )
        .on_press(Message::Refresh);
        let modal = self.modal.then(|| {
            View::new(
                Node::new(Role::Dialog, "Modal").id("modal").styled(
                    Style::column()
                        .width(Length::Px(100.))
                        .height(Length::Px(70.))
                        .fixed(surface::Insets {
                            top: Length::Px(20.),
                            left: Length::Px(220.),
                            ..surface::Insets::default()
                        }),
                ),
            )
            .children([
                View::new(Node::new(Role::Button, "Close").id("close")).on_press(Message::Modal)
            ])
            .modal_overlay(Message::Modal)
        });
        let page =
            View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(520.))))
                .children([shell, after, foreign]);
        View::new(
            Node::new(Role::Window, "Page")
                .id("page")
                .styled(Style::column().clip(true)),
        )
        .children([page].into_iter().chain(modal))
    }
}

fn floats(state: TextInputPaintState) -> bool {
    state.is_focused || state.is_composing || !state.is_empty
}

fn ink(state: TextInputPaintState) -> Color {
    if state.is_focused {
        FOCUSED
    } else if state.is_hovered {
        HOVER
    } else {
        IDLE
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * (size / 2.), size * 1.5)
    }
}

fn draw(runtime: &mut Runtime<Model>) -> ExtendedDisplayList {
    runtime.render_extended(320., 260., &mut Metrics).unwrap()
}

fn geometry(runtime: &Runtime<Model>, key: &str) -> Rect {
    let id = runtime.dom().by_id(key).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn text_ink(list: &ExtendedDisplayList, wanted: &str) -> Option<Color> {
    list.ops().iter().find_map(|op| match op {
        ExtendedPaintOp::Legacy(
            PaintOp::Text { text, color, .. } | PaintOp::StyledText { text, color, .. },
        ) if text == wanted => Some(*color),
        _ => None,
    })
}

fn contour_ink(list: &ExtendedDisplayList) -> Option<Color> {
    list.ops().iter().find_map(|op| match op {
        ExtendedPaintOp::FillPath(path) => Some(path.color()),
        _ => None,
    })
}

#[test]
fn focus_only_changes_paint_and_keeps_one_semantic_editor() {
    let mut runtime = Runtime::new(Model::default()).unwrap();
    let idle = draw(&mut runtime);
    assert_eq!(text_ink(&idle, "Resting"), Some(IDLE));
    assert_eq!(text_ink(&idle, "Floating"), None);
    assert_eq!(text_ink(&idle, "Conditional"), None);
    assert_eq!(text_ink(&idle, "Unbound"), None);
    assert_eq!(text_ink(&idle, "Hint"), None);
    assert_eq!(contour_ink(&idle), Some(IDLE));
    let bounds = geometry(&runtime, EDITOR);
    let label = geometry(&runtime, FLOATING);
    let observation = runtime.app().observation.get().unwrap();
    assert_eq!(observation.0.bounds, bounds);
    assert_eq!(observation.2, geometry(&runtime, OUTLINE));
    assert_ne!(observation.2, bounds);
    assert_eq!(
        observation.1,
        Some(label),
        "reference geometry survives hidden paint"
    );
    let editor = runtime.dom().by_id(EDITOR).unwrap();
    let views = runtime.app().views.get();

    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(EDITOR));
    let focused = draw(&mut runtime);
    assert_eq!(text_ink(&focused, "Floating"), Some(FOCUSED));
    assert_eq!(text_ink(&focused, "Resting"), None);
    assert_eq!(text_ink(&focused, "Conditional"), Some(MARKER));
    assert_eq!(text_ink(&focused, "Hint"), Some(HINT));
    assert_eq!(contour_ink(&focused), Some(FOCUSED));
    assert_eq!(geometry(&runtime, EDITOR), bounds);
    assert_eq!(geometry(&runtime, FLOATING), label);
    assert!(runtime.input_snapshot().unwrap().caret.is_some());
    assert_eq!(runtime.app().views.get(), views);
    assert_eq!(runtime.app().input_count, 0);
    let frame = runtime.accessibility_frame().unwrap();
    let shell = runtime.dom().by_id("shell").unwrap();
    assert_eq!(frame.children(shell).collect::<Vec<_>>(), [editor]);
    assert_eq!(frame.node(editor).unwrap().node.name(), "Editor name");
    assert_eq!(frame.node(editor).unwrap().node.text_value(), Some(""));
    assert!(frame.focus_target(editor).is_some());

    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("after"));
    let blurred = draw(&mut runtime);
    assert_eq!(text_ink(&blurred, "Resting"), Some(IDLE));
    assert_eq!(text_ink(&blurred, "Floating"), None);
    assert_eq!(text_ink(&blurred, "Hint"), None);
    assert_eq!(geometry(&runtime, EDITOR), bounds);
    assert_eq!(runtime.app().views.get(), views);
}

#[test]
fn hover_uses_final_layout_and_ignores_an_out_of_scope_editor() {
    let mut runtime = Runtime::new(Model::default()).unwrap();
    draw(&mut runtime);
    let bounds = geometry(&runtime, EDITOR);
    runtime
        .dispatch(Event::PointerMove {
            x: bounds.x + 18.,
            y: bounds.y + 17.,
        })
        .unwrap();
    let hovered = draw(&mut runtime);
    assert_eq!(text_ink(&hovered, "Resting"), Some(HOVER));
    assert_eq!(contour_ink(&hovered), Some(HOVER));
    assert!(runtime.scroll_to("page", ScrollTarget::Offset { x: 0., y: 70. }));
    draw(&mut runtime);
    let shifted = runtime.app().observation.get().unwrap().0;
    assert_eq!(shifted.bounds.y, bounds.y - 70.);
    assert!(!shifted.is_hovered);
    assert!(shifted.bounds.intersect(shifted.clip).height <= 0.);

    assert!(runtime.scroll_to("page", ScrollTarget::Top));
    draw(&mut runtime);
    assert!(runtime.app().observation.get().unwrap().0.is_hovered);
    runtime.apply(Message::Modal).unwrap();
    let modal = draw(&mut runtime);
    assert_eq!(runtime.focused(), Some("close"));
    let state = runtime.app().observation.get().unwrap().0;
    assert!(!state.is_focused && !state.is_hovered && !state.is_composing);
    assert_eq!(contour_ink(&modal), Some(IDLE));
}

#[test]
fn empty_and_real_preedit_hide_the_hint_without_replacing_the_editor() {
    let mut runtime = Runtime::new(Model::default()).unwrap();
    runtime.activate(EDITOR).unwrap();
    let focused = draw(&mut runtime);
    assert_eq!(text_ink(&focused, "Hint"), Some(HINT));
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    let begun = draw(&mut runtime);
    assert_eq!(text_ink(&begun, "Hint"), None);
    let state = runtime.app().observation.get().unwrap().0;
    assert!(state.is_empty && state.is_composing);
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "中".to_owned(),
            selection: Some((0, 3)),
        }))
        .unwrap();
    draw(&mut runtime);
    runtime.apply(Message::Refresh).unwrap();
    draw(&mut runtime);
    let editor = runtime.editor(EDITOR).unwrap();
    assert!(editor.is_composing());
    assert_eq!(editor.value(), "");
    assert_eq!(editor.composition().unwrap().text, "中");
    assert_eq!(runtime.app().input_count, 0);
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("中".into())))
        .unwrap();
    draw(&mut runtime);
    assert_eq!(runtime.app().value, "中");
    assert_eq!(runtime.app().input_count, 1);
    runtime
        .dispatch(Event::Select {
            anchor: 3,
            cursor: 0,
        })
        .unwrap();
    draw(&mut runtime);
    assert_eq!(runtime.editor(EDITOR).unwrap().selected_text(), "中");
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    let settled = draw(&mut runtime);
    assert_eq!(text_ink(&settled, "Floating"), Some(IDLE));
    assert_eq!(text_ink(&settled, "Hint"), None);
    assert_eq!(runtime.app().input_count, 1);
}

#[test]
fn disabled_value_and_invalid_reference_have_safe_paint_defaults() {
    let mut runtime = Runtime::new(Model {
        value: "保存".into(),
        disabled: true,
        foreign_reference: true,
        ..Model::default()
    })
    .unwrap();
    let list = draw(&mut runtime);
    let (state, reference, _) = runtime.app().observation.get().unwrap();
    assert_eq!(reference, None);
    assert!(!state.is_focused && !state.is_hovered && !state.is_composing && !state.is_empty);
    assert!(runtime.editor(EDITOR).is_none());
    assert_eq!(text_ink(&list, "Floating"), Some(IDLE));
    assert_eq!(text_ink(&list, "Hint"), None);
    let editor = runtime.dom().by_id(EDITOR).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.node(editor).unwrap().node.is_disabled());
    assert!(frame.focus_target(editor).is_none());
}
