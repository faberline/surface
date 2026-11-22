use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Event, Key, KeyPress, Modifiers, Roving, Runtime, View};

#[derive(Clone, Copy)]
enum Message {
    Open,
    Dismiss,
    CloseContaining,
    Run,
}

#[derive(Default)]
struct App {
    opt_in: bool,
    open: bool,
    nested: bool,
    retain_menu: bool,
    opener_without_tab_stop: bool,
    collapsed_neighbors: bool,
    mount_on_close: bool,
    roving_after: bool,
    dismissals: usize,
    activations: usize,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.open = true,
            Message::Dismiss => {
                self.dismissals += 1;
                self.open = self.retain_menu;
            }
            Message::CloseContaining => self.nested = false,
            Message::Run => self.activations += 1,
        }
    }

    fn view(&self) -> View<Message> {
        let menu = self.open.then(|| {
            let menu = View::new(Node::new(Role::Menu, "Commands").id("menu"))
                .children([
                    control(Role::MenuItem, "first-item", Message::Run),
                    control(Role::MenuItem, "last-item", Message::Run),
                ])
                .modal_overlay(Message::Dismiss);
            if self.opt_in {
                menu.dismiss_modal_on_tab()
            } else {
                menu
            }
        });
        let container_role = if self.nested {
            Role::Dialog
        } else {
            Role::Group
        };
        let opener = if self.opener_without_tab_stop {
            Node::new(Role::Button, "opener").id("opener").tab_index(-1)
        } else {
            Node::new(Role::Button, "opener").id("opener")
        };
        let mut controls = vec![
            control(Role::Button, "first", Message::Run),
            control(Role::Button, "before", Message::Run),
        ];
        if self.collapsed_neighbors {
            controls.push(collapsed("collapsed-before"));
        }
        controls.push(button(opener, Message::Open).menu_expanded(self.open));
        if self.collapsed_neighbors {
            controls.push(collapsed("collapsed-after"));
        }
        if self.mount_on_close && self.dismissals > 0 {
            controls.push(control(Role::Button, "mounted", Message::Run));
        }
        let after = control(Role::Button, "after", Message::Run);
        if self.roving_after {
            let collapsed = Node::new(Role::Button, "Hidden selection")
                .id("collapsed-selection")
                .selected(true)
                .styled(Style::row().height(Length::Px(0.)));
            controls.push(
                View::new(Node::new(Role::Group, "Choices").id("choices"))
                    .roving(Roving::horizontal(false))
                    .children([View::new(collapsed), after]),
            );
        } else {
            controls.push(after);
        }
        controls.push(control(Role::Button, "last", Message::Run));
        let container = View::new(Node::new(container_role, "Container").id("container"))
            .children(controls.into_iter().chain(menu));
        let container = if self.nested {
            container
                .modal_overlay(Message::CloseContaining)
                .dismiss_modal_on_tab()
        } else {
            container
        };
        View::new(Node::new(Role::Window, "Test"))
            .children([control(Role::Button, "outside", Message::Run), container])
    }
}

fn control(role: Role, id: &str, message: Message) -> View<Message> {
    button(Node::new(role, id).id(id), message)
}

fn button(node: Node, message: Message) -> View<Message> {
    View::new(node.styled(Style::row().height(Length::Px(32.)))).on_press(message)
}

fn collapsed(id: &str) -> View<Message> {
    View::new(
        Node::new(Role::Button, id)
            .id(id)
            .styled(Style::row().height(Length::Px(0.))),
    )
    .on_press(Message::Run)
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn open(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    assert!(runtime.activate("opener").unwrap());
    assert_eq!(runtime.focused(), Some("first-item"));
    runtime.render(320., 240., &mut Metrics).unwrap();
    runtime
}

#[test]
fn a_projected_tab_dismisses_the_menu_and_resumes_from_its_opener() {
    for (reverse, next) in [(false, "after"), (true, "before")] {
        let mut runtime = open(App {
            opt_in: true,
            ..App::default()
        });
        let source = runtime
            .accessibility_frame()
            .unwrap()
            .focus_target(runtime.dom().by_id("first-item").unwrap())
            .unwrap();
        let press = KeyPress::new(
            Key::Tab,
            Modifiers {
                shift: reverse,
                ..Modifiers::default()
            },
        );
        let dispatch = Event::FocusNext { reverse };
        assert!(runtime
            .accessibility_key(&source, &press, dispatch)
            .unwrap());
        assert_eq!(runtime.focused(), Some(next));
        assert!(!runtime.app().open);
        assert_eq!(runtime.app().dismissals, 1);
        assert_eq!(runtime.app().activations, 0);
    }
}

#[test]
fn a_nested_close_traverses_the_containing_scope_without_dismissing_it() {
    let mut runtime = open(App {
        opt_in: true,
        nested: true,
        ..App::default()
    });
    assert!(runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap());
    assert!(!runtime.app().open);
    assert!(runtime.app().nested);
    assert_eq!(runtime.focused(), Some("after"));
    assert_eq!(runtime.app().dismissals, 1);
    assert!(runtime.activate("outside").is_err());
    assert_eq!(runtime.app().activations, 0);
}

#[test]
fn dismissing_tabs_from_the_position_of_a_programmatic_only_opener() {
    for (reverse, next) in [(false, "after"), (true, "before")] {
        let mut runtime = open(App {
            opt_in: true,
            nested: true,
            opener_without_tab_stop: true,
            ..App::default()
        });
        assert!(runtime.dispatch(Event::FocusNext { reverse }).unwrap());
        assert_eq!(runtime.focused(), Some(next));
        assert!(runtime.app().nested);
        assert_eq!(runtime.app().dismissals, 1);
    }
}

#[test]
fn dismissing_tab_skips_previously_measured_zero_extent() {
    for (reverse, mount_on_close, next) in [
        (false, false, "after"),
        (true, false, "before"),
        (false, true, "mounted"),
    ] {
        let mut runtime = open(App {
            opt_in: true,
            collapsed_neighbors: true,
            mount_on_close,
            ..App::default()
        });
        let layout = runtime.snapshot().layout.unwrap();
        for key in ["collapsed-before", "collapsed-after"] {
            let id = runtime.dom().by_id(key).unwrap();
            assert_eq!(layout.nodes[id.index()].bounds.height, 0.);
        }
        assert!(runtime.dispatch(Event::FocusNext { reverse }).unwrap());
        assert_eq!(runtime.focused(), Some(next));
        assert_eq!(runtime.app().activations, 0);
    }
}

#[test]
fn dismissing_tab_enters_a_roving_group_after_skipping_its_collapsed_selection() {
    let mut runtime = open(App {
        opt_in: true,
        roving_after: true,
        ..App::default()
    });
    assert!(runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap());
    assert_eq!(runtime.focused(), Some("after"));
    assert_eq!(runtime.app().activations, 0);
}

#[test]
fn a_default_modal_traps_tab_and_an_ignored_dismiss_stays_focused() {
    for (opt_in, retain_menu, next, dismissals) in [
        (false, false, "last-item", 0),
        (true, true, "first-item", 1),
    ] {
        let mut runtime = open(App {
            opt_in,
            retain_menu,
            ..App::default()
        });
        assert!(runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap());
        assert!(runtime.app().open);
        assert_eq!(runtime.focused(), Some(next));
        assert_eq!(runtime.app().dismissals, dismissals);
        assert_eq!(runtime.app().activations, 0);
    }
}

#[cfg(feature = "devtools")]
#[test]
fn semantic_tab_in_both_directions_returns_the_next_control() {
    use serde_json::json;
    use surface_devtools_protocol::{Request, VERSION};

    for (reverse, next) in [(false, "after"), (true, "before")] {
        let mut runtime = open(App {
            opt_in: true,
            ..App::default()
        });
        let reply = runtime.execute_devtools(Request {
            v: VERSION,
            id: 1,
            method: "Action.key".into(),
            params: json!({"key":"tab", "extend": reverse}),
        });
        assert!(reply.ok, "{:?}", reply.error);
        assert_eq!(
            reply.result.unwrap(),
            json!({"changed": true, "focused": next})
        );
        assert!(!runtime.app().open);
        assert_eq!(runtime.app().dismissals, 1);
        assert_eq!(runtime.app().activations, 0);
    }
}
