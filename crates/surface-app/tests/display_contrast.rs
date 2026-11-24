use std::cell::Cell;
use surface::{
    Color, DomError, ExtendedDisplayList, ExtendedPaintOp, Length, Node, PaintOp, Role, Style,
    TextMeasurer, TextSelectionColors,
};
use surface_app::{
    AppError, Application, DisplayContrast, Event, ImeEvent, PointDrag, PointDragSession, Runtime,
    SystemColorPair, SystemContrastColors, View,
};

const NORMAL_INK: Color = Color::rgb(30, 31, 32);
const MORE_INK: Color = Color::rgb(0, 0, 0);

fn forced(alpha: u8) -> DisplayContrast {
    DisplayContrast::Forced(SystemContrastColors::new(
        SystemColorPair {
            background: Color::rgb(0, 0, 0),
            foreground: Color::rgb(255, 255, 255),
        },
        SystemColorPair {
            background: Color::rgb(35, 35, 35),
            foreground: Color::rgb(255, 255, 255),
        },
        SystemColorPair {
            background: Color(0, 230, 255, alpha),
            foreground: Color::rgb(0, 0, 0),
        },
    ))
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size / 2., size)
    }
}

#[derive(Clone)]
enum Message {
    Draft(String),
    Dismiss,
}

#[derive(Default)]
struct App {
    contrast: DisplayContrast,
    calls: Vec<DisplayContrast>,
    views: Cell<usize>,
    draft: String,
    closed: bool,
    fail_next_increased_view: Cell<bool>,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Draft(text) => self.draft = text,
            Message::Dismiss => self.closed = true,
        }
    }

    fn display_contrast_changed(&mut self, contrast: DisplayContrast) -> bool {
        self.calls.push(contrast);
        self.contrast = contrast;
        true
    }

    fn view(&self) -> View<Message> {
        self.views.set(self.views.get() + 1);
        let ink = match self.contrast {
            DisplayContrast::Forced(colors) => colors.canvas.foreground,
            DisplayContrast::Increased => MORE_INK,
            _ => NORMAL_INK,
        };
        let root = View::new(Node::new(Role::Window, "App").styled(Style::column()));
        if self.contrast == DisplayContrast::Increased
            && self.fail_next_increased_view.replace(false)
        {
            let duplicate = || View::new(Node::new(Role::Group, "").id("duplicate"));
            return root.children([duplicate(), duplicate()]);
        }
        if self.closed {
            return root;
        }
        let field = View::new(
            Node::new(Role::TextInput, "Quantity")
                .id("quantity")
                .text(&self.draft)
                .invalid(self.draft == "1203")
                .described_by_text("validation")
                .styled(Style::column().height(Length::Px(48.)).foreground(ink)),
        )
        .on_input(Message::Draft);
        let help = View::new(
            Node::new(Role::Text, "")
                .id("validation")
                .text("Enter an even quantity")
                .styled(Style::column().height(Length::Px(24.))),
        );
        let modal = View::new(Node::new(Role::Dialog, "Edit quantity").id("dialog"))
            .children([field, help])
            .modal(Message::Dismiss)
            .modal_initial_focus("quantity");
        root.children([modal])
    }
}

fn draft_with_preedit(runtime: &mut Runtime<App>) {
    runtime.render(320., 240., &mut Metrics).unwrap();
    assert_eq!(runtime.focused(), Some("quantity"));
    runtime
        .dispatch(Event::InputValue {
            value: "1203".into(),
            anchor: 2,
            cursor: 4,
        })
        .unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄋㄧ".into(),
            selection: Some((6, 6)),
        }))
        .unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    assert!(runtime.input_snapshot().unwrap().composing);
}

#[test]
fn color_changes_deduplicate_exact_pairs_and_keep_the_open_modal_draft() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    assert_eq!(runtime.display_contrast(), DisplayContrast::Normal);
    draft_with_preedit(&mut runtime);
    let draft = runtime.snapshot().text_inputs["quantity"].clone();
    let views = runtime.app().views.get();
    let first = forced(204);
    let second = forced(192);

    for (preference, ink) in [
        (first, Color::rgb(255, 255, 255)),
        (second, Color::rgb(255, 255, 255)),
        (DisplayContrast::Increased, MORE_INK),
        (DisplayContrast::Normal, NORMAL_INK),
    ] {
        assert!(runtime.set_display_contrast(preference).unwrap());
        assert_eq!(runtime.display_contrast(), preference);
        assert!(!runtime.set_display_contrast(preference).unwrap());
        runtime.render(320., 240., &mut Metrics).unwrap();
        assert_eq!(runtime.snapshot().text_inputs["quantity"], draft);
        assert_eq!(runtime.app().draft, "1203");
        assert_eq!(runtime.focused(), Some("quantity"));
        let frame = runtime.accessibility_frame().unwrap();
        let field = runtime.dom().by_id("quantity").unwrap();
        assert_eq!(frame.modal_root(), runtime.dom().by_id("dialog"));
        assert_eq!(
            frame.description_target(field),
            runtime.dom().by_id("validation")
        );
        let node = frame.node(field).unwrap().node;
        assert!(node.is_invalid());
        assert_eq!(node.style().foreground, ink);
    }
    assert_eq!(runtime.app().views.get(), views + 4);
    assert_eq!(
        runtime.app().calls,
        [
            first,
            second,
            DisplayContrast::Increased,
            DisplayContrast::Normal
        ]
    );
}

#[test]
fn failed_preference_view_validation_rolls_back_and_accepts_a_retry() {
    let mut runtime = Runtime::new(App {
        fail_next_increased_view: Cell::new(true),
        ..App::default()
    })
    .unwrap();
    draft_with_preedit(&mut runtime);
    let before = runtime.snapshot();
    let generation = runtime.accessibility_frame().unwrap().generation();
    let failed = runtime.set_display_contrast(DisplayContrast::Increased);
    assert!(matches!(failed, Err(AppError::Dom(DomError::DuplicateId(id))) if id == "duplicate"));
    assert_eq!(runtime.display_contrast(), DisplayContrast::Normal);
    assert_eq!(runtime.app().contrast, DisplayContrast::Normal);
    assert_eq!(runtime.snapshot(), before);
    assert_eq!(
        runtime.accessibility_frame().unwrap().generation(),
        generation
    );

    assert!(runtime
        .set_display_contrast(DisplayContrast::Increased)
        .unwrap());
    assert_eq!(runtime.display_contrast(), DisplayContrast::Increased);
    assert_eq!(runtime.snapshot().text_inputs, before.text_inputs);
    assert_eq!(runtime.focused(), Some("quantity"));
    assert_eq!(
        runtime.app().calls,
        [
            DisplayContrast::Increased,
            DisplayContrast::Normal,
            DisplayContrast::Increased
        ]
    );
}

#[test]
fn installed_contrast_survives_a_failed_drag_cancel_rebuild() {
    #[derive(Clone)]
    enum DragMessage {
        Drag(PointDrag),
        Dismiss,
    }

    #[derive(Default)]
    struct DragApp {
        contrast: DisplayContrast,
        session: PointDragSession,
        moved: bool,
        cancellations: usize,
    }

    impl Application for DragApp {
        type Message = DragMessage;

        fn update(&mut self, message: DragMessage) {
            match message {
                DragMessage::Drag(PointDrag::Move { .. }) => self.moved = true,
                DragMessage::Drag(PointDrag::Cancel) => self.cancellations += 1,
                _ => {}
            }
        }

        fn display_contrast_changed(&mut self, contrast: DisplayContrast) -> bool {
            self.contrast = contrast;
            true
        }

        fn view(&self) -> View<DragMessage> {
            let mut root = Node::new(Role::Window, "App");
            if let DisplayContrast::Forced(colors) = self.contrast {
                root = root.styled(Style::column().background(colors.canvas.background));
            }
            let root = View::new(root);
            if self.cancellations > 0 {
                let duplicate = || View::new(Node::new(Role::Group, "").id("cancel-duplicate"));
                return root.children([duplicate(), duplicate()]);
            }
            let grip = View::new(
                Node::new(Role::Group, "Grip")
                    .id("grip")
                    .styled(Style::row().width(Length::Px(96.)).height(Length::Px(80.))),
            )
            .on_point_drag_with_bounds(self.session.clone(), |point, _| {
                Some(DragMessage::Drag(point))
            });
            // The new scope leaves the grip and its drag session mounted.
            let modal = matches!(self.contrast, DisplayContrast::Forced(_)).then(|| {
                View::new(Node::new(Role::Dialog, "Dialog").id("dialog"))
                    .modal_overlay(DragMessage::Dismiss)
            });
            root.children([grip].into_iter().chain(modal))
        }
    }

    let mut runtime = Runtime::new(DragApp::default()).unwrap();
    let normal = runtime.render(320., 240., &mut Metrics).unwrap();
    runtime
        .dispatch(Event::PointerDown { x: 24., y: 24. })
        .unwrap();
    runtime
        .dispatch(Event::PointerMove { x: 30., y: 30. })
        .unwrap();
    assert!(runtime.app().moved && runtime.has_point_drag());

    let previous = runtime.display_contrast();
    let appearance = forced(204);
    let result = runtime.set_display_contrast(appearance);
    assert!(
        matches!(result, Err(AppError::Dom(DomError::DuplicateId(id))) if id == "cancel-duplicate")
    );
    assert_ne!(runtime.display_contrast(), previous);
    assert_eq!(runtime.display_contrast(), appearance);
    assert_eq!(runtime.app().cancellations, 1);
    assert!(!runtime.has_point_drag());
    assert!(runtime.dom().by_id("dialog").is_some());
    assert!(runtime.dom().by_id("cancel-duplicate").is_none());
    assert!(!runtime.set_display_contrast(appearance).unwrap());
    let installed = runtime.render(320., 240., &mut Metrics).unwrap();
    assert!(!installed.damage(&normal).is_empty());
    assert!(installed.ops.iter().any(|paint| {
        matches!(paint, PaintOp::Quad { color, .. } if *color == Color::rgb(0, 0, 0))
    }));
}

#[test]
fn the_default_hook_records_the_preference_without_rebuilding() {
    struct Legacy(Cell<usize>);
    impl Application for Legacy {
        type Message = ();

        fn update(&mut self, (): ()) {}

        fn view(&self) -> View<()> {
            self.0.set(self.0.get() + 1);
            View::new(Node::new(Role::Window, "Legacy"))
        }
    }

    let mut runtime = Runtime::new(Legacy(Cell::new(0))).unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    let before = runtime.snapshot();
    let contrast = forced(204);
    assert!(runtime.set_display_contrast(contrast).unwrap());
    assert_eq!(runtime.display_contrast(), contrast);
    assert!(!runtime.set_display_contrast(contrast).unwrap());
    assert!(runtime
        .set_display_contrast(DisplayContrast::Increased)
        .unwrap());
    assert!(!runtime
        .set_display_contrast(DisplayContrast::Normal)
        .unwrap());
    assert_eq!(runtime.app().0.get(), 1);
    assert_eq!(runtime.snapshot(), before);
}

#[test]
fn forced_selection_repaints_without_rebuilding_or_resetting_preedit() {
    struct Editor(Cell<usize>);

    impl Application for Editor {
        type Message = ();

        fn update(&mut self, (): ()) {}

        fn view(&self) -> View<()> {
            self.0.set(self.0.get() + 1);
            let field = View::new(
                Node::new(Role::TextInput, "Quantity")
                    .id("quantity")
                    .text("1203")
                    .styled(
                        Style::column()
                            .width(Length::Px(140.))
                            .height(Length::Px(48.))
                            .foreground(NORMAL_INK),
                    ),
            )
            .on_input(|_| ());
            let dialog = View::new(Node::new(Role::Dialog, "Edit quantity").id("dialog"))
                .children([field])
                .modal(())
                .modal_initial_focus("quantity");
            View::new(Node::new(Role::Window, "App")).children([dialog])
        }
    }

    fn painted_pair(frame: &ExtendedDisplayList) -> Option<TextSelectionColors> {
        frame.ops().iter().find_map(|op| match op {
            ExtendedPaintOp::TextSelection(paint) => Some(paint.colors()),
            _ => None,
        })
    }

    let mut runtime = Runtime::new(Editor(Cell::new(0))).unwrap();
    runtime.render_extended(320., 240., &mut Metrics).unwrap();
    runtime
        .dispatch(Event::Select {
            anchor: 2,
            cursor: 4,
        })
        .unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄋㄧ".into(),
            selection: Some((0, 3)),
        }))
        .unwrap();
    let normal = runtime.render_extended(320., 240., &mut Metrics).unwrap();
    assert_eq!(painted_pair(&normal), None);
    assert!(normal.ops().iter().any(|op| matches!(
        op,
        ExtendedPaintOp::Legacy(PaintOp::Quad {
            color: Color(103, 80, 164, 80),
            ..
        })
    )));
    #[cfg(feature = "devtools")]
    let attribution = runtime.paint_snapshot().unwrap();
    let legacy = runtime.render(320., 240., &mut Metrics).unwrap();
    let before = runtime.snapshot();
    assert!(before.text_inputs["quantity"].composing);

    for contrast in [forced(204), forced(192)] {
        assert!(runtime.set_display_contrast(contrast).unwrap());
        assert!(!runtime.set_display_contrast(contrast).unwrap());
        let DisplayContrast::Forced(colors) = contrast else {
            unreachable!()
        };
        let frame = runtime.render_extended(320., 240., &mut Metrics).unwrap();
        assert_eq!(
            painted_pair(&frame),
            Some(TextSelectionColors {
                background: colors.highlight.background,
                foreground: colors.highlight.foreground,
            })
        );
        assert_eq!(runtime.snapshot(), before);
        #[cfg(feature = "devtools")]
        assert_eq!(runtime.paint_snapshot().unwrap(), attribution);
        let unchanged = runtime.render(320., 240., &mut Metrics).unwrap();
        assert!(unchanged.damage(&legacy).is_empty());
    }

    let DisplayContrast::Forced(colors) = runtime.display_contrast() else {
        unreachable!()
    };
    let canvas_only = DisplayContrast::Forced(SystemContrastColors::new(
        SystemColorPair {
            background: Color::rgb(16, 19, 22),
            ..colors.canvas
        },
        colors.button,
        colors.highlight,
    ));
    assert!(!runtime.set_display_contrast(canvas_only).unwrap());
    assert_eq!(runtime.display_contrast(), canvas_only);
    assert!(runtime
        .set_display_contrast(DisplayContrast::Increased)
        .unwrap());
    assert_eq!(
        runtime.render_extended(320., 240., &mut Metrics).unwrap(),
        normal
    );
    assert!(!runtime
        .set_display_contrast(DisplayContrast::Normal)
        .unwrap());
    assert_eq!(runtime.snapshot(), before);
    assert_eq!(runtime.focused(), Some("quantity"));
    assert_eq!(runtime.app().0.get(), 1);
}
