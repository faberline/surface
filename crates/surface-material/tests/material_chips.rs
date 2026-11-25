use surface::{
    Align, Color, ExtendedDisplayList, ExtendedPaintOp, GridRole, Node, PaintOp, RasterImage, Rect,
    Role, Style,
};
use surface_app::{
    AccessibilityActivationResult, Application, Event, Key, KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, ChipContent, ChipKind, ChipLeading, ChipOptions, ChipRemove, ChipStyle,
    ChipTrailing, ChoiceState, Theme, VectorIcon,
};
use surface_wgpu::TextSystem;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Body,
    Remove,
    Open,
    Next,
    Select(bool),
}

#[derive(Clone, Copy)]
enum Kind {
    Assist,
    Filter,
    Input,
    Suggestion,
    Legacy,
}

struct Model {
    kind: Kind,
    style: ChipStyle,
    selected: bool,
    disabled: bool,
    split: bool,
    compact_menu: bool,
    leading: bool,
    rtl: bool,
    dark: bool,
    enlarged: bool,
    max_width: f32,
    grid: bool,
    hide_on_remove: bool,
    requests: Vec<Action>,
    asset: RasterImage,
    icon: VectorIcon,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            kind: Kind::Assist,
            style: ChipStyle::Flat,
            selected: false,
            disabled: false,
            split: false,
            compact_menu: false,
            leading: false,
            rtl: false,
            dark: false,
            enlarged: false,
            max_width: 240.,
            grid: false,
            hide_on_remove: false,
            requests: vec![],
            asset: RasterImage::new(2, 1, [30, 20, 10, 255, 60, 50, 40, 50]).unwrap(),
            icon: VectorIcon::new([[3., 4.], [20., 4.], [13., 22.]]).unwrap(),
        }
    }
}

impl Model {
    fn theme(&self) -> Theme {
        if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        }
    }

    fn options(&self, max_width: f32) -> ChipOptions<'static> {
        ChipOptions {
            id: "chip",
            label: if self.enlarged {
                "將這個專案的訊息傳送給選定的常用聯絡人"
            } else {
                "Mina"
            },
            accessible_name: Some("聯絡人"),
            direction: if self.rtl {
                LayoutDirection::RightToLeft
            } else {
                LayoutDirection::LeftToRight
            },
            text_scale: if self.enlarged { 2. } else { 1. },
            max_width,
            disabled: self.disabled,
        }
    }

    fn content(&self) -> ChipContent<'_, Action> {
        let leading = self.leading.then_some(ChipLeading::Icon(&self.icon));
        let remove = ChipRemove {
            accessible_name: "移除聯絡人",
            message: Action::Remove,
        };
        match self.kind {
            Kind::Assist => ChipContent::Assist {
                style: self.style,
                leading,
                message: Action::Body,
            },
            Kind::Suggestion => ChipContent::Suggestion {
                style: self.style,
                leading,
                message: Action::Body,
            },
            Kind::Filter => ChipContent::Filter {
                style: self.style,
                leading,
                selected: self.selected,
                show_checkmark: true,
                message: Action::Body,
                trailing: if self.compact_menu {
                    Some(ChipTrailing::MenuOnBody {
                        expanded: self.selected,
                    })
                } else if self.split {
                    Some(ChipTrailing::Menu {
                        accessible_name: "更多選項",
                        expanded: self.selected,
                        message: Action::Open,
                    })
                } else {
                    None
                },
            },
            Kind::Input => ChipContent::Input {
                leading: self.leading.then_some(ChipLeading::Avatar(&self.asset)),
                selected: self.selected,
                body: self.split.then_some(Action::Body),
                remove,
            },
            Kind::Legacy => unreachable!(),
        }
    }
}

impl Application for Model {
    type Message = Action;

    fn update(&mut self, message: Action) {
        if let Action::Select(value) = message {
            self.selected = value;
        } else {
            self.requests.push(message);
        }
    }

    fn view(&self) -> View<Action> {
        let theme = self.theme();
        let chip = if self.hide_on_remove && self.requests.contains(&Action::Remove) {
            None
        } else if matches!(self.kind, Kind::Legacy) {
            Some(theme.chip(
                "chip",
                "Mina",
                ChipKind::Input,
                ChoiceState::default(),
                Action::Body,
                Some(Action::Remove),
            ))
        } else {
            let options = self.options(self.max_width);
            let chip = if self.grid {
                theme.chip_in_grid(options, self.content())
            } else {
                theme.chip_with_options(options, self.content())
            };
            chip.map(|view| {
                let view = view.restore_focus_in_subtree_on_removal("next");
                if self.grid {
                    let row = View::new(
                        Node::new(Role::Group, "")
                            .id("row")
                            .grid_role(GridRole::Row)
                            .styled(Style::row()),
                    )
                    .children([view]);
                    View::new(
                        Node::new(Role::Group, "Contacts")
                            .id("grid")
                            .grid_role(GridRole::Grid),
                    )
                    .children([row])
                } else {
                    view
                }
            })
        };
        View::new(
            Node::new(Role::Window, "Chips")
                .styled(Style::column().padding(20.).gap(12.).align(Align::Start)),
        )
        .children(
            chip.into_iter()
                .chain([theme.button("next", "Next", Action::Next)]),
        )
    }
}

fn bounds(runtime: &Runtime<Model>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn edge(rect: Rect) -> f32 {
    rect.x + rect.width
}

fn paint(runtime: &mut Runtime<Model>, text: &mut TextSystem) -> ExtendedDisplayList {
    runtime.render_extended(320., 480., text).unwrap()
}

fn shape(paint: &ExtendedDisplayList, area: Rect, ink: Color) -> bool {
    paint.ops().iter().any(|op| matches!(op, ExtendedPaintOp::Legacy(PaintOp::Shape { rect, color, .. } | PaintOp::Quad { rect, color, .. }) if *rect == area && *color == ink))
}

fn stroke(paint: &ExtendedDisplayList, area: Rect, ink: Color, thickness: f32) -> bool {
    paint.ops().iter().any(|op| matches!(op, ExtendedPaintOp::Legacy(PaintOp::ShapeStroke { rect, color, width, .. } | PaintOp::Stroke { rect, color, width, .. }) if *rect == area && *color == ink && *width == thickness))
}

fn tap(runtime: &mut Runtime<Model>, area: Rect, touch: bool) {
    let x = area.x + 1.;
    let y = area.y + 1.;
    for event in [Event::PointerDown { x, y }, Event::PointerUp { x, y }] {
        if touch {
            runtime.dispatch_touch(event).unwrap();
        } else {
            runtime.dispatch(event).unwrap();
        }
    }
}

#[test]
fn seven_variants_use_the_standard_face_ink_and_enabled_action() {
    let mut text = TextSystem::default();
    for (kind, style) in [
        (Kind::Assist, ChipStyle::Flat),
        (Kind::Assist, ChipStyle::Elevated),
        (Kind::Filter, ChipStyle::Flat),
        (Kind::Filter, ChipStyle::Elevated),
        (Kind::Input, ChipStyle::Flat),
        (Kind::Suggestion, ChipStyle::Flat),
        (Kind::Suggestion, ChipStyle::Elevated),
    ] {
        for dark in [false, true] {
            let mut runtime = Runtime::new(Model {
                kind,
                style,
                dark,
                leading: !matches!(kind, Kind::Input),
                ..Model::default()
            })
            .unwrap();
            let list = paint(&mut runtime, &mut text);
            let colors = runtime.app().theme().colors;
            let face = bounds(&runtime, "chip-face");
            let hit = bounds(&runtime, "chip");
            let label = bounds(&runtime, "chip-label");
            let frame = runtime.accessibility_frame().unwrap();
            let action = runtime.dom().by_id("chip").unwrap();
            assert_eq!((face.height, hit.height), (32., 48.));
            assert!(hit.width >= 48.);
            assert_eq!(face.y - hit.y, 8.);
            assert_eq!(frame.children(action).count(), 0);
            assert!(frame.activation_target(action).is_some());
            let action = frame.node(action).unwrap().node;
            assert_eq!(
                action.role(),
                if matches!(kind, Kind::Filter) {
                    Role::Checkbox
                } else {
                    Role::Button
                }
            );
            assert_eq!(
                action.is_checked(),
                matches!(kind, Kind::Filter).then_some(false)
            );
            assert_eq!(action.is_selected(), None);
            assert_eq!(
                action.name(),
                if matches!(kind, Kind::Input) {
                    "移除聯絡人"
                } else {
                    "聯絡人"
                }
            );
            if style == ChipStyle::Elevated {
                assert!(shape(&list, face, colors.surface_container_low));
            } else {
                assert!(stroke(&list, face, colors.outline_variant, 1.));
            }
            let text_ink = if matches!(kind, Kind::Assist) {
                colors.on_surface
            } else {
                colors.on_surface_variant
            };
            assert!(list.ops().iter().any(|op| match op {
                ExtendedPaintOp::AlignedText(text) => matches!(text.run(), PaintOp::StyledText { rect, style, color, .. }
                    if *rect == label && style.font_size == 14. && style.line_height == Some(20.) && style.weight == 500 && *color == text_ink),
                _ => false,
            }));
        }
    }
}

#[test]
fn compact_split_input_has_separate_mirrored_targets_and_scaled_text() {
    let mut text = TextSystem::default();
    for rtl in [false, true] {
        let mut runtime = Runtime::new(Model {
            kind: Kind::Input,
            split: true,
            selected: true,
            leading: true,
            dark: true,
            rtl,
            enlarged: true,
            ..Model::default()
        })
        .unwrap();
        let list = paint(&mut runtime, &mut text);
        let face = bounds(&runtime, "chip-face");
        let hit = bounds(&runtime, "chip");
        let remove = bounds(&runtime, "chip-remove");
        let icon = bounds(&runtime, "chip-remove-icon");
        let avatar = bounds(&runtime, "chip-leading");
        let label = bounds(&runtime, "chip-label");
        assert!(
            face.height > 32. && label.height >= 80.,
            "{face:?} {label:?}"
        );
        assert!(face.width >= 88. && face.width <= 240.);
        assert!(hit.width >= 48. && hit.height >= 48.);
        assert_eq!(remove.width, 48.);
        assert_eq!(remove.height, hit.height);
        assert_eq!(hit.intersect(remove).width, 0.);
        assert_eq!((avatar.width, avatar.height), (24., 24.));
        assert!(list.ops().iter().any(|op| matches!(op, ExtendedPaintOp::Raster(raster) if raster.rect() == avatar
            && raster.corners().uniform() == Some(12.) && raster.image().shares_pixels(&runtime.app().asset))));
        assert_eq!((icon.width, icon.height), (18., 18.));
        assert_eq!(icon.y + icon.height / 2., hit.y + hit.height / 2.);
        assert_eq!(label.intersect(hit), label);
        if rtl {
            assert_eq!(label.x - edge(icon), 8.);
            assert_eq!(icon.x - face.x, 8.);
            assert_eq!(edge(face) - edge(avatar), 4.);
        } else {
            assert_eq!(icon.x - edge(label), 8.);
            assert_eq!(edge(face) - edge(icon), 8.);
            assert_eq!(avatar.x - face.x, 4.);
        }
        assert!(shape(
            &list,
            face,
            runtime.app().theme().colors.secondary_container
        ));
        let dom = runtime.dom();
        let frame = runtime.accessibility_frame().unwrap();
        let body = dom.by_id("chip").unwrap();
        let close = dom.by_id("chip-remove").unwrap();
        assert_eq!(
            dom.get(body).unwrap().parent(),
            dom.get(close).unwrap().parent()
        );
        assert_eq!(frame.node(close).unwrap().node.name(), "移除聯絡人");
        assert!(
            frame.activation_target(body).is_some() && frame.activation_target(close).is_some()
        );
        let body_point = (hit.x + 1., hit.y + 1.);
        let close_point = (remove.x + 1., remove.y + 1.);
        runtime
            .dispatch(Event::PointerDown {
                x: body_point.0,
                y: body_point.1,
            })
            .unwrap();
        runtime
            .dispatch(Event::PointerUp {
                x: close_point.0,
                y: close_point.1,
            })
            .unwrap();
        assert!(runtime.app().requests.is_empty());
        tap(&mut runtime, hit, false);
        assert_eq!(runtime.focused(), Some("chip"));
        let focused = paint(&mut runtime, &mut text);
        let ring = bounds(&runtime, "chip-focus");
        assert!(stroke(
            &focused,
            ring,
            runtime.app().theme().colors.secondary,
            3.
        ));
        assert_eq!(ring.width - face.width, 10.);
        tap(&mut runtime, remove, true);
        assert_eq!(runtime.app().requests, [Action::Body, Action::Remove]);
        assert_eq!(runtime.focused(), Some("chip-remove"));
        let focused = paint(&mut runtime, &mut text);
        assert!(stroke(
            &focused,
            bounds(&runtime, "chip-remove-focus"),
            runtime.app().theme().colors.secondary,
            3.
        ));
    }
}

#[test]
fn focused_deletion_is_local_and_focus_recovers_after_controlled_removal() {
    let mut runtime = Runtime::new(Model {
        kind: Kind::Input,
        split: true,
        hide_on_remove: true,
        ..Model::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    paint(&mut runtime, &mut text);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("chip"));
    let key = |key, repeat| KeyPress {
        key,
        repeat,
        modifiers: Modifiers::default(),
    };
    assert!(!runtime.key(&key(Key::Delete, true)).unwrap());
    assert!(runtime.key(&key(Key::Backspace, false)).unwrap());
    assert_eq!(runtime.app().requests, [Action::Remove]);
    paint(&mut runtime, &mut text);
    assert_eq!(runtime.focused(), Some("next"));
    assert!(!runtime.key(&key(Key::Delete, false)).unwrap());
    assert_eq!(runtime.app().requests, [Action::Remove]);
}

#[test]
fn disabled_choice_and_menu_keep_their_semantics_without_activation() {
    let mut text = TextSystem::default();
    let mut runtime = Runtime::new(Model {
        kind: Kind::Filter,
        style: ChipStyle::Elevated,
        selected: true,
        disabled: true,
        split: true,
        ..Model::default()
    })
    .unwrap();
    let list = paint(&mut runtime, &mut text);
    let colors = runtime.app().theme().colors;
    assert!(shape(
        &list,
        bounds(&runtime, "chip-face"),
        Color(
            colors.on_surface.0,
            colors.on_surface.1,
            colors.on_surface.2,
            31
        )
    ));
    let frame = runtime.accessibility_frame().unwrap();
    for (key, role) in [("chip", Role::Checkbox), ("chip-menu", Role::Button)] {
        let id = runtime.dom().by_id(key).unwrap();
        let node = frame.node(id).unwrap().node;
        assert!(node.is_disabled());
        assert_eq!(node.role(), role);
        assert_eq!(
            node.expanded_state(),
            (role == Role::Button).then_some(true)
        );
        assert!(frame.activation_target(id).is_none());
    }
    for id in ["chip", "chip-menu"] {
        let hit = bounds(&runtime, id);
        tap(&mut runtime, hit, false);
    }
    assert!(runtime.app().requests.is_empty());
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("next"));
    let mut runtime = Runtime::new(Model {
        kind: Kind::Filter,
        compact_menu: true,
        ..Model::default()
    })
    .unwrap();
    paint(&mut runtime, &mut text);
    let body = runtime
        .dom()
        .get(runtime.dom().by_id("chip").unwrap())
        .unwrap();
    assert!(body.has_menu_popup());
    assert_eq!(body.expanded_state(), Some(false));
    assert!(runtime.dom().by_id("chip-menu").is_none());
}

#[test]
fn validated_width_boundaries_and_legacy_geometry() {
    let theme = Theme::light();
    let split = Model {
        kind: Kind::Input,
        split: true,
        ..Model::default()
    };
    assert!(theme
        .chip_with_options(split.options(101.9), split.content())
        .is_none());
    assert!(theme
        .chip_with_options(split.options(102.), split.content())
        .is_some());
    assert!(theme
        .chip_with_options(split.options(f32::INFINITY), split.content())
        .is_none());
    let mut boundary = Runtime::new(Model {
        max_width: 102.,
        ..split
    })
    .unwrap();
    paint(&mut boundary, &mut TextSystem::default());
    assert_eq!(bounds(&boundary, "chip-chip").width, 102.);
    assert_eq!(bounds(&boundary, "chip-face").width, 88.);
    assert_eq!(bounds(&boundary, "chip").width, 54.);
    assert_eq!(bounds(&boundary, "chip-remove").width, 48.);
    let mut runtime = Runtime::new(Model {
        kind: Kind::Legacy,
        ..Model::default()
    })
    .unwrap();
    paint(&mut runtime, &mut TextSystem::default());
    let dom = runtime.dom();
    let body = dom.by_id("chip").unwrap();
    let remove = dom.get(dom.by_id("chip-remove").unwrap()).unwrap();
    assert_eq!(dom.get(body).unwrap().is_selected(), Some(false));
    assert_eq!(remove.parent(), Some(body));
    assert_eq!(remove.name(), "Remove Mina");
    assert_eq!(bounds(&runtime, "chip-remove").width, 24.);
}

#[test]
fn grid_cells_wrap_each_action_without_changing_hits_or_face_ownership() {
    for rtl in [false, true] {
        let make = |grid| {
            Runtime::new(Model {
                kind: Kind::Input,
                split: true,
                leading: true,
                enlarged: true,
                rtl,
                grid,
                ..Model::default()
            })
            .unwrap()
        };
        let (mut grid, mut plain) = (make(true), make(false));
        let mut text = TextSystem::default();
        paint(&mut grid, &mut text);
        paint(&mut plain, &mut text);
        for action_id in ["chip", "chip-remove"] {
            let cell = format!("{action_id}-cell");
            let face = format!("{action_id}-face");
            let dom = grid.dom();
            let action = dom.by_id(action_id).unwrap();
            let cell_id = dom.by_id(&cell).unwrap();
            let frame = grid.accessibility_frame().unwrap();
            assert_eq!(
                frame.node(cell_id).unwrap().node.accessibility_grid_role(),
                Some(GridRole::GridCell)
            );
            assert_eq!(frame.children(cell_id).collect::<Vec<_>>(), [action]);
            assert_eq!(
                dom.get(dom.by_id(&face).unwrap()).unwrap().parent(),
                Some(action)
            );
            assert_eq!(bounds(&grid, action_id), bounds(&grid, &cell));
            assert_eq!(bounds(&grid, action_id), bounds(&plain, action_id));
            assert_eq!(bounds(&grid, &face), bounds(&plain, &face));
        }
    }
}

#[test]
fn accepted_ripples_follow_the_two_faces_and_respect_reduce_motion() {
    let mut runtime = Runtime::new(Model {
        kind: Kind::Input,
        split: true,
        selected: true,
        ..Model::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    let colors = runtime.app().theme().colors;
    let wave_color = Color(
        colors.on_secondary_container.0,
        colors.on_secondary_container.1,
        colors.on_secondary_container.2,
        26,
    );
    for (index, id) in ["chip", "chip-remove"].into_iter().enumerate() {
        let time = index as f64 * 200.;
        runtime.set_time(time);
        paint(&mut runtime, &mut text);
        let hit = bounds(&runtime, id);
        let face = bounds(&runtime, &format!("{id}-face"));
        let (x, y) = (hit.x + 1., hit.y + 1.);
        runtime.dispatch(Event::PointerDown { x, y }).unwrap();
        paint(&mut runtime, &mut text);
        runtime.set_time(time + 80.);
        let list = paint(&mut runtime, &mut text);
        let wave = list
            .ops()
            .iter()
            .filter_map(|op| match op {
                ExtendedPaintOp::FillPath(path) if path.color() == wave_color => Some(path),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(wave.len(), 1, "{id}");
        for &[x, y] in wave[0].points() {
            assert!(
                x >= face.x && x <= edge(face) && y >= face.y && y <= face.y + face.height,
                "{id} wave point ({x}, {y}) outside {face:?}"
            );
        }
        runtime.cancel_pointer().unwrap();
        assert!(runtime.app().requests.is_empty());
    }
    runtime.set_reduced_motion(true);
    let x = bounds(&runtime, "chip").x + 1.;
    let y = bounds(&runtime, "chip").y + 1.;
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    let list = paint(&mut runtime, &mut text);
    assert!(shape(&list, bounds(&runtime, "chip-face"), wave_color));
    assert!(!list
        .ops()
        .iter()
        .any(|op| matches!(op, ExtendedPaintOp::FillPath(path) if path.color() == wave_color)));
}

#[test]
fn filter_changes_only_when_authored_and_elevates_the_face_on_hover() {
    let mut runtime = Runtime::new(Model {
        kind: Kind::Filter,
        style: ChipStyle::Elevated,
        ..Model::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    let idle = paint(&mut runtime, &mut text);
    let colors = runtime.app().theme().colors;
    let face = bounds(&runtime, "chip-face");
    let hit = bounds(&runtime, "chip");
    let x = hit.x + 2.;
    let y = hit.y + 2.;
    let shadow = colors.shadow;
    let shadow_edge = |list: &ExtendedDisplayList| {
        list.ops()
            .iter()
            .filter_map(|op| match op {
                ExtendedPaintOp::Legacy(
                    PaintOp::Quad { rect, color, .. } | PaintOp::Stroke { rect, color, .. },
                ) if color.0 == shadow.0
                    && color.1 == shadow.1
                    && color.2 == shadow.2
                    && rect.y < face.y + face.height =>
                {
                    Some(rect.x)
                }
                _ => None,
            })
            .reduce(f32::min)
            .unwrap()
    };
    runtime.dispatch(Event::PointerMove { x, y }).unwrap();
    let hover = paint(&mut runtime, &mut text);
    assert!(shadow_edge(&hover) < shadow_edge(&idle));
    let with_alpha = |c: Color, a| Color(c.0, c.1, c.2, a);
    assert!(shape(
        &hover,
        face,
        with_alpha(colors.on_surface_variant, 20)
    ));
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    let press = paint(&mut runtime, &mut text);
    assert_eq!(shadow_edge(&press), shadow_edge(&idle));
    assert!(shape(
        &press,
        face,
        with_alpha(colors.on_secondary_container, 26)
    ));
    runtime.cancel_pointer().unwrap();
    paint(&mut runtime, &mut text);
    assert!(runtime.app().requests.is_empty());
    let key = "chip";
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(runtime.dom().by_id(key).unwrap())
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert_eq!(runtime.app().requests, [Action::Body]);
    let checked = |runtime: &Runtime<Model>| {
        runtime
            .dom()
            .get(runtime.dom().by_id(key).unwrap())
            .unwrap()
            .is_checked()
    };
    assert_eq!(checked(&runtime), Some(false));
    runtime.apply(Action::Select(true)).unwrap();
    let selected = paint(&mut runtime, &mut text);
    assert!(shape(
        &selected,
        bounds(&runtime, "chip-face"),
        colors.secondary_container
    ));
    assert_eq!(checked(&runtime), Some(true));
    assert_eq!(bounds(&runtime, "chip-leading").width, 18.);
}
