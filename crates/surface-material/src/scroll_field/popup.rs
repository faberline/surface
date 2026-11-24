//! Position an inline Scroll Field body in a nonmodal dialog.

use surface::{Align, Color, Insets, Length, Node, Rect, Role, Style, TextScaling};
use surface_app::View;

use crate::{
    shape::{self, LayoutDirection},
    Elevation, IconButtonKind, Theme,
};

use super::{shared_surface_edge, FOCUS_OUTSET, TARGET};

const VIEWPORT_MARGIN: f32 = 8.;
const ANCHOR_GAP: f32 = 4.;
const BODY_PADDING: f32 = FOCUS_OUTSET;

/// Names, anchor and preferred outer size for a nonmodal Scroll Field popup.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollFieldPopupOptions<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub body_id: &'a str,
    pub close_id: &'a str,
    pub close_label: &'a str,
    /// Opener or other stable target for focus after the popup disappears.
    pub restore_focus_to: &'a str,
    /// The visible part of the opener in logical viewport coordinates.
    pub anchor: Rect,
    /// The safe area in the same coordinate space as `anchor`.
    pub viewport: Rect,
    pub direction: LayoutDirection,
    pub preferred_size: (f32, f32),
}

/// The popup bounds and the space available to its caller-owned contents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollFieldPopupLayout {
    pub bounds: Rect,
    pub above: bool,
    /// Available width after body padding.
    pub body_width: f32,
    /// Available height after body padding and the fixed Close row.
    pub body_height: f32,
}

/// Place the popup at the opener's logical start edge.
///
/// Prefer below. Flip above if that side fits, or constrain the height to
/// the roomier side. Keep the result inside the viewport inset and clear of
/// the opener. Return no placement if the opener is hidden or the popup
/// cannot fit its Close button and a 48 dp body target.
pub fn scroll_field_popup_layout(
    options: ScrollFieldPopupOptions<'_>,
) -> Option<ScrollFieldPopupLayout> {
    if !valid_rect(options.anchor) || !valid_rect(options.viewport) {
        return None;
    }
    let anchor = options.anchor.intersect(options.viewport);
    if !valid_rect(anchor) {
        return None;
    }
    let safe = options.viewport.inset(VIEWPORT_MARGIN);
    let (preferred_width, preferred_height) = options.preferred_size;
    if !preferred_width.is_finite() || !preferred_height.is_finite() {
        return None;
    }
    let width = preferred_width.min(safe.width);
    let body_width = width - 2. * BODY_PADDING;
    if body_width < TARGET {
        return None;
    }

    let below = (anchor.y + anchor.height + ANCHOR_GAP).max(safe.y);
    let above = (anchor.y - ANCHOR_GAP).min(safe.y + safe.height);
    let room_below = (safe.y + safe.height - below).max(0.);
    let room_above = (above - safe.y).max(0.);
    let use_above = room_below < preferred_height
        && (room_above >= preferred_height || room_above > room_below);
    let room = if use_above { room_above } else { room_below };
    let height = preferred_height.min(room);
    let body_height = height - 2. * BODY_PADDING - TARGET;
    if body_height < TARGET {
        return None;
    }

    let x = match options.direction {
        LayoutDirection::LeftToRight => anchor.x,
        LayoutDirection::RightToLeft => anchor.x + anchor.width - width,
    }
    .clamp(safe.x, (safe.x + safe.width - width).max(safe.x));
    let y = if use_above { above - height } else { below };
    Some(ScrollFieldPopupLayout {
        bounds: Rect::new(x, y, width, height),
        above: use_above,
        body_width,
        body_height,
    })
}

impl Theme {
    /// Build a fixed, nonmodal popup from a caller-owned Scroll Field body.
    ///
    /// Plan the body with [scroll_field_popup_layout] before creating it.
    /// Keep the popup after its opener in the view so Tab can enter and leave
    /// in the normal order. Use the same opener as `restore_focus_to`.
    /// Close and unhandled Escape inside the popup send `on_close`. The
    /// caller owns visibility and what happens to any pending drafts.
    pub fn scroll_field_popup<M: Clone + 'static>(
        &self,
        options: ScrollFieldPopupOptions<'_>,
        body: View<M>,
        on_close: M,
    ) -> Option<View<M>> {
        let layout = scroll_field_popup_layout(options)?;
        let bounds = layout.bounds;
        let rtl = options.direction == LayoutDirection::RightToLeft;
        let body = View::new(
            Node::new(Role::Group, "").id(options.body_id).styled(
                Style::column()
                    .width(Length::Px(bounds.width))
                    .height(Length::Px(bounds.height - TARGET))
                    .padding(BODY_PADDING)
                    .align(if rtl { Align::End } else { Align::Start })
                    .clip(true),
            ),
        )
        .children([body])
        .accessibility_scroll_pages();
        let close = self
            .icon_button_with_text_scaling(
                options.close_id,
                options.close_label,
                ("×", TextScaling::Fixed),
                IconButtonKind::Standard,
                None,
                false,
                on_close.clone(),
            )
            .accessible_activation()
            .protect_composition_on_press();
        let footer = View::new(
            Node::new(Role::Group, "").styled(
                Style::row()
                    .width(Length::Px(layout.body_width))
                    .height(Length::Px(TARGET))
                    .justify(if rtl { Align::Start } else { Align::End }),
            ),
        )
        .children([close]);

        let mut style = Style::column()
            .width(Length::Px(bounds.width))
            .height(Length::Px(bounds.height))
            .align(Align::Center)
            .corners(shape::LARGE)
            .background(self.colors.surface_container)
            .fixed(Insets {
                left: Length::Px(bounds.x),
                top: Length::Px(bounds.y),
                ..Insets::default()
            });
        style.z_index = 1000;
        let style = shared_surface_edge(style, self.colors.surface, self.colors.outline);
        let panel = Node::new(Role::Dialog, options.label)
            .id(options.id)
            // Shield blank panel space while keeping it out of Tab order.
            .tab_index(-1)
            .focus_color(Color::TRANSPARENT)
            .styled(style)
            .box_shadows(self.elevation(Elevation::Level2));
        Some(
            View::new(panel)
                .children([body, footer])
                .restore_focus_in_subtree_on_removal(options.restore_focus_to)
                .on_cancel_within(on_close),
        )
    }
}

fn valid_rect(rect: Rect) -> bool {
    rect.width > 0.
        && rect.height > 0.
        && rect.x.is_finite()
        && rect.y.is_finite()
        && (rect.x + rect.width).is_finite()
        && (rect.y + rect.height).is_finite()
}
