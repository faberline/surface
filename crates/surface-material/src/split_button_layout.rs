//! Geometry shared by the built-in menu and caller-owned split button popups.

use surface::Rect;

use crate::shape::LayoutDirection;

const POPUP_GAP: f32 = 4.;

/// Visible faces and viewport in logical window coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitButtonAnchor {
    pub group: Rect,
    pub trailing: Rect,
    pub viewport: Rect,
}

/// Place a popup near the trailing button. Prefer its logical start edge,
/// then its logical end edge. Keep 4 dp between the popup and the faces
/// when the viewport permits, using the other vertical side when needed.
/// The result may be smaller than the requested size on a short viewport.
pub fn place_split_button_popup(
    anchor: SplitButtonAnchor,
    size: (f32, f32),
    direction: LayoutDirection,
) -> Option<Rect> {
    let SplitButtonAnchor {
        group,
        trailing,
        viewport,
    } = anchor;
    if ![group, trailing, viewport].into_iter().all(valid) || !positive(size.0) || !positive(size.1)
    {
        return None;
    }
    let group = group.intersect(viewport);
    let trailing = trailing.intersect(viewport);
    if !valid(group) || !valid(trailing) {
        return None;
    }
    let width = size.0.min(viewport.width);
    let max_x = viewport.x + viewport.width - width;
    let (group_left, group_right) = (group.x, group.x + group.width - width);
    let (trigger_left, trigger_right) = (trailing.x, trailing.x + trailing.width - width);
    let alignments = match direction {
        LayoutDirection::LeftToRight => [trigger_left, trigger_right, group_left, group_right],
        LayoutDirection::RightToLeft => [trigger_right, trigger_left, group_right, group_left],
    };
    let x = alignments
        .into_iter()
        .find(|position| (viewport.x..=max_x).contains(position))
        .unwrap_or_else(|| alignments[0].clamp(viewport.x, max_x));

    let below = group.y + group.height + POPUP_GAP;
    let above = group.y - POPUP_GAP;
    let room_below = (viewport.y + viewport.height - below).max(0.);
    let room_above = (above - viewport.y).max(0.);
    if size.1 <= room_below || room_below >= room_above && room_below > 0. {
        return Some(Rect::new(x, below, width, size.1.min(room_below)));
    }
    if room_above > 0. {
        let height = size.1.min(room_above);
        return Some(Rect::new(x, above - height, width, height));
    }
    // The faces can fill the viewport at a very large text scale. Keep a
    // bounded popup available when neither vertical side has usable space.
    Some(Rect::new(x, viewport.y, width, size.1.min(viewport.height)))
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
