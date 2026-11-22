use surface::{Color, ExtendedPaintOp, Insets, Length, Node, NodeId, PaintOp, Rect, Role, Style};
use surface_app::{
    AccessibilityActivationResult, AccessibilityScrollDirection, AccessibilityScrollResult,
    AnimationFrame, AnimationUpdate, Application, EditKey, Event, HorizontalDrag, Runtime, View,
};
use surface_material::{
    shape::{self, LayoutDirection},
    typography, ChoiceState, Elevation, ModalNavigationDrawerOptions, NavigationDrawerItem,
    NavigationDrawerSwipe, Theme, VectorIcon,
};
use surface_wgpu::TextSystem;

const DIALOG: &str = "drawer";
const HEADER: &str = "drawer-header";
const CLOSE: &str = "drawer-close";
const NAVIGATION: &str = "drawer-navigation";
const OPENER: &str = "opener";
const LONG_LABEL: &str = "使用者的私人專案與協作成員設定";
const DESTINATIONS: [(&str, &str, &str); 5] = [
    ("home", "首頁", "前往首頁"),
    ("archive", "封存", ""),
    ("files", "檔案", "檔案清單"),
    ("settings", LONG_LABEL, LONG_LABEL),
    ("history", "歷程", "瀏覽歷程"),
];

#[derive(Clone)]
enum Message {
    Open,
    Dismiss,
    Select(usize),
    Direction(LayoutDirection),
    Background,
    Drag(HorizontalDrag),
}

struct Drawer {
    theme: Theme,
    direction: LayoutDirection,
    scale: f32,
    icon: VectorIcon,
    open: bool,
    selected: usize,
    presses: usize,
    background_presses: usize,
    swipe: Option<NavigationDrawerSwipe>,
    swipe_commits: Vec<bool>,
    dismissals: usize,
}

impl Drawer {
    fn new(theme: Theme, direction: LayoutDirection, scale: f32) -> Self {
        Self {
            theme,
            direction,
            scale,
            icon: VectorIcon::new([[0., 0.], [24., 0.], [12., 24.]]).unwrap(),
            open: false,
            selected: 0,
            presses: 0,
            background_presses: 0,
            swipe: None,
            swipe_commits: Vec::new(),
            dismissals: 0,
        }
    }
}

impl Application for Drawer {
    type Message = Message;

    fn update(&mut self, event: Message) {
        match event {
            Message::Open => {
                self.swipe.iter_mut().for_each(NavigationDrawerSwipe::reset);
                self.open = true;
            }
            Message::Dismiss => {
                self.swipe.iter_mut().for_each(NavigationDrawerSwipe::reset);
                self.open = false;
                self.dismissals += 1;
            }
            Message::Select(index) => {
                self.selected = index;
                self.presses += 1;
            }
            Message::Direction(direction) => {
                self.swipe.iter_mut().for_each(NavigationDrawerSwipe::reset);
                self.direction = direction;
            }
            Message::Background => self.background_presses += 1,
            Message::Drag(drag) => {
                self.swipe
                    .as_mut()
                    .unwrap()
                    .apply(drag, self.open, self.direction);
            }
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        self.swipe
            .as_mut()
            .is_some_and(|swipe| swipe.set_viewport(width, height))
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        let Some(swipe) = &mut self.swipe else {
            return AnimationUpdate::default();
        };
        let (update, commit) = swipe.advance(frame.now_ms, frame.reduced_motion);
        if let Some(open) = commit {
            self.open = open;
            self.swipe_commits.push(open);
        }
        update
    }

    fn view(&self) -> View<Message> {
        let opener = self
            .theme
            .button(OPENER, "開啟導覽", Message::Open)
            .accessible_activation();
        let backdrop_control = View::new(
            Node::new(Role::Button, "背景操作").id("background").styled(
                Style::row()
                    .width(Length::Px(80.))
                    .height(Length::Px(80.))
                    .fixed(Insets {
                        top: Length::Px(160.),
                        right: Length::Px(0.),
                        ..Insets::default()
                    }),
            ),
        )
        .on_press(Message::Background)
        .accessible_activation();
        let options = ModalNavigationDrawerOptions {
            id: DIALOG,
            name: "導覽抽屜",
            navigation_name: "主要導覽",
            close_name: "關閉導覽",
            direction: self.direction,
        };
        let items =
            DESTINATIONS
                .into_iter()
                .enumerate()
                .map(|(index, (id, label, accessible_name))| {
                    self.theme
                        .navigation_drawer_item_with_icon(NavigationDrawerItem {
                            id,
                            label,
                            accessible_name,
                            icon: &self.icon,
                            direction: self.direction,
                            state: ChoiceState {
                                selected: self.selected == index,
                                disabled: index == 1,
                            },
                            message: Message::Select(index),
                        })
                });
        let drawer: Vec<_> = if let Some(swipe) = &self.swipe {
            self.theme
                .swipeable_modal_navigation_drawer(
                    options,
                    items,
                    Message::Dismiss,
                    swipe,
                    self.open,
                    Message::Drag,
                )
                .collect()
        } else if self.open {
            vec![self.theme.modal_navigation_drawer_with_direction(
                options,
                items,
                Message::Dismiss,
            )]
        } else {
            Vec::new()
        };
        self.theme
            .page([opener, backdrop_control].into_iter().chain(drawer))
    }
}

fn node_id(runtime: &Runtime<Drawer>, id: &str) -> NodeId {
    runtime.dom().by_id(id).expect("the named node exists")
}

fn rect(runtime: &Runtime<Drawer>, node: NodeId) -> Rect {
    runtime.computed().unwrap().nodes[node.index()].bounds
}

fn bounds(runtime: &Runtime<Drawer>, id: &str) -> Rect {
    rect(runtime, node_id(runtime, id))
}

fn press_at(runtime: &mut Runtime<Drawer>, x: f32, y: f32) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
}

#[test]
fn drawer_mirrors_the_sheet_and_icon_while_exposing_one_named_action() {
    let mut fonts = TextSystem::default();
    for theme in [Theme::light(), Theme::dark()] {
        for (direction, left) in [
            (LayoutDirection::LeftToRight, true),
            (LayoutDirection::RightToLeft, false),
        ] {
            let mut runtime = Runtime::new(Drawer::new(theme, direction, 1.)).unwrap();
            runtime.render(800., 480., &mut fonts).unwrap();
            runtime.activate(OPENER).unwrap();
            let paint = runtime.render_extended(800., 480., &mut fonts).unwrap();
            assert_eq!(runtime.focused(), Some(CLOSE));
            let dialog = node_id(&runtime, DIALOG);
            let navigation = node_id(&runtime, NAVIGATION);
            let home = node_id(&runtime, "home");
            let archive = node_id(&runtime, "archive");
            let home_area = rect(&runtime, home);
            assert_eq!((home_area.width, home_area.height), (336., 56.));
            assert_eq!(
                rect(&runtime, dialog),
                Rect::new(if left { 0. } else { 440. }, 0., 360., 480.)
            );
            let header = bounds(&runtime, HEADER);
            let body = rect(&runtime, navigation);
            assert_eq!(header.y + header.height, body.y);
            assert_eq!(body.y + body.height, 480.);

            let panel = runtime.dom().get(dialog).unwrap();
            let style = panel.style();
            assert_eq!(style.corners, Some(shape::large_end(direction)));
            assert_eq!(style.background, theme.colors.surface_container_low);
            assert_eq!(
                panel.box_shadow_layers(),
                theme.elevation(Elevation::Level1)
            );
            let scrim = runtime
                .dom()
                .get(node_id(&runtime, "drawer-layer"))
                .unwrap();
            let Color(red, green, blue, _) = theme.colors.scrim;
            assert_eq!(scrim.style().background, Color(red, green, blue, 82));
            assert_eq!(
                runtime.dom().get(home).unwrap().style().background,
                theme.colors.secondary_container
            );
            let (icon, label) = {
                let mut children = runtime.dom().children(home);
                (
                    rect(&runtime, children.next().unwrap()),
                    rect(&runtime, children.next().unwrap()),
                )
            };
            assert_eq!((icon.width, icon.height), (24., 24.));
            if left {
                assert!(
                    icon.x < label.x
                        && bounds(&runtime, CLOSE).x < bounds(&runtime, "drawer-title").x
                );
            } else {
                assert!(
                    label.x < icon.x
                        && bounds(&runtime, "drawer-title").x < bounds(&runtime, CLOSE).x
                );
            }
            assert!(paint.ops().iter().any(|op| matches!(
                op, ExtendedPaintOp::FillPath(path) if path.clip() == icon
                    && path.color() == theme.colors.on_secondary_container
            )));

            let frame = runtime.accessibility_frame().unwrap();
            assert_eq!(frame.modal_root(), Some(dialog));
            let close = node_id(&runtime, CLOSE);
            assert_eq!(frame.node(close).unwrap().node.name(), "關閉導覽");
            assert!(frame.children(close).next().is_none());
            assert!(paint.ops().iter().any(|op| matches!(
                op, ExtendedPaintOp::FillPath(path)
                    if path.color() == theme.colors.on_surface_variant
                    && path.rect().intersect(rect(&runtime, close)) == path.rect()
            )));
            assert_eq!(frame.node(dialog).unwrap().node.role(), Role::Dialog);
            assert_eq!(frame.node(dialog).unwrap().node.name(), "導覽抽屜");
            assert_eq!(
                frame.node(navigation).unwrap().node.role(),
                Role::Navigation
            );
            assert_eq!(frame.node(navigation).unwrap().node.name(), "主要導覽");
            assert!(frame.node(node_id(&runtime, OPENER)).is_none());
            assert_eq!(frame.node(home).unwrap().node.name(), "前往首頁");
            assert_eq!(frame.node(home).unwrap().node.is_selected(), Some(true));
            assert_eq!(frame.node(home).unwrap().node.is_current_page(), Some(true));
            assert!(frame.activation_target(home).is_some());
            assert!(frame.children(home).next().is_none());
            assert_eq!(frame.node(archive).unwrap().node.name(), "封存");
            assert_eq!(
                frame.node(archive).unwrap().node.is_current_page(),
                Some(false)
            );
            assert!(frame.node(archive).unwrap().node.is_disabled());
            assert!(frame.activation_target(archive).is_none());
            assert!(frame.scroll_target(archive).is_none());

            // At this size the opposite edge is uncovered by the panel, but the
            // scrim consumes the press instead of activating the background.
            press_at(&mut runtime, if left { 760. } else { 40. }, 200.);
            assert!(!runtime.app().open);
            assert_eq!(runtime.focused(), Some(OPENER));
            assert_eq!(runtime.app().background_presses, 0);
        }
    }
}

#[test]
fn compact_drawer_scrolls_a_wrapped_label_without_moving_close_or_losing_focus() {
    let mut runtime =
        Runtime::new(Drawer::new(Theme::dark(), LayoutDirection::LeftToRight, 2.)).unwrap();
    let mut fonts = TextSystem::default();
    runtime.render(320., 240., &mut fonts).unwrap();
    runtime.activate(OPENER).unwrap();
    runtime.render(320., 240., &mut fonts).unwrap();
    assert_eq!(bounds(&runtime, DIALOG), Rect::new(0., 0., 320., 240.));
    assert_eq!(runtime.focused(), Some(CLOSE));
    for expected in ["home", CLOSE, "home"] {
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some(expected));
    }
    runtime.render(320., 240., &mut fonts).unwrap();
    let navigation = node_id(&runtime, NAVIGATION);
    let page = runtime
        .accessibility_frame()
        .unwrap()
        .scroll_page_target(navigation, AccessibilityScrollDirection::Down)
        .unwrap();
    assert_eq!(
        runtime.accessibility_scroll_page(&page).unwrap(),
        AccessibilityScrollResult::Scheduled
    );
    runtime.render(320., 240., &mut fonts).unwrap();
    assert!(runtime.scroll_metrics(NAVIGATION).unwrap().offset.1 > 0.);
    assert_eq!(runtime.focused(), Some("home"));

    for id in ["files", "settings"] {
        runtime
            .dispatch(Event::Edit {
                key: EditKey::Down,
                extend: false,
                by_word: false,
            })
            .unwrap();
        runtime.render(320., 240., &mut fonts).unwrap();
        assert_eq!(runtime.focused(), Some(id));
    }
    assert_eq!(runtime.app().selected, 0);
    assert_eq!(runtime.app().presses, 0);
    runtime
        .apply(Message::Direction(LayoutDirection::RightToLeft))
        .unwrap();
    let paint = runtime.render(320., 240., &mut fonts).unwrap();
    let header = bounds(&runtime, HEADER);
    let close = bounds(&runtime, CLOSE);
    assert_eq!(runtime.focused(), Some("settings"));
    assert_eq!(header.y, 0.);
    assert_eq!(close.x + close.width, 308.);
    let (label, clip, style) = paint
        .ops
        .iter()
        .find_map(|op| match op {
            PaintOp::StyledText {
                text,
                rect,
                clip,
                style,
                ..
            } if text == LONG_LABEL => Some((*rect, *clip, style)),
            _ => None,
        })
        .expect("the wrapped destination paints");
    assert_eq!(style, &typography::LABEL_LARGE.scaled(2.));
    assert!(label.height >= style.line_height_px() * 2.);
    assert_eq!(label.intersect(clip), label);
    assert_eq!(label.intersect(bounds(&runtime, "settings")), label);
    press_at(
        &mut runtime,
        label.x + label.width / 2.,
        label.y + label.height - 2.,
    );
    assert_eq!(runtime.app().selected, 3);
    assert_eq!(runtime.app().presses, 1);
    runtime.render(320., 240., &mut fonts).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    for (id, current) in [("home", false), ("settings", true)] {
        assert_eq!(
            frame
                .node(node_id(&runtime, id))
                .unwrap()
                .node
                .is_current_page(),
            Some(current)
        );
    }
    assert_eq!(bounds(&runtime, HEADER), header);
    assert_eq!(bounds(&runtime, CLOSE), close);

    runtime.dispatch(Event::Cancel).unwrap();
    assert!(!runtime.app().open);
    assert_eq!(runtime.focused(), Some(OPENER));
    runtime.render(320., 240., &mut fonts).unwrap();
    runtime.activate(OPENER).unwrap();
    runtime.render(320., 240., &mut fonts).unwrap();
    assert_eq!(runtime.focused(), Some(CLOSE));
    let close_action = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(node_id(&runtime, CLOSE))
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&close_action).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert!(!runtime.app().open);
    assert_eq!(runtime.focused(), Some(OPENER));
}

#[test]
fn edge_drag_previews_in_both_directions_before_committing_focus_once() {
    let mut fonts = TextSystem::default();
    for (direction, viewport_width, x, sign) in [
        (LayoutDirection::LeftToRight, 800., 12., 1.),
        (LayoutDirection::RightToLeft, 320., 308., -1.),
    ] {
        let mut drawer = Drawer::new(Theme::light(), direction, 1.);
        drawer.swipe = Some(NavigationDrawerSwipe::default());
        let mut runtime = Runtime::new(drawer).unwrap();
        let y = 210.;
        let render = |runtime: &mut Runtime<Drawer>, fonts: &mut TextSystem, now| {
            runtime.set_time(now);
            runtime.render(viewport_width, 480., fonts).unwrap();
        };

        render(&mut runtime, &mut fonts, 0.);
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some(OPENER));
        assert_eq!(bounds(&runtime, "drawer-swipe-edge").width, 24.);
        assert!(runtime.dom().by_id(DIALOG).is_none());
        runtime.dispatch(Event::PointerDown { x, y }).unwrap();
        runtime.set_time(150.);
        runtime
            .dispatch(Event::PointerMove {
                x: x + sign * 90.,
                y,
            })
            .unwrap();
        render(&mut runtime, &mut fonts, 150.);

        let preview = bounds(&runtime, DIALOG);
        assert!(
            (preview
                .intersect(Rect::new(0., 0., viewport_width, 480.))
                .width
                - 90.)
                .abs()
                < 0.01
        );
        let active_width = viewport_width.min(360.);
        let expected_alpha = (82. * 90. / active_width).round() as u8;
        assert_eq!(
            runtime
                .dom()
                .get(node_id(&runtime, "drawer-layer"))
                .unwrap()
                .style()
                .background
                .3,
            expected_alpha
        );
        let frame = runtime.accessibility_frame().unwrap();
        assert!(frame.modal_root().is_none());
        assert!(frame.node(node_id(&runtime, DIALOG)).is_none());
        assert!(frame.node(node_id(&runtime, OPENER)).is_some());
        assert_eq!(runtime.focused(), Some(OPENER));

        runtime.set_time(300.);
        runtime
            .dispatch(Event::PointerUp {
                x: x + sign * active_width / 2.,
                y,
            })
            .unwrap();
        render(&mut runtime, &mut fonts, 300.);
        assert!(runtime.needs_frame());
        render(&mut runtime, &mut fonts, 555.);
        assert!(!runtime.app().open);
        assert_eq!(runtime.focused(), Some(OPENER));
        render(&mut runtime, &mut fonts, 556.);
        render(&mut runtime, &mut fonts, 700.);
        assert_eq!(runtime.app().swipe_commits, [true]);
        assert_eq!(runtime.focused(), Some(CLOSE));
        assert_eq!(
            runtime.accessibility_frame().unwrap().modal_root(),
            Some(node_id(&runtime, DIALOG))
        );
        assert!(runtime.dom().by_id("drawer-swipe-edge").is_none());
    }
}

#[test]
fn canceled_vertical_and_outside_edge_drags_leave_the_drawer_closed() {
    let mut drawer = Drawer::new(Theme::light(), LayoutDirection::LeftToRight, 1.);
    let mut fonts = TextSystem::default();
    let mut static_runtime =
        Runtime::new(Drawer::new(Theme::light(), drawer.direction, 1.)).unwrap();
    static_runtime.render(800., 480., &mut fonts).unwrap();
    assert!(static_runtime.dom().by_id("drawer-swipe-edge").is_none());

    drawer.swipe = Some(NavigationDrawerSwipe::default());
    let mut runtime = Runtime::new(drawer).unwrap();
    runtime.render(800., 480., &mut fonts).unwrap();
    for (start_x, move_x, move_y) in [(26., 210., 210.), (12., 14., 250.)] {
        runtime
            .dispatch(Event::PointerDown {
                x: start_x,
                y: 210.,
            })
            .unwrap();
        runtime
            .dispatch(Event::PointerMove {
                x: move_x,
                y: move_y,
            })
            .unwrap();
        runtime
            .dispatch(Event::PointerUp { x: 225., y: move_y })
            .unwrap();
        runtime.render(800., 480., &mut fonts).unwrap();
        assert!(runtime.dom().by_id(DIALOG).is_none());
    }
    for resize in [false, true] {
        runtime
            .dispatch(Event::PointerDown { x: 12., y: 210. })
            .unwrap();
        runtime
            .dispatch(Event::PointerMove { x: 225., y: 210. })
            .unwrap();
        runtime.render(800., 480., &mut fonts).unwrap();
        assert!(runtime.dom().by_id(DIALOG).is_some());
        if resize {
            runtime.render(320., 480., &mut fonts).unwrap();
        } else {
            runtime.cancel_pointer().unwrap();
        }
        runtime
            .dispatch(Event::PointerUp { x: 225., y: 210. })
            .unwrap();
        runtime
            .render(if resize { 320. } else { 800. }, 480., &mut fonts)
            .unwrap();
        assert!(runtime.dom().by_id(DIALOG).is_none());
        assert!(runtime.app().swipe_commits.is_empty());
        assert_eq!(runtime.app().presses, 0);
    }
}

#[test]
fn short_fling_closes_with_reduce_motion_and_restores_the_opener() {
    let mut drawer = Drawer::new(Theme::dark(), LayoutDirection::RightToLeft, 1.);
    drawer.swipe = Some(NavigationDrawerSwipe::default());
    let mut runtime = Runtime::new(drawer).unwrap();
    let mut fonts = TextSystem::default();
    runtime.render(800., 480., &mut fonts).unwrap();
    runtime.activate(OPENER).unwrap();
    runtime.render(800., 480., &mut fonts).unwrap();
    assert_eq!(runtime.focused(), Some(CLOSE));
    let home = bounds(&runtime, "home");
    let x = home.x + 60.;
    let y = home.y + home.height / 2.;

    // A slow drag below the threshold settles back to the current endpoint.
    runtime.set_time(0.);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.set_time(200.);
    runtime
        .dispatch(Event::PointerUp { x: x + 20., y })
        .unwrap();
    runtime.render(800., 480., &mut fonts).unwrap();
    assert!(runtime.needs_frame());
    assert_eq!(runtime.focused(), Some(CLOSE));
    assert_eq!(
        runtime.accessibility_frame().unwrap().modal_root(),
        Some(node_id(&runtime, DIALOG))
    );
    runtime.set_time(456.);
    runtime.render(800., 480., &mut fonts).unwrap();
    assert!(runtime.app().swipe_commits.is_empty());
    assert!(runtime.app().open);

    // The same displacement over 50 ms reaches 400 logical pixels per second.
    runtime.set_reduced_motion(true);
    runtime.set_time(500.);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.set_time(550.);
    runtime
        .dispatch(Event::PointerUp { x: x + 20., y })
        .unwrap();
    runtime.render(800., 480., &mut fonts).unwrap();
    assert!(!runtime.needs_frame());
    assert_eq!(runtime.focused(), Some(OPENER));
    assert!(runtime.dom().by_id(DIALOG).is_none());
    assert_eq!(runtime.app().swipe_commits, [false]);
    assert_eq!(runtime.app().dismissals, 0);
    assert_eq!(runtime.app().presses, 0);
}
