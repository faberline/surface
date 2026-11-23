use surface::*;

/// Arena order: root=0, save=1, save-copy=2, label=3.
fn sample() -> MicroDom {
    let mut dom = MicroDom::new(Node::new(Role::Window, "App").id("app"));
    let root = dom.root();
    let a = dom
        .append(root, Node::new(Role::Button, "Save").id("save"))
        .unwrap();
    dom.append(root, Node::new(Role::Button, "Save").id("save-copy"))
        .unwrap();
    dom.append(a, Node::new(Role::Text, "Label").id("label"))
        .unwrap();
    dom
}

fn id(dom: &MicroDom, stable: &str) -> NodeId {
    dom.by_id(stable).unwrap()
}

fn json_edit(edit: impl FnOnce(&mut serde_json::Value)) -> DomSnapshot {
    let mut value = serde_json::to_value(sample().snapshot()).unwrap();
    edit(&mut value);
    serde_json::from_value(value).unwrap()
}

#[test]
fn traversal_keeps_insertion_order_and_parents() {
    let dom = sample();
    let root = dom.root();
    let a = id(&dom, "save");
    let b = id(&dom, "save-copy");
    let label = id(&dom, "label");
    assert_eq!(dom.children(root).collect::<Vec<_>>(), vec![a, b]);
    assert_eq!(dom.children(a).collect::<Vec<_>>(), vec![label]);
    assert_eq!(dom.get(label).unwrap().parent(), Some(a));
    assert_eq!(dom.get(root).unwrap().parent(), None);
    let indices: Vec<usize> = dom.iter().map(|(n, _)| n.index()).collect();
    assert_eq!(indices, (0..dom.len()).collect::<Vec<_>>());
}

#[test]
fn selectors_are_deterministic() {
    let dom = sample();
    let a = id(&dom, "save");
    let b = id(&dom, "save-copy");
    assert_eq!(dom.by_id("save"), Some(a));
    assert_eq!(dom.by_id("missing"), None);
    assert_eq!(dom.by_role_name(Role::Button, "Save"), &[a, b]);
    assert!(dom.by_role_name(Role::Text, "Save").is_empty());
}

#[test]
fn errors_are_typed() {
    let mut dom = sample();
    let len = dom.len();
    let root = dom.root();
    assert_eq!(
        dom.append(root, Node::new(Role::Group, "dup").id("save")),
        Err(DomError::DuplicateId("save".to_owned()))
    );
    assert_eq!(dom.len(), len);
    let invalid: NodeId = serde_json::from_str("999").unwrap();
    assert_eq!(
        dom.append(invalid, Node::new(Role::Text, "bad")),
        Err(DomError::InvalidNode(999))
    );
    assert_eq!(dom.get(invalid).unwrap_err(), DomError::InvalidNode(999));
}

#[test]
fn snapshots_are_byte_stable_across_independent_builds() {
    let first = serde_json::to_vec(&sample().snapshot()).unwrap();
    let second = serde_json::to_vec(&sample().snapshot()).unwrap();
    assert_eq!(first, second);

    let mut third = sample();
    let root = third.root();
    third.append(root, Node::new(Role::Text, "Extra")).unwrap();
    assert_ne!(first, serde_json::to_vec(&third.snapshot()).unwrap());
}

#[test]
fn menu_popup_uses_the_existing_expanded_and_disclosure_wire_fields() {
    let button = Node::new(Role::Button, "Actions");
    let plain = serde_json::to_value(&button).unwrap();
    assert!(plain.get("expanded").is_none());
    assert!(plain.get("disclosure").is_none());

    for open in [false, true] {
        let menu = button.clone().menu_expanded(open);
        assert_eq!(menu.expanded_state(), Some(open));
        assert!(menu.has_menu_popup());
        let disclosure = button.clone().disclosure_expanded(open);
        assert_eq!(disclosure.expanded_state(), Some(open));
        assert!(!disclosure.has_menu_popup());

        let mut expected = plain.clone();
        expected["expanded"] = open.into();
        assert_eq!(serde_json::to_value(&menu).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<Node>(expected.clone()).unwrap(),
            menu
        );
        expected["disclosure"] = true.into();
        assert_eq!(serde_json::to_value(&disclosure).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<Node>(expected).unwrap(),
            disclosure
        );
    }
    let non_button = Node::new(Role::Group, "Menu").menu_expanded(true);
    assert_eq!(non_button.expanded_state(), None);
    assert!(!non_button.has_menu_popup());
}

#[test]
fn grid_metadata_round_trips_without_changing_base_roles() {
    let field = Node::new(Role::TextInput, "Date").described_by_text("date-format");
    let plain = Node::new(Role::Button, "June 1").selected(false);
    let date = plain
        .clone()
        .grid_role(GridRole::GridCell)
        .current_date(true);
    let mut dom = MicroDom::new(Node::new(Role::Group, "June").grid_role(GridRole::Grid));
    let row = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "Week").grid_role(GridRole::Row),
        )
        .unwrap();
    let day = dom.append(row, date).unwrap();
    let input = dom.append(dom.root(), field).unwrap();

    let json = serde_json::to_value(dom.snapshot()).unwrap();
    assert_eq!(json["version"], 1);
    let node = &json["nodes"][day.index()];
    assert_eq!(node["grid_role"], "GridCell");
    assert_eq!(node["current_date"], true);
    assert_eq!(node["selected"], false);
    let decoded = MicroDom::from_snapshot(serde_json::from_value(json).unwrap()).unwrap();
    let decoded_date = decoded.get(day).unwrap();
    assert_eq!(decoded_date.role(), Role::Button);
    assert_eq!(decoded_date.parent(), Some(row));
    assert!(decoded_date.is_current_date());
    assert!(decoded_date.is_focusable());
    assert_eq!(
        decoded.get(input).unwrap().description_text_target(),
        Some("date-format")
    );

    let ordinary = serde_json::to_value(&plain).unwrap();
    assert!(ordinary.get("grid_role").is_none());
    assert!(ordinary.get("current_date").is_none());
    assert!(ordinary.get("described_by_text").is_none());
    assert_eq!(
        plain
            .grid_role(GridRole::ColumnHeader)
            .accessibility_grid_role(),
        None
    );
}

#[test]
fn snapshot_has_golden_bytes() {
    let dom = MicroDom::new(Node::new(Role::Window, "App").id("app"));
    let json = serde_json::to_string(&dom.snapshot()).unwrap();
    assert!(json.starts_with(
        "{\"version\":1,\"nodes\":[{\"stable_id\":\"app\",\"role\":\"Window\",\"name\":\"App\""
    ));
}

#[test]
fn from_snapshot_round_trips() {
    let s = sample().snapshot();
    let r = MicroDom::from_snapshot(s.clone()).unwrap();
    assert_eq!(r.snapshot(), s);
    assert!(r.by_id("label").is_some());
    assert_eq!(r.by_role_name(Role::Button, "Save").len(), 2);

    let bytes = serde_json::to_vec(&s).unwrap();
    let back: DomSnapshot = serde_json::from_slice(&bytes).unwrap();
    let r2 = MicroDom::from_snapshot(back).unwrap();
    assert_eq!(serde_json::to_vec(&r2.snapshot()).unwrap(), bytes);
}

#[test]
fn from_snapshot_rejects_bad_version() {
    let mut s = sample().snapshot();
    s.version = 2;
    assert_eq!(
        MicroDom::from_snapshot(s).unwrap_err(),
        DomError::UnsupportedVersion(2)
    );
}

#[test]
fn from_snapshot_rejects_empty() {
    let s = DomSnapshot {
        version: 1,
        nodes: vec![],
    };
    assert_eq!(
        MicroDom::from_snapshot(s).unwrap_err(),
        DomError::EmptySnapshot
    );
}

#[test]
fn from_snapshot_rejects_duplicate_ids() {
    let s = json_edit(|v| v["nodes"][2]["stable_id"] = serde_json::json!("save"));
    assert_eq!(
        MicroDom::from_snapshot(s).unwrap_err(),
        DomError::DuplicateId("save".to_owned())
    );
}

#[test]
fn from_snapshot_rejects_bad_link_index() {
    let s = json_edit(|v| v["nodes"][0]["first_child"] = serde_json::json!(99));
    assert_eq!(
        MicroDom::from_snapshot(s).unwrap_err(),
        DomError::InvalidNode(99)
    );
}

#[test]
fn from_snapshot_rejects_wrong_parent() {
    let s = json_edit(|v| v["nodes"][2]["parent"] = serde_json::json!(1));
    assert_eq!(
        MicroDom::from_snapshot(s).unwrap_err(),
        DomError::InvalidTree(2)
    );
}
