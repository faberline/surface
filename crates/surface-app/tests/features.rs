//! Motion, per-corner shapes and text styles in one runtime.
//!
//! Each feature has its own tests. This one checks that they compose: the
//! display copy that motion blends must keep `corners` and the text style,
//! and layout must keep using the target style.
use surface::{
    Color, Corners, DisplayList, Easing, Length, Node, PaintOp, Role, Style, TextMeasurer,
    Transition,
};
use surface_app::{Application, Runtime, View};

const FROM: Color = Color::rgb(0, 0, 0);
const TO: Color = Color::rgb(200, 100, 60);
const HALF: Color = Color(100, 50, 30, 255);
const TOP: Corners = Corners::top(12.);

struct App {
    on: bool,
}

impl Application for App {
    type Message = ();
    fn update(&mut self, _: ()) {
        self.on = !self.on;
    }
    fn view(&self) -> View<()> {
        let color = if self.on { TO } else { FROM };
        let transition = Transition::new(100., Easing::Linear);
        View::new(
            Node::new(Role::Window, "Features")
                .id("root")
                .styled(Style::column()),
        )
        .children([
            View::new(
                Node::new(Role::Group, "chip").id("chip").styled(
                    Style::default()
                        .width(Length::Px(120.))
                        .height(Length::Px(48.))
                        // `radius` changes with the state too. The shape in
                        // `corners` must still win while it is blended.
                        .radius(if self.on { 20. } else { 4. })
                        .corners(TOP)
                        .background(color)
                        .transition(transition),
                ),
            ),
            View::new(
                Node::new(Role::Text, "Label")
                    .id("label")
                    .text("Label")
                    .styled(
                        Style::default()
                            .font_size(24.)
                            .line_height(32.)
                            .font_weight(500)
                            .foreground(color)
                            .transition(transition),
                    ),
            ),
            View::new(
                Node::new(Role::Button, "Toggle")
                    .id("toggle")
                    .text("Toggle")
                    .styled(Style::default().height(Length::Px(40.))),
            )
            .on_press(()),
        ])
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size / 2., size * 1.5)
    }
}

fn frame(runtime: &mut Runtime<App>) -> DisplayList {
    runtime.render(320., 240., &mut Metrics).unwrap()
}

/// The fill color of the shape painted at the chip bounds.
fn chip_fill(runtime: &Runtime<App>, list: &DisplayList) -> Color {
    let node = runtime.dom().by_id("chip").expect("chip exists");
    let bounds = runtime.computed().expect("frame rendered").nodes[node.index()].bounds;
    list.ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Shape {
                rect,
                corners,
                color,
                ..
            } if *rect == bounds => {
                assert_eq!(*corners, TOP, "the blended copy keeps the corner shape");
                Some(*color)
            }
            _ => None,
        })
        .expect("an asymmetric chip paints as a Shape op")
}

/// The color of the styled text run, after checking its text style.
fn label_color(list: &DisplayList) -> Color {
    list.ops
        .iter()
        .find_map(|op| match op {
            PaintOp::StyledText {
                text,
                style,
                color,
                editable,
                ..
            } if text == "Label" => {
                assert!(!editable);
                assert_eq!(style.font_size, 24.);
                assert_eq!(style.line_height, Some(32.));
                assert_eq!(style.weight, 500);
                Some(*color)
            }
            _ => None,
        })
        .expect("a styled label paints as StyledText")
}

#[test]
fn blended_copy_keeps_corners_and_text_style() {
    let mut runtime = Runtime::new(App { on: false }).unwrap();
    runtime.set_time(0.);
    let list = frame(&mut runtime);
    assert_eq!(chip_fill(&runtime, &list), FROM);
    assert_eq!(label_color(&list), FROM);

    runtime.activate("toggle").unwrap();
    // The transition starts at the first frame that sees the new target.
    let list = frame(&mut runtime);
    assert!(runtime.needs_frame());
    assert_eq!(chip_fill(&runtime, &list), FROM);
    assert_eq!(label_color(&list), FROM);

    runtime.set_time(50.);
    let list = frame(&mut runtime);
    assert!(runtime.needs_frame());
    assert_eq!(chip_fill(&runtime, &list), HALF);
    assert_eq!(label_color(&list), HALF);

    runtime.set_time(100.);
    let list = frame(&mut runtime);
    assert!(!runtime.needs_frame());
    assert_eq!(chip_fill(&runtime, &list), TO);
    assert_eq!(label_color(&list), TO);
}

#[test]
fn layout_uses_the_target_style_and_line_height_while_blending() {
    let mut runtime = Runtime::new(App { on: false }).unwrap();
    runtime.set_time(0.);
    frame(&mut runtime);
    let before = runtime.computed().unwrap().clone();
    runtime.activate("toggle").unwrap();
    frame(&mut runtime);
    runtime.set_time(50.);
    frame(&mut runtime);
    let label = runtime.dom().by_id("label").unwrap().index();
    let during = runtime.computed().unwrap();
    assert_eq!(
        during.nodes[label].bounds.height, 32.,
        "the 32 pixel line height sets the node height"
    );
    assert_eq!(
        during.nodes[label].bounds, before.nodes[label].bounds,
        "a color transition never moves layout"
    );
}
