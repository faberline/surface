use surface::{Node, Role};

#[test]
fn tab_relations_are_additive_roles_and_optional_wire_fields() {
    let tab = Node::new(Role::Tab, "Inbox");
    let panel = Node::new(Role::Group, "Messages");
    for node in [&tab, &panel] {
        let legacy = serde_json::to_value(node).unwrap();
        assert!(legacy.get("tab_controls").is_none());
        assert!(legacy.get("tab_panel_label").is_none());
        assert_eq!(serde_json::from_value::<Node>(legacy).unwrap(), *node);
    }

    let tab = tab.tab_controls("inbox panel");
    let panel = panel.tab_panel_labelled_by("destination tabs", "inbox");
    let tab: Node = serde_json::from_value(serde_json::to_value(tab).unwrap()).unwrap();
    let panel: Node = serde_json::from_value(serde_json::to_value(panel).unwrap()).unwrap();
    assert_eq!(tab.role(), Role::Tab);
    assert_eq!(tab.tab_controls_target(), Some("inbox panel"));
    assert_eq!(panel.role(), Role::Group);
    assert_eq!(
        panel.tab_panel_label_target(),
        Some(("destination tabs", "inbox"))
    );
    assert_eq!(tab.tab_panel_label_target(), None);
    assert_eq!(panel.tab_controls_target(), None);
}

#[test]
fn wrong_roles_ignore_builders_and_authored_wire_metadata() {
    let button = Node::new(Role::Button, "Open").id("open");
    assert_eq!(
        button
            .clone()
            .tab_controls("panel")
            .tab_panel_labelled_by("list", "tab"),
        button
    );
    let mut wire = serde_json::to_value(button).unwrap();
    wire["tab_controls"] = serde_json::json!("panel");
    wire["tab_panel_label"] = serde_json::json!(["list", "tab"]);
    let restored: Node = serde_json::from_value(wire).unwrap();
    assert_eq!(restored.tab_controls_target(), None);
    assert_eq!(restored.tab_panel_label_target(), None);
}
