use surface::{ComputedDom, ComputedNode, MicroDom, Node, Rect, Role};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput};
use surface_material::{FabMenuItemMotion, FabMenuMotion, FabMenuMotionTarget};

const ITEMS: [&str; 6] = ["first", "second", "third", "fourth", "fifth", "nearest"];
const INPUT: FrameInput<'static> = FrameInput {
    pointer: None,
    focused: None,
    touch: None,
    touch_cancelled: false,
};

fn tick(
    motion: &mut FabMenuMotion,
    time: Option<f64>,
    open: bool,
    reduced: bool,
    item_ids: &[&str],
) -> AnimationUpdate {
    motion.advance(
        &AnimationFrame {
            now_ms: time,
            reduced_motion: reduced,
            dom: &MicroDom::new(Node::new(Role::Window, "Test")),
            layout: None,
        },
        &INPUT,
        None,
        None,
        FabMenuMotionTarget {
            id: "fab-menu",
            open,
            item_ids,
        },
    )
}

fn item_poses(motion: &FabMenuMotion) -> [FabMenuItemMotion; 6] {
    std::array::from_fn(|index| motion.item(index))
}

#[test]
fn items_enter_nearest_first_and_exit_top_first() {
    let mut motion = FabMenuMotion::new(false);
    assert!(tick(&mut motion, Some(0.), true, false, &ITEMS).request_next);
    let mut entered = [None; 6];
    for time in (8..=800).step_by(8) {
        tick(&mut motion, Some(f64::from(time)), true, false, &ITEMS);
        for (index, first) in entered.iter_mut().enumerate() {
            let item = motion.item(index);
            if item.width > 0. && item.opacity > 0. {
                first.get_or_insert(time);
            }
        }
    }
    let entered = entered.map(Option::unwrap);
    assert!(
        entered.windows(2).all(|pair| pair[0] > pair[1]),
        "{entered:?}"
    );
    assert_eq!(motion.progress(), 1.);
    assert_eq!(motion.visible_items(), 6.);

    tick(&mut motion, Some(1000.), false, false, &ITEMS);
    assert!(motion.items_present());
    let mut left = [None; 6];
    for time in (1008..=1800).step_by(8) {
        tick(&mut motion, Some(f64::from(time)), false, false, &ITEMS);
        for (index, first) in left.iter_mut().enumerate() {
            let item = motion.item(index);
            if item.width < 1. || item.opacity < 1. {
                first.get_or_insert(time);
            }
        }
    }
    let left = left.map(Option::unwrap);
    assert!(left.windows(2).all(|pair| pair[0] < pair[1]), "{left:?}");
    assert_eq!(motion.progress(), 0.);
    assert_eq!(motion.visible_items(), 0.);
    assert!(!motion.items_present());
}

#[test]
fn reversal_keeps_the_current_pose_even_with_a_clock_step_back() {
    let mut motion = FabMenuMotion::default();
    tick(&mut motion, Some(0.), true, false, &ITEMS);
    tick(&mut motion, Some(67.), true, false, &ITEMS);
    let current = (
        motion.progress(),
        motion.visible_items(),
        item_poses(&motion),
    );
    assert!(current.0 > 0. && current.0 < 1.);
    assert!(current.1 > 0. && current.1 < 6.);
    assert!(current.2[5].opacity > current.2[4].opacity);
    tick(&mut motion, Some(67.), false, false, &ITEMS);
    assert_eq!(
        (
            motion.progress(),
            motion.visible_items(),
            item_poses(&motion)
        ),
        current
    );
    tick(&mut motion, Some(60.), false, false, &ITEMS);
    assert_eq!(
        (
            motion.progress(),
            motion.visible_items(),
            item_poses(&motion)
        ),
        current
    );
    tick(&mut motion, Some(700.), false, false, &ITEMS);
    assert_eq!(motion.progress(), 0.);
    assert!(!motion.items_present());
}

#[test]
fn reduce_motion_and_missing_or_invalid_time_switch_to_requested_endpoints() {
    let mut motion = FabMenuMotion::new(false);
    let two = &ITEMS[..2];
    tick(&mut motion, Some(0.), true, true, two);
    assert_eq!(motion.progress(), 1.);
    assert_eq!(motion.visible_items(), 2.);
    assert_eq!(
        motion.item(0),
        FabMenuItemMotion {
            width: 1.,
            opacity: 1.
        }
    );
    assert!(motion.items_present());
    tick(&mut motion, None, false, false, two);
    assert_eq!(motion.progress(), 0.);
    assert_eq!(motion.visible_items(), 0.);
    assert!(!motion.items_present());
    tick(&mut motion, Some(f64::NAN), true, false, two);
    assert_eq!(motion.visible_items(), 2.);
    assert!(motion.items_present());
}

#[test]
fn initially_open_items_appear_without_a_transition() {
    let mut motion = FabMenuMotion::new(true);
    tick(&mut motion, Some(0.), true, false, &ITEMS);
    assert_eq!(motion.progress(), 1.);
    assert_eq!(motion.visible_items(), 6.);
    assert_eq!(
        item_poses(&motion),
        [FabMenuItemMotion {
            width: 1.,
            opacity: 1.
        }; 6]
    );
    assert_eq!(
        tick(&mut motion, Some(10.), true, false, &ITEMS),
        AnimationUpdate::default()
    );
}

#[test]
fn the_anchor_is_the_original_slot_and_survives_a_layout_reset() {
    let mut dom = MicroDom::new(Node::new(Role::Window, "Test"));
    let root = dom.root();
    let slot = dom
        .append(root, Node::new(Role::Group, "").id("fab-menu"))
        .unwrap();
    let trigger = dom
        .append(
            slot,
            Node::new(Role::Button, "Close").id("fab-menu-trigger"),
        )
        .unwrap();
    let viewport = Rect::new(0., 0., 320., 240.);
    let anchor = Rect::new(208., 128., 96., 96.);
    let face = Rect::new(248., 128., 56., 56.);
    let layout = ComputedDom {
        viewport,
        nodes: [(root, viewport), (slot, anchor), (trigger, face)]
            .into_iter()
            .map(|(id, bounds)| ComputedNode {
                id,
                bounds,
                clip: viewport,
            })
            .collect(),
        paint_order: vec![root, slot, trigger],
    };
    let mut motion = FabMenuMotion::default();
    let draw = |motion: &mut FabMenuMotion, layout: Option<&ComputedDom>| {
        motion.advance(
            &AnimationFrame {
                now_ms: Some(0.),
                reduced_motion: true,
                dom: &dom,
                layout,
            },
            &INPUT,
            None,
            None,
            FabMenuMotionTarget {
                id: "fab-menu",
                open: true,
                item_ids: &ITEMS,
            },
        )
    };
    assert!(draw(&mut motion, None).request_next);
    assert_eq!(motion.anchor(), None);
    assert!(draw(&mut motion, Some(&layout)).rebuild);
    assert_eq!(motion.anchor(), Some(anchor));
    assert!(draw(&mut motion, None).request_next);
    assert_eq!(motion.anchor(), Some(anchor));
    tick(&mut motion, Some(0.), false, true, &ITEMS);
    assert_eq!(motion.anchor(), None);
}
