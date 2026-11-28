// Copyright 2022 The AccessKit Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0 (found in
// the LICENSE-APACHE file) or the MIT license (found in
// the LICENSE-MIT file), at your option.

use accesskit::{
    Action, ActionHandler, ActionRequest, ActivationHandler, Node, NodeId, Role, Toggled, TreeId,
    TreeInfo, TreeUpdate,
};
use accesskit_macos::Adapter;
use objc2::{
    msg_send, msg_send_id,
    rc::{Id, autoreleasepool},
    sel,
};
use objc2_app_kit::NSView;
use objc2_foundation::{MainThreadMarker, NSArray, NSNumber, NSObject, NSObjectProtocol, NSString};

struct InitialTree(Option<TreeUpdate>);

impl ActivationHandler for InitialTree {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.0.take()
    }
}

struct NoActions;

impl ActionHandler for NoActions {
    fn do_action(&mut self, _request: ActionRequest) {
        panic!("reading attributes must not send an action");
    }
}

fn check_mark(
    view: &NSView,
    role: Role,
    toggled: Option<(Toggled, i32)>,
    disabled: bool,
    expected_attribute: bool,
    expected_mark: Option<&str>,
) {
    let id = NodeId(1);
    let mut node = Node::new(role);
    node.set_label("Choice");
    if let Some((state, _)) = toggled {
        node.set_toggled(state);
    }
    if disabled {
        node.set_disabled();
    } else {
        node.add_action(Action::Click);
    }
    let mut initial = InitialTree(Some(TreeUpdate {
        nodes: vec![(id, node)],
        tree: Some(TreeInfo::new(id)),
        tree_id: TreeId::ROOT,
        focus: id,
    }));
    // The detached view lives through the adapter and all its AppKit queries.
    let mut adapter =
        unsafe { Adapter::new((view as *const NSView).cast_mut().cast(), false, NoActions) };
    let children = adapter.view_children(&mut initial);
    // The adapter returns an autoreleased array which remains valid inside main's pool.
    let item = unsafe { children.as_ref() }.unwrap().first().unwrap();
    let ax_role: Id<NSString> = unsafe { msg_send_id![item, accessibilityRole] };
    let expected_role = match role {
        Role::CheckBox => "AXCheckBox",
        Role::RadioButton => "AXRadioButton",
        _ => "AXMenuItem",
    };
    assert_eq!(ax_role.to_string(), expected_role);
    for selector in [
        sel!(accessibilityAttributeNames),
        sel!(accessibilityAttributeValue:),
    ] {
        let allowed: bool = unsafe { msg_send![item, isAccessibilitySelectorAllowed: selector] };
        assert_eq!(allowed, expected_attribute, "{role:?}, {toggled:?}");
    }
    let attribute = NSString::from_str("AXMenuItemMarkChar");
    let names: Id<NSArray<NSString>> = unsafe { msg_send_id![item, accessibilityAttributeNames] };
    let count = (0..names.len())
        .filter(|&index| names.get(index) == Some(&*attribute))
        .count();
    assert_eq!(
        count,
        usize::from(expected_attribute),
        "{role:?}, {toggled:?}"
    );

    let mark: Option<Id<NSString>> =
        unsafe { msg_send_id![item, accessibilityAttributeValue: &*attribute] };
    let mark = mark.map(|value| value.to_string());
    assert_eq!(mark.as_deref(), expected_mark, "{role:?}, {toggled:?}");

    let value: Option<Id<NSObject>> = unsafe { msg_send_id![item, accessibilityValue] };
    let number = value.as_deref().map(|value| {
        assert!(value.is_kind_of::<NSNumber>());
        let number: i32 = unsafe { msg_send![value, intValue] };
        number
    });
    assert_eq!(
        number,
        toggled.map(|(_, value)| value),
        "{role:?}, {toggled:?}"
    );
    let enabled: bool = unsafe { msg_send![item, isAccessibilityEnabled] };
    let press_allowed: bool =
        unsafe { msg_send![item, isAccessibilitySelectorAllowed: sel!(accessibilityPerformPress)] };
    assert_eq!(enabled, !disabled);
    assert_eq!(press_allowed, !disabled);
}

fn main() {
    autoreleasepool(|_| {
        let mtm = MainThreadMarker::new().expect("run on the AppKit main thread");
        let view = unsafe { NSView::new(mtm) };
        for role in [Role::MenuItemCheckBox, Role::MenuItemRadio] {
            for (state, value, mark) in [
                (Toggled::True, 1, Some("✓")),
                (Toggled::False, 0, None),
                (Toggled::Mixed, 2, None),
            ] {
                check_mark(&view, role, Some((state, value)), false, true, mark);
            }
            check_mark(&view, role, Some((Toggled::True, 1)), true, true, Some("✓"));
            check_mark(&view, role, None, false, false, None);
        }
        for role in [Role::MenuItem, Role::CheckBox, Role::RadioButton] {
            check_mark(&view, role, Some((Toggled::True, 1)), false, false, None);
        }
        check_mark(&view, Role::MenuItem, None, false, false, None);
    });
}
