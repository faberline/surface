use std::num::NonZeroU16;

use surface::{Insets, Length, Node, NodeId, RangeValue, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityAdjustment, AccessibilityAdjustmentResult, AppError, Application, EditKey, Event,
    Roving, Runtime, SliderInputOptions, SliderScale, View,
};

const OWNER: &str = "pair";
const RAIL: &str = "rail";
const LOWER: &str = "lower";
const UPPER: &str = "upper";
const ORDINARY: &str = "ordinary";

#[derive(Clone)]
enum Message {
    Set(&'static str, f64),
    Disable,
    Open,
}

#[derive(Clone, Copy, Default)]
enum Rail {
    #[default]
    Passive,
    Press,
    Tab,
}

struct Model {
    values: [f64; 2],
    changes: Vec<(&'static str, f64)>,
    rtl: bool,
    disabled: bool,
    modal: bool,
    reference: [&'static str; 3],
    upper_reverse: bool,
    owner_id: &'static str,
    rail: Rail,
    overlap_hits: bool,
    cover_hits: bool,
    inside_roving: bool,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            values: [2.5, 52.5],
            changes: Vec::new(),
            rtl: false,
            disabled: false,
            modal: false,
            reference: [RAIL, LOWER, UPPER],
            upper_reverse: false,
            owner_id: OWNER,
            rail: Rail::Passive,
            overlap_hits: false,
            cover_hits: false,
            inside_roving: false,
        }
    }
}

impl Application for Model {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Set(thumb, value) if thumb != ORDINARY => {
                self.values[usize::from(thumb == UPPER)] = value;
                self.changes.push((thumb, value));
            }
            Message::Set(_, _) => {}
            Message::Disable => self.disabled = true,
            Message::Open => self.modal = true,
        }
    }

    fn view(&self) -> View<Message> {
        let placed = |x| {
            Style::row()
                .width(Length::Px(48.))
                .height(Length::Px(48.))
                .absolute(Insets {
                    left: Length::Px(x),
                    top: Length::Px(0.),
                    ..Insets::default()
                })
        };
        let thumb = |id, name, value, reverse_horizontal| {
            let fraction = (value + 10.) as f32 / 100.;
            let center = if reverse_horizontal {
                160. * (1. - fraction)
            } else {
                160. * fraction
            };
            let style = if self.overlap_hits && id != ORDINARY {
                placed((center - 24.).clamp(0., 112.))
            } else {
                Style::row().width(Length::Px(48.)).height(Length::Px(48.))
            };
            let mut node = Node::new(Role::Slider, name)
                .id(id)
                .range(RangeValue {
                    min: -10.,
                    max: 90.,
                    value,
                    step: 12.5,
                })
                .styled(style);
            if id == ORDINARY {
                node = node.tab_index(0);
            }
            View::new(node)
                .on_value(move |value| Message::Set(id, value))
                .with_slider_input(SliderInputOptions {
                    scale: SliderScale::Discrete(NonZeroU16::new(8).unwrap()),
                    reverse_horizontal,
                })
                .accessible_slider_adjustment()
        };
        let mut rail = Node::new(Role::Group, "")
            .id(RAIL)
            .styled(Style::row().width(Length::Px(160.)).height(Length::Px(24.)));
        if matches!(self.rail, Rail::Tab) {
            rail = rail.tab_index(0);
        }
        let mut rail = View::new(rail);
        if matches!(self.rail, Rail::Press) {
            rail = rail.on_press(Message::Disable);
        }
        let overlay = self.cover_hits.then(|| {
            View::new(
                Node::new(Role::Button, "Overlay")
                    .id("overlay")
                    .styled(placed(56.)),
            )
            .on_press(Message::Disable)
        });
        let owner = View::new(
            Node::new(Role::Group, "Price")
                .id(self.owner_id)
                .disabled(self.disabled)
                .styled(Style::row()),
        )
        .children(
            [
                rail,
                thumb(LOWER, "Minimum", self.values[0], self.rtl),
                thumb(
                    UPPER,
                    "Maximum",
                    self.values[1],
                    self.rtl ^ self.upper_reverse,
                ),
            ]
            .into_iter()
            .chain(overlay),
        );
        let [rail, lower, upper] = self.reference;
        let owner = owner.with_horizontal_slider_pair(rail, lower, upper);
        let owner = if self.inside_roving {
            View::new(Node::new(Role::Group, "Roving").id("roving"))
                .roving(Roving::horizontal(false))
                .children([owner])
        } else {
            owner
        };
        let modal = self.modal.then(|| {
            View::new(
                Node::new(Role::Dialog, "Modal").id("modal").styled(
                    Style::column()
                        .width(Length::Px(180.))
                        .height(Length::Px(100.)),
                ),
            )
            .modal(Message::Disable)
        });
        View::new(Node::new(Role::Window, "Sliders").styled(Style::column())).children(
            [owner, thumb(ORDINARY, "Standalone", 15., false)]
                .into_iter()
                .chain(modal),
        )
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<Model>) {
    runtime.render(420., 240., &mut Metrics).unwrap();
}

fn open(model: Model) -> Runtime<Model> {
    let mut runtime = Runtime::new(model).unwrap();
    render(&mut runtime);
    runtime
}

fn id(runtime: &Runtime<Model>, key: &str) -> NodeId {
    runtime.dom().by_id(key).unwrap()
}

fn limits(runtime: &Runtime<Model>, key: &str) -> Option<(f64, f64)> {
    runtime
        .accessibility_frame()
        .unwrap()
        .slider_adjustment_bounds(id(runtime, key))
}

fn focus_next(runtime: &mut Runtime<Model>, key: &str) {
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    assert_eq!(runtime.focused(), Some(key));
}

fn key(runtime: &mut Runtime<Model>, key: EditKey) -> bool {
    runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false,
        })
        .unwrap()
}

#[test]
fn setters_and_keys_snap_on_the_full_signed_lattice_then_enforce_the_peer_limit() {
    let mut runtime = open(Model::default());
    assert_eq!(limits(&runtime, LOWER), Some((-10., 52.5)));
    assert_eq!(limits(&runtime, UPPER), Some((2.5, 90.)));
    assert_eq!(limits(&runtime, ORDINARY), Some((-10., 90.)));
    assert!(runtime.set_value(LOWER, 80.).unwrap());
    assert_eq!(runtime.app().values, [52.5, 52.5]);
    assert!(!runtime.set_value(LOWER, 100.).unwrap());
    assert!(!runtime.set_value(UPPER, 22.).unwrap());
    assert!(runtime.set_value(LOWER, 14.8).unwrap());
    assert!(runtime.set_value(UPPER, 23.).unwrap());
    assert_eq!(runtime.app().values, [15., 27.5]);
    render(&mut runtime);
    assert_eq!(limits(&runtime, LOWER), Some((-10., 27.5)));
    assert_eq!(limits(&runtime, UPPER), Some((15., 90.)));
    focus_next(&mut runtime, LOWER);
    assert!(key(&mut runtime, EditKey::End));
    assert_eq!(runtime.app().values, [27.5, 27.5]);
    focus_next(&mut runtime, UPPER);
    assert!(!key(&mut runtime, EditKey::Home));
    assert!(key(&mut runtime, EditKey::End));
    assert_eq!(runtime.app().values, [27.5, 90.]);
    assert_eq!(
        runtime.app().changes,
        [
            (LOWER, 52.5),
            (LOWER, 15.),
            (UPPER, 27.5),
            (LOWER, 27.5),
            (UPPER, 90.)
        ]
    );
}

#[test]
fn coincident_endpoint_keeps_two_tab_stops_and_describes_current_ax_limits() {
    let mut runtime = open(Model {
        values: [-10., -10.],
        rtl: true,
        inside_roving: true,
        ..Model::default()
    });
    let frame = runtime.accessibility_frame().unwrap();
    let lower_id = id(&runtime, LOWER);
    let upper_id = id(&runtime, UPPER);
    let lower = frame.adjustment_target(lower_id).unwrap();
    let upper = frame.adjustment_target(upper_id).unwrap();
    assert_eq!(frame.slider_adjustment_bounds(lower_id), Some((-10., -10.)));
    assert_eq!(frame.slider_adjustment_bounds(upper_id), Some((-10., 90.)));
    assert!(!frame.supports_adjustment_step(lower_id, true));
    assert!(!frame.supports_adjustment_step(lower_id, false));
    assert!(frame.supports_adjustment_step(upper_id, true));
    assert_eq!(
        runtime
            .accessibility_adjust(&lower, AccessibilityAdjustment::SetValue(100.))
            .unwrap(),
        AccessibilityAdjustmentResult::Rejected
    );
    focus_next(&mut runtime, LOWER);
    assert!(!key(&mut runtime, EditKey::End));
    focus_next(&mut runtime, UPPER);
    assert!(key(&mut runtime, EditKey::Left));
    assert_eq!(runtime.app().values, [-10., 2.5]);
    render(&mut runtime);
    assert_eq!(
        runtime
            .accessibility_adjust(&upper, AccessibilityAdjustment::Increment)
            .unwrap(),
        AccessibilityAdjustmentResult::Rejected
    );
    let frame = runtime.accessibility_frame().unwrap();
    let lower = frame.adjustment_target(id(&runtime, LOWER)).unwrap();
    assert_eq!(limits(&runtime, LOWER), Some((-10., 2.5)));
    assert_eq!(
        runtime
            .accessibility_adjust(&lower, AccessibilityAdjustment::SetValue(80.))
            .unwrap(),
        AccessibilityAdjustmentResult::Adjusted
    );
    assert_eq!(runtime.app().values, [2.5, 2.5]);
    assert_eq!(runtime.focused(), Some(UPPER));
}

#[test]
fn whole_pair_disable_and_modal_scope_block_actions_without_changing_the_wire_range() {
    let mut runtime = open(Model::default());
    let lower = id(&runtime, LOWER);
    let token = runtime
        .accessibility_frame()
        .unwrap()
        .adjustment_target(lower)
        .unwrap();
    runtime.apply(Message::Disable).unwrap();
    render(&mut runtime);
    assert_eq!(limits(&runtime, LOWER), Some((-10., 52.5)));
    assert_eq!(
        runtime
            .dom()
            .get(id(&runtime, LOWER))
            .unwrap()
            .range_value()
            .unwrap()
            .max,
        90.
    );
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .adjustment_target(id(&runtime, LOWER))
        .is_none());
    assert!(matches!(
        runtime.set_value(LOWER, 90.),
        Err(AppError::MissingControl(_))
    ));
    assert_eq!(
        runtime
            .accessibility_adjust(&token, AccessibilityAdjustment::Increment)
            .unwrap(),
        AccessibilityAdjustmentResult::Rejected
    );
    focus_next(&mut runtime, ORDINARY);

    let mut runtime = open(Model::default());
    runtime.apply(Message::Open).unwrap();
    render(&mut runtime);
    assert_eq!(limits(&runtime, LOWER), None);
    assert_eq!(limits(&runtime, UPPER), None);
    assert!(matches!(
        runtime.set_value(UPPER, 90.),
        Err(AppError::MissingControl(_))
    ));
    assert!(runtime.app().changes.is_empty());
}

#[test]
fn invalid_authored_pairs_are_rejected_before_mount() {
    for model in [
        Model {
            reference: ["missing", LOWER, UPPER],
            ..Model::default()
        },
        Model {
            reference: [RAIL, UPPER, LOWER],
            ..Model::default()
        },
        Model {
            reference: [RAIL, LOWER, ORDINARY],
            ..Model::default()
        },
        Model {
            values: [52.5, 2.5],
            ..Model::default()
        },
        Model {
            values: [3., 52.5],
            ..Model::default()
        },
        Model {
            upper_reverse: true,
            ..Model::default()
        },
        Model {
            rail: Rail::Press,
            ..Model::default()
        },
        Model {
            rail: Rail::Tab,
            disabled: true,
            ..Model::default()
        },
    ] {
        assert!(
            matches!(Runtime::new(model), Err(AppError::InvalidRange(owner)) if owner == OWNER)
        );
    }
    assert!(Runtime::new(Model {
        owner_id: " ",
        ..Model::default()
    })
    .is_err());
}

#[test]
fn bare_rail_selects_nearest_thumb_and_capture_never_switches_on_crossing() {
    for rtl in [false, true] {
        let mut runtime = open(Model {
            rtl,
            ..Model::default()
        });
        let at = |fraction: f32| 160. * if rtl { 1. - fraction } else { fraction };
        runtime
            .dispatch(Event::PointerDown {
                x: at(0.25),
                y: 12.,
            })
            .unwrap();
        assert_eq!(runtime.focused(), Some(LOWER));
        assert_eq!(runtime.app().values, [15., 52.5]);
        render(&mut runtime);
        runtime
            .dispatch(Event::PointerMove {
                x: at(0.95),
                y: 120.,
            })
            .unwrap();
        assert_eq!(runtime.app().values, [52.5, 52.5]);
        runtime
            .dispatch(Event::PointerUp {
                x: at(0.99),
                y: 120.,
            })
            .unwrap();
        assert_eq!(runtime.app().changes, [(LOWER, 15.), (LOWER, 52.5)]);
        assert_eq!(runtime.focused(), Some(LOWER));
        render(&mut runtime);
        runtime
            .dispatch(Event::PointerDown { x: at(0.9), y: 12. })
            .unwrap();
        assert_eq!(runtime.focused(), Some(UPPER));
        assert_eq!(runtime.app().values, [52.5, 77.5]);
        runtime
            .dispatch(Event::PointerUp { x: at(1.), y: 12. })
            .unwrap();
        assert_eq!(runtime.app().values, [52.5, 90.]);
    }
}

#[test]
fn coincident_touch_can_cancel_vertically_then_claim_either_logical_thumb() {
    for rtl in [false, true] {
        let mut runtime = open(Model {
            values: [40., 40.],
            rtl,
            ..Model::default()
        });
        let left = if rtl { 100. } else { 60. };
        let right = 160. - left;
        for event in [
            Event::PointerDown { x: 80., y: 12. },
            Event::PointerMove { x: 81., y: 28. },
            Event::PointerUp { x: left, y: 12. },
        ] {
            runtime.dispatch_touch(event).unwrap();
        }
        assert!(runtime.app().changes.is_empty());
        assert_eq!(runtime.focused(), None);
        runtime
            .dispatch_touch(Event::PointerDown { x: 80., y: 12. })
            .unwrap();
        assert_eq!(runtime.focused(), None);
        runtime
            .dispatch_touch(Event::PointerMove { x: left, y: 12. })
            .unwrap();
        assert_eq!(runtime.focused(), Some(LOWER));
        assert_eq!(runtime.app().values, [27.5, 40.]);
        runtime
            .dispatch_touch(Event::PointerMove { x: right, y: 12. })
            .unwrap();
        assert_eq!(runtime.app().values, [40., 40.]);
        runtime
            .dispatch_touch(Event::PointerUp { x: right, y: 12. })
            .unwrap();
        assert_eq!(runtime.app().changes, [(LOWER, 27.5), (LOWER, 40.)]);
        render(&mut runtime);
        runtime
            .dispatch_touch(Event::PointerDown { x: 80., y: 12. })
            .unwrap();
        runtime
            .dispatch_touch(Event::PointerUp { x: right, y: 12. })
            .unwrap();
        assert_eq!(runtime.focused(), Some(UPPER));
        assert_eq!(runtime.app().values, [40., 52.5]);
    }
}

#[test]
fn stale_peer_resize_and_escape_retire_each_contact_without_late_writes() {
    let mut runtime = open(Model::default());
    runtime
        .dispatch_touch(Event::PointerDown { x: 40., y: 12. })
        .unwrap();
    assert_eq!(runtime.app().values, [15., 52.5]);
    runtime.set_value(UPPER, 65.).unwrap();
    render(&mut runtime);
    runtime
        .dispatch_touch(Event::PointerUp { x: 120., y: 12. })
        .unwrap();
    assert_eq!(runtime.app().values, [15., 65.]);
    for stop in [Event::Cancel, Event::Blur] {
        runtime
            .dispatch(Event::PointerDown { x: 40., y: 12. })
            .unwrap();
        assert!(runtime.dispatch(stop).unwrap());
        runtime
            .dispatch(Event::PointerUp { x: 100., y: 12. })
            .unwrap();
        assert_eq!(runtime.app().values, [15., 65.]);
    }
    runtime
        .dispatch(Event::PointerDown { x: 40., y: 12. })
        .unwrap();
    runtime.pointer_geometry_changed().unwrap();
    runtime
        .dispatch(Event::PointerUp { x: 100., y: 12. })
        .unwrap();
    assert_eq!(runtime.app().changes, [(LOWER, 15.), (UPPER, 65.)]);
    render(&mut runtime);
    runtime
        .dispatch(Event::PointerDown { x: 70., y: 12. })
        .unwrap();
    assert_eq!(runtime.app().values, [40., 65.]);
}

#[test]
fn overlapping_hit_rects_do_not_let_paint_order_choose_the_thumb() {
    let mut runtime = open(Model {
        values: [27.5, 40.],
        overlap_hits: true,
        ..Model::default()
    });
    let x = 61.;
    let y = 12.;
    let upper = id(&runtime, UPPER);
    assert_eq!(
        runtime.computed().unwrap().hit_test(runtime.dom(), x, y),
        Some(upper),
        "the stock hit test resolves overlap by paint order"
    );
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(runtime.focused(), Some(LOWER));
    assert!(runtime.app().changes.is_empty());

    let mut runtime = open(Model {
        overlap_hits: true,
        cover_hits: true,
        ..Model::default()
    });
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    assert_eq!(runtime.focused(), Some("overlay"));
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    assert!(runtime.app().disabled);
    assert!(runtime.app().changes.is_empty());
}

#[test]
fn host_can_cancel_pending_pair_without_cancelling_a_standalone_slider() {
    let mut runtime = open(Model {
        values: [40., 40.],
        ..Model::default()
    });
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .adjustment_target(id(&runtime, UPPER))
        .unwrap();
    runtime
        .dispatch_touch(Event::PointerDown { x: 80., y: 12. })
        .unwrap();
    assert!(runtime.has_slider_pair_capture());
    assert_eq!(
        runtime
            .accessibility_adjust(&target, AccessibilityAdjustment::Increment)
            .unwrap(),
        AccessibilityAdjustmentResult::BlockedByPointer
    );
    assert!(runtime.cancel_slider_pair_capture().unwrap());
    assert!(!runtime.has_slider_pair_capture());
    runtime
        .dispatch_touch(Event::PointerUp { x: 160., y: 12. })
        .unwrap();
    assert!(runtime.app().changes.is_empty());

    let ordinary = id(&runtime, ORDINARY);
    let bounds = runtime.computed().unwrap().nodes[ordinary.index()].bounds;
    let (x, y) = (bounds.x + 10., bounds.y + 10.);
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    assert!(!runtime.has_slider_pair_capture());
    assert!(!runtime.cancel_slider_pair_capture().unwrap());
    assert!(
        runtime.cancel_pointer().unwrap(),
        "the ordinary contact is still held"
    );
    assert_eq!(runtime.focused(), Some(ORDINARY));
}
