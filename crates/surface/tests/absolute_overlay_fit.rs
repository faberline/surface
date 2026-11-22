use std::collections::BTreeMap;
use surface::{
    scroll_extent, BoxShadow, Color, ComputedDom, DisplayList, Insets, InteractionState,
    LayoutEngine, Length, MicroDom, Node, NodeId, OverlayBoundary, OverlayHorizontalEdge, Rect,
    Role, Style, TextMeasurer,
};

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

fn dom() -> MicroDom {
    MicroDom::new(
        Node::new(Role::Window, "Test").styled(
            Style::column()
                .width(Length::Percent(1.))
                .height(Length::Percent(1.)),
        ),
    )
}

fn size(width: f32, height: f32) -> Style {
    Style::column()
        .width(Length::Px(width))
        .height(Length::Px(height))
}

fn absolute(x: f32, y: f32, width: f32, height: f32) -> Style {
    size(width, height).absolute(Insets {
        left: Length::Px(x),
        top: Length::Px(y),
        ..Insets::default()
    })
}

fn add(dom: &mut MicroDom, parent: NodeId, node: Node) -> NodeId {
    dom.append(parent, node).unwrap()
}

fn anchor(dom: &mut MicroDom, parent: NodeId, style: Style) -> NodeId {
    add(
        dom,
        parent,
        Node::new(Role::Button, "Anchor").id("owner").styled(style),
    )
}

fn overlay(
    dom: &mut MicroDom,
    parent: NodeId,
    owner: &str,
    style: Style,
    edge: OverlayHorizontalEdge,
    boundary: OverlayBoundary,
) -> NodeId {
    add(
        dom,
        parent,
        Node::new(Role::Group, "")
            .fit_absolute_overlay(owner, edge, boundary)
            .styled(style),
    )
}

fn paint_owners(dom: &MicroDom, computed: &ComputedDom) -> Vec<NodeId> {
    DisplayList::compile_attributed(
        dom,
        computed,
        None,
        &BTreeMap::new(),
        InteractionState::default(),
    )
    .1
}

fn bounds(computed: &ComputedDom, id: NodeId) -> Rect {
    computed.nodes[id.index()].bounds
}

#[test]
fn fits_the_selected_physical_edge_and_top_without_moving_the_owner() {
    use OverlayHorizontalEdge::{Left, Right};
    for (edge, desired_x, fitted_x) in [(Right, 92., 80.), (Left, -7., 0.)] {
        let mut dom = dom();
        let root = dom.root();
        let owner = anchor(&mut dom, root, size(100., 35.));
        let decoration = overlay(
            &mut dom,
            root,
            "owner",
            absolute(desired_x, -5., 20., 12.).clip(true),
            edge,
            OverlayBoundary::Clip,
        );
        let leaf = add(
            &mut dom,
            decoration,
            Node::new(Role::Group, "")
                .styled(absolute(3., 2., 5., 4.).background(Color::rgb(200, 10, 0))),
        );
        let computed = LayoutEngine::default()
            .compute(&dom, 100., 100., &mut Metrics)
            .unwrap();
        assert_eq!(bounds(&computed, owner), Rect::new(0., 0., 100., 35.));
        assert_eq!(
            bounds(&computed, decoration),
            Rect::new(fitted_x, 0., 20., 12.)
        );
        assert_eq!(
            bounds(&computed, leaf),
            Rect::new(fitted_x + 3., 2., 5., 4.)
        );
        assert_eq!(
            computed.nodes[leaf.index()].clip,
            bounds(&computed, decoration)
        );
        assert_eq!(computed.hit_test(&dom, fitted_x + 4., 4.), Some(owner));
        assert_eq!(paint_owners(&dom, &computed), [leaf]);
    }
}

#[test]
fn clips_to_the_intersection_of_nested_clips_and_viewport_on_every_resize() {
    let mut dom = dom();
    let root = dom.root();
    let outer = add(
        &mut dom,
        root,
        Node::new(Role::Group, "").styled(absolute(20., 10., 85., 70.).clip(true)),
    );
    let inner = add(
        &mut dom,
        outer,
        Node::new(Role::Group, "").styled(absolute(15., 12., 85., 55.).clip(true)),
    );
    let owner = anchor(&mut dom, inner, size(90., 30.));
    let decoration = overlay(
        &mut dom,
        owner,
        "owner",
        absolute(77., -4., 28., 18.),
        OverlayHorizontalEdge::Right,
        OverlayBoundary::Clip,
    );
    let mut engine = LayoutEngine::default();
    for (viewport_width, clip_width) in [(100., 65.), (150., 70.), (90., 55.), (100., 65.)] {
        let computed = engine
            .compute(&dom, viewport_width, 100., &mut Metrics)
            .unwrap();
        assert_eq!(bounds(&computed, owner), Rect::new(35., 22., 90., 30.));
        assert_eq!(
            computed.nodes[decoration.index()].clip,
            Rect::new(35., 22., clip_width, 55.)
        );
        assert_eq!(
            bounds(&computed, decoration),
            Rect::new(35. + clip_width - 28., 22., 28., 18.)
        );
    }
}

#[test]
fn uses_the_owner_on_each_axis_where_the_measured_overlay_fits() {
    use OverlayBoundary::{Clip, Owner};
    use OverlayHorizontalEdge::{Left, Right};

    for (edge, boundary, desired_x, width, height, x, y) in [
        (Right, Clip, 34., 24., 12., 84., 43.),
        (Right, Owner, 34., 24., 12., 66., 50.),
        (Right, Owner, 34., 60., 12., 84., 50.),
        (Right, Owner, 34., 24., 40., 66., 43.),
        (Right, Owner, 34., 40., 30., 50., 50.),
        (Right, Owner, 34., 220., 160., -20., 43.),
        (Left, Owner, -8., 24., 12., 50., 50.),
    ] {
        let mut dom = dom();
        let root = dom.root();
        let owner = anchor(&mut dom, root, absolute(50., 50., 40., 30.));
        let decoration = overlay(
            &mut dom,
            owner,
            "owner",
            absolute(desired_x, -7., width, height),
            edge,
            boundary,
        );
        let computed = LayoutEngine::default()
            .compute(&dom, 200., 150., &mut Metrics)
            .unwrap();
        assert_eq!(
            bounds(&computed, decoration),
            Rect::new(x, y, width, height)
        );
        assert_eq!(computed.nodes[decoration.index()].clip, computed.viewport);
    }
}

#[test]
fn all_paint_follows_the_owners_visibility_including_shadows_and_fixed_descendants() {
    let mut dom = dom();
    let root = dom.root();
    let scroller = add(
        &mut dom,
        root,
        Node::new(Role::Group, "").styled(absolute(12., 8., 110., 55.).clip(true)),
    );
    let owner = anchor(&mut dom, scroller, size(50., 20.));
    add(
        &mut dom,
        scroller,
        Node::new(Role::Group, "").styled(size(50., 50.)),
    );

    let decoration = add(
        &mut dom,
        owner,
        Node::new(Role::Group, "")
            .styled(absolute(40., -4., 16., 12.))
            .box_shadows([
                BoxShadow::new(0., 24., 3., 0., Color::rgb(20, 20, 20)),
                BoxShadow::NONE,
            ])
            .fit_absolute_overlay("owner", OverlayHorizontalEdge::Right, OverlayBoundary::Clip),
    );
    let label = add(
        &mut dom,
        decoration,
        Node::new(Role::Group, "").text("9").styled(size(10., 10.)),
    );
    let fixed = add(
        &mut dom,
        label,
        Node::new(Role::Group, "").styled(
            size(10., 10.)
                .fixed(Insets::all(2.))
                .background(Color::rgb(200, 10, 0)),
        ),
    );
    let mut engine = LayoutEngine::default();
    let mut style = dom.get(scroller).unwrap().style().clone();
    for (offset, visible) in [(0., true), (12., true), (20., false), (0., true)] {
        style.scroll_y = offset;
        dom.set_style(scroller, style.clone()).unwrap();
        let computed = engine.compute(&dom, 150., 100., &mut Metrics).unwrap();
        let owners = paint_owners(&dom, &computed);
        for part in [decoration, label, fixed] {
            assert_eq!(owners.contains(&part), visible);
            if !visible {
                assert_eq!(computed.nodes[part.index()].clip, Rect::default());
            }
        }
        assert_eq!(scroll_extent(&dom, &computed, scroller), Some((110., 70.)));
    }
}

#[test]
fn an_absolute_overlay_is_not_measured_as_scrollable_content() {
    let mut dom = dom();
    let root = dom.root();
    let scroller = add(
        &mut dom,
        root,
        Node::new(Role::Group, "").styled(size(100., 50.).clip(true)),
    );
    anchor(&mut dom, scroller, size(10., 10.));
    add(
        &mut dom,
        scroller,
        Node::new(Role::Group, "").styled(absolute(5., 60., 20., 12.)),
    );
    overlay(
        &mut dom,
        scroller,
        "owner",
        absolute(5., 200., 20., 12.),
        OverlayHorizontalEdge::Right,
        OverlayBoundary::Clip,
    );

    let computed = LayoutEngine::default()
        .compute(&dom, 150., 100., &mut Metrics)
        .unwrap();
    assert_eq!(scroll_extent(&dom, &computed, scroller), Some((100., 72.)));
}

#[test]
fn missing_and_later_owner_references_are_hidden_and_flow_nodes_ignore_the_opt_in() {
    let mut dom = dom();
    let root = dom.root();
    let ids = ["missing", "owner"].map(|owner| {
        overlay(
            &mut dom,
            root,
            owner,
            absolute(4., 4., 40., 20.).background(Color::rgb(200, 10, 0)),
            OverlayHorizontalEdge::Right,
            OverlayBoundary::Clip,
        )
    });
    let owner = anchor(&mut dom, root, size(100., 20.));
    let flow = overlay(
        &mut dom,
        root,
        "missing",
        size(40., 20.).background(Color::rgb(200, 10, 0)),
        OverlayHorizontalEdge::Right,
        OverlayBoundary::Clip,
    );
    let computed = LayoutEngine::default()
        .compute(&dom, 100., 100., &mut Metrics)
        .unwrap();
    for id in ids {
        assert_eq!(computed.nodes[id.index()].clip, Rect::default());
    }
    assert_eq!(computed.nodes[flow.index()].clip, computed.viewport);
    assert_eq!(bounds(&computed, flow).y, 20.);
    assert_eq!(computed.hit_test(&dom, 5., 5.), Some(owner));
    assert_eq!(paint_owners(&dom, &computed), [flow]);
}

#[test]
fn the_layout_option_stays_off_the_dom_wire() {
    let mut dom = dom();
    let root = dom.root();
    anchor(&mut dom, root, size(100., 40.));
    let decoration = overlay(
        &mut dom,
        root,
        "owner",
        absolute(95., -5., 20., 12.),
        OverlayHorizontalEdge::Right,
        OverlayBoundary::Clip,
    );
    let snapshot = serde_json::to_value(dom.snapshot()).unwrap();
    assert_eq!(snapshot["version"], 1);
    assert!(snapshot["nodes"][decoration.index()]
        .get("absolute_overlay_fit")
        .is_none());
    let restored =
        MicroDom::from_snapshot(serde_json::from_value(snapshot.clone()).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(restored.snapshot()).unwrap(), snapshot);
    let mut engine = LayoutEngine::default();
    let authored = engine.compute(&dom, 100., 100., &mut Metrics).unwrap();
    let restored = engine.compute(&restored, 100., 100., &mut Metrics).unwrap();
    assert_eq!(bounds(&authored, decoration), Rect::new(80., 0., 20., 12.));
    assert_eq!(bounds(&restored, decoration), Rect::new(95., -5., 20., 12.));
}

#[test]
fn an_owner_in_a_separate_branch_or_fixed_layer_cannot_bring_a_decoration_into_view() {
    let mut dom = dom();
    let root = dom.root();
    let branch = add(
        &mut dom,
        root,
        Node::new(Role::Group, "").styled(absolute(0., 0., 60., 40.)),
    );
    anchor(&mut dom, branch, size(60., 40.));
    let unrelated = overlay(
        &mut dom,
        root,
        "owner",
        absolute(0., 0., 20., 12.).background(Color::rgb(200, 10, 0)),
        OverlayHorizontalEdge::Right,
        OverlayBoundary::Clip,
    );
    let outside = add(
        &mut dom,
        root,
        Node::new(Role::Button, "Outside")
            .id("outside")
            .styled(size(60., 40.)),
    );
    let layer = add(
        &mut dom,
        root,
        Node::new(Role::Group, "").styled(size(60., 60.).fixed(Insets::all(0.))),
    );
    let cross_layer = overlay(
        &mut dom,
        layer,
        "outside",
        absolute(8., 8., 20., 12.).background(Color::rgb(200, 10, 0)),
        OverlayHorizontalEdge::Right,
        OverlayBoundary::Clip,
    );
    let inert_subtree = add(
        &mut dom,
        cross_layer,
        Node::new(Role::Button, "Invalid").styled(size(20., 12.).fixed(Insets::all(0.))),
    );
    let computed = LayoutEngine::default()
        .compute(&dom, 100., 100., &mut Metrics)
        .unwrap();
    for id in [unrelated, cross_layer, inert_subtree] {
        assert_eq!(computed.nodes[id.index()].clip, Rect::default());
    }
    assert_eq!(computed.hit_test(&dom, 10., 10.), Some(outside));
    assert!(paint_owners(&dom, &computed).is_empty());
}

#[test]
fn an_anchor_can_precede_the_lane_that_contains_its_absolute_decoration() {
    let mut dom = dom();
    let root = dom.root();
    let row = add(
        &mut dom,
        root,
        Node::new(Role::Group, "").styled(Style::row()),
    );
    let owner = anchor(&mut dom, row, size(90., 34.));
    let lane = add(
        &mut dom,
        row,
        Node::new(Role::Group, "").styled(size(0., 0.)),
    );
    let decoration = overlay(
        &mut dom,
        lane,
        "owner",
        absolute(0., -5., 20., 12.),
        OverlayHorizontalEdge::Right,
        OverlayBoundary::Clip,
    );
    let computed = LayoutEngine::default()
        .compute(&dom, 100., 100., &mut Metrics)
        .unwrap();
    assert_eq!(bounds(&computed, owner), Rect::new(0., 0., 90., 34.));
    assert_eq!(bounds(&computed, lane).width, 0.);
    assert_eq!(bounds(&computed, decoration), Rect::new(80., 0., 20., 12.));
}
