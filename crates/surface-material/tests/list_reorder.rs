use surface::{Length, Node, NodeId, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityActivationResult, AnimationFrame, AnimationUpdate, Application, EditKey, Event,
    Key, KeyPress, Modifiers, Queue, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, ListItem, ListLines, ListReorderEntry, ListReorderInput,
    ListReorderRequest, ListReorderState, ListStyle, ReorderableListItem, ReorderableListOptions,
    Theme,
};

const LIST: &str = "list";
const SCROLL: &str = "scroll";
const LIFT: &str = "list-lift";
const DROP: &str = "list-drop";
const SHORT_ORDER: [&str; 5] = ["a", "b", "fixed", "j", "k"];
const LONG_ORDER: [&str; 11] = ["a", "b", "c", "d", "e", "f", "g", "h", "fixed", "j", "k"];

#[derive(Clone)]
enum Message {
    Reorder(ListReorderInput),
    Open(&'static str),
    ExternalSwap,
}

struct Fixture {
    order: Vec<&'static str>,
    revision: u64,
    state: ListReorderState,
    queue: Option<Queue<Message>>,
    requests: Vec<ListReorderRequest>,
    opened: Vec<&'static str>,
    accept_requests: bool,
    style: ListStyle,
    direction: LayoutDirection,
    text_scale: f32,
    width: f32,
    height: f32,
}

impl Fixture {
    fn new(order: &[&'static str], width: f32, height: f32) -> Self {
        Self {
            order: order.to_vec(),
            revision: 0,
            state: ListReorderState::new(),
            queue: None,
            requests: Vec::new(),
            opened: Vec::new(),
            accept_requests: true,
            style: ListStyle::Segmented,
            direction: LayoutDirection::LeftToRight,
            text_scale: 1.,
            width,
            height,
        }
    }

    fn options(&self) -> ReorderableListOptions<'static> {
        ReorderableListOptions {
            id: LIST,
            name: "旅程資料",
            scroller_id: SCROLL,
            style: self.style,
            direction: self.direction,
            available_width: self.width,
            revision: self.revision,
        }
    }

    fn entries(&self) -> Vec<ListReorderEntry<'static>> {
        self.order
            .iter()
            .map(|&id| ListReorderEntry {
                id,
                movable: id != "fixed",
            })
            .collect()
    }

    fn apply_request(&mut self, request: &ListReorderRequest) {
        let position = |id| self.order.iter().position(|row| *row == id).unwrap();
        let from = position(request.item.as_str());
        let before = request.before.as_deref().map_or(self.order.len(), position);
        let to = if before > from { before - 1 } else { before };
        let crossed = &mut self.order[from.min(to)..=from.max(to)];
        assert_ne!(from, to, "the component emitted a no-op request");
        assert!(
            !crossed.contains(&"fixed"),
            "the request crossed the barrier"
        );
        if from < to {
            crossed.rotate_left(1);
        } else {
            crossed.rotate_right(1);
        }
        self.revision += 1;
    }
}

impl Application for Fixture {
    type Message = Message;

    fn attach_queue(&mut self, queue: Queue<Message>) {
        self.queue = Some(queue);
    }

    fn text_scale(&self) -> f32 {
        self.text_scale
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Reorder(input) => {
                let entries = self.entries();
                if let Some(request) = self.state.update(input, self.revision, &entries) {
                    if self.accept_requests {
                        self.apply_request(&request);
                    }
                    self.requests.push(request);
                }
            }
            Message::Open(id) => self.opened.push(id),
            Message::ExternalSwap => {
                let len = self.order.len();
                self.order.swap(len - 2, len - 1);
                self.revision += 1;
            }
        }
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        let options = self.options();
        let entries = self.entries();
        self.state
            .advance(options, &entries, &frame, self.queue.as_ref())
    }

    fn view(&self) -> View<Message> {
        let items = self.order.iter().map(|&id| {
            let movable = id != "fixed";
            ReorderableListItem {
                row: ListItem {
                    id,
                    headline: if id == "b" {
                        "旅程計畫和離線路線"
                    } else {
                        "資料"
                    },
                    overline: (id == "c").then_some("備用"),
                    supporting: match id {
                        "b" => Some("出發前檢查地圖"),
                        "c" => Some("保存在手機"),
                        _ => None,
                    },
                    leading: None,
                    trailing: None,
                    lines: match id {
                        "b" => ListLines::Two,
                        "c" => ListLines::Three,
                        _ => ListLines::One,
                    },
                    disabled: !movable,
                    on_activate: movable.then_some(Message::Open(id)),
                },
                movable,
                move_up: "上移",
                move_down: "下移",
            }
        });
        let list =
            Theme::dark().reorderable_list(self.options(), items, &self.state, Message::Reorder);
        let scroll = View::new(
            Node::new(Role::Group, "清單捲動區").id(SCROLL).styled(
                Style::column()
                    .width(Length::Px(self.width))
                    .height(Length::Px(self.height))
                    .clip(true),
            ),
        )
        .accessibility_scroll_pages()
        .children(list);
        View::new(Node::new(Role::Window, "排列順序").styled(Style::column())).children([scroll])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let natural = text.chars().count() as f32 * size;
        let width = width.map_or(natural, |width| natural.min(width));
        let lines = (natural / width.max(1.)).ceil().max(1.);
        (width, size * 1.5 * lines)
    }
}

fn render(runtime: &mut Runtime<Fixture>, time_ms: f64) {
    let (width, height) = (runtime.app().width, runtime.app().height);
    runtime.set_time(time_ms);
    runtime
        .render_extended(width, height, &mut Metrics)
        .unwrap();
}

fn runtime(app: Fixture) -> Runtime<Fixture> {
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime, 0.);
    runtime
}

fn id(runtime: &Runtime<Fixture>, key: &str) -> NodeId {
    runtime
        .dom()
        .by_id(key)
        .unwrap_or_else(|| panic!("missing {key}"))
}

fn node<'a>(runtime: &'a Runtime<Fixture>, key: &str) -> &'a Node {
    runtime.dom().get(id(runtime, key)).unwrap()
}

fn bounds(runtime: &Runtime<Fixture>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].bounds
}

fn clip(runtime: &Runtime<Fixture>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].clip
}

fn send(runtime: &mut Runtime<Fixture>, event: Event) {
    runtime.dispatch(event).unwrap();
}

fn pointer(runtime: &mut Runtime<Fixture>, element: &str) -> (f32, f32) {
    let (x, y) = center(bounds(runtime, element));
    send(runtime, Event::PointerDown { x, y });
    (x, y)
}

fn center(bounds: Rect) -> (f32, f32) {
    (bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.)
}

fn bottom(bounds: Rect) -> f32 {
    bounds.y + bounds.height
}

fn request(item: &str, before: Option<&str>) -> ListReorderRequest {
    ListReorderRequest {
        item: item.to_owned(),
        before: before.map(str::to_owned),
    }
}

#[test]
fn discrete_moves_stop_at_the_fixed_row_and_keep_focus_on_the_same_item() {
    let mut runtime = runtime(Fixture::new(&SHORT_ORDER, 480., 480.));
    for key in [
        "a-up",
        "b-down",
        "fixed",
        "fixed-up",
        "fixed-down",
        "fixed-grip",
        "j-up",
    ] {
        let item = node(&runtime, key);
        assert!(item.is_disabled(), "{key} must be disabled");
        assert!(
            !item.is_focusable(),
            "{key} must stay out of focus navigation"
        );
        let frame = runtime.accessibility_frame().unwrap();
        assert!(
            frame.activation_target(id(&runtime, key)).is_none(),
            "{key}"
        );
    }
    send(&mut runtime, Event::FocusNext { reverse: false });
    assert_eq!(runtime.focused(), Some("a"));
    send(
        &mut runtime,
        Event::Edit {
            key: EditKey::Down,
            extend: false,
            by_word: false,
        },
    );
    assert_eq!(runtime.focused(), Some("a-down"));

    let enter = KeyPress::new(Key::Enter, Modifiers::default());
    assert!(!runtime.key(&enter).unwrap());
    send(&mut runtime, Event::Activate);
    render(&mut runtime, 16.);
    assert_eq!(runtime.app().requests, [request("a", Some("fixed"))]);
    assert_eq!(runtime.app().order, ["b", "a", "fixed", "j", "k"]);
    assert_eq!(runtime.focused(), Some("a-up"));
    assert!(node(&runtime, "a-down").is_disabled());
    assert!(runtime.key(&enter.with_repeat(true)).unwrap());
    assert_eq!(runtime.app().requests.len(), 1);

    let move_j = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id(&runtime, "j-down"))
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&move_j).unwrap(),
        AccessibilityActivationResult::Activated
    );
    render(&mut runtime, 32.);
    assert_eq!(
        runtime.app().requests,
        [request("a", Some("fixed")), request("j", None)]
    );
    assert_eq!(runtime.app().order, ["b", "a", "fixed", "k", "j"]);
    let y = center(bounds(&runtime, "fixed-item")).1;
    let (x, _) = pointer(&mut runtime, "a-grip");
    send(&mut runtime, Event::PointerUp { x, y });
    assert_eq!(runtime.app().requests.len(), 2);
    assert!(runtime.app().opened.is_empty());
}

#[test]
fn release_uses_the_offset_of_the_last_rendered_edge_scroll_without_another_move() {
    let mut runtime = runtime(Fixture::new(&LONG_ORDER, 480., 172.));
    let (x, _) = pointer(&mut runtime, "a-grip");
    let edge_y = bottom(bounds(&runtime, SCROLL)) - 4.;
    assert!(edge_y < center(bounds(&runtime, "c-item")).1);
    assert!(edge_y > center(bounds(&runtime, "b-item")).1);
    send(&mut runtime, Event::PointerMove { x, y: edge_y });
    render(&mut runtime, 16.);
    assert_eq!(runtime.app().order, LONG_ORDER);
    assert!(runtime.app().requests.is_empty());
    assert!(runtime.dom().by_id(LIFT).is_some());
    assert!(runtime.dom().by_id(DROP).is_some());
    let lift = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id(&runtime, LIFT));
    assert!(lift.is_none());

    let mut crossed_c = false;
    for frame in 2..=12 {
        let previous_c = center(bounds(&runtime, "c-item")).1;
        let previous_offset = runtime.scroll_metrics(SCROLL).unwrap().offset.1;
        render(&mut runtime, f64::from(frame) * 16.);
        let offset = runtime.scroll_metrics(SCROLL).unwrap().offset.1;
        if center(bounds(&runtime, "c-item")).1 < edge_y {
            assert!(previous_c >= edge_y);
            assert!(offset > previous_offset);
            assert!(edge_y < center(bounds(&runtime, "d-item")).1);
            crossed_c = true;
            break;
        }
    }
    assert!(
        crossed_c,
        "edge scrolling did not move the release across C's center"
    );
    assert!(runtime.app().requests.is_empty());

    // The release moves A before D. Using the previous frame would move it before C.
    send(&mut runtime, Event::PointerUp { x, y: edge_y });
    assert_eq!(runtime.app().requests, [request("a", Some("d"))]);
    assert_eq!(&runtime.app().order[..4], ["b", "c", "a", "d"]);
    assert_eq!(runtime.app().revision, 1);
    assert!(runtime.app().opened.is_empty());
    assert!(runtime.dom().by_id(LIFT).is_none());
    assert!(runtime.dom().by_id(DROP).is_none());
}

#[test]
fn grip_tap_is_inert_and_touch_release_alone_proposes_one_caller_owned_move() {
    let mut app = Fixture::new(&LONG_ORDER, 480., 172.);
    app.accept_requests = false;
    let mut runtime = runtime(app);
    let (x, origin) = center(bounds(&runtime, "a-grip"));
    runtime
        .dispatch_touch(Event::PointerDown { x, y: origin })
        .unwrap();
    runtime
        .dispatch_touch(Event::PointerUp { x, y: origin })
        .unwrap();
    assert!(runtime.app().requests.is_empty());
    assert!(runtime.app().opened.is_empty());

    let y = center(bounds(&runtime, "b-item")).1 + 12.;
    runtime
        .dispatch_touch(Event::PointerDown { x, y: origin })
        .unwrap();
    runtime.dispatch_touch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.app().requests, [request("a", Some("c"))]);
    assert_eq!(runtime.app().order, LONG_ORDER);
    assert_eq!(runtime.app().revision, 0);
    assert!(runtime.dom().by_id(LIFT).is_none());

    render(&mut runtime, 16.);
    let (x, y) = pointer(&mut runtime, "a");
    send(&mut runtime, Event::PointerUp { x, y });
    assert_eq!(runtime.app().opened, ["a"]);
    assert_eq!(runtime.app().requests.len(), 1);
}

#[test]
fn cancelled_or_superseded_contacts_cannot_move_rows_and_a_new_contact_can() {
    let mut runtime = runtime(Fixture::new(&LONG_ORDER, 480., 172.));
    let (x, _) = pointer(&mut runtime, "a-grip");
    let y = center(bounds(&runtime, "b-item")).1 + 12.;
    send(&mut runtime, Event::PointerMove { x, y });
    render(&mut runtime, 16.);
    assert!(runtime.dom().by_id(LIFT).is_some());
    send(&mut runtime, Event::Cancel);
    send(&mut runtime, Event::PointerUp { x, y });
    render(&mut runtime, 32.);
    assert!(runtime.dom().by_id(LIFT).is_none());
    assert_eq!(runtime.app().order, LONG_ORDER);
    assert!(runtime.app().requests.is_empty());

    let (x, _) = pointer(&mut runtime, "a-grip");
    send(&mut runtime, Event::PointerMove { x, y });
    render(&mut runtime, 48.);
    assert!(runtime.dom().by_id(LIFT).is_some());
    runtime.apply(Message::ExternalSwap).unwrap();
    // Release before drawing the new order. The old contact cannot apply to it.
    send(&mut runtime, Event::PointerUp { x, y });
    render(&mut runtime, 64.);
    assert!(runtime.dom().by_id(LIFT).is_none());
    assert!(runtime.app().requests.is_empty());
    assert_eq!(&runtime.app().order[..9], &LONG_ORDER[..9]);
    assert_eq!(&runtime.app().order[9..], ["k", "j"]);
    assert_eq!(runtime.app().revision, 1);

    let (x, _) = pointer(&mut runtime, "a-grip");
    send(&mut runtime, Event::PointerUp { x, y });
    assert_eq!(runtime.app().requests, [request("a", Some("c"))]);
    assert_eq!(runtime.app().revision, 2);
    assert!(runtime.app().opened.is_empty());
}

#[test]
fn compact_rtl_large_text_keeps_all_three_targets_separate_and_visible() {
    for style in [
        ListStyle::Baseline,
        ListStyle::Standard,
        ListStyle::Segmented,
    ] {
        let mut app = Fixture::new(&SHORT_ORDER, 320., 520.);
        app.style = style;
        app.direction = LayoutDirection::RightToLeft;
        app.text_scale = 2.;
        let runtime = runtime(app);
        let row = bounds(&runtime, "b-item");
        let primary = bounds(&runtime, "b");
        let up = bounds(&runtime, "b-up");
        let down = bounds(&runtime, "b-down");
        let grip = bounds(&runtime, "b-grip");
        assert_eq!(row.width, 320.);
        assert!(primary.height > 72.);
        assert!(bottom(primary) <= up.y && bottom(primary) <= down.y && bottom(primary) <= grip.y);
        assert!(grip.x + grip.width <= down.x && down.x + down.width <= up.x);
        for key in ["b-up", "b-down", "b-grip"] {
            let target = bounds(&runtime, key);
            assert!(
                target.width >= 48. && target.height >= 48.,
                "{style:?} {key}: {target:?}"
            );
            assert_eq!(target.intersect(row).intersect(clip(&runtime, key)), target);
        }
        assert_eq!(node(&runtime, "b-up").name(), "上移");
        assert_eq!(node(&runtime, "b-down").name(), "下移");
        assert!(!node(&runtime, "b-grip").is_focusable());
    }
}
