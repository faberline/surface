//! A full-screen Material dialog with a measured header and scrollable body.

mod motion;

pub use motion::{FullScreenDialogAnchor, FullScreenDialogMotion};

use surface::{
    Align, Color, Insets, Length, MeasuredGroupLayout, Node, PaintTransform, Rect, Role, Style,
    TextAlignment, TextMeasurer,
};
use surface_app::{ModalDismissal, View};

use crate::{
    date_picker::{button, disabled_ink, Glyph},
    shape::{self, LayoutDirection},
    typography, Theme,
};

const MAX_CONTENT_RAIL: f32 = 560.;
const BODY_GUTTER: f32 = 24.;
const HEADER_SIDE: f32 = 12.;
const HEADER_HEIGHT: f32 = 56.;
const HEADER_VERTICAL_PADDING: f32 = 4.;
const HEADER_GAP: f32 = 8.;
const TOUCH_TARGET: f32 = 48.;
const CLOSE_ICON: f32 = 24.;
const ACTION_INSET: f32 = 8.;

/// Names and placement for a full-screen dialog with one header action.
///
/// Give the dialog a stable prefix. Controls use the suffixes `-close` and
/// `-action`. The dialog uses the supplied ID directly. Its header and
/// scrolling area use `-header` and `-body`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FullScreenDialogOptions<'a> {
    pub id: &'a str,
    /// Visible heading and accessible dialog name.
    pub title: &'a str,
    /// Accessible name for the scrolling group.
    pub body_label: &'a str,
    pub close_label: &'a str,
    /// Localized action caption, such as Save.
    pub action_label: &'a str,
    pub action_disabled: bool,
    /// Preferred focus target in this dialog, otherwise use the first control.
    pub initial_focus: Option<&'a str>,
    /// Fallback if the control that opened the dialog is gone.
    pub restore_focus_to: Option<&'a str>,
    /// The area to cover, in logical window coordinates.
    pub viewport: Rect,
    pub direction: LayoutDirection,
}

/// Outer bounds and available width for the caller's body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FullScreenDialogLayout {
    pub bounds: Rect,
    /// Width available after the body gutters. The actual header and body
    /// heights are measured with the app's fonts and text scale.
    pub content_width: f32,
}

/// Plan the full viewport surface and its interior column.
/// A viewport must have room for both header controls and a scrolling area.
pub fn full_screen_dialog_layout(
    options: FullScreenDialogOptions<'_>,
) -> Option<FullScreenDialogLayout> {
    let bounds = options.viewport;
    if ![
        bounds.x,
        bounds.y,
        bounds.x + bounds.width,
        bounds.y + bounds.height,
    ]
    .into_iter()
    .all(f32::is_finite)
        || bounds.width < 2. * (HEADER_SIDE + TOUCH_TARGET) + HEADER_GAP
        || bounds.height < HEADER_HEIGHT + 2. * BODY_GUTTER
    {
        return None;
    }
    Some(FullScreenDialogLayout {
        bounds,
        content_width: bounds.width.min(MAX_CONTENT_RAIL) - 2. * BODY_GUTTER,
    })
}

/// Presentation inputs for a dialog opened from a measured FAB.
pub struct FullScreenDialogTransition<'a> {
    pub open: bool,
    pub anchor: Option<FullScreenDialogAnchor>,
    pub motion: &'a FullScreenDialogMotion,
}

impl Theme {
    /// Build a modal dialog over the entire supplied viewport.
    ///
    /// The close button and Escape send `on_close`. The header action sends
    /// `on_action`. The caller owns the task, validation, and any confirmation
    /// for discarding edits. Pending composition receives Escape first and
    /// guards both header actions until it has ended.
    pub fn full_screen_dialog<M: Clone + 'static>(
        &self,
        options: FullScreenDialogOptions<'_>,
        body: View<M>,
        on_close: M,
        on_action: M,
    ) -> Option<View<M>> {
        self.full_screen_dialog_panel(options, body, on_close.clone(), on_action)
            .map(|panel| panel.modal_overlay(on_close))
    }

    /// Build a dialog whose rounded container expands from a measured FAB.
    /// Keep the body and stable IDs while the outgoing view paints. The live
    /// modal owns input on entry. Retirement restores the previous scope
    /// immediately. A missing origin uses the resting full-screen layout.
    pub fn full_screen_dialog_with_motion<M: Clone + 'static>(
        &self,
        options: FullScreenDialogOptions<'_>,
        body: View<M>,
        on_close: M,
        on_action: M,
        transition: FullScreenDialogTransition<'_>,
    ) -> Option<View<M>> {
        let viewport = options.viewport;
        let Some(anchor) = transition.anchor.filter(|anchor| anchor.fits(viewport)) else {
            return if transition.open {
                self.full_screen_dialog(options, body, on_close, on_action)
            } else {
                None
            };
        };
        if !transition.motion.should_mount(transition.open) {
            return None;
        }
        let pose = transition.motion.pose_painter(anchor, viewport);
        let panel = self
            .full_screen_dialog_panel(options, body, on_close.clone(), on_action)?
            .paint_modal_panel_transform(move || {
                let pose = pose()?;
                Some(PaintTransform {
                    source: viewport,
                    destination: pose.destination,
                    clip: pose.clip,
                    radius: pose.radius,
                    opacity: pose.content_opacity,
                })
            });
        let backdrop = Node::new(Role::Group, "")
            .id(format!("{}-transform-layer", options.id))
            .styled(
                Style::column()
                    .width(Length::Px(viewport.width))
                    .height(Length::Px(viewport.height))
                    .fixed(Insets {
                        left: Length::Px(viewport.x),
                        top: Length::Px(viewport.y),
                        ..Insets::default()
                    }),
            );
        let layer = if transition.open {
            panel.modal_with_backdrop(backdrop, on_close)
        } else {
            panel.retiring_modal_with_backdrop(backdrop)
        };
        Some(layer.paint_fill_contour(transition.motion.shell_painter(
            anchor,
            viewport,
            self.colors.surface,
        )))
    }

    fn full_screen_dialog_panel<M: Clone + 'static>(
        &self,
        options: FullScreenDialogOptions<'_>,
        body: View<M>,
        on_close: M,
        on_action: M,
    ) -> Option<View<M>> {
        let layout = full_screen_dialog_layout(options)?;
        let bounds = layout.bounds;
        let rail_width = layout.content_width + 2. * BODY_GUTTER;
        let id = |suffix| format!("{}-{suffix}", options.id);
        let rtl = options.direction == LayoutDirection::RightToLeft;
        let body = View::new(
            Node::new(Role::Group, options.body_label)
                .id(id("body"))
                .styled(
                    Style::column()
                        .width(Length::Px(rail_width))
                        .height(Length::Px(0.))
                        .grow(1.)
                        .align(if rtl { Align::End } else { Align::Start })
                        .padding(BODY_GUTTER)
                        .clip(true),
                ),
        )
        .children([body])
        .accessibility_scroll_pages();
        let surface = Style::column()
            .width(Length::Px(bounds.width))
            .height(Length::Px(bounds.height))
            .fixed(Insets {
                left: Length::Px(bounds.x),
                top: Length::Px(bounds.y),
                ..Insets::default()
            })
            .align(Align::Center)
            .background(self.colors.surface)
            .clip(true);
        let mut panel = View::new(
            Node::new(Role::Dialog, options.title)
                .id(options.id)
                .dialog_labelled_by_heading(id("title"))
                .tab_index(-1)
                .focus_color(Color::TRANSPARENT)
                .styled(surface),
        )
        .children([
            self.full_screen_dialog_header(options, rail_width, on_close, on_action),
            body,
        ]);
        if let Some(target) = options.initial_focus {
            panel = panel.modal_initial_focus(target);
        }
        if let Some(target) = options.restore_focus_to {
            panel = panel.modal_focus_fallback(target);
        }
        Some(
            panel
                .modal_dismissal(ModalDismissal::Escape)
                .block_application_shortcuts(),
        )
    }

    fn full_screen_dialog_header<M>(
        &self,
        options: FullScreenDialogOptions<'_>,
        width: f32,
        on_close: M,
        on_action: M,
    ) -> View<M> {
        let id = |suffix| format!("{}-{suffix}", options.id);
        let on_surface = self.colors.on_surface;
        let primary = self.colors.primary;
        let close = header_button(id("close"), options.close_label, on_surface, Some(on_close));
        let icon = View::new(
            Node::new(Role::Group, "")
                .id(id("close-icon"))
                .fill_contour(
                    Glyph::Close
                        .vector()
                        .contour_dp(CLOSE_ICON, on_surface)
                        .expect("the standard close icon has valid geometry"),
                ),
        )
        .without_interaction(None);
        let title = View::new(
            Node::new(Role::Heading, options.title)
                .id(id("title"))
                .text(options.title)
                .text_alignment(if options.direction == LayoutDirection::RightToLeft {
                    TextAlignment::Right
                } else {
                    TextAlignment::Left
                })
                .styled(
                    Style::column()
                        .clip(true)
                        .with_text_style(typography::TITLE_LARGE)
                        .foreground(on_surface),
                ),
        );
        let action = header_button(
            id("action"),
            options.action_label,
            primary,
            (!options.action_disabled).then_some(on_action),
        );
        let caption = View::new(
            Node::new(Role::Text, "")
                .id(id("action-label"))
                .text(options.action_label)
                .styled(
                    Style::column()
                        .clip(true)
                        .with_text_style(typography::LABEL_LARGE)
                        .foreground(if options.action_disabled {
                            disabled_ink(self)
                        } else {
                            primary
                        }),
                ),
        )
        .without_interaction(None);
        View::new(
            Node::new(Role::Group, "")
                .id(id("header"))
                .styled(
                    Style::column()
                        .width(Length::Px(width))
                        .background(self.colors.surface),
                )
                .measured_group(HeaderLayout {
                    width,
                    maximum_height: options.viewport.height - 2. * BODY_GUTTER,
                    direction: options.direction,
                }),
        )
        .children([close, icon, title, action, caption])
    }
}

fn header_button<M>(id: String, name: &str, ink: Color, message: Option<M>) -> View<M> {
    button(
        Node::new(Role::Button, name)
            .id(id)
            .styled(Style::row().corners(shape::FULL).state_layer(ink)),
        message,
    )
    .suppress_repeated_activation()
    .protect_composition_on_press()
}

#[derive(Debug)]
struct HeaderLayout {
    width: f32,
    maximum_height: f32,
    direction: LayoutDirection,
}

struct HeaderParts {
    title_size: (f32, f32),
    action_width: f32,
    caption_size: (f32, f32),
}

impl HeaderLayout {
    fn parts(width: f32, children: &[&Node], fonts: &mut dyn TextMeasurer) -> HeaderParts {
        // The measured header always contains the three semantic leaves and
        // two passive paint leaves in this order.
        let [_, _, title, _, caption] = children else {
            unreachable!("full-screen dialog header has five leaves");
        };
        let natural = fonts
            .measure_text(
                caption.text_value().unwrap_or_default(),
                &caption.style().text_style(),
                None,
            )
            .0
            .ceil();
        let half_remaining = (width - 2. * HEADER_SIDE - TOUCH_TARGET) / 2.;
        let action_width =
            (natural + 2. * ACTION_INSET).clamp(TOUCH_TARGET, half_remaining.max(TOUCH_TARGET));
        let title_width =
            (width - 2. * (HEADER_SIDE + HEADER_GAP) - TOUCH_TARGET - action_width).max(0.);
        let caption_width = natural.min(action_width - 2. * ACTION_INSET);
        HeaderParts {
            title_size: (title_width, text_height(title, title_width, fonts)),
            action_width,
            caption_size: (caption_width, text_height(caption, caption_width, fonts)),
        }
    }
}

impl MeasuredGroupLayout for HeaderLayout {
    fn measure(
        &self,
        children: &[&Node],
        available_width: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let width = available_width.map_or(self.width, |limit| self.width.min(limit));
        let parts = Self::parts(width, children, fonts);
        let height = (parts.title_size.1.max(parts.caption_size.1) + 2. * HEADER_VERTICAL_PADDING)
            .clamp(HEADER_HEIGHT, self.maximum_height);
        (width, height)
    }

    fn place(
        &self,
        children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let parts = Self::parts(width, children, fonts);
        let action_x = width - HEADER_SIDE - parts.action_width;
        let rect = |x, w, h: f32| {
            let h = h.min(height);
            let x = if self.direction == LayoutDirection::RightToLeft {
                width - x - w
            } else {
                x
            };
            Rect::new(x, (height - h) / 2., w, h)
        };
        let (title_width, title_height) = parts.title_size;
        let (caption_width, caption_height) = parts.caption_size;
        vec![
            rect(HEADER_SIDE, TOUCH_TARGET, TOUCH_TARGET),
            rect(
                HEADER_SIDE + (TOUCH_TARGET - CLOSE_ICON) / 2.,
                CLOSE_ICON,
                CLOSE_ICON,
            ),
            rect(
                HEADER_SIDE + TOUCH_TARGET + HEADER_GAP,
                title_width,
                title_height,
            ),
            rect(
                action_x,
                parts.action_width,
                (caption_height + 2. * HEADER_VERTICAL_PADDING).max(TOUCH_TARGET),
            ),
            rect(
                action_x + (parts.action_width - caption_width) / 2.,
                caption_width,
                caption_height,
            ),
        ]
    }
}

fn text_height(node: &Node, width: f32, fonts: &mut dyn TextMeasurer) -> f32 {
    if width <= 0. {
        return 0.;
    }
    let style = node.style().text_style();
    fonts
        .measure_text(node.text_value().unwrap_or_default(), &style, Some(width))
        .1
        .min(2. * style.line_height_px())
}
