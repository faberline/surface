use surface::{
    Color, ExtendedDisplayList, ExtendedPaintOp, Length, Node, PaintOp, Rect, Role, Style,
    TextMeasurer,
};
use surface_app::{
    AccessibilityScrollDirection, AccessibilityScrollResult, AnimationFrame, AnimationUpdate,
    Application, EditKey, Event, FrameInput, FramePress, InputAnimationUpdate, Queue, Runtime,
    View,
};
use surface_material::{
    shape::LayoutDirection, TabActivation, TabBarInput, TabBarKind, TabBarLayout, TabBarMotion,
    TabBarOptions, TabItem, TabPanelEntry, Theme, VectorIcon,
};

const KEYS: [&str; 4] = ["inbox", "disabled", "archive", "labels"];
const PANELS: [&str; 4] = [
    "panel-inbox",
    "panel-disabled",
    "panel-archive",
    "panel-labels",
];
const LABELS: [&str; 4] = ["A", "Disabled", "Archive labels", "所有已保存的分類"];

#[derive(Clone)]
enum Message {
    Select(&'static str),
    Drag(TabBarInput),
    Ignore,
}

struct App {
    kind: TabBarKind,
    layout: TabBarLayout,
    activation: TabActivation,
    direction: LayoutDirection,
    motion: TabBarMotion,
    dark: bool,
    width: f32,
    scale: f32,
    icon: bool,
    short_labels: bool,
    panel_entry: TabPanelEntry,
    selected: &'static str,
    requests: usize,
    queue: Option<Queue<Message>>,
}

impl App {
    fn options(&self) -> TabBarOptions<'static> {
        TabBarOptions {
            id: "tabs",
            name: "Messages",
            kind: self.kind,
            layout: self.layout,
            direction: self.direction,
            available_width: self.width,
            text_scale: self.scale,
            selected: Some(self.selected),
            activation: self.activation,
        }
    }

    fn items(&self) -> impl Iterator<Item = TabItem<'static, Message>> {
        let icon = self.icon;
        let labels = if self.short_labels {
            ["A", "B", "C", "D"]
        } else {
            LABELS
        };
        KEYS.into_iter().enumerate().map(move |(i, id)| TabItem {
            id,
            label: labels[i],
            name: (i == 2).then_some("Archive, complete label"),
            icon: (icon && i != 0)
                .then(|| VectorIcon::new([[2., 2.], [22., 2.], [12., 22.]]).unwrap()),
            disabled: i == 1,
            panel_id: PANELS[i],
            on_select: Message::Select(id),
        })
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

    fn attach_queue(&mut self, queue: Queue<Message>) {
        self.queue = Some(queue);
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Select(id) => {
                self.selected = id;
                self.requests += 1;
            }
            Message::Drag(input) => {
                self.motion
                    .apply(self.options(), input, self.queue.as_ref().unwrap())
            }
            Message::Ignore => {}
        }
    }

    fn view(&self) -> View<Message> {
        let theme = self.theme();
        let content = match self.panel_entry {
            TabPanelEntry::Panel => View::new(Node::new(Role::Text, "").text("Active panel")),
            TabPanelEntry::Contents => button("panel-content", "Panel content"),
        };
        let selected = KEYS.iter().position(|id| *id == self.selected).unwrap();
        let bar = theme
            .tab_bar(self.options(), self.items(), &self.motion, Message::Drag)
            .unwrap();
        View::new(Node::new(Role::Group, "").styled(Style::column())).children([
            button("before", "Before"),
            bar,
            theme.tab_panel(
                PANELS[selected],
                "tabs",
                self.selected,
                self.panel_entry,
                content,
            ),
            button("after", "After"),
        ])
    }

    fn observes_frame_input(&self) -> bool {
        true
    }

    fn animation_frame_with_press_detail(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
        pressed: Option<&str>,
        _: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> InputAnimationUpdate {
        let options = self.options();
        let appearance = self
            .motion
            .observe_interaction(options, &frame, input, pressed, hovered);
        let motion = self
            .motion
            .advance(options, &frame, self.queue.as_ref().unwrap());
        InputAnimationUpdate {
            animation: AnimationUpdate {
                rebuild: appearance || motion.rebuild,
                ..motion
            },
            suppress_touch_activation: false,
        }
    }
}

fn button(id: &str, label: &str) -> View<Message> {
    View::new(
        Node::new(Role::Button, label)
            .id(id)
            .styled(Style::column().height(Length::Px(48.))),
    )
    .on_press(Message::Ignore)
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let line_width = text.chars().count() as f32 * size * 0.62;
        let limit = width.unwrap_or(line_width).max(1.);
        (
            line_width.min(limit),
            (line_width / limit).ceil().max(1.) * size * 1.5,
        )
    }
}

fn new(kind: TabBarKind, layout: TabBarLayout, direction: LayoutDirection) -> App {
    App {
        kind,
        layout,
        activation: TabActivation::Manual,
        direction,
        motion: TabBarMotion::default(),
        dark: false,
        width: 384.,
        scale: 1.,
        icon: false,
        short_labels: false,
        panel_entry: TabPanelEntry::Panel,
        selected: "inbox",
        requests: 0,
        queue: None,
    }
}

fn tick(runtime: &mut Runtime<App>, time: f64) -> ExtendedDisplayList {
    runtime.set_time(time);
    runtime
        .render_extended(runtime.app().width, 520., &mut Metrics)
        .unwrap()
}

fn make(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    tick(&mut runtime, 0.);
    tick(&mut runtime, 16.);
    runtime
}

fn node<'a, A: Application>(runtime: &'a Runtime<A>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn bounds(runtime: &Runtime<impl Application>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
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

fn tap(runtime: &mut Runtime<App>, reverse: bool) {
    runtime.dispatch(Event::FocusNext { reverse }).unwrap();
}

fn alpha(color: Color, a: u8) -> Color {
    Color(color.0, color.1, color.2, a)
}

#[test]
fn fixed_geometry_paint_and_focus_are_distinct_for_both_kinds() {
    for (kind, base_height, underline) in [
        (TabBarKind::Primary, 64., 3.),
        (TabBarKind::Secondary, 48., 2.),
    ] {
        let mut app = new(kind, TabBarLayout::Fixed, LayoutDirection::LeftToRight);
        app.icon = true;
        app.short_labels = true;
        app.dark = kind == TabBarKind::Secondary;
        let expected = app.theme();
        let mut runtime = make(app);
        assert_eq!(node(&runtime, "tabs").role(), Role::TabList);
        assert_eq!(node(&runtime, "tabs").name(), "Messages");
        assert_eq!(bounds(&runtime, "tabs").height, base_height);
        assert_eq!(bounds(&runtime, "tabs-divider").height, 1.);
        assert_eq!(
            node(&runtime, "tabs-divider").style().background,
            expected.colors.surface_variant
        );
        assert_eq!(
            node(&runtime, "tabs").style().background,
            expected.colors.surface
        );
        assert_eq!(KEYS.map(|id| bounds(&runtime, id).width), [96.; 4]);
        assert_eq!(node(&runtime, "archive").name(), "Archive, complete label");
        assert_eq!(node(&runtime, "inbox").is_selected(), Some(true));
        assert_eq!(node(&runtime, "disabled").is_selected(), Some(false));
        assert!(node(&runtime, "disabled").is_disabled());
        let icon = bounds(&runtime, "archive-icon");
        let label = bounds(&runtime, "archive-label");
        assert_eq!((icon.width, icon.height), (24., 24.));
        if kind == TabBarKind::Primary {
            assert_eq!(label.y - (icon.y + icon.height), 4.);
        } else {
            assert_eq!(label.x - (icon.x + icon.width), 8.);
        }
        let pose = runtime.app().motion.indicator().unwrap();
        assert_eq!(pose.height, underline);
        assert_eq!(pose.y + pose.height, base_height);
        assert_eq!(
            pose.width,
            if kind == TabBarKind::Primary {
                24.
            } else {
                96.
            }
        );
        assert_eq!(
            node(&runtime, "disabled-label").style().foreground,
            alpha(expected.colors.on_surface, 97)
        );
        assert_eq!(
            node(&runtime, "inbox-label").style().foreground,
            if kind == TabBarKind::Primary {
                expected.colors.primary
            } else {
                expected.colors.on_surface
            }
        );
        tap(&mut runtime, false);
        tap(&mut runtime, false);
        assert_eq!(runtime.focused(), Some("inbox"));
        let paint = tick(&mut runtime, 32.);
        assert!(paint.ops().iter().any(|op| matches!(op,
            ExtendedPaintOp::Legacy(PaintOp::ShapeStroke { width, color, .. })
            | ExtendedPaintOp::Legacy(PaintOp::Stroke { width, color, .. })
            if *width == 3. && *color == expected.colors.secondary)));
    }
}

#[test]
fn manual_and_automatic_selection_keep_one_tab_stop_and_the_active_panel() {
    for activation in [TabActivation::Manual, TabActivation::Automatic] {
        let mut app = new(
            TabBarKind::Primary,
            TabBarLayout::Fixed,
            LayoutDirection::RightToLeft,
        );
        app.selected = "archive";
        app.activation = activation;
        app.panel_entry = if activation == TabActivation::Manual {
            TabPanelEntry::Panel
        } else {
            TabPanelEntry::Contents
        };
        let mut runtime = make(app);
        let automated = usize::from(activation == TabActivation::Automatic);
        tap(&mut runtime, false);
        tap(&mut runtime, false);
        assert_eq!(runtime.focused(), Some("archive"));
        assert_eq!(runtime.app().requests, 0);
        let active_tab = runtime.dom().by_id("archive").unwrap();
        let active_panel = runtime.dom().by_id("panel-archive").unwrap();
        let accessible = runtime.accessibility_frame().unwrap();
        assert_eq!(accessible.tab_panel_tab(active_panel), Some(active_tab));
        assert_eq!(
            accessible.lazy_tab_panel(runtime.dom().by_id("inbox").unwrap()),
            Some("panel-inbox")
        );
        edit(&mut runtime, EditKey::Right);
        assert_eq!(runtime.focused(), Some("inbox"), "RTL Right skips disabled");
        assert_eq!(runtime.app().requests, automated);
        assert_eq!(
            node(&runtime, if automated == 0 { "archive" } else { "inbox" }).is_selected(),
            Some(true)
        );
        runtime.dispatch(Event::Activate).unwrap();
        assert_eq!(
            runtime.app().requests,
            1,
            "already-selected activation does not repeat the request"
        );
        tick(&mut runtime, 32.);
        assert!(runtime.dom().by_id("panel-archive").is_none());
        assert!(runtime.dom().by_id("panel-inbox").is_some());
        tap(&mut runtime, false);
        assert_eq!(
            runtime.focused(),
            Some(if automated == 0 {
                "panel-inbox"
            } else {
                "panel-content"
            })
        );
        tap(&mut runtime, true);
        assert_eq!(runtime.focused(), Some("inbox"));
        edit(&mut runtime, EditKey::Up);
        assert_eq!(
            runtime.focused(),
            Some("inbox"),
            "horizontal tab arrows leave Up alone"
        );
        assert!(runtime.activate("disabled").is_err());
        assert_eq!(runtime.app().requests, 1);
    }
}

#[test]
fn both_kinds_keep_wrapped_fixed_labels_inside_equal_cells_at_200_percent() {
    for kind in [TabBarKind::Primary, TabBarKind::Secondary] {
        let mut app = new(kind, TabBarLayout::Fixed, LayoutDirection::RightToLeft);
        app.width = 320.;
        app.scale = 2.;
        app.icon = true;
        let runtime = make(app);
        for tab in KEYS {
            let rect = bounds(&runtime, tab);
            let label = bounds(&runtime, &format!("{tab}-label"));
            assert_eq!(rect.width, 80.);
            assert_eq!(rect.height, bounds(&runtime, "tabs").height);
            assert_eq!(rect.intersect(label), label);
        }
        assert!(bounds(&runtime, "labels-label").height > 80.);
        assert!(bounds(&runtime, "inbox").x > bounds(&runtime, "labels").x);
    }
}

#[test]
fn exact_interaction_ink_and_selected_pointer_feedback_never_repeat_selection() {
    for kind in [TabBarKind::Primary, TabBarKind::Secondary] {
        let mut runtime = make(new(kind, TabBarLayout::Fixed, LayoutDirection::LeftToRight));
        let theme = runtime.app().theme();
        let (rest, interactive) = (theme.colors.on_surface_variant, theme.colors.on_surface);
        assert_eq!(node(&runtime, "archive-label").style().foreground, rest);
        let tab = bounds(&runtime, "archive");
        let x = tab.x + tab.width / 2.;
        let y = tab.y + 14.;
        runtime.dispatch(Event::PointerMove { x, y }).unwrap();
        let hovered = tick(&mut runtime, 32.);
        assert_eq!(
            node(&runtime, "archive-label").style().foreground,
            interactive
        );
        assert!(hovered.ops().iter().any(|op| matches!(
            op, ExtendedPaintOp::Legacy(PaintOp::Quad { color, rect, .. })
            if *color == alpha(interactive, 20) && *rect == tab
        )));

        let tab = bounds(&runtime, "inbox");
        let x = tab.x + 17.;
        runtime.dispatch(Event::PointerDown { x, y }).unwrap();
        tick(&mut runtime, 48.);
        let held = tick(&mut runtime, 123.);
        let press_ink = if kind == TabBarKind::Primary {
            theme.colors.primary
        } else {
            interactive
        };
        assert!(held.ops().iter().any(|op| matches!(
            op, ExtendedPaintOp::FillPath(path) if path.color() == alpha(press_ink, 26)
        )));
        runtime.dispatch(Event::PointerUp { x, y }).unwrap();
        assert_eq!(runtime.app().requests, 0);
        assert_eq!(runtime.focused(), Some("inbox"));

        runtime.set_reduced_motion(true);
        runtime.dispatch(Event::PointerDown { x, y }).unwrap();
        let reduced = tick(&mut runtime, 140.);
        assert!(reduced.ops().iter().any(|op| matches!(
            op, ExtendedPaintOp::Legacy(PaintOp::Quad { color, rect, .. })
            if *color == alpha(press_ink, 26) && *rect == tab
        )));
        assert!(!reduced.ops().iter().any(|op| matches!(
            op, ExtendedPaintOp::FillPath(path) if path.color() == alpha(press_ink, 26)
        )));
        runtime.cancel_pointer().unwrap();
        assert_eq!(runtime.app().requests, 0);
    }
}

#[test]
fn focus_reveal_and_physical_wheel_use_a_real_bounded_scroller_in_both_directions() {
    for (direction, kind) in [
        (LayoutDirection::LeftToRight, TabBarKind::Primary),
        (LayoutDirection::RightToLeft, TabBarKind::Secondary),
    ] {
        let mut app = new(kind, TabBarLayout::Scrollable, direction);
        app.width = 320.;
        app.scale = 2.;
        app.icon = true;
        let mut runtime = make(app);
        let rtl = direction == LayoutDirection::RightToLeft;
        let scroller = bounds(&runtime, "tabs-scroll");
        let first = bounds(&runtime, "inbox");
        let metrics = runtime.scroll_metrics("tabs-scroll").unwrap();
        let maximum = metrics.content.0 - metrics.viewport.0;
        assert!(maximum > 100., "{metrics:?}");
        assert_eq!(metrics.offset.0, if rtl { maximum } else { 0. });
        assert_eq!(
            if rtl {
                scroller.x + scroller.width - first.x - first.width
            } else {
                first.x - scroller.x
            },
            52.,
            "the logical edge inset is 52 dp"
        );
        assert!(KEYS.into_iter().all(|id| bounds(&runtime, id).width >= 90.));
        let wheel = |runtime: &mut Runtime<App>, delta_x| {
            runtime
                .dispatch(Event::Scroll {
                    x: scroller.x + 4.,
                    y: scroller.y + 4.,
                    delta_x,
                    delta_y: 0.,
                })
                .unwrap();
        };
        let indicator = runtime.app().motion.indicator();
        wheel(&mut runtime, if rtl { -5000. } else { 5000. });
        tick(&mut runtime, 32.);
        assert_eq!(
            runtime.scroll_metrics("tabs-scroll").unwrap().offset.0,
            if rtl { 0. } else { maximum }
        );
        assert_eq!(
            runtime.app().motion.indicator(),
            indicator,
            "scrolling does not move the content-local indicator"
        );
        assert_eq!(runtime.app().requests, 0);
        tap(&mut runtime, false);
        tap(&mut runtime, false);
        tick(&mut runtime, 48.);
        assert_eq!(runtime.focused(), Some("inbox"));
        assert_eq!(
            bounds(&runtime, "inbox").intersect(scroller),
            bounds(&runtime, "inbox")
        );
        edit(&mut runtime, EditKey::End);
        tick(&mut runtime, 64.);
        assert_eq!(runtime.focused(), Some("labels"));
        assert_eq!(
            bounds(&runtime, "labels").intersect(scroller),
            bounds(&runtime, "labels")
        );
        assert!(bounds(&runtime, "labels-label").intersect(scroller).width > 0.);
        assert_eq!(runtime.app().requests, 0);
    }
}

#[test]
fn captured_touch_scroll_previews_cancels_and_releases_without_activating() {
    let mut app = new(
        TabBarKind::Primary,
        TabBarLayout::Scrollable,
        LayoutDirection::LeftToRight,
    );
    app.width = 320.;
    let mut runtime = make(app);
    let tab = bounds(&runtime, "archive");
    let scroller = bounds(&runtime, "tabs-scroll");
    let origin = (
        (tab.x + 3.).min(scroller.x + scroller.width - 3.),
        tab.y + 12.,
    );
    for cancel in [true, false] {
        runtime
            .dispatch_touch(Event::PointerDown {
                x: origin.0,
                y: origin.1,
            })
            .unwrap();
        runtime
            .dispatch_touch(Event::PointerMove {
                x: origin.0 - 74.,
                y: origin.1,
            })
            .unwrap();
        tick(&mut runtime, 48.);
        assert_eq!(runtime.scroll_metrics("tabs-scroll").unwrap().offset.0, 74.);
        assert_eq!(runtime.app().requests, 0);
        if cancel {
            runtime.cancel_pointer().unwrap();
        } else {
            runtime
                .dispatch_touch(Event::PointerUp {
                    x: origin.0 - 88.,
                    y: origin.1,
                })
                .unwrap();
        }
        tick(&mut runtime, 64.);
        assert_eq!(
            runtime.scroll_metrics("tabs-scroll").unwrap().offset.0,
            if cancel { 0. } else { 88. }
        );
        assert_eq!(runtime.focused(), None);
        assert_eq!(runtime.app().requests, 0);
    }
}

#[test]
fn accessibility_pages_and_reveals_without_activating_or_moving_focus() {
    let mut app = new(
        TabBarKind::Secondary,
        TabBarLayout::Scrollable,
        LayoutDirection::LeftToRight,
    );
    app.width = 320.;
    app.scale = 2.;
    let mut runtime = make(app);
    tap(&mut runtime, false);
    tick(&mut runtime, 32.);
    let tab = runtime.dom().by_id("labels").unwrap();
    let disabled = runtime.dom().by_id("disabled").unwrap();
    let scroll_id = runtime.dom().by_id("tabs-scroll").unwrap();
    let viewport = bounds(&runtime, "tabs-scroll");
    assert_eq!(bounds(&runtime, "labels").intersect(viewport).width, 0.);
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.scroll_target(disabled).is_none());
    assert!(frame
        .scroll_page_target(scroll_id, AccessibilityScrollDirection::Left)
        .is_none());
    assert!(frame
        .scroll_page_target(scroll_id, AccessibilityScrollDirection::Down)
        .is_none());
    let reveal = frame
        .scroll_target(tab)
        .expect("fully clipped enabled tab can request reveal");
    assert_eq!(
        runtime.accessibility_scroll_into_view(&reveal).unwrap(),
        AccessibilityScrollResult::Scheduled
    );
    tick(&mut runtime, 48.);
    assert_eq!(
        bounds(&runtime, "labels").intersect(viewport),
        bounds(&runtime, "labels")
    );
    assert_eq!(runtime.focused(), Some("before"));
    assert_eq!(runtime.app().requests, 0);

    let before_page = runtime.scroll_metrics("tabs-scroll").unwrap().offset.0;
    let frame = runtime.accessibility_frame().unwrap();
    let back = frame
        .scroll_page_target(scroll_id, AccessibilityScrollDirection::Left)
        .unwrap();
    assert_eq!(
        runtime.accessibility_scroll_page(&back).unwrap(),
        AccessibilityScrollResult::Scheduled
    );
    tick(&mut runtime, 64.);
    assert_eq!(
        runtime.scroll_metrics("tabs-scroll").unwrap().offset.0,
        (before_page - viewport.width).max(0.)
    );
    assert_eq!(runtime.focused(), Some("before"));
    assert_eq!(runtime.app().requests, 0);
}

#[test]
fn indicator_retargets_in_place_while_semantics_update_immediately() {
    let mut runtime = make(new(
        TabBarKind::Primary,
        TabBarLayout::Fixed,
        LayoutDirection::LeftToRight,
    ));
    let first = runtime.app().motion.indicator().unwrap();
    assert!(runtime.activate("labels").unwrap());
    assert_eq!(node(&runtime, "labels").is_selected(), Some(true));
    assert!(runtime.dom().by_id("panel-labels").is_some());
    tick(&mut runtime, 100.);
    let first_moving_frame = tick(&mut runtime, 150.);
    let middle = runtime.app().motion.indicator().unwrap();
    let generation = runtime.accessibility_frame().unwrap().generation();
    let later = tick(&mut runtime, 178.);
    assert!(middle.x > first.x && middle.x < bounds(&runtime, "labels").x);
    assert!(middle.width > first.width);
    assert_eq!(middle.y, first.y);
    assert_ne!(first_moving_frame.ops(), later.ops());
    assert_eq!(
        runtime.accessibility_frame().unwrap().generation(),
        generation
    );
    assert!(later.ops().iter().any(|op| matches!(op, ExtendedPaintOp::FillPath(path) if path.color() == runtime.app().theme().colors.primary)));

    let before_reversal = runtime.app().motion.indicator().unwrap();
    runtime.activate("inbox").unwrap();
    tick(&mut runtime, 178.);
    assert_eq!(runtime.app().motion.indicator().unwrap(), before_reversal);
    tick(&mut runtime, 235.);
    assert!(runtime.app().motion.indicator().unwrap().x < before_reversal.x);
    runtime.set_reduced_motion(true);
    tick(&mut runtime, 240.);
    assert_eq!(runtime.app().motion.indicator(), Some(first));
    assert_eq!(runtime.app().requests, 2);
}

#[test]
fn fixed_rejects_undersized_targets_and_preserves_legacy_builders() {
    let mut app = new(
        TabBarKind::Primary,
        TabBarLayout::Fixed,
        LayoutDirection::LeftToRight,
    );
    app.width = 191.;
    let theme = Theme::light();
    assert!(theme
        .tab_bar(app.options(), app.items(), &app.motion, Message::Drag)
        .is_none());
    struct Legacy(usize);
    impl Application for Legacy {
        type Message = ();

        fn update(&mut self, (): ()) {
            self.0 += 1;
        }

        fn view(&self) -> View<()> {
            let theme = Theme::light();
            let choice = surface_material::ChoiceState {
                selected: true,
                disabled: false,
            };
            theme.tabs(
                "legacy",
                "Legacy",
                [theme.tab("legacy-tab", "One", choice, ())],
            )
        }
    }
    let mut legacy = Runtime::new(Legacy(0)).unwrap();
    legacy.render(320., 120., &mut Metrics).unwrap();
    assert_eq!(node(&legacy, "legacy").role(), Role::TabList);
    assert_eq!(node(&legacy, "legacy-tab").is_selected(), Some(true));
    assert_eq!(bounds(&legacy, "legacy-tab").height, 48.);
    assert!(legacy.activate("legacy-tab").unwrap());
    assert_eq!(legacy.app().0, 1);
}
