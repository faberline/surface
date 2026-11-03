use std::{any::Any, rc::Rc};
use surface::{Callback, Component, Element, Props};
use ui_runtime::{mount, use_state};

fn counter(_: &Rc<dyn Any>) -> Element {
    let (count, set_count) = use_state(0u32);
    Element::intrinsic(
        "button",
        Props::default()
            .with_id("counter")
            .with_on_click(Callback::new(move |()| set_count.set(count + 1))),
        vec![Element::text(count.to_string())],
    )
}
#[test]
fn imported_hook_api_updates_its_own_component_without_cross_mount_state() {
    let a = mount(Component::new("Counter", counter, Rc::new(())));
    let b = mount(Component::new("Counter", counter, Rc::new(())));
    assert_eq!(a.snapshot().text_content(), "0");
    a.snapshot().find_on_click("counter").unwrap().call(());
    assert_eq!(
        a.snapshot().text_content(),
        "0",
        "writes schedule work rather than rendering in the event callback"
    );
    assert!(a.flush());
    assert_eq!(a.snapshot().text_content(), "1");
    assert!(!a.flush());
    assert!(!b.flush());
    assert_eq!(b.snapshot().text_content(), "0");
}
