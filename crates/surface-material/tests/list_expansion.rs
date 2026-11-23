use surface::{Corners, Length, Node, NodeId, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityActivationResult, AccessibilityFocusResult, AnimationFrame, AnimationUpdate,
    Application, EditKey, Event, Runtime, ScrollTarget, View,
};
use surface_material::{
    shape::LayoutDirection, ExpandableListOptions, ExpandableListRow, ListExpansionMotion,
    ListItem, ListLines, ListStyle, Theme,
};

const BEFORE: &str = "before";
const HEADER: &str = "folder";
const BRANCH: &str = "folder-branch";
const CONTENT: &str = "folder-expansion-content";
const INDICATOR: &str = "folder-indicator";
const FIRST: &str = "child-first";
const DISABLED: &str = "child-disabled";
const LAST: &str = "child-last";
const AFTER: &str = "after";
const SCROLLER: &str = "list-scroll";
const MODAL_CLOSE: &str = "cover-close";
const LONG_TITLE: &str = "這個很長的專案名稱到了窄畫面會按照可用寬度自動換行";

#[derive(Clone)]
enum Message {
    Toggle,
    Open(bool),
    Press(&'static str),
    TextScale(f32),
    CoverAndClose,
    DismissCover,
}

struct Fixture {
    style: ListStyle,
    direction: LayoutDirection,
    expanded: bool,
    header_disabled: bool,
    cover: bool,
    motion: ListExpansionMotion,
    text_scale: f32,
    scroll_height: f32,
    presses: Vec<&'static str>,
}

impl Fixture {
    fn new(style: ListStyle, expanded: bool) -> Self {
        Self {
            style,
            direction: LayoutDirection::LeftToRight,
            expanded,
            header_disabled: false,
            cover: false,
            motion: ListExpansionMotion::new(expanded),
            text_scale: 1.,
            scroll_height: 590.,
            presses: Vec::new(),
        }
    }
}

impl Application for Fixture {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => self.expanded = !self.expanded,
            Message::Open(open) => self.expanded = open,
            Message::Press(id) => self.presses.push(id),
            Message::TextScale(scale) => self.text_scale = scale,
            Message::CoverAndClose => {
                self.expanded = false;
                self.cover = true;
            }
            Message::DismissCover => self.cover = false,
        }
    }

    fn text_scale(&self) -> f32 {
        self.text_scale
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        self.motion.advance(HEADER, self.expanded, &frame)
    }

    fn view(&self) -> View<Message> {
        let list = Theme::light().expandable_list(
            ExpandableListOptions {
                id: "work-list",
                name: "工作清單",
                style: self.style,
                direction: self.direction,
            },
            [
                ExpandableListRow::Leaf(item(BEFORE, "最早", Message::Press(BEFORE), false)),
                ExpandableListRow::Branch {
                    header: item(HEADER, "專案資料夾", Message::Toggle, self.header_disabled),
                    children: vec![
                        item(FIRST, "第一個子項目", Message::Press(FIRST), false),
                        item(DISABLED, "暫停的專案", Message::Press(DISABLED), true),
                        item(LAST, LONG_TITLE, Message::Press(LAST), false),
                    ],
                    expanded: self.expanded,
                    motion: &self.motion,
                },
                ExpandableListRow::Leaf(item(AFTER, "最後一列", Message::Press(AFTER), false)),
            ],
        );
        let body = View::new(
            Node::new(Role::Group, "工作清單捲動區")
                .id(SCROLLER)
                .styled(
                    Style::column()
                        .height(Length::Px(self.scroll_height))
                        .clip(true),
                ),
        )
        .children([list])
        .accessibility_scroll_pages();
        let cover = self.cover.then(|| {
            View::new(
                Node::new(Role::Dialog, "Other scope")
                    .id("cover")
                    .styled(Style::column().height(Length::Px(70.))),
            )
            .children([View::new(
                Node::new(Role::Button, "Close")
                    .id(MODAL_CLOSE)
                    .styled(Style::row().height(Length::Px(48.))),
            )
            .on_press(Message::DismissCover)
            .accessible_activation()])
            .modal_overlay(Message::DismissCover)
            .modal_initial_focus(MODAL_CLOSE)
        });
        View::new(Node::new(Role::Window, "Expandable list"))
            .children(std::iter::once(body).chain(cover))
    }
}

fn item(
    id: &'static str,
    headline: &'static str,
    on_activate: Message,
    disabled: bool,
) -> ListItem<'static, Message> {
    ListItem {
        id,
        headline,
        overline: None,
        supporting: (id == LAST).then_some("完整說明會一起保留"),
        leading: None,
        trailing: None,
        lines: if id == LAST {
            ListLines::Two
        } else {
            ListLines::One
        },
        disabled,
        on_activate: Some(on_activate),
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, value: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let natural_width = value.chars().count() as f32 * size;
        let line_width = width.map_or(natural_width, |width| width.min(natural_width));
        let lines = (natural_width / line_width.max(1.)).ceil().max(1.);
        (line_width, size * 1.5 * lines)
    }
}

fn render(runtime: &mut Runtime<Fixture>, width: f32) {
    runtime.render_extended(width, 720., &mut Metrics).unwrap();
}

fn tick(runtime: &mut Runtime<Fixture>, at_ms: f64) {
    runtime.set_time(at_ms);
    render(runtime, 760.);
}

fn id(runtime: &Runtime<Fixture>, key: &str) -> NodeId {
    runtime
        .dom()
        .by_id(key)
        .unwrap_or_else(|| panic!("no {key}"))
}

fn node<'a>(runtime: &'a Runtime<Fixture>, key: &str) -> &'a Node {
    runtime.dom().get(id(runtime, key)).unwrap()
}

fn bounds(runtime: &Runtime<Fixture>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].bounds
}

fn focus_next(runtime: &mut Runtime<Fixture>, key: EditKey, expected: &str) {
    assert!(runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false,
        })
        .unwrap());
    assert_eq!(runtime.focused(), Some(expected));
}

fn tab(runtime: &mut Runtime<Fixture>, reverse: bool, expected: &str) {
    assert!(runtime.dispatch(Event::FocusNext { reverse }).unwrap());
    assert_eq!(runtime.focused(), Some(expected));
}

fn close_enough(a: f32, b: f32) {
    assert!((a - b).abs() <= 1., "{a} differs from {b}");
}

fn assert_disclosure(runtime: &Runtime<Fixture>, expanded: bool) {
    let header = node(runtime, HEADER);
    assert_eq!(header.role(), Role::Button);
    assert_eq!(header.name(), "專案資料夾");
    assert_eq!(header.expanded_state(), Some(expanded));
    assert!(!header.has_menu_popup());
    assert_eq!(header.parent(), Some(id(runtime, BRANCH)));
    assert_eq!(node(runtime, BRANCH).role(), Role::ListItem);
}

fn assert_inert(runtime: &mut Runtime<Fixture>, key: &str) {
    let mounted = id(runtime, key);
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.node(mounted).is_none());
    assert!(frame.focus_target(mounted).is_none());
    assert!(frame.activation_target(mounted).is_none());
    assert!(runtime.activate(key).is_err());
}

#[test]
fn endpoints_are_inline_and_real_children_mount_in_each_style() {
    let colors = Theme::light().colors;
    for style in [
        ListStyle::Baseline,
        ListStyle::Standard,
        ListStyle::Segmented,
    ] {
        let mut runtime = Runtime::new(Fixture::new(style, false)).unwrap();
        render(&mut runtime, 760.);
        assert_disclosure(&runtime, false);
        assert!(!runtime.needs_frame());
        assert!(runtime.dom().by_id(CONTENT).is_none());
        assert!(runtime.dom().by_id(FIRST).is_none());
        assert_eq!(node(&runtime, "work-list").role(), Role::List);
        assert_eq!(
            node(&runtime, INDICATOR).style().corner_radii(),
            Corners::full()
        );
        assert_eq!(node(&runtime, INDICATOR).style().background, colors.surface);
        let header = bounds(&runtime, HEADER);
        let before = bounds(&runtime, BEFORE);
        let after = bounds(&runtime, AFTER);

        tab(&mut runtime, false, BEFORE);
        focus_next(&mut runtime, EditKey::Down, HEADER);
        assert!(runtime.dispatch(Event::Activate).unwrap());
        render(&mut runtime, 760.);
        assert_disclosure(&runtime, true);
        assert_eq!(runtime.focused(), Some(HEADER));
        assert!(runtime.app().presses.is_empty());
        assert!(!runtime.needs_frame());
        assert_eq!(bounds(&runtime, HEADER), header);
        assert_eq!(bounds(&runtime, BEFORE), before);
        let geometry = bounds(&runtime, CONTENT);
        close_enough(bounds(&runtime, AFTER).y - after.y, geometry.height);
        close_enough(
            bounds(&runtime, BRANCH).height,
            header.height + geometry.height,
        );
        assert_eq!(
            node(&runtime, BRANCH).style().corner_radii(),
            Corners::all(16.)
        );
        assert_eq!(node(&runtime, FIRST).style().background, colors.surface);
        assert_eq!(
            node(&runtime, INDICATOR).style().background,
            colors.surface_container
        );
        assert!(runtime
            .accessibility_frame()
            .unwrap()
            .activation_target(id(&runtime, FIRST))
            .is_some());
        assert!(runtime
            .accessibility_frame()
            .unwrap()
            .focus_target(id(&runtime, DISABLED))
            .is_none());
    }
}

#[test]
fn opening_and_closing_move_the_sibling_while_partial_children_stay_inert() {
    let mut runtime = Runtime::new(Fixture::new(ListStyle::Segmented, false)).unwrap();
    tick(&mut runtime, 0.);
    let after_closed = bounds(&runtime, AFTER).y;
    runtime.activate(HEADER).unwrap();
    tick(&mut runtime, 0.);
    tick(&mut runtime, 0.);
    assert_eq!(bounds(&runtime, AFTER).y, after_closed);
    tick(&mut runtime, 48.);
    assert_disclosure(&runtime, true);
    let body = bounds(&runtime, CONTENT);
    let opening = bounds(&runtime, AFTER).y - after_closed;
    assert!(opening > 0. && opening < body.height);
    assert!(runtime.needs_frame());
    assert_inert(&mut runtime, FIRST);
    assert!(runtime.app().presses.is_empty());
    assert_eq!(runtime.focused(), Some(HEADER));

    tick(&mut runtime, 520.);
    assert!(!runtime.needs_frame());
    let after_open = bounds(&runtime, AFTER).y;
    close_enough(after_open - after_closed, body.height);
    focus_next(&mut runtime, EditKey::Down, FIRST);
    render(&mut runtime, 760.);
    let (focus, press) = {
        let id = id(&runtime, FIRST);
        let frame = runtime.accessibility_frame().unwrap();
        (
            frame.focus_target(id).unwrap(),
            frame.activation_target(id).unwrap(),
        )
    };
    let child = bounds(&runtime, FIRST);
    runtime
        .dispatch(Event::PointerDown {
            x: child.x + child.width * 0.5,
            y: child.y + child.height * 0.5,
        })
        .unwrap();

    runtime.apply(Message::Open(false)).unwrap();
    assert_eq!(runtime.focused(), Some(HEADER));
    tick(&mut runtime, 520.);
    assert_disclosure(&runtime, false);
    close_enough(bounds(&runtime, AFTER).y, after_open);
    tick(&mut runtime, 568.);
    let closing = bounds(&runtime, AFTER).y;
    assert!(after_closed < closing && closing < after_open);
    assert_inert(&mut runtime, FIRST);
    assert_eq!(
        runtime.accessibility_focus(&focus).unwrap(),
        AccessibilityFocusResult::Rejected
    );
    assert_eq!(
        runtime.accessibility_activate(&press).unwrap(),
        AccessibilityActivationResult::Rejected
    );
    tick(&mut runtime, 1100.);
    assert_eq!(bounds(&runtime, AFTER).y, after_closed);
    assert!(runtime.dom().by_id(CONTENT).is_none());
    assert!(runtime.dom().by_id(FIRST).is_none());
    assert_eq!(runtime.focused(), Some(HEADER));
    runtime.set_reduced_motion(true);
    runtime.apply(Message::Open(true)).unwrap();
    tick(&mut runtime, 1100.);
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id(&runtime, FIRST))
        .is_some());
    assert_eq!(
        runtime.accessibility_focus(&focus).unwrap(),
        AccessibilityFocusResult::Rejected
    );
    assert_eq!(
        runtime.accessibility_activate(&press).unwrap(),
        AccessibilityActivationResult::Rejected
    );
    let child = bounds(&runtime, FIRST);
    runtime
        .dispatch(Event::PointerUp {
            x: child.x + child.width * 0.5,
            y: child.y + child.height * 0.5,
        })
        .unwrap();
    assert_eq!(runtime.focused(), Some(HEADER));
    assert!(runtime.app().presses.is_empty());
}

#[test]
fn reversing_the_transform_carries_its_pose_and_reduced_motion_snaps() {
    let mut runtime = Runtime::new(Fixture::new(ListStyle::Standard, true)).unwrap();
    tick(&mut runtime, 0.);
    // Retain the natural body height before starting the first close.
    tick(&mut runtime, 0.);
    let open = bounds(&runtime, BRANCH).height;
    let header = bounds(&runtime, HEADER).height;
    runtime.apply(Message::Open(false)).unwrap();
    tick(&mut runtime, 0.);
    tick(&mut runtime, 48.);
    let middle = bounds(&runtime, BRANCH).height;
    assert!(middle < open && middle > header);

    runtime.apply(Message::Open(true)).unwrap();
    tick(&mut runtime, 48.);
    close_enough(bounds(&runtime, BRANCH).height, middle);
    assert_inert(&mut runtime, FIRST);
    tick(&mut runtime, 600.);
    close_enough(bounds(&runtime, BRANCH).height, open);
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id(&runtime, FIRST))
        .is_some());
    runtime.apply(Message::Open(false)).unwrap();
    tick(&mut runtime, 600.);
    tick(&mut runtime, 644.);
    assert!(runtime.needs_frame());

    runtime.set_reduced_motion(true);
    tick(&mut runtime, 644.);
    assert!(!runtime.needs_frame());
    assert!(runtime.dom().by_id(CONTENT).is_none());
    assert_eq!(bounds(&runtime, BRANCH).height, header);
    runtime.apply(Message::Open(true)).unwrap();
    tick(&mut runtime, 644.);
    assert!(!runtime.needs_frame());
    close_enough(bounds(&runtime, BRANCH).height, open);
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id(&runtime, FIRST))
        .is_some());
}

#[test]
fn open_content_measures_again_at_narrow_rtl_and_large_text() {
    let mut fixture = Fixture::new(ListStyle::Segmented, true);
    fixture.direction = LayoutDirection::RightToLeft;
    let mut runtime = Runtime::new(fixture).unwrap();
    render(&mut runtime, 760.);
    let wide = bounds(&runtime, CONTENT);
    let headline = bounds(&runtime, "child-last-headline");
    let indicator = bounds(&runtime, INDICATOR);
    let header_label = bounds(&runtime, "folder-headline");
    assert!(indicator.x + indicator.width < header_label.x);

    runtime.apply(Message::TextScale(2.)).unwrap();
    render(&mut runtime, 320.);
    let narrow = bounds(&runtime, CONTENT);
    let narrow_headline = bounds(&runtime, "child-last-headline");
    let header = bounds(&runtime, HEADER);
    let branch = bounds(&runtime, BRANCH);
    let after = bounds(&runtime, AFTER);
    assert!(narrow.width < wide.width);
    assert!(narrow.height > wide.height);
    assert!(narrow_headline.height > headline.height * 2.);
    assert_eq!(branch.intersect(narrow_headline), narrow_headline);
    close_enough(branch.height, header.height + narrow.height);
    assert!(after.y >= branch.y + branch.height - 1.);
    assert_disclosure(&runtime, true);
    assert_eq!(node(&runtime, LAST).name(), LONG_TITLE);
}

#[test]
fn one_roving_group_reveals_enabled_children_and_respects_modal_scope() {
    let mut fixture = Fixture::new(ListStyle::Segmented, true);
    fixture.scroll_height = 166.;
    let mut runtime = Runtime::new(fixture).unwrap();
    render(&mut runtime, 760.);
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(id(&runtime, LAST))
        .is_none());
    tab(&mut runtime, false, BEFORE);
    focus_next(&mut runtime, EditKey::Down, HEADER);
    focus_next(&mut runtime, EditKey::Down, FIRST);
    focus_next(&mut runtime, EditKey::Down, LAST);
    render(&mut runtime, 760.);
    assert!(runtime.scroll_metrics(SCROLLER).unwrap().offset.1 > 0.);
    assert_eq!(
        runtime.accessibility_frame().unwrap().focused(),
        Some(id(&runtime, LAST))
    );
    assert!(runtime.app().presses.is_empty());
    assert!(runtime.dispatch(Event::Activate).unwrap());
    assert_eq!(runtime.app().presses, [LAST]);
    render(&mut runtime, 760.);
    let last = id(&runtime, LAST);
    let token = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(last)
        .unwrap();
    runtime.apply(Message::CoverAndClose).unwrap();
    render(&mut runtime, 760.);
    assert_eq!(runtime.focused(), Some(MODAL_CLOSE));
    assert!(runtime.dom().by_id(LAST).is_none());
    assert_eq!(
        runtime.accessibility_activate(&token).unwrap(),
        AccessibilityActivationResult::Rejected
    );
    assert_eq!(runtime.app().presses, [LAST]);
}

#[test]
fn settled_expansion_reveals_previously_visible_following_focus() {
    let mut fixture = Fixture::new(ListStyle::Segmented, false);
    fixture.scroll_height = 176.;
    let mut runtime = Runtime::new(fixture).unwrap();
    tick(&mut runtime, 0.);
    tab(&mut runtime, false, BEFORE);
    focus_next(&mut runtime, EditKey::End, AFTER);
    tick(&mut runtime, 0.);
    let before = bounds(&runtime, AFTER);
    let clip = bounds(&runtime, SCROLLER);
    assert_eq!(before.intersect(clip), before);

    runtime.apply(Message::Open(true)).unwrap();
    for time in [0., 0., 18., 40., 75., 135., 260., 520.] {
        tick(&mut runtime, time);
        assert_eq!(runtime.focused(), Some(AFTER));
    }
    assert!(!runtime.needs_frame());
    let after = bounds(&runtime, AFTER);
    let offset = runtime.scroll_metrics(SCROLLER).unwrap().offset.1;
    assert_eq!(
        after.intersect(clip),
        after,
        "following focus disappeared after inline expansion: start={before:?}, end={after:?}, clip={clip:?}, offset={offset}"
    );
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.focused(), Some(id(&runtime, AFTER)));
    assert!(runtime.app().presses.is_empty());
}

#[test]
fn settled_expansion_leaves_intentionally_hidden_focus_offscreen() {
    let mut fixture = Fixture::new(ListStyle::Segmented, false);
    fixture.scroll_height = 104.;
    let mut runtime = Runtime::new(fixture).unwrap();
    tick(&mut runtime, 0.);
    tab(&mut runtime, false, BEFORE);
    focus_next(&mut runtime, EditKey::End, AFTER);
    tick(&mut runtime, 0.);
    assert!(runtime.scroll_metrics(SCROLLER).unwrap().offset.1 > 0.);
    assert!(runtime.scroll_to(SCROLLER, ScrollTarget::Top));
    tick(&mut runtime, 0.);
    let clip = bounds(&runtime, SCROLLER);
    assert_eq!(bounds(&runtime, AFTER).intersect(clip).height, 0.);
    assert_eq!(runtime.accessibility_frame().unwrap().focused(), None);

    runtime.apply(Message::Open(true)).unwrap();
    for time in [0., 0., 18., 40., 75., 135., 260., 520.] {
        tick(&mut runtime, time);
        assert_eq!(runtime.focused(), Some(AFTER));
        assert_eq!(runtime.scroll_metrics(SCROLLER).unwrap().offset.1, 0.);
    }
    assert_eq!(bounds(&runtime, AFTER).intersect(clip).height, 0.);
    assert_eq!(runtime.accessibility_frame().unwrap().focused(), None);
    assert!(runtime.app().presses.is_empty());
}

#[test]
fn disabled_closed_header_remains_visible_and_is_skipped() {
    let mut fixture = Fixture::new(ListStyle::Baseline, false);
    fixture.header_disabled = true;
    let mut runtime = Runtime::new(fixture).unwrap();
    render(&mut runtime, 760.);
    assert_disclosure(&runtime, false);
    assert!(node(&runtime, HEADER).is_disabled());
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id(&runtime, HEADER))
        .is_none());
    tab(&mut runtime, false, BEFORE);
    focus_next(&mut runtime, EditKey::Down, AFTER);
    assert!(runtime.activate(HEADER).is_err());
    assert!(!runtime.app().expanded);
}
