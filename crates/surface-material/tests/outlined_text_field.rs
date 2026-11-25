use std::cell::Cell;

use surface::{
    Color, ExtendedDisplayList, ExtendedPaintOp, Node, PaintOp, PathFill, Rect, Role, Style,
    TextAlignment,
};
use surface_app::{Application, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime, View};
use surface_material::{
    shape::LayoutDirection, OutlinedTextFieldOptions, TextFieldEditability, Theme,
};
use surface_wgpu::TextSystem;

const ID: &str = "project-name";
const LABEL: &str = "專案名稱";
const LONG_LABEL: &str = "輸入團隊成員在共用專案清單中容易辨認的完整名稱";
const HINT: &str = "輸入名稱";
const SUPPORT: &str = "請輸入中文或英文名稱。";
const ERROR: &str = "請輸入有效的專案名稱。";
const AFTER: &str = "after-field";

#[derive(Clone)]
enum Message {
    Edit(String),
    Submit,
    ChangeAppearance,
    SetEditability(TextFieldEditability),
}

struct Form {
    label: &'static str,
    value: String,
    dark: bool,
    rtl: bool,
    scale: f32,
    invalid: bool,
    editability: TextFieldEditability,
    edits: Vec<String>,
    submits: Vec<String>,
    builds: Cell<usize>,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            label: LABEL,
            value: String::new(),
            dark: false,
            rtl: false,
            scale: 1.,
            invalid: false,
            editability: TextFieldEditability::Editable,
            edits: Vec::new(),
            submits: Vec::new(),
            builds: Cell::new(0),
        }
    }
}

impl Application for Form {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Edit(value) => {
                self.edits.push(value.clone());
                self.value = value;
            }
            Message::Submit => self.submits.push(self.value.clone()),
            Message::ChangeAppearance => {
                self.dark = !self.dark;
                self.rtl = !self.rtl;
                self.scale = 2.;
                self.invalid = !self.invalid;
            }
            Message::SetEditability(editability) => self.editability = editability,
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn view(&self) -> View<Message> {
        self.builds.set(self.builds.get() + 1);
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let mut options = OutlinedTextFieldOptions::new(ID, self.label, &self.value);
        options.placeholder = Some(HINT);
        options.supporting_text = Some(SUPPORT);
        options.error = self.invalid.then_some(ERROR);
        options.editability = self.editability;
        options.direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        options.text_scale = self.scale;
        View::new(Node::new(Role::Window, "Form").styled(Style::column().padding(12.).gap(8.)))
            .children([
                theme
                    .outlined_text_field_with_submit(options, Message::Edit, || {
                        Some(Message::Submit)
                    })
                    .expect("valid Material field"),
                theme.button(AFTER, "Next", Message::ChangeAppearance),
            ])
    }
}

fn render(runtime: &mut Runtime<Form>, width: f32, text: &mut TextSystem) -> ExtendedDisplayList {
    runtime.render_extended(width, 440., text).unwrap()
}

fn bounds(runtime: &Runtime<Form>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn find_outline(list: &ExtendedDisplayList, color: Color, face: Rect) -> &PathFill {
    list.ops()
        .iter()
        .find_map(|op| match op {
            ExtendedPaintOp::FillPath(path) if path.color() == color && path.rect() == face => {
                Some(path)
            }
            _ => None,
        })
        .expect("inside outline contour at the editor bounds")
}

// The renderer uses even-odd fill. Sample between edges, away from vertices.
fn covers(path: &PathFill, x: f32, y: f32) -> bool {
    let points = path.points();
    path.clip().contains(x, y)
        && points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .filter(|(a, b)| {
                (a[1] > y) != (b[1] > y) && x < a[0] + (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1])
            })
            .count()
            % 2
            == 1
}

fn text_run<'a>(list: &'a ExtendedDisplayList, text: &str) -> Option<(&'a PaintOp, TextAlignment)> {
    list.ops().iter().find_map(|op| {
        let (run, alignment) = match op {
            ExtendedPaintOp::Legacy(run) => (run, TextAlignment::Left),
            ExtendedPaintOp::AlignedText(run) => (run.run(), run.alignment()),
            _ => return None,
        };
        matches!(run, PaintOp::StyledText { text: value, .. } if value == text)
            .then_some((run, alignment))
    })
}

fn tab(runtime: &mut Runtime<Form>) {
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
}

fn enter() -> KeyPress {
    KeyPress::new(Key::Enter, Modifiers::default())
}

#[test]
fn focus_repaints_one_semantic_editor_and_cuts_a_transparent_three_dp_outline() {
    let mut runtime = Runtime::new(Form::default()).unwrap();
    let mut text = TextSystem::default();
    let idle = render(&mut runtime, 400., &mut text);
    let face = bounds(&runtime, ID);
    assert_eq!(face.height, 56.);
    let label = bounds(&runtime, "project-name-floating-label");
    let notch_x = label.x + label.width / 2.;
    let colors = Theme::light().colors;
    let outline = find_outline(&idle, colors.outline, face);
    assert!(covers(outline, notch_x, face.y + 0.5));
    assert!(!covers(outline, notch_x, face.y + 1.5));
    assert_eq!(
        text_run(&idle, LABEL).unwrap().0.rect(),
        bounds(&runtime, "project-name-inline-label")
    );
    assert!(text_run(&idle, HINT).is_none());

    let builds = runtime.app().builds.get();
    tab(&mut runtime);
    let focus = render(&mut runtime, 400., &mut text);
    assert_eq!(runtime.focused(), Some(ID));
    assert_eq!(runtime.app().builds.get(), builds);
    assert_eq!(bounds(&runtime, ID), face);
    assert_eq!(text_run(&focus, LABEL).unwrap().0.rect(), label);
    assert!(text_run(&focus, HINT).is_some());
    let outline = find_outline(&focus, colors.primary, face);
    assert!(!covers(outline, notch_x, face.y + 0.5));
    assert!(covers(outline, label.x - 5., face.y + 2.5));
    assert!(covers(outline, label.x + label.width + 5., face.y + 2.5));
    assert!(!covers(outline, label.x - 5., face.y + 3.5));
    assert!(!focus.ops().iter().any(|op| matches!(
        op,
        ExtendedPaintOp::Legacy(PaintOp::Quad { rect, color, .. } | PaintOp::Shape { rect, color, .. })
            if color.3 > 0 && rect.contains(notch_x, face.y + 0.5)
    )));

    let frame = runtime.accessibility_frame().unwrap();
    let editor = runtime.dom().by_id(ID).unwrap();
    let shell = runtime.dom().by_id("project-name-field").unwrap();
    let description = runtime.dom().by_id("project-name-description").unwrap();
    let status = runtime
        .dom()
        .by_id("project-name-validation-status")
        .unwrap();
    assert_eq!(
        frame.children(shell).collect::<Vec<_>>(),
        [editor, description, status]
    );
    let live = frame.node(status).unwrap().node;
    assert_eq!(live.role(), Role::Status);
    assert!(live.is_assertive_live_status());
    assert_eq!(live.text_value(), Some(""));
    assert!(frame.focus_target(status).is_none());
    let projected = frame.node(editor).unwrap().node;
    assert_eq!(projected.name(), LABEL);
    assert_eq!(projected.role(), Role::TextInput);
    assert!(!projected.is_invalid());
    assert_eq!(frame.description_target(editor), Some(description));
    assert_eq!(
        frame.node(description).unwrap().node.text_value(),
        Some(SUPPORT)
    );
    assert_eq!(
        bounds(&runtime, "project-name-description").y,
        face.y + face.height + 4.
    );

    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄓ".into(),
            selection: None,
        }))
        .unwrap();
    let composing = render(&mut runtime, 400., &mut text);
    assert_eq!(runtime.input_snapshot().unwrap().value, "");
    assert!(text_run(&composing, HINT).is_none());
    runtime.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();
    assert!(text_run(&render(&mut runtime, 400., &mut text), HINT).is_some());
    assert_eq!(runtime.app().builds.get(), builds);
}

#[test]
fn same_id_theme_scale_and_validation_rebuild_preserve_backward_selection_and_preedit() {
    let mut runtime = Runtime::new(Form {
        value: "後界".into(),
        ..Form::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    render(&mut runtime, 500., &mut text);
    tab(&mut runtime);
    runtime
        .dispatch(Event::Select {
            anchor: 6,
            cursor: 3,
        })
        .unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄨㄣ".into(),
            selection: None,
        }))
        .unwrap();
    runtime.apply(Message::ChangeAppearance).unwrap();
    let paint = render(&mut runtime, 320., &mut text);

    let input = runtime.input_snapshot().unwrap();
    assert_eq!((input.id.as_str(), input.value.as_str()), (ID, "後界"));
    assert_eq!((input.selection.anchor, input.selection.cursor), (6, 3));
    assert!(input.composing);
    assert_eq!(
        runtime.editor(ID).unwrap().composition().unwrap().text,
        "ㄨㄣ"
    );
    assert!(text_run(&paint, HINT).is_none());
    let frame = runtime.accessibility_frame().unwrap();
    let editor = runtime.dom().by_id(ID).unwrap();
    assert!(frame.node(editor).unwrap().node.is_invalid());
    let description = frame.description_target(editor).unwrap();
    assert_eq!(
        frame.node(description).unwrap().node.text_value(),
        Some(ERROR)
    );
    let status = runtime
        .dom()
        .by_id("project-name-validation-status")
        .unwrap();
    let live = frame.node(status).unwrap().node;
    assert!(live.is_assertive_live_status());
    assert_eq!(live.text_value(), Some(ERROR));
    find_outline(&paint, Theme::dark().colors.error, bounds(&runtime, ID));

    assert!(!runtime.key(&enter()).unwrap());
    assert!(runtime.app().submits.is_empty());
    assert!(runtime.app().edits.is_empty());
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("文".into())))
        .unwrap();
    assert_eq!(runtime.app().edits, ["後文"]);
    assert!(!runtime.key(&enter().with_repeat(true)).unwrap());
    let shifted = KeyPress::new(
        Key::Enter,
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
    );
    assert!(!runtime.key(&shifted).unwrap());
    assert!(runtime.key(&enter()).unwrap());
    assert_eq!(runtime.app().submits, ["後文"]);
    assert_eq!(runtime.app().edits, ["後文"]);
}

#[test]
fn compact_rtl_long_label_wraps_above_a_closed_outline() {
    let mut runtime = Runtime::new(Form {
        label: LONG_LABEL,
        value: "共用名稱".into(),
        scale: 2.,
        rtl: true,
        dark: true,
        ..Form::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    render(&mut runtime, 320., &mut text);
    tab(&mut runtime);
    let paint = render(&mut runtime, 320., &mut text);
    let face = bounds(&runtime, ID);
    let above = bounds(&runtime, "project-name-above-label");
    assert_eq!(face.width, 296.);
    assert!(face.height >= 56.);
    assert_eq!(face.y - above.y - above.height, 4.);
    assert!(above.height > 32.);
    assert_eq!(above.x, face.x + 16.);
    assert_eq!(above.x + above.width, face.x + face.width - 16.);
    let (caption, alignment) = text_run(&paint, LONG_LABEL).unwrap();
    assert_eq!(caption.rect(), above);
    assert_eq!(alignment, TextAlignment::Right);
    let outline = find_outline(&paint, Theme::dark().colors.primary, face);
    for fraction in [0.1, 0.5, 0.9] {
        assert!(covers(
            outline,
            face.x + face.width * fraction,
            face.y + 1.5
        ));
    }
    let caret = runtime.input_snapshot().unwrap().caret.unwrap();
    assert!(caret.x > face.x + face.width / 2.);
    assert!(caret.x < face.x + face.width - 16.);
}

#[test]
fn read_only_selects_without_edits_and_disabled_leaves_the_tab_order() {
    let mut runtime = Runtime::new(Form {
        value: "既有專案".into(),
        invalid: true,
        editability: TextFieldEditability::ReadOnly,
        ..Form::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    render(&mut runtime, 320., &mut text);
    tab(&mut runtime);
    render(&mut runtime, 320., &mut text);
    let editor = runtime.dom().by_id(ID).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.node(editor).unwrap().text_options.unwrap().read_only);
    assert!(frame.node(editor).unwrap().node.is_invalid());
    assert!(frame.accepts_focus(editor));
    runtime
        .dispatch(Event::Select {
            anchor: 12,
            cursor: 0,
        })
        .unwrap();
    assert_eq!(runtime.selected_text().as_deref(), Some("既有專案"));
    assert!(!runtime.key(&enter()).unwrap());
    runtime.dispatch(Event::Text("更改".into())).unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("更".into())))
        .unwrap();
    assert_eq!(runtime.selected_text().as_deref(), Some("既有專案"));
    assert!(runtime.app().edits.is_empty());
    assert!(runtime.app().submits.is_empty());

    runtime
        .apply(Message::SetEditability(TextFieldEditability::Disabled))
        .unwrap();
    render(&mut runtime, 320., &mut text);
    let editor = runtime.dom().by_id(ID).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    let node = frame.node(editor).unwrap().node;
    assert!(node.is_disabled());
    assert!(!node.is_invalid());
    assert!(frame.description_target(editor).is_none());
    assert!(!frame.accepts_focus(editor));
    assert!(runtime.editor(ID).is_none());
    tab(&mut runtime);
    assert_eq!(runtime.focused(), Some(AFTER));
}
