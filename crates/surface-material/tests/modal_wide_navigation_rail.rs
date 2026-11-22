use surface::{Align, Color, ExtendedPaintOp, Length, Node, PaintOp, Position, Rect, Role, Style};
use surface_app::{
    AccessibilityScrollDirection, AccessibilityScrollResult, AnimationFrame, AnimationUpdate,
    Application, EditKey, Event, HorizontalDrag, Runtime, View,
};
use surface_material::{
    shape::{self, LayoutDirection},
    ChoiceState, Elevation, ModalWideNavigationRailCollapse, ModalWideNavigationRailOptions,
    ModalWideNavigationRailSwipeOptions, NavigationRailItem, Theme, VectorIcon,
    WideNavigationRailMode, WideNavigationRailMotion, WideNavigationRailMotionKind,
    WideNavigationRailSwipe,
};
use surface_wgpu::TextSystem;

const OPEN: &str = "open";
const PAGE: &str = "page";
const INLINE: &str = "inline-rail";
const DIALOG: &str = "modal-rail";
const LAYER: &str = "modal-rail-layer";
const HEADER: &str = "modal-rail-header";
const CLOSE: &str = "modal-rail-close";
const NAVIGATION: &str = "modal-rail-navigation";
const TITLE: &str = "應用程式導覽";
const DESTINATIONS: &str = "主要目的地";
const CLOSE_NAME: &str = "關閉導覽列";
const LONG_LABEL: &str = "跨部門協作報告、專案進度與歷史紀錄總覽";
const INBOX_LABEL: &str = "跨部門協作與專案通知收件匣";

#[derive(Clone, Copy)]
enum Message {
    Open,
    Close,
    Background,
    Select(usize),
    Drag(HorizontalDrag, f32),
}

struct Rail {
    theme: Theme,
    direction: LayoutDirection,
    retain_inline: bool,
    expanded: bool,
    labels: [&'static str; 5],
    text_scale: f32,
    selected: usize,
    dismissals: usize,
    background_presses: usize,
    motion: Option<WideNavigationRailMotion>,
    swipe: Option<WideNavigationRailSwipe>,
}

impl Rail {
    fn new(direction: LayoutDirection, retain_inline: bool) -> Self {
        Self {
            theme: if direction == LayoutDirection::LeftToRight {
                Theme::light()
            } else {
                Theme::dark()
            },
            direction,
            retain_inline,
            expanded: false,
            labels: ["首頁", "封存", "收件匣", "設定", "說明"],
            text_scale: 1.,
            selected: 0,
            dismissals: 0,
            background_presses: 0,
            motion: None,
            swipe: None,
        }
    }

    fn opener(&self) -> View<Message> {
        View::new(
            Node::new(Role::Button, "開啟導覽列")
                .id(OPEN)
                .disclosure_expanded(self.expanded)
                .styled(Style::row().width(Length::Px(48.)).height(Length::Px(48.))),
        )
        .on_press(Message::Open)
        .accessible_activation()
    }

    fn destinations(
        &self,
        mode: WideNavigationRailMode,
    ) -> impl Iterator<Item = View<Message>> + '_ {
        let ids = match mode {
            WideNavigationRailMode::Collapsed => [
                "inline-home",
                "inline-archive",
                "inline-inbox",
                "inline-settings",
                "inline-help",
            ],
            WideNavigationRailMode::Expanded => ["home", "archive", "inbox", "settings", "help"],
        };
        let icon = VectorIcon::new([[2., 3.], [22., 3.], [12., 21.]]).unwrap();
        ids.into_iter()
            .zip(self.labels)
            .enumerate()
            .map(move |(index, (id, label))| {
                self.theme.wide_navigation_rail_item_with_icon(
                    mode,
                    self.direction,
                    NavigationRailItem {
                        id,
                        label,
                        accessible_name: label,
                        icon: &icon,
                        state: ChoiceState {
                            selected: self.selected == index,
                            disabled: index == 1,
                        },
                        message: Message::Select(index),
                    },
                )
            })
    }
}

impl Application for Rail {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.expanded = true,
            Message::Close => {
                if let Some(swipe) = &mut self.swipe {
                    swipe.reset();
                }
                self.expanded = false;
                self.dismissals += 1;
            }
            Message::Background => self.background_presses += 1,
            Message::Select(index) => self.selected = index,
            Message::Drag(drag, width) => {
                self.swipe
                    .as_mut()
                    .unwrap()
                    .apply(drag, width, self.direction)
            }
        }
    }

    fn text_scale(&self) -> f32 {
        self.text_scale
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        let mut animation = self
            .motion
            .as_mut()
            .map_or(AnimationUpdate::default(), |motion| {
                motion.advance(self.expanded, frame.now_ms, frame.reduced_motion)
            });
        if let Some(swipe) = &mut self.swipe {
            let (update, dismiss) = swipe.advance(frame.now_ms, frame.reduced_motion);
            if dismiss {
                self.expanded = false;
                self.dismissals += 1;
                self.motion.as_mut().unwrap().snap(false);
            }
            animation.rebuild |= update.rebuild;
            animation.request_next |= update.request_next;
        }
        animation
    }

    fn view(&self) -> View<Message> {
        let background = View::new(
            Node::new(Role::Button, "內容").id("background").styled(
                Style::column()
                    .height(Length::Px(360.))
                    .width(Length::Percent(1.)),
            ),
        )
        .on_press(Message::Background);
        let page = View::new(
            Node::new(Role::Group, "內容")
                .id(PAGE)
                .styled(Style::column().width(Length::Px(0.)).grow(1.).clip(true)),
        )
        .children(
            (!self.retain_inline)
                .then(|| self.opener())
                .into_iter()
                .chain([background]),
        );
        let mut row = Node::new(Role::Group, "").styled(
            Style::row()
                .align(Align::Stretch)
                .height(Length::Px(0.))
                .grow(1.),
        );
        if self.direction == LayoutDirection::RightToLeft {
            row = row.reverse_row();
        }
        let inline = self.retain_inline.then(|| {
            self.theme.wide_navigation_rail(
                INLINE,
                DESTINATIONS,
                WideNavigationRailMode::Collapsed,
                Some(self.opener()),
                self.destinations(WideNavigationRailMode::Collapsed),
            )
        });
        let row = View::new(row).children(inline.into_iter().chain([page]));
        let options = ModalWideNavigationRailOptions {
            id: DIALOG,
            name: TITLE,
            navigation_name: DESTINATIONS,
            close_name: CLOSE_NAME,
            direction: self.direction,
        };
        let modal = if let (Some(swipe), Some(motion)) = (&self.swipe, &self.motion) {
            self.theme.swipeable_modal_wide_navigation_rail(
                options,
                self.destinations(WideNavigationRailMode::Expanded),
                Message::Close,
                ModalWideNavigationRailSwipeOptions {
                    open: self.expanded,
                    frame: motion.frame(),
                    swipe,
                },
                Message::Drag,
            )
        } else if let Some(motion) = &self.motion {
            self.theme.animated_modal_wide_navigation_rail(
                options,
                self.destinations(WideNavigationRailMode::Expanded),
                Message::Close,
                self.expanded,
                if self.retain_inline {
                    ModalWideNavigationRailCollapse::Retain
                } else {
                    ModalWideNavigationRailCollapse::Hide {
                        viewport_width: 960.,
                    }
                },
                motion.frame(),
            )
        } else {
            self.expanded.then(|| {
                self.theme.modal_wide_navigation_rail(
                    options,
                    self.destinations(WideNavigationRailMode::Expanded),
                    Message::Close,
                )
            })
        };
        View::new(Node::new(Role::Window, "Window").styled(Style::column()))
            .children([row].into_iter().chain(modal))
    }
}

fn node<'a>(runtime: &'a Runtime<Rail>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn bounds(runtime: &Runtime<Rail>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

#[test]
fn modal_keeps_the_page_and_optional_inline_rail_in_place() {
    let mut fonts = TextSystem::default();
    for retain_inline in [false, true] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let mut runtime = Runtime::new(Rail::new(direction, retain_inline)).unwrap();
            runtime.render(960., 640., &mut fonts).unwrap();
            let page = bounds(&runtime, PAGE);
            let inline = retain_inline.then(|| bounds(&runtime, INLINE));
            assert_eq!(page.width, if retain_inline { 864. } else { 960. });
            assert_eq!(runtime.dom().by_id(INLINE).is_some(), retain_inline);
            assert!(runtime.dom().by_id(DIALOG).is_none());

            runtime.activate(OPEN).unwrap();
            runtime.render(960., 640., &mut fonts).unwrap();
            assert_eq!(bounds(&runtime, PAGE), page);
            assert_eq!(retain_inline.then(|| bounds(&runtime, INLINE)), inline);
            let panel = bounds(&runtime, DIALOG);
            assert_eq!((panel.y, panel.width, panel.height), (0., 220., 640.));
            let close = bounds(&runtime, CLOSE);
            assert_eq!((close.width, close.height), (48., 48.));
            assert_eq!(bounds(&runtime, HEADER).y, 44.);
            assert_eq!(bounds(&runtime, NAVIGATION).y, 156.);
            if direction == LayoutDirection::LeftToRight {
                assert_eq!(panel.x, 0.);
                assert_eq!(close.x - panel.x, 12.);
            } else {
                assert_eq!(panel.x + panel.width, 960.);
                assert_eq!(panel.x + panel.width - close.x - close.width, 12.);
            }

            let theme = &runtime.app().theme;
            let layer = node(&runtime, LAYER).style();
            assert_eq!(layer.position, Position::Fixed);
            assert!(layer.z_index > 0);
            let Color(red, green, blue, _) = theme.colors.scrim;
            assert_eq!(layer.background, Color(red, green, blue, 82));
            let panel = node(&runtime, DIALOG);
            assert_eq!(panel.style().background, theme.colors.surface_container);
            assert_eq!(panel.style().corners, Some(shape::large_end(direction)));
            assert_eq!(
                panel.box_shadow_layers(),
                theme.elevation(Elevation::Level2)
            );
        }
    }
}

#[test]
fn modal_exposes_only_its_named_navigation_and_traps_focus_until_dismissal() {
    let mut fonts = TextSystem::default();
    for retain_inline in [false, true] {
        let mut runtime =
            Runtime::new(Rail::new(LayoutDirection::LeftToRight, retain_inline)).unwrap();
        runtime.render(960., 640., &mut fonts).unwrap();
        runtime.activate(OPEN).unwrap();
        runtime.render(960., 640., &mut fonts).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        let id = |name| runtime.dom().by_id(name).unwrap();
        assert_eq!(frame.root(), id(DIALOG));
        assert_eq!(frame.modal_root(), Some(id(DIALOG)));
        assert_eq!(frame.focused(), Some(id(CLOSE)));
        assert!(frame.node(id(OPEN)).is_none());
        assert!(frame.node(id(PAGE)).is_none());
        assert_eq!(frame.node(id(DIALOG)).unwrap().node.role(), Role::Dialog);
        assert_eq!(frame.node(id(DIALOG)).unwrap().node.name(), TITLE);
        let navigation = frame.node(id(NAVIGATION)).unwrap().node;
        assert_eq!(
            (navigation.role(), navigation.name()),
            (Role::Navigation, DESTINATIONS)
        );
        let close = frame.node(id(CLOSE)).unwrap().node;
        assert_eq!((close.role(), close.name()), (Role::Button, CLOSE_NAME));
        assert_eq!(node(&runtime, "modal-rail-title").role(), Role::Heading);
        assert_eq!(node(&runtime, "home").is_current_page(), Some(true));
        assert!(node(&runtime, "archive").is_disabled());
        assert!(runtime.activate("background").is_err());

        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some("home"));
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some(CLOSE));
        runtime
            .dispatch(Event::FocusNext { reverse: true })
            .unwrap();
        assert_eq!(runtime.focused(), Some("home"));
        runtime
            .dispatch(Event::Edit {
                key: EditKey::Down,
                extend: false,
                by_word: false,
            })
            .unwrap();
        assert_eq!(runtime.focused(), Some("inbox"));
        runtime.dispatch(Event::Activate).unwrap();
        assert_eq!(runtime.app().selected, 2);
        assert_eq!(node(&runtime, "inbox").is_current_page(), Some(true));
        assert_eq!(runtime.focused(), Some("inbox"));

        runtime.dispatch(Event::Cancel).unwrap();
        assert_eq!(runtime.focused(), Some(OPEN));
        assert_eq!(runtime.app().dismissals, 1);
        runtime.render(960., 640., &mut fonts).unwrap();
        runtime.activate(OPEN).unwrap();
        runtime.render(960., 640., &mut fonts).unwrap();
        runtime.dispatch(Event::Activate).unwrap();
        assert_eq!(runtime.focused(), Some(OPEN));
        assert_eq!(runtime.app().dismissals, 2);
        runtime.render(960., 640., &mut fonts).unwrap();
        runtime.activate(OPEN).unwrap();
        runtime.render(960., 640., &mut fonts).unwrap();
        runtime
            .dispatch(Event::PointerDown { x: 500., y: 80. })
            .unwrap();
        runtime
            .dispatch(Event::PointerUp { x: 500., y: 80. })
            .unwrap();
        assert_eq!(runtime.focused(), Some(OPEN));
        assert_eq!(runtime.app().dismissals, 3);
        assert_eq!(runtime.app().background_presses, 0);
    }
}

#[test]
fn short_viewport_scrolls_destinations_without_moving_header_or_page() {
    let mut fonts = TextSystem::default();
    let mut runtime = Runtime::new(Rail::new(LayoutDirection::LeftToRight, true)).unwrap();
    runtime.render(960., 240., &mut fonts).unwrap();
    runtime.activate(OPEN).unwrap();
    runtime.render(960., 240., &mut fonts).unwrap();
    let close = bounds(&runtime, CLOSE);
    let page = runtime.scroll_metrics(PAGE).unwrap();
    assert!(page.content.1 > page.viewport.1);
    let navigation = runtime.scroll_metrics(NAVIGATION).unwrap();
    assert_eq!(navigation.viewport, (220., 120.));
    assert_eq!(navigation.offset, (0., 0.));
    assert!(navigation.content.1 > navigation.viewport.1);
    runtime
        .dispatch(Event::Scroll {
            x: 80.,
            y: 185.,
            delta_x: 0.,
            delta_y: 64.,
        })
        .unwrap();
    runtime.render(960., 240., &mut fonts).unwrap();
    assert_eq!(runtime.scroll_metrics(NAVIGATION).unwrap().offset.1, 64.);
    assert_eq!(bounds(&runtime, CLOSE), close);
    assert_eq!(runtime.scroll_metrics(PAGE).unwrap(), page);

    let target = runtime
        .accessibility_frame()
        .unwrap()
        .scroll_page_target(
            runtime.dom().by_id(NAVIGATION).unwrap(),
            AccessibilityScrollDirection::Down,
        )
        .unwrap();
    assert_eq!(
        runtime.accessibility_scroll_page(&target).unwrap(),
        AccessibilityScrollResult::Scheduled
    );
    runtime.render(960., 240., &mut fonts).unwrap();
    assert_eq!(
        runtime.scroll_metrics(NAVIGATION).unwrap().offset.1,
        (64. + navigation.viewport.1).min(navigation.content.1 - navigation.viewport.1)
    );
    assert_eq!(runtime.focused(), Some(CLOSE));

    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    for _ in 0..3 {
        runtime
            .dispatch(Event::Edit {
                key: EditKey::Down,
                extend: false,
                by_word: false,
            })
            .unwrap();
    }
    runtime.render(960., 240., &mut fonts).unwrap();
    assert_eq!(runtime.focused(), Some("help"));
    assert!(runtime.scroll_metrics(NAVIGATION).unwrap().at_bottom);
    let destination = bounds(&runtime, "help");
    assert_eq!(
        destination.intersect(bounds(&runtime, NAVIGATION)),
        destination
    );
    assert_eq!(bounds(&runtime, CLOSE), close);
    assert_eq!(runtime.scroll_metrics(PAGE).unwrap(), page);
}

#[test]
fn long_scaled_destination_wraps_at_the_modal_width_limit() {
    let mut fonts = TextSystem::default();
    let mut app = Rail::new(LayoutDirection::RightToLeft, false);
    app.labels[0] = LONG_LABEL;
    app.text_scale = 2.;
    app.expanded = true;
    let mut runtime = Runtime::new(app).unwrap();

    for (width, panel_width) in [(960., 360.), (320., 320.)] {
        let paint = runtime.render_extended(width, 480., &mut fonts).unwrap();
        let panel = bounds(&runtime, DIALOG);
        let destination = bounds(&runtime, "home");
        assert_eq!(panel.width, panel_width);
        assert_eq!(panel.x + panel.width, width);
        assert_eq!(destination.width, panel.width);
        assert!(destination.height > 56.);
        assert_eq!(destination.intersect(panel), destination);
        assert!(paint.ops().iter().any(|op| matches!(
            op,
            ExtendedPaintOp::Legacy(PaintOp::StyledText { rect, clip, text, .. })
                if text == LONG_LABEL && rect.intersect(*clip) == *rect
        )));
    }
}

#[test]
fn focused_scaled_inbox_stays_fully_visible_at_320_by_240() {
    let mut fonts = TextSystem::default();
    let mut app = Rail::new(LayoutDirection::RightToLeft, false);
    app.labels[2] = INBOX_LABEL;
    app.text_scale = 2.;
    let mut runtime = Runtime::new(app).unwrap();
    runtime.render(960., 800., &mut fonts).unwrap();
    runtime.activate(OPEN).unwrap();
    runtime.render(960., 800., &mut fonts).unwrap();
    assert_eq!(bounds(&runtime, HEADER).y, 44.);
    assert_eq!(bounds(&runtime, NAVIGATION).y, 156.);

    runtime.render(320., 240., &mut fonts).unwrap();
    assert_eq!(runtime.focused(), Some(CLOSE));
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    runtime
        .dispatch(Event::Edit {
            key: EditKey::Down,
            extend: false,
            by_word: false,
        })
        .unwrap();

    let paint = runtime.render_extended(320., 240., &mut fonts).unwrap();
    let nav = bounds(&runtime, NAVIGATION);
    let inbox = bounds(&runtime, "inbox");
    assert_eq!(runtime.focused(), Some("inbox"));
    assert_eq!(
        runtime.accessibility_frame().unwrap().focused(),
        runtime.dom().by_id("inbox")
    );
    assert_eq!((nav.y, nav.height), (120., 120.));
    assert_eq!(inbox.height, 112.);
    assert_eq!(inbox.intersect(nav), inbox);
    assert!(runtime.scroll_metrics(NAVIGATION).unwrap().offset.1 > 0.);
    let close = bounds(&runtime, CLOSE);
    assert_eq!((close.width, close.height), (48., 48.));
    assert!(paint.ops().iter().any(|op| matches!(
        op,
        ExtendedPaintOp::Legacy(PaintOp::Text { rect, text, font_size, .. })
            if text == "×" && *font_size == 24. && rect.intersect(close) == *rect
    )));
    assert!(paint.ops().iter().any(|op| matches!(
        op,
        ExtendedPaintOp::Legacy(PaintOp::StyledText { rect, clip, text, .. })
            if text == INBOX_LABEL && rect.height == 80. && rect.intersect(*clip) == *rect
    )));
    let focus_color = runtime.app().theme.colors.secondary;
    assert!(paint.ops().iter().any(|op| matches!(
        op,
        ExtendedPaintOp::Legacy(PaintOp::Stroke { rect, clip, width, color, .. })
            if *color == focus_color && *width == 2.
                && rect.y == inbox.y && rect.height == inbox.height
                && rect.intersect(*clip) == *rect
    )));
}

#[test]
fn animated_modal_moves_its_measured_panel_and_releases_the_scope_on_close() {
    const CLOSING: &str = "modal-rail-closing";
    let mut fonts = TextSystem::default();
    for retain in [false, true] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let mut app = Rail::new(direction, retain);
            app.motion = Some(WideNavigationRailMotion::new(
                false,
                WideNavigationRailMotionKind::Modal,
            ));
            let mut runtime = Runtime::new(app).unwrap();
            runtime.set_time(100.);
            runtime.render(960., 640., &mut fonts).unwrap();
            runtime
                .dispatch(Event::FocusNext { reverse: false })
                .unwrap();
            assert_eq!(runtime.focused(), Some(OPEN));
            let page = bounds(&runtime, PAGE);
            runtime.dispatch(Event::Activate).unwrap();
            runtime.render(960., 640., &mut fonts).unwrap();

            let start = bounds(&runtime, DIALOG);
            let rtl = direction == LayoutDirection::RightToLeft;
            if retain {
                assert_eq!(start.width, 96.);
                assert_eq!(start.x, if rtl { 960. - start.width } else { 0. });
            } else {
                assert_eq!(start.width, 220.);
                assert_eq!(start.x, if rtl { 960. } else { -start.width });
            }
            assert_eq!(runtime.focused(), Some(CLOSE));

            runtime.set_time(140.);
            runtime.render(960., 640., &mut fonts).unwrap();
            let open_frame = bounds(&runtime, DIALOG);
            let tracks = runtime.app().motion.as_ref().unwrap().frame();
            assert_eq!(bounds(&runtime, PAGE), page);
            let alpha = node(&runtime, LAYER).style().background.3;
            assert!((1..82).contains(&alpha));
            if retain {
                assert!(open_frame.width > start.width && open_frame.width < 220.);
                assert_eq!(open_frame.x, if rtl { 960. - open_frame.width } else { 0. });
            } else {
                let distance = (1. - tracks.slide) * open_frame.width;
                let expected = if rtl {
                    960. - open_frame.width + distance
                } else {
                    -distance
                };
                assert!((open_frame.x - expected).abs() < 0.01, "{open_frame:?}");
            }

            runtime.dispatch(Event::Cancel).unwrap();
            assert_eq!(runtime.focused(), Some(OPEN));
            assert!(runtime.dom().by_id(DIALOG).is_none());
            runtime.render(960., 640., &mut fonts).unwrap();
            assert_eq!(bounds(&runtime, CLOSING), open_frame);
            let ghost = runtime.dom().by_id(CLOSING).unwrap();
            assert!(runtime.accessibility_frame().unwrap().node(ghost).is_none());
            runtime.activate("background").unwrap();
            assert_eq!(runtime.app().background_presses, 1);

            runtime.set_time(700.);
            runtime.render(960., 640., &mut fonts).unwrap();
            assert!(runtime.dom().by_id(CLOSING).is_none());
            assert_eq!(bounds(&runtime, PAGE), page);
        }
    }
}

#[test]
fn closing_narrow_modal_preserves_the_last_visible_destination() {
    let mut app = Rail::new(LayoutDirection::RightToLeft, false);
    app.labels[2] = INBOX_LABEL;
    app.text_scale = 2.;
    app.motion = Some(WideNavigationRailMotion::new(
        false,
        WideNavigationRailMotionKind::Modal,
    ));
    let mut runtime = Runtime::new(app).unwrap();
    let mut fonts = TextSystem::default();
    runtime.set_time(100.);
    runtime.render(320., 240., &mut fonts).unwrap();
    runtime.activate(OPEN).unwrap();
    runtime.render(320., 240., &mut fonts).unwrap();
    runtime.set_time(700.);
    runtime.render(320., 240., &mut fonts).unwrap();
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    runtime
        .dispatch(Event::Edit {
            key: EditKey::Down,
            extend: false,
            by_word: false,
        })
        .unwrap();
    runtime.render(320., 240., &mut fonts).unwrap();
    assert_eq!(runtime.focused(), Some("inbox"));
    let before = bounds(&runtime, "inbox");
    let scrolled = runtime.scroll_metrics(NAVIGATION).unwrap().offset;
    assert!(scrolled.1 > 0.);

    runtime.dispatch(Event::Cancel).unwrap();
    runtime.render(320., 240., &mut fonts).unwrap();
    assert_eq!(runtime.focused(), Some(OPEN));
    assert_eq!(bounds(&runtime, "inbox"), before);
    assert_eq!(runtime.scroll_metrics(NAVIGATION).unwrap().offset, scrolled);
    let retiring_navigation = runtime.dom().by_id(NAVIGATION).unwrap();
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .node(retiring_navigation)
        .is_none());
}

#[test]
fn swipe_uses_the_panel_anchors_and_commits_the_dismissal_once() {
    let mut swipe = WideNavigationRailSwipe::default();
    assert!(swipe.set_viewport(960., 640.));
    for (direction, sign) in [
        (LayoutDirection::LeftToRight, -1.),
        (LayoutDirection::RightToLeft, 1.),
    ] {
        for (travel, speed, dismiss) in [
            (55., 124., false),
            (55., 125., true),
            (132., -125., false),
            (132., 0., true),
            (110., 0., false),
            (110., 1., true),
            (0., 500., false),
        ] {
            let delta_x = sign * travel;
            swipe.apply(HorizontalDrag::Move { delta_x }, 220., direction);
            assert_eq!(swipe.progress(), Some(1. - travel / 220.));
            swipe.apply(
                HorizontalDrag::End {
                    delta_x,
                    velocity_x: sign * speed,
                },
                220.,
                direction,
            );
            assert!(swipe.is_settling());
            let (update, committed) = swipe.advance(Some(700.), true);
            assert!(update.rebuild && !update.request_next);
            assert_eq!(committed, dismiss);
            assert_eq!(swipe.progress(), None);
            assert_eq!(
                swipe.advance(Some(1200.), false),
                (AnimationUpdate::default(), false)
            );
        }
    }

    let direction = LayoutDirection::LeftToRight;
    swipe.apply(HorizontalDrag::Move { delta_x: -300. }, 220., direction);
    assert_eq!(swipe.progress(), Some(0.));
    swipe.apply(HorizontalDrag::Cancel, 220., direction);
    assert!(!swipe.advance(None, false).1);
    assert_eq!(swipe.progress(), None);

    swipe.apply(HorizontalDrag::Move { delta_x: -50. }, 220., direction);
    assert!(swipe.set_viewport(320., 240.));
    assert!(!swipe.set_viewport(320., 240.));
    assert_eq!(swipe.progress(), None);
    assert!(!swipe.is_settling());
}

fn open_swipe_rail(
    direction: LayoutDirection,
    width: f32,
    height: f32,
    fonts: &mut TextSystem,
) -> Runtime<Rail> {
    let mut swipe = WideNavigationRailSwipe::default();
    swipe.set_viewport(width, height);
    let mut app = Rail::new(direction, false);
    if width == 320. {
        app.labels[0] = LONG_LABEL;
        app.text_scale = 2.;
    }
    app.motion = Some(WideNavigationRailMotion::new(
        false,
        WideNavigationRailMotionKind::Modal,
    ));
    app.swipe = Some(swipe);
    let mut runtime = Runtime::new(app).unwrap();
    runtime.set_time(100.);
    runtime.render(width, height, fonts).unwrap();
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(OPEN));
    runtime.dispatch(Event::Activate).unwrap();
    runtime.render(width, height, fonts).unwrap();
    runtime.set_time(700.);
    runtime.render(width, height, fonts).unwrap();
    assert_eq!(runtime.focused(), Some(CLOSE));
    runtime
}

#[test]
fn standalone_swipe_tracks_the_measured_panel_and_keeps_modal_focus_until_settled() {
    let mut fonts = TextSystem::default();
    for (direction, viewport, expected_width, sign) in [
        (LayoutDirection::LeftToRight, 960., 220., -1.),
        (LayoutDirection::RightToLeft, 320., 320., 1.),
    ] {
        let height = 640.;
        let mut runtime = open_swipe_rail(direction, viewport, height, &mut fonts);
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some("home"));
        let page = bounds(&runtime, PAGE);
        let panel = bounds(&runtime, DIALOG);
        assert_eq!(panel.width, expected_width);
        let x = panel.x + panel.width * if sign < 0. { 0.8 } else { 0.2 };
        let inbox = bounds(&runtime, "inbox");
        let y = inbox.y + inbox.height * 0.5;
        let target = x + sign * panel.width * 0.6;
        runtime.dispatch(Event::PointerDown { x, y }).unwrap();
        runtime.set_time(900.);
        runtime
            .dispatch(Event::PointerMove { x: target, y })
            .unwrap();
        runtime.render(viewport, height, &mut fonts).unwrap();

        let held = bounds(&runtime, DIALOG);
        assert!((held.x - panel.x - (target - x)).abs() < 0.01, "{held:?}");
        assert_eq!(held.width, panel.width);
        assert_eq!(node(&runtime, LAYER).style().background.3, 33);
        assert_eq!(runtime.focused(), Some("home"));
        assert_eq!(node(&runtime, "home").is_current_page(), Some(true));
        assert!(runtime.activate("background").is_err());

        runtime.set_time(1100.);
        runtime.dispatch(Event::PointerUp { x: target, y }).unwrap();
        runtime.render(viewport, height, &mut fonts).unwrap();
        assert_eq!(bounds(&runtime, DIALOG), held);
        runtime.set_time(1140.);
        runtime.render(viewport, height, &mut fonts).unwrap();
        assert!(runtime.app().swipe.as_ref().unwrap().is_settling());
        let modal = runtime.accessibility_frame().unwrap();
        assert_eq!(modal.modal_root(), runtime.dom().by_id(DIALOG));
        assert!(modal.node(runtime.dom().by_id(PAGE).unwrap()).is_none());
        assert_eq!(node(&runtime, "home").is_current_page(), Some(true));
        assert_eq!(runtime.app().selected, 0);
        assert_eq!(runtime.app().dismissals, 0);
        assert!((1..33).contains(&node(&runtime, LAYER).style().background.3));

        runtime.set_time(1900.);
        runtime.render(viewport, height, &mut fonts).unwrap();
        assert_eq!(runtime.focused(), Some(OPEN));
        assert!(runtime.dom().by_id(DIALOG).is_none());
        assert!(runtime.dom().by_id("modal-rail-closing").is_none());
        assert_eq!(runtime.app().dismissals, 1);
        assert_eq!(runtime.app().selected, 0);
        assert_eq!(bounds(&runtime, PAGE), page);
    }
}

#[test]
fn swipe_cancel_and_vertical_input_keep_the_modal_open_and_scrim_taps_close_it() {
    let (width, height) = (960., 240.);
    let mut fonts = TextSystem::default();
    let mut runtime = open_swipe_rail(LayoutDirection::LeftToRight, width, height, &mut fonts);
    let x = 150.;
    let y = 145.;
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime
        .dispatch(Event::PointerMove {
            x: x - 2.,
            y: y + 30.,
        })
        .unwrap();
    runtime
        .dispatch(Event::PointerUp {
            x: x - 2.,
            y: y + 30.,
        })
        .unwrap();
    assert_eq!(runtime.app().swipe.as_ref().unwrap().progress(), None);
    assert_eq!(runtime.app().selected, 0);
    assert_eq!(runtime.focused(), Some(CLOSE));
    runtime
        .dispatch(Event::Scroll {
            x,
            y,
            delta_x: 0.,
            delta_y: 60.,
        })
        .unwrap();
    runtime.render(width, height, &mut fonts).unwrap();
    assert_eq!(runtime.scroll_metrics(NAVIGATION).unwrap().offset.1, 60.);

    runtime.set_reduced_motion(true);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime
        .dispatch(Event::PointerMove { x: x - 132., y })
        .unwrap();
    assert!(runtime.app().swipe.as_ref().unwrap().progress().unwrap() < 0.5);
    runtime.cancel_pointer().unwrap();
    runtime.render(width, height, &mut fonts).unwrap();
    assert_eq!(bounds(&runtime, DIALOG).x, 0.);
    assert_eq!(runtime.app().dismissals, 0);
    assert_eq!(runtime.scroll_metrics(NAVIGATION).unwrap().offset.1, 60.);

    runtime.dispatch(Event::PointerDown { x: 500., y }).unwrap();
    runtime.dispatch(Event::PointerMove { x: 400., y }).unwrap();
    assert_eq!(runtime.app().swipe.as_ref().unwrap().progress(), None);
    runtime.cancel_pointer().unwrap();
    runtime.dispatch(Event::PointerDown { x: 500., y }).unwrap();
    runtime.dispatch(Event::PointerUp { x: 500., y }).unwrap();
    assert_eq!(runtime.focused(), Some(OPEN));
    assert_eq!(runtime.app().dismissals, 1);
    assert_eq!(runtime.app().background_presses, 0);
}
