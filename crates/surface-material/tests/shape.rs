use surface::{Corners, MicroDom, NodeId, PaintOp, Role, TextMeasurer};
use surface_app::{Application, Runtime, View};
use surface_material::{shape, ChipKind, ChoiceState, Theme};

#[test]
fn shape_tokens_match_androidx() {
    // androidx.compose.material3.tokens.ShapeTokens (androidx-main).
    assert_eq!(shape::NONE, Corners::all(0.));
    assert_eq!(shape::EXTRA_SMALL, Corners::all(4.));
    assert_eq!(shape::EXTRA_SMALL_TOP, Corners::top(4.));
    assert_eq!(shape::SMALL, Corners::all(8.));
    assert_eq!(shape::MEDIUM, Corners::all(12.));
    assert_eq!(shape::LARGE, Corners::all(16.));
    assert_eq!(shape::LARGE_TOP, Corners::top(16.));
    assert_eq!(shape::LARGE_START, Corners::start(16.));
    assert_eq!(shape::LARGE_END, Corners::end(16.));
    assert_eq!(shape::LARGE_INCREASED, Corners::all(20.));
    assert_eq!(shape::EXTRA_LARGE, Corners::all(28.));
    assert_eq!(shape::EXTRA_LARGE_TOP, Corners::top(28.));
    assert_eq!(shape::EXTRA_LARGE_INCREASED, Corners::all(32.));
    assert_eq!(shape::EXTRA_EXTRA_LARGE, Corners::all(48.));
    assert_eq!(shape::FULL, Corners::full());
    assert_eq!(shape::EXTRA_SMALL_TOP, Corners::new(4., 4., 0., 0.));
    assert_eq!(shape::LARGE_START, Corners::new(16., 0., 0., 16.));
    assert_eq!(shape::LARGE_END, Corners::new(0., 16., 16., 0.));
}

type Build = Box<dyn Fn(&Theme) -> View<()>>;
struct Tiny(Build);
impl Application for Tiny {
    type Message = ();
    fn update(&mut self, _: ()) {}
    fn view(&self) -> View<()> {
        self.0(&Theme::light())
    }
}
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}
fn runtime(build: impl Fn(&Theme) -> View<()> + 'static) -> Runtime<Tiny> {
    Runtime::new(Tiny(Box::new(build))).unwrap()
}
fn corners_of(dom: &MicroDom, id: NodeId) -> Option<Corners> {
    dom.get(id).unwrap().style().corners
}
fn by_id(dom: &MicroDom, id: &str) -> NodeId {
    dom.by_id(id).unwrap_or_else(|| panic!("no node {id}"))
}
fn child(dom: &MicroDom, id: NodeId, index: usize) -> NodeId {
    dom.children(id)
        .nth(index)
        .unwrap_or_else(|| panic!("no child {index}"))
}

#[test]
fn components_use_shape_tokens() {
    let r = runtime(|t| t.card("card", []));
    assert_eq!(
        corners_of(r.dom(), by_id(r.dom(), "card")),
        Some(shape::MEDIUM)
    );

    let r = runtime(|t| t.outlined_card("outlined", []));
    assert_eq!(
        corners_of(r.dom(), by_id(r.dom(), "outlined")),
        Some(shape::MEDIUM)
    );

    let r = runtime(|t| t.text_field("name", "Name", "", |_| ()));
    assert_eq!(
        corners_of(r.dom(), by_id(r.dom(), "name")),
        Some(shape::EXTRA_SMALL_TOP)
    );

    let r = runtime(|t| t.button("ok", "OK", ()));
    let visual = child(r.dom(), by_id(r.dom(), "ok"), 0);
    assert_eq!(corners_of(r.dom(), visual), Some(shape::FULL));

    let r = runtime(|t| t.dialog("dlg", "Title", [], [], ()));
    assert_eq!(
        corners_of(r.dom(), by_id(r.dom(), "dlg")),
        Some(shape::EXTRA_LARGE)
    );

    let r = runtime(|t| {
        t.menu_at(
            "menu",
            "Menu",
            (0., 0.),
            [t.menu_item("one", "One", false, ())],
            (),
        )
    });
    assert_eq!(
        corners_of(r.dom(), by_id(r.dom(), "menu")),
        Some(shape::EXTRA_SMALL)
    );

    let r = runtime(|t| {
        t.chip(
            "chip",
            "Chip",
            ChipKind::Filter,
            ChoiceState::default(),
            (),
            None,
        )
    });
    assert_eq!(
        corners_of(r.dom(), by_id(r.dom(), "chip")),
        Some(shape::SMALL)
    );

    let r = runtime(|t| t.checkbox("check", "Check", true, ()));
    let checkbox = child(r.dom(), by_id(r.dom(), "check"), 0);
    assert_eq!(corners_of(r.dom(), checkbox), Some(Corners::all(2.)));

    let r = runtime(|t| {
        t.tab(
            "tab",
            "Tab",
            ChoiceState {
                selected: true,
                disabled: false,
            },
            (),
        )
    });
    let indicator = child(r.dom(), by_id(r.dom(), "tab"), 1);
    assert_eq!(corners_of(r.dom(), indicator), Some(Corners::all(3.)));
    assert_eq!(
        r.dom().get(indicator).unwrap().role(),
        Role::Group,
        "second child of a selected tab is the indicator"
    );

    let r = runtime(|t| t.segmented_button("seg", "Seg", ChoiceState::default(), ()));
    assert_eq!(
        corners_of(r.dom(), by_id(r.dom(), "seg")),
        Some(shape::FULL)
    );

    let r = runtime(|t| t.switch("sw", "Switch", ChoiceState::default(), ()));
    let track = child(r.dom(), by_id(r.dom(), "sw"), 0);
    let thumb = child(r.dom(), track, 0);
    assert_eq!(corners_of(r.dom(), track), Some(shape::FULL));
    assert_eq!(corners_of(r.dom(), thumb), Some(shape::FULL));

    let r = runtime(|t| t.radio("radio", "Radio", ChoiceState::default(), ()));
    let outer = child(r.dom(), by_id(r.dom(), "radio"), 0);
    let dot = child(r.dom(), outer, 0);
    assert_eq!(corners_of(r.dom(), outer), Some(shape::FULL));
    assert_eq!(corners_of(r.dom(), dot), Some(shape::FULL));

    let r = runtime(|t| {
        t.slider(
            "slider",
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
    });
    let track = child(r.dom(), by_id(r.dom(), "slider"), 0);
    let kids: Vec<_> = r.dom().children(track).collect();
    assert_eq!(kids.len(), 3);
    assert_eq!(corners_of(r.dom(), kids[0]), Some(shape::FULL));
    assert_eq!(corners_of(r.dom(), kids[1]), Some(shape::FULL));
    let handle = child(r.dom(), kids[2], 0);
    assert_eq!(corners_of(r.dom(), handle), Some(shape::FULL));
}

#[test]
fn text_field_renders_a_top_rounded_shape() {
    let mut r = runtime(|t| t.page([t.text_field("name", "Name", "Ada", |_| ())]));
    let list = r.render(400., 300., &mut Metrics).unwrap();
    assert!(
        list.ops.iter().any(|op| matches!(op,
            PaintOp::Shape { corners, .. } if *corners == Corners::new(4., 4., 0., 0.))),
        "{:#?}",
        list.ops
    );
}
