//! Placement in logical window coordinates for both FAB Menu presentations.

use surface::Rect;

use crate::{
    shape::LayoutDirection,
    split_button_layout::{place_split_button_popup, SplitButtonAnchor},
};

const CLOSE_GAP: f32 = 8.;
const LARGE_WINDOW: f32 = 1200.;

/// Padding for the scroll view around the floating pills' outer focus rings.
pub const FAB_MENU_LIST_INSET: f32 = 5.;

/// Default edge margin for the original closed FAB slot.
pub fn fab_menu_edge_margin(viewport_width: f32) -> f32 {
    if viewport_width >= LARGE_WINDOW {
        24.
    } else {
        16.
    }
}

/// Place the changing face against the original FAB's top trailing corner.
/// The parent retains the original slot while this square changes size.
pub fn place_fab_menu_trigger(anchor: Rect, side: f32, direction: LayoutDirection) -> Option<Rect> {
    if !valid(anchor) || !positive(side) {
        return None;
    }
    let x = match direction {
        LayoutDirection::LeftToRight => anchor.x + anchor.width - side,
        LayoutDirection::RightToLeft => anchor.x,
    };
    let face = Rect::new(x, anchor.y, side, side);
    valid(face).then_some(face)
}

/// Place the floating action list above the close face. Pass the desired
/// content size without padding. The result includes the focus-ring padding
/// on all sides and can be shorter or narrower than requested. Use a clipped
/// scroll view in this rectangle. Supply an inset viewport to respect safe
/// areas or application chrome.
pub fn place_fab_menu_floating(
    anchor: Rect,
    viewport: Rect,
    content_size: (f32, f32),
    direction: LayoutDirection,
) -> Option<Rect> {
    if !valid(anchor)
        || !valid(viewport)
        || !valid(anchor.intersect(viewport))
        || !positive(content_size.0)
        || !positive(content_size.1)
    {
        return None;
    }
    let inset = FAB_MENU_LIST_INSET;
    let viewport_right = viewport.x + viewport.width;
    let bottom = (anchor.y - CLOSE_GAP + inset).min(viewport.y + viewport.height);
    let height = (content_size.1 + 2. * inset).min(bottom - viewport.y);
    let width = content_size.0 + 2. * inset;
    let (x, width) = match direction {
        LayoutDirection::LeftToRight => {
            let right = (anchor.x + anchor.width + inset).min(viewport_right);
            let width = width.min(right - viewport.x);
            (right - width, width)
        }
        LayoutDirection::RightToLeft => {
            let left = (anchor.x - inset).max(viewport.x);
            (left, width.min(viewport_right - left))
        }
    };
    if width <= 2. * inset || height <= 2. * inset {
        return None;
    }
    Some(Rect::new(x, bottom - height, width, height))
}

/// Place the baseline Web menu with the same 4 dp gap and flip rules as the
/// split-button menu. An inset viewport bounds the result to its safe area.
pub fn place_fab_menu_web(
    anchor: Rect,
    viewport: Rect,
    content_size: (f32, f32),
    direction: LayoutDirection,
) -> Option<Rect> {
    place_split_button_popup(
        SplitButtonAnchor {
            group: anchor,
            trailing: anchor,
            viewport,
        },
        content_size,
        direction,
    )
}

fn valid(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && positive(rect.width)
        && positive(rect.height)
        && (rect.x + rect.width).is_finite()
        && (rect.y + rect.height).is_finite()
}

fn positive(value: f32) -> bool {
    value.is_finite() && value > 0.
}
