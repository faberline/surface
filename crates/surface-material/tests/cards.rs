use surface::{
    Color, DisplayList, Length, MicroDom, Node, NodeId, PaintOp, Rect, Role, Style, TextMeasurer,
};
use surface_app::{AccessibilityActivationResult, Application, Event, Runtime, View};
use surface_material::{
    shape, CardAction, CardContent, CardInteraction, CardKind, Elevation, MaterialCardOptions,
    Theme,
};

type Build = Box<dyn Fn(&Theme) -> View<()>>;
struct App(bool, Build);
impl Application for App {
    type Message = ();
    fn update(&mut self, _: ()) {}
    fn view(&self) -> View<()> {
        let theme = if self.0 {
            Theme::dark()
        } else {
            Theme::light()
        };
        self.1(&theme)
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

fn runtime(dark: bool, build: impl Fn(&Theme) -> View<()> + 'static) -> Runtime<App> {
    Runtime::new(App(dark, Box::new(build))).unwrap()
}

fn theme_for(dark: bool) -> Theme {
    if dark {
        Theme::dark()
    } else {
        Theme::light()
    }
}

fn by_id(dom: &MicroDom, id: &str) -> NodeId {
    dom.by_id(id).unwrap_or_else(|| panic!("no node {id}"))
}

const KINDS: [CardKind; 3] = [CardKind::Elevated, CardKind::Filled, CardKind::Outlined];

#[test]
fn kinds_use_the_m3_container_tokens() {
    for dark in [false, true] {
        let theme = theme_for(dark);
        for kind in KINDS {
            let mut r = runtime(dark, move |t| t.page([t.card_with("c", "Card", kind, [])]));
            r.render(400., 300., &mut Metrics).unwrap();
            let style = r.dom().get(by_id(r.dom(), "c")).unwrap().style().clone();
            assert_eq!(style.corners, Some(shape::MEDIUM), "{kind:?} dark={dark}");
            match kind {
                CardKind::Elevated => {
                    assert_eq!(style.background, theme.colors.surface_container_low);
                    assert_eq!(style.border_width, 0.);
                }
                CardKind::Filled => {
                    assert_eq!(style.background, theme.colors.surface_container_highest);
                    assert_eq!(style.border_width, 0.);
                }
                CardKind::Outlined => {
                    assert_eq!(style.background, theme.colors.surface);
                    assert_eq!(style.border_width, 1.);
                    assert_eq!(style.border_color, theme.colors.outline_variant);
                }
            }
        }
    }
}

#[test]
fn kinds_are_different_surfaces() {
    // Negative: if the kinds shared one color, this fails.
    let theme = Theme::light();
    let mut backgrounds: Vec<Color> = Vec::new();
    for kind in KINDS {
        let mut r = runtime(false, move |t| t.page([t.card_with("c", "Card", kind, [])]));
        r.render(400., 300., &mut Metrics).unwrap();
        backgrounds.push(r.dom().get(by_id(r.dom(), "c")).unwrap().style().background);
    }
    assert_ne!(backgrounds[0], backgrounds[1], "elevated vs filled");
    assert_ne!(backgrounds[1], backgrounds[2], "filled vs outlined");
    assert_ne!(backgrounds[0], backgrounds[2], "elevated vs outlined");
    assert_ne!(
        backgrounds[1], theme.container,
        "filled must not reuse the old card color"
    );
}

#[test]
fn card_with_is_a_group_named_by_the_caller() {
    let mut r = runtime(false, |t| {
        t.page([t.card_with("c", "Card", CardKind::Filled, [])])
    });
    r.render(400., 300., &mut Metrics).unwrap();
    let node = r.dom().get(by_id(r.dom(), "c")).unwrap();
    assert_eq!(node.role(), Role::Group);
    assert_eq!(node.name(), "Card");

    let mut r = runtime(false, |t| {
        t.page([t.card_with("c", "Other", CardKind::Filled, [])])
    });
    r.render(400., 300., &mut Metrics).unwrap();
    assert_eq!(r.dom().get(by_id(r.dom(), "c")).unwrap().name(), "Other");
}

struct Counter {
    count: usize,
    disabled: bool,
}
impl Application for Counter {
    type Message = ();
    fn update(&mut self, _: ()) {
        self.count += 1;
    }
    fn view(&self) -> View<()> {
        let theme = Theme::light();
        theme.page([theme.action_card(
            "c",
            "Open",
            CardKind::Filled,
            self.disabled,
            (),
            [theme.text("t", "Open")],
        )])
    }
}

fn counter(disabled: bool) -> Runtime<Counter> {
    Runtime::new(Counter { count: 0, disabled }).unwrap()
}

#[test]
fn action_card_is_a_button_and_sends_the_message() {
    let mut r = counter(false);
    r.render(400., 300., &mut Metrics).unwrap();
    assert_eq!(
        r.dom().get(by_id(r.dom(), "c")).unwrap().role(),
        Role::Button
    );

    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), Some("c"));
    r.dispatch(Event::Activate).unwrap();
    assert_eq!(r.app().count, 1);

    r.render(400., 300., &mut Metrics).unwrap();
    let b = r.computed().unwrap().nodes[by_id(r.dom(), "c").index()].bounds;
    r.dispatch(Event::PointerDown {
        x: b.x + 4.,
        y: b.y + 4.,
    })
    .unwrap();
    r.dispatch(Event::PointerUp {
        x: b.x + 4.,
        y: b.y + 4.,
    })
    .unwrap();
    assert_eq!(r.app().count, 2);
}

#[test]
fn disabled_action_card_sends_nothing() {
    let theme = Theme::light();
    let mut r = counter(true);
    r.render(400., 300., &mut Metrics).unwrap();
    let node = r.dom().get(by_id(r.dom(), "c")).unwrap();
    assert!(node.is_disabled());
    assert_eq!(node.style().background, theme.disabled);
    assert_eq!(node.style().foreground, theme.outline);

    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), None);
    // No node is focused, so Activate may return Err; the message count is the proof.
    let _ = r.dispatch(Event::Activate);

    r.render(400., 300., &mut Metrics).unwrap();
    let b = r.computed().unwrap().nodes[by_id(r.dom(), "c").index()].bounds;
    let _ = r.dispatch(Event::PointerDown {
        x: b.x + 4.,
        y: b.y + 4.,
    });
    let _ = r.dispatch(Event::PointerUp {
        x: b.x + 4.,
        y: b.y + 4.,
    });
    assert_eq!(r.app().count, 0);
}

#[test]
fn existing_cards_are_unchanged() {
    let theme = Theme::light();
    let mut r = runtime(false, |t| {
        t.page([t.card("a", []), t.outlined_card("b", [])])
    });
    r.render(400., 300., &mut Metrics).unwrap();

    let a = r.dom().get(by_id(r.dom(), "a")).unwrap().style().clone();
    assert_eq!(a.background, theme.container);
    assert_eq!(a.corners, Some(shape::MEDIUM));

    let b = r.dom().get(by_id(r.dom(), "b")).unwrap().style().clone();
    assert_eq!(b.border_width, 1.);
    assert_eq!(b.border_color, theme.outline);
}

fn material_options(kind: CardKind) -> MaterialCardOptions<'static> {
    MaterialCardOptions {
        id: "material",
        name: "卡片項目",
        kind,
        direction: shape::LayoutDirection::LeftToRight,
        disabled: false,
        interaction: CardInteraction::Rest,
        restore_focus_to: Some("back"),
    }
}

const TITLE: &str = "可換行的卡片標題和中文顯示大小測試";

fn content<M>() -> CardContent<'static, M> {
    CardContent {
        title: Some(TITLE),
        subtitle: Some("工具組"),
        supporting: Some("每個卡片有清楚的內容"),
        media: Some(View::new(
            Node::new(Role::Group, "Sample media")
                .id("sample-media")
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Px(56.))
                        .background(Color::rgb(49, 100, 178)),
                ),
        )),
        body: None,
    }
}

fn node<'a>(dom: &'a MicroDom, id: &str) -> &'a Node {
    dom.get(by_id(dom, id)).unwrap()
}

fn bounds<A: Application>(runtime: &Runtime<A>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[by_id(runtime.dom(), id).index()].bounds
}

#[test]
fn material_card_states_use_distinct_levels_and_layers_in_both_palettes() {
    use CardInteraction::{Dragged, Focused, Hovered, Pressed, Rest};
    use Elevation::{Level0, Level1, Level2, Level3, Level4};
    for dark in [false, true] {
        let theme = theme_for(dark);
        for (kind, background, levels) in [
            (
                CardKind::Elevated,
                theme.colors.surface_container_low,
                [Level1, Level2, Level1, Level1, Level4],
            ),
            (
                CardKind::Filled,
                theme.colors.surface_container_highest,
                [Level0, Level1, Level0, Level0, Level3],
            ),
            (
                CardKind::Outlined,
                theme.colors.surface,
                [Level0, Level1, Level0, Level0, Level3],
            ),
        ] {
            for ((interaction, alpha), level) in [
                (Rest, 0),
                (Hovered, 20),
                (Focused, 31),
                (Pressed, 31),
                (Dragged, 41),
            ]
            .into_iter()
            .zip(levels)
            {
                let mut r = runtime(dark, move |t| {
                    t.page([t.material_action_card(
                        MaterialCardOptions {
                            interaction,
                            ..material_options(kind)
                        },
                        content(),
                        CardAction::Press(()),
                    )])
                });
                r.render(400., 550., &mut Metrics).unwrap();
                let card = node(r.dom(), "material");
                let layer = node(r.dom(), "material-state");
                assert_eq!(card.style().corners, Some(shape::MEDIUM));
                assert_eq!(card.style().background, background);
                assert_eq!(card.box_shadow_layers(), theme.elevation(level));
                assert_eq!(layer.style().background.3, alpha);
                if interaction == Focused {
                    let indicator = if kind == CardKind::Outlined {
                        theme.colors.on_surface
                    } else {
                        theme.colors.secondary
                    };
                    assert_eq!(layer.style().border_width, 2.);
                    assert_eq!(layer.style().border_color, indicator);
                } else if kind == CardKind::Outlined {
                    assert_eq!(layer.style().border_width, 1.);
                    assert_eq!(layer.style().border_color, theme.colors.outline_variant);
                }
            }

            let mut r = runtime(dark, move |t| {
                t.page([t.material_action_card(
                    MaterialCardOptions {
                        disabled: true,
                        interaction: Dragged,
                        ..material_options(kind)
                    },
                    content(),
                    CardAction::Press(()),
                )])
            });
            r.render(400., 550., &mut Metrics).unwrap();
            let card = node(r.dom(), "material");
            let layer = node(r.dom(), "material-state");
            assert_eq!(card.box_shadow_layers(), theme.elevation(levels[0]));
            assert_eq!(layer.style().background.3, 0);
            assert_eq!(node(r.dom(), "material-title").style().foreground.3, 97);
            match kind {
                CardKind::Elevated => assert_eq!(card.style().background, theme.colors.surface),
                CardKind::Filled => assert_ne!(card.style().background, background),
                CardKind::Outlined => {
                    let Color(r, g, b, _) = theme.colors.outline;
                    assert_eq!(layer.style().border_color, Color(r, g, b, 31));
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum CardMessage {
    Select,
    ActivateChild,
    Disable,
    Back,
}

struct CardDemo {
    actionable: bool,
    disabled: bool,
    selected: bool,
    child_hits: usize,
}

impl Application for CardDemo {
    type Message = CardMessage;

    fn update(&mut self, message: CardMessage) {
        match message {
            CardMessage::Select => self.selected = !self.selected,
            CardMessage::ActivateChild => self.child_hits += 1,
            CardMessage::Disable => self.disabled = true,
            CardMessage::Back => {}
        }
    }

    fn text_scale(&self) -> f32 {
        2.
    }

    fn view(&self) -> View<CardMessage> {
        let theme = Theme::light();
        let options = MaterialCardOptions {
            direction: shape::LayoutDirection::RightToLeft,
            disabled: self.disabled,
            ..material_options(CardKind::Elevated)
        };
        let action = |id| {
            View::new(
                Node::new(Role::Button, id).id(id).styled(
                    Style::column()
                        .width(Length::Px(112.))
                        .height(Length::Px(48.)),
                ),
            )
            .on_press(CardMessage::ActivateChild)
            .accessible_activation()
        };
        let card = if self.actionable {
            theme.material_action_card(
                options,
                CardContent {
                    body: Some(action("inner")),
                    ..content()
                },
                CardAction::Toggle {
                    checked: self.selected,
                    on_toggle: CardMessage::Select,
                },
            )
        } else {
            theme.material_card(options, content(), ["first", "second", "third"].map(action))
        };
        View::new(
            Node::new(Role::Window, "Card demo").styled(Style::column().padding(16.).gap(12.)),
        )
        .children([theme.button("back", "Back", CardMessage::Back), card])
    }
}

struct WrappingMetrics;

impl TextMeasurer for WrappingMetrics {
    fn measure(&mut self, text: &str, size: f32, limit: Option<f32>) -> (f32, f32) {
        let width: f32 = text
            .chars()
            .map(|c| size * if c.is_ascii() { 0.5 } else { 1. })
            .sum();
        let available = limit.unwrap_or(width).max(1.);
        (
            width.min(available),
            size * 1.5 * (width / available).ceil().max(1.),
        )
    }
}

fn demo(actionable: bool) -> Runtime<CardDemo> {
    Runtime::new(CardDemo {
        actionable,
        disabled: false,
        selected: false,
        child_hits: 0,
    })
    .unwrap()
}

#[test]
fn material_toggle_has_one_accessible_action_and_keeps_live_feedback_above_content() {
    let mut r = demo(true);
    r.render(320., 700., &mut WrappingMetrics).unwrap();
    let card = by_id(r.dom(), "material");
    let inner = by_id(r.dom(), "inner");
    let frame = r.accessibility_frame().unwrap();
    assert_eq!(frame.node(card).unwrap().node.role(), Role::Button);
    assert_eq!(frame.node(card).unwrap().node.name(), "卡片項目");
    assert_eq!(frame.node(card).unwrap().node.is_checked(), Some(false));
    assert!(frame.children(card).next().is_none());
    assert!(frame.activation_target(inner).is_none());
    let ax_press = frame.activation_target(card).unwrap();
    assert_eq!(
        r.accessibility_activate(&ax_press).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert!(r.app().selected);

    let idle = r.render(320., 700., &mut WrappingMetrics).unwrap();
    let card = bounds(&r, "material");
    let state = bounds(&r, "material-state");
    let media = bounds(&r, "sample-media");
    let inner = bounds(&r, "inner");
    assert_eq!(state, card);
    let (x, y) = (inner.x + 8., inner.y + 8.);
    let layer_index = |display: &DisplayList, alpha| {
        display.ops.iter().rposition(|op| {
            matches!(op, PaintOp::Quad { rect, color, .. } if *rect == state && color.3 == alpha)
        }).unwrap()
    };
    let media_index = |display: &DisplayList| {
        display.ops.iter().position(|op| {
            matches!(op, PaintOp::Quad { rect, color, .. } if *rect == media && *color == Color::rgb(49, 100, 178))
        }).unwrap()
    };
    r.dispatch(Event::PointerMove { x, y }).unwrap();
    let hover = r.render(320., 700., &mut WrappingMetrics).unwrap();
    assert!(layer_index(&hover, 20) > media_index(&hover));
    assert_ne!(
        &hover.ops[..media_index(&hover)],
        &idle.ops[..media_index(&idle)],
        "hover raises the card shadow"
    );
    r.dispatch(Event::PointerDown { x, y }).unwrap();
    let pressed = r.render(320., 700., &mut WrappingMetrics).unwrap();
    assert!(layer_index(&pressed, 31) > media_index(&pressed));
    r.dispatch(Event::PointerUp { x, y }).unwrap();
    assert!(!r.app().selected);
    assert_eq!(r.app().child_hits, 0);

    r.dispatch(Event::PointerLeave).unwrap();
    assert_eq!(r.focused(), Some("material"));
    let focused = r.render(320., 700., &mut WrappingMetrics).unwrap();
    let focus_ring = focused.ops.iter().rposition(|op| {
        matches!(op, PaintOp::Stroke { rect, color, width, .. } if *rect == state && *width == 2. && *color == Theme::light().colors.secondary)
    }).unwrap();
    assert!(focus_ring > media_index(&focused));
    r.dispatch(Event::Activate).unwrap();
    assert!(r.app().selected);
    r.apply(CardMessage::Disable).unwrap();
    assert_eq!(r.focused(), Some("back"));
    r.render(320., 700., &mut WrappingMetrics).unwrap();
    let frame = r.accessibility_frame().unwrap();
    let card = by_id(r.dom(), "material");
    assert!(frame.node(card).unwrap().node.is_disabled());
    assert_eq!(frame.node(card).unwrap().node.is_checked(), Some(true));
    assert!(frame.activation_target(card).is_none());
}

#[test]
fn passive_card_wraps_large_rtl_content_and_keeps_distinct_tab_stops() {
    let mut r = demo(false);
    r.render(320., 700., &mut WrappingMetrics).unwrap();
    let card = bounds(&r, "material");
    let title = bounds(&r, "material-title");
    let first = bounds(&r, "first");
    let second = bounds(&r, "second");
    let third = bounds(&r, "third");
    assert!(title.height > 2. * 22. * 1.5);
    for item in [title, first, second, third] {
        assert!(item.x >= card.x + 16.);
        assert!(item.x + item.width <= card.x + card.width - 16.);
    }
    assert!(first.x > second.x);
    assert!(third.y >= first.y + 48.);
    let frame = r.accessibility_frame().unwrap();
    let card = by_id(r.dom(), "material");
    assert_eq!(frame.node(card).unwrap().node.role(), Role::Group);
    assert_eq!(
        frame
            .node(by_id(r.dom(), "material-title"))
            .unwrap()
            .node
            .role(),
        Role::Heading
    );
    assert!(frame.activation_target(card).is_none());
    assert!(frame.activation_target(by_id(r.dom(), "first")).is_some());
    for id in ["back", "first", "second", "third"] {
        r.dispatch(Event::FocusNext { reverse: false }).unwrap();
        assert_eq!(r.focused(), Some(id));
    }
    r.dispatch(Event::Activate).unwrap();
    assert_eq!(r.app().child_hits, 1);
    r.apply(CardMessage::Disable).unwrap();
    assert_eq!(r.focused(), Some("back"));
    r.render(320., 700., &mut WrappingMetrics).unwrap();
    let frame = r.accessibility_frame().unwrap();
    assert!(frame.node(by_id(r.dom(), "material-title")).is_some());
    assert!(frame.activation_target(by_id(r.dom(), "third")).is_none());
}
