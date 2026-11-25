use surface::{ComputedDom, NodeId, Rect};

use crate::EditKey;

pub(crate) type Center = (f64, f64);

pub(crate) fn center(rect: Rect) -> Center {
    (
        f64::from(rect.x) + f64::from(rect.width) / 2.,
        f64::from(rect.y) + f64::from(rect.height) / 2.,
    )
}

struct Cell {
    id: NodeId,
    rect: Rect,
}

struct Row {
    top: f32,
    bottom: f32,
    cells: Vec<Cell>,
}

impl Row {
    fn new(cell: Cell) -> Self {
        Self {
            top: cell.rect.y,
            bottom: cell.rect.y + cell.rect.height,
            cells: vec![cell],
        }
    }

    fn overlaps(&self, cell: &Cell) -> bool {
        cell.rect.y < self.bottom && cell.rect.y + cell.rect.height > self.top
    }

    fn add(&mut self, cell: Cell) {
        // Keep the shared vertical span. A tall or offset cell must not
        // merge two other rows that do not overlap each other.
        self.top = self.top.max(cell.rect.y);
        self.bottom = self.bottom.min(cell.rect.y + cell.rect.height);
        self.cells.push(cell);
    }
}

pub(crate) fn is_navigation(key: EditKey) -> bool {
    matches!(
        key,
        EditKey::Left | EditKey::Right | EditKey::Up | EditKey::Down | EditKey::Home | EditKey::End
    )
}

/// The chip's focusable actions supply the geometry. Rows follow the layout's
/// vertical overlap. Authored order breaks ties and gives Home/End their
/// logical direction even when the physical row is mirrored.
pub(crate) fn target(
    key: EditKey,
    focused: NodeId,
    members: &[NodeId],
    layout: &ComputedDom,
) -> Option<NodeId> {
    let mut cells: Vec<_> = members
        .iter()
        .map(|&id| Cell {
            id,
            rect: layout.nodes[id.index()].bounds,
        })
        .collect();
    cells.sort_by(|a, b| a.rect.y.total_cmp(&b.rect.y));
    let mut rows: Vec<Row> = Vec::new();
    for cell in cells {
        if let Some(row) = rows.last_mut().filter(|row| row.overlaps(&cell)) {
            row.add(cell);
        } else {
            rows.push(Row::new(cell));
        }
    }
    let row_index = rows
        .iter()
        .position(|row| row.cells.iter().any(|cell| cell.id == focused))?;
    let row = &rows[row_index];
    let (x, _) = center(layout.nodes[focused.index()].bounds);
    let candidate = match key {
        EditKey::Home => row.cells.iter().min_by_key(|cell| cell.id.index()),
        EditKey::End => row.cells.iter().max_by_key(|cell| cell.id.index()),
        EditKey::Left => nearest_x(row.cells.iter().filter(|cell| center(cell.rect).0 < x), x),
        EditKey::Right => nearest_x(row.cells.iter().filter(|cell| center(cell.rect).0 > x), x),
        EditKey::Up => nearest_x(rows.get(row_index.checked_sub(1)?)?.cells.iter(), x),
        EditKey::Down => nearest_x(rows.get(row_index + 1)?.cells.iter(), x),
        _ => None,
    }?;
    Some(candidate.id)
}

fn nearest_x<'a>(cells: impl Iterator<Item = &'a Cell>, x: f64) -> Option<&'a Cell> {
    cells.min_by(|a, b| {
        (center(a.rect).0 - x)
            .abs()
            .total_cmp(&(center(b.rect).0 - x).abs())
            .then_with(|| a.id.index().cmp(&b.id.index()))
    })
}
