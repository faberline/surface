use surface::{Color, Insets, Length, MicroDom, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{AccessibilityScrollDirection, Application, Event, Runtime, View};
use surface_material::{
    shape, BottomSheetContent, BottomSheetHeights, BottomSheetOptions, BottomSheetPresentation,
    BottomSheetState, ButtonKind, Elevation, Theme,
};

#[derive(Clone)]
enum Msg {
    Open,
    Close,
    Click,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Standard,
    Modal,
}

struct Sheet {
    dark: bool,
    mode: Mode,
    open: bool,
    clicks: usize,
}

impl Sheet {
    fn new(dark: bool, mode: Mode, open: bool) -> Self {
        Self {
            dark,
            mode,
            open,
            clicks: 0,
        }
    }
}

impl Application for Sheet {
    type Message = Msg;
    fn update(&mut self, m: Msg) {
        match m {
            Msg::Open => self.open = true,
            Msg::Close => self.open = false,
            Msg::Click => self.clicks += 1,
        }
    }
    fn view(&self) -> View<Msg> {
        let t = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let mut children = vec![
            t.button_with("outside", "外部", ButtonKind::Text, false, Msg::Click),
            t.button_with("open", "開啟", ButtonKind::Filled, false, Msg::Open),
        ];
        match self.mode {
            Mode::Standard => {
                children.push(t.bottom_sheet("sheet", "Sheet", [t.text("body", "Body")]))
            }
            Mode::Modal => {
                if self.open {
                    children.push(t.modal_bottom_sheet(
                        "sheet",
                        "Sheet",
                        [t.button_with("close", "關閉", ButtonKind::Text, false, Msg::Close)],
                        Msg::Close,
                    ));
                }
            }
        }
        t.page(children)
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

fn render(dark: bool, mode: Mode, open: bool, w: f32, h: f32) -> Runtime<Sheet> {
    let mut r = Runtime::new(Sheet::new(dark, mode, open)).unwrap();
    r.render(w, h, &mut Metrics).unwrap();
    r
}

fn sheet_bounds(r: &Runtime<Sheet>, id: &str) -> surface::Rect {
    let dom: &MicroDom = r.dom();
    let node = dom.by_id(id).expect("sheet node");
    r.computed().unwrap().nodes[node.index()].bounds
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn standard_sheet_has_handle_and_top_corners() {
    let r = render(false, Mode::Standard, false, 400., 600.);
    let dom = r.dom();
    let sheet = dom.by_id("sheet").expect("sheet");
    let handle = dom.children(sheet).next().expect("handle child");
    let handle_bounds = r.computed().unwrap().nodes[handle.index()].bounds;
    assert!(approx(handle_bounds.width, 32.), "handle width");
    assert!(approx(handle_bounds.height, 4.), "handle height");
    let handle_style = dom.get(handle).unwrap().style().clone();
    assert_eq!(
        handle_style.background,
        Theme::light().colors.on_surface_variant
    );
    let style = dom.get(sheet).unwrap().style().clone();
    assert_eq!(style.corners, Some(shape::EXTRA_LARGE_TOP));
    assert_eq!(
        style.background,
        Theme::light().colors.surface_container_low
    );
}

#[test]
fn sheet_is_capped_at_640_wide() {
    let wide = render(false, Mode::Standard, false, 900., 600.);
    assert!(approx(sheet_bounds(&wide, "sheet").width, 640.), "capped");
    let narrow = render(false, Mode::Standard, false, 400., 600.);
    assert!(sheet_bounds(&narrow, "sheet").width <= 400., "narrow");
}

#[test]
fn colors_use_tokens_in_both_themes() {
    for dark in [false, true] {
        let r = render(dark, Mode::Standard, false, 400., 600.);
        let theme = if dark { Theme::dark() } else { Theme::light() };
        let style = r
            .dom()
            .get(r.dom().by_id("sheet").unwrap())
            .unwrap()
            .style()
            .clone();
        assert_eq!(
            style.background, theme.colors.surface_container_low,
            "dark={dark}"
        );
    }
}

#[test]
fn modal_sheet_sits_at_the_bottom() {
    let r = render(false, Mode::Modal, true, 400., 600.);
    let b = sheet_bounds(&r, "sheet");
    assert!(
        approx(b.y + b.height, 600.),
        "bottom edge at {}",
        b.y + b.height
    );
}

#[test]
fn modal_sheet_traps_focus_and_dismisses_on_escape() {
    let mut r = render(false, Mode::Modal, false, 400., 600.);
    r.activate("open").unwrap();
    assert_eq!(r.focused(), Some("close"));
    r.dispatch(Event::Cancel).unwrap();
    assert!(!r.app().open);
    assert_eq!(r.focused(), Some("open"));
}

#[test]
fn outside_click_dismisses_the_modal_sheet() {
    let mut r = render(false, Mode::Modal, false, 400., 600.);
    r.activate("open").unwrap();
    r.render(400., 600., &mut Metrics).unwrap();
    // The "outside" button sits above the sheet, under the scrim.
    let b = r.computed().unwrap().nodes[r.dom().by_id("outside").unwrap().index()].bounds;
    let (x, y) = (b.x + b.width / 2., b.y + b.height / 2.);
    r.dispatch(Event::PointerDown { x, y }).unwrap();
    r.dispatch(Event::PointerUp { x, y }).unwrap();
    assert!(!r.app().open, "outside click closes the sheet");
    assert_eq!(
        r.app().clicks,
        0,
        "control under the scrim is not activated"
    );
}

#[test]
fn standard_sheet_does_not_block_the_page() {
    let mut r = render(false, Mode::Standard, false, 400., 600.);
    assert!(r.activate("outside").unwrap());
    assert_eq!(r.app().clicks, 1);
}

const CONTROLLED_TITLE: &str = "顯示工作區的完整設定與進階功能";

#[derive(Clone, Copy)]
enum ControlledMessage {
    Open,
    Toggle,
    Close,
    Main,
    Covered,
}

struct ControlledSheet {
    theme: Theme,
    presentation: BottomSheetPresentation,
    heights: BottomSheetHeights,
    state: BottomSheetState,
    text_scale: f32,
    main_presses: u32,
    covered_presses: u32,
}

impl ControlledSheet {
    fn new(presentation: BottomSheetPresentation) -> Self {
        Self {
            theme: Theme::light(),
            presentation,
            heights: BottomSheetHeights::new(192., 465.),
            state: BottomSheetState::Closed,
            text_scale: 1.,
            main_presses: 0,
            covered_presses: 0,
        }
    }
}

impl Application for ControlledSheet {
    type Message = ControlledMessage;

    fn text_scale(&self) -> f32 {
        self.text_scale
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            ControlledMessage::Open => self.state = BottomSheetState::Peek,
            ControlledMessage::Toggle if self.state == BottomSheetState::Peek => {
                self.state = BottomSheetState::Expanded;
            }
            ControlledMessage::Toggle => self.state = BottomSheetState::Peek,
            ControlledMessage::Close => self.state = BottomSheetState::Closed,
            ControlledMessage::Main => self.main_presses += 1,
            ControlledMessage::Covered => self.covered_presses += 1,
        }
    }

    fn view(&self) -> View<Self::Message> {
        let action = |id, label, style, message| {
            View::new(Node::new(Role::Button, label).id(id).styled(style))
                .on_press(message)
                .accessible_activation()
        };
        let button_style = || {
            Style::column()
                .width(Length::Px(80.))
                .height(Length::Px(48.))
        };
        let body = View::new(
            Node::new(Role::Group, "Main").styled(Style::column().height(Length::Percent(1.))),
        )
        .children([
            action(
                "open-sheet",
                "Open",
                button_style(),
                ControlledMessage::Open,
            ),
            action(
                "main-action",
                "Main",
                button_style(),
                ControlledMessage::Main,
            ),
            action(
                "covered-action",
                "Covered",
                button_style().absolute(Insets {
                    right: Length::Px(80.),
                    bottom: Length::Px(56.),
                    ..Insets::default()
                }),
                ControlledMessage::Covered,
            ),
        ]);
        let panel = View::new(Node::new(Role::Group, "")).children([
            action(
                "first-action",
                "First",
                button_style(),
                ControlledMessage::Close,
            )
            .restore_focus_on_removal("last-action"),
            View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(500.)))),
            action(
                "last-action",
                "Last",
                button_style(),
                ControlledMessage::Close,
            ),
        ]);
        self.theme.bottom_sheet_layout(
            BottomSheetOptions {
                id: "controlled",
                title: CONTROLLED_TITLE,
                handle_label: if self.state == BottomSheetState::Expanded {
                    "Collapse details"
                } else {
                    "Expand details"
                },
                close_label: "Close details",
                restore_focus_to: "open-sheet",
                presentation: self.presentation,
                heights: self.heights,
            },
            self.state,
            BottomSheetContent {
                body,
                panel,
                on_toggle: ControlledMessage::Toggle,
                on_dismiss: ControlledMessage::Close,
            },
        )
    }
}

struct WrappingMetrics;

impl TextMeasurer for WrappingMetrics {
    fn measure(&mut self, text: &str, size: f32, limit: Option<f32>) -> (f32, f32) {
        let width = text.chars().count() as f32 * size;
        let available = limit.unwrap_or(width).max(1.);
        (
            width.min(available),
            size * 1.3 * (width / available).ceil().max(1.),
        )
    }
}

fn controlled_bounds(runtime: &Runtime<ControlledSheet>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn controlled_click(runtime: &mut Runtime<ControlledSheet>, x: f32, y: f32) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
}

#[test]
fn bottom_sheet_heights_are_finite_and_keep_room_for_controls() {
    let invalid = BottomSheetHeights::new(f32::NAN, f32::INFINITY);
    assert_eq!(invalid.height(BottomSheetState::Closed), 0.);
    assert_eq!(invalid.peek(), 48.);
    assert_eq!(invalid.expanded(), 48.);
    let reversed = BottomSheetHeights::new(200., -30.);
    assert_eq!(reversed.peek(), 200.);
    assert_eq!(reversed.expanded(), 200.);
}

#[test]
fn expanded_standard_sheet_with_equal_anchors_disables_only_the_handle() {
    let mut app = ControlledSheet::new(BottomSheetPresentation::Standard);
    app.heights = BottomSheetHeights::new(192., 192.);
    app.state = BottomSheetState::Expanded;
    let mut runtime = Runtime::new(app).unwrap();
    runtime.render(400., 600., &mut WrappingMetrics).unwrap();
    let handle_id = runtime.dom().by_id("controlled-handle").unwrap();
    assert!(runtime.dom().get(handle_id).unwrap().is_disabled());
    assert_eq!(controlled_bounds(&runtime, "controlled-handle").width, 48.);
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(handle_id)
        .is_none());
    runtime.activate("controlled-close").unwrap();
    assert_eq!(runtime.app().state, BottomSheetState::Closed);
    assert_eq!(runtime.focused(), Some("open-sheet"));
}

#[test]
fn controlled_sheets_are_bottom_anchored_and_modal_controls_have_real_targets() {
    for theme in [Theme::light(), Theme::dark()] {
        for presentation in [
            BottomSheetPresentation::Standard,
            BottomSheetPresentation::Modal,
        ] {
            let mut app = ControlledSheet::new(presentation);
            app.theme = theme;
            let modal = presentation == BottomSheetPresentation::Modal;
            let mut runtime = Runtime::new(app).unwrap();
            runtime.render(1000., 640., &mut WrappingMetrics).unwrap();
            let main = controlled_bounds(&runtime, "controlled-body");
            assert!(runtime.dom().by_id("controlled").is_none());
            runtime.activate("open-sheet").unwrap();
            runtime.render(1000., 640., &mut WrappingMetrics).unwrap();
            let panel = runtime
                .dom()
                .get(runtime.dom().by_id("controlled").unwrap())
                .unwrap();
            assert_eq!(panel.name(), CONTROLLED_TITLE);
            assert_eq!(panel.role(), if modal { Role::Dialog } else { Role::Group });
            assert_eq!(panel.style().corner_radii(), shape::EXTRA_LARGE_TOP);
            assert_eq!(panel.style().background, theme.colors.surface_container_low);
            assert_eq!(
                panel.box_shadow_layers(),
                theme.elevation(Elevation::Level1)
            );
            assert_eq!(
                controlled_bounds(&runtime, "controlled"),
                Rect::new(180., 448., 640., 192.)
            );
            assert_eq!(
                controlled_bounds(&runtime, "controlled-handle"),
                Rect::new(476., 448., 48., 48.)
            );
            assert_eq!(
                controlled_bounds(&runtime, "controlled-close"),
                Rect::new(772., 448., 48., 48.)
            );
            assert_eq!(
                controlled_bounds(&runtime, "controlled-content"),
                Rect::new(180., 496., 640., 144.)
            );
            assert_eq!(controlled_bounds(&runtime, "controlled-body"), main);
            let layer = runtime
                .dom()
                .get(runtime.dom().by_id("controlled-layer").unwrap())
                .unwrap();
            let Color(red, green, blue, _) = theme.colors.scrim;
            assert_eq!(
                layer.style().background,
                if modal {
                    Color(red, green, blue, 82)
                } else {
                    Color::TRANSPARENT
                }
            );
            assert_eq!(
                runtime.focused(),
                Some(if modal {
                    "controlled-handle"
                } else {
                    "open-sheet"
                })
            );
            let handle_id = runtime.dom().by_id("controlled-handle").unwrap();
            let handle = runtime.dom().get(handle_id).unwrap();
            assert_eq!(handle.name(), "Expand details");
            assert_eq!(handle.expanded_state(), Some(false));
            let frame = runtime.accessibility_frame().unwrap();
            assert!(frame.activation_target(handle_id).is_some());
            assert_eq!(
                frame
                    .activation_target(runtime.dom().by_id("main-action").unwrap())
                    .is_some(),
                !modal
            );

            if !modal {
                runtime
                    .dispatch(Event::FocusNext { reverse: false })
                    .unwrap();
                runtime
                    .dispatch(Event::FocusNext { reverse: false })
                    .unwrap();
                runtime
                    .dispatch(Event::FocusNext { reverse: false })
                    .unwrap();
                assert_eq!(runtime.focused(), Some("controlled-handle"));
            }
            runtime.dispatch(Event::Activate).unwrap();
            runtime.render(1000., 640., &mut WrappingMetrics).unwrap();
            assert_eq!(runtime.app().state, BottomSheetState::Expanded);
            assert_eq!(runtime.focused(), Some("controlled-handle"));
            let handle = runtime
                .dom()
                .get(runtime.dom().by_id("controlled-handle").unwrap())
                .unwrap();
            assert_eq!(handle.name(), "Collapse details");
            assert_eq!(handle.expanded_state(), Some(true));
            assert_eq!(
                controlled_bounds(&runtime, "controlled"),
                Rect::new(180., 175., 640., 465.)
            );
            assert_eq!(controlled_bounds(&runtime, "controlled-body"), main);
            runtime.dispatch(Event::Activate).unwrap();
            assert_eq!(runtime.app().state, BottomSheetState::Peek);
        }
    }
}

#[test]
fn pointer_shields_and_escape_use_the_presentation_scope() {
    let mut standard =
        Runtime::new(ControlledSheet::new(BottomSheetPresentation::Standard)).unwrap();
    standard.activate("open-sheet").unwrap();
    standard.render(400., 600., &mut WrappingMetrics).unwrap();
    controlled_click(&mut standard, 40., 70.);
    assert_eq!(standard.app().main_presses, 1);
    standard.dispatch(Event::Cancel).unwrap();
    assert_eq!(standard.app().state, BottomSheetState::Peek);
    // The bottom sheet covers this action, even in a blank part of the sheet.
    controlled_click(&mut standard, 280., 520.);
    assert_eq!(standard.app().covered_presses, 0);
    standard.activate("first-action").unwrap();
    assert_eq!(standard.app().state, BottomSheetState::Closed);
    assert_eq!(standard.focused(), Some("open-sheet"));
    standard.activate("open-sheet").unwrap();
    standard.activate("controlled-handle").unwrap();
    standard.dispatch(Event::Cancel).unwrap();
    assert_eq!(standard.app().state, BottomSheetState::Closed);
    assert_eq!(standard.focused(), Some("open-sheet"));

    let mut modal = Runtime::new(ControlledSheet::new(BottomSheetPresentation::Modal)).unwrap();
    modal.activate("open-sheet").unwrap();
    modal.render(400., 600., &mut WrappingMetrics).unwrap();
    controlled_click(&mut modal, 40., 70.);
    assert_eq!(modal.app().main_presses, 0);
    assert_eq!(modal.app().state, BottomSheetState::Closed);
    assert_eq!(modal.focused(), Some("open-sheet"));
    modal.activate("open-sheet").unwrap();
    modal.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(modal.focused(), Some("controlled-close"));
    modal.dispatch(Event::Activate).unwrap();
    assert_eq!(modal.app().state, BottomSheetState::Closed);
    assert_eq!(modal.focused(), Some("open-sheet"));
}

#[test]
fn clipped_height_leaves_controls_reachable_and_scrolls_the_wrapped_title() {
    for presentation in [
        BottomSheetPresentation::Standard,
        BottomSheetPresentation::Modal,
    ] {
        let mut app = ControlledSheet::new(presentation);
        app.text_scale = 2.;
        app.state = BottomSheetState::Expanded;
        let mut runtime = Runtime::new(app).unwrap();
        runtime.render(120., 128., &mut WrappingMetrics).unwrap();
        assert_eq!(
            controlled_bounds(&runtime, "controlled"),
            Rect::new(0., 0., 120., 128.)
        );
        let handle = controlled_bounds(&runtime, "controlled-handle");
        let close = controlled_bounds(&runtime, "controlled-close");
        assert_eq!((handle.width, handle.height), (48., 48.));
        assert!(handle.x >= 0. && handle.x + handle.width <= close.x);
        assert_eq!(close.x + close.width, 120.);
        let content = controlled_bounds(&runtime, "controlled-content");
        assert_eq!(content, Rect::new(0., 48., 120., 80.));
        let title = controlled_bounds(&runtime, "controlled-title");
        assert!(title.height > content.height);
        assert!(runtime
            .accessibility_frame()
            .unwrap()
            .scroll_page_target(
                runtime.dom().by_id("controlled-content").unwrap(),
                AccessibilityScrollDirection::Down,
            )
            .is_some());
        runtime
            .dispatch(Event::Scroll {
                x: 30.,
                y: 96.,
                delta_x: 0.,
                delta_y: 160.,
            })
            .unwrap();
        runtime.render(120., 128., &mut WrappingMetrics).unwrap();
        assert!(
            runtime
                .scroll_metrics("controlled-content")
                .unwrap()
                .offset
                .1
                > 0.
        );
        assert!(controlled_bounds(&runtime, "controlled-title").y < title.y);
        assert_eq!(controlled_bounds(&runtime, "controlled-handle"), handle);
        assert_eq!(controlled_bounds(&runtime, "controlled-close"), close);
    }
}
