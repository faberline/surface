use surface::{AdjustableValue, GridRole, Node, Role};

fn value() -> AdjustableValue {
    AdjustableValue {
        min: 1.,
        max: 12.,
        step: 1.,
        value: None,
        value_text: Some("未設定".into()),
    }
}

#[test]
fn opt_in_leaves_existing_nodes_and_role_wire_values_untouched() {
    let plain = Node::new(Role::Group, "Hours");
    let old = serde_json::to_value(&plain).unwrap();
    assert!(!plain.is_focusable());
    assert!(old.get("adjustable").is_none());
    assert_eq!(old["role"], "Group");
    let control = plain.adjustable(value());
    let new = serde_json::to_value(&control).unwrap();
    assert_eq!(new["role"], old["role"]);
    assert!(new["adjustable"].get("value").is_none());
    assert!(control.is_focusable());
    assert!(serde_json::from_value::<Node>(new).unwrap().is_focusable());
    assert!(!control.clone().disabled(true).is_focusable());
    assert!(!control.grid_role(GridRole::Grid).is_focusable());
    assert!(Node::new(Role::Slider, "Slider")
        .adjustable(value())
        .accessibility_adjustable()
        .is_none());
}

#[test]
fn missing_is_distinct_from_an_accepted_minimum() {
    let mut adjustable = value();
    assert_eq!(adjustable.step_value(true), Some(1.));
    assert_eq!(adjustable.step_value(false), Some(1.));
    assert_eq!(adjustable.resolve(-17.), Some(1.));
    assert_eq!(adjustable.resolve(20.), Some(12.));
    adjustable.value = Some(1.);
    assert_eq!(adjustable.step_value(false), None);
    assert_eq!(adjustable.resolve(1.1), None);
    adjustable.max = f64::NAN;
    assert!(!adjustable.is_valid());
    assert_eq!(adjustable.step_value(false), None);
}
