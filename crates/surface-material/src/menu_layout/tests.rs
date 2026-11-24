use super::*;
use crate::shape::LayoutDirection::{LeftToRight, RightToLeft};
use surface::{Insets, Length, Node, Role, Style};
use surface_app::{Application, Runtime, View};
use surface_wgpu::TextSystem;

#[test]
fn places_below_and_applies_the_minimum_width() {
    let anchor = MenuAnchor {
        trigger: Rect::new(50., 70., 48., 44.),
        viewport: Rect::new(10., 20., 340., 360.),
    };
    let layout = place_menu(anchor, (80., 96.), LeftToRight).unwrap();
    assert_eq!(layout.placement, MenuPlacement::Below);
    assert_eq!(layout.bounds, Rect::new(50., 118., 112., 96.));
}

#[test]
fn places_above_and_aligns_to_a_fitting_trigger_edge() {
    let anchor = MenuAnchor {
        trigger: Rect::new(300., 335., 40., 30.),
        viewport: Rect::new(10., 20., 340., 360.),
    };
    for direction in [LeftToRight, RightToLeft] {
        let layout = place_menu(anchor, (180., 120.), direction).unwrap();
        assert_eq!(layout.placement, MenuPlacement::Above);
        assert_eq!(layout.bounds, Rect::new(160., 211., 180., 120.));
    }
}

#[test]
fn repositions_to_either_horizontal_side_and_clamps_its_height() {
    let mut anchor = MenuAnchor {
        trigger: Rect::new(260., 120., 80., 48.),
        viewport: Rect::new(0., 0., 600., 320.),
    };
    for (direction, x) in [(LeftToRight, 344.), (RightToLeft, 56.)] {
        let layout = place_menu(anchor, (200., 400.), direction).unwrap();
        assert_eq!(layout.placement, MenuPlacement::Trailing);
        assert_eq!(layout.bounds, Rect::new(x, 8., 200., 304.));
    }

    anchor.trigger.x = 480.;
    let layout = place_menu(anchor, (200., 240.), LeftToRight).unwrap();
    assert_eq!(layout.placement, MenuPlacement::Leading);
    assert_eq!(layout.bounds, Rect::new(276., 72., 200., 240.));
}

#[test]
fn bounds_a_long_menu_on_the_roomier_side_in_a_narrow_viewport() {
    let anchor = MenuAnchor {
        trigger: Rect::new(60., 95., 110., 48.),
        viewport: Rect::new(0., 0., 230., 245.),
    };
    let layout = place_menu(anchor, (400., 300.), RightToLeft).unwrap();
    assert_eq!(layout.placement, MenuPlacement::Below);
    assert_eq!(layout.bounds, Rect::new(8., 147., 214., 90.));

    let spacious = MenuAnchor {
        viewport: Rect::new(0., 0., 620., 800.),
        ..anchor
    };
    assert_eq!(
        place_menu(spacious, (400., 50.), LeftToRight)
            .unwrap()
            .bounds
            .width,
        280.
    );
}

#[test]
fn uses_the_viewport_when_adjacent_space_is_too_small_for_a_row() {
    let anchor = MenuAnchor {
        trigger: Rect::new(25., 27., 118., 70.),
        viewport: Rect::new(0., 0., 168., 120.),
    };
    let layout = place_menu(anchor, (112., 200.), LeftToRight).unwrap();
    assert_eq!(layout.placement, MenuPlacement::Overlapping);
    assert_eq!(layout.bounds, Rect::new(25., 8., 112., 104.));
}

#[test]
fn uses_the_visible_trigger_and_rejects_unusable_geometry() {
    let mut anchor = MenuAnchor {
        trigger: Rect::new(-20., -12., 48., 30.),
        viewport: Rect::new(0., 0., 360., 300.),
    };
    let layout = place_menu(anchor, (160., 80.), LeftToRight).unwrap();
    assert_eq!(layout.bounds, Rect::new(8., 22., 160., 80.));
    assert!(place_menu(anchor, (f32::NAN, 80.), LeftToRight).is_none());
    anchor.trigger.x = 362.;
    assert!(place_menu(anchor, (160., 80.), LeftToRight).is_none());
    anchor.trigger.x = f32::INFINITY;
    assert!(place_menu(anchor, (160., 80.), LeftToRight).is_none());
}

#[test]
fn reads_the_clipped_trigger_from_the_active_layout() {
    struct Host;
    impl Application for Host {
        type Message = ();

        fn update(&mut self, _: ()) {}

        fn view(&self) -> View<()> {
            let trigger = |id: &str, x| {
                View::new(
                    Node::new(Role::Button, id).id(id).styled(
                        Style::column()
                            .width(Length::Px(48.))
                            .height(Length::Px(40.))
                            .fixed(Insets {
                                left: Length::Px(x),
                                top: Length::Px(20.),
                                ..Insets::default()
                            }),
                    ),
                )
            };
            View::new(Node::new(Role::Window, "Menu anchors").styled(Style::column().clip(true)))
                .children([trigger("partly-visible", -12.), trigger("hidden", 400.)])
        }
    }

    let mut runtime = Runtime::new(Host).unwrap();
    runtime
        .render(360., 300., &mut TextSystem::default())
        .unwrap();
    let frame = AnimationFrame {
        now_ms: None,
        reduced_motion: false,
        dom: runtime.dom(),
        layout: runtime.computed(),
    };
    assert_eq!(
        MenuAnchor::from_frame(&frame, "partly-visible"),
        Some(MenuAnchor {
            trigger: Rect::new(0., 20., 36., 40.),
            viewport: Rect::new(0., 0., 360., 300.),
        })
    );
    assert!(MenuAnchor::from_frame(&frame, "hidden").is_none());
    assert!(MenuAnchor::from_frame(&frame, "removed").is_none());
}
