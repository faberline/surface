use surface::{Align, Color, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, Roving, ToggleMotion, ToggleSpring, View};

use super::{
    action_item, list_gap, list_icon_size, ListInteraction, ListItem, ListPresentation, ListStyle,
    RowPlacement, EDGE_INSET, SLOT_GAP,
};
use crate::{date_picker::disabled_ink, motion, shape, vector_icon::chevron_contour, Theme};

const INDICATOR_SIZE: f32 = 32.;
const CHEVRON_SIZE: f32 = 24.;

/// Stable identity, accessible name, and style of an expandable list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpandableListOptions<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub style: ListStyle,
    pub direction: shape::LayoutDirection,
}

/// A command row or a disclosure with one level of child commands.
pub enum ExpandableListRow<'a, M> {
    Leaf(ListItem<'a, M>),
    Branch {
        /// The row's message asks the caller to change `expanded`. Keep its ID
        /// stable. Reserve `-branch`, `-expansion-content`, `-indicator`, and
        /// `-indicator-icon` for this branch's descendants.
        header: ListItem<'a, M>,
        /// Keep child IDs stable and supply the rows during the closing motion.
        children: Vec<ListItem<'a, M>>,
        /// The caller owns this state. Update it when the header activates.
        expanded: bool,
        /// Retain one motion value per header ID.
        motion: &'a ListExpansionMotion,
    },
}

#[derive(Clone, Copy, PartialEq)]
enum Body {
    Closed,
    Moving { height: f32, fraction: f32 },
    Open,
}

impl Body {
    fn height(self) -> Length {
        match self {
            Self::Closed => Length::Px(0.),
            Self::Moving { height, .. } => Length::Px(height),
            Self::Open => Length::Auto,
        }
    }

    fn fraction(self) -> f32 {
        match self {
            Self::Closed => 0.,
            Self::Moving { fraction, .. } => fraction,
            Self::Open => 1.,
        }
    }
}

/// Retained geometry and motion for one disclosure row.
///
/// Initialize with the caller's starting state. Advance from the app's frame
/// hook and return the update to the runtime. When advancing several branches,
/// combine their `rebuild` and `request_next` flags with logical OR.
pub struct ListExpansionMotion {
    spring: ToggleMotion,
    body: Body,
    natural_height: Option<f32>,
}

impl ListExpansionMotion {
    pub fn new(expanded: bool) -> Self {
        Self {
            spring: ToggleMotion::new(expanded),
            body: if expanded { Body::Open } else { Body::Closed },
            natural_height: None,
        }
    }

    /// Advance the body from its current pose. With Reduce Motion or no
    /// reliable clock, snap to the caller's state in one frame.
    pub fn advance(
        &mut self,
        header_id: &str,
        expanded: bool,
        frame: &AnimationFrame<'_>,
    ) -> AnimationUpdate {
        let target = if expanded { Body::Open } else { Body::Closed };
        let content = frame.dom.by_id(&content_id(header_id));
        if content.is_none()
            || frame.reduced_motion
            || frame.now_ms.is_none_or(|time| !time.is_finite())
        {
            self.spring = ToggleMotion::new(expanded);
            self.natural_height = None;
            return self.present(target, false);
        }
        // A rebuild invalidates layout until the next frame. Use the last
        // natural height through a reversal and remeasure at the current width.
        if let Some(measured) = content
            .zip(frame.layout)
            .and_then(|(id, layout)| layout.nodes.get(id.index()))
            .map(|node| node.bounds.height)
            .filter(|height| height.is_finite() && *height >= 0.)
        {
            self.natural_height = Some(measured);
        }
        if self.body == target {
            return AnimationUpdate::default();
        }
        let Some(natural_height) = self.natural_height else {
            return AnimationUpdate {
                rebuild: false,
                request_next: true,
            };
        };
        if natural_height == 0. {
            self.spring = ToggleMotion::new(expanded);
            return self.present(target, false);
        }
        let (fraction, running) = self.spring.sample(
            expanded,
            frame.now_ms,
            false,
            ToggleSpring::StandardDefaultSpatial,
        );
        if running {
            self.present(
                Body::Moving {
                    height: fraction * natural_height,
                    fraction,
                },
                true,
            )
        } else {
            if !expanded {
                self.natural_height = None;
            }
            self.present(target, false)
        }
    }

    fn present(&mut self, body: Body, request_next: bool) -> AnimationUpdate {
        let rebuild = self.body != body;
        self.body = body;
        AnimationUpdate {
            rebuild,
            request_next,
        }
    }
}

impl Theme {
    /// Build a list with inline disclosure rows. The header stays in the same
    /// keyboard group as the ordinary rows and the revealed child commands.
    /// Arrow keys move focus, and Enter or Space acts on the focused button.
    /// A collapse moves focus from a child back to its enabled header.
    pub fn expandable_list<'a, M>(
        &self,
        options: ExpandableListOptions<'_>,
        rows: impl IntoIterator<Item = ExpandableListRow<'a, M>>,
    ) -> View<M> {
        let presentation = ListPresentation {
            style: options.style,
            direction: options.direction,
            interaction: ListInteraction::Action,
        };
        let gap = list_gap(options.style);
        let style = Style::column().width(Length::Percent(1.)).gap(gap);
        let rows: Vec<_> = rows.into_iter().collect();
        let last = rows.len().saturating_sub(1);
        View::new(
            Node::new(Role::List, options.name)
                .id(options.id)
                .styled(style),
        )
        .children(rows.into_iter().enumerate().map(|(index, row)| match row {
            ExpandableListRow::Leaf(item) => {
                self.list_row(presentation, item, index == 0, index == last)
            }
            ExpandableListRow::Branch {
                header,
                children,
                expanded,
                motion,
            } => self.list_branch(presentation, header, children, expanded, motion),
        }))
        .roving_retained_layout(Roving::vertical(false))
    }

    fn list_branch<M>(
        &self,
        options: ListPresentation<'_>,
        header: ListItem<'_, M>,
        children: Vec<ListItem<'_, M>>,
        expanded: bool,
        motion: &ListExpansionMotion,
    ) -> View<M> {
        let id = header.id;
        let keep_body = expanded || motion.body != Body::Closed;
        let body_visible = keep_body && !children.is_empty();
        let indent = if header.leading.is_some() {
            list_icon_size(options.style) + SLOT_GAP
        } else {
            EDGE_INSET
        };
        let indicator = self.expansion_indicator(
            id,
            expanded,
            motion.body.fraction(),
            header.disabled || header.on_activate.is_none(),
        );
        let header = self
            .list_row_control(
                options,
                header,
                RowPlacement::ExpansionHeader { body_visible },
                Some(indicator),
            )
            .disclosure_expanded(expanded)
            .suppress_repeated_activation();
        let style = Style::column()
            .width(Length::Percent(1.))
            .corners(shape::LARGE)
            .background(if body_visible && options.style != ListStyle::Segmented {
                self.colors.surface
            } else {
                Color::TRANSPARENT
            });
        let body = keep_body
            .then(|| self.expansion_body(options, id, indent, children, expanded, motion.body));
        View::new(
            Node::new(Role::ListItem, "")
                .id(format!("{id}-branch"))
                .styled(style),
        )
        .children(std::iter::once(header).chain(body))
    }

    fn expansion_body<M>(
        &self,
        options: ListPresentation<'_>,
        header_id: &str,
        indent: f32,
        children: Vec<ListItem<'_, M>>,
        expanded: bool,
        body: Body,
    ) -> View<M> {
        let gap = list_gap(options.style);
        let last = children.len().saturating_sub(1);
        let first_gap = (gap > 0. && !children.is_empty())
            .then(|| View::new(Node::new(Role::Group, "")).without_interaction(None));
        let rows = children.into_iter().enumerate().map(|(index, item)| {
            let id = item.id;
            let row = self.list_row_control(
                options,
                item,
                RowPlacement::ExpansionChild {
                    last: index == last,
                    indent,
                },
                None,
            );
            action_item(id, row)
        });
        let content = View::new(
            Node::new(Role::List, "")
                .id(content_id(header_id))
                .styled(Style::column().width(Length::Percent(1.)).gap(gap)),
        )
        .children(first_gap.into_iter().chain(rows));
        let visible = expanded && body == Body::Open;
        // Only the unnamed wrapper clips. The content stays at its natural
        // height for measurement. The wrapper is not a scroll container.
        let clip = View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Percent(1.))
                    .height(body.height())
                    .clip(!visible),
            ),
        )
        .children([content]);
        if visible {
            clip.restore_focus_in_subtree_on_removal(header_id)
        } else {
            clip.without_interaction(Some(header_id))
        }
    }

    fn expansion_indicator<M>(
        &self,
        header_id: &str,
        expanded: bool,
        fraction: f32,
        disabled: bool,
    ) -> View<M> {
        let size = Length::Px(INDICATOR_SIZE);
        let background = if expanded {
            self.colors.surface_container
        } else {
            self.colors.surface
        };
        let ink = if disabled {
            disabled_ink(self)
        } else {
            self.colors.on_surface
        };
        let style = Style::row()
            .width(size)
            .height(size)
            .align(Align::Center)
            .justify(Align::Center)
            .corners(shape::FULL)
            .background(background)
            .transition(motion::STATE_CHANGE);
        let chevron = super::list_icon(
            format!("{header_id}-indicator-icon"),
            CHEVRON_SIZE,
            chevron_contour(CHEVRON_SIZE, 180. * fraction, ink),
        );
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{header_id}-indicator"))
                .styled(style),
        )
        .children([chevron])
        .without_interaction(None)
    }
}

fn content_id(header_id: &str) -> String {
    format!("{header_id}-expansion-content")
}
