//! Material 3 Basic dialogs with separate heading, scroll body and actions.

use surface::{
    Align, Color, Insets, Length, MeasuredGroupLayout, Node, Rect, Role, Style, TextAlignment,
    TextMeasurer, TextScaling,
};
use surface_app::{CursorStyle, ModalDismissal, View};

use crate::{shape, shape::LayoutDirection, typography, Elevation, Theme};

mod motion;
pub use motion::BasicDialogMotion;

const MIN_WIDTH: f32 = 280.;
const MAX_WIDTH: f32 = 560.;
const CONTENT_INSET: f32 = 24.;
const SECTION_GAP: f32 = 16.;
const ACTION_GAP: f32 = 8.;
const ACTION_SIDE_INSET: f32 = 12.;
const ACTION_VERTICAL_INSET: f32 = 4.;
const TOUCH_TARGET: f32 = 48.;

/// Names, presentation and focus for a Basic dialog.
#[derive(Clone, Copy, Debug)]
pub struct BasicDialogOptions<'a> {
    /// Stable prefix for the panel and its slots.
    pub id: &'a str,
    /// Accessible name of the dialog, including when the heading is omitted.
    pub label: &'a str,
    pub heading: Option<&'a str>,
    pub supporting: Option<&'a str>,
    pub icon: Option<&'a str>,
    /// Current window dimensions in logical pixels.
    pub window_size: (f32, f32),
    /// Preferred outer width. A Basic dialog normally uses 280 to 560 dp.
    pub preferred_width: f32,
    pub direction: LayoutDirection,
    pub dismissal: ModalDismissal,
    /// Publish an urgent alert role when the message requires attention.
    /// The dismissal policy remains a separate choice.
    pub alert: bool,
    pub initial_focus: Option<&'a str>,
    pub restore_focus_to: Option<&'a str>,
}

/// Body views and actions authored by the application.
pub struct BasicDialogContent<'a, M> {
    pub extra: Vec<View<M>>,
    /// The sole acknowledgement or the positive choice of a two-action dialog.
    pub confirm: (&'a str, M),
    pub cancel: Option<(&'a str, M)>,
    /// The application message for an allowed Escape or backdrop dismissal.
    pub on_dismiss: M,
}

impl Theme {
    /// Build a Basic dialog with a scrollable middle region. Mount it beside
    /// the page. Keep the same prefix and body IDs through resize and rebuilds.
    /// The heading and actions stay above and below the scrolling region.
    /// Action labels are measured at the current text scale before placement.
    /// The Tab order stays stable when a narrow layout stacks the actions.
    ///
    /// This reserves `{id}-layer`, `{id}-heading`, `{id}-body`,
    /// `{id}-actions`, `{id}-confirm` and `{id}-cancel`.
    pub fn basic_dialog<M>(
        &self,
        options: BasicDialogOptions<'_>,
        content: BasicDialogContent<'_, M>,
    ) -> Option<View<M>> {
        self.build_basic_dialog(options, content, None)
    }

    /// Build the current live or retiring frame of a Basic dialog.
    ///
    /// Keep the same IDs and authored layer position until the exit ends.
    /// While retiring, the content only contributes to paint. The caller
    /// owns all dismissal and action messages. The fade is rendered via
    /// `Runtime::render_extended`.
    pub fn basic_dialog_with_motion<M>(
        &self,
        options: BasicDialogOptions<'_>,
        content: BasicDialogContent<'_, M>,
        open: bool,
        motion: &BasicDialogMotion,
    ) -> Option<View<M>> {
        motion
            .should_mount(open)
            .then(|| self.build_basic_dialog(options, content, Some((open, motion))))
            .flatten()
    }

    fn build_basic_dialog<M>(
        &self,
        options: BasicDialogOptions<'_>,
        content: BasicDialogContent<'_, M>,
        motion: Option<(bool, &BasicDialogMotion)>,
    ) -> Option<View<M>> {
        let (window_width, window_height) = options.window_size;
        if [window_width, window_height, options.preferred_width]
            .into_iter()
            .any(|extent| !extent.is_finite() || extent <= 0.)
        {
            return None;
        }
        let margin_x = CONTENT_INSET.min(((window_width - MIN_WIDTH) / 2.).max(0.));
        let width = options
            .preferred_width
            .clamp(MIN_WIDTH, MAX_WIDTH)
            .min(window_width - 2. * margin_x);
        let margin_y = CONTENT_INSET.min((window_height / 8.).floor());
        let max_height = window_height - 2. * margin_y;
        let inset = CONTENT_INSET.min(width / 4.).min(max_height / 8.);
        let id = |suffix| format!("{}-{suffix}", options.id);
        let start = if options.direction == LayoutDirection::RightToLeft {
            TextAlignment::Right
        } else {
            TextAlignment::Left
        };

        let mut children = Vec::new();
        if let Some(glyph) = options.icon {
            let mut icon = Style::column()
                .width(Length::Px(24.))
                .height(Length::Px(24.))
                .font_size(24.)
                .line_height(24.)
                .foreground(self.colors.secondary);
            icon.align = Align::Center;
            children.push(
                View::new(
                    Node::new(Role::Group, "")
                        .id(id("icon"))
                        .text(glyph)
                        .text_scaling(TextScaling::Fixed)
                        .styled(icon),
                )
                .without_interaction(None),
            );
            if options.heading.is_some() {
                children.push(gap(SECTION_GAP));
            }
        }
        if let Some(heading) = options.heading {
            let mut style = Style::column()
                .width(Length::Percent(1.))
                .with_text_style(typography::HEADLINE_SMALL)
                .foreground(self.colors.on_surface);
            style.max_width = Length::Percent(1.);
            children.push(View::new(
                Node::new(Role::Heading, heading)
                    .id(id("heading"))
                    .text(heading)
                    .text_alignment(if options.icon.is_some() {
                        TextAlignment::Center
                    } else {
                        start
                    })
                    .styled(style),
            ));
        }
        let status_text = options.supporting.filter(|text| !text.is_empty());
        let has_body = status_text.is_some() || !content.extra.is_empty();
        if has_body {
            if !children.is_empty() {
                children.push(gap(SECTION_GAP));
            }
            let supporting = status_text.map(|text| {
                View::new(
                    Node::new(Role::Text, text)
                        .id(id("supporting"))
                        .text(text)
                        .text_alignment(start)
                        .styled(
                            Style::column()
                                .width(Length::Percent(1.))
                                .with_text_style(typography::BODY_MEDIUM)
                                .foreground(self.colors.on_surface_variant),
                        ),
                )
            });
            children.push(
                View::new(
                    Node::new(Role::Group, "")
                        .id(id("body"))
                        .shrink_to_fit_height()
                        .styled(
                            Style::column()
                                .width(Length::Percent(1.))
                                .gap(SECTION_GAP)
                                .clip(true),
                        ),
                )
                .children(supporting.into_iter().chain(content.extra))
                .accessibility_scroll_pages(),
            );
        }
        if !children.is_empty() {
            children.push(gap(CONTENT_INSET));
        }
        let actions = content
            .cancel
            .map(|action| self.basic_dialog_action(&id("cancel"), action))
            .into_iter()
            .chain(std::iter::once(
                self.basic_dialog_action(&id("confirm"), content.confirm),
            ))
            .flatten();
        children.push(
            View::new(
                Node::new(Role::Group, "")
                    .id(id("actions"))
                    .styled(Style::column().width(Length::Percent(1.)))
                    .measured_group(ActionLayout {
                        rtl: options.direction == LayoutDirection::RightToLeft,
                    }),
            )
            .children(actions),
        );

        let mut style = Style::column()
            .width(Length::Px(width))
            .padding(inset)
            .corners(shape::EXTRA_LARGE)
            .background(self.colors.surface_container_high)
            .clip(true);
        style.max_height = Length::Px(max_height);
        if options.icon.is_some() {
            style.align = Align::Center;
        }
        let mut panel = Node::new(Role::Dialog, options.label)
            .id(options.id)
            .tab_index(-1)
            .focus_color(Color::TRANSPARENT)
            .styled(style)
            .box_shadows(self.elevation(Elevation::Level3));
        if options.alert {
            panel = panel.alert_dialog();
        }
        if options.heading.is_some() {
            panel = panel.dialog_labelled_by_heading(id("heading"));
        }
        if status_text.is_some() {
            panel = panel.described_by_text(id("supporting"));
        }
        let mut panel = View::new(panel)
            .children(children)
            .modal_dismissal(options.dismissal)
            .block_application_shortcuts();
        if let Some(target) = options.initial_focus {
            panel = panel.modal_initial_focus(target);
        }
        if let Some(target) = options.restore_focus_to {
            panel = panel.modal_focus_fallback(target);
        }
        let Color(red, green, blue, _) = self.colors.scrim;
        let scrim = Color(red, green, blue, 82);
        let mut backdrop = Style::column()
            .fixed(Insets::all(0.))
            .align(Align::Center)
            .justify(Align::Center);
        backdrop.z_index = 1000;
        let backdrop_node = Node::new(Role::Group, "").id(id("layer"));
        if let Some((open, motion)) = motion {
            let panel = panel.paint_modal_panel_opacity(motion.panel_painter());
            let backdrop = backdrop_node.styled(backdrop);
            let layer = if open {
                panel.modal_with_backdrop(backdrop, content.on_dismiss)
            } else {
                panel.retiring_modal_with_backdrop(backdrop)
            };
            return Some(
                layer.paint_fill_contour(motion.scrim_painter(options.window_size, scrim)),
            );
        }
        Some(panel.modal_with_backdrop(
            backdrop_node.styled(backdrop.background(scrim)),
            content.on_dismiss,
        ))
    }

    fn basic_dialog_action<M>(&self, id: &str, (label, action): (&str, M)) -> [View<M>; 2] {
        let ink = self.colors.primary;
        [
            View::new(
                Node::new(Role::Button, label).id(id).styled(
                    Style::column()
                        .corners(shape::FULL)
                        .foreground(ink)
                        .state_layer(ink),
                ),
            )
            .on_press(action)
            .accessible_activation()
            .protect_composition_on_press()
            .cursor(CursorStyle::Pointer),
            View::new(
                Node::new(Role::Group, "").text(label).styled(
                    Style::column()
                        .with_text_style(typography::LABEL_LARGE)
                        .foreground(ink),
                ),
            )
            .without_interaction(None),
        ]
    }
}

fn gap<M>(height: f32) -> View<M> {
    View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(height))))
}

#[derive(Debug)]
struct ActionLayout {
    rtl: bool,
}

impl MeasuredGroupLayout for ActionLayout {
    fn measure(
        &self,
        children: &[&Node],
        available_width: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let width = available_width.unwrap_or(MAX_WIDTH - 2. * CONTENT_INSET);
        let (_, height) = self.arrange(children, width, fonts);
        (width, height)
    }

    fn place(
        &self,
        children: &[&Node],
        (width, _): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        self.arrange(children, width, fonts).0
    }
}

impl ActionLayout {
    fn arrange(
        &self,
        children: &[&Node],
        width: f32,
        fonts: &mut dyn TextMeasurer,
    ) -> (Vec<Rect>, f32) {
        let metrics: Vec<_> = children
            .chunks_exact(2)
            .map(|pair| {
                let label = pair[1];
                let text = label.text_value().unwrap_or_default();
                let style = label.style().text_style();
                let desired = fonts.measure_text(text, &style, None).0;
                let side = ACTION_SIDE_INSET.min(width / 4.);
                let button_width = (desired + 2. * side).max(TOUCH_TARGET).min(width);
                let line_width = (button_width - 2. * side).max(0.);
                let (label_width, label_height) =
                    fonts.measure_text(text, &style, Some(line_width));
                let height = (label_height + 2. * ACTION_VERTICAL_INSET).max(TOUCH_TARGET);
                (
                    button_width,
                    height,
                    label_width.min(line_width),
                    label_height,
                )
            })
            .collect();
        let row_width = metrics.iter().map(|item| item.0).sum::<f32>()
            + ACTION_GAP * (metrics.len() - 1) as f32;
        let row_height = metrics.iter().map(|item| item.1).fold(0., f32::max);
        let stacked = row_width > width;
        let height = if stacked {
            metrics.iter().map(|item| item.1).sum::<f32>() + ACTION_GAP
        } else {
            row_height
        };
        let mut next_x = width - row_width;
        let mut placed = Vec::with_capacity(children.len());
        for (index, &(button_width, button_height, label_width, label_height)) in
            metrics.iter().enumerate()
        {
            let x = if stacked {
                width - button_width
            } else {
                next_x
            };
            let y = if stacked && index == 0 {
                metrics[1].1 + ACTION_GAP
            } else if stacked {
                0.
            } else {
                (row_height - button_height) / 2.
            };
            let x = if self.rtl {
                width - x - button_width
            } else {
                x
            };
            placed.push(Rect::new(x, y, button_width, button_height));
            placed.push(Rect::new(
                x + (button_width - label_width) / 2.,
                y + (button_height - label_height) / 2.,
                label_width,
                label_height,
            ));
            next_x += button_width + ACTION_GAP;
        }
        (placed, height)
    }
}
