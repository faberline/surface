//! Viewport placement for a Material command menu.

use surface::Rect;
use surface_app::AnimationFrame;

use crate::shape::LayoutDirection;

const MIN_WIDTH: f32 = 112.;
const MAX_WIDTH: f32 = 280.;
const VIEWPORT_INSET: f32 = 8.;
const ANCHOR_GAP: f32 = 4.;
pub(super) const MIN_ITEM_HEIGHT: f32 = 48.;

/// Visible trigger and viewport in the same logical coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuAnchor {
    pub trigger: Rect,
    pub viewport: Rect,
}

impl MenuAnchor {
    /// Read the visible part of a stable trigger from the current layout.
    pub fn from_frame(frame: &AnimationFrame<'_>, trigger_id: &str) -> Option<Self> {
        let layout = frame.layout?;
        let node = layout.nodes.get(frame.dom.by_id(trigger_id)?.index())?;
        let trigger = node.bounds.intersect(node.clip).intersect(layout.viewport);
        valid(trigger).then_some(Self {
            trigger,
            viewport: layout.viewport,
        })
    }
}

/// Where the menu was placed relative to its trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuPlacement {
    Below,
    Above,
    Trailing,
    Leading,
    Overlapping,
}

/// Bounds available for the scrollable menu and their relation to the trigger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuLayout {
    pub bounds: Rect,
    pub placement: MenuPlacement,
}

/// Position a command menu using its preferred outer width and content height.
///
/// The usual width is 112 to 280 dp. A smaller viewport constrains the width.
/// Try below, above, then the logical trailing and leading sides. A bounded
/// placement on the roomier vertical side can scroll longer content. If
/// neither side can display a row, the bounded menu may overlap the trigger.
pub fn place_menu(
    anchor: MenuAnchor,
    preferred_size: (f32, f32),
    direction: LayoutDirection,
) -> Option<MenuLayout> {
    let (preferred_width, preferred_height) = preferred_size;
    if !valid(anchor.trigger)
        || !valid(anchor.viewport)
        || !positive(preferred_width)
        || !positive(preferred_height)
    {
        return None;
    }
    let trigger = anchor.trigger.intersect(anchor.viewport);
    let safe = anchor.viewport.inset(VIEWPORT_INSET);
    if !valid(trigger) || !valid(safe) {
        return None;
    }

    let right = safe.x + safe.width;
    let bottom = safe.y + safe.height;
    let width = preferred_width.clamp(MIN_WIDTH, MAX_WIDTH).min(safe.width);
    let height = preferred_height.min(safe.height);
    let last_x = (right - width).max(safe.x);
    let last_y = (bottom - height).max(safe.y);
    let alignments = match direction {
        LayoutDirection::LeftToRight => [trigger.x, trigger.x + trigger.width - width],
        LayoutDirection::RightToLeft => [trigger.x + trigger.width - width, trigger.x],
    };
    let x = alignments
        .into_iter()
        .find(|edge| (safe.x..=last_x).contains(edge))
        .unwrap_or_else(|| alignments[0].clamp(safe.x, last_x));
    let below = (trigger.y + trigger.height + ANCHOR_GAP).max(safe.y);
    let above = (trigger.y - ANCHOR_GAP).min(bottom);
    let below_room = (bottom - below).max(0.);
    let above_room = (above - safe.y).max(0.);

    if height <= below_room {
        return Some(MenuLayout {
            bounds: Rect::new(x, below, width, height),
            placement: MenuPlacement::Below,
        });
    }
    if height <= above_room {
        return Some(MenuLayout {
            bounds: Rect::new(x, above - height, width, height),
            placement: MenuPlacement::Above,
        });
    }

    let left_side = trigger.x - ANCHOR_GAP - width;
    let right_side = trigger.x + trigger.width + ANCHOR_GAP;
    let sides = match direction {
        LayoutDirection::LeftToRight => [right_side, left_side],
        LayoutDirection::RightToLeft => [left_side, right_side],
    };
    for (edge, placement) in sides
        .into_iter()
        .zip([MenuPlacement::Trailing, MenuPlacement::Leading])
    {
        if (safe.x..=last_x).contains(&edge) {
            return Some(MenuLayout {
                bounds: Rect::new(edge, trigger.y.clamp(safe.y, last_y), width, height),
                placement,
            });
        }
    }

    let row_height = height.min(MIN_ITEM_HEIGHT);
    let (y, height, placement) = if below_room >= above_room && below_room >= row_height {
        (below, below_room, MenuPlacement::Below)
    } else if above_room >= row_height {
        (safe.y, above_room, MenuPlacement::Above)
    } else {
        (
            trigger.y.clamp(safe.y, last_y),
            height,
            MenuPlacement::Overlapping,
        )
    };
    Some(MenuLayout {
        bounds: Rect::new(x, y, width, height),
        placement,
    })
}

fn valid(rect: Rect) -> bool {
    positive(rect.width)
        && positive(rect.height)
        && [rect.x, rect.y, rect.x + rect.width, rect.y + rect.height]
            .into_iter()
            .all(f32::is_finite)
}

fn positive(value: f32) -> bool {
    value.is_finite() && value > 0.
}

#[cfg(test)]
mod tests;
