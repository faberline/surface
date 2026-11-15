//! Right click and the context menu pattern (issue #120). Headless: a
//! fixed-metric measurer, no window, no real mouse. The hosts send
//! `Event::SecondaryDown` for a right click; the runtime finds the nearest
//! `on_context_menu` handler and posts its message with the point.
use surface::{Insets, Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Event, PointerPoint, Runtime, TextOptions, View};

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.5, size * 1.5)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Pressed(&'static str),
    Context(&'static str, PointerPoint),
    Pick,
    Close,
    Text(String),
}

#[derive(Default)]
struct App {
    menu: Option<(f32, f32)>,
    log: Vec<Msg>,
    text: String,
    /// No handler opens a menu, so state changes can only come from the press.
    quiet: bool,
}
impl Application for App {
    type Message = Msg;
    fn update(&mut self, message: Msg) {
        self.log.push(message.clone());
        match message {
            Msg::Context(name, p) => {
                self.menu = (!self.quiet && name != "field").then_some((p.x, p.y))
            }
            Msg::Pick | Msg::Close => self.menu = None,
            Msg::Text(text) => self.text = text,
            Msg::Pressed(_) => {}
        }
    }
    fn view(&self) -> View<Msg> {
        let row = |id: &'static str| {
            Node::new(Role::Button, id)
                .id(id)
                .styled(Style::default().height(Length::Px(40.)))
        };
        // Rows are 40 px tall and stacked: row y = 0, parent y = 40,
        // field y = 80, plain y = 120.
        let mut children = vec![
            View::new(row("row"))
                .on_press(Msg::Pressed("row"))
                .on_context_menu(|p| Msg::Context("row", p)),
            View::new(
                Node::new(Role::Group, "parent")
                    .id("parent")
                    .styled(Style::column().height(Length::Px(40.))),
            )
            .on_context_menu(|p| Msg::Context("parent", p))
            .children([View::new(
                Node::new(Role::Button, "child")
                    .id("child")
                    .styled(Style::default().height(Length::Px(40.))),
            )
            .on_press(Msg::Pressed("child"))]),
            View::new(
                Node::new(Role::TextInput, "Field")
                    .id("field")
                    .text(&self.text)
                    .styled(Style::default().height(Length::Px(40.))),
            )
            .text_options(TextOptions::default())
            .on_input(Msg::Text)
            .on_context_menu(|p| Msg::Context("field", p)),
            View::new(row("plain")).on_press(Msg::Pressed("plain")),
        ];
        if let Some((x, y)) = self.menu {
            // Same shape as `Theme::menu_at`: a fixed, modal menu panel.
            let mut style = Style::column().width(Length::Px(240.)).fixed(Insets {
                left: Length::Px(x),
                top: Length::Px(y),
                ..Default::default()
            });
            style.z_index = 1001;
            children.push(
                View::new(
                    Node::new(Role::Menu, "Menu")
                        .id("menu")
                        .tab_index(-1)
                        .styled(style),
                )
                .children([View::new(
                    Node::new(Role::MenuItem, "Pick")
                        .id("pick")
                        .styled(Style::default().height(Length::Px(40.))),
                )
                .on_press(Msg::Pick)])
                .modal(Msg::Close),
            );
        }
        View::new(Node::new(Role::Window, "App").styled(Style::column())).children(children)
    }
}

fn runtime() -> Runtime<App> {
    let mut rt = Runtime::new(App::default()).unwrap();
    rt.render(640., 480., &mut Metrics).unwrap();
    rt
}
fn secondary(rt: &mut Runtime<App>, x: f32, y: f32) -> bool {
    let result = rt.dispatch(Event::SecondaryDown { x, y }).unwrap();
    rt.render(640., 480., &mut Metrics).unwrap();
    result
}
fn left_click(rt: &mut Runtime<App>, x: f32, y: f32) {
    rt.dispatch(Event::PointerDown { x, y }).unwrap();
    rt.dispatch(Event::PointerUp { x, y }).unwrap();
    rt.render(640., 480., &mut Metrics).unwrap();
}
fn menu_bounds(rt: &Runtime<App>) -> Option<(f32, f32)> {
    let id = rt.dom().by_id("menu")?;
    let b = rt.computed()?.nodes[id.index()].bounds;
    Some((b.x, b.y))
}

// Row 1.
#[test]
fn secondary_press_posts_the_handler_message_with_the_exact_point() {
    let mut rt = runtime();
    assert!(secondary(&mut rt, 12.5, 7.25));
    assert_eq!(
        rt.app().log,
        [Msg::Context("row", PointerPoint { x: 12.5, y: 7.25 })]
    );
}

// Row 1: nearest ancestor wins.
#[test]
fn a_handler_on_a_parent_runs_when_a_child_is_hit() {
    let mut rt = runtime();
    assert!(secondary(&mut rt, 20., 50.));
    assert_eq!(
        rt.app().log,
        [Msg::Context("parent", PointerPoint { x: 20., y: 50. })]
    );
}

#[test]
fn no_handler_on_the_node_or_any_ancestor_does_nothing() {
    let mut rt = runtime();
    assert!(!secondary(&mut rt, 20., 130.));
    assert!(rt.app().log.is_empty());
    assert_eq!(rt.snapshot().interaction.focused, None);
}

// Row 2.
#[test]
fn the_app_opens_a_menu_at_the_point_and_an_outside_press_closes_it() {
    let mut rt = runtime();
    secondary(&mut rt, 200., 20.);
    assert_eq!(menu_bounds(&rt), Some((200., 20.)));
    // Outside the menu: a secondary press dismisses and posts no context message.
    assert!(secondary(&mut rt, 20., 10.));
    assert_eq!(
        rt.app().log,
        [
            Msg::Context("row", PointerPoint { x: 200., y: 20. }),
            Msg::Close
        ]
    );
    assert_eq!(menu_bounds(&rt), None);
}

#[test]
fn a_left_click_outside_closes_the_menu() {
    let mut rt = runtime();
    secondary(&mut rt, 200., 20.);
    left_click(&mut rt, 20., 10.);
    assert_eq!(menu_bounds(&rt), None);
    assert!(!rt.app().log.contains(&Msg::Pressed("row")));
}

#[test]
fn escape_closes_the_menu() {
    let mut rt = runtime();
    secondary(&mut rt, 200., 20.);
    rt.dispatch(Event::Cancel).unwrap();
    rt.render(640., 480., &mut Metrics).unwrap();
    assert_eq!(menu_bounds(&rt), None);
}

#[test]
fn a_secondary_press_on_a_menu_item_does_nothing() {
    let mut rt = runtime();
    secondary(&mut rt, 200., 20.);
    let before = rt.app().log.len();
    assert!(!secondary(&mut rt, 210., 30.));
    assert_eq!(rt.app().log.len(), before);
    assert_eq!(menu_bounds(&rt), Some((200., 20.)), "the menu stays open");
}

// Row 3: negative. A secondary press changes no pointer state.
#[test]
fn a_secondary_press_changes_no_pointer_state() {
    // `quiet`: the handlers open no menu, so the whole snapshot must stay equal.
    let mut rt = Runtime::new(App {
        quiet: true,
        ..App::default()
    })
    .unwrap();
    rt.render(640., 480., &mut Metrics).unwrap();
    rt.activate("field").unwrap();
    rt.dispatch(Event::Text("hi".into())).unwrap();
    rt.render(640., 480., &mut Metrics).unwrap();
    let before = rt.snapshot();

    // On the text field: neither cursor nor focus moves.
    secondary(&mut rt, 2., 90.);
    assert_eq!(
        rt.snapshot().text_inputs,
        before.text_inputs,
        "value and cursor"
    );
    assert_eq!(rt.snapshot().interaction, before.interaction);

    // On a pressable row: its handler runs, on_press does not, and focus,
    // hover and pressed stay as they were. A release after it activates nothing.
    let log_len = rt.app().log.len();
    secondary(&mut rt, 12., 10.);
    assert_eq!(
        rt.snapshot().interaction,
        before.interaction,
        "focus, hover, pressed"
    );
    assert_eq!(rt.snapshot().text_inputs, before.text_inputs);
    rt.dispatch(Event::PointerUp { x: 12., y: 10. }).unwrap();
    assert_eq!(rt.app().log.len(), log_len + 1, "only the context message");
    assert!(!rt.app().log.contains(&Msg::Pressed("row")));
}

#[test]
fn a_left_press_posts_only_the_primary_message() {
    let mut rt = runtime();
    left_click(&mut rt, 12., 10.);
    assert_eq!(rt.app().log, [Msg::Pressed("row")]);
}

/// With the `devtools` feature the trace names the input.
#[cfg(feature = "devtools")]
#[test]
fn the_trace_records_a_secondary_down_input() {
    use surface_app::trace::Recorder;
    let mut rt = runtime();
    let mut now = 0.;
    rt.attach_trace(Recorder::new(Box::new(move || {
        now += 1.;
        now
    })));
    secondary(&mut rt, 12., 10.);
    let trace = rt.trace().unwrap();
    let found = trace
        .records_after(None)
        .any(|r| format!("{r:?}").contains("SecondaryDown"));
    assert!(found, "no SecondaryDown input record");
}
