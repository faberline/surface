use surface::{Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};
use surface_material::{
    shape::{self, LayoutDirection},
    typography, AppBarContent, AppBarKind, AppBarOptions, Elevation, Theme,
};

const CHINESE_TITLE: &str = "協作工作區的完整顯示設定";
const CHINESE_SUBTITLE: &str = "自訂專案、成員與通知";

#[derive(Clone, Copy)]
enum Message {
    Change(f32, bool),
    Search,
    ActivateLeading,
    ActivateAction,
}

struct Demo {
    theme: Theme,
    kind: AppBarKind,
    direction: LayoutDirection,
    collapse: f32,
    scrolled: bool,
    subtitle: bool,
    scale: f32,
    title: &'static str,
    search_opens: u32,
}

impl Demo {
    fn new(kind: AppBarKind) -> Self {
        Self {
            theme: Theme::light(),
            kind,
            direction: LayoutDirection::LeftToRight,
            collapse: 0.,
            scrolled: false,
            subtitle: false,
            scale: 1.,
            title: "Workspace",
            search_opens: 0,
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
            Message::Change(collapse, scrolled) => {
                self.collapse = collapse;
                self.scrolled = scrolled;
            }
            Message::Search => self.search_opens += 1,
            Message::ActivateLeading | Message::ActivateAction => {}
        }
    }

    fn view(&self) -> View<Self::Message> {
        let button = |id, name, message| {
            View::new(
                Node::new(Role::Button, name).id(id).styled(
                    Style::column()
                        .width(Length::Px(48.))
                        .height(Length::Px(48.)),
                ),
            )
            .accessible_activation()
            .on_press(message)
        };
        View::new(Node::new(Role::Window, "Workspace")).children([self.theme.app_bar(
            AppBarOptions {
                id: "app-bar",
                title: self.title,
                subtitle: self.subtitle.then_some(CHINESE_SUBTITLE),
                kind: self.kind,
                direction: self.direction,
                collapse: self.collapse,
                scrolled: self.scrolled,
            },
            AppBarContent {
                leading: Some(button("back", "Previous", Message::ActivateLeading)),
                actions: vec![
                    button("edit", "Edit", Message::ActivateAction),
                    button("more", "More", Message::ActivateAction),
                ],
                search: (self.kind == AppBarKind::Search).then(|| {
                    self.theme.app_bar_search_entry_in_direction(
                        "search",
                        "搜尋協作工作區",
                        self.direction,
                        Message::Search,
                    )
                }),
            },
        )])
    }
}

struct Wrapping;

impl TextMeasurer for Wrapping {
    fn measure(&mut self, text: &str, size: f32, limit: Option<f32>) -> (f32, f32) {
        let width: f32 = text
            .chars()
            .map(|character| size * if character.is_ascii() { 0.5 } else { 1. })
            .sum();
        let available = limit.unwrap_or(width).max(1.);
        (
            width.min(available),
            size * 1.5 * (width / available).ceil().max(1.),
        )
    }
}

fn layout(app: &mut Runtime<Demo>, width: f32) {
    app.render(width, 600., &mut Wrapping).unwrap();
}

fn node<'a>(app: &'a Runtime<Demo>, id: &str) -> &'a Node {
    app.dom().get(app.dom().by_id(id).unwrap()).unwrap()
}

fn bounds(app: &Runtime<Demo>, id: &str) -> Rect {
    app.computed().unwrap().nodes[app.dom().by_id(id).unwrap().index()].bounds
}

fn center(rect: Rect) -> f32 {
    rect.x + rect.width / 2.
}

#[test]
fn flexible_endpoints_match_the_pinned_height_and_text_tokens() {
    for (kind, bare_height, subtitle_height, title_font, subtitle_font) in [
        (
            AppBarKind::MediumFlexible,
            112.,
            136.,
            typography::HEADLINE_MEDIUM,
            typography::LABEL_LARGE,
        ),
        (
            AppBarKind::LargeFlexible,
            120.,
            152.,
            typography::DISPLAY_SMALL,
            typography::TITLE_MEDIUM,
        ),
    ] {
        for subtitle in [false, true] {
            let mut fixture = Demo::new(kind);
            fixture.subtitle = subtitle;
            let mut app = Runtime::new(fixture).unwrap();
            layout(&mut app, 800.);
            let expanded = if subtitle {
                subtitle_height
            } else {
                bare_height
            };
            assert_eq!(kind.collapse_distance(subtitle), expanded - 64.);
            assert_eq!(bounds(&app, "app-bar").height, expanded);
            assert_eq!(bounds(&app, "app-bar-title").x, 16.);
            assert!(bounds(&app, "app-bar-title").y >= 64.);
            assert_eq!(node(&app, "app-bar-title").style().text_style(), title_font);
            if subtitle {
                assert_eq!(
                    node(&app, "app-bar-subtitle").style().text_style(),
                    subtitle_font,
                );
            }
            app.activate("more").unwrap();
            app.apply(Message::Change(1., true)).unwrap();
            layout(&mut app, 800.);
            assert_eq!(bounds(&app, "app-bar").height, 64.);
            assert!(bounds(&app, "app-bar-title").y < 64.);
            assert_eq!(
                node(&app, "app-bar-title").style().text_style(),
                typography::TITLE_LARGE,
            );
            assert_eq!(app.focused(), Some("more"));
            assert_eq!(
                app.dom()
                    .iter()
                    .filter(|(_, node)| node.role() == Role::Heading)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn centered_title_under_asymmetric_actions_is_geometric_and_rtl_keeps_tab_order() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let mut fixture = Demo::new(AppBarKind::CenterAligned);
        fixture.direction = direction;
        let mut app = Runtime::new(fixture).unwrap();
        layout(&mut app, 320.);
        let title = bounds(&app, "app-bar-title");
        let leading = bounds(&app, "back");
        let edit = bounds(&app, "edit");
        let more = bounds(&app, "more");
        assert!((center(title) - 160.).abs() <= 0.5, "title {title:?}");
        assert_eq!(bounds(&app, "app-bar").height, 64.);
        if direction == LayoutDirection::LeftToRight {
            assert!(leading.x + leading.width <= title.x);
            assert!(title.x + title.width <= edit.x && edit.x < more.x);
        } else {
            assert!(more.x < edit.x && edit.x + edit.width <= title.x);
            assert!(title.x + title.width <= leading.x);
        }
        for id in ["back", "edit", "more"] {
            app.dispatch(Event::FocusNext { reverse: false }).unwrap();
            assert_eq!(app.focused(), Some(id));
        }
    }
}

#[test]
fn large_chinese_text_reflows_without_clipping_the_heading_or_controls() {
    for kind in [
        AppBarKind::Small,
        AppBarKind::CenterAligned,
        AppBarKind::MediumFlexible,
        AppBarKind::LargeFlexible,
    ] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let mut fixture = Demo::new(kind);
            fixture.title = CHINESE_TITLE;
            fixture.subtitle = true;
            fixture.direction = direction;
            fixture.scale = 2.;
            let mut app = Runtime::new(fixture).unwrap();
            for collapse in [f32::NAN, 0.4, 1.] {
                app.apply(Message::Change(collapse, false)).unwrap();
                layout(&mut app, 320.);
                let bar = bounds(&app, "app-bar");
                let title = bounds(&app, "app-bar-title");
                let subtitle = bounds(&app, "app-bar-subtitle");
                assert!(bar.height > 64.);
                for id in ["back", "edit", "more", "app-bar-title", "app-bar-subtitle"] {
                    let child = bounds(&app, id);
                    assert_eq!(child.intersect(bar), child, "{kind:?} {direction:?} {id}");
                }
                assert_eq!(node(&app, "app-bar-title").name(), CHINESE_TITLE);
                assert!(subtitle.y >= title.y + title.height);
                if matches!(kind, AppBarKind::Small | AppBarKind::CenterAligned) || collapse == 1. {
                    for id in ["back", "edit", "more"] {
                        assert_eq!(
                            title.intersect(bounds(&app, id)).width,
                            0.,
                            "{kind:?} {direction:?} {id}",
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn scroll_appearance_keeps_the_same_geometry_and_uses_theme_colors() {
    for theme in [Theme::light(), Theme::dark()] {
        let mut fixture = Demo::new(AppBarKind::Small);
        fixture.theme = theme;
        let mut app = Runtime::new(fixture).unwrap();
        layout(&mut app, 800.);
        let rest = bounds(&app, "app-bar");
        assert_eq!(
            node(&app, "app-bar").style().background,
            theme.colors.surface
        );
        assert_eq!(node(&app, "app-bar").style().corner_radii(), shape::NONE);
        assert_eq!(
            node(&app, "app-bar").box_shadow_layers(),
            theme.elevation(Elevation::Level0)
        );
        app.apply(Message::Change(1., true)).unwrap();
        layout(&mut app, 800.);
        assert_eq!(bounds(&app, "app-bar"), rest);
        assert_eq!(
            node(&app, "app-bar").style().background,
            theme.colors.surface_container
        );
        assert_eq!(
            node(&app, "app-bar").box_shadow_layers(),
            theme.elevation(Elevation::Level2)
        );
    }
}

#[test]
fn search_entry_has_a_name_and_activates_without_a_text_field() {
    let mut fixture = Demo::new(AppBarKind::Search);
    fixture.direction = LayoutDirection::RightToLeft;
    let mut app = Runtime::new(fixture).unwrap();
    layout(&mut app, 440.);
    let search = bounds(&app, "search");
    assert_eq!(search.height, 56.);
    assert_eq!(bounds(&app, "app-bar").height, 64.);
    assert_eq!(node(&app, "search").name(), "搜尋協作工作區");
    assert_eq!(node(&app, "search").role(), Role::Button);
    assert!(node(&app, "search").is_row_reversed());
    assert_eq!(node(&app, "search").style().corner_radii(), shape::FULL);
    assert_eq!(
        node(&app, "search").style().background,
        Theme::light().colors.surface_container_high
    );
    assert!(app.dom().by_id("app-bar-title").is_none());
    assert!(app
        .dom()
        .iter()
        .all(|(_, node)| node.role() != Role::TextInput));
    app.dispatch(Event::FocusNext { reverse: false }).unwrap();
    app.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(app.focused(), Some("search"));
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.app().search_opens, 1);
    layout(&mut app, 440.);
    let frame = app.accessibility_frame().unwrap();
    assert!(frame
        .activation_target(app.dom().by_id("search").unwrap())
        .is_some());
}
