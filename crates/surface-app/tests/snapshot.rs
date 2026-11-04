use surface::{Color, Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, DomExport, Event, ImeEvent, Runtime, View};

#[derive(Default)]
struct App {
    name: String,
}
#[derive(Clone)]
enum Message {
    Name(String),
}
impl Application for App {
    type Message = Message;
    fn update(&mut self, message: Message) {
        let Message::Name(value) = message;
        self.name = value;
    }
    fn view(&self) -> View<Message> {
        View::new(
            Node::new(Role::Window, "Export test")
                .id("root")
                .styled(Style::column().clip(true)),
        )
        .children([View::new(
            Node::new(Role::TextInput, "姓名")
                .id("name")
                .text(&self.name)
                .styled(
                    Style::default()
                        .height(Length::Px(48.))
                        .background(Color::rgb(255, 255, 255)),
                ),
        )
        .on_input(Message::Name)])
    }
}
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size, size * 1.5)
    }
}
#[test]
fn export_tracks_live_state_and_never_labels_old_geometry_as_current() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    let first: DomExport = serde_json::from_str(&runtime.export_dom().unwrap()).unwrap();
    assert_eq!(first.version, 1);
    assert!(first.layout.is_none());
    assert_eq!(first.dom.nodes.len(), 2);
    assert_eq!(first.dom.nodes[1].parent().unwrap().index(), 0);
    runtime.activate("name").unwrap();
    runtime.render(320., 180., &mut Metrics).unwrap();
    let rendered = runtime.snapshot();
    assert_eq!(rendered.interaction.focused.as_deref(), Some("name"));
    let layout = rendered.layout.as_ref().unwrap();
    assert_eq!(layout.nodes.len(), 2);
    assert_eq!(layout.nodes[1].bounds.height, 48.);
    let before = runtime.export_dom().unwrap();
    assert_eq!(
        before,
        runtime.export_dom().unwrap(),
        "read-only and deterministic"
    );
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄋㄧ".into(),
            selection: Some((6, 6)),
        }))
        .unwrap();
    let pending = runtime.snapshot();
    assert!(
        pending.layout.is_none(),
        "the previous frame used different text"
    );
    assert_eq!(pending.dom.nodes[1].text_value(), Some(""));
    assert_eq!(pending.text_inputs["name"].value, "");
    assert_eq!(pending.text_inputs["name"].display_value, "ㄋㄧ");
    assert!(pending.text_inputs["name"].composing);
    assert_eq!(runtime.app().name, "");
    let before_rejected_action = runtime.export_dom().unwrap();
    assert!(runtime.activate("missing").is_err());
    assert_eq!(
        runtime.export_dom().unwrap(),
        before_rejected_action,
        "a rejected action must not cancel the active preedit or mutate the capture"
    );
    runtime.render(320., 180., &mut Metrics).unwrap();
    assert!(runtime.snapshot().layout.is_some());
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("你".into())))
        .unwrap();
    let committed: DomExport = serde_json::from_str(&runtime.export_dom().unwrap()).unwrap();
    assert!(committed.layout.is_none());
    assert_eq!(committed.dom.nodes[1].text_value(), Some("你"));
    assert_eq!(committed.text_inputs["name"].value, "你");
    assert!(!committed.text_inputs["name"].composing);
    assert_eq!(committed.text_inputs["name"].selection_cursor, 3);
    assert_eq!(runtime.app().name, "你");
    runtime.render(320., 180., &mut Metrics).unwrap();
    runtime
        .dispatch(Event::Scroll {
            x: 1.,
            y: 1.,
            delta_x: 0.,
            delta_y: 20.,
        })
        .unwrap();
    assert!(
        runtime.snapshot().layout.is_none(),
        "scroll invalidates old geometry"
    );
}
