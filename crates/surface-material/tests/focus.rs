use surface::{Color, Corners, DisplayList, MicroDom, PaintOp, TextMeasurer};
use surface_app::{Application, Runtime, View};
use surface_material::{shape, ButtonKind, ChipKind, ChoiceState, Theme};

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

fn luminance(c: Color) -> f64 {
    let f = |v: u8| {
        let v = v as f64 / 255.;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
}
fn contrast(a: Color, b: Color) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn color_of(op: &PaintOp) -> Option<Color> {
    match op {
        PaintOp::Quad { color, .. }
        | PaintOp::Stroke { color, .. }
        | PaintOp::Shape { color, .. }
        | PaintOp::ShapeStroke { color, .. } => Some(*color),
        _ => None,
    }
}
fn follows(op: &PaintOp, corners: Corners, w: f32, h: f32) -> bool {
    let resolved = corners.resolve(w, h);
    match (op, resolved.uniform()) {
        (PaintOp::Stroke { radius, .. }, Some(r)) => *radius == r,
        (PaintOp::ShapeStroke { corners, .. }, None) => *corners == resolved,
        _ => false,
    }
}

struct Ring {
    ops: Vec<PaintOp>,
    size: (f32, f32),
}
fn ring(dark: bool, build: impl Fn(&Theme) -> View<()> + 'static, id: &str) -> (Theme, Ring) {
    let theme = if dark { Theme::dark() } else { Theme::light() };
    let mut r = Runtime::new(App(dark, Box::new(move |t| t.page([build(t)])))).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    let dom: &MicroDom = r.dom();
    let node = dom.by_id(id).unwrap_or_else(|| panic!("no node {id}"));
    let computed = r.computed().unwrap();
    let plain = DisplayList::compile(dom, computed, None);
    let focused = DisplayList::compile(dom, computed, Some(node));
    let ops = focused
        .ops
        .into_iter()
        .filter(|op| !plain.ops.contains(op))
        .collect();
    let b = computed.nodes[node.index()].bounds;
    (
        theme,
        Ring {
            ops,
            size: (b.width, b.height),
        },
    )
}

fn check_contrast(name: &str, dark: bool, theme: &Theme, ring: &Ring) {
    assert!(!ring.ops.is_empty(), "{name} dark={dark}: no ring");
    for op in &ring.ops {
        let c = color_of(op).expect("ring op has a color");
        for (what, bg) in [
            ("background", theme.background),
            ("container", theme.container),
        ] {
            let ratio = contrast(c, bg);
            assert!(
                ratio >= 3.,
                "{name} dark={dark}: ring {c:?} on {what} {bg:?} is {ratio:.2}:1"
            );
        }
    }
}

fn controls() -> Vec<(String, &'static str, Build)> {
    let mut v: Vec<(String, &'static str, Build)> = Vec::new();
    for kind in [
        ButtonKind::Filled,
        ButtonKind::Tonal,
        ButtonKind::Outlined,
        ButtonKind::Text,
    ] {
        v.push((
            format!("button {kind:?}"),
            "c",
            Box::new(move |t| t.button_with("c", "Open", kind, false, ())),
        ));
    }
    for kind in [
        ChipKind::Assist,
        ChipKind::Suggestion,
        ChipKind::Filter,
        ChipKind::Input,
    ] {
        v.push((
            format!("chip {kind:?}"),
            "c",
            Box::new(move |t| t.chip("c", "Chip", kind, ChoiceState::default(), (), Some(()))),
        ));
    }
    v.push((
        "input chip remove".into(),
        "c-remove",
        Box::new(|t| {
            t.chip(
                "c",
                "Chip",
                ChipKind::Input,
                ChoiceState::default(),
                (),
                Some(()),
            )
        }),
    ));
    v.push((
        "checkbox".into(),
        "c",
        Box::new(|t| t.checkbox("c", "Check", true, ())),
    ));
    v.push((
        "radio".into(),
        "c",
        Box::new(|t| t.radio("c", "Radio", ChoiceState::default(), ())),
    ));
    v.push((
        "switch".into(),
        "c",
        Box::new(|t| t.switch("c", "Switch", ChoiceState::default(), ())),
    ));
    v.push((
        "segmented".into(),
        "c",
        Box::new(|t| t.segmented_button("c", "Seg", ChoiceState::default(), ())),
    ));
    v.push((
        "tab".into(),
        "c",
        Box::new(|t| t.tab("c", "Tab", ChoiceState::default(), ())),
    ));
    v.push((
        "slider".into(),
        "c",
        Box::new(|t| {
            t.slider(
                "c",
                "Volume",
                surface::RangeValue {
                    min: 0.,
                    max: 10.,
                    value: 5.,
                    step: 1.,
                },
                false,
                |_| (),
            )
        }),
    ));
    v.push((
        "menu item".into(),
        "c",
        Box::new(|t| t.menu_item("c", "One", false, ())),
    ));
    v.push((
        "text field".into(),
        "c",
        Box::new(|t| t.text_field("c", "Name", "Ada", |_| ())),
    ));
    v
}

#[test]
fn focus_ring_contrasts_with_background_and_container() {
    for dark in [false, true] {
        for (name, id, build) in controls() {
            let build = std::rc::Rc::new(build);
            let b = build.clone();
            let (theme, r) = ring(dark, move |t| b(t), id);
            check_contrast(&name, dark, &theme, &r);
        }
    }
}

#[test]
fn button_ring_is_one_full_stroke() {
    for dark in [false, true] {
        for kind in [
            ButtonKind::Filled,
            ButtonKind::Tonal,
            ButtonKind::Outlined,
            ButtonKind::Text,
        ] {
            let (_, r) = ring(
                dark,
                move |t| t.button_with("c", "Open", kind, false, ()),
                "c",
            );
            assert_eq!(r.ops.len(), 1, "{kind:?} dark={dark}: {:#?}", r.ops);
            assert!(
                follows(&r.ops[0], shape::FULL, r.size.0, r.size.1),
                "{kind:?} dark={dark}: {:#?}",
                r.ops
            );
        }
    }
}

#[test]
fn chip_ring_follows_the_small_shape() {
    for dark in [false, true] {
        for kind in [
            ChipKind::Assist,
            ChipKind::Suggestion,
            ChipKind::Filter,
            ChipKind::Input,
        ] {
            let (_, r) = ring(
                dark,
                move |t| t.chip("c", "Chip", kind, ChoiceState::default(), (), Some(())),
                "c",
            );
            assert_eq!(r.ops.len(), 1, "{kind:?} dark={dark}: {:#?}", r.ops);
            assert!(
                follows(&r.ops[0], shape::SMALL, r.size.0, r.size.1),
                "{kind:?} dark={dark}: {:#?}",
                r.ops
            );
        }
    }
}
