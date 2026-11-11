use surface::{Rect, Role, TextMeasurer};
use surface_app::{Application, Event, Runtime};
use surface_showcase::{Message, Showcase};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        (
            (text.len() as f32 * size * 0.5).min(width.unwrap_or(f32::MAX)),
            size * 1.5,
        )
    }
}
fn draw(runtime: &mut Runtime<Showcase>) {
    runtime.render(960., 800., &mut Metrics).unwrap();
}
#[test]
fn state_drives_the_view_and_pointer_keyboard_input_share_one_runtime() {
    let mut app = Runtime::new(Showcase::default()).unwrap();
    draw(&mut app);
    let id = app.dom().by_id("increment").unwrap();
    let rect = app.computed().unwrap().nodes[id.index()].bounds;
    app.dispatch(Event::PointerDown {
        x: rect.x + 20.,
        y: rect.y + 20.,
    })
    .unwrap();
    app.dispatch(Event::PointerUp {
        x: rect.x + 20.,
        y: rect.y + 20.,
    })
    .unwrap();
    assert_eq!(app.app().count, 1);
    draw(&mut app);
    assert_eq!(
        app.dom()
            .get(app.dom().by_id("counter").unwrap())
            .unwrap()
            .text_value(),
        Some("1")
    );
    assert_eq!(app.focused(), Some("increment"));
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.app().count, 2);
    draw(&mut app);
    app.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(app.focused(), Some("reset"));
    app.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(
        app.focused(),
        Some("name"),
        "disabled control is not focusable"
    );
    app.dispatch(Event::Text("Chris陳".to_owned())).unwrap();
    draw(&mut app);
    assert_eq!(app.app().name, "Chris陳");
    app.dispatch(Event::Backspace).unwrap();
    assert_eq!(app.app().name, "Chris");
    draw(&mut app);
    assert_eq!(
        app.dom()
            .get(app.dom().by_id("greeting").unwrap())
            .unwrap()
            .text_value(),
        Some("Hello, Chris.")
    );
    app.activate("dark-mode").unwrap();
    assert!(app.app().dark);
    draw(&mut app);
    assert_eq!(
        app.dom().by_role_name(Role::Checkbox, "Dark theme").len(),
        1
    );
    assert_eq!(
        app.dom()
            .get(app.dom().by_id("dark-mode").unwrap())
            .unwrap()
            .is_checked(),
        Some(true)
    );
    assert!(app.activate("disabled").is_err());
    assert_eq!(app.app().count, 2);
    app.activate("reset").unwrap();
    assert_eq!(app.app().count, 0);
}
#[test]
fn releasing_elsewhere_cancels_click_and_app_instances_are_independent() {
    let mut app = Runtime::new(Showcase::default()).unwrap();
    draw(&mut app);
    let rect = app.computed().unwrap().nodes[app.dom().by_id("increment").unwrap().index()].bounds;
    app.dispatch(Event::PointerDown {
        x: rect.x + 1.,
        y: rect.y + 1.,
    })
    .unwrap();
    app.dispatch(Event::PointerUp { x: 900., y: 790. }).unwrap();
    assert_eq!(app.app().count, 0);
    app.activate("increment").unwrap();
    let other = Runtime::new(Showcase::default()).unwrap();
    assert_eq!(other.app().count, 0);
}
#[test]
fn view_is_a_repeatable_function_of_state() {
    let mut a = Showcase::default();
    a.update(Message::Increment);
    let first = Runtime::new(a).unwrap();
    let second = Runtime::new(Showcase {
        count: 1,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(first.dom().snapshot(), second.dom().snapshot());
}

fn bounds(runtime: &Runtime<Showcase>, id: &str) -> Rect {
    let node = runtime.dom().by_id(id).unwrap();
    runtime.computed().unwrap().nodes[node.index()].bounds
}
fn visible(runtime: &Runtime<Showcase>, id: &str) -> bool {
    let node = runtime.dom().by_id(id).unwrap();
    let computed = &runtime.computed().unwrap().nodes[node.index()];
    let shown = computed.bounds.intersect(computed.clip);
    shown.width > 0. && shown.height == computed.bounds.height
}
fn scroll_y(runtime: &Runtime<Showcase>, id: &str) -> f32 {
    let node = runtime.dom().by_id(id).unwrap();
    runtime.dom().get(node).unwrap().style().scroll_y
}
fn focus_scroll_name(runtime: &mut Runtime<Showcase>) {
    for _ in 0..10 {
        if runtime.focused() == Some("scroll-name") {
            return;
        }
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        draw(runtime);
    }
    assert_eq!(runtime.focused(), Some("scroll-name"));
}
#[test]
fn scroll_fixture_clips_and_hides_the_field() {
    let mut app = Runtime::new(Showcase::default()).unwrap();
    draw(&mut app);
    let fixture = app.dom().by_id("scroll-fixture").unwrap();
    assert!(app.dom().get(fixture).unwrap().style().clip);
    assert_eq!(bounds(&app, "scroll-fixture").height, 160.);
    assert!(!visible(&app, "scroll-name"));
}
#[test]
fn tab_into_the_field_scrolls_the_container() {
    let mut app = Runtime::new(Showcase::default()).unwrap();
    draw(&mut app);
    assert_eq!(scroll_y(&app, "scroll-fixture"), 0.);
    focus_scroll_name(&mut app);
    assert_eq!(app.focused(), Some("scroll-name"));
    assert!(visible(&app, "scroll-name"));
    assert!(scroll_y(&app, "scroll-fixture") > 0.);
}
#[test]
fn caret_includes_the_scroll_offset_and_stays_visible() {
    let mut app = Runtime::new(Showcase::default()).unwrap();
    draw(&mut app);
    focus_scroll_name(&mut app);
    app.dispatch(Event::Text("abc".to_owned())).unwrap();
    draw(&mut app);
    let caret = app.input_snapshot().unwrap().caret.unwrap();
    let container = bounds(&app, "scroll-fixture");
    let field = bounds(&app, "scroll-name");
    assert!(caret.x >= container.x && caret.x + caret.width <= container.x + container.width);
    assert!(caret.y >= container.y && caret.y + caret.height <= container.y + container.height);
    assert!(caret.y >= field.y && caret.y + caret.height <= field.y + field.height);
}
#[test]
fn wheel_scroll_moves_the_field_and_caret_by_the_amount() {
    let mut app = Runtime::new(Showcase::default()).unwrap();
    draw(&mut app);
    focus_scroll_name(&mut app);
    app.dispatch(Event::Text("abc".to_owned())).unwrap();
    draw(&mut app);
    let field_y = bounds(&app, "scroll-name").y;
    let caret_y = app.input_snapshot().unwrap().caret.unwrap().y;
    let before = scroll_y(&app, "scroll-fixture");
    assert!(before >= 20.);
    let container = bounds(&app, "scroll-fixture");
    let handled = app
        .dispatch(Event::Scroll {
            x: container.x + container.width / 2.,
            y: container.y + 10.,
            delta_x: 0.,
            delta_y: -20.,
        })
        .unwrap();
    assert!(handled);
    draw(&mut app);
    assert!((scroll_y(&app, "scroll-fixture") - (before - 20.)).abs() < 0.01);
    assert!((bounds(&app, "scroll-name").y - (field_y + 20.)).abs() < 0.01);
    let caret_after = app.input_snapshot().unwrap().caret.unwrap().y;
    assert!((caret_after - (caret_y + 20.)).abs() < 0.01);
}
#[test]
fn existing_name_field_keeps_its_id_and_tab_order() {
    let mut app = Runtime::new(Showcase::default()).unwrap();
    draw(&mut app);
    assert!(app.dom().by_id("name").is_some());
    assert!(app.dom().by_id("surface-page").is_some());
    let mut order = Vec::new();
    for _ in 0..6 {
        app.dispatch(Event::FocusNext { reverse: false }).unwrap();
        order.push(app.focused().unwrap().to_owned());
        draw(&mut app);
    }
    assert_eq!(
        order,
        [
            "increment",
            "reset",
            "name",
            "dark-mode",
            "scroll-first",
            "scroll-name"
        ]
    );
}
