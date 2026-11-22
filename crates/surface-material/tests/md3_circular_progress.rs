use surface::{Color, ExtendedDisplayList, ExtendedPaintOp, PathFill, Rect, Role, TextMeasurer};
use surface_app::{Application, Runtime, View};
use surface_material::{ProgressIndicatorOptions, ProgressShape, ProgressState, Theme};

const ID: &str = "circular-progress";
const NAME: &str = "正在匯入";

#[derive(Clone, Copy)]
struct ProgressApp {
    dark: bool,
    shape: ProgressShape,
    state: ProgressState,
    elapsed_ms: f64,
    reduced_motion: bool,
}

impl Default for ProgressApp {
    fn default() -> Self {
        Self {
            dark: false,
            shape: ProgressShape::Flat,
            state: ProgressState::Determinate(0.25),
            elapsed_ms: 0.,
            reduced_motion: false,
        }
    }
}

impl ProgressApp {
    fn theme(self) -> Theme {
        if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        }
    }
}

impl Application for ProgressApp {
    type Message = Self;

    fn view(&self) -> View<Self> {
        let theme = self.theme();
        theme.page(
            [theme.circular_progress_indicator(ProgressIndicatorOptions {
                id: ID,
                name: NAME,
                shape: self.shape,
                state: self.state,
                elapsed_ms: self.elapsed_ms,
                reduced_motion: self.reduced_motion,
            })],
        )
    }

    fn update(&mut self, state: Self) {
        *self = state;
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

struct Frame {
    runtime: Runtime<ProgressApp>,
    paint: ExtendedDisplayList,
}

impl Frame {
    fn new(app: ProgressApp) -> Self {
        let mut runtime = Runtime::new(app).unwrap();
        let paint = runtime.render_extended(100., 120., &mut Metrics).unwrap();
        Self { runtime, paint }
    }

    fn bounds(&self) -> Rect {
        let id = self.runtime.dom().by_id(ID).unwrap();
        self.runtime.computed().unwrap().nodes[id.index()].bounds
    }

    fn fills(&self) -> impl Iterator<Item = &PathFill> {
        self.paint.ops().iter().filter_map(|op| match op {
            ExtendedPaintOp::FillPath(fill) => Some(fill),
            _ => None,
        })
    }

    fn color_at(&self, point: [f32; 2]) -> Option<Color> {
        let bounds = self.bounds();
        let point = [point[0] + bounds.x, point[1] + bounds.y];
        self.fills()
            .filter(|path| contains(path, point))
            .last()
            .map(PathFill::color)
    }
}

fn contains(path: &PathFill, [x, y]: [f32; 2]) -> bool {
    if !path.clip().contains(x, y) {
        return false;
    }
    let points = path.points();
    let crossings = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .filter(|&([ax, ay], [bx, by])| {
            (*ay > y) != (*by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax
        })
        .count();
    crossings % 2 == 1
}

#[test]
fn flat_grows_clockwise_and_leaves_a_gap_and_hole_in_both_themes() {
    for dark in [false, true] {
        let app = ProgressApp {
            dark,
            ..Default::default()
        };
        let frame = Frame::new(app);
        let bounds = frame.bounds();
        assert_eq!((bounds.width, bounds.height), (40., 40.));
        assert_eq!(frame.fills().count(), 2);
        assert_eq!(
            frame.color_at([32.7, 7.3]),
            Some(app.theme().colors.primary)
        );
        assert_eq!(
            frame.color_at([2., 20.]),
            Some(app.theme().colors.secondary_container)
        );
        assert_eq!(frame.color_at([37.6, 23.6]), None);
        assert_eq!(frame.color_at([20., 20.]), None);
        // The end cap extends along the tangent but rounds away at the corner.
        assert_eq!(
            frame.color_at([38., 21.4]),
            Some(app.theme().colors.primary)
        );
        assert_eq!(frame.color_at([39.8, 21.4]), None);
    }
}

#[test]
fn endpoints_have_a_closed_band_and_non_finite_values_paint_zero() {
    for shape in [ProgressShape::Flat, ProgressShape::Wavy] {
        for (value, active) in [
            (0., false),
            (-12., false),
            (f64::NAN, false),
            (f64::INFINITY, false),
            (1., true),
            (12., true),
        ] {
            let app = ProgressApp {
                shape,
                state: ProgressState::Determinate(value),
                ..Default::default()
            };
            let frame = Frame::new(app);
            let colors = app.theme().colors;
            let expected = if active {
                colors.primary
            } else {
                colors.secondary_container
            };
            let center = frame.bounds().width / 2.;
            let inset = if shape == ProgressShape::Wavy {
                3.6
            } else {
                2.
            };
            let spots = [
                [center, inset],
                [center * 2. - inset, center],
                [center, center * 2. - inset],
                [inset, center],
            ];
            assert_eq!(frame.fills().count(), 1, "{shape:?} {value}");
            for spot in spots {
                assert_eq!(
                    frame.color_at(spot),
                    Some(expected),
                    "{shape:?} {value} at {spot:?}"
                );
            }
            assert_eq!(frame.color_at([center, center]), None);
        }
    }
}

#[test]
fn accessibility_exposes_one_read_only_progress_node_and_clears_the_range() {
    let state = ProgressApp::default();
    let mut frame = Frame::new(state);
    let id = frame.runtime.dom().by_id(ID).unwrap();
    {
        let tree = frame.runtime.accessibility_frame().unwrap();
        let node = tree.node(id).unwrap().node;
        assert_eq!(node.role(), Role::Progress);
        assert_eq!(node.name(), NAME);
        let value = node.range_value().unwrap();
        assert_eq!((value.min, value.value, value.max), (0., 0.25, 1.));
        assert_eq!(tree.children(id).count(), 0);
        assert!(tree.focus_target(id).is_none());
        assert!(!node.is_focusable());
    }

    frame
        .runtime
        .apply(ProgressApp {
            state: ProgressState::Indeterminate,
            ..state
        })
        .unwrap();
    frame
        .runtime
        .render_extended(100., 120., &mut Metrics)
        .unwrap();
    let id = frame.runtime.dom().by_id(ID).unwrap();
    let tree = frame.runtime.accessibility_frame().unwrap();
    let node = tree.node(id).unwrap().node;
    assert_eq!(node.role(), Role::Progress);
    assert_eq!(node.name(), NAME);
    assert!(node.range_value().is_none());
    assert_eq!(tree.children(id).count(), 0);
    assert!(tree.focus_target(id).is_none());
}

#[test]
fn indeterminate_variants_move_and_keep_visible_reduced_motion_frames() {
    for shape in [ProgressShape::Flat, ProgressShape::Wavy] {
        let app = ProgressApp {
            shape,
            state: ProgressState::Indeterminate,
            ..Default::default()
        };
        let start = Frame::new(app);
        let later = Frame::new(ProgressApp {
            elapsed_ms: 1780.,
            ..app
        });
        let cycle = Frame::new(ProgressApp {
            elapsed_ms: 6000.,
            ..app
        });
        let reduced = Frame::new(ProgressApp {
            elapsed_ms: 1780.,
            reduced_motion: true,
            ..app
        });
        let expected_layers = if shape == ProgressShape::Wavy { 2 } else { 1 };
        assert_eq!(start.fills().count(), expected_layers);
        assert_ne!(start.paint, later.paint);
        assert_eq!(start.paint, cycle.paint);
        assert_eq!(start.paint, reduced.paint);
        assert!(reduced
            .fills()
            .any(|fill| fill.color() == app.theme().colors.primary));
    }
}

#[test]
fn expressive_ring_has_bounded_wave_geometry_and_retains_progress_endpoints() {
    let app = ProgressApp {
        shape: ProgressShape::Wavy,
        ..Default::default()
    };
    let start = Frame::new(app);
    let changed = Frame::new(ProgressApp {
        elapsed_ms: 500.,
        ..app
    });
    let reduced = Frame::new(ProgressApp {
        elapsed_ms: 500.,
        reduced_motion: true,
        ..app
    });
    let bounds = start.bounds();
    assert_eq!((bounds.width, bounds.height), (48., 48.));
    assert_eq!(start.color_at([24., 24.]), None);
    assert_eq!(
        start.color_at([38.4, 9.6]),
        Some(app.theme().colors.primary)
    );
    assert_eq!(
        start.color_at([3.6, 24.]),
        Some(app.theme().colors.secondary_container)
    );
    assert_ne!(start.paint, changed.paint);
    assert_eq!(start.paint, reduced.paint);
    for frame in [start, changed, reduced] {
        for fill in frame.fills() {
            for [x, y] in fill.points() {
                assert!((bounds.x - 0.01..=bounds.x + 48.01).contains(x));
                assert!((bounds.y - 0.01..=bounds.y + 48.01).contains(y));
            }
        }
    }
}
