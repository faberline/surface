use super::*;

const FACE: Rect = Rect {
    x: 12.,
    y: 28.,
    width: 112.,
    height: 56.,
};
const COLOR: Color = Color(60, 44, 122, 255);

fn label(progress: f32, direction: LayoutDirection) -> LabelNotch {
    let x = match direction {
        LayoutDirection::LeftToRight => FACE.x + 16.,
        LayoutDirection::RightToLeft => FACE.x + FACE.width - 16. - 60.,
    };
    LabelNotch {
        progress,
        minimized: Rect::new(x, FACE.y - 8., 60., 16.),
        measured_size: (64., 16.),
        direction,
    }
}

// The exact .66 point sequence, including its shared seam for a closed ring.
fn legacy(stroke: f32, gap: Option<(f32, f32)>) -> FillContour {
    let (width, height) = (FACE.width, FACE.height);
    let (left, right) = gap.unwrap_or((width / 2., width / 2.));
    let points = |inset| {
        let radius = 4. - inset;
        let mut outline = vec![[right, inset]];
        for (x, y, start) in [
            (width - 4., 4., -FRAC_PI_2),
            (width - 4., height - 4., 0.),
            (4., height - 4., FRAC_PI_2),
            (4., 4., 2. * FRAC_PI_2),
        ] {
            for step in 0..=4 {
                let angle = start + step as f32 * FRAC_PI_2 / 4.;
                outline.push([x + radius * angle.cos(), y + radius * angle.sin()]);
            }
        }
        outline.push([left, inset]);
        outline
    };
    FillContour::new(
        points(0.)
            .into_iter()
            .chain(points(stroke).into_iter().rev()),
        COLOR,
    )
    .unwrap()
}

fn ink(points: &[[f32; 2]], x: f32, y: f32) -> bool {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .filter(|(a, b)| {
            (a[1] > y) != (b[1] > y) && x < a[0] + (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1])
        })
        .count()
        % 2
        == 1
}

#[test]
fn exact_endpoints_preserve_the_previous_contour_for_both_directions() {
    for stroke in [1., 3.] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let expected = match direction {
                LayoutDirection::LeftToRight => (12., 80.),
                LayoutDirection::RightToLeft => (32., 100.),
            };
            assert_eq!(
                outline_contour(FACE, stroke, Some(label(0., direction)), COLOR),
                Some(legacy(stroke, None))
            );
            assert_eq!(
                outline_contour(FACE, stroke, Some(label(1., direction)), COLOR),
                Some(legacy(stroke, Some(expected)))
            );
        }
    }
}

#[test]
fn shallow_cut_removes_only_the_upper_part_of_the_stroke() {
    for stroke in [1., 3.] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let gap = cutout(FACE, stroke, label(0.0625, direction)).unwrap();
            let expected = match direction {
                LayoutDirection::LeftToRight => (12., 24.),
                LayoutDirection::RightToLeft => (88., 100.),
            };
            assert_eq!(
                (gap.left, gap.right, gap.depth),
                (expected.0, expected.1, 0.5)
            );
            let points = outline_points(FACE, stroke, Some(gap));
            let middle = (gap.left + gap.right) / 2.;
            assert!(!ink(&points, middle, 0.25));
            assert!(ink(&points, middle, 0.75));
            assert!(!ink(&points, middle, stroke + 1.));
            assert!(ink(&points, gap.left - 1., 0.25));
            assert!(ink(&points, gap.right + 1., 0.25));
            assert!(ink(&points, 0.5, FACE.height / 2.));
            assert!(ink(&points, FACE.width - 0.5, FACE.height / 2.));
            assert!(ink(&points, FACE.width / 2., FACE.height - 0.5));
            assert!(FillContour::new(points, COLOR).is_some());
        }
    }
}

#[test]
fn measured_size_is_scaled_and_anchored_once_in_each_direction() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let gap = cutout(FACE, 3., label(0.5, direction)).unwrap();
        let expected = match direction {
            LayoutDirection::LeftToRight => (12., 52.),
            LayoutDirection::RightToLeft => (60., 100.),
        };
        assert_eq!(
            (gap.left, gap.right, gap.depth),
            (expected.0, expected.1, 3.)
        );
        let points = outline_points(FACE, 3., Some(gap));
        let middle = (gap.left + gap.right) / 2.;
        assert!(!ink(&points, middle, 0.5));
        assert!(!ink(&points, middle, 2.5));
        assert!(ink(&points, gap.left - 1., 2.5));
        assert!(ink(&points, gap.right + 1., 2.5));
    }
}

#[test]
fn wide_intermediate_label_stays_inside_the_straight_edge() {
    for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
        let widest = LabelNotch {
            measured_size: (600., 16.),
            ..label(0.5, direction)
        };
        let gap = cutout(FACE, 3., widest).unwrap();
        let expected = match direction {
            LayoutDirection::LeftToRight => (12., 108.),
            LayoutDirection::RightToLeft => (4., 100.),
        };
        assert_eq!((gap.left, gap.right), expected);
        let points = outline_points(FACE, 3., Some(gap));
        assert!(ink(&points, 0.5, FACE.height / 2.));
        assert!(ink(&points, FACE.width - 0.5, FACE.height / 2.));
    }
}

#[test]
fn empty_above_or_oversized_minimized_label_has_no_notch() {
    for progress in [0.25, 1.] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            for minimized in [
                Rect::new(FACE.x + 16., FACE.y - 8., 0., 16.),
                Rect::new(FACE.x + 16., FACE.y - 16., 60., 16.),
                Rect::new(FACE.x + 2., FACE.y - 8., 90., 16.),
                Rect::new(FACE.x + 16., FACE.y - 8., 102., 16.),
            ] {
                let fallback = LabelNotch {
                    minimized,
                    ..label(progress, direction)
                };
                assert_eq!(
                    outline_contour(FACE, 3., Some(fallback), COLOR),
                    Some(legacy(3., None))
                );
            }
        }
    }
}

#[test]
fn invalid_motion_values_leave_the_border_closed() {
    let direction = LayoutDirection::LeftToRight;
    for invalid in [
        label(f32::NAN, direction),
        label(f32::INFINITY, direction),
        label(-0.5, direction),
        LabelNotch {
            measured_size: (0., 16.),
            ..label(0.5, direction)
        },
        LabelNotch {
            measured_size: (64., 0.),
            ..label(0.5, direction)
        },
        LabelNotch {
            measured_size: (64., f32::NAN),
            ..label(0.5, direction)
        },
    ] {
        assert_eq!(
            outline_contour(FACE, 3., Some(invalid), COLOR),
            Some(legacy(3., None))
        );
    }
    assert!(outline_contour(Rect::new(0., 0., 7., 56.), 3., None, COLOR).is_none());
    assert!(outline_contour(FACE, f32::NAN, None, COLOR).is_none());
}
