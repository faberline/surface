use surface::{
    Align, Color, ExtendedDisplayList, ExtendedPaintOp, Length, Node, NodeId, PaintOp, Rect, Role,
    Style,
};
use surface_app::{
    AccessibilityActivationResult, AnimationFrame, AnimationUpdate, Application, EditKey, Event,
    Runtime, View,
};
use surface_material::{
    shape::{self, LayoutDirection},
    typography, ChoiceState, NavigationRailItem, Theme, VectorIcon, WideNavigationRailMode,
    WideNavigationRailMotion, WideNavigationRailMotionKind,
};
use surface_wgpu::TextSystem;

const RAIL: &str = "wide-rail";
const DESTINATIONS: &str = "wide-rail-destinations";
const HEADER: &str = "toggle";
const PAGE: &str = "page";
const HOME: &str = "home";
const ARCHIVE: &str = "archive";
const INBOX: &str = "inbox";
const HOME_NAME: &str = "前往首頁";
const LONG_LABEL: &str = "跨部門協作報告、專案進度與歷史紀錄總覽";
const GALLERY_LABEL: &str = "跨部門協作與專案通知收件匣";

#[derive(Clone, Copy)]
enum Message {
    Toggle,
    Mode(WideNavigationRailMode),
    Select(usize),
}

struct Rail {
    theme: Theme,
    mode: WideNavigationRailMode,
    direction: LayoutDirection,
    scale: f32,
    label: &'static str,
    header: bool,
    gallery_layout: bool,
    icon: VectorIcon,
    selected: usize,
    actions: usize,
    motion: Option<WideNavigationRailMotion>,
}

impl Rail {
    fn new(mode: WideNavigationRailMode, direction: LayoutDirection) -> Self {
        Self {
            theme: Theme::light(),
            mode,
            direction,
            scale: 1.,
            label: "首頁",
            header: true,
            gallery_layout: false,
            icon: VectorIcon::new([[2., 3.], [22., 3.], [12., 21.]]).unwrap(),
            selected: 0,
            actions: 0,
            motion: None,
        }
    }
}

impl Application for Rail {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Mode(mode) => self.mode = mode,
            Message::Toggle => {
                self.mode = match self.mode {
                    WideNavigationRailMode::Collapsed => WideNavigationRailMode::Expanded,
                    WideNavigationRailMode::Expanded => WideNavigationRailMode::Collapsed,
                };
            }
            Message::Select(index) => {
                self.selected = index;
                self.actions += 1;
            }
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        self.motion
            .as_mut()
            .map_or(AnimationUpdate::default(), |motion| {
                motion.advance(
                    self.mode == WideNavigationRailMode::Expanded,
                    frame.now_ms,
                    frame.reduced_motion,
                )
            })
    }

    fn view(&self) -> View<Message> {
        let display_mode = self.motion.as_ref().map_or(self.mode, |motion| {
            if motion.frame().rail == 0. {
                WideNavigationRailMode::Collapsed
            } else {
                WideNavigationRailMode::Expanded
            }
        });
        let header = self.header.then(|| {
            let toggle = View::new(
                Node::new(Role::Button, "導覽列")
                    .id(HEADER)
                    .disclosure_expanded(self.mode == WideNavigationRailMode::Expanded)
                    .styled(Style::row().width(Length::Px(48.)).height(Length::Px(48.))),
            )
            .on_press(Message::Toggle)
            .accessible_activation();
            if self.gallery_layout {
                let align = match (display_mode, self.direction) {
                    (WideNavigationRailMode::Collapsed, _) => Align::Center,
                    (_, LayoutDirection::LeftToRight) => Align::Start,
                    (_, LayoutDirection::RightToLeft) => Align::End,
                };
                View::new(
                    Node::new(Role::Group, "")
                        .styled(Style::row().justify(align).width(Length::Percent(1.))),
                )
                .children([toggle])
            } else {
                toggle
            }
        });
        let destinations = [
            (HOME, self.label, HOME_NAME),
            (ARCHIVE, "封存", "封存資料"),
            (
                INBOX,
                if self.gallery_layout {
                    GALLERY_LABEL
                } else {
                    "收件匣"
                },
                " ",
            ),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (id, label, accessible_name))| {
            let item = NavigationRailItem {
                id,
                label,
                accessible_name,
                icon: &self.icon,
                state: ChoiceState {
                    selected: self.selected == index,
                    disabled: id == ARCHIVE,
                },
                message: Message::Select(index),
            };
            match &self.motion {
                Some(motion) => self.theme.animated_wide_navigation_rail_item_with_icon(
                    motion.frame().rail,
                    self.direction,
                    item,
                ),
                None => self.theme.wide_navigation_rail_item_with_icon(
                    display_mode,
                    self.direction,
                    item,
                ),
            }
        });
        let rail = match &self.motion {
            Some(motion) => self.theme.animated_wide_navigation_rail(
                RAIL,
                "主要導覽",
                motion.frame(),
                header,
                destinations,
            ),
            None => {
                self.theme
                    .wide_navigation_rail(RAIL, "主要導覽", self.mode, header, destinations)
            }
        };
        let mut row = Style::row().align(Align::Stretch);
        let mut pane = Style::column().width(Length::Px(0.)).grow(1.);
        if self.gallery_layout {
            row = row.gap(8.);
            pane = pane
                .padding(12.)
                .background(self.theme.colors.surface_container_low);
        }
        let mut page = View::new(Node::new(Role::Group, "Page").id(PAGE).styled(pane));
        if self.gallery_layout {
            page = page.children([self.theme.text("page-title", HOME_NAME)]);
        }
        let root = Node::new(Role::Window, "Window").styled(row);
        let root = if self.direction == LayoutDirection::RightToLeft {
            root.reverse_row()
        } else {
            root
        };
        let layout = View::new(root).children([rail, page]);
        if self.gallery_layout {
            self.theme.page([self.theme.card(
                "rail-card",
                [self.theme.text("rail-title", "寬導覽列"), layout],
            )])
        } else {
            layout
        }
    }
}

fn geometry(runtime: &Runtime<Rail>, id: NodeId) -> Rect {
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn bounds(runtime: &Runtime<Rail>, name: &str) -> Rect {
    geometry(runtime, runtime.dom().by_id(name).unwrap())
}

fn contains_rect(outer: Rect, inner: Rect) {
    let tolerance = 0.01;
    assert!(
        outer.x - tolerance <= inner.x
            && outer.y - tolerance <= inner.y
            && outer.x + outer.width + tolerance >= inner.x + inner.width
            && outer.y + outer.height + tolerance >= inner.y + inner.height,
        "{outer:?} must contain {inner:?}"
    );
}

fn indicator(runtime: &Runtime<Rail>, name: &str) -> NodeId {
    let id = runtime.dom().by_id(name).unwrap();
    let index = usize::from(runtime.app().mode == WideNavigationRailMode::Expanded);
    runtime.dom().children(id).nth(index).unwrap()
}

fn label(paint: &ExtendedDisplayList, text: &str) -> (Rect, Rect, surface::TextStyle, Color) {
    paint
        .ops()
        .iter()
        .find_map(|op| match op {
            ExtendedPaintOp::Legacy(PaintOp::StyledText {
                rect,
                clip,
                text: value,
                style,
                color,
                ..
            }) if value == text => Some((*rect, *clip, style.clone(), *color)),
            _ => None,
        })
        .expect("the caller's label paints")
}

#[test]
fn endpoint_metrics_and_slots_use_the_rail_tokens_in_both_directions() {
    let mut fonts = TextSystem::default();
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        for (mode, expected_width, text_style) in [
            (
                WideNavigationRailMode::Collapsed,
                96.,
                typography::LABEL_MEDIUM,
            ),
            (
                WideNavigationRailMode::Expanded,
                220.,
                typography::LABEL_LARGE,
            ),
        ] {
            let mut app = Rail::new(mode, direction);
            if direction == LayoutDirection::RightToLeft {
                app.theme = Theme::dark();
            }
            let colors = app.theme.colors;
            let mut runtime = Runtime::new(app).unwrap();
            let paint = runtime.render_extended(960., 720., &mut fonts).unwrap();
            let rail = bounds(&runtime, RAIL);
            let page = bounds(&runtime, PAGE);
            assert_eq!(rail.width, expected_width);
            assert_eq!(rail.height, 720.);
            assert_eq!(page.width, 960. - expected_width);
            if direction == LayoutDirection::RightToLeft {
                assert_eq!(page.x + page.width, rail.x);
            } else {
                assert_eq!(rail.x + rail.width, page.x);
            }
            let header = bounds(&runtime, HEADER);
            let owner = bounds(&runtime, HOME);
            let next = bounds(&runtime, ARCHIVE);
            assert_eq!(header.y - rail.y, 44.);
            assert_eq!(owner.y - (header.y + header.height), 40.);
            assert_eq!(owner.width, expected_width);

            let selected = indicator(&runtime, HOME);
            let pill = geometry(&runtime, selected);
            let icon = geometry(&runtime, runtime.dom().children(selected).next().unwrap());
            let (label_box, clip, style, ink) = label(&paint, runtime.app().label);
            assert_eq!(style, text_style);
            assert_eq!(label_box.intersect(clip), label_box);
            assert_eq!((icon.width, icon.height), (24., 24.));
            assert_eq!(
                runtime.dom().get(selected).unwrap().style().background,
                colors.secondary_container
            );
            assert_eq!(
                runtime.dom().get(selected).unwrap().style().corners,
                Some(shape::FULL)
            );
            assert_eq!(
                runtime
                    .dom()
                    .get(indicator(&runtime, INBOX))
                    .unwrap()
                    .style()
                    .background,
                Color::TRANSPARENT
            );
            assert!(paint
                .ops()
                .iter()
                .any(|op| matches!(op, ExtendedPaintOp::FillPath(path)
                if path.rect() == Rect::new(icon.x + 2., icon.y + 3., 20., 18.)
                    && path.clip() == icon && path.color() == colors.on_secondary_container)));
            let (_, _, _, disabled_ink) = label(&paint, "封存");
            let Color(r, g, b, _) = colors.on_surface_variant;
            assert_eq!(disabled_ink, Color(r, g, b, 97));
            match mode {
                WideNavigationRailMode::Collapsed => {
                    assert_eq!((pill.width, pill.height, owner.height), (56., 32., 64.));
                    assert_eq!(pill.x - owner.x, 20.);
                    assert_eq!(label_box.y - (pill.y + pill.height), 4.);
                    assert_eq!(next.y - (owner.y + owner.height), 4.);
                    assert_eq!(ink, colors.secondary);
                }
                WideNavigationRailMode::Expanded => {
                    assert_eq!(pill.height, 56.);
                    assert_eq!(runtime.dom().get(selected).unwrap().style().padding, 16.);
                    assert!(pill.width >= 24. + 8. + label_box.width + 32.);
                    assert!(pill.width + 40. < owner.width);
                    assert_eq!(label_box.intersect(pill), label_box);
                    assert_eq!(next.y, owner.y + owner.height);
                    assert_eq!(ink, colors.on_secondary_container);
                    if direction == LayoutDirection::RightToLeft {
                        assert_eq!(owner.x + owner.width - (pill.x + pill.width), 20.);
                        assert_eq!(icon.x - (label_box.x + label_box.width), 8.);
                        assert_eq!(pill.x + pill.width - (icon.x + icon.width), 16.);
                    } else {
                        assert_eq!(pill.x - owner.x, 20.);
                        assert_eq!(label_box.x - (icon.x + icon.width), 8.);
                        assert_eq!(icon.x - pill.x, 16.);
                    }
                }
            }
        }
    }
}

#[test]
fn width_fits_content_and_both_modes_wrap_at_their_limits_with_real_fonts() {
    let mut fonts = TextSystem::default();
    let mut app = Rail::new(
        WideNavigationRailMode::Expanded,
        LayoutDirection::RightToLeft,
    );
    app.label = "協作與專案進度總覽";
    app.header = false;
    let mut medium = Runtime::new(app).unwrap();
    let paint = medium.render_extended(840., 480., &mut fonts).unwrap();
    let rail = bounds(&medium, RAIL);
    let (label_box, _, style, _) = label(&paint, medium.app().label);
    assert_eq!(label_box.height, style.line_height_px());
    assert!(rail.width > 220. && rail.width < 360., "{rail:?}");
    assert_eq!(
        rail.width,
        geometry(&medium, indicator(&medium, HOME)).width + 40.
    );
    assert_eq!(bounds(&medium, HOME).y - rail.y, 44.);

    let mut app = Rail::new(
        WideNavigationRailMode::Expanded,
        LayoutDirection::RightToLeft,
    );
    app.theme = Theme::dark();
    app.scale = 2.;
    app.label = LONG_LABEL;
    let mut large = Runtime::new(app).unwrap();
    let paint = large.render_extended(840., 480., &mut fonts).unwrap();
    let rail = bounds(&large, RAIL);
    let item = bounds(&large, HOME);
    let pill = geometry(&large, indicator(&large, HOME));
    let (label_box, clip, style, _) = label(&paint, LONG_LABEL);
    assert_eq!(rail.width, 360.);
    assert_eq!(style, typography::LABEL_LARGE.scaled(2.));
    assert!(label_box.height >= style.line_height_px() * 2.);
    assert_eq!(pill.intersect(item), pill);
    assert!(pill.width <= 320.);
    assert_eq!(
        label_box
            .intersect(clip)
            .intersect(pill)
            .intersect(item)
            .intersect(rail),
        label_box
    );
    assert_eq!(pill.height, label_box.height + 32.);
    assert_eq!(bounds(&large, ARCHIVE).y, item.y + item.height);

    large
        .apply(Message::Mode(WideNavigationRailMode::Collapsed))
        .unwrap();
    let paint = large.render_extended(840., 720., &mut fonts).unwrap();
    let item = bounds(&large, HOME);
    let (label, clip, style, _) = label(&paint, LONG_LABEL);
    assert_eq!(item.width, 96.);
    assert_eq!(style, typography::LABEL_MEDIUM.scaled(2.));
    assert!(label.height >= style.line_height_px() * 2.);
    assert_eq!(label.intersect(clip).intersect(item), label);
    assert_eq!(bounds(&large, ARCHIVE).y - (item.y + item.height), 4.);
}

#[test]
fn width_capped_rail_reflows_its_full_height_in_a_stretched_gallery_row() {
    let mut fonts = TextSystem::default();
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let mut app = Rail::new(WideNavigationRailMode::Collapsed, direction);
        app.label = GALLERY_LABEL;
        app.scale = 2.;
        app.scale = 2.;
        app.theme = Theme::dark();
        app.gallery_layout = true;
        let mut runtime = Runtime::new(app).unwrap();
        use WideNavigationRailMode::{Collapsed, Expanded};
        for (width, mode, expected_width) in [
            (960., Collapsed, 96.),
            (960., Expanded, 360.),
            (600., Expanded, 360.),
            (600., Collapsed, 96.),
            (960., Collapsed, 96.),
        ] {
            runtime.apply(Message::Mode(mode)).unwrap();
            let paint = runtime.render_extended(width, 800., &mut fonts).unwrap();
            let rail = bounds(&runtime, RAIL);
            let pane = bounds(&runtime, PAGE);
            let destinations = bounds(&runtime, DESTINATIONS);
            let inbox = bounds(&runtime, INBOX);
            let pill = geometry(&runtime, indicator(&runtime, INBOX));
            let (label_box, clip, style, _) = label(&paint, GALLERY_LABEL);
            assert_eq!(rail.width, expected_width);
            assert_eq!(rail.y, pane.y);
            assert_eq!(rail.height, pane.height);
            assert!(label_box.height >= 2. * style.line_height_px());
            for bounds in [destinations, inbox, pill, label_box] {
                assert_eq!(
                    bounds.intersect(rail),
                    bounds,
                    "{direction:?} {width}dp, rail {rail:?}, Inbox {inbox:?}, label {label_box:?}"
                );
            }
            assert_eq!(
                rail.y + rail.height,
                destinations.y + destinations.height,
                "{direction:?} {width}dp {mode:?}: destinations should fill the rail"
            );
            assert_eq!(label_box.intersect(clip), label_box);
        }
    }
}

#[test]
fn disclosure_and_destination_focus_stay_separate_and_keep_stable_actions() {
    let app = Rail::new(
        WideNavigationRailMode::Collapsed,
        LayoutDirection::LeftToRight,
    );
    let mut runtime = Runtime::new(app).unwrap();
    let mut fonts = TextSystem::default();
    runtime.render(960., 720., &mut fonts).unwrap();
    let id = |runtime: &Runtime<Rail>, name| runtime.dom().by_id(name).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    let nav = frame.node(id(&runtime, RAIL)).unwrap().node;
    assert_eq!(nav.role(), Role::Navigation);
    assert_eq!(nav.name(), "主要導覽");
    assert_eq!(
        frame.children(id(&runtime, RAIL)).collect::<Vec<_>>(),
        [id(&runtime, HEADER), id(&runtime, DESTINATIONS)]
    );
    let header = frame.node(id(&runtime, HEADER)).unwrap().node;
    assert_eq!(header.expanded_state(), Some(false));
    assert!(!header.has_menu_popup());
    for (name, spoken, current, disabled) in [
        (HOME, HOME_NAME, true, false),
        (ARCHIVE, "封存資料", false, true),
        (INBOX, "收件匣", false, false),
    ] {
        let id = id(&runtime, name);
        let node = frame.node(id).unwrap().node;
        assert_eq!(node.role(), Role::Button);
        assert_eq!(node.name(), spoken);
        assert_eq!(node.is_current_page(), Some(current));
        assert_eq!(node.is_disabled(), disabled);
        assert_eq!(frame.activation_target(id).is_none(), disabled);
        assert!(frame.children(id).next().is_none());
    }
    let retired = frame.activation_target(id(&runtime, INBOX)).unwrap();

    let tab = Event::FocusNext { reverse: false };
    let next = Event::Edit {
        key: EditKey::Down,
        extend: false,
        by_word: false,
    };
    runtime.dispatch(tab.clone()).unwrap();
    assert_eq!(runtime.focused(), Some(HEADER));
    assert!(!runtime.dispatch(next.clone()).unwrap());
    runtime.dispatch(Event::Activate).unwrap();
    assert_eq!(runtime.focused(), Some(HEADER));
    runtime.render(960., 720., &mut fonts).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(
        frame
            .node(id(&runtime, HEADER))
            .unwrap()
            .node
            .expanded_state(),
        Some(true)
    );
    assert_eq!(
        runtime.accessibility_activate(&retired).unwrap(),
        AccessibilityActivationResult::Rejected
    );

    runtime.dispatch(tab).unwrap();
    assert_eq!(runtime.focused(), Some(HOME));
    runtime.dispatch(next).unwrap();
    assert_eq!(runtime.focused(), Some(INBOX));
    assert_eq!(runtime.app().selected, 0);
    assert_eq!(runtime.app().actions, 0);
    runtime
        .apply(Message::Mode(WideNavigationRailMode::Collapsed))
        .unwrap();
    runtime.render(960., 720., &mut fonts).unwrap();
    assert_eq!(runtime.focused(), Some(INBOX));
    runtime.dispatch(Event::Activate).unwrap();
    assert_eq!(runtime.app().actions, 1);
    assert_eq!(runtime.app().selected, 2);
    runtime.render(960., 720., &mut fonts).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(
        frame
            .node(id(&runtime, HOME))
            .unwrap()
            .node
            .is_current_page(),
        Some(false)
    );
    assert_eq!(
        frame
            .node(id(&runtime, INBOX))
            .unwrap()
            .node
            .is_current_page(),
        Some(true)
    );
}

#[test]
fn inline_motion_reflows_the_page_and_preserves_focus_through_reversal() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let mut app = Rail::new(WideNavigationRailMode::Collapsed, direction);
        app.motion = Some(WideNavigationRailMotion::new(
            false,
            WideNavigationRailMotionKind::Inline,
        ));
        let mut runtime = Runtime::new(app).unwrap();
        let mut fonts = TextSystem::default();
        runtime.set_time(100.);
        runtime.render(960., 720., &mut fonts).unwrap();
        let tab = Event::FocusNext { reverse: false };
        runtime.dispatch(tab.clone()).unwrap();
        runtime.dispatch(tab).unwrap();
        assert_eq!(runtime.focused(), Some(HOME));
        let closed_page = bounds(&runtime, PAGE);

        runtime.apply(Message::Toggle).unwrap();
        runtime.render(960., 720., &mut fonts).unwrap();
        assert_eq!(bounds(&runtime, RAIL).width, 96.);
        runtime.set_time(140.);
        runtime.render(960., 720., &mut fonts).unwrap();
        let opening = bounds(&runtime, RAIL);
        let reflowed = bounds(&runtime, PAGE);
        assert!(opening.width > 96. && opening.width < 220., "{opening:?}");
        assert_eq!(opening.width + reflowed.width, 960.);
        assert!(reflowed.width < closed_page.width);
        assert_eq!(runtime.focused(), Some(HOME));

        runtime.apply(Message::Toggle).unwrap();
        runtime.render(960., 720., &mut fonts).unwrap();
        assert_eq!(bounds(&runtime, RAIL), opening);
        assert_eq!(bounds(&runtime, PAGE), reflowed);
        runtime.set_time(700.);
        runtime.render(960., 720., &mut fonts).unwrap();
        assert_eq!(bounds(&runtime, PAGE), closed_page);
        assert_eq!(runtime.focused(), Some(HOME));
        assert!(!runtime.app().motion.as_ref().unwrap().is_running());

        runtime.apply(Message::Toggle).unwrap();
        runtime.render(960., 720., &mut fonts).unwrap();
        let mut previous = 96.;
        for elapsed in (2..=200).step_by(2) {
            runtime.set_time(700. + f64::from(elapsed));
            runtime.render(960., 720., &mut fonts).unwrap();
            let width = bounds(&runtime, RAIL).width;
            assert!(
                (width - previous).abs() <= 8.,
                "width jumped at {elapsed} ms from {previous} to {width}"
            );
            previous = width;
        }
    }
}

#[test]
fn moving_inline_rail_contains_the_wrapped_inbox_in_a_gallery_row() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let mut app = Rail::new(WideNavigationRailMode::Collapsed, direction);
        app.gallery_layout = true;
        app.scale = 2.;
        app.motion = Some(WideNavigationRailMotion::new(
            false,
            WideNavigationRailMotionKind::Inline,
        ));
        let mut runtime = Runtime::new(app).unwrap();
        let mut fonts = TextSystem::default();
        runtime.set_time(100.);
        runtime.render(960., 760., &mut fonts).unwrap();
        runtime.apply(Message::Toggle).unwrap();
        runtime.render(960., 760., &mut fonts).unwrap();
        for elapsed in [20, 40, 60, 100] {
            runtime.set_time(100. + f64::from(elapsed));
            let paint = runtime.render_extended(960., 760., &mut fonts).unwrap();
            let rail = bounds(&runtime, RAIL);
            let inbox = bounds(&runtime, INBOX);
            let (text, clip, text_style, _) = label(&paint, GALLERY_LABEL);
            let progress = runtime.app().motion.as_ref().unwrap().frame().rail;
            if progress < 0.5 {
                assert_eq!(text_style, typography::LABEL_MEDIUM.scaled(2.));
                assert!(text.height >= 2. * text_style.line_height_px());
                assert_eq!(text.intersect(clip).intersect(inbox), text);
            }
            assert_eq!(
                inbox.intersect(rail),
                inbox,
                "{direction:?} {elapsed} ms, rail {rail:?}"
            );
        }
    }
}

#[test]
fn early_item_motion_keeps_the_full_label_and_one_keyboard_target() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let mut app = Rail::new(WideNavigationRailMode::Collapsed, direction);
        app.gallery_layout = true;
        app.scale = 2.;
        app.motion = Some(WideNavigationRailMotion::new(
            false,
            WideNavigationRailMotionKind::Inline,
        ));
        let mut runtime = Runtime::new(app).unwrap();
        let mut fonts = TextSystem::default();
        runtime.set_time(100.);
        let closed = runtime.render_extended(704., 760., &mut fonts).unwrap();
        let (closed_label, _, _, _) = label(&closed, GALLERY_LABEL);
        let tab = Event::FocusNext { reverse: false };
        runtime.dispatch(tab.clone()).unwrap();
        runtime.dispatch(tab).unwrap();
        runtime
            .dispatch(Event::Edit {
                key: EditKey::Down,
                extend: false,
                by_word: false,
            })
            .unwrap();
        assert_eq!(runtime.focused(), Some(INBOX));
        runtime.apply(Message::Toggle).unwrap();
        runtime.render_extended(704., 760., &mut fonts).unwrap();
        let mut previous_progress = 0.;
        let mut previous_alpha = 255;
        for elapsed in [11, 16, 20, 22, 40, 54, 60, 66, 110] {
            runtime.set_time(100. + f64::from(elapsed));
            let paint = runtime.render_extended(704., 760., &mut fonts).unwrap();
            let progress = runtime.app().motion.as_ref().unwrap().frame().rail;
            assert!(progress > previous_progress);
            previous_progress = progress;
            let (text, clip, style, Color(_, _, _, alpha)) = label(&paint, GALLERY_LABEL);
            let item = bounds(&runtime, INBOX);
            let pill = bounds(&runtime, "inbox-motion-indicator");
            let icon = bounds(&runtime, "inbox-motion-icon");
            contains_rect(item, pill);
            contains_rect(pill, icon);
            if elapsed <= 22 {
                assert!((101. ..=135.).contains(&item.width), "{elapsed}: {item:?}");
                assert_eq!(style, typography::LABEL_MEDIUM.scaled(2.));
                assert_eq!(text.height, closed_label.height);
                assert!((text.width - closed_label.width).abs() <= 2.);
                assert!(text.y >= pill.y + pill.height);
                contains_rect(item.intersect(clip), text);
                assert!(alpha < previous_alpha);
                previous_alpha = alpha;
            }
            if (0.44..=0.56).contains(&progress) {
                assert!(alpha < 5, "item should fade through the handoff");
            }
            let id = runtime.dom().by_id(INBOX).unwrap();
            let ax = runtime.accessibility_frame().unwrap();
            let target = ax.node(id).unwrap().node;
            assert_eq!(target.role(), Role::Button);
            assert_eq!(target.name(), GALLERY_LABEL);
            assert_eq!(target.is_current_page(), Some(false));
            assert!(ax.children(id).next().is_none());
            assert!(ax.activation_target(id).is_some());
            assert_eq!(runtime.focused(), Some(INBOX));
        }
        let before_reversal = bounds(&runtime, "inbox-motion-icon");
        runtime.apply(Message::Toggle).unwrap();
        runtime.render_extended(704., 760., &mut fonts).unwrap();
        assert_eq!(bounds(&runtime, "inbox-motion-icon"), before_reversal);
        runtime.dispatch(Event::Activate).unwrap();
        assert_eq!(runtime.app().actions, 1);
        assert_eq!(runtime.app().selected, 2);
        runtime.set_time(900.);
        runtime.render_extended(704., 760., &mut fonts).unwrap();
        assert_eq!(bounds(&runtime, RAIL).width, 96.);
        assert_eq!(runtime.focused(), Some(INBOX));
    }
}
