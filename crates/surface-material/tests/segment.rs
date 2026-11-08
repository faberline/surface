use surface::{Color, Corners, MicroDom, Role, TextMeasurer};
use surface_app::{Application, EditKey, Event, Runtime, View};
use surface_material::{shape, ChoiceState, SegmentPosition, Theme};

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

struct Multi {
    flags: [bool; 3],
}
impl Application for Multi {
    type Message = usize;
    fn update(&mut self, i: usize) {
        self.flags[i] = !self.flags[i];
    }
    fn view(&self) -> View<usize> {
        let theme = Theme::light();
        let f = self.flags;
        let state = |selected| ChoiceState {
            selected,
            disabled: false,
        };
        theme.page([theme.segmented_buttons(
            "g",
            "Style",
            [
                theme.segment("s0", "日", SegmentPosition::First, state(f[0]), 0),
                theme.segment("s1", "週", SegmentPosition::Middle, state(f[1]), 1),
                theme.segment("s2", "月", SegmentPosition::Last, state(f[2]), 2),
            ],
        )])
    }
}

fn render(dark: bool, position: SegmentPosition, state: ChoiceState) -> (Theme, Runtime<App>) {
    let build: Build = Box::new(move |t| t.segment("s", "Day", position, state, ()));
    let mut r = Runtime::new(App(dark, Box::new(move |t| t.page([build(t)])))).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    let theme = if dark { Theme::dark() } else { Theme::light() };
    (theme, r)
}

fn on(selected: bool) -> ChoiceState {
    ChoiceState {
        selected,
        disabled: false,
    }
}

#[test]
fn corners_follow_the_position() {
    let cases = [
        (SegmentPosition::First, Corners::start(20.)),
        (SegmentPosition::Last, Corners::end(20.)),
        (SegmentPosition::Middle, Corners::all(0.)),
        (SegmentPosition::Only, shape::FULL),
    ];
    for (position, expected) in cases {
        let (_, r) = render(false, position, on(false));
        let dom: &MicroDom = r.dom();
        let id = dom.by_id("s").expect("segment node");
        assert_eq!(
            dom.get(id).unwrap().style().corners,
            Some(expected),
            "{position:?}"
        );
    }
    // NEGATIVE: First and Middle must differ; every segment using FULL would fail here.
    let (_, first) = render(false, SegmentPosition::First, on(false));
    let (_, middle) = render(false, SegmentPosition::Middle, on(false));
    let f = first
        .dom()
        .get(first.dom().by_id("s").unwrap())
        .unwrap()
        .style()
        .corners;
    let m = middle
        .dom()
        .get(middle.dom().by_id("s").unwrap())
        .unwrap()
        .style()
        .corners;
    assert_ne!(f, m);
}

#[test]
fn height_is_40_with_a_one_pixel_outline() {
    let (_, r) = render(false, SegmentPosition::Only, on(false));
    let id = r.dom().by_id("s").unwrap();
    let bounds = r.computed().unwrap().nodes[id.index()].bounds;
    assert_eq!(bounds.height, 40.);
    assert_eq!(r.dom().get(id).unwrap().style().border_width, 1.);
}

#[test]
fn selected_shows_a_check_mark_and_uses_the_tonal_colors() {
    for dark in [false, true] {
        let (theme, r) = render(dark, SegmentPosition::Only, on(true));
        let dom = r.dom();
        let id = dom.by_id("s").unwrap();
        let node = dom.get(id).unwrap();
        assert_eq!(node.style().background, theme.tonal, "dark={dark}");
        assert_eq!(node.is_selected(), Some(true), "dark={dark}");
        let children = dom.children(id).collect::<Vec<_>>();
        assert_eq!(children.len(), 2, "dark={dark}");
        assert_eq!(dom.get(children[0]).unwrap().text_value(), Some("✓"));
        assert_eq!(dom.get(children[1]).unwrap().text_value(), Some("Day"));
    }
}

#[test]
fn unselected_has_no_check_mark_and_is_transparent() {
    for dark in [false, true] {
        let (_, r) = render(dark, SegmentPosition::Only, on(false));
        let dom = r.dom();
        let id = dom.by_id("s").unwrap();
        let node = dom.get(id).unwrap();
        assert_eq!(node.style().background, Color::TRANSPARENT, "dark={dark}");
        assert_eq!(node.is_selected(), Some(false), "dark={dark}");
        let children = dom.children(id).collect::<Vec<_>>();
        assert_eq!(children.len(), 1, "dark={dark}");
        assert_eq!(dom.get(children[0]).unwrap().text_value(), Some("Day"));
    }
}

#[test]
fn disabled_uses_the_disabled_color_and_is_not_focusable() {
    let (theme, mut r) = render(
        false,
        SegmentPosition::Only,
        ChoiceState {
            selected: false,
            disabled: true,
        },
    );
    let id = r.dom().by_id("s").unwrap();
    let node = r.dom().get(id).unwrap();
    assert_eq!(node.style().background, theme.disabled);
    assert!(node.is_disabled());
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_ne!(r.focused(), Some("s"));
}

#[test]
fn multi_select_is_owned_by_the_application() {
    let mut r = Runtime::new(Multi { flags: [false; 3] }).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), Some("s0"));
    r.dispatch(Event::Activate).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();

    r.dispatch(Event::Edit {
        key: EditKey::Right,
        extend: false,
        by_word: false,
    })
    .unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    assert_eq!(r.focused(), Some("s1"));
    r.dispatch(Event::Activate).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();

    assert_eq!(r.app().flags, [true, true, false]);
}

#[test]
fn exposes_a_button_role_and_the_text_as_its_name() {
    let (_, r) = render(false, SegmentPosition::Only, on(false));
    let node = r.dom().get(r.dom().by_id("s").unwrap()).unwrap();
    assert_eq!(node.role(), Role::Button);
    assert_eq!(node.name(), "Day");
}
