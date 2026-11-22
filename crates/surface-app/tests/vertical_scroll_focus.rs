use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Event, Runtime, VerticalScrollResponse, View};

#[derive(Clone)]
enum Message {
    Focus(Option<String>),
}

#[derive(Default)]
struct App {
    seen: Vec<Option<String>>,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        let Message::Focus(focused) = message;
        self.seen.push(focused);
    }

    fn view(&self) -> View<Message> {
        let toolbar = View::new(
            Node::new(Role::Button, "Action")
                .id("toolbar-action")
                .styled(Style::default().height(Length::Px(48.))),
        );
        let body = View::new(
            Node::new(Role::Group, "Body")
                .id("body")
                .styled(Style::column().height(Length::Px(100.)).clip(true)),
        )
        .children([View::new(
            Node::new(Role::Group, "Content").styled(Style::default().height(Length::Px(300.))),
        )])
        .on_vertical_scroll_with_focus(|_, focus| {
            Some(VerticalScrollResponse {
                consumed: 0.,
                message: Message::Focus(focus.map(str::to_owned)),
            })
        });
        View::new(Node::new(Role::Window, "Test").styled(Style::column())).children([toolbar, body])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

#[test]
fn a_scroll_hook_sees_current_focus_even_outside_the_scroll_view() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    let event = Event::Scroll {
        x: 20.,
        y: 70.,
        delta_x: 0.,
        delta_y: 20.,
    };
    runtime.render(240., 180., &mut Metrics).unwrap();
    assert!(runtime.dispatch(event.clone()).unwrap());
    assert!(runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap());
    runtime.render(240., 180., &mut Metrics).unwrap();
    assert!(runtime.dispatch(event).unwrap());
    runtime.render(240., 180., &mut Metrics).unwrap();

    assert_eq!(runtime.app().seen, [None, Some("toolbar-action".into())]);
    assert_eq!(runtime.focused(), Some("toolbar-action"));
    assert_eq!(runtime.scroll_metrics("body").unwrap().offset.1, 40.);
}
