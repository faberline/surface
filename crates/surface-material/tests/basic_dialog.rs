use surface::{Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityScrollDirection, Application, Event, ImeEvent, Key, KeyPress, ModalDismissal,
    Modifiers, Runtime, Shortcut, View,
};
use surface_material::{
    shape::{self, LayoutDirection},
    BasicDialogContent, BasicDialogOptions, ButtonKind, Elevation, Theme,
};

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let natural = text.chars().count() as f32 * size * 0.6;
        let width = width.unwrap_or(natural).max(1.);
        let lines = (natural / width).ceil().max(1.);
        (natural.min(width), size * 1.5 * lines)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Message {
    Open,
    Background,
    Nested(bool),
    Dismiss,
    Confirm,
    Cancel,
    Text,
}

struct App {
    open: bool,
    nested: bool,
    viewport: (f32, f32),
    rtl: bool,
    big: bool,
    long_labels: bool,
    long_body: bool,
    editor: bool,
    policy: ModalDismissal,
    actions: Vec<Message>,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        self.actions.push(message);
        match message {
            Message::Open => self.open = true,
            Message::Nested(open) => self.nested = open,
            Message::Dismiss | Message::Confirm | Message::Cancel => self.open = false,
            Message::Background | Message::Text => {}
        }
    }

    fn shortcuts(&self) -> Vec<Shortcut<Message>> {
        vec![Shortcut::new(
            Key::Character("k".into()),
            Modifiers {
                meta: true,
                ..Modifiers::default()
            },
            Message::Background,
        )]
    }

    fn text_scale(&self) -> f32 {
        if self.big {
            2.
        } else {
            1.
        }
    }

    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let changed = self.viewport != (width, height);
        self.viewport = (width, height);
        changed
    }

    fn view(&self) -> View<Message> {
        let theme = if self.rtl {
            Theme::dark()
        } else {
            Theme::light()
        };
        let mut children = vec![
            theme.button_with("opener", "Open", ButtonKind::Filled, false, Message::Open),
            theme.button_with(
                "background",
                "Behind dialog",
                ButtonKind::Text,
                false,
                Message::Background,
            ),
        ];
        if self.open {
            let mut extra = Vec::new();
            if self.editor {
                extra.push(theme.text_field("d-editor", "Name", "", |_| Message::Text));
            }
            if self.long_body {
                extra.extend((0..12).map(|i| {
                    View::new(
                        Node::new(Role::Text, "A body line")
                            .id(format!("line-{i}"))
                            .text("這是一段捲動內容")
                            .styled(Style::column().height(Length::Px(72.))),
                    )
                }));
            }
            if self.nested {
                extra.push(theme.dialog(
                    "legacy-child",
                    "Second prompt",
                    [],
                    [theme.button_with(
                        "child-close",
                        "Close",
                        ButtonKind::Text,
                        false,
                        Message::Nested(false),
                    )],
                    Message::Nested(false),
                ));
            }
            let (confirm, cancel) = if self.long_labels {
                ("保留目前所選的全部設定", "取消並返回先前內容")
            } else {
                ("Save", "Cancel")
            };
            children.extend(theme.basic_dialog(
                BasicDialogOptions {
                    id: "d",
                    label: "Rename document",
                    heading: Some("重新命名"),
                    supporting: Some("選擇儲存或返回原畫面。"),
                    icon: Some("✎"),
                    window_size: self.viewport,
                    preferred_width: 360.,
                    direction: if self.rtl {
                        LayoutDirection::RightToLeft
                    } else {
                        LayoutDirection::LeftToRight
                    },
                    dismissal: self.policy,
                    alert: self.policy == ModalDismissal::ActionOnly,
                    initial_focus: Some(if self.editor { "d-editor" } else { "d-cancel" }),
                    restore_focus_to: Some("background"),
                },
                BasicDialogContent {
                    extra,
                    confirm: (confirm, Message::Confirm),
                    cancel: Some((cancel, Message::Cancel)),
                    on_dismiss: Message::Dismiss,
                },
            ));
        }
        theme.page(children)
    }
}

fn runtime(policy: ModalDismissal, rtl: bool, big: bool, long: bool, editor: bool) -> Runtime<App> {
    Runtime::new(App {
        open: false,
        nested: false,
        viewport: (640., 480.),
        rtl,
        big,
        long_labels: long,
        long_body: long,
        editor,
        policy,
        actions: vec![],
    })
    .unwrap()
}

fn open(runtime: &mut Runtime<App>, size: (f32, f32)) {
    render(runtime, size);
    runtime.activate("opener").unwrap();
    render(runtime, size);
}

fn render(runtime: &mut Runtime<App>, (width, height): (f32, f32)) {
    runtime.render(width, height, &mut Metrics).unwrap();
}

fn rect(runtime: &Runtime<App>, id: &str) -> Rect {
    let node = runtime.dom().by_id(id).unwrap();
    runtime.computed().unwrap().nodes[node.index()].bounds
}

fn click(runtime: &mut Runtime<App>, x: f32, y: f32) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
}

#[test]
fn basic_dialog_uses_tokens_and_logical_inline_actions() {
    for rtl in [false, true] {
        let mut runtime = runtime(ModalDismissal::default(), rtl, false, false, false);
        open(&mut runtime, (960., 800.));
        let panel = rect(&runtime, "d");
        let heading = rect(&runtime, "d-heading");
        let body = rect(&runtime, "d-body");
        let actions = rect(&runtime, "d-actions");
        let confirm = rect(&runtime, "d-confirm");
        let cancel = rect(&runtime, "d-cancel");
        let node = runtime
            .dom()
            .get(runtime.dom().by_id("d").unwrap())
            .unwrap();
        let theme = if rtl { Theme::dark() } else { Theme::light() };
        assert_eq!(node.name(), "Rename document");
        assert_eq!(node.style().corners, Some(shape::EXTRA_LARGE));
        assert_eq!(node.style().background, theme.colors.surface_container_high);
        assert_eq!(node.box_shadow_layers(), theme.elevation(Elevation::Level3));
        assert_eq!((panel.width, panel.x), (360., 300.));
        assert_eq!(body.x - panel.x, 24.);
        assert_eq!(body.y - heading.y - heading.height, 16.);
        assert_eq!(actions.y - body.y - body.height, 24.);
        assert_eq!(confirm.height, 48.);
        assert_eq!(cancel.height, 48.);
        if rtl {
            assert_eq!(confirm.x, actions.x);
            assert!((cancel.x - confirm.x - confirm.width - 8.).abs() < 0.01);
        } else {
            assert_eq!(confirm.x + confirm.width, actions.x + actions.width);
            assert!((confirm.x - cancel.x - cancel.width - 8.).abs() < 0.01);
        }
        assert_eq!(runtime.focused(), Some("d-cancel"));
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some("d-confirm"));
        assert!(runtime.activate("background").is_err());
        assert!(!runtime.app().actions.contains(&Message::Background));
    }
}

#[test]
fn narrow_large_text_scrolls_only_the_middle_and_stacks_positive_action_first() {
    let mut runtime = runtime(ModalDismissal::ActionOnly, true, true, true, false);
    let size = (320., 400.);
    open(&mut runtime, size);
    let panel = rect(&runtime, "d");
    let heading = rect(&runtime, "d-heading");
    let body = rect(&runtime, "d-body");
    let confirm = rect(&runtime, "d-confirm");
    let cancel = rect(&runtime, "d-cancel");
    assert_eq!((panel.x, panel.width), (20., 280.));
    assert!(body.height > 0.);
    assert!(panel.contains(cancel.x + cancel.width / 2., cancel.y + cancel.height - 1.));
    assert!(confirm.y < cancel.y);
    assert_eq!((confirm.x, cancel.x), (44., 44.));
    let first = rect(&runtime, "line-0");
    let frame = runtime.accessibility_frame().unwrap();
    let id = runtime.dom().by_id("d-body").unwrap();
    assert!(
        frame.scroll_page_target(id, AccessibilityScrollDirection::Down).is_some(),
        "panel={panel:?} heading={heading:?} body={body:?} confirm={confirm:?} cancel={cancel:?} first={first:?}, body_metrics={:?} panel_metrics={:?}",
        runtime.scroll_metrics("d-body"), runtime.scroll_metrics("d")
    );
    runtime
        .dispatch(Event::Scroll {
            x: body.x + 20.,
            y: body.y + 20.,
            delta_x: 0.,
            delta_y: 160.,
        })
        .unwrap();
    render(&mut runtime, size);
    assert!(runtime.scroll_metrics("d-body").unwrap().offset.1 > 0.);
    assert!(rect(&runtime, "line-0").y < first.y);
    assert_eq!(rect(&runtime, "d-heading"), heading);
    assert_eq!(rect(&runtime, "d-confirm"), confirm);
    assert_eq!(rect(&runtime, "d-cancel"), cancel);
}

#[test]
fn explicit_only_and_escape_only_consume_outside_input_and_restore_focus() {
    for policy in [
        ModalDismissal::ActionOnly,
        ModalDismissal::Escape,
        ModalDismissal::EscapeOrOutside,
    ] {
        let mut runtime = runtime(policy, false, false, false, false);
        open(&mut runtime, (960., 800.));
        click(&mut runtime, 2., 2.);
        assert!(!runtime.app().actions.contains(&Message::Background));
        if policy == ModalDismissal::EscapeOrOutside {
            assert!(!runtime.app().open);
            assert_eq!(runtime.focused(), Some("opener"));
            continue;
        }
        assert!(runtime.app().open);
        assert!(runtime
            .dispatch(Event::SecondaryDown { x: 2., y: 2. })
            .unwrap());
        assert!(runtime.app().open);
        assert!(runtime.dispatch(Event::Cancel).unwrap());
        if policy == ModalDismissal::ActionOnly {
            assert!(runtime.app().open);
            runtime.activate("d-confirm").unwrap();
            assert_eq!(runtime.app().actions, [Message::Open, Message::Confirm]);
        } else {
            assert_eq!(runtime.app().actions, [Message::Open, Message::Dismiss]);
        }
        assert!(!runtime.app().open);
        assert_eq!(runtime.focused(), Some("opener"));
    }
}

#[test]
fn composition_blocks_the_first_escape_and_confirming_pointer() {
    let mut runtime = runtime(ModalDismissal::Escape, false, false, false, true);
    let size = (640., 480.);
    open(&mut runtime, size);
    assert_eq!(runtime.focused(), Some("d-editor"));
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄕ".into(),
            selection: Some((3, 3)),
        }))
        .unwrap();
    render(&mut runtime, size);
    let confirm = rect(&runtime, "d-confirm");
    click(&mut runtime, confirm.x + 24., confirm.y + 24.);
    assert_eq!(runtime.app().actions, [Message::Open]);
    assert!(runtime.input_snapshot().unwrap().composing);
    runtime.dispatch(Event::Cancel).unwrap();
    assert!(!runtime.input_snapshot().unwrap().composing);
    assert!(runtime.app().open);
    runtime.dispatch(Event::Cancel).unwrap();
    assert_eq!(runtime.app().actions, [Message::Open, Message::Dismiss]);
    assert_eq!(runtime.focused(), Some("opener"));
}

#[test]
fn basic_dialog_consumes_the_background_shortcut_until_it_closes() {
    let mut runtime = runtime(ModalDismissal::ActionOnly, true, false, false, false);
    open(&mut runtime, (640., 480.));
    let chord = KeyPress::new(
        Key::Character("k".into()),
        Modifiers {
            meta: true,
            ..Modifiers::default()
        },
    );
    assert!(runtime.shortcut(&chord).unwrap());
    assert_eq!(runtime.app().actions, [Message::Open]);
    runtime.activate("d-cancel").unwrap();
    assert!(runtime.shortcut(&chord).unwrap());
    assert_eq!(
        runtime.app().actions,
        [Message::Open, Message::Cancel, Message::Background]
    );
}

#[test]
fn nested_legacy_modal_inherits_the_new_parent_shortcut_block() {
    let mut runtime = runtime(ModalDismissal::ActionOnly, false, false, false, false);
    open(&mut runtime, (800., 600.));
    runtime.apply(Message::Nested(true)).unwrap();
    render(&mut runtime, (800., 600.));
    let chord = KeyPress::new(
        Key::Character("k".into()),
        Modifiers {
            meta: true,
            ..Modifiers::default()
        },
    );
    assert_eq!(runtime.focused(), Some("child-close"));
    assert!(runtime.shortcut(&chord).unwrap());
    assert_eq!(
        runtime.app().actions,
        [Message::Open, Message::Nested(true)]
    );
    runtime.activate("child-close").unwrap();
    assert_eq!(runtime.focused(), Some("d-cancel"));
    assert!(runtime.shortcut(&chord).unwrap());
    runtime.activate("d-cancel").unwrap();
    assert!(runtime.shortcut(&chord).unwrap());
    assert_eq!(
        runtime.app().actions,
        [
            Message::Open,
            Message::Nested(true),
            Message::Nested(false),
            Message::Cancel,
            Message::Background
        ]
    );
}
