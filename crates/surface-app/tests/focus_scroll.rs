use surface::{Insets, Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};
struct App {
    modal: bool,
    nested: bool,
}
#[derive(Clone)]
enum Message {
    Open,
    Close,
}
impl Application for App {
    type Message = Message;
    fn update(&mut self, message: Message) {
        self.modal = matches!(message, Message::Open);
    }
    fn view(&self) -> View<Message> {
        let button = |id: &str| {
            View::new(
                Node::new(Role::Button, id)
                    .id(id)
                    .text(id)
                    .styled(Style::default().height(Length::Px(40.))),
            )
            .on_press(Message::Open)
        };
        let contents = [
            button("first"),
            View::new(
                Node::new(Role::Group, "spacer").styled(Style::default().height(Length::Px(180.))),
            ),
            button("last"),
        ];
        let mut children = if self.nested {
            vec![
                View::new(
                    Node::new(Role::Group, "top").styled(Style::default().height(Length::Px(120.))),
                ),
                View::new(
                    Node::new(Role::Group, "inner")
                        .id("inner")
                        .styled(Style::column().height(Length::Px(90.)).clip(true)),
                )
                .children(contents),
            ]
        } else {
            Vec::from(contents)
        };
        if self.modal {
            children.push(
                View::new(
                    Node::new(Role::Dialog, "Dialog").id("dialog").styled(
                        Style::column()
                            .height(Length::Px(80.))
                            .fixed(Insets::all(0.))
                            .clip(true),
                    ),
                )
                .modal(Message::Close)
                .children([View::new(
                    Node::new(Role::Button, "Close")
                        .id("close")
                        .text("Close")
                        .styled(Style::default().height(Length::Px(40.))),
                )
                .on_press(Message::Close)]),
            );
        }
        View::new(
            Node::new(Role::Window, "Scroll test")
                .id("root")
                .styled(Style::column().clip(true)),
        )
        .children(children)
    }
}
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size / 2., size * 1.5)
    }
}
fn render(runtime: &mut Runtime<App>) {
    runtime.render(240., 100., &mut Metrics).unwrap();
}
fn visible(runtime: &Runtime<App>, key: &str) -> bool {
    let id = runtime.dom().by_id(key).unwrap();
    let bounds = &runtime.computed().unwrap().nodes[id.index()];
    let visible = bounds.bounds.intersect(bounds.clip);
    visible.width > 0. && visible.height == bounds.bounds.height
}
#[test]
fn tab_reaches_clipped_controls_and_reverse_tab_reveals_the_first() {
    let mut runtime = Runtime::new(App {
        modal: false,
        nested: false,
    })
    .unwrap();
    render(&mut runtime);
    assert!(!visible(&runtime, "last"));
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("first"));
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("last"));
    assert!(visible(&runtime, "last"));
    runtime
        .dispatch(Event::FocusNext { reverse: true })
        .unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("first"));
    assert!(visible(&runtime, "first"));
}
#[test]
fn nested_scrollers_reveal_inner_then_outer_and_do_not_fight_manual_scroll() {
    let mut runtime = Runtime::new(App {
        modal: false,
        nested: true,
    })
    .unwrap();
    render(&mut runtime);
    // Activating a control opens a fixed modal, so use keyboard focus here.
    runtime
        .dispatch(Event::FocusNext { reverse: true })
        .unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("last"));
    assert!(visible(&runtime, "last"));
    let snapshot = runtime.snapshot();
    let inner = snapshot
        .dom
        .nodes
        .iter()
        .find(|node| node.stable_id() == Some("inner"))
        .unwrap();
    assert!(inner.style().scroll_y > 0.);
    let id = runtime.dom().by_id("root").unwrap();
    let before = runtime.dom().get(id).unwrap().style().scroll_y;
    runtime
        .dispatch(Event::Scroll {
            x: 10.,
            y: 1.,
            delta_x: 0.,
            delta_y: -30.,
        })
        .unwrap();
    render(&mut runtime);
    assert!(
        runtime.dom().get(id).unwrap().style().scroll_y < before,
        "unchanged focus must not undo a deliberate scroll"
    );
}
#[test]
fn a_fixed_modal_does_not_scroll_its_background_ancestors() {
    let mut runtime = Runtime::new(App {
        modal: false,
        nested: false,
    })
    .unwrap();
    render(&mut runtime);
    runtime.activate("first").unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("close"));
    assert!(visible(&runtime, "close"));
    assert_eq!(
        runtime
            .dom()
            .get(runtime.dom().root())
            .unwrap()
            .style()
            .scroll_y,
        0.
    );
}

#[test]
fn resizing_keeps_the_same_focused_control_in_view() {
    let mut runtime = Runtime::new(App {
        modal: false,
        nested: false,
    })
    .unwrap();
    runtime.render(240., 300., &mut Metrics).unwrap();
    runtime
        .dispatch(Event::FocusNext { reverse: true })
        .unwrap();
    runtime.render(240., 300., &mut Metrics).unwrap();
    assert_eq!(runtime.focused(), Some("last"));
    assert!(visible(&runtime, "last"));
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("last"));
    assert!(visible(&runtime, "last"));
}
