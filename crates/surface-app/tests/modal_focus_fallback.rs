use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};

#[derive(Clone, Copy)]
enum WidenedCommand {
    Inline,
    Removed,
    OutsideContainingModal,
}

#[derive(Clone)]
enum Message {
    Open,
    Dismiss,
    Widen,
    Run,
}

struct App {
    opt_in: bool,
    widened: WidenedCommand,
    wide: bool,
    open: bool,
    activations: usize,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.open = true,
            Message::Dismiss => self.open = false,
            Message::Widen => {
                self.wide = true;
                self.open = false;
            }
            Message::Run => self.activations += 1,
        }
    }

    fn view(&self) -> View<Message> {
        let nested = matches!(self.widened, WidenedCommand::OutsideContainingModal);
        let opener = (!self.wide).then(|| button("opener", Message::Open));
        let menu = self.open.then(|| {
            let panel = View::new(Node::new(Role::Menu, "Menu").id("menu"))
                .children([button("command", Message::Run)])
                .modal_overlay(Message::Dismiss);
            if self.opt_in {
                panel.modal_focus_fallback("first")
            } else {
                panel
            }
        });
        let inline = (self.wide && matches!(self.widened, WidenedCommand::Inline))
            .then(|| button("command", Message::Run));
        let outside = (self.wide && nested).then(|| button("command", Message::Run));
        let role = if nested { Role::Dialog } else { Role::Group };
        let container = View::new(Node::new(role, "Container").id("container")).children(
            opener
                .into_iter()
                .chain([button("first", Message::Run)])
                .chain(menu)
                .chain(inline),
        );
        let container = if nested {
            container.modal_overlay(Message::Dismiss)
        } else {
            container
        };
        View::new(Node::new(Role::Window, "Test"))
            .children(std::iter::once(container).chain(outside))
    }
}

fn button(id: &str, message: Message) -> View<Message> {
    View::new(
        Node::new(Role::Button, id)
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

fn open(opt_in: bool, widened: WidenedCommand) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        opt_in,
        widened,
        wide: false,
        open: false,
        activations: 0,
    })
    .unwrap();
    runtime.render(240., 180., &mut Metrics).unwrap();
    runtime.activate("opener").unwrap();
    assert_eq!(runtime.focused(), Some("command"));
    runtime
}

#[test]
fn a_present_opener_has_priority_over_any_fallback() {
    let mut runtime = open(true, WidenedCommand::Inline);
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert_eq!(runtime.focused(), Some("opener"));
    assert_eq!(runtime.app().activations, 0);
}

#[test]
fn opt_in_keeps_the_command_when_it_moves_inline_and_legacy_still_clears_focus() {
    for (opt_in, expect) in [(true, Some("command")), (false, None)] {
        let mut runtime = open(opt_in, WidenedCommand::Inline);
        runtime.apply(Message::Widen).unwrap();
        assert_eq!(runtime.focused(), expect);
        assert_eq!(runtime.app().activations, 0);
    }
}

#[test]
fn the_fallback_stays_in_the_containing_scope_when_the_command_is_unavailable() {
    for destination in [
        WidenedCommand::Removed,
        WidenedCommand::OutsideContainingModal,
    ] {
        let mut runtime = open(true, destination);
        runtime.apply(Message::Widen).unwrap();
        assert_eq!(runtime.focused(), Some("first"));
        assert_eq!(runtime.app().activations, 0);
    }
}
