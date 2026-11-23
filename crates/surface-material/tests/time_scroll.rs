use surface::{Node, Rect, TextMeasurer};
use surface_app::{
    AccessibilityAdjustment, AccessibilityAdjustmentResult, AccessibilityFocusResult,
    AnimationFrame, Application, EditKey, Event, FrameInput, InputAnimationUpdate, Key, KeyPress,
    Modifiers, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, time_scroll_layout, ClockFormat, Theme, TimeField, TimeInputEffect,
    TimeInputLabels, TimeInputLocale, TimeInputOptions, TimeInputPresentation, TimeInputSession,
    TimeOfDay, TimeScrollEvent, TimeScrollLabels, TimeScrollMode, TimeScrollOptions,
    TimeScrollState,
};

const PICKER: &str = "picker";
const HOUR: &str = "picker-hour-wheel";
const MINUTE: &str = "picker-minute-wheel";
const OPEN: &str = "open";
const INPUT_LABELS: TimeInputLabels<'static> = TimeInputLabels {
    hour: "小時",
    minute: "分鐘",
    period: "時段",
    hour_hint: "輸入小時",
    minute_hint: "輸入分鐘",
    confirm: "儲存",
    cancel: "取消",
    error_text: &|_, _, _| "無效的時間".into(),
};
const SCROLL_LABELS: TimeScrollLabels<'static> = TimeScrollLabels {
    use_input: "文字輸入",
    use_scroll: "捲動輸入",
    unfit: "空間不足",
    unset: "未設定",
};

fn time(hour: u8, minute: u8) -> TimeOfDay {
    TimeOfDay::new(hour, minute).unwrap()
}

fn locale() -> TimeInputLocale<'static> {
    TimeInputLocale::new("上午", "下午", ":")
}

#[derive(Clone)]
enum Message {
    Open,
    Picker(TimeScrollEvent),
}

struct App {
    state: Option<TimeScrollState>,
    viewport: Rect,
    scale: f32,
    clock: ClockFormat,
    presentation: TimeInputPresentation,
    disabled: bool,
    saved: TimeOfDay,
    commits: usize,
}

impl App {
    fn new(presentation: TimeInputPresentation, clock: ClockFormat, scale: f32) -> Self {
        let mut app = Self {
            state: None,
            viewport: Rect::new(0., 0., 840., 720.),
            scale,
            clock,
            presentation,
            disabled: false,
            saved: time(9, 7),
            commits: 0,
        };
        if presentation == TimeInputPresentation::Inline {
            app.open();
        }
        app
    }

    fn fits(&self) -> bool {
        time_scroll_layout(
            self.viewport.width,
            self.viewport.height,
            self.scale,
            self.clock,
            self.presentation,
        )
        .is_some()
    }

    fn open(&mut self) {
        let draft = TimeInputSession::new(Some(self.saved), self.clock, locale());
        self.state = Some(TimeScrollState::new(
            draft,
            TimeScrollMode::Scroll,
            self.fits(),
        ));
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        let Message::Picker(event) = message else {
            self.open();
            return;
        };
        let Some(state) = self.state.as_mut() else {
            return;
        };
        match state.apply(event, false) {
            TimeInputEffect::Commit(value) => {
                self.saved = value;
                self.commits += 1;
                if self.presentation == TimeInputPresentation::Modal {
                    self.state = None;
                }
            }
            TimeInputEffect::Dismiss => self.state = None,
            TimeInputEffect::None => {}
        }
    }

    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let viewport = Rect::new(0., 0., width, height);
        if self.viewport == viewport {
            return false;
        }
        self.viewport = viewport;
        let fits = self.fits();
        if let Some(state) = self.state.as_mut() {
            state.apply(TimeScrollEvent::SetScrollFits(fits), true);
        }
        true
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn animation_frame_with_input(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
    ) -> InputAnimationUpdate {
        let Some(state) = self.state.as_mut() else {
            return InputAnimationUpdate::default();
        };
        let field = match input.focused {
            Some(HOUR | "picker-hour-input") => Some(TimeField::Hour),
            Some(MINUTE | "picker-minute-input") => Some(TimeField::Minute),
            _ => None,
        };
        let focus_change = match state.mode() {
            TimeScrollMode::Scroll => state.observe_scroll_focus(field),
            TimeScrollMode::Input => state.observe_input_focus(field),
        };
        let mut animation = state.advance(frame.now_ms, frame.reduced_motion);
        animation.rebuild |= focus_change;
        InputAnimationUpdate {
            animation,
            suppress_touch_activation: false,
        }
    }

    fn view(&self) -> View<Message> {
        let theme = Theme::dark();
        let picker = self.state.as_ref().map(|state| {
            theme.time_scroll(
                TimeScrollOptions {
                    input: TimeInputOptions {
                        id: PICKER,
                        title: "選擇時間",
                        labels: &INPUT_LABELS,
                        locale: locale(),
                        direction: LayoutDirection::RightToLeft,
                        presentation: self.presentation,
                        viewport: self.viewport,
                        disabled: self.disabled,
                        restore_focus_to: Some(OPEN),
                    },
                    labels: &SCROLL_LABELS,
                    text_scale: self.scale,
                },
                state,
                Message::Picker,
            )
        });
        theme.page(std::iter::once(theme.button(OPEN, "開啟", Message::Open)).chain(picker))
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let limit = width.unwrap_or(f32::INFINITY);
        let mut line_width: f32 = 0.;
        let mut longest: f32 = 0.;
        let mut lines = 1;
        for ch in text.chars() {
            let advance = if ch.is_ascii() { size * 0.58 } else { size };
            if line_width > 0. && line_width + advance > limit {
                longest = longest.max(line_width);
                line_width = 0.;
                lines += 1;
            }
            line_width += advance;
        }
        (longest.max(line_width), lines as f32 * size * 1.5)
    }
}

fn draw(runtime: &mut Runtime<App>) {
    let viewport = runtime.app().viewport;
    runtime
        .render(viewport.width, viewport.height, &mut Metrics)
        .unwrap();
}

fn start(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    runtime.set_reduced_motion(true);
    draw(&mut runtime);
    runtime
}

fn open(runtime: &mut Runtime<App>) {
    runtime.activate(OPEN).unwrap();
    draw(runtime);
}

fn state(runtime: &Runtime<App>) -> &TimeScrollState {
    runtime.app().state.as_ref().unwrap()
}

fn node<'a>(runtime: &'a Runtime<App>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn visible_rect(runtime: &Runtime<App>, id: &str) -> Rect {
    let id = runtime.dom().by_id(id).unwrap();
    let computed = runtime.computed().unwrap();
    let node = &computed.nodes[id.index()];
    assert!(node.bounds.width > 0. && node.bounds.height > 0.);
    assert_eq!(node.bounds, node.clip.intersect(node.bounds));
    assert_eq!(node.bounds, computed.viewport.intersect(node.bounds));
    node.bounds
}

fn projected_focus(runtime: &mut Runtime<App>, id: &str) {
    let node = runtime.dom().by_id(id).unwrap();
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(node)
        .unwrap();
    let result = runtime.accessibility_focus(&target).unwrap();
    assert!(matches!(
        result,
        AccessibilityFocusResult::Moved | AccessibilityFocusResult::Unchanged
    ));
    draw(runtime);
    assert_eq!(runtime.focused(), Some(id));
}

fn projected_key(runtime: &mut Runtime<App>, source: &str, key: Key, fallback: Event) {
    let source = runtime.dom().by_id(source).unwrap();
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(source)
        .unwrap();
    let press = KeyPress::new(key, Modifiers::default());
    assert!(runtime
        .accessibility_key(&target, &press, fallback)
        .unwrap());
    draw(runtime);
}

fn edit(key: EditKey) -> Event {
    Event::Edit {
        key,
        extend: false,
        by_word: false,
    }
}

#[test]
fn inline_wheels_have_three_target_rows_and_one_adjustable_owner_each() {
    let mut runtime = start(App::new(
        TimeInputPresentation::Inline,
        ClockFormat::TwentyFourHour,
        1.,
    ));
    assert_eq!(runtime.accessibility_frame().unwrap().modal_root(), None);
    for id in [HOUR, MINUTE] {
        assert_eq!(visible_rect(&runtime, id).height, 3. * 48.);
    }
    assert!(visible_rect(&runtime, HOUR).x > visible_rect(&runtime, MINUTE).x);
    assert!(runtime.dom().by_id("picker-period").is_none());

    projected_focus(&mut runtime, HOUR);
    assert_eq!(
        node(&runtime, HOUR)
            .accessibility_adjustable()
            .unwrap()
            .value,
        Some(9.)
    );
    projected_key(&mut runtime, HOUR, Key::ArrowLeft, edit(EditKey::Left));
    assert_eq!(runtime.focused(), Some(MINUTE));
    assert_eq!(state(&runtime).active_field(), TimeField::Minute);
    projected_key(&mut runtime, MINUTE, Key::ArrowDown, edit(EditKey::Down));
    let id = runtime.dom().by_id(MINUTE).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.children(id).count(), 0);
    assert_eq!(
        frame
            .node(id)
            .unwrap()
            .node
            .accessibility_adjustable()
            .unwrap()
            .value,
        Some(8.)
    );
    let target = frame.adjustment_target(id).unwrap();
    assert_eq!(
        runtime
            .accessibility_adjust(&target, AccessibilityAdjustment::SetValue(59.))
            .unwrap(),
        AccessibilityAdjustmentResult::Adjusted,
    );
    assert_eq!(
        state(&runtime).input().validation().candidate(),
        Some(time(9, 59))
    );
    assert_eq!(runtime.focused(), Some(MINUTE));
    assert_eq!(runtime.app().commits, 0);
}

#[test]
fn disabled_inline_wheels_offer_no_focus_or_adjustment() {
    let mut app = App::new(
        TimeInputPresentation::Inline,
        ClockFormat::TwentyFourHour,
        1.,
    );
    app.disabled = true;
    let runtime = start(app);
    let frame = runtime.accessibility_frame().unwrap();
    for key in [HOUR, MINUTE] {
        let id = runtime.dom().by_id(key).unwrap();
        assert!(frame.node(id).unwrap().node.is_disabled());
        assert!(frame.focus_target(id).is_none());
        assert!(frame.adjustment_target(id).is_none());
    }
    assert!(runtime.dom().by_id("picker-period").is_none());
}

#[test]
fn modal_enter_commits_once_and_escape_dismisses_new_draft() {
    let mut runtime = start(App::new(
        TimeInputPresentation::Modal,
        ClockFormat::TwelveHour,
        1.,
    ));
    open(&mut runtime);
    assert_eq!(runtime.focused(), Some(HOUR));
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.modal_root(), runtime.dom().by_id(PICKER));
    assert!(frame.node(runtime.dom().by_id(OPEN).unwrap()).is_none());
    projected_key(&mut runtime, HOUR, Key::ArrowDown, edit(EditKey::Down));
    projected_key(&mut runtime, HOUR, Key::Enter, Event::Activate);
    assert!(runtime.app().state.is_none());
    assert_eq!(runtime.focused(), Some(OPEN));
    assert_eq!(
        (runtime.app().saved, runtime.app().commits),
        (time(10, 7), 1)
    );

    open(&mut runtime);
    projected_focus(&mut runtime, MINUTE);
    projected_key(&mut runtime, MINUTE, Key::ArrowDown, edit(EditKey::Down));
    assert_eq!(
        state(&runtime).input().validation().candidate(),
        Some(time(10, 8))
    );
    projected_key(&mut runtime, MINUTE, Key::Escape, Event::Cancel);
    assert!(runtime.app().state.is_none());
    assert_eq!(runtime.focused(), Some(OPEN));
    assert_eq!(
        (runtime.app().saved, runtime.app().commits),
        (time(10, 7), 1)
    );
}

#[test]
fn compact_fallback_and_explicit_return_preserve_the_exact_focused_field() {
    for (wheel, input) in [(HOUR, "picker-hour-input"), (MINUTE, "picker-minute-input")] {
        let mut runtime = start(App::new(
            TimeInputPresentation::Modal,
            ClockFormat::TwelveHour,
            2.,
        ));
        open(&mut runtime);
        projected_focus(&mut runtime, wheel);
        runtime.render(320., 240., &mut Metrics).unwrap();
        assert_eq!(state(&runtime).mode(), TimeScrollMode::Input);
        assert_eq!(runtime.focused(), Some(input));
        assert!(runtime.dom().by_id(wheel).is_none());
        assert!(node(&runtime, "picker-use-scroll").is_disabled());

        runtime.render(840., 720., &mut Metrics).unwrap();
        assert_eq!(state(&runtime).mode(), TimeScrollMode::Input);
        assert_eq!(runtime.focused(), Some(input));
        runtime.activate("picker-use-scroll").unwrap();
        draw(&mut runtime);
        assert_eq!(state(&runtime).mode(), TimeScrollMode::Scroll);
        assert_eq!(runtime.focused(), Some(wheel));
        assert_eq!(
            state(&runtime).input().validation().candidate(),
            Some(time(9, 7))
        );
        assert_eq!(runtime.app().commits, 0);
    }
}

#[test]
fn physical_neighbor_tap_and_release_only_drag_accept_one_value_each() {
    let mut runtime = start(App::new(
        TimeInputPresentation::Inline,
        ClockFormat::TwentyFourHour,
        1.,
    ));
    let wheel = visible_rect(&runtime, MINUTE);
    let (x, y) = (wheel.x + wheel.width / 2., wheel.y + wheel.height / 6.);

    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    draw(&mut runtime);
    assert_eq!(
        state(&runtime).input().validation().candidate(),
        Some(time(9, 6))
    );

    let middle = wheel.y + wheel.height / 2.;
    runtime
        .dispatch(Event::PointerDown { x, y: middle })
        .unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    draw(&mut runtime);
    assert_eq!(
        state(&runtime).input().validation().candidate(),
        Some(time(9, 7))
    );
    assert_eq!(state(&runtime).column(TimeField::Minute).preview, None);
    assert_eq!(runtime.app().commits, 0);
}

#[test]
fn roomy_two_hundred_percent_layout_keeps_whole_wheels_and_actions_visible() {
    assert!(time_scroll_layout(
        320.,
        240.,
        2.,
        ClockFormat::TwelveHour,
        TimeInputPresentation::Modal
    )
    .is_none());
    for presentation in [TimeInputPresentation::Inline, TimeInputPresentation::Modal] {
        let mut runtime = start(App::new(presentation, ClockFormat::TwelveHour, 2.));
        if presentation == TimeInputPresentation::Modal {
            open(&mut runtime);
            for id in ["picker-cancel", "picker-confirm"] {
                let action = visible_rect(&runtime, id);
                assert!(action.height >= 48. && action.width >= 48.);
            }
        }
        for id in [HOUR, MINUTE] {
            let wheel = visible_rect(&runtime, id);
            assert!(wheel.height >= 3. * 48. && wheel.width >= 48.);
        }
        for id in ["picker-am", "picker-pm", "picker-use-input"] {
            let action = visible_rect(&runtime, id);
            assert!(action.height >= 48. && action.width >= 48.);
        }
    }
}
