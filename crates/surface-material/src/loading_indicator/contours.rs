//! Matched points from the official Graphics Shapes morphs.

#[path = "contour_data.rs"]
mod data;

const SAMPLE_UNITS: f32 = 32_768.;
// A damping ratio of 0.6 yields a maximum normalized spring progress of 1.094781.
const MAX_PROGRESS: f32 = 1.10;

pub(super) fn contour(pair: usize, progress: f32) -> impl Iterator<Item = [f32; 2]> {
    let pair = pair % data::PAIRS.len();
    let progress = if progress.is_finite() {
        progress.clamp(0., MAX_PROGRESS)
    } else {
        0.
    };
    // Use the next pair's first polygon at the exact junction. Both sets of
    // points describe the same curve, but each pair can subdivide it differently.
    let (points, progress) = if progress == 1. {
        (data::PAIRS[(pair + 1) % data::PAIRS.len()], 0.)
    } else {
        (data::PAIRS[pair], progress)
    };
    let interpolate = move |&[x0, y0, x1, y1]: &[i16; 4]| {
        let axis = |a: i16, b: i16| {
            (f32::from(a) + (f32::from(b) - f32::from(a)) * progress) / SAMPLE_UNITS
        };
        [axis(x0, x1), axis(y0, y1)]
    };
    let first = interpolate(&points[0]);
    let (minimum, maximum) = points.iter().skip(1).map(interpolate).fold(
        (first, first),
        |([min_x, min_y], [max_x, max_y]), [x, y]| {
            ([min_x.min(x), min_y.min(y)], [max_x.max(x), max_y.max(y)])
        },
    );
    let center = [
        (minimum[0] + maximum[0]) * 0.5,
        (minimum[1] + maximum[1]) * 0.5,
    ];
    points.iter().map(move |point| {
        let [x, y] = interpolate(point);
        [x - center[0], y - center[1]]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use surface::{Color, FillContour};

    #[test]
    fn every_pair_fits_at_intermediate_and_overshoot_progress() {
        for pair in 0..data::PAIRS.len() {
            for step in 0..=110 {
                let points: Vec<_> = contour(pair, step as f32 / 100.).collect();
                assert!(points.len() <= FillContour::MAX_POINTS);
                assert!(points.iter().all(|[x, y]| x.hypot(*y) < 0.5));
                assert!(FillContour::new(points, Color::rgb(0, 0, 0)).is_some());
            }
        }
    }

    #[test]
    fn pair_junctions_match_and_morphs_include_intermediate_and_overshoot_shapes() {
        for pair in 0..data::PAIRS.len() {
            assert!(contour(pair, 1.).eq(contour(pair + 1, 0.)));
            assert!(contour(pair, f32::NAN).eq(contour(pair, 0.)));
            assert!(!contour(pair, 0.4).eq(contour(pair, 0.)));
            assert!(!contour(pair, 1.09).eq(contour(pair, 1.)));
        }
    }
}
