use surface::{Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{AccessibilityScrollDirection, Application, EditKey, Event, Runtime, View};
use surface_material::{
    shape::{self, LayoutDirection},
    typography, ChoiceState, NavigationDrawerItem, StandardNavigationDrawerMode,
    StandardNavigationDrawerOptions, Theme, VectorIcon,
};

const DESTINATIONS: [&str; 10] = [
    "home",
    "archived",
    "inbox",
    "drafts",
    "favorites",
    "shared",
    "teams",
    "projects",
    "files",
    "settings",
];

#[derive(Clone, Copy)]
enum Mode {
    Sheet,
    Permanent,
    Dismissible,
}

#[derive(Clone, Copy)]
enum Message {
    Toggle,
    Content,
    Destination(&'static str),
}

struct App {
    mode: Mode,
    open: bool,
    dark: bool,
    direction: LayoutDirection,
    content_clicks: usize,
    selected: &'static str,
}

impl App {
    fn new(mode: Mode, direction: LayoutDirection, dark: bool) -> Self {
        Self {
            mode,
            open: false,
            dark,
            direction,
            content_clicks: 0,
            selected: "home",
        }
    }

    fn theme(&self) -> Theme {
        if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        }
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => self.open = !self.open,
            Message::Content => self.content_clicks += 1,
            Message::Destination(destination) => {
                self.selected = destination;
                if matches!(self.mode, Mode::Dismissible) {
                    self.open = false;
                }
            }
        }
    }

    fn view(&self) -> View<Message> {
        let theme = self.theme();
        let options = StandardNavigationDrawerOptions {
            id: "drawer",
            navigation_name: "Workspace destinations",
            headline: Some("Workspace"),
            direction: self.direction,
        };
        let icon = VectorIcon::new([[3., 3.], [21., 3.], [21., 21.], [3., 21.]]).unwrap();
        let items = [
            theme.navigation_drawer_section_heading("Places", self.direction),
            theme.divider("drawer-divider"),
        ]
        .into_iter()
        .chain(DESTINATIONS.into_iter().map(|id| {
            theme.navigation_drawer_item_with_icon(NavigationDrawerItem {
                id,
                label: id,
                accessible_name: id,
                icon: &icon,
                direction: self.direction,
                state: ChoiceState {
                    selected: self.selected == id,
                    disabled: id == "archived",
                },
                message: Message::Destination(id),
            })
        }));
        let root = View::new(Node::new(Role::Window, "Workspace"));
        let mode = match self.mode {
            Mode::Sheet => {
                return root.children([theme.standard_navigation_drawer_sheet(options, items)])
            }
            Mode::Permanent => StandardNavigationDrawerMode::Permanent,
            Mode::Dismissible => StandardNavigationDrawerMode::Dismissible {
                open: self.open,
                restore_focus_to: "opener",
            },
        };
        let button = |id, name| {
            View::new(
                Node::new(Role::Button, name)
                    .id(id)
                    .styled(Style::column().height(Length::Px(48.))),
            )
        };
        let content = View::new(Node::new(Role::Group, "Main content")).children([
            button("opener", "Toggle navigation")
                .disclosure_expanded(self.open)
                .on_press(Message::Toggle),
            button("main-action", "Refresh")
                .on_press(Message::Content)
                .accessible_activation(),
        ]);
        root.children([theme.standard_navigation_drawer_layout(options, mode, items, content)])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

fn render(runtime: &mut Runtime<App>, width: f32, height: f32) {
    runtime.render(width, height, &mut Metrics).unwrap();
}

fn bounds(runtime: &Runtime<App>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn pointer_click(runtime: &mut Runtime<App>, id: &str) {
    let rect = bounds(runtime, id);
    let x = rect.x + rect.width / 2.;
    let y = rect.y + rect.height / 2.;
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
}

fn edit(runtime: &mut Runtime<App>, key: EditKey) {
    runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false,
        })
        .unwrap();
}

#[test]
fn standalone_sheet_uses_material_tokens_and_fits_its_parent() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        for dark in [false, true] {
            let mut runtime = Runtime::new(App::new(Mode::Sheet, direction, dark)).unwrap();
            render(&mut runtime, 800., 640.);

            let dom = runtime.dom();
            let sheet = dom.get(dom.by_id("drawer").unwrap()).unwrap();
            assert_eq!(sheet.role(), Role::Navigation);
            assert_eq!(sheet.name(), "Workspace destinations");
            assert_eq!(
                sheet.style().background,
                runtime.app().theme().colors.surface
            );
            assert_eq!(sheet.style().corner_radii(), shape::large_end(direction));
            assert!(sheet
                .box_shadow_layers()
                .iter()
                .all(|shadow| shadow.color.3 == 0));
            assert_eq!(bounds(&runtime, "drawer").width, 360.);
            assert_eq!(bounds(&runtime, "drawer").height, 640.);
            assert_eq!(bounds(&runtime, "home").width, 336.);
            assert_eq!(bounds(&runtime, "home").height, 56.);

            let headings = dom
                .children(dom.by_id("drawer").unwrap())
                .filter(|id| dom.get(*id).unwrap().role() == Role::Heading)
                .collect::<Vec<_>>();
            assert_eq!(headings.len(), 2);
            for (id, name) in headings.into_iter().zip(["Workspace", "Places"]) {
                let heading = dom.get(id).unwrap();
                assert_eq!(heading.name(), name);
                let label_id = dom.children(id).next().unwrap();
                let label = dom.get(label_id).unwrap();
                assert_eq!(label.style().text_style(), typography::TITLE_SMALL);
                assert_eq!(
                    label.style().foreground,
                    runtime.app().theme().colors.on_surface_variant
                );
                let layout = runtime.computed().unwrap();
                let heading_bounds = layout.nodes[id.index()].bounds;
                let label_bounds = layout.nodes[label_id.index()].bounds;
                match direction {
                    LayoutDirection::LeftToRight => {
                        assert_eq!(label_bounds.x, heading_bounds.x + 16.);
                    }
                    LayoutDirection::RightToLeft => {
                        assert_eq!(
                            label_bounds.x + label_bounds.width,
                            heading_bounds.x + heading_bounds.width - 16.
                        );
                    }
                }
            }

            render(&mut runtime, 244., 640.);
            assert_eq!(bounds(&runtime, "drawer").width, 244.);
            assert_eq!(bounds(&runtime, "home").width, 220.);
        }
    }
}

#[test]
fn permanent_drawer_mirrors_layout_and_keeps_narrow_content_actionable() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let mut runtime = Runtime::new(App::new(Mode::Permanent, direction, false)).unwrap();
        for (width, drawer_width) in [(900., 360.), (320., 192.)] {
            render(&mut runtime, width, 640.);
            let drawer = bounds(&runtime, "drawer");
            let content = bounds(&runtime, "drawer-content");
            assert_eq!(drawer.width, drawer_width);
            assert_eq!(content.width, width - drawer_width);
            assert_eq!(drawer.height, 640.);
            assert_eq!(content.height, 640.);
            match direction {
                LayoutDirection::LeftToRight => {
                    assert_eq!(drawer.x, 0.);
                    assert_eq!(content.x, drawer.width);
                }
                LayoutDirection::RightToLeft => {
                    assert_eq!(content.x, 0.);
                    assert_eq!(drawer.x, content.width);
                }
            }
            let frame = runtime.accessibility_frame().unwrap();
            assert!(frame
                .activation_target(runtime.dom().by_id("main-action").unwrap())
                .is_some());
            pointer_click(&mut runtime, "main-action");
            assert_eq!(runtime.focused(), Some("main-action"));
        }
        assert_eq!(runtime.app().content_clicks, 2);
    }
}

#[test]
fn dismissible_drawer_reflows_and_restores_focus_from_a_closed_destination() {
    let mut runtime = Runtime::new(App::new(
        Mode::Dismissible,
        LayoutDirection::LeftToRight,
        false,
    ))
    .unwrap();
    render(&mut runtime, 800., 280.);
    assert_eq!(bounds(&runtime, "drawer-content").width, 800.);
    assert!(runtime.dom().by_id("drawer").is_none());
    pointer_click(&mut runtime, "opener");
    assert_eq!(runtime.focused(), Some("opener"));
    render(&mut runtime, 800., 280.);
    assert_eq!(bounds(&runtime, "drawer-content").width, 440.);
    runtime
        .dispatch(Event::FocusNext { reverse: true })
        .unwrap();
    assert_eq!(runtime.focused(), Some("home"));
    edit(&mut runtime, EditKey::Down);
    assert_eq!(runtime.focused(), Some("inbox"));
    assert_eq!(runtime.app().selected, "home");
    runtime.dispatch(Event::Activate).unwrap();
    assert_eq!(runtime.app().selected, "inbox");
    assert_eq!(runtime.focused(), Some("opener"));
    assert!(runtime.dom().by_id("drawer").is_none());

    runtime.apply(Message::Toggle).unwrap();
    render(&mut runtime, 800., 280.);
    pointer_click(&mut runtime, "main-action");
    assert_eq!(runtime.focused(), Some("main-action"));
    runtime.apply(Message::Toggle).unwrap();
    assert_eq!(runtime.focused(), Some("main-action"));
    render(&mut runtime, 800., 280.);
    assert_eq!(bounds(&runtime, "drawer-content").width, 800.);
}

#[test]
fn scrolling_and_focus_reveal_keep_the_destination_inside_the_sheet() {
    let mut runtime = Runtime::new(App::new(
        Mode::Permanent,
        LayoutDirection::LeftToRight,
        false,
    ))
    .unwrap();
    render(&mut runtime, 900., 250.);
    let sheet_id = runtime.dom().by_id("drawer").unwrap();
    let last = runtime.dom().by_id("settings").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame
        .scroll_page_target(sheet_id, AccessibilityScrollDirection::Down)
        .is_some());
    assert!(frame.scroll_target(last).is_some());
    assert!(frame.focus_target(last).is_none());

    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("home"));
    edit(&mut runtime, EditKey::End);
    assert_eq!(runtime.focused(), Some("settings"));
    render(&mut runtime, 900., 250.);
    let scrolled = runtime.scroll_metrics("drawer").unwrap();
    assert!(scrolled.offset.1 > 0.);
    assert_eq!(
        runtime.scroll_metrics("drawer-content").unwrap().offset,
        (0., 0.)
    );
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(last)
        .is_some());
}
