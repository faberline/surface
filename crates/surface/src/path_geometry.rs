use crate::{Color, PaintOp, Rect};
use serde::de;
use std::{fmt, ops::Range};

pub(crate) const MAX_POINTS: usize = 256;

pub(crate) fn collect_points(points: impl IntoIterator<Item = [f32; 2]>) -> Option<Vec<[f32; 2]>> {
    let mut distinct = Vec::new();
    for (index, point) in points.into_iter().enumerate() {
        if index == MAX_POINTS || point.iter().any(|value| !value.is_finite()) {
            return None;
        }
        if distinct.last() != Some(&point) {
            distinct.push(point);
        }
    }
    Some(distinct)
}

pub(crate) fn point_extrema(points: &[[f32; 2]]) -> Option<[f32; 4]> {
    let [mut left, mut top] = *points.first()?;
    let (mut right, mut bottom) = (left, top);
    for &[x, y] in &points[1..] {
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    Some([left, top, right, bottom])
}

pub(crate) fn valid_rect(rect: Rect) -> bool {
    let right = rect.x + rect.width;
    let bottom = rect.y + rect.height;
    rect.x.is_finite()
        && rect.y.is_finite()
        && right.is_finite()
        && bottom.is_finite()
        && right > rect.x
        && bottom > rect.y
}

pub(crate) fn bounded_points<'de, D: serde::Deserializer<'de>>(
    input: D,
) -> Result<Vec<[f32; 2]>, D::Error> {
    struct Points;

    impl<'de> de::Visitor<'de> for Points {
        type Value = Vec<[f32; 2]>;

        fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(fmt, "at most {MAX_POINTS} path points")
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let capacity = seq.size_hint().unwrap_or(0).min(MAX_POINTS);
            let mut points = Vec::with_capacity(capacity);
            while let Some(point) = seq.next_element()? {
                if points.len() == MAX_POINTS {
                    return Err(de::Error::invalid_length(points.len() + 1, &self));
                }
                points.push(point);
            }
            Ok(points)
        }
    }

    input.deserialize_seq(Points)
}

pub(crate) struct Axis {
    origin: f64,
    step: f64,
    pub(crate) cells: usize,
}

impl Axis {
    pub(crate) fn new(start: f32, size: f32) -> Self {
        const CELL_LIMIT: usize = 256;
        let start = f64::from(start);
        let size = f64::from(size);
        // Coarsen each axis independently to keep detail along thin paths.
        let step = (size / (CELL_LIMIT - 1) as f64).ceil().max(1.);
        let origin = (start / step).floor() * step;
        let cells = (((start - origin + size) / step).ceil() as usize).clamp(1, CELL_LIMIT);
        Self {
            origin,
            step,
            cells,
        }
    }

    /// Cell indices with the given sample offset in [start, end).
    pub(crate) fn samples_between(&self, start: f64, end: f64, offset: f64) -> Range<usize> {
        let index = |position: f64| {
            ((((position - self.origin) / self.step) - offset).ceil() as usize).min(self.cells)
        };
        index(start)..index(end)
    }

    pub(crate) fn position(&self, cell: usize, offset: f64) -> f64 {
        self.origin + (cell as f64 + offset) * self.step
    }
}

/// Merge same-coverage neighbors into disjoint legacy rectangles.
pub(crate) fn emit_coverage_row(
    horizontal: &Axis,
    row_bounds: [f32; 2],
    cells: &[u8],
    clip: Rect,
    color: Color,
    mut push: impl FnMut(PaintOp),
) {
    let alphas = cells
        .iter()
        .map(|mask| ((u32::from(color.3) * mask.count_ones() + 2) / 4) as u8)
        .chain([0]);
    let [y, bottom] = row_bounds;
    let mut run_start = 0;
    let mut previous = 0;
    for (column, alpha) in alphas.enumerate() {
        if previous == alpha {
            continue;
        }
        if previous != 0 {
            let x = horizontal.position(run_start, 0.) as f32;
            let right = horizontal.position(column, 0.) as f32;
            let rect = Rect::new(x, y, right - x, bottom - y);
            if valid_rect(rect.intersect(clip)) {
                push(PaintOp::Quad {
                    rect,
                    clip,
                    radius: 0.,
                    color: Color(color.0, color.1, color.2, previous),
                });
            }
        }
        run_start = column;
        previous = alpha;
    }
}
