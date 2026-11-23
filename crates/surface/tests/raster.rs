use std::{collections::BTreeMap, sync::Arc};

use surface::{
    Align, Color, Corners, DisplayList, DomSnapshot, ExtendedDisplayList, ExtendedPaintOp, Insets,
    InteractionState, LayoutEngine, Length, MicroDom, Node, PaintOp, RasterImage, RasterPaint,
    Rect, Role, Style, TextMeasurer,
};

fn source() -> RasterImage {
    RasterImage::new(8, 2, vec![123; 64]).unwrap()
}

#[test]
fn raw_pixels_validate_dimensions_and_clones_share_the_original_allocation() {
    assert!(RasterImage::new(0, 1, vec![]).is_none());
    assert!(RasterImage::new(RasterImage::MAX_SIDE + 1, 1, vec![]).is_none());
    assert!(RasterImage::new(2, 2, vec![0; 15]).is_none());
    assert!(RasterImage::new(2, 2, vec![0; 17]).is_none());

    let pixels: Arc<[u8]> = Arc::from([119; 16]);
    let image = RasterImage::new(2, 2, pixels.clone()).unwrap();
    assert!(image.shares_pixels(&image.clone()));
    assert!(!image.shares_pixels(&RasterImage::new(4, 1, pixels).unwrap()));
    assert!(!image.shares_pixels(&RasterImage::new(2, 2, vec![119; 16]).unwrap()));
    assert_eq!(image.rgba8(), [119; 16]);
    assert!(!format!("{image:?}").contains("119"));
}

#[test]
fn centered_cover_crops_either_axis_without_rescaling_after_clipping() {
    let rect = Rect::new(5., 6., 24., 12.);
    let clip = Rect::new(9., 7., 14., 20.);
    let corners = Corners::new(2., 99., 3., 0.);
    let image = source();
    let paint = RasterPaint::new(image.clone(), rect, clip, corners).unwrap();
    assert!(paint.image().shares_pixels(&image));
    assert_eq!(paint.source_uv(), [0.25, 0., 0.5, 1.]);
    assert_eq!(paint.rect(), rect);
    assert_eq!(paint.clip(), clip);
    assert_eq!(paint.corners(), Corners::new(2., 6., 3., 0.));

    let portrait = RasterImage::new(2, 8, vec![255; 64]).unwrap();
    let paint = RasterPaint::new(portrait, rect, clip, corners).unwrap();
    assert_eq!(paint.source_uv(), [0., 0.4375, 1., 0.125]);
    assert!(RasterPaint::new(image.clone(), rect, Rect::new(30., 6., 4., 4.), corners).is_none());
    assert!(RasterPaint::new(image.clone(), Rect::default(), clip, corners).is_none());
    assert!(RasterPaint::new(image, rect, clip, Corners::all(-1.)).is_none());
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn scene(with_image: bool, hidden: bool) -> MicroDom {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "Raster").styled(Style::column().align(Align::Start).padding(8.)),
    );
    let clip = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Px(30.))
                    .height(Length::Px(20.))
                    .clip(true),
            ),
        )
        .unwrap();
    if hidden {
        dom.append(
            clip,
            Node::new(Role::Group, "").styled(Style::column().height(Length::Px(20.))),
        )
        .unwrap();
    }
    let row = Node::new(Role::Group, "").id("art").styled(
        Style::column()
            .width(Length::Px(40.))
            .height(Length::Px(20.))
            .background(Color::rgb(230, 230, 230))
            .border(1., Color::rgb(0, 0, 0))
            .corners(Corners::new(4., 0., 5., 6.)),
    );
    dom.append(
        clip,
        if with_image {
            row.raster_image(source())
        } else {
            row
        },
    )
    .unwrap();
    dom.append(
        dom.root(),
        Node::new(Role::Group, "").id("overlay").styled(
            Style::column()
                .width(Length::Px(4.))
                .height(Length::Px(4.))
                .background(Color::rgb(255, 60, 20))
                .absolute(Insets {
                    top: Length::Px(16.),
                    left: Length::Px(16.),
                    ..Insets::default()
                }),
        ),
    )
    .unwrap();
    dom
}

fn layout(dom: &MicroDom) -> surface::ComputedDom {
    LayoutEngine::default()
        .compute(dom, 80., 60., &mut Metrics)
        .unwrap()
}

#[test]
fn raster_is_attributed_between_background_and_border_and_omitted_from_existing_wire() {
    let dom = scene(true, false);
    let plain = scene(false, false);
    let computed = layout(&dom);
    let owner = dom.by_id("art").unwrap();
    assert_eq!(computed, layout(&plain));
    assert!(!dom.get(owner).unwrap().is_focusable());
    assert_eq!(dom.snapshot_version(), plain.snapshot_version());
    let encoded = serde_json::to_vec(&dom.snapshot()).unwrap();
    assert_eq!(encoded, serde_json::to_vec(&plain.snapshot()).unwrap());
    let decoded: DomSnapshot = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        MicroDom::from_snapshot(decoded).unwrap().snapshot(),
        plain.snapshot()
    );
    assert_eq!(
        DisplayList::compile(&dom, &computed, None),
        DisplayList::compile(&plain, &computed, None)
    );

    let (extended, owners) = ExtendedDisplayList::compile_attributed(
        &dom,
        &computed,
        None,
        &BTreeMap::new(),
        InteractionState::default(),
    );
    let [ExtendedPaintOp::Legacy(PaintOp::Shape { .. }), ExtendedPaintOp::Raster(raster), ExtendedPaintOp::Legacy(PaintOp::ShapeStroke { .. }), ExtendedPaintOp::Legacy(PaintOp::Quad { .. })] =
        extended.ops()
    else {
        panic!("expected background, image, border and sibling overlay");
    };
    assert_eq!(raster.rect(), Rect::new(8., 8., 40., 20.));
    assert_eq!(raster.clip(), Rect::new(8., 8., 30., 20.));
    assert_eq!(raster.source_uv(), [0.25, 0., 0.5, 1.]);
    assert_eq!(owners, [owner, owner, owner, dom.by_id("overlay").unwrap()]);
    assert!(serde_json::to_vec(&extended).is_err());
}

#[test]
fn fully_clipped_images_emit_no_operation() {
    let dom = scene(true, true);
    let computed = layout(&dom);
    let list = ExtendedDisplayList::compile_with_interaction(
        &dom,
        &computed,
        None,
        &BTreeMap::new(),
        InteractionState::default(),
    );
    assert!(list
        .ops()
        .iter()
        .all(|op| !matches!(op, ExtendedPaintOp::Raster(_))));
}
