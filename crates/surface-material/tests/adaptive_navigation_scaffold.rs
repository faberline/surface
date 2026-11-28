use surface::{Length, Node, NodeId, Rect, Role, Style};
use surface_app::{Application, EditKey, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime, View};
use surface_material::{
    shape::LayoutDirection, AdaptiveNavigationScaffoldOptions, ChoiceState, NavigationRailItem,
    Theme, VectorIcon,
};
use surface_wgpu::TextSystem;

const SHELL: &str = "adaptive";
const NAV: &str = "adaptive-nav";
const SCROLL: &str = "adaptive-nav-scroll";
const CONTENT: &str = "adaptive-content";
const PAGE: &str = "page";
const EDITOR: &str = "page-editor";
const ITEMS: [&str; 6] = ["home", "explore", "saved", "alerts", "settings", "extra"];
const LABELS: [&str; 6] = ["首頁", "探索", "收藏", "通知", "閱讀清單與個人設定", "更多"];

#[derive(Clone)]
enum Message {
    Select(usize),
    Edit(String),
}

struct Page {
    width: f32,
    height: f32,
    direction: LayoutDirection,
    enlarged: bool,
    selected: usize,
    calls: Vec<usize>,
    draft: String,
    icon: VectorIcon,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            width: 839.,
            height: 720.,
            direction: LayoutDirection::LeftToRight,
            enlarged: false,
            selected: 0,
            calls: Vec::new(),
            draft: "先走後".into(),
            icon: VectorIcon::new([[2., 2.], [22., 2.], [22., 22.], [2., 22.]]).unwrap(),
        }
    }
}

impl Page {
    fn scaffold(&self, count: usize) -> Option<View<Message>> {
        let theme = if self.enlarged {
            Theme::dark()
        } else {
            Theme::light()
        };
        let editor = View::new(
            Node::new(Role::TextInput, "備註")
                .id(EDITOR)
                .text(&self.draft)
                .styled(Style::column().height(Length::Px(48.))),
        )
        .on_input(Message::Edit);
        let page = View::new(
            Node::new(Role::Group, "")
                .id(PAGE)
                .styled(Style::column().height(Length::Percent(1.)).clip(true)),
        )
        .children([
            editor,
            View::new(
                Node::new(Role::Group, "")
                    .id("page-tail")
                    .styled(Style::column().height(Length::Px(1400.))),
            ),
        ])
        .accessibility_scroll_pages();
        theme.adaptive_navigation_scaffold(
            AdaptiveNavigationScaffoldOptions {
                id: SHELL,
                navigation_id: NAV,
                name: "主要導覽",
                available_width: self.width,
                available_height: self.height,
                direction: self.direction,
            },
            page,
            ITEMS[..count]
                .iter()
                .enumerate()
                .map(|(index, id)| NavigationRailItem {
                    id,
                    label: if index == 4 && !self.enlarged {
                        "設定"
                    } else {
                        LABELS[index]
                    },
                    accessible_name: if index == 4 { "個人設定" } else { "" },
                    icon: &self.icon,
                    state: ChoiceState {
                        selected: self.selected == index,
                        disabled: index == 1,
                    },
                    message: Message::Select(index),
                }),
        )
    }
}

impl Application for Page {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Select(index) => {
                self.calls.push(index);
                self.selected = index;
            }
            Message::Edit(value) => self.draft = value,
        }
    }

    fn view(&self) -> View<Message> {
        View::new(Node::new(Role::Window, "Workspace")).children(self.scaffold(5))
    }

    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let changed = (width, height) != (self.width, self.height);
        (self.width, self.height) = (width, height);
        changed
    }

    fn text_scale(&self) -> f32 {
        if self.enlarged {
            2.
        } else {
            1.
        }
    }
}

fn id(runtime: &Runtime<Page>, key: &str) -> NodeId {
    runtime.dom().by_id(key).unwrap()
}

fn rect(runtime: &Runtime<Page>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].bounds
}

fn render(runtime: &mut Runtime<Page>, fonts: &mut TextSystem, width: f32, height: f32) {
    runtime.render(width, height, fonts).unwrap();
}

fn tap(runtime: &mut Runtime<Page>, key: &str) {
    let bounds = rect(runtime, key);
    let (x, y) = (
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    );
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
}

fn arrow_down(runtime: &mut Runtime<Page>) {
    assert!(runtime
        .dispatch(Event::Edit {
            key: EditKey::Down,
            extend: false,
            by_word: false,
        })
        .unwrap());
}

#[test]
fn rejects_invalid_size_or_destination_count_without_losing_supported_breakpoints() {
    for (width, height, count, valid) in [
        (f32::NAN, 720., 5, false),
        (840., f32::INFINITY, 5, false),
        (840., 63., 5, false),
        (840., 720., 2, false),
        (840., 720., 6, false),
        (279., 720., 5, false),
        (280., 720., 5, true),
        (599., 720., 3, true),
        (600., 720., 5, true),
        (839., 720., 5, true),
        (840., 720., 5, true),
        (1600., 720., 4, true),
    ] {
        assert_eq!(
            Page {
                width,
                height,
                ..Page::default()
            }
            .scaffold(count)
            .is_some(),
            valid,
            "{width} × {height} with {count} destinations"
        );
    }
}

#[test]
fn one_shell_keeps_exact_page_and_navigation_slots_in_both_directions() {
    let mut fonts = TextSystem::default();
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let mut runtime = Runtime::new(Page {
            direction,
            selected: 4,
            ..Page::default()
        })
        .unwrap();
        for width in [599., 600., 839., 840., 1024., 839., 599.] {
            render(&mut runtime, &mut fonts, width, 720.);
            assert_eq!(rect(&runtime, SHELL), Rect::new(0., 0., width, 720.));
            let nav = rect(&runtime, NAV);
            let content = rect(&runtime, CONTENT);
            assert_eq!(rect(&runtime, PAGE), content);
            let home = rect(&runtime, ITEMS[0]);
            let settings = rect(&runtime, ITEMS[4]);
            let visible_rail = width >= 840.;
            assert_eq!(runtime.dom().by_id(SCROLL).is_some(), visible_rail);
            if visible_rail {
                assert_eq!(rect(&runtime, SCROLL), nav);
                assert_eq!((nav.width, nav.height), (96., 720.));
                assert_eq!((content.width, content.height), (width - 96., 720.));
                let rtl = direction == LayoutDirection::RightToLeft;
                assert_eq!(nav.x, if rtl { width - 96. } else { 0. });
                assert_eq!(content.x, if rtl { 0. } else { 96. });
                assert!(settings.y > home.y);
            } else {
                assert_eq!((nav.width, nav.height), (width, 64.));
                assert_eq!(nav.y, 720. - 64.);
                assert_eq!(content, Rect::new(0., 0., width, nav.y));
                assert_eq!(
                    home.x > settings.x,
                    direction == LayoutDirection::RightToLeft
                );
                let expected = if width >= 600. { 112. } else { width / 5. };
                assert!((home.width - expected).abs() < 1.);
            }
            assert!(home.width >= 48. && home.height >= 48.);
            let dom = runtime.dom();
            let nav_id = id(&runtime, NAV);
            assert_eq!(
                dom.iter()
                    .filter(|(_, node)| node.role() == Role::Navigation)
                    .count(),
                1
            );
            let projection = runtime.accessibility_frame().unwrap();
            let landmark = projection.node(nav_id).unwrap().node;
            assert_eq!(
                (landmark.role(), landmark.name()),
                (Role::Navigation, "主要導覽")
            );
            let current = projection.node(id(&runtime, ITEMS[4])).unwrap().node;
            assert_eq!(current.name(), "個人設定");
            assert_eq!(current.role(), Role::Button);
            assert_eq!(current.is_current_page(), Some(true));
            let disabled = projection.node(id(&runtime, ITEMS[1])).unwrap().node;
            assert!(disabled.is_disabled());
            assert_eq!(runtime.app().selected, 4);
            assert!(runtime.app().calls.is_empty());
        }
    }
}

#[test]
fn resize_retains_distinct_current_page_and_focus_and_never_activates_on_its_own() {
    let mut fonts = TextSystem::default();
    let mut runtime = Runtime::new(Page::default()).unwrap();
    render(&mut runtime, &mut fonts, 839., 720.);
    tap(&mut runtime, "explore");
    assert!(runtime.app().calls.is_empty());
    tap(&mut runtime, "saved");
    assert_eq!(runtime.focused(), Some("saved"));
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("alerts"));
    assert_eq!(runtime.app().calls, [2]);
    for width in [840., 839., 599., 600., 840.] {
        render(&mut runtime, &mut fonts, width, 720.);
        assert_eq!(runtime.focused(), Some("alerts"));
        assert_eq!(runtime.app().selected, 2);
        assert_eq!(runtime.app().calls, [2]);
    }
    arrow_down(&mut runtime);
    assert_eq!(runtime.focused(), Some("settings"));
    assert_eq!(runtime.app().calls, [2]);
    render(&mut runtime, &mut fonts, 840., 720.);
    let frame = runtime.accessibility_frame().unwrap();
    let focus = frame.focus_target(id(&runtime, "settings")).unwrap();
    let enter = KeyPress::new(Key::Enter, Modifiers::default());
    assert!(runtime
        .accessibility_key(&focus, &enter, Event::Activate)
        .unwrap());
    render(&mut runtime, &mut fonts, 840., 720.);
    let repeated = runtime.accessibility_frame().unwrap();
    let focus = repeated.focus_target(id(&runtime, "settings")).unwrap();
    assert!(runtime
        .accessibility_key(&focus, &enter.with_repeat(true), Event::Activate)
        .unwrap());
    assert_eq!(runtime.app().calls, [2, 4]);
    assert_eq!(runtime.app().selected, 4);
}

#[test]
fn short_rtl_rail_scrolls_to_focused_destinations_without_moving_the_page() {
    let mut fonts = TextSystem::default();
    let mut runtime = Runtime::new(Page {
        direction: LayoutDirection::RightToLeft,
        enlarged: true,
        ..Page::default()
    })
    .unwrap();
    render(&mut runtime, &mut fonts, 840., 310.);
    let nav = rect(&runtime, NAV);
    let page = rect(&runtime, PAGE);
    let original = rect(&runtime, "settings");
    assert_eq!(nav, Rect::new(744., 0., 96., 310.));
    assert_ne!(nav.intersect(original), original);
    let page_tail = rect(&runtime, "page-tail");
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some("home"));
    arrow_down(&mut runtime);
    assert_eq!(runtime.focused(), Some("saved"));
    arrow_down(&mut runtime);
    arrow_down(&mut runtime);
    assert_eq!(runtime.focused(), Some("settings"));
    render(&mut runtime, &mut fonts, 840., 310.);
    let settings = rect(&runtime, "settings");
    assert!(settings.height > 64. && settings.y < original.y);
    assert_eq!(settings.intersect(nav), settings);
    assert_eq!(rect(&runtime, PAGE), page);
    assert_eq!(rect(&runtime, "page-tail"), page_tail);
    assert!(runtime.app().calls.is_empty());
    runtime
        .dispatch(Event::Scroll {
            x: page.x + 20.,
            y: page.y + 150.,
            delta_x: 0.,
            delta_y: 300.,
        })
        .unwrap();
    render(&mut runtime, &mut fonts, 840., 310.);
    assert!(rect(&runtime, "page-tail").y < page_tail.y);
    assert_eq!(rect(&runtime, "settings"), settings);
}

#[test]
fn resizing_during_editor_composition_keeps_the_same_focused_page_and_draft() {
    let mut fonts = TextSystem::default();
    let mut runtime = Runtime::new(Page::default()).unwrap();
    render(&mut runtime, &mut fonts, 839., 720.);
    tap(&mut runtime, EDITOR);
    runtime
        .dispatch(Event::Select {
            anchor: 6,
            cursor: 3,
        })
        .unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄓ".into(),
            selection: None,
        }))
        .unwrap();
    for width in [840., 839., 599.] {
        render(&mut runtime, &mut fonts, width, 720.);
        let editor = runtime.input_snapshot().unwrap();
        assert_eq!(editor.id, EDITOR);
        assert_eq!(editor.label, "備註");
        assert!(editor.composing);
        assert!(rect(&runtime, PAGE).intersect(rect(&runtime, EDITOR)) == rect(&runtime, EDITOR));
        assert_eq!(runtime.app().draft, "先走後");
        assert!(runtime.app().calls.is_empty());
    }
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("中".into())))
        .unwrap();
    assert_eq!(runtime.app().draft, "先中後");
    assert_eq!(runtime.focused(), Some(EDITOR));
}
