use surface::{
    Easing, LayoutEngine, LayoutError, MicroDom, Node, Role, Style, TextMeasurer, Transition,
};

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1e-3,
        "{actual} is not within 1e-3 of {expected}"
    );
}

#[test]
fn easing_endpoints_and_clamp() {
    for easing in [
        Easing::Linear,
        Easing::CubicBezier(0.2, 0., 0., 1.),
        Easing::CubicBezier(0.05, 0.7, 0.1, 1.),
        Easing::CubicBezier(0.3, 0., 0.8, 0.15),
        Easing::CubicBezier(0., 0., 1., 1.),
    ] {
        assert_eq!(easing.sample(0.), 0., "{easing:?}");
        assert_eq!(easing.sample(1.), 1., "{easing:?}");
        assert_eq!(easing.sample(-1.), 0., "{easing:?}");
        assert_eq!(easing.sample(2.), 1., "{easing:?}");
        assert_eq!(easing.sample(f32::NAN), 0., "{easing:?}");
    }
    assert!((Easing::Linear.sample(0.3) - 0.3).abs() < 1e-6);
}

#[test]
fn cubic_bezier_matches_reference_values() {
    let standard = Easing::CubicBezier(0.2, 0.0, 0.0, 1.0);
    for (t, expected) in [
        (0.1, 0.156250),
        (0.25, 0.607220),
        (0.5, 0.877834),
        (0.75, 0.975480),
        (0.9, 0.996459),
    ] {
        assert_near(standard.sample(t), expected);
    }
    assert_near(
        Easing::CubicBezier(0.05, 0.7, 0.1, 1.0).sample(0.5),
        0.950247,
    );
    let accelerate = Easing::CubicBezier(0.3, 0.0, 0.8, 0.15);
    assert_near(accelerate.sample(0.5), 0.153998);
    assert_near(accelerate.sample(0.9), 0.683301);
    assert_near(
        Easing::CubicBezier(0.4, 0.0, 0.2, 1.0).sample(0.5),
        0.775561,
    );
    assert_near(
        Easing::CubicBezier(0.3, 0.0, 1.0, 1.0).sample(0.5),
        0.372029,
    );
    assert_near(
        Easing::CubicBezier(0.0, 0.0, 0.0, 1.0).sample(0.25),
        0.690551,
    );
    // Control points on the diagonal give a linear curve.
    let diagonal = Easing::CubicBezier(0.0, 0.0, 1.0, 1.0);
    for t in [0.1, 0.25, 0.5, 0.8, 0.95] {
        assert_near(diagonal.sample(t), t);
    }
}

#[test]
fn easing_and_transition_validity() {
    assert!(Easing::Linear.is_valid());
    assert!(Easing::CubicBezier(0.2, 0., 0., 1.).is_valid());
    // Overshoot is allowed on the y axis. Only the x axis maps time.
    assert!(Easing::CubicBezier(0.2, -0.5, 0.2, 1.5).is_valid());
    assert!(!Easing::CubicBezier(1.2, 0., 0., 1.).is_valid());
    assert!(!Easing::CubicBezier(-0.1, 0., 0.5, 1.).is_valid());
    assert!(!Easing::CubicBezier(0.2, f32::NAN, 0.2, 1.).is_valid());
    assert!(Transition::new(200., Easing::CubicBezier(0.2, 0., 0., 1.)).is_valid());
    assert!(Transition::new(0., Easing::Linear).is_valid());
    assert!(!Transition::new(200., Easing::CubicBezier(-0.1, 0., 0., 1.)).is_valid());
    assert!(!Transition::new(-1., Easing::Linear).is_valid());
    assert!(!Transition::new(f32::NAN, Easing::Linear).is_valid());
    assert!(!Transition::new(f32::INFINITY, Easing::Linear).is_valid());
}

#[test]
fn style_json_omits_transition_and_round_trips() {
    let plain = serde_json::to_value(Style::default()).unwrap();
    assert!(plain.get("transition").is_none(), "{plain}");

    let animated =
        Style::default().transition(Transition::new(200., Easing::CubicBezier(0.2, 0., 0., 1.)));
    let json = serde_json::to_value(&animated).unwrap();
    assert_eq!(json["transition"]["duration_ms"], 200.0_f64);
    let restored: Style = serde_json::from_value(json).unwrap();
    assert_eq!(restored, animated);

    // Documents written before transitions existed must still decode.
    let legacy: Style = serde_json::from_str("{}").unwrap();
    assert_eq!(legacy.transition, None);
}

struct Measure;
impl TextMeasurer for Measure {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size / 2., size * 1.5)
    }
}

#[test]
fn layout_rejects_invalid_transition() {
    let valid =
        Style::default().transition(Transition::new(200., Easing::CubicBezier(0.2, 0., 0., 1.)));
    let dom = MicroDom::new(Node::new(Role::Window, "App").styled(valid));
    assert!(LayoutEngine::default()
        .compute(&dom, 320., 240., &mut Measure)
        .is_ok());

    let invalid =
        Style::default().transition(Transition::new(200., Easing::CubicBezier(1.5, 0., 0., 1.)));
    let dom = MicroDom::new(Node::new(Role::Window, "App").styled(invalid));
    let err = LayoutEngine::default()
        .compute(&dom, 320., 240., &mut Measure)
        .unwrap_err();
    assert!(matches!(err, LayoutError::InvalidStyle(0)), "{err:?}");
}
