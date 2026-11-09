use surface::{Color, MicroDom, NodeId, Role, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};
use surface_material::{shape, CardKind, Theme};

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

fn runtime(dark: bool, build: impl Fn(&Theme) -> View<()> + 'static) -> Runtime<App> {
    Runtime::new(App(dark, Box::new(build))).unwrap()
}

fn theme_for(dark: bool) -> Theme {
    if dark {
        Theme::dark()
    } else {
        Theme::light()
    }
}

fn by_id(dom: &MicroDom, id: &str) -> NodeId {
    dom.by_id(id).unwrap_or_else(|| panic!("no node {id}"))
}

const KINDS: [CardKind; 3] = [CardKind::Elevated, CardKind::Filled, CardKind::Outlined];

#[test]
fn kinds_use_the_m3_container_tokens() {
    for dark in [false, true] {
        let theme = theme_for(dark);
        for kind in KINDS {
            let mut r = runtime(dark, move |t| t.page([t.card_with("c", "Card", kind, [])]));
            r.render(400., 300., &mut Metrics).unwrap();
            let style = r.dom().get(by_id(r.dom(), "c")).unwrap().style().clone();
            assert_eq!(style.corners, Some(shape::MEDIUM), "{kind:?} dark={dark}");
            match kind {
                CardKind::Elevated => {
                    assert_eq!(style.background, theme.colors.surface_container_low);
                    assert_eq!(style.border_width, 0.);
                }
                CardKind::Filled => {
                    assert_eq!(style.background, theme.colors.surface_container_highest);
                    assert_eq!(style.border_width, 0.);
                }
                CardKind::Outlined => {
                    assert_eq!(style.background, theme.colors.surface);
                    assert_eq!(style.border_width, 1.);
                    assert_eq!(style.border_color, theme.colors.outline_variant);
                }
            }
        }
    }
}

#[test]
fn kinds_are_different_surfaces() {
    // Negative: if the kinds shared one color, this fails.
    let theme = Theme::light();
    let mut backgrounds: Vec<Color> = Vec::new();
    for kind in KINDS {
        let mut r = runtime(false, move |t| t.page([t.card_with("c", "Card", kind, [])]));
        r.render(400., 300., &mut Metrics).unwrap();
        backgrounds.push(r.dom().get(by_id(r.dom(), "c")).unwrap().style().background);
    }
    assert_ne!(backgrounds[0], backgrounds[1], "elevated vs filled");
    assert_ne!(backgrounds[1], backgrounds[2], "filled vs outlined");
    assert_ne!(backgrounds[0], backgrounds[2], "elevated vs outlined");
    assert_ne!(
        backgrounds[1], theme.container,
        "filled must not reuse the old card color"
    );
}

#[test]
fn card_with_is_a_group_named_by_the_caller() {
    let mut r = runtime(false, |t| {
        t.page([t.card_with("c", "Card", CardKind::Filled, [])])
    });
    r.render(400., 300., &mut Metrics).unwrap();
    let node = r.dom().get(by_id(r.dom(), "c")).unwrap();
    assert_eq!(node.role(), Role::Group);
    assert_eq!(node.name(), "Card");

    let mut r = runtime(false, |t| {
        t.page([t.card_with("c", "Other", CardKind::Filled, [])])
    });
    r.render(400., 300., &mut Metrics).unwrap();
    assert_eq!(r.dom().get(by_id(r.dom(), "c")).unwrap().name(), "Other");
}

struct Counter {
    count: usize,
    disabled: bool,
}
impl Application for Counter {
    type Message = ();
    fn update(&mut self, _: ()) {
        self.count += 1;
    }
    fn view(&self) -> View<()> {
        let theme = Theme::light();
        theme.page([theme.action_card(
            "c",
            "Open",
            CardKind::Filled,
            self.disabled,
            (),
            [theme.text("t", "Open")],
        )])
    }
}

fn counter(disabled: bool) -> Runtime<Counter> {
    Runtime::new(Counter { count: 0, disabled }).unwrap()
}

#[test]
fn action_card_is_a_button_and_sends_the_message() {
    let mut r = counter(false);
    r.render(400., 300., &mut Metrics).unwrap();
    assert_eq!(
        r.dom().get(by_id(r.dom(), "c")).unwrap().role(),
        Role::Button
    );

    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), Some("c"));
    r.dispatch(Event::Activate).unwrap();
    assert_eq!(r.app().count, 1);

    r.render(400., 300., &mut Metrics).unwrap();
    let b = r.computed().unwrap().nodes[by_id(r.dom(), "c").index()].bounds;
    r.dispatch(Event::PointerDown {
        x: b.x + 4.,
        y: b.y + 4.,
    })
    .unwrap();
    r.dispatch(Event::PointerUp {
        x: b.x + 4.,
        y: b.y + 4.,
    })
    .unwrap();
    assert_eq!(r.app().count, 2);
}

#[test]
fn disabled_action_card_sends_nothing() {
    let theme = Theme::light();
    let mut r = counter(true);
    r.render(400., 300., &mut Metrics).unwrap();
    let node = r.dom().get(by_id(r.dom(), "c")).unwrap();
    assert!(node.is_disabled());
    assert_eq!(node.style().background, theme.disabled);
    assert_eq!(node.style().foreground, theme.outline);

    r.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(r.focused(), None);
    // No node is focused, so Activate may return Err; the message count is the proof.
    let _ = r.dispatch(Event::Activate);

    r.render(400., 300., &mut Metrics).unwrap();
    let b = r.computed().unwrap().nodes[by_id(r.dom(), "c").index()].bounds;
    let _ = r.dispatch(Event::PointerDown {
        x: b.x + 4.,
        y: b.y + 4.,
    });
    let _ = r.dispatch(Event::PointerUp {
        x: b.x + 4.,
        y: b.y + 4.,
    });
    assert_eq!(r.app().count, 0);
}

#[test]
fn existing_cards_are_unchanged() {
    let theme = Theme::light();
    let mut r = runtime(false, |t| {
        t.page([t.card("a", []), t.outlined_card("b", [])])
    });
    r.render(400., 300., &mut Metrics).unwrap();

    let a = r.dom().get(by_id(r.dom(), "a")).unwrap().style().clone();
    assert_eq!(a.background, theme.container);
    assert_eq!(a.corners, Some(shape::MEDIUM));

    let b = r.dom().get(by_id(r.dom(), "b")).unwrap().style().clone();
    assert_eq!(b.border_width, 1.);
    assert_eq!(b.border_color, theme.outline);
}
