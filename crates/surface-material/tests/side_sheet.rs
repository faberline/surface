use surface::{Color, Insets, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{AccessibilityScrollDirection, Application, Event, Runtime, View};
use surface_material::{
    shape::{self, LayoutDirection},
    Elevation, SideSheetContent, SideSheetEdge, SideSheetOptions, SideSheetPresentation,
    SideSheetStyle, Theme,
};

const TITLE: &str = "篩選與顯示選項";

#[derive(Clone, Copy)]
enum Message {
    Open,
    Dismiss,
    Main,
    Covered,
    HideTemporary,
    OpenNested,
    DismissNested,
}

struct Sheet {
    theme: Theme,
    presentation: SideSheetPresentation,
    style: SideSheetStyle,
    edge: SideSheetEdge,
    direction: LayoutDirection,
    scale: f32,
    open: bool,
    temporary: bool,
    nested: bool,
    main_clicks: usize,
    covered_clicks: usize,
}

impl Sheet {
    fn new(presentation: SideSheetPresentation) -> Self {
        Self {
            theme: Theme::light(),
            presentation,
            style: SideSheetStyle::Docked,
            edge: SideSheetEdge::End,
            direction: LayoutDirection::LeftToRight,
            scale: 1.,
            open: false,
            temporary: true,
            nested: false,
            main_clicks: 0,
            covered_clicks: 0,
        }
    }
}

impl Application for Sheet {
    type Message = Message;

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::Open => {
                self.open = true;
                self.temporary = true;
            }
            Message::Dismiss => self.open = false,
            Message::Main => self.main_clicks += 1,
            Message::Covered => self.covered_clicks += 1,
            Message::HideTemporary => self.temporary = false,
            Message::OpenNested => self.nested = true,
            Message::DismissNested => self.nested = false,
        }
    }

    fn view(&self) -> View<Self::Message> {
        let button = |id, label, style| {
            View::new(Node::new(Role::Button, label).id(id).styled(style)).accessible_activation()
        };
        let button_style = || Style::column().height(Length::Px(48.));
        let body = View::new(
            Node::new(Role::Group, "Main").styled(Style::column().height(Length::Percent(1.))),
        )
        .children([
            button("opener", "Filter", button_style().width(Length::Px(100.)))
                .disclosure_expanded(self.open)
                .on_press(Message::Open),
            button(
                "main-action",
                "Refresh",
                button_style().width(Length::Px(100.)),
            )
            .on_press(Message::Main),
            button(
                "covered-action",
                "Covered action",
                button_style().width(Length::Px(80.)).absolute(Insets {
                    right: Length::Px(8.),
                    bottom: Length::Px(8.),
                    ..Insets::default()
                }),
            )
            .on_press(Message::Covered),
        ]);
        let nested = self.nested.then(|| {
            View::new(
                Node::new(Role::Dialog, "Confirm")
                    .id("nested")
                    .styled(Style::column().fixed(Insets::all(100.))),
            )
            .children([
                button("nested-close", "Cancel confirmation", button_style())
                    .on_press(Message::DismissNested),
            ])
            .modal_overlay(Message::DismissNested)
        });
        let panel = View::new(Node::new(Role::Group, "")).children(
            [
                View::new(Node::new(Role::Group, "")).children(
                    [
                        button("inner-action", "Apply filter", button_style())
                            .on_press(Message::Dismiss),
                        button("custom-fallback", "Apply and refresh", button_style())
                            .restore_focus_on_removal("main-action")
                            .on_press(Message::Dismiss),
                        button("nested-opener", "Confirm", button_style())
                            .on_press(Message::OpenNested),
                    ]
                    .into_iter()
                    .chain(self.temporary.then(|| {
                        button("temporary", "Remove filter", button_style())
                            .restore_focus_on_removal("nested-opener")
                            .on_press(Message::HideTemporary)
                    })),
                ),
                View::new(
                    Node::new(Role::Group, "").styled(Style::column().height(Length::Px(850.))),
                ),
                button("last-action", "Last action", button_style()).on_press(Message::Dismiss),
            ]
            .into_iter()
            .chain(nested),
        );
        View::new(Node::new(Role::Window, "Workspace")).children([self.theme.side_sheet(
            SideSheetOptions {
                id: "sheet",
                title: TITLE,
                close_label: "Close filters",
                restore_focus_to: "opener",
                presentation: self.presentation,
                style: self.style,
                edge: self.edge,
                direction: self.direction,
            },
            self.open,
            SideSheetContent {
                body,
                panel,
                on_dismiss: Message::Dismiss,
            },
        )])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, limit: Option<f32>) -> (f32, f32) {
        let width = text.chars().count() as f32 * size;
        let measured = limit.unwrap_or(width).max(1.);
        (
            width.min(measured),
            size * 1.3 * (width / measured).ceil().max(1.),
        )
    }
}

fn render(runtime: &mut Runtime<Sheet>, width: f32, height: f32) {
    runtime.render(width, height, &mut Metrics).unwrap();
}

fn bounds(runtime: &Runtime<Sheet>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn press(runtime: &mut Runtime<Sheet>, x: f32, y: f32) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
}

fn click(runtime: &mut Runtime<Sheet>, id: &str) {
    let rect = bounds(runtime, id);
    press(runtime, rect.x + rect.width / 2., rect.y + rect.height / 2.);
}

#[test]
fn all_presentations_and_styles_match_their_geometry_and_tokens() {
    for theme in [Theme::light(), Theme::dark()] {
        for presentation in [
            SideSheetPresentation::StandardOverlay,
            SideSheetPresentation::StandardCoplanar,
            SideSheetPresentation::Modal,
        ] {
            for style in [SideSheetStyle::Docked, SideSheetStyle::Detached] {
                for (edge, direction, left) in [
                    (SideSheetEdge::Start, LayoutDirection::LeftToRight, true),
                    (SideSheetEdge::End, LayoutDirection::LeftToRight, false),
                    (SideSheetEdge::Start, LayoutDirection::RightToLeft, false),
                    (SideSheetEdge::End, LayoutDirection::RightToLeft, true),
                ] {
                    let mut app = Sheet::new(presentation);
                    app.theme = theme;
                    app.edge = edge;
                    app.direction = direction;
                    app.style = style;
                    app.open = true;
                    let mut runtime = Runtime::new(app).unwrap();
                    render(&mut runtime, 800., 600.);
                    let margin = if style == SideSheetStyle::Detached {
                        16.
                    } else {
                        0.
                    };
                    assert_eq!(
                        bounds(&runtime, "sheet"),
                        Rect::new(
                            if left { margin } else { 800. - 256. - margin },
                            margin,
                            256.,
                            600. - 2. * margin,
                        )
                    );
                    let reserved = if presentation == SideSheetPresentation::StandardCoplanar {
                        256. + 2. * margin
                    } else {
                        0.
                    };
                    assert_eq!(
                        bounds(&runtime, "sheet-body"),
                        Rect::new(if left { reserved } else { 0. }, 0., 800. - reserved, 600.,)
                    );
                    let modal = presentation == SideSheetPresentation::Modal;
                    let sheet = runtime
                        .dom()
                        .get(runtime.dom().by_id("sheet").unwrap())
                        .unwrap();
                    assert_eq!(sheet.name(), TITLE);
                    assert_eq!(sheet.role(), if modal { Role::Dialog } else { Role::Group });
                    assert!(!sheet.is_tab_stop());
                    assert_eq!(
                        sheet.style().background,
                        if modal {
                            theme.colors.surface_container_low
                        } else {
                            theme.colors.surface
                        }
                    );
                    assert_eq!(
                        sheet.box_shadow_layers(),
                        theme.elevation(if modal {
                            Elevation::Level1
                        } else {
                            Elevation::Level0
                        })
                    );
                    assert_eq!(
                        sheet.style().corner_radii(),
                        match (style, modal, left) {
                            (SideSheetStyle::Detached, _, _) => shape::LARGE,
                            (_, true, true) => shape::LARGE_END,
                            (_, true, false) => shape::LARGE_START,
                            _ => shape::NONE,
                        }
                    );
                    let layer = runtime
                        .dom()
                        .get(runtime.dom().by_id("sheet-layer").unwrap())
                        .unwrap();
                    if modal {
                        assert_eq!(
                            bounds(&runtime, "sheet-layer"),
                            Rect::new(0., 0., 800., 600.)
                        );
                        assert_eq!(layer.style().background.3, 82);
                        assert_eq!(runtime.focused(), Some("sheet-close"));
                    } else {
                        assert_eq!(layer.style().background, Color::TRANSPARENT);
                    }
                    let close = bounds(&runtime, "sheet-close");
                    let title = bounds(&runtime, "sheet-title");
                    assert_eq!((close.width, close.height), (48., 48.));
                    if direction == LayoutDirection::LeftToRight {
                        assert!(close.x >= title.x + title.width);
                    } else {
                        assert!(close.x + close.width <= title.x);
                    }
                }
            }
        }
    }
}

#[test]
fn standard_overlay_keeps_exposed_controls_live_and_shields_covered_controls() {
    let mut runtime = Runtime::new(Sheet::new(SideSheetPresentation::StandardOverlay)).unwrap();
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "opener");
    assert_eq!(runtime.focused(), Some("opener"));
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "main-action");
    assert_eq!(runtime.app().main_clicks, 1);
    assert_eq!(runtime.focused(), Some("main-action"));
    runtime.dispatch(Event::Cancel).unwrap();
    assert!(runtime.app().open);
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "covered-action");
    assert_eq!(runtime.app().covered_clicks, 0);
    assert!(runtime.app().open);
}

#[test]
fn standard_escape_is_local_and_restores_nested_focus_without_overwriting_an_override() {
    let mut runtime = Runtime::new(Sheet::new(SideSheetPresentation::StandardCoplanar)).unwrap();
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "opener");
    render(&mut runtime, 800., 600.);
    runtime.dispatch(Event::Cancel).unwrap();
    assert!(runtime.app().open);
    for _ in 0..4 {
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
    }
    assert_eq!(runtime.focused(), Some("inner-action"));
    runtime.dispatch(Event::Cancel).unwrap();
    assert!(!runtime.app().open);
    assert_eq!(runtime.focused(), Some("opener"));
    assert!(runtime.dom().by_id("sheet").is_none());
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "opener");
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "custom-fallback");
    assert_eq!(runtime.focused(), Some("main-action"));
    assert!(!runtime.app().open);
    render(&mut runtime, 800., 600.);
    assert_eq!(bounds(&runtime, "sheet-body").width, 800.);
}

#[test]
fn focus_falls_back_to_the_opener_only_when_the_explicit_target_is_removed() {
    let mut runtime = Runtime::new(Sheet::new(SideSheetPresentation::StandardCoplanar)).unwrap();
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "opener");
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "temporary");
    assert!(runtime.app().open);
    assert!(runtime.dom().by_id("temporary").is_none());
    assert_eq!(runtime.focused(), Some("nested-opener"));

    runtime.dispatch(Event::Cancel).unwrap();
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "opener");
    render(&mut runtime, 800., 600.);
    for _ in 0..7 {
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
    }
    assert_eq!(runtime.focused(), Some("temporary"));
    runtime.dispatch(Event::Cancel).unwrap();
    assert!(!runtime.app().open);
    assert!(runtime.dom().by_id("nested-opener").is_none());
    assert_eq!(runtime.focused(), Some("opener"));
}

#[test]
fn the_modal_closes_on_scrim_or_escape_and_restores_the_opener() {
    let mut app = Sheet::new(SideSheetPresentation::Modal);
    app.style = SideSheetStyle::Detached;
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "opener");
    render(&mut runtime, 800., 600.);
    assert_eq!(runtime.focused(), Some("sheet-close"));
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame
        .activation_target(runtime.dom().by_id("main-action").unwrap())
        .is_none());
    assert!(frame
        .activation_target(runtime.dom().by_id("sheet-close").unwrap())
        .is_some());
    click(&mut runtime, "main-action");
    assert_eq!(runtime.app().main_clicks, 0);
    assert!(!runtime.app().open);
    assert_eq!(runtime.focused(), Some("opener"));
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "opener");
    render(&mut runtime, 800., 600.);
    press(&mut runtime, 799., 300.);
    assert!(
        !runtime.app().open,
        "the detached margin is outside the dialog"
    );
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "opener");
    runtime.dispatch(Event::Cancel).unwrap();
    assert!(!runtime.app().open);
    assert_eq!(runtime.focused(), Some("opener"));
}

#[test]
fn escape_dismisses_a_nested_modal_before_its_standard_sheet() {
    let mut runtime = Runtime::new(Sheet::new(SideSheetPresentation::StandardOverlay)).unwrap();
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "opener");
    render(&mut runtime, 800., 600.);
    click(&mut runtime, "nested-opener");
    render(&mut runtime, 800., 600.);
    assert_eq!(runtime.focused(), Some("nested-close"));
    runtime.dispatch(Event::Cancel).unwrap();
    assert!(!runtime.app().nested);
    assert!(runtime.app().open);
    assert_eq!(runtime.focused(), Some("nested-opener"));
    runtime.dispatch(Event::Cancel).unwrap();
    assert!(!runtime.app().open);
    assert_eq!(runtime.focused(), Some("opener"));
}

#[test]
fn narrow_detached_dialog_wraps_its_large_title_and_scrolls_below_the_header() {
    let mut app = Sheet::new(SideSheetPresentation::Modal);
    app.style = SideSheetStyle::Detached;
    app.direction = LayoutDirection::RightToLeft;
    app.scale = 2.;
    app.open = true;
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime, 236., 460.);
    assert_eq!(bounds(&runtime, "sheet"), Rect::new(16., 16., 204., 428.));
    let title = bounds(&runtime, "sheet-title");
    let close = bounds(&runtime, "sheet-close");
    assert!(title.height > 44.);
    assert_eq!((close.width, close.height), (48., 48.));
    let content = bounds(&runtime, "sheet-content");
    let content_id = runtime.dom().by_id("sheet-content").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame
        .scroll_page_target(content_id, AccessibilityScrollDirection::Down)
        .is_some());
    runtime
        .dispatch(Event::Scroll {
            x: content.x + 20.,
            y: content.y + 20.,
            delta_x: 0.,
            delta_y: 160.,
        })
        .unwrap();
    render(&mut runtime, 236., 460.);
    assert!(runtime.scroll_metrics("sheet-content").unwrap().offset.1 > 0.);
    assert_eq!(bounds(&runtime, "sheet-title"), title);
    assert_eq!(bounds(&runtime, "sheet-close"), close);
}
