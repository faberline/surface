use surface::{Length, Node, RangeValue, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityAdjustment as Action, AccessibilityAdjustmentResult as Outcome,
    AccessibilityAdjustmentTarget, AppError, Application, Event, ImeEvent, Queue, Runtime, View,
};

const SLIDER: &str = "volume";

#[derive(Clone)]
enum Message {
    Change(f64),
    Disable,
    Hide,
    Open,
    Close,
    Edit,
}

struct App {
    range: RangeValue,
    opt_in: bool,
    bound: bool,
    disabled: bool,
    hidden: bool,
    modal: bool,
    hide_on_request: bool,
    writes: usize,
    queue: Option<Queue<Message>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            range: RangeValue {
                min: 0.,
                max: 10.,
                value: 6.,
                step: 2.,
            },
            opt_in: true,
            bound: true,
            disabled: false,
            hidden: false,
            modal: false,
            hide_on_request: false,
            writes: 0,
            queue: None,
        }
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Change(value) => {
                self.range.value = value;
                self.writes += 1;
            }
            Message::Disable => self.disabled = true,
            Message::Hide => self.hidden = true,
            Message::Open => self.modal = true,
            Message::Close => self.modal = false,
            Message::Edit => {}
        }
    }

    fn attach_queue(&mut self, queue: Queue<Message>) {
        self.queue = Some(queue);
    }

    fn view(&self) -> View<Message> {
        let size = || Style::row().width(Length::Px(120.)).height(Length::Px(48.));
        let mut slider = View::new(
            Node::new(Role::Slider, "Volume")
                .id(SLIDER)
                .range(self.range)
                .disabled(self.disabled)
                .styled(size()),
        );
        if self.opt_in {
            slider = slider.accessible_slider_adjustment();
        }
        if self.bound {
            let queue = self.queue.clone().filter(|_| self.hide_on_request);
            slider = slider.on_value(move |value| {
                if let Some(queue) = &queue {
                    queue.post(Message::Hide);
                }
                Message::Change(value)
            });
        }
        if self.hidden {
            slider = slider.without_interaction(None);
        }
        let opener = View::new(Node::new(Role::Button, "Open").id("open").styled(size()))
            .on_press(Message::Open);
        let editor = View::new(
            Node::new(Role::TextInput, "Editor")
                .id("editor")
                .text("sample")
                .styled(size()),
        )
        .on_input(|_| Message::Edit);
        let panel = self.modal.then(|| {
            View::new(Node::new(Role::Dialog, "Modal").id("panel").styled(size()))
                .on_value(Message::Change)
                .accessible_slider_adjustment()
                .modal_overlay(Message::Close)
        });
        View::new(Node::new(Role::Window, "Slider fixture").styled(Style::column()))
            .children([opener, editor, slider].into_iter().chain(panel))
    }
}

struct Text;
impl TextMeasurer for Text {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (24., 24.)
    }
}

fn draw(runtime: &mut Runtime<App>) {
    runtime.render(320., 400., &mut Text).unwrap();
}

fn fixture(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    draw(&mut runtime);
    runtime
}

fn target(runtime: &Runtime<App>, key: &str) -> Option<AccessibilityAdjustmentTarget> {
    let id = runtime.dom().by_id(key).unwrap();
    runtime.accessibility_frame()?.adjustment_target(id)
}

fn offers_steps(runtime: &Runtime<App>) -> [bool; 2] {
    let id = runtime.dom().by_id(SLIDER).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    [true, false].map(|increment| frame.supports_adjustment_step(id, increment))
}

fn adjust(
    runtime: &mut Runtime<App>,
    token: &AccessibilityAdjustmentTarget,
    action: Action,
) -> Outcome {
    runtime.accessibility_adjust(token, action).unwrap()
}

#[test]
fn native_steps_and_set_value_share_legacy_snapping_and_keep_focus() {
    let mut rt = Runtime::new(App::default()).unwrap();
    assert!(target(&rt, SLIDER).is_none());
    rt.activate("editor").unwrap();
    draw(&mut rt);

    let token = target(&rt, SLIDER).unwrap();
    assert_eq!(offers_steps(&rt), [true, true]);
    for request in [6., 6.2, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            adjust(&mut rt, &token, Action::SetValue(request)),
            Outcome::Rejected
        );
    }
    assert_eq!(rt.app().writes, 0);
    assert_eq!(
        adjust(&mut rt, &token, Action::SetValue(7.2)),
        Outcome::Adjusted
    );
    assert_eq!(rt.app().range.value, 8.);
    assert_eq!(rt.focused(), Some("editor"));

    draw(&mut rt);
    assert_eq!(
        adjust(&mut rt, &token, Action::Increment),
        Outcome::Rejected
    );
    for (request, expected, offered) in [
        (Action::Increment, 10., [false, true]),
        (Action::Decrement, 8., [true, true]),
        (Action::SetValue(-80.), 0., [true, false]),
    ] {
        let token = target(&rt, SLIDER).unwrap();
        assert_eq!(adjust(&mut rt, &token, request), Outcome::Adjusted);
        assert_eq!(rt.app().range.value, expected);
        assert_eq!(rt.focused(), Some("editor"));
        draw(&mut rt);
        assert_eq!(offers_steps(&rt), offered);
    }
    let token = target(&rt, SLIDER).unwrap();
    assert_eq!(
        adjust(&mut rt, &token, Action::Decrement),
        Outcome::Rejected
    );
    assert_eq!(rt.app().writes, 4);
}

#[test]
fn legacy_unaligned_endpoint_matches_setter_for_opted_in_slider() {
    let app = || App {
        range: RangeValue {
            min: 0.,
            max: 100.,
            value: 50.,
            step: 30.,
        },
        ..App::default()
    };
    let mut ordinary = fixture(app());
    let mut native = fixture(app());
    assert!(ordinary.set_value(SLIDER, 100.).unwrap());
    let token = target(&native, SLIDER).unwrap();
    assert_eq!(
        adjust(&mut native, &token, Action::SetValue(100.)),
        Outcome::Adjusted
    );
    assert_eq!(native.app().range.value, ordinary.app().range.value);
    assert_eq!(native.app().range.value, 90.);
    draw(&mut native);
    assert_eq!(offers_steps(&native), [false, true]);
}

#[test]
fn adjustment_requires_opt_in_a_value_binding_visibility_and_scope() {
    let mut legacy = fixture(App {
        opt_in: false,
        ..App::default()
    });
    assert!(target(&legacy, SLIDER).is_none());
    assert_eq!(offers_steps(&legacy), [false, false]);
    assert!(legacy.set_value(SLIDER, 9.2).unwrap());
    assert_eq!(legacy.app().range.value, 10.);
    assert!(!legacy.set_value("missing", f64::NAN).unwrap());

    let unbound_disabled = fixture(App {
        bound: false,
        disabled: true,
        ..App::default()
    });
    assert!(target(&unbound_disabled, SLIDER).is_none());
    assert!(
        matches!(Runtime::new(App { bound: false, ..App::default() }), Err(AppError::InvalidRange(id)) if id == SLIDER)
    );

    let mut rt = fixture(App::default());
    let before_modal = target(&rt, SLIDER).unwrap();
    rt.apply(Message::Open).unwrap();
    draw(&mut rt);
    assert!(target(&rt, SLIDER).is_none());
    assert!(target(&rt, "panel").is_none());
    assert_eq!(
        adjust(&mut rt, &before_modal, Action::Increment),
        Outcome::Rejected
    );

    rt.apply(Message::Close).unwrap();
    draw(&mut rt);
    let enabled = target(&rt, SLIDER).unwrap();
    rt.apply(Message::Disable).unwrap();
    draw(&mut rt);
    assert!(target(&rt, SLIDER).is_none());
    assert_eq!(
        adjust(&mut rt, &enabled, Action::Increment),
        Outcome::Rejected
    );
    assert_eq!(rt.app().writes, 0);
}

#[test]
fn composition_pressed_pointer_and_a_queued_rebuild_reject_the_adjustment() {
    let mut rt = fixture(App {
        hide_on_request: true,
        ..App::default()
    });
    rt.activate("editor").unwrap();
    rt.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    draw(&mut rt);
    let token = target(&rt, SLIDER).unwrap();
    assert_eq!(
        adjust(&mut rt, &token, Action::Increment),
        Outcome::BlockedByComposition
    );
    rt.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();
    draw(&mut rt);

    let opener = rt.dom().by_id("open").unwrap();
    let bounds = rt.computed().unwrap().nodes[opener.index()].bounds;
    rt.dispatch(Event::PointerDown {
        x: bounds.x + 4.,
        y: bounds.y + 4.,
    })
    .unwrap();
    draw(&mut rt);
    let token = target(&rt, SLIDER).unwrap();
    assert_eq!(
        adjust(&mut rt, &token, Action::Increment),
        Outcome::BlockedByPointer
    );
    rt.cancel_pointer().unwrap();
    draw(&mut rt);

    let token = target(&rt, SLIDER).unwrap();
    assert_eq!(
        adjust(&mut rt, &token, Action::SetValue(9.)),
        Outcome::Rebuilt
    );
    assert!(rt.app().hidden);
    assert_eq!(rt.app().range.value, 6.);
    assert_eq!(rt.app().writes, 0);
    draw(&mut rt);
    assert!(target(&rt, SLIDER).is_none());
    assert_eq!(
        adjust(&mut rt, &token, Action::Decrement),
        Outcome::Rejected
    );
}
