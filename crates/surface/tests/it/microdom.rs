use surface::*;

#[test]
fn arena_preserves_links_and_semantic_indexes_and_rejects_duplicate_ids() {
    let mut dom = MicroDom::new(Node::new(Role::Window, "App").id("app"));
    let first = dom
        .append(dom.root(), Node::new(Role::Button, "Save").id("save"))
        .unwrap();
    let second = dom
        .append(dom.root(), Node::new(Role::Button, "Save").id("save-copy"))
        .unwrap();
    assert_eq!(
        dom.children(dom.root()).collect::<Vec<_>>(),
        vec![first, second]
    );
    assert_eq!(dom.by_id("save"), Some(first));
    assert_eq!(dom.by_role_name(Role::Button, "Save"), &[first, second]);
    assert_eq!(dom.get(first).unwrap().parent(), Some(dom.root()));
    assert_eq!(
        dom.append(first, Node::new(Role::Group, "bad").id("save")),
        Err(DomError::DuplicateId("save".to_owned()))
    );
    assert_eq!(dom.len(), 3);
    assert_eq!(dom.children(first).count(), 0);
    let invalid: NodeId = serde_json::from_str("999").unwrap();
    assert_eq!(
        dom.append(invalid, Node::new(Role::Text, "bad")),
        Err(DomError::InvalidNode(999))
    );
    let one = serde_json::to_vec(&dom.snapshot()).unwrap();
    assert_eq!(one, serde_json::to_vec(&dom.snapshot()).unwrap());
    let restored: DomSnapshot = serde_json::from_slice(&one).unwrap();
    assert_eq!(restored, dom.snapshot());
}

#[derive(Default)]
struct Metrics {
    calls: Vec<String>,
}
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        self.calls.push(text.to_owned());
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}
fn fixture() -> MicroDom {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "App").styled(Style::column().padding(10.).gap(10.).clip(true)),
    );
    for (id, text) in [("first", "One"), ("second", "Two")] {
        dom.append(
            dom.root(),
            Node::new(Role::Button, text).id(id).text(text).styled(
                Style::default()
                    .padding(4.)
                    .background(Color::rgb(103, 80, 164))
                    .radius(12.),
            ),
        )
        .unwrap();
    }
    dom
}
#[test]
fn layout_reuses_measurements_and_updates_changed_leaf() {
    let mut dom = fixture();
    let mut engine = LayoutEngine::default();
    let mut metrics = Metrics::default();
    let a = engine.compute(&dom, 300., 200., &mut metrics).unwrap();
    assert!(!metrics.calls.is_empty());
    metrics.calls.clear();
    let b = engine.compute(&dom, 300., 200., &mut metrics).unwrap();
    assert!(
        metrics.calls.is_empty(),
        "unchanged frame must reuse text measures"
    );
    assert_eq!(a.nodes[1].bounds, b.nodes[1].bounds);
    let first = dom.by_id("first").unwrap();
    dom.set_text(first, "Changed").unwrap();
    engine.compute(&dom, 300., 200., &mut metrics).unwrap();
    assert!(metrics.calls.iter().any(|s| s == "Changed"));
    assert!(
        !metrics.calls.iter().any(|s| s == "Two"),
        "unchanged sibling uses Taffy cache"
    );
}

#[test]
fn fixed_layers_escape_scroll_clips_and_paint_above_flow_content() {
    let mut root = Style::default()
        .width(Length::Px(120.))
        .height(Length::Px(60.))
        .clip(true);
    root.scroll_y = 160.;
    let mut dom = MicroDom::new(Node::new(Role::Window, "App").styled(root));
    let parent = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "clipped").styled(
                Style::default()
                    .width(Length::Px(80.))
                    .height(Length::Px(40.))
                    .padding(10.)
                    .clip(true),
            ),
        )
        .unwrap();
    let layer = dom
        .append(
            parent,
            Node::new(Role::Button, "overlay").id("overlay").styled(
                Style::default()
                    .width(Length::Px(100.))
                    .height(Length::Px(80.))
                    .fixed(Insets {
                        left: Length::Px(140.),
                        top: Length::Px(90.),
                        ..Default::default()
                    }),
            ),
        )
        .unwrap();
    let computed = LayoutEngine::default()
        .compute(&dom, 320., 240., &mut Metrics::default())
        .unwrap();
    assert_eq!(
        computed.nodes[layer.index()].bounds,
        Rect::new(140., 90., 100., 80.)
    );
    assert_eq!(
        computed.nodes[layer.index()].clip,
        Rect::new(0., 0., 320., 240.)
    );
    assert_eq!(computed.hit_test(&dom, 150., 100.), Some(layer));
    assert_eq!(computed.paint_order.last(), Some(&layer));
}
#[test]
fn scroll_offsets_and_ancestor_clips_control_hit_testing() {
    let mut dom = fixture();
    let mut metrics = Metrics::default();
    let mut engine = LayoutEngine::default();
    let id = dom.by_id("second").unwrap();
    let original = engine.compute(&dom, 300., 50., &mut metrics).unwrap();
    let r = original.nodes[id.index()].bounds;
    assert!(r.y >= 50.);
    assert_eq!(original.hit_test(&dom, r.x + 2., r.y + 2.), None);
    let mut style = dom.get(dom.root()).unwrap().style().clone();
    style.scroll_y = 30.;
    dom.set_style(dom.root(), style).unwrap();
    let scrolled = engine.compute(&dom, 300., 50., &mut metrics).unwrap();
    let r = scrolled.nodes[id.index()].bounds;
    assert_eq!(r.y, original.nodes[id.index()].bounds.y - 30.);
    assert_eq!(scrolled.hit_test(&dom, r.x + 2., r.y + 2.), Some(id));
    assert_eq!(scrolled.hit_test(&dom, r.x + 2., 51.), None);
}
#[test]
fn invalid_input_returns_errors_and_display_list_is_canonical() {
    let mut dom = fixture();
    let mut engine = LayoutEngine::default();
    let mut metrics = Metrics::default();
    assert!(matches!(
        engine.compute(&dom, f32::NAN, 100., &mut metrics),
        Err(LayoutError::InvalidViewport)
    ));
    let c = engine.compute(&dom, 300., 200., &mut metrics).unwrap();
    let a = DisplayList::compile(&dom, &c, None);
    let b = DisplayList::compile(&dom, &c, None);
    assert_eq!(
        serde_json::to_vec(&a).unwrap(),
        serde_json::to_vec(&b).unwrap()
    );
    assert!(a
        .ops
        .iter()
        .any(|op| matches!(op,PaintOp::Text{text,..} if text=="One")));
    let style = Style {
        padding: -1.,
        ..Default::default()
    };
    dom.set_style(dom.root(), style).unwrap();
    assert!(matches!(
        engine.compute(&dom, 300., 200., &mut metrics),
        Err(LayoutError::InvalidStyle(0))
    ));
}
