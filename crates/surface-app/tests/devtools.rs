//! Executor tests for `Runtime::execute_devtools` (issue #130, O6), and the
//! parity of the jet `execute_bridge` adapter with pinned `origin/main` output.
#![cfg(feature = "devtools")]
use serde_json::{json, Value};
use surface::{Node, RangeValue, Role, Style, TextMeasurer};
use surface_app::{Application, DomExport, Event, ImeEvent, Key, Runtime, TextOptions, View};
use surface_devtools_protocol::{ErrorCode, Reply, Request, VERSION};

#[derive(Default)]
struct App {
    name: String,
    volume: f64,
    dialog: bool,
    submit_on_enter: bool,
    handle_delete: bool,
    submissions: usize,
}
#[derive(Clone)]
enum Message {
    Name(String),
    Volume(f64),
    Dialog(bool),
    Submit,
}
impl Application for App {
    type Message = Message;
    fn update(&mut self, message: Message) {
        match message {
            Message::Name(name) => self.name = name,
            Message::Volume(value) => self.volume = value,
            Message::Dialog(open) => self.dialog = open,
            Message::Submit => self.submissions += 1,
        }
    }
    fn view(&self) -> View<Message> {
        let name = View::new(
            Node::new(Role::TextInput, "姓名")
                .id("name")
                .text(&self.name),
        )
        .on_input(Message::Name)
        .on_press(Message::Dialog(true));
        let name = if self.submit_on_enter {
            name.on_key(|press| {
                (press.key == Key::Enter && press.modifiers.is_empty()).then_some(Message::Submit)
            })
        } else {
            name
        };
        let mut children = vec![
            name,
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
            let close = View::new(Node::new(Role::Button, "Close").id("close"))
                .on_press(Message::Dialog(false));
            let close = if self.handle_delete {
                close.on_key(|press| {
                    (matches!(press.key, Key::Backspace | Key::Delete)
                        && press.modifiers.is_empty())
                    .then_some(Message::Submit)
                })
            } else {
                close
            };
            children.push(
                View::new(Node::new(Role::Dialog, "Dialog").id("dialog"))
                    .modal(Message::Dialog(false))
                    .children([close]),
            );
        }
        View::new(
            Node::new(Role::Window, "Devtools test")
                .id("root")
                .styled(Style::column()),
        )
        .children(children)
    }
}

/// Fixed metrics, as the other `surface-app` tests use.
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size, size * 1.5)
    }
}

fn runtime() -> Runtime<App> {
    let mut runtime = Runtime::new(App::default()).unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    runtime
}
fn send(runtime: &mut Runtime<App>, method: &str, params: Value) -> Reply {
    let reply = runtime.execute_devtools(Request {
        v: VERSION,
        id: 7,
        method: method.into(),
        params,
    });
    assert_eq!(reply.id, 7);
    reply
}
fn ok(runtime: &mut Runtime<App>, method: &str, params: Value) -> Value {
    let reply = send(runtime, method, params);
    assert!(reply.ok, "{method} failed: {:?}", reply.error);
    reply.result.unwrap()
}
fn err(runtime: &mut Runtime<App>, method: &str, params: Value) -> (ErrorCode, String) {
    let reply = send(runtime, method, params);
    assert!(!reply.ok, "{method} should fail, got {:?}", reply.result);
    assert!(reply.result.is_none());
    let error = reply.error.unwrap();
    (error.code, error.message)
}
/// Runs a rejected request and checks that focus and the exported state stay.
fn rejected(runtime: &mut Runtime<App>, method: &str, params: Value) -> (ErrorCode, String) {
    let focused = runtime.focused().map(str::to_owned);
    let before = runtime.export_dom().unwrap();
    let error = err(runtime, method, params);
    assert_eq!(runtime.focused().map(str::to_owned), focused, "{method}");
    assert_eq!(runtime.export_dom().unwrap(), before, "{method}");
    error
}
fn open_dialog(runtime: &mut Runtime<App>) {
    ok(runtime, "Action.activate", json!({"id":"open"}));
    assert!(runtime.app().dialog);
    assert_eq!(runtime.focused(), Some("close"));
}

const METHODS: [&str; 7] = [
    "Devtools.capabilities",
    "DOM.snapshot",
    "DOM.query",
    "Action.activate",
    "Action.fill",
    "Action.setValue",
    "Action.key",
];

// Devtools.capabilities

#[test]
fn capabilities_lists_the_executor_methods() {
    let mut runtime = runtime();
    let result = ok(&mut runtime, "Devtools.capabilities", json!({}));
    assert_eq!(result["version"], 1);
    let methods: Vec<&str> = result["methods"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap())
        .collect();
    assert_eq!(methods, METHODS);
    assert_eq!(result["domains"], json!(["Devtools", "DOM", "Action"]));
    assert!(result["schemas"]
        .as_array()
        .unwrap()
        .contains(&json!("surface.dom.v1")));
    assert_eq!(result["application"], "Surface");
    // The wire form is the protocol's `Capabilities`, plus the application.
    let caps: surface_devtools_protocol::Capabilities = serde_json::from_value(result).unwrap();
    assert_eq!(caps.version, VERSION);
}

#[test]
fn capabilities_advertises_native_capture_only_when_the_host_installs_it() {
    use surface_devtools_protocol::PAGE_SCREENSHOT;

    let mut runtime = runtime();
    runtime.set_host_methods(&[PAGE_SCREENSHOT]);
    let result = ok(&mut runtime, "Devtools.capabilities", json!({}));
    assert_eq!(
        result["methods"],
        json!(METHODS
            .into_iter()
            .chain([PAGE_SCREENSHOT])
            .collect::<Vec<_>>())
    );
    assert_eq!(
        result["domains"],
        json!(["Devtools", "DOM", "Action", "Page"])
    );
    assert_eq!(
        err(&mut runtime, PAGE_SCREENSHOT, json!({})).0,
        ErrorCode::UnknownMethod
    );

    runtime.set_host_methods(&[]);
    let result = ok(&mut runtime, "Devtools.capabilities", json!({}));
    assert_eq!(result["methods"], json!(METHODS));
}

#[test]
fn capabilities_rejects_another_version_and_unknown_methods() {
    let mut runtime = runtime();
    let reply = runtime.execute_devtools(Request {
        v: 2,
        id: 9,
        method: "Devtools.capabilities".into(),
        params: json!({}),
    });
    assert!(!reply.ok);
    assert_eq!(reply.id, 9);
    assert_eq!(reply.error.unwrap().code, ErrorCode::UnsupportedVersion);
    let (code, message) = err(&mut runtime, "Devtools.nope", json!({}));
    assert_eq!(code, ErrorCode::UnknownMethod);
    assert!(message.contains("Devtools.nope"), "{message}");
}

// DOM.snapshot

#[test]
fn dom_snapshot_returns_the_dom_export() {
    let mut runtime = runtime();
    let result = ok(&mut runtime, "DOM.snapshot", json!({}));
    assert_eq!(result, serde_json::to_value(runtime.snapshot()).unwrap());
    let export: DomExport = serde_json::from_value(result).unwrap();
    assert_eq!(export.version, 1);
    assert!(export.layout.is_some());
    // Unknown params are tolerated.
    ok(&mut runtime, "DOM.snapshot", json!({"extra":true}));
}
#[test]
fn dom_snapshot_rejects_another_version_without_side_effects() {
    let mut runtime = runtime();
    let before = runtime.export_dom().unwrap();
    let reply = runtime.execute_devtools(Request {
        v: 0,
        id: 1,
        method: "DOM.snapshot".into(),
        params: json!({}),
    });
    assert_eq!(reply.error.unwrap().code, ErrorCode::UnsupportedVersion);
    assert_eq!(runtime.export_dom().unwrap(), before);
}

// DOM.query

#[test]
fn dom_query_matches_stable_id_role_and_name() {
    let mut runtime = runtime();
    let result = ok(
        &mut runtime,
        "DOM.query",
        json!({"role":"TextInput","name":"姓名"}),
    );
    let matches = result["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["node"]["stable_id"], "name");
    assert_eq!(matches[0]["in_scope"], true);
    assert!(matches[0]["layout"].is_object());
    let by_id = ok(&mut runtime, "DOM.query", json!({"id":"volume"}));
    assert_eq!(by_id["matches"][0]["node"]["role"], "Slider");
    let inputs = ok(&mut runtime, "DOM.query", json!({"role":"TextInput"}));
    assert_eq!(inputs["matches"].as_array().unwrap().len(), 3);
    let none = ok(&mut runtime, "DOM.query", json!({"id":"missing"}));
    assert_eq!(none["matches"], json!([]));
    open_dialog(&mut runtime);
    let open = ok(&mut runtime, "DOM.query", json!({"id":"open"}));
    assert_eq!(open["matches"][0]["in_scope"], false);
}
#[test]
fn dom_query_rejects_an_unknown_role_and_bad_params() {
    let mut runtime = runtime();
    let (code, message) = err(&mut runtime, "DOM.query", json!({"role":"Bogus"}));
    assert_eq!(code, ErrorCode::InvalidParams);
    assert!(message.starts_with("invalid_role: "), "{message}");
    let (code, _) = err(&mut runtime, "DOM.query", json!({"id":5}));
    assert_eq!(code, ErrorCode::InvalidParams);
}

// Action.activate

#[test]
fn action_activate_runs_the_press_binding_and_focuses() {
    let mut runtime = runtime();
    let result = ok(&mut runtime, "Action.activate", json!({"id":"open"}));
    assert_eq!(result, json!({"changed":true,"focused":"close"}));
    assert!(runtime.app().dialog);
}
#[test]
fn action_activate_rejects_missing_disabled_and_bad_params() {
    let mut runtime = runtime();
    let (code, message) = rejected(&mut runtime, "Action.activate", json!({"id":"missing"}));
    assert_eq!(code, ErrorCode::MissingNode);
    assert_eq!(message, "no current node with ID missing");
    let (code, _) = rejected(&mut runtime, "Action.activate", json!({"id":"disabled"}));
    assert_eq!(code, ErrorCode::Disabled);
    let (code, _) = rejected(&mut runtime, "Action.activate", json!({}));
    assert_eq!(code, ErrorCode::InvalidParams);
}

// Action.fill

#[test]
fn action_fill_sets_the_value_and_focus_without_pressing() {
    let mut runtime = runtime();
    let result = ok(
        &mut runtime,
        "Action.fill",
        json!({"id":"name","value":"中文🦀"}),
    );
    assert_eq!(result, json!({"changed":true,"focused":"name"}));
    assert_eq!(runtime.app().name, "中文🦀");
    assert!(!runtime.app().dialog, "fill must not press the field");
    let snapshot = ok(&mut runtime, "DOM.snapshot", json!({}));
    assert_eq!(snapshot["text_inputs"]["name"]["value"], "中文🦀");
}
#[test]
fn action_fill_rejects_read_only_wrong_role_and_composition() {
    let mut runtime = runtime();
    let (code, message) = rejected(
        &mut runtime,
        "Action.fill",
        json!({"id":"readonly","value":"x"}),
    );
    assert_eq!(code, ErrorCode::ActionFailed);
    assert_eq!(message, "read_only: field readonly is read-only");
    let (code, message) = rejected(
        &mut runtime,
        "Action.fill",
        json!({"id":"volume","value":"x"}),
    );
    assert_eq!(code, ErrorCode::InvalidParams);
    assert!(message.starts_with("wrong_role: "), "{message}");
    let (code, _) = rejected(&mut runtime, "Action.fill", json!({"id":"name"}));
    assert_eq!(code, ErrorCode::InvalidParams);
    ok(
        &mut runtime,
        "Action.fill",
        json!({"id":"name","value":"前"}),
    );
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄋㄧ".into(),
            selection: Some((6, 6)),
        }))
        .unwrap();
    let (code, message) = rejected(
        &mut runtime,
        "Action.fill",
        json!({"id":"name","value":"bad"}),
    );
    assert_eq!(code, ErrorCode::NotReady);
    assert!(message.starts_with("composition_active: "), "{message}");
}

// Action.setValue

#[test]
fn action_set_value_snaps_to_the_step() {
    let mut runtime = runtime();
    let result = ok(
        &mut runtime,
        "Action.setValue",
        json!({"id":"volume","value":24.0}),
    );
    assert_eq!(result["changed"], true);
    assert_eq!(runtime.app().volume, 20.);
}
#[test]
fn action_set_value_rejects_a_non_slider_and_bad_params() {
    let mut runtime = runtime();
    let (code, message) = rejected(
        &mut runtime,
        "Action.setValue",
        json!({"id":"name","value":1.0}),
    );
    assert_eq!(code, ErrorCode::InvalidParams);
    assert!(message.starts_with("wrong_role: "), "{message}");
    let (code, _) = rejected(
        &mut runtime,
        "Action.setValue",
        json!({"id":"volume","value":"loud"}),
    );
    assert_eq!(code, ErrorCode::InvalidParams);
}

// Action.key

#[test]
fn action_key_edits_the_focused_field() {
    let mut runtime = runtime();
    ok(
        &mut runtime,
        "Action.fill",
        json!({"id":"name","value":"中文"}),
    );
    ok(&mut runtime, "Action.key", json!({"key":"select_all"}));
    ok(&mut runtime, "Action.key", json!({"key":"backspace"}));
    assert_eq!(runtime.app().name, "");
    ok(&mut runtime, "Action.key", json!({"key":"undo"}));
    assert_eq!(runtime.app().name, "中文");
    let result = ok(&mut runtime, "Action.key", json!({"key":"tab"}));
    assert_eq!(result["focused"], "readonly");
    ok(
        &mut runtime,
        "Action.key",
        json!({"key":"tab","extend":true}),
    );
    assert_eq!(runtime.focused(), Some("name"));
}
#[test]
fn action_enter_routes_to_single_line_key_handler_after_composition() {
    let mut runtime = Runtime::new(App {
        submit_on_enter: true,
        ..App::default()
    })
    .unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    ok(
        &mut runtime,
        "Action.fill",
        json!({"id":"name","value":"2024/02/"}),
    );
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "２９".into(),
            selection: Some((6, 6)),
        }))
        .unwrap();
    let (code, message) = rejected(&mut runtime, "Action.key", json!({"key":"enter"}));
    assert_eq!(code, ErrorCode::NotReady);
    assert!(message.starts_with("composition_active: "));
    assert_eq!(runtime.app().submissions, 0);
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("２９".into())))
        .unwrap();
    assert_eq!(runtime.app().name, "2024/02/２９");
    assert_eq!(runtime.app().submissions, 0);
    ok(&mut runtime, "Action.key", json!({"key":"enter"}));
    assert_eq!(runtime.app().submissions, 1);
    assert!(!runtime.app().dialog);
}
#[test]
fn action_delete_and_backspace_reach_a_focused_control_handler() {
    let mut runtime = Runtime::new(App {
        handle_delete: true,
        ..App::default()
    })
    .unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    open_dialog(&mut runtime);
    for key in ["delete", "backspace"] {
        let result = ok(&mut runtime, "Action.key", json!({ "key": key }));
        assert_eq!(result, json!({ "changed": true, "focused": "close" }));
    }
    let modified = ok(
        &mut runtime,
        "Action.key",
        json!({ "key": "delete", "extend": true }),
    );
    assert_eq!(modified, json!({ "changed": false, "focused": "close" }));
    assert_eq!(runtime.app().submissions, 2);
}
#[test]
fn action_key_rejects_no_focus_read_only_and_unknown_keys() {
    let mut runtime = runtime();
    let (code, message) = rejected(&mut runtime, "Action.key", json!({"key":"left"}));
    assert_eq!(code, ErrorCode::NotReady);
    assert_eq!(message, "no_focused_control: focus a control first");
    let (code, _) = rejected(&mut runtime, "Action.key", json!({"key":"f13"}));
    assert_eq!(code, ErrorCode::InvalidParams);
    ok(&mut runtime, "Action.activate", json!({"id":"readonly"}));
    let (code, message) = rejected(&mut runtime, "Action.key", json!({"key":"enter"}));
    assert_eq!(code, ErrorCode::ActionFailed);
    assert!(message.starts_with("read_only: "), "{message}");
}

// Acceptance row 4: rejected targets keep focus and state.

#[test]
fn disabled_and_out_of_modal_targets_keep_focus_and_state() {
    let mut runtime = runtime();
    ok(&mut runtime, "Action.activate", json!({"id":"volume"}));
    for (method, params) in [
        ("Action.activate", json!({"id":"disabled"})),
        ("Action.fill", json!({"id":"disabled","value":"bad"})),
    ] {
        let (code, message) = rejected(&mut runtime, method, params);
        assert_eq!(code, ErrorCode::Disabled, "{method}");
        assert_eq!(message, "control disabled is disabled");
        assert_eq!(runtime.focused(), Some("volume"));
    }
    open_dialog(&mut runtime);
    for (method, params) in [
        ("Action.activate", json!({"id":"open"})),
        ("Action.fill", json!({"id":"name","value":"bad"})),
        ("Action.setValue", json!({"id":"volume","value":70.0})),
    ] {
        let (code, message) = rejected(&mut runtime, method, params);
        assert_eq!(code, ErrorCode::OutsideModal, "{method}");
        assert!(
            message.ends_with("is outside the active modal"),
            "{message}"
        );
        assert_eq!(runtime.focused(), Some("close"));
    }
    assert!(runtime.app().dialog);
    assert_eq!(runtime.app().volume, 0.);
}

// Acceptance row 3: the jet adapter gives the same bytes as `origin/main`.

#[cfg(feature = "bridge")]
mod parity {
    use super::*;
    use jet_bridge_protocol::{Command, Key, Request as JetRequest, VERSION as JET};

    fn key(key: Key) -> Command {
        Command::Key {
            key,
            extend: false,
            by_word: false,
        }
    }
    /// Runs a fixed script through `execute_bridge` and returns each
    /// response as compact JSON.
    fn script() -> Vec<String> {
        let mut runtime = runtime();
        let mut lines = Vec::new();
        let mut id = 0;
        let mut call = |runtime: &mut Runtime<App>, version: u32, command: Command| {
            id += 1;
            let response = runtime.execute_bridge(JetRequest {
                version,
                id,
                command,
            });
            lines.push(serde_json::to_string(&response).unwrap());
        };
        let fill = |id: &str, value: &str| Command::Fill {
            id: id.into(),
            value: value.into(),
        };
        let set = |id: &str, value: f64| Command::SetValue {
            id: id.into(),
            value,
        };
        let activate = |id: &str| Command::Activate { id: id.into() };
        let query = |id: Option<&str>, role: Option<&str>, name: Option<&str>| Command::Query {
            id: id.map(Into::into),
            role: role.map(Into::into),
            name: name.map(Into::into),
        };
        let r = &mut runtime;
        call(r, JET, Command::Capabilities);
        call(r, JET, key(Key::Left));
        call(r, JET, Command::Snapshot);
        call(r, JET, query(None, Some("TextInput"), Some("姓名")));
        call(r, JET, query(Some("volume"), None, None));
        call(r, JET, query(None, Some("Bogus"), None));
        call(r, JET, fill("readonly", "bad"));
        call(r, JET, fill("disabled", "bad"));
        call(r, JET, fill("volume", "bad"));
        call(r, JET, set("volume", 24.));
        call(r, JET, set("name", 1.));
        call(r, JET, set("volume", f64::NAN));
        call(r, JET, set("disabled", 1.));
        call(r, JET, fill("name", "中文🦀"));
        call(r, JET, key(Key::SelectAll));
        call(r, JET, key(Key::Backspace));
        call(r, JET, key(Key::Undo));
        call(
            r,
            JET,
            Command::Key {
                key: Key::Left,
                extend: true,
                by_word: true,
            },
        );
        call(r, JET, key(Key::End));
        call(r, JET, key(Key::Space));
        r.dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄋㄧ".into(),
            selection: Some((6, 6)),
        }))
        .unwrap();
        call(r, JET, fill("name", "bad"));
        call(r, JET, key(Key::Tab));
        call(r, JET, key(Key::Escape));
        call(r, JET, key(Key::Tab));
        call(r, JET, key(Key::Enter));
        call(r, JET, key(Key::Delete));
        call(r, JET, activate("missing"));
        call(r, JET, activate("disabled"));
        call(r, JET, activate("open"));
        call(r, JET, activate("open"));
        call(r, JET, fill("name", "bad"));
        call(r, JET, set("volume", 70.));
        call(r, JET, query(Some("open"), None, None));
        call(r, JET, key(Key::Tab));
        call(r, JET, Command::Snapshot);
        call(r, JET, key(Key::Escape));
        call(r, 999, Command::Capabilities);
        call(r, JET, Command::Snapshot);
        lines
    }

    /// Captured from `execute_bridge` on `origin/main` (`02197c2`), default build.
    const FIXTURES: &[&str] = &[
        r##"{"version":1,"id":1,"status":"ok","result":{"application":"Surface","methods":["capabilities","snapshot","query","activate","fill","set_value","key"],"role_names":"Surface Role enum names; for example TextInput","runtime":"surface","selection_offsets":"utf8_bytes","snapshot_schema":"surface.dom.v1","version":1}}"##,
        r##"{"version":1,"id":2,"status":"error","code":"no_focused_control","message":"focus a control first"}"##,
        r##"{"version":1,"id":3,"status":"ok","result":{"dom":{"nodes":[{"checked":null,"disabled":false,"first_child":1,"last_child":5,"mixed":false,"name":"Devtools test","next_sibling":null,"parent":null,"range":null,"role":"Window","selected":null,"stable_id":"root","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"姓名","next_sibling":2,"parent":0,"range":null,"role":"TextInput","selected":null,"stable_id":"name","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":""},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Read only","next_sibling":3,"parent":0,"range":null,"role":"TextInput","selected":null,"stable_id":"readonly","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":"locked"},{"checked":null,"disabled":true,"first_child":null,"last_child":null,"mixed":false,"name":"Disabled","next_sibling":4,"parent":0,"range":null,"role":"TextInput","selected":null,"stable_id":"disabled","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Volume","next_sibling":5,"parent":0,"range":{"max":100.0,"min":0.0,"step":10.0,"value":0.0},"role":"Slider","selected":null,"stable_id":"volume","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Open","next_sibling":null,"parent":0,"range":null,"role":"Button","selected":null,"stable_id":"open","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null}],"version":1},"interaction":{"focused":null,"hovered":null,"pressed":null},"layout":{"nodes":[{"bounds":{"height":240.0,"width":320.0,"x":0.0,"y":0.0},"clip":{"height":240.0,"width":320.0,"x":0.0,"y":0.0},"id":0},{"bounds":{"height":0.0,"width":320.0,"x":0.0,"y":0.0},"clip":{"height":240.0,"width":320.0,"x":0.0,"y":0.0},"id":1},{"bounds":{"height":24.0,"width":320.0,"x":0.0,"y":0.0},"clip":{"height":240.0,"width":320.0,"x":0.0,"y":0.0},"id":2},{"bounds":{"height":0.0,"width":320.0,"x":0.0,"y":24.0},"clip":{"height":240.0,"width":320.0,"x":0.0,"y":0.0},"id":3},{"bounds":{"height":0.0,"width":320.0,"x":0.0,"y":24.0},"clip":{"height":240.0,"width":320.0,"x":0.0,"y":0.0},"id":4},{"bounds":{"height":0.0,"width":320.0,"x":0.0,"y":24.0},"clip":{"height":240.0,"width":320.0,"x":0.0,"y":0.0},"id":5}],"paint_order":[0,1,2,3,4,5],"viewport":{"height":240.0,"width":320.0,"x":0.0,"y":0.0}},"text_inputs":{"name":{"composing":false,"display_value":"","preedit_range":null,"selection_anchor":0,"selection_cursor":0,"value":""},"readonly":{"composing":false,"display_value":"locked","preedit_range":null,"selection_anchor":6,"selection_cursor":6,"value":"locked"}},"version":1}}"##,
        r##"{"version":1,"id":4,"status":"ok","result":{"matches":[{"in_scope":true,"layout":{"bounds":{"height":0.0,"width":320.0,"x":0.0,"y":0.0},"clip":{"height":240.0,"width":320.0,"x":0.0,"y":0.0},"id":1},"node":{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"姓名","next_sibling":2,"parent":0,"range":null,"role":"TextInput","selected":null,"stable_id":"name","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":""}}]}}"##,
        r##"{"version":1,"id":5,"status":"ok","result":{"matches":[{"in_scope":true,"layout":{"bounds":{"height":0.0,"width":320.0,"x":0.0,"y":24.0},"clip":{"height":240.0,"width":320.0,"x":0.0,"y":0.0},"id":4},"node":{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Volume","next_sibling":5,"parent":0,"range":{"max":100.0,"min":0.0,"step":10.0,"value":0.0},"role":"Slider","selected":null,"stable_id":"volume","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null}}]}}"##,
        r##"{"version":1,"id":6,"status":"error","code":"invalid_role","message":"use a Surface Role enum name; for example TextInput"}"##,
        r##"{"version":1,"id":7,"status":"error","code":"read_only","message":"field readonly is read-only"}"##,
        r##"{"version":1,"id":8,"status":"error","code":"disabled","message":"control disabled is disabled"}"##,
        r##"{"version":1,"id":9,"status":"error","code":"wrong_role","message":"fill requires a text input"}"##,
        r##"{"version":1,"id":10,"status":"ok","result":{"changed":true,"focused":null}}"##,
        r##"{"version":1,"id":11,"status":"error","code":"wrong_role","message":"set_value requires a slider"}"##,
        r##"{"version":1,"id":12,"status":"error","code":"invalid_value","message":"range value must be finite"}"##,
        r##"{"version":1,"id":13,"status":"error","code":"disabled","message":"control disabled is disabled"}"##,
        r##"{"version":1,"id":14,"status":"ok","result":{"changed":true,"focused":"name"}}"##,
        r##"{"version":1,"id":15,"status":"ok","result":{"changed":true,"focused":"name"}}"##,
        r##"{"version":1,"id":16,"status":"ok","result":{"changed":true,"focused":"name"}}"##,
        r##"{"version":1,"id":17,"status":"ok","result":{"changed":true,"focused":"name"}}"##,
        r##"{"version":1,"id":18,"status":"ok","result":{"changed":true,"focused":"name"}}"##,
        r##"{"version":1,"id":19,"status":"ok","result":{"changed":true,"focused":"name"}}"##,
        r##"{"version":1,"id":20,"status":"ok","result":{"changed":true,"focused":"name"}}"##,
        r##"{"version":1,"id":21,"status":"error","code":"composition_active","message":"finish composition or send Escape before filling"}"##,
        r##"{"version":1,"id":22,"status":"error","code":"composition_active","message":"finish composition or send Escape before a semantic key"}"##,
        r##"{"version":1,"id":23,"status":"ok","result":{"changed":true,"focused":"name"}}"##,
        r##"{"version":1,"id":24,"status":"ok","result":{"changed":true,"focused":"readonly"}}"##,
        r##"{"version":1,"id":25,"status":"error","code":"read_only","message":"the focused field is read-only"}"##,
        r##"{"version":1,"id":26,"status":"error","code":"read_only","message":"the focused field is read-only"}"##,
        r##"{"version":1,"id":27,"status":"error","code":"missing_node","message":"no current node with ID missing"}"##,
        r##"{"version":1,"id":28,"status":"error","code":"disabled","message":"control disabled is disabled"}"##,
        r##"{"version":1,"id":29,"status":"ok","result":{"changed":true,"focused":"close"}}"##,
        r##"{"version":1,"id":30,"status":"error","code":"outside_modal","message":"control open is outside the active modal"}"##,
        r##"{"version":1,"id":31,"status":"error","code":"outside_modal","message":"control name is outside the active modal"}"##,
        r##"{"version":1,"id":32,"status":"error","code":"outside_modal","message":"control volume is outside the active modal"}"##,
        r##"{"version":1,"id":33,"status":"ok","result":{"matches":[{"in_scope":false,"layout":null,"node":{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Open","next_sibling":6,"parent":0,"range":null,"role":"Button","selected":null,"stable_id":"open","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null}}]}}"##,
        r##"{"version":1,"id":34,"status":"ok","result":{"changed":true,"focused":"close"}}"##,
        r##"{"version":1,"id":35,"status":"ok","result":{"dom":{"nodes":[{"checked":null,"disabled":false,"first_child":1,"last_child":6,"mixed":false,"name":"Devtools test","next_sibling":null,"parent":null,"range":null,"role":"Window","selected":null,"stable_id":"root","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"姓名","next_sibling":2,"parent":0,"range":null,"role":"TextInput","selected":null,"stable_id":"name","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":"中文🦀 "},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Read only","next_sibling":3,"parent":0,"range":null,"role":"TextInput","selected":null,"stable_id":"readonly","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":"locked"},{"checked":null,"disabled":true,"first_child":null,"last_child":null,"mixed":false,"name":"Disabled","next_sibling":4,"parent":0,"range":null,"role":"TextInput","selected":null,"stable_id":"disabled","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Volume","next_sibling":5,"parent":0,"range":{"max":100.0,"min":0.0,"step":10.0,"value":20.0},"role":"Slider","selected":null,"stable_id":"volume","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Open","next_sibling":6,"parent":0,"range":null,"role":"Button","selected":null,"stable_id":"open","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null},{"checked":null,"disabled":false,"first_child":7,"last_child":7,"mixed":false,"name":"Dialog","next_sibling":null,"parent":0,"range":null,"role":"Dialog","selected":null,"stable_id":"dialog","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Close","next_sibling":null,"parent":6,"range":null,"role":"Button","selected":null,"stable_id":"close","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null}],"version":1},"interaction":{"focused":"close","hovered":null,"pressed":null},"layout":null,"text_inputs":{"name":{"composing":false,"display_value":"中文🦀 ","preedit_range":null,"selection_anchor":11,"selection_cursor":11,"value":"中文🦀 "},"readonly":{"composing":false,"display_value":"locked","preedit_range":null,"selection_anchor":6,"selection_cursor":6,"value":"locked"}},"version":1}}"##,
        r##"{"version":1,"id":36,"status":"ok","result":{"changed":true,"focused":"open"}}"##,
        r##"{"version":1,"id":37,"status":"error","code":"unsupported_version","message":"use bridge version 1"}"##,
        r##"{"version":1,"id":38,"status":"ok","result":{"dom":{"nodes":[{"checked":null,"disabled":false,"first_child":1,"last_child":5,"mixed":false,"name":"Devtools test","next_sibling":null,"parent":null,"range":null,"role":"Window","selected":null,"stable_id":"root","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"姓名","next_sibling":2,"parent":0,"range":null,"role":"TextInput","selected":null,"stable_id":"name","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":"中文🦀 "},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Read only","next_sibling":3,"parent":0,"range":null,"role":"TextInput","selected":null,"stable_id":"readonly","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":"locked"},{"checked":null,"disabled":true,"first_child":null,"last_child":null,"mixed":false,"name":"Disabled","next_sibling":4,"parent":0,"range":null,"role":"TextInput","selected":null,"stable_id":"disabled","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Volume","next_sibling":5,"parent":0,"range":{"max":100.0,"min":0.0,"step":10.0,"value":20.0},"role":"Slider","selected":null,"stable_id":"volume","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null},{"checked":null,"disabled":false,"first_child":null,"last_child":null,"mixed":false,"name":"Open","next_sibling":null,"parent":0,"range":null,"role":"Button","selected":null,"stable_id":"open","style":{"align":"Stretch","background":[0,0,0,0],"border_color":[0,0,0,0],"border_width":0.0,"clip":false,"direction":"Column","font_size":16.0,"foreground":[29,27,32,255],"gap":0.0,"grow":0.0,"height":"Auto","inset":{"bottom":"Auto","left":"Auto","right":"Auto","top":"Auto"},"justify":"Start","max_height":"Auto","max_width":"Auto","min_height":0.0,"min_width":0.0,"padding":0.0,"position":"Flow","radius":0.0,"scroll_x":0.0,"scroll_y":0.0,"state_layer":[0,0,0,0],"width":"Auto","wrap":false,"z_index":0},"tab_index":null,"text":null}],"version":1},"interaction":{"focused":"open","hovered":null,"pressed":null},"layout":null,"text_inputs":{"name":{"composing":false,"display_value":"中文🦀 ","preedit_range":null,"selection_anchor":11,"selection_cursor":11,"value":"中文🦀 "},"readonly":{"composing":false,"display_value":"locked","preedit_range":null,"selection_anchor":6,"selection_cursor":6,"value":"locked"}},"version":1}}"##,
    ];

    #[test]
    fn bridge_adapter_output_is_byte_identical_to_origin_main() {
        let lines = script();
        if std::env::var_os("SURFACE_PRINT_BRIDGE_FIXTURES").is_some() {
            for line in &lines {
                println!("FIXTURE {line}");
            }
        }
        assert_eq!(lines.len(), FIXTURES.len());
        // The fixtures are from the default build, where serde_json sorts
        // object keys. A workspace build with all features turns on
        // serde_json `preserve_order`; there key order follows insertion
        // for old and new code alike, so compare both sides with sorted keys.
        let sorted = !serde_json::to_string(&json!({"b":0,"a":0}))
            .unwrap()
            .starts_with("{\"b\"");
        for (index, (line, fixture)) in lines.iter().zip(FIXTURES).enumerate() {
            if sorted {
                assert_eq!(line, fixture, "response {}", index + 1);
            } else {
                assert_eq!(
                    canonical(line),
                    canonical(fixture),
                    "response {}",
                    index + 1
                );
            }
        }
    }
    /// Compact JSON with object keys in sorted order.
    fn canonical(line: &str) -> String {
        fn sort(value: Value) -> Value {
            match value {
                Value::Object(map) => {
                    let mut entries: Vec<_> = map.into_iter().collect();
                    entries.sort_by(|a, b| a.0.cmp(&b.0));
                    Value::Object(entries.into_iter().map(|(k, v)| (k, sort(v))).collect())
                }
                Value::Array(items) => Value::Array(items.into_iter().map(sort).collect()),
                other => other,
            }
        }
        serde_json::to_string(&sort(serde_json::from_str(line).unwrap())).unwrap()
    }
}
