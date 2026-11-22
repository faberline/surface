use surface::{Corners, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AnimationFrame, Application, EditKey, Event, FrameInput, InputAnimationUpdate, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, ButtonGroupAppearance, ButtonGroupItem, ButtonGroupKind,
    ButtonGroupMotion, ButtonGroupOptions, ButtonGroupOverflow, ButtonGroupSelection,
    ButtonGroupShape, ButtonGroupSize, ButtonGroupWidth, Theme, VectorIcon,
};

#[derive(Clone, Copy)]
enum Message {
    Press(usize),
    Menu(bool),
    Resize(f32),
}

const IDS: [&str; 4] = ["first", "second", "third", "fourth"];

struct Demo {
    kind: ButtonGroupKind,
    size: ButtonGroupSize,
    shape: ButtonGroupShape,
    direction: LayoutDirection,
    width: f32,
    scale: f32,
    selected: Vec<&'static str>,
    required: bool,
    multiple: bool,
    disable_third: bool,
    use_menu: bool,
    menu: bool,
    icons: bool,
    mixed_icon: bool,
    dark: bool,
    hits: Vec<usize>,
    motion: ButtonGroupMotion,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            kind: ButtonGroupKind::Standard,
            size: ButtonGroupSize::Small,
            shape: ButtonGroupShape::Round,
            direction: LayoutDirection::LeftToRight,
            width: 620.,
            scale: 1.,
            selected: vec!["second"],
            required: false,
            multiple: false,
            disable_third: false,
            use_menu: false,
            menu: false,
            icons: false,
            mixed_icon: false,
            dark: false,
            hits: Vec::new(),
            motion: ButtonGroupMotion::default(),
        }
    }
}

impl Application for Demo {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Press(index) => {
                let id = IDS[index];
                self.hits.push(index);
                self.motion.pulse(id);
                if index < 3 || self.kind == ButtonGroupKind::Connected {
                    if let Some(index) = self.selected.iter().position(|selected| *selected == id) {
                        self.selected.remove(index);
                    } else {
                        if !self.multiple {
                            self.selected.clear();
                        }
                        self.selected.push(id);
                    }
                }
                self.menu = false;
            }
            Message::Menu(open) => self.menu = open,
            Message::Resize(width) => self.width = width,
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn observes_frame_input(&self) -> bool {
        true
    }

    fn animation_frame_with_press(
        &mut self,
        frame: AnimationFrame<'_>,
        _input: FrameInput<'_>,
        pressed: Option<&str>,
    ) -> InputAnimationUpdate {
        InputAnimationUpdate {
            animation: self.motion.advance("group", &frame, pressed),
            suppress_touch_activation: false,
        }
    }

    fn view(&self) -> View<Message> {
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let selection = if self.multiple {
            ButtonGroupSelection::Multiple {
                selected: &self.selected,
                required: self.required,
            }
        } else {
            ButtonGroupSelection::Single {
                selected: self.selected.first().copied(),
                required: self.required,
            }
        };
        let options = ButtonGroupOptions {
            id: "group",
            name: "顯示選項",
            kind: self.kind,
            size: self.size,
            shape: self.shape,
            direction: self.direction,
            selection,
            available_width: self.width,
            appearance: ButtonGroupAppearance::Filled,
        };
        let icon = VectorIcon::new([[2., 2.], [22., 2.], [12., 22.]]).unwrap();
        let items = IDS
            .into_iter()
            .enumerate()
            .map(|(index, id)| ButtonGroupItem {
                id,
                label: ["第一個篩選", "第二個標籤", "第三項資料", "其他操作"][index],
                icon: (self.icons || (self.mixed_icon && index == 3)).then(|| icon.clone()),
                icon_only: self.icons || (self.mixed_icon && index == 3),
                appearance: None,
                width: if self.icons || self.use_menu || self.scale > 1. {
                    ButtonGroupWidth::Intrinsic
                } else {
                    ButtonGroupWidth::Fixed(100.)
                },
                toggle: index < 3,
                disabled: index == 2 && self.disable_third,
                on_press: Message::Press(index),
            });
        let overflow = self.use_menu.then_some(ButtonGroupOverflow {
            label: "更多操作",
            open: self.menu,
            on_toggle: Message::Menu(!self.menu),
            on_dismiss: Message::Menu(false),
        });
        View::new(Node::new(Role::Window, "Demo").styled(Style::column().padding(16.)))
            .children([theme.button_group(options, items, &self.motion, overflow)])
    }
}

struct TextMetrics;

impl TextMeasurer for TextMetrics {
    fn measure(&mut self, text: &str, size: f32, max: Option<f32>) -> (f32, f32) {
        let chars = text.chars().count().max(1);
        let per_line = max.map_or(chars, |max| ((max / size).floor() as usize).max(1));
        (
            (chars.min(per_line) as f32) * size,
            chars.div_ceil(per_line) as f32 * size,
        )
    }
}

fn paint(app: &mut Runtime<Demo>, time: f64) {
    app.set_time(time);
    app.render(900., 600., &mut TextMetrics).unwrap();
}

fn geometry(app: &Runtime<Demo>, id: &str) -> Rect {
    app.computed().unwrap().nodes[app.dom().by_id(id).unwrap().index()].bounds
}

fn node<'a>(app: &'a Runtime<Demo>, id: &str) -> &'a Node {
    app.dom().get(app.dom().by_id(id).unwrap()).unwrap()
}

fn close(value: f32, target: f32) {
    assert!((value - target).abs() <= 0.3, "{value} != {target}");
}

fn key(app: &mut Runtime<Demo>, key: EditKey) {
    app.dispatch(Event::Edit {
        key,
        extend: false,
        by_word: false,
    })
    .unwrap();
}

fn center(rect: Rect) -> (f32, f32) {
    (rect.x + rect.width / 2., rect.y + rect.height / 2.)
}

#[test]
fn standard_sizes_keep_visual_spacing_and_real_hit_areas() {
    use ButtonGroupSize::*;
    for size in [ExtraSmall, Small, Medium, Large, ExtraLarge] {
        let mut app = Runtime::new(Demo {
            size,
            width: 850.,
            icons: true,
            ..Demo::default()
        })
        .unwrap();
        paint(&mut app, 0.);
        let index = [ExtraSmall, Small, Medium, Large, ExtraLarge]
            .iter()
            .position(|it| *it == size)
            .unwrap();
        let first = geometry(&app, "first-face");
        let second = geometry(&app, "second-face");
        close(first.height, [32., 40., 56., 96., 136.][index]);
        close(first.width, [32., 40., 56., 96., 136.][index]);
        close(
            second.x - first.x - first.width,
            [18., 12., 8., 8., 8.][index],
        );
        assert!(geometry(&app, "first").width >= 48.);
        assert!(geometry(&app, "first").height >= 48.);
    }
}

#[test]
fn standard_press_moves_only_neighbors_and_releases_to_the_same_action() {
    let mut app = Runtime::new(Demo::default()).unwrap();
    paint(&mut app, 0.);
    paint(&mut app, 16.);
    let old = ["first", "second", "third", "fourth", "group-row"].map(|id| geometry(&app, id));
    let (x, y) = (old[1].x + 1., old[1].y + 20.);
    app.dispatch(Event::PointerDown { x, y }).unwrap();
    paint(&mut app, 32.);
    paint(&mut app, 132.);
    let pressed = ["first", "second", "third", "fourth", "group-row"].map(|id| geometry(&app, id));
    assert!(pressed[1].width > old[1].width + 10.);
    assert!(pressed[0].width < old[0].width);
    assert!(pressed[2].width < old[2].width);
    assert_eq!(pressed[3], old[3]);
    close(pressed[4].width, old[4].width);
    app.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(app.app().hits, [1]);
    paint(&mut app, 160.);
    paint(&mut app, 800.);
    for (index, id) in IDS.iter().enumerate() {
        close(geometry(&app, id).width, old[index].width);
    }

    let (x, y) = center(geometry(&app, "first"));
    app.dispatch_touch(Event::PointerDown { x, y }).unwrap();
    paint(&mut app, 820.);
    paint(&mut app, 930.);
    assert!(geometry(&app, "first").width > old[0].width);
    app.cancel_pointer().unwrap();
    paint(&mut app, 950.);
    paint(&mut app, 1500.);
    assert_eq!(app.app().hits, [1]);
    close(geometry(&app, "first").width, old[0].width);

    app.set_reduced_motion(true);
    app.dispatch(Event::PointerDown { x, y }).unwrap();
    paint(&mut app, 1510.);
    close(geometry(&app, "first").width, old[0].width);
}

#[test]
fn connected_rtl_first_focus_and_required_selection_leave_width_unchanged() {
    let mut app = Runtime::new(Demo {
        kind: ButtonGroupKind::Connected,
        direction: LayoutDirection::RightToLeft,
        shape: ButtonGroupShape::Square,
        size: ButtonGroupSize::ExtraSmall,
        disable_third: true,
        required: true,
        dark: true,
        ..Demo::default()
    })
    .unwrap();
    paint(&mut app, 0.);
    let first = geometry(&app, "first");
    let second = geometry(&app, "second");
    assert!(first.x > second.x);
    assert_eq!(
        node(&app, "first-face").style().corners,
        Some(Corners::new(8., 4., 4., 8.))
    );
    assert_eq!(
        node(&app, "second-face").style().corners,
        Some(Corners::full())
    );
    app.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(app.focused(), Some("first"));
    key(&mut app, EditKey::Left);
    assert_eq!(app.focused(), Some("second"));
    app.dispatch(Event::Activate).unwrap();
    assert!(app.app().hits.is_empty());
    key(&mut app, EditKey::Left);
    assert_eq!(app.focused(), Some("fourth"));
    assert_eq!(node(&app, "second").is_checked(), Some(true));
    let (x, y) = center(second);
    app.dispatch(Event::PointerDown { x, y }).unwrap();
    paint(&mut app, 20.);
    paint(&mut app, 120.);
    assert_eq!(geometry(&app, "second"), second);
    assert_eq!(
        node(&app, "second-face").style().corners,
        Some(Corners::all(4.))
    );
    app.dispatch(Event::PointerUp { x, y }).unwrap();
    assert!(app.app().hits.is_empty());
    key(&mut app, EditKey::Right);
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.app().hits, [0]);
    assert_eq!(node(&app, "first").is_checked(), Some(true));
}

#[test]
fn overflow_preserves_one_row_and_restores_focus_across_resize() {
    let mut app = Runtime::new(Demo {
        use_menu: true,
        scale: 2.,
        width: 850.,
        ..Demo::default()
    })
    .unwrap();
    paint(&mut app, 0.);
    paint(&mut app, 16.);
    assert!(app.dom().by_id("group-overflow").is_none());
    app.dispatch(Event::FocusNext { reverse: false }).unwrap();
    key(&mut app, EditKey::End);
    assert_eq!(app.focused(), Some("fourth"));
    app.apply(Message::Resize(260.)).unwrap();
    paint(&mut app, 32.);
    paint(&mut app, 40.);
    assert_eq!(app.focused(), Some("group-overflow"));
    assert!(app.dom().by_id("fourth").is_none());
    assert_eq!(node(&app, "group-overflow").expanded_state(), Some(false));
    let trigger = geometry(&app, "group-overflow");
    app.dispatch(Event::Activate).unwrap();
    paint(&mut app, 48.);
    let menu = geometry(&app, "group-menu");
    assert!(menu.y >= trigger.y + trigger.height);
    assert_eq!(node(&app, "group-overflow").expanded_state(), Some(true));
    assert_eq!(node(&app, "fourth").role(), Role::MenuItem);
    assert_eq!(app.focused(), Some("fourth"));
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.app().hits, [3]);
    assert_eq!(app.focused(), Some("group-overflow"));
    app.apply(Message::Resize(850.)).unwrap();
    paint(&mut app, 64.);
    assert_eq!(app.focused(), Some("first"));
    assert!(app.dom().by_id("group-overflow").is_none());
    assert_eq!(node(&app, "fourth").role(), Role::Button);

    app.apply(Message::Resize(260.)).unwrap();
    paint(&mut app, 80.);
    paint(&mut app, 96.);
    key(&mut app, EditKey::End);
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.focused(), Some("fourth"));
    app.apply(Message::Resize(850.)).unwrap();
    paint(&mut app, 112.);
    assert_eq!(app.focused(), Some("fourth"));
    assert_eq!(node(&app, "fourth").role(), Role::Button);
}

#[test]
fn connected_large_text_keeps_a_uniform_row_and_reveals_offscreen_choice() {
    let mut app = Runtime::new(Demo {
        kind: ButtonGroupKind::Connected,
        size: ButtonGroupSize::ExtraSmall,
        width: 118.,
        scale: 2.,
        multiple: true,
        required: true,
        selected: vec!["first", "second"],
        mixed_icon: true,
        ..Demo::default()
    })
    .unwrap();
    paint(&mut app, 0.);
    paint(&mut app, 16.);
    paint(&mut app, 32.);
    let first = geometry(&app, "first-face");
    let viewport = geometry(&app, "group-viewport");
    assert!(first.height > 32.);
    assert!(geometry(&app, "group-row").width > viewport.width);
    for id in IDS {
        let face = geometry(&app, &format!("{id}-face"));
        close(face.y, first.y);
        close(face.height, first.height);
        assert!(geometry(&app, id).width >= 48.);
    }
    let label = geometry(&app, "first-label");
    assert!(label.width >= 28.);
    close(label.height, first.height);

    app.dispatch(Event::FocusNext { reverse: false }).unwrap();
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.app().selected, ["second"]);
    key(&mut app, EditKey::Right);
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.app().selected, ["second"]);
    key(&mut app, EditKey::End);
    assert_eq!(app.focused(), Some("fourth"));
    paint(&mut app, 48.);
    assert!(node(&app, "group-viewport").style().scroll_x > 0.);
    assert!(
        geometry(&app, "fourth").x + geometry(&app, "fourth").width
            <= viewport.x + viewport.width + 0.3
    );
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.app().selected, ["second", "fourth"]);
}

#[test]
fn rtl_overflow_starts_at_logical_start_before_focus_moves() {
    let mut app = Runtime::new(Demo {
        kind: ButtonGroupKind::Connected,
        direction: LayoutDirection::RightToLeft,
        width: 118.,
        ..Demo::default()
    })
    .unwrap();
    paint(&mut app, 0.);
    paint(&mut app, 16.);
    let viewport = geometry(&app, "group-viewport");
    let first = geometry(&app, "first");
    close(first.x + first.width, viewport.x + viewport.width);
    assert!(geometry(&app, "fourth").x < viewport.x);
    app.dispatch(Event::FocusNext { reverse: false }).unwrap();
    key(&mut app, EditKey::End);
    paint(&mut app, 32.);
    close(geometry(&app, "fourth").x, viewport.x);
}
