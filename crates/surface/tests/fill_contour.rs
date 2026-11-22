use std::collections::BTreeMap;

use surface::{
    Align, Color, DisplayList, DomError, DomSnapshot, ExtendedDisplayList, ExtendedPaintOp,
    FillContour, Insets, InteractionState, LayoutEngine, Length, MicroDom, Node, PaintOp, PathFill,
    Rect, Role, Style, TextMeasurer,
};

const INK: Color = Color(52, 88, 184, 96);
const VIEWPORT: Rect = Rect {
    x: 0.,
    y: 0.,
    width: 80.,
    height: 60.,
};
const NOTCHED: [[f32; 2]; 8] = [
    [3., 2.],
    [37., 2.],
    [37., 16.],
    [24., 16.],
    [24., 9.],
    [13., 9.],
    [13., 16.],
    [3., 16.],
];

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn layout(dom: &MicroDom) -> surface::ComputedDom {
    LayoutEngine::default()
        .compute(dom, VIEWPORT.width, VIEWPORT.height, &mut Metrics)
        .unwrap()
}

fn scene(contour: Option<FillContour>) -> MicroDom {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "Window").styled(Style::column().align(Align::Start).padding(10.)),
    );
    let clip = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "").styled(
                Style::column()
                    .align(Align::Start)
                    .width(Length::Px(30.))
                    .height(Length::Px(16.))
                    .clip(true),
            ),
        )
        .unwrap();
    let mut owner = Node::new(Role::Progress, "Loading").id("loading").styled(
        Style::default()
            .width(Length::Px(52.))
            .height(Length::Px(16.))
            .background(Color::rgb(255, 255, 255))
            .border(1., Color::rgb(0, 0, 0)),
    );
    if let Some(contour) = contour {
        owner = owner.fill_contour(contour);
    }
    dom.append(clip, owner).unwrap();
    dom.append(
        dom.root(),
        Node::new(Role::Group, "").id("overlay").styled(
            Style::default()
                .width(Length::Px(4.))
                .height(Length::Px(4.))
                .background(Color::rgb(255, 128, 0))
                .absolute(Insets {
                    top: Length::Px(18.),
                    left: Length::Px(25.),
                    ..Default::default()
                }),
        ),
    )
    .unwrap();
    dom
}

#[test]
fn constructors_and_decoders_validate_and_bound_the_contour() {
    let accepted = [[2., 3.], [2., 3.], [10., 3.], [5., 8.], [2., 3.]];
    let fill = PathFill::new(accepted, INK, VIEWPORT).unwrap();
    assert_eq!(fill.points(), &[[2., 3.], [10., 3.], [5., 8.]]);
    assert_eq!(fill.rect(), Rect::new(2., 3., 8., 5.));
    assert_eq!(fill.clip(), VIEWPORT);
    assert_eq!(fill.color(), INK);

    assert!(FillContour::new([[0., 0.], [1., 1.], [4., 4.]], INK).is_none());
    assert!(FillContour::new([[0., 0.], [1., 0.], [0., 0.]], INK).is_none());
    assert!(FillContour::new([[f32::NAN, 0.], [2., 0.], [0., 4.]], INK).is_none());
    assert!(FillContour::new([[f32::INFINITY, 0.], [2., 0.], [0., 4.]], INK).is_none());
    assert!(FillContour::new([[-f32::MAX, 0.], [f32::MAX, 0.], [0., 4.]], INK).is_none());
    assert!(FillContour::new(NOTCHED.into_iter().cycle(), INK).is_none());
    assert!(FillContour::new(
        NOTCHED.into_iter().cycle().take(FillContour::MAX_POINTS),
        INK,
    )
    .is_some());
    assert!(PathFill::new(accepted, INK, Rect::default()).is_none());

    let mut json = serde_json::to_value(&fill).unwrap();
    assert_eq!(
        serde_json::from_value::<PathFill>(json.clone()).unwrap(),
        fill
    );
    let mut invalid_clip = json.clone();
    invalid_clip["clip"]["width"] = 0.into();
    assert!(serde_json::from_value::<PathFill>(invalid_clip).is_err());
    json["contour"]["points"] = serde_json::json!(vec![[0., 0.]; FillContour::MAX_POINTS + 1]);
    assert!(serde_json::from_value::<PathFill>(json).is_err());
}

#[test]
fn only_authored_fills_require_dom_version_four() {
    let owner = Node::new(Role::Progress, "Loading").id("loading");
    for (owner, version) in [
        (owner.clone(), 1),
        (
            owner.fill_contour(FillContour::new(NOTCHED, INK).unwrap()),
            4,
        ),
    ] {
        let snapshot = MicroDom::new(owner).snapshot();
        assert_eq!(snapshot.version, version);
        let json = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(json["nodes"][0].get("fill_contour").is_some(), version == 4);
        let parsed = serde_json::from_value::<DomSnapshot>(json.clone()).unwrap();
        assert_eq!(
            MicroDom::from_snapshot(parsed.clone()).unwrap().snapshot(),
            snapshot
        );
        if version == 4 {
            let mut wrong_version = parsed;
            wrong_version.version = 3;
            assert_eq!(
                MicroDom::from_snapshot(wrong_version).unwrap_err(),
                DomError::UnsupportedVersion(3)
            );
            let mut without_fill = json;
            without_fill["nodes"][0]
                .as_object_mut()
                .unwrap()
                .remove("fill_contour");
            assert_eq!(
                MicroDom::from_snapshot(serde_json::from_value(without_fill).unwrap()).unwrap_err(),
                DomError::UnsupportedVersion(4)
            );
        }
    }
}

#[test]
fn fill_keeps_input_geometry_and_its_place_in_the_attributed_paint_order() {
    let dom = scene(FillContour::new(NOTCHED, INK));
    let computed = layout(&dom);
    let plain = scene(None);
    let plain_layout = layout(&plain);
    assert_eq!(computed, plain_layout);
    assert_eq!(
        computed.hit_test(&dom, 22., 17.),
        plain_layout.hit_test(&plain, 22., 17.)
    );
    let owner = dom.by_id("loading").unwrap();
    assert_eq!(dom.children(owner).count(), 0);
    assert_eq!(dom.get(owner).unwrap().role(), Role::Progress);
    assert!(!dom.get(owner).unwrap().is_focusable());

    let (list, owners) = ExtendedDisplayList::compile_attributed(
        &dom,
        &computed,
        None,
        &BTreeMap::new(),
        InteractionState::default(),
    );
    let [ExtendedPaintOp::Legacy(PaintOp::Quad { .. }), ExtendedPaintOp::FillPath(fill), ExtendedPaintOp::Legacy(PaintOp::Stroke { .. }), ExtendedPaintOp::Legacy(PaintOp::Quad { .. })] =
        list.ops()
    else {
        panic!("expected background, fill, border and overlay");
    };
    assert_eq!(fill.rect(), Rect::new(13., 12., 34., 14.));
    assert_eq!(fill.clip(), Rect::new(10., 10., 30., 16.));
    assert_eq!(fill.points()[0], [13., 12.]);
    assert_eq!(&owners[..3], &[owner; 3]);
    assert_eq!(owners[3], dom.by_id("overlay").unwrap());
    assert_eq!(
        serde_json::from_str::<ExtendedPaintOp>(&serde_json::to_string(&list.ops()[1]).unwrap())
            .unwrap(),
        list.ops()[1]
    );
}

#[test]
fn legacy_fills_use_disjoint_coverage_and_the_even_odd_rule() {
    fn coverage(ops: &[PaintOp]) -> Vec<&PaintOp> {
        ops.iter()
            .filter(|op| {
                matches!(
                    op,
                    PaintOp::Quad {
                        color: Color(52, 88, 184, _),
                        ..
                    }
                )
            })
            .collect()
    }

    let dom = scene(FillContour::new(NOTCHED, INK));
    let (list, owners) = DisplayList::compile_attributed(
        &dom,
        &layout(&dom),
        None,
        &BTreeMap::new(),
        InteractionState::default(),
    );
    let fills = coverage(&list.ops);
    assert!(fills.iter().enumerate().all(|(index, op)| {
        op.clip() == Rect::new(13., 12., 27., 14.)
            && fills[..index].iter().all(|other| {
                let overlap = op.rect().intersect(other.rect());
                overlap.width == 0. || overlap.height == 0.
            })
    }));
    let alpha_at = |x, y| {
        fills.iter().find_map(|op| match op {
            PaintOp::Quad {
                rect, clip, color, ..
            } if rect.contains(x, y) && clip.contains(x, y) => Some(color.3),
            _ => None,
        })
    };
    assert_eq!(alpha_at(21.5, 16.5), Some(INK.3));
    assert_eq!(alpha_at(21.5, 22.5), Some(INK.3));
    assert_eq!(alpha_at(25.5, 22.5), None);
    assert_eq!(alpha_at(39.5, 22.5), Some(INK.3));
    assert_eq!(alpha_at(40.5, 22.5), None);
    let last_owner = dom.by_id("overlay").unwrap();
    assert!(owners[..owners.len() - 1]
        .iter()
        .all(|&owner| Some(owner) == dom.by_id("loading")));
    assert_eq!(owners.last(), Some(&last_owner));

    let reversed = scene(FillContour::new(NOTCHED.into_iter().rev(), INK));
    let reversed_paint = DisplayList::compile(&reversed, &layout(&reversed), None);
    assert_eq!(coverage(&reversed_paint.ops), fills);

    // Winding twice around the same perimeter cancels under even-odd filling.
    let twice = scene(FillContour::new(NOTCHED.into_iter().chain(NOTCHED), INK));
    let twice_paint = DisplayList::compile(&twice, &layout(&twice), None);
    assert!(coverage(&twice_paint.ops).is_empty());
}

#[test]
fn legacy_sampling_stays_bounded_for_a_large_diagonal() {
    let size = 65_536.;
    let points = [[0., 0.], [size, 0.], [0., size]];
    let dom = MicroDom::new(
        Node::new(Role::Window, "Window").fill_contour(FillContour::new(points, INK).unwrap()),
    );
    let computed = LayoutEngine::default()
        .compute(&dom, size, size, &mut Metrics)
        .unwrap();
    let list = DisplayList::compile(&dom, &computed, None);
    assert!(list.ops.len() <= 512);
    assert!(list
        .ops
        .iter()
        .any(|op| { matches!(op, PaintOp::Quad { color, .. } if color.3 > 0 && color.3 < INK.3) }));
    assert!(list
        .ops
        .iter()
        .any(|op| { matches!(op, PaintOp::Quad { color, .. } if *color == INK) }));
}

#[test]
fn transparent_and_fully_clipped_fills_do_not_publish_a_paint_op() {
    let outside = NOTCHED.map(|[x, y]| [x + 100., y]);
    for (points, color) in [(outside, INK), (NOTCHED, Color(52, 88, 184, 0))] {
        let dom = scene(FillContour::new(points, color));
        assert_eq!(dom.snapshot_version(), 4);
        let list = ExtendedDisplayList::compile_with_interaction(
            &dom,
            &layout(&dom),
            None,
            &BTreeMap::new(),
            InteractionState::default(),
        );
        assert!(!list
            .ops()
            .iter()
            .any(|op| matches!(op, ExtendedPaintOp::FillPath(_))));
    }
}
