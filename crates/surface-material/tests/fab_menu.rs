use surface::{Corners, Insets, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AnimationFrame, Application, EditKey, Event, FrameInput, FramePress, InputAnimationUpdate, Key,
    KeyPress, Modifiers, Runtime, View,
};
use surface_material::{
    fab_menu_edge_margin, shape::LayoutDirection, Elevation, ExpressiveFabSize, FabMenuAppearance,
    FabMenuContent, FabMenuItem, FabMenuMotion, FabMenuMotionTarget, FabMenuOptions,
    FabMenuPresentation, Theme, VectorIcon,
};

const TRIGGER: &str = "fab-menu-trigger";
const ITEM_IDS: [&str; 6] = ["item-0", "item-1", "item-2", "item-3", "item-4", "item-5"];
const LONG: &str = "新增完整的每月中文報表範本";
const OPEN_LABEL: &str = "建立新項目";
const CLOSE_LABEL: &str = "關閉建立選單";

#[derive(Clone)]
enum Message {
    Toggle,
    Dismiss,
    Disable,
    Item(usize),
    Other,
}

struct Demo {
    open: bool,
    disabled: bool,
    dark: bool,
    direction: LayoutDirection,
    presentation: FabMenuPresentation,
    appearance: FabMenuAppearance,
    size: ExpressiveFabSize,
    top: bool,
    long: bool,
    item_count: usize,
    scale: f32,
    viewport: Rect,
    activated: Option<usize>,
    motion: FabMenuMotion,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            open: false,
            disabled: false,
            dark: false,
            direction: LayoutDirection::LeftToRight,
            presentation: FabMenuPresentation::NativeFloating,
            appearance: FabMenuAppearance::Primary,
            size: ExpressiveFabSize::Medium,
            top: false,
            long: false,
            item_count: 6,
            scale: 1.,
            viewport: Rect::new(0., 0., 640., 720.),
            activated: None,
            motion: FabMenuMotion::new(false),
        }
    }
}

impl Application for Demo {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => self.open = !self.open,
            Message::Dismiss => self.open = false,
            Message::Disable => self.disabled = true,
            Message::Item(index) => {
                self.activated = Some(index);
                self.open = false;
            }
            Message::Other => {}
        }
    }

    fn observes_frame_input(&self) -> bool {
        true
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn animation_frame_with_press_detail(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
        _: Option<&str>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> InputAnimationUpdate {
        InputAnimationUpdate {
            animation: self.motion.advance(
                &frame,
                &input,
                press,
                hovered,
                FabMenuMotionTarget {
                    id: "fab-menu",
                    open: self.open && !self.disabled,
                    item_ids: &ITEM_IDS[..self.item_count],
                },
            ),
            suppress_touch_activation: false,
        }
    }

    fn view(&self) -> View<Message> {
        let theme = self.theme();
        let side = match self.size {
            ExpressiveFabSize::Regular => 56.,
            ExpressiveFabSize::Medium => 80.,
            ExpressiveFabSize::Large => 96.,
        };
        let edge = fab_menu_edge_margin(self.viewport.width);
        let x = match self.direction {
            LayoutDirection::LeftToRight => self.viewport.width - edge - side,
            LayoutDirection::RightToLeft => edge,
        };
        let y = if self.top {
            edge
        } else {
            self.viewport.height - edge - side
        };
        let glyph = VectorIcon::new([[2., 3.], [21., 3.], [12., 22.]]).unwrap();
        let fab = theme.fab_menu(
            FabMenuOptions {
                id: "fab-menu",
                open_label: OPEN_LABEL,
                close_label: CLOSE_LABEL,
                menu_label: "建立項目",
                presentation: self.presentation,
                appearance: self.appearance,
                size: self.size,
                direction: self.direction,
                viewport: self.viewport,
                open: self.open,
                disabled: self.disabled,
                restore_focus_to: Some("before"),
            },
            FabMenuContent {
                icon: glyph.clone(),
                items: ITEM_IDS[..self.item_count]
                    .iter()
                    .enumerate()
                    .map(|(index, id)| FabMenuItem {
                        id,
                        label: if self.long {
                            LONG
                        } else {
                            ["文字", "相片", "完整報表", "掃描", "便條", "檔案"][index]
                        },
                        icon: (index != 4).then(|| glyph.clone()),
                        disabled: index == 1,
                        on_press: Message::Item(index),
                    })
                    .collect(),
                on_toggle: Message::Toggle,
                on_dismiss: Message::Dismiss,
            },
            &self.motion,
        );
        let positioned = View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Px(side))
                    .height(Length::Px(side))
                    .fixed(Insets {
                        left: Length::Px(x),
                        top: Length::Px(y),
                        ..Insets::default()
                    }),
            ),
        )
        .children([fab]);
        theme.page([
            theme.button("before", "返回", Message::Other),
            positioned,
            theme.button("after", "繼續", Message::Other),
        ])
    }
}

impl Demo {
    fn theme(&self) -> Theme {
        if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        }
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let length = text.chars().count();
        let slots = width.map_or(length.max(1), |width| {
            ((width / size).floor() as usize).max(1)
        });
        (
            length.min(slots) as f32 * size,
            length.div_ceil(slots).max(1) as f32 * size * 1.5,
        )
    }
}

fn paint(a: &mut Runtime<Demo>, time: f64) {
    a.set_time(time);
    let vp = a.app().viewport;
    a.render_extended(vp.width, vp.height, &mut Metrics)
        .unwrap();
}

fn settle(a: &mut Runtime<Demo>, time: f64) {
    for offset in [0., 16., 32., 48.] {
        paint(a, time + offset);
    }
}

fn fixture(demo: Demo) -> Runtime<Demo> {
    let mut a = Runtime::new(demo).unwrap();
    a.set_reduced_motion(true);
    settle(&mut a, 0.);
    a
}

fn node<'a>(a: &'a Runtime<Demo>, id: &str) -> &'a Node {
    a.dom().get(a.dom().by_id(id).unwrap()).unwrap()
}

fn bounds(a: &Runtime<Demo>, id: &str) -> Rect {
    a.computed().unwrap().nodes[a.dom().by_id(id).unwrap().index()].bounds
}

fn activate(a: &mut Runtime<Demo>) {
    assert!(a.dispatch(Event::Activate).unwrap());
}

fn tab(a: &mut Runtime<Demo>, reverse: bool) {
    assert!(a.dispatch(Event::FocusNext { reverse }).unwrap());
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.6, "{actual} != {expected}");
}

#[test]
fn native_sizes_palettes_and_anchor_are_exact_for_two_through_six_actions() {
    for dark in [false, true] {
        for (appearance, palette) in [
            (FabMenuAppearance::Primary, 0),
            (FabMenuAppearance::Secondary, 1),
            (FabMenuAppearance::Tertiary, 2),
        ] {
            for (size, side, item_count) in [
                (ExpressiveFabSize::Regular, 56., 2),
                (ExpressiveFabSize::Medium, 80., 3),
                (ExpressiveFabSize::Medium, 80., 4),
                (ExpressiveFabSize::Large, 96., 5),
                (ExpressiveFabSize::Large, 96., 6),
            ] {
                let mut a = fixture(Demo {
                    dark,
                    appearance,
                    size,
                    item_count,
                    ..Demo::default()
                });
                let theme = a.app().theme();
                let c = theme.colors;
                let (tonal, solid) = [
                    (c.primary_container, c.primary),
                    (c.secondary_container, c.secondary),
                    (c.tertiary_container, c.tertiary),
                ][palette];
                let slot = bounds(&a, "fab-menu");
                assert_eq!(bounds(&a, TRIGGER), slot);
                assert_eq!((slot.width, slot.height), (side, side));
                assert_eq!(node(&a, "fab-menu-trigger-face").style().background, tonal);
                a.activate(TRIGGER).unwrap();
                settle(&mut a, 64.);

                let close = bounds(&a, TRIGGER);
                let nearest = bounds(&a, ITEM_IDS[item_count - 1]);
                assert_eq!(bounds(&a, "fab-menu"), slot);
                assert_eq!((close.width, close.height), (56., 56.));
                near(close.y, slot.y);
                near(close.x + close.width, slot.x + slot.width);
                near(nearest.x + nearest.width, close.x + close.width);
                near(close.y - (nearest.y + nearest.height), 8.);
                for pair in ITEM_IDS[..item_count].windows(2) {
                    let prior = bounds(&a, pair[0]);
                    let next = bounds(&a, pair[1]);
                    near(next.y - (prior.y + prior.height), 4.);
                }
                assert_eq!(a.focused(), Some(TRIGGER));
                assert_eq!(a.dom().by_role_name(Role::Button, CLOSE_LABEL).len(), 1);
                assert_eq!(node(&a, TRIGGER).expanded_state(), Some(true));
                assert!(!node(&a, TRIGGER).has_menu_popup());
                assert_eq!(node(&a, "fab-menu-trigger-face").style().background, solid);
                assert_eq!(
                    node(&a, "fab-menu-trigger-face").style().corners,
                    Some(Corners::all(28.))
                );
                assert_eq!(node(&a, "item-0-face").style().background, tonal);
                near(bounds(&a, "fab-menu-trigger-icon").width, 20.);
                near(bounds(&a, "item-0-icon").width, 24.);
                near(bounds(&a, "item-0").height, 56.);
                assert_eq!(
                    node(&a, "item-0-face").box_shadow_layers(),
                    theme.elevation(Elevation::Level0)
                );
            }
        }
    }
}

#[test]
fn native_compact_rtl_wrap_and_scroll_keep_close_first_and_reveal_the_focused_pill() {
    let mut a = fixture(Demo {
        dark: true,
        direction: LayoutDirection::RightToLeft,
        viewport: Rect::new(0., 0., 320., 400.),
        long: true,
        scale: 2.,
        size: ExpressiveFabSize::Large,
        ..Demo::default()
    });
    let slot = bounds(&a, "fab-menu");
    a.activate(TRIGGER).unwrap();
    settle(&mut a, 64.);
    let close = bounds(&a, TRIGGER);
    near(close.x, slot.x);
    near(close.y, slot.y);
    assert_eq!(a.focused(), Some(TRIGGER));
    let nearest = bounds(&a, "item-5-face");
    let viewport = bounds(&a, "fab-menu-list");
    assert_eq!(nearest, nearest.intersect(viewport));
    tab(&mut a, false);
    settle(&mut a, 128.);
    assert_eq!(a.focused(), Some(ITEM_IDS[0]));

    let pill = bounds(&a, "item-0-face");
    let icon = bounds(&a, "item-0-icon");
    let label = bounds(&a, "item-0-label");
    assert!(label.height > 48.);
    assert!(icon.x > label.x);
    let r = pill.width.min(pill.height) / 2.;
    for rect in [icon, label] {
        for x in [rect.x, rect.x + rect.width] {
            for y in [rect.y, rect.y + rect.height] {
                let dx = (x - pill.x).min(pill.x + pill.width - x);
                let dy = (y - pill.y).min(pill.y + pill.height - y);
                assert!(dx >= -0.6 && dy >= -0.6);
                assert!((r - dx).max(0.).hypot((r - dy).max(0.)) <= r + 0.6);
            }
        }
    }
    for index in [2, 3, 4, 5] {
        tab(&mut a, false);
        settle(&mut a, 192. + index as f64 * 64.);
        assert_eq!(a.focused(), Some(ITEM_IDS[index]));
    }
    let id = a.dom().by_id("item-5").unwrap();
    let ring = bounds(&a, "item-5-focus-ring");
    let viewport = bounds(&a, "fab-menu-list");
    assert_eq!(ring, ring.intersect(viewport));
    assert_eq!(bounds(&a, TRIGGER), close);
    assert!(node(&a, "fab-menu-list").style().scroll_y > 0.);
    assert!(a.accessibility_frame().unwrap().focus_target(id).is_some());
    activate(&mut a);
    assert_eq!(a.app().activated, Some(5));
    assert_eq!(a.focused(), Some(TRIGGER));
}

#[test]
fn web_projects_one_expanded_trigger_in_scope_and_flips_without_moving_it() {
    for top in [true, false] {
        let mut a = fixture(Demo {
            presentation: FabMenuPresentation::WebAnchored,
            direction: LayoutDirection::RightToLeft,
            viewport: Rect::new(0., 0., 320., 420.),
            top,
            item_count: 3,
            ..Demo::default()
        });
        let trigger = bounds(&a, TRIGGER);
        tab(&mut a, false);
        tab(&mut a, false);
        assert_eq!(a.focused(), Some(TRIGGER));
        let modified = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        assert!(!a.key(&KeyPress::new(Key::ArrowDown, modified)).unwrap());
        assert!(!a.app().open);
        let key = KeyPress::new(Key::ArrowDown, Modifiers::default());
        assert!(!a.key(&key.clone().with_repeat(true)).unwrap());
        assert!(!a.app().open);
        assert!(a.key(&key).unwrap());
        settle(&mut a, 64.);
        assert_eq!(bounds(&a, TRIGGER), trigger);
        assert_eq!(a.focused(), Some("item-0"));
        assert_eq!(node(&a, "fab-menu-menu").role(), Role::Menu);
        let menu = bounds(&a, "fab-menu-menu");
        if top {
            near(menu.y - (trigger.y + trigger.height), 4.);
        } else {
            near(trigger.y - (menu.y + menu.height), 4.);
        }
        let ax = a.accessibility_frame().unwrap();
        let trigger_id = a.dom().by_id(TRIGGER).unwrap();
        let opener = ax.node(trigger_id).unwrap().node;
        assert_eq!(ax.root(), a.dom().by_id("fab-menu-scope").unwrap());
        assert_eq!(
            a.dom().by_role_name(Role::Button, OPEN_LABEL),
            &[trigger_id]
        );
        assert_eq!(opener.expanded_state(), Some(true));
        assert!(opener.has_menu_popup());

        a.dispatch(Event::Edit {
            key: EditKey::Down,
            extend: false,
            by_word: false,
        })
        .unwrap();
        paint(&mut a, 128.);
        assert_eq!(a.focused(), Some("item-2"));
        tab(&mut a, false);
        assert!(!a.app().open);
        assert_eq!(a.focused(), Some("after"));

        paint(&mut a, 144.);
        a.activate(TRIGGER).unwrap();
        settle(&mut a, 160.);
        tab(&mut a, true);
        assert_eq!(a.focused(), Some("before"));
        paint(&mut a, 224.);
        a.activate(TRIGGER).unwrap();
        settle(&mut a, 256.);
        a.dispatch(Event::Cancel).unwrap();
        assert!(!a.app().open);
        assert_eq!(a.focused(), Some(TRIGGER));
    }
}

#[test]
fn native_and_web_outside_touch_dismiss_without_activating_background() {
    for presentation in [
        FabMenuPresentation::NativeFloating,
        FabMenuPresentation::WebAnchored,
    ] {
        let mut a = fixture(Demo {
            presentation,
            ..Demo::default()
        });
        a.activate(TRIGGER).unwrap();
        settle(&mut a, 64.);
        assert!(a.app().open);
        let (x, y) = (3., 4.);
        a.dispatch_touch(Event::PointerDown { x, y }).unwrap();
        a.dispatch_touch(Event::PointerUp { x, y }).unwrap();
        assert!(!a.app().open);
        assert_eq!(a.app().activated, None);
        assert_eq!(a.focused(), Some(TRIGGER));
    }
}

#[test]
fn losing_the_trigger_moves_focus_to_the_caller_fallback() {
    for presentation in [
        FabMenuPresentation::NativeFloating,
        FabMenuPresentation::WebAnchored,
    ] {
        let mut a = fixture(Demo {
            presentation,
            ..Demo::default()
        });
        a.activate(TRIGGER).unwrap();
        settle(&mut a, 64.);
        assert!(a.accessibility_frame().unwrap().modal_root().is_some());
        a.apply(Message::Disable).unwrap();
        assert_eq!(a.focused(), Some("before"));
        assert!(node(&a, TRIGGER).is_disabled());
        assert!(a.activate(TRIGGER).is_err());
        settle(&mut a, 128.);
        assert!(a.accessibility_frame().unwrap().modal_root().is_none());
    }
}

#[test]
fn native_exit_paints_existing_pills_with_no_stale_item_actions() {
    let mut a = Runtime::new(Demo::default()).unwrap();
    settle(&mut a, 0.);
    a.activate(TRIGGER).unwrap();
    settle(&mut a, 64.);
    settle(&mut a, 1000.);
    settle(&mut a, 2000.);
    assert_eq!(a.focused(), Some(TRIGGER));
    a.dispatch(Event::Cancel).unwrap();
    paint(&mut a, 2064.);
    paint(&mut a, 2080.);
    assert_eq!(a.focused(), Some(TRIGGER));
    let id = a.dom().by_id("item-5").unwrap();
    assert!(node(&a, "item-5-face").style().background.3 > 0);
    let ax = a.accessibility_frame().unwrap();
    assert!(ax.modal_root().is_none());
    assert!(ax.node(id).is_none());
    assert!(a.activate("item-5").is_err());
    settle(&mut a, 3000.);
    settle(&mut a, 4000.);
    assert!(a.dom().by_id("fab-menu-list").is_none());
}
