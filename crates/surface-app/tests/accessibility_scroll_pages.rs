use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityScrollDirection::{self, Down, Left, Right, Up},
    AccessibilityScrollPageTarget, AccessibilityScrollResult as Outcome, AppError, Application,
    Event, ImeEvent, Runtime, View,
};

#[derive(Clone)]
enum Message {
    Text(String),
    ContentHeight(f32),
    Modal,
}

struct Example {
    text: String,
    content_height: f32,
    sticky: bool,
    opted_in: bool,
    identified: bool,
    disabled: bool,
    modal: bool,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            text: "Draft".into(),
            content_height: 260.,
            sticky: false,
            opted_in: true,
            identified: true,
            disabled: false,
            modal: false,
        }
    }
}

fn block(width: f32, height: f32) -> View<Message> {
    View::new(
        Node::new(Role::Group, "").styled(
            Style::column()
                .width(Length::Px(width))
                .height(Length::Px(height)),
        ),
    )
}

impl Application for Example {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Text(text) => self.text = text,
            Message::ContentHeight(height) => self.content_height = height,
            Message::Modal => self.modal = !self.modal,
        }
    }

    fn view(&self) -> View<Message> {
        let mut node = Node::new(Role::Group, "Pages")
            .disabled(self.disabled)
            .styled(
                Style::column()
                    .width(Length::Px(80.))
                    .height(Length::Px(60.))
                    .clip(true),
            );
        if self.identified {
            node = node.id("pages");
        }
        let mut viewport = View::new(node)
            .stick_to_bottom(self.sticky)
            .children([block(280., self.content_height)]);
        if self.opted_in {
            viewport = viewport.accessibility_scroll_pages();
        }
        let outer = View::new(
            Node::new(Role::Group, "Outer")
                .id("outer")
                .styled(Style::column().height(Length::Px(60.)).clip(true)),
        )
        .children([block(100., 30.), viewport]);
        let draft = View::new(
            Node::new(Role::TextInput, "Draft")
                .id("draft")
                .text(&self.text)
                .styled(Style::column().height(Length::Px(30.))),
        )
        .on_input(Message::Text);
        let mut contents = vec![draft, outer];
        if self.modal {
            contents.push(
                View::new(Node::new(Role::Dialog, "Modal").id("modal"))
                    .modal_overlay(Message::Modal),
            );
        }
        View::new(Node::new(Role::Window, "Test window")).children(contents)
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size)
    }
}

fn render(runtime: &mut Runtime<Example>) {
    runtime.render(240., 150., &mut Metrics).unwrap();
}

fn start(example: Example) -> Runtime<Example> {
    let mut runtime = Runtime::new(example).unwrap();
    runtime.activate("draft").unwrap();
    render(&mut runtime);
    runtime
}

fn target(
    runtime: &Runtime<Example>,
    id: &str,
    direction: AccessibilityScrollDirection,
) -> Option<AccessibilityScrollPageTarget> {
    runtime
        .accessibility_frame()?
        .scroll_page_target(runtime.dom().by_id(id)?, direction)
}

fn page(runtime: &mut Runtime<Example>, direction: AccessibilityScrollDirection) {
    let target = target(runtime, "pages", direction).unwrap();
    assert_eq!(
        runtime.accessibility_scroll_page(&target).unwrap(),
        Outcome::Scheduled
    );
    render(runtime);
}

#[test]
fn relative_pages_use_the_visible_size_and_leave_the_other_axis_and_parent_alone() {
    let mut runtime = start(Example::default());
    assert!(target(&runtime, "pages", Up).is_none());
    assert!(target(&runtime, "pages", Left).is_none());
    assert!(target(&runtime, "outer", Down).is_none());
    let outer = runtime.scroll_metrics("outer").unwrap();
    page(&mut runtime, Down);
    assert_eq!(runtime.scroll_metrics("pages").unwrap().offset, (0., 30.));
    page(&mut runtime, Down);
    assert_eq!(runtime.scroll_metrics("pages").unwrap().offset, (0., 60.));
    page(&mut runtime, Right);
    assert_eq!(runtime.scroll_metrics("pages").unwrap().offset, (80., 60.));
    page(&mut runtime, Up);
    page(&mut runtime, Left);
    assert_eq!(runtime.scroll_metrics("pages").unwrap().offset, (0., 30.));
    assert_eq!(runtime.scroll_metrics("outer").unwrap(), outer);
    assert_eq!(runtime.focused(), Some("draft"));
}

#[test]
fn vertical_pages_pause_and_resume_sticky_following() {
    let mut runtime = start(Example {
        sticky: true,
        ..Example::default()
    });
    assert!(runtime.scroll_metrics("pages").unwrap().stuck);
    assert!(target(&runtime, "pages", Down).is_none());
    page(&mut runtime, Up);
    assert!(!runtime.scroll_metrics("pages").unwrap().stuck);
    assert_eq!(runtime.scroll_metrics("pages").unwrap().offset.1, 170.);
    runtime.apply(Message::ContentHeight(300.)).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.scroll_metrics("pages").unwrap().offset.1, 170.);
    for _ in 0..3 {
        page(&mut runtime, Down);
    }
    assert!(runtime.scroll_metrics("pages").unwrap().stuck);
    assert_eq!(runtime.scroll_metrics("pages").unwrap().offset.1, 240.);
}

#[test]
fn ordinary_reflows_preserve_live_targets_and_loss_of_visibility_retires_them() {
    let mut runtime = start(Example::default());
    let retained = target(&runtime, "pages", Down).unwrap();
    render(&mut runtime);
    assert_eq!(target(&runtime, "pages", Down).as_ref(), Some(&retained));

    runtime.render(240., 45., &mut Metrics).unwrap();
    assert!(target(&runtime, "pages", Down).is_none());
    render(&mut runtime);
    assert!(target(&runtime, "pages", Down).is_some());
    assert_eq!(
        runtime.accessibility_scroll_page(&retained).unwrap(),
        Outcome::Rejected
    );
}

#[test]
fn rebuilt_and_modal_views_reject_delayed_pages() {
    let mut runtime = start(Example::default());
    let old = target(&runtime, "pages", Down).unwrap();
    runtime.queue().post(Message::ContentHeight(270.));
    assert_eq!(
        runtime.accessibility_scroll_page(&old).unwrap(),
        Outcome::Rebuilt
    );
    render(&mut runtime);
    let pending = target(&runtime, "pages", Down).unwrap();
    assert_eq!(
        runtime.accessibility_scroll_page(&pending).unwrap(),
        Outcome::Scheduled
    );
    runtime.apply(Message::Modal).unwrap();
    render(&mut runtime);
    assert!(target(&runtime, "pages", Down).is_none());
    assert_eq!(
        runtime.accessibility_scroll_page(&pending).unwrap(),
        Outcome::Rejected
    );
    assert_eq!(runtime.scroll_metrics("pages").unwrap().offset, (0., 0.));
}

#[test]
fn viewport_actions_require_opt_in_and_do_not_interrupt_native_input() {
    for app in [
        Example {
            opted_in: false,
            ..Example::default()
        },
        Example {
            disabled: true,
            ..Example::default()
        },
    ] {
        let runtime = start(app);
        assert!(target(&runtime, "pages", Down).is_none());
    }
    let mut runtime = start(Example::default());
    let down = target(&runtime, "pages", Down).unwrap();
    runtime
        .dispatch(Event::PointerDown { x: 12., y: 12. })
        .unwrap();
    assert_eq!(
        runtime.accessibility_scroll_page(&down).unwrap(),
        Outcome::BlockedByPointer
    );
    runtime
        .dispatch(Event::PointerUp { x: 12., y: 12. })
        .unwrap();
    render(&mut runtime);
    let down = target(&runtime, "pages", Down).unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    assert_eq!(
        runtime.accessibility_scroll_page(&down).unwrap(),
        Outcome::BlockedByComposition
    );
    render(&mut runtime);
    assert_eq!(runtime.scroll_metrics("pages").unwrap().offset, (0., 0.));
    assert_eq!(runtime.focused(), Some("draft"));
}

#[test]
fn opted_in_viewport_requires_a_stable_id() {
    let runtime = Runtime::new(Example {
        identified: false,
        ..Example::default()
    });
    assert!(matches!(runtime, Err(AppError::MissingId(name)) if name == "Pages"));
}
