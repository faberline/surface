use surface::{Color, Insets, Length, Node, PaintOp, Rect, Role, Style, TextMeasurer};
use surface_app::{Application, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime, View};
use surface_material::{
    full_screen_dialog_layout, shape::LayoutDirection, FullScreenDialogOptions, Theme,
};

const DIALOG: &str = "full";
const HEADER: &str = "full-header";
const BODY: &str = "full-body";
const CLOSE: &str = "full-close";
const ACTION: &str = "full-action";
const TITLE: &str = "full-title";
const ICON: &str = "full-close-icon";
const CAPTION: &str = "full-action-label";
const FIELD: &str = "entry";
const OPENER: &str = "opener";
const FALLBACK: &str = "background";

#[derive(Clone)]
enum Message {
    Open,
    Close,
    Save,
    Input(String),
    Background,
    RemoveOpener,
    KeepEditing,
    Discard,
}

struct Fixture {
    viewport: Rect,
    direction: LayoutDirection,
    scale: f32,
    dark: bool,
    title: &'static str,
    action_label: &'static str,
    action_disabled: bool,
    opener: bool,
    open: bool,
    asking: bool,
    draft: String,
    saved: String,
    close_requests: usize,
    saves: usize,
    background_clicks: usize,
}

impl Fixture {
    fn new(direction: LayoutDirection, scale: f32) -> Self {
        Self {
            viewport: Rect::default(),
            direction,
            scale,
            dark: direction == LayoutDirection::RightToLeft,
            title: "新增行程",
            action_label: "儲存",
            action_disabled: false,
            opener: true,
            open: false,
            asking: false,
            draft: String::new(),
            saved: String::new(),
            close_requests: 0,
            saves: 0,
            background_clicks: 0,
        }
    }

    fn theme(&self) -> Theme {
        if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        }
    }

    fn options(&self) -> FullScreenDialogOptions<'_> {
        FullScreenDialogOptions {
            id: "full",
            title: self.title,
            body_label: "行程內容",
            close_label: "關閉",
            action_label: self.action_label,
            action_disabled: self.action_disabled,
            initial_focus: Some(FIELD),
            restore_focus_to: Some(FALLBACK),
            viewport: self.viewport,
            direction: self.direction,
        }
    }
}

impl Application for Fixture {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => {
                self.draft = self.saved.clone();
                self.open = true;
            }
            Message::Close => {
                self.close_requests += 1;
                if self.draft == self.saved {
                    self.open = false;
                } else {
                    self.asking = true;
                }
            }
            Message::Save => {
                self.saves += 1;
                self.saved = self.draft.clone();
                self.open = false;
            }
            Message::Input(draft) => self.draft = draft,
            Message::Background => self.background_clicks += 1,
            Message::RemoveOpener => self.opener = false,
            Message::KeepEditing => self.asking = false,
            Message::Discard => {
                self.draft = self.saved.clone();
                self.asking = false;
                self.open = false;
            }
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        self.viewport = Rect::new(0., 0., width, height);
        true
    }

    fn view(&self) -> View<Message> {
        let opener = self
            .opener
            .then(|| command(OPENER, "開啟表單", Message::Open).restore_focus_on_removal(FALLBACK));
        let open = self
            .open
            .then(|| {
                let input = View::new(
                    Node::new(Role::TextInput, "行程名稱")
                        .id(FIELD)
                        .text(&self.draft)
                        .styled(
                            Style::column()
                                .width(Length::Percent(1.))
                                .height(Length::Px(48.)),
                        ),
                )
                .on_input(Message::Input)
                .accessibility_scroll_into_view();
                let mut rows = vec![input];
                for _ in 0..14 {
                    rows.push(View::new(
                        Node::new(Role::Text, "詳細資料")
                            .text("詳細資料")
                            .styled(Style::column().height(Length::Px(48.))),
                    ));
                }
                rows.push(command("last", "最後欄位", Message::Background));
                self.theme().full_screen_dialog(
                    self.options(),
                    View::new(
                        Node::new(Role::Group, "")
                            .id("contents")
                            .styled(Style::column().width(Length::Percent(1.))),
                    )
                    .children(rows),
                    Message::Close,
                    Message::Save,
                )
            })
            .flatten();
        let confirm = self.asking.then(|| {
            View::new(
                Node::new(Role::Dialog, "捨棄變更")
                    .id("discard-dialog")
                    .styled(
                        Style::column()
                            .width(Length::Px(160.))
                            .height(Length::Px(96.))
                            .fixed(Insets {
                                left: Length::Px(80.),
                                top: Length::Px(80.),
                                ..Insets::default()
                            })
                            .background(self.theme().colors.surface_container_high),
                    ),
            )
            .children([
                command("keep", "繼續編輯", Message::KeepEditing),
                command("discard", "捨棄", Message::Discard),
            ])
            .modal_overlay(Message::KeepEditing)
        });
        View::new(Node::new(Role::Group, "").styled(Style::column())).children(
            opener
                .into_iter()
                .chain([command(FALLBACK, "背景", Message::Background)])
                .chain(open)
                .chain(confirm),
        )
    }
}

fn command(id: &str, name: &str, message: Message) -> View<Message> {
    View::new(
        Node::new(Role::Button, name).id(id).styled(
            Style::column()
                .width(Length::Px(144.))
                .height(Length::Px(48.)),
        ),
    )
    .on_press(message)
    .accessible_activation()
}

struct Fonts;

impl TextMeasurer for Fonts {
    fn measure(&mut self, text: &str, font_size: f32, width: Option<f32>) -> (f32, f32) {
        let natural = text
            .chars()
            .map(|character| {
                if character.is_ascii() {
                    font_size * 0.58
                } else {
                    font_size
                }
            })
            .sum::<f32>();
        let width = width.unwrap_or(natural).min(natural);
        let lines = (natural / width.max(1.)).ceil().max(1.);
        (width, lines * font_size * 1.5)
    }
}

fn open(fixture: Fixture, viewport: (f32, f32)) -> Runtime<Fixture> {
    let mut runtime = Runtime::new(fixture).unwrap();
    runtime.render(viewport.0, viewport.1, &mut Fonts).unwrap();
    runtime.activate(OPENER).unwrap();
    draw(&mut runtime);
    assert_eq!(runtime.focused(), Some(FIELD));
    runtime
}

fn draw(runtime: &mut Runtime<Fixture>) -> surface::DisplayList {
    let viewport = runtime.app().viewport;
    runtime
        .render(viewport.width, viewport.height, &mut Fonts)
        .unwrap()
}

fn bounds(runtime: &Runtime<Fixture>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(key).unwrap().index()].bounds
}

fn click(runtime: &mut Runtime<Fixture>, x: f32, y: f32) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
}

fn contained(outer: Rect, inner: Rect) {
    assert_eq!(outer.intersect(inner), inner);
}

#[test]
fn viewport_coverage_and_measured_header_match_the_dialog_tokens() {
    for (width, height, direction, scale) in [
        (320., 400., LayoutDirection::LeftToRight, 1.),
        (320., 240., LayoutDirection::RightToLeft, 2.),
        (561., 720., LayoutDirection::RightToLeft, 1.),
        (960., 640., LayoutDirection::LeftToRight, 1.),
    ] {
        let mut runtime = open(Fixture::new(direction, scale), (width, height));
        let paint = draw(&mut runtime);
        let safe = Rect::new(0., 0., width, height);
        let panel = bounds(&runtime, DIALOG);
        assert_eq!(panel, safe);
        let theme = runtime.app().theme();
        assert!(paint.ops.iter().any(|op| matches!(
            op,
            PaintOp::Quad { rect, radius, color, .. }
                if *rect == safe && *radius == 0. && *color == theme.colors.surface
        )));
        let header = bounds(&runtime, HEADER);
        let body = bounds(&runtime, BODY);
        let close = bounds(&runtime, CLOSE);
        let icon = bounds(&runtime, ICON);
        let action = bounds(&runtime, ACTION);
        let caption = bounds(&runtime, CAPTION);
        let title = bounds(&runtime, TITLE);
        assert_eq!(header.width, width.min(560.));
        assert!((header.x - (width - header.width) / 2.).abs() <= 0.5);
        if scale == 1. {
            assert_eq!(header.height, 56.);
        } else {
            assert_eq!(header.height, 120.);
            assert_eq!(title.height, 112.);
        }
        assert_eq!(body.y, header.y + header.height);
        assert_eq!(body.height, height - header.height);
        assert_eq!(bounds(&runtime, "contents").width, body.width - 48.);
        assert_eq!(bounds(&runtime, "contents").x, body.x + 24.);
        assert_eq!((icon.width, icon.height), (24., 24.));
        assert_eq!((close.width, close.height), (48., 48.));
        assert!(action.width >= 48. && action.height >= 48.);
        contained(close, icon);
        contained(action, caption);
        contained(header, action);
        contained(header, title);
        if direction == LayoutDirection::RightToLeft {
            assert_eq!(header.x + header.width - icon.x - icon.width, 24.);
            assert!(close.x > title.x && title.x > action.x);
        } else {
            assert_eq!(icon.x - header.x, 24.);
            assert!(close.x < title.x && title.x < action.x);
        }
        let frame = runtime.accessibility_frame().unwrap();
        assert_eq!(frame.modal_root(), runtime.dom().by_id(DIALOG));
        let node = |id| frame.node(runtime.dom().by_id(id).unwrap());
        assert_eq!(node(TITLE).unwrap().node.role(), Role::Heading);
        assert_eq!(node(ACTION).unwrap().node.name(), "儲存");
        assert_eq!(node(BODY).unwrap().node.name(), "行程內容");
        assert!(node(ICON).is_none() && node(CAPTION).is_none() && node(FALLBACK).is_none());
    }
}

#[test]
fn body_scrolls_while_header_and_scope_stay_fixed() {
    let mut runtime = open(Fixture::new(LayoutDirection::RightToLeft, 2.), (320., 240.));
    let stable = [HEADER, CLOSE, TITLE, ACTION].map(|key| bounds(&runtime, key));
    let body = bounds(&runtime, BODY);
    let bottom = bounds(&runtime, "last").y;
    runtime
        .dispatch(Event::Scroll {
            x: body.x + body.width / 2.,
            y: body.y + body.height / 2.,
            delta_x: 0.,
            delta_y: 300.,
        })
        .unwrap();
    draw(&mut runtime);
    assert!(bounds(&runtime, "last").y < bottom);
    assert_eq!(
        stable,
        [HEADER, CLOSE, TITLE, ACTION].map(|key| bounds(&runtime, key))
    );
    assert!(runtime.activate(FALLBACK).is_err());
    click(&mut runtime, 319., 239.);
    draw(&mut runtime);
    assert!(runtime.app().open);
    assert_eq!(runtime.app().background_clicks, 0);
    assert_eq!(runtime.app().close_requests, 0);

    let icon = bounds(&runtime, ICON);
    click(&mut runtime, icon.x + 12., icon.y + 12.);
    assert!(!runtime.app().open);
    assert_eq!(runtime.app().close_requests, 1);
    assert_eq!(runtime.focused(), Some(OPENER));
}

#[test]
fn pending_composition_owns_enter_escape_and_header_presses() {
    let mut runtime = open(Fixture::new(LayoutDirection::LeftToRight, 1.), (320., 400.));
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄅ".into(),
            selection: None,
        }))
        .unwrap();
    draw(&mut runtime);
    for key in [ICON, CAPTION] {
        let target = bounds(&runtime, key);
        click(
            &mut runtime,
            target.x + target.width / 2.,
            target.y + target.height / 2.,
        );
        assert_eq!(runtime.focused(), Some(FIELD));
        assert!(runtime.input_snapshot().unwrap().composing);
    }
    assert!(!runtime.activate(ACTION).unwrap());
    assert!(!runtime.activate(CLOSE).unwrap());
    assert!(!runtime
        .key(&KeyPress::new(Key::Enter, Modifiers::default()))
        .unwrap());
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert!(runtime.app().open);
    assert!(!runtime.input_snapshot().unwrap().composing);
    assert_eq!(runtime.app().saves, 0);
    assert_eq!(runtime.app().close_requests, 0);
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert_eq!(runtime.app().close_requests, 1);
    assert_eq!(runtime.focused(), Some(OPENER));

    runtime.activate(OPENER).unwrap();
    runtime.apply(Message::RemoveOpener).unwrap();
    draw(&mut runtime);
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    assert_eq!(runtime.focused(), Some(FALLBACK));
}

#[test]
fn caller_controls_save_discard_and_confirmation_focus() {
    let mut runtime = open(Fixture::new(LayoutDirection::RightToLeft, 2.), (320., 400.));
    runtime
        .dispatch(Event::InputValue {
            value: "旅行".into(),
            anchor: 6,
            cursor: 6,
        })
        .unwrap();
    draw(&mut runtime);
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    draw(&mut runtime);
    assert!(runtime.app().asking);
    assert_eq!(runtime.focused(), Some("keep"));
    assert!(runtime.activate(ACTION).is_err());
    assert!(runtime.dispatch(Event::Cancel).unwrap());
    draw(&mut runtime);
    assert_eq!(runtime.focused(), Some(FIELD));
    assert_eq!(runtime.input_snapshot().unwrap().value, "旅行");
    assert_eq!(runtime.app().saved, "");

    runtime
        .dispatch(Event::FocusNext { reverse: true })
        .unwrap();
    assert_eq!(runtime.focused(), Some(ACTION));
    assert!(runtime.dispatch(Event::Activate).unwrap());
    assert_eq!(runtime.app().saved, "旅行");
    assert_eq!(runtime.app().saves, 1);
    assert_eq!(runtime.focused(), Some(OPENER));

    runtime.activate(OPENER).unwrap();
    runtime.apply(Message::Input("待捨棄".into())).unwrap();
    draw(&mut runtime);
    assert!(runtime.activate(CLOSE).unwrap());
    draw(&mut runtime);
    assert!(runtime.activate("discard").unwrap());
    assert_eq!(runtime.app().draft, "旅行");
    assert_eq!(runtime.app().saved, "旅行");
    assert_eq!(runtime.app().saves, 1);
    assert_eq!(runtime.focused(), Some(OPENER));

    runtime.activate(OPENER).unwrap();
    draw(&mut runtime);
    let caption = bounds(&runtime, CAPTION);
    click(
        &mut runtime,
        caption.x + caption.width / 2.,
        caption.y + caption.height / 2.,
    );
    assert_eq!(runtime.app().saves, 2);
    assert_eq!(runtime.focused(), Some(OPENER));
}

#[test]
fn long_localized_labels_and_disabled_action_stay_bounded() {
    let mut fixture = Fixture::new(LayoutDirection::RightToLeft, 2.);
    fixture.title = "新增一筆長名稱的私人行程變更";
    fixture.action_label = "儲存這份行程並關閉";
    fixture.action_disabled = true;
    let mut runtime = open(fixture, (320., 240.));
    let header = bounds(&runtime, HEADER);
    let action = bounds(&runtime, ACTION);
    let glyph = bounds(&runtime, CAPTION);
    contained(action, glyph);
    contained(header, action);
    assert!(bounds(&runtime, BODY).height >= 48.);
    let node = runtime
        .dom()
        .get(runtime.dom().by_id(ACTION).unwrap())
        .unwrap();
    assert!(node.is_disabled());
    assert_eq!(node.name(), "儲存這份行程並關閉");
    assert!(runtime.activate(ACTION).is_err());
    click(
        &mut runtime,
        glyph.x + glyph.width / 2.,
        glyph.y + glyph.height / 2.,
    );
    assert_eq!(runtime.app().saves, 0);
    assert!(runtime.app().open);
}

#[test]
fn invalid_or_unfit_viewports_do_not_mount_partial_dialogs() {
    let mut fixture = Fixture::new(LayoutDirection::LeftToRight, 1.);
    for viewport in [
        Rect::new(f32::NAN, 0., 320., 400.),
        Rect::new(0., 0., 100., 400.),
        Rect::new(0., 0., 320., 100.),
    ] {
        fixture.viewport = viewport;
        assert!(full_screen_dialog_layout(fixture.options()).is_none());
    }
    fixture.viewport = Rect::new(0., 0., 320., 240.);
    let layout = full_screen_dialog_layout(fixture.options()).unwrap();
    assert_eq!(layout.bounds, fixture.viewport);
    assert_eq!(layout.content_width, 272.);
    assert_ne!(fixture.theme().colors.surface, Color::TRANSPARENT);
}
