use std::ops::Range;

use surface::{Align, GridRole, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};

use crate::{shape::LayoutDirection, Theme};

const GAP: f32 = 8.;

/// Keep the measured rows of a set of Material chips. Use one instance per
/// set. Child IDs are the stable IDs passed to [Theme::chip_in_grid].
#[derive(Default)]
pub struct ChipGridLayout {
    width: f32,
    text_scale: f32,
    members: Vec<String>,
    rows: Vec<Range<usize>>,
}

impl ChipGridLayout {
    /// The current row ranges. Start new or resized content with one chip
    /// per row so it stays reachable while the runtime measures its width.
    pub fn rows(&self, width: f32, text_scale: f32, members: &[&str]) -> Vec<Range<usize>> {
        if self.matches(width, text_scale, members) {
            self.rows.clone()
        } else {
            (0..members.len()).map(|index| index..index + 1).collect()
        }
    }

    /// Pack the measured chip widths in author order. Return the update
    /// from the application frame hook so both paint and grid rows change
    /// together. Give each chip the available width as its width cap.
    pub fn advance(
        &mut self,
        frame: &AnimationFrame<'_>,
        width: f32,
        text_scale: f32,
        members: &[&str],
    ) -> AnimationUpdate {
        if members.is_empty() || !width.is_finite() || width <= 0. {
            return AnimationUpdate::default();
        }
        let Some(layout) = frame.layout else {
            return AnimationUpdate {
                request_next: true,
                rebuild: false,
            };
        };
        let widths: Option<Vec<_>> = members
            .iter()
            .map(|member| {
                let id = frame.dom.by_id(&format!("{member}-chip"))?;
                let measured = layout.nodes.get(id.index())?.bounds.width;
                (measured.is_finite() && measured > 0.).then_some(measured)
            })
            .collect();
        let Some(widths) = widths else {
            return AnimationUpdate::default();
        };
        let rows = pack(width, &widths);
        let changed = if self.matches(width, text_scale, members) {
            self.rows != rows
        } else {
            rows.len() != members.len()
        };
        self.width = width;
        self.text_scale = text_scale;
        self.members = members.iter().map(|member| (*member).to_owned()).collect();
        self.rows = rows;
        AnimationUpdate {
            rebuild: changed,
            request_next: false,
        }
    }

    fn matches(&self, width: f32, text_scale: f32, members: &[&str]) -> bool {
        self.width == width
            && self.text_scale == text_scale
            && self
                .members
                .iter()
                .map(String::as_str)
                .eq(members.iter().copied())
    }
}

fn pack(width: f32, chips: &[f32]) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut used = 0.;
    for (index, &chip) in chips.iter().enumerate() {
        let additional = if index == start { chip } else { GAP + chip };
        if index > start && used + additional > width {
            result.push(start..index);
            start = index;
            used = chip;
        } else {
            used += additional;
        }
    }
    if start < chips.len() {
        result.push(start..chips.len());
    }
    result
}

impl Theme {
    /// Lay out named, ordered rows of chips. Each chip's actions get their
    /// own grid cells when built with [Self::chip_in_grid]. Tab enters once.
    /// Arrows move between the actual controls without invoking them.
    pub fn chip_grid<M, Rows, Row>(
        &self,
        id: &str,
        name: &str,
        direction: LayoutDirection,
        rows: Rows,
    ) -> View<M>
    where
        Rows: IntoIterator<Item = Row>,
        Row: IntoIterator<Item = View<M>>,
    {
        let rows = rows.into_iter().enumerate().map(|(index, children)| {
            let node = Node::new(Role::Group, "")
                .id(format!("{id}-row-{index}"))
                .grid_role(GridRole::Row)
                .styled(
                    Style::row()
                        .width(Length::Percent(1.))
                        .align(Align::Center)
                        .gap(GAP),
                );
            let node = if direction == LayoutDirection::RightToLeft {
                node.reverse_row()
            } else {
                node
            };
            View::new(node).children(children)
        });
        View::new(
            Node::new(Role::Group, name)
                .id(id)
                .grid_role(GridRole::Grid)
                .styled(Style::column().width(Length::Percent(1.)).gap(GAP)),
        )
        .children(rows)
        .roving_grid()
    }
}

#[cfg(test)]
mod tests {
    use super::pack;

    #[test]
    fn rows_repack_with_the_actual_width_of_each_chip() {
        let widths = &[92., 144., 76., 55.];
        assert_eq!(pack(340., widths), [0..3, 3..4]);
        assert_eq!(pack(250., widths), [0..2, 2..4]);
        assert_eq!(pack(220., widths), [0..1, 1..2, 2..4]);
        assert_eq!(pack(72., widths), [0..1, 1..2, 2..3, 3..4]);
        assert!(pack(250., &[]).is_empty());
    }
}
