use surface::{Node, Role};

#[test]
fn dialog_accessibility_metadata_is_opt_in_and_keeps_the_base_role() {
    let plain = Node::new(Role::Dialog, "Rename document");
    let legacy = serde_json::to_value(&plain).unwrap();
    assert_eq!(legacy["role"], "Dialog");
    assert!(legacy.get("alert_dialog").is_none());
    assert!(legacy.get("dialog_heading").is_none());
    let restored: Node = serde_json::from_value(legacy).unwrap();
    assert_eq!(restored, plain);
    assert!(!restored.is_alert_dialog());
    assert_eq!(restored.dialog_heading_target(), None);

    let alert = plain.alert_dialog().dialog_labelled_by_heading("heading");
    let restored: Node = serde_json::from_value(serde_json::to_value(&alert).unwrap()).unwrap();
    assert_eq!(restored.role(), Role::Dialog);
    assert!(restored.is_alert_dialog());
    assert_eq!(restored.dialog_heading_target(), Some("heading"));

    let button = Node::new(Role::Button, "Open dialog");
    assert_eq!(
        button
            .clone()
            .alert_dialog()
            .dialog_labelled_by_heading("heading"),
        button
    );
}
