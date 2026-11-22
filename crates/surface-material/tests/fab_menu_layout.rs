use surface::Rect;
use surface_material::{
    fab_menu_edge_margin, place_fab_menu_floating, place_fab_menu_trigger, place_fab_menu_web,
    shape::LayoutDirection::{LeftToRight, RightToLeft},
    FAB_MENU_LIST_INSET,
};

#[test]
fn the_close_face_keeps_the_original_top_logical_trailing_corner() {
    for (side, bottom) in [(56., 16.), (80., 40.), (96., 56.)] {
        let ltr = Rect::new(960. - 16. - side, 800. - 16. - side, side, side);
        let rtl = Rect::new(16., ltr.y, side, side);
        let left = place_fab_menu_trigger(ltr, 56., LeftToRight).unwrap();
        let right = place_fab_menu_trigger(rtl, 56., RightToLeft).unwrap();
        assert_eq!(left, Rect::new(960. - 16. - 56., ltr.y, 56., 56.));
        assert_eq!(right, Rect::new(16., rtl.y, 56., 56.));
        assert_eq!(800. - (left.y + left.height), bottom);
    }
}

#[test]
fn large_and_extra_large_windows_use_a_24_dp_edge_margin() {
    for width in [320., 600., 840., 1199.] {
        assert_eq!(fab_menu_edge_margin(width), 16.);
    }
    for width in [1200., 1599., 1600., 1920.] {
        assert_eq!(fab_menu_edge_margin(width), 24.);
    }
}

#[test]
fn a_long_native_list_is_bounded_by_safe_areas_and_has_space_for_its_focus_rings() {
    let safe_area = Rect::new(12., 24., 296., 180.);
    let ltr = Rect::new(236., 128., 56., 56.);
    let rtl = Rect::new(28., 128., 56., 56.);
    let left = place_fab_menu_floating(ltr, safe_area, (1000., 356.), LeftToRight).unwrap();
    let right = place_fab_menu_floating(rtl, safe_area, (1000., 356.), RightToLeft).unwrap();
    assert_eq!(left, Rect::new(12., 24., 285., 101.));
    assert_eq!(right, Rect::new(23., 24., 285., 101.));
    assert_eq!(left.x + left.width - FAB_MENU_LIST_INSET, ltr.x + ltr.width);
    assert_eq!(right.x + FAB_MENU_LIST_INSET, rtl.x);
    let nearest_pill_bottom = left.y + left.height - FAB_MENU_LIST_INSET;
    assert_eq!(ltr.y - nearest_pill_bottom, 8.);
    assert_eq!(left.width - 2. * FAB_MENU_LIST_INSET, 275.);
}

#[test]
fn a_short_native_list_hugs_content_and_invalid_geometry_leaves_no_list() {
    let viewport = Rect::new(0., 0., 360., 640.);
    let anchor = Rect::new(288., 568., 56., 56.);
    assert_eq!(
        place_fab_menu_floating(anchor, viewport, (160., 116.), LeftToRight),
        Some(Rect::new(179., 439., 170., 126.))
    );
    assert_eq!(
        place_fab_menu_floating(
            Rect::new(288., 8., 56., 56.),
            viewport,
            (160., 116.),
            LeftToRight
        ),
        None
    );
    assert_eq!(
        place_fab_menu_floating(anchor, viewport, (f32::NAN, 116.), RightToLeft),
        None
    );
    assert_eq!(place_fab_menu_trigger(anchor, 0., LeftToRight), None);
}

#[test]
fn web_menu_keeps_four_dp_below_or_flips_and_preserves_safe_bounds() {
    let viewport = Rect::new(10., 20., 300., 360.);
    let top = Rect::new(234., 32., 56., 56.);
    let bottom = Rect::new(234., 292., 56., 56.);
    assert_eq!(
        place_fab_menu_web(top, viewport, (160., 170.), LeftToRight),
        Some(Rect::new(130., 92., 160., 170.))
    );
    assert_eq!(
        place_fab_menu_web(bottom, viewport, (160., 170.), LeftToRight),
        Some(Rect::new(130., 118., 160., 170.))
    );
    assert_eq!(
        place_fab_menu_web(bottom, viewport, (800., 600.), RightToLeft),
        Some(Rect::new(10., 20., 300., 268.))
    );
}
