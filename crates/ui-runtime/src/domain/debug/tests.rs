use std::any::Any;
use std::rc::Rc;

use surface::{Component, Element};

use crate::{debug_snapshot_fibers, debug_snapshot_hooks, mount, use_state, FiberId};

fn counter(_: &Rc<dyn Any>) -> Element {
    let (count, _set) = use_state(3_i64);
    Element::from_number(count)
}

#[test]
fn debug_snapshots_address_fibers_by_fiber_id() {
    let handle = mount(Component::new("Counter", counter, Rc::new(())));

    let fibers = debug_snapshot_fibers();
    let meta = fibers
        .iter()
        .find(|f| f.id == handle.fiber_id)
        .expect("the mounted fiber is listed");
    assert_eq!(meta.hook_count, 1);
    assert!(!meta.dirty);

    let hooks = debug_snapshot_hooks(handle.fiber_id);
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].kind, "State");
    assert_eq!(hooks[0].type_name, Some("i64"));
    assert_eq!(hooks[0].value_json, Some(serde_json::json!(3)));

    let raw = FiberId::new(handle.fiber_id.get());
    assert_eq!(debug_snapshot_hooks(raw).len(), 1);
    assert!(debug_snapshot_hooks(FiberId::new(u64::MAX)).is_empty());
}
