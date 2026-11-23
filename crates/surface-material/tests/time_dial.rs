use surface::{Color, ExtendedPaintOp, Length, Node, PaintOp, Rect, Role, Style, TextMeasurer};
use surface_app::{AnimationFrame, AnimationUpdate, Application, EditKey, Event, Runtime, View};
use surface_material::{
    shape::LayoutDirection, time_dial_layout, ClockFormat, DialEvent, DialHandMotion, DialMode,
    DialPoint, DialPreview, DialState, Theme, TimeDialLabels, TimeDialLayout, TimeDialOptions,
    TimeField, TimeInputEffect, TimeInputLabels, TimeInputLocale, TimeInputOptions,
    TimeInputPresentation, TimeInputSession, TimeOfDay,
};

const HOUR: &str = "time-hour-selector";
const MINUTE: &str = "time-minute-selector";
const FACE: &str = "time-dial-face";
const HANDLE: &str = "time-dial-handle";
const LABELS: TimeInputLabels<'static> = TimeInputLabels {
    hour: "小時",
    minute: "分鐘",
    period: "時段",
    hour_hint: "請輸入小時",
    minute_hint: "00–59",
    confirm: "確定",
    cancel: "取消",
    error_text: &|field, _, _| match field {
        TimeField::Hour => "小時範圍錯誤".into(),
        TimeField::Minute => "分鐘範圍錯誤".into(),
    },
};
const DIAL_LABELS: TimeDialLabels<'static> = TimeDialLabels {
    use_input: "使用文字輸入",
    use_dial: "使用錶盤",
    unfit: "空間不足，請用文字輸入",
    value_name: &|field, digits, count| {
        let field = match field {
            TimeField::Hour => LABELS.hour,
            TimeField::Minute => LABELS.minute,
        };
        format!("{field} {digits}，共 {count}")
    },
};

fn locale() -> TimeInputLocale<'static> {
    TimeInputLocale::new("上午", "下午", ":")
}

#[derive(Clone)]
enum Message {
    Open,
    Dial(DialEvent),
    Resize(Rect, f32),
}

struct App {
    state: Option<DialState>,
    animate_hand: bool,
    hand: Option<DialHandMotion>,
    viewport: Rect,
    direction: LayoutDirection,
    clock: ClockFormat,
    locale: TimeInputLocale<'static>,
    title: &'static str,
    scale: f32,
    saved: TimeOfDay,
    commits: usize,
}

impl App {
    fn new(width: f32, height: f32, clock: ClockFormat, direction: LayoutDirection) -> Self {
        Self {
            state: None,
            animate_hand: false,
            hand: None,
            viewport: Rect::new(0., 0., width, height),
            direction,
            clock,
            locale: locale(),
            title: "選擇時間",
            scale: 1.,
            saved: TimeOfDay::new(9, 7).unwrap(),
            commits: 0,
        }
    }

    fn layout(&self) -> Option<TimeDialLayout> {
        time_dial_layout(
            self.viewport.width,
            self.viewport.height,
            self.scale,
            self.clock,
        )
    }

    fn dial_event(&mut self, event: DialEvent) -> Option<TimeInputEffect> {
        let state = self.state.as_mut()?;
        Some(match &mut self.hand {
            Some(hand) => hand.apply(state, event),
            None => state.apply(event),
        })
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => {
                let input = TimeInputSession::new(Some(self.saved), self.clock, self.locale);
                let state = DialState::new(input, DialMode::Dial, self.layout().is_some());
                self.hand = self.animate_hand.then(|| DialHandMotion::new(&state));
                self.state = Some(state);
            }
            Message::Resize(viewport, scale) => {
                self.viewport = viewport;
                self.scale = scale;
                let fits = self.layout().is_some();
                self.dial_event(DialEvent::SetDialFits(fits));
            }
            Message::Dial(event) => match self.dial_event(event) {
                Some(TimeInputEffect::Commit(value)) => {
                    self.saved = value;
                    self.commits += 1;
                    self.state = None;
                    self.hand = None;
                }
                Some(TimeInputEffect::Dismiss) => {
                    self.state = None;
                    self.hand = None;
                }
                _ => {}
            },
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        self.hand
            .as_mut()
            .map_or_else(AnimationUpdate::default, |hand| {
                hand.advance(frame.now_ms, frame.reduced_motion)
            })
    }

    fn view(&self) -> View<Self::Message> {
        let opener = View::new(
            Node::new(Role::Button, "開啟時間")
                .id("open")
                .styled(Style::row().height(Length::Px(48.))),
        )
        .on_press(Message::Open);
        let picker = self.state.as_ref().map(|state| {
            let theme = if self.scale > 1. {
                Theme::dark()
            } else {
                Theme::light()
            };
            let options = TimeDialOptions {
                input: TimeInputOptions {
                    id: "time",
                    title: self.title,
                    labels: &LABELS,
                    locale: self.locale,
                    direction: self.direction,
                    presentation: TimeInputPresentation::Modal,
                    viewport: self.viewport,
                    disabled: false,
                    restore_focus_to: Some("open"),
                },
                labels: &DIAL_LABELS,
                text_scale: self.scale,
            };
            match &self.hand {
                Some(hand) => theme.time_dial_with_motion(options, state, hand, Message::Dial),
                None => theme.time_dial(options, state, Message::Dial),
            }
        });
        View::new(Node::new(Role::Window, "").styled(Style::column().width(Length::Percent(1.))))
            .children(std::iter::once(opener).chain(picker))
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, content: &str, size: f32, max: Option<f32>) -> (f32, f32) {
        let width = content
            .chars()
            .map(|ch| if ch.is_ascii() { 0.58 } else { 1. })
            .sum::<f32>()
            * size;
        let fitted = max.map_or(width, |max| width.min(max));
        let lines = (width / fitted.max(1.)).ceil().max(1.);
        (fitted, size * 1.5 * lines)
    }
}

fn render(runtime: &mut Runtime<App>) {
    let viewport = runtime.app().viewport;
    runtime
        .render(viewport.width, viewport.height, &mut Metrics)
        .unwrap();
}

fn open(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    runtime.activate("open").unwrap();
    render(&mut runtime);
    runtime
}

fn node<'a>(runtime: &'a Runtime<App>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn rect(runtime: &Runtime<App>, id: &str) -> Rect {
    let id = runtime.dom().by_id(id).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn is_contained(inner: Rect, outer: Rect) {
    assert!(
        inner.x >= outer.x && inner.y >= outer.y,
        "{inner:?} vs {outer:?}"
    );
    assert!(
        inner.x + inner.width <= outer.x + outer.width,
        "{inner:?} vs {outer:?}"
    );
    assert!(
        inner.y + inner.height <= outer.y + outer.height,
        "{inner:?} vs {outer:?}"
    );
}

fn state(runtime: &Runtime<App>) -> &DialState {
    runtime.app().state.as_ref().unwrap()
}

#[test]
fn arrangements_keep_the_full_dial_face_and_modal_actions() {
    use ClockFormat::{TwelveHour, TwentyFourHour};
    use LayoutDirection::{LeftToRight, RightToLeft};
    use TimeDialLayout::{Horizontal, Vertical};
    assert_eq!(time_dial_layout(390., 720., 1., TwelveHour), Some(Vertical));
    assert_eq!(
        time_dial_layout(844., 390., 1., TwelveHour),
        Some(Horizontal)
    );
    assert_eq!(
        time_dial_layout(320., 580., 1., TwentyFourHour),
        Some(Vertical)
    );
    assert_eq!(time_dial_layout(319., 580., 1., TwentyFourHour), None);
    assert_eq!(time_dial_layout(320., 300., 2., TwelveHour), None);
    assert_eq!(time_dial_layout(f32::INFINITY, 800., 1., TwelveHour), None);
    for (width, height, clock, direction) in [
        (390., 720., TwelveHour, LeftToRight),
        (844., 390., TwelveHour, LeftToRight),
        (844., 390., TwentyFourHour, RightToLeft),
    ] {
        let runtime = open(App::new(width, height, clock, direction));
        let face = rect(&runtime, FACE);
        assert_eq!((face.width, face.height), (256., 256.));
        let dialog = rect(&runtime, "time");
        is_contained(dialog, runtime.app().viewport);
        for id in [
            FACE,
            HOUR,
            MINUTE,
            "time-confirm",
            "time-cancel",
            "time-use-input",
        ] {
            is_contained(rect(&runtime, id), dialog);
        }
        assert_eq!(node(&runtime, "time").role(), Role::Dialog);
        assert_eq!(
            runtime.accessibility_frame().unwrap().modal_root(),
            runtime.dom().by_id("time")
        );
        assert_eq!(runtime.focused(), Some(HOUR));
        assert_eq!(
            runtime.dom().by_id("time-period").is_some(),
            clock == TwelveHour
        );
    }
}

#[test]
fn magnified_horizontal_dial_keeps_both_period_buttons_visible() {
    let mut app = App::new(
        1120.,
        880.,
        ClockFormat::TwelveHour,
        LayoutDirection::RightToLeft,
    );
    app.scale = 2.;
    assert_eq!(app.layout(), Some(TimeDialLayout::Horizontal));

    let runtime = open(app);
    let controls = rect(&runtime, "time-dial-controls");
    is_contained(controls, rect(&runtime, "time"));
    for id in ["time-am", "time-pm"] {
        let target = rect(&runtime, id);
        assert!(target.width >= 48. && target.height >= 48.);
        is_contained(target, controls);
    }
    let face = rect(&runtime, FACE);
    assert_eq!((face.width, face.height), (256., 256.));
}

#[test]
fn horizontal_dial_keeps_face_and_actions_visible_with_a_long_title() {
    let mut app = App::new(
        844.,
        390.,
        ClockFormat::TwentyFourHour,
        LayoutDirection::LeftToRight,
    );
    app.scale = 2.;
    app.title = "請使用時鐘的錶盤選擇目前的時間。請先選擇正確的小時，然後選擇正確的分鐘。確認顯示的時間後，再儲存更改。";
    assert_eq!(app.layout(), Some(TimeDialLayout::Horizontal));

    let runtime = open(app);
    let dialog = rect(&runtime, "time");
    let face = rect(&runtime, FACE);
    assert_eq!((face.width, face.height), (256., 256.));
    for id in ["time-dial-controls", FACE, "time-confirm", "time-cancel"] {
        is_contained(rect(&runtime, id), dialog);
    }
    let scroll = runtime.scroll_metrics("time-dial-controls").unwrap();
    assert!(scroll.content.1 > scroll.viewport.1);
}

#[test]
fn runtime_point_capture_previews_cancels_and_accepts_only_at_release() {
    let mut runtime = open(App::new(
        390.,
        720.,
        ClockFormat::TwelveHour,
        LayoutDirection::LeftToRight,
    ));
    let face = rect(&runtime, FACE);
    let start = (face.x + 228., face.y + 128.);
    let end = (face.x + 128., face.y + 228.);
    let drag = |runtime: &mut Runtime<App>| {
        runtime
            .dispatch(Event::PointerDown {
                x: start.0,
                y: start.1,
            })
            .unwrap();
        runtime
            .dispatch(Event::PointerMove { x: end.0, y: end.1 })
            .unwrap();
        render(runtime);
    };
    drag(&mut runtime);
    assert_eq!(
        state(&runtime).preview(),
        Some(DialPreview {
            field: TimeField::Hour,
            value: 6
        })
    );
    assert_eq!(state(&runtime).input().raw(TimeField::Hour), "9");
    assert!(runtime.cancel_pointer().unwrap());
    render(&mut runtime);
    assert_eq!(state(&runtime).preview(), None);
    assert_eq!(state(&runtime).field(), TimeField::Hour);

    drag(&mut runtime);
    runtime
        .dispatch(Event::PointerUp { x: end.0, y: end.1 })
        .unwrap();
    render(&mut runtime);
    assert_eq!(state(&runtime).preview(), None);
    assert_eq!(state(&runtime).field(), TimeField::Minute);
    assert_eq!(state(&runtime).input().raw(TimeField::Hour), "6");
    assert_eq!(state(&runtime).input().raw(TimeField::Minute), "07");
    assert_eq!(runtime.app().commits, 0);
    assert_eq!(runtime.app().saved, TimeOfDay::new(9, 7).unwrap());
    runtime.activate("time-confirm").unwrap();
    render(&mut runtime);
    assert_eq!(runtime.app().commits, 1);
    assert_eq!(runtime.app().saved, TimeOfDay::new(6, 7).unwrap());
    assert_eq!(runtime.focused(), Some("open"));
}

#[test]
fn keyboard_hands_off_hour_focus_and_reaches_every_minute_one_step_at_a_time() {
    let mut runtime = open(App::new(
        844.,
        390.,
        ClockFormat::TwentyFourHour,
        LayoutDirection::RightToLeft,
    ));
    runtime
        .dispatch(Event::Edit {
            key: EditKey::Down,
            extend: false,
            by_word: false,
        })
        .unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("time-dial-hour-9"));
    runtime.dispatch(Event::Activate).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some(MINUTE));
    runtime
        .dispatch(Event::Edit {
            key: EditKey::Down,
            extend: false,
            by_word: false,
        })
        .unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("time-dial-minute-7"));
    for step in 1..=60 {
        runtime
            .dispatch(Event::Edit {
                key: EditKey::Right,
                extend: false,
                by_word: false,
            })
            .unwrap();
        render(&mut runtime);
        let next = (7 + step) % 60;
        let id = format!("time-dial-minute-{next}");
        assert_eq!(runtime.focused(), Some(id.as_str()));
        assert_eq!(node(&runtime, &id).is_checked(), Some(true));
        assert_eq!(node(&runtime, &id).name(), format!("分鐘 {next:02}，共 60"));
    }
    assert_eq!(runtime.app().commits, 0);
}

#[test]
fn compact_uses_the_same_dialog_and_needs_an_explicit_return() {
    let mut runtime = open(App::new(
        390.,
        720.,
        ClockFormat::TwelveHour,
        LayoutDirection::RightToLeft,
    ));
    runtime
        .apply(Message::Resize(Rect::new(0., 0., 320., 300.), 2.))
        .unwrap();
    render(&mut runtime);
    assert_eq!(state(&runtime).mode(), DialMode::Input);
    assert!(runtime.dom().by_id(FACE).is_none());
    assert!(node(&runtime, "time-use-dial").is_disabled());
    assert_eq!(node(&runtime, "time-unfit").name(), DIAL_LABELS.unfit);
    assert_eq!(runtime.focused(), Some("time-hour-input"));
    for id in ["time-confirm", "time-cancel", "time-use-dial"] {
        is_contained(rect(&runtime, id), runtime.app().viewport);
    }
    runtime
        .apply(Message::Resize(Rect::new(0., 0., 700., 860.), 2.))
        .unwrap();
    render(&mut runtime);
    assert_eq!(state(&runtime).mode(), DialMode::Input);
    assert!(!node(&runtime, "time-use-dial").is_disabled());
    runtime.activate("time-use-dial").unwrap();
    render(&mut runtime);
    assert_eq!(state(&runtime).mode(), DialMode::Dial);
    assert_eq!(state(&runtime).input().raw(TimeField::Minute), "07");
    assert_eq!(
        (rect(&runtime, FACE).width, rect(&runtime, FACE).height),
        (256., 256.)
    );
}

#[test]
fn enter_on_selectors_enters_exact_face_start_without_changing_the_draft() {
    use surface_app::{Key, KeyPress, Modifiers};

    let enter = KeyPress::new(Key::Enter, Modifiers::default());
    for (clock, start) in [
        (ClockFormat::TwelveHour, "time-dial-hour-12"),
        (ClockFormat::TwentyFourHour, "time-dial-hour-0"),
    ] {
        let mut runtime = open(App::new(390., 720., clock, LayoutDirection::LeftToRight));
        let initial = state(&runtime).input().validation().candidate();
        assert_eq!(runtime.focused(), Some(HOUR));
        assert!(runtime.key(&enter).unwrap());
        assert_eq!(runtime.focused(), Some(start));
        assert_eq!(node(&runtime, start).is_checked(), Some(false));
        assert_eq!(state(&runtime).input().validation().candidate(), initial);
        assert!(runtime.key(&enter.clone().with_repeat(true)).unwrap());
        assert_eq!(state(&runtime).field(), TimeField::Hour);
        assert_eq!(state(&runtime).input().validation().candidate(), initial);

        render(&mut runtime);
        let minute = runtime.dom().by_id(MINUTE).unwrap();
        let source = runtime
            .accessibility_frame()
            .unwrap()
            .focus_target(minute)
            .unwrap();
        runtime.accessibility_focus(&source).unwrap();
        render(&mut runtime);
        assert_eq!(runtime.focused(), Some(MINUTE));
        assert_eq!(state(&runtime).field(), TimeField::Hour);
        let minute = runtime.dom().by_id(MINUTE).unwrap();
        let source = runtime
            .accessibility_frame()
            .unwrap()
            .focus_target(minute)
            .unwrap();
        assert!(runtime
            .accessibility_key(&source, &enter, Event::Activate)
            .unwrap());
        assert_eq!(runtime.focused(), Some("time-dial-minute-0"));
        assert_eq!(state(&runtime).field(), TimeField::Minute);
        assert_eq!(state(&runtime).input().validation().candidate(), initial);
        assert!(runtime.key(&enter.clone().with_repeat(true)).unwrap());
        assert_eq!(state(&runtime).input().raw(TimeField::Minute), "07");
        assert_eq!(runtime.app().commits, 0);

        assert!(!runtime.key(&enter).unwrap());
        runtime.dispatch(Event::Activate).unwrap();
        assert_eq!(state(&runtime).input().raw(TimeField::Minute), "00");
        assert_eq!(runtime.app().commits, 0);
    }
}

fn face_text(runtime: &mut Runtime<App>) -> Vec<(String, f32)> {
    let viewport = runtime.app().viewport;
    let frame = runtime
        .render_extended(viewport.width, viewport.height, &mut Metrics)
        .unwrap();
    let face = rect(runtime, FACE);
    frame
        .ops()
        .iter()
        .filter_map(|op| match op {
            ExtendedPaintOp::Legacy(PaintOp::StyledText {
                rect, text, style, ..
            }) if rect.intersect(face) == *rect => Some((text.to_owned(), style.font_size)),
            _ => None,
        })
        .collect()
}

#[test]
fn selected_exact_minute_omits_colliding_ink_while_keeping_the_canonical_radio() {
    let mut runtime = open(App::new(
        844.,
        390.,
        ClockFormat::TwentyFourHour,
        LayoutDirection::RightToLeft,
    ));
    runtime.activate("time-dial-hour-9").unwrap();
    let zero = "time-dial-minute-0";
    let zero_target = {
        let painted = face_text(&mut runtime);
        assert!(painted.iter().any(|(text, _)| text == "00"));
        rect(&runtime, zero)
    };

    runtime
        .apply(Message::Dial(DialEvent::Point {
            field: TimeField::Minute,
            phase: surface_material::DialPoint::Move(Some(59)),
        }))
        .unwrap();
    let painted = face_text(&mut runtime);
    assert!(painted.iter().any(|(text, _)| text == "59"));
    assert!(!painted.iter().any(|(text, _)| text == "00"));
    assert_eq!(
        node(&runtime, "time-dial-minute-7").is_checked(),
        Some(true)
    );
    assert!(runtime.dom().by_id("time-dial-minute-59").is_none());
    assert_eq!(node(&runtime, zero).name(), "分鐘 00，共 60");
    assert_eq!(rect(&runtime, zero), zero_target);

    runtime
        .apply(Message::Dial(DialEvent::Point {
            field: TimeField::Minute,
            phase: surface_material::DialPoint::End(Some(59)),
        }))
        .unwrap();
    let painted = face_text(&mut runtime);
    assert!(painted.iter().any(|(text, _)| text == "59"));
    assert!(!painted.iter().any(|(text, _)| text == "00"));
    assert_eq!(
        node(&runtime, "time-dial-minute-59").is_checked(),
        Some(true)
    );
    assert_eq!(node(&runtime, zero).is_checked(), Some(false));
    assert_eq!(rect(&runtime, zero), zero_target);

    runtime.activate(zero).unwrap();
    assert!(face_text(&mut runtime).iter().any(|(text, _)| text == "00"));
    assert_eq!(node(&runtime, zero).is_checked(), Some(true));
}

#[test]
fn doubled_full_width_hour_digits_keep_body_large_faces_and_full_sized_summaries() {
    let mut app = App::new(
        1120.,
        880.,
        ClockFormat::TwentyFourHour,
        LayoutDirection::RightToLeft,
    );
    app.scale = 2.;
    app.locale = locale()
        .with_digits(['０', '１', '２', '３', '４', '５', '６', '７', '８', '９'])
        .unwrap();
    let mut runtime = open(app);
    let painted = face_text(&mut runtime);
    assert!(painted.len() >= 22);
    assert!(painted.iter().all(|(_, size)| *size == 16.));
    for id in ["time-dial-hour-9", "time-dial-hour-21"] {
        let bounds = rect(&runtime, id);
        assert_eq!((bounds.width, bounds.height), (48., 48.));
    }
    assert_eq!(
        node(&runtime, "time-dial-hour-21").name(),
        "小時 ２１，共 24"
    );
    assert_eq!(node(&runtime, "time-dial-hour-9").is_checked(), Some(true));
    let viewport = runtime.app().viewport;
    let summary = rect(&runtime, "time-hour-dial-value");
    let paint = runtime
        .render_extended(viewport.width, viewport.height, &mut Metrics)
        .unwrap();
    assert!(paint.ops().iter().any(|op| matches!(
        op,
        ExtendedPaintOp::Legacy(PaintOp::StyledText { rect, text, style, .. })
            if text == "０９" && rect.intersect(summary) == *rect && style.font_size == 64.
    )));
}

fn open_animated(saved: TimeOfDay, dark_large: bool) -> Runtime<App> {
    let mut app = App::new(
        900.,
        860.,
        ClockFormat::TwentyFourHour,
        LayoutDirection::RightToLeft,
    );
    app.saved = saved;
    app.animate_hand = true;
    if dark_large {
        app.scale = 2.;
        app.locale = locale()
            .with_digits(['０', '１', '２', '３', '４', '５', '６', '７', '８', '９'])
            .unwrap();
    }
    open(app)
}

fn handle_center(runtime: &Runtime<App>) -> (f32, f32) {
    let handle = rect(runtime, HANDLE);
    (handle.x + handle.width / 2., handle.y + handle.height / 2.)
}

fn sample(runtime: &mut Runtime<App>, time: f64) -> (f32, f32) {
    runtime.set_time(time);
    render(runtime);
    handle_center(runtime)
}

fn face_ink(runtime: &mut Runtime<App>, digits: &str) -> Option<Color> {
    let viewport = runtime.app().viewport;
    let frame = runtime
        .render_extended(viewport.width, viewport.height, &mut Metrics)
        .unwrap();
    let face = rect(runtime, FACE);
    frame.ops().iter().find_map(|op| match op {
        ExtendedPaintOp::Legacy(PaintOp::StyledText {
            rect, text, color, ..
        }) if text == digits && rect.intersect(face) == *rect => Some(*color),
        _ => None,
    })
}

fn edit_key(runtime: &mut Runtime<App>, key: EditKey) {
    runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false,
        })
        .unwrap();
}

#[test]
fn animated_minute_wrap_changes_focus_and_checked_state_before_the_hand_moves() {
    let mut runtime = open_animated(TimeOfDay::new(9, 59).unwrap(), true);
    edit_key(&mut runtime, EditKey::Down);
    assert_eq!(runtime.focused(), Some("time-dial-hour-9"));
    runtime.dispatch(Event::Activate).unwrap();
    render(&mut runtime);
    edit_key(&mut runtime, EditKey::Down);
    assert_eq!(runtime.focused(), Some("time-dial-minute-59"));
    let old = sample(&mut runtime, 0.);

    edit_key(&mut runtime, EditKey::Right);
    let first = sample(&mut runtime, 0.);
    assert_eq!(old, first);
    assert_eq!(runtime.focused(), Some("time-dial-minute-0"));
    assert_eq!(
        node(&runtime, "time-dial-minute-0").is_checked(),
        Some(true)
    );
    assert_eq!(state(&runtime).input().raw(TimeField::Minute), "００");
    assert!(runtime.needs_frame());
    // At large text the moving knob only covers part of the new label.
    assert_eq!(face_ink(&mut runtime, "００"), None);
    assert_eq!(
        node(&runtime, "time-dial-minute-0").name(),
        "分鐘 ００，共 60"
    );

    let midway = sample(&mut runtime, 25.);
    let goal = sample(&mut runtime, 500.);
    assert!(old.0 < midway.0 && midway.0 < goal.0);
    assert!(!runtime.needs_frame());
    assert_eq!(
        face_ink(&mut runtime, "００"),
        Some(Theme::dark().colors.on_primary)
    );

    edit_key(&mut runtime, EditKey::Left);
    assert_eq!(sample(&mut runtime, 500.), goal);
    assert_eq!(runtime.focused(), Some("time-dial-minute-59"));
    let reverse = sample(&mut runtime, 525.);
    assert!(old.0 < reverse.0 && reverse.0 < goal.0);
    assert_eq!(sample(&mut runtime, 1000.), old);
    assert_eq!(runtime.app().commits, 0);
}

#[test]
fn an_exact_minute_radio_has_one_painted_label_after_the_hand_settles() {
    let mut runtime = open_animated(TimeOfDay::new(9, 0).unwrap(), false);
    runtime.activate("time-dial-hour-9").unwrap();
    render(&mut runtime);
    edit_key(&mut runtime, EditKey::Down);
    assert_eq!(runtime.focused(), Some("time-dial-minute-0"));
    let start = sample(&mut runtime, 0.);

    edit_key(&mut runtime, EditKey::Left);
    let primary_ink = Theme::light().colors.on_primary;
    let mut last_x = start.0;
    for time in [0., 24.] {
        let handle = sample(&mut runtime, time);
        if time > 0. {
            assert!(handle.0 < last_x);
        }
        last_x = handle.0;
        let exact = "time-dial-minute-59";
        assert_eq!(runtime.focused(), Some(exact));
        assert_eq!(node(&runtime, exact).is_checked(), Some(true));
        assert_eq!(node(&runtime, exact).name(), "分鐘 59，共 60");
        assert_eq!(state(&runtime).input().raw(TimeField::Minute), "59");
        assert_eq!(face_ink(&mut runtime, "59"), None);
        assert_eq!(face_ink(&mut runtime, "00"), Some(primary_ink));
    }

    let goal = sample(&mut runtime, 500.);
    assert!(goal.0 < last_x);
    assert!(!runtime.needs_frame());
    assert_eq!(face_ink(&mut runtime, "00"), None);
    assert_eq!(face_ink(&mut runtime, "59"), Some(primary_ink));
    assert_eq!(runtime.focused(), Some("time-dial-minute-59"));
    assert_eq!(runtime.app().commits, 0);
}

fn dial_event(runtime: &mut Runtime<App>, event: DialEvent) {
    runtime.apply(Message::Dial(event)).unwrap();
}

fn hour(runtime: &mut Runtime<App>, value: u8) {
    dial_event(
        runtime,
        DialEvent::Adjust {
            field: TimeField::Hour,
            value,
        },
    );
}

#[test]
fn animated_ring_reversal_carries_velocity_and_capture_retires_it() {
    let mut runtime = open_animated(TimeOfDay::new(0, 7).unwrap(), false);
    let outer = sample(&mut runtime, 0.);
    hour(&mut runtime, 12);
    assert_eq!(sample(&mut runtime, 0.), outer);
    let inward = sample(&mut runtime, 28.);
    assert_eq!(inward.0, outer.0);
    assert!(outer.1 < inward.1 && inward.1 < outer.1 + 36.);
    assert_eq!(node(&runtime, "time-dial-hour-12").is_checked(), Some(true));

    hour(&mut runtime, 0);
    assert_eq!(sample(&mut runtime, 28.), inward);
    // Sample enough travel to pass Taffy's whole-pixel layout rounding.
    let still_inward = sample(&mut runtime, 35.);
    assert!(still_inward.1 > inward.1);
    assert!(runtime.needs_frame());

    for (phase, expected) in [
        (DialPoint::Move(Some(3)), (outer.0 + 100., outer.1 + 100.)),
        (DialPoint::Cancel, outer),
    ] {
        dial_event(
            &mut runtime,
            DialEvent::Point {
                field: TimeField::Hour,
                phase,
            },
        );
        let hand = sample(&mut runtime, 36.);
        assert!((hand.0 - expected.0).abs() < 0.01);
        assert!((hand.1 - expected.1).abs() < 0.01);
        assert!(!runtime.needs_frame());
        assert_eq!(node(&runtime, "time-dial-hour-0").is_checked(), Some(true));
    }
}

#[test]
fn reduced_motion_and_unfit_layout_retire_an_active_hand_track() {
    let mut runtime = open_animated(TimeOfDay::new(0, 7).unwrap(), false);
    let start = sample(&mut runtime, 0.);
    hour(&mut runtime, 12);
    sample(&mut runtime, 0.);
    sample(&mut runtime, 20.);
    assert!(runtime.needs_frame());
    runtime.set_reduced_motion(true);
    let inner = sample(&mut runtime, 21.);
    assert!((inner.1 - start.1 - 36.).abs() < 0.01);
    assert!(!runtime.needs_frame());
    runtime.set_reduced_motion(false);
    assert_eq!(sample(&mut runtime, 22.), inner);

    hour(&mut runtime, 0);
    assert_eq!(sample(&mut runtime, 22.), inner);
    assert!(runtime.needs_frame());
    runtime
        .apply(Message::Resize(Rect::new(0., 0., 320., 300.), 2.))
        .unwrap();
    render(&mut runtime);
    assert!(runtime.dom().by_id(HANDLE).is_none());
    assert!(!runtime.needs_frame());
    runtime
        .apply(Message::Resize(Rect::new(0., 0., 900., 860.), 1.))
        .unwrap();
    render(&mut runtime);
    runtime.activate("time-use-dial").unwrap();
    assert_eq!(sample(&mut runtime, 22.), start);
    assert!(!runtime.needs_frame());
}
