use surface::{Insets, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{Application, Event, ImeEvent, Runtime, View};
use surface_material::{
    scroll_field_popup_layout, shape::LayoutDirection, ScrollFieldPopupOptions, Theme,
};

const PANEL: &str = "measurement";
const BODY: &str = "measurement-body";
const CLOSE: &str = "measurement-close";
const OPENER: &str = "measurement-opener";
const QUANTITY: &str = "quantity";
const UNIT: &str = "unit";
const OUTSIDE: &str = "outside";
const UNDER: &str = "under";

fn options(
    anchor: Rect,
    viewport: Rect,
    direction: LayoutDirection,
) -> ScrollFieldPopupOptions<'static> {
    ScrollFieldPopupOptions {
        id: PANEL,
        label: "自訂容量",
        body_id: BODY,
        close_id: CLOSE,
        close_label: "關閉容量欄位",
        restore_focus_to: OPENER,
        anchor,
        viewport,
        direction,
        preferred_size: (360., 220.),
    }
}

#[test]
fn popup_prefers_below_and_flips_above_at_the_logical_start_edge() {
    let viewport = Rect::new(10., 20., 800., 500.);
    let below = scroll_field_popup_layout(options(
        Rect::new(60., 80., 120., 48.),
        viewport,
        LayoutDirection::LeftToRight,
    ))
    .unwrap();
    assert!(!below.above);
    assert_eq!(below.bounds, Rect::new(60., 132., 360., 220.));
    assert_eq!((below.body_width, below.body_height), (350., 162.));

    let above = scroll_field_popup_layout(options(
        Rect::new(550., 440., 120., 48.),
        viewport,
        LayoutDirection::RightToLeft,
    ))
    .unwrap();
    assert!(above.above);
    assert_eq!(above.bounds, Rect::new(310., 216., 360., 220.));
}

#[test]
fn narrow_popup_caps_to_the_roomier_side_and_requires_two_reachable_targets() {
    let viewport = Rect::new(0., 0., 320., 240.);
    let narrow = |anchor| options(anchor, viewport, LayoutDirection::RightToLeft);
    let layout = scroll_field_popup_layout(narrow(Rect::new(296., 172., 60., 48.))).unwrap();
    assert!(layout.above);
    assert_eq!(layout.bounds, Rect::new(8., 8., 304., 160.));
    assert_eq!((layout.body_width, layout.body_height), (294., 102.));
    assert!(scroll_field_popup_layout(narrow(Rect::new(60., 96., 120., 48.))).is_none());
    assert!(scroll_field_popup_layout(narrow(Rect::new(320., 20., 36., 48.))).is_none());
    assert!(scroll_field_popup_layout(narrow(Rect::new(f32::NAN, 20., 36., 48.))).is_none());
    assert!(scroll_field_popup_layout(ScrollFieldPopupOptions {
        preferred_size: (57., 180.),
        ..narrow(Rect::new(60., 20., 120., 48.))
    })
    .is_none());
    assert!(scroll_field_popup_layout(ScrollFieldPopupOptions {
        preferred_size: (320., 105.),
        ..narrow(Rect::new(60., 20., 120., 48.))
    })
    .is_none());
}

#[derive(Clone)]
enum Message {
    Open,
    Close,
    Outside,
    Under,
    Edit(String),
    CancelEdit,
}

#[derive(Default)]
struct App {
    open: bool,
    closes: usize,
    outside: usize,
    under: usize,
    raw: String,
    editor_cancels: usize,
}

const VIEWPORT: Rect = Rect {
    x: 0.,
    y: 0.,
    width: 800.,
    height: 560.,
};
const ANCHOR: Rect = Rect {
    x: 20.,
    y: 20.,
    width: 120.,
    height: 48.,
};

fn fixed_box(rect: Rect) -> Style {
    Style::column()
        .width(Length::Px(rect.width))
        .height(Length::Px(rect.height))
        .fixed(Insets {
            left: Length::Px(rect.x),
            top: Length::Px(rect.y),
            ..Default::default()
        })
}

fn button(id: &str, name: &str, rect: Rect, action: Message) -> View<Message> {
    View::new(Node::new(Role::Button, name).id(id).styled(fixed_box(rect)))
        .on_press(action)
        .accessible_activation()
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.open = true,
            Message::Close => {
                self.open = false;
                self.closes += 1;
            }
            Message::Outside => self.outside += 1,
            Message::Under => self.under += 1,
            Message::Edit(raw) => self.raw = raw,
            Message::CancelEdit => {
                self.raw.clear();
                self.editor_cancels += 1;
            }
        }
    }

    fn view(&self) -> View<Message> {
        let opener = button(OPENER, "自訂", ANCHOR, Message::Open).disclosure_expanded(self.open);
        let popup = self.open.then(|| {
            let options = options(ANCHOR, VIEWPORT, LayoutDirection::LeftToRight);
            let plan = scroll_field_popup_layout(options).unwrap();
            let control = Style::column()
                .width(Length::Px(120.))
                .height(Length::Px(48.));
            let quantity = View::new(
                Node::new(Role::TextInput, "數量")
                    .id(QUANTITY)
                    .text(&self.raw)
                    .styled(control.clone()),
            )
            .on_input(Message::Edit)
            .on_cancel_within(Message::CancelEdit)
            .accessibility_scroll_into_view();
            let unit = View::new(Node::new(Role::Button, "單位").id(UNIT).styled(control));
            let rest = View::new(
                Node::new(Role::Group, "").styled(Style::column().height(Length::Px(240.))),
            );
            let body = View::new(
                Node::new(Role::Group, "")
                    .styled(Style::column().width(Length::Px(plan.body_width)).gap(16.)),
            )
            .children([quantity, unit, rest]);
            Theme::light()
                .scroll_field_popup(options, body, Message::Close)
                .unwrap()
        });
        let outside = button(
            OUTSIDE,
            "頁面指令",
            Rect::new(560., 76., 120., 48.),
            Message::Outside,
        );
        let under = button(
            UNDER,
            "下層指令",
            Rect::new(220., 120., 48., 48.),
            Message::Under,
        );
        View::new(Node::new(Role::Window, "設定"))
            .children(std::iter::once(opener).chain(popup).chain([outside, under]))
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, limit: Option<f32>) -> (f32, f32) {
        let natural = text.chars().count() as f32 * size;
        (natural.min(limit.unwrap_or(natural)), size * 1.5)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime
        .render_extended(VIEWPORT.width, VIEWPORT.height, &mut Metrics)
        .unwrap();
}

fn open() -> Runtime<App> {
    let mut runtime = Runtime::new(App::default()).unwrap();
    render(&mut runtime);
    assert!(runtime.activate(OPENER).unwrap());
    render(&mut runtime);
    runtime
}

fn rect(runtime: &Runtime<App>, id: &str) -> Rect {
    let id = runtime.dom().by_id(id).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn click(runtime: &mut Runtime<App>, x: f32, y: f32) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    render(runtime);
}

#[test]
fn modeless_popup_keeps_background_actions_in_its_normal_tab_order() {
    let mut runtime = open();
    let dom = runtime.dom();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.modal_root().is_none());
    let panel = frame.node(dom.by_id(PANEL).unwrap()).unwrap().node;
    assert_eq!(panel.role(), Role::Dialog);
    assert_eq!(panel.name(), "自訂容量");
    assert!(!panel.is_tab_stop());
    assert!(frame
        .activation_target(dom.by_id(OUTSIDE).unwrap())
        .is_some());
    let layout =
        scroll_field_popup_layout(options(ANCHOR, VIEWPORT, LayoutDirection::LeftToRight)).unwrap();
    assert_eq!(rect(&runtime, PANEL), layout.bounds);
    let body = rect(&runtime, BODY);
    let close = rect(&runtime, CLOSE);
    assert_eq!(rect(&runtime, QUANTITY).x - body.x, 5.);
    assert_eq!(rect(&runtime, QUANTITY).y - body.y, 5.);
    assert_eq!(body.height, layout.body_height + 10.);
    assert_eq!((close.width, close.height), (48., 48.));
    assert_eq!(close.y, body.y + body.height);

    for expected in [QUANTITY, UNIT, CLOSE, OUTSIDE] {
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some(expected));
    }
    runtime.dispatch(Event::Activate).unwrap();
    assert_eq!(runtime.app().outside, 1);
    assert!(runtime.app().open);

    render(&mut runtime);
    runtime
        .dispatch(Event::Scroll {
            x: body.x + 20.,
            y: body.y + 24.,
            delta_x: 0.,
            delta_y: 36.,
        })
        .unwrap();
    render(&mut runtime);
    assert!(runtime.scroll_metrics(BODY).unwrap().offset.1 > 0.);
    assert_eq!(rect(&runtime, CLOSE), close);
}

#[test]
fn blank_panel_shields_underlying_clicks_but_outside_focus_is_preserved_on_close() {
    let mut runtime = open();
    click(&mut runtime, 232., 132.);
    assert_eq!(runtime.app().under, 0);
    assert_eq!(runtime.focused(), Some(PANEL));
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert_eq!(runtime.app().closes, 1);
    assert_eq!(runtime.focused(), Some(OPENER));

    runtime.activate(OPENER).unwrap();
    render(&mut runtime);
    click(&mut runtime, 580., 92.);
    assert_eq!(runtime.app().outside, 1);
    assert_eq!(runtime.focused(), Some(OUTSIDE));
    assert!(runtime.app().open);
    runtime.dispatch(Event::Cancel).unwrap();
    assert!(runtime.app().open);
    runtime.apply(Message::Close).unwrap();
    assert_eq!(runtime.app().closes, 2);
    assert_eq!(runtime.focused(), Some(OUTSIDE));

    render(&mut runtime);
    click(&mut runtime, 232., 132.);
    assert_eq!(runtime.app().under, 1);
}

#[test]
fn pending_composition_and_editor_cancel_take_precedence_over_popup_close() {
    let mut runtime = open();
    runtime.activate(QUANTITY).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "注音".into(),
            selection: None,
        }))
        .unwrap();
    render(&mut runtime);
    let close = rect(&runtime, CLOSE);
    click(&mut runtime, close.x + 24., close.y + 24.);
    assert!(runtime.app().open);
    assert_eq!(runtime.focused(), Some(QUANTITY));
    assert!(runtime.input_snapshot().unwrap().composing);

    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert!(runtime.app().open);
    assert!(!runtime.input_snapshot().unwrap().composing);
    assert_eq!(runtime.app().editor_cancels, 0);
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert_eq!(runtime.app().editor_cancels, 1);
    assert!(runtime.app().open);

    runtime.activate(UNIT).unwrap();
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert!(!runtime.app().open);
    assert_eq!(runtime.app().closes, 1);
    assert_eq!(runtime.focused(), Some(OPENER));
}
