//! Material 3 Baseline and Expressive lists.

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
        let items: Vec<_> = items.into_iter().collect();
        let last = items.len().saturating_sub(1);
        let mut node = Node::new(
            if matches!(options.mode, ListMode::SingleSelect { .. }) {
                Role::RadioGroup
            } else {
                Role::List
            },
            options.name,
        )
        .id(options.id)
        .styled(Style::column().width(Length::Percent(1.)).gap(
            if options.style == ListStyle::Segmented {
                SEGMENT_GAP
            } else {
                0.
            },
        ));
        if matches!(options.mode, ListMode::SingleSelect { .. }) {
            node = node.web_listbox_semantics();
        }
        let view = View::new(node).children(
            items
                .into_iter()
                .enumerate()
                .map(|(index, item)| self.list_row(options, item, index == 0, index == last)),
        );
        if options.mode == ListMode::Passive {
            view
        } else {
            view.roving_retained(Roving::vertical(false))
        }
    }

    fn list_row<M>(
        &self,
        options: ListOptions<'_>,
        item: ListItem<'_, M>,
        first: bool,
        last: bool,
    ) -> View<M> {
        let passive = options.mode == ListMode::Passive;
        let disabled = item.disabled || (!passive && item.on_activate.is_none());
        let role = match options.mode {
            ListMode::Passive => Role::ListItem,
            ListMode::Action => Role::Button,
            ListMode::SingleSelect { .. } => Role::Radio,
        };
        let selection = match options.mode {
            ListMode::SingleSelect { selected_id } => Some(selected_id == Some(item.id)),
            ListMode::Passive | ListMode::Action => None,
        };
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
        let background = if selected {
            self.colors.secondary_container
        } else if options.style == ListStyle::Segmented {
            self.colors.surface
        } else {
            Color::TRANSPARENT
        };
        let mut style = Style::column()
            .width(Length::Percent(1.))
            .justify(row_alignment(item.lines))
            .padding(ROW_PADDING)
            .corners(row_corners(options.style, selected, first, last))
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
            if !baseline && !selected {
                node = node.interaction_corners(
                    Some(shape::MEDIUM),
                    Some(shape::LARGE),
                    Some(shape::LARGE),
                );
            }
        }
        if let Some(checked) = selection {
            node = node.checked(checked).web_listbox_semantics();
        }

        let content = row_content(options, &item, selection, ink, supporting_ink);
        let row = if passive {
            View::new(node)
        } else {
            button(node, item.on_activate.filter(|_| !disabled))
        }
        .children([content]);
        if options.mode == ListMode::Action {
            View::new(
                Node::new(Role::ListItem, "")
                    .id(format!("{}-item", item.id))
                    .styled(Style::column().width(Length::Percent(1.))),
            )
            .children([row])
        } else {
            row
        }
    }
}

fn row_content<M>(
    options: ListOptions<'_>,
    item: &ListItem<'_, M>,
    selection: Option<bool>,
    ink: Color,
    supporting_ink: Color,
) -> View<M> {
    let id = |part| format!("{}-{part}", item.id);
    let passive = options.mode == ListMode::Passive;
    let baseline = options.style == ListStyle::Baseline;
    let icon_size = if baseline { 24. } else { 20. };
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
    let mark = selection.map(|checked| {
        list_icon(
            id("mark"),
            icon_size,
            if checked {
                selection_mark(icon_size, ink)
            } else {
                None
            },
        )
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
    let end = if baseline {
        BASELINE_END_INSET
    } else {
        EDGE_INSET
    };
    View::new(logical_row(
        Style::row().width(Length::Percent(1.)).align(align),
        options.direction,
    ))
    .children([
        spacer(EDGE_INSET - ROW_PADDING),
        content,
        spacer(end - ROW_PADDING),
    ])
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
