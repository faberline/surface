use surface::{Node, Role};
use surface_app::{Application, Event, Runtime, View};

#[derive(Clone, Copy, Default)]
enum Rail {
    #[default]
    Enabled,
    Disabled,
    Removed,
}

#[derive(Clone, Copy, Default)]
enum Fab {
    #[default]
    Enabled,
    Disabled,
    Missing,
    OutsideScope,
}

#[derive(Clone, Copy)]
enum Message {
    Rail(Rail),
    Modal(bool, Rail),
    Primary,
}

#[derive(Default)]
struct App {
    rail: Rail,
    fab: Fab,
    modal: bool,
    primary_presses: usize,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Rail(rail) => self.rail = rail,
            Message::Modal(modal, rail) => {
                self.modal = modal;
                self.rail = rail;
            }
            Message::Primary => self.primary_presses += 1,
        }
    }

    fn view(&self) -> View<Message> {
        let button = |id| View::new(Node::new(Role::Button, id).id(id));
        let mut page = vec![button("outside")];
        let mut active = Vec::new();
        if self.modal {
            active.push(button("dialog-first"));
        }
        if !matches!(self.rail, Rail::Removed) {
            active.push(
                View::new(
                    Node::new(Role::Button, "Rail action")
                        .id("rail")
                        .disabled(matches!(self.rail, Rail::Disabled)),
                )
                .restore_focus_on_removal("primary"),
            );
        }
        if !matches!(self.fab, Fab::Missing) {
            let primary = View::new(
                Node::new(Role::Button, "Primary action")
                    .id("primary")
                    .disabled(matches!(self.fab, Fab::Disabled)),
            )
            .on_press(Message::Primary);
            if matches!(self.fab, Fab::OutsideScope) {
                page.push(primary);
            } else {
                active.push(primary);
            }
        }
        if self.modal {
            page.push(
                View::new(Node::new(Role::Dialog, "Dialog").id("dialog"))
                    .children(active)
                    .modal(Message::Modal(false, self.rail)),
            );
        } else {
            page.extend(active);
        }
        page.push(button("after"));
        View::new(Node::new(Role::Window, "Page")).children(page)
    }
}

#[test]
fn removed_or_disabled_action_transfers_focus_without_pressing_destination() {
    for rail in [Rail::Removed, Rail::Disabled] {
        let mut runtime = Runtime::new(App::default()).unwrap();
        runtime.activate("rail").unwrap();
        runtime.apply(Message::Rail(rail)).unwrap();
        assert_eq!(runtime.focused(), Some("primary"));
        assert_eq!(runtime.app().primary_presses, 0);

        runtime.apply(Message::Rail(Rail::Enabled)).unwrap();
        assert_eq!(runtime.focused(), Some("primary"));
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some("after"));
    }
}

#[test]
fn fallback_requires_lost_focus_and_an_available_destination() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    runtime.activate("rail").unwrap();
    runtime.apply(Message::Rail(Rail::Enabled)).unwrap();
    assert_eq!(runtime.focused(), Some("rail"));
    runtime.activate("outside").unwrap();
    runtime.apply(Message::Rail(Rail::Removed)).unwrap();
    assert_eq!(runtime.focused(), Some("outside"));

    for fab in [Fab::Disabled, Fab::Missing] {
        let mut runtime = Runtime::new(App {
            fab,
            ..App::default()
        })
        .unwrap();
        runtime.activate("rail").unwrap();
        runtime.apply(Message::Rail(Rail::Removed)).unwrap();
        assert_eq!(runtime.focused(), None);
    }
}

#[test]
fn fallback_stays_inside_modal_and_preserves_its_opener() {
    for (fab, expected) in [
        (Fab::Enabled, "primary"),
        (Fab::OutsideScope, "dialog-first"),
    ] {
        let mut runtime = Runtime::new(App {
            fab,
            ..App::default()
        })
        .unwrap();
        runtime.activate("outside").unwrap();
        runtime.apply(Message::Modal(true, Rail::Enabled)).unwrap();
        runtime.activate("rail").unwrap();
        runtime.apply(Message::Rail(Rail::Removed)).unwrap();
        assert_eq!(runtime.focused(), Some(expected));
        assert_eq!(runtime.app().primary_presses, 0);
        runtime.dispatch(Event::Cancel).unwrap();
        assert_eq!(runtime.focused(), Some("outside"));
    }
}

#[test]
fn a_new_modal_takes_focus_and_restores_the_previous_control_when_it_returns() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    runtime.activate("rail").unwrap();
    runtime.apply(Message::Modal(true, Rail::Removed)).unwrap();
    assert_eq!(runtime.focused(), Some("dialog-first"));
    runtime.apply(Message::Rail(Rail::Enabled)).unwrap();
    assert_eq!(runtime.focused(), Some("dialog-first"));
    runtime.dispatch(Event::Cancel).unwrap();
    assert_eq!(runtime.focused(), Some("rail"));
}
