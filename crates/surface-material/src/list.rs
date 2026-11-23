//! Material 3 Baseline and Expressive lists.

mod expansion;
mod multi_action;
mod reorder;
pub use expansion::{ExpandableListOptions, ExpandableListRow, ListExpansionMotion};
pub use multi_action::{ListSupplementaryAction, MultiActionListItem, MultiActionListOptions};
pub use reorder::{
    ListReorderEntry, ListReorderInput, ListReorderRequest, ListReorderState, ReorderableListItem,
    ReorderableListOptions,
};

use surface::{Align, Color, Corners, FillContour, Length, Node, Role, Style};
use surface_app::{Roving, View};

use crate::{
    date_picker::{button, disabled_ink, text},
    motion,
    shape::{self, LayoutDirection},
    typography, Theme, VectorIcon,
};

const SLOT_GAP: f32 = 12.;
const ROW_PADDING: f32 = 10.;
const EDGE_INSET: f32 = 16.;
const BASELINE_END_INSET: f32 = 24.;
const SEGMENT_GAP: f32 = 2.;
const FOCUS_WIDTH: f32 = 3.;
const CHECKBOX_SIZE: f32 = 18.;

/// The visual treatment of the list and its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListStyle {
    /// Baseline standard rows with square resting corners.
    Baseline,
    /// Expressive standard rows with no gap between rows.
    Standard,
    /// Expressive segmented rows with rounded group ends and a gap.
    Segmented,
}

/// The interaction shared by the whole list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListMode<'a> {
    /// Readable content with no row actions or keyboard stops.
    Passive,
    /// Each enabled row performs one command.
    Action,
    /// Each enabled row can select its ID. The caller supplies the selection.
    SingleSelect { selected_id: Option<&'a str> },
}

/// The minimum row size. Text can grow beyond this size when it wraps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListLines {
    One,
    Two,
    Three,
}

impl ListLines {
    fn min_height(self) -> f32 {
        match self {
            Self::One => 56.,
            Self::Two => 72.,
            Self::Three => 88.,
        }
    }
}

/// Stable identity, accessible name, and presentation for a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListOptions<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub style: ListStyle,
    pub mode: ListMode<'a>,
    pub direction: LayoutDirection,
}

/// Stable identity and caller-owned selection for a list of checkboxes.
#[derive(Clone, Copy)]
pub struct MultiSelectListOptions<'a> {
    pub id: &'a str,
    /// Name the items people can choose. Use a localized label.
    pub name: &'a str,
    pub style: ListStyle,
    pub direction: LayoutDirection,
    /// Report whether each stable row ID is checked.
    pub is_selected: &'a dyn Fn(&str) -> bool,
}

/// One row with one headline, optional details and a single available action.
pub struct ListItem<'a, M> {
    /// Stable ID of the row or its interactive control. Reserve the generated
    /// `-item`, `-overline`, `-headline`, `-supporting`, `-leading`, `-trailing`
    /// and `-mark` suffixes of this ID for the component's descendants.
    pub id: &'a str,
    /// The visible headline also names an interactive row.
    pub headline: &'a str,
    pub overline: Option<&'a str>,
    /// Additional visible text. It describes an enabled interactive row.
    pub supporting: Option<&'a str>,
    /// A decorative icon sized and colored by the list.
    pub leading: Option<&'a VectorIcon>,
    /// A short visual label at the end of the row. Supply essential
    /// information in the headline or supporting text.
    pub trailing: Option<&'a str>,
    pub lines: ListLines,
    pub disabled: bool,
    /// The command or selection message. An interactive row with no message
    /// is disabled. Passive rows ignore this value.
    pub on_activate: Option<M>,
}

#[derive(Clone, Copy)]
enum ListInteraction<'a> {
    Passive,
    Action,
    SingleSelect(Option<&'a str>),
    MultiSelect(&'a dyn Fn(&str) -> bool),
}

impl ListInteraction<'_> {
    fn row_role(self) -> Role {
        match self {
            Self::Passive => Role::ListItem,
            Self::Action => Role::Button,
            Self::SingleSelect(_) => Role::Radio,
            Self::MultiSelect(_) => Role::Checkbox,
        }
    }

    fn checked(self, id: &str) -> Option<bool> {
        match self {
            Self::SingleSelect(id_selected) => Some(id_selected == Some(id)),
            Self::MultiSelect(is_selected) => Some(is_selected(id)),
            Self::Passive | Self::Action => None,
        }
    }

    fn web_selection(self, node: Node) -> Node {
        match self {
            Self::SingleSelect(_) => node.web_listbox_semantics(),
            Self::MultiSelect(_) => node.web_multiselect_listbox_semantics(),
            Self::Passive | Self::Action => node,
        }
    }
}

#[derive(Clone, Copy)]
struct ListPresentation<'a> {
    style: ListStyle,
    direction: LayoutDirection,
    interaction: ListInteraction<'a>,
}

#[derive(Clone, Copy)]
enum RowPlacement {
    Regular { first: bool, last: bool },
    ExpansionHeader { body_visible: bool },
    ExpansionChild { last: bool, indent: f32 },
}

impl Theme {
    /// Build a list that fills the available width and fits its row heights.
    ///
    /// Place it inside a clipped scroller when it needs to scroll. In an
    /// interactive list, Tab enters or leaves the group, and the arrow keys
    /// move focus without running the action. Enter or Space activates the
    /// focused row. Keep each row ID stable across changes to order or selection.
    ///
    /// [Material 3 lists](https://m3.material.io/components/lists/overview)
    pub fn list<'a, M>(
        &self,
        options: ListOptions<'a>,
        items: impl IntoIterator<Item = ListItem<'a, M>>,
    ) -> View<M> {
        let interaction = match options.mode {
            ListMode::Passive => ListInteraction::Passive,
            ListMode::Action => ListInteraction::Action,
            ListMode::SingleSelect { selected_id } => ListInteraction::SingleSelect(selected_id),
        };
        self.list_with(
            options.id,
            options.name,
            ListPresentation {
                style: options.style,
                direction: options.direction,
                interaction,
            },
            items,
        )
    }

    /// Build a multi-select list with one checkbox action per row.
    ///
    /// Give each enabled item an `on_activate` message with its ID and
    /// desired checked state. The caller updates its selection and supplies
    /// the result through `is_selected` on the next build. Arrow keys move
    /// focus. Enter, Space or a press switches only the focused or pressed row.
    /// Web exposes options in a multi-select listbox. Native exposes checkboxes.
    pub fn multi_select_list<'a, M>(
        &self,
        options: MultiSelectListOptions<'_>,
        items: impl IntoIterator<Item = ListItem<'a, M>>,
    ) -> View<M> {
        self.list_with(
            options.id,
            options.name,
            ListPresentation {
                style: options.style,
                direction: options.direction,
                interaction: ListInteraction::MultiSelect(options.is_selected),
            },
            items,
        )
    }

    fn list_with<'a, M>(
        &self,
        id: &str,
        name: &str,
        options: ListPresentation<'_>,
        items: impl IntoIterator<Item = ListItem<'a, M>>,
    ) -> View<M> {
        let items: Vec<_> = items.into_iter().collect();
        let last = items.len().saturating_sub(1);
        let role = match options.interaction {
            ListInteraction::Passive | ListInteraction::Action => Role::List,
            ListInteraction::SingleSelect(_) => Role::RadioGroup,
            ListInteraction::MultiSelect(_) => Role::Group,
        };
        let style = Style::column()
            .width(Length::Percent(1.))
            .gap(list_gap(options.style));
        let node = options
            .interaction
            .web_selection(Node::new(role, name).id(id).styled(style));
        let view = View::new(node).children(
            items
                .into_iter()
                .enumerate()
                .map(|(index, item)| self.list_row(options, item, index == 0, index == last)),
        );
        if matches!(options.interaction, ListInteraction::Passive) {
            view
        } else {
            view.roving_retained(Roving::vertical(false))
        }
    }

    fn list_row<M>(
        &self,
        options: ListPresentation<'_>,
        item: ListItem<'_, M>,
        first: bool,
        last: bool,
    ) -> View<M> {
        let id = item.id;
        let row = self.list_row_control(options, item, RowPlacement::Regular { first, last }, None);
        if options.interaction.row_role() == Role::Button {
            action_item(id, row)
        } else {
            row
        }
    }

    fn list_row_control<M>(
        &self,
        options: ListPresentation<'_>,
        item: ListItem<'_, M>,
        placement: RowPlacement,
        accessory: Option<View<M>>,
    ) -> View<M> {
        let passive = matches!(options.interaction, ListInteraction::Passive);
        let disabled = item.disabled || (!passive && item.on_activate.is_none());
        let role = options.interaction.row_role();
        let selection = options.interaction.checked(item.id);
        let selected = selection == Some(true);
        let baseline = options.style == ListStyle::Baseline;
        let ink = if disabled {
            disabled_ink(self)
        } else if selected {
            self.colors.on_secondary_container
        } else {
            self.colors.on_surface
        };
        let supporting_ink = if disabled || selected {
            ink
        } else {
            self.colors.on_surface_variant
        };
        let regular_background = if selected && disabled && role == Role::Checkbox {
            // The selected, disabled Lists container uses OnSurface at 38%.
            ink
        } else if selected {
            self.colors.secondary_container
        } else if options.style == ListStyle::Segmented {
            self.colors.surface
        } else {
            Color::TRANSPARENT
        };
        let segmented = options.style == ListStyle::Segmented;
        let (corners, background, indent) = match placement {
            RowPlacement::Regular { first, last } => (
                row_corners(options.style, selected, first, last),
                regular_background,
                0.,
            ),
            RowPlacement::ExpansionHeader { body_visible } => (
                if body_visible {
                    Corners::new(16., 16., 4., 4.)
                } else {
                    shape::LARGE
                },
                if body_visible || segmented {
                    self.colors.surface
                } else {
                    Color::TRANSPARENT
                },
                0.,
            ),
            RowPlacement::ExpansionChild { last, indent } => (
                if last {
                    Corners::new(4., 4., 16., 16.)
                } else {
                    shape::EXTRA_SMALL
                },
                self.colors.surface,
                indent,
            ),
        };
        let mut style = Style::column()
            .width(Length::Percent(1.))
            .justify(row_alignment(item.lines))
            .padding(ROW_PADDING)
            .corners(corners)
            .background(background);
        style.min_height = item.lines.min_height();
        if !passive {
            style = style.state_layer(ink).transition(motion::STATE_CHANGE);
        }
        let label = if passive { "" } else { item.headline };
        let mut node = Node::new(role, label)
            .id(item.id)
            .disabled(disabled)
            .styled(style);
        if !passive {
            node = node
                .focus_color(self.colors.secondary)
                .focus_outline_width(FOCUS_WIDTH)
                .state_layer_alphas(20, 26, 26);
            if item.supporting.is_some() {
                node = node.described_by_text(format!("{}-supporting", item.id));
            }
            if !baseline && !selected && matches!(placement, RowPlacement::Regular { .. }) {
                node = node.interaction_corners(
                    Some(shape::MEDIUM),
                    Some(shape::LARGE),
                    Some(shape::LARGE),
                );
            }
        }
        if let Some(checked) = selection {
            node = options.interaction.web_selection(node.checked(checked));
        }

        let mark = accessory.or_else(|| {
            selection.map(|checked| {
                let id = format!("{}-mark", item.id);
                if role == Role::Checkbox {
                    self.list_checkbox(id, checked, disabled)
                } else {
                    let size = list_icon_size(options.style);
                    let contour = if checked {
                        selection_mark(size, ink)
                    } else {
                        None
                    };
                    list_icon(id, size, contour)
                }
            })
        });
        let content = row_content(options, &item, mark, ink, supporting_ink, indent);
        let row = if passive {
            View::new(node)
        } else {
            button(node, item.on_activate.filter(|_| !disabled))
        }
        .children([content]);
        if role == Role::Checkbox {
            row.suppress_repeated_activation()
        } else {
            row
        }
    }

    fn list_checkbox<M>(&self, id: String, checked: bool, disabled: bool) -> View<M> {
        let colors = self.colors;
        let ink = if disabled {
            disabled_ink(self)
        } else if checked {
            colors.primary
        } else {
            colors.on_surface_variant
        };
        let size = Length::Px(CHECKBOX_SIZE);
        let style = Style::column()
            .width(size)
            .height(size)
            .corners(Corners::all(2.));
        let style = if checked {
            style.background(ink)
        } else {
            style.border(2., ink)
        };
        let mut node = Node::new(Role::Group, "").id(id).styled(style);
        if checked {
            let color = if disabled {
                colors.surface
            } else {
                colors.on_primary
            };
            if let Some(check) = selection_mark(CHECKBOX_SIZE, color) {
                node = node.fill_contour(check);
            }
        }
        View::new(node).without_interaction(None)
    }
}

fn action_item<M>(id: &str, row: View<M>) -> View<M> {
    View::new(
        Node::new(Role::ListItem, "")
            .id(format!("{id}-item"))
            .styled(Style::column().width(Length::Percent(1.))),
    )
    .children([row])
}

fn list_gap(style: ListStyle) -> f32 {
    if style == ListStyle::Segmented {
        SEGMENT_GAP
    } else {
        0.
    }
}

fn row_content<M>(
    options: ListPresentation<'_>,
    item: &ListItem<'_, M>,
    mark: Option<View<M>>,
    ink: Color,
    supporting_ink: Color,
    indent: f32,
) -> View<M> {
    let id = |part| format!("{}-{part}", item.id);
    let passive = matches!(options.interaction, ListInteraction::Passive);
    let icon_size = list_icon_size(options.style);
    let align = row_alignment(item.lines);
    let copy = row_copy(item, ink, supporting_ink, passive);
    let leading = item.leading.map(|icon| {
        list_icon(
            id("leading"),
            icon_size,
            icon.contour_dp(icon_size, supporting_ink),
        )
    });
    let trailing = item.trailing.map(|label| {
        let mut style = Style::column()
            .with_text_style(typography::LABEL_SMALL)
            .foreground(supporting_ink);
        style.max_width = Length::Percent(0.28);
        let trailing = View::new(
            Node::new(Role::Text, label)
                .id(id("trailing"))
                .text(label)
                .shrink_to_fit_width()
                .styled(style),
        );
        if passive {
            trailing
        } else {
            trailing.without_interaction(None)
        }
    });
    let content = View::new(logical_row(
        Style::row()
            .width(Length::Px(0.))
            .grow(1.)
            .align(align)
            .gap(SLOT_GAP),
        options.direction,
    ))
    .children(
        leading
            .into_iter()
            .chain([copy])
            .chain(trailing)
            .chain(mark),
    );
    let end = if options.style == ListStyle::Baseline {
        BASELINE_END_INSET
    } else {
        EDGE_INSET
    };
    View::new(logical_row(
        Style::row().width(Length::Percent(1.)).align(align),
        options.direction,
    ))
    .children([
        spacer(EDGE_INSET - ROW_PADDING + indent),
        content,
        spacer(end - ROW_PADDING),
    ])
}

fn list_icon_size(style: ListStyle) -> f32 {
    if style == ListStyle::Baseline {
        24.
    } else {
        20.
    }
}

fn row_copy<M>(
    item: &ListItem<'_, M>,
    ink: Color,
    supporting_ink: Color,
    passive: bool,
) -> View<M> {
    let id = item.id;
    let caption = |part, copy, typography, ink| {
        let label = text(format!("{id}-{part}"), copy, typography, ink);
        if passive {
            label
        } else {
            label.without_interaction(None)
        }
    };
    let overline = item
        .overline
        .map(|copy| caption("overline", copy, typography::LABEL_SMALL, supporting_ink));
    let headline = caption("headline", item.headline, typography::BODY_LARGE, ink);
    let supporting = item.supporting.map(|copy| {
        text(
            format!("{id}-supporting"),
            copy,
            typography::BODY_MEDIUM,
            supporting_ink,
        )
    });
    View::new(Node::new(Role::Group, "").styled(Style::column().width(Length::Px(0.)).grow(1.)))
        .children(overline.into_iter().chain([headline]).chain(supporting))
}

fn row_alignment(lines: ListLines) -> Align {
    if lines == ListLines::Three {
        Align::Start
    } else {
        Align::Center
    }
}

fn row_corners(style: ListStyle, selected: bool, first: bool, last: bool) -> Corners {
    if selected {
        return shape::LARGE;
    }
    match style {
        ListStyle::Baseline => shape::NONE,
        ListStyle::Standard => shape::EXTRA_SMALL,
        ListStyle::Segmented => {
            let top = if first { 16. } else { 4. };
            let bottom = if last { 16. } else { 4. };
            Corners::new(top, top, bottom, bottom)
        }
    }
}

fn logical_row(style: Style, direction: LayoutDirection) -> Node {
    let node = Node::new(Role::Group, "").styled(style);
    match direction {
        LayoutDirection::LeftToRight => node,
        LayoutDirection::RightToLeft => node.reverse_row(),
    }
}

fn spacer<M>(width: f32) -> View<M> {
    View::new(Node::new(Role::Group, "").styled(Style::column().width(Length::Px(width))))
        .without_interaction(None)
}

fn list_icon<M>(id: String, size: f32, contour: Option<FillContour>) -> View<M> {
    let edge = Length::Px(size);
    let node = Node::new(Role::Group, "")
        .id(id)
        .styled(Style::column().width(edge).height(edge));
    let node = if let Some(contour) = contour {
        node.fill_contour(contour)
    } else {
        node
    };
    View::new(node).without_interaction(None)
}

fn selection_mark(size: f32, color: Color) -> Option<FillContour> {
    let points = [
        [9., 16.2],
        [4.8, 12.],
        [3.4, 13.4],
        [9., 19.],
        [21., 7.],
        [19.6, 5.6],
    ];
    FillContour::new(
        points.map(|point| point.map(|value| value * size / 24.)),
        color,
    )
}
