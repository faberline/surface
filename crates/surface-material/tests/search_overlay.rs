use surface::{Color, Length, Node, PaintOp, Rect, Role, Style, TextMeasurer};
use surface_app::{
    Application, EditKey, Event, ImeEvent, Key, KeyPress, Modifiers, Roving, Runtime, ScrollTarget,
    View,
};
use surface_material::{
    shape::{self, LayoutDirection},
    SearchOverlayContent, SearchOverlayOptions, SearchStyle, Theme,
};

const INPUT: &str = "search-input";
const RESULTS: &str = "search-results";

#[derive(Clone)]
enum Message {
    Open,
    Close,
    Query(String),
    Clear,
    Submit,
    Result(usize),
    RemoveFirstResult,
    RemoveOpener,
    Background,
}

struct SearchApp {
    style: SearchStyle,
    direction: LayoutDirection,
    viewport: Rect,
    open: bool,
    query: String,
    submits: Vec<String>,
    picked: Vec<usize>,
    first_result: bool,
    opener: bool,
    clears: usize,
    closes: usize,
    background: usize,
}

impl Application for SearchApp {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.open = true,
            Message::Close => {
                self.open = false;
                self.closes += 1;
            }
            Message::Query(value) => self.query = value,
            Message::Clear => {
                self.query.clear();
                self.clears += 1;
            }
            Message::Submit => self.submits.push(self.query.clone()),
            Message::Result(id) => self.picked.push(id),
            Message::RemoveFirstResult => self.first_result = false,
            Message::RemoveOpener => self.opener = false,
            Message::Background => self.background += 1,
        }
    }

    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        self.viewport = Rect::new(0., 0., width, height);
        true
    }

    fn text_scale(&self) -> f32 {
        2.
    }

    fn view(&self) -> View<Message> {
        let theme = Theme::dark();
        let mut children = Vec::new();
        if self.opener {
            children.push(button("opener", "Open Search", false, Message::Open));
        }
        children.push(button(
            "background",
            "Background",
            false,
            Message::Background,
        ));
        if self.open {
            let start = usize::from(!self.first_result);
            let results = View::new(
                Node::new(Role::List, "Suggestions")
                    .id("rows")
                    .styled(Style::column().width(Length::Percent(1.))),
            )
            .children((start..24).map(|index| {
                button(
                    &format!("result-{index}"),
                    &format!("Suggestion {index}"),
                    index == 1,
                    Message::Result(index),
                )
                .protect_composition_on_press()
            }))
            .roving_retained_layout(Roving::vertical(false));
            children.extend(theme.search_overlay(
                SearchOverlayOptions {
                    id: "search",
                    open: self.open,
                    title: "Search",
                    input_label: "Query",
                    placeholder: Some("搜尋文件"),
                    results_label: "Results",
                    back_label: "Back",
                    clear_label: "Clear query",
                    query: &self.query,
                    style: self.style,
                    direction: self.direction,
                    viewport: self.viewport,
                    anchor: Some(Rect::new(48., 36., 464., 56.)),
                    preferred_docked_height: Some(300.),
                    restore_focus_to: Some("background"),
                    results_status: Some(if self.query.is_empty() {
                        "History"
                    } else {
                        "24 results"
                    }),
                },
                SearchOverlayContent {
                    results,
                    first_result: None,
                    on_submit: Message::Submit,
                    on_clear: Message::Clear,
                    on_close: Message::Close,
                },
                Message::Query,
            ));
        }
        theme.page(children)
    }
}

fn button(id: &str, label: &str, disabled: bool, action: Message) -> View<Message> {
    View::new(
        Node::new(Role::Button, label)
            .id(id)
            .disabled(disabled)
            .styled(
                Style::row()
                    .width(Length::Percent(1.))
                    .height(Length::Px(48.)),
            ),
    )
    .on_press(action)
    .accessible_activation()
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.55, size * 1.2)
    }
}

fn open(style: SearchStyle, direction: LayoutDirection, width: f32) -> Runtime<SearchApp> {
    let mut runtime = Runtime::new(SearchApp {
        style,
        direction,
        viewport: Rect::default(),
        open: false,
        query: "alpha".into(),
        submits: vec![],
        picked: vec![],
        first_result: true,
        opener: true,
        clears: 0,
        closes: 0,
        background: 0,
    })
    .unwrap();
    render(&mut runtime, width);
    assert!(runtime.activate("opener").unwrap());
    render(&mut runtime, width);
    assert_eq!(runtime.focused(), Some(INPUT));
    runtime
}

fn render(runtime: &mut Runtime<SearchApp>, width: f32) {
    runtime.render(width, 720., &mut Metrics).unwrap();
}

fn node<'a>(runtime: &'a Runtime<SearchApp>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn bounds(runtime: &Runtime<SearchApp>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn click(runtime: &mut Runtime<SearchApp>, x: f32, y: f32) {
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
}

fn tab(runtime: &mut Runtime<SearchApp>, reverse: bool) {
    runtime.dispatch(Event::FocusNext { reverse }).unwrap();
}

fn enter(runtime: &mut Runtime<SearchApp>) -> bool {
    runtime
        .key(&KeyPress::new(Key::Enter, Modifiers::default()))
        .unwrap()
}

#[test]
fn builds_both_variants_with_unclipped_buttons_and_named_results() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let rtl = direction == LayoutDirection::RightToLeft;
        for style in [SearchStyle::Contained, SearchStyle::Divided] {
            let mut runtime = open(style, direction, 600.);
            assert_eq!(bounds(&runtime, "search-dialog").height, 300.);
            assert_eq!(bounds(&runtime, "search-bar").height, 56.);
            assert_eq!(node(&runtime, "search-layer").style().background.3, 82);
            assert_eq!(node(&runtime, INPUT).name(), "Query");
            assert_eq!(node(&runtime, RESULTS).name(), "Results");
            assert_eq!(node(&runtime, "search-status").role(), Role::Status);
            assert_eq!(node(&runtime, "search-status").name(), "24 results");
            assert_eq!(
                runtime.dom().by_id("search-divider").is_some(),
                style == SearchStyle::Divided
            );
            let frame = runtime.accessibility_frame().unwrap();
            assert_eq!(frame.modal_root(), runtime.dom().by_id("search-dialog"));
            assert!(frame
                .node(runtime.dom().by_id("background").unwrap())
                .is_none());
            assert!(frame
                .node(runtime.dom().by_id("search-status").unwrap())
                .is_some());

            render(&mut runtime, 320.);
            assert_eq!(
                bounds(&runtime, "search-dialog"),
                Rect::new(0., 0., 320., 720.)
            );
            assert_eq!(
                node(&runtime, "search-layer").style().background,
                Color::TRANSPARENT
            );
            let (back, clear) = (
                bounds(&runtime, "search-back"),
                bounds(&runtime, "search-clear"),
            );
            for target in [back, clear] {
                assert_eq!((target.width, target.height), (48., 48.));
                assert_eq!(bounds(&runtime, "search-bar").intersect(target), target);
            }
            assert_eq!(back.x > clear.x, rtl);
            let bar = bounds(&runtime, "search-bar");
            match style {
                SearchStyle::Contained => {
                    assert_eq!(
                        node(&runtime, "search-bar").style().corners,
                        Some(shape::FULL)
                    );
                    assert_eq!(bar, Rect::new(12., 12., 296., 56.));
                }
                SearchStyle::Divided => {
                    assert_eq!(bar.height, 72.);
                    assert_eq!(bounds(&runtime, "search-divider").y, 72.);
                }
            }
            assert!(bounds(&runtime, RESULTS).y >= bar.y + bar.height);
        }
    }
}

#[test]
fn editor_submit_is_distinct_from_click_candidate_and_clear() {
    let mut runtime = open(SearchStyle::Contained, LayoutDirection::LeftToRight, 600.);
    let rect = bounds(&runtime, INPUT);
    click(&mut runtime, rect.x + 2., rect.y + 8.);
    assert!(runtime.app().submits.is_empty());
    runtime
        .dispatch(Event::Select {
            anchor: 0,
            cursor: 5,
        })
        .unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄓ".into(),
            selection: None,
        }))
        .unwrap();
    assert!(!enter(&mut runtime));
    assert!(runtime.app().submits.is_empty());
    assert!(runtime.input_snapshot().unwrap().composing);
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("中".into())))
        .unwrap();
    assert_eq!(runtime.app().query, "中");
    assert!(enter(&mut runtime));
    assert_eq!(runtime.app().submits, ["中"]);

    runtime.activate("search-clear").unwrap();
    assert_eq!(runtime.app().clears, 1);
    assert_eq!(runtime.focused(), Some(INPUT));
    assert!(runtime.dom().by_id("search-clear").is_none());
    assert_eq!(node(&runtime, "search-status").name(), "History");
}

#[test]
fn clearing_the_search_paints_the_hint_without_changing_the_editor_or_history() {
    let mut runtime = open(SearchStyle::Divided, LayoutDirection::RightToLeft, 320.);
    runtime.activate("search-clear").unwrap();
    let paint = runtime.render(320., 720., &mut Metrics).unwrap();
    assert_eq!(runtime.focused(), Some(INPUT));
    assert_eq!(node(&runtime, INPUT).name(), "Query");
    assert_eq!(runtime.snapshot().text_inputs[INPUT].display_value, "");
    assert_eq!(node(&runtime, "search-status").name(), "History");
    assert_eq!(bounds(&runtime, INPUT).height, 48.);
    assert!(paint.ops.iter().any(|op| matches!(op, PaintOp::StyledText {
        text, color, editable: true, ..
    } if text == "搜尋文件" && *color == Theme::dark().colors.on_surface_variant)));
}

#[test]
fn scroll_selection_and_preedit_survive_mode_switch() {
    let mut runtime = open(SearchStyle::Contained, LayoutDirection::RightToLeft, 600.);
    assert!(runtime.scroll_to(RESULTS, ScrollTarget::Offset { x: 0., y: 144. }));
    render(&mut runtime, 600.);
    assert_eq!(runtime.scroll_metrics(RESULTS).unwrap().offset.1, 144.);
    assert_eq!(bounds(&runtime, "search-bar").y, 36.);
    runtime
        .dispatch(Event::Select {
            anchor: 4,
            cursor: 1,
        })
        .unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄓ".into(),
            selection: None,
        }))
        .unwrap();
    render(&mut runtime, 599.);
    assert_eq!(runtime.scroll_metrics(RESULTS).unwrap().offset.1, 144.);
    assert_eq!(bounds(&runtime, "search-bar").y, 12.);
    let input = runtime.input_snapshot().unwrap();
    assert_eq!(input.id, INPUT);
    assert_eq!(input.value, "alpha");
    assert_eq!((input.selection.anchor, input.selection.cursor), (4, 1));
    assert!(input.composing);
    assert_eq!(runtime.focused(), Some(INPUT));

    runtime.dispatch(Event::Cancel).unwrap();
    assert!(runtime.app().open);
    assert!(!runtime.input_snapshot().unwrap().composing);
    runtime.dispatch(Event::Cancel).unwrap();
    assert!(!runtime.app().open);
    assert_eq!(runtime.focused(), Some("opener"));
}

#[test]
fn caller_results_keep_disabled_and_ordinary_arrow_rules() {
    let mut runtime = open(SearchStyle::Divided, LayoutDirection::RightToLeft, 600.);
    tab(&mut runtime, true);
    assert_eq!(runtime.focused(), Some("search-back"));
    tab(&mut runtime, false);
    assert_eq!(runtime.focused(), Some(INPUT));
    tab(&mut runtime, false);
    assert_eq!(runtime.focused(), Some("search-clear"));
    tab(&mut runtime, false);
    assert_eq!(runtime.focused(), Some("result-0"));
    runtime
        .dispatch(Event::Edit {
            key: EditKey::Down,
            extend: false,
            by_word: false,
        })
        .unwrap();
    assert_eq!(runtime.focused(), Some("result-2"));
    assert!(runtime.app().picked.is_empty());
    runtime.dispatch(Event::Activate).unwrap();
    assert_eq!(runtime.app().picked, [2]);
    assert!(runtime.activate("result-1").is_err());
    render(&mut runtime, 600.);
    runtime
        .dispatch(Event::Edit {
            key: EditKey::Up,
            extend: false,
            by_word: false,
        })
        .unwrap();
    assert_eq!(runtime.focused(), Some("result-0"));
    runtime.apply(Message::RemoveFirstResult).unwrap();
    assert_eq!(runtime.focused(), Some(INPUT));
}

#[test]
fn modal_dismissal_does_not_activate_background_and_recovers_missing_opener() {
    let mut runtime = open(SearchStyle::Contained, LayoutDirection::LeftToRight, 900.);
    assert!(runtime.activate("background").is_err());
    click(&mut runtime, 800., 120.);
    assert_eq!(runtime.app().background, 0);
    assert_eq!(runtime.app().closes, 1);
    assert_eq!(runtime.focused(), Some("opener"));

    runtime.activate("opener").unwrap();
    runtime.apply(Message::RemoveOpener).unwrap();
    render(&mut runtime, 900.);
    runtime.activate("search-back").unwrap();
    assert_eq!(runtime.app().closes, 2);
    assert_eq!(runtime.focused(), Some("background"));
    assert_eq!(runtime.app().background, 0);
}
