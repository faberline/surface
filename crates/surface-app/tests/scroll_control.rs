use std::{cell::RefCell, rc::Rc};
use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{
    AppError, Application, Event, Queue, Runtime, ScrollMetrics, ScrollTarget, View,
};

#[derive(Clone)]
enum Message {
    Rows(usize),
    Bottom,
    Poke,
}
struct App {
    rows: usize,
    stick: bool,
    inner: bool,
    no_id: bool,
    wide: bool,
    poke: u32,
    queue: Rc<RefCell<Option<Queue<Message>>>>,
}
impl App {
    fn new(rows: usize) -> Self {
        Self {
            rows,
            stick: true,
            inner: false,
            no_id: false,
            wide: false,
            poke: 0,
            queue: Rc::default(),
        }
    }
}
impl Application for App {
    type Message = Message;
    fn update(&mut self, message: Message) {
        match message {
            Message::Rows(n) => self.rows = n,
            Message::Bottom => self
                .queue
                .borrow()
                .as_ref()
                .unwrap()
                .scroll_to("log", ScrollTarget::Bottom),
            Message::Poke => self.poke += 1,
        }
    }
    fn view(&self) -> View<Message> {
        let row = |i: usize| {
            View::new(
                Node::new(Role::Button, format!("row{i}"))
                    .id(format!("row{i}"))
                    .text("x")
                    .styled(Style::default().height(Length::Px(20.))),
            )
            .on_press(Message::Poke)
        };
        let mut rows: Vec<View<Message>> = (0..self.rows).map(row).collect();
        if self.wide {
            rows.push(View::new(
                Node::new(Role::Group, "wide").styled(
                    Style::default()
                        .width(Length::Px(40.))
                        .height(Length::Px(20.)),
                ),
            ));
        }
        let style = Style::column()
            .width(Length::Px(100.))
            .height(Length::Px(200.))
            .clip(true);
        let mut node = Node::new(Role::Group, "log").styled(style);
        if !self.no_id {
            node = node.id("log");
        }
        let log = View::new(node).stick_to_bottom(self.stick).children(rows);
        let content = if self.inner {
            let outer = Style::column()
                .width(Length::Px(100.))
                .height(Length::Px(200.))
                .clip(true);
            View::new(Node::new(Role::Group, "outer").id("outer").styled(outer)).children(
                (0..30)
                    .map(|i| {
                        View::new(
                            Node::new(Role::Group, format!("o{i}"))
                                .styled(Style::default().height(Length::Px(20.))),
                        )
                    })
                    .chain([log]),
            )
        } else {
            log
        };
        View::new(
            Node::new(Role::Window, "App")
                .id("root")
                .styled(Style::column()),
        )
        .children([content])
    }
}
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size / 2., size * 1.5)
    }
}
fn start(app: App) -> Runtime<App> {
    let slot = app.queue.clone();
    let runtime = Runtime::new(app).unwrap();
    *slot.borrow_mut() = Some(runtime.queue());
    runtime
}
fn rows(runtime: &mut Runtime<App>, n: usize) {
    runtime.apply(Message::Rows(n)).unwrap();
}
fn render(runtime: &mut Runtime<App>) {
    runtime.render(300., 400., &mut Metrics).unwrap();
}
fn metrics(runtime: &Runtime<App>, id: &str) -> ScrollMetrics {
    runtime.scroll_metrics(id).unwrap()
}
fn wheel(runtime: &mut Runtime<App>, delta_y: f32) {
    runtime
        .dispatch(Event::Scroll {
            x: 50.,
            y: 100.,
            delta_x: 0.,
            delta_y,
        })
        .unwrap();
    render(runtime);
}
fn row_bounds(runtime: &Runtime<App>, i: usize) -> surface::Rect {
    let id = runtime.dom().by_id(&format!("row{i}")).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}
fn max_y(n: usize) -> f32 {
    (20. * n as f32 - 200.).max(0.)
}

#[test]
fn stick_follows_growth() {
    let mut r = start(App::new(5));
    render(&mut r);
    for n in [10, 30, 60] {
        rows(&mut r, n);
        render(&mut r);
        let m = metrics(&r, "log");
        assert_eq!(m.offset.1, max_y(n), "n = {n}");
        assert!(m.at_bottom && m.stuck);
        let last = row_bounds(&r, n - 1);
        assert_eq!(
            last.y + last.height,
            200.,
            "newest row ends at the viewport bottom"
        );
    }
}

#[test]
fn stick_starts_stuck_with_content_that_fits() {
    let mut r = start(App::new(5));
    render(&mut r);
    let m = metrics(&r, "log");
    assert!(m.stuck && m.at_bottom);
    assert_eq!(m.offset, (0., 0.));
    rows(&mut r, 12);
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, 40.);
}

#[test]
fn user_scroll_up_stops_following() {
    let mut r = start(App::new(30));
    render(&mut r);
    wheel(&mut r, -50.);
    let m = metrics(&r, "log");
    assert_eq!(m.offset.1, max_y(30) - 50.);
    assert!(!m.stuck && !m.at_bottom);
    rows(&mut r, 60);
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, max_y(30) - 50.);
}

#[test]
fn user_scroll_to_bottom_resumes() {
    let mut r = start(App::new(30));
    render(&mut r);
    wheel(&mut r, -50.);
    rows(&mut r, 60);
    render(&mut r);
    wheel(&mut r, 10000.);
    let m = metrics(&r, "log");
    assert!(m.stuck && m.at_bottom);
    assert_eq!(m.offset.1, max_y(60));
    rows(&mut r, 80);
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, max_y(80));
}

#[test]
fn user_scroll_tiny_scroll_up_is_still_bottom() {
    let mut r = start(App::new(30));
    render(&mut r);
    wheel(&mut r, -0.3);
    let m = metrics(&r, "log");
    assert!(m.stuck && m.at_bottom);
    assert_eq!(m.offset.1, max_y(30));
}

#[test]
fn focus_reveal_off_bottom_stops_following() {
    let mut r = start(App::new(30));
    render(&mut r);
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    render(&mut r);
    assert_eq!(r.focused(), Some("row0"));
    let m = metrics(&r, "log");
    assert_eq!(m.offset.1, 0.);
    assert!(!m.stuck);
    rows(&mut r, 60);
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, 0.);
}

#[test]
fn no_stick_flag_no_follow() {
    let mut app = App::new(30);
    app.stick = false;
    let mut r = start(app);
    render(&mut r);
    rows(&mut r, 60);
    render(&mut r);
    let m = metrics(&r, "log");
    assert_eq!(m.offset.1, 0.);
    assert!(!m.stuck);
}

#[test]
fn clamp_large_offset() {
    let mut app = App::new(30);
    app.stick = false;
    let mut r = start(app);
    render(&mut r);
    assert!(r.scroll_to("log", ScrollTarget::Offset { x: 0., y: 1e6 }));
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, max_y(30));
    let last = row_bounds(&r, 29);
    assert_eq!(last.y + last.height, 200.);
}

#[test]
fn clamp_after_content_shrinks() {
    let mut r = start(App::new(30));
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, 400.);
    rows(&mut r, 12);
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, 40.);
    assert_eq!(row_bounds(&r, 0).y, -40.);
    rows(&mut r, 5);
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, 0.);
    assert_eq!(row_bounds(&r, 0).y, 0.);
}

#[test]
fn clamp_without_stick_after_the_user_scrolled() {
    let mut app = App::new(30);
    app.stick = false;
    let mut r = start(app);
    render(&mut r);
    wheel(&mut r, 300.);
    assert_eq!(metrics(&r, "log").offset.1, 300.);
    rows(&mut r, 12);
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, 40.);
}

#[test]
fn clamp_negative_and_nonfinite() {
    let mut r = start(App::new(30));
    render(&mut r);
    assert!(r.scroll_to("log", ScrollTarget::Offset { x: 0., y: -5. }));
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, 0.);
    assert!(!r.scroll_to("log", ScrollTarget::Offset { x: 0., y: f32::NAN }));
    assert!(!r.scroll_to(
        "log",
        ScrollTarget::Offset {
            x: f32::INFINITY,
            y: 0.
        }
    ));
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, 0.);
}

#[test]
fn api_top_bottom_offset() {
    let mut r = start(App::new(30));
    render(&mut r);
    assert!(r.scroll_to("log", ScrollTarget::Top));
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset, (0., 0.));
    assert!(r.scroll_to("log", ScrollTarget::Bottom));
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset, (0., 400.));
    assert!(r.scroll_to("log", ScrollTarget::Offset { x: 0., y: 130. }));
    render(&mut r);
    let m = metrics(&r, "log");
    assert_eq!(m.offset, (0., 130.));
    assert!(!m.stuck && !m.at_bottom);
    assert!(r.scroll_to("log", ScrollTarget::Offset { x: 30., y: 130. }));
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset, (0., 130.));
}

#[test]
fn api_horizontal_offset_clamps_to_the_content() {
    let mut app = App::new(5);
    app.wide = true;
    app.stick = false;
    let mut r = start(app);
    render(&mut r);
    // The wide child is 40 px in a 100 px scroller, so x never moves.
    assert!(r.scroll_to("log", ScrollTarget::Offset { x: 50., y: 0. }));
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.0, 0.);
}

#[test]
fn api_queue_from_update() {
    let mut app = App::new(30);
    app.stick = false;
    let mut r = start(app);
    render(&mut r);
    r.apply(Message::Bottom).unwrap();
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset.1, 400.);
}

#[test]
fn api_metrics() {
    let mut app = App::new(30);
    app.stick = false;
    let mut r = start(app);
    assert!(r.scroll_metrics("log").is_none());
    render(&mut r);
    assert_eq!(
        metrics(&r, "log"),
        ScrollMetrics {
            offset: (0., 0.),
            content: (100., 600.),
            viewport: (100., 200.),
            at_bottom: false,
            stuck: false,
        }
    );
    assert!(r.scroll_metrics("nope").is_none());
    assert!(r.scroll_metrics("root").is_none(), "no clip, no metrics");
    // A pending request does not change the values of the last render.
    assert!(r.scroll_to("log", ScrollTarget::Bottom));
    assert_eq!(metrics(&r, "log").offset, (0., 0.));
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset, (0., 400.));
    assert!(metrics(&r, "log").at_bottom);
}

#[test]
fn api_unknown_id() {
    let mut r = start(App::new(5));
    render(&mut r);
    assert!(!r.scroll_to("nope", ScrollTarget::Top));
    assert!(!r.scroll_to("root", ScrollTarget::Top));
    r.queue().scroll_to("nope", ScrollTarget::Top);
    render(&mut r);
    assert!(r.scroll_metrics("nope").is_none());
}

#[test]
fn stick_without_id_errors() {
    let mut app = App::new(5);
    app.no_id = true;
    assert!(matches!(Runtime::new(app), Err(AppError::MissingId(_))));
}

#[test]
fn offset_survives_rebuild() {
    let mut r = start(App::new(30));
    render(&mut r);
    wheel(&mut r, -100.);
    let before = metrics(&r, "log").offset;
    r.apply(Message::Poke).unwrap();
    render(&mut r);
    assert_eq!(metrics(&r, "log").offset, before);
}

#[test]
fn nested_scrollers_independent() {
    let mut app = App::new(5);
    app.inner = true;
    let mut r = start(app);
    render(&mut r);
    let outer = metrics(&r, "outer");
    for n in [10, 30] {
        rows(&mut r, n);
        render(&mut r);
        assert_eq!(metrics(&r, "log").offset.1, max_y(n));
        assert_eq!(metrics(&r, "outer").offset, outer.offset);
        assert!(!metrics(&r, "outer").stuck);
    }
}

#[test]
fn render_is_idempotent() {
    let mut r = start(App::new(30));
    let a = r.render(300., 400., &mut Metrics).unwrap();
    let b = r.render(300., 400., &mut Metrics).unwrap();
    assert_eq!(a, b);
}
