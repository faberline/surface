use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use surface::{Callback, Component, Element, Props, SurfaceProps};

#[test]
fn default_props_leave_every_field_unset() {
    let props = Props::default();
    assert_eq!(props.class_name(), None);
    assert_eq!(props.style(), None);
    assert!(props.on_click().is_none());
    assert!(props.on_change().is_none());
    assert!(props.on_checked_change().is_none());
    assert_eq!(props.id(), None);
    assert_eq!(props.value(), None);
    assert_eq!(props.input_type(), None);
    assert_eq!(props.placeholder(), None);
    assert_eq!(props.checked(), None);
    assert_eq!(props.aria_label(), None);
    assert_eq!(props.html_for(), None);
    assert!(!props.disabled());
    assert_eq!(SurfaceProps::from(&props), SurfaceProps::default());
}

#[test]
fn props_builders_set_what_the_getters_read() {
    let clicks = Rc::new(Cell::new(0));
    let changed = Rc::new(Cell::new(String::new()));
    let toggled = Rc::new(Cell::new(false));
    let (c, v, t) = (clicks.clone(), changed.clone(), toggled.clone());

    let props = Props::default()
        .with_class_name("row")
        .with_style("color: red")
        .with_on_click(Callback::new(move |()| c.set(c.get() + 1)))
        .with_on_change(Callback::new(move |s: String| v.set(s)))
        .with_on_checked_change(Callback::new(move |b| t.set(b)))
        .with_id("agree")
        .with_value(String::from("yes"))
        .with_input_type("checkbox")
        .with_placeholder("Agree?")
        .with_checked(true)
        .with_aria_label("Agree to terms")
        .with_html_for("terms")
        .with_disabled(true);

    assert_eq!(props.class_name(), Some("row"));
    assert_eq!(props.style(), Some("color: red"));
    assert_eq!(props.id(), Some("agree"));
    assert_eq!(props.value(), Some("yes"));
    assert_eq!(props.input_type(), Some("checkbox"));
    assert_eq!(props.placeholder(), Some("Agree?"));
    assert_eq!(props.checked(), Some(true));
    assert_eq!(props.aria_label(), Some("Agree to terms"));
    assert_eq!(props.html_for(), Some("terms"));
    assert!(props.disabled());

    props.on_click().unwrap().call(());
    props.on_change().unwrap().call("no".to_string());
    props.on_checked_change().unwrap().call(true);
    assert_eq!(clicks.get(), 1);
    assert_eq!(changed.take(), "no");
    assert!(toggled.get());

    let surface = SurfaceProps::from(&props);
    assert_eq!(
        surface,
        SurfaceProps {
            id: Some("agree".to_string()),
            class_name: Some("row".to_string()),
            style: Some("color: red".to_string()),
            value: Some("yes".to_string()),
            input_type: Some("checkbox".to_string()),
            placeholder: Some("Agree?".to_string()),
            checked: Some(true),
            aria_label: Some("Agree to terms".to_string()),
            html_for: Some("terms".to_string()),
            disabled: true,
            has_on_click: true,
            has_on_change: true,
            has_on_checked_change: true,
        }
    );
}

fn greeting(props: &Rc<dyn Any>) -> Element {
    let who = props.downcast_ref::<String>().unwrap();
    Element::text(format!("Hello, {who}"))
}

#[test]
fn component_renders_its_props_through_its_render_fn() {
    let component = Component::new("Greeting", greeting, Rc::new("Ada".to_string()));
    assert_eq!(component.name(), "Greeting");
    assert_eq!(component.render().text_content(), "Hello, Ada");

    let snapshot = Element::Component(component).surface_snapshot();
    assert_eq!(snapshot.nodes[0].component.as_deref(), Some("Greeting"));
}
