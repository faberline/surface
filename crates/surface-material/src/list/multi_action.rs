use surface::{Align, Color, Length, Node, Role, Style};
use surface_app::View;

use super::{
    button, disabled_ink, list_icon, list_icon_size, logical_row, motion, row_alignment, row_copy,
    row_corners, shape, spacer, LayoutDirection, ListItem, ListLines, ListStyle, Theme, VectorIcon,
    BASELINE_END_INSET, EDGE_INSET, FOCUS_WIDTH, ROW_PADDING, SEGMENT_GAP, SLOT_GAP,
};

pub(super) const TARGET: f32 = 48.;
const ACTION_GAP: f32 = 4.;
const COMPACT_PADDING: f32 = 8.;
const COMPACT_COPY_WIDTH: f32 = 32.;
const TRAILING_LABEL_WIDTH: f32 = 0.28;

#[derive(Clone, Copy)]
pub(super) struct ListActionRow {
    inline: bool,
    compact: bool,
    end: f32,
    gutter: f32,
    inset: f32,
}

impl ListActionRow {
    pub(super) fn new<M>(
        width: f32,
        style: ListStyle,
        item: &ListItem<'_, M>,
        count: usize,
    ) -> Self {
        let baseline = style == ListStyle::Baseline;
        let end = if baseline {
            BASELINE_END_INSET
        } else {
            EDGE_INSET
        };
        let gutter = if baseline { EDGE_INSET } else { SLOT_GAP } - ROW_PADDING;
        let leading_width = item
            .leading
            .map_or(0., |_| list_icon_size(style) + SLOT_GAP);
        let trailing_width = TARGET * count as f32 + ACTION_GAP * count.saturating_sub(1) as f32;
        let copy_width = |content_width: f32| {
            let metadata = item.trailing.map_or(0., |_| {
                content_width.max(0.) * TRAILING_LABEL_WIDTH + SLOT_GAP
            });
            content_width - leading_width - metadata
        };
        let inline_width = width - end - gutter - trailing_width - EDGE_INSET - ROW_PADDING;
        let inline = count > 0 && copy_width(inline_width) > trailing_width;
        let inset = if count > 0 && !inline {
            ROW_PADDING.min((width - TARGET) / 2.)
        } else {
            0.
        };
        let compact =
            !inline && copy_width(width - 2. * inset - EDGE_INSET - end) < COMPACT_COPY_WIDTH;
        Self {
            inline,
            compact,
            end,
            gutter,
            inset,
        }
    }

    pub(super) fn style(self) -> Style {
        let row = if self.inline {
            Style::row()
        } else {
            // The full-width primary occupies the first line. The remaining
            // fixed-width buttons wrap in authored order and align to the end.
            Style::row()
                .wrap(true)
                .justify(Align::End)
                .gap(ACTION_GAP)
                .padding(self.inset)
        };
        row.width(Length::Percent(1.)).align(Align::Stretch)
    }

    pub(super) fn children<M>(self, primary: View<M>, extras: Vec<View<M>>) -> Vec<View<M>> {
        let mut children = vec![primary];
        if self.inline {
            children.push(spacer(self.gutter));
        }
        for (index, action) in extras.into_iter().enumerate() {
            if self.inline && index != 0 {
                children.push(spacer(ACTION_GAP));
            }
            children.push(action);
        }
        if self.inline {
            children.push(spacer(self.end));
        }
        children
    }

    fn padding(self) -> f32 {
        if self.compact {
            COMPACT_PADDING
        } else {
            ROW_PADDING
        }
    }

    pub(super) fn primary_style(self, list_style: ListStyle, lines: ListLines) -> Style {
        let mut style = Style::column()
            .justify(row_alignment(lines))
            .padding(self.padding())
            .corners(if list_style == ListStyle::Baseline {
                shape::NONE
            } else {
                shape::LARGE
            });
        style.min_width = TARGET;
        style.min_height = lines.min_height();
        if self.inline {
            style.width(Length::Px(0.)).grow(1.)
        } else {
            style.width(Length::Percent(1.))
        }
    }
}

/// Placement and style of a list with independent row actions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MultiActionListOptions<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub style: ListStyle,
    pub direction: LayoutDirection,
    /// Width allocated to the list after the caller's insets.
    pub available_width: f32,
}

/// A row with a primary action and supplementary actions.
pub struct MultiActionListItem<'a, M> {
    /// The primary button uses this ID. The row uses `{id}-item`.
    /// Reserve `-headline`, `-overline`, `-supporting` and `-leading` for its visuals.
    pub id: &'a str,
    /// The visible headline and primary button name.
    pub headline: &'a str,
    pub overline: Option<&'a str>,
    pub supporting: Option<&'a str>,
    pub leading: Option<&'a VectorIcon>,
    pub lines: ListLines,
    /// Disable just the primary action.
    pub disabled: bool,
    /// A primary action with no message is disabled.
    pub on_activate: Option<M>,
    /// Secondary buttons in keyboard order. An empty list leaves just the primary button.
    pub actions: Vec<ListSupplementaryAction<'a, M>>,
}

/// An icon button with an independent name, state and action.
pub struct ListSupplementaryAction<'a, M> {
    /// Stable button ID. The passive glyph uses `{id}-icon`.
    pub id: &'a str,
    /// Localized accessible name of the action.
    pub label: &'a str,
    pub icon: &'a VectorIcon,
    /// Give a toggle button its current pressed state. Use `None` for a command.
    pub checked: Option<bool>,
    pub disabled: bool,
    /// A button with no message is disabled.
    pub on_activate: Option<M>,
}

impl Theme {
    /// Build a multiple-action list for its current allocated width.
    ///
    /// Each action is a separate button. Tab enters at the first enabled button.
    /// Right and Down move to the next action, and Left and Up move to the
    /// previous action. Navigation wraps. RTL mirrors placement and keeps
    /// the same keyboard order. Enter and Space run the focused button's action.
    ///
    /// The action strip wraps below the primary button if the inline content
    /// would be too narrow. Keep the item and action IDs across layout changes.
    /// This returns `None` for a nonfinite width or a width below 48 dp.
    /// Place a long list in a named clipped scroller.
    pub fn multi_action_list<'a, M>(
        &self,
        options: MultiActionListOptions<'_>,
        items: impl IntoIterator<Item = MultiActionListItem<'a, M>>,
    ) -> Option<View<M>> {
        if !options.available_width.is_finite() || options.available_width < TARGET {
            return None;
        }
        let mut items = items.into_iter().peekable();
        let mut first = true;
        let mut rows = Vec::new();
        while let Some(item) = items.next() {
            rows.push(self.multi_action_row(options, item, first, items.peek().is_none()));
            first = false;
        }
        let gap = if options.style == ListStyle::Segmented {
            SEGMENT_GAP
        } else {
            0.
        };
        Some(
            View::new(
                Node::new(Role::List, options.name).id(options.id).styled(
                    Style::column()
                        .width(Length::Px(options.available_width))
                        .gap(gap),
                ),
            )
            .children(rows)
            .roving_all_arrows(),
        )
    }

    fn multi_action_row<M>(
        &self,
        options: MultiActionListOptions<'_>,
        item: MultiActionListItem<'_, M>,
        first: bool,
        last: bool,
    ) -> View<M> {
        let icon_size = list_icon_size(options.style);
        let actions = item
            .actions
            .into_iter()
            .map(|action| self.supplementary_action(action, item.id, icon_size))
            .collect();
        self.list_action_row(
            options,
            ListItem {
                id: item.id,
                headline: item.headline,
                overline: item.overline,
                supporting: item.supporting,
                leading: item.leading,
                trailing: None,
                lines: item.lines,
                disabled: item.disabled,
                on_activate: item.on_activate,
            },
            actions,
            first,
            last,
        )
    }

    pub(super) fn list_action_row<M>(
        &self,
        options: MultiActionListOptions<'_>,
        item: ListItem<'_, M>,
        actions: Vec<View<M>>,
        first: bool,
        last: bool,
    ) -> View<M> {
        let placement =
            ListActionRow::new(options.available_width, options.style, &item, actions.len());
        let corners = row_corners(options.style, false, first, last);
        let mut style = placement.style().corners(corners);
        if options.style == ListStyle::Segmented {
            style = style.background(self.colors.surface);
        }
        let mut row = Node::new(Role::ListItem, "")
            .id(format!("{}-item", item.id))
            .styled(style);
        if options.direction == LayoutDirection::RightToLeft {
            row = row.reverse_row();
        }
        View::new(row).children(
            placement.children(self.multi_action_primary(options, item, placement), actions),
        )
    }

    fn multi_action_primary<M>(
        &self,
        options: MultiActionListOptions<'_>,
        item: ListItem<'_, M>,
        placement: ListActionRow,
    ) -> View<M> {
        let disabled = item.disabled || item.on_activate.is_none();
        let (ink, secondary_ink) = if disabled {
            let ink = disabled_ink(self);
            (ink, ink)
        } else {
            (self.colors.on_surface, self.colors.on_surface_variant)
        };
        let style = placement.primary_style(options.style, item.lines);
        let mut node = Node::new(Role::Button, item.headline).id(item.id);
        if item.supporting.is_some() {
            node = node.described_by_text(format!("{}-supporting", item.id));
        }
        let content = list_action_copy(options, &item, placement, ink, secondary_ink);
        self.list_action_button(node, style, ink, item.on_activate.filter(|_| !disabled))
            .children([content])
    }

    fn supplementary_action<M>(
        &self,
        action: ListSupplementaryAction<'_, M>,
        primary: &str,
        icon_size: f32,
    ) -> View<M> {
        let disabled = action.disabled || action.on_activate.is_none();
        let ink = if disabled {
            disabled_ink(self)
        } else if action.checked == Some(true) {
            self.colors.primary
        } else {
            self.colors.on_surface_variant
        };
        let mut node = Node::new(Role::Button, action.label).id(action.id);
        if let Some(checked) = action.checked {
            node = node.checked(checked);
        }
        let icon = list_icon(
            format!("{}-icon", action.id),
            icon_size,
            action.icon.contour_dp(icon_size, ink),
        );
        self.list_action_button(
            node,
            list_action_target(),
            ink,
            action.on_activate.filter(|_| !disabled),
        )
        .children([icon])
        .restore_focus_on_removal(primary)
    }

    pub(super) fn list_action_button<M>(
        &self,
        node: Node,
        style: Style,
        ink: Color,
        on_activate: Option<M>,
    ) -> View<M> {
        button(
            node.styled(style.state_layer(ink).transition(motion::STATE_CHANGE))
                .state_layer_alphas(20, 26, 26)
                .focus_color(self.colors.secondary)
                .focus_outline_width(FOCUS_WIDTH),
            on_activate,
        )
        .suppress_repeated_activation()
        .accessibility_scroll_into_view()
    }
}

pub(super) fn list_action_target() -> Style {
    let mut style = Style::column()
        .width(Length::Px(TARGET))
        .align(Align::Center)
        .justify(Align::Center)
        .corners(shape::LARGE);
    style.min_width = TARGET;
    style.min_height = TARGET;
    style
}

pub(super) fn list_action_copy<M>(
    options: MultiActionListOptions<'_>,
    item: &ListItem<'_, M>,
    placement: ListActionRow,
    ink: Color,
    secondary_ink: Color,
) -> View<M> {
    let copy = row_copy(item, ink, secondary_ink, false);
    let leading = item.leading.map(|icon| {
        let size = list_icon_size(options.style);
        list_icon(
            format!("{}-leading", item.id),
            size,
            icon.contour_dp(size, secondary_ink),
        )
    });
    let trailing = item.trailing.map(|label| {
        let mut style = Style::column()
            .with_text_style(crate::typography::LABEL_SMALL)
            .foreground(secondary_ink);
        if !placement.compact {
            style.max_width = Length::Percent(TRAILING_LABEL_WIDTH);
        }
        View::new(
            Node::new(Role::Text, label)
                .id(format!("{}-trailing", item.id))
                .text(label)
                .shrink_to_fit_width()
                .styled(style),
        )
        .without_interaction(None)
    });
    if placement.compact {
        let line =
            |child| View::new(logical_row(Style::row(), options.direction)).children([child]);
        View::new(Node::new(Role::Group, "").styled(Style::column().gap(ACTION_GAP))).children(
            leading
                .map(line)
                .into_iter()
                .chain([line(copy)])
                .chain(trailing),
        )
    } else {
        let content = View::new(logical_row(
            Style::row()
                .width(Length::Px(0.))
                .grow(1.)
                .align(row_alignment(item.lines))
                .gap(SLOT_GAP),
            options.direction,
        ))
        .children(leading.into_iter().chain([copy]).chain(trailing));
        let padding = placement.padding();
        View::new(logical_row(Style::row(), options.direction)).children(
            [spacer(EDGE_INSET - padding), content]
                .into_iter()
                .chain((!placement.inline).then(|| spacer(placement.end - padding))),
        )
    }
}
