use crate::{
    path_geometry::{
        bounded_points, collect_points, emit_coverage_row, point_extrema, valid_rect, Axis,
    },
    Color, PaintOp, Rect,
};
use serde::{Deserialize, Serialize};

/// One closed, even-odd fill in node-local logical pixels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ContourFields")]
pub struct FillContour {
    points: Vec<[f32; 2]>,
    color: Color,
}

/// A closed, even-odd fill in display-list coordinates with an ancestor clip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "FillFields")]
pub struct PathFill {
    clip: Rect,
    contour: FillContour,
    #[serde(skip)]
    bounds: Rect,
}

#[derive(Deserialize)]
struct ContourFields {
    #[serde(deserialize_with = "bounded_points")]
    points: Vec<[f32; 2]>,
    color: Color,
}

impl TryFrom<ContourFields> for FillContour {
    type Error = &'static str;

    fn try_from(fields: ContourFields) -> Result<Self, Self::Error> {
        Self::new(fields.points, fields.color).ok_or("invalid fill contour")
    }
}

#[derive(Deserialize)]
struct FillFields {
    clip: Rect,
    contour: FillContour,
}

impl TryFrom<FillFields> for PathFill {
    type Error = &'static str;

    fn try_from(fields: FillFields) -> Result<Self, Self::Error> {
        Self::from_contour(fields.contour, fields.clip).ok_or("invalid fill clip or bounds")
    }
}

impl FillContour {
    pub const MAX_POINTS: usize = crate::path_geometry::MAX_POINTS;

    /// Accepts 3 to 256 finite points with at least three non-collinear
    /// points. Consecutive and closing duplicates collapse. The last point
    /// connects to the first. Self-crossings follow the even-odd fill rule.
    pub fn new(points: impl IntoIterator<Item = [f32; 2]>, color: Color) -> Option<Self> {
        let mut points = collect_points(points)?;
        if points.first() == points.last() {
            points.pop();
        }
        let [origin, second, rest @ ..] = points.as_slice() else {
            return None;
        };
        let [x, y] = origin.map(f64::from);
        let [dx, dy] = [f64::from(second[0]) - x, f64::from(second[1]) - y];
        if !rest
            .iter()
            .any(|&[px, py]| dx * (f64::from(py) - y) != dy * (f64::from(px) - x))
        {
            return None;
        }
        fill_bounds(&points)?;
        Some(Self { points, color })
    }

    pub(crate) fn resolve(&self, node: Rect, clip: Rect) -> Option<PathFill> {
        if self.color.3 == 0 {
            return None;
        }
        let points = self.points.iter().map(|&[x, y]| [node.x + x, node.y + y]);
        let path = PathFill::new(points, self.color, clip)?;
        valid_rect(path.rect().intersect(clip)).then_some(path)
    }
}

impl PathFill {
    /// Creates a fill from global logical points and its independent clip.
    pub fn new(
        points: impl IntoIterator<Item = [f32; 2]>,
        color: Color,
        clip: Rect,
    ) -> Option<Self> {
        Self::from_contour(FillContour::new(points, color)?, clip)
    }

    fn from_contour(contour: FillContour, clip: Rect) -> Option<Self> {
        let bounds = fill_bounds(&contour.points)?;
        valid_rect(clip).then_some(Self {
            clip,
            contour,
            bounds,
        })
    }

    pub fn points(&self) -> &[[f32; 2]] {
        &self.contour.points
    }

    pub fn color(&self) -> Color {
        self.contour.color
    }

    pub fn rect(&self) -> Rect {
        self.bounds
    }

    pub fn clip(&self) -> Rect {
        self.clip
    }

    /// Approximate fill coverage for legacy display lists. Crossings are
    /// sorted once per sample row so each pixel does not walk all edges.
    pub(crate) fn legacy_ops(self, mut push: impl FnMut(PaintOp)) {
        let color = self.color();
        let clip = self.rect().intersect(self.clip());
        if color.3 == 0 || !valid_rect(clip) {
            return;
        }
        let points = self.points();
        let edges: Vec<_> = points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .filter_map(|(a, b)| {
                let (low, high) = if a[1] <= b[1] { (a, b) } else { (b, a) };
                let [x, y] = low.map(f64::from);
                let [end_x, end_y] = high.map(f64::from);
                // Half-open y intervals count a shared vertex only once.
                (y < end_y).then(|| (x, y, end_y, (end_x - x) / (end_y - y)))
            })
            .collect();

        let horizontal = Axis::new(clip.x, clip.width);
        let vertical = Axis::new(clip.y, clip.height);
        let mut masks = vec![0u8; horizontal.cells];
        let mut crossings = Vec::with_capacity(edges.len());
        const SAMPLES: [f64; 2] = [0.25, 0.75];
        for row in 0..vertical.cells {
            masks.fill(0);
            for (sample_row, offset) in SAMPLES.into_iter().enumerate() {
                let sample_y = vertical.position(row, offset);
                crossings.clear();
                crossings.extend(edges.iter().filter_map(|&(x, y, end_y, slope)| {
                    (y <= sample_y && sample_y < end_y).then_some(x + (sample_y - y) * slope)
                }));
                crossings.sort_unstable_by(f64::total_cmp);
                for pair in crossings.chunks_exact(2) {
                    for (sample_column, offset) in SAMPLES.into_iter().enumerate() {
                        let columns = horizontal.samples_between(pair[0], pair[1], offset);
                        let bit = 1 << (sample_row * SAMPLES.len() + sample_column);
                        for mask in &mut masks[columns] {
                            *mask |= bit;
                        }
                    }
                }
            }
            emit_coverage_row(
                &horizontal,
                [
                    vertical.position(row, 0.) as f32,
                    vertical.position(row + 1, 0.) as f32,
                ],
                &masks,
                clip,
                color,
                &mut push,
            );
        }
    }
}

fn fill_bounds(points: &[[f32; 2]]) -> Option<Rect> {
    let [left, top, right, bottom] = point_extrema(points)?;
    let rect = Rect::new(left, top, right - left, bottom - top);
    valid_rect(rect).then_some(rect)
}
