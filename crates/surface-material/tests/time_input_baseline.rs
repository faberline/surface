use surface::{Length, Node, Rect, Role, Style, TextMeasurer, TextScaling};
use surface_app::{
    AnimationFrame, AnimationUpdate, Application, EditKey, Event, FrameInput, ImeEvent,
    InputAnimationUpdate, Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    shape::{self, LayoutDirection},
    ClockFormat, Elevation, Theme, TimeField, TimeInputEffect, TimeInputError, TimeInputEvent,
    TimeInputLabels, TimeInputLocale, TimeInputOptions, TimeInputPresentation, TimeInputSession,
    TimeOfDay,
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

const COMPACT_LABELS: TimeInputLabels<'static> = TimeInputLabels {
    confirm: "儲存時間設定",
    cancel: "取消時間設定",
    ..LABELS
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
    locale: TimeInputLocale<'static>,
    labels: &'static TimeInputLabels<'static>,
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
                self.session = Some(TimeInputSession::new(self.saved, self.clock, self.locale));
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
            .is_some_and(|session| session.observe_baseline_input_focus(field));
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
            theme.time_input_baseline(
                TimeInputOptions {
                    id: "time",
                    title: "輸入時間",
                    labels: self.labels,
                    locale: self.locale,
                    direction: self.direction,
                    presentation: TimeInputPresentation::Inline,
                    viewport: self.viewport,
                    disabled: self.disabled,
                    restore_focus_to: Some("open"),
                },
                self.scale,
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

fn demo(clock: ClockFormat, width: f32, height: f32) -> Demo {
    Demo {
        session: None,
        clock,
        locale: locale(),
        labels: &LABELS,
        direction: LayoutDirection::LeftToRight,
        viewport: Rect::new(0., 0., width, height),
        scale: 1.,
        disabled: false,
        saved: TimeOfDay::new(9, 7),
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
fn baseline_twelve_hour_separates_token_paint_from_period_hit_bounds() {
    let theme = Theme::light();
    let mut runtime = open(demo(ClockFormat::TwelveHour, 390., 720.));
    let panel = node(&runtime, "time");
    assert_eq!(panel.role(), Role::Dialog);
    assert_eq!(panel.style().padding, 24.);
    assert_eq!(
        panel.style().background,
        theme.colors.surface_container_high
    );
    assert_eq!(panel.style().corner_radii(), shape::EXTRA_LARGE);
    assert_eq!(
        panel.box_shadow_layers(),
        theme.elevation(Elevation::Level3)
    );
    assert_eq!(
        runtime.accessibility_frame().unwrap().modal_root(),
        runtime.dom().by_id("time"),
    );
    let title = node(&runtime, "time-title").style();
    assert_eq!(title.font_size, 12.);
    assert_eq!(title.foreground, theme.colors.on_surface_variant);

    for id in [HOUR, MINUTE] {
        let target = bounds(&runtime, id);
        assert_eq!((target.width, target.height), (96., 72.));
        let editor = node(&runtime, id).style();
        assert_eq!(editor.font_size, 45.);
        assert_eq!(editor.corner_radii(), shape::SMALL);
    }
    let separator = node(&runtime, "time-separator");
    assert_eq!(separator.style().font_size, 57.);
    assert_eq!(separator.text_scaling_mode(), TextScaling::Fixed);

    let group = bounds(&runtime, "time-period");
    let paint = bounds(&runtime, "time-period-paint");
    assert_eq!((group.width, group.height), (52., 96.));
    assert_eq!(paint, Rect::new(group.x, group.y + 12., 52., 72.));
    let am = bounds(&runtime, "time-am");
    let pm = bounds(&runtime, "time-pm");
    assert_eq!(am, Rect::new(group.x, group.y, 52., 48.));
    assert_eq!(pm, Rect::new(group.x, group.y + 48., 52., 48.));
    assert_eq!(
        bounds(&runtime, "time-am-paint"),
        Rect::new(paint.x, paint.y, 52., 36.)
    );
    assert_eq!(
        bounds(&runtime, "time-pm-paint"),
        Rect::new(paint.x, paint.y + 36., 52., 36.)
    );
    assert_eq!(
        node(&runtime, "time-period-paint").style().border_color,
        theme.colors.outline,
    );
    assert_eq!(node(&runtime, "time-period").role(), Role::RadioGroup);
    assert_eq!(node(&runtime, "time-am").is_checked(), Some(true));
    assert_eq!(
        node(&runtime, "time-am-paint").style().background,
        theme.colors.tertiary_container
    );
    runtime.activate("time-am").unwrap();
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
    assert_eq!(
        node(&runtime, "time-pm-paint").style().background,
        theme.colors.tertiary_container
    );
    assert_eq!(runtime.app().commits, 0);
}

#[test]
fn baseline_editor_focus_shifts_tint_and_preserves_validation_and_ime() {
    let theme = Theme::light();
    let mut runtime = open(demo(ClockFormat::TwentyFourHour, 480., 600.));
    assert_eq!(runtime.focused(), Some(HOUR));
    let focused = node(&runtime, HOUR).style();
    assert_eq!(focused.background, theme.colors.primary_container);
    assert_eq!(focused.foreground, theme.colors.on_primary_container);
    assert_eq!(
        (focused.border_width, focused.border_color),
        (2., theme.colors.primary)
    );
    assert_eq!(
        node(&runtime, MINUTE).style().background,
        theme.colors.surface_container_highest
    );
    assert!(runtime.dom().by_id("time-period").is_none());

    tab(&mut runtime);
    assert_eq!(runtime.focused(), Some(MINUTE));
    assert_eq!(
        node(&runtime, HOUR).style().background,
        theme.colors.surface_container_highest
    );
    assert_eq!(
        node(&runtime, MINUTE).style().background,
        theme.colors.primary_container
    );
    replace(&mut runtime, "60");
    assert!(enter(&mut runtime));
    let invalid = node(&runtime, MINUTE).style();
    assert_eq!(
        (invalid.border_width, invalid.border_color),
        (2., theme.colors.error)
    );
    assert_eq!(
        description(&runtime, MINUTE).as_deref(),
        Some("time-minute-error")
    );
    assert_eq!(runtime.app().commits, 0);

    runtime
        .dispatch(Event::Select {
            anchor: 0,
            cursor: 2,
        })
        .unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "０８".into(),
            selection: Some((6, 6)),
        }))
        .unwrap();
    assert!(!enter(&mut runtime));
    assert!(runtime.input_snapshot().unwrap().composing);
    assert_eq!(
        runtime
            .app()
            .session
            .as_ref()
            .unwrap()
            .raw(TimeField::Minute),
        "60"
    );
    assert_eq!(runtime.app().commits, 0);
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("０８".into())))
        .unwrap();
    render(&mut runtime);
    assert!(enter(&mut runtime));
    assert_eq!(node(&runtime, MINUTE).text_value(), Some("08"));
    runtime.activate("time-confirm").unwrap();
    render(&mut runtime);
    assert_eq!(runtime.app().saved, TimeOfDay::new(9, 8));
    assert_eq!(runtime.app().commits, 1);
    assert_eq!(runtime.focused(), Some("open"));
}

#[test]
fn compact_rtl_baseline_keeps_measured_footer_and_scrolls_between_the_fields() {
    let mut app = demo(ClockFormat::TwelveHour, 320., 240.);
    app.scale = 2.;
    app.direction = LayoutDirection::RightToLeft;
    app.labels = &COMPACT_LABELS;
    app.locale = locale()
        .with_digits(['０', '１', '２', '３', '４', '５', '６', '７', '８', '９'])
        .unwrap();
    let mut runtime = open(app);
    let panel = bounds(&runtime, "time");
    assert_eq!(panel.intersect(runtime.app().viewport), panel);
    assert_eq!(node(&runtime, "time").name(), "輸入時間");
    assert!(runtime.dom().by_id("time-title").is_none());
    assert!(runtime.dom().by_id("time-separator").is_none());
    let actions = bounds(&runtime, "time-actions");
    assert!(actions.height > 48.);
    for id in ["time-cancel", "time-confirm"] {
        let target = bounds(&runtime, id);
        assert!(target.width >= 48. && target.height >= 48.);
        assert_eq!(target.intersect(panel), target);
    }
    let body = bounds(&runtime, "time-body");
    let hour = bounds(&runtime, HOUR);
    assert!(hour.height >= 116.);
    assert!(body.y + body.height <= actions.y);
    assert!(runtime.scroll_metrics("time-body").unwrap().content.1 > body.height);
    assert_eq!(runtime.focused(), Some(HOUR));

    tab(&mut runtime);
    assert_eq!(runtime.focused(), Some(MINUTE));
    assert!(runtime.scroll_metrics("time-body").unwrap().offset.1 > 0.);
    let minute = bounds(&runtime, MINUTE);
    assert!(minute.intersect(body).height >= 48.);
    runtime.dispatch(Event::Cancel).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.app().commits, 0);
    assert_eq!(runtime.focused(), Some("open"));
}

#[test]
fn baseline_input_contains_long_localized_periods_with_full_radio_names() {
    let am = "夜明けから正午までの午前の時間を選びます";
    let pm = "正午から夜までの午後の時間を選びます";
    let mut app = demo(ClockFormat::TwelveHour, 320., 720.);
    app.direction = LayoutDirection::RightToLeft;
    app.scale = 2.;
    app.locale = TimeInputLocale::new(am, pm, ":");
    let runtime = open(app);
    let form = bounds(&runtime, "time-form");
    let group = bounds(&runtime, "time-period");
    assert!(group.x >= form.x && group.x + group.width <= form.x + form.width);
    assert!(group.width >= 48.);
    for (id, name) in [("time-am", am), ("time-pm", pm)] {
        let target = bounds(&runtime, id);
        assert_eq!(target.intersect(group), target);
        assert!(target.width >= 48. && target.height >= 48.);
        let frame = runtime.accessibility_frame().unwrap();
        let index = runtime.dom().by_id(id).unwrap();
        assert_eq!(frame.node(index).unwrap().node.name(), name);
        assert!(frame.accepts_focus(index));
    }
}

#[test]
fn disabled_baseline_opens_at_cancel_with_resting_input_paint() {
    let mut app = demo(ClockFormat::TwelveHour, 390., 600.);
    app.disabled = true;
    let runtime = open(app);
    assert_eq!(runtime.focused(), Some("time-cancel"));
    for id in [HOUR, MINUTE, "time-am", "time-pm", "time-confirm"] {
        assert!(node(&runtime, id).is_disabled());
        let id = runtime.dom().by_id(id).unwrap();
        assert!(!runtime.accessibility_frame().unwrap().accepts_focus(id));
    }
    assert_eq!(
        node(&runtime, HOUR).style().background,
        Theme::light().colors.surface_container_highest,
    );
}
