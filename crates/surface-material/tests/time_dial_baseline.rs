use surface::{Node, Rect, TextMeasurer};
use surface_app::{
    AnimationFrame, Application, EditKey, Event, FrameInput, ImeEvent, InputAnimationUpdate, Key,
    KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    shape::{self, LayoutDirection},
    time_dial_baseline_layout, typography, ClockFormat, DialEvent, DialMode, DialPreview,
    DialState, Elevation, Theme, TimeDialLabels, TimeDialLayout, TimeDialOptions, TimeField,
    TimeInputEffect, TimeInputLabels, TimeInputLocale, TimeInputOptions, TimeInputPresentation,
    TimeInputSession, TimeOfDay,
};

const PICKER: &str = "baseline";
const HOUR: &str = "baseline-hour-selector";
const MINUTE: &str = "baseline-minute-selector";
const FACE: &str = "baseline-dial-face";
const OPEN: &str = "open";
const INPUT_LABELS: TimeInputLabels<'static> = TimeInputLabels {
    hour: "時",
    minute: "分",
    period: "午前/午後",
    hour_hint: "時を入力",
    minute_hint: "分を入力",
    confirm: "保存",
    cancel: "取消",
    error_text: &|_, _, _| "無効な時刻".into(),
};
const DIAL_LABELS: TimeDialLabels<'static> = TimeDialLabels {
    use_input: "文字で入力",
    use_dial: "文字盤を使う",
    unfit: "文字盤には空間が必要です",
    value_name: &|field, value, _| {
        let label = if field == TimeField::Hour {
            "時"
        } else {
            "分"
        };
        format!("{label} {value}")
    },
};

#[derive(Clone)]
enum Message {
    Open,
    Dial(DialEvent),
}

struct App {
    clock: ClockFormat,
    viewport: Rect,
    text_scale: f32,
    direction: LayoutDirection,
    locale: TimeInputLocale<'static>,
    labels: TimeInputLabels<'static>,
    saved: TimeOfDay,
    commits: usize,
    draft: Option<DialState>,
}

impl App {
    fn new(width: f32, height: f32, clock: ClockFormat) -> Self {
        Self {
            viewport: Rect::new(0., 0., width, height),
            clock,
            text_scale: 1.,
            direction: LayoutDirection::LeftToRight,
            locale: TimeInputLocale::new("午前", "午後", ":"),
            labels: INPUT_LABELS,
            saved: TimeOfDay::new(9, 7).unwrap(),
            commits: 0,
            draft: None,
        }
    }

    fn fit(&self) -> Option<TimeDialLayout> {
        time_dial_baseline_layout(
            self.viewport.width,
            self.viewport.height,
            self.text_scale,
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
                self.draft = Some(DialState::new(input, DialMode::Dial, self.fit().is_some()));
            }
            Message::Dial(event) => {
                let Some(draft) = self.draft.as_mut() else {
                    return;
                };
                match draft.apply(event) {
                    TimeInputEffect::Commit(value) => {
                        self.saved = value;
                        self.commits += 1;
                        self.draft = None;
                    }
                    TimeInputEffect::Dismiss => self.draft = None,
                    TimeInputEffect::None => {}
                }
            }
        }
    }

    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let viewport = Rect::new(0., 0., width, height);
        if self.viewport == viewport {
            return false;
        }
        self.viewport = viewport;
        let fits = self.fit().is_some();
        if let Some(draft) = &mut self.draft {
            draft.apply(DialEvent::SetDialFits(fits));
        }
        true
    }

    fn text_scale(&self) -> f32 {
        self.text_scale
    }

    fn animation_frame_with_input(
        &mut self,
        _frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
    ) -> InputAnimationUpdate {
        let field = match input.focused {
            Some("baseline-hour-input") => Some(TimeField::Hour),
            Some("baseline-minute-input") => Some(TimeField::Minute),
            _ => None,
        };
        let mut update = InputAnimationUpdate::default();
        update.animation.rebuild = self
            .draft
            .as_mut()
            .is_some_and(|draft| draft.observe_baseline_input_focus(field));
        update
    }

    fn view(&self) -> View<Message> {
        let theme = Theme::dark();
        let modal = self.draft.as_ref().map(|draft| {
            theme.time_dial_baseline(
                TimeDialOptions {
                    input: TimeInputOptions {
                        id: PICKER,
                        title: "時刻を入力",
                        labels: &self.labels,
                        locale: self.locale,
                        direction: self.direction,
                        presentation: TimeInputPresentation::Modal,
                        viewport: self.viewport,
                        disabled: false,
                        restore_focus_to: Some(OPEN),
                    },
                    labels: &DIAL_LABELS,
                    text_scale: self.text_scale,
                },
                draft,
                Message::Dial,
            )
        });
        theme.page(std::iter::once(theme.button(OPEN, "開く", Message::Open)).chain(modal))
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let limit = width.unwrap_or(f32::INFINITY);
        let mut current: f32 = 0.;
        let mut widest: f32 = 0.;
        let mut lines = 1;
        for ch in text.chars() {
            let advance = size * if ch.is_ascii() { 0.58 } else { 1. };
            if current > 0. && current + advance > limit {
                widest = widest.max(current);
                current = 0.;
                lines += 1;
            }
            current += advance;
        }
        (widest.max(current), lines as f32 * size * 1.5)
    }
}

fn draw(runtime: &mut Runtime<App>) {
    let view = runtime.app().viewport;
    runtime
        .render(view.width, view.height, &mut Metrics)
        .unwrap();
}

fn open(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    draw(&mut runtime);
    runtime.activate(OPEN).unwrap();
    draw(&mut runtime);
    runtime
}

fn state(runtime: &Runtime<App>) -> &DialState {
    runtime.app().draft.as_ref().unwrap()
}

fn node<'a>(runtime: &'a Runtime<App>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn visible(runtime: &Runtime<App>, id: &str) -> Rect {
    let id = runtime.dom().by_id(id).unwrap();
    let layout = runtime.computed().unwrap();
    let geom = &layout.nodes[id.index()];
    assert!(geom.bounds.width > 0. && geom.bounds.height > 0., "{id:?}");
    assert_eq!(
        geom.bounds,
        geom.bounds.intersect(geom.clip),
        "{id:?} clipped"
    );
    assert_eq!(
        geom.bounds,
        geom.bounds.intersect(layout.viewport),
        "{id:?} outside viewport"
    );
    geom.bounds
}

fn size(rect: Rect) -> (f32, f32) {
    (rect.width, rect.height)
}

#[test]
fn baseline_four_arrangements_keep_token_geometry_and_full_period_targets() {
    for clock in [ClockFormat::TwelveHour, ClockFormat::TwentyFourHour] {
        for (width, height, layout) in [
            (390., 720., TimeDialLayout::Vertical),
            (844., 390., TimeDialLayout::Horizontal),
        ] {
            let app = App::new(width, height, clock);
            assert_eq!(app.fit(), Some(layout));
            let runtime = open(app);
            let expected =
                if clock == ClockFormat::TwentyFourHour && layout == TimeDialLayout::Vertical {
                    114.
                } else {
                    96.
                };
            assert_eq!(size(visible(&runtime, HOUR)), (expected, 80.));
            assert_eq!(size(visible(&runtime, MINUTE)), (expected, 80.));
            assert_eq!(size(visible(&runtime, FACE)), (256., 256.));
            assert_eq!(size(visible(&runtime, "baseline-dial-handle")), (48., 48.));
            assert_eq!(size(visible(&runtime, "baseline-dial-center")), (8., 8.));
            assert_eq!(runtime.focused(), Some(HOUR));
            for id in ["baseline-use-input", "baseline-cancel", "baseline-confirm"] {
                let bounds = visible(&runtime, id);
                assert!(bounds.width >= 48. && bounds.height >= 48.);
            }
            if clock == ClockFormat::TwelveHour {
                let am = visible(&runtime, "baseline-am");
                let pm = visible(&runtime, "baseline-pm");
                for period in [am, pm] {
                    assert!(period.width >= 48. && period.height >= 48.);
                }
                let overlap = am.intersect(pm);
                assert!(overlap.width == 0. || overlap.height == 0.);
                let (target, paint) = if layout == TimeDialLayout::Vertical {
                    ((52., 96.), (52., 80.))
                } else {
                    ((216., 48.), (216., 38.))
                };
                assert_eq!(size(visible(&runtime, "baseline-period")), target);
                assert_eq!(size(visible(&runtime, "baseline-period-paint")), paint);
                assert_eq!(
                    node(&runtime, "baseline-period-paint").style().border_width,
                    1.,
                );
                assert_eq!(
                    node(&runtime, "baseline-am-paint").style().background,
                    Theme::dark().colors.tertiary_container,
                );
                for (paint_id, hit) in [("baseline-am-paint", am), ("baseline-pm-paint", pm)] {
                    let paint = visible(&runtime, paint_id);
                    assert_eq!(paint, paint.intersect(hit));
                }
            } else {
                assert!(runtime.dom().by_id("baseline-period").is_none());
            }
        }
    }
}

#[test]
fn baseline_modal_uses_the_legacy_surface_shape_elevation_and_type() {
    let runtime = open(App::new(390., 720., ClockFormat::TwentyFourHour));
    let theme = Theme::dark();
    let panel = node(&runtime, PICKER);
    assert_eq!(
        panel.style().background,
        theme.colors.surface_container_high
    );
    assert_eq!(panel.style().corners, Some(shape::EXTRA_LARGE));
    assert_eq!(panel.style().padding, 24.);
    assert_eq!(
        panel.box_shadow_layers(),
        theme.elevation(Elevation::Level3)
    );
    assert_eq!(
        node(&runtime, "baseline-title").style().text_style(),
        typography::LABEL_MEDIUM
    );
    assert_eq!(
        node(&runtime, HOUR).style().background,
        theme.colors.primary_container
    );
    assert_eq!(
        node(&runtime, MINUTE).style().background,
        theme.colors.surface_container_highest
    );
    assert_eq!(node(&runtime, MINUTE).style().corners, Some(shape::SMALL));
    assert_eq!(
        node(&runtime, FACE).style().background,
        theme.colors.surface_container_highest
    );
    assert_eq!(
        node(&runtime, "baseline-minute-dial-value")
            .style()
            .text_style(),
        typography::DISPLAY_LARGE,
    );
}

fn dial_point(face: Rect, count: f32, value: f32, radius: f32) -> (f32, f32) {
    let (sin, cos) = (std::f32::consts::TAU * value / count).sin_cos();
    (face.x + 128. + sin * radius, face.y + 128. - cos * radius)
}

fn tap(runtime: &mut Runtime<App>, (x, y): (f32, f32)) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    draw(runtime);
}

#[test]
fn baseline_uses_shared_polar_capture_and_keeps_the_draft_through_input_ime() {
    let mut runtime = open(App::new(390., 720., ClockFormat::TwentyFourHour));
    let face = visible(&runtime, FACE);
    let (x, y) = dial_point(face, 12., 3., 100.);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    let (x, y) = dial_point(face, 12., 6., 64.);
    runtime.dispatch(Event::PointerMove { x, y }).unwrap();
    assert_eq!(
        state(&runtime).preview(),
        Some(DialPreview {
            field: TimeField::Hour,
            value: 18
        })
    );
    assert_eq!(state(&runtime).input().raw(TimeField::Hour), "09");
    assert!(runtime.cancel_pointer().unwrap());
    draw(&mut runtime);
    assert_eq!(state(&runtime).preview(), None);

    tap(&mut runtime, (x, y));
    assert_eq!(state(&runtime).field(), TimeField::Minute);
    tap(&mut runtime, dial_point(face, 60., 59., 100.));
    assert_eq!(
        state(&runtime).input().validation().candidate(),
        TimeOfDay::new(18, 59)
    );
    runtime.activate("baseline-use-input").unwrap();
    draw(&mut runtime);
    assert_eq!(state(&runtime).mode(), DialMode::Input);
    assert_eq!(runtime.focused(), Some("baseline-hour-input"));

    runtime
        .dispatch(Event::Edit {
            key: EditKey::SelectAll,
            extend: false,
            by_word: false,
        })
        .unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "２３".into(),
            selection: None,
        }))
        .unwrap();
    assert!(!runtime
        .key(&KeyPress::new(Key::Enter, Modifiers::default()))
        .unwrap());
    assert_eq!(state(&runtime).input().raw(TimeField::Hour), "18");
    assert_eq!(runtime.app().commits, 0);
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("２３".into())))
        .unwrap();
    draw(&mut runtime);
    assert_eq!(
        state(&runtime).input().validation().candidate(),
        TimeOfDay::new(23, 59)
    );
    runtime.activate("baseline-confirm").unwrap();
    draw(&mut runtime);
    assert_eq!(runtime.app().saved, TimeOfDay::new(23, 59).unwrap());
    assert_eq!(runtime.app().commits, 1);
    assert_eq!(runtime.focused(), Some(OPEN));
}

#[test]
fn baseline_magnified_rtl_keeps_whole_face_then_falls_back_with_explicit_return() {
    let mut app = App::new(1120., 880., ClockFormat::TwelveHour);
    app.text_scale = 2.;
    app.direction = LayoutDirection::RightToLeft;
    app.locale = app
        .locale
        .with_digits(['０', '１', '２', '３', '４', '５', '６', '７', '８', '９'])
        .unwrap();
    assert_eq!(app.fit(), Some(TimeDialLayout::Horizontal));
    let mut runtime = open(app);
    assert_eq!(size(visible(&runtime, FACE)), (256., 256.));
    assert!(visible(&runtime, HOUR).x > visible(&runtime, MINUTE).x);
    let digits = visible(&runtime, "baseline-minute-dial-value");
    assert_eq!(digits, digits.intersect(visible(&runtime, MINUTE)));
    assert_eq!(
        node(&runtime, "baseline-minute-dial-value")
            .style()
            .text_style()
            .font_size,
        114.,
    );
    for id in [
        "baseline-am",
        "baseline-pm",
        "baseline-confirm",
        "baseline-cancel",
    ] {
        visible(&runtime, id);
    }

    runtime.render(320., 240., &mut Metrics).unwrap();
    assert_eq!(state(&runtime).mode(), DialMode::Input);
    assert!(runtime.dom().by_id(FACE).is_none());
    assert!(node(&runtime, "baseline-use-dial").is_disabled());
    visible(&runtime, "baseline-confirm");
    visible(&runtime, "baseline-cancel");
    runtime.render(1120., 880., &mut Metrics).unwrap();
    assert_eq!(state(&runtime).mode(), DialMode::Input);
    runtime.activate("baseline-use-dial").unwrap();
    draw(&mut runtime);
    assert_eq!(state(&runtime).mode(), DialMode::Dial);
    assert_eq!(size(visible(&runtime, FACE)), (256., 256.));
    assert_eq!(
        state(&runtime).input().validation().candidate(),
        TimeOfDay::new(9, 7)
    );
    assert_eq!(runtime.app().commits, 0);
}

#[test]
fn baseline_custom_period_labels_preserve_whole_targets_and_face() {
    let mut app = App::new(390., 720., ClockFormat::TwelveHour);
    app.locale = TimeInputLocale::new("ante meridiem", "post meridiem", ":");
    let mut runtime = open(app);
    for (id, label) in [
        ("baseline-am", "ante meridiem"),
        ("baseline-pm", "post meridiem"),
    ] {
        let target = visible(&runtime, id);
        assert!(target.width >= 48. && target.height >= 48.);
        assert_eq!(node(&runtime, id).name(), label);
    }
    assert_eq!(size(visible(&runtime, FACE)), (256., 256.));
    runtime.activate("baseline-pm").unwrap();
    draw(&mut runtime);
    assert_eq!(
        state(&runtime).input().validation().candidate(),
        TimeOfDay::new(21, 7)
    );
    assert_eq!(runtime.app().commits, 0);
}

#[test]
fn baseline_extra_long_confirm_keeps_whole_control_and_full_accessible_name() {
    const LONG: &str = concat!(
        "Save this appointment and keep its exact start time for everyone.",
        " Save this appointment and keep its exact start time for everyone.",
        " Save this appointment and keep its exact start time for everyone.",
        " Save this appointment and keep its exact start time for everyone.",
        " Save this appointment and keep its exact start time for everyone.",
    );
    let mut app = App::new(844., 390., ClockFormat::TwelveHour);
    app.labels.confirm = LONG;
    let mut runtime = open(app);
    let target = visible(&runtime, "baseline-confirm");
    assert!(target.width >= 48. && target.height >= 48.);
    assert_eq!(node(&runtime, "baseline-confirm").name(), LONG);
    assert_eq!(size(visible(&runtime, FACE)), (256., 256.));
    runtime.activate("baseline-confirm").unwrap();
    draw(&mut runtime);
    assert_eq!(runtime.app().commits, 1);
    assert_eq!(runtime.focused(), Some(OPEN));
}
