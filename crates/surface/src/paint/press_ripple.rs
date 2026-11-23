use std::f64::consts::{FRAC_PI_2, TAU};

use crate::{microdom::PresentedPressRipple, Color, Corners, PathFill, Rect};

type Point = [f64; 2];

// The convex intersection carries at most the sum of the input edge counts.
const CIRCLE_STEPS: u32 = 96;
const CORNER_STEPS: u32 = 12;

pub(super) fn contour(
    wave: PresentedPressRipple,
    rect: Rect,
    clip: Rect,
    corners: Corners,
    color: Color,
) -> Option<PathFill> {
    if color.3 == 0 || !valid_rect(rect) || !valid_rect(clip) || !corners.is_valid() {
        return None;
    }
    let visible = rect.intersect(clip);
    if !valid_rect(visible) {
        return None;
    }
    let resolved = corners.resolve(rect.width, rect.height);
    let width = f64::from(rect.width);
    let height = f64::from(rect.height);
    let boundary = rounded_boundary(width, height, resolved);
    let mut points = if wave.progress >= 1. {
        boundary
    } else {
        let x = f64::from(wave.center.0).clamp(0., width);
        let y = f64::from(wave.center.1).clamp(0., height);
        let cover = x.max(width - x).hypot(y.max(height - y));
        let start = 0.3 * width.min(height);
        let radius = start + (cover - start) * f64::from(wave.progress);
        let circle = (0..CIRCLE_STEPS)
            .map(|step| {
                let angle = TAU * f64::from(step) / f64::from(CIRCLE_STEPS);
                let (sin, cos) = angle.sin_cos();
                [x + radius * cos, y + radius * sin]
            })
            .collect();
        clip_convex(circle, &boundary)
    };
    if visible != rect {
        let x = f64::from(visible.x) - f64::from(rect.x);
        let y = f64::from(visible.y) - f64::from(rect.y);
        let right = x + f64::from(visible.width);
        let bottom = y + f64::from(visible.height);
        points = clip_convex(points, &[[x, y], [right, y], [right, bottom], [x, bottom]]);
    }
    let origin_x = f64::from(rect.x);
    let origin_y = f64::from(rect.y);
    PathFill::new(
        points.into_iter().map(|[x, y]| {
            [
                (origin_x + x.clamp(0., width)) as f32,
                (origin_y + y.clamp(0., height)) as f32,
            ]
        }),
        color,
        clip,
    )
}

fn valid_rect(rect: Rect) -> bool {
    rect.width > 0.
        && rect.height > 0.
        && [rect.x, rect.y, rect.x + rect.width, rect.y + rect.height]
            .into_iter()
            .all(f32::is_finite)
}

fn rounded_boundary(width: f64, height: f64, corners: Corners) -> Vec<Point> {
    let top_left = f64::from(corners.top_left);
    let top_right = f64::from(corners.top_right);
    let bottom_right = f64::from(corners.bottom_right);
    let bottom_left = f64::from(corners.bottom_left);
    [
        ([width - top_right, top_right], top_right),
        ([width - bottom_right, height - bottom_right], bottom_right),
        ([bottom_left, height - bottom_left], bottom_left),
        ([top_left, top_left], top_left),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(corner, ([x, y], radius))| {
        let steps = if radius == 0. { 0 } else { CORNER_STEPS };
        (0..=steps).map(move |step| {
            let angle =
                FRAC_PI_2 * (corner as f64 - 1. + f64::from(step) / f64::from(CORNER_STEPS));
            let (sin, cos) = angle.sin_cos();
            [x + radius * cos, y + radius * sin]
        })
    })
    .collect()
}

/// Sutherland–Hodgman clipping against a clockwise convex polygon in y-down coordinates.
fn clip_convex(mut points: Vec<Point>, boundary: &[Point]) -> Vec<Point> {
    let mut scratch = Vec::with_capacity(points.len() + boundary.len());
    for (&start, &end) in boundary.iter().zip(boundary.iter().cycle().skip(1)) {
        let Some(&last) = points.last() else {
            break;
        };
        let side = |point: Point| {
            (end[0] - start[0]) * (point[1] - start[1])
                - (end[1] - start[1]) * (point[0] - start[0])
        };
        let mut previous = last;
        let mut previous_side = side(previous);
        for &point in &points {
            let current_side = side(point);
            let inside = current_side >= 0.;
            if inside != (previous_side >= 0.) {
                let fraction = previous_side / (previous_side - current_side);
                scratch.push([
                    previous[0] + (point[0] - previous[0]) * fraction,
                    previous[1] + (point[1] - previous[1]) * fraction,
                ]);
            }
            if inside {
                scratch.push(point);
            }
            previous = point;
            previous_side = current_side;
        }
        std::mem::swap(&mut points, &mut scratch);
        scratch.clear();
    }
    points
}
