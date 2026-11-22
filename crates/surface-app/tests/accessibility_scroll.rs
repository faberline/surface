use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityScrollResult as Outcome, AccessibilityScrollTarget, Application, Event, ImeEvent,
    Runtime, View,
};

#[derive(Default)]
struct App {
    text: String,
    modal: bool,
}

#[derive(Clone)]
enum Msg {
    Edit(String),
    Modal(bool),
    Rebuild,
}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Edit(text) => self.text = text,
            Msg::Modal(value) => self.modal = value,
            Msg::Rebuild => {}
        }
    }

    fn view(&self) -> View<Msg> {
        let target = row("reply").accessibility_scroll_into_view();
        let inner = View::new(
            Node::new(Role::Group, "Replies")
                .id("inner")
                .styled(height(70.).clip(true)),
        )
        .stick_to_bottom(true)
        .children([spacer(95.), target, spacer(190.)]);
        let anonymous_clip =
            View::new(Node::new(Role::Group, "Tile").styled(height(20.).clip(true))).children([
                spacer(40.),
                row("unreachable").accessibility_scroll_into_view(),
            ]);
        let mut contents = vec![spacer(130.), inner, anonymous_clip, spacer(160.)];
        if self.modal {
            // This dialog is in flow below the outer viewport. The external
            // scroller is forbidden to move on behalf of the active modal.
            contents.push(
                View::new(Node::new(Role::Dialog, "Modal").id("modal"))
                    .children([row("modal-reply").accessibility_scroll_into_view()])
                    .modal_overlay(Msg::Modal(false)),
            );
        }
        View::new(Node::new(Role::Window, "Scroll test")).children([
            View::new(
                Node::new(Role::TextInput, "Draft")
                    .id("draft")
                    .styled(height(30.))
                    .text(&self.text),
            )
            .on_input(Msg::Edit),
            View::new(
                Node::new(Role::Group, "Viewport")
                    .id("outer")
                    .styled(height(95.).clip(true)),
            )
            .children(contents),
        ])
    }
}

fn height(value: f32) -> Style {
    Style::column().height(Length::Px(value))
}

fn spacer(value: f32) -> View<Msg> {
    View::new(Node::new(Role::Group, "").styled(height(value)))
}

fn row(id: &str) -> View<Msg> {
    View::new(
        Node::new(Role::Text, id)
            .id(id)
            .text(id)
            .styled(height(28.)),
    )
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size)
    }
}

fn render(rt: &mut Runtime<App>) {
    rt.render(240., 170., &mut Metrics).unwrap();
}

fn start() -> Runtime<App> {
    let mut rt = Runtime::new(App::default()).unwrap();
    rt.activate("draft").unwrap();
    render(&mut rt);
    rt
}

fn ticket(rt: &Runtime<App>, id: &str) -> Option<AccessibilityScrollTarget> {
    rt.accessibility_frame()?.scroll_target(rt.dom().by_id(id)?)
}

fn shown(rt: &Runtime<App>, id: &str) -> bool {
    let id = rt.dom().by_id(id).unwrap();
    let computed = &rt.computed().unwrap().nodes[id.index()];
    let shown = computed.bounds.intersect(computed.clip);
    shown.width > 0. && shown.height > 0.
}

#[test]
fn nested_reveal_keeps_focus_and_publishes_manual_offsets() {
    let mut rt = start();
    assert!(!shown(&rt, "reply"));
    assert!(rt.scroll_metrics("inner").unwrap().stuck);
    assert!(ticket(&rt, "unreachable").is_none());
    let target = ticket(&rt, "reply").unwrap();
    assert_eq!(
        rt.accessibility_scroll_into_view(&target).unwrap(),
        Outcome::Scheduled
    );
    assert!(!shown(&rt, "reply"));
    render(&mut rt);
    assert!(shown(&rt, "reply"));
    let inner = rt.scroll_metrics("inner").unwrap();
    let outer = rt.scroll_metrics("outer").unwrap();
    assert!(!inner.stuck);
    assert!(!inner.at_bottom);
    assert!(outer.offset.1 > 0.);
    assert_eq!(rt.focused(), Some("draft"));
    rt.apply(Msg::Rebuild).unwrap();
    render(&mut rt);
    assert_eq!(rt.scroll_metrics("inner").unwrap(), inner);
    assert_eq!(rt.scroll_metrics("outer").unwrap(), outer);
    assert!(shown(&rt, "reply"));
}

#[test]
fn replaced_views_and_external_modal_ancestors_do_not_consume_reveal() {
    let mut rt = start();
    let original = ticket(&rt, "reply").unwrap();
    assert_eq!(
        rt.accessibility_scroll_into_view(&original).unwrap(),
        Outcome::Scheduled
    );
    rt.apply(Msg::Rebuild).unwrap();
    render(&mut rt);
    assert!(!shown(&rt, "reply"));
    assert_eq!(
        rt.accessibility_scroll_into_view(&original).unwrap(),
        Outcome::Rejected
    );

    let old = ticket(&rt, "reply").unwrap();
    rt.apply(Msg::Modal(true)).unwrap();
    render(&mut rt);
    assert!(ticket(&rt, "modal-reply").is_none());
    assert!(ticket(&rt, "reply").is_none());
    assert_eq!(
        rt.accessibility_scroll_into_view(&old).unwrap(),
        Outcome::Rejected
    );
    assert_eq!(rt.scroll_metrics("outer").unwrap().offset.1, 0.);
}

#[test]
fn reveal_respects_pointer_and_pending_composition() {
    let mut rt = start();
    let original = ticket(&rt, "reply").unwrap();
    rt.dispatch(Event::PointerDown { x: 12., y: 12. }).unwrap();
    assert_eq!(
        rt.accessibility_scroll_into_view(&original).unwrap(),
        Outcome::BlockedByPointer
    );
    rt.dispatch(Event::PointerUp { x: 12., y: 12. }).unwrap();
    render(&mut rt);
    let old = ticket(&rt, "reply").unwrap();
    rt.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    assert_eq!(
        rt.accessibility_scroll_into_view(&old).unwrap(),
        Outcome::BlockedByComposition
    );
    render(&mut rt);
    assert!(!shown(&rt, "reply"));
}
