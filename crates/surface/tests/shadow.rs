use std::collections::BTreeMap;
use surface::{
    BoxShadow, Color, ComputedDom, Corners, DisplayList, Insets, InteractionState, LayoutEngine,
    Length, MicroDom, Node, NodeId, PaintOp, Role, Style, TextMeasurer,
};

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

const FACE: Color = Color::rgb(250, 245, 255);
const NO_SHADOW: [BoxShadow; 2] = [BoxShadow::NONE; 2];

fn at(x: f32, y: f32) -> Insets {
    Insets {
        left: Length::Px(x),
        top: Length::Px(y),
        ..Insets::default()
    }
}

fn scene(caster_y: f32, shadows: [BoxShadow; 2]) -> (MicroDom, NodeId, NodeId) {
    let mut dom = MicroDom::new(Node::new(Role::Window, "Window"));
    let clipper = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "Viewport").styled(
                Style::column()
                    .absolute(at(10., 10.))
                    .width(Length::Px(100.))
                    .height(Length::Px(64.))
                    .clip(true),
            ),
        )
        .unwrap();
    let caster = dom
        .append(
            clipper,
            Node::new(Role::Button, "Caster")
                .id("caster")
                .box_shadows(shadows)
                .styled(
                    Style::row()
                        .absolute(at(18., caster_y))
                        .width(Length::Px(48.))
                        .height(Length::Px(48.))
                        .corners(Corners::new(8., 4., 12., 0.))
                        .background(FACE)
                        .clip(true),
                ),
        )
        .unwrap();
    (dom, clipper, caster)
}

fn compute(dom: &MicroDom) -> ComputedDom {
    LayoutEngine::default()
        .compute(dom, 148., 128., &mut Metrics)
        .unwrap()
}

fn color(op: &PaintOp) -> Color {
    match op {
        PaintOp::Quad { color, .. }
        | PaintOp::Shape { color, .. }
        | PaintOp::Stroke { color, .. }
        | PaintOp::ShapeStroke { color, .. } => *color,
        other => panic!("expected a shape, got {other:?}"),
    }
}

fn stroke_width(op: &PaintOp) -> Option<f32> {
    match op {
        PaintOp::Stroke { width, .. } | PaintOp::ShapeStroke { width, .. } => Some(*width),
        _ => None,
    }
}

#[test]
fn shadow_does_not_change_layout_or_hits_and_paints_before_face_and_focus() {
    let (plain, _, _) = scene(8., NO_SHADOW);
    let layout_before = compute(&plain);
    let inputs = [(-4., 4., 2., 0.), (0., 4., 6., 2.)];
    let shadows =
        inputs.map(|(x, y, blur, spread)| BoxShadow::new(x, y, blur, spread, Color(10, 0, 0, 36)));
    let (dom, clipper, caster) = scene(8., shadows);
    let computed = compute(&dom);
    assert_eq!(computed, layout_before);
    let bounds = computed.nodes[caster.index()].bounds;
    let clip = computed.nodes[clipper.index()].bounds;
    assert_eq!(
        computed.hit_test(&dom, bounds.x + 5., bounds.y + 5.),
        Some(caster)
    );
    assert_eq!(computed.hit_test(&dom, bounds.x - 1., bounds.y + 14.), None);
    let (list, owners) = DisplayList::compile_attributed(
        &dom,
        &computed,
        Some(caster),
        &BTreeMap::new(),
        InteractionState::default(),
    );
    let shadow_ops = &list.ops[..list.ops.len() - 2];
    assert!(shadow_ops.len() > 2);
    assert!(shadow_ops.iter().all(|op| op.clip() == clip));
    assert!(shadow_ops
        .iter()
        .any(|op| op.rect().contains(bounds.x - 1., bounds.y + 14.)));
    assert!(owners.iter().all(|owner| *owner == caster));
    assert_eq!(list.ops[list.ops.len() - 2].rect(), bounds);
    assert_eq!(color(&list.ops[list.ops.len() - 2]), FACE);
    assert!(matches!(list.ops.last(), Some(PaintOp::ShapeStroke { .. })));
    assert_eq!(
        serde_json::from_str::<DisplayList>(&serde_json::to_string(&list).unwrap()).unwrap(),
        list
    );
}

#[test]
fn an_offscreen_caster_can_leave_a_visible_shadow_inside_the_ancestor_clip() {
    let (dom, clipper, caster) = scene(
        65.,
        [
            BoxShadow::new(0., -4., 4., 0., Color(0, 0, 0, 60)),
            BoxShadow::NONE,
        ],
    );
    let computed = compute(&dom);
    let clip = computed.nodes[clipper.index()].bounds;
    let bounds = computed.nodes[caster.index()].bounds;
    assert_eq!(bounds.intersect(clip).height, 0.);
    let list = DisplayList::compile(&dom, &computed, None);
    assert!(!list.ops.is_empty());
    assert!(list
        .ops
        .iter()
        .all(|op| op.clip() == clip && op.rect().intersect(clip).height > 0. && color(op) != FACE));
}

#[test]
fn soft_shadow_is_bounded_and_reaches_the_requested_opacity() {
    let base = Style::default()
        .absolute(at(48., 28.))
        .width(Length::Px(48.))
        .height(Length::Px(48.))
        .radius(4.);
    let mut dom = MicroDom::new(Node::new(Role::Window, "Window"));
    let caster = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "Translucent caster")
                .styled(base)
                .box_shadows([
                    BoxShadow::new(0., 0., 12., 0., Color(49, 40, 32, 200)),
                    BoxShadow::NONE,
                ]),
        )
        .unwrap();
    let computed = compute(&dom);
    let ops = DisplayList::compile(&dom, &computed, None).ops;
    assert!(ops.len() > 8 && ops.len() <= 25);
    let center = computed.nodes[caster.index()].bounds;
    let (fill, rings) = ops.split_last().unwrap();
    assert_eq!(fill.rect(), center.inset(12.));
    assert_eq!(stroke_width(fill), None);
    assert_eq!(color(fill), Color(49, 40, 32, 200));
    for (ring, next) in rings.iter().zip(ops.iter().skip(1)) {
        let edge = ring.rect().inset(stroke_width(ring).expect("one ring"));
        assert!((edge.x - next.rect().x).abs() < 0.001);
        assert!((edge.width - next.rect().width).abs() < 0.001);
        let shade = color(ring);
        assert_eq!((shade.0, shade.1, shade.2), (49, 40, 32));
        assert!(shade.3 <= color(next).3);
    }
}

#[test]
fn the_higher_shadow_darkens_the_dark_backdrop_after_rgba8_rounding() {
    let key = BoxShadow::new(0., 1., 2., 0., Color(0, 0, 0, 77));
    let mut profiles = Vec::new();
    for (y, blur, spread) in [(1., 3., 1.), (2., 6., 2.)] {
        let ambient = BoxShadow::new(0., y, blur, spread, Color(0, 0, 0, 38));
        let (dom, _, caster) = scene(2., [ambient, key]);
        let computed = compute(&dom);
        let bounds = computed.nodes[caster.index()].bounds;
        let list = DisplayList::compile(&dom, &computed, None);
        let x = bounds.x + bounds.width / 2.;
        let profile = (0..11)
            .map(|dy| {
                let y = bounds.y + bounds.height + dy as f32 + 0.5;
                // On the straight edge at this pixel center, integer 1dp
                // contours cover the sample exactly. Quantize after each op.
                list.ops
                    .iter()
                    .filter(|op| {
                        op.clip().contains(x, y)
                            && op.rect().contains(x, y)
                            && stroke_width(op)
                                .is_none_or(|width| !op.rect().inset(width).contains(x, y))
                    })
                    .fold(22u16, |gray, op| {
                        (gray * (255 - u16::from(color(op).3)) + 127) / 255
                    })
            })
            .collect::<Vec<_>>();
        profiles.push(profile);
    }
    let [rest, hover] = profiles.as_slice() else {
        panic!("two profiles");
    };
    assert!(rest.iter().zip(hover).all(|(rest, hover)| hover <= rest));
    assert!(rest.iter().zip(hover).any(|(rest, hover)| hover < rest));
}

#[test]
fn omitted_and_invalid_shadow_values_leave_legacy_paint_intact() {
    let empty = serde_json::to_value(Style::default()).unwrap();
    assert!(empty.get("box_shadows").is_none());
    assert_eq!(
        serde_json::from_value::<Style>(empty).unwrap(),
        Style::default()
    );
    let (plain, _, _) = scene(8., NO_SHADOW);
    let original = DisplayList::compile(&plain, &compute(&plain), None);
    let color = Color(0, 0, 0, 80);
    let invalid = [
        BoxShadow::new(0., 0., -1., 0., color),
        BoxShadow::new(f32::NAN, 0., 3., 0., color),
    ];
    let (dom, _, _) = scene(8., invalid);
    assert_eq!(DisplayList::compile(&dom, &compute(&dom), None), original);
}

#[test]
fn node_shadow_round_trips_without_changing_style_or_legacy_node_json() {
    let plain = Node::new(Role::Dialog, "Navigation");
    let legacy = serde_json::to_value(&plain).unwrap();
    assert!(legacy.get("box_shadows").is_none());
    let read_legacy: Node = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(read_legacy.box_shadow_layers(), NO_SHADOW);

    let layers = [
        BoxShadow::new(0., 2., 4., 0., Color(0, 0, 0, 80)),
        BoxShadow::NONE,
    ];
    let shadowed = plain.box_shadows(layers);
    let mut json = serde_json::to_value(&shadowed).unwrap();
    assert_eq!(json["style"], legacy["style"]);
    let read_shadowed: Node = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(read_shadowed.box_shadow_layers(), layers);
    json.as_object_mut().unwrap().remove("box_shadows");
    assert_eq!(json, legacy);
}
