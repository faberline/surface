use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use surface_devtools_protocol::*;

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let text = serde_json::to_string(value).unwrap();
    let back: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&back, value, "{text}");
}

// Row 2: pinned fixtures, compared as exact text.
const REQUEST: &str = r#"{"v":1,"id":7,"method":"Trace.read","params":{}}"#;
const REPLY_OK: &str = r#"{"id":7,"ok":true,"result":{}}"#;
const REPLY_ERROR: &str =
    r#"{"id":7,"ok":false,"error":{"code":"unknown_method","message":"no such method"}}"#;
const EVENT: &str = r#"{"event":"Trace.record","seq":3,"params":{}}"#;
const CAPABILITIES: &str = r#"{"version":1,"domains":["Devtools","Trace"],"methods":["Devtools.capabilities","Trace.read"],"limits":{"request_bytes":1048576,"response_bytes":16777216,"ring_size":4096},"schemas":["surface.dom.v1","surface.trace.v1","surface.paint.v1"]}"#;
const TRACE_RECORD: &str =
    r#"{"seq":1,"frame":0,"kind":"Input","t_ms":1.5,"dur_ms":0.25,"data":{"x":1}}"#;

fn request() -> Request {
    Request {
        v: 1,
        id: 7,
        method: "Trace.read".into(),
        params: json!({}),
    }
}
fn capabilities() -> Capabilities {
    Capabilities {
        version: 1,
        domains: vec!["Devtools".into(), "Trace".into()],
        methods: vec!["Devtools.capabilities".into(), "Trace.read".into()],
        limits: Limits {
            request_bytes: 1_048_576,
            response_bytes: 16_777_216,
            ring_size: 4096,
        },
        schemas: vec![SCHEMA_DOM.into(), SCHEMA_TRACE.into(), SCHEMA_PAINT.into()],
    }
}

#[test]
fn version_is_one() {
    assert_eq!(VERSION, 1);
}

#[test]
fn fixtures_are_exact_text() {
    assert_eq!(serde_json::to_string(&request()).unwrap(), REQUEST);
    assert_eq!(
        serde_json::to_string(&Reply::ok(7, json!({}))).unwrap(),
        REPLY_OK
    );
    assert_eq!(
        serde_json::to_string(&Reply::error(7, ErrorCode::UnknownMethod, "no such method"))
            .unwrap(),
        REPLY_ERROR
    );
    let event = Event {
        event: "Trace.record".into(),
        seq: 3,
        params: json!({}),
    };
    assert_eq!(serde_json::to_string(&event).unwrap(), EVENT);
    assert_eq!(
        serde_json::to_string(&capabilities()).unwrap(),
        CAPABILITIES
    );
    let record = TraceRecord {
        seq: 1,
        frame: 0,
        kind: TraceKind::Input,
        t_ms: 1.5,
        dur_ms: 0.25,
        data: json!({"x":1}),
    };
    assert_eq!(serde_json::to_string(&record).unwrap(), TRACE_RECORD);
}

#[test]
fn fixtures_parse_back() {
    assert_eq!(serde_json::from_str::<Request>(REQUEST).unwrap(), request());
    assert_eq!(
        serde_json::from_str::<Reply>(REPLY_OK).unwrap(),
        Reply::ok(7, json!({}))
    );
    assert_eq!(
        serde_json::from_str::<Reply>(REPLY_ERROR).unwrap(),
        Reply::error(7, ErrorCode::UnknownMethod, "no such method")
    );
    assert_eq!(
        serde_json::from_str::<Capabilities>(CAPABILITIES).unwrap(),
        capabilities()
    );
}

// Row 1: round trip of every type.
#[test]
fn every_type_round_trips() {
    round_trip(&request());
    round_trip(&Reply::ok(1, json!({"a":[1,2]})));
    round_trip(&Reply::error(2, ErrorCode::Timeout, "slow"));
    round_trip(&Event {
        event: "Trace.record".into(),
        seq: 9,
        params: json!({"k":"v"}),
    });
    round_trip(&capabilities());
    round_trip(&Capabilities::current());
    for kind in [
        TraceKind::Input,
        TraceKind::Message,
        TraceKind::Update,
        TraceKind::Rebuild,
        TraceKind::Layout,
        TraceKind::Paint,
        TraceKind::Present,
        TraceKind::Error,
    ] {
        round_trip(&TraceRecord {
            seq: 2,
            frame: 1,
            kind,
            t_ms: 0.5,
            dur_ms: 0.0,
            data: Value::Null,
        });
    }
    for code in ErrorCode::ALL {
        round_trip(&code);
    }
    for edit in [
        EditKey::Left,
        EditKey::Right,
        EditKey::Home,
        EditKey::End,
        EditKey::PageUp,
        EditKey::PageDown,
        EditKey::Up,
        EditKey::Down,
        EditKey::Backspace,
        EditKey::Delete,
        EditKey::SelectAll,
        EditKey::Undo,
        EditKey::Redo,
    ] {
        round_trip(&edit);
    }
    let ime = [
        ImeEvent::Begin,
        ImeEvent::Preedit {
            text: "ni".into(),
            selection: Some((0, 2)),
        },
        ImeEvent::Preedit {
            text: "".into(),
            selection: None,
        },
        ImeEvent::Commit { text: "你".into() },
        ImeEvent::Cancel,
    ];
    for event in &ime {
        round_trip(event);
    }
    let mut inputs = vec![
        InputEvent::PointerDown { x: 1.5, y: 2.0 },
        InputEvent::PointerUp { x: 1.5, y: 2.0 },
        InputEvent::PointerMove { x: 0.0, y: 4.25 },
        InputEvent::PointerLeave,
        InputEvent::FocusNext { reverse: true },
        InputEvent::Activate,
        InputEvent::Text { text: "a".into() },
        InputEvent::Backspace,
        InputEvent::Edit {
            key: EditKey::Left,
            extend: true,
            by_word: false,
        },
        InputEvent::InputValue {
            value: "v".into(),
            anchor: 0,
            cursor: 1,
        },
        InputEvent::Select {
            anchor: 1,
            cursor: 2,
        },
        InputEvent::Blur,
        InputEvent::Cancel,
        InputEvent::Scroll {
            x: 1.0,
            y: 2.0,
            delta_x: 0.0,
            delta_y: -8.0,
        },
        InputEvent::SetValue {
            id: "s".into(),
            value: 0.5,
        },
    ];
    for event in ime {
        inputs.push(InputEvent::Ime {
            event: event.clone(),
        });
        inputs.push(InputEvent::ImeFor {
            id: "f".into(),
            event,
        });
    }
    for event in &inputs {
        round_trip(event);
    }
}

#[test]
fn error_code_strings_are_pinned() {
    let expected = [
        "unsupported_version",
        "unknown_method",
        "invalid_params",
        "unauthorized",
        "not_ready",
        "missing_node",
        "disabled",
        "outside_modal",
        "too_large",
        "timeout",
        "action_failed",
        "host_timeout",
        "host_closed",
        "render_failed",
    ];
    assert_eq!(ErrorCode::ALL.len(), expected.len());
    for (code, name) in ErrorCode::ALL.into_iter().zip(expected) {
        assert_eq!(code.as_str(), name);
        assert_eq!(serde_json::to_string(&code).unwrap(), format!("\"{name}\""));
    }
}

#[test]
fn input_wire_form_is_tagged() {
    let text = serde_json::to_string(&InputEvent::PointerDown { x: 1.5, y: 2.0 }).unwrap();
    assert_eq!(text, r#"{"type":"pointer_down","x":1.5,"y":2.0}"#);
    let text = serde_json::to_string(&InputEvent::Ime {
        event: ImeEvent::Commit { text: "a".into() },
    })
    .unwrap();
    assert_eq!(
        text,
        r#"{"type":"ime","event":{"type":"commit","text":"a"}}"#
    );
}

#[test]
fn capabilities_current_describes_the_crate() {
    let caps = Capabilities::current();
    assert_eq!(caps.version, VERSION);
    assert_eq!(caps.limits.request_bytes, 1 << 20);
    assert_eq!(caps.limits.response_bytes, 16 << 20);
    assert_eq!(caps.limits.ring_size, 4096);
    assert_eq!(
        caps.schemas,
        ["surface.dom.v1", "surface.trace.v1", "surface.paint.v1"]
    );
    assert!(caps.methods.iter().any(|m| m == "Devtools.capabilities"));
    assert_eq!(caps.methods.len(), METHODS.len());
}

// Row 3.
#[test]
fn parse_accepts_a_known_request() {
    assert_eq!(parse_request(REQUEST).unwrap(), request());
}

#[test]
fn parse_rejects_unknown_method() {
    let reply = parse_request(r#"{"v":1,"id":9,"method":"Nope.nothing","params":{}}"#).unwrap_err();
    assert_eq!(reply.id, 9);
    assert!(!reply.ok);
    assert_eq!(reply.error.unwrap().code, ErrorCode::UnknownMethod);
}

#[test]
fn parse_rejects_bad_version() {
    for v in [0, 2, 99] {
        let line = format!(r#"{{"v":{v},"id":4,"method":"Trace.read","params":{{}}}}"#);
        let reply = parse_request(&line).unwrap_err();
        assert_eq!(reply.id, 4);
        assert_eq!(reply.error.unwrap().code, ErrorCode::UnsupportedVersion);
    }
}

#[test]
fn version_is_checked_before_method() {
    let reply = parse_request(r#"{"v":2,"id":5,"method":"Nope.nothing"}"#).unwrap_err();
    assert_eq!(reply.error.unwrap().code, ErrorCode::UnsupportedVersion);
}

#[test]
fn parse_rejects_malformed_lines() {
    for line in [
        "",
        "not json",
        "[]",
        r#"{"v":1}"#,
        r#"{"v":1,"id":"x","method":"Trace.read"}"#,
    ] {
        let reply = parse_request(line).unwrap_err();
        assert_eq!(
            reply.error.unwrap().code,
            ErrorCode::InvalidParams,
            "{line}"
        );
    }
}

// Row 4.
#[test]
fn unknown_fields_are_tolerated() {
    let request = parse_request(
        r#"{"v":1,"id":3,"method":"Trace.read","params":{"after_seq":1,"future":true},"extra":1}"#,
    )
    .unwrap();
    assert_eq!(request.params, json!({"after_seq":1,"future":true}));
    let no_params = parse_request(r#"{"v":1,"id":3,"method":"Trace.read"}"#).unwrap();
    assert_eq!(no_params.params, json!({}));
    let record: TraceRecord = serde_json::from_str(
        r#"{"seq":1,"frame":0,"kind":"Paint","t_ms":0.0,"dur_ms":0.0,"data":{},"later":1}"#,
    )
    .unwrap();
    assert_eq!(record.kind, TraceKind::Paint);
    let caps: Capabilities = serde_json::from_str(
        r#"{"version":1,"domains":[],"methods":[],"limits":{"request_bytes":1,"response_bytes":2,"ring_size":3,"more":4},"schemas":[],"more":true}"#,
    )
    .unwrap();
    assert_eq!(caps.limits.ring_size, 3);
}
