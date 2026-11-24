use serde_json::json;
use surface::{
    Color, DisplayList, ExtendedDisplayList, ExtendedPaintOp, PaintOp, PaintTransform, Rect,
};

#[test]
fn opacity_markers_round_trip_without_changing_legacy_paint() {
    let viewport = Rect::new(0., 0., 32., 32.);
    let paint = PaintOp::Quad {
        rect: Rect::new(2., 3., 7., 8.),
        clip: viewport,
        radius: 1.,
        color: Color::rgb(10, 20, 90),
    };
    let legacy = DisplayList {
        viewport,
        ops: vec![paint.clone()],
    };
    let plain = ExtendedDisplayList::new(viewport, vec![paint.clone().into()]);
    let old_wire = json!({ "viewport": viewport, "ops": [{ "Legacy": paint }] });
    assert_eq!(serde_json::to_value(&plain).unwrap(), old_wire);
    assert_eq!(
        serde_json::from_value::<ExtendedDisplayList>(old_wire).unwrap(),
        plain
    );
    assert_eq!(
        serde_json::to_value(&legacy).unwrap(),
        json!({
            "viewport": viewport, "ops": [paint]
        })
    );

    let grouped = ExtendedDisplayList::new(
        viewport,
        vec![
            ExtendedPaintOp::BeginOpacityGroup { opacity: 0.375 },
            paint.clone().into(),
            ExtendedPaintOp::EndOpacityGroup,
            paint.into(),
        ],
    );
    let wire = serde_json::to_value(&grouped).unwrap();
    assert_eq!(
        wire["ops"][0],
        json!({"BeginOpacityGroup": {"opacity": 0.375}})
    );
    assert_eq!(wire["ops"][2], json!("EndOpacityGroup"));
    assert_eq!(wire["ops"][1], wire["ops"][3]);
    let decoded = serde_json::from_value::<ExtendedDisplayList>(wire).unwrap();
    assert_eq!(decoded.viewport(), viewport);
    assert_eq!(decoded.into_ops(), grouped.into_ops());
}

#[test]
fn transform_group_preserves_geometry_and_uses_the_existing_end_marker() {
    let viewport = Rect::new(0., 0., 120., 80.);
    let transform = PaintTransform {
        source: Rect::new(6., -4., 40., 20.),
        destination: Rect::new(40., 18., 72., 36.),
        clip: Rect::new(44., 20., 60., 28.),
        radius: 6.25,
        opacity: 0.625,
    };
    assert!(transform.is_valid());
    let list = ExtendedDisplayList::new(
        viewport,
        vec![
            ExtendedPaintOp::BeginTransformGroup { transform },
            ExtendedPaintOp::EndOpacityGroup,
        ],
    );
    let wire = serde_json::to_value(&list).unwrap();
    assert_eq!(
        wire["ops"][0],
        json!({"BeginTransformGroup": {"transform": transform}})
    );
    assert_eq!(wire["ops"][1], json!("EndOpacityGroup"));
    assert_eq!(
        serde_json::from_value::<ExtendedDisplayList>(wire).unwrap(),
        list
    );
    for invalid in [
        PaintTransform {
            radius: -1.,
            ..transform
        },
        PaintTransform {
            radius: f32::INFINITY,
            ..transform
        },
        PaintTransform {
            opacity: f32::NAN,
            ..transform
        },
        PaintTransform {
            opacity: 1.1,
            ..transform
        },
        PaintTransform {
            source: Rect::new(f32::MAX, 0., f32::MAX, 2.),
            ..transform
        },
        PaintTransform {
            destination: Rect::new(3., 2., 0., 1.),
            ..transform
        },
        PaintTransform {
            clip: Rect::new(3., 2., 5., -1.),
            ..transform
        },
    ] {
        assert!(!invalid.is_valid(), "{invalid:?}");
    }
}
