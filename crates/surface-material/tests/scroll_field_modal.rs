use surface::{Insets, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityScrollDirection, Application, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime,
    View,
};
use surface_material::{
    scroll_field_modal_layout, shape::LayoutDirection, ScrollFieldModalOptions, ScrollFieldSize,
    Theme,
};

const DIALOG: &str = "measure-dialog";
const BODY: &str = "measure-body";
const CANCEL: &str = "measure-cancel";
const SAVE: &str = "measure-save";
const OPENER: &str = "open-measure";
const BACKGROUND: &str = "other-action";
const QUANTITY: &str = "quantity";
const UNIT: &str = "unit";

const COMPACT: Rect = Rect {
    x: 0.,
    y: 0.,
    width: 320.,
    height: 240.,
};

fn options(viewport: Rect) -> ScrollFieldModalOptions<'static> {
    ScrollFieldModalOptions {
        id: "measure",
        label: "選擇容量",
        body_label: "數量和單位",
        cancel_label: "取消",
        save_label: "儲存",
        initial_focus: Some(QUANTITY),
        restore_focus_to: Some(OPENER),
        viewport,
        direction: LayoutDirection::RightToLeft,
        preferred_size: (640., 440.),
        minimum_body_height: 48.,
        text_scale: 2.,
    }
}

#[test]
fn modal_plan_centers_in_safe_area_and_reserves_complete_targets() {
    let layout = scroll_field_modal_layout(options(Rect::new(25., 40., 900., 700.))).unwrap();
    assert_eq!(layout.bounds, Rect::new(195., 170., 560., 440.));
    assert_eq!((layout.body_width, layout.body_height), (526., 358.));
    assert!(!layout.actions_stacked);

    let compact = scroll_field_modal_layout(options(COMPACT)).unwrap();
    assert_eq!(compact.bounds, Rect::new(16., 16., 288., 208.));
    assert_eq!((compact.body_width, compact.body_height), (254., 126.));
    assert!(!compact.actions_stacked);

    let stacked = scroll_field_modal_layout(ScrollFieldModalOptions {
        cancel_label: "取消目前設定",
        save_label: "儲存容量設定",
        ..options(COMPACT)
    })
    .unwrap();
    assert!(stacked.actions_stacked);
    assert_eq!(stacked.bounds, compact.bounds);
    assert_eq!(stacked.body_height, 70.);

    for (width, height) in [(113., 240.), (320., 161.), (f32::INFINITY, 240.)] {
        assert!(scroll_field_modal_layout(options(Rect::new(0., 0., width, height))).is_none());
    }

    let minimum = ScrollFieldSize::Large.compact_editor_height(2.);
    assert_eq!(minimum, 104.);
    let short = ScrollFieldModalOptions {
        viewport: Rect::new(0., 0., 320., 218.),
        minimum_body_height: minimum,
        ..options(COMPACT)
    };
    assert_eq!(
        scroll_field_modal_layout(short).unwrap().body_height,
        minimum
    );
    assert!(scroll_field_modal_layout(ScrollFieldModalOptions {
        viewport: Rect::new(0., 0., 320., 217.),
        ..short
    })
    .is_none());
    assert!(scroll_field_modal_layout(ScrollFieldModalOptions {
        cancel_label: "取消目前設定",
        save_label: "儲存容量設定",
        viewport: COMPACT,
        ..short
    })
    .is_none());
}

#[derive(Clone)]
enum Message {
    Open,
    Cancel,
    Save,
    Background,
    ReadQuantity(String),
    ReadUnit(String),
    SubmitQuantity,
}

struct App {
    long_actions: bool,
    open: bool,
    quantity: String,
    unit: String,
    saved: String,
    cancels: usize,
    saves: usize,
    local_submits: usize,
    background_presses: usize,
}

impl App {
    fn options(&self) -> ScrollFieldModalOptions<'static> {
        let normal = options(COMPACT);
        if self.long_actions {
            ScrollFieldModalOptions {
                cancel_label: "取消目前設定",
                save_label: "儲存容量設定",
                ..normal
            }
        } else {
            normal
        }
    }
}

impl Application for App {
    type Message = Message;

    fn text_scale(&self) -> f32 {
        2.
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => {
                self.quantity.clone_from(&self.saved);
                self.open = true;
            }
            Message::Cancel => {
                self.quantity.clone_from(&self.saved);
                self.cancels += 1;
                self.open = false;
            }
            Message::Save => {
                self.saves += 1;
                if self
                    .quantity
                    .parse::<u32>()
                    .is_ok_and(|value| value % 2 == 0)
                {
                    self.saved.clone_from(&self.quantity);
                    self.open = false;
                }
            }
            Message::Background => self.background_presses += 1,
            Message::ReadQuantity(raw) => self.quantity = raw,
            Message::ReadUnit(raw) => self.unit = raw,
            Message::SubmitQuantity => self.local_submits += 1,
        }
    }

    fn view(&self) -> View<Message> {
        let interact = |id: &str, name: &str, style, message| {
            View::new(Node::new(Role::Button, name).id(id).styled(style))
                .on_press(message)
                .accessible_activation()
        };
        let opener = interact(OPENER, "開啟", square(), Message::Open);
        let background = interact(
            BACKGROUND,
            "其他",
            square().fixed(Insets {
                left: Length::Px(240.),
                top: Length::Px(4.),
                ..Default::default()
            }),
            Message::Background,
        );
        let modal = self.open.then(|| {
            let options = self.options();
            let layout = scroll_field_modal_layout(options).unwrap();
            let body = View::new(
                Node::new(Role::Group, "").styled(
                    Style::column()
                        .width(Length::Px(layout.body_width))
                        .gap(12.),
                ),
            )
            .children([
                View::new(
                    Node::new(Role::TextInput, "數量")
                        .id(QUANTITY)
                        .text(&self.quantity)
                        .styled(square()),
                )
                .on_input(Message::ReadQuantity)
                .on_key(|key| (key.key == Key::Enter).then_some(Message::SubmitQuantity))
                .accessibility_scroll_into_view(),
                View::new(
                    Node::new(Role::Group, "").styled(Style::column().height(Length::Px(180.))),
                ),
                View::new(
                    Node::new(Role::TextInput, "單位")
                        .id(UNIT)
                        .text(&self.unit)
                        .styled(square()),
                )
                .on_input(Message::ReadUnit)
                .accessibility_scroll_into_view(),
            ]);
            Theme::dark()
                .scroll_field_modal(options, body, Message::Cancel, Message::Save)
                .unwrap()
        });
        View::new(Node::new(Role::Window, "設定"))
            .children([opener, background].into_iter().chain(modal))
    }
}

fn square() -> Style {
    Style::column()
        .width(Length::Px(48.))
        .height(Length::Px(48.))
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, limit: Option<f32>) -> (f32, f32) {
        let natural = text.chars().count() as f32 * size;
        let width = natural.min(limit.unwrap_or(natural)).max(1.);
        let lines = (natural / width).ceil().max(1.);
        (width, size * 1.4 * lines)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime
        .render_extended(COMPACT.width, COMPACT.height, &mut Metrics)
        .unwrap();
}

fn open(long_actions: bool) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        long_actions,
        open: false,
        quantity: "1200".into(),
        unit: "毫升".into(),
        saved: "1200".into(),
        cancels: 0,
        saves: 0,
        local_submits: 0,
        background_presses: 0,
    })
    .unwrap();
    render(&mut runtime);
    assert!(runtime.activate(OPENER).unwrap());
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some(QUANTITY));
    runtime
}

fn rect(runtime: &Runtime<App>, id: &str) -> Rect {
    let id = runtime.dom().by_id(id).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn assert_fits_in_body(runtime: &Runtime<App>, id: &str) {
    let target = rect(runtime, id);
    let outer = Rect::new(
        target.x - 5.,
        target.y - 5.,
        target.width + 10.,
        target.height + 10.,
    );
    assert_eq!(outer.intersect(rect(runtime, BODY)), outer);
}

fn tab(runtime: &mut Runtime<App>, reverse: bool, expected: &str) {
    runtime.dispatch(Event::FocusNext { reverse }).unwrap();
    render(runtime);
    assert_eq!(runtime.focused(), Some(expected));
}

fn click(runtime: &mut Runtime<App>, x: f32, y: f32) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    render(runtime);
}

#[test]
fn compact_rtl_modal_traps_tab_and_reveals_fields_without_moving_actions() {
    for long_actions in [false, true] {
        let mut runtime = open(long_actions);
        let dom = runtime.dom();
        let dialog = dom.by_id(DIALOG).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        assert_eq!(frame.modal_root(), Some(dialog));
        let panel_node = frame.node(dialog).unwrap().node;
        assert_eq!(panel_node.role(), Role::Dialog);
        assert_eq!(panel_node.name(), "選擇容量");
        assert_eq!(
            frame.node(dom.by_id(BODY).unwrap()).unwrap().node.name(),
            "數量和單位"
        );
        assert!(frame
            .scroll_page_target(dom.by_id(BODY).unwrap(), AccessibilityScrollDirection::Down,)
            .is_some());
        assert!(frame
            .activation_target(dom.by_id(BACKGROUND).unwrap())
            .is_none());
        assert!(frame
            .activation_target(dom.by_id(CANCEL).unwrap())
            .is_some());
        assert!(frame.activation_target(dom.by_id(SAVE).unwrap()).is_some());
        let bounds = scroll_field_modal_layout(runtime.app().options()).unwrap();
        assert_eq!(rect(&runtime, DIALOG), bounds.bounds);

        let cancel = rect(&runtime, CANCEL);
        let save = rect(&runtime, SAVE);
        for target in [cancel, save] {
            assert!(target.width >= 48.);
            assert_eq!(target.height, 48.);
            assert_eq!(target.intersect(bounds.bounds), target);
            assert!(target.y >= rect(&runtime, BODY).y + rect(&runtime, BODY).height);
        }
        assert_eq!(
            cancel.intersect(save).width * cancel.intersect(save).height,
            0.
        );
        if long_actions {
            assert!(cancel.y < save.y);
        } else {
            assert!(save.x < cancel.x);
            assert_eq!(save.y, cancel.y);
        }

        assert_fits_in_body(&runtime, QUANTITY);
        let before = runtime.scroll_metrics(BODY).unwrap().offset.1;
        tab(&mut runtime, false, UNIT);
        assert_fits_in_body(&runtime, UNIT);
        assert!(runtime.scroll_metrics(BODY).unwrap().offset.1 > before);
        assert_eq!(rect(&runtime, CANCEL), cancel);
        assert_eq!(rect(&runtime, SAVE), save);
        tab(&mut runtime, false, CANCEL);
        tab(&mut runtime, false, SAVE);
        tab(&mut runtime, false, QUANTITY);
        tab(&mut runtime, true, SAVE);
        runtime.dispatch(Event::Activate).unwrap();
        assert!(!runtime.app().open);
        assert_eq!(runtime.app().saves, 1);
        assert_eq!(runtime.focused(), Some(OPENER));
    }
}

#[test]
fn field_edit_cancel_and_save_stay_caller_owned() {
    let mut runtime = open(false);
    let enter = KeyPress::new(Key::Enter, Modifiers::default());
    assert!(runtime.key(&enter).unwrap());
    assert_eq!(runtime.app().local_submits, 1);
    assert_eq!(runtime.app().saves, 0);
    assert!(runtime.app().open);

    runtime.apply(Message::ReadQuantity("1300".into())).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.app().saved, "1200");
    assert!(runtime.activate(CANCEL).unwrap());
    assert_eq!(runtime.app().saves, 0);
    assert_eq!(runtime.app().quantity, "1200");
    assert_eq!(runtime.focused(), Some(OPENER));

    assert!(runtime.activate(OPENER).unwrap());
    runtime.apply(Message::ReadQuantity("1300".into())).unwrap();
    render(&mut runtime);
    assert!(runtime.activate(SAVE).unwrap());
    assert_eq!(runtime.app().saved, "1300");
    assert_eq!(runtime.app().saves, 1);
    assert_eq!(runtime.focused(), Some(OPENER));
}

#[test]
fn action_guards_and_escape_give_composition_priority() {
    let mut runtime = open(false);
    assert!(runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "注音".into(),
            selection: None,
        }))
        .unwrap());
    render(&mut runtime);
    for action in [CANCEL, SAVE] {
        let target = rect(&runtime, action);
        click(&mut runtime, target.x + 24., target.y + 24.);
        assert_eq!(runtime.focused(), Some(QUANTITY));
        assert!(runtime.input_snapshot().unwrap().composing);
        assert!(!runtime.activate(action).unwrap());
    }
    assert_eq!(runtime.app().saves, 0);
    assert_eq!(runtime.app().cancels, 0);
    assert!(!runtime
        .key(&KeyPress::new(Key::Enter, Modifiers::default()))
        .unwrap());
    assert_eq!(runtime.app().local_submits, 0);
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert!(runtime.app().open);
    assert!(!runtime.input_snapshot().unwrap().composing);
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert!(!runtime.app().open);
    assert_eq!(runtime.app().cancels, 1);
    assert_eq!(runtime.focused(), Some(OPENER));
}

#[test]
fn backdrop_blocks_background_press_and_dismisses_with_cancel() {
    let mut runtime = open(false);
    runtime.apply(Message::ReadQuantity("1300".into())).unwrap();
    render(&mut runtime);
    assert!(runtime.activate(BACKGROUND).is_err());
    let background = rect(&runtime, BACKGROUND);
    click(&mut runtime, background.x + 6., background.y + 8.);
    assert_eq!(runtime.app().background_presses, 0);
    assert_eq!(runtime.app().saves, 0);
    assert_eq!(runtime.app().cancels, 1);
    assert_eq!(runtime.app().quantity, "1200");
    assert_eq!(runtime.focused(), Some(OPENER));

    click(&mut runtime, background.x + 6., background.y + 8.);
    assert_eq!(runtime.app().background_presses, 1);
}

#[test]
fn backdrop_first_ends_active_composition_before_later_dismissal() {
    let mut runtime = open(false);
    assert!(runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "注音".into(),
            selection: None,
        }))
        .unwrap());
    render(&mut runtime);
    let background = rect(&runtime, BACKGROUND);
    let point = (background.x + 6., background.y + 8.);
    click(&mut runtime, point.0, point.1);
    assert!(runtime.app().open);
    assert!(!runtime.input_snapshot().unwrap().composing);
    assert_eq!(runtime.app().cancels, 0);
    click(&mut runtime, point.0, point.1);
    assert!(!runtime.app().open);
    assert_eq!(runtime.app().cancels, 1);
    assert_eq!(runtime.app().background_presses, 0);
    assert_eq!(runtime.app().saves, 0);
    assert_eq!(runtime.focused(), Some(OPENER));
}

#[test]
fn caller_can_reject_save_and_keep_the_modal_open() {
    let mut runtime = open(false);
    runtime.apply(Message::ReadQuantity("1301".into())).unwrap();
    render(&mut runtime);
    assert!(runtime.activate(SAVE).unwrap());
    render(&mut runtime);
    assert_eq!(runtime.app().saves, 1);
    assert_eq!(runtime.app().saved, "1200");
    assert_eq!(runtime.app().quantity, "1301");
    assert!(runtime.app().open);

    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.modal_root(), runtime.dom().by_id(DIALOG));
    assert!(runtime.activate(CANCEL).unwrap());
    assert_eq!(runtime.app().quantity, "1200");
    assert_eq!(runtime.focused(), Some(OPENER));
}
