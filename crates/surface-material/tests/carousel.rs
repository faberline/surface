use surface::{Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{Application, EditKey, Event, HorizontalDrag, Runtime, VerticalDrag, View};
use surface_material::{
    shape::{self, LayoutDirection},
    CarouselInput, CarouselItem, CarouselKind, CarouselLayout, CarouselOptions,
    CarouselOrientation, CarouselState, Theme,
};

fn options(kind: CarouselKind) -> CarouselOptions<'static> {
    CarouselOptions {
        id: "carousel",
        name: "精選專案",
        kind,
        orientation: if kind == CarouselKind::FullScreen {
            CarouselOrientation::Vertical
        } else {
            CarouselOrientation::Horizontal
        },
        direction: LayoutDirection::LeftToRight,
        viewport: (320., 184.),
        preferred_item_extent: 176.,
        reduced_motion: false,
    }
}

fn approximate(a: f32, b: f32) {
    assert!((a - b).abs() < 0.01, "{a} != {b}");
}

#[test]
fn contained_keylines_keep_padding_peeks_and_full_size_artwork() {
    let multi = CarouselLayout::new(options(CarouselKind::MultiBrowse), &[1.; 8]);
    let at_start = multi.slots(0.);
    let [large, medium, small, next, ..] = at_start.as_slice() else {
        unreachable!()
    };
    assert_eq!(large.bounds.x, 16.);
    assert_eq!(large.bounds.y, 8.);
    assert!(large.bounds.width > medium.bounds.width);
    assert!(medium.bounds.width > small.bounds.width);
    assert!((40. ..=56.).contains(&small.bounds.width));
    approximate(medium.bounds.x - (large.bounds.x + large.bounds.width), 8.);
    approximate(small.bounds.x - (medium.bounds.x + medium.bounds.width), 8.);
    approximate(small.bounds.x + small.bounds.width, 304.);
    assert_eq!(next.bounds.width, 0.);
    approximate(medium.artwork.width, large.artwork.width);
    approximate(small.artwork.width, large.artwork.width);
    let during = multi.slots(multi.item_offset(1).unwrap() / 2.);
    assert!(during[0].bounds.width < large.bounds.width);
    assert!(during[1].bounds.width > medium.bounds.width);
    assert_eq!(during[1].artwork.width, medium.artwork.width);
    assert!(during[1].artwork.x != medium.artwork.x);
    assert_eq!(multi.nearest_index(multi.max_offset()), Some(7));
    let end = multi.slots(multi.max_offset());
    approximate(end[7].bounds.x + end[7].bounds.width, 304.);

    let start = CarouselLayout::new(options(CarouselKind::HeroStart), &[1.; 6]);
    let frame = start.slots(0.);
    assert_eq!(frame[0].bounds.x, 16.);
    assert!(frame[0].bounds.width > frame[1].bounds.width * 2.);
    approximate(frame[1].bounds.x + frame[1].bounds.width, 304.);
    let center = CarouselLayout::new(options(CarouselKind::HeroCenter), &[1.; 6]);
    let frame = center.slots(center.item_offset(2).unwrap());
    assert!((40. ..=56.).contains(&frame[1].bounds.width));
    assert_eq!(frame[1].bounds.width, frame[3].bounds.width);
    approximate(frame[2].bounds.x + frame[2].bounds.width / 2., 160.);
    for (width, height) in [(960., 184.), (1100., 266.)] {
        for reduced_motion in [false, true] {
            let mut config = options(CarouselKind::HeroCenter);
            config.viewport = (width, height);
            config.reduced_motion = reduced_motion;
            let layout = CarouselLayout::new(config, &[1.; 12]);
            let item = layout.slots(layout.item_offset(5).unwrap())[5];
            approximate(item.bounds.x + item.bounds.width / 2., width / 2.);
            assert!(item.bounds.width <= (height - 16.) * 2.);
        }
    }
}

#[test]
fn uncontained_layout_bleeds_preserves_sizes_and_mirrors_physical_edges() {
    let mut config = options(CarouselKind::Uncontained);
    config.preferred_item_extent = 120.;
    let ltr = CarouselLayout::new(config, &[1.; 6]);
    let start = ltr.slots(0.);
    assert_eq!(start[0].bounds, Rect::new(16., 8., 120., 168.));
    assert_eq!(start[1].bounds.x, 144.);
    assert_eq!(start[2].bounds.width, 48.);
    assert_eq!(start[2].artwork.width, 120.);
    assert_eq!(ltr.slots(30.)[0].bounds, Rect::new(0., 8., 106., 168.));
    let end = ltr.slots(ltr.max_offset());
    approximate(end[5].bounds.x + end[5].bounds.width, 320.);
    config.direction = LayoutDirection::RightToLeft;
    let rtl = CarouselLayout::new(config, &[1.; 6]);
    for (left, right) in ltr.slots(60.).iter().zip(rtl.slots(60.)) {
        if left.bounds.width > 0. {
            assert_eq!(left.bounds.width, right.bounds.width);
            approximate(left.bounds.x, 320. - right.bounds.x - right.bounds.width);
        }
    }
    config.kind = CarouselKind::UncontainedMultiAspectRatio;
    config.viewport.0 = 800.;
    let varied = CarouselLayout::new(config, &[0.1, 1., 20., f32::NAN]).slots(0.);
    approximate(varied[0].artwork.width / 168., 9. / 16.);
    approximate(varied[1].artwork.width / 168., 1.);
    approximate(varied[2].artwork.width / 168., 16. / 9.);
    approximate(varied[3].artwork.width / 168., 1.);
}

#[test]
fn full_screen_and_reduced_motion_use_static_sizes() {
    let mut config = options(CarouselKind::FullScreen);
    config.viewport = (320., 560.);
    let full = CarouselLayout::new(config, &[1.; 3]);
    assert_eq!(full.slots(0.)[0].bounds, Rect::new(0., 0., 320., 560.));
    assert_eq!(full.slots(0.)[1].bounds.height, 0.);
    assert_eq!(full.item_offset(1), Some(576.));
    assert_eq!(full.slots(576.)[1].bounds, Rect::new(0., 0., 320., 560.));
    let mut state = CarouselState::default();
    state.apply(
        CarouselInput::VerticalDrag(VerticalDrag::End {
            delta_y: -300.,
            velocity_y: -4_000.,
        }),
        &full,
    );
    state.advance(Some(0.), true, &full);
    assert_eq!(
        state.active_index(&full),
        Some(1),
        "a fullscreen fling moves one page"
    );

    for kind in [
        CarouselKind::MultiBrowse,
        CarouselKind::HeroStart,
        CarouselKind::HeroCenter,
    ] {
        let mut config = options(kind);
        config.reduced_motion = true;
        let layout = CarouselLayout::new(config, &[1.; 7]);
        let initial = layout.slots(0.);
        if kind == CarouselKind::HeroCenter {
            approximate(initial[0].bounds.x + initial[0].bounds.width / 2., 160.);
        } else {
            assert_eq!(initial[0].bounds.x, 16.);
        }
        let during = layout.slots(30.);
        for (before, after) in initial.iter().zip(during) {
            assert_eq!(before.artwork.width, after.artwork.width);
            assert_eq!(after.artwork.x, before.artwork.x - 30.);
        }
        assert!(initial[1].bounds.width > 0.);
    }
}

#[test]
fn empty_and_thin_viewports_do_not_make_phantom_items_or_nonfinite_geometry() {
    for viewport in [(f32::NAN, 300.), (300., 0.), (44., 18.), (12., 4.)] {
        let mut config = options(CarouselKind::MultiBrowse);
        config.viewport = viewport;
        config.preferred_item_extent = f32::NAN;
        let layout = CarouselLayout::new(config, &[1., 1.]);
        assert!(layout.max_offset().is_finite());
        assert!(layout.slots(f32::INFINITY).iter().all(|slot| {
            [
                slot.bounds.x,
                slot.bounds.y,
                slot.bounds.width,
                slot.bounds.height,
            ]
            .into_iter()
            .all(f32::is_finite)
        }));
        if !viewport.0.is_finite() || viewport.1 == 0. {
            assert_eq!(layout.item_count(), 0);
        }
    }
    let empty = CarouselLayout::new(options(CarouselKind::HeroCenter), &[]);
    assert_eq!(empty.nearest_index(f32::NAN), None);
    let mut state = CarouselState::default();
    assert!(!state.apply(CarouselInput::GoTo(0), &empty));
}

#[test]
fn wheel_settles_drag_cancels_and_explicit_targets_survive_shared_offsets() {
    let layout = CarouselLayout::new(options(CarouselKind::MultiBrowse), &[1.; 7]);
    let mut state = CarouselState::default();
    let step = layout.item_offset(1).unwrap();
    assert!(state.apply(CarouselInput::Wheel(step * 0.7), &layout));
    assert!(state.advance(Some(0.), false, &layout).request_next);
    assert_eq!(state.offset(), step * 0.7);
    assert!(state.advance(Some(92.), false, &layout).request_next);
    let midway = state.advance(Some(150.), false, &layout);
    assert!(midway.rebuild && midway.request_next);
    assert!(state.offset() > step * 0.7 && state.offset() < step);
    assert!(!state.advance(Some(600.), false, &layout).request_next);
    assert_eq!(state.offset(), step);
    state.apply(
        CarouselInput::HorizontalDrag(HorizontalDrag::Move { delta_x: -80. }),
        &layout,
    );
    assert!(state.dragging());
    assert!(state.offset() > step);
    assert!(state.apply(
        CarouselInput::HorizontalDrag(HorizontalDrag::Cancel),
        &layout
    ));
    state.advance(None, true, &layout);
    assert!(!state.dragging());
    assert_eq!(state.offset(), step);

    let mut config = options(CarouselKind::Uncontained);
    config.preferred_item_extent = 120.;
    let free = CarouselLayout::new(config, &[1.; 6]);
    assert_eq!(free.item_offset(4), free.item_offset(5));
    state = CarouselState::default();
    state.apply(CarouselInput::Wheel(37.), &free);
    state.apply(
        CarouselInput::HorizontalDrag(HorizontalDrag::Move { delta_x: -70. }),
        &free,
    );
    state.apply(CarouselInput::HorizontalDrag(HorizontalDrag::Cancel), &free);
    state.advance(None, true, &free);
    assert_eq!(state.offset(), 37.);
    assert!(state.apply(CarouselInput::Focus(5), &free));
    assert_eq!(state.offset(), free.max_offset());
    assert_eq!(state.active_index(&free), Some(5));
    assert!(state.apply(CarouselInput::GoTo(1), &free));
    assert_eq!(state.target_index(&free), Some(1));
    assert_eq!(state.active_index(&free), Some(4));
    assert!(state.apply(
        CarouselInput::GoTo(state.target_index(&free).unwrap() + 1),
        &free
    ));
    assert_eq!(state.target_index(&free), Some(2));
    state.advance(None, true, &free);
    assert_eq!(state.active_index(&free), Some(2));

    let hero = CarouselLayout::new(options(CarouselKind::HeroStart), &[1.; 7]);
    state = CarouselState::default();
    let step = hero.item_offset(1).unwrap();
    state.apply(CarouselInput::Wheel(step * 0.3), &hero);
    assert!(state.advance(Some(0.), true, &hero).request_next);
    state.apply(CarouselInput::Wheel(step * 0.3), &hero);
    assert!(state.advance(Some(30.), true, &hero).request_next);
    assert_eq!(state.offset(), step * 0.6);
    state.apply(CarouselInput::Wheel(step * 3.), &hero);
    assert_eq!(
        state.offset(),
        step,
        "one continuous hero gesture moves at most one item"
    );
    state.advance(Some(60.), true, &hero);
    assert!(!state.advance(Some(160.), true, &hero).request_next);
    assert_eq!(state.active_index(&hero), Some(1));
}

struct Demo {
    state: CarouselState,
    direction: LayoutDirection,
    presses: usize,
}

#[derive(Clone)]
enum Message {
    Scroll(CarouselInput),
    Press,
}

impl Demo {
    fn config(&self) -> CarouselOptions<'static> {
        let mut config = options(CarouselKind::MultiBrowse);
        config.direction = self.direction;
        config
    }
}

impl Application for Demo {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Scroll(input) => {
                let layout = CarouselLayout::new(self.config(), &[1.; 6]);
                self.state.apply(input, &layout);
            }
            Message::Press => self.presses += 1,
        }
    }

    fn view(&self) -> View<Message> {
        let config = self.config();
        let layout = CarouselLayout::new(config, &[1.; 6]);
        let items = (0..6)
            .map(|index| CarouselItem {
                id: format!("item-{index}"),
                label: format!("鐵道探索，第 {} 項，共六項", index + 1),
                aspect_ratio: 1.,
                content: View::new(
                    Node::new(Role::Group, "").styled(
                        Style::column()
                            .width(Length::Percent(1.))
                            .height(Length::Percent(1.))
                            .background(Theme::dark().colors.primary_container),
                    ),
                ),
                on_activate: Message::Press,
                disabled: index == 1,
            })
            .collect();
        View::new(Node::new(Role::Window, "Projects")).children([Theme::dark().carousel(
            config,
            &layout,
            &self.state,
            items,
            Message::Scroll,
        )])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.5, size * 1.5)
    }
}

fn key(runtime: &mut Runtime<Demo>, key: EditKey) {
    runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false,
        })
        .unwrap();
    runtime.render(320., 220., &mut Metrics).unwrap();
}

#[test]
fn keyboard_reveals_offscreen_items_without_activating_and_mirrors_rtl() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let mut runtime = Runtime::new(Demo {
            state: CarouselState::default(),
            direction,
            presses: 0,
        })
        .unwrap();
        runtime.render(320., 220., &mut Metrics).unwrap();
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        assert_eq!(runtime.focused(), Some("item-0"));
        let last = runtime.dom().by_id("item-5").unwrap();
        assert!(runtime
            .accessibility_frame()
            .unwrap()
            .activation_target(last)
            .is_none());
        key(
            &mut runtime,
            if direction == LayoutDirection::LeftToRight {
                EditKey::Right
            } else {
                EditKey::Left
            },
        );
        assert_eq!(runtime.focused(), Some("item-2"));
        key(&mut runtime, EditKey::End);
        assert_eq!(runtime.focused(), Some("item-5"));
        assert_eq!(runtime.app().presses, 0);
        let last = runtime.dom().by_id("item-5").unwrap();
        let item = runtime.dom().get(last).unwrap();
        assert_eq!(item.name(), "鐵道探索，第 6 項，共六項");
        assert_eq!(item.style().corner_radii(), shape::EXTRA_LARGE);
        assert!(runtime
            .accessibility_frame()
            .unwrap()
            .activation_target(last)
            .is_some());
        runtime.dispatch(Event::Activate).unwrap();
        assert_eq!(runtime.app().presses, 1);
        key(&mut runtime, EditKey::Home);
        assert_eq!(runtime.focused(), Some("item-0"));
        key(&mut runtime, EditKey::PageDown);
        assert_eq!(runtime.focused(), Some("item-2"));
    }
}
