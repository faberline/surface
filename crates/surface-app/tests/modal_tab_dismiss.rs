use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Event, Key, KeyPress, Modifiers, Runtime, View};

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
        let container = View::new(Node::new(container_role, "Container").id("container")).children(
            [
                control(Role::Button, "before", Message::Run),
                control(Role::Button, "opener", Message::Open).menu_expanded(self.open),
                control(Role::Button, "after", Message::Run),
            ]
            .into_iter()
            .chain(menu),
        );
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
    View::new(
        Node::new(role, id)
            .id(id)
            .styled(Style::row().height(Length::Px(32.))),
    )
    .on_press(message)
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
