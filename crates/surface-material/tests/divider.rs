use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Runtime, View};
use surface_material::{DividerKind, Theme};

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

fn render(dark: bool, build: Build) -> Runtime<App> {
    let mut r = Runtime::new(App(dark, build)).unwrap();
    r.render(400., 300., &mut Metrics).unwrap();
    r
}

fn theme(dark: bool) -> Theme {
    if dark {
        Theme::dark()
    } else {
        Theme::light()
    }
}

fn page_with_kind(kind: DividerKind) -> Build {
    Box::new(move |t| t.page([t.divider_with("d", kind)]))
}

fn node_id(r: &Runtime<App>, id: &str) -> surface::NodeId {
    r.dom().by_id(id).expect(id)
}

// (x, y, width, height) of the node with `id`.
fn bounds(r: &Runtime<App>, id: &str) -> (f32, f32, f32, f32) {
    let computed = r.computed().unwrap();
    let b = computed.nodes[node_id(r, id).index()].bounds;
    (b.x, b.y, b.width, b.height)
}

#[test]
fn all_kinds_use_the_outline_variant_token() {
    for dark in [false, true] {
        for kind in [
            DividerKind::Full,
            DividerKind::Inset,
            DividerKind::MiddleInset,
        ] {
            let t = theme(dark);
            let r = render(dark, page_with_kind(kind));
            let node = r.dom().get(node_id(&r, "d")).unwrap();
            assert!(
                matches!(node.role(), Role::Separator),
                "{kind:?} dark={dark}: role"
            );
            assert_eq!(
                node.style().background,
                t.colors.outline_variant,
                "{kind:?} dark={dark}: token"
            );
            assert_eq!(bounds(&r, "d").3, 1., "{kind:?} dark={dark}: thickness");
            if !dark {
                assert_ne!(
                    node.style().background,
                    t.colors.outline,
                    "{kind:?}: must not use the stronger outline token"
                );
            }
        }
    }
}

#[test]
fn inset_kinds_shift_and_shorten_the_line() {
    let full = bounds(&render(false, page_with_kind(DividerKind::Full)), "d");
    let inset = bounds(&render(false, page_with_kind(DividerKind::Inset)), "d");
    let middle = bounds(
        &render(false, page_with_kind(DividerKind::MiddleInset)),
        "d",
    );
    assert_eq!(inset.0, full.0 + 16., "inset x");
    assert_eq!(inset.2, full.2 - 16., "inset width");
    assert_eq!(middle.0, full.0 + 16., "middle-inset x");
    assert_eq!(middle.2, full.2 - 32., "middle-inset width");
}

#[test]
fn inset_kinds_wrap_the_line_in_a_group() {
    let r = render(false, page_with_kind(DividerKind::Inset));
    let wrapper = r.dom().by_id("d-inset").expect("d-inset wrapper");
    assert!(matches!(r.dom().get(wrapper).unwrap().role(), Role::Group));

    let r = render(false, page_with_kind(DividerKind::Full));
    assert!(r.dom().by_id("d-inset").is_none());
}

#[test]
fn vertical_divider_is_one_dp_wide_and_fills_height() {
    let r = render(
        false,
        Box::new(|t| {
            t.page([View::new(
                Node::new(Role::Group, "Row").styled(Style::row().height(Length::Px(80.))),
            )
            .children([t.vertical_divider("d")])])
        }),
    );
    let (_, _, w, h) = bounds(&r, "d");
    assert_eq!(w, 1., "width");
    assert_eq!(h, 80., "height");
    let node = r.dom().get(node_id(&r, "d")).unwrap();
    assert!(matches!(node.role(), Role::Separator));
    assert_eq!(node.style().background, theme(false).colors.outline_variant);
}

#[test]
fn old_divider_is_unchanged() {
    let r = render(false, Box::new(|t| t.page([t.divider("d")])));
    let node = r.dom().get(node_id(&r, "d")).unwrap();
    assert!(matches!(node.role(), Role::Separator));
    assert_eq!(node.name(), "Divider");
    assert_eq!(bounds(&r, "d").3, 1., "height");
    assert_eq!(node.style().background, theme(false).colors.outline_variant);
}
