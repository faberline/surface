use surface::{
    BoxShadow, Color, Corners, ExtendedDisplayList, ExtendedPaintOp, Insets, Length, Node, PaintOp,
    PathFill, Rect, Role, Style, TextMeasurer,
};
use surface_app::{
    AnimationFrame, Application, Event, FrameInput, FramePress, FramePressPhase,
    InputAnimationUpdate, Runtime, View,
};

const BODY: &str = "body";
const BODY_FACE: &str = "body-face";
const REMOVE: &str = "remove";
const REMOVE_FACE: &str = "remove-face";
const BODY_INK: Color = Color(46, 48, 81, 255);
const REMOVE_INK: Color = Color(87, 31, 10, 255);
const HOVER_SHADOW: Color = Color(15, 75, 100, 255);

#[derive(Clone)]
enum Message {
    Body,
    Remove,
    Retarget,
    MoveFace,
}

struct App {
    body_face: &'static str,
    requested_face: &'static str,
    face_offset: f32,
    taps: Vec<&'static str>,
    press: Option<(&'static str, Rect, FramePressPhase)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            body_face: BODY_FACE,
            requested_face: BODY_FACE,
            face_offset: 0.,
            taps: Vec::new(),
            press: None,
        }
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Body => self.taps.push(BODY),
            Message::Remove => self.taps.push(REMOVE),
            Message::Retarget => {
                self.body_face = "replacement-face";
                self.requested_face = self.body_face;
            }
            Message::MoveFace => self.face_offset = 6.,
        }
    }

    fn observes_frame_input(&self) -> bool {
        true
    }

    fn animation_frame_with_press_detail(
        &mut self,
        _: AnimationFrame<'_>,
        _: FrameInput<'_>,
        _: Option<&str>,
        press: Option<FramePress<'_>>,
        _: Option<&str>,
    ) -> InputAnimationUpdate {
        if let Some(press) = press {
            let id = match press.id {
                BODY => BODY,
                REMOVE => REMOVE,
                _ => panic!("unexpected press owner"),
            };
            self.press = Some((id, press.bounds, press.phase));
        }
        InputAnimationUpdate::default()
    }

    fn view(&self) -> View<Message> {
        let action = |id, target| {
            Node::new(Role::Button, id)
                .id(id)
                .styled(
                    Style::column()
                        .width(Length::Px(48.))
                        .height(Length::Px(48.)),
                )
                .press_ripple_to(target)
        };
        let face = |id, x, y, width, height, ink, radius| {
            Node::new(Role::Group, "")
                .id(id)
                .styled(
                    Style::column()
                        .width(Length::Px(width))
                        .height(Length::Px(height))
                        .absolute(Insets {
                            left: Length::Px(x),
                            top: Length::Px(y),
                            ..Insets::default()
                        })
                        .corners(Corners::all(radius))
                        .background(Color::rgb(224, 215, 240))
                        .state_layer(Color::rgb(15, 38, 49)),
                )
                .press_state_layer_ink(ink)
                .state_layer_alphas(20, 26, 26)
        };
        let body_face = face(self.body_face, self.face_offset, 8., 88., 32., BODY_INK, 8.)
            .interaction_box_shadows(
                Some([
                    BoxShadow::new(0., 3., 0., 0., HOVER_SHADOW),
                    BoxShadow::NONE,
                ]),
                None,
                None,
            )
            .interaction_box_shadows_from_parent();
        let remove_face = face(REMOVE_FACE, 12., 12., 24., 24., REMOVE_INK, 12.);
        View::new(Node::new(Role::Window, "Feedback").styled(Style::row())).children([
            View::new(action(BODY, self.requested_face))
                .on_press(Message::Body)
                .children([View::new(body_face).without_interaction(None)]),
            View::new(action(REMOVE, REMOVE_FACE))
                .on_press(Message::Remove)
                .children([View::new(remove_face).without_interaction(None)]),
        ])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn frame(runtime: &mut Runtime<App>, time: f64) -> ExtendedDisplayList {
    runtime.set_time(time);
    runtime.render_extended(160., 96., &mut Metrics).unwrap()
}

fn bounds(runtime: &Runtime<App>, key: &str) -> Rect {
    let id = runtime.dom().by_id(key).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn wave(display: &ExtendedDisplayList) -> Option<&PathFill> {
    let mut paths = display.ops().iter().filter_map(|op| match op {
        ExtendedPaintOp::FillPath(path) => Some(path),
        _ => None,
    });
    let path = paths.next();
    assert!(paths.next().is_none());
    path
}

fn tinted_rect(display: &ExtendedDisplayList, ink: Color) -> Option<Rect> {
    display.ops().iter().find_map(|op| match op {
        ExtendedPaintOp::Legacy(PaintOp::Quad { color, rect, .. })
        | ExtendedPaintOp::Legacy(PaintOp::Shape { color, rect, .. })
            if *color == ink =>
        {
            Some(*rect)
        }
        _ => None,
    })
}

fn primary(x: f32, y: f32, down: bool) -> Event {
    if down {
        Event::PointerDown { x, y }
    } else {
        Event::PointerUp { x, y }
    }
}

fn press_ink(ink: Color) -> Color {
    Color(ink.0, ink.1, ink.2, 26)
}

#[test]
fn the_face_uses_its_own_press_ink_and_bounds_without_changing_the_hit_area() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    frame(&mut runtime, 0.);
    let owner = bounds(&runtime, BODY);
    let face = bounds(&runtime, BODY_FACE);
    assert_eq!(owner, Rect::new(0., 0., 48., 48.));
    assert_eq!(face, Rect::new(0., 8., 88., 32.));

    runtime
        .dispatch(Event::PointerMove { x: 25., y: 2. })
        .unwrap();
    let hover = frame(&mut runtime, 1.);
    let shadow = tinted_rect(&hover, HOVER_SHADOW).expect("hover shadow");
    assert_eq!(
        shadow,
        Rect::new(face.x, face.y + 3., face.width, face.height)
    );

    runtime.dispatch(primary(25., 2., true)).unwrap();
    let first = frame(&mut runtime, 10.);
    assert!(wave(&first).is_none());
    assert!(tinted_rect(&first, press_ink(BODY_INK)).is_none());
    assert_eq!(
        runtime.app().press,
        Some((BODY, owner, FramePressPhase::Held))
    );

    let growing = frame(&mut runtime, 85.);
    let partial = wave(&growing).unwrap();
    assert_eq!(partial.color(), press_ink(BODY_INK));
    assert_eq!(
        partial.rect().y,
        face.y,
        "margin press clamps to top of the face"
    );
    assert!(partial.rect().width < face.width);
    assert!(tinted_rect(&growing, press_ink(BODY_INK)).is_none());

    let expanded = frame(&mut runtime, 235.);
    assert_eq!(wave(&expanded).unwrap().rect(), face);
    runtime.dispatch(primary(25., 2., false)).unwrap();
    let released = frame(&mut runtime, 235.);
    assert_eq!(wave(&released).unwrap().rect(), face);
    assert_eq!(runtime.app().taps, [BODY]);
    assert_eq!(
        runtime.app().press,
        Some((BODY, owner, FramePressPhase::Released))
    );

    runtime.dispatch_touch(primary(72., 24., true)).unwrap();
    frame(&mut runtime, 240.);
    let removing = frame(&mut runtime, 465.);
    assert_eq!(
        wave(&removing).unwrap().rect(),
        bounds(&runtime, REMOVE_FACE)
    );
    assert_eq!(wave(&removing).unwrap().color(), press_ink(REMOVE_INK));
    runtime.dispatch_touch(primary(72., 24., false)).unwrap();
    assert_eq!(runtime.app().taps, [BODY, REMOVE]);
}

#[test]
fn invalid_sibling_target_uses_static_feedback_and_does_not_paint_a_wave() {
    let mut runtime = Runtime::new(App {
        requested_face: REMOVE_FACE,
        ..App::default()
    })
    .unwrap();
    frame(&mut runtime, 0.);
    runtime.dispatch(primary(25., 2., true)).unwrap();
    let image = frame(&mut runtime, 75.);
    assert!(wave(&image).is_none());
    assert_eq!(
        tinted_rect(&image, press_ink(BODY_INK)),
        Some(bounds(&runtime, BODY_FACE))
    );
    runtime.dispatch(primary(25., 2., false)).unwrap();
    assert_eq!(runtime.app().taps, [BODY]);
}

#[test]
fn rebuilding_with_a_different_face_cancels_the_in_flight_wave() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    frame(&mut runtime, 0.);
    runtime.dispatch(primary(25., 12., true)).unwrap();
    frame(&mut runtime, 10.);
    assert!(wave(&frame(&mut runtime, 85.)).is_some());
    runtime.apply(Message::Retarget).unwrap();
    let rebuilt = frame(&mut runtime, 120.);
    assert!(wave(&rebuilt).is_none());
    assert_eq!(
        tinted_rect(&rebuilt, press_ink(BODY_INK)),
        Some(bounds(&runtime, "replacement-face"))
    );
    assert_eq!(runtime.focused(), Some(BODY));
    runtime.dispatch(primary(25., 12., false)).unwrap();
    assert_eq!(runtime.app().taps, [BODY]);
}

#[test]
fn moving_only_the_face_cancels_an_accepted_wave() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    frame(&mut runtime, 0.);
    runtime.dispatch(primary(25., 12., true)).unwrap();
    frame(&mut runtime, 10.);
    assert!(wave(&frame(&mut runtime, 85.)).is_some());
    runtime.apply(Message::MoveFace).unwrap();
    let rebuilt = frame(&mut runtime, 120.);
    assert!(wave(&rebuilt).is_none());
    assert_eq!(
        tinted_rect(&rebuilt, press_ink(BODY_INK)),
        Some(bounds(&runtime, BODY_FACE))
    );
    runtime.dispatch(primary(25., 12., false)).unwrap();
    assert_eq!(runtime.app().taps, [BODY]);
}

#[test]
fn reduced_motion_paints_the_same_face_without_a_radial_wave() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    runtime.set_reduced_motion(true);
    frame(&mut runtime, 0.);
    runtime.dispatch(primary(25., 2., true)).unwrap();
    let image = frame(&mut runtime, 80.);
    assert!(wave(&image).is_none());
    assert_eq!(
        tinted_rect(&image, press_ink(BODY_INK)),
        Some(bounds(&runtime, BODY_FACE))
    );
    runtime.dispatch(primary(25., 2., false)).unwrap();
    assert_eq!(runtime.app().taps, [BODY]);
}
