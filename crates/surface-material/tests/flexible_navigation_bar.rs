use surface::{
    Color, ExtendedDisplayList, ExtendedPaintOp, Node, NodeId, PaintOp, Rect, Role, TextAlignment,
    TextMeasurer,
};
use surface_app::{Application, Event, Key, KeyPress, Modifiers, Runtime, View};
use surface_material::{
    shape::LayoutDirection, typography, ChoiceState, Elevation, FlexibleNavigationBarOptions,
    NavigationRailItem, Theme, VectorIcon,
};
use surface_wgpu::TextSystem;

const BAR: &str = "flex-nav";
const ITEMS: [&str; 6] = ["home", "explore", "saved", "alerts", "settings", "extra"];
const LONG_LABEL: &str = "收藏項目與閱讀清單";

struct Page {
    dark: bool,
    direction: LayoutDirection,
    width: f32,
    count: usize,
    enlarged: bool,
    long_label: bool,
    selected: usize,
    accept: bool,
    received: Vec<usize>,
    icon: VectorIcon,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            dark: false,
            direction: LayoutDirection::LeftToRight,
            width: 320.,
            count: 5,
            enlarged: false,
            long_label: false,
            selected: 0,
            accept: true,
            received: Vec::new(),
            icon: VectorIcon::new([[2., 2.], [22., 2.], [22., 22.], [2., 22.]]).unwrap(),
        }
    }
}

impl Page {
    fn bar(&self) -> Option<View<usize>> {
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let items = ITEMS[..self.count]
            .iter()
            .enumerate()
            .map(|(index, id)| NavigationRailItem {
                id,
                label: if index == 2 && self.long_label {
                    LONG_LABEL
                } else {
                    ["首頁", "探索", "收藏", "通知", "設定", "更多"][index]
                },
                accessible_name: if index == 0 { "主要頁面" } else { "" },
                icon: &self.icon,
                state: ChoiceState {
                    selected: self.selected == index,
                    disabled: index == 1,
                },
                message: index,
            });
        theme.flexible_navigation_bar(
            FlexibleNavigationBarOptions {
                id: BAR,
                name: "主要導覽",
                available_width: self.width,
                direction: self.direction,
            },
            items,
        )
    }
}

impl Application for Page {
    type Message = usize;

    fn update(&mut self, index: usize) {
        self.received.push(index);
        if self.accept {
            self.selected = index;
        }
    }

    fn view(&self) -> View<usize> {
        View::new(Node::new(Role::Window, "Test")).children(self.bar())
    }

    fn text_scale(&self) -> f32 {
        if self.enlarged {
            2.
        } else {
            1.
        }
    }
}

struct Fonts;

impl TextMeasurer for Fonts {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let natural = text.chars().count() as f32 * size;
        let width = width.unwrap_or(natural).min(natural);
        let lines = (natural / width.max(1.)).ceil().max(1.);
        (width, lines * size * 1.5)
    }
}

fn draw(runtime: &mut Runtime<Page>) -> ExtendedDisplayList {
    runtime.render_extended(320., 260., &mut Fonts).unwrap()
}

fn id(runtime: &Runtime<Page>, key: &str) -> NodeId {
    runtime.dom().by_id(key).unwrap()
}

fn node<'a>(runtime: &'a Runtime<Page>, key: &str) -> &'a Node {
    runtime.dom().get(id(runtime, key)).unwrap()
}

fn rect(runtime: &Runtime<Page>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].bounds
}

fn ink_at(list: &ExtendedDisplayList, bounds: Rect, expected: Color, radius: f32) -> bool {
    list.ops().iter().any(|entry| {
        matches!(
            entry,
            ExtendedPaintOp::Legacy(PaintOp::Quad { rect, color, radius: r, .. })
                if *rect == bounds && *color == expected && *r == radius
        )
    })
}

#[test]
fn accepts_only_compact_width_and_three_to_five_complete_cells() {
    for (count, width, valid) in [
        (2, 320., false),
        (3, 167., false),
        (3, 168., true),
        (4, 320., true),
        (5, 279., false),
        (5, 280., true),
        (5, 599., true),
        (5, 600., false),
        (6, 320., false),
        (3, f32::NAN, false),
        (3, f32::INFINITY, false),
        (3, 0., false),
    ] {
        assert_eq!(
            Page {
                count,
                width,
                ..Page::default()
            }
            .bar()
            .is_some(),
            valid,
            "{count} at {width}"
        );
    }
}

#[test]
fn compact_three_four_five_destinations_have_equal_hit_cells() {
    for count in [3, 4, 5] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let mut runtime = Runtime::new(Page {
                count,
                direction,
                ..Page::default()
            })
            .unwrap();
            draw(&mut runtime);
            assert_eq!(rect(&runtime, BAR), Rect::new(0., 0., 320., 64.));
            let mut cells = ITEMS[..count]
                .iter()
                .map(|key| rect(&runtime, key))
                .collect::<Vec<_>>();
            if direction == LayoutDirection::RightToLeft {
                cells.reverse();
            }
            assert_eq!(cells[0].x, 0.);
            assert_eq!(cells.last().unwrap().x + cells.last().unwrap().width, 320.);
            assert!(cells.iter().all(|cell| {
                cell.y == 0. && cell.height == 64. && (cell.width - 320. / count as f32).abs() < 1.
            }));
            assert!(cells
                .windows(2)
                .all(|pair| pair[0].x + pair[0].width == pair[1].x));
        }
    }
}

#[test]
fn pill_icons_and_text_use_expressive_geometry_and_palette_in_both_themes() {
    for dark in [false, true] {
        let theme = if dark { Theme::dark() } else { Theme::light() };
        let mut runtime = Runtime::new(Page {
            dark,
            ..Page::default()
        })
        .unwrap();
        let paint = draw(&mut runtime);
        let bar = node(&runtime, BAR);
        assert_eq!(bar.style().background, theme.colors.surface_container);
        assert_eq!(bar.box_shadow_layers(), theme.elevation(Elevation::Level2));
        assert!(ink_at(
            &paint,
            rect(&runtime, BAR),
            theme.colors.surface_container,
            0.
        ));
        for key in ["home", "explore", "saved"] {
            let hit = rect(&runtime, key);
            let indicator = format!("{key}-indicator");
            let pill = rect(&runtime, &indicator);
            let caption = format!("{key}-label");
            let label = rect(&runtime, &caption);
            assert_eq!((pill.width, pill.height), (56., 32.));
            assert_eq!(pill.y - hit.y, 6.);
            assert_eq!(pill.x + pill.width / 2., hit.x + hit.width / 2.);
            assert_eq!(label.y - (pill.y + pill.height), 4.);
            assert_eq!((hit.y + hit.height) - (label.y + label.height), 6.);
            assert_eq!(label.x + label.width / 2., hit.x + hit.width / 2.);
            let artwork = runtime
                .dom()
                .children(id(&runtime, &indicator))
                .next()
                .unwrap();
            let icon = runtime.computed().unwrap().nodes[artwork.index()].bounds;
            assert_eq!(icon, Rect::new(pill.x + 16., pill.y + 4., 24., 24.));

            let selected = key == "home";
            let indicator_color = if selected {
                theme.colors.secondary_container
            } else {
                Color::TRANSPARENT
            };
            assert_eq!(
                node(&runtime, &indicator).style().background,
                indicator_color
            );
            assert_eq!(
                ink_at(&paint, pill, theme.colors.secondary_container, 16.),
                selected
            );
            let Color(r, g, b, _) = theme.colors.on_surface_variant;
            let (icon_ink, text_ink, text_style) = match key {
                "home" => (
                    theme.colors.on_secondary_container,
                    theme.colors.secondary,
                    typography::LABEL_MEDIUM_EMPHASIZED,
                ),
                "explore" => (
                    Color(r, g, b, 97),
                    Color(r, g, b, 97),
                    typography::LABEL_MEDIUM,
                ),
                _ => (
                    theme.colors.on_surface_variant,
                    theme.colors.on_surface_variant,
                    typography::LABEL_MEDIUM,
                ),
            };
            assert!(paint.ops().iter().any(|op| matches!(op, ExtendedPaintOp::FillPath(path)
                if path.color() == icon_ink && path.rect() == Rect::new(icon.x + 2., icon.y + 2., 20., 20.))));
            assert!(paint
                .ops()
                .iter()
                .any(|op| matches!(op, ExtendedPaintOp::AlignedText(text)
                if text.alignment() == TextAlignment::Center
                    && matches!(text.run(), PaintOp::StyledText { rect, color, style, .. }
                        if *rect == label && *color == text_ink && *style == text_style))));
        }
    }
}

#[test]
fn full_rtl_labels_wrap_at_two_hundred_percent_without_moving_the_icon_stack() {
    let mut runtime = Runtime::new(Page {
        dark: true,
        direction: LayoutDirection::RightToLeft,
        enlarged: true,
        long_label: true,
        selected: 2,
        ..Page::default()
    })
    .unwrap();
    let draw = runtime
        .render_extended(320., 260., &mut TextSystem::default())
        .unwrap();
    let bar = rect(&runtime, BAR);
    assert!(bar.height > 64.);
    let destination = rect(&runtime, "saved");
    let label = rect(&runtime, "saved-label");
    assert!(label.height >= 64.);
    assert_eq!(bar.intersect(label), label);
    assert_eq!(destination.intersect(label), label);
    assert_eq!(
        destination.y + destination.height - (label.y + label.height),
        6.
    );
    assert!(draw
        .ops()
        .iter()
        .any(|op| matches!(op, ExtendedPaintOp::AlignedText(aligned)
        if matches!(aligned.run(), PaintOp::StyledText { text, style, clip, .. }
            if text == LONG_LABEL && style.font_size == 24. && style.line_height == Some(32.)
                && clip.intersect(label) == label))));
    for key in ITEMS[..5].iter() {
        let hit = rect(&runtime, key);
        let pill = rect(&runtime, &format!("{key}-indicator"));
        assert_eq!((hit.width, hit.height), (64., bar.height));
        assert_eq!((pill.width, pill.height, pill.y - hit.y), (56., 32., 6.));
    }
}

#[test]
fn pointer_uses_the_full_target_and_keeps_state_ink_within_the_pill() {
    let mut runtime = Runtime::new(Page::default()).unwrap();
    draw(&mut runtime);
    let row = rect(&runtime, "saved");
    let pill = rect(&runtime, "saved-indicator");
    let (x, y) = (row.x + 2., row.y + row.height - 2.);
    assert!(!pill.contains(x, y));
    let Color(r, g, b, _) = Theme::light().colors.on_secondary_container;
    runtime.dispatch(Event::PointerMove { x, y }).unwrap();
    let hover = draw(&mut runtime);
    assert!(ink_at(&hover, pill, Color(r, g, b, 20), 16.));
    assert!(!ink_at(&hover, row, Color(r, g, b, 20), 0.));
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    assert_eq!(runtime.focused(), Some("saved"));
    assert!(runtime.app().received.is_empty());
    let press = draw(&mut runtime);
    assert!(ink_at(&press, pill, Color(r, g, b, 26), 16.));
    assert!(!ink_at(&press, row, Color(r, g, b, 26), 0.));
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().received, [2]);
    assert_eq!(node(&runtime, "saved").is_current_page(), Some(true));
    let focus = draw(&mut runtime);
    assert!(ink_at(&focus, pill, Color(r, g, b, 26), 16.));
    assert_eq!(rect(&runtime, "saved"), row);
}

#[test]
fn each_enabled_button_tabs_in_reading_order_and_only_caller_changes_the_page() {
    for accept in [false, true] {
        let mut runtime = Runtime::new(Page {
            direction: LayoutDirection::RightToLeft,
            enlarged: true,
            long_label: true,
            accept,
            ..Page::default()
        })
        .unwrap();
        draw(&mut runtime);
        for target in ["home", "saved", "alerts", "settings"] {
            assert!(runtime
                .dispatch(Event::FocusNext { reverse: false })
                .unwrap());
            assert_eq!(runtime.focused(), Some(target));
            assert_eq!(runtime.app().selected, 0);
            assert!(runtime.app().received.is_empty());
        }
        runtime
            .dispatch(Event::FocusNext { reverse: true })
            .unwrap();
        assert_eq!(runtime.focused(), Some("alerts"));
        let paint = draw(&mut runtime);
        let ink = Theme::light().colors.secondary;
        assert!(paint.ops().iter().any(|op| matches!(
            op, ExtendedPaintOp::Legacy(PaintOp::Stroke { rect: focus, color, width, .. })
                if *focus == rect(&runtime, "alerts") && *color == ink && *width == 3.
        )));
        for control in [Key::Enter, Key::Space] {
            let focus = runtime
                .accessibility_frame()
                .unwrap()
                .focus_target(id(&runtime, "alerts"))
                .unwrap();
            let key = KeyPress::new(control, Modifiers::default());
            assert!(runtime
                .accessibility_key(&focus, &key, Event::Activate)
                .unwrap());
            draw(&mut runtime);
            let current = runtime
                .accessibility_frame()
                .unwrap()
                .focus_target(id(&runtime, "alerts"))
                .unwrap();
            let before_repeat = runtime.app().received.len();
            assert!(runtime
                .accessibility_key(&current, &key.with_repeat(true), Event::Activate)
                .unwrap());
            assert_eq!(runtime.app().received.len(), before_repeat);
        }
        assert_eq!(runtime.app().received, [3, 3]);
        assert_eq!(runtime.app().selected, if accept { 3 } else { 0 });
        assert_eq!(node(&runtime, "alerts").is_current_page(), Some(accept));
        assert_eq!(node(&runtime, "home").is_current_page(), Some(!accept));
        assert_eq!(runtime.focused(), Some("alerts"));
    }
}

#[test]
fn accessible_buttons_exclude_decoration_and_block_disabled_activation() {
    let mut runtime = Runtime::new(Page {
        long_label: true,
        ..Page::default()
    })
    .unwrap();
    draw(&mut runtime);
    let frame = runtime.accessibility_frame().unwrap();
    let bar = id(&runtime, BAR);
    assert_eq!(frame.node(bar).unwrap().node.name(), "主要導覽");
    assert_eq!(frame.node(bar).unwrap().node.role(), Role::Navigation);
    assert_eq!(
        frame.children(bar).collect::<Vec<_>>(),
        ITEMS[..5]
            .iter()
            .map(|key| id(&runtime, key))
            .collect::<Vec<_>>()
    );
    for key in &ITEMS[..5] {
        let target = id(&runtime, key);
        assert!(frame.children(target).next().is_none());
        assert_eq!(frame.node(target).unwrap().node.role(), Role::Button);
        let enabled = *key != "explore";
        assert_eq!(frame.focus_target(target).is_some(), enabled);
        assert_eq!(frame.activation_target(target).is_some(), enabled);
    }
    assert_eq!(node(&runtime, "home").name(), "主要頁面");
    assert_eq!(node(&runtime, "saved").name(), LONG_LABEL);
    let disabled = rect(&runtime, "explore");
    let (x, y) = (
        disabled.x + disabled.width / 2.,
        disabled.y + disabled.height / 2.,
    );
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert!(runtime.activate("explore").is_err());
    assert!(runtime.app().received.is_empty());
}
