use surface::{ComputedDom, ComputedNode, MicroDom, Node, Rect, Role};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, FramePress, FramePressPhase};
use surface_material::ExpressiveFabMotion;

const INPUT: FrameInput<'static> = FrameInput {
    pointer: None,
    focused: None,
    touch: None,
    touch_cancelled: false,
};
const HIT: Rect = Rect {
    x: 20.,
    y: 40.,
    width: 56.,
    height: 56.,
};

fn dom() -> MicroDom {
    let mut dom = MicroDom::new(Node::new(Role::Window, "Motion test"));
    let root = dom.root();
    let fab = dom
        .append(root, Node::new(Role::Button, "Add").id("fab"))
        .unwrap();
    dom.append(fab, Node::new(Role::Group, "").id("fab-face"))
        .unwrap();
    dom
}

fn press(sequence: u64, phase: FramePressPhase) -> FramePress<'static> {
    FramePress {
        id: "fab",
        position: (24., 52.),
        bounds: HIT,
        started_ms: Some(-1000.),
        sequence,
        phase,
    }
}

fn tick(
    motion: &mut ExpressiveFabMotion,
    time: Option<f64>,
    reduced: bool,
    press: Option<FramePress<'_>>,
    visible: bool,
) -> AnimationUpdate {
    motion.advance(
        "fab",
        &AnimationFrame {
            now_ms: time,
            reduced_motion: reduced,
            dom: &dom(),
            layout: None,
        },
        &INPUT,
        press,
        None,
        visible,
    )
}

#[test]
fn accepted_origin_stays_fixed_while_radius_grows_and_the_release_fades() {
    let mut motion = ExpressiveFabMotion::new();
    let held = press(1, FramePressPhase::Held);
    assert!(tick(&mut motion, Some(0.), false, Some(held), true).request_next);
    let first = motion.ripple().unwrap();
    assert_eq!(first.center, (4. / 56., 12. / 56.));
    assert_eq!((first.progress, first.opacity), (0., 0.));

    tick(&mut motion, Some(75.), false, Some(held), true);
    let larger = motion.ripple().unwrap();
    assert_eq!(larger.center, first.center);
    assert!(larger.progress > 0. && larger.progress < 1.);
    assert_eq!(larger.opacity, 1.);
    tick(
        &mut motion,
        Some(100.),
        false,
        Some(press(1, FramePressPhase::Released)),
        true,
    );
    tick(&mut motion, Some(225.), false, None, true);
    let complete = motion.ripple().unwrap();
    assert_eq!((complete.progress, complete.opacity), (1., 1.));
    tick(&mut motion, Some(300.), false, None, true);
    assert_eq!(motion.ripple().unwrap().opacity, 0.5);
    tick(&mut motion, Some(375.), false, None, true);
    assert!(motion.ripple().is_none());
}

#[test]
fn a_fast_tap_still_has_a_pulse_and_cancel_or_replacement_retires_its_old_pose() {
    let mut motion = ExpressiveFabMotion::new();
    tick(
        &mut motion,
        Some(200.),
        false,
        Some(press(1, FramePressPhase::Released)),
        true,
    );
    let tap = motion.ripple().unwrap();
    assert_eq!(tap.progress, 0.);
    assert_eq!(tap.opacity, 1.);

    let next = FramePress {
        position: (60., 76.),
        ..press(2, FramePressPhase::Held)
    };
    tick(&mut motion, Some(220.), false, Some(next), true);
    assert_eq!(motion.ripple().unwrap().center, (40. / 56., 36. / 56.));
    assert_eq!(motion.ripple().unwrap().opacity, 0.);
    tick(&mut motion, Some(300.), false, Some(next), true);
    assert_eq!(motion.ripple().unwrap().opacity, 1.);
    tick(
        &mut motion,
        Some(301.),
        false,
        Some(press(2, FramePressPhase::Cancelled)),
        true,
    );
    assert!(motion.ripple().is_none());
    tick(&mut motion, Some(302.), false, None, true);
    assert!(motion.ripple().is_none());
}

#[test]
fn the_origin_uses_the_actual_face_in_a_larger_touch_target() {
    let dom = dom();
    let root = dom.root();
    let button = dom.by_id("fab").unwrap();
    let face = dom.by_id("fab-face").unwrap();
    let viewport = Rect::new(0., 0., 320., 240.);
    let layout = ComputedDom {
        viewport,
        nodes: [
            (root, viewport),
            (button, HIT),
            (face, Rect::new(34., 54., 28., 28.)),
        ]
        .into_iter()
        .map(|(id, bounds)| ComputedNode {
            id,
            bounds,
            clip: viewport,
        })
        .collect(),
        paint_order: vec![root, button, face],
    };
    let mut motion = ExpressiveFabMotion::new();
    motion.advance(
        "fab",
        &AnimationFrame {
            now_ms: Some(0.),
            reduced_motion: false,
            dom: &dom,
            layout: Some(&layout),
        },
        &INPUT,
        Some(FramePress {
            position: (38., 62.),
            ..press(1, FramePressPhase::Held)
        }),
        None,
        true,
    );
    assert_eq!(motion.ripple().unwrap().center, (4. / 28., 8. / 28.));
}

#[test]
fn reduced_motion_or_missing_clock_does_not_replay_an_existing_press() {
    for (time, reduced) in [(None, false), (Some(0.), true), (Some(f64::NAN), false)] {
        let mut motion = ExpressiveFabMotion::new();
        let held = press(1, FramePressPhase::Held);
        tick(&mut motion, time, reduced, Some(held), true);
        assert!(motion.ripple().is_none());
        tick(&mut motion, Some(50.), false, Some(held), true);
        assert!(motion.ripple().is_none());
        tick(
            &mut motion,
            Some(80.),
            false,
            Some(press(2, FramePressPhase::Held)),
            true,
        );
        assert!(motion.ripple().is_some());
        tick(
            &mut motion,
            Some(90.),
            false,
            Some(press(2, FramePressPhase::Held)),
            false,
        );
        assert!(motion.ripple().is_none());
    }
}

#[test]
fn a_fast_tap_without_layout_uses_the_previously_presented_scaled_face() {
    let mut motion = ExpressiveFabMotion::new().with_visibility(false);
    tick(&mut motion, Some(0.), false, None, true);
    tick(&mut motion, Some(32.), false, None, true);
    let previously_visible = (HIT.width * motion.visibility_scale()).round();
    assert!(previously_visible > 0. && previously_visible < HIT.width);
    let inset = (HIT.width - previously_visible) / 2.;
    let tap = FramePress {
        position: (
            HIT.x + inset + previously_visible * 0.25,
            HIT.y + inset + previously_visible * 0.75,
        ),
        ..press(1, FramePressPhase::Released)
    };

    // The caller's action invalidated layout before the tap was observed.
    // Visibility advances again within this same frame.
    tick(&mut motion, Some(80.), false, Some(tap), true);
    assert!((HIT.width * motion.visibility_scale()).round() > previously_visible);
    let wave = motion.ripple().unwrap();
    assert_eq!((wave.center, wave.opacity), ((0.25, 0.75), 1.));
}

#[test]
fn scale_and_opacity_keep_their_pose_on_reversal_and_snap_when_needed() {
    let mut motion = ExpressiveFabMotion::new().with_visibility(false);
    assert_eq!(
        (motion.visibility_scale(), motion.visibility_opacity()),
        (0.2, 0.)
    );
    tick(&mut motion, Some(0.), false, None, true);
    tick(&mut motion, Some(20.), false, None, true);
    let partial = (motion.visibility_scale(), motion.visibility_opacity());
    assert!(partial.0 > 0.2 && partial.0 < 1. && partial.1 > 0. && partial.1 < 1.);
    assert!(tick(&mut motion, Some(20.), false, None, false).request_next);
    assert_eq!(
        (motion.visibility_scale(), motion.visibility_opacity()),
        partial
    );
    tick(&mut motion, Some(10.), false, None, false);
    assert_eq!(
        (motion.visibility_scale(), motion.visibility_opacity()),
        partial
    );

    tick(&mut motion, Some(1000.), false, None, false);
    assert_eq!(
        (motion.visibility_scale(), motion.visibility_opacity()),
        (0.2, 0.)
    );
    tick(&mut motion, Some(1001.), true, None, true);
    assert_eq!(
        (motion.visibility_scale(), motion.visibility_opacity()),
        (1., 1.)
    );
    tick(&mut motion, None, false, None, false);
    assert_eq!(
        (motion.visibility_scale(), motion.visibility_opacity()),
        (0.2, 0.)
    );
}
