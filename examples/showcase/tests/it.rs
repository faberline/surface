use surface::{Role, TextMeasurer};
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
