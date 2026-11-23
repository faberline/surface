use surface::{Length, Node, Rect, Role, Style, TextMeasurer, TextScaling};
use surface_app::{
    AnimationFrame, AnimationUpdate, Application, EditKey, Event, FrameInput, ImeEvent,
    InputAnimationUpdate, Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, ClockFormat, Theme, TimeField, TimeInputEffect, TimeInputError,
    TimeInputEvent, TimeInputLabels, TimeInputLocale, TimeInputOptions, TimeInputPresentation,
    TimeInputSession, TimeOfDay,
};

const HOUR: &str = "time-hour-input";
const MINUTE: &str = "time-minute-input";
const LABELS: TimeInputLabels<'static> = TimeInputLabels {
    hour: "小時",
    minute: "分鐘",
    period: "時段",
    hour_hint: "請輸入小時",
    minute_hint: "00–59",
    confirm: "確定",
    cancel: "取消",
    error_text: &|field, error, _| match (field, error) {
        (TimeField::Hour, TimeInputError::Required) => "請輸入小時".into(),
        (TimeField::Minute, TimeInputError::Required) => "請輸入分鐘".into(),
        (TimeField::Hour, _) => "小時範圍錯誤".into(),
        (TimeField::Minute, _) => "分鐘範圍錯誤".into(),
    },
};

fn locale() -> TimeInputLocale<'static> {
    TimeInputLocale::new("上午", "下午", ":")
}

#[derive(Clone)]
enum Message {
    Open,
    Time(TimeInputEvent),
}

struct Demo {
    session: Option<TimeInputSession>,
    clock: ClockFormat,
    presentation: TimeInputPresentation,
    direction: LayoutDirection,
    viewport: Rect,
    scale: f32,
    disabled: bool,
    saved: Option<TimeOfDay>,
    commits: usize,
}

impl Application for Demo {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => {
                self.session = Some(TimeInputSession::new(self.saved, self.clock, locale()));
            }
            Message::Time(event) => {
                let Some(session) = &mut self.session else {
                    return;
                };
                match session.apply(event) {
                    TimeInputEffect::None => {}
                    TimeInputEffect::Commit(time) => {
                        self.saved = Some(time);
                        self.commits += 1;
                        self.session = None;
                    }
                    TimeInputEffect::Dismiss => self.session = None,
                }
            }
        }
    }

    fn observes_frame_input(&self) -> bool {
        self.session.is_some()
    }

    fn animation_frame_with_input(
        &mut self,
        _frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
    ) -> InputAnimationUpdate {
        let field = match input.focused {
            Some(HOUR) => Some(TimeField::Hour),
            Some(MINUTE) => Some(TimeField::Minute),
            _ => None,
        };
        let updated = self
            .session
            .as_mut()
            .is_some_and(|session| session.observe_input_focus(field));
        InputAnimationUpdate {
            animation: AnimationUpdate {
                rebuild: updated,
                request_next: false,
            },
            ..Default::default()
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn view(&self) -> View<Message> {
        let opener = View::new(
            Node::new(Role::Button, "開啟時間")
                .id("open")
                .styled(Style::row().height(Length::Px(48.))),
        )
        .on_press(Message::Open);
        let picker = self.session.as_ref().map(|session| {
            let theme = if self.scale > 1. {
                Theme::dark()
            } else {
                Theme::light()
            };
            theme.time_input(
                TimeInputOptions {
                    id: "time",
                    title: "輸入時間",
                    labels: &LABELS,
                    locale: locale(),
                    direction: self.direction,
                    presentation: self.presentation,
                    viewport: self.viewport,
                    disabled: self.disabled,
                    restore_focus_to: Some("open"),
                },
                session,
                Message::Time,
            )
        });
        View::new(
            Node::new(Role::Window, "Time test").styled(Style::column().width(Length::Percent(1.))),
        )
        .children(std::iter::once(opener).chain(picker))
    }
}

struct TestMetrics;

impl TextMeasurer for TestMetrics {
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

fn demo(presentation: TimeInputPresentation, clock: ClockFormat, width: f32, height: f32) -> Demo {
    Demo {
        session: None,
        clock,
        presentation,
        direction: LayoutDirection::LeftToRight,
        viewport: Rect::new(0., 0., width, height),
        scale: 1.,
        disabled: false,
        saved: None,
        commits: 0,
    }
}

fn render(runtime: &mut Runtime<Demo>) {
    let view = runtime.app().viewport;
    runtime
        .render(view.width, view.height, &mut TestMetrics)
        .unwrap();
}

fn open(app: Demo) -> Runtime<Demo> {
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime);
    tab(&mut runtime);
    runtime.activate("open").unwrap();
    render(&mut runtime);
    runtime
}

fn tab(runtime: &mut Runtime<Demo>) {
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    render(runtime);
}

fn node<'a>(runtime: &'a Runtime<Demo>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn bounds(runtime: &Runtime<Demo>, id: &str) -> Rect {
    let id = runtime.dom().by_id(id).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn description(runtime: &Runtime<Demo>, id: &str) -> Option<String> {
    let frame = runtime.accessibility_frame().unwrap();
    let id = runtime.dom().by_id(id).unwrap();
    frame
        .description_target(id)
        .and_then(|id| frame.node(id))
        .and_then(|source| source.node.stable_id())
        .map(str::to_owned)
}

fn replace(runtime: &mut Runtime<Demo>, value: &str) {
    runtime
        .dispatch(Event::InputValue {
            value: value.to_owned(),
            anchor: value.len(),
            cursor: value.len(),
        })
        .unwrap();
    render(runtime);
}

fn enter(runtime: &mut Runtime<Demo>) -> bool {
    let handled = runtime
        .key(&KeyPress::new(Key::Enter, Modifiers::default()))
        .unwrap();
    render(runtime);
    handled
}

#[test]
fn modal_input_exposes_per_field_help_and_errors_then_commits_only_on_confirm() {
    let mut runtime = open(demo(
        TimeInputPresentation::Modal,
        ClockFormat::TwelveHour,
        960.,
        720.,
    ));
    assert_eq!(runtime.focused(), Some(HOUR));
    assert_eq!(node(&runtime, "time").role(), Role::Dialog);
    assert_eq!(node(&runtime, "time-separator").style().font_size, 57.);
    assert_eq!(
        node(&runtime, "time-separator").text_scaling_mode(),
        TextScaling::Fixed
    );
    assert_eq!(
        runtime.accessibility_frame().unwrap().modal_root(),
        runtime.dom().by_id("time")
    );
    assert_eq!(node(&runtime, HOUR).name(), "小時");
    assert_eq!(node(&runtime, MINUTE).name(), "分鐘");
    assert_eq!(
        description(&runtime, HOUR).as_deref(),
        Some("time-hour-hint")
    );
    assert_eq!(
        description(&runtime, MINUTE).as_deref(),
        Some("time-minute-hint")
    );

    runtime.activate("time-confirm").unwrap();
    render(&mut runtime);
    assert_eq!(runtime.app().commits, 0);
    assert!(runtime.app().session.is_some());
    assert_eq!(
        description(&runtime, HOUR).as_deref(),
        Some("time-hour-error")
    );
    assert_eq!(
        description(&runtime, MINUTE).as_deref(),
        Some("time-minute-error")
    );

    tab(&mut runtime);
    assert_eq!(runtime.focused(), Some(HOUR));
    replace(&mut runtime, "１２");
    assert!(enter(&mut runtime));
    assert_eq!(node(&runtime, HOUR).text_value(), Some("12"));
    assert_eq!(
        description(&runtime, HOUR).as_deref(),
        Some("time-hour-hint")
    );
    assert_eq!(
        description(&runtime, MINUTE).as_deref(),
        Some("time-minute-error")
    );
    tab(&mut runtime);
    assert_eq!(runtime.focused(), Some(MINUTE));
    replace(&mut runtime, "６");
    tab(&mut runtime);
    assert_eq!(runtime.focused(), Some("time-am"));
    assert_eq!(node(&runtime, MINUTE).text_value(), Some("06"));
    runtime
        .dispatch(Event::Edit {
            key: EditKey::Down,
            extend: false,
            by_word: false,
        })
        .unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("time-pm"));
    assert_eq!(node(&runtime, "time-pm").is_checked(), Some(true));
    assert_eq!(runtime.app().commits, 0);

    runtime.activate("time-confirm").unwrap();
    render(&mut runtime);
    assert_eq!(runtime.app().saved, TimeOfDay::new(12, 6));
    assert_eq!(runtime.app().commits, 1);
    assert_eq!(runtime.focused(), Some("open"));
}

#[test]
fn ime_enter_does_not_finish_or_save_active_composition() {
    let mut app = demo(
        TimeInputPresentation::Modal,
        ClockFormat::TwentyFourHour,
        960.,
        720.,
    );
    app.saved = TimeOfDay::new(8, 7);
    let mut runtime = open(app);
    replace(&mut runtime, "");
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "９".into(),
            selection: None,
        }))
        .unwrap();
    assert!(!enter(&mut runtime));
    assert!(runtime.input_snapshot().unwrap().composing);
    assert_eq!(runtime.app().commits, 0);
    let raw = runtime.app().session.as_ref().unwrap().raw(TimeField::Hour);
    assert_eq!(raw, "");
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("９".into())))
        .unwrap();
    render(&mut runtime);
    assert_eq!(node(&runtime, HOUR).text_value(), Some("９"));
    assert!(enter(&mut runtime));
    assert_eq!(node(&runtime, HOUR).text_value(), Some("09"));
    assert_eq!(runtime.app().saved, TimeOfDay::new(8, 7));
    assert_eq!(runtime.app().commits, 0);
    assert!(runtime.dom().by_id("time-am").is_none());

    runtime
        .dispatch(Event::PointerDown { x: 4., y: 4. })
        .unwrap();
    runtime.dispatch(Event::PointerUp { x: 4., y: 4. }).unwrap();
    render(&mut runtime);
    assert!(runtime.app().session.is_none());
    assert_eq!(runtime.app().saved, TimeOfDay::new(8, 7));
    assert_eq!(runtime.focused(), Some("open"));
}

#[test]
fn compact_rtl_modal_keeps_actions_visible_while_the_editor_scrolls_at_double_text_size() {
    let mut app = demo(
        TimeInputPresentation::Modal,
        ClockFormat::TwelveHour,
        320.,
        240.,
    );
    app.direction = LayoutDirection::RightToLeft;
    app.scale = 2.;
    app.saved = TimeOfDay::new(9, 7);
    let mut runtime = open(app);
    assert_eq!(runtime.focused(), Some(HOUR));
    let panel = bounds(&runtime, "time");
    let hour = bounds(&runtime, HOUR);
    let minute = bounds(&runtime, MINUTE);
    let confirm = bounds(&runtime, "time-confirm");
    let cancel = bounds(&runtime, "time-cancel");
    let body = bounds(&runtime, "time-body");
    let view = runtime.app().viewport;
    assert_eq!(panel.intersect(view), panel);
    assert_eq!(node(&runtime, "time").name(), "輸入時間");
    assert!(runtime.dom().by_id("time-title").is_none());
    assert_eq!(node(&runtime, "time-separator").style().font_size, 22.);
    assert_eq!(hour.width, minute.width);
    assert!(hour.x >= minute.x + minute.width);
    assert!(hour.intersect(body).height >= 48.);
    assert_eq!(confirm.intersect(view), confirm);
    assert_eq!(cancel.intersect(view), cancel);
    assert!(confirm.y >= body.y + body.height);
    assert!(confirm.height >= 48. && cancel.height >= 48.);
    for hint in ["time-hour-hint", "time-minute-hint"] {
        let bounds = bounds(&runtime, hint);
        assert_eq!(
            bounds.intersect(body),
            bounds,
            "initial compact hint {hint}"
        );
    }
    assert_eq!(
        description(&runtime, HOUR).as_deref(),
        Some("time-hour-hint")
    );
    assert_eq!(
        description(&runtime, MINUTE).as_deref(),
        Some("time-minute-hint")
    );
    replace(&mut runtime, "13");
    assert!(enter(&mut runtime));
    assert_eq!(
        description(&runtime, HOUR).as_deref(),
        Some("time-hour-error")
    );
    assert_eq!(
        description(&runtime, MINUTE).as_deref(),
        Some("time-minute-hint")
    );
    assert!(bounds(&runtime, "time-hour-error").intersect(body).height > 0.);
    let scroll = runtime.scroll_metrics("time-body").unwrap();
    assert!(scroll.content.1 > scroll.viewport.1);
    let y = body.y + body.height / 2.;
    runtime
        .dispatch(Event::Scroll {
            x: body.x + body.width / 2.,
            y,
            delta_x: 0.,
            delta_y: 500.,
        })
        .unwrap();
    render(&mut runtime);
    assert_ne!(
        runtime.scroll_metrics("time-body").unwrap().offset.1,
        scroll.offset.1
    );
    assert_eq!(bounds(&runtime, "time-confirm"), confirm);
    assert_eq!(bounds(&runtime, "time-cancel"), cancel);
}

#[test]
fn inline_keeps_the_surrounding_scope_and_exposes_preview_without_committing() {
    let mut app = demo(
        TimeInputPresentation::Inline,
        ClockFormat::TwelveHour,
        320.,
        540.,
    );
    app.saved = TimeOfDay::new(0, 5);
    app.direction = LayoutDirection::RightToLeft;
    let mut runtime = open(app);
    assert_eq!(runtime.focused(), Some("open"));
    assert_eq!(node(&runtime, "time").role(), Role::Group);
    assert_eq!(runtime.accessibility_frame().unwrap().modal_root(), None);
    assert!(runtime.dom().by_id("time-confirm").is_none());
    assert!(runtime.dom().by_id("time-cancel").is_none());
    assert_eq!(node(&runtime, "time-am").is_checked(), Some(true));
    tab(&mut runtime);
    assert_eq!(runtime.focused(), Some(HOUR));
    replace(&mut runtime, "１１");
    assert_eq!(
        runtime
            .app()
            .session
            .as_ref()
            .unwrap()
            .validation()
            .candidate(),
        TimeOfDay::new(11, 5)
    );
    assert_eq!(runtime.app().saved, TimeOfDay::new(0, 5));
}

#[test]
fn disabled_modal_gives_focus_to_cancel_and_has_no_writable_field() {
    let mut app = demo(
        TimeInputPresentation::Modal,
        ClockFormat::TwelveHour,
        960.,
        720.,
    );
    app.disabled = true;
    app.saved = TimeOfDay::new(9, 7);
    let mut runtime = open(app);
    assert_eq!(runtime.focused(), Some("time-cancel"));
    for id in [HOUR, MINUTE, "time-am", "time-pm", "time-confirm"] {
        assert!(node(&runtime, id).is_disabled());
    }
    assert!(runtime.activate("time-confirm").is_err());
    runtime.activate("time-cancel").unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("open"));
    assert_eq!(runtime.app().commits, 0);
}
