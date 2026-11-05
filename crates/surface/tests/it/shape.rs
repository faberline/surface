use surface::*;

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

/// A window with one button child. The window paints nothing.
fn scene(style: Style) -> (MicroDom, NodeId, ComputedDom) {
    let mut dom = MicroDom::new(Node::new(Role::Window, "App"));
    let id = dom
        .append(
            dom.root(),
            Node::new(Role::Button, "Target").id("target").styled(style),
        )
        .unwrap();
    let computed = LayoutEngine::default()
        .compute(&dom, 400., 300., &mut Metrics)
        .unwrap();
    (dom, id, computed)
}
fn sized(width: f32, height: f32) -> Style {
    Style::default()
        .width(Length::Px(width))
        .height(Length::Px(height))
}
const FILL: Color = Color(103, 80, 164, 255);

#[test]
fn corners_constructors_place_radii_on_the_named_corners() {
    assert_eq!(
        Corners::ZERO,
        Corners {
            top_left: 0.,
            top_right: 0.,
            bottom_right: 0.,
            bottom_left: 0.
        }
    );
    assert_eq!(Corners::default(), Corners::ZERO);
    assert_eq!(Corners::FULL_RADIUS, f32::MAX);
    assert_eq!(Corners::all(4.), Corners::new(4., 4., 4., 4.));
    let c = Corners::new(1., 2., 3., 4.);
    assert_eq!(
        (c.top_left, c.top_right, c.bottom_right, c.bottom_left),
        (1., 2., 3., 4.)
    );
    assert_eq!(Corners::top(5.), Corners::new(5., 5., 0., 0.));
    assert_eq!(Corners::bottom(5.), Corners::new(0., 0., 5., 5.));
    assert_eq!(Corners::start(5.), Corners::new(5., 0., 0., 5.));
    assert_eq!(Corners::end(5.), Corners::new(0., 5., 5., 0.));
    assert_eq!(Corners::full(), Corners::all(f32::MAX));
}

#[test]
fn uniform_detection() {
    assert_eq!(Corners::all(4.).uniform(), Some(4.));
    assert_eq!(Corners::ZERO.uniform(), Some(0.));
    assert_eq!(Corners::top(4.).uniform(), None);
    assert_eq!(Corners::new(1., 1., 1., 2.).uniform(), None);
    assert!(Corners::all(4.).is_uniform());
    assert!(Corners::full().is_uniform());
    assert!(!Corners::top(4.).is_uniform());
}

#[test]
fn resolve_clamps_each_corner_to_half_the_short_side() {
    assert_eq!(Corners::full().resolve(100., 40.), Corners::all(20.));
    assert_eq!(
        Corners::new(0., 30., 5., 100.).resolve(100., 40.),
        Corners::new(0., 20., 5., 20.)
    );
    assert_eq!(Corners::all(8.).resolve(0., 10.), Corners::ZERO);
    assert_eq!(Corners::all(8.).resolve(-5., 10.), Corners::ZERO);
    assert_eq!(Corners::all(8.).resolve(100., 40.), Corners::all(8.));
}

#[test]
fn corner_validity() {
    assert!(Corners::ZERO.is_valid());
    assert!(Corners::full().is_valid());
    assert!(Corners::top(12.).is_valid());
    assert!(!Corners::all(f32::NAN).is_valid());
    assert!(!Corners::new(1., f32::NAN, 1., 1.).is_valid());
    assert!(!Corners::new(0., 0., -1., 0.).is_valid());
    assert!(!Corners::all(f32::INFINITY).is_valid());
}

#[test]
fn style_json_omits_corners_by_default_and_round_trips_full() {
    let json = serde_json::to_value(Style::default()).unwrap();
    assert!(json.get("corners").is_none(), "{json}");
    let styled = Style::default().corners(Corners::full());
    assert_eq!(styled.corners, Some(Corners::full()));
    let text = serde_json::to_string(&styled).unwrap();
    assert!(text.contains("\"corners\""), "{text}");
    let back: Style = serde_json::from_str(&text).unwrap();
    assert_eq!(back, styled);
    let old: Style = serde_json::from_str("{}").unwrap();
    assert_eq!(old.corners, None);
}

#[test]
fn corner_radii_falls_back_to_legacy_radius() {
    assert_eq!(Style::default().corner_radii(), Corners::ZERO);
    assert_eq!(Style::default().radius(8.).corner_radii(), Corners::all(8.));
    assert_eq!(
        Style::default()
            .radius(8.)
            .corners(Corners::top(3.))
            .corner_radii(),
        Corners::top(3.),
        "corners win over the legacy radius"
    );
}

#[test]
fn layout_rejects_invalid_corners() {
    for corners in [
        Corners::all(f32::NAN),
        Corners::new(0., 0., -1., 0.),
        Corners::all(f32::INFINITY),
    ] {
        let dom =
            MicroDom::new(Node::new(Role::Window, "App").styled(Style::default().corners(corners)));
        assert!(
            matches!(
                LayoutEngine::default().compute(&dom, 100., 100., &mut Metrics),
                Err(LayoutError::InvalidStyle(_))
            ),
            "{corners:?}"
        );
    }
    let ok = MicroDom::new(
        Node::new(Role::Window, "App").styled(Style::default().corners(Corners::full())),
    );
    assert!(LayoutEngine::default()
        .compute(&ok, 100., 100., &mut Metrics)
        .is_ok());
}

#[test]
fn paint_uses_quad_for_uniform_and_shape_for_asymmetric_corners() {
    let rect = |w, h| Rect::new(0., 0., w, h);
    let clip = Rect::new(0., 0., 400., 300.);

    // Fully rounded resolves to half of the short side and stays a Quad.
    let (dom, _, computed) = scene(sized(100., 40.).background(FILL).corners(Corners::full()));
    assert_eq!(
        DisplayList::compile(&dom, &computed, None).ops,
        vec![PaintOp::Quad {
            rect: rect(100., 40.),
            clip,
            radius: 20.,
            color: FILL
        }]
    );

    // Top corners only.
    let (dom, _, computed) = scene(sized(200., 56.).background(FILL).corners(Corners::top(12.)));
    assert_eq!(
        DisplayList::compile(&dom, &computed, None).ops,
        vec![PaintOp::Shape {
            rect: rect(200., 56.),
            clip,
            corners: Corners::new(12., 12., 0., 0.),
            color: FILL
        }]
    );

    // Each corner is clamped on its own.
    let (dom, _, computed) = scene(
        sized(100., 40.)
            .background(FILL)
            .corners(Corners::new(0., 30., 5., 100.)),
    );
    assert_eq!(
        DisplayList::compile(&dom, &computed, None).ops,
        vec![PaintOp::Shape {
            rect: rect(100., 40.),
            clip,
            corners: Corners::new(0., 20., 5., 20.),
            color: FILL
        }]
    );

    // The border follows the same asymmetric shape.
    let border = Color::rgb(10, 20, 30);
    let (dom, _, computed) = scene(
        sized(200., 56.)
            .background(FILL)
            .border(2., border)
            .corners(Corners::top(12.)),
    );
    assert_eq!(
        DisplayList::compile(&dom, &computed, None).ops,
        vec![
            PaintOp::Shape {
                rect: rect(200., 56.),
                clip,
                corners: Corners::new(12., 12., 0., 0.),
                color: FILL
            },
            PaintOp::ShapeStroke {
                rect: rect(200., 56.),
                clip,
                corners: Corners::new(12., 12., 0., 0.),
                width: 2.,
                color: border
            }
        ]
    );

    // A uniform border keeps the legacy Stroke.
    let (dom, _, computed) = scene(
        sized(100., 40.)
            .background(FILL)
            .border(1., border)
            .corners(Corners::all(6.)),
    );
    assert_eq!(
        DisplayList::compile(&dom, &computed, None).ops[1],
        PaintOp::Stroke {
            rect: rect(100., 40.),
            clip,
            radius: 6.,
            width: 1.,
            color: border
        }
    );

    // The legacy radius path is unchanged.
    let (dom, _, computed) = scene(sized(100., 40.).background(FILL).radius(8.));
    assert_eq!(
        DisplayList::compile(&dom, &computed, None).ops,
        vec![PaintOp::Quad {
            rect: rect(100., 40.),
            clip,
            radius: 8.,
            color: FILL
        }]
    );

    // Legacy output is not resolved: an oversized radius is passed through.
    let (dom, _, computed) = scene(sized(100., 40.).background(FILL).radius(500.));
    assert_eq!(
        DisplayList::compile(&dom, &computed, None).ops,
        vec![PaintOp::Quad {
            rect: rect(100., 40.),
            clip,
            radius: 500.,
            color: FILL
        }]
    );
}

#[test]
fn state_layer_follows_corners() {
    let layer = Color(10, 20, 30, 200);
    let (dom, id, computed) = scene(
        sized(100., 40.)
            .state_layer(layer)
            .corners(Corners::top(10.)),
    );
    let list = DisplayList::compile_with_interaction(
        &dom,
        &computed,
        None,
        &Default::default(),
        InteractionState {
            hovered: Some(id),
            pressed: None,
        },
    );
    assert_eq!(
        list.ops,
        vec![PaintOp::Shape {
            rect: Rect::new(0., 0., 100., 40.),
            clip: Rect::new(0., 0., 400., 300.),
            corners: Corners::new(10., 10., 0., 0.),
            color: Color(10, 20, 30, (200u16 * 20 / 255) as u8)
        }]
    );
}

#[test]
fn focus_ring_keeps_legacy_quads_and_follows_corners() {
    let foreground = Color::rgb(1, 2, 3);
    let clip = Rect::new(0., 0., 400., 300.);

    // A legacy node keeps the four 2px edge quads from before shapes existed.
    let (dom, id, computed) = scene(sized(100., 40.).background(FILL).foreground(foreground));
    let r = computed.nodes[id.index()].bounds;
    let ops = DisplayList::compile(&dom, &computed, Some(id)).ops;
    let edge = |rect| PaintOp::Quad {
        rect,
        clip,
        radius: 0.,
        color: foreground,
    };
    assert_eq!(
        ops,
        vec![
            PaintOp::Quad {
                rect: r,
                clip,
                radius: 0.,
                color: FILL
            },
            edge(Rect::new(r.x, r.y, r.width, 2.)),
            edge(Rect::new(r.x, r.y + r.height - 2., r.width, 2.)),
            edge(Rect::new(r.x, r.y, 2., r.height)),
            edge(Rect::new(r.x + r.width - 2., r.y, 2., r.height)),
        ]
    );

    let (dom, id, computed) = scene(
        sized(100., 40.)
            .foreground(foreground)
            .corners(Corners::top(12.)),
    );
    let bounds = computed.nodes[id.index()].bounds;
    assert_eq!(
        DisplayList::compile(&dom, &computed, Some(id)).ops,
        vec![PaintOp::ShapeStroke {
            rect: bounds,
            clip,
            corners: Corners::new(12., 12., 0., 0.),
            width: 2.,
            color: foreground
        }]
    );

    // A uniform resolved shape uses the plain Stroke with the resolved radius.
    let (dom, id, computed) = scene(
        sized(100., 40.)
            .foreground(foreground)
            .corners(Corners::full()),
    );
    let bounds = computed.nodes[id.index()].bounds;
    assert_eq!(
        DisplayList::compile(&dom, &computed, Some(id)).ops,
        vec![PaintOp::Stroke {
            rect: bounds,
            clip,
            radius: 20.,
            width: 2.,
            color: foreground
        }]
    );
}

#[test]
fn children_keep_rectangular_parent_clip() {
    // Rounded clip masks are a known gap: descendants are clipped to the
    // parent rectangle, not to its rounded shape.
    let mut dom = MicroDom::new(Node::new(Role::Window, "App"));
    let parent = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "Clip").styled(
                sized(100., 40.)
                    .clip(true)
                    .background(FILL)
                    .corners(Corners::full()),
            ),
        )
        .unwrap();
    let child = dom
        .append(
            parent,
            Node::new(Role::Group, "Child").styled(sized(50., 20.).background(Color::rgb(9, 9, 9))),
        )
        .unwrap();
    let computed = LayoutEngine::default()
        .compute(&dom, 400., 300., &mut Metrics)
        .unwrap();
    let parent_bounds = computed.nodes[parent.index()].bounds;
    assert_eq!(computed.nodes[child.index()].clip, parent_bounds);
    let list = DisplayList::compile(&dom, &computed, None);
    let child_bounds = computed.nodes[child.index()].bounds;
    assert!(list.ops.iter().any(|op| matches!(op,
        PaintOp::Quad { rect, clip, .. } if *rect == child_bounds && *clip == parent_bounds)));
}
