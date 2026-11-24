use surface::{ExtendedDisplayList, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, Application, EditKey, Event, Runtime, View};
use surface_material::{
    shape::{self, LayoutDirection},
    ButtonKind, CommandMenuEntry, CommandMenuItem, CommandMenuMeasurement, CommandMenuOptions,
    Theme, VectorIcon,
};
use surface_wgpu::TextSystem;

#[derive(Clone, Copy)]
enum Message {
    Toggle,
    Dismiss,
    Choose(u8),
    Background,
    HideTrigger,
}

struct Demo {
    open: bool,
    hide: bool,
    trigger_width: Option<Length>,
    rtl: bool,
    scale: f32,
    wrapped: bool,
    chosen: Option<u8>,
    dismissed: usize,
    background: usize,
    measurement: CommandMenuMeasurement,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            open: false,
            hide: false,
            trigger_width: None,
            rtl: false,
            scale: 1.,
            wrapped: false,
            chosen: None,
            dismissed: 0,
            background: 0,
            measurement: CommandMenuMeasurement::default(),
        }
    }
}

impl Application for Demo {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => self.open = !self.open,
            Message::Dismiss => {
                self.open = false;
                self.dismissed += 1;
            }
            Message::Choose(index) => {
                self.open = false;
                self.chosen = Some(index);
            }
            Message::Background => self.background += 1,
            Message::HideTrigger => {
                self.hide = true;
                self.open = false;
            }
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        self.measurement.advance("commands", "trigger", &frame)
    }

    fn view(&self) -> View<Message> {
        let theme = if self.rtl {
            Theme::dark()
        } else {
            Theme::light()
        };
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let open_button = if let Some(width) = self.trigger_width {
            View::new(
                Node::new(Role::Button, "開啟")
                    .id("trigger")
                    .styled(Style::row().width(width).height(Length::Px(48.))),
            )
            .on_press(Message::Toggle)
            .accessible_activation()
        } else {
            theme.button("trigger", "開啟", Message::Toggle)
        };
        let icon = || VectorIcon::new([[3., 3.], [21., 3.], [21., 21.], [3., 21.]]);
        let item = |id, label, leading, trailing, disabled, action| {
            CommandMenuEntry::Item(CommandMenuItem {
                id,
                label,
                leading_icon: leading,
                trailing_icon: trailing,
                disabled,
                on_press: Message::Choose(action),
            })
        };
        let ordinary = if self.wrapped {
            "儲存報告及工作區的最新版本"
        } else {
            "儲存"
        };
        let options = CommandMenuOptions {
            id: "commands",
            label: "報告操作",
            trigger_id: "trigger",
            open: self.open,
            direction,
            preferred_width: 240.,
            restore_focus_to: Some("after"),
        };
        let menu = (!self.hide).then(|| {
            theme.command_menu(
                options,
                open_button,
                [
                    item("first", ordinary, icon(), icon(), false, 1),
                    CommandMenuEntry::Separator {
                        id: "group-separator",
                    },
                    item("disabled", "列印", None, None, true, 2),
                    item("last", "刪除", None, None, false, 3),
                ],
                Message::Dismiss,
                &self.measurement,
            )
        });
        let after = theme.button("after", "下一個", Message::Background);
        let children = menu.into_iter().chain([after]);
        let neighbors = if self.trigger_width.is_some() {
            View::new(Node::new(Role::Group, "").styled(Style::column())).children(children)
        } else {
            theme.row(children)
        };
        View::new(
            Node::new(Role::Window, "Example").styled(
                Style::column()
                    .padding(20.)
                    .gap(16.)
                    .background(theme.background)
                    .clip(true),
            ),
        )
        .children([
            theme.button_with(
                "before",
                "前一個",
                ButtonKind::Text,
                false,
                Message::Background,
            ),
            View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(20.)))),
            neighbors,
        ])
    }
}

fn paint(app: &mut Runtime<Demo>, viewport: (f32, f32)) -> ExtendedDisplayList {
    let mut fonts = TextSystem::default();
    app.render_extended(viewport.0, viewport.1, &mut fonts)
        .unwrap()
}

fn settle(app: &mut Runtime<Demo>, viewport: (f32, f32)) {
    for _ in 0..5 {
        paint(app, viewport);
    }
}

fn node<'a>(app: &'a Runtime<Demo>, key: &str) -> &'a Node {
    app.dom().get(app.dom().by_id(key).unwrap()).unwrap()
}

fn bounds(app: &Runtime<Demo>, key: &str) -> Rect {
    app.computed().unwrap().nodes[app.dom().by_id(key).unwrap().index()].bounds
}

fn tab(app: &mut Runtime<Demo>, reverse: bool) {
    app.dispatch(Event::FocusNext { reverse }).unwrap();
}

fn edit(app: &mut Runtime<Demo>, key: EditKey) {
    app.dispatch(Event::Edit {
        key,
        extend: false,
        by_word: false,
    })
    .unwrap();
}

fn open(app: &mut Runtime<Demo>, viewport: (f32, f32)) {
    settle(app, viewport);
    if app.focused().is_none() {
        tab(app, false);
    }
    if app.focused() == Some("before") {
        tab(app, false);
    }
    assert_eq!(app.focused(), Some("trigger"));
    app.dispatch(Event::Activate).unwrap();
    settle(app, viewport);
    assert_eq!(app.focused(), Some("first"));
}

fn click(app: &mut Runtime<Demo>, rect: Rect) {
    let (x, y) = (rect.x + rect.width / 2., rect.y + rect.height / 2.);
    app.dispatch(Event::PointerDown { x, y }).unwrap();
    app.dispatch(Event::PointerUp { x, y }).unwrap();
}

#[test]
fn layout_keeps_trigger_and_neighbors_fixed_and_mirrors_exact_material_slots() {
    let viewport = (640., 600.);
    for rtl in [false, true] {
        let mut app = Runtime::new(Demo {
            rtl,
            ..Demo::default()
        })
        .unwrap();
        settle(&mut app, viewport);
        let trigger = bounds(&app, "trigger");
        let after = bounds(&app, "after");
        assert_eq!(node(&app, "trigger").expanded_state(), Some(false));
        open(&mut app, viewport);
        assert_eq!(bounds(&app, "trigger"), trigger);
        assert_eq!(bounds(&app, "after"), after);
        let panel = bounds(&app, "commands");
        let row = bounds(&app, "first");
        let label = bounds(&app, "first-label");
        let leading = bounds(&app, "first-leading");
        let trailing = bounds(&app, "first-trailing");
        let separator = bounds(&app, "group-separator");
        assert_eq!(panel.width, 240.);
        assert_eq!(panel.y - trigger.y - trigger.height, 4.);
        assert_eq!(row.y - panel.y, 8.);
        assert_eq!(row.height, 48.);
        assert_eq!((leading.width, leading.height), (24., 24.));
        assert_eq!((trailing.width, trailing.height), (24., 24.));
        let (left, right) = if rtl {
            (trailing, leading)
        } else {
            (leading, trailing)
        };
        assert_eq!(left.x - row.x, 12.);
        assert_eq!(row.x + row.width - right.x - right.width, 12.);
        assert_eq!(label.x - left.x - left.width, 12.);
        assert_eq!(right.x - label.x - label.width, 12.);
        assert_eq!(
            separator,
            Rect::new(row.x, row.y + row.height + 8., row.width, 1.)
        );
        assert_eq!(
            bounds(&app, "disabled").y - separator.y - separator.height,
            8.
        );
        assert_eq!(node(&app, "commands").role(), Role::Menu);
        assert_eq!(node(&app, "commands").name(), "報告操作");
        let theme = if rtl { Theme::dark() } else { Theme::light() };
        assert_eq!(
            node(&app, "commands").style().background,
            theme.colors.surface_container
        );
        assert_eq!(
            node(&app, "commands").style().corners,
            Some(shape::EXTRA_SMALL)
        );
        assert_eq!(
            node(&app, "group-separator").style().background,
            theme.colors.outline_variant
        );
        let frame = app.accessibility_frame().unwrap();
        for (key, role) in [
            ("trigger", Role::Button),
            ("commands", Role::Menu),
            ("first", Role::MenuItem),
            ("group-separator", Role::Separator),
        ] {
            assert_eq!(
                frame
                    .node(app.dom().by_id(key).unwrap())
                    .unwrap()
                    .node
                    .role(),
                role
            );
        }
        assert!(frame
            .node(app.dom().by_id("first-leading").unwrap())
            .is_none());
        assert!(frame.node(app.dom().by_id("after").unwrap()).is_none());
        assert_eq!(node(&app, "trigger").expanded_state(), Some(true));
    }
}

#[test]
fn dispatch_keeps_disabled_items_in_navigation_without_running_them() {
    let viewport = (640., 600.);
    let mut app = Runtime::new(Demo::default()).unwrap();
    open(&mut app, viewport);
    edit(&mut app, EditKey::Down);
    assert_eq!(app.focused(), Some("disabled"));
    settle(&mut app, viewport);
    {
        let accessibility = app.accessibility_frame().unwrap();
        let id = app.dom().by_id("disabled").unwrap();
        assert!(accessibility.node(id).unwrap().node.is_disabled());
        assert!(accessibility.activation_target(id).is_none());
    }
    assert!(!app.dispatch(Event::Activate).unwrap());
    let disabled = bounds(&app, "disabled");
    click(&mut app, disabled);
    assert!(app.app().open);
    assert!(app.app().chosen.is_none());
    assert_eq!(app.focused(), Some("disabled"));
    edit(&mut app, EditKey::Down);
    assert_eq!(app.focused(), Some("last"));
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.app().chosen, Some(3));
    assert_eq!(app.focused(), Some("trigger"));
}

#[test]
fn menu_dismissal_restores_focus_or_continues_tab_and_never_clicks_through() {
    let viewport = (640., 600.);
    let mut app = Runtime::new(Demo::default()).unwrap();
    open(&mut app, viewport);
    let after = bounds(&app, "after");
    click(&mut app, after);
    assert_eq!(app.app().dismissed, 1);
    assert_eq!(app.app().background, 0);
    assert_eq!(app.focused(), Some("trigger"));
    open(&mut app, viewport);
    app.dispatch(Event::Cancel).unwrap();
    assert_eq!(app.focused(), Some("trigger"));
    open(&mut app, viewport);
    tab(&mut app, false);
    assert!(!app.app().open);
    assert_eq!(app.focused(), Some("after"));
    tab(&mut app, true);
    open(&mut app, viewport);
    tab(&mut app, true);
    assert_eq!(app.focused(), Some("before"));
    assert_eq!(app.app().dismissed, 4);
    open(&mut app, viewport);
    app.apply(Message::HideTrigger).unwrap();
    assert_eq!(app.focused(), Some("after"));
}

#[test]
fn natural_content_size_responds_to_wrapping_and_constraints_without_losing_focus() {
    let mut app = Runtime::new(Demo {
        rtl: true,
        scale: 2.,
        wrapped: true,
        ..Demo::default()
    })
    .unwrap();
    let roomy = (320., 760.);
    open(&mut app, roomy);
    let panel = bounds(&app, "commands");
    assert!(bounds(&app, "first").height > 48.);
    let measured = bounds(&app, "commands-content").height;
    assert_eq!(panel.height, measured);
    assert_eq!(app.app().measurement.content_height(), Some(measured));
    let short = (320., 392.);
    settle(&mut app, short);
    let bounded = bounds(&app, "commands");
    assert!(bounded.height < measured && bounded.height >= 48.);
    assert_eq!(bounded, bounded.intersect(Rect::new(8., 8., 304., 376.)));
    assert_eq!(app.focused(), Some("first"));
    let original_offset = app.scroll_metrics("commands").unwrap().offset.1;
    app.dispatch(Event::Scroll {
        x: bounded.x + bounded.width / 2.,
        y: bounded.y + bounded.height / 2.,
        delta_x: 0.,
        delta_y: 24.,
    })
    .unwrap();
    settle(&mut app, short);
    assert!(app.scroll_metrics("commands").unwrap().offset.1 > original_offset);
    assert!(app.app().open);
    edit(&mut app, EditKey::End);
    settle(&mut app, short);
    assert_eq!(app.focused(), Some("last"));
    let last = bounds(&app, "last");
    assert_eq!(last, last.intersect(bounds(&app, "commands")));
    assert!(app.scroll_metrics("commands").unwrap().offset.1 > 0.);
}

#[test]
fn popup_uses_the_anchored_rectangles_instead_of_the_wide_layout_wrapper() {
    for width in [Length::Px(100.), Length::Percent(0.5)] {
        let viewport = (640., 600.);
        let mut app = Runtime::new(Demo {
            trigger_width: Some(width),
            ..Demo::default()
        })
        .unwrap();
        settle(&mut app, viewport);
        let trigger = bounds(&app, "trigger");
        let after = bounds(&app, "after");
        open(&mut app, viewport);
        assert_eq!(bounds(&app, "trigger"), trigger);
        assert_eq!(bounds(&app, "after"), after);

        let panel = bounds(&app, "commands");
        click(&mut app, Rect::new(panel.x + 2., panel.y + 3., 0., 0.));
        assert!(
            app.app().open,
            "blank scrollable panel padding stays inside"
        );
        assert_eq!(app.focused(), Some("first"));

        let smaller = (440., 600.);
        settle(&mut app, smaller);
        let trigger = bounds(&app, "trigger");
        let after = bounds(&app, "after");
        let x = trigger.x + trigger.width + 40.;
        let y = trigger.y + trigger.height / 2.;
        assert!(bounds(&app, "commands-scope").contains(x, y));
        assert!(!trigger.contains(x, y));
        assert!(!bounds(&app, "commands").contains(x, y));
        click(&mut app, Rect::new(x, y, 0., 0.));
        assert_eq!(app.app().dismissed, 1);
        assert_eq!(app.focused(), Some("trigger"));
        settle(&mut app, smaller);
        assert_eq!(bounds(&app, "trigger"), trigger);
        assert_eq!(bounds(&app, "after"), after);

        open(&mut app, smaller);
        click(&mut app, trigger);
        assert!(!app.app().open, "the opener still receives its own action");
        assert_eq!(
            app.app().dismissed,
            1,
            "the opener did not use outside dismiss"
        );
        assert_eq!(app.focused(), Some("trigger"));
    }
}
