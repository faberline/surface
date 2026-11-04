use surface::{Node, PaintOp, Role, Style, TextMeasurer};
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
