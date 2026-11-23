use surface::{
    Align, Color, Corners, DomSnapshot, Easing, ExtendedDisplayList, ExtendedPaintOp, Insets,
    Length, MicroDom, Node, PaintOp, RasterImage, Rect, Role, Style, TextMeasurer, Transition,
};
use surface_app::{Application, Event, Runtime, View};

const REST: Corners = Corners::new(16., 16., 4., 4.);
const HOVER: Corners = Corners::all(12.);
const ACTIVE: Corners = Corners::all(16.);
const BACKGROUND: Color = Color::rgb(28, 54, 75);
const INK: Color = Color::rgb(224, 194, 162);
const FOCUS: Color = Color::rgb(34, 124, 225);

#[derive(Clone)]
enum Message {
    Select,
    Open,
    Close,
    Disable,
}

struct App {
    animate: bool,
    selected: bool,
    disabled: bool,
    modal: bool,
    activations: u8,
    image: RasterImage,
}

impl App {
    fn new(animate: bool) -> Self {
        Self {
            animate,
            selected: false,
            disabled: false,
            modal: false,
            activations: 0,
            image: RasterImage::new(1, 1, vec![25, 97, 170, 255]).unwrap(),
        }
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Select => {
                self.selected = !self.selected;
                self.activations += 1;
            }
            Message::Open => self.modal = true,
            Message::Close => self.modal = false,
            Message::Disable => self.disabled = true,
        }
    }

    fn view(&self) -> View<Message> {
        let style = Style::column()
            .width(Length::Px(120.))
            .height(Length::Px(48.))
            .corners(if self.selected { ACTIVE } else { REST })
            .background(BACKGROUND)
            .state_layer(INK)
            .transition(Transition::new(100., Easing::Linear));
        let mut row = Node::new(Role::Checkbox, "Row")
            .id("row")
            .checked(self.selected)
            .disabled(self.disabled)
            .styled(style)
            .raster_image(self.image.clone())
            .state_layer_alphas(20, 26, 26)
            .focus_color(FOCUS)
            .focus_outline_width(3.);
        if !self.selected {
            row = row.interaction_corners(Some(HOVER), Some(ACTIVE), Some(ACTIVE));
        }
        if self.animate {
            row = row.animate_interaction_corners();
        }
        let modal = self.modal.then(|| {
            View::new(
                Node::new(Role::Dialog, "More")
                    .id("modal")
                    .styled(Style::column().fixed(Insets::all(0.))),
            )
            .modal(Message::Close)
            .children([
                View::new(Node::new(Role::Button, "Close").id("modal-close"))
                    .on_press(Message::Close),
            ])
        });
        View::new(
            Node::new(Role::Window, "Motion scene")
                .id("root")
                .styled(Style::column().align(Align::Start).clip(true)),
        )
        .children(std::iter::once(View::new(row).on_press(Message::Select)).chain(modal))
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size / 2., size * 1.5)
    }
}

fn frame(runtime: &mut Runtime<App>, at_ms: f64) -> ExtendedDisplayList {
    runtime.set_time(at_ms);
    draw(runtime)
}

fn draw(runtime: &mut Runtime<App>) -> ExtendedDisplayList {
    runtime.render_extended(200., 180., &mut Metrics).unwrap()
}

fn row_bounds(runtime: &Runtime<App>) -> Rect {
    let id = runtime.dom().by_id("row").unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn hover(runtime: &mut Runtime<App>) {
    runtime
        .dispatch(Event::PointerMove { x: 60., y: 24. })
        .unwrap();
}

fn assert_face(
    frame: &ExtendedDisplayList,
    bounds: Rect,
    expected: Corners,
    opacity: Option<u8>,
    focused: bool,
) {
    let mut background = None;
    let mut state_layer = None;
    let mut outline = None;
    let mut raster = None;
    for op in frame.ops() {
        let (rect, color, corners, stroke) = match op {
            ExtendedPaintOp::Raster(image) if image.rect() == bounds => {
                raster = Some(image.corners());
                continue;
            }
            ExtendedPaintOp::Legacy(PaintOp::Quad {
                rect,
                radius,
                color,
                ..
            }) => (*rect, *color, Corners::all(*radius), None),
            ExtendedPaintOp::Legacy(PaintOp::Shape {
                rect,
                corners,
                color,
                ..
            }) => (*rect, *color, *corners, None),
            ExtendedPaintOp::Legacy(PaintOp::Stroke {
                rect,
                radius,
                width,
                color,
                ..
            }) => (*rect, *color, Corners::all(*radius), Some(*width)),
            ExtendedPaintOp::Legacy(PaintOp::ShapeStroke {
                rect,
                corners,
                width,
                color,
                ..
            }) => (*rect, *color, *corners, Some(*width)),
            _ => continue,
        };
        if rect != bounds {
            continue;
        }
        if color == BACKGROUND {
            background = Some(corners);
        }
        if color == Color(INK.0, INK.1, INK.2, color.3) {
            state_layer = Some((corners, color.3));
        }
        if color == FOCUS {
            outline = Some((corners, stroke.unwrap()));
        }
    }
    assert_eq!(background, Some(expected));
    assert_eq!(raster, Some(expected));
    assert_eq!(state_layer, opacity.map(|alpha| (expected, alpha)));
    assert_eq!(outline, focused.then_some((expected, 3.)));
}

#[test]
fn hover_press_and_selection_share_the_presented_outline_without_moving_the_target() {
    let mut runtime = Runtime::new(App::new(true)).unwrap();
    let initial = frame(&mut runtime, 0.);
    let bounds = row_bounds(&runtime);
    assert_face(&initial, bounds, REST, None, false);

    hover(&mut runtime);
    assert_face(&frame(&mut runtime, 10.), bounds, REST, Some(20), false);
    assert!(runtime.needs_frame());
    let mid_hover = Corners::new(14., 14., 8., 8.);
    assert_face(
        &frame(&mut runtime, 60.),
        bounds,
        mid_hover,
        Some(20),
        false,
    );
    let id = runtime.dom().by_id("row").unwrap();
    assert_eq!(runtime.dom().get(id).unwrap().style().corner_radii(), REST);

    runtime
        .dispatch(Event::PointerDown { x: 60., y: 24. })
        .unwrap();
    assert_eq!(runtime.focused(), Some("row"));
    assert_face(&frame(&mut runtime, 60.), bounds, mid_hover, Some(26), true);
    let mid_press = Corners::new(15., 15., 12., 12.);
    assert_face(
        &frame(&mut runtime, 110.),
        bounds,
        mid_press,
        Some(26),
        true,
    );
    assert_eq!(runtime.app().activations, 0);

    runtime
        .dispatch(Event::PointerUp { x: 60., y: 24. })
        .unwrap();
    assert_eq!(runtime.app().activations, 1);
    assert!(runtime.app().selected);
    assert_face(
        &frame(&mut runtime, 110.),
        bounds,
        mid_press,
        Some(26),
        true,
    );
    assert_face(&frame(&mut runtime, 160.), bounds, ACTIVE, Some(26), true);
    assert_eq!(row_bounds(&runtime), bounds);
    assert!(!runtime.needs_frame());
}

#[test]
fn leaving_hover_reverses_from_its_last_pose_and_reduce_motion_finishes_it() {
    let mut runtime = Runtime::new(App::new(true)).unwrap();
    frame(&mut runtime, 0.);
    let bounds = row_bounds(&runtime);
    hover(&mut runtime);
    frame(&mut runtime, 0.);
    let turn = Corners::new(15., 15., 6., 6.);
    assert_face(&frame(&mut runtime, 25.), bounds, turn, Some(20), false);
    runtime.dispatch(Event::PointerLeave).unwrap();
    assert_face(&frame(&mut runtime, 25.), bounds, turn, None, false);
    assert_face(
        &frame(&mut runtime, 75.),
        bounds,
        Corners::new(15.5, 15.5, 5., 5.),
        None,
        false,
    );
    assert!(runtime.needs_frame());

    runtime.set_reduced_motion(true);
    assert_face(&frame(&mut runtime, 80.), bounds, REST, None, false);
    assert!(!runtime.needs_frame());
    hover(&mut runtime);
    assert_face(&frame(&mut runtime, 80.), bounds, HOVER, Some(20), false);
    assert!(!runtime.needs_frame());
}

#[test]
fn modal_scope_and_disabled_rows_discard_running_corner_motion() {
    let mut runtime = Runtime::new(App::new(true)).unwrap();
    frame(&mut runtime, 0.);
    let bounds = row_bounds(&runtime);
    hover(&mut runtime);
    frame(&mut runtime, 0.);
    frame(&mut runtime, 25.);
    assert!(runtime.needs_frame());
    runtime.apply(Message::Open).unwrap();
    let dimmed = frame(&mut runtime, 25.);
    assert_face(&dimmed, bounds, REST, None, false);
    assert!(!runtime.needs_frame());

    runtime.dispatch(Event::PointerLeave).unwrap();
    runtime.apply(Message::Close).unwrap();
    assert_face(&frame(&mut runtime, 25.), bounds, REST, None, false);
    hover(&mut runtime);
    frame(&mut runtime, 25.);
    frame(&mut runtime, 50.);
    assert!(runtime.needs_frame());
    runtime.apply(Message::Disable).unwrap();
    assert_face(&frame(&mut runtime, 50.), bounds, REST, None, false);
    assert!(!runtime.needs_frame());
}

#[test]
fn legacy_and_no_clock_paint_at_the_immediate_endpoint_and_snapshot_wire_is_unchanged() {
    let mut ordinary = Runtime::new(App::new(false)).unwrap();
    let mut animated = Runtime::new(App::new(true)).unwrap();
    let before = serde_json::to_vec(&ordinary.snapshot().dom).unwrap();
    assert_eq!(
        serde_json::to_vec(&animated.snapshot().dom).unwrap(),
        before
    );
    let decoded: DomSnapshot = serde_json::from_slice(&before).unwrap();
    let mut dom = MicroDom::from_snapshot(decoded).unwrap();
    let id = dom.by_id("row").unwrap();
    dom.set_presented_interaction_corners(id, Corners::all(f32::NAN))
        .unwrap();
    dom.set_presented_interaction_corners(id, Corners::all(-1.))
        .unwrap();
    assert_eq!(dom.get(id).unwrap().style().corner_radii(), REST);

    frame(&mut ordinary, 0.);
    draw(&mut animated);
    for runtime in [&mut ordinary, &mut animated] {
        let bounds = row_bounds(runtime);
        hover(runtime);
        assert_face(&draw(runtime), bounds, HOVER, Some(20), false);
        assert!(!runtime.needs_frame());
    }
}
