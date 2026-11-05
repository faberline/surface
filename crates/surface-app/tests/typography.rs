use surface::{Node, PaintOp, Role, Style, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};

struct App {
    style: Style,
}
#[derive(Clone)]
enum Message {
    /// The field only needs an input handler. The payload is never read.
    Text(#[allow(dead_code)] String),
}
impl Application for App {
    type Message = Message;
    fn update(&mut self, _: Message) {}
    fn view(&self) -> View<Message> {
        View::new(
            Node::new(Role::TextInput, "Name")
                .id("name")
                .text("abc")
                .styled(self.style.clone()),
        )
        .on_input(Message::Text)
    }
}
/// A size-only measurer, like the ones that predate text styles.
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size, size * 1.5)
    }
}

fn focused_runtime(style: Style) -> Runtime<App> {
    let mut runtime = Runtime::new(App { style }).unwrap();
    runtime.activate("name").unwrap();
    runtime
        .dispatch(Event::Select {
            anchor: 3,
            cursor: 3,
        })
        .unwrap();
    runtime
}
fn caret_height(style: Style) -> f32 {
    let mut runtime = focused_runtime(style);
    runtime.render(300., 120., &mut Metrics).unwrap();
    runtime.input_snapshot().unwrap().caret.unwrap().height
}

#[test]
fn caret_height_follows_styled_line_height() {
    // The contract case: a focused input with line height 30 paints a caret
    // quad that is exactly 30 high. The factor 30 / 24 is exact in f32.
    let mut runtime = focused_runtime(Style::default().padding(12.).line_height(30.));
    let paint = runtime.render(300., 120., &mut Metrics).unwrap();
    let caret = runtime.input_snapshot().unwrap().caret.unwrap();
    assert_eq!(caret.height, 30.);
    let quads: Vec<_> = paint
        .ops
        .iter()
        .filter_map(|op| match op {
            PaintOp::Quad { rect, .. } if *rect == caret => Some(rect),
            _ => None,
        })
        .collect();
    assert_eq!(quads.len(), 1, "one caret quad in {:?}", paint.ops);
    assert_eq!(quads[0].height, 30.);

    // Without a line height the caret is as tall as it always was.
    assert_eq!(caret_height(Style::default().padding(12.)), 24.);
    assert_eq!(
        caret_height(Style::default().padding(12.).font_size(20.)),
        30.
    );
    // A line height sets the caret height, larger or smaller than 1.5 times size.
    assert_eq!(
        caret_height(Style::default().padding(12.).line_height(40.)),
        40.
    );
    let short = caret_height(Style::default().padding(12.).line_height(18.));
    assert!((short - 18.).abs() < 0.01, "{short}");
    let sized = caret_height(
        Style::default()
            .padding(12.)
            .font_size(20.)
            .line_height(36.),
    );
    assert!((sized - 36.).abs() < 0.01, "{sized}");

    // The painted caret and the field text come from the same geometry.
    let mut runtime = focused_runtime(
        Style::default()
            .padding(12.)
            .font_size(14.)
            .line_height(20.)
            .font_weight(500),
    );
    let paint = runtime.render(300., 120., &mut Metrics).unwrap();
    let caret = runtime.input_snapshot().unwrap().caret.unwrap();
    assert!((caret.height - 20.).abs() < 0.01, "{caret:?}");
    assert!(paint.ops.iter().any(|op| matches!(
        op,
        PaintOp::StyledText { text, editable: true, style, .. }
            if text == "abc" && style.line_height == Some(20.) && style.weight == 500
    )));
    assert!(!paint
        .ops
        .iter()
        .any(|op| matches!(op, PaintOp::EditableText { .. })));
    assert!(paint.ops.iter().any(|op| matches!(
        op,
        PaintOp::Quad { rect, .. } if (rect.height - 20.).abs() < 0.01 && rect.width < 4.
    )));
}
