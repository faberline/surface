use std::f32::consts::FRAC_PI_2;

use surface::{Color, FillContour, Rect};

use crate::shape::LayoutDirection;

const CORNER: f32 = 4.;
const NOTCH_PAD: f32 = 4.;

/// The notch and label use the same sampled progress and measured label size.
pub(crate) struct LabelNotch {
    pub(crate) progress: f32,
    pub(crate) measured_size: (f32, f32),
    /// The floating endpoint in the same physical coordinate space as the face.
    pub(crate) minimized: Rect,
    pub(crate) direction: LayoutDirection,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Cutout {
    left: f32,
    right: f32,
    depth: f32,
}

pub(crate) fn outline_contour(
    face: Rect,
    stroke: f32,
    notch: Option<LabelNotch>,
    color: Color,
) -> Option<FillContour> {
    if ![face.x, face.y, face.width, face.height, stroke]
        .into_iter()
        .all(f32::is_finite)
        || face.width < 2. * CORNER
        || face.height < 2. * CORNER
        || stroke <= 0.
        || stroke > CORNER
    {
        return None;
    }
    let cutout = notch.and_then(|notch| cutout(face, stroke, notch));
    FillContour::new(outline_points(face, stroke, cutout), color)
}

fn cutout(face: Rect, stroke: f32, notch: LabelNotch) -> Option<Cutout> {
    if !notch.progress.is_finite() || notch.progress <= 0. {
        return None;
    }
    let minimized = notch.minimized;
    let left = minimized.x - face.x;
    let end = left + minimized.width;
    let max = face.width - CORNER;
    let full = Cutout {
        left: left - NOTCH_PAD,
        right: end + NOTCH_PAD,
        depth: stroke,
    };
    // The minimized label must fit the straight edge. Empty or above-field
    // fallback bounds keep the border closed throughout the transition.
    if ![left, end, minimized.y, minimized.height]
        .into_iter()
        .all(f32::is_finite)
        || minimized.width <= 0.
        || minimized.height <= 0.
        || minimized.y + minimized.height <= face.y
        || full.left < CORNER
        || full.right > max
        || full.left >= full.right
    {
        return None;
    }
    if notch.progress >= 1. {
        return Some(full);
    }
    let width = notch.progress * notch.measured_size.0;
    let depth = notch.progress * notch.measured_size.1 / 2.;
    if !width.is_finite() || !depth.is_finite() || width <= 0. || depth <= 0. {
        return None;
    }
    let (start, end) = match notch.direction {
        LayoutDirection::LeftToRight => (left, left + width),
        LayoutDirection::RightToLeft => (end - width, end),
    };
    let left = (start - NOTCH_PAD).max(CORNER);
    let right = (end + NOTCH_PAD).min(max);
    (left < right).then_some(Cutout {
        left,
        right,
        depth: depth.min(stroke),
    })
}

fn outline_points(face: Rect, stroke: f32, cutout: Option<Cutout>) -> Vec<[f32; 2]> {
    let (left, right) = cutout
        .map(|gap| (gap.left, gap.right))
        .unwrap_or((face.width / 2., face.width / 2.));
    let mut points = edge(face, 0., left, right);
    let inner_left = if let Some(gap) = cutout.filter(|gap| gap.depth < stroke) {
        // Close the outside loop along the shallow cut, then join the closed
        // inner loop at the right edge. The shared seam cancels in even-odd fill.
        points.extend([[left, gap.depth], [right, gap.depth], [right, 0.]]);
        right
    } else {
        left
    };
    points.extend(edge(face, stroke, inner_left, right).into_iter().rev());
    points
}

fn edge(face: Rect, inset: f32, left: f32, right: f32) -> Vec<[f32; 2]> {
    let radius = CORNER - inset;
    let mut points = vec![[right, inset]];
    for (x, y, start) in [
        (face.width - CORNER, CORNER, -FRAC_PI_2),
        (face.width - CORNER, face.height - CORNER, 0.),
        (CORNER, face.height - CORNER, FRAC_PI_2),
        (CORNER, CORNER, 2. * FRAC_PI_2),
    ] {
        for step in 0..=4 {
            let angle = start + step as f32 * FRAC_PI_2 / 4.;
            points.push([x + radius * angle.cos(), y + radius * angle.sin()]);
        }
    }
    points.push([left, inset]);
    points
}

#[cfg(test)]
#[path = "text_field_label_notch_tests.rs"]
mod tests;
