use surface::{Insets, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{AccessibilityScrollDirection, Application, Event, Runtime, View};
use surface_material::{
    place_tooltip,
    shape::{self, LayoutDirection},
    typography, Elevation, Theme, TooltipKind, TooltipOptions, TooltipSide,
};

const OWNER: &str = "tooltip-anchor";
const OUTSIDE: &str = "tooltip-outside";
const SURFACE: &str = "tooltip";
const ACTION: &str = "tooltip-action";
const TITLE: &str = "顯示設定";
const BODY: &str = "更多顯示設定可以幫助你清楚瞭解及管理工作區的內容。";

#[test]
fn placement_centers_and_flips_with_a_four_dp_gap() {
    let viewport = Rect::new(0., 0., 320., 240.);
    let base = Rect::new(140., 100., 40., 48.);
    let size = (120., 36.);
    let above = place_tooltip(base, size, viewport, TooltipSide::Above).unwrap();
    assert_eq!(above.side, TooltipSide::Above);
    assert_eq!(above.bounds, Rect::new(100., 60., 120., 36.));
    assert_eq!(above.caret_x, 60.);
    let below = place_tooltip(base, size, viewport, TooltipSide::Below).unwrap();
    assert_eq!(below.side, TooltipSide::Below);
    assert_eq!(below.bounds, Rect::new(100., 152., 120., 36.));
    let near_top = Rect::new(8., 8., 40., 48.);
    let flipped = place_tooltip(near_top, size, viewport, TooltipSide::Above).unwrap();
    assert_eq!(flipped.side, TooltipSide::Below);
    assert_eq!(flipped.bounds, Rect::new(0., 60., 120., 36.));
    assert_eq!(flipped.caret_x, 28.);
    let near_right_bottom = Rect::new(280., 188., 40., 48.);
    let flipped = place_tooltip(near_right_bottom, size, viewport, TooltipSide::Below).unwrap();
    assert_eq!(flipped.side, TooltipSide::Above);
    assert_eq!(flipped.bounds, Rect::new(200., 148., 120., 36.));
    assert_eq!(flipped.caret_x, 100.);
}

#[test]
fn placement_respects_the_available_rect_even_when_neither_side_fits() {
    let viewport = Rect::new(12., 20., 112., 72.);
    let anchor = Rect::new(-4., 40., 48., 20.);
    let placed = place_tooltip(anchor, (240., 200.), viewport, TooltipSide::Above).unwrap();
    assert_eq!(placed.bounds, viewport);
    assert_eq!(placed.side, TooltipSide::Below);
    assert_eq!(placed.caret_x, 16.);
    for invalid in [
        Rect::default(),
        Rect::new(200., 40., 48., 48.),
        Rect::new(f32::INFINITY, 0., 10., 10.),
        Rect::new(f32::MAX, 0., f32::MAX, 10.),
    ] {
        assert!(place_tooltip(invalid, (40., 24.), viewport, TooltipSide::Above).is_none());
    }
    for size in [(0., 40.), (40., -1.), (f32::NAN, 40.), (40., f32::INFINITY)] {
        assert!(place_tooltip(anchor, size, viewport, TooltipSide::Above).is_none());
    }
}

#[derive(Clone, Copy)]
enum Message {
    Owner,
    Measured(f32, f32),
    Remeasure,
    Act,
    Outside,
}

struct Demo {
    kind: TooltipKind,
    theme: Theme,
    title: Option<&'static str>,
    text: &'static str,
    anchor: Rect,
    viewport: Rect,
    direction: LayoutDirection,
    scale: f32,
    action: bool,
    visible: bool,
    measured: Option<(f32, f32)>,
    actions: u32,
    outside: u32,
}

impl Demo {
    fn new(kind: TooltipKind) -> Self {
        Self {
            kind,
            theme: Theme::light(),
            title: (kind == TooltipKind::Rich).then_some(TITLE),
            text: if kind == TooltipKind::Plain {
                "釘選"
            } else {
                BODY
            },
            anchor: Rect::new(132., 120., 48., 48.),
            viewport: Rect::new(4., 4., 312., 232.),
            direction: LayoutDirection::LeftToRight,
            scale: 1.,
            action: false,
            visible: true,
            measured: None,
            actions: 0,
            outside: 0,
        }
    }
}

impl Application for Demo {
    type Message = Message;

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::Owner => {}
            Message::Measured(width, height) => self.measured = Some((width, height)),
            Message::Remeasure => self.measured = None,
            Message::Act => {
                self.actions += 1;
                self.visible = false;
            }
            Message::Outside => self.outside += 1,
        }
    }

    fn view(&self) -> View<Self::Message> {
        let button = |id, name, style, message| {
            View::new(Node::new(Role::Button, name).id(id).styled(style))
                .on_press(message)
                .accessible_activation()
        };
        let tip = self.visible.then(|| {
            let options = TooltipOptions {
                id: SURFACE,
                text: self.text,
                title: self.title,
                anchor: self.anchor,
                viewport: self.viewport,
                preferred: TooltipSide::Above,
                direction: self.direction,
                caret: true,
                restore_focus_to: OWNER,
                measured_size: self.measured,
            };
            match self.kind {
                TooltipKind::Plain => self.theme.plain_tooltip(options),
                TooltipKind::Rich => self.theme.rich_tooltip(
                    options,
                    self.action.then(|| {
                        button(
                            ACTION,
                            "套用",
                            Style::row().width(Length::Px(48.)).height(Length::Px(48.)),
                            Message::Act,
                        )
                    }),
                ),
            }
        });
        View::new(Node::new(Role::Window, "Work")).children(
            [button(
                OWNER,
                "釘選",
                Style::row()
                    .width(Length::Px(self.anchor.width))
                    .height(Length::Px(self.anchor.height))
                    .absolute(Insets {
                        left: Length::Px(self.anchor.x),
                        top: Length::Px(self.anchor.y),
                        ..Insets::default()
                    }),
                Message::Owner,
            )]
            .into_iter()
            .chain(tip)
            .chain([button(
                OUTSIDE,
                "Refresh",
                Style::row().width(Length::Px(100.)).height(Length::Px(48.)),
                Message::Outside,
            )]),
        )
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let natural = text.chars().count() as f32 * size;
        let width = width.unwrap_or(natural).max(1.);
        (
            natural.min(width),
            (natural / width).ceil().max(1.) * size * 1.5,
        )
    }
}

fn render(runtime: &mut Runtime<Demo>, width: f32, height: f32) {
    runtime.render(width, height, &mut Metrics).unwrap();
}

fn bounds(runtime: &Runtime<Demo>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn node<'a>(runtime: &'a Runtime<Demo>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn measure(runtime: &mut Runtime<Demo>, width: f32, height: f32) -> Rect {
    render(runtime, width, height);
    let probe = bounds(runtime, SURFACE);
    assert!(probe.height > 0. && probe.width > 0.);
    assert!(probe.y + probe.height < 0.);
    runtime
        .apply(Message::Measured(probe.width, probe.height))
        .unwrap();
    render(runtime, width, height);
    bounds(runtime, SURFACE)
}

#[test]
fn plain_layout_has_material_tokens_without_moving_owner_focus() {
    for theme in [Theme::light(), Theme::dark()] {
        let mut demo = Demo::new(TooltipKind::Plain);
        demo.theme = theme;
        let mut runtime = Runtime::new(demo).unwrap();
        runtime.activate(OWNER).unwrap();
        let tip = measure(&mut runtime, 320., 240.);
        assert_eq!(tip, Rect::new(136., 92., 40., 24.));
        let surface = node(&runtime, SURFACE);
        assert_eq!(surface.role(), Role::Tooltip);
        assert_eq!(surface.name(), "釘選");
        assert_eq!(surface.style().background, theme.colors.inverse_surface);
        assert_eq!(surface.style().corner_radii(), shape::EXTRA_SMALL);
        assert_eq!(
            surface.box_shadow_layers(),
            theme.elevation(Elevation::Level0)
        );
        let text = node(&runtime, "tooltip-text");
        assert_eq!(text.style().foreground, theme.colors.inverse_on_surface);
        assert_eq!(text.style().text_style(), typography::BODY_SMALL);
        assert_eq!(bounds(&runtime, "tooltip-text").x, tip.x + 8.);
        assert_eq!(bounds(&runtime, "tooltip-text").y, tip.y + 4.);
        assert_eq!(
            bounds(&runtime, "tooltip-caret"),
            Rect::new(148., tip.y + tip.height, 16., 8.),
        );
        assert_eq!(runtime.focused(), Some(OWNER));
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some(OUTSIDE));
    }
}

#[test]
fn passive_rich_and_plain_widths_are_capped_and_flip_when_near_the_top() {
    for (kind, max_width) in [(TooltipKind::Plain, 200.), (TooltipKind::Rich, 320.)] {
        let mut demo = Demo::new(kind);
        demo.text = BODY;
        demo.viewport = Rect::new(4., 4., 632., 472.);
        demo.anchor = Rect::new(584., 6., 48., 48.);
        let mut runtime = Runtime::new(demo).unwrap();
        let tip = measure(&mut runtime, 640., 480.);
        assert_eq!(tip.width, max_width);
        assert_eq!(tip.y, 58.);
        assert_eq!(tip.x + tip.width, 636.);
        let surface = node(&runtime, SURFACE);
        assert_eq!(surface.role(), Role::Tooltip);
        assert!(surface.name().contains(BODY));
        assert_eq!(bounds(&runtime, "tooltip-caret").y + 8., tip.y);
    }
}

#[test]
fn rich_action_is_nonmodal_and_restores_its_anchor_after_dismissal() {
    let mut demo = Demo::new(TooltipKind::Rich);
    demo.action = true;
    let mut runtime = Runtime::new(demo).unwrap();
    runtime.activate(OWNER).unwrap();
    render(&mut runtime, 320., 240.);
    let probe_frame = runtime.accessibility_frame().unwrap();
    assert!(probe_frame
        .activation_target(runtime.dom().by_id(ACTION).unwrap())
        .is_none());
    let tip = measure(&mut runtime, 320., 240.);
    let surface = node(&runtime, SURFACE);
    assert_eq!(surface.name(), TITLE);
    assert_eq!(surface.role(), Role::Dialog);
    assert_eq!(surface.style().corner_radii(), shape::MEDIUM);
    assert_eq!(
        surface.style().background,
        Theme::light().colors.surface_container
    );
    assert_eq!(
        surface.box_shadow_layers(),
        Theme::light().elevation(Elevation::Level2)
    );
    assert_eq!(
        node(&runtime, "tooltip-title").style().text_style(),
        typography::TITLE_SMALL
    );
    assert_eq!(
        node(&runtime, "tooltip-text").style().text_style(),
        typography::BODY_MEDIUM
    );
    assert!(tip.width <= 320. && runtime.app().viewport.intersect(tip) == tip);
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame
        .activation_target(runtime.dom().by_id(ACTION).unwrap())
        .is_some());
    assert!(frame
        .activation_target(runtime.dom().by_id(OUTSIDE).unwrap())
        .is_some());
    runtime.activate(OUTSIDE).unwrap();
    assert_eq!(runtime.app().outside, 1);
    runtime.activate(OWNER).unwrap();
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(ACTION));
    runtime.apply(Message::Remeasure).unwrap();
    assert_eq!(runtime.focused(), Some(OWNER));
    measure(&mut runtime, 320., 240.);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(ACTION));
    runtime.dispatch(Event::Activate).unwrap();
    assert_eq!(runtime.app().actions, 1);
    assert_eq!(runtime.focused(), Some(OWNER));
    assert!(runtime.dom().by_id(SURFACE).is_none());
}

#[test]
fn narrow_rtl_large_text_stays_bounded_and_lets_the_rich_action_scroll_into_view() {
    let mut demo = Demo::new(TooltipKind::Rich);
    demo.theme = Theme::dark();
    demo.direction = LayoutDirection::RightToLeft;
    demo.scale = 2.;
    demo.viewport = Rect::new(8., 8., 112., 124.);
    demo.anchor = Rect::new(84., 12., 40., 40.);
    demo.action = true;
    let mut runtime = Runtime::new(demo).unwrap();
    let tip = measure(&mut runtime, 128., 140.);
    assert_eq!(tip, runtime.app().viewport);
    let text = bounds(&runtime, "tooltip-text");
    assert!(text.height > tip.height);
    assert!(text.x >= tip.x + 16. && text.x + text.width <= tip.x + tip.width - 16.);
    let content = runtime.dom().by_id("tooltip-content").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame
        .scroll_page_target(content, AccessibilityScrollDirection::Down)
        .is_some());
    runtime.activate(OWNER).unwrap();
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    render(&mut runtime, 128., 140.);
    assert_eq!(runtime.focused(), Some(ACTION));
    let action = bounds(&runtime, ACTION);
    assert_eq!(tip.intersect(action), action);
}
