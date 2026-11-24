use surface::{
    Color, DisplayList, ExtendedPaintOp, Length, Node, PaintOp, Rect, Role, Style, TextMeasurer,
};
use surface_app::{
    utf16_to_utf8, utf8_to_utf16, Application, EditKey, Event, ImeEvent, Runtime, TextEditor,
    TextOptions, View,
};

#[test]
fn cleared_native_preedit_preserves_a_commit_range_but_does_not_lock_normal_keys() {
    let options = TextOptions::default();
    let mut editor = TextEditor::new("台灣");
    editor.select(0, "台".len());
    editor.ime(
        ImeEvent::Preedit {
            text: "ㄓㄨㄥ".into(),
            selection: None,
        },
        &options,
    );
    editor.ime(
        ImeEvent::Preedit {
            text: String::new(),
            selection: None,
        },
        &options,
    );
    assert!(!editor.is_composing());
    assert!(editor.ime(ImeEvent::Commit("中".into()), &options));
    assert_eq!(editor.value(), "中灣");
    editor.select(editor.value().len(), editor.value().len());
    editor.ime(
        ImeEvent::Preedit {
            text: "ㄅ".into(),
            selection: None,
        },
        &options,
    );
    editor.ime(
        ImeEvent::Preedit {
            text: String::new(),
            selection: None,
        },
        &options,
    );
    assert!(editor.insert("文", &options));
    assert_eq!(editor.value(), "中灣文");
    assert!(editor.key(EditKey::Backspace, false, false, &options));
    assert_eq!(editor.value(), "中灣");
}

#[derive(Default)]
struct App {
    text: String,
    changes: usize,
}
#[derive(Clone)]
enum Message {
    Text(String),
}
impl Application for App {
    type Message = Message;
    fn update(&mut self, message: Message) {
        match message {
            Message::Text(text) => {
                self.text = text;
                self.changes += 1;
            }
        }
    }
    fn view(&self) -> View<Message> {
        View::new(
            Node::new(Role::TextInput, "姓名")
                .id("name")
                .text(&self.text)
                .styled(Style::default().padding(12.)),
        )
        .on_input(Message::Text)
    }
}
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size, size * 1.5)
    }
}

#[test]
fn placeholder_only_paints_and_yields_to_both_native_composition_routes() {
    const HINT: &str = "搜尋文件";
    const HINT_COLOR: Color = Color::rgb(90, 78, 87);
    const INK: Color = Color::rgb(14, 17, 23);

    #[derive(Default)]
    struct Editor {
        value: String,
        hint: bool,
        multiline: bool,
    }
    impl Application for Editor {
        type Message = String;
        fn update(&mut self, value: String) {
            self.value = value;
        }
        fn view(&self) -> View<String> {
            let field = View::new(
                Node::new(Role::TextInput, "Accessible search field")
                    .id("field")
                    .text(&self.value)
                    .styled(
                        Style::column()
                            .height(Length::Px(48.))
                            .line_height(24.)
                            .foreground(INK),
                    ),
            )
            .center_single_line_text()
            .text_options(TextOptions {
                multiline: self.multiline,
                ..TextOptions::default()
            })
            .on_input(|value| value);
            let field = if self.hint {
                field.text_input_placeholder(HINT, HINT_COLOR)
            } else {
                field
            };
            View::new(Node::new(Role::Group, "Root")).children([field])
        }
    }

    fn run(list: &DisplayList) -> (&str, Color) {
        list.ops
            .iter()
            .find_map(|op| match op {
                PaintOp::StyledText {
                    text,
                    color,
                    editable: true,
                    ..
                } => Some((text.as_str(), *color)),
                _ => None,
            })
            .unwrap()
    }
    fn render(runtime: &mut Runtime<Editor>) -> DisplayList {
        runtime.render(200., 84., &mut Metrics).unwrap()
    }

    let mut runtime = Runtime::new(Editor {
        hint: true,
        ..Editor::default()
    })
    .unwrap();
    let mut legacy = Runtime::new(Editor::default()).unwrap();
    for app in [&mut runtime, &mut legacy] {
        app.activate("field").unwrap();
    }
    let paint = render(&mut runtime);
    assert_eq!(run(&paint), (HINT, HINT_COLOR));
    assert_eq!(run(&render(&mut legacy)), ("", INK));
    assert_eq!(runtime.export_dom().unwrap(), legacy.export_dom().unwrap());
    let field = runtime.dom().by_id("field").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(
        frame.node(field).unwrap().node.name(),
        "Accessible search field"
    );
    assert_eq!(frame.node(field).unwrap().node.text_value(), Some(""));
    let caret = runtime.input_snapshot().unwrap().caret.unwrap();
    assert_eq!(Some(caret), legacy.input_snapshot().unwrap().caret);
    assert!(paint.ops.iter().any(
        |op| matches!(op, PaintOp::Quad { rect, color, .. } if *rect == caret && *color == INK)
    ));
    let extended = runtime.render_extended(200., 84., &mut Metrics).unwrap();
    assert!(extended.ops().iter().any(
        |op| matches!(op, ExtendedPaintOp::Legacy(PaintOp::StyledText {
        text, color, ..
    }) if text == HINT && *color == HINT_COLOR)
    ));

    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    assert_eq!(run(&render(&mut runtime)).0, "");
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄕ".into(),
            selection: None,
        }))
        .unwrap();
    assert_eq!(run(&render(&mut runtime)), ("ㄕ", INK));
    assert!(runtime.app().value.is_empty());
    assert_eq!(runtime.snapshot().text_inputs["field"].display_value, "ㄕ");
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: String::new(),
            selection: None,
        }))
        .unwrap();
    assert!(!runtime.input_snapshot().unwrap().composing);
    assert!(runtime.editor("field").unwrap().composition().is_some());
    assert_eq!(run(&render(&mut runtime)), (HINT, HINT_COLOR));
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("設".into())))
        .unwrap();
    assert_eq!(run(&render(&mut runtime)), ("設", INK));
    assert_eq!(runtime.app().value, "設");
    runtime
        .dispatch(Event::InputValue {
            value: String::new(),
            anchor: 0,
            cursor: 0,
        })
        .unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();
    assert_eq!(run(&render(&mut runtime)), (HINT, HINT_COLOR));
    assert!(runtime.app().value.is_empty());

    let mut multiline = Runtime::new(Editor {
        value: "value\n".repeat(16),
        hint: true,
        multiline: true,
    })
    .unwrap();
    multiline.activate("field").unwrap();
    assert!(render(&mut multiline).ops.iter().any(|op| matches!(op,
        PaintOp::StyledText { scroll, editable: true, .. } if scroll.1 > 0.)));
    multiline.dispatch(Event::Blur).unwrap();
    multiline.apply(String::new()).unwrap();
    assert!(render(&mut multiline).ops.iter().any(|op| matches!(op,
        PaintOp::StyledText { text, scroll, editable: true, .. } if text == HINT && *scroll == (0., 0.))));
}

#[test]
fn centered_single_line_uses_one_origin_for_paint_and_caret_without_shrinking_the_target() {
    struct Editor {
        centered: bool,
        multiline: bool,
    }
    impl Application for Editor {
        type Message = ();
        fn update(&mut self, _: ()) {}
        fn view(&self) -> View<()> {
            let field = View::new(
                Node::new(Role::TextInput, "Query")
                    .id("query")
                    .text("ab")
                    .styled(
                        Style::column()
                            .height(Length::Px(48.))
                            .font_size(16.)
                            .line_height(24.),
                    ),
            )
            .on_input(|_| ())
            .text_options(TextOptions {
                multiline: self.multiline,
                ..TextOptions::default()
            });
            let field = if self.centered {
                field.center_single_line_text()
            } else {
                field
            };
            View::new(Node::new(Role::Group, "Page")).children([field])
        }
    }

    for (centered, multiline, inset) in [(false, false, 0.), (true, true, 0.), (true, false, 12.)] {
        let mut runtime = Runtime::new(Editor {
            centered,
            multiline,
        })
        .unwrap();
        runtime.render(220., 100., &mut Metrics).unwrap();
        runtime
            .dispatch(Event::PointerDown { x: 5., y: 1. })
            .unwrap();
        runtime.dispatch(Event::PointerUp { x: 5., y: 1. }).unwrap();
        assert_eq!(runtime.focused(), Some("query"));
        let paint = runtime.render(220., 100., &mut Metrics).unwrap();
        let field = runtime.dom().by_id("query").unwrap();
        let bounds = runtime.computed().unwrap().nodes[field.index()].bounds;
        assert_eq!(bounds, Rect::new(0., 0., 220., 48.));
        let caret = runtime.input_snapshot().unwrap().caret.unwrap();
        assert_eq!((caret.y, caret.height), (inset, 24.));
        assert!(paint.ops.iter().any(|op| matches!(op, PaintOp::StyledText {
            rect, scroll, editable: true, ..
        } if *rect == bounds && rect.y - scroll.1 == caret.y)));
    }
}

#[test]
fn right_aligned_editor_shares_paint_pointer_and_composition_geometry() {
    struct Editor(String);

    impl Application for Editor {
        type Message = String;

        fn update(&mut self, value: String) {
            self.0 = value;
        }

        fn view(&self) -> View<String> {
            View::new(Node::new(Role::Group, "Page")).children([View::new(
                Node::new(Role::TextInput, "搜尋文件")
                    .id("query")
                    .text(&self.0)
                    .styled(
                        Style::column()
                            .height(Length::Px(48.))
                            .font_size(16.)
                            .line_height(24.),
                    ),
            )
            .center_single_line_text()
            .right_align_single_line_text()
            .inset_single_line_text_horizontally(4.)
            .text_input_placeholder("搜尋文件", Color::rgb(90, 78, 87))
            .on_input(|value| value)])
        }
    }

    fn text_origin(list: &DisplayList, expected: &str) -> (f32, f32) {
        list.ops
            .iter()
            .find_map(|op| match op {
                PaintOp::StyledText {
                    text,
                    rect,
                    scroll,
                    editable: true,
                    ..
                } if text == expected => Some((rect.x - scroll.0, rect.y - scroll.1)),
                _ => None,
            })
            .expect("expected editor paint run")
    }

    let mut runtime = Runtime::new(Editor(String::new())).unwrap();
    runtime.activate("query").unwrap();
    let empty = runtime.render(200., 80., &mut Metrics).unwrap();
    assert_eq!(text_origin(&empty, "搜尋文件"), (130., 12.));
    let input = runtime.input_snapshot().unwrap();
    assert_eq!(input.value, "");
    assert_eq!(input.caret.unwrap(), Rect::new(194., 12., 1., 24.));

    runtime.dispatch(Event::Text("設計".into())).unwrap();
    let query = runtime.render(200., 80., &mut Metrics).unwrap();
    assert_eq!(text_origin(&query, "設計"), (162., 12.));
    for event in [
        Event::PointerDown { x: 164., y: 20. },
        Event::PointerMove { x: 175., y: 20. },
        Event::PointerUp { x: 175., y: 20. },
    ] {
        runtime.dispatch(event).unwrap();
    }
    let snapshot = runtime.input_snapshot().unwrap();
    assert_eq!(
        (snapshot.selection.anchor, snapshot.selection.cursor),
        (0, 3)
    );
    let range = runtime.render(200., 80., &mut Metrics).unwrap();
    assert!(range
        .ops
        .iter()
        .any(|op| matches!(op, PaintOp::Quad { rect, .. }
        if *rect == Rect::new(162., 12., 16., 24.))));

    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄕ".into(),
            selection: Some((3, 3)),
        }))
        .unwrap();
    let preedit = runtime.render(224., 80., &mut Metrics).unwrap();
    assert_eq!(text_origin(&preedit, "ㄕ計"), (186., 12.));
    assert!(preedit
        .ops
        .iter()
        .any(|op| matches!(op, PaintOp::Quad { rect, .. }
        if *rect == Rect::new(186., 34., 16., 1.))));
    let snapshot = runtime.input_snapshot().unwrap();
    assert!(snapshot.composing);
    assert_eq!(snapshot.value, "設計");
    assert_eq!(snapshot.caret.unwrap().x, 202.);
    runtime.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();

    let long = "文".repeat(20);
    runtime
        .dispatch(Event::InputValue {
            anchor: long.len(),
            cursor: long.len(),
            value: long.clone(),
        })
        .unwrap();
    let overflow = runtime.render(200., 80., &mut Metrics).unwrap();
    assert_eq!(text_origin(&overflow, &long), (-126., 12.));
    assert!(overflow
        .ops
        .iter()
        .any(|op| matches!(op, PaintOp::StyledText { text, clip, .. }
        if text == &long && *clip == Rect::new(4., 0., 192., 48.))));
    assert_eq!(runtime.input_snapshot().unwrap().caret.unwrap().x, 194.);

    runtime.dispatch(Event::Blur).unwrap();
    runtime.apply(String::new()).unwrap();
    let cleared = runtime.render(200., 80., &mut Metrics).unwrap();
    assert_eq!(text_origin(&cleared, "搜尋文件"), (130., 12.));
}

#[test]
fn composition_is_visible_but_does_not_change_app_state_until_one_commit() {
    let mut runtime = Runtime::new(App {
        text: "台灣".to_owned(),
        changes: 0,
    })
    .unwrap();
    runtime.activate("name").unwrap();
    runtime
        .dispatch(Event::Select {
            anchor: 0,
            cursor: "台".len(),
        })
        .unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄓㄨㄥ".to_owned(),
            selection: Some((9, 9)),
        }))
        .unwrap();
    runtime.dispatch(Event::Text("n".to_owned())).unwrap();
    assert_eq!(runtime.app().text, "台灣");
    assert_eq!(runtime.app().changes, 0);
    let paint = runtime.render(300., 56., &mut Metrics).unwrap();
    assert!(paint
        .ops
        .iter()
        .any(|op| matches!(op,PaintOp::EditableText{text,..} if text=="ㄓㄨㄥ灣")));
    assert!(
        paint
            .ops
            .iter()
            .any(|op| matches!(op,PaintOp::Quad{rect,..} if rect.height==1.&&rect.width>1.)),
        "preedit underline is painted"
    );
    assert!(runtime.input_snapshot().unwrap().caret.is_some());
    // Native winit clears preedit before it sends the final commit.
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: String::new(),
            selection: None,
        }))
        .unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("中".to_owned())))
        .unwrap();
    assert_eq!(runtime.app().text, "中灣");
    assert_eq!(runtime.app().changes, 1);
    runtime.dispatch(Event::Text("文".to_owned())).unwrap();
    assert_eq!(runtime.app().text, "中文灣");
}
#[test]
fn cancelled_composition_preserves_selected_text_and_blur_clears_it() {
    let mut runtime = Runtime::new(App {
        text: "台灣".to_owned(),
        changes: 0,
    })
    .unwrap();
    runtime.activate("name").unwrap();
    runtime
        .dispatch(Event::Select {
            anchor: 0,
            cursor: 3,
        })
        .unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ni".to_owned(),
            selection: Some((2, 2)),
        }))
        .unwrap();
    runtime.dispatch(Event::Blur).unwrap();
    assert_eq!(runtime.app().text, "台灣");
    assert_eq!(runtime.app().changes, 0);
    assert!(!runtime.editor("name").unwrap().is_composing());
    assert!(runtime.input_snapshot().is_none());
}
#[test]
fn editor_deletes_graphemes_replaces_selection_and_undoes_composition() {
    let options = TextOptions::default();
    let mut editor = TextEditor::new("中👨‍👩‍👧‍👦e\u{301}");
    editor.key(EditKey::Backspace, false, false, &options);
    assert_eq!(editor.value(), "中👨‍👩‍👧‍👦");
    editor.key(EditKey::Backspace, false, false, &options);
    assert_eq!(editor.value(), "中");
    editor.key(EditKey::Undo, false, false, &options);
    assert_eq!(editor.value(), "中👨‍👩‍👧‍👦");
    editor.key(EditKey::SelectAll, false, false, &options);
    editor.ime(
        ImeEvent::Preedit {
            text: "ㄨㄣˊ".to_owned(),
            selection: Some((999, 999)),
        },
        &options,
    );
    assert_eq!(editor.value(), "中👨‍👩‍👧‍👦");
    editor.ime(ImeEvent::Commit("文".to_owned()), &options);
    assert_eq!(editor.value(), "文");
    editor.key(EditKey::Undo, false, false, &options);
    assert_eq!(editor.value(), "中👨‍👩‍👧‍👦");
    editor.key(EditKey::Redo, false, false, &options);
    assert_eq!(editor.value(), "文");
    editor.key(EditKey::Home, false, false, &options);
    editor.insert("中", &options);
    assert_eq!(editor.value(), "中文");
}
#[test]
fn browser_offsets_and_field_policies_do_not_split_unicode() {
    let text = "中😀文";
    assert_eq!(utf16_to_utf8(text, 3), 7);
    assert_eq!(utf16_to_utf8(text, 2), 3);
    assert_eq!(utf8_to_utf16(text, 7), 3);
    let mut editor = TextEditor::new("");
    let options = TextOptions {
        max_graphemes: Some(2),
        ..Default::default()
    };
    editor.insert("中\n👨‍👩‍👧‍👦文", &options);
    assert_eq!(editor.value(), "中👨‍👩‍👧‍👦");
    let readonly = TextOptions {
        read_only: true,
        ..Default::default()
    };
    assert!(!editor.insert("壞", &readonly));
    assert!(!editor.key(EditKey::Backspace, false, false, &readonly));
    assert_eq!(editor.value(), "中👨‍👩‍👧‍👦");
}

#[test]
fn late_ime_commits_do_not_follow_focus_to_another_control() {
    struct Pair {
        left: String,
        right: String,
    }
    #[derive(Clone)]
    enum Msg {
        Left(String),
        Right(String),
    }
    impl Application for Pair {
        type Message = Msg;
        fn update(&mut self, m: Msg) {
            match m {
                Msg::Left(v) => self.left = v,
                Msg::Right(v) => self.right = v,
            }
        }
        fn view(&self) -> View<Msg> {
            View::new(Node::new(Role::Group, "Fields")).children([
                View::new(
                    Node::new(Role::TextInput, "Left")
                        .id("left")
                        .text(&self.left),
                )
                .on_input(Msg::Left),
                View::new(
                    Node::new(Role::TextInput, "Right")
                        .id("right")
                        .text(&self.right),
                )
                .on_input(Msg::Right),
            ])
        }
    }
    let mut r = Runtime::new(Pair {
        left: "原文".to_owned(),
        right: "新欄位".to_owned(),
    })
    .unwrap();
    r.activate("left").unwrap();
    r.dispatch(Event::ImeFor {
        id: "left".to_owned(),
        event: ImeEvent::Preedit {
            text: "待選字".to_owned(),
            selection: Some((9, 9)),
        },
    })
    .unwrap();
    r.activate("right").unwrap();
    assert!(!r
        .dispatch(Event::ImeFor {
            id: "left".to_owned(),
            event: ImeEvent::Commit("錯欄位".to_owned())
        })
        .unwrap());
    assert_eq!(r.app().left, "原文");
    assert_eq!(r.app().right, "新欄位");
}

#[test]
fn browser_rejected_value_does_not_create_a_change_or_break_controlled_state() {
    let mut e = TextEditor::new("中文");
    let options = TextOptions {
        max_graphemes: Some(2),
        ..Default::default()
    };
    assert!(!e.replace_from_host("中文多字", 12, 12, &options));
    assert_eq!(e.value(), "中文");
    let mut e = TextEditor::new("hello world");
    e.key(EditKey::Backspace, false, true, &Default::default());
    assert_eq!(e.value(), "hello ");
    e.key(EditKey::Undo, false, false, &Default::default());
    assert_eq!(e.value(), "hello world");
}
