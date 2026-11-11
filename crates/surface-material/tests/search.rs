use surface::{Color, NodeId, Role, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};
use surface_material::{shape, Theme};

#[derive(Clone)]
enum Message {
    Input(String),
    Clear,
}

struct Search {
    dark: bool,
    query: String,
    clears: usize,
}
impl Application for Search {
    type Message = Message;
    fn update(&mut self, m: Message) {
        match m {
            Message::Input(q) => self.query = q,
            Message::Clear => {
                self.query.clear();
                self.clears += 1;
            }
        }
    }
    fn view(&self) -> View<Message> {
        let t = theme(self.dark);
        t.page([t.search_bar(
            "search",
            "Search",
            &self.query,
            "S",
            Message::Input,
            Message::Clear,
        )])
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

fn theme(dark: bool) -> Theme {
    if dark {
        Theme::dark()
    } else {
        Theme::light()
    }
}

fn render(dark: bool, query: &str) -> Runtime<Search> {
    let mut r = Runtime::new(Search {
        dark,
        query: query.into(),
        clears: 0,
    })
    .unwrap();
    r.render(600., 400., &mut Metrics).unwrap();
    r
}

fn bounds(r: &Runtime<Search>, id: &str) -> surface::Rect {
    let computed = r.computed().unwrap();
    computed.nodes[r.dom().by_id(id).unwrap().index()].bounds
}

fn background(r: &Runtime<Search>, id: &str) -> Color {
    let node = r.dom().get(r.dom().by_id(id).unwrap()).unwrap();
    node.style().background
}

fn child_ids(r: &Runtime<Search>, id: &str) -> Vec<NodeId> {
    r.dom().children(r.dom().by_id(id).unwrap()).collect()
}

fn foreground(r: &Runtime<Search>, node: NodeId) -> Color {
    r.dom().get(node).unwrap().style().foreground
}

#[test]
fn bar_is_a_56_high_pill() {
    let r = render(false, "");
    let node = r.dom().get(r.dom().by_id("search").unwrap()).unwrap();
    assert_eq!(node.role(), Role::Group);
    assert_eq!(node.name(), "Search");
    assert_eq!(bounds(&r, "search").height, 56.);
    assert_eq!(node.style().corners, Some(shape::FULL));
}

#[test]
fn container_uses_the_token_in_both_themes() {
    for dark in [false, true] {
        let r = render(dark, "");
        let t = theme(dark);
        assert_eq!(
            background(&r, "search"),
            t.colors.surface_container_high,
            "dark={dark}"
        );
        // Negative: the bar is not the page background.
        assert_ne!(background(&r, "search"), t.background, "dark={dark}");
    }
}

#[test]
fn children_follow_the_query() {
    let r = render(false, "");
    assert_eq!(child_ids(&r, "search").len(), 2);
    let input = r.dom().get(r.dom().by_id("search-input").unwrap()).unwrap();
    assert_eq!(input.role(), Role::TextInput);
    assert_eq!(input.name(), "Search");
    assert!(r.dom().by_id("search-clear").is_none());

    let r = render(false, "abc");
    assert_eq!(child_ids(&r, "search").len(), 3);
    let clear = r.dom().get(r.dom().by_id("search-clear").unwrap()).unwrap();
    assert_eq!(clear.role(), Role::Button);
    assert_eq!(clear.name(), "Clear");
}

#[test]
fn glyph_and_clear_colors_use_tokens() {
    for dark in [false, true] {
        let r = render(dark, "abc");
        let t = theme(dark);
        let kids = child_ids(&r, "search");
        assert_eq!(
            foreground(&r, kids[0]),
            t.colors.on_surface_variant,
            "glyph dark={dark}"
        );
        let clear_label = r
            .dom()
            .children(r.dom().by_id("search-clear").unwrap())
            .next();
        let clear_label = clear_label.expect("clear icon");
        assert_eq!(
            foreground(&r, clear_label),
            t.colors.on_surface_variant,
            "clear dark={dark}"
        );
        let input = r.dom().by_id("search-input").unwrap();
        assert_eq!(
            foreground(&r, input),
            t.colors.on_surface,
            "input dark={dark}"
        );
    }
}

#[test]
fn typing_updates_the_controlled_query() {
    let mut r = render(false, "");
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    if r.focused() == Some("search") {
        r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    }
    assert_eq!(r.focused(), Some("search-input"));

    r.dispatch(Event::Text("ab".into())).unwrap();
    r.render(600., 400., &mut Metrics).unwrap();
    assert_eq!(r.app().query, "ab");
    assert!(r.dom().by_id("search-clear").is_some());
}

#[test]
fn clear_button_empties_the_query() {
    let mut r = render(false, "abc");
    assert!(r.activate("search-clear").unwrap());
    assert_eq!(r.app().clears, 1);
    assert!(r.app().query.is_empty());
    r.render(600., 400., &mut Metrics).unwrap();
    assert!(r.dom().by_id("search-clear").is_none());
}
