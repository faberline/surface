use surface::{Insets, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AnimationFrame, Application, Event, FrameInput, FramePress, FramePressPhase,
    InputAnimationUpdate, Runtime, View,
};

#[derive(Clone)]
enum Message {
    Activate,
    Disable,
    Quiet(bool),
    Modal(bool),
}

#[derive(Default)]
struct App {
    hits: usize,
    disabled: bool,
    quiet: bool,
    modal: bool,
    pressed: Option<String>,
    pointer: Option<(f32, f32)>,
    hovered: Option<String>,
    observed: Option<Seen>,
}

struct Seen {
    id: String,
    origin: (f32, f32),
    bounds: Rect,
    started_ms: Option<f64>,
    sequence: u64,
    phase: FramePressPhase,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, event: Message) {
        match event {
            Message::Activate => self.hits += 1,
            Message::Disable => self.disabled = true,
            Message::Quiet(quiet) => self.quiet = quiet,
            Message::Modal(open) => self.modal = open,
        }
    }

    fn view(&self) -> View<Message> {
        let button = |id, disabled| {
            View::new(
                Node::new(Role::Button, id)
                    .id(id)
                    .disabled(disabled)
                    .styled(Style::row().width(Length::Px(100.)).height(Length::Px(60.))),
            )
            .on_press(Message::Activate)
        };
        let modal = self.modal.then(|| {
            let anchored = Style::column().width(Length::Px(100.)).fixed(Insets {
                left: Length::Px(120.),
                top: Length::Px(0.),
                ..Insets::default()
            });
            View::new(
                Node::new(Role::Dialog, "Modal")
                    .id("modal")
                    .styled(anchored),
            )
            .children([button("modal-action", false)])
            .modal_overlay(Message::Modal(false))
        });
        View::new(Node::new(Role::Window, "Test")).children(
            [button("first", self.disabled), button("second", false)]
                .into_iter()
                .chain(modal),
        )
    }

    fn observes_frame_input(&self) -> bool {
        !self.quiet
    }

    fn animation_frame_with_press_detail(
        &mut self,
        _: AnimationFrame<'_>,
        input: FrameInput<'_>,
        pressed: Option<&str>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> InputAnimationUpdate {
        self.pressed = pressed.map(str::to_owned);
        self.pointer = input.pointer;
        self.hovered = hovered.map(str::to_owned);
        self.observed = press.map(|event| Seen {
            id: event.id.to_owned(),
            origin: event.position,
            bounds: event.bounds,
            started_ms: event.started_ms,
            sequence: event.sequence,
            phase: event.phase,
        });
        InputAnimationUpdate::default()
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn frame(runtime: &mut Runtime<App>) {
    runtime.render(240., 200., &mut Metrics).unwrap();
}

fn fixture() -> Runtime<App> {
    let mut runtime = Runtime::new(App::default()).unwrap();
    frame(&mut runtime);
    runtime
}

fn observation(runtime: &Runtime<App>) -> &Seen {
    runtime.app().observed.as_ref().unwrap()
}

#[test]
fn mouse_origin_survives_coalesced_moves_and_release_outside_is_cancelled_once() {
    let mut runtime = fixture();
    runtime.set_time(12.);
    runtime
        .dispatch(Event::PointerDown { x: 7., y: 9. })
        .unwrap();
    runtime
        .dispatch(Event::PointerMove { x: 65., y: 45. })
        .unwrap();
    frame(&mut runtime);
    let seen = observation(&runtime);
    assert_eq!(runtime.app().pressed.as_deref(), Some("first"));
    assert_eq!(runtime.app().pointer, Some((65., 45.)));
    assert_eq!(seen.origin, (7., 9.));
    assert_eq!(seen.bounds, Rect::new(0., 0., 100., 60.));
    assert_eq!(seen.started_ms, Some(12.));
    assert_eq!(seen.phase, FramePressPhase::Held);
    let sequence = seen.sequence;

    runtime
        .dispatch(Event::PointerUp { x: 200., y: 45. })
        .unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.app().pressed, None);
    assert_eq!(observation(&runtime).phase, FramePressPhase::Cancelled);
    assert_eq!(observation(&runtime).sequence, sequence);
    assert_eq!(runtime.app().hits, 0);
    frame(&mut runtime);
    assert!(runtime.app().observed.is_none());
}

#[test]
fn fast_tap_reports_its_release_and_a_second_press_gets_a_new_origin() {
    let mut runtime = fixture();
    runtime
        .dispatch(Event::PointerDown { x: 32., y: 14. })
        .unwrap();
    runtime
        .dispatch(Event::PointerUp { x: 40., y: 35. })
        .unwrap();
    frame(&mut runtime);
    let tap = observation(&runtime);
    assert_eq!(tap.id, "first");
    assert_eq!(tap.origin, (32., 14.));
    assert_eq!(tap.phase, FramePressPhase::Released);
    assert_eq!(runtime.app().pressed, None);
    assert_eq!(runtime.app().hits, 1);
    let sequence = tap.sequence;

    frame(&mut runtime);
    assert!(runtime.app().observed.is_none());
    runtime
        .dispatch(Event::PointerDown { x: 10., y: 12. })
        .unwrap();
    runtime
        .dispatch(Event::PointerDown { x: 20., y: 90. })
        .unwrap();
    frame(&mut runtime);
    let latest = observation(&runtime);
    assert_eq!(latest.id, "second");
    assert_eq!(latest.origin, (20., 90.));
    assert_eq!(latest.phase, FramePressPhase::Held);
    assert_ne!(latest.sequence, sequence);
    assert_eq!(runtime.app().hits, 1);
}

#[test]
fn touch_cancel_and_disabling_a_held_mouse_target_keep_the_original_origin() {
    let mut runtime = fixture();
    runtime
        .dispatch_touch(Event::PointerDown { x: 11., y: 22. })
        .unwrap();
    runtime
        .dispatch_touch(Event::PointerMove { x: 13., y: 35. })
        .unwrap();
    runtime.cancel_pointer().unwrap();
    frame(&mut runtime);
    assert_eq!(observation(&runtime).origin, (11., 22.));
    assert_eq!(observation(&runtime).phase, FramePressPhase::Cancelled);
    assert_eq!(runtime.app().hits, 0);

    runtime
        .dispatch(Event::PointerDown { x: 18., y: 17. })
        .unwrap();
    runtime.apply(Message::Disable).unwrap();
    frame(&mut runtime);
    assert_eq!(observation(&runtime).origin, (18., 17.));
    assert_eq!(observation(&runtime).phase, FramePressPhase::Cancelled);
    assert_eq!(runtime.app().hits, 0);
}

#[test]
fn leaving_and_reentering_observation_discards_a_pending_terminal() {
    let mut runtime = fixture();
    runtime
        .dispatch(Event::PointerDown { x: 16., y: 22. })
        .unwrap();
    frame(&mut runtime);
    let sequence = observation(&runtime).sequence;
    runtime
        .dispatch(Event::PointerUp { x: 16., y: 22. })
        .unwrap();
    runtime.apply(Message::Quiet(true)).unwrap();
    runtime.apply(Message::Quiet(false)).unwrap();
    frame(&mut runtime);
    assert!(runtime.app().observed.is_none());

    runtime.apply(Message::Quiet(true)).unwrap();
    frame(&mut runtime);
    runtime
        .dispatch(Event::PointerDown { x: 18., y: 26. })
        .unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.app().pressed.as_deref(), Some("first"));
    assert!(runtime.app().observed.is_none());
    runtime.cancel_pointer().unwrap();

    runtime.apply(Message::Quiet(false)).unwrap();
    frame(&mut runtime);
    runtime
        .dispatch(Event::PointerDown { x: 22., y: 28. })
        .unwrap();
    frame(&mut runtime);
    assert!(observation(&runtime).sequence > sequence);
    assert_eq!(observation(&runtime).origin, (22., 28.));
}

#[test]
fn observed_hover_stays_inside_the_active_modal_scope() {
    let mut runtime = fixture();
    runtime
        .dispatch(Event::PointerMove { x: 7., y: 9. })
        .unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.app().hovered.as_deref(), Some("first"));

    runtime.apply(Message::Modal(true)).unwrap();
    frame(&mut runtime);
    let layout = runtime.snapshot().layout.unwrap();
    assert_eq!(
        layout.hit_test(runtime.dom(), 7., 9.),
        runtime.dom().by_id("first")
    );
    frame(&mut runtime);
    assert_eq!(runtime.app().pointer, Some((7., 9.)));
    assert_eq!(runtime.app().hovered, None);

    runtime
        .dispatch(Event::PointerMove { x: 137., y: 18. })
        .unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.app().hovered.as_deref(), Some("modal-action"));
}
