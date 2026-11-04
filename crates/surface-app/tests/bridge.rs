#![cfg(feature = "bridge")]
use jet_bridge_protocol::{Command, Key, Outcome, Request, VERSION};
use serde_json::Value;
use surface::{Node, RangeValue, Role, Style};
use surface_app::{Application, Event, ImeEvent, Runtime, TextOptions, View};

#[derive(Default)]
struct App {
    name: String,
    volume: f64,
    dialog: bool,
}
#[derive(Clone)]
enum Message {
    Name(String),
    Volume(f64),
    Dialog(bool),
}
impl Application for App {
    type Message = Message;
    fn update(&mut self, message: Message) {
        match message {
            Message::Name(name) => self.name = name,
            Message::Volume(value) => self.volume = value,
            Message::Dialog(open) => self.dialog = open,
        }
    }
    fn view(&self) -> View<Message> {
        let mut children = vec![
            View::new(
                Node::new(Role::TextInput, "姓名")
                    .id("name")
                    .text(&self.name),
            )
            .on_input(Message::Name)
            .on_press(Message::Dialog(true)),
            View::new(
                Node::new(Role::TextInput, "Read only")
                    .id("readonly")
                    .text("locked"),
            )
            .text_options(TextOptions {
                read_only: true,
                multiline: true,
                ..Default::default()
            }),
            View::new(
                Node::new(Role::TextInput, "Disabled")
                    .id("disabled")
                    .disabled(true),
            ),
            View::new(
                Node::new(Role::Slider, "Volume")
                    .id("volume")
                    .range(RangeValue {
                        min: 0.,
                        max: 100.,
                        step: 10.,
                        value: self.volume,
                    }),
            )
            .on_value(Message::Volume),
            View::new(Node::new(Role::Button, "Open").id("open")).on_press(Message::Dialog(true)),
        ];
        if self.dialog {
            children.push(
                View::new(Node::new(Role::Dialog, "Dialog").id("dialog"))
                    .modal(Message::Dialog(false))
                    .children([View::new(Node::new(Role::Button, "Close").id("close"))
                        .on_press(Message::Dialog(false))]),
            );
        }
        View::new(
            Node::new(Role::Window, "Bridge test")
                .id("root")
                .styled(Style::column()),
        )
        .children(children)
    }
}
fn call(runtime: &mut Runtime<App>, command: Command) -> Result<Value, String> {
    let response = runtime.execute_bridge(Request {
        version: VERSION,
        id: 42,
        command,
    });
    let wire = serde_json::to_value(&response).unwrap();
    assert_eq!(wire["version"], 1);
    assert_eq!(wire["id"], 42);
    match response.outcome {
        Outcome::Ok { result } => Ok(result),
        Outcome::Error { code, .. } => Err(code),
    }
}
fn key(key: Key) -> Command {
    Command::Key {
        key,
        extend: false,
        by_word: false,
    }
}
#[test]
fn wire_actions_update_controlled_state_and_export_the_new_tree() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    call(
        &mut runtime,
        Command::Fill {
            id: "name".into(),
            value: "中文🦀".into(),
        },
    )
    .unwrap();
    assert_eq!(runtime.app().name, "中文🦀");
    assert!(
        !runtime.app().dialog,
        "fill must focus without activating the field's press binding"
    );
    let snapshot = call(&mut runtime, Command::Snapshot).unwrap();
    assert_eq!(snapshot["text_inputs"]["name"]["value"], "中文🦀");
    assert_eq!(snapshot["interaction"]["focused"], "name");
    assert!(snapshot["layout"].is_null());
    let query = call(
        &mut runtime,
        Command::Query {
            id: None,
            role: Some("TextInput".into()),
            name: Some("姓名".into()),
        },
    )
    .unwrap();
    assert_eq!(query["matches"].as_array().unwrap().len(), 1);
    assert_eq!(query["matches"][0]["node"]["stable_id"], "name");
    call(
        &mut runtime,
        Command::SetValue {
            id: "volume".into(),
            value: 24.,
        },
    )
    .unwrap();
    assert_eq!(runtime.app().volume, 20.);
    call(&mut runtime, key(Key::SelectAll)).unwrap();
    call(&mut runtime, key(Key::Backspace)).unwrap();
    assert_eq!(runtime.app().name, "");
    call(&mut runtime, key(Key::Undo)).unwrap();
    assert_eq!(runtime.app().name, "中文🦀");
}
#[test]
fn rejected_requests_preserve_focus_composition_and_state() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    call(
        &mut runtime,
        Command::Fill {
            id: "name".into(),
            value: "前".into(),
        },
    )
    .unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄋㄧ".into(),
            selection: Some((6, 6)),
        }))
        .unwrap();
    let before = runtime.export_dom().unwrap();
    for (command, code) in [
        (
            Command::Fill {
                id: "readonly".into(),
                value: "bad".into(),
            },
            "read_only",
        ),
        (
            Command::Fill {
                id: "disabled".into(),
                value: "bad".into(),
            },
            "disabled",
        ),
        (
            Command::Activate {
                id: "missing".into(),
            },
            "missing_node",
        ),
        (
            Command::Fill {
                id: "name".into(),
                value: "bad".into(),
            },
            "composition_active",
        ),
        (key(Key::Tab), "composition_active"),
    ] {
        assert_eq!(call(&mut runtime, command).unwrap_err(), code);
        assert_eq!(runtime.export_dom().unwrap(), before);
    }
    let response = runtime.execute_bridge(Request {
        version: 999,
        id: 1,
        command: Command::Fill {
            id: "name".into(),
            value: "bad".into(),
        },
    });
    assert!(matches!(response.outcome,Outcome::Error {code,..} if code == "unsupported_version"));
    assert_eq!(runtime.export_dom().unwrap(), before);
    call(&mut runtime, key(Key::Escape)).unwrap();
    assert!(!runtime.editor("name").unwrap().is_composing());
    assert_eq!(runtime.app().name, "前");
    call(
        &mut runtime,
        Command::Activate {
            id: "readonly".into(),
        },
    )
    .unwrap();
    let before = runtime.export_dom().unwrap();
    assert_eq!(
        call(&mut runtime, key(Key::Enter)).unwrap_err(),
        "read_only"
    );
    assert_eq!(runtime.export_dom().unwrap(), before);
}
#[test]
fn modal_scope_blocks_background_actions_and_escape_restores_the_opener() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    call(&mut runtime, Command::Activate { id: "open".into() }).unwrap();
    assert_eq!(runtime.focused(), Some("close"));
    let before = runtime.export_dom().unwrap();
    for command in [
        Command::Fill {
            id: "name".into(),
            value: "bad".into(),
        },
        Command::SetValue {
            id: "volume".into(),
            value: 70.,
        },
        Command::Activate { id: "open".into() },
    ] {
        assert_eq!(call(&mut runtime, command).unwrap_err(), "outside_modal");
        assert_eq!(runtime.export_dom().unwrap(), before);
    }
    call(&mut runtime, key(Key::Tab)).unwrap();
    assert_eq!(runtime.focused(), Some("close"));
    call(&mut runtime, key(Key::Escape)).unwrap();
    assert!(!runtime.app().dialog);
    assert_eq!(runtime.focused(), Some("open"));
}
