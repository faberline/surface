use std::{cell::Cell, rc::Rc};

use surface::{
    Align, Color, FillContour, Length, MeasuredGroupLayout, Node, Rect, Role, Style, TextMeasurer,
};
use surface_app::{AnimationFrame, AnimationUpdate, View};

use crate::{fab_motion::Spring, shape, LoadingKind, Theme};

mod contours;

pub(super) const DEFAULT_SIZE: f32 = 48.;
pub(super) const ACTIVE_SIZE: f32 = 38.;
const SHAPE_COUNT: usize = 7;
const PAIR_MS: f64 = 650.;
const ROTATION_MS: f64 = 4666.;
const QUARTER_TURN: f64 = 90.;
const REFERENCE_SETTLE_MS: f64 = 297.;
const CONTINUOUS_SETTLE_MS: f64 = 450.;

/// Content and size of a Material 3 Expressive loading indicator.
#[derive(Clone, Copy, Debug)]
pub struct LoadingIndicatorOptions<'a> {
    /// Stable semantic ID of the progress indicator.
    pub id: &'a str,
    /// Localized name of the operation in progress.
    pub name: &'a str,
    pub kind: LoadingKind,
    /// Side length in logical pixels. Finite sizes are clamped to 24–240.
    /// A non-finite size uses the 48 dp default.
    pub size: f32,
}

/// The visible pose of the active contour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoadingIndicatorPose {
    /// Start of the current shape pair. Zero is SoftBurst and six is Oval.
    pub shape_index: usize,
    /// Spring progress toward the next shape. May exceed one during the bounce.
    pub morph: f32,
    /// Clockwise angle in degrees modulo one turn.
    pub rotation_degrees: f32,
}

/// Retained clock for the indeterminate shape cycle.
pub struct LoadingIndicatorMotion {
    started_ms: Option<f64>,
    elapsed_ms: f64,
    pose: Rc<Cell<LoadingIndicatorPose>>,
}

impl Default for LoadingIndicatorMotion {
    fn default() -> Self {
        Self {
            started_ms: None,
            elapsed_ms: 0.,
            pose: Rc::new(Cell::new(pose_at(0.))),
        }
    }
}

impl LoadingIndicatorMotion {
    /// Advance from the application's frame callback while the indicator is in use.
    /// A new active run, reduced motion, or an absent clock starts at the first pose.
    /// Updating the paint leaves the mounted view and accessibility tree intact.
    pub fn advance(&mut self, frame: &AnimationFrame<'_>, active: bool) -> AnimationUpdate {
        let previous_elapsed = self.elapsed_ms;
        let now = frame
            .now_ms
            .filter(|time| time.is_finite() && active && !frame.reduced_motion);
        if let Some(now) = now {
            let start = *self.started_ms.get_or_insert(now);
            let elapsed_ms = now - start;
            if elapsed_ms.is_finite() {
                self.elapsed_ms = self.elapsed_ms.max(elapsed_ms);
            }
        } else {
            self.started_ms = None;
            self.elapsed_ms = 0.;
        }
        if self.elapsed_ms != previous_elapsed {
            self.pose.set(pose_at(self.elapsed_ms));
        }
        AnimationUpdate {
            rebuild: false,
            request_next: now.is_some(),
        }
    }

    /// Milliseconds since the start of the current animated run. Zero when idle.
    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed_ms
    }

    /// The current visible shape, including its rotation.
    pub fn pose(&self) -> LoadingIndicatorPose {
        self.pose.get()
    }
}

fn pose_at(elapsed_ms: f64) -> LoadingIndicatorPose {
    // Seven shapes and four quarter-turns return to their joint origin after
    // 28 intervals. The independent spin keeps running across that wrap.
    let time_in_cycle = elapsed_ms % (PAIR_MS * (4 * SHAPE_COUNT) as f64);
    let pair = (time_in_cycle / PAIR_MS) as usize;
    let morph = morph_progress(time_in_cycle % PAIR_MS);
    let spin = (elapsed_ms % ROTATION_MS) / ROTATION_MS * 360.;
    let angle = (1. + (pair % 4) as f64 + morph) * QUARTER_TURN + spin;
    LoadingIndicatorPose {
        shape_index: pair % SHAPE_COUNT,
        morph: morph as f32,
        rotation_degrees: (angle as f32).rem_euclid(360.),
    }
}

fn morph_progress(elapsed_ms: f64) -> f64 {
    if elapsed_ms >= CONTINUOUS_SETTLE_MS {
        return 1.;
    }
    // AndroidX LoadingIndicator.kt uses stiffness 200 and damping ratio 0.6.
    // Its 0.1 visibility threshold ends the spring at 297 ms while it is
    // visibly overshooting. Fade out the remaining displacement continuously
    // so neither the outline nor the spring-driven angle snaps at that point.
    let mut spring = Spring::new(0.);
    spring.advance(1., elapsed_ms / 1000., 200., 0.6);
    let tail =
        ((elapsed_ms - REFERENCE_SETTLE_MS) / (CONTINUOUS_SETTLE_MS - REFERENCE_SETTLE_MS)).max(0.);
    let blend = tail * tail * (3. - 2. * tail);
    1. + (spring.position - 1.) * (1. - blend)
}

impl Theme {
    /// Draw a continuously morphing indeterminate indicator.
    /// Retain the motion in the application and advance it on each frame.
    pub fn loading_indicator_with_motion<M>(
        &self,
        options: LoadingIndicatorOptions<'_>,
        motion: &LoadingIndicatorMotion,
    ) -> View<M> {
        let size = if options.size.is_finite() {
            options.size.clamp(24., 240.)
        } else {
            DEFAULT_SIZE
        };
        let (outer, ink) = loading_container(self, options.kind, size);
        let active_size = size * (ACTIVE_SIZE / DEFAULT_SIZE);
        let pose = Rc::clone(&motion.pose);
        let shape = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-shape", options.id))
                .styled(square(active_size)),
        )
        .paint_fill_contour(move || paint_outline(active_size, ink, pose.get()))
        .without_interaction(None);
        View::new(
            Node::new(Role::Progress, options.name)
                .id(options.id)
                .styled(outer.clip(true))
                .measured_group(LoadingLayout { size, active_size }),
        )
        .children([shape])
    }
}

fn paint_outline(active_size: f32, ink: Color, pose: LoadingIndicatorPose) -> FillContour {
    let center = active_size * 0.5;
    let (sin, cos) = pose.rotation_degrees.to_radians().sin_cos();
    let points = contours::contour(pose.shape_index, pose.morph).map(|[x, y]| {
        [
            center + (x * cos - y * sin) * active_size,
            center + (x * sin + y * cos) * active_size,
        ]
    });
    FillContour::new(points, ink).expect("valid loading indicator contour")
}

#[derive(Debug)]
struct LoadingLayout {
    size: f32,
    active_size: f32,
}

impl MeasuredGroupLayout for LoadingLayout {
    fn measure(
        &self,
        _children: &[&Node],
        _available_width: Option<f32>,
        _fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        (self.size, self.size)
    }

    fn place(
        &self,
        _children: &[&Node],
        (width, height): (f32, f32),
        _fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        // A 19 dp outline in a 24 dp slot has a 2.5 dp inset. Preserve that
        // inset so the outline rotates around the center of the container.
        let active = self.active_size;
        vec![Rect::new(
            (width - active) / 2.,
            (height - active) / 2.,
            active,
            active,
        )]
    }
}

pub(super) fn loading_container(theme: &Theme, kind: LoadingKind, size: f32) -> (Style, Color) {
    let outer = Style::row()
        .align(Align::Center)
        .justify(Align::Center)
        .width(Length::Px(size))
        .height(Length::Px(size));
    match kind {
        LoadingKind::Default => (outer, theme.colors.primary),
        LoadingKind::Contained => (
            outer
                .corners(shape::FULL)
                .background(theme.colors.primary_container),
            theme.colors.on_primary_container,
        ),
    }
}

fn square(size: f32) -> Style {
    Style::default()
        .width(Length::Px(size))
        .height(Length::Px(size))
}
