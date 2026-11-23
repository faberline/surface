use surface::{
    Color, Corners, DisplayList, Length, Node, NodeId, PaintOp, Rect, Role, Style, TextMeasurer,
};
use surface_app::{Application, EditKey, Event, Runtime, View};
use surface_material::{
    shape::{self, LayoutDirection},
    typography, ListItem, ListLines, ListMode, ListOptions, ListStyle, Theme, VectorIcon,
};

#[derive(Clone)]
enum Message {
    Boundary,
    Activate(&'static str),
    Reverse,
    Remove(&'static str),
}

struct App {
    mode: ListMode<'static>,
    style: ListStyle,
    rows: Vec<&'static str>,
    direction: LayoutDirection,
    scale: f32,
    dark: bool,
    viewport: (f32, f32),
    scroll_height: f32,
    presses: Vec<&'static str>,
}

impl App {
    fn new(style: ListStyle, mode: ListMode<'static>) -> Self {
        Self {
            style,
            mode,
            rows: vec!["a", "b", "c"],
            direction: LayoutDirection::LeftToRight,
            scale: 1.,
            dark: false,
            viewport: (400., 440.),
            scroll_height: 350.,
            presses: Vec::new(),
        }
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Boundary => {}
            Message::Activate(id) => {
                self.presses.push(id);
                if matches!(self.mode, ListMode::SingleSelect { .. }) {
                    self.mode = ListMode::SingleSelect {
                        selected_id: Some(id),
                    };
                }
            }
            Message::Reverse => self.rows.reverse(),
            Message::Remove(id) => self.rows.retain(|row| *row != id),
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn view(&self) -> View<Message> {
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let icon = VectorIcon::new([[4., 3.], [20., 3.], [12., 21.]]).unwrap();
        let list = theme.list(
            ListOptions {
                id: "list",
                name: "專案",
                style: self.style,
                mode: self.mode,
                direction: self.direction,
            },
            self.rows.iter().map(|&id| {
                let (headline, overline, supporting, lines) = match id {
                    "a" => ("收入", None, None, ListLines::One),
                    "b" => ("暫停", None, Some("待處理"), ListLines::Two),
                    "c" => ("完成", Some("現況"), Some("已經審核"), ListLines::Three),
                    "long" => (
                        "這個很長的專案名稱會在窄視窗內依照可用寬度換行",
                        Some("工作狀態"),
                        Some("完整的說明可從無障礙介面取得"),
                        ListLines::Three,
                    ),
                    _ => ("無法開啟", None, None, ListLines::One),
                };
                ListItem {
                    id,
                    headline,
                    overline,
                    supporting,
                    leading: Some(&icon),
                    trailing: Some(if id == "long" {
                        "這是一個較長的狀態標籤"
                    } else {
                        "今天"
                    }),
                    lines,
                    disabled: id == "b",
                    on_activate: (id != "missing").then_some(Message::Activate(id)),
                }
            }),
        );
        let boundary = |id| {
            View::new(
                Node::new(Role::Button, id)
                    .id(id)
                    .styled(Style::row().height(Length::Px(36.))),
            )
            .on_press(Message::Boundary)
        };
        View::new(Node::new(Role::Window, "").styled(Style::column().width(Length::Percent(1.))))
            .children([
                boundary("before"),
                View::new(
                    Node::new(Role::Group, "捲動區").id("scroll").styled(
                        Style::column()
                            .height(Length::Px(self.scroll_height))
                            .clip(true),
                    ),
                )
                .children([list])
                .accessibility_scroll_pages(),
                boundary("after"),
            ])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, value: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let natural = value.chars().count() as f32 * size;
        let line_width = width.map_or(natural, |width| width.min(natural));
        let count = (natural / line_width.max(1.)).ceil().max(1.);
        (line_width, size * 1.5 * count)
    }
}

fn runtime(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime);
    runtime
}

fn render(runtime: &mut Runtime<App>) -> DisplayList {
    let (width, height) = runtime.app().viewport;
    runtime.render(width, height, &mut Metrics).unwrap()
}

fn id(runtime: &Runtime<App>, key: &str) -> NodeId {
    runtime.dom().by_id(key).unwrap()
}

fn node<'a>(runtime: &'a Runtime<App>, key: &str) -> &'a Node {
    runtime.dom().get(id(runtime, key)).unwrap()
}

fn bounds(runtime: &Runtime<App>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].bounds
}

fn tab(runtime: &mut Runtime<App>, reverse: bool, expected: &str) {
    assert!(runtime.dispatch(Event::FocusNext { reverse }).unwrap());
    render(runtime);
    assert_eq!(runtime.focused(), Some(expected));
}

fn navigate(runtime: &mut Runtime<App>, key: EditKey, expected: &str) {
    assert!(runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false
        })
        .unwrap());
    render(runtime);
    assert_eq!(runtime.focused(), Some(expected));
}

fn tap(runtime: &mut Runtime<App>, x: f32, y: f32) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    render(runtime);
}

#[test]
fn all_list_styles_keep_row_minimums_gaps_and_exact_resting_corners() {
    for (style, corners, gap, icon_size, end_inset) in [
        (ListStyle::Baseline, [shape::NONE; 3], 0., 24., 24.),
        (ListStyle::Standard, [shape::EXTRA_SMALL; 3], 0., 20., 16.),
        (
            ListStyle::Segmented,
            [
                Corners::new(16., 16., 4., 4.),
                shape::EXTRA_SMALL,
                Corners::new(4., 4., 16., 16.),
            ],
            2.,
            20.,
            16.,
        ),
    ] {
        let runtime = runtime(App::new(style, ListMode::Passive));
        assert_eq!(node(&runtime, "list").role(), Role::List);
        for ((key, height), corners) in ["a", "b", "c"]
            .into_iter()
            .zip([56., 72., 88.])
            .zip(corners)
        {
            let row = bounds(&runtime, key);
            assert_eq!((row.width, row.height), (400., height), "{style:?} {key}");
            let item = node(&runtime, key);
            assert_eq!(item.role(), Role::ListItem);
            assert_eq!(item.style().corner_radii(), corners);
            assert!(!item.is_focusable());
            let leading = bounds(&runtime, &format!("{key}-leading"));
            let trailing = bounds(&runtime, &format!("{key}-trailing"));
            assert_eq!(
                (leading.x - row.x, leading.width, leading.height),
                (16., icon_size, icon_size)
            );
            assert_eq!(row.x + row.width - trailing.x - trailing.width, end_inset);
        }
        assert_eq!(bounds(&runtime, "b").y - bounds(&runtime, "a").y - 56., gap);
        assert_eq!(bounds(&runtime, "c").y - bounds(&runtime, "b").y - 72., gap);
        assert_eq!(
            node(&runtime, "c-headline").style().font_size,
            typography::BODY_LARGE.font_size
        );
        assert_eq!(
            node(&runtime, "c-supporting").style().font_size,
            typography::BODY_MEDIUM.font_size
        );
        assert_eq!(
            node(&runtime, "c-overline").style().font_size,
            typography::LABEL_SMALL.font_size
        );
        let frame = runtime.accessibility_frame().unwrap();
        assert_eq!(
            frame.node(id(&runtime, "c-headline")).unwrap().node.name(),
            "完成"
        );
        assert!(frame.node(id(&runtime, "c-leading")).is_none());
    }
    let mut app = App::new(ListStyle::Segmented, ListMode::Passive);
    app.rows.truncate(1);
    assert_eq!(
        node(&runtime(app), "a").style().corner_radii(),
        shape::LARGE
    );
}

#[test]
fn single_select_separates_focus_activation_and_controlled_selection() {
    let mut app = App::new(
        ListStyle::Segmented,
        ListMode::SingleSelect {
            selected_id: Some("c"),
        },
    );
    app.rows.push("missing");
    let mut runtime = runtime(app);
    assert_eq!(node(&runtime, "list").role(), Role::RadioGroup);
    assert_eq!(node(&runtime, "c").role(), Role::Radio);
    assert_eq!(node(&runtime, "c").name(), "完成");
    assert_eq!(node(&runtime, "c").is_checked(), Some(true));
    assert_eq!(
        node(&runtime, "c").style().background,
        Theme::light().colors.secondary_container
    );
    assert_eq!(node(&runtime, "c").style().corner_radii(), shape::LARGE);
    assert_eq!(node(&runtime, "a").is_checked(), Some(false));
    assert!(node(&runtime, "b").is_disabled());
    assert!(node(&runtime, "missing").is_disabled());
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(
        frame.description_target(id(&runtime, "c")),
        Some(id(&runtime, "c-supporting"))
    );
    assert_eq!(
        frame
            .node(id(&runtime, "c-supporting"))
            .unwrap()
            .node
            .name(),
        "已經審核"
    );
    assert!(frame.node(id(&runtime, "c-headline")).is_none());
    assert!(frame.node(id(&runtime, "c-mark")).is_none());
    assert!(frame.activation_target(id(&runtime, "b")).is_none());
    assert!(frame.focus_target(id(&runtime, "missing")).is_none());
    tab(&mut runtime, false, "before");
    tab(&mut runtime, false, "c");
    navigate(&mut runtime, EditKey::Up, "a");
    navigate(&mut runtime, EditKey::Up, "c");
    navigate(&mut runtime, EditKey::Home, "a");
    navigate(&mut runtime, EditKey::End, "c");
    assert!(runtime.app().presses.is_empty());
    assert_eq!(node(&runtime, "c").is_checked(), Some(true));
    tab(&mut runtime, false, "after");
    tab(&mut runtime, true, "c");
    navigate(&mut runtime, EditKey::Down, "a");
    assert!(runtime.dispatch(Event::Activate).unwrap());
    render(&mut runtime);
    assert_eq!(runtime.app().presses, ["a"]);
    assert_eq!(node(&runtime, "a").is_checked(), Some(true));
    assert_eq!(node(&runtime, "c").is_checked(), Some(false));
    runtime.apply(Message::Reverse).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("a"));
    runtime.apply(Message::Remove("a")).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("c"));
    assert_eq!(runtime.app().presses, ["a"]);
}

#[test]
fn action_uses_one_named_button_and_keeps_gap_and_clipped_content_inert() {
    let mut app = App::new(ListStyle::Segmented, ListMode::Action);
    app.scroll_height = 80.;
    let mut runtime = runtime(app);
    let parent = node(&runtime, "a").parent().unwrap();
    assert_eq!(runtime.dom().get(parent).unwrap().role(), Role::ListItem);
    assert_eq!(node(&runtime, "a").role(), Role::Button);
    assert_eq!(node(&runtime, "a").name(), "收入");
    assert_eq!(node(&runtime, "a").is_checked(), None);
    let a = bounds(&runtime, "a");
    tap(&mut runtime, a.x + 1., a.y + 1.);
    assert_eq!(runtime.app().presses, ["a"]);
    tap(&mut runtime, a.x + 40., a.y + a.height + 1.);
    let c = bounds(&runtime, "c");
    tap(&mut runtime, c.x + 40., c.y + 10.);
    assert_eq!(runtime.app().presses, ["a"]);
    tab(&mut runtime, false, "before");
    tab(&mut runtime, false, "a");
    navigate(&mut runtime, EditKey::Down, "c");
    assert!(runtime.scroll_metrics("scroll").unwrap().offset.1 > 0.);
    let press = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id(&runtime, "c"))
        .unwrap();
    runtime.accessibility_activate(&press).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.app().presses, ["a", "c"]);
}

#[test]
fn compact_rtl_contains_wrapped_copy_and_mirrors_the_typed_slots() {
    let mut app = App::new(ListStyle::Segmented, ListMode::Action);
    app.rows = vec!["long"];
    app.viewport = (320., 480.);
    app.direction = LayoutDirection::RightToLeft;
    app.scale = 2.;
    app.dark = true;
    let runtime = runtime(app);
    let row = bounds(&runtime, "long");
    let headline = bounds(&runtime, "long-headline");
    let supporting = bounds(&runtime, "long-supporting");
    let leading = bounds(&runtime, "long-leading");
    let trailing = bounds(&runtime, "long-trailing");
    assert_eq!(row.width, 320.);
    assert!(row.height > 88.);
    assert!(headline.height > 48.);
    assert!(supporting.height > 40.);
    for child in [headline, supporting, leading, trailing] {
        assert_eq!(child.intersect(row), child);
    }
    assert!(trailing.x + trailing.width < headline.x);
    assert!(headline.x + headline.width < leading.x);
    assert_eq!(row.x + row.width - leading.x - leading.width, 16.);
    assert_eq!(leading.y - row.y, 10.);
    assert_eq!(
        node(&runtime, "long").style().background,
        Theme::dark().colors.surface
    );
}

#[test]
fn expressive_live_corners_and_focus_stroke_share_the_row_geometry() {
    let mut runtime = runtime(App::new(ListStyle::Segmented, ListMode::Action));
    let row = bounds(&runtime, "a");
    let fill = Theme::light().colors.surface;
    let colors = Theme::light().colors;
    let x = row.x + row.width / 2.;
    let y = row.y + row.height / 2.;
    let surface_radius = |list: &DisplayList, radius| {
        assert!(list.ops.iter().any(|op| matches!(op, PaintOp::Quad {
            rect, radius: shown, color, ..
        } if *rect == row && *shown == radius && *color == fill)));
    };
    runtime.dispatch(Event::PointerMove { x, y }).unwrap();
    surface_radius(&render(&mut runtime), 12.);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    surface_radius(&render(&mut runtime), 16.);
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    runtime.dispatch(Event::PointerLeave).unwrap();
    tab(&mut runtime, false, "after");
    tab(&mut runtime, true, "a");
    let list = render(&mut runtime);
    surface_radius(&list, 16.);
    assert!(list.ops.iter().any(|op| matches!(op, PaintOp::Stroke {
        rect, clip, radius: 16., width: 3., color,
    } if *rect == row && *clip == bounds(&runtime, "scroll") && *color == colors.secondary)));
    let Color(r, g, b, _) = colors.on_surface;
    assert!(list.ops.iter().any(|op| matches!(op, PaintOp::Quad {
        rect, radius: 16., color, ..
    } if *rect == row && *color == Color(r, g, b, 26))));
}
