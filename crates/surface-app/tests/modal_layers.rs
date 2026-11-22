use surface::{Color, Insets, Length, Node, PaintOp, Rect, Role, Style, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};

const SCRIM: Color = Color(0, 0, 0, 128);
const BACKGROUND: Color = Color::rgb(246, 221, 123);
const PANEL: Color = Color::rgb(220, 219, 247);
const POPUP: Color = Color::rgb(174, 229, 208);
const INNER: Color = Color::rgb(233, 186, 205);
const PREVIEW: Color = Color::rgb(110, 102, 135);
const PREVIEW_CHILD: Color = Color::rgb(107, 87, 127);
const PREVIEW_FIXED: Color = Color::rgb(152, 120, 147);

#[derive(Clone, Copy, Debug, PartialEq)]
enum Msg {
    Open,
    Close,
    OpenInner,
    CloseInner,
    Background,
    Popup,
    Tick,
    BadRebuild,
}

struct App {
    open: bool,
    inner: bool,
    legacy: bool,
    invalid: bool,
    preview: bool,
    actions: Vec<Msg>,
}

impl Application for App {
    type Message = Msg;

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Open => self.open = true,
            Msg::Close => self.open = false,
            Msg::OpenInner => self.inner = true,
            Msg::CloseInner => self.inner = false,
            Msg::BadRebuild => self.invalid = true,
            Msg::Background | Msg::Popup | Msg::Tick => (),
        }
        self.actions.push(message);
    }

    fn view(&self) -> View<Msg> {
        let opener = button("open", Msg::Open, box_style(40., 30., PANEL));
        let mut children = vec![opener];
        if self.open {
            children.push(self.panel());
        }
        if self.preview {
            children.push(
                View::new(
                    Node::new(Role::Group, "Preview")
                        .id("preview")
                        .styled(box_style(122., 96., PREVIEW).fixed(at(8., 4.))),
                )
                .children([
                    button(
                        "preview-child",
                        Msg::Popup,
                        box_style(42., 25., PREVIEW_CHILD),
                    )
                    .accessible_activation(),
                    View::new(
                        Node::new(Role::Dialog, "Preview dialog")
                            .id("preview-fixed")
                            .styled(box_style(30., 24., PREVIEW_FIXED).fixed(at(16., 46.))),
                    )
                    .modal_overlay(Msg::CloseInner),
                ])
                .visual_overlay(),
            );
        }
        if self.invalid {
            children.push(button("open", Msg::Open, Style::default()));
        }
        // Keep the background last at the highest possible z-index.
        let mut style = box_style(320., 240., BACKGROUND).fixed(at(0., 0.));
        style.z_index = i32::MAX;
        children.push(button("background", Msg::Background, style));
        View::new(Node::new(Role::Window, "Layers")).children(children)
    }
}

impl App {
    fn panel(&self) -> View<Msg> {
        let fixed_popup = Node::new(Role::Group, "Fixed popup")
            .id("fixed")
            .styled(box_style(56., 44., POPUP).padding(6.).fixed(at(12., 30.)));
        let clipped_popup = Node::new(Role::Group, "Popup viewport")
            .id("absolute")
            .styled(
                box_style(40., 40., POPUP)
                    .clip(true)
                    .absolute(at(156., 25.)),
            );
        let mut children = vec![
            button("open-inner", Msg::OpenInner, box_style(56., 40., PANEL)),
            View::new(fixed_popup).children([button(
                "fixed-item",
                Msg::Popup,
                box_style(20., 20., POPUP),
            )]),
            View::new(clipped_popup).children([button(
                "absolute-item",
                Msg::Popup,
                box_style(44., 24., POPUP).absolute(at(18., 8.)),
            )]),
            View::new(
                Node::new(Role::Group, "Clear").styled(Style::column().fixed(Insets::all(0.))),
            ),
        ];
        if self.inner {
            let mut style = box_style(88., 60., INNER).fixed(at(118., 100.));
            style.z_index = i32::MIN;
            children.push(
                View::new(Node::new(Role::Dialog, "Inner").id("inner").styled(style))
                    .children([button(
                        "close-inner",
                        Msg::CloseInner,
                        box_style(60., 48., INNER),
                    )])
                    .modal_with_backdrop(scrim("inner-scrim"), Msg::CloseInner),
            );
        }
        let mut style = box_style(144., 112., PANEL).fixed(at(80., 60.));
        style.z_index = i32::MAX - 1;
        let panel = View::new(Node::new(Role::Dialog, "Outer").id("outer").styled(style))
            .children(children);
        if self.legacy {
            panel.modal(Msg::Close)
        } else {
            panel.modal_with_backdrop(scrim("outer-scrim"), Msg::Close)
        }
    }
}

fn scrim(id: &str) -> Node {
    Node::new(Role::Group, "")
        .id(id)
        .styled(Style::column().background(SCRIM).fixed(Insets::all(0.)))
}

fn box_style(width: f32, height: f32, color: Color) -> Style {
    Style::column()
        .width(Length::Px(width))
        .height(Length::Px(height))
        .background(color)
}

fn at(x: f32, y: f32) -> Insets {
    Insets {
        left: Length::Px(x),
        top: Length::Px(y),
        ..Insets::default()
    }
}

fn button(id: &str, message: Msg, style: Style) -> View<Msg> {
    View::new(Node::new(Role::Button, id).id(id).styled(style)).on_press(message)
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size, size * 1.5)
    }
}

fn opened(legacy: bool) -> Runtime<App> {
    let mut rt = Runtime::new(App {
        open: false,
        inner: false,
        legacy,
        invalid: false,
        preview: false,
        actions: Vec::new(),
    })
    .unwrap();
    rt.activate("open").unwrap();
    rt.render(320., 240., &mut Metrics).unwrap();
    rt
}

fn bounds(rt: &Runtime<App>, name: &str) -> Rect {
    let id = rt.dom().by_id(name).unwrap();
    rt.computed().unwrap().nodes[id.index()].bounds
}

fn click(rt: &mut Runtime<App>, x: f32, y: f32) {
    rt.dispatch(Event::PointerDown { x, y }).unwrap();
    rt.dispatch(Event::PointerUp { x, y }).unwrap();
}

#[test]
fn painted_and_clipped_popup_regions_are_inside_the_modal() {
    let mut rt = opened(false);
    let paint = rt.render(320., 240., &mut Metrics).unwrap();
    let quads: Vec<_> = paint
        .ops
        .iter()
        .filter_map(|op| match op {
            PaintOp::Quad { color, .. } => Some(*color),
            _ => None,
        })
        .collect();
    let position = |color| {
        quads
            .iter()
            .rposition(|candidate| *candidate == color)
            .unwrap()
    };
    assert!(position(BACKGROUND) < position(SCRIM));
    assert!(position(SCRIM) < position(PANEL));
    assert!(position(PANEL) < position(POPUP));

    let fixed = bounds(&rt, "fixed-item");
    assert!(fixed.x + fixed.width < bounds(&rt, "outer").x);
    click(&mut rt, fixed.x + 2., fixed.y + 2.);
    rt.render(320., 240., &mut Metrics).unwrap();
    let popup = bounds(&rt, "fixed");
    click(&mut rt, popup.x + popup.width - 2., popup.y + 2.);
    assert_eq!(rt.app().actions, [Msg::Open, Msg::Popup]);

    let absolute = bounds(&rt, "absolute-item");
    let clip = bounds(&rt, "absolute");
    assert!(absolute.x < clip.x + clip.width && absolute.x + absolute.width > clip.x + clip.width);
    click(&mut rt, absolute.x + 2., absolute.y + 2.);
    assert_eq!(rt.app().actions, [Msg::Open, Msg::Popup, Msg::Popup]);
    rt.render(320., 240., &mut Metrics).unwrap();
    click(&mut rt, clip.x + clip.width + 3., absolute.y + 2.);
    assert_eq!(
        rt.app().actions,
        [Msg::Open, Msg::Popup, Msg::Popup, Msg::Close]
    );
    assert_eq!(rt.focused(), Some("open"));
}

#[test]
fn legacy_scope_search_passes_over_a_higher_background_hit() {
    let mut rt = opened(true);
    let open = bounds(&rt, "open-inner");
    let (x, y) = (open.x + 2., open.y + 2.);
    assert_eq!(
        rt.computed().unwrap().hit_test(rt.dom(), x, y),
        rt.dom().by_id("background"),
    );
    click(&mut rt, x, y);
    assert_eq!(rt.app().actions, [Msg::Open, Msg::OpenInner]);
}

#[test]
fn nested_layer_wins_over_outer_fixed_layers_and_cancels_cross_scope_press() {
    let mut rt = opened(false);
    rt.dispatch(Event::PointerDown { x: 304., y: 210. })
        .unwrap();
    rt.activate("open-inner").unwrap();
    assert_eq!(rt.focused(), Some("close-inner"));
    rt.render(320., 240., &mut Metrics).unwrap();
    let order = &rt.computed().unwrap().paint_order;
    let position = |id| {
        order
            .iter()
            .position(|node| Some(*node) == rt.dom().by_id(id))
            .unwrap()
    };
    assert!(position("fixed-item") < position("inner-scrim"));
    assert!(position("inner-scrim") < position("inner"));
    rt.dispatch(Event::PointerUp { x: 304., y: 210. }).unwrap();
    assert_eq!(rt.app().actions, [Msg::Open, Msg::OpenInner]);
    click(&mut rt, 304., 210.);
    assert_eq!(
        rt.app().actions,
        [Msg::Open, Msg::OpenInner, Msg::CloseInner]
    );
    assert_eq!(rt.focused(), Some("open-inner"));
}

#[test]
fn normal_rebuild_preserves_pointer_but_failed_rebuild_cancels_it() {
    let mut rt = opened(false);
    let item = bounds(&rt, "fixed-item");
    let (x, y) = (item.x + 2., item.y + 2.);
    rt.dispatch(Event::PointerDown { x, y }).unwrap();
    rt.apply(Msg::Tick).unwrap();
    rt.render(320., 240., &mut Metrics).unwrap();
    rt.dispatch(Event::PointerUp { x, y }).unwrap();
    rt.render(320., 240., &mut Metrics).unwrap();
    rt.dispatch(Event::PointerDown { x, y }).unwrap();
    assert!(rt.apply(Msg::BadRebuild).is_err());
    rt.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(
        rt.app().actions,
        [Msg::Open, Msg::Tick, Msg::Popup, Msg::BadRebuild]
    );
}

#[test]
fn visual_overlay_raises_its_whole_subtree_without_exposing_a_modal_or_press() {
    let mut rt = Runtime::new(App {
        open: false,
        inner: false,
        legacy: false,
        invalid: false,
        preview: true,
        actions: Vec::new(),
    })
    .unwrap();
    let paint = rt.render(320., 240., &mut Metrics).unwrap();
    let position = |paint: &surface::DisplayList, wanted| {
        paint
            .ops
            .iter()
            .rposition(|op| matches!(op, PaintOp::Quad { color, .. } if *color == wanted))
            .expect("the layer was painted")
    };
    assert!(position(&paint, BACKGROUND) < position(&paint, PREVIEW));
    assert!(position(&paint, PREVIEW) < position(&paint, PREVIEW_CHILD));
    assert!(position(&paint, PREVIEW_CHILD) < position(&paint, PREVIEW_FIXED));
    let frame = rt.accessibility_frame().unwrap();
    assert!(frame.modal_root().is_none());
    for name in ["preview", "preview-child", "preview-fixed"] {
        let id = rt.dom().by_id(name).unwrap();
        assert!(frame.node(id).is_none());
        assert!(frame.activation_target(id).is_none());
    }
    rt.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(rt.focused(), Some("open"));
    let child = bounds(&rt, "preview-child");
    click(&mut rt, child.x + 2., child.y + 2.);
    assert_eq!(rt.app().actions, [Msg::Background]);

    rt.activate("open").unwrap();
    let paint = rt.render(320., 240., &mut Metrics).unwrap();
    assert!(position(&paint, BACKGROUND) < position(&paint, PREVIEW));
    assert!(position(&paint, PREVIEW_FIXED) < position(&paint, SCRIM));
    assert!(position(&paint, SCRIM) < position(&paint, PANEL));
    let modal = rt.dom().by_id("outer").unwrap();
    assert_eq!(rt.accessibility_frame().unwrap().modal_root(), Some(modal));
    assert_eq!(rt.focused(), Some("open-inner"));
}
