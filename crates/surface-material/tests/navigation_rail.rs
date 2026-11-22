use surface::{Color, ExtendedPaintOp, PaintOp, Rect, Role, TextMeasurer};
use surface_app::{AccessibilityActivationResult, Application, EditKey, Event, Runtime, View};
use surface_material::{typography, ChoiceState, NavigationRailItem, Theme, VectorIcon};
use surface_wgpu::TextSystem;

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

struct Counter {
    count: usize,
}
impl Application for Counter {
    type Message = ();
    fn update(&mut self, _: ()) {
        self.count += 1;
    }
    fn view(&self) -> View<()> {
        let theme = Theme::light();
        theme.page([theme.navigation_rail("rail", "Destinations", items(&theme, 0))])
    }
}

/// Three destinations: `a` Home, `b` Search, `c` More (disabled).
fn items(t: &Theme, sel: usize) -> Vec<View<()>> {
    [("a", "Home", "H"), ("b", "Search", "S"), ("c", "More", "M")]
        .into_iter()
        .enumerate()
        .map(|(i, (id, name, icon))| {
            t.navigation_rail_item(
                id,
                name,
                icon,
                ChoiceState {
                    selected: sel == i,
                    disabled: i == 2,
                },
                (),
            )
        })
        .collect()
}

fn render(dark: bool, sel: usize) -> (Theme, Runtime<App>) {
    let build: Build =
        Box::new(move |t| t.page([t.navigation_rail("rail", "Destinations", items(t, sel))]));
    let mut r = Runtime::new(App(dark, build)).unwrap();
    r.render(400., 600., &mut Metrics).unwrap();
    let theme = if dark { Theme::dark() } else { Theme::light() };
    (theme, r)
}

fn bounds<A: Application>(r: &Runtime<A>, id: &str) -> surface::Rect {
    let computed = r.computed().unwrap();
    computed.nodes[r.dom().by_id(id).unwrap().index()].bounds
}

fn first_child(r: &Runtime<App>, id: &str) -> surface::NodeId {
    let node = r.dom().by_id(id).unwrap();
    r.dom().children(node).next().expect("indicator")
}

#[test]
fn rail_is_80_wide_and_items_64_high() {
    let (_, r) = render(false, 0);
    assert_eq!(bounds(&r, "rail").width, 80.);
    for id in ["a", "b", "c"] {
        assert_eq!(bounds(&r, id).height, 64., "item {id}");
    }
}

#[test]
fn roles_and_state() {
    let (_, r) = render(false, 0);
    let rail = r.dom().get(r.dom().by_id("rail").unwrap()).unwrap();
    assert_eq!(rail.role(), Role::Navigation);
    for (id, name) in [("a", "Home"), ("b", "Search"), ("c", "More")] {
        let node = r.dom().get(r.dom().by_id(id).unwrap()).unwrap();
        assert_eq!(node.role(), Role::Button, "{id}");
        assert_eq!(node.name(), name, "{id}");
    }
    let node = |id: &str| r.dom().get(r.dom().by_id(id).unwrap()).unwrap();
    assert_eq!(node("a").is_selected(), Some(true));
    assert_eq!(node("b").is_selected(), Some(false));
    assert_eq!(node("a").is_current_page(), None);
    assert_eq!(node("b").is_current_page(), None);
    assert!(node("c").is_disabled());
    assert!(!node("a").is_disabled());
}

#[test]
fn indicator_is_56_by_32_and_only_when_selected() {
    let (theme, r) = render(false, 0);
    let indicator_a = first_child(&r, "a");
    let rect = r.computed().unwrap().nodes[indicator_a.index()].bounds;
    assert_eq!((rect.width, rect.height), (56., 32.));
    assert_eq!(
        r.dom().get(indicator_a).unwrap().style().background,
        theme.colors.secondary_container
    );
    let indicator_b = first_child(&r, "b");
    assert_eq!(
        r.dom().get(indicator_b).unwrap().style().background,
        Color::TRANSPARENT
    );
}

#[test]
fn colors_use_tokens_in_both_themes() {
    for dark in [false, true] {
        let (theme, r) = render(dark, 0);
        let icon_of = |id: &str| {
            let indicator = first_child(&r, id);
            let icon = r.dom().children(indicator).next().expect("icon");
            r.dom().get(icon).unwrap().style().foreground
        };
        assert_eq!(
            icon_of("a"),
            theme.colors.on_secondary_container,
            "selected dark={dark}"
        );
        assert_eq!(
            icon_of("b"),
            theme.colors.on_surface_variant,
            "unselected dark={dark}"
        );
        assert_eq!(icon_of("c"), theme.outline, "disabled dark={dark}");
    }
}

#[test]
fn press_sends_the_message() {
    let mut r = Runtime::new(Counter { count: 0 }).unwrap();
    r.render(400., 600., &mut Metrics).unwrap();
    assert!(r.activate("b").unwrap());
    assert_eq!(r.app().count, 1);
    // A disabled destination sends nothing; the call may fail or report false.
    let _ = r.activate("c");
    assert_eq!(r.app().count, 1);
}

#[test]
fn selection_changes_the_output() {
    let (theme, r) = render(false, 0);
    assert_eq!(
        r.dom()
            .get(first_child(&r, "a"))
            .unwrap()
            .style()
            .background,
        theme.colors.secondary_container
    );
    assert_eq!(
        r.dom()
            .get(first_child(&r, "b"))
            .unwrap()
            .style()
            .background,
        Color::TRANSPARENT
    );

    let (theme, r) = render(false, 1);
    assert_eq!(
        r.dom()
            .get(first_child(&r, "b"))
            .unwrap()
            .style()
            .background,
        theme.colors.secondary_container
    );
    assert_eq!(
        r.dom()
            .get(first_child(&r, "a"))
            .unwrap()
            .style()
            .background,
        Color::TRANSPARENT
    );
}

#[test]
fn arrow_keys_move_focus() {
    let (_, mut r) = render(false, 0);
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), Some("a"));

    let down = Event::Edit {
        key: EditKey::Down,
        extend: false,
        by_word: false,
    };
    r.dispatch(down.clone()).unwrap();
    assert_eq!(r.focused(), Some("b"));

    // Down again skips the disabled `c` and wraps to `a`.
    r.dispatch(down).unwrap();
    assert_eq!(r.focused(), Some("a"));
}

const SETTINGS: &str = "全部使用者設定";
const SETTINGS_NAME: &str = "全部使用者設定，管理帳號";
const DESTINATIONS: [(&str, &str, &str); 3] = [
    ("settings", SETTINGS, SETTINGS_NAME),
    ("disabled", "封存", " "),
    ("search", "搜尋", "搜尋檔案"),
];

struct VectorRail {
    theme: Theme,
    scale: f32,
    icon: VectorIcon,
    selected: usize,
    activations: usize,
}

impl VectorRail {
    fn new(theme: Theme, scale: f32) -> Self {
        Self {
            theme,
            scale,
            icon: VectorIcon::new([[2., 3.], [22., 3.], [12., 21.]]).unwrap(),
            selected: 0,
            activations: 0,
        }
    }
}

impl Application for VectorRail {
    type Message = usize;

    fn update(&mut self, index: usize) {
        self.selected = index;
        self.activations += 1;
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn view(&self) -> View<usize> {
        self.theme.page([self.theme.navigation_rail(
            "compact-rail",
            "主要導覽",
            DESTINATIONS
                .into_iter()
                .enumerate()
                .map(|(index, (id, label, accessible_name))| {
                    self.theme
                        .navigation_rail_item_with_icon(NavigationRailItem {
                            id,
                            label,
                            accessible_name,
                            icon: &self.icon,
                            state: ChoiceState {
                                selected: self.selected == index,
                                disabled: index == 1,
                            },
                            message: index,
                        })
                }),
        )])
    }
}

#[test]
fn vector_rail_keeps_classic_geometry_and_wraps_real_chinese_glyphs_at_200_percent() {
    let mut fonts = TextSystem::default();
    for theme in [Theme::light(), Theme::dark()] {
        let mut normal = Runtime::new(VectorRail::new(theme, 1.)).unwrap();
        normal.render(320., 560., &mut fonts).unwrap();
        assert_eq!(bounds(&normal, "search").height, 64.);
        let mut large = Runtime::new(VectorRail::new(theme, 2.)).unwrap();
        let frame = large.render_extended(320., 560., &mut fonts).unwrap();
        assert_eq!(bounds(&large, "compact-rail").width, 80.);
        let settings = bounds(&large, "settings");
        assert!(settings.height > bounds(&normal, "settings").height);
        assert!(settings.y + settings.height <= bounds(&large, "disabled").y);

        let layout = large.computed().unwrap();
        for (id, fill, ink) in [
            (
                "settings",
                theme.colors.secondary_container,
                theme.colors.on_secondary_container,
            ),
            ("disabled", Color::TRANSPARENT, theme.outline),
            (
                "search",
                Color::TRANSPARENT,
                theme.colors.on_surface_variant,
            ),
        ] {
            let destination = large.dom().by_id(id).unwrap();
            let indicator = large.dom().children(destination).next().unwrap();
            let icon = large.dom().children(indicator).next().unwrap();
            let indicator_rect = layout.nodes[indicator.index()].bounds;
            let icon_rect = layout.nodes[icon.index()].bounds;
            assert_eq!((indicator_rect.width, indicator_rect.height), (56., 32.));
            assert_eq!(
                icon_rect,
                Rect::new(indicator_rect.x + 16., indicator_rect.y + 4., 24., 24.)
            );
            assert_eq!(large.dom().get(indicator).unwrap().style().background, fill);
            assert!(frame.ops().iter().any(|op| match op {
                ExtendedPaintOp::FillPath(path) => {
                    path.rect() == Rect::new(icon_rect.x + 2., icon_rect.y + 3., 20., 18.)
                        && path.clip() == icon_rect
                        && path.color() == ink
                }
                _ => false,
            }));
        }

        let (label, clip, style) = frame
            .ops()
            .iter()
            .find_map(|op| match op {
                ExtendedPaintOp::Legacy(PaintOp::StyledText {
                    rect,
                    clip,
                    text,
                    style,
                    ..
                }) if text == SETTINGS => Some((*rect, *clip, style)),
                _ => None,
            })
            .expect("the caller's visible label paints");
        assert_eq!(style, &typography::LABEL_MEDIUM.scaled(2.));
        assert!(label.height >= style.line_height_px() * 2.);
        assert_eq!(label.intersect(clip), label);
        assert_eq!(label.intersect(settings), label);

        for id in ["disabled", "search"] {
            let item = large.dom().by_id(id).unwrap();
            let label = large.dom().children(item).nth(1).unwrap();
            let geometry = &layout.nodes[label.index()];
            assert_eq!(geometry.bounds.height, style.line_height_px(), "{id}");
            assert_eq!(
                geometry.bounds.intersect(bounds(&large, id)),
                geometry.bounds
            );
        }
    }
}

#[test]
fn named_vector_rail_has_one_press_per_enabled_destination_and_manual_vertical_focus() {
    let mut runtime = Runtime::new(VectorRail::new(Theme::light(), 2.)).unwrap();
    let mut fonts = TextSystem::default();
    runtime.render(320., 560., &mut fonts).unwrap();

    let rail = runtime.dom().by_id("compact-rail").unwrap();
    let settings = runtime.dom().by_id("settings").unwrap();
    let disabled = runtime.dom().by_id("disabled").unwrap();
    let search = runtime.dom().by_id("search").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.node(rail).unwrap().node.role(), Role::Navigation);
    assert_eq!(
        frame.children(rail).collect::<Vec<_>>(),
        [settings, disabled, search]
    );
    for (id, name, selected, enabled) in [
        (settings, SETTINGS_NAME, true, true),
        (disabled, "封存", false, false),
        (search, "搜尋檔案", false, true),
    ] {
        let node = frame.node(id).unwrap().node;
        assert_eq!(node.role(), Role::Button);
        assert_eq!(node.name(), name);
        assert_eq!(node.is_selected(), Some(selected));
        assert_eq!(node.is_current_page(), Some(selected));
        assert_eq!(node.is_disabled(), !enabled);
        assert_eq!(frame.activation_target(id).is_some(), enabled);
        assert!(frame.children(id).next().is_none());
    }

    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("settings"));
    for (key, focus) in [
        (EditKey::Down, "search"),
        (EditKey::Down, "settings"),
        (EditKey::Up, "search"),
    ] {
        runtime
            .dispatch(Event::Edit {
                key,
                extend: false,
                by_word: false,
            })
            .unwrap();
        assert_eq!(runtime.focused(), Some(focus));
    }
    assert_eq!(runtime.app().selected, 0);
    assert_eq!(runtime.app().activations, 0);

    runtime.render(320., 560., &mut fonts).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    let action = frame.activation_target(search).unwrap();
    assert_eq!(
        runtime.accessibility_activate(&action).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert_eq!(runtime.app().selected, 2);
    assert_eq!(runtime.app().activations, 1);
    assert_eq!(runtime.focused(), Some("search"));
    assert_eq!(
        runtime.accessibility_activate(&action).unwrap(),
        AccessibilityActivationResult::Rejected
    );

    runtime.render(320., 560., &mut fonts).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(
        frame.node(settings).unwrap().node.is_selected(),
        Some(false)
    );
    assert_eq!(frame.node(search).unwrap().node.is_selected(), Some(true));
    assert_eq!(
        frame.node(settings).unwrap().node.is_current_page(),
        Some(false)
    );
    assert_eq!(
        frame.node(search).unwrap().node.is_current_page(),
        Some(true)
    );
    for (id, expected) in [(settings, 2), (disabled, 2)] {
        runtime.render(320., 560., &mut fonts).unwrap();
        let label = runtime.dom().children(id).nth(1).unwrap();
        let rect = runtime.computed().unwrap().nodes[label.index()].bounds;
        let (x, y) = (rect.x + rect.width / 2., rect.y + rect.height - 1.);
        runtime.dispatch(Event::PointerDown { x, y }).unwrap();
        runtime.dispatch(Event::PointerUp { x, y }).unwrap();
        assert_eq!(runtime.app().activations, expected);
    }
    assert_eq!(runtime.app().selected, 0);
}
