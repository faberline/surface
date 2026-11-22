use surface::{DisplayList, Length, Node, NodeId, PaintOp, Rect, Role, Style};
use surface_app::{AccessibilityActivationResult, Application, EditKey, Event, Runtime, View};
use surface_material::{
    shape::LayoutDirection::{self, LeftToRight, RightToLeft},
    BadgeBoundary, BadgeContent, BadgedNavigationBarItem, ChoiceState, NavigationBarIcon,
    NavigationRailItem, Theme, VectorIcon,
};
use surface_wgpu::TextSystem;

const IDS: [&str; 3] = ["home", "inbox", "account"];
const LABELS: [&str; 3] = ["首頁", "訊息", "帳戶"];
const INBOX_NAME: &str = "訊息，1234 封未讀訊息";

#[derive(Clone)]
enum Message {
    Badge(BadgeContent<'static>),
    FitBadge,
    Select(usize),
    Other,
}

struct App {
    badge: BadgeContent<'static>,
    fit_badge: bool,
    use_unbadged_items: bool,
    direction: LayoutDirection,
    scale: f32,
    disabled: Option<usize>,
    selected: usize,
    selections: Vec<usize>,
    icon: VectorIcon,
}

impl App {
    fn new(direction: LayoutDirection, scale: f32, disabled: Option<usize>) -> Self {
        Self {
            badge: BadgeContent::Hidden,
            fit_badge: false,
            use_unbadged_items: false,
            direction,
            scale,
            disabled,
            selected: 1,
            selections: Vec::new(),
            icon: VectorIcon::new([[0., 0.], [24., 0.], [24., 24.], [0., 24.]]).unwrap(),
        }
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Badge(badge) => self.badge = badge,
            Message::FitBadge => self.fit_badge = true,
            Message::Select(index) => {
                self.selected = index;
                self.selections.push(index);
            }
            Message::Other => {}
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn view(&self) -> View<Message> {
        let theme = Theme::light();
        let side = |id| {
            View::new(
                Node::new(Role::Button, id)
                    .id(id)
                    .styled(Style::row().width(Length::Px(48.)).height(Length::Px(48.))),
            )
            .on_press(Message::Other)
        };
        let items = IDS
            .iter()
            .zip(LABELS)
            .enumerate()
            .map(|(index, (id, label))| {
                let state = ChoiceState {
                    selected: self.selected == index,
                    disabled: self.disabled == Some(index),
                };
                if self.use_unbadged_items {
                    return theme.navigation_bar_item_with_icon(NavigationRailItem {
                        id,
                        label,
                        accessible_name: label,
                        icon: &self.icon,
                        state,
                        message: Message::Select(index),
                    });
                }
                let (badge, accessible_name, icon) = match index {
                    0 => (
                        BadgeContent::Dot,
                        "首頁，有新活動",
                        NavigationBarIcon::Glyph("H"),
                    ),
                    1 => (
                        self.badge,
                        if self.badge == BadgeContent::Hidden {
                            ""
                        } else {
                            INBOX_NAME
                        },
                        NavigationBarIcon::Vector(&self.icon),
                    ),
                    _ => (
                        BadgeContent::Hidden,
                        label,
                        NavigationBarIcon::Vector(&self.icon),
                    ),
                };
                let item = BadgedNavigationBarItem {
                    id,
                    label,
                    accessible_name,
                    icon,
                    badge,
                    state,
                    direction: self.direction,
                    message: Message::Select(index),
                };
                let destination = if self.fit_badge {
                    theme.navigation_bar_item_with_badge_clearance(item, BadgeBoundary::Owner)
                } else {
                    theme.navigation_bar_item_with_badge(item)
                };
                destination.accessible_activation()
            });
        theme.page([
            side("before"),
            theme.navigation_bar_with_direction("navigation", "主要導覽", self.direction, items),
            side("after"),
        ])
    }
}

fn id(runtime: &Runtime<App>, name: &str) -> NodeId {
    runtime.dom().by_id(name).unwrap()
}

fn rect(runtime: &Runtime<App>, id: NodeId) -> Rect {
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn item_rects(runtime: &Runtime<App>) -> [Rect; 3] {
    IDS.map(|name| rect(runtime, id(runtime, name)))
}

fn indicator(runtime: &Runtime<App>, name: &str) -> NodeId {
    let artwork = runtime.dom().children(id(runtime, name)).next().unwrap();
    runtime.dom().children(artwork).last().unwrap()
}

fn art(runtime: &Runtime<App>, name: &str) -> Rect {
    let anchor = runtime
        .dom()
        .children(indicator(runtime, name))
        .next()
        .unwrap();
    let art = runtime
        .dom()
        .children(anchor)
        .find(|id| runtime.dom().get(*id).unwrap().style().width == Length::Px(24.))
        .unwrap();
    rect(runtime, art)
}

fn badge_face(display: &DisplayList, min_width: f32) -> Rect {
    let ink = Theme::light().colors.error;
    display
        .ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Shape { rect, color, .. } | PaintOp::Quad { rect, color, .. }
                if *color == ink && rect.width >= min_width =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .unwrap()
}

fn arrow(runtime: &mut Runtime<App>, key: EditKey) {
    runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false,
        })
        .unwrap();
}

fn tap(runtime: &mut Runtime<App>, x: f32, y: f32) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
}

#[test]
fn default_text_size_matches_the_existing_vector_destination_geometry() {
    let mut fonts = TextSystem::default();
    for direction in [LeftToRight, RightToLeft] {
        let mut existing = App::new(direction, 1., None);
        existing.use_unbadged_items = true;
        let mut existing = Runtime::new(existing).unwrap();
        existing.render(320., 400., &mut fonts).unwrap();
        let inbox = id(&existing, "inbox");
        let indicator = existing.dom().children(inbox).next().unwrap();
        let icon = existing.dom().children(indicator).next().unwrap();

        let mut opted_in = Runtime::new(App::new(direction, 1., None)).unwrap();
        opted_in.apply(Message::FitBadge).unwrap();
        for content in [
            BadgeContent::Hidden,
            BadgeContent::Dot,
            BadgeContent::Count(1234),
        ] {
            opted_in.apply(Message::Badge(content)).unwrap();
            opted_in.render(320., 400., &mut fonts).unwrap();
            assert_eq!(item_rects(&opted_in), item_rects(&existing));
            assert_eq!(art(&opted_in, "inbox"), rect(&existing, icon));
        }
    }
}

#[test]
fn clearance_fits_the_measured_badge_inside_the_navigation_destination() {
    let mut fonts = TextSystem::default();
    for direction in [LeftToRight, RightToLeft] {
        let mut runtime = Runtime::new(App::new(direction, 2., None)).unwrap();
        runtime
            .apply(Message::Badge(BadgeContent::Count(1234)))
            .unwrap();
        let display = runtime.render(320., 400., &mut fonts).unwrap();
        let items = item_rects(&runtime);
        let inbox = items[1];
        let uncontrolled = badge_face(&display, 16.);
        assert_ne!(uncontrolled.intersect(inbox), uncontrolled);
        runtime.activate("inbox").unwrap();

        runtime.apply(Message::FitBadge).unwrap();
        let display = runtime.render(320., 400., &mut fonts).unwrap();
        let fitted = badge_face(&display, 16.);
        assert_eq!(fitted.intersect(inbox), fitted);
        assert_eq!(
            (fitted.width, fitted.height),
            (uncontrolled.width, uncontrolled.height)
        );
        assert_eq!(item_rects(&runtime), items);
        assert_eq!(runtime.focused(), Some("inbox"));
        let frame = runtime.accessibility_frame().unwrap();
        assert_eq!(
            frame.node(id(&runtime, "inbox")).unwrap().node.name(),
            INBOX_NAME
        );
        assert!(display.ops.iter().any(|op| matches!(
            op,
            PaintOp::StyledText { text, rect, clip, .. }
                if text == "999+" && rect.intersect(fitted) == *rect && rect.intersect(*clip) == *rect
        )));
        tap(
            &mut runtime,
            fitted.x + fitted.width / 2.,
            fitted.y + fitted.height / 2.,
        );
        assert_eq!(runtime.app().selections, [1, 1]);
    }
}

#[test]
fn large_text_reserves_constant_headroom_for_the_icon_and_hit_area() {
    let mut fonts = TextSystem::default();
    for direction in [LeftToRight, RightToLeft] {
        let mut runtime = Runtime::new(App::new(direction, 2., None)).unwrap();
        runtime.apply(Message::FitBadge).unwrap();
        runtime.render(320., 400., &mut fonts).unwrap();
        runtime.activate("inbox").unwrap();
        runtime.render(320., 400., &mut fonts).unwrap();
        let items = item_rects(&runtime);
        let icon = art(&runtime, "inbox");
        assert_eq!((icon.width, icon.height), (24., 24.));
        for content in [
            BadgeContent::Dot,
            BadgeContent::Count(1234),
            BadgeContent::Label("新"),
            BadgeContent::Hidden,
        ] {
            runtime.apply(Message::Badge(content)).unwrap();
            let display = runtime.render(320., 400., &mut fonts).unwrap();
            assert_eq!(item_rects(&runtime), items);
            assert_eq!(art(&runtime, "inbox"), icon);
            assert_eq!(runtime.focused(), Some("inbox"));
            if matches!(content, BadgeContent::Count(_) | BadgeContent::Label(_)) {
                let badge = badge_face(&display, 16.);
                assert_eq!(badge.height, 32.);
                assert_eq!(badge.intersect(items[1]), badge);
                assert!(badge.y + badge.height <= icon.y + 4.);
            }
        }
        tap(
            &mut runtime,
            icon.x + icon.width * 0.5,
            icon.y + icon.height * 0.5,
        );
        assert_eq!(runtime.app().selections, [1, 1]);
    }
}

#[test]
fn the_badge_preserves_item_geometry_and_stays_clear_of_neighbor_art_at_320_dp() {
    let mut fonts = TextSystem::default();
    for direction in [LeftToRight, RightToLeft] {
        for scale in [1., 2.] {
            let mut runtime = Runtime::new(App::new(direction, scale, None)).unwrap();
            let display = runtime.render(320., 400., &mut fonts).unwrap();
            let items_when_hidden = item_rects(&runtime);
            let icon_when_hidden = art(&runtime, "inbox");
            let outside_dot = badge_face(&display, 6.);
            assert_eq!((outside_dot.width, outside_dot.height), (6., 6.));

            runtime
                .apply(Message::Badge(BadgeContent::Count(1234)))
                .unwrap();
            let display = runtime.render(320., 400., &mut fonts).unwrap();
            let [home, inbox, account] = item_rects(&runtime);
            assert_eq!([home, inbox, account], items_when_hidden);
            assert!(inbox.width >= 48. && inbox.height >= 80.);
            let selected_indicator = indicator(&runtime, "inbox");
            let indicator = rect(&runtime, selected_indicator);
            assert_eq!((indicator.width, indicator.height), (64., 32.));
            assert_eq!(
                runtime
                    .dom()
                    .get(selected_indicator)
                    .unwrap()
                    .style()
                    .background,
                Theme::light().colors.secondary_container
            );
            let icon = art(&runtime, "inbox");
            assert_eq!((icon.width, icon.height), (24., 24.));
            assert_eq!(icon, icon_when_hidden);
            let badge = badge_face(&display, 16.);
            assert_eq!(badge.y + badge.height, icon.y + 4.);
            if direction == LeftToRight {
                assert!(home.x < inbox.x && inbox.x < account.x);
                assert_eq!(badge.x, icon.x + 12.);
            } else {
                assert!(account.x < inbox.x && inbox.x < home.x);
                assert_eq!(badge.x + badge.width, icon.x + 12.);
            }
            let nearby_art = art(&runtime, "account");
            assert_eq!(badge.intersect(nearby_art).width, 0.);
            assert!(display
                .ops
                .iter()
                .any(|op| matches!(op, PaintOp::StyledText {
                text, rect, clip, ..
            } if text == "999+"
                && rect.intersect(badge) == *rect && rect.intersect(*clip) == *rect)));
            for label in LABELS {
                assert!(display
                    .ops
                    .iter()
                    .any(|op| matches!(op, PaintOp::StyledText {
                    text, rect, ..
                } if text == label && rect.y > badge.y + badge.height)));
            }
        }
    }
}

#[test]
fn reading_order_selected_tab_entry_and_disabled_skip_survive_mirroring_and_badges() {
    let mut fonts = TextSystem::default();
    for (direction, toward_trailing) in
        [(LeftToRight, EditKey::Right), (RightToLeft, EditKey::Left)]
    {
        let mut runtime = Runtime::new(App::new(direction, 1., Some(2))).unwrap();
        runtime.render(320., 400., &mut fonts).unwrap();
        for target in ["before", "inbox"] {
            runtime
                .dispatch(Event::FocusNext { reverse: false })
                .unwrap();
            assert_eq!(runtime.focused(), Some(target));
        }
        runtime
            .apply(Message::Badge(BadgeContent::Label("新")))
            .unwrap();
        runtime.render(320., 400., &mut fonts).unwrap();
        assert_eq!(runtime.focused(), Some("inbox"));
        let frame = runtime.accessibility_frame().unwrap();
        let navigation = id(&runtime, "navigation");
        assert_eq!(
            frame.children(navigation).collect::<Vec<_>>(),
            IDS.map(|name| id(&runtime, name))
        );
        let inbox = frame.node(id(&runtime, "inbox")).unwrap().node;
        assert_eq!(inbox.role(), Role::Button);
        assert_eq!(inbox.name(), INBOX_NAME);
        assert_eq!(inbox.is_selected(), Some(true));
        let account = id(&runtime, "account");
        assert!(frame.node(account).unwrap().node.is_disabled());
        assert!(frame.focus_target(account).is_none());
        assert!(frame.activation_target(account).is_none());
        for (id, node) in runtime.dom().iter() {
            if matches!(node.text_value(), Some("新" | "訊息")) {
                assert!(frame.node(id).is_none());
            }
        }
        arrow(&mut runtime, toward_trailing);
        assert_eq!(runtime.focused(), Some("home"));
        assert!(runtime.app().selections.is_empty());
        arrow(&mut runtime, EditKey::End);
        assert_eq!(runtime.focused(), Some("inbox"));
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some("after"));
        runtime
            .dispatch(Event::FocusNext { reverse: true })
            .unwrap();
        assert_eq!(runtime.focused(), Some("inbox"));
        runtime.render(320., 400., &mut fonts).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        let inbox_action = frame.activation_target(id(&runtime, "inbox")).unwrap();
        assert_eq!(
            runtime.accessibility_activate(&inbox_action).unwrap(),
            AccessibilityActivationResult::Activated
        );
        assert_eq!(runtime.app().selections, [1]);
    }
}

#[test]
fn badge_ink_never_widens_the_destination_hit_area() {
    let mut fonts = TextSystem::default();
    for direction in [LeftToRight, RightToLeft] {
        let mut runtime = Runtime::new(App::new(direction, 2., None)).unwrap();
        runtime
            .apply(Message::Badge(BadgeContent::Count(1234)))
            .unwrap();
        let display = runtime.render(320., 400., &mut fonts).unwrap();
        let [_, inbox, account] = item_rects(&runtime);
        let badge = badge_face(&display, 16.);
        let inside = badge.intersect(inbox);
        let outside = badge.intersect(account);
        assert!(inside.width > 0. && inside.height > 0.);
        assert!(outside.width > 0. && outside.height > 0.);
        for area in [inside, outside] {
            runtime.render(320., 400., &mut fonts).unwrap();
            tap(
                &mut runtime,
                area.x + area.width / 2.,
                area.y + area.height / 2.,
            );
        }
        assert_eq!(runtime.app().selections, [1, 2]);
    }
}
