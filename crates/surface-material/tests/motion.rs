use surface::{Easing, MicroDom, Node, NodeId, Role, Transition};
use surface_app::{Application, Runtime, View};
use surface_material::motion::{duration, easing, STATE_CHANGE};
use surface_material::{ChipKind, ChoiceState, Theme};

#[test]
fn motion_tokens_match_androidx() {
    for (actual, expected) in [
        (duration::SHORT1, 50.),
        (duration::SHORT2, 100.),
        (duration::SHORT3, 150.),
        (duration::SHORT4, 200.),
        (duration::MEDIUM1, 250.),
        (duration::MEDIUM2, 300.),
        (duration::MEDIUM3, 350.),
        (duration::MEDIUM4, 400.),
        (duration::LONG1, 450.),
        (duration::LONG2, 500.),
        (duration::LONG3, 550.),
        (duration::LONG4, 600.),
        (duration::EXTRA_LONG1, 700.),
        (duration::EXTRA_LONG2, 800.),
        (duration::EXTRA_LONG3, 900.),
        (duration::EXTRA_LONG4, 1000.),
    ] {
        assert_eq!(actual, expected);
    }
    for (actual, expected) in [
        (easing::EMPHASIZED, Easing::CubicBezier(0.2, 0., 0., 1.)),
        (
            easing::EMPHASIZED_DECELERATE,
            Easing::CubicBezier(0.05, 0.7, 0.1, 1.),
        ),
        (
            easing::EMPHASIZED_ACCELERATE,
            Easing::CubicBezier(0.3, 0., 0.8, 0.15),
        ),
        (easing::STANDARD, Easing::CubicBezier(0.2, 0., 0., 1.)),
        (
            easing::STANDARD_DECELERATE,
            Easing::CubicBezier(0., 0., 0., 1.),
        ),
        (
            easing::STANDARD_ACCELERATE,
            Easing::CubicBezier(0.3, 0., 1., 1.),
        ),
        (easing::LEGACY, Easing::CubicBezier(0.4, 0., 0.2, 1.)),
        (
            easing::LEGACY_DECELERATE,
            Easing::CubicBezier(0., 0., 0.2, 1.),
        ),
        (
            easing::LEGACY_ACCELERATE,
            Easing::CubicBezier(0.4, 0., 1., 1.),
        ),
        (easing::LINEAR, Easing::Linear),
    ] {
        assert_eq!(actual, expected);
        assert!(actual.is_valid(), "{actual:?}");
    }
    assert_eq!(
        STATE_CHANGE,
        Transition::new(200., easing::STANDARD),
        "the state change transition is a Surface choice"
    );
}

#[derive(Clone)]
enum Message {
    Noop,
}

struct App {
    theme: Theme,
}

impl Application for App {
    type Message = Message;
    fn update(&mut self, _: Message) {}
    fn view(&self) -> View<Message> {
        let t = &self.theme;
        let selected = ChoiceState {
            selected: true,
            disabled: false,
        };
        View::new(Node::new(Role::Window, "Components").id("root")).children([
            t.button("save", "Save", Message::Noop),
            t.checkbox("agree", "Agree", false, Message::Noop),
            t.radio("choice", "Choice", selected, Message::Noop),
            t.switch("wifi", "Wi-Fi", selected, Message::Noop),
            t.tab("tab-a", "A", selected, Message::Noop),
            t.tab("tab-b", "B", ChoiceState::default(), Message::Noop),
            t.segmented_button("seg", "Day", ChoiceState::default(), Message::Noop),
            t.chip(
                "chip",
                "Filter",
                ChipKind::Filter,
                ChoiceState::default(),
                Message::Noop,
                None,
            ),
            t.divider("divider"),
            t.menu_item("item", "Item", false, Message::Noop),
            t.outlined_card("card", []),
        ])
    }
}

fn node(dom: &MicroDom, id: &str) -> NodeId {
    dom.by_id(id).unwrap_or_else(|| panic!("{id} exists"))
}

fn child(dom: &MicroDom, parent: NodeId, index: usize) -> NodeId {
    dom.children(parent)
        .nth(index)
        .unwrap_or_else(|| panic!("child {index} exists"))
}

fn transition(dom: &MicroDom, id: NodeId) -> Option<Transition> {
    dom.get(id).unwrap().style().transition
}

#[test]
fn components_use_state_change_transition() {
    let runtime = Runtime::new(App {
        theme: Theme::light(),
    })
    .unwrap();
    let dom = runtime.dom();
    let animated = Some(STATE_CHANGE);

    // Button: the filled visual container, not the touch target or the label.
    let save = node(dom, "save");
    assert_eq!(transition(dom, save), None, "button touch target");
    let visual = child(dom, save, 0);
    assert_eq!(transition(dom, visual), animated, "button visual container");
    assert_eq!(transition(dom, child(dom, visual, 0)), None, "button label");

    // Checkbox: the box, not the control or the label.
    let agree = node(dom, "agree");
    assert_eq!(transition(dom, agree), None, "checkbox control");
    assert_eq!(
        transition(dom, child(dom, agree, 0)),
        animated,
        "checkbox box"
    );
    assert_eq!(
        transition(dom, child(dom, agree, 1)),
        None,
        "checkbox label"
    );

    // Radio: the outer ring and the dot.
    let choice = node(dom, "choice");
    assert_eq!(transition(dom, choice), None, "radio control");
    let ring = child(dom, choice, 0);
    assert_eq!(transition(dom, ring), animated, "radio ring");
    assert_eq!(transition(dom, child(dom, ring, 0)), animated, "radio dot");
    assert_eq!(transition(dom, child(dom, choice, 1)), None, "radio label");

    // Switch: the track and the thumb.
    let wifi = node(dom, "wifi");
    assert_eq!(transition(dom, wifi), None, "switch control");
    let track = child(dom, wifi, 0);
    assert_eq!(transition(dom, track), animated, "switch track");
    assert_eq!(
        transition(dom, child(dom, track, 0)),
        animated,
        "switch thumb"
    );
    assert_eq!(transition(dom, child(dom, wifi, 1)), None, "switch label");

    // Tab: only the active indicator, a child that exists when selected.
    let tab = node(dom, "tab-a");
    assert_eq!(transition(dom, tab), None, "tab");
    assert_eq!(transition(dom, child(dom, tab, 0)), None, "tab label");
    assert_eq!(
        transition(dom, child(dom, tab, 1)),
        animated,
        "tab indicator"
    );
    let inactive = node(dom, "tab-b");
    assert_eq!(
        dom.children(inactive).count(),
        1,
        "no indicator when unselected"
    );
    assert_eq!(transition(dom, inactive), None, "unselected tab");

    // Segmented button and chip: the id'd container itself.
    for id in ["seg", "chip"] {
        let container = node(dom, id);
        assert_eq!(transition(dom, container), animated, "{id}");
        assert_eq!(
            transition(dom, child(dom, container, 0)),
            None,
            "{id} label"
        );
    }

    // Nothing else changes: nine nodes animate in this view.
    let animated_count = dom
        .iter()
        .filter(|(_, n)| n.style().transition.is_some())
        .count();
    assert_eq!(animated_count, 9, "only the listed nodes animate");
}
