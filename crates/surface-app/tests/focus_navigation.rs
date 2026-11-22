use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, EditKey, Event, FocusNavigation, Runtime, View};

#[derive(Clone, Copy, Default)]
enum Destination {
    #[default]
    Enabled,
    Disabled,
    OutsideModal,
}

#[derive(Clone)]
enum Message {
    Mount,
    Press,
    Close,
}

#[derive(Default)]
struct App {
    destination: Destination,
    mounted: bool,
    modal: bool,
    presses: usize,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Mount => {
                self.mounted = true;
                self.modal = matches!(self.destination, Destination::OutsideModal);
            }
            Message::Press => self.presses += 1,
            Message::Close => self.modal = false,
        }
    }

    fn view(&self) -> View<Message> {
        let start = button("start").on_focus_navigation(|key| {
            (key == EditKey::Right).then(|| FocusNavigation {
                target: "next".into(),
                message: Message::Mount,
            })
        });
        let next = self.mounted.then(|| {
            [
                View::new(
                    Node::new(Role::Group, "Spacer")
                        .styled(Style::default().height(Length::Px(100.))),
                ),
                View::new(
                    Node::new(Role::Button, "Next")
                        .id("next")
                        .disabled(matches!(self.destination, Destination::Disabled))
                        .styled(Style::default().height(Length::Px(24.))),
                )
                .on_press(Message::Press),
            ]
        });
        let body = View::new(
            Node::new(Role::Group, "Viewport")
                .id("body")
                .styled(Style::column().height(Length::Px(64.)).clip(true)),
        )
        .children(std::iter::once(start).chain(next.into_iter().flatten()));
        let modal = self.modal.then(|| {
            View::new(Node::new(Role::Dialog, "Modal").id("modal"))
                .children([button("close").on_press(Message::Close)])
                .modal_overlay(Message::Close)
        });
        View::new(Node::new(Role::Window, "Test")).children(std::iter::once(body).chain(modal))
    }
}

fn button(id: &str) -> View<Message> {
    View::new(
        Node::new(Role::Button, id)
            .id(id)
            .styled(Style::default().height(Length::Px(24.))),
    )
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime.render(200., 150., &mut Metrics).unwrap();
}

fn runtime(destination: Destination) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        destination,
        ..App::default()
    })
    .unwrap();
    runtime.activate("start").unwrap();
    render(&mut runtime);
    runtime
}

fn right(runtime: &mut Runtime<App>) -> bool {
    runtime
        .dispatch(Event::Edit {
            key: EditKey::Right,
            extend: false,
            by_word: false,
        })
        .unwrap()
}

#[test]
fn navigation_mounts_and_reveals_the_target_without_pressing_it() {
    let mut runtime = runtime(Destination::Enabled);
    assert_eq!(runtime.focused(), Some("start"));
    assert!(runtime.dom().by_id("next").is_none());
    assert!(right(&mut runtime));
    assert_eq!(runtime.focused(), Some("next"));
    assert_eq!(runtime.app().presses, 0);
    render(&mut runtime);
    assert!(runtime.scroll_metrics("body").unwrap().offset.1 > 0.);
}

#[test]
fn navigation_cannot_focus_a_disabled_target_or_cross_the_new_modal_scope() {
    for (destination, focused) in [
        (Destination::Disabled, "start"),
        (Destination::OutsideModal, "close"),
    ] {
        let mut runtime = runtime(destination);
        assert!(right(&mut runtime));
        assert!(runtime.dom().by_id("next").is_some());
        assert_eq!(runtime.focused(), Some(focused));
        assert_eq!(runtime.app().presses, 0);
    }
}
