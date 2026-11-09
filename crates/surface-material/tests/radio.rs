use surface::{Color, Role, TextMeasurer};
use surface_app::{Application, EditKey, Event, Runtime, View};
use surface_material::{ChoiceState, Theme};

type Build = Box<dyn Fn(&Theme) -> View<()>>;
struct App(bool, Build);
impl Application for App {
    type Message = ();
    fn update(&mut self, _: ()) {}
    fn view(&self) -> View<()> {
        let theme = if self.0 {
            Theme::dark()
        } else {
            Theme::light()
        };
        self.1(&theme)
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

/// Three radios: `a` Standard, `b` Express, `c` Paused (disabled).
/// `selected` is the index of the selected radio.
fn items<M: Clone>(t: &Theme, selected: usize, message: impl Fn(usize) -> M) -> Vec<View<M>> {
    [("a", "Standard"), ("b", "Express"), ("c", "Paused")]
        .into_iter()
        .enumerate()
        .map(|(i, (id, text))| {
            t.radio(
                id,
                text,
                ChoiceState {
                    selected: selected == i,
                    disabled: i == 2,
                },
                message(i),
            )
        })
        .collect()
}

fn render(dark: bool, selected: usize) -> (Theme, Runtime<App>) {
    let build: Build =
        Box::new(move |t| t.page([t.radio_group("group", "Delivery", items(t, selected, |_| ()))]));
    let mut r = Runtime::new(App(dark, build)).unwrap();
    r.render(600., 400., &mut Metrics).unwrap();
    let theme = if dark { Theme::dark() } else { Theme::light() };
    (theme, r)
}

fn bounds<A: Application>(r: &Runtime<A>, id: &str) -> surface::Rect {
    let computed = r.computed().unwrap();
    computed.nodes[r.dom().by_id(id).unwrap().index()].bounds
}

/// Ring box: the first child of the radio. Its border colour is the ring.
fn ring_color(r: &Runtime<App>, id: &str) -> Color {
    let radio = r.dom().by_id(id).unwrap();
    let ring = r.dom().children(radio).next().expect("ring");
    r.dom().get(ring).unwrap().style().border_color
}

/// Dot: the only child of the ring. Its background is the dot colour.
fn dot_color(r: &Runtime<App>, id: &str) -> Color {
    let radio = r.dom().by_id(id).unwrap();
    let ring = r.dom().children(radio).next().expect("ring");
    let dot = r.dom().children(ring).next().expect("dot");
    r.dom().get(dot).unwrap().style().background
}

/// Label: the second child of the radio.
fn label_color(r: &Runtime<App>, id: &str) -> Color {
    let radio = r.dom().by_id(id).unwrap();
    let label = r.dom().children(radio).nth(1).expect("label");
    r.dom().get(label).unwrap().style().foreground
}

#[test]
fn roles_names_and_checked_state() {
    let (_, r) = render(false, 0);
    let node = |id: &str| r.dom().get(r.dom().by_id(id).unwrap()).unwrap();
    let group = r.dom().get(r.dom().by_id("group").unwrap()).unwrap();
    assert_eq!(group.role(), Role::RadioGroup);
    for (id, name) in [("a", "Standard"), ("b", "Express"), ("c", "Paused")] {
        assert_eq!(node(id).role(), Role::Radio, "{id}");
        assert_eq!(node(id).name(), name, "{id}");
    }
    assert_eq!(node("a").is_checked(), Some(true));
    assert_eq!(node("b").is_checked(), Some(false));
    assert!(node("c").is_disabled());
    assert!(!node("a").is_disabled());
}

#[test]
fn ring_color_follows_the_state_in_both_themes() {
    for dark in [false, true] {
        let (theme, r) = render(dark, 0);
        assert_eq!(ring_color(&r, "a"), theme.primary, "selected dark={dark}");
        assert_eq!(
            ring_color(&r, "b"),
            theme.colors.on_surface_variant,
            "unselected dark={dark}"
        );
        assert_eq!(ring_color(&r, "c"), theme.outline, "disabled dark={dark}");
        // Negative: an unselected ring is not the primary colour.
        assert_ne!(ring_color(&r, "b"), theme.primary, "dark={dark}");
    }
}

#[test]
fn dot_is_filled_only_when_selected() {
    for dark in [false, true] {
        let (theme, r) = render(dark, 0);
        assert_eq!(dot_color(&r, "a"), theme.primary, "dark={dark}");
        assert_eq!(dot_color(&r, "b"), Color::TRANSPARENT, "dark={dark}");
        assert_eq!(dot_color(&r, "c"), Color::TRANSPARENT, "dark={dark}");
    }
}

#[test]
fn disabled_label_uses_the_disabled_color() {
    for dark in [false, true] {
        let (theme, r) = render(dark, 0);
        assert_eq!(label_color(&r, "a"), theme.on_container, "dark={dark}");
        assert_eq!(label_color(&r, "c"), theme.outline, "dark={dark}");
        // Negative: the disabled label is not the enabled label colour.
        assert_ne!(label_color(&r, "c"), theme.on_container, "dark={dark}");
    }
}

#[test]
fn selection_changes_the_output() {
    let (theme, r) = render(false, 0);
    assert_eq!(dot_color(&r, "a"), theme.primary);
    assert_eq!(dot_color(&r, "b"), Color::TRANSPARENT);

    let (theme, r) = render(false, 1);
    assert_eq!(dot_color(&r, "a"), Color::TRANSPARENT);
    assert_eq!(dot_color(&r, "b"), theme.primary);
    assert_eq!(ring_color(&r, "a"), theme.colors.on_surface_variant);
}

#[test]
fn row_keeps_the_48_dp_touch_target() {
    let (_, r) = render(false, 0);
    for id in ["a", "b", "c"] {
        assert!(bounds(&r, id).height >= 48., "radio {id}");
    }
}

struct Counter {
    count: usize,
}
impl Application for Counter {
    type Message = ();
    fn update(&mut self, _: ()) {
        self.count += 1;
    }
    fn view(&self) -> View<()> {
        let theme = Theme::light();
        theme.page([theme.radio_group("group", "Delivery", items(&theme, 0, |_| ()))])
    }
}

#[test]
fn press_sends_the_message_and_disabled_does_not() {
    let mut r = Runtime::new(Counter { count: 0 }).unwrap();
    r.render(600., 400., &mut Metrics).unwrap();
    assert!(r.activate("b").unwrap());
    assert_eq!(r.app().count, 1);
    // A disabled radio sends nothing; the call may fail or report false.
    let _ = r.activate("c");
    assert_eq!(r.app().count, 1);
}

#[derive(Default)]
struct Picker {
    selected: usize,
}
impl Application for Picker {
    type Message = usize;
    fn update(&mut self, m: usize) {
        self.selected = m;
    }
    fn view(&self) -> View<usize> {
        let theme = Theme::light();
        theme.page([theme.radio_group("group", "Delivery", items(&theme, self.selected, |i| i))])
    }
}

#[test]
fn arrow_keys_select_and_skip_disabled() {
    let mut r = Runtime::new(Picker::default()).unwrap();
    r.render(600., 400., &mut Metrics).unwrap();
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), Some("a"));

    let down = Event::Edit {
        key: EditKey::Down,
        extend: false,
        by_word: false,
    };
    r.dispatch(down.clone()).unwrap();
    assert_eq!(r.focused(), Some("b"));
    assert_eq!(r.app().selected, 1, "arrow selects the focused radio");

    // Down again skips the disabled `c` and wraps to `a`.
    r.dispatch(down).unwrap();
    assert_eq!(r.focused(), Some("a"));
    assert_eq!(r.app().selected, 0);
}
