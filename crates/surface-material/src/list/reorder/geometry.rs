use surface::{scroll_extent, NodeId, Rect};
use surface_app::{AnimationFrame, PointDragLayout, PointDragTarget};

use super::{ListReorderEntry, ReorderPreview, ReorderableListOptions};

const DROP_HEIGHT: f32 = 8.;

pub(super) struct Geometry {
    list: Rect,
    pub visible: Rect,
    rows: Vec<Row>,
}

struct Row {
    id: String,
    bounds: Rect,
}

pub(super) struct Scroll {
    pub x: f32,
    pub y: f32,
    pub max_y: f32,
}

impl Geometry {
    fn new(list: Rect, scroller: PointDragTarget, viewport: Rect, rows: Vec<Row>) -> Option<Self> {
        let visible = scroller.bounds.intersect(scroller.clip).intersect(viewport);
        (visible.width > 0. && visible.height > 0.).then_some(Self {
            list,
            visible,
            rows,
        })
    }

    pub fn captured(layout: &PointDragLayout, list: &str, scroller: &str) -> Option<Self> {
        let mut targets = layout.targets();
        let first = targets.next().filter(|(id, _)| *id == list)?.1?;
        let second = targets.next().filter(|(id, _)| *id == scroller)?.1?;
        let rows = targets
            .map(|(id, geometry)| {
                Some(Row {
                    id: id.strip_suffix("-item")?.to_owned(),
                    bounds: geometry?.bounds,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Self::new(first.bounds, second, layout.viewport, rows)
    }

    pub fn frame(
        options: ReorderableListOptions<'_>,
        order: &[ListReorderEntry<'_>],
        frame: &AnimationFrame<'_>,
    ) -> Option<(Self, Scroll)> {
        let layout = frame.layout?;
        let list = frame.dom.by_id(options.id)?;
        let scroller = frame.dom.by_id(options.scroller_id)?;
        let style = frame.dom.get(scroller).ok()?.style();
        if !style.clip || !ancestor(frame, list, scroller) {
            return None;
        }
        let scroller_geometry = layout.nodes.get(scroller.index())?;
        let rows = order
            .iter()
            .map(|row| {
                let id = frame.dom.by_id(&format!("{}-item", row.id))?;
                Some(Row {
                    id: row.id.to_owned(),
                    bounds: layout.nodes.get(id.index())?.bounds,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let (_, content_height) = scroll_extent(frame.dom, layout, scroller)?;
        let scroll = Scroll {
            x: style.scroll_x,
            y: style.scroll_y,
            max_y: (content_height - scroller_geometry.bounds.height).max(0.),
        };
        let geometry = Self::new(
            layout.nodes.get(list.index())?.bounds,
            PointDragTarget {
                bounds: scroller_geometry.bounds,
                clip: scroller_geometry.clip,
            },
            layout.viewport,
            rows,
        )?;
        Some((geometry, scroll))
    }

    pub fn matches(&self, order: &[ListReorderEntry<'_>]) -> bool {
        self.rows.len() == order.len()
            && self
                .rows
                .iter()
                .zip(order)
                .all(|(row, entry)| row.id == entry.id)
    }

    pub fn grab_offset(&self, index: usize, origin_y: f32) -> f32 {
        let row = self.rows[index].bounds;
        (origin_y - row.y).clamp(0., row.height)
    }

    pub fn destination(
        &self,
        order: &[ListReorderEntry<'_>],
        index: usize,
        position: (f32, f32),
    ) -> Option<usize> {
        if !self.visible.contains(position.0, position.1) {
            return None;
        }
        let y = position.1;
        let start = order[..index]
            .iter()
            .rposition(|entry| !entry.movable)
            .map_or(0, |barrier| barrier + 1);
        let end = order[index + 1..]
            .iter()
            .position(|entry| !entry.movable)
            .map_or(order.len(), |relative| index + 1 + relative);
        if (start > 0 && y < bottom(self.rows[start - 1].bounds))
            || (end < self.rows.len() && y >= self.rows[end].bounds.y)
        {
            return None;
        }
        let before = (start..end)
            .filter(|row| *row != index)
            .find(|row| {
                let bounds = self.rows[*row].bounds;
                y < bounds.y + bounds.height / 2.
            })
            .unwrap_or(end);
        (before != index + 1).then_some(before)
    }

    pub fn preview(
        &self,
        index: usize,
        position: (f32, f32),
        grab_y: f32,
        destination: Option<usize>,
    ) -> ReorderPreview {
        let row = &self.rows[index];
        let lift = Rect {
            x: row.bounds.x - self.list.x,
            y: position.1 - grab_y - self.list.y,
            width: row.bounds.width,
            height: row.bounds.height,
        };
        let drop = destination.map(|index| {
            let boundary = self
                .rows
                .get(index)
                .map_or_else(|| bottom(self.list), |row| row.bounds.y);
            let height = DROP_HEIGHT.min(self.visible.height);
            Rect {
                x: lift.x,
                y: (boundary - height / 2.).clamp(self.visible.y, bottom(self.visible) - height)
                    - self.list.y,
                width: lift.width,
                height,
            }
        });
        ReorderPreview {
            item: row.id.clone(),
            lift,
            drop,
        }
    }

    /// Relate the preview to the same offset the runtime will apply this frame.
    pub fn scroll_by(&mut self, delta: f32) {
        self.list.y -= delta;
        for row in &mut self.rows {
            row.bounds.y -= delta;
        }
    }
}

fn bottom(rect: Rect) -> f32 {
    rect.y + rect.height
}

fn ancestor(frame: &AnimationFrame<'_>, mut child: NodeId, ancestor: NodeId) -> bool {
    while let Some(parent) = frame.dom.get(child).ok().and_then(|node| node.parent()) {
        if parent == ancestor {
            return true;
        }
        child = parent;
    }
    false
}
