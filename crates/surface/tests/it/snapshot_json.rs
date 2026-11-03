//! Pins the `SurfaceSnapshot` JSON bytes. The snapshot types are the wire
//! format that renderers, readers and parity tools exchange, so a refactor
//! must leave both the encoding and the decoding of this document unchanged.

use surface::{SurfaceNode, SurfaceNodeKind, SurfaceProps, SurfaceRect, SurfaceSnapshot};

/// `serde_json::to_string` of the snapshot that `pinned_snapshot` builds, as
/// `Element::surface_snapshot` produced it before P2 (plus one `set_bounds`).
const PINNED_JSON: &str = concat!(
    r#"{"schema_version":1,"nodes":["#,
    r#"{"node_id":"root","semantic_id":"app","parent_id":null,"kind":"element","tag":"main","component":null,"role":"main","name":null,"text":null,"#,
    r#""props":{"id":"app","class_name":"shell","style":null,"value":null,"input_type":null,"placeholder":null,"checked":null,"aria_label":null,"html_for":null,"disabled":false,"has_on_click":false,"has_on_change":false,"has_on_checked_change":false}"#,
    r#","bounds":null},"#,
    r#"{"node_id":"root/0","semantic_id":"save","parent_id":"root","kind":"element","tag":"button","component":null,"role":"button","name":"Save draft","text":null,"#,
    r#""props":{"id":"save","class_name":null,"style":"color: red","value":null,"input_type":null,"placeholder":null,"checked":null,"aria_label":"Save draft","html_for":null,"disabled":false,"has_on_click":true,"has_on_change":false,"has_on_checked_change":false}"#,
    r#","bounds":{"x":8.0,"y":12.5,"w":120.0,"h":24.0}},"#,
    r#"{"node_id":"root/0/0","semantic_id":"root/0/0","parent_id":"root/0","kind":"text","tag":null,"component":null,"role":null,"name":null,"text":"Save","#,
    r#""props":{"id":null,"class_name":null,"style":null,"value":null,"input_type":null,"placeholder":null,"checked":null,"aria_label":null,"html_for":null,"disabled":false,"has_on_click":false,"has_on_change":false,"has_on_checked_change":false}"#,
    r#","bounds":null},"#,
    r#"{"node_id":"root/1","semantic_id":"root/1","parent_id":"root","kind":"element","tag":"label","component":null,"role":"label","name":"Agree","text":null,"#,
    r#""props":{"id":null,"class_name":null,"style":null,"value":null,"input_type":null,"placeholder":null,"checked":null,"aria_label":null,"html_for":"agree","disabled":false,"has_on_click":false,"has_on_change":false,"has_on_checked_change":false}"#,
    r#","bounds":null},"#,
    r#"{"node_id":"root/1/0","semantic_id":"root/1/0","parent_id":"root/1","kind":"text","tag":null,"component":null,"role":null,"name":null,"text":"Agree","#,
    r#""props":{"id":null,"class_name":null,"style":null,"value":null,"input_type":null,"placeholder":null,"checked":null,"aria_label":null,"html_for":null,"disabled":false,"has_on_click":false,"has_on_change":false,"has_on_checked_change":false}"#,
    r#","bounds":null},"#,
    r#"{"node_id":"root/2","semantic_id":"agree","parent_id":"root","kind":"element","tag":"input","component":null,"role":"checkbox","name":null,"text":null,"#,
    r#""props":{"id":"agree","class_name":null,"style":null,"value":null,"input_type":"checkbox","placeholder":null,"checked":true,"aria_label":null,"html_for":null,"disabled":false,"has_on_click":false,"has_on_change":false,"has_on_checked_change":true}"#,
    r#","bounds":null},"#,
    r#"{"node_id":"root/3/fragment/0","semantic_id":"root/3/fragment/0","parent_id":"root","kind":"text","tag":null,"component":null,"role":null,"name":null,"text":"a","#,
    r#""props":{"id":null,"class_name":null,"style":null,"value":null,"input_type":null,"placeholder":null,"checked":null,"aria_label":null,"html_for":null,"disabled":false,"has_on_click":false,"has_on_change":false,"has_on_checked_change":false}"#,
    r#","bounds":null},"#,
    r#"{"node_id":"root/3/fragment/2","semantic_id":"root/3/fragment/2","parent_id":"root","kind":"component","tag":null,"component":"Badge","role":null,"name":"Badge","text":null,"#,
    r#""props":{"id":null,"class_name":null,"style":null,"value":null,"input_type":null,"placeholder":null,"checked":null,"aria_label":null,"html_for":null,"disabled":false,"has_on_click":false,"has_on_change":false,"has_on_checked_change":false}"#,
    r#","bounds":null},"#,
    r#"{"node_id":"root/4","semantic_id":"root/4","parent_id":"root","kind":"element","tag":"input","component":null,"role":"textbox","name":"hi","text":null,"#,
    r#""props":{"id":null,"class_name":null,"style":null,"value":"hi","input_type":null,"placeholder":"Search","checked":null,"aria_label":null,"html_for":null,"disabled":true,"has_on_click":false,"has_on_change":true,"has_on_checked_change":false}"#,
    r#","bounds":null}"#,
    r#"]}"#,
);

fn some(s: &str) -> Option<String> {
    Some(s.to_string())
}

/// A node with every optional field empty and default props.
fn node(
    node_id: &str,
    semantic_id: &str,
    parent_id: Option<&str>,
    kind: SurfaceNodeKind,
) -> SurfaceNode {
    SurfaceNode {
        node_id: node_id.to_string(),
        semantic_id: semantic_id.to_string(),
        parent_id: parent_id.map(str::to_string),
        kind,
        tag: None,
        component: None,
        role: None,
        name: None,
        text: None,
        props: SurfaceProps::default(),
        bounds: None,
    }
}

/// Nested elements, text leaves, a fragment's children, a component node,
/// every props flag and one node with bounds.
fn pinned_snapshot() -> SurfaceSnapshot {
    use SurfaceNodeKind::{Component, Element, Text};
    SurfaceSnapshot {
        schema_version: 1,
        nodes: vec![
            SurfaceNode {
                tag: some("main"),
                role: some("main"),
                props: SurfaceProps {
                    id: some("app"),
                    class_name: some("shell"),
                    ..SurfaceProps::default()
                },
                ..node("root", "app", None, Element)
            },
            SurfaceNode {
                tag: some("button"),
                role: some("button"),
                name: some("Save draft"),
                props: SurfaceProps {
                    id: some("save"),
                    style: some("color: red"),
                    aria_label: some("Save draft"),
                    has_on_click: true,
                    ..SurfaceProps::default()
                },
                bounds: Some(SurfaceRect {
                    x: 8.0,
                    y: 12.5,
                    w: 120.0,
                    h: 24.0,
                }),
                ..node("root/0", "save", Some("root"), Element)
            },
            SurfaceNode {
                text: some("Save"),
                ..node("root/0/0", "root/0/0", Some("root/0"), Text)
            },
            SurfaceNode {
                tag: some("label"),
                role: some("label"),
                name: some("Agree"),
                props: SurfaceProps {
                    html_for: some("agree"),
                    ..SurfaceProps::default()
                },
                ..node("root/1", "root/1", Some("root"), Element)
            },
            SurfaceNode {
                text: some("Agree"),
                ..node("root/1/0", "root/1/0", Some("root/1"), Text)
            },
            SurfaceNode {
                tag: some("input"),
                role: some("checkbox"),
                props: SurfaceProps {
                    id: some("agree"),
                    input_type: some("checkbox"),
                    checked: Some(true),
                    has_on_checked_change: true,
                    ..SurfaceProps::default()
                },
                ..node("root/2", "agree", Some("root"), Element)
            },
            SurfaceNode {
                text: some("a"),
                ..node("root/3/fragment/0", "root/3/fragment/0", Some("root"), Text)
            },
            SurfaceNode {
                component: some("Badge"),
                name: some("Badge"),
                ..node(
                    "root/3/fragment/2",
                    "root/3/fragment/2",
                    Some("root"),
                    Component,
                )
            },
            SurfaceNode {
                tag: some("input"),
                role: some("textbox"),
                name: some("hi"),
                props: SurfaceProps {
                    value: some("hi"),
                    placeholder: some("Search"),
                    disabled: true,
                    has_on_change: true,
                    ..SurfaceProps::default()
                },
                ..node("root/4", "root/4", Some("root"), Element)
            },
        ],
    }
}

#[test]
fn surface_snapshot_encodes_to_pinned_json() {
    let json = serde_json::to_string(&pinned_snapshot()).unwrap();
    assert_eq!(json, PINNED_JSON);
}

#[test]
fn pinned_json_decodes_to_surface_snapshot() {
    let decoded: SurfaceSnapshot = serde_json::from_str(PINNED_JSON).unwrap();
    assert_eq!(decoded, pinned_snapshot());
}
