use surface::*;

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

fn fixture() -> (MicroDom, NodeId, NodeId) {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "App").styled(Style::column().padding(10.).gap(10.).clip(true)),
    );
    let mut add = |id: &str, text: &str| {
        dom.append(
            dom.root(),
            Node::new(Role::Button, text)
                .id(id)
                .text(text)
                .styled(Style::default().padding(4.)),
        )
        .unwrap()
    };
    let first = add("first", "One");
    let second = add("second", "Two");
    (dom, first, second)
}

#[test]
fn bounds_and_clips_are_deterministic() {
    let (dom, first, second) = fixture();
    let a = LayoutEngine::default()
        .compute(&dom, 300., 200., &mut Metrics)
        .unwrap();
    let b = LayoutEngine::default()
        .compute(&dom, 300., 200., &mut Metrics)
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(a.nodes.len(), dom.len());
    let root = a.nodes[dom.root().index()].bounds;
    assert_eq!(root, Rect::new(0., 0., 300., 200.));
    let clip = Rect::new(0., 0., 300., 200.);
    for id in [first, second] {
        let n = &a.nodes[id.index()];
        assert_eq!(n.clip, clip);
        assert!(n.bounds.x >= root.x && n.bounds.y >= root.y);
        assert!(n.bounds.x + n.bounds.width <= root.x + root.width);
        assert!(n.bounds.y + n.bounds.height <= root.y + root.height);
    }
}

#[test]
fn changed_since_lists_only_the_changed_subtree() {
    let (mut dom, first, second) = fixture();
    let mut engine = LayoutEngine::default();
    let a = engine.compute(&dom, 300., 200., &mut Metrics).unwrap();
    dom.set_style(second, Style::default().padding(4.).height(Length::Px(80.)))
        .unwrap();
    let b = engine.compute(&dom, 300., 200., &mut Metrics).unwrap();
    let changed = b.changed_since(&a);
    assert!(changed.contains(&second));
    assert!(!changed.contains(&first));
    assert!(!changed.contains(&dom.root()));
    assert!(b.changed_since(&b).is_empty());
}

#[test]
fn changed_since_reports_new_nodes() {
    let (mut dom, _, _) = fixture();
    let mut engine = LayoutEngine::default();
    let a = engine.compute(&dom, 300., 200., &mut Metrics).unwrap();
    let third = dom
        .append(
            dom.root(),
            Node::new(Role::Button, "Three")
                .id("third")
                .text("Three")
                .styled(Style::default().padding(4.)),
        )
        .unwrap();
    let b = engine.compute(&dom, 300., 200., &mut Metrics).unwrap();
    assert!(b.changed_since(&a).contains(&third));
}

#[test]
fn topmost_overlapping_layer_wins_hit_test() {
    for (z_front, z_back, expected_front) in [(2, 1, "second"), (1, 2, "first")] {
        let mut dom = MicroDom::new(
            Node::new(Role::Window, "App").styled(
                Style::default()
                    .width(Length::Px(200.))
                    .height(Length::Px(200.)),
            ),
        );
        let inset = Insets {
            left: Length::Px(10.),
            top: Length::Px(10.),
            ..Default::default()
        };
        let mut first_style = Style::default()
            .width(Length::Px(50.))
            .height(Length::Px(50.))
            .fixed(inset);
        first_style.z_index = z_back;
        let mut second_style = Style::default()
            .width(Length::Px(50.))
            .height(Length::Px(50.))
            .fixed(inset);
        second_style.z_index = z_front;
        let first = dom
            .append(
                dom.root(),
                Node::new(Role::Button, "First")
                    .id("first")
                    .styled(first_style),
            )
            .unwrap();
        let second = dom
            .append(
                dom.root(),
                Node::new(Role::Button, "Second")
                    .id("second")
                    .styled(second_style),
            )
            .unwrap();
        let computed = LayoutEngine::default()
            .compute(&dom, 200., 200., &mut Metrics)
            .unwrap();
        let hit = computed.hit_test(&dom, 20., 20.);
        let expected = if expected_front == "second" {
            second
        } else {
            first
        };
        assert_eq!(hit, Some(expected));
    }
}

#[test]
fn clipped_descendant_is_not_hit() {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "App").styled(
            Style::default()
                .width(Length::Px(100.))
                .height(Length::Px(100.)),
        ),
    );
    let group = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "Clip").styled(
                Style::default()
                    .width(Length::Px(40.))
                    .height(Length::Px(40.))
                    .clip(true),
            ),
        )
        .unwrap();
    let button = dom
        .append(
            group,
            Node::new(Role::Button, "Tall").id("tall").styled(
                Style::default()
                    .width(Length::Px(40.))
                    .height(Length::Px(120.)),
            ),
        )
        .unwrap();
    let computed = LayoutEngine::default()
        .compute(&dom, 100., 100., &mut Metrics)
        .unwrap();
    assert_eq!(computed.hit_test(&dom, 10., 20.), Some(button));
    assert_eq!(computed.hit_test(&dom, 10., 60.), None);
}

#[test]
fn scroll_offset_moves_hit_target() {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "App").styled(
            Style::column()
                .width(Length::Px(100.))
                .height(Length::Px(50.))
                .clip(true),
        ),
    );
    dom.append(
        dom.root(),
        Node::new(Role::Group, "Spacer").styled(
            Style::default()
                .width(Length::Px(100.))
                .height(Length::Px(40.)),
        ),
    )
    .unwrap();
    let button = dom
        .append(
            dom.root(),
            Node::new(Role::Button, "Target").id("target").styled(
                Style::default()
                    .width(Length::Px(100.))
                    .height(Length::Px(30.)),
            ),
        )
        .unwrap();
    let mut engine = LayoutEngine::default();
    let before = engine.compute(&dom, 100., 50., &mut Metrics).unwrap();
    assert_eq!(before.nodes[button.index()].bounds.y, 40.);
    assert_eq!(before.hit_test(&dom, 5., 15.), None);

    let mut root = dom.get(dom.root()).unwrap().style().clone();
    root.scroll_y = 30.;
    dom.set_style(dom.root(), root).unwrap();
    let after = engine.compute(&dom, 100., 50., &mut Metrics).unwrap();
    assert_eq!(after.nodes[button.index()].bounds.y, 10.);
    assert_eq!(after.hit_test(&dom, 5., 15.), Some(button));
}

#[test]
fn disabled_and_zero_size_nodes_are_not_hit() {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "App").styled(
            Style::column()
                .width(Length::Px(200.))
                .height(Length::Px(200.)),
        ),
    );
    let disabled = dom
        .append(
            dom.root(),
            Node::new(Role::Button, "Off")
                .id("off")
                .disabled(true)
                .styled(
                    Style::default()
                        .width(Length::Px(50.))
                        .height(Length::Px(50.)),
                ),
        )
        .unwrap();
    let zero = dom
        .append(
            dom.root(),
            Node::new(Role::Button, "Zero").id("zero").styled(
                Style::default()
                    .width(Length::Px(0.))
                    .height(Length::Px(50.)),
            ),
        )
        .unwrap();
    let computed = LayoutEngine::default()
        .compute(&dom, 200., 200., &mut Metrics)
        .unwrap();
    let d = computed.nodes[disabled.index()].bounds;
    assert_eq!(
        computed.hit_test(&dom, d.x + d.width / 2., d.y + d.height / 2.),
        None
    );
    let z = computed.nodes[zero.index()].bounds;
    assert_eq!(computed.hit_test(&dom, z.x, z.y), None);
}

#[test]
fn containers_do_not_steal_input() {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "App").styled(
            Style::default()
                .width(Length::Px(200.))
                .height(Length::Px(200.)),
        ),
    );
    let group = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "Panel").styled(
                Style::default()
                    .width(Length::Px(100.))
                    .height(Length::Px(100.))
                    .padding(20.),
            ),
        )
        .unwrap();
    let child = dom
        .append(
            group,
            Node::new(Role::Button, "Child").id("child").styled(
                Style::default()
                    .width(Length::Px(20.))
                    .height(Length::Px(20.)),
            ),
        )
        .unwrap();
    let computed = LayoutEngine::default()
        .compute(&dom, 200., 200., &mut Metrics)
        .unwrap();
    let c = computed.nodes[child.index()].bounds;
    assert_eq!(computed.hit_test(&dom, c.x + 1., c.y + 1.), Some(child));
    let g = computed.nodes[group.index()].bounds;
    // Inside the group, outside the child (the group padding area).
    assert_eq!(computed.hit_test(&dom, g.x + 2., g.y + 2.), None);
}

#[test]
fn invalid_viewport_and_style_return_errors() {
    let (mut dom, _, _) = fixture();
    let mut engine = LayoutEngine::default();
    assert!(matches!(
        engine.compute(&dom, 0., 100., &mut Metrics),
        Err(LayoutError::InvalidViewport)
    ));
    assert!(matches!(
        engine.compute(&dom, f32::NAN, 100., &mut Metrics),
        Err(LayoutError::InvalidViewport)
    ));
    dom.set_style(
        dom.root(),
        Style {
            padding: -1.,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        engine.compute(&dom, 300., 200., &mut Metrics),
        Err(LayoutError::InvalidStyle(0))
    ));
}
