use surface::{Insets, Length, Node, Role, Style};
use surface_app::View;

use crate::{shape::LayoutDirection, DividerKind, Theme};

const INSET: f32 = 16.;

/// The thickness of a decorative divider in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DividerWeight {
    /// A 1 dp line to group related content.
    Regular,
    /// An 8 dp line to mark a break between major sections.
    Heavy,
}

/// Position a decorative divider in its parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DividerLayout {
    /// Draw a horizontal line across the width available to the divider.
    Horizontal {
        kind: DividerKind,
        direction: LayoutDirection,
    },
    /// Draw a vertical line across the parent's available height.
    Vertical,
}

impl Theme {
    /// Draw a Material divider that does not take focus or appear in the
    /// accessibility tree. Group related content with headings or other
    /// semantics when the visual separation matters to the reader.
    pub fn decorative_divider<M>(&self, id: &str, layout: DividerLayout) -> View<M> {
        self.decorative_divider_with_weight(id, layout, DividerWeight::Regular)
    }

    /// Draw a decorative Material divider with the selected thickness.
    /// The weight controls height for a horizontal line and width for a
    /// vertical line. Insets and the outline-variant color are shared.
    pub fn decorative_divider_with_weight<M>(
        &self,
        id: &str,
        layout: DividerLayout,
        weight: DividerWeight,
    ) -> View<M> {
        let thickness = match weight {
            DividerWeight::Regular => 1.,
            DividerWeight::Heavy => 8.,
        };
        let line = |style: Style| {
            View::new(
                Node::new(Role::Separator, "")
                    .id(id)
                    .styled(style.background(self.colors.outline_variant)),
            )
        };
        match layout {
            DividerLayout::Vertical => line(
                Style::column()
                    .width(Length::Px(thickness))
                    .height(Length::Percent(1.)),
            ),
            DividerLayout::Horizontal { kind, direction } => {
                let row = Style::column()
                    .width(Length::Percent(1.))
                    .height(Length::Px(thickness));
                match kind {
                    DividerKind::Full => line(row),
                    DividerKind::Inset | DividerKind::MiddleInset => {
                        let end = if kind == DividerKind::MiddleInset {
                            INSET
                        } else {
                            0.
                        };
                        let (left, right) = match direction {
                            LayoutDirection::LeftToRight => (INSET, end),
                            LayoutDirection::RightToLeft => (end, INSET),
                        };
                        let ink = Style::column()
                            .height(Length::Px(thickness))
                            .absolute(Insets {
                                left: Length::Px(left),
                                right: Length::Px(right),
                                top: Length::Px(0.),
                                ..Insets::default()
                            });
                        View::new(
                            Node::new(Role::Group, "")
                                .id(format!("{id}-container"))
                                .styled(row.clip(true)),
                        )
                        .children([line(ink)])
                    }
                }
            }
        }
        .without_interaction(None)
    }
}
