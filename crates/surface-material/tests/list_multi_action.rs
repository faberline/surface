use surface::{DisplayList, Length, Node, NodeId, PaintOp, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityActivationResult, Application, EditKey, Event, Key, KeyPress, Modifiers, Runtime,
    View,
};
use surface_material::{
    shape::LayoutDirection, ListLines, ListStyle, ListSupplementaryAction, MultiActionListItem,
    MultiActionListOptions, Theme, VectorIcon,
};

const LONG_TITLE: &str = "最完整的專案資料會自動換行";
const LONG_ACTION: &str = "查看這個專案的更多操作";

#[derive(Clone)]
enum Message {
    Activate(&'static str),
    Width(f32),
    Remove(&'static str),
}

struct Demo {
    style: ListStyle,
    width: f32,
    height: f32,
    direction: LayoutDirection,
    large: bool,
    a: Vec<&'static str>,
    b: Vec<&'static str>,
    b_disabled: bool,
    shown: Vec<&'static str>,
}

impl Demo {
    fn new(width: f32, style: ListStyle, direction: LayoutDirection) -> Self {
        Self {
            style,
            width,
            height: 640.,
            direction,
            large: false,
            a: vec!["bookmark", "more"],
            b: Vec::new(),
            b_disabled: false,
            shown: Vec::new(),
        }
    }
}

impl Application for Demo {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Activate(id) => self.shown.push(id),
            Message::Width(width) => self.width = width,
            Message::Remove(id) => {
                self.a.retain(|action| *action != id);
                self.b.retain(|action| *action != id);
            }
        }
    }

    fn text_scale(&self) -> f32 {
        if self.large {
            2.
        } else {
            1.
        }
    }

    fn view(&self) -> View<Message> {
        let icon = VectorIcon::new([[12., 3.], [21., 21.], [3., 21.]]).unwrap();
        let row = |id, disabled, actions: &[&'static str]| MultiActionListItem {
            id,
            headline: if self.large { LONG_TITLE } else { "專案" },
            overline: self.large.then_some("全部"),
            supporting: Some("最近更新"),
            leading: Some(&icon),
            lines: ListLines::Two,
            disabled,
            on_activate: Some(Message::Activate(id)),
            actions: actions
                .iter()
                .map(|&id| ListSupplementaryAction {
                    id,
                    label: LONG_ACTION,
                    icon: &icon,
                    checked: (id == "bookmark").then_some(true),
                    disabled: id == "disabled",
                    on_activate: (id != "missing").then_some(Message::Activate(id)),
                })
                .collect(),
        };
        let rows = std::iter::once(row("primary", false, &self.a))
            .chain((!self.b.is_empty()).then(|| row("next", self.b_disabled, &self.b)));
        let theme = if self.large {
            Theme::dark()
        } else {
            Theme::light()
        };
        let list = theme.multi_action_list(
            MultiActionListOptions {
                id: "list",
                name: "專案動作",
                style: self.style,
                direction: self.direction,
                available_width: self.width,
            },
            rows,
        );
        let boundary = |key| {
            View::new(
                Node::new(Role::Button, key)
                    .id(key)
                    .styled(Style::column().height(Length::Px(48.))),
            )
            .on_press(Message::Activate(key))
        };
        View::new(Node::new(Role::Window, "").styled(Style::column())).children([
            boundary("before"),
            View::new(
                Node::new(Role::Group, "").id("scroll").styled(
                    Style::column()
                        .height(Length::Px(self.height - 96.))
                        .clip(true),
                ),
            )
            .children(list)
            .accessibility_scroll_pages(),
            boundary("after"),
        ])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, label: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let natural_width = label.chars().count() as f32 * size;
        let width = width.map_or(natural_width, |available| natural_width.min(available));
        let chars_per_line = (width / size).floor().max(1.);
        let lines = (label.chars().count() as f32 / chars_per_line)
            .ceil()
            .max(1.);
        (width, size * 1.5 * lines)
    }
}

fn render(runtime: &mut Runtime<Demo>) -> DisplayList {
    let width = runtime.app().width;
    let height = runtime.app().height;
    runtime.render(width, height, &mut Metrics).unwrap()
}

fn runtime(app: Demo) -> Runtime<Demo> {
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime);
    runtime
}

fn id(runtime: &Runtime<Demo>, key: &str) -> NodeId {
    runtime.dom().by_id(key).unwrap()
}

fn node<'a>(runtime: &'a Runtime<Demo>, key: &str) -> &'a Node {
    runtime.dom().get(id(runtime, key)).unwrap()
}

fn bounds(runtime: &Runtime<Demo>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].bounds
}

fn route(runtime: &mut Runtime<Demo>, event: Event, expected: &str) {
    assert!(runtime.dispatch(event).unwrap());
    render(runtime);
    assert_eq!(runtime.focused(), Some(expected));
}

fn tab(runtime: &mut Runtime<Demo>, reverse: bool, expected: &str) {
    route(runtime, Event::FocusNext { reverse }, expected);
}

fn arrow(runtime: &mut Runtime<Demo>, key: EditKey, expected: &str) {
    route(
        runtime,
        Event::Edit {
            key,
            extend: false,
            by_word: false,
        },
        expected,
    );
}

fn center(rect: Rect) -> (f32, f32) {
    (rect.x + rect.width / 2., rect.y + rect.height / 2.)
}

fn tap(runtime: &mut Runtime<Demo>, point: (f32, f32)) {
    let (x, y) = point;
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    render(runtime);
}

#[test]
fn two_inline_actions_leave_the_widest_slot_for_wrapped_copy_in_both_directions() {
    for (style, primary_width, copy_width, trailing_x, icon_size) in [
        (ListStyle::Baseline, 190., 128., [196., 248.], 24.),
        (ListStyle::Standard, 202., 144., [204., 256.], 20.),
        (ListStyle::Segmented, 202., 144., [204., 256.], 20.),
    ] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let mut app = Demo::new(320., style, direction);
            app.large = true;
            let runtime = runtime(app);
            let row = bounds(&runtime, "primary-item");
            assert_eq!(row.width, 320.);
            assert!(row.height > 88.);
            let mirror = |x, width| {
                if direction == LayoutDirection::RightToLeft {
                    row.width - x - width
                } else {
                    x
                }
            };
            let primary = bounds(&runtime, "primary");
            assert_eq!(
                primary,
                Rect::new(
                    row.x + mirror(0., primary_width),
                    row.y,
                    primary_width,
                    row.height
                )
            );
            let headline = bounds(&runtime, "primary-headline");
            assert!(headline.height > 48.);
            assert_eq!(headline.width, copy_width);
            assert_eq!(headline.intersect(primary), headline);
            for (action, x) in ["bookmark", "more"].into_iter().zip(trailing_x) {
                let target = bounds(&runtime, action);
                assert_eq!(
                    target,
                    Rect::new(row.x + mirror(x, 48.), row.y, 48., row.height)
                );
                let icon = bounds(&runtime, &format!("{action}-icon"));
                assert_eq!((icon.width, icon.height), (icon_size, icon_size));
                assert_eq!(icon.intersect(target), icon);
                assert_eq!(target.intersect(primary).width, 0.);
            }
            let frame = runtime.accessibility_frame().unwrap();
            assert_eq!(node(&runtime, "list").role(), Role::List);
            assert_eq!(node(&runtime, "primary-item").role(), Role::ListItem);
            assert!(!node(&runtime, "primary-item").is_focusable());
            assert_eq!(node(&runtime, "primary-item").is_checked(), None);
            for action in ["primary", "bookmark", "more"] {
                let button = node(&runtime, action);
                assert_eq!(button.role(), Role::Button);
                assert_eq!(button.parent(), Some(id(&runtime, "primary-item")));
                assert!(frame.activation_target(id(&runtime, action)).is_some());
            }
            assert_eq!(node(&runtime, "bookmark").is_checked(), Some(true));
            assert_eq!(node(&runtime, "more").name(), LONG_ACTION);
            assert_eq!(node(&runtime, "primary").name(), LONG_TITLE);
            assert!(frame.node(id(&runtime, "more-icon")).is_none());
            assert_eq!(
                frame.description_target(id(&runtime, "primary")),
                Some(id(&runtime, "primary-supporting"))
            );
        }
    }
}

#[test]
fn adaptive_layout_wraps_sibling_targets_and_keeps_focused_identity() {
    let mut app = Demo::new(400., ListStyle::Baseline, LayoutDirection::RightToLeft);
    app.a.push("archive");
    let mut runtime = runtime(app);
    tab(&mut runtime, false, "before");
    tab(&mut runtime, false, "primary");
    arrow(&mut runtime, EditKey::End, "archive");
    assert_eq!(bounds(&runtime, "archive").y, bounds(&runtime, "primary").y);

    runtime.apply(Message::Width(320.)).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("archive"));
    let primary = bounds(&runtime, "primary");
    let row = bounds(&runtime, "primary-item");
    for key in ["bookmark", "more", "archive"] {
        let target = bounds(&runtime, key);
        assert!(target.y >= primary.y + primary.height);
        assert_eq!((target.width, target.height), (48., 48.));
        assert_eq!(target.intersect(row), target);
        assert_eq!(
            node(&runtime, key).parent(),
            Some(id(&runtime, "primary-item"))
        );
    }
    assert!(bounds(&runtime, "archive").x < bounds(&runtime, "more").x);
    assert!(bounds(&runtime, "more").x < bounds(&runtime, "bookmark").x);
    runtime.apply(Message::Remove("archive")).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("primary"));
    runtime.apply(Message::Width(288.)).unwrap();
    render(&mut runtime);
    assert!(bounds(&runtime, "bookmark").y > bounds(&runtime, "primary").y);
    assert!(runtime.app().shown.is_empty());
}

#[test]
fn widths_from_48_to_100_keep_the_copy_and_each_wrapped_action_inside_the_item() {
    for width in [48., 64., 100.] {
        let mut app = Demo::new(width, ListStyle::Segmented, LayoutDirection::RightToLeft);
        app.a.extend(["archive", "other"]);
        app.large = true;
        app.height = 1800.;
        let runtime = runtime(app);
        let row = bounds(&runtime, "primary-item");
        let primary = bounds(&runtime, "primary");
        let headline = bounds(&runtime, "primary-headline");
        let leading = bounds(&runtime, "primary-leading");
        assert_eq!(row.width, width);
        assert!(primary.width >= 48.);
        assert!(headline.width >= 32. && headline.height > 48.);
        assert_eq!(headline.intersect(primary), headline);
        assert!(headline.y >= leading.y + leading.height);
        let mut earlier = Vec::new();
        for key in ["bookmark", "more", "archive", "other"] {
            let action = bounds(&runtime, key);
            assert_eq!((action.width, action.height), (48., 48.));
            assert_eq!(action.intersect(row), action);
            assert!(action.y >= primary.y + primary.height);
            assert!(earlier
                .iter()
                .all(|other| action.intersect(*other).width == 0.
                    || action.intersect(*other).height == 0.));
            earlier.push(action);
        }
    }
    for width in [47., f32::NAN, f32::INFINITY] {
        let result = Theme::light().multi_action_list::<()>(
            MultiActionListOptions {
                id: "list",
                name: "",
                style: ListStyle::Baseline,
                direction: LayoutDirection::LeftToRight,
                available_width: width,
            },
            [],
        );
        assert!(result.is_none());
    }
}

#[test]
fn all_four_arrows_skip_disabled_actions_and_only_activation_sends_a_message() {
    let mut app = Demo::new(320., ListStyle::Segmented, LayoutDirection::RightToLeft);
    app.b = vec!["disabled", "last", "missing"];
    app.b_disabled = true;
    let mut runtime = runtime(app);
    for key in ["next", "disabled", "missing"] {
        assert!(node(&runtime, key).is_disabled());
        assert!(runtime
            .accessibility_frame()
            .unwrap()
            .activation_target(id(&runtime, key))
            .is_none());
    }
    tab(&mut runtime, false, "before");
    tab(&mut runtime, false, "primary");
    arrow(&mut runtime, EditKey::Right, "bookmark");
    arrow(&mut runtime, EditKey::Down, "more");
    arrow(&mut runtime, EditKey::Right, "last");
    arrow(&mut runtime, EditKey::Down, "primary");
    arrow(&mut runtime, EditKey::Up, "last");
    arrow(&mut runtime, EditKey::Left, "more");
    assert!(runtime.app().shown.is_empty());

    let focused = bounds(&runtime, "more");
    let paint = render(&mut runtime);
    assert!(paint.ops.iter().any(|op| matches!(op, PaintOp::Stroke {
        rect, width: 3., color, ..
    } if *rect == focused && *color == Theme::light().colors.secondary)));
    let enter = KeyPress::new(Key::Enter, Modifiers::default());
    assert!(!runtime.key(&enter).unwrap());
    assert!(runtime.dispatch(Event::Activate).unwrap());
    assert!(runtime.key(&enter.with_repeat(true)).unwrap());
    assert_eq!(runtime.app().shown, ["more"]);
    tab(&mut runtime, false, "after");
    tab(&mut runtime, true, "primary");

    let last = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id(&runtime, "last"))
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&last).unwrap(),
        AccessibilityActivationResult::Activated
    );
    render(&mut runtime);
    assert_eq!(runtime.app().shown, ["more", "last"]);
}

#[test]
fn pointer_targets_and_gaps_do_not_route_into_neighbor_actions() {
    let mut app = Demo::new(320., ListStyle::Standard, LayoutDirection::LeftToRight);
    app.a = vec!["disabled", "more"];
    let mut runtime = runtime(app);
    let primary = bounds(&runtime, "primary");
    let more = bounds(&runtime, "more");
    let disabled = bounds(&runtime, "disabled");
    tap(&mut runtime, center(disabled));
    tap(
        &mut runtime,
        (primary.x + primary.width + 1., center(primary).1),
    );
    let (x, y) = center(primary);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    let (x, y) = center(more);
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    render(&mut runtime);
    assert!(runtime.app().shown.is_empty());

    tap(&mut runtime, center(primary));
    tap(&mut runtime, center(more));
    assert_eq!(runtime.app().shown, ["primary", "more"]);
}
