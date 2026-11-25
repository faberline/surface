use surface::{
    Color, Corners, ExtendedDisplayList, ExtendedPaintOp, Insets, Length, Node, PaintOp, PathFill,
    Rect, Role, Style, TextMeasurer,
};
use surface_app::{
    AnimationFrame, Application, Event, FrameInput, FramePress, FramePressPhase,
    InputAnimationUpdate, PointDragSession, Runtime, View,
};

const ROW: &str = "row";
const INK: Color = Color::rgb(26, 44, 77);
const PRESS_INK: Color = Color::rgb(63, 35, 114);

#[derive(Clone)]
enum Message {
    Activate,
    Observe,
    Show(bool),
    Role(Role),
    Open,
    Close,
}

struct App {
    observe: bool,
    passive_tab: bool,
    shown: bool,
    role: Role,
    modal: bool,
    drag: Option<PointDragSession>,
    taps: usize,
    reports: Vec<(FramePressPhase, u64)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            observe: false,
            passive_tab: false,
            shown: true,
            role: Role::Button,
            modal: false,
            drag: None,
            taps: 0,
            reports: Vec::new(),
        }
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Activate => self.taps += 1,
            Message::Observe => self.observe = true,
            Message::Show(value) => self.shown = value,
            Message::Role(role) => self.role = role,
            Message::Open => self.modal = true,
            Message::Close => self.modal = false,
        }
    }

    fn observes_frame_input(&self) -> bool {
        self.observe
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
            self.reports.push((press.phase, press.sequence));
        }
        InputAnimationUpdate::default()
    }

    fn view(&self) -> View<Message> {
        let row = self.shown.then(|| {
            let node = Node::new(self.role, "Row")
                .id(ROW)
                .styled(
                    Style::column()
                        .width(Length::Px(240.))
                        .height(Length::Px(60.))
                        .corners(Corners::new(4., 16., 16., 4.))
                        .state_layer(INK),
                )
                .state_layer_alphas(20, 26, 26);
            let row = if self.passive_tab {
                View::new(
                    node.press_state_layer_ink(PRESS_INK)
                        .press_ripple_on_focusable(),
                )
            } else {
                View::new(node.press_ripple(None)).on_press(Message::Activate)
            };
            if let Some(session) = &self.drag {
                row.on_point_drag_with_bounds(session.clone(), |_, _| None)
            } else {
                row
            }
        });
        let scroll = View::new(
            Node::new(Role::Group, "Scroll").id("scroll").styled(
                Style::column()
                    .width(Length::Px(240.))
                    .height(Length::Px(80.))
                    .clip(true),
            ),
        )
        .children(row.into_iter().chain([View::new(
            Node::new(Role::Group, "").styled(Style::column().height(Length::Px(240.))),
        )]));
        let modal = self.modal.then(|| {
            View::new(
                Node::new(Role::Dialog, "Dialog")
                    .id("modal")
                    .styled(Style::column().fixed(Insets::all(0.))),
            )
            .modal_overlay(Message::Close)
        });
        View::new(Node::new(Role::Window, "Ripple")).children(std::iter::once(scroll).chain(modal))
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn draw(runtime: &mut Runtime<App>) -> ExtendedDisplayList {
    runtime.render_extended(320., 220., &mut Metrics).unwrap()
}

fn frame(runtime: &mut Runtime<App>, time: f64) -> ExtendedDisplayList {
    runtime.set_time(time);
    draw(runtime)
}

fn event(runtime: &mut Runtime<App>, event: Event) {
    runtime.dispatch(event).unwrap();
}

fn down(runtime: &mut Runtime<App>) {
    event(runtime, Event::PointerDown { x: 24., y: 28. });
}

fn up(runtime: &mut Runtime<App>) {
    event(runtime, Event::PointerUp { x: 24., y: 28. });
}

fn row(runtime: &Runtime<App>) -> Rect {
    let id = runtime.dom().by_id(ROW).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn wave(frame: &ExtendedDisplayList) -> Option<&PathFill> {
    let mut paths = frame.ops().iter().filter_map(|op| match op {
        ExtendedPaintOp::FillPath(path) => Some(path),
        _ => None,
    });
    let wave = paths.next();
    assert!(paths.next().is_none(), "a pointer sequence has one wave");
    wave
}

fn steady_press(frame: &ExtendedDisplayList) -> bool {
    has_tint(frame, INK, 26)
}

fn has_tint(frame: &ExtendedDisplayList, ink: Color, alpha: u8) -> bool {
    frame.ops().iter().any(|op| {
        let color = match op {
            ExtendedPaintOp::Legacy(PaintOp::Quad { color, .. })
            | ExtendedPaintOp::Legacy(PaintOp::Shape { color, .. }) => color,
            _ => return false,
        };
        *color == Color(ink.0, ink.1, ink.2, alpha)
    })
}

#[test]
fn a_selected_tab_can_paint_a_distinct_press_without_an_action() {
    let mut runtime = Runtime::new(App {
        role: Role::Tab,
        passive_tab: true,
        ..App::default()
    })
    .unwrap();
    frame(&mut runtime, 0.);
    event(&mut runtime, Event::PointerMove { x: 24., y: 28. });
    assert!(has_tint(&frame(&mut runtime, 0.), INK, 20));

    down(&mut runtime);
    frame(&mut runtime, 1000.);
    let held = frame(&mut runtime, 1075.);
    let expected = Color(PRESS_INK.0, PRESS_INK.1, PRESS_INK.2, 26);
    assert_eq!(wave(&held).unwrap().color(), expected);
    assert!(!has_tint(&held, PRESS_INK, 26));
    assert!(!steady_press(&held));

    up(&mut runtime);
    assert_eq!(runtime.focused(), Some(ROW));
    assert!(!runtime.activate(ROW).unwrap());
    assert_eq!(runtime.app().taps, 0);

    runtime.set_reduced_motion(true);
    down(&mut runtime);
    let reduced = frame(&mut runtime, 1175.);
    assert!(wave(&reduced).is_none());
    assert!(has_tint(&reduced, PRESS_INK, 26));
    assert!(!steady_press(&reduced));
    up(&mut runtime);
    assert!(steady_press(&frame(&mut runtime, 1200.)));
    assert_eq!(runtime.app().taps, 0);
}

#[test]
fn held_press_starts_transparent_grows_to_cover_and_fades_after_release() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    frame(&mut runtime, 0.);
    let bounds = row(&runtime);
    down(&mut runtime);

    let first = frame(&mut runtime, 1000.);
    assert!(wave(&first).is_none());
    assert!(
        !steady_press(&first),
        "the held first sample must not flash across the row"
    );
    assert!(runtime.needs_frame());

    let growing = frame(&mut runtime, 1075.);
    let early = wave(&growing).unwrap();
    assert_eq!(early.color().3, 26);
    assert!(early.rect().contains(24., 28.));
    assert!(early.rect().width < bounds.width);
    assert!(!steady_press(&growing));

    let covered = frame(&mut runtime, 1225.);
    assert_eq!(wave(&covered).unwrap().rect(), bounds);
    assert!(!runtime.needs_frame());
    assert_eq!(runtime.app().taps, 0);

    up(&mut runtime);
    assert_eq!(wave(&frame(&mut runtime, 1225.)).unwrap().color().3, 26);
    assert_eq!(wave(&frame(&mut runtime, 1300.)).unwrap().color().3, 13);
    assert!(wave(&frame(&mut runtime, 1375.)).is_none());
    assert_eq!(runtime.app().taps, 1);
    assert_eq!(runtime.focused(), Some(ROW));
    assert_eq!(runtime.app().reports, []);
    assert!(!runtime.needs_frame());
}

#[test]
fn a_coalesced_touch_release_still_paints_and_does_not_leak_to_an_app_observer() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    frame(&mut runtime, 0.);
    runtime
        .dispatch_touch(Event::PointerDown { x: 225., y: 26. })
        .unwrap();
    runtime
        .dispatch_touch(Event::PointerUp { x: 225., y: 26. })
        .unwrap();
    runtime.apply(Message::Observe).unwrap();
    let started = frame(&mut runtime, 25.);
    let pulse = wave(&started).unwrap();
    assert!(pulse.rect().contains(225., 26.));
    assert!(!pulse.rect().contains(20., 26.));
    assert!(runtime.app().reports.is_empty());
    assert_eq!(runtime.app().taps, 1);

    // The next sequence is visible to the app, and its terminal is delivered once.
    down(&mut runtime);
    frame(&mut runtime, 25.);
    let [(FramePressPhase::Held, sequence)] = runtime.app().reports.as_slice() else {
        panic!("observer did not receive exactly one held record");
    };
    let sequence = *sequence;
    up(&mut runtime);
    assert!(wave(&frame(&mut runtime, 30.)).is_some());
    assert!(wave(&frame(&mut runtime, 45.)).is_some());
    assert_eq!(
        runtime.app().reports,
        [
            (FramePressPhase::Held, sequence),
            (FramePressPhase::Released, sequence)
        ]
    );
    assert_eq!(runtime.app().taps, 2);
}

#[test]
fn rejected_release_scope_replacement_and_geometry_changes_clear_the_wave() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    frame(&mut runtime, 0.);
    down(&mut runtime);
    frame(&mut runtime, 0.);
    assert!(wave(&frame(&mut runtime, 75.)).is_some());
    event(&mut runtime, Event::PointerUp { x: 280., y: 35. });
    assert!(wave(&frame(&mut runtime, 75.)).is_none());
    assert_eq!(runtime.app().taps, 0);

    down(&mut runtime);
    up(&mut runtime);
    runtime.apply(Message::Show(false)).unwrap();
    runtime.apply(Message::Show(true)).unwrap();
    assert!(wave(&frame(&mut runtime, 80.)).is_none());

    down(&mut runtime);
    up(&mut runtime);
    runtime.apply(Message::Role(Role::Checkbox)).unwrap();
    runtime.apply(Message::Role(Role::Button)).unwrap();
    assert!(wave(&frame(&mut runtime, 85.)).is_none());

    down(&mut runtime);
    up(&mut runtime);
    assert!(wave(&frame(&mut runtime, 90.)).is_some());
    runtime.apply(Message::Open).unwrap();
    runtime.apply(Message::Close).unwrap();
    assert!(wave(&frame(&mut runtime, 95.)).is_none());

    down(&mut runtime);
    up(&mut runtime);
    assert!(wave(&frame(&mut runtime, 100.)).is_some());
    assert!(runtime.pointer_geometry_changed().unwrap());
    assert!(wave(&frame(&mut runtime, 105.)).is_none());

    down(&mut runtime);
    up(&mut runtime);
    assert!(wave(&frame(&mut runtime, 110.)).is_some());
    event(&mut runtime, Event::Blur);
    assert!(wave(&frame(&mut runtime, 115.)).is_none());
    assert_eq!(runtime.app().taps, 5);
    assert!(!runtime.needs_frame());
}

#[test]
fn reduced_motion_and_no_clock_keep_the_steady_feedback_without_rearming() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    draw(&mut runtime);
    down(&mut runtime);
    let still = draw(&mut runtime);
    assert!(steady_press(&still));
    assert!(wave(&still).is_none());
    assert!(!runtime.needs_frame());
    let clock_started = frame(&mut runtime, 500.);
    assert!(steady_press(&clock_started));
    assert!(wave(&clock_started).is_none());
    up(&mut runtime);
    frame(&mut runtime, 500.);

    down(&mut runtime);
    frame(&mut runtime, 500.);
    assert!(wave(&frame(&mut runtime, 575.)).is_some());
    runtime.set_reduced_motion(true);
    let reduced = frame(&mut runtime, 575.);
    assert!(steady_press(&reduced));
    assert!(wave(&reduced).is_none());
    assert!(!runtime.needs_frame());
    runtime.set_reduced_motion(false);
    assert!(wave(&frame(&mut runtime, 600.)).is_none());
    up(&mut runtime);
    assert!(wave(&frame(&mut runtime, 600.)).is_none());
    assert_eq!(runtime.app().taps, 2);
}

#[test]
fn scroll_translates_the_local_wave_and_full_clip_retires_the_sequence() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    frame(&mut runtime, 0.);
    down(&mut runtime);
    frame(&mut runtime, 0.);
    let first = frame(&mut runtime, 75.);
    let start = wave(&first).unwrap();
    let bounds = row(&runtime);
    let scroll = |runtime: &mut Runtime<App>, delta_y| {
        event(
            runtime,
            Event::Scroll {
                x: 24.,
                y: 28.,
                delta_x: 0.,
                delta_y,
            },
        );
    };

    scroll(&mut runtime, 20.);
    let scrolled = frame(&mut runtime, 75.);
    let translated = wave(&scrolled).unwrap();
    assert_eq!(row(&runtime).y, bounds.y - 20.);
    assert_eq!(translated.rect().y, translated.clip().y);
    assert_eq!(
        translated.rect().y + translated.rect().height,
        start.rect().y + start.rect().height - 20.
    );
    assert_eq!(translated.clip(), Rect::new(0., 0., 240., 80.));

    scroll(&mut runtime, 80.);
    assert!(wave(&frame(&mut runtime, 100.)).is_none());
    assert!(!runtime.needs_frame());
    scroll(&mut runtime, -100.);
    assert!(wave(&frame(&mut runtime, 150.)).is_none());
    assert_eq!(runtime.app().taps, 0);
}

#[test]
fn a_drag_claim_cancels_the_stock_wave_and_its_activation() {
    let mut runtime = Runtime::new(App {
        drag: Some(PointDragSession::new()),
        ..App::default()
    })
    .unwrap();
    frame(&mut runtime, 0.);
    down(&mut runtime);
    frame(&mut runtime, 0.);
    assert!(wave(&frame(&mut runtime, 75.)).is_some());
    event(&mut runtime, Event::PointerMove { x: 70., y: 28. });
    assert!(wave(&frame(&mut runtime, 75.)).is_none());
    event(&mut runtime, Event::PointerUp { x: 70., y: 28. });
    assert!(wave(&frame(&mut runtime, 100.)).is_none());
    assert_eq!(runtime.app().taps, 0);
    assert!(!runtime.needs_frame());
}
