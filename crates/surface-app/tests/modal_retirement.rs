use std::{cell::Cell, rc::Rc};

use surface::{
    Color, Corners, ExtendedDisplayList, ExtendedPaintOp, Insets, Length, Node, PaintOp,
    PaintTransform, RasterImage, Rect, Role, Style, TextMeasurer,
};
use surface_app::{Application, Event, Runtime, View};

const PAGE: Color = Color::rgb(230, 211, 187);
const VISUAL: Color = Color::rgb(183, 163, 195);
const OUTER_SCRIM: Color = Color(0, 0, 0, 64);
const OUTER: Color = Color::rgb(190, 220, 212);
const ROW: Color = Color::rgb(160, 217, 169);
const FIXED: Color = Color::rgb(107, 178, 240);
const INNER_SCRIM: Color = Color(9, 11, 13, 72);
const INNER: Color = Color::rgb(233, 167, 170);
const INNER_ACTION: Color = Color::rgb(245, 130, 188);
const FOCUS: Color = Color::rgb(33, 66, 99);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Phase {
    #[default]
    Absent,
    Active,
    Retiring,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Message {
    Open,
    OpenInner,
    Retire,
    RetireInner,
    RemoveInner,
    Page,
    Row,
    Inner,
}

#[derive(Default)]
struct App {
    outer: Phase,
    inner: Phase,
    inner_with_backdrop: bool,
    outer_alpha: Rc<Cell<f32>>,
    inner_alpha: Rc<Cell<f32>>,
    outer_transform: Rc<Cell<Option<PaintTransform>>>,
    inner_transform: Rc<Cell<Option<PaintTransform>>>,
    opacity_calls: Rc<Cell<usize>>,
    transform_calls: Rc<Cell<usize>>,
    messages: Vec<Message>,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.outer = Phase::Active,
            Message::OpenInner => self.inner = Phase::Active,
            Message::Retire => {
                self.outer = Phase::Retiring;
                if self.inner != Phase::Absent {
                    self.inner = Phase::Retiring;
                }
            }
            Message::RetireInner => self.inner = Phase::Retiring,
            Message::RemoveInner => self.inner = Phase::Absent,
            Message::Page | Message::Row | Message::Inner => {}
        }
        self.messages.push(message);
    }

    fn view(&self) -> View<Message> {
        let page = button(
            "page",
            Message::Page,
            block(320., 240., PAGE).fixed(at(0., 0.)),
        );
        let opener = button("open", Message::Open, block(36., 28., Color::TRANSPARENT));
        let visual = View::new(
            Node::new(Role::Group, "")
                .id("preview")
                .styled(block(320., 240., VISUAL).fixed(at(0., 0.))),
        )
        .visual_overlay();
        let outer = (self.outer != Phase::Absent).then(|| self.outer_panel());
        View::new(Node::new(Role::Window, "Window"))
            .children([page, opener, visual].into_iter().chain(outer))
    }
}

impl App {
    fn outer_panel(&self) -> View<Message> {
        let alpha = self.outer_alpha.clone();
        let transform = self.outer_transform.clone();
        let calls = self.opacity_calls.clone();
        let transform_calls = self.transform_calls.clone();
        let reopen = button(
            "open-inner",
            Message::OpenInner,
            block(42., 24., Color::TRANSPARENT).fixed(at(66., 45.)),
        );
        let row = View::new(
            Node::new(Role::Button, "Row")
                .id("row")
                .focus_color(FOCUS)
                .styled(
                    block(118., 62., ROW)
                        .corners(Corners::all(8.))
                        .state_layer(FOCUS)
                        .fixed(at(90., 82.)),
                )
                .press_ripple(Some("art".into())),
        )
        .on_press(Message::Row)
        .accessible_activation()
        .children((self.inner != Phase::Absent).then(|| self.inner_panel()));
        let fixed = View::new(
            Node::new(Role::Group, "Outer fixed")
                .id("outer-fixed")
                .styled(block(20., 20., FIXED).fixed(at(75., 120.))),
        );
        let panel = View::new(
            Node::new(Role::Dialog, "Outer")
                .id("outer")
                .styled(block(190., 150., OUTER).fixed(at(60., 40.))),
        )
        .children([reopen, row, fixed])
        .modal_initial_focus("open-inner")
        .paint_modal_panel_opacity(move || {
            calls.set(calls.get() + 1);
            alpha.get()
        })
        .paint_modal_panel_transform(move || {
            transform_calls.set(transform_calls.get() + 1);
            transform.get()
        });

        let scrim = backdrop("outer-scrim", OUTER_SCRIM);
        match self.outer {
            Phase::Active => panel.modal_with_backdrop(scrim, Message::Retire),
            Phase::Retiring => panel.retiring_modal_with_backdrop(scrim),
            Phase::Absent => unreachable!(),
        }
    }

    fn inner_panel(&self) -> View<Message> {
        let alpha = self.inner_alpha.clone();
        let transform = self.inner_transform.clone();
        let image = RasterImage::new(1, 1, [56, 211, 153, 255]).unwrap();
        let art = View::new(
            Node::new(Role::Group, "")
                .id("art")
                .styled(block(12., 12., Color::TRANSPARENT).fixed(at(121., 106.)))
                .raster_image(image),
        );
        let target = button(
            "inner-action",
            Message::Inner,
            block(36., 30., INNER_ACTION).fixed(at(105., 90.)),
        );
        let panel = View::new(
            Node::new(Role::Dialog, "Inner")
                .id("inner")
                .styled(block(112., 70., INNER).fixed(at(95., 86.))),
        )
        .children([target, art])
        .modal_initial_focus("inner-action")
        .paint_modal_panel_opacity(move || alpha.get())
        .paint_modal_panel_transform(move || transform.get());
        match (self.inner, self.inner_with_backdrop) {
            (Phase::Active, true) => panel
                .modal_with_backdrop(backdrop("inner-scrim", INNER_SCRIM), Message::RetireInner),
            (Phase::Retiring, true) => {
                panel.retiring_modal_with_backdrop(backdrop("inner-scrim", INNER_SCRIM))
            }
            (Phase::Active, false) => panel.modal_overlay(Message::RetireInner),
            (Phase::Retiring, false) => panel.retiring_modal_overlay(),
            (Phase::Absent, _) => unreachable!(),
        }
    }
}

fn button(id: &str, action: Message, style: Style) -> View<Message> {
    View::new(Node::new(Role::Button, id).id(id).styled(style))
        .on_press(action)
        .accessible_activation()
}

fn backdrop(id: &str, color: Color) -> Node {
    Node::new(Role::Group, "")
        .id(id)
        .styled(Style::column().fixed(Insets::all(0.)).background(color))
}

fn block(w: f32, h: f32, color: Color) -> Style {
    Style::column()
        .width(Length::Px(w))
        .height(Length::Px(h))
        .background(color)
}

fn at(x: f32, y: f32) -> Insets {
    Insets {
        left: Length::Px(x),
        top: Length::Px(y),
        ..Insets::default()
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn fixture(inner_with_backdrop: bool) -> Runtime<App> {
    Runtime::new(App {
        inner_with_backdrop,
        outer_alpha: Rc::new(Cell::new(1.)),
        inner_alpha: Rc::new(Cell::new(1.)),
        ..App::default()
    })
    .unwrap()
}

fn draw(rt: &mut Runtime<App>) -> ExtendedDisplayList {
    rt.render_extended(320., 240., &mut Metrics).unwrap()
}

fn layers(rt: &Runtime<App>, ids: &[&str]) -> Vec<String> {
    rt.computed()
        .unwrap()
        .paint_order
        .iter()
        .filter_map(|id| rt.dom().get(*id).unwrap().stable_id())
        .filter(|id| ids.contains(id))
        .map(str::to_owned)
        .collect()
}

fn scope(rt: &Runtime<App>, expected: Option<&str>) {
    assert_eq!(
        rt.accessibility_frame().unwrap().modal_root(),
        expected.map(|id| rt.dom().by_id(id).unwrap())
    );
}

fn no_ax(rt: &Runtime<App>, ids: &[&str]) {
    let ax = rt.accessibility_frame().unwrap();
    for stable_id in ids {
        let id = rt.dom().by_id(stable_id).unwrap();
        assert!(ax.node(id).is_none(), "AX included {stable_id}");
        assert!(ax.focus_target(id).is_none(), "AX could focus {stable_id}");
        assert!(
            ax.activation_target(id).is_none(),
            "AX could activate {stable_id}"
        );
    }
}

fn down(rt: &mut Runtime<App>) {
    rt.dispatch(Event::PointerDown { x: 110., y: 96. }).unwrap();
}

fn up(rt: &mut Runtime<App>) {
    rt.dispatch(Event::PointerUp { x: 110., y: 96. }).unwrap();
}

#[test]
fn nested_retirement_keeps_the_paint_slot_and_gives_input_to_the_lower_scope() {
    let mut rt = fixture(true);
    rt.activate("open").unwrap();
    rt.activate("open-inner").unwrap();
    draw(&mut rt);
    scope(&rt, Some("inner"));
    assert_eq!(rt.focused(), Some("inner-action"));
    let ids = [
        "page",
        "preview",
        "outer-scrim",
        "outer",
        "row",
        "outer-fixed",
        "inner-scrim",
        "inner",
        "inner-action",
        "art",
    ];
    assert_eq!(layers(&rt, &ids), ids);

    // Release started before the exit is not delivered to either surface.
    down(&mut rt);
    rt.apply(Message::RetireInner).unwrap();
    assert_eq!(rt.focused(), Some("open-inner"));
    draw(&mut rt);
    scope(&rt, Some("outer"));
    assert_eq!(layers(&rt, &ids), ids);
    no_ax(&rt, &["inner-scrim", "inner", "inner-action", "art"]);
    assert!(rt.activate("inner-action").is_err());
    up(&mut rt);
    assert_eq!(
        rt.app().messages,
        [Message::Open, Message::OpenInner, Message::RetireInner]
    );
    down(&mut rt);
    up(&mut rt);
    assert_eq!(rt.app().messages.last(), Some(&Message::Row));

    rt.apply(Message::Retire).unwrap();
    assert_eq!(rt.focused(), Some("open"));
    draw(&mut rt);
    scope(&rt, None);
    assert_eq!(layers(&rt, &ids), ids);
    no_ax(
        &rt,
        &[
            "outer-scrim",
            "outer",
            "row",
            "inner-scrim",
            "inner",
            "inner-action",
        ],
    );
    down(&mut rt);
    up(&mut rt);
    assert_eq!(rt.app().messages.last(), Some(&Message::Page));
}

#[test]
fn reversing_a_retained_panel_reenters_its_scope_without_reusing_a_press() {
    let mut rt = fixture(false);
    rt.activate("open").unwrap();
    rt.activate("open-inner").unwrap();
    draw(&mut rt);
    let ids = ["outer", "row", "inner", "inner-action"];
    let active_order = layers(&rt, &ids);
    down(&mut rt);
    rt.apply(Message::RetireInner).unwrap();
    draw(&mut rt);
    no_ax(&rt, &["inner", "inner-action"]);
    assert_eq!(layers(&rt, &ids), active_order);
    rt.activate("open-inner").unwrap();
    draw(&mut rt);
    scope(&rt, Some("inner"));
    assert_eq!(rt.focused(), Some("inner-action"));
    assert_eq!(layers(&rt, &ids), active_order);
    up(&mut rt);
    assert!(!rt.app().messages.contains(&Message::Inner));
    down(&mut rt);
    up(&mut rt);
    assert_eq!(rt.app().messages.last(), Some(&Message::Inner));
    rt.apply(Message::RemoveInner).unwrap();
    draw(&mut rt);
    assert!(rt.dom().by_id("inner").is_none());
    scope(&rt, Some("outer"));
}

#[derive(Debug, PartialEq)]
enum Mark {
    Begin(f32),
    Transform(PaintTransform),
    End,
    Fill(Color),
    Wave,
    Focus,
    Art,
}

fn sequence(display: &ExtendedDisplayList) -> Vec<Mark> {
    display
        .ops()
        .iter()
        .filter_map(|op| match op {
            ExtendedPaintOp::BeginOpacityGroup { opacity } => Some(Mark::Begin(*opacity)),
            ExtendedPaintOp::BeginTransformGroup { transform } => Some(Mark::Transform(*transform)),
            ExtendedPaintOp::EndOpacityGroup => Some(Mark::End),
            ExtendedPaintOp::FillPath(_) => Some(Mark::Wave),
            ExtendedPaintOp::Raster(_) => Some(Mark::Art),
            ExtendedPaintOp::Legacy(PaintOp::Stroke { color, .. }) if *color == FOCUS => {
                Some(Mark::Focus)
            }
            ExtendedPaintOp::Legacy(PaintOp::Quad { color, .. })
                if [
                    PAGE,
                    VISUAL,
                    OUTER_SCRIM,
                    OUTER,
                    ROW,
                    FIXED,
                    INNER_SCRIM,
                    INNER,
                    INNER_ACTION,
                ]
                .contains(color) =>
            {
                Some(Mark::Fill(*color))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn opacity_ranges_cover_panel_and_fixed_children_without_dimmed_scrims_or_nested_groups() {
    let mut rt = fixture(true);
    rt.activate("open").unwrap();
    rt.activate("open-inner").unwrap();
    rt.app().outer_alpha.set(0.625);
    rt.app().inner_alpha.set(0.25);
    let list = draw(&mut rt);
    use Mark::{Art, Begin, End, Fill};
    assert_eq!(
        sequence(&list),
        [
            Fill(PAGE),
            Fill(VISUAL),
            Fill(OUTER_SCRIM),
            Begin(0.625),
            Fill(OUTER),
            Fill(ROW),
            Fill(FIXED),
            End,
            Fill(INNER_SCRIM),
            Begin(0.25),
            Fill(INNER),
            Fill(INNER_ACTION),
            Art,
            End,
        ]
    );
    let calls = rt.app().opacity_calls.get();
    let revision = rt.accessibility_frame().unwrap().generation();
    rt.app().outer_alpha.set(0.75);
    assert!(sequence(&draw(&mut rt)).contains(&Begin(0.75)));
    assert_eq!(rt.accessibility_frame().unwrap().generation(), revision);
    assert_eq!(rt.app().opacity_calls.get(), calls + 1);

    let legacy = rt.render(320., 240., &mut Metrics).unwrap();
    assert_eq!(rt.app().opacity_calls.get(), calls + 1);
    assert!(legacy
        .ops
        .iter()
        .any(|op| matches!(op, PaintOp::Quad { color, .. } if *color == OUTER)));
    rt.app().outer_alpha.set(f32::NAN);
    rt.app().inner_alpha.set(1.5);
    assert!(!sequence(&draw(&mut rt))
        .iter()
        .any(|mark| matches!(mark, Begin(_) | End)));
    rt.app().outer_alpha.set(-0.25);
    assert!(sequence(&draw(&mut rt)).contains(&Begin(0.)));
}

#[test]
fn a_raster_in_a_separate_modal_slice_cannot_defer_the_owners_wave_or_focus_past_opacity() {
    let mut rt = fixture(true);
    rt.activate("open").unwrap();
    rt.apply(Message::RetireInner).unwrap();
    rt.set_time(10.);
    draw(&mut rt);
    down(&mut rt);
    draw(&mut rt);
    rt.set_time(100.);
    let plain = sequence(&draw(&mut rt));
    let position = |marks: &[Mark], mark| marks.iter().position(|m| *m == mark).unwrap();
    assert!(position(&plain, Mark::Art) < position(&plain, Mark::Wave));
    assert!(position(&plain, Mark::Wave) < position(&plain, Mark::Focus));

    rt.app().outer_alpha.set(0.5);
    rt.app().inner_alpha.set(0.25);
    let grouped = sequence(&draw(&mut rt));
    let wave = position(&grouped, Mark::Wave);
    let focus = position(&grouped, Mark::Focus);
    let end = position(&grouped, Mark::End);
    assert!(position(&grouped, Mark::Begin(0.5)) < wave);
    assert!(wave < focus && focus < end);
    assert!(end < position(&grouped, Mark::Fill(INNER_SCRIM)));
    assert!(position(&grouped, Mark::Begin(0.25)) < position(&grouped, Mark::Art));
}

#[test]
fn absent_invalid_and_neutral_transforms_use_ordinary_paint_or_independent_opacity() {
    let mut rt = fixture(true);
    rt.activate("open").unwrap();
    let steady = draw(&mut rt);
    let legacy = rt.render(320., 240., &mut Metrics).unwrap();
    let viewport = Rect::new(0., 0., 320., 240.);
    let identity = PaintTransform {
        source: viewport,
        destination: viewport,
        clip: viewport,
        radius: 0.,
        opacity: 1.,
    };
    rt.app().outer_transform.set(Some(identity));
    assert_eq!(draw(&mut rt), steady);

    rt.app().outer_transform.set(Some(PaintTransform {
        opacity: 0.5,
        ..identity
    }));
    rt.app().outer_alpha.set(0.5);
    let translucent = sequence(&draw(&mut rt));
    assert!(translucent.contains(&Mark::Begin(0.25)));
    assert!(!translucent
        .iter()
        .any(|mark| matches!(mark, Mark::Transform(_))));

    rt.app().outer_transform.set(Some(PaintTransform {
        radius: f32::NAN,
        ..identity
    }));
    let invalid = sequence(&draw(&mut rt));
    assert!(invalid.contains(&Mark::Begin(0.5)));
    assert!(!invalid
        .iter()
        .any(|mark| matches!(mark, Mark::Transform(_))));

    let crop = Rect::new(50., 40., 210., 160.);
    let cropping_identity = PaintTransform {
        source: crop,
        destination: crop,
        ..identity
    };
    rt.app().outer_alpha.set(1.);
    rt.app().outer_transform.set(Some(cropping_identity));
    assert!(sequence(&draw(&mut rt)).contains(&Mark::Transform(cropping_identity)));

    let before_legacy = rt.app().transform_calls.get();
    assert_eq!(rt.render(320., 240., &mut Metrics).unwrap(), legacy);
    assert_eq!(rt.app().transform_calls.get(), before_legacy);
    rt.app().outer_transform.set(None);
    assert_eq!(draw(&mut rt), steady);
}

#[test]
fn separate_nested_transforms_keep_fixed_content_and_scrims_in_order() {
    let mut rt = fixture(true);
    rt.activate("open").unwrap();
    rt.activate("open-inner").unwrap();
    draw(&mut rt);
    let geometry = rt.computed().unwrap().nodes.clone();
    let revision = rt.accessibility_frame().unwrap().generation();

    let outer = PaintTransform {
        source: Rect::new(60., 40., 190., 150.),
        destination: Rect::new(18., 22., 100., 90.),
        clip: Rect::new(22., 25., 94., 80.),
        radius: 12.,
        opacity: 0.8,
    };
    let inner = PaintTransform {
        source: Rect::new(95., 86., 112., 70.),
        destination: Rect::new(130., 120., 105., 54.),
        clip: Rect::new(132., 122., 100., 50.),
        radius: 6.,
        opacity: 0.75,
    };
    rt.app().outer_transform.set(Some(outer));
    rt.app().inner_transform.set(Some(inner));
    rt.app().outer_alpha.set(0.5);
    rt.app().inner_alpha.set(0.5);

    use Mark::{Art, End, Fill, Transform};
    let transformed = draw(&mut rt);
    assert_eq!(
        sequence(&transformed),
        [
            Fill(PAGE),
            Fill(VISUAL),
            Fill(OUTER_SCRIM),
            Transform(PaintTransform {
                opacity: 0.4,
                ..outer
            }),
            Fill(OUTER),
            Fill(ROW),
            Fill(FIXED),
            End,
            Fill(INNER_SCRIM),
            Transform(PaintTransform {
                opacity: 0.375,
                ..inner
            }),
            Fill(INNER),
            Fill(INNER_ACTION),
            Art,
            End,
        ]
    );
    assert_eq!(rt.computed().unwrap().nodes, geometry);
    assert_eq!(rt.accessibility_frame().unwrap().generation(), revision);
    assert_eq!(rt.focused(), Some("inner-action"));
    scope(&rt, Some("inner"));

    // Input uses the authored layout while active. A press from that scope
    // is rejected once the retained transform becomes paint-only.
    down(&mut rt);
    rt.apply(Message::RetireInner).unwrap();
    let retiring = sequence(&draw(&mut rt));
    assert!(retiring.contains(&Transform(PaintTransform {
        opacity: 0.375,
        ..inner
    })));
    no_ax(&rt, &["inner-scrim", "inner", "inner-action", "art"]);
    scope(&rt, Some("outer"));
    assert_eq!(rt.focused(), Some("open-inner"));
    up(&mut rt);
    assert_eq!(rt.app().messages.last(), Some(&Message::RetireInner));
    down(&mut rt);
    up(&mut rt);
    assert_eq!(rt.app().messages.last(), Some(&Message::Row));
}
