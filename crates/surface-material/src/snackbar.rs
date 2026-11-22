//! Material 3 Snackbar visuals. The application owns presence and responses.

use crate::{shape, shape::LayoutDirection, typography, Elevation, Theme};
use surface::{
    Color, FillContour, Length, MeasuredGroupLayout, Node, Rect, Role, Style, TextMeasurer,
};
use surface_app::{CursorStyle, View};

const MAX_WIDTH: f32 = 600.;
const TOUCH_TARGET: f32 = 48.;
const CLOSE_ICON_SIZE: f32 = 24.;
const SINGLE_LINE_HEIGHT: f32 = 48.;
const MULTILINE_HEIGHT: f32 = 68.;
const TEXT_SIDE_PADDING: f32 = 16.;
const TEXT_VERTICAL_PADDING: f32 = 14.;
const ACTION_SIDE_PADDING: f32 = 12.;
const ACTION_VERTICAL_PADDING: f32 = 8.;
const END_PADDING: f32 = 8.;
const CONTROL_GAP: f32 = 8.;
const STACK_BOTTOM_PADDING: f32 = 2.;

// Close icon path in its 24 dp viewport.
const CLOSE_ICON: [[f32; 2]; 12] = [
    [5., 6.4],
    [6.4, 5.],
    [12., 10.6],
    [17.6, 5.],
    [19., 6.4],
    [13.4, 12.],
    [19., 17.6],
    [17.6, 19.],
    [12., 13.4],
    [6.4, 19.],
    [5., 17.6],
    [10.6, 12.],
];

/// Optional layout and focus settings for a Material Snackbar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SnackbarOptions<'a> {
    /// Direction of the visual layout. Focus follows the source order.
    pub direction: LayoutDirection,
    /// Put the action beneath the message even when both fit on one row.
    pub action_on_new_line: bool,
    /// When a focused Snackbar control is removed, move focus to this ID
    /// if the target is enabled and in the active focus scope.
    pub restore_focus_to: Option<&'a str>,
}

impl Theme {
    /// Show a passive notification with an optional text action and close icon.
    ///
    /// The app decides when to place and remove the view. A Snackbar with an
    /// action stays available until the user makes a choice or takes another
    /// action. Moving focus alone is not an action. The close label is
    /// localized accessible text for the icon.
    pub fn snackbar<M>(
        &self,
        id: &str,
        message: &str,
        action: Option<(&str, M)>,
        dismiss: Option<(&str, M)>,
    ) -> View<M> {
        self.snackbar_with(id, message, action, dismiss, SnackbarOptions::default())
    }

    /// Build a Snackbar without opening a focus scope. Put it at the bottom
    /// of the application layout, above any bottom navigation.
    ///
    /// The message uses `{id}-text`. The separate controls use
    /// `{id}-action` and `{id}-dismiss`. The default layout moves the controls
    /// below the message when the inline layout cannot fit. The message and
    /// action can grow vertically so that large text stays readable.
    pub fn snackbar_with<M>(
        &self,
        id: &str,
        message: &str,
        action: Option<(&str, M)>,
        dismiss: Option<(&str, M)>,
        options: SnackbarOptions<'_>,
    ) -> View<M> {
        let mut surface = Style::column()
            .width(Length::Percent(1.))
            .background(self.colors.inverse_surface)
            .foreground(self.colors.inverse_on_surface)
            .corners(shape::EXTRA_SMALL)
            .clip(true);
        surface.max_width = Length::Px(MAX_WIDTH);
        surface.min_height = SINGLE_LINE_HEIGHT;

        let layout = SnackbarLayout {
            direction: options.direction,
            action_on_new_line: options.action_on_new_line,
            has_action: action.is_some(),
            has_dismiss: dismiss.is_some(),
        };
        let mut children = vec![View::new(
            Node::new(Role::Status, message)
                .id(format!("{id}-text"))
                .text(message)
                .styled(
                    Style::default()
                        .with_text_style(typography::BODY_MEDIUM)
                        .foreground(self.colors.inverse_on_surface),
                ),
        )];
        if let Some((label, message)) = action {
            let ink = self.colors.inverse_primary;
            children.push(control(
                format!("{id}-action"),
                label,
                ink,
                message,
                options,
            ));
            // The button owns the accessible label. This leaf only paints it
            // at the measured center of the larger hit target.
            children.push(
                View::new(
                    Node::new(Role::Group, "").text(label).styled(
                        Style::default()
                            .with_text_style(typography::LABEL_LARGE)
                            .foreground(ink),
                    ),
                )
                .without_interaction(None),
            );
        }
        if let Some((label, message)) = dismiss {
            let ink = self.colors.inverse_on_surface;
            children.push(control(
                format!("{id}-dismiss"),
                label,
                ink,
                message,
                options,
            ));
            children.push(
                View::new(Node::new(Role::Group, "").fill_contour(
                    FillContour::new(CLOSE_ICON, ink).expect("valid close icon contour"),
                ))
                .without_interaction(None),
            );
        }
        View::new(
            Node::new(Role::Group, "")
                .id(id)
                .styled(surface)
                .box_shadows(self.elevation(Elevation::Level3))
                .measured_group(layout),
        )
        .children(children)
    }
}

fn control<M>(
    id: String,
    label: &str,
    ink: Color,
    message: M,
    options: SnackbarOptions<'_>,
) -> View<M> {
    let view = View::new(
        Node::new(Role::Button, label).id(id).styled(
            Style::default()
                .foreground(ink)
                .corners(shape::FULL)
                .state_layer(ink),
        ),
    )
    .on_press(message)
    .accessible_activation()
    .cursor(CursorStyle::Pointer);
    match options.restore_focus_to {
        Some(id) => view.restore_focus_on_removal(id),
        None => view,
    }
}

#[derive(Debug)]
struct SnackbarLayout {
    direction: LayoutDirection,
    action_on_new_line: bool,
    has_action: bool,
    has_dismiss: bool,
}

struct Geometry {
    height: f32,
    message: Rect,
    action: Option<[Rect; 2]>,
    dismiss: Option<[Rect; 2]>,
}

impl MeasuredGroupLayout for SnackbarLayout {
    fn measure(
        &self,
        children: &[&Node],
        available_width: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let width = available_width.unwrap_or(MAX_WIDTH).min(MAX_WIDTH);
        (width, self.geometry(width, children, fonts).height)
    }

    fn place(
        &self,
        children: &[&Node],
        (width, _): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let geometry = self.geometry(width, children, fonts);
        std::iter::once(geometry.message)
            .chain(geometry.action.into_iter().flatten())
            .chain(geometry.dismiss.into_iter().flatten())
            .map(|mut rect| {
                if self.direction == LayoutDirection::RightToLeft {
                    rect.x = width - rect.x - rect.width;
                }
                rect
            })
            .collect()
    }
}

impl SnackbarLayout {
    fn geometry(&self, width: f32, children: &[&Node], fonts: &mut dyn TextMeasurer) -> Geometry {
        let body = children[0];
        let action_label = self.has_action.then(|| children[2]);

        let text_inset = TEXT_SIDE_PADDING.min(width / 4.);
        let end = if self.has_dismiss || !self.has_action {
            width
        } else {
            width - END_PADDING.min((width - TOUCH_TARGET).max(0.))
        };
        let dismiss_width = if self.has_dismiss {
            TOUCH_TARGET.min(width)
        } else {
            0.
        };
        let (action_width, action_multiline) = action_label.map_or((0., false), |label| {
            let (w, h) = text_size(label, fonts, None);
            (
                (w + 2. * ACTION_SIDE_PADDING).max(TOUCH_TARGET),
                h > label.style().text_style().line_height_px(),
            )
        });
        let gap = if self.has_action || self.has_dismiss {
            CONTROL_GAP
        } else {
            text_inset
        };
        let after_close = end - dismiss_width;
        let inline_text_limit = (after_close - action_width - text_inset - gap).max(0.);
        let (inline_width, inline_height) = text_size(body, fonts, Some(inline_text_limit));

        let body_line = body.style().text_style().line_height_px();
        // Keep two readable message lines next to a compact action. If the
        // action dominates the row or leaves no room for one glyph, give the
        // message its own row and wrap using the actual text metrics.
        let stack_action = self.has_action
            && (self.action_on_new_line
                || action_multiline
                || action_width > (after_close - text_inset) / 2.
                || inline_height > body_line * 2.
                || inline_text_limit < body_line);
        let stack_dismiss = self.has_dismiss && inline_text_limit < body_line;
        let stacked = stack_action || stack_dismiss;
        let (message_width, message_height) = if stacked {
            text_size(body, fonts, Some((width - 2. * text_inset).max(0.)))
        } else {
            (inline_width, inline_height)
        };
        let text_row =
            (message_height + 2. * TEXT_VERTICAL_PADDING).max(if message_height > body_line {
                MULTILINE_HEIGHT
            } else {
                SINGLE_LINE_HEIGHT
            });
        let message = Rect::new(
            text_inset,
            (text_row - message_height) / 2.,
            message_width,
            message_height,
        );

        let split_controls =
            stacked && self.has_action && self.has_dismiss && after_close < TOUCH_TARGET;
        let action_end = if split_controls { end } else { after_close };
        let action = action_label.map(|label| {
            let x = (action_end - action_width).max(0.);
            let control_width = action_end - x;
            let padding = ACTION_SIDE_PADDING.min(control_width / 5.);
            let (label_width, label_height) =
                text_size(label, fonts, Some((control_width - 2. * padding).max(0.)));
            let height = (label_height + 2. * ACTION_VERTICAL_PADDING).max(TOUCH_TARGET);
            let y = if stacked {
                text_row
            } else {
                (text_row - height) / 2.
            };
            let hit = Rect::new(x, y, control_width, height);
            let ink = centered(hit, label_width, label_height);
            [hit, ink]
        });
        let action_height = action.map_or(0., |[hit, _]| hit.height);

        let dismiss = self.has_dismiss.then(|| {
            let y = if split_controls {
                text_row + action_height
            } else if stacked {
                text_row + (action_height.max(TOUCH_TARGET) - TOUCH_TARGET) / 2.
            } else {
                (text_row - TOUCH_TARGET) / 2.
            };
            let hit = Rect::new(after_close, y, dismiss_width, TOUCH_TARGET);
            let ink = centered(hit, CLOSE_ICON_SIZE, CLOSE_ICON_SIZE);
            [hit, ink]
        });
        let controls_height = if split_controls {
            action_height + TOUCH_TARGET
        } else {
            action_height.max(if self.has_dismiss { TOUCH_TARGET } else { 0. })
        };
        let height = if stacked {
            text_row + controls_height + STACK_BOTTOM_PADDING
        } else {
            text_row.max(controls_height)
        };
        Geometry {
            height,
            message,
            action,
            dismiss,
        }
    }
}

fn text_size(node: &Node, fonts: &mut dyn TextMeasurer, width: Option<f32>) -> (f32, f32) {
    let text = node.text_value().unwrap_or_default();
    if text.is_empty() || width == Some(0.) {
        return (0., 0.);
    }
    let (w, h) = fonts.measure_text(text, &node.style().text_style(), width);
    (w.ceil().min(width.unwrap_or(f32::INFINITY)), h.ceil())
}

fn centered(area: Rect, width: f32, height: f32) -> Rect {
    Rect::new(
        area.x + (area.width - width) / 2.,
        area.y + (area.height - height) / 2.,
        width,
        height,
    )
}
