use surface::{ExtendedPaintOp, Length, Node, PaintOp, Rect, Role, Style, TextMeasurer};
use surface_app::{Application, EditKey, Event, Runtime, View};
use surface_material::{
    shape::LayoutDirection, time_dial_layout, ClockFormat, DialEvent, DialMode, DialPreview,
    DialState, Theme, TimeDialLabels, TimeDialLayout, TimeDialOptions, TimeField, TimeInputEffect,
    TimeInputLabels, TimeInputLocale, TimeInputOptions, TimeInputPresentation, TimeInputSession,
    TimeOfDay,
};

const HOUR: &str = "time-hour-selector";
const MINUTE: &str = "time-minute-selector";
const FACE: &str = "time-dial-face";
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
    viewport: Rect,
    direction: LayoutDirection,
    clock: ClockFormat,
    locale: TimeInputLocale<'static>,
    scale: f32,
    saved: TimeOfDay,
    commits: usize,
}

impl App {
    fn new(width: f32, height: f32, clock: ClockFormat, direction: LayoutDirection) -> Self {
        Self {
            state: None,
            viewport: Rect::new(0., 0., width, height),
            direction,
            clock,
            locale: locale(),
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
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => {
                let input = TimeInputSession::new(Some(self.saved), self.clock, self.locale);
                self.state = Some(DialState::new(
                    input,
                    DialMode::Dial,
                    self.layout().is_some(),
                ));
            }
            Message::Resize(viewport, scale) => {
                self.viewport = viewport;
                self.scale = scale;
                let fits = self.layout().is_some();
                if let Some(state) = &mut self.state {
                    state.apply(DialEvent::SetDialFits(fits));
                }
            }
            Message::Dial(event) => {
                let Some(state) = &mut self.state else { return };
                match state.apply(event) {
                    TimeInputEffect::Commit(value) => {
                        self.saved = value;
                        self.commits += 1;
                        self.state = None;
                    }
                    TimeInputEffect::Dismiss => self.state = None,
                    TimeInputEffect::None => {}
                }
            }
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
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
            theme.time_dial(
                TimeDialOptions {
                    input: TimeInputOptions {
                        id: "time",
                        title: "選擇時間",
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
                },
                state,
                Message::Dial,
            )
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
