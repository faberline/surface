use surface::{
    Color, ExtendedDisplayList, ExtendedPaintOp, NodeId, PaintOp, Rect, Role, TextMeasurer,
};
use surface_app::{Application, Runtime, View};
use surface_material::{
    shape::LayoutDirection::{self, LeftToRight, RightToLeft},
    ProgressIndicatorOptions, ProgressShape, ProgressState, Theme,
};

const ID: &str = "upload";
const NAME: &str = "上傳進度";

#[derive(Clone, Copy)]
struct Sample {
    width: f32,
    side: LayoutDirection,
    dark: bool,
    options: ProgressIndicatorOptions<'static>,
}

impl Default for Sample {
    fn default() -> Self {
        Self {
            width: 240.,
            side: LeftToRight,
            dark: false,
            options: ProgressIndicatorOptions {
                id: ID,
                name: NAME,
                shape: ProgressShape::Flat,
                state: ProgressState::Determinate(0.25),
                elapsed_ms: 0.,
                reduced_motion: false,
            },
        }
    }
}

impl Application for Sample {
    type Message = ();

    fn update(&mut self, _: ()) {}

    fn view(&self) -> View<()> {
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        theme.page([theme.linear_progress_indicator(self.options, self.width, self.side)])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (0., size)
    }
}

fn render(sample: Sample) -> (Runtime<Sample>, ExtendedDisplayList) {
    let mut runtime = Runtime::new(sample).unwrap();
    let paint = runtime
        .render_extended(sample.width.clamp(320., 1000.), 80., &mut Metrics)
        .unwrap();
    (runtime, paint)
}

fn indicator(runtime: &Runtime<Sample>) -> NodeId {
    runtime.dom().by_id(ID).unwrap()
}

fn bounds(runtime: &Runtime<Sample>) -> Rect {
    runtime.computed().unwrap().nodes[indicator(runtime).index()].bounds
}

fn pills(paint: &ExtendedDisplayList, bounds: Rect, ink: Color) -> Vec<(f32, f32, f32, f32)> {
    let mut rectangles: Vec<_> = paint
        .ops()
        .iter()
        .filter_map(|op| match op {
            ExtendedPaintOp::Legacy(PaintOp::Quad {
                rect,
                color,
                radius,
                ..
            }) if *color == ink => {
                Some((rect.x - bounds.x, rect.y - bounds.y, rect.width, *radius))
            }
            _ => None,
        })
        .collect();
    rectangles.sort_by(|left, right| left.0.total_cmp(&right.0));
    rectangles
}

#[test]
fn flat_quarter_has_visible_gap_and_stop_at_the_logical_end() {
    for (side, active_x, track_x, stop_x) in
        [(LeftToRight, 0., 64., 236.), (RightToLeft, 180., 0., 0.)]
    {
        for (dark, theme) in [(false, Theme::light()), (true, Theme::dark())] {
            let (runtime, paint) = render(Sample {
                side,
                dark,
                ..Sample::default()
            });
            let bounds = bounds(&runtime);
            assert_eq!((bounds.width, bounds.height), (240., 4.));
            let mut active = [(active_x, 0., 60., 2.), (stop_x, 0., 4., 2.)];
            active.sort_by(|left: &(_, _, _, _), right| f32::total_cmp(&left.0, &right.0));
            assert_eq!(pills(&paint, bounds, theme.primary), active);
            assert_eq!(
                pills(&paint, bounds, theme.colors.secondary_container),
                [(track_x, 0., 176., 2.)]
            );
        }
    }
}

#[test]
fn flat_empty_and_complete_use_the_entire_track_without_rounding_outside_the_box() {
    for (value, want_track_width, want_active_width) in [(0., 240., 4.), (1., 0., 240.)] {
        let mut sample = Sample::default();
        sample.options.state = ProgressState::Determinate(value);
        let (runtime, paint) = render(sample);
        let bounds = bounds(&runtime);
        let theme = Theme::light();
        let active = pills(&paint, bounds, theme.primary);
        let track = pills(&paint, bounds, theme.colors.secondary_container);
        assert_eq!(active[0].2, want_active_width);
        assert_eq!(active[0].0 + want_active_width, bounds.width);
        assert_eq!(track.first().map_or(0., |piece| piece.2), want_track_width);
    }
}

#[test]
fn progress_is_read_only_and_paint_pieces_are_absent_from_accessibility() {
    for (state, expected_value) in [
        (ProgressState::Determinate(0.125), Some(0.125)),
        (ProgressState::Determinate(f64::NAN), Some(0.)),
        (ProgressState::Determinate(-0.5), Some(0.)),
        (ProgressState::Determinate(5.), Some(1.)),
        (ProgressState::Indeterminate, None),
    ] {
        let mut sample = Sample::default();
        sample.options.state = state;
        sample.options.shape = ProgressShape::Wavy;
        sample.options.elapsed_ms = 975.;
        let (runtime, _) = render(sample);
        let indicator = indicator(&runtime);
        let frame = runtime.accessibility_frame().unwrap();
        let node = frame.node(indicator).unwrap().node;
        assert_eq!((node.role(), node.name()), (Role::Progress, NAME));
        assert_eq!(node.range_value().map(|range| range.value), expected_value);
        if let Some(range) = node.range_value() {
            assert_eq!((range.min, range.max), (0., 1.));
        }
        assert!(runtime.dom().children(indicator).count() > 0);
        assert_eq!(frame.children(indicator).count(), 0);
        assert!(frame.focus_target(indicator).is_none());
        assert!(frame.activation_target(indicator).is_none());
    }
}

#[test]
fn flat_indeterminate_has_two_moving_segments_and_a_nonempty_reduced_motion_frame() {
    let mut sample = Sample::default();
    sample.options.state = ProgressState::Indeterminate;
    sample.options.elapsed_ms = 975.;
    let (runtime, first) = render(sample);
    let bounds = bounds(&runtime);
    let ink = Theme::light().primary;
    let active = pills(&first, bounds, ink);
    assert_eq!(active.len(), 2);
    assert!(active[0].0 + active[0].2 < active[1].0);
    sample.options.elapsed_ms += 1750.;
    assert_eq!(first, render(sample).1);
    sample.options.elapsed_ms = 1100.;
    assert_ne!(active, pills(&render(sample).1, bounds, ink));

    sample.options.reduced_motion = true;
    let reduced = render(sample).1;
    sample.options.elapsed_ms = 99999.;
    assert_eq!(reduced, render(sample).1);
    assert_eq!(pills(&reduced, bounds, ink), [(48., 0., 96., 2.)]);
    sample.side = RightToLeft;
    assert_eq!(pills(&render(sample).1, bounds, ink), [(96., 0., 96., 2.)]);
}

#[test]
fn waves_keep_their_height_and_move_only_when_motion_is_allowed() {
    for state in [
        ProgressState::Determinate(0.5),
        ProgressState::Indeterminate,
    ] {
        let mut sample = Sample::default();
        sample.options.shape = ProgressShape::Wavy;
        sample.options.state = state;
        sample.options.elapsed_ms = 975.;
        let (runtime, paint) = render(sample);
        let bounds = bounds(&runtime);
        assert_eq!((bounds.width, bounds.height), (240., 10.));
        let paths: Vec<_> = paint
            .ops()
            .iter()
            .filter_map(|op| match op {
                ExtendedPaintOp::FillPath(path) => Some(path),
                _ => None,
            })
            .collect();
        assert!(!paths.is_empty());
        assert!(paths.iter().any(|path| path.rect().height > 8.));
        assert!(paths.iter().all(|path| {
            let path = path.rect();
            path.y >= bounds.y - 0.01 && path.y + path.height <= bounds.y + 10.01
        }));

        sample.options.elapsed_ms = 1100.;
        assert_ne!(paint, render(sample).1);
        sample.options.reduced_motion = true;
        let reduced = render(sample).1;
        assert!(reduced
            .ops()
            .iter()
            .any(|op| matches!(op, ExtendedPaintOp::FillPath(_))));
        sample.options.elapsed_ms = f64::MAX;
        assert_eq!(reduced, render(sample).1);
    }
}

#[test]
fn tiny_and_out_of_range_widths_keep_valid_geometry() {
    for (width, expected) in [
        (2., 2.),
        (48., 48.),
        (-10., 0.),
        (f32::INFINITY, 0.),
        (9000., 8192.),
    ] {
        let mut sample = Sample {
            width,
            side: RightToLeft,
            ..Sample::default()
        };
        sample.options.shape = ProgressShape::Wavy;
        sample.options.state = ProgressState::Determinate(0.5);
        let (runtime, _) = render(sample);
        let bounds = bounds(&runtime);
        assert_eq!((bounds.width, bounds.height), (expected, 10.));
    }
}
