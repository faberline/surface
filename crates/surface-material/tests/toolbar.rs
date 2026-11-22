use surface::{Align, Color, Insets, Node, Rect, Role, Style, TextMeasurer, TextScaling};
use surface_app::{Application, EditKey, Event, Runtime, View};
use surface_material::{
    shape::{self, LayoutDirection},
    Elevation, FabColor, FabSize, Theme, ToolbarButtonOptions, ToolbarContent, ToolbarKind,
    ToolbarOptions, ToolbarPalette, ToolbarVisibility,
};

#[derive(Clone, Copy)]
enum Message {
    Press,
    Toggle,
    SetVisibility(ToolbarVisibility),
}

struct Demo {
    kind: ToolbarKind,
    theme: Theme,
    palette: ToolbarPalette,
    direction: LayoutDirection,
    visibility: ToolbarVisibility,
    fab: bool,
    extra_actions: usize,
    floating_alignment: Option<Align>,
    selected: bool,
    presses: u32,
    scale: f32,
}

impl Demo {
    fn new(kind: ToolbarKind) -> Self {
        Self {
            kind,
            theme: Theme::light(),
            palette: ToolbarPalette::Standard,
            direction: LayoutDirection::LeftToRight,
            visibility: ToolbarVisibility::Expanded,
            fab: false,
            extra_actions: 0,
            floating_alignment: None,
            selected: false,
            presses: 0,
            scale: 1.,
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
            Message::Press => self.presses += 1,
            Message::Toggle => self.selected = !self.selected,
            Message::SetVisibility(visibility) => self.visibility = visibility,
        }
    }

    fn view(&self) -> View<Self::Message> {
        let button = |id, label, icon, selected, disabled, message| {
            self.theme.toolbar_button(
                ToolbarButtonOptions {
                    id,
                    label,
                    icon,
                    palette: self.palette,
                    selected,
                    disabled,
                },
                message,
            )
        };
        let regular = |id, label| button(id, label, "⋯", None, false, Message::Press);
        let fab = self.fab.then(|| {
            self.theme.fab(
                "fab",
                "Add",
                "+",
                FabSize::Regular,
                FabColor::Primary,
                false,
                Message::Press,
            )
        });
        let trailing = [regular("more", "More actions").restore_focus_on_removal("toggle")]
            .into_iter()
            .chain(
                [("undo", "Undo"), ("redo", "Redo"), ("delete", "Delete")]
                    .into_iter()
                    .take(self.extra_actions)
                    .map(|(id, label)| regular(id, label)),
            )
            .collect();
        let toolbar = self.theme.toolbar(
            ToolbarOptions {
                id: "tools",
                name: "編輯工具",
                kind: self.kind,
                palette: self.palette,
                direction: self.direction,
                visibility: self.visibility,
                restore_focus_to: "body",
            },
            ToolbarContent {
                leading: vec![regular("cut", "Cut")],
                core: vec![
                    button(
                        "toggle",
                        "粗體",
                        "B",
                        Some(self.selected),
                        false,
                        Message::Toggle,
                    ),
                    button(
                        "disabled",
                        "Read only",
                        "×",
                        Some(true),
                        true,
                        Message::Press,
                    ),
                ],
                trailing,
                fab,
            },
        );
        let toolbar = match self.floating_alignment {
            Some(align) => View::new(
                Node::new(Role::Group, "").id("inset-layer").styled(
                    Style::column()
                        .absolute(Insets::all(16.))
                        .align(align)
                        .justify(Align::End),
                ),
            )
            .children([toolbar]),
            None => toolbar,
        };
        View::new(Node::new(Role::Window, "Workspace")).children([
            regular("body", "Workspace content"),
            toolbar,
            regular("outside", "Outside"),
        ])
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _width: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.75, size * 1.5)
    }
}

fn render(runtime: &mut Runtime<Demo>, width: f32) {
    runtime.render(width, 600., &mut Metrics).unwrap();
}

fn bounds(runtime: &Runtime<Demo>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn node<'a>(runtime: &'a Runtime<Demo>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn tab(runtime: &mut Runtime<Demo>) {
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
}

fn key(runtime: &mut Runtime<Demo>, key: EditKey) {
    runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false,
        })
        .unwrap();
}

#[test]
fn toolbar_variants_use_material_container_tokens_in_both_themes() {
    for theme in [Theme::light(), Theme::dark()] {
        for palette in [ToolbarPalette::Standard, ToolbarPalette::Vibrant] {
            for kind in [
                ToolbarKind::Docked,
                ToolbarKind::FloatingHorizontal,
                ToolbarKind::FloatingVertical,
            ] {
                let mut app = Demo::new(kind);
                app.theme = theme;
                app.palette = palette;
                let mut runtime = Runtime::new(app).unwrap();
                render(&mut runtime, 320.);
                let group = node(&runtime, "tools");
                let area = bounds(&runtime, "tools");
                let leading = bounds(&runtime, "cut");
                let (width, height, corner, elevation) = match kind {
                    ToolbarKind::Docked => (320., 64., shape::NONE, Elevation::Level0),
                    ToolbarKind::FloatingHorizontal => (220., 64., shape::FULL, Elevation::Level3),
                    ToolbarKind::FloatingVertical => (64., 220., shape::FULL, Elevation::Level3),
                };
                assert_eq!(
                    (area.width, area.height),
                    (width, height),
                    "{kind:?} {area:?}"
                );
                assert_eq!(
                    leading.x - area.x,
                    if kind == ToolbarKind::Docked { 16. } else { 8. }
                );
                assert_eq!(area.y + 8., leading.y);
                assert_eq!(group.role(), Role::Group);
                assert_eq!(group.name(), "編輯工具");
                assert_eq!(group.style().corner_radii(), corner);
                assert_eq!(group.box_shadow_layers(), theme.elevation(elevation));
                assert_eq!(
                    group.style().background,
                    if palette == ToolbarPalette::Standard {
                        theme.colors.surface_container
                    } else {
                        theme.colors.primary_container
                    }
                );
            }
        }
    }
}

#[test]
fn button_keeps_a_fixed_glyph_and_touch_target_with_palette_and_toggle_state() {
    for theme in [Theme::light(), Theme::dark()] {
        for palette in [ToolbarPalette::Standard, ToolbarPalette::Vibrant] {
            let mut app = Demo::new(ToolbarKind::FloatingHorizontal);
            app.theme = theme;
            app.palette = palette;
            app.scale = 2.;
            let mut runtime = Runtime::new(app).unwrap();
            render(&mut runtime, 320.);
            let glyph_styles = |runtime: &Runtime<Demo>, id| {
                let dom = runtime.dom();
                let face = dom.children(dom.by_id(id).unwrap()).next().unwrap();
                let glyph = dom.children(face).next().unwrap();
                let vis = dom.get(face).unwrap();
                let symbol = dom.get(glyph).unwrap();
                assert_eq!(symbol.text_scaling_mode(), TextScaling::Fixed);
                assert_eq!(symbol.style().font_size, 24.);
                (vis.style().background, symbol.style().foreground)
            };
            let unselected = if palette == ToolbarPalette::Standard {
                (
                    theme.colors.surface_container,
                    theme.colors.on_surface_variant,
                )
            } else {
                (
                    theme.colors.primary_container,
                    theme.colors.on_primary_container,
                )
            };
            assert_eq!(glyph_styles(&runtime, "toggle"), unselected);
            assert_eq!(node(&runtime, "cut").is_checked(), None);
            assert_eq!(node(&runtime, "toggle").is_checked(), Some(false));
            let touch = bounds(&runtime, "toggle");
            assert_eq!((touch.width, touch.height), (48., 48.));
            assert_eq!(node(&runtime, "toggle").name(), "粗體");
            let Color(red, green, blue, _) = theme.colors.on_surface;
            assert_eq!(
                glyph_styles(&runtime, "disabled"),
                (unselected.0, Color(red, green, blue, 97))
            );
            let frame = runtime.accessibility_frame().unwrap();
            assert!(frame
                .activation_target(runtime.dom().by_id("toggle").unwrap())
                .is_some());
            assert!(frame
                .activation_target(runtime.dom().by_id("disabled").unwrap())
                .is_none());
            runtime
                .dispatch(Event::PointerDown {
                    x: touch.x + 1.,
                    y: touch.y + 24.,
                })
                .unwrap();
            runtime
                .dispatch(Event::PointerUp {
                    x: touch.x + 1.,
                    y: touch.y + 24.,
                })
                .unwrap();
            render(&mut runtime, 320.);
            assert_eq!(runtime.focused(), Some("toggle"));
            assert_eq!(node(&runtime, "toggle").is_checked(), Some(true));
            let selected = if palette == ToolbarPalette::Standard {
                (
                    theme.colors.secondary_container,
                    theme.colors.on_secondary_container,
                )
            } else {
                (theme.colors.surface_container, theme.colors.on_surface)
            };
            assert_eq!(glyph_styles(&runtime, "toggle"), selected);
        }
    }
}

#[test]
fn arrows_move_in_physical_direction_and_never_press_actions() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        for kind in [
            ToolbarKind::Docked,
            ToolbarKind::FloatingHorizontal,
            ToolbarKind::FloatingVertical,
        ] {
            let mut app = Demo::new(kind);
            app.direction = direction;
            let mut runtime = Runtime::new(app).unwrap();
            render(&mut runtime, 320.);
            tab(&mut runtime);
            assert_eq!(runtime.focused(), Some("body"));
            tab(&mut runtime);
            assert_eq!(runtime.focused(), Some("cut"));
            let forward = match (kind, direction) {
                (ToolbarKind::FloatingVertical, _) => EditKey::Down,
                (_, LayoutDirection::LeftToRight) => EditKey::Right,
                (_, LayoutDirection::RightToLeft) => EditKey::Left,
            };
            key(&mut runtime, forward);
            assert_eq!(runtime.focused(), Some("toggle"));
            assert!(!runtime.app().selected);
            key(&mut runtime, forward);
            assert_eq!(runtime.focused(), Some("more"));
            assert_eq!(runtime.app().presses, 0);
            let a = bounds(&runtime, "cut");
            let b = bounds(&runtime, "toggle");
            match forward {
                EditKey::Down => assert!(a.y < b.y),
                EditKey::Right => assert!(a.x < b.x),
                EditKey::Left => assert!(a.x > b.x),
                _ => unreachable!(),
            }
            runtime.dispatch(Event::Activate).unwrap();
            assert_eq!(runtime.app().presses, 1);
            tab(&mut runtime);
            assert_eq!(runtime.focused(), Some("outside"));
        }
    }
}

#[test]
fn collapse_keeps_core_or_fab_and_restores_focus_when_controls_disappear() {
    let collapse = || Message::SetVisibility(ToolbarVisibility::Collapsed);
    let hide = || Message::SetVisibility(ToolbarVisibility::Hidden);
    let mut bare = Runtime::new(Demo::new(ToolbarKind::FloatingVertical)).unwrap();
    bare.activate("more").unwrap();
    bare.apply(collapse()).unwrap();
    assert!(bare.dom().by_id("cut").is_none());
    assert!(bare.dom().by_id("more").is_none());
    assert_eq!(bare.focused(), Some("toggle"));
    bare.apply(hide()).unwrap();
    assert!(bare.dom().by_id("tools").is_none());
    assert!(bare.dom().by_id("tools-layout").is_some());
    assert_eq!(bare.focused(), Some("body"));

    let mut app = Demo::new(ToolbarKind::FloatingHorizontal);
    app.fab = true;
    let mut paired = Runtime::new(app).unwrap();
    render(&mut paired, 320.);
    let toolbar = bounds(&paired, "tools");
    let fab = bounds(&paired, "fab");
    assert_eq!(fab.x - (toolbar.x + toolbar.width), 8.);
    tab(&mut paired);
    tab(&mut paired);
    tab(&mut paired);
    assert_eq!(paired.focused(), Some("fab"));
    paired.apply(collapse()).unwrap();
    assert!(paired.dom().by_id("tools").is_none());
    assert_eq!(paired.focused(), Some("fab"));
    paired.apply(hide()).unwrap();
    assert!(paired.dom().by_id("fab").is_none());
    assert_eq!(paired.focused(), Some("body"));

    let mut docked = Runtime::new(Demo::new(ToolbarKind::Docked)).unwrap();
    docked.activate("more").unwrap();
    docked.apply(collapse()).unwrap();
    assert!(docked.dom().by_id("cut").is_some());
    assert_eq!(docked.focused(), Some("more"));
}

#[test]
fn constrained_horizontal_layout_wraps_controls_without_small_targets() {
    for kind in [ToolbarKind::Docked, ToolbarKind::FloatingHorizontal] {
        let mut app = Demo::new(kind);
        app.extra_actions = 3;
        app.fab = true;
        let mut runtime = Runtime::new(app).unwrap();
        render(&mut runtime, 320.);
        let area = bounds(&runtime, "tools");
        assert!(area.height > 64., "{kind:?} {area:?}");
        for id in [
            "cut", "toggle", "disabled", "more", "undo", "redo", "delete",
        ] {
            let target = bounds(&runtime, id);
            assert_eq!((target.width, target.height), (48., 48.));
            assert_eq!(
                area.intersect(target),
                target,
                "{kind:?} {id} {area:?} {target:?}"
            );
        }
        if kind == ToolbarKind::FloatingHorizontal {
            let fab = bounds(&runtime, "fab");
            assert!(fab.x + fab.width <= 320.);
        }
    }
}

#[test]
fn inset_floating_pair_fits_the_viewport_in_both_directions() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        for alignment in [Align::Start, Align::Center, Align::End] {
            let mut app = Demo::new(ToolbarKind::FloatingHorizontal);
            app.theme = Theme::dark();
            app.direction = direction;
            app.fab = true;
            app.extra_actions = 1;
            app.scale = 2.;
            app.floating_alignment = Some(alignment);
            let mut runtime = Runtime::new(app).unwrap();
            for width in [320., 560.] {
                runtime.render(width, 580., &mut Metrics).unwrap();
                let layer = bounds(&runtime, "inset-layer");
                let toolbar = bounds(&runtime, "tools");
                let fab = bounds(&runtime, "fab");
                assert_eq!(layer, Rect::new(16., 16., width - 32., 548.));
                for rect in [toolbar, fab] {
                    assert_eq!(
                        rect.intersect(layer),
                        rect,
                        "{direction:?} {alignment:?} at {width}: {rect:?} exceeds {layer:?}",
                    );
                }
                for id in ["cut", "toggle", "disabled", "more", "undo"] {
                    let target = bounds(&runtime, id);
                    assert_eq!((target.width, target.height), (48., 48.));
                    assert_eq!(toolbar.intersect(target), target);
                }
                assert_eq!((fab.width, fab.height), (56., 56.));
                let gap = if direction == LayoutDirection::RightToLeft {
                    toolbar.x - (fab.x + fab.width)
                } else {
                    fab.x - (toolbar.x + toolbar.width)
                };
                assert_eq!(gap, 8.);
                assert_eq!(toolbar.height > 64., width == 320.);
            }
        }
    }
}
