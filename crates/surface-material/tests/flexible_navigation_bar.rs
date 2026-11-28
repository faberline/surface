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
    horizontal: bool,
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
            horizontal: false,
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
        let options = FlexibleNavigationBarOptions {
            id: BAR,
            name: "主要導覽",
            available_width: self.width,
            direction: self.direction,
        };
        if self.horizontal {
            theme.horizontal_flexible_navigation_bar(options, items)
        } else {
            theme.flexible_navigation_bar(options, items)
        }
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

    fn viewport_changed(&mut self, width: f32, _height: f32) -> bool {
        if self.horizontal && self.width != width {
            self.width = width;
            true
        } else {
            false
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

fn medium(width: f32) -> Page {
    Page {
        horizontal: true,
        width,
        ..Page::default()
    }
}

fn right(rect: Rect) -> f32 {
    rect.x + rect.width
}

fn bottom(rect: Rect) -> f32 {
    rect.y + rect.height
}

fn horizontal_icon(runtime: &Runtime<Page>, indicator: &str) -> Rect {
    let icon = runtime
        .dom()
        .children(id(runtime, indicator))
        .next()
        .unwrap();
    runtime.computed().unwrap().nodes[icon.index()].bounds
}

#[test]
fn medium_bar_accepts_only_medium_width_and_three_to_five_destinations() {
    for (count, width, valid) in [
        (2, 600., false),
        (3, 599., false),
        (3, 600., true),
        (4, 720., true),
        (5, 839., true),
        (5, 839.5, true),
        (5, 840., false),
        (6, 720., false),
        (5, f32::NAN, false),
        (5, f32::INFINITY, false),
    ] {
        assert_eq!(
            Page {
                count,
                ..medium(width)
            }
            .bar()
            .is_some(),
            valid,
            "{count} at {width}"
        );
    }
}

#[test]
fn medium_cells_keep_their_width_and_center_as_the_window_changes() {
    for count in [3, 4, 5] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let mut runtime = Runtime::new(Page {
                count,
                direction,
                ..medium(600.)
            })
            .unwrap();
            let mut preceding = None;
            for width in [600., 720., 839.] {
                runtime.render_extended(width, 300., &mut Fonts).unwrap();
                let bar = rect(&runtime, BAR);
                assert_eq!(bar, Rect::new(0., 0., width, 64.));
                let mut cells = ITEMS[..count]
                    .iter()
                    .map(|key| rect(&runtime, key))
                    .collect::<Vec<_>>();
                if direction == LayoutDirection::RightToLeft {
                    cells.reverse();
                }
                let centered = (width - 112. * count as f32) / 2.;
                let leading = cells[0].x;
                let trailing = right(bar) - right(*cells.last().unwrap());
                assert!((leading - centered).abs() <= 0.5 && (trailing - centered).abs() <= 0.5);
                assert!(cells
                    .iter()
                    .all(|cell| cell.width == 112. && cell.y == 0. && cell.height == 64.));
                assert!(cells.windows(2).all(|pair| right(pair[0]) == pair[1].x));
                if let Some(other) = preceding {
                    assert!(leading > other);
                }
                preceding = Some(leading);
            }
        }
    }
}

#[test]
fn medium_active_label_and_icon_share_the_pill_color_and_mirror() {
    for (dark, direction) in [
        (false, LayoutDirection::LeftToRight),
        (true, LayoutDirection::RightToLeft),
    ] {
        let theme = if dark { Theme::dark() } else { Theme::light() };
        let mut runtime = Runtime::new(Page {
            dark,
            direction,
            ..medium(720.)
        })
        .unwrap();
        let draw = runtime.render_extended(720., 300., &mut Fonts).unwrap();
        assert_eq!(
            node(&runtime, BAR).style().background,
            theme.colors.surface_container
        );
        assert_eq!(
            node(&runtime, BAR).box_shadow_layers(),
            theme.elevation(Elevation::Level2)
        );
        for key in ["home", "explore", "saved"] {
            let cell = rect(&runtime, key);
            let indicator = format!("{key}-indicator");
            let pill = rect(&runtime, &indicator);
            let icon = horizontal_icon(&runtime, &indicator);
            let label = rect(&runtime, &format!("{key}-label"));
            assert!(pill.width < cell.width);
            assert_eq!(pill.height, 40.);
            assert_eq!(pill.y - cell.y, 12.);
            assert_eq!(bottom(cell) - bottom(pill), 12.);
            assert_eq!((icon.width, icon.height), (24., 24.));
            assert_eq!(icon.y - pill.y, 8.);
            let alignment = match direction {
                LayoutDirection::LeftToRight => {
                    assert_eq!(icon.x - pill.x, 16.);
                    assert_eq!(label.x - right(icon), 4.);
                    assert_eq!(right(pill) - right(label), 16.);
                    TextAlignment::Left
                }
                LayoutDirection::RightToLeft => {
                    assert_eq!(right(pill) - right(icon), 16.);
                    assert_eq!(icon.x - right(label), 4.);
                    assert_eq!(label.x - pill.x, 16.);
                    TextAlignment::Right
                }
            };
            assert_eq!(
                ink_at(&draw, pill, theme.colors.secondary_container, 20.),
                key == "home"
            );
            let Color(r, g, b, _) = theme.colors.on_surface_variant;
            let ink = match key {
                "home" => theme.colors.on_secondary_container,
                "explore" => Color(r, g, b, 97),
                _ => theme.colors.on_surface_variant,
            };
            assert!(draw.ops().iter().any(|op| matches!(op, ExtendedPaintOp::FillPath(path)
                if path.color() == ink && path.rect() == Rect::new(icon.x + 2., icon.y + 2., 20., 20.))));
            assert!(draw.ops().iter().any(|op| matches!(op, ExtendedPaintOp::AlignedText(text)
                if text.alignment() == alignment && matches!(text.run(), PaintOp::StyledText { rect, color, style, .. }
                    if *rect == label && *color == ink && *style == if key == "home" {
                        typography::LABEL_MEDIUM_EMPHASIZED
                    } else { typography::LABEL_MEDIUM }))));
        }
    }
}

#[test]
fn medium_large_rtl_names_wrap_without_reflow_on_selection() {
    let mut runtime = Runtime::new(Page {
        dark: true,
        direction: LayoutDirection::RightToLeft,
        enlarged: true,
        long_label: true,
        ..medium(600.)
    })
    .unwrap();
    let mut fonts = TextSystem::default();
    let draw = runtime.render_extended(600., 500., &mut fonts).unwrap();
    let bar = rect(&runtime, BAR);
    let cell = rect(&runtime, "saved");
    let label = rect(&runtime, "saved-label");
    let pill = rect(&runtime, "saved-indicator");
    assert!(bar.height > 64. && label.height >= 64. && label.width <= 52.);
    assert_eq!(bar.intersect(cell), cell);
    assert_eq!(cell.intersect(pill), pill);
    assert_eq!(pill.intersect(label), label);
    assert_eq!(pill.x + 16., label.x);
    assert_eq!(pill.y - cell.y, 12.);
    assert_eq!(bottom(cell) - bottom(pill), 12.);
    let screen = runtime.accessibility_frame().unwrap();
    let bar_id = id(&runtime, BAR);
    assert_eq!(screen.node(bar_id).unwrap().node.role(), Role::Navigation);
    assert_eq!(screen.children(bar_id).count(), 5);
    let saved = id(&runtime, "saved");
    assert_eq!(screen.node(saved).unwrap().node.name(), LONG_LABEL);
    assert!(screen.children(saved).next().is_none());
    assert!(draw.ops().iter().any(|op| matches!(op, ExtendedPaintOp::AlignedText(aligned)
        if aligned.alignment() == TextAlignment::Right && matches!(aligned.run(), PaintOp::StyledText { text, style, clip, .. }
            if text == LONG_LABEL && style.font_size == 24. && style.line_height == Some(32.)
                && clip.intersect(label) == label))));
    runtime.activate("saved").unwrap();
    runtime.render_extended(600., 500., &mut fonts).unwrap();
    assert_eq!(rect(&runtime, BAR), bar);
    assert_eq!(rect(&runtime, "saved"), cell);
    assert_eq!(rect(&runtime, "saved-label"), label);
    assert_eq!(rect(&runtime, "saved-indicator"), pill);
    assert_eq!(node(&runtime, "saved").is_current_page(), Some(true));
    assert_eq!(node(&runtime, "home").is_current_page(), Some(false));
}

#[test]
fn medium_reuses_trusted_focus_pointer_and_controlled_state() {
    let mut runtime = Runtime::new(Page {
        accept: false,
        ..medium(600.)
    })
    .unwrap();
    runtime.render_extended(600., 300., &mut Fonts).unwrap();
    for target in ["home", "saved", "alerts"] {
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some(target));
        assert_eq!(runtime.app().selected, 0);
    }
    runtime
        .dispatch(Event::FocusNext { reverse: true })
        .unwrap();
    assert_eq!(runtime.focused(), Some("saved"));
    runtime.render_extended(839., 300., &mut Fonts).unwrap();
    assert_eq!(runtime.focused(), Some("saved"));
    assert_eq!(runtime.app().selected, 0);
    let draw = runtime.render_extended(839., 300., &mut Fonts).unwrap();
    let saved = rect(&runtime, "saved");
    assert!(draw.ops().iter().any(
        |op| matches!(op, ExtendedPaintOp::Legacy(PaintOp::Stroke { rect, width, .. })
        if *rect == saved && *width == 3.)
    ));
    let key = KeyPress::new(Key::Space, Modifiers::default());
    let action = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(id(&runtime, "saved"))
        .unwrap();
    assert!(runtime
        .accessibility_key(&action, &key, Event::Activate)
        .unwrap());
    assert_eq!(runtime.app().received, [2]);
    runtime.render_extended(839., 300., &mut Fonts).unwrap();
    let action = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(id(&runtime, "saved"))
        .unwrap();
    assert!(runtime
        .accessibility_key(&action, &key.with_repeat(true), Event::Activate)
        .unwrap());
    assert_eq!(runtime.app().received, [2]);
    let disabled = rect(&runtime, "explore");
    assert!(runtime.activate("explore").is_err());
    runtime
        .dispatch(Event::PointerDown {
            x: disabled.x + 5.,
            y: disabled.y + 5.,
        })
        .unwrap();
    runtime
        .dispatch(Event::PointerUp {
            x: disabled.x + 5.,
            y: disabled.y + 5.,
        })
        .unwrap();
    assert_eq!(runtime.app().received, [2]);
    let hit = rect(&runtime, "alerts");
    let (x, y) = (right(hit) - 2., bottom(hit) - 2.);
    assert!(!rect(&runtime, "alerts-indicator").contains(x, y));
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.cancel_pointer().unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().received, [2]);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().received, [2, 3]);
    assert_eq!(runtime.focused(), Some("alerts"));
    assert_eq!(runtime.app().selected, 0);
}
