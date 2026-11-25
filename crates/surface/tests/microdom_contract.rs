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
fn grid_multiselection_is_opt_in_and_keeps_the_existing_snapshot_version() {
    let grid = Node::new(Role::Group, "June").grid_role(GridRole::Grid);
    let ordinary = serde_json::to_value(MicroDom::new(grid.clone()).snapshot()).unwrap();
    assert!(ordinary["nodes"][0].get("grid_multiselectable").is_none());
    let restored = MicroDom::from_snapshot(serde_json::from_value(ordinary).unwrap()).unwrap();
    assert!(!restored
        .get(restored.root())
        .unwrap()
        .is_grid_multiselectable());

    let range_grid = grid.grid_multiselectable(true);
    let json = serde_json::to_value(MicroDom::new(range_grid).snapshot()).unwrap();
    assert_eq!(json["version"], 1);
    assert_eq!(json["nodes"][0]["grid_multiselectable"], true);
    let restored = MicroDom::from_snapshot(serde_json::from_value(json).unwrap()).unwrap();
    let root = restored.get(restored.root()).unwrap();
    assert_eq!(root.role(), Role::Group);
    assert!(root.is_grid_multiselectable());
    assert!(!root
        .clone()
        .grid_multiselectable(false)
        .is_grid_multiselectable());

    for role in [GridRole::Row, GridRole::ColumnHeader, GridRole::GridCell] {
        assert!(!root.clone().grid_role(role).is_grid_multiselectable());
    }
    assert!(!Node::new(Role::Button, "June 1")
        .grid_role(GridRole::Grid)
        .grid_multiselectable(true)
        .is_grid_multiselectable());
}

#[test]
fn web_listbox_round_trip_preserves_checked_radios_and_legacy_json() {
    let root = Node::new(Role::RadioGroup, "Delivery");
    let option = Node::new(Role::Radio, "Standard").checked(true);
    let plain = serde_json::to_value(&option).unwrap();
    assert!(plain.get("web_listbox").is_none());
    assert!(!serde_json::from_value::<Node>(plain)
        .unwrap()
        .uses_web_listbox_semantics());

    let mut dom = MicroDom::new(root.web_listbox_semantics());
    let row = dom
        .append(dom.root(), option.web_listbox_semantics())
        .unwrap();
    let json = serde_json::to_value(dom.snapshot()).unwrap();
    assert_eq!(json["version"], 1);
    assert_eq!(json["nodes"][0]["web_listbox"], true);
    assert_eq!(json["nodes"][row.index()]["web_listbox"], true);
    let restored = MicroDom::from_snapshot(serde_json::from_value(json).unwrap()).unwrap();
    let parent = restored.get(restored.root()).unwrap();
    assert_eq!(parent.role(), Role::RadioGroup);
    assert!(parent.uses_web_listbox_semantics());
    assert!(!parent.is_focusable());
    let radio = restored.get(row).unwrap();
    assert_eq!(radio.role(), Role::Radio);
    assert!(radio.uses_web_listbox_semantics());
    assert!(radio.is_focusable());
    assert_eq!(radio.is_checked(), Some(true));
    assert_eq!(radio.is_selected(), None);

    let unrelated = Node::new(Role::List, "Routes").web_listbox_semantics();
    let mut json = serde_json::to_value(&unrelated).unwrap();
    assert!(json.get("web_listbox").is_none());
    json["web_listbox"] = serde_json::Value::Bool(true);
    assert!(!serde_json::from_value::<Node>(json)
        .unwrap()
        .uses_web_listbox_semantics());
}

#[test]
fn parent_corners_are_opt_in_and_spring_state_stays_live_only() {
    let plain = Node::new(Role::Group, "Face");
    let json = serde_json::to_value(&plain).unwrap();
    assert!(json.get("interaction_corners_from_parent").is_none());
    assert!(json.get("interaction_corner_spring").is_none());
    assert_eq!(serde_json::from_value::<Node>(json).unwrap(), plain);

    let live = plain
        .id("face")
        .interaction_corners(None, Some(Corners::all(8.)), None)
        .interaction_corners_from_parent()
        .animate_interaction_corners_with_spring(800., 0.6);
    assert_eq!(live.interaction_corner_spring(), Some([800., 0.6]));
    let json = serde_json::to_value(&live).unwrap();
    assert_eq!(json["interaction_corners_from_parent"], true);
    assert!(json.get("interaction_corner_spring").is_none());
    let restored: Node = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(restored.interaction_corner_spring(), None);
    assert_eq!(serde_json::to_value(restored).unwrap(), json);
}

#[test]
fn slider_value_text_is_optional_and_preserves_the_numeric_range() {
    let range = RangeValue {
        min: 0.,
        max: 100.,
        value: 25.,
        step: 5.,
    };
    let slider = Node::new(Role::Slider, "音量").id("volume").range(range);
    let ordinary = serde_json::to_value(&slider).unwrap();
    assert!(ordinary.get("slider_value_text").is_none());
    assert_eq!(serde_json::from_value::<Node>(ordinary).unwrap(), slider);

    let json = serde_json::to_value(slider.with_slider_value_text("25％")).unwrap();
    let restored: Node = serde_json::from_value(json).unwrap();
    assert_eq!(restored.range_value(), Some(range));
    assert_eq!(restored.slider_value_text(), Some("25％"));
    assert_eq!(
        restored.with_slider_value_text("  ").slider_value_text(),
        None
    );
    assert_eq!(
        Node::new(Role::Progress, "進度")
            .range(range)
            .with_slider_value_text("25％")
            .slider_value_text(),
        None
    );
}

#[test]
fn vertical_slider_orientation_is_role_checked_and_omits_the_legacy_default() {
    let slider = Node::new(Role::Slider, "Volume").id("volume");
    let old_wire = serde_json::to_string(&slider).unwrap();
    assert!(!old_wire.contains("slider_vertical"));
    let old: Node = serde_json::from_str(&old_wire).unwrap();
    assert!(!old.is_vertical_slider());

    let authored = old.with_slider_vertical(true);
    let wire = serde_json::to_value(authored).unwrap();
    assert_eq!(wire["slider_vertical"], true);
    let restored: Node = serde_json::from_value(wire).unwrap();
    assert!(restored.is_vertical_slider());
    assert_eq!(
        serde_json::to_string(&restored.with_slider_vertical(false)).unwrap(),
        old_wire
    );

    let progress = Node::new(Role::Progress, "Progress").with_slider_vertical(true);
    assert!(!progress.is_vertical_slider());
    let mut wire = serde_json::to_value(progress).unwrap();
    assert!(wire.get("slider_vertical").is_none());
    wire["slider_vertical"] = true.into();
    let invalid: Node = serde_json::from_value(wire).unwrap();
    assert!(!invalid.is_vertical_slider());
}

#[test]
fn web_multiselect_listbox_round_trip_keeps_base_roles_and_checked_values() {
    let ordinary = Node::new(Role::Checkbox, "Coffee").checked(true);
    let json = serde_json::to_value(&ordinary).unwrap();
    assert!(json.get("web_multiselect_listbox").is_none());
    assert!(!serde_json::from_value::<Node>(json)
        .unwrap()
        .uses_web_multiselect_listbox_semantics());

    let mut dom =
        MicroDom::new(Node::new(Role::Group, "Drinks").web_multiselect_listbox_semantics());
    let coffee = dom
        .append(dom.root(), ordinary.web_multiselect_listbox_semantics())
        .unwrap();
    let tea = dom
        .append(
            dom.root(),
            Node::new(Role::Checkbox, "Tea")
                .checked(false)
                .disabled(true)
                .web_multiselect_listbox_semantics(),
        )
        .unwrap();
    let json = serde_json::to_value(dom.snapshot()).unwrap();
    assert_eq!(json["version"], 1);
    assert_eq!(json["nodes"][0]["web_multiselect_listbox"], true);
    let restored = MicroDom::from_snapshot(serde_json::from_value(json).unwrap()).unwrap();
    let group = restored.get(restored.root()).unwrap();
    assert_eq!(group.role(), Role::Group);
    assert!(group.uses_web_multiselect_listbox_semantics());
    assert!(!group.is_focusable());
    for (id, checked, disabled) in [(coffee, true, false), (tea, false, true)] {
        let node = restored.get(id).unwrap();
        assert_eq!(node.role(), Role::Checkbox);
        assert!(node.uses_web_multiselect_listbox_semantics());
        assert!(!node.uses_web_listbox_semantics());
        assert_eq!(node.is_checked(), Some(checked));
        assert_eq!(node.is_selected(), None);
        assert_eq!(node.is_disabled(), disabled);
    }

    let radio = Node::new(Role::Radio, "Choice")
        .web_listbox_semantics()
        .web_multiselect_listbox_semantics();
    let mut json = serde_json::to_value(&radio).unwrap();
    assert!(json.get("web_multiselect_listbox").is_none());
    json["web_multiselect_listbox"] = serde_json::Value::Bool(true);
    let radio: Node = serde_json::from_value(json).unwrap();
    assert!(radio.uses_web_listbox_semantics());
    assert!(!radio.uses_web_multiselect_listbox_semantics());
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
