//! Place caller-owned Scroll Fields in a focused modal dialog.

use surface::{Align, Color, Insets, Length, Node, Rect, Role, Style};
use surface_app::View;

use crate::{
    date_picker::{button, text},
    shape::{self, LayoutDirection},
    typography, Elevation, Theme,
};

use super::{at, fixed, scale, FOCUS_OUTSET, TARGET};

const VIEWPORT_MARGIN: f32 = 16.;
const MAX_WIDTH: f32 = 560.;
const PANEL_PADDING: f32 = 12.;
const BODY_GAP: f32 = 12.;
const ACTION_GAP: f32 = 8.;
const ACTION_PADDING: f32 = 8.;
const ACTION_TEXT_STYLE: surface::TextStyle = typography::LABEL_LARGE;

/// Names, focus targets and size constraints for a Scroll Field modal.
///
/// Use a distinct stable prefix. The dialog, scrolling area, Cancel and Save
/// buttons have the suffixes `-dialog`, `-body`, `-cancel` and `-save`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollFieldModalOptions<'a> {
    pub id: &'a str,
    /// Visible title and accessible dialog name.
    pub label: &'a str,
    /// Accessible name for the scrolling group that contains the fields.
    pub body_label: &'a str,
    /// Short, localized caption for the Cancel button.
    pub cancel_label: &'a str,
    /// Short, localized caption for the Save button.
    pub save_label: &'a str,
    /// Preferred stable ID of a focusable field. Otherwise use the first control.
    pub initial_focus: Option<&'a str>,
    /// Stable fallback in the previous scope if the original opener is gone.
    pub restore_focus_to: Option<&'a str>,
    /// Safe area in logical window coordinates. The backdrop covers the window.
    pub viewport: Rect,
    pub direction: LayoutDirection,
    /// Preferred outer size, before applying viewport insets and the width cap.
    pub preferred_size: (f32, f32),
    /// Height needed for one complete control, including its touch target.
    /// The scroll body reserves exterior focus clearance separately.
    pub minimum_body_height: f32,
    /// The effective text scale used by the app.
    pub text_scale: f32,
}

/// Centered dialog bounds and space available to its scrollable contents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollFieldModalLayout {
    pub bounds: Rect,
    /// Available width after padding reserved for the outside focus stroke.
    pub body_width: f32,
    /// Available height after the pinned actions and scroll-area padding.
    /// The title scrolls with the fields.
    pub body_height: f32,
    pub actions_stacked: bool,
}

/// Plan a modal inside the viewport with space for the requested body
/// target and pinned actions. Return no placement if that space is unavailable.
pub fn scroll_field_modal_layout(
    options: ScrollFieldModalOptions<'_>,
) -> Option<ScrollFieldModalLayout> {
    plan(options).map(|plan| plan.layout)
}

struct ModalPlan {
    layout: ScrollFieldModalLayout,
    content_width: f32,
    actions_height: f32,
    action_widths: [f32; 2],
}

fn plan(options: ScrollFieldModalOptions<'_>) -> Option<ModalPlan> {
    let viewport = options.viewport;
    let (preferred_width, preferred_height) = options.preferred_size;
    if viewport.width <= 0.
        || viewport.height <= 0.
        || ![
            viewport.x + viewport.width,
            viewport.y + viewport.height,
            preferred_width,
            preferred_height,
            options.minimum_body_height,
        ]
        .into_iter()
        .all(f32::is_finite)
    {
        return None;
    }

    let safe = viewport.inset(VIEWPORT_MARGIN);
    let width = preferred_width.min(MAX_WIDTH).min(safe.width);
    let height = preferred_height.min(safe.height);
    let content_width = width - 2. * PANEL_PADDING;
    let body_width = content_width - 2. * FOCUS_OUTSET;
    if body_width < TARGET {
        return None;
    }

    let text_scale = scale(options.text_scale);
    let action_widths = [options.cancel_label, options.save_label]
        .map(|label| preferred_action_width(label, text_scale).min(content_width));
    let actions_stacked = action_widths.iter().sum::<f32>() + ACTION_GAP > content_width;
    let actions_height = if actions_stacked {
        2. * TARGET + ACTION_GAP
    } else {
        TARGET
    };
    let body_height = height - 2. * PANEL_PADDING - actions_height - 2. * FOCUS_OUTSET;
    if body_height < TARGET.max(options.minimum_body_height) {
        return None;
    }

    Some(ModalPlan {
        layout: ScrollFieldModalLayout {
            bounds: Rect::new(
                safe.x + (safe.width - width) / 2.,
                safe.y + (safe.height - height) / 2.,
                width,
                height,
            ),
            body_width,
            body_height,
            actions_stacked,
        },
        content_width,
        actions_height,
        action_widths,
    })
}

fn preferred_action_width(label: &str, scale: f32) -> f32 {
    // Prefer a single line for short localized verbs. Give wide scripts the
    // full font size so they stack before the rendered buttons collide.
    let ems: f32 = label
        .chars()
        .map(|character| if character.is_ascii() { 0.75 } else { 1. })
        .sum();
    (ems * ACTION_TEXT_STYLE.font_size * scale + 2. * ACTION_PADDING).max(TARGET)
}

impl Theme {
    /// Put inline Scroll Field views in a named modal with pinned Cancel and Save.
    ///
    /// Plan the field allocation with [scroll_field_modal_layout] first. Apply
    /// field events to caller-owned draft state. Save sends `on_save`; the
    /// caller validates the drafts and decides whether to commit and close.
    /// Cancel, unhandled Escape and backdrop dismissal send `on_cancel`.
    /// An active IME composition gets Escape and blocks explicit action
    /// buttons. A backdrop press can first end composition. Backdrop
    /// dismissal sends the caller's cancellation message.
    pub fn scroll_field_modal<M: Clone + 'static>(
        &self,
        options: ScrollFieldModalOptions<'_>,
        body: View<M>,
        on_cancel: M,
        on_save: M,
    ) -> Option<View<M>> {
        let plan = plan(options)?;
        let layout = plan.layout;
        let bounds = layout.bounds;
        let rtl = options.direction == LayoutDirection::RightToLeft;
        let start = if rtl { Align::End } else { Align::Start };
        let title = View::new(
            Node::new(Role::Heading, options.label)
                .id(format!("{}-title", options.id))
                .text(options.label)
                .shrink_to_fit_width()
                .styled(
                    Style {
                        max_width: Length::Px(layout.body_width),
                        ..Style::column()
                    }
                    .with_text_style(typography::HEADLINE_SMALL)
                    .foreground(self.colors.on_surface),
                ),
        );
        let body = View::new(
            Node::new(Role::Group, options.body_label)
                .id(format!("{}-body", options.id))
                .styled(
                    fixed(plan.content_width, layout.body_height + 2. * FOCUS_OUTSET)
                        .padding(FOCUS_OUTSET)
                        .gap(BODY_GAP)
                        .align(start)
                        .clip(true),
                ),
        )
        .children([title, body])
        .accessibility_scroll_pages();

        let actions_style = if layout.actions_stacked {
            fixed(plan.content_width, plan.actions_height).align(if rtl {
                Align::Start
            } else {
                Align::End
            })
        } else {
            Style::row()
                .width(Length::Px(plan.content_width))
                .height(Length::Px(plan.actions_height))
                .justify(Align::End)
        }
        .gap(ACTION_GAP);
        let actions = Node::new(Role::Group, "")
            .id(format!("{}-actions", options.id))
            .styled(actions_style);
        let actions = if rtl && !layout.actions_stacked {
            actions.reverse_row()
        } else {
            actions
        };
        let [cancel_width, save_width] = plan.action_widths;
        let caption_height =
            (ACTION_TEXT_STYLE.line_height_px() * scale(options.text_scale)).min(TARGET - 8.);
        let actions = View::new(actions).children([
            modal_action(
                self,
                format!("{}-cancel", options.id),
                options.cancel_label,
                cancel_width,
                caption_height,
                on_cancel.clone(),
            ),
            modal_action(
                self,
                format!("{}-save", options.id),
                options.save_label,
                save_width,
                caption_height,
                on_save,
            ),
        ]);

        let panel = Node::new(Role::Dialog, options.label)
            .id(format!("{}-dialog", options.id))
            .tab_index(-1)
            .focus_color(Color::TRANSPARENT)
            .styled(at(
                fixed(bounds.width, bounds.height)
                    .padding(PANEL_PADDING)
                    .align(Align::Center)
                    .corners(shape::EXTRA_LARGE)
                    .background(self.colors.surface_container_high),
                bounds.x,
                bounds.y,
            ))
            .box_shadows(self.elevation(Elevation::Level3));
        let mut panel = View::new(panel).children([body, actions]);
        if let Some(target) = options.initial_focus {
            panel = panel.modal_initial_focus(target);
        }
        if let Some(target) = options.restore_focus_to {
            panel = panel.modal_focus_fallback(target);
        }

        let Color(red, green, blue, _) = self.colors.scrim;
        let mut backdrop = Style::column()
            .fixed(Insets::all(0.))
            .background(Color(red, green, blue, 82));
        backdrop.z_index = 1000;
        Some(
            panel.modal_with_backdrop(
                Node::new(Role::Group, "")
                    .id(format!("{}-layer", options.id))
                    .styled(backdrop),
                on_cancel,
            ),
        )
    }
}

fn modal_action<M>(
    theme: &Theme,
    id: String,
    label: &str,
    width: f32,
    caption_height: f32,
    message: M,
) -> View<M> {
    let ink = theme.colors.primary;
    let caption = View::new(
        Node::new(Role::Group, "").styled(
            fixed(width - 2. * ACTION_PADDING, caption_height)
                .align(Align::Center)
                .clip(true),
        ),
    )
    .children([text("", label, ACTION_TEXT_STYLE, ink)])
    .without_interaction(None);
    button(
        Node::new(Role::Button, label)
            .id(id)
            .focus_color(theme.colors.secondary)
            .styled(
                fixed(width, TARGET)
                    .align(Align::Center)
                    .justify(Align::Center)
                    .corners(shape::FULL)
                    .state_layer(ink),
            ),
        Some(message),
    )
    .suppress_repeated_activation()
    .protect_composition_on_press()
    .children([caption])
}
