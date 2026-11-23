use std::collections::BTreeMap;

use surface::{
    Align, Color, ComputedDom, Corners, DisplayList, DomSnapshot, ExtendedDisplayList,
    ExtendedPaintOp, InteractionState, LayoutEngine, Length, MicroDom, Node, NodeId, PaintOp,
    PathFill, RasterImage, Rect, Role, Style, TextMeasurer,
};

const FACE: Color = Color(220, 218, 226, 255);
const INK: Color = Color(22, 23, 24, 255);
const FOCUS: Color = Color(80, 74, 112, 255);
const PRESSED: Color = Color(22, 23, 24, 26);
const FOCUSED: Color = Color(22, 23, 24, 25);
const HOVERED: Color = Color(22, 23, 24, 20);
const STYLE_CORNERS: Corners = Corners::new(16., 16., 4., 4.);
const SHOWN_CORNERS: Corners = Corners::new(14., 11., 8., 6.);

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size / 2., size)
    }
}

fn scene(media: bool, ripple: Option<Option<&str>>, scroll_y: f32) -> MicroDom {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "List scene")
            .styled(Style::column().align(Align::Start).padding(8.)),
    );
    let scroller = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "").id("scroll").styled(Style {
                scroll_y,
                ..Style::column()
                    .width(Length::Px(160.))
                    .height(Length::Px(if scroll_y > 0. { 40. } else { 80. }))
                    .clip(true)
            }),
        )
        .unwrap();
    let row = Node::new(Role::Button, "Open item")
        .id("row")
        .styled(
            Style::column()
                .width(Length::Px(140.))
                .height(Length::Px(80.))
                .corners(STYLE_CORNERS)
                .state_layer(INK)
                .background(FACE),
        )
        .state_layer_alphas(HOVERED.3, FOCUSED.3, PRESSED.3)
        .focus_color(FOCUS)
        .focus_outline_width(3.);
    let row = match ripple {
        Some(after) => row.press_ripple(after.map(str::to_owned)),
        None => row,
    };
    let row_id = dom.append(scroller, row).unwrap();
    if media {
        dom.append(
            row_id,
            Node::new(Role::Group, "")
                .id("media")
                .styled(
                    Style::column()
                        .width(Length::Px(24.))
                        .height(Length::Px(12.))
                        .corners(Corners::all(3.)),
                )
                .raster_image(RasterImage::new(1, 1, [14, 85, 150, 255].as_slice()).unwrap()),
        )
        .unwrap();
    }
    dom.append(
        row_id,
        Node::new(Role::Text, "Copy")
            .id("copy")
            .text("Copy")
            .styled(Style::column().height(Length::Px(24.))),
    )
    .unwrap();
    dom.append(
        row_id,
        Node::new(Role::Group, "").id("mark").styled(
            Style::column()
                .width(Length::Px(12.))
                .height(Length::Px(10.))
                .background(Color::rgb(130, 60, 40)),
        ),
    )
    .unwrap();
    dom
}

fn layout(dom: &MicroDom) -> ComputedDom {
    LayoutEngine::default()
        .compute(dom, 180., 100., &mut Metrics)
        .unwrap()
}

fn lookup(dom: &MicroDom, name: &str) -> NodeId {
    dom.by_id(name).unwrap()
}

fn held(row: NodeId) -> InteractionState {
    InteractionState {
        hovered: Some(row),
        pressed: Some(row),
    }
}

fn display(
    dom: &MicroDom,
    computed: &ComputedDom,
    focused: Option<NodeId>,
    interaction: InteractionState,
) -> (ExtendedDisplayList, Vec<NodeId>) {
    ExtendedDisplayList::compile_attributed(dom, computed, focused, &BTreeMap::new(), interaction)
}

fn is_fill(op: &ExtendedPaintOp, want: Color) -> bool {
    matches!(op, ExtendedPaintOp::Legacy(PaintOp::Shape { color, .. } | PaintOp::Quad { color, .. }) if *color == want)
}

fn is_focus(op: &ExtendedPaintOp, want: Corners) -> bool {
    matches!(op, ExtendedPaintOp::Legacy(PaintOp::ShapeStroke { corners, color: FOCUS, width: 3., .. }) if *corners == want)
}

fn wave(list: &ExtendedDisplayList) -> (usize, &PathFill) {
    let mut paths = list.ops().iter().enumerate().filter_map(|(index, op)| {
        if let ExtendedPaintOp::FillPath(path) = op {
            Some((index, path))
        } else {
            None
        }
    });
    let hit = paths.next().expect("one wave");
    assert!(paths.next().is_none());
    hit
}

#[test]
fn live_opt_in_preserves_exact_dom_and_legacy_paint() {
    let former = scene(true, None, 0.);
    let mut current = scene(true, Some(Some("media")), 0.);
    let row = lookup(&current, "row");
    assert!(current.get(row).unwrap().has_press_ripple());
    assert!(!former.get(row).unwrap().has_press_ripple());
    current
        .set_presented_press_ripple(row, (72., 14.), 0.5, 0.5)
        .unwrap();

    let old_dom = serde_json::to_string(&former.snapshot()).unwrap();
    assert_eq!(serde_json::to_string(&current.snapshot()).unwrap(), old_dom);
    let decoded: DomSnapshot = serde_json::from_str(&old_dom).unwrap();
    let restored = MicroDom::from_snapshot(decoded).unwrap();
    assert!(!restored.get(row).unwrap().has_press_ripple());

    let compile = |dom: &MicroDom| {
        DisplayList::compile_attributed(dom, &layout(dom), Some(row), &BTreeMap::new(), held(row))
    };
    let (old_paint, old_owners) = compile(&former);
    let (paint, owners) = compile(&current);
    assert_eq!(paint, old_paint);
    assert_eq!(owners, old_owners);
    assert_eq!(
        serde_json::to_vec(&paint).unwrap(),
        serde_json::to_vec(&old_paint).unwrap()
    );
    assert!(paint
        .ops
        .iter()
        .any(|op| matches!(op, PaintOp::Shape { color: PRESSED, .. })));
}

#[test]
fn held_wave_replaces_press_and_released_wave_keeps_hover_or_focus() {
    let mut dom = scene(false, Some(None), 0.);
    let row = lookup(&dom, "row");
    let computed = layout(&dom);
    dom.set_presented_press_ripple(row, (92., 34.), 0.5, 0.)
        .unwrap();
    let (transparent, _) = display(&dom, &computed, Some(row), held(row));
    assert!(transparent
        .ops()
        .iter()
        .any(|op| is_focus(op, STYLE_CORNERS)));
    assert!(!transparent.ops().iter().any(|op| {
        is_fill(op, PRESSED)
            || is_fill(op, FOCUSED)
            || is_fill(op, HOVERED)
            || matches!(op, ExtendedPaintOp::FillPath(_))
    }));

    dom.set_presented_press_ripple(row, (92., 34.), 0.5, 0.5)
        .unwrap();
    let (held_wave, _) = display(&dom, &computed, Some(row), held(row));
    assert_eq!(wave(&held_wave).1.color(), Color(22, 23, 24, 13));
    assert!(!held_wave
        .ops()
        .iter()
        .any(|op| is_fill(op, PRESSED) || is_fill(op, FOCUSED)));
    let json = serde_json::to_string(&held_wave).unwrap();
    assert_eq!(
        serde_json::from_str::<ExtendedDisplayList>(&json).unwrap(),
        held_wave
    );

    let hover = InteractionState {
        hovered: Some(row),
        pressed: None,
    };
    for (focus, color) in [(Some(row), FOCUSED), (None, HOVERED)] {
        let (released, _) = display(&dom, &computed, focus, hover);
        let (wave_position, _) = wave(&released);
        let static_state = released
            .ops()
            .iter()
            .position(|op| is_fill(op, color))
            .unwrap();
        assert!(static_state < wave_position);
        assert!(!released.ops().iter().any(|op| is_fill(op, PRESSED)));
    }

    dom.set_presented_press_ripple(row, (f32::NAN, 34.), 0.5, 0.5)
        .unwrap();
    let (invalid, _) = display(&dom, &computed, Some(row), held(row));
    assert!(invalid.ops().iter().any(|op| is_fill(op, PRESSED)));
    assert!(!invalid
        .ops()
        .iter()
        .any(|op| matches!(op, ExtendedPaintOp::FillPath(_))));
}

#[test]
fn wave_starts_at_physical_origin_and_resolves_current_asymmetric_corners() {
    let mut dom = scene(false, Some(None), 0.);
    let row = lookup(&dom, "row");
    let computed = layout(&dom);
    let geometry = &computed.nodes[row.index()];
    dom.set_presented_interaction_corners(row, SHOWN_CORNERS)
        .unwrap();
    dom.set_presented_press_ripple(row, (92., 34.), 0., 1.)
        .unwrap();
    let (initial, _) = display(&dom, &computed, Some(row), held(row));
    let path = wave(&initial).1;
    assert_eq!(
        path.rect(),
        Rect::new(geometry.bounds.x + 68., geometry.bounds.y + 10., 48., 48.)
    );
    assert_eq!(path.color(), PRESSED);
    assert_eq!(path.clip(), geometry.clip);

    dom.set_presented_press_ripple(row, (92., 34.), 2., 1.5)
        .unwrap();
    let (ready, _) = display(&dom, &computed, Some(row), held(row));
    let path = wave(&ready).1;
    assert_eq!(path.rect(), geometry.bounds);
    assert_eq!(path.color(), PRESSED);
    let Rect {
        x,
        y,
        width,
        height,
    } = geometry.bounds;
    for (origin, radius) in [
        ([x, y], SHOWN_CORNERS.top_left),
        ([x + width, y], SHOWN_CORNERS.top_right),
        ([x + width, y + height], SHOWN_CORNERS.bottom_right),
        ([x, y + height], SHOWN_CORNERS.bottom_left),
    ] {
        assert!(!path.points().contains(&origin));
        for [px, py] in path.points() {
            let dx = (*px - origin[0]).abs();
            let dy = (*py - origin[1]).abs();
            if dx < radius && dy < radius {
                assert!((radius - dx).hypot(radius - dy) <= radius + 0.001);
            }
        }
    }
    assert!(ready.ops().iter().any(|op| is_focus(op, SHOWN_CORNERS)));
}

#[test]
fn checkpoint_paints_over_media_below_copy_and_flushes_when_media_is_clipped() {
    for scroll_y in [0., 20.] {
        let mut dom = scene(true, Some(Some("media")), scroll_y);
        let row = lookup(&dom, "row");
        let media = lookup(&dom, "media");
        let copy = lookup(&dom, "copy");
        let mark = lookup(&dom, "mark");
        let computed = layout(&dom);
        let row_clip = computed.nodes[row.index()].clip;
        dom.set_presented_press_ripple(row, (22., 11.), 1., 1.)
            .unwrap();
        let (list, owners) = display(&dom, &computed, Some(row), held(row));
        let (wave_index, wave) = wave(&list);
        assert_eq!(wave.clip(), row_clip);
        assert_eq!(
            wave.rect(),
            computed.nodes[row.index()].bounds.intersect(row_clip)
        );
        let focus_index = list
            .ops()
            .iter()
            .position(|op| is_focus(op, STYLE_CORNERS))
            .unwrap();
        assert_eq!((owners[wave_index], owners[focus_index]), (row, row));
        assert_eq!(focus_index, wave_index + 1);
        assert!(focus_index < owners.iter().position(|id| *id == copy).unwrap());
        assert!(focus_index < owners.iter().position(|id| *id == mark).unwrap());

        let raster_index = list
            .ops()
            .iter()
            .position(|op| matches!(op, ExtendedPaintOp::Raster(_)));
        if scroll_y == 0. {
            assert_eq!(owners[raster_index.unwrap()], media);
            assert_eq!(raster_index.unwrap() + 1, wave_index);
        } else {
            let media = &computed.nodes[media.index()];
            assert_eq!(media.bounds.intersect(media.clip).height, 0.);
            assert!(raster_index.is_none());
        }
    }
}

#[test]
fn invalid_checkpoints_keep_feedback_at_the_owner() {
    for name in ["missing", "copy", "row", "media", "outside"] {
        let mut dom = scene(true, Some(Some(name)), 0.);
        let row = lookup(&dom, "row");
        let media = lookup(&dom, "media");
        if name == "media" {
            // A raster with a semantic descendant is not a passive leaf.
            dom.append(media, Node::new(Role::Group, "Child")).unwrap();
        }
        if name == "outside" {
            let raster = RasterImage::new(1, 1, [14, 85, 150, 255].as_slice()).unwrap();
            dom.append(
                dom.root(),
                Node::new(Role::Group, "").id(name).raster_image(raster),
            )
            .unwrap();
        }
        dom.set_presented_press_ripple(row, (12., 12.), 0.5, 1.)
            .unwrap();
        let (paint, owners) = display(&dom, &layout(&dom), Some(row), held(row));
        let wave_index = wave(&paint).0;
        let media_index = owners.iter().position(|id| *id == media).unwrap();
        assert!(wave_index < media_index, "{name}");
        assert!(is_focus(&paint.ops()[wave_index + 1], STYLE_CORNERS));
    }
}
