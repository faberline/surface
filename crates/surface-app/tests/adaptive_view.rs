use std::cell::Cell;
use surface::{Node, Role, Style, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};

#[derive(Clone)]
enum Message {
    Edited(String),
}

#[derive(Default)]
struct App {
    compact: bool,
    calls: Vec<(f32, f32)>,
    text: String,
    view_count: Cell<usize>,
}
impl Application for App {
    type Message = Message;
    fn update(&mut self, message: Message) {
        let Message::Edited(text) = message;
        self.text = text;
    }
    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        self.calls.push((width, height));
        let compact = width < 600.;
        let changed = self.compact != compact;
        self.compact = compact;
        changed
    }
    fn view(&self) -> View<Message> {
        self.view_count.set(self.view_count.get() + 1);
        let mut children = vec![View::new(
            Node::new(Role::TextInput, "Composer")
                .id("composer")
                .text(&self.text),
        )
        .on_input(Message::Edited)];
        if !self.compact {
            children.push(View::new(Node::new(Role::Group, "Panel").id("panel")));
        }
        View::new(Node::new(Role::Window, "Window").styled(Style::column())).children(children)
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size / 2., size * 1.5)
    }
}

#[test]
fn resize_rebuilds_the_view_only_when_the_app_needs_it_and_keeps_input() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    runtime.render(900., 700., &mut Metrics).unwrap();
    assert!(runtime.dom().by_id("panel").is_some());
    assert_eq!(runtime.app().view_count.get(), 1);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    runtime.dispatch(Event::Text("preserve me".into())).unwrap();
    assert_eq!(runtime.app().text, "preserve me");
    assert_eq!(runtime.focused(), Some("composer"));
    runtime.render(599., 700., &mut Metrics).unwrap();
    assert!(runtime.dom().by_id("panel").is_none());
    assert_eq!(runtime.app().text, "preserve me");
    assert_eq!(runtime.focused(), Some("composer"));
    runtime.render(600., 700., &mut Metrics).unwrap();
    assert!(runtime.dom().by_id("panel").is_some());
    assert_eq!(runtime.app().text, "preserve me");
    assert_eq!(runtime.focused(), Some("composer"));
    let views_after_resize = runtime.app().view_count.get();
    runtime.render(700., 700., &mut Metrics).unwrap();
    assert_eq!(runtime.app().view_count.get(), views_after_resize);
    runtime.render(700., 700., &mut Metrics).unwrap();
    assert_eq!(
        runtime.app().calls,
        [(900., 700.), (599., 700.), (600., 700.), (700., 700.)]
    );
}

#[derive(Default)]
struct FailsOnce {
    fail_next_view: Cell<bool>,
    resized: bool,
    calls: usize,
}
impl Application for FailsOnce {
    type Message = ();
    fn update(&mut self, _: ()) {}
    fn viewport_changed(&mut self, _: f32, _: f32) -> bool {
        self.calls += 1;
        if self.resized {
            false
        } else {
            self.resized = true;
            self.fail_next_view.set(true);
            true
        }
    }
    fn view(&self) -> View<()> {
        let fail = self.fail_next_view.replace(false);
        let root = View::new(Node::new(Role::Window, "Window"));
        let child = || View::new(Node::new(Role::Group, "item").id("item"));
        if fail {
            root.children([child(), child()])
        } else {
            root.children([child()])
        }
    }
}

#[test]
fn failed_resize_rebuild_is_retried_at_the_same_size() {
    let mut runtime = Runtime::new(FailsOnce::default()).unwrap();
    assert!(runtime.render(600., 500., &mut Metrics).is_err());
    runtime.render(600., 500., &mut Metrics).unwrap();
    assert!(runtime.dom().by_id("item").is_some());
    assert_eq!(runtime.app().calls, 1);
}

#[test]
fn invalid_dimensions_do_not_change_app_state() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    for (width, height) in [
        (0., 1.),
        (1., 0.),
        (-1., 1.),
        (1., -1.),
        (f32::NAN, 1.),
        (1., f32::NAN),
        (f32::INFINITY, 1.),
        (1., f32::INFINITY),
    ] {
        assert!(runtime.render(width, height, &mut Metrics).is_err());
        assert!(runtime.app().calls.is_empty());
    }
    runtime.render(599., 400., &mut Metrics).unwrap();
    assert_eq!(runtime.app().calls, [(599., 400.)]);
    assert!(runtime.dom().by_id("panel").is_none());
}
