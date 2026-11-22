use std::f64::consts::{FRAC_PI_2, TAU};

use surface::{Color, FillContour, Rect, Role};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, FramePress, FramePressPhase};

use crate::motion::easing;

const RADIUS_MS: f64 = 225.;
const FADE_IN_MS: f64 = 75.;
const FADE_OUT_MS: f64 = 150.;
const HIDDEN_SCALE: f64 = 0.2;

/// Retained visibility and accepted pointer feedback for one expressive FAB.
pub struct ExpressiveFabMotion {
    visible: bool,
    last_time: Option<f64>,
    scale: Spring,
    opacity: Spring,
    seen_press: Option<u64>,
    wave: Option<Wave>,
    interaction: Interaction,
}

/// Current radial press feedback in the FAB's local coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExpressiveFabRipple {
    /// The accepted press point, normalized against the face.
    pub center: (f32, f32),
    /// Interpolation from the starting radius to the covering radius.
    pub progress: f32,
    /// The multiplier of the pressed ink opacity.
    pub opacity: f32,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Interaction {
    pub hovered: bool,
    pub focused: bool,
    pub pressed: bool,
}

struct Wave {
    started_at: f64,
    released_at: Option<f64>,
    pose: ExpressiveFabRipple,
}

#[derive(Clone, Copy)]
struct Spring {
    position: f64,
    velocity: f64,
}

impl Default for ExpressiveFabMotion {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpressiveFabMotion {
    /// Initialize a visible action with no pending feedback.
    pub const fn new() -> Self {
        Self::initial(true)
    }

    /// Initialize directly at the caller's initial visibility endpoint.
    pub const fn with_visibility(self, visible: bool) -> Self {
        Self::initial(visible)
    }

    const fn initial(visible: bool) -> Self {
        Self {
            visible,
            last_time: None,
            scale: Spring::new(scale_endpoint(visible)),
            opacity: Spring::new(opacity_endpoint(visible)),
            seen_press: None,
            wave: None,
            interaction: Interaction {
                hovered: false,
                focused: false,
                pressed: false,
            },
        }
    }

    /// Current press feedback, when a radial effect is active.
    pub fn ripple(&self) -> Option<ExpressiveFabRipple> {
        self.wave.as_ref().map(|wave| wave.pose)
    }

    /// The presented scale, bounded between 0.2 and 1.
    pub fn visibility_scale(&self) -> f32 {
        self.scale.position.clamp(HIDDEN_SCALE, 1.) as f32
    }

    /// The presented opacity for the face, glyph and elevation.
    pub fn visibility_opacity(&self) -> f32 {
        self.opacity.position.clamp(0., 1.) as f32
    }

    pub(super) fn interaction(&self) -> Interaction {
        self.interaction
    }

    /// Advance from `Application::animation_frame_with_press_detail`.
    /// Pass the FAB's stable ID, accepted hovered target, and caller visibility.
    pub fn advance(
        &mut self,
        id: &str,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
        visible: bool,
    ) -> AnimationUpdate {
        let previous = self.visual_state();
        let now = frame
            .now_ms
            .filter(|time| time.is_finite() && !frame.reduced_motion)
            .map(|time| time.max(self.last_time.unwrap_or(time)));
        self.advance_visibility(now, visible);

        let source = frame.dom.by_id(id).filter(|&node| {
            visible
                && frame
                    .dom
                    .get(node)
                    .is_ok_and(|button| button.role() == Role::Button && !button.is_disabled())
        });
        let press = press.filter(|event| event.id == id);
        self.interaction = source
            .map(|_| Interaction {
                hovered: hovered == Some(id),
                focused: input.focused == Some(id),
                pressed: press.is_some_and(|event| event.phase == FramePressPhase::Held),
            })
            .unwrap_or_default();

        if let Some(now) = now.filter(|_| source.is_some()) {
            self.advance_wave(id, frame, press, now, previous.0);
        } else {
            self.wave = None;
            if let Some(event) = press {
                self.seen_press = Some(event.sequence);
            }
        }

        let changed = previous != self.visual_state();
        AnimationUpdate {
            rebuild: changed,
            request_next: changed
                || self
                    .wave
                    .as_ref()
                    .is_some_and(|wave| wave.released_at.is_some() || wave.pose.progress < 1.)
                || !self.scale.resting_at(scale_endpoint(visible))
                || !self.opacity.resting_at(opacity_endpoint(visible)),
        }
    }

    fn visual_state(&self) -> (f32, f32, Option<ExpressiveFabRipple>, Interaction) {
        (
            self.visibility_scale(),
            self.visibility_opacity(),
            self.ripple(),
            self.interaction,
        )
    }

    fn advance_visibility(&mut self, now: Option<f64>, visible: bool) {
        if let Some(now) = now {
            let seconds = ((now - self.last_time.unwrap_or(now)) * 0.001).clamp(0., 2.);
            // Compose Expressive Fast Spatial and Fast Effects. Evaluate the
            // old target before retargeting so a reversal keeps its velocity.
            self.scale
                .advance(scale_endpoint(self.visible), seconds, 800., 0.6);
            self.opacity
                .advance(opacity_endpoint(self.visible), seconds, 3800., 1.);
        } else {
            self.scale = Spring::new(scale_endpoint(visible));
            self.opacity = Spring::new(opacity_endpoint(visible));
        }
        self.visible = visible;
        self.last_time = now;
    }

    fn advance_wave(
        &mut self,
        id: &str,
        frame: &AnimationFrame<'_>,
        press: Option<FramePress<'_>>,
        now: f64,
        previous_scale: f32,
    ) {
        if let Some(event) = press {
            if event.phase == FramePressPhase::Cancelled {
                self.seen_press = Some(event.sequence);
                self.wave = None;
                return;
            }
            if self.seen_press != Some(event.sequence) {
                self.seen_press = Some(event.sequence);
                // Start drawing at the first observing frame, even if the
                // host clock before pointer down was from an idle frame.
                self.wave = Some(Wave {
                    started_at: now,
                    released_at: None,
                    pose: ExpressiveFabRipple {
                        center: local_center(id, frame, event, previous_scale),
                        progress: 0.,
                        opacity: 0.,
                    },
                });
            }
            if event.phase == FramePressPhase::Released {
                if let Some(wave) = &mut self.wave {
                    wave.released_at.get_or_insert(now);
                }
            }
        } else if self
            .wave
            .as_ref()
            .is_some_and(|wave| wave.released_at.is_none())
        {
            self.wave = None;
        }

        if let Some(wave) = &mut self.wave {
            let elapsed = now - wave.started_at;
            wave.pose.progress = easing::LEGACY.sample((elapsed / RADIUS_MS) as f32);
            wave.pose.opacity = match wave.released_at {
                Some(released) => {
                    let fade_start = released.max(wave.started_at + RADIUS_MS);
                    1. - (((now - fade_start) / FADE_OUT_MS) as f32).clamp(0., 1.)
                }
                None => ((elapsed / FADE_IN_MS) as f32).clamp(0., 1.),
            };
            if wave.released_at.is_some() && wave.pose.opacity == 0. {
                self.wave = None;
            }
        }
    }
}

impl Spring {
    const fn new(position: f64) -> Self {
        Self {
            position,
            velocity: 0.,
        }
    }

    fn resting_at(self, target: f64) -> bool {
        self.position == target && self.velocity == 0.
    }

    fn advance(&mut self, target: f64, seconds: f64, stiffness: f64, damping: f64) {
        if self.resting_at(target) || seconds == 0. {
            return;
        }
        let omega = stiffness.sqrt();
        let decay = damping * omega;
        let delta = self.position - target;
        let envelope = (-decay * seconds).exp();
        let (position, derivative) = if damping < 1. {
            let frequency = omega * (1. - damping * damping).sqrt();
            let (sin, cos) = (frequency * seconds).sin_cos();
            let coefficient = (self.velocity + decay * delta) / frequency;
            (
                delta * cos + coefficient * sin,
                frequency * (coefficient * cos - delta * sin),
            )
        } else {
            let coefficient = self.velocity + decay * delta;
            (delta + coefficient * seconds, coefficient)
        };
        let offset = envelope * position;
        let velocity = envelope * (derivative - decay * position);
        *self = if offset.abs() <= 0.001 && velocity.abs() <= 0.005 {
            Self::new(target)
        } else {
            Self {
                position: target + offset,
                velocity,
            }
        };
    }
}

const fn scale_endpoint(visible: bool) -> f64 {
    if visible {
        1.
    } else {
        HIDDEN_SCALE
    }
}

const fn opacity_endpoint(visible: bool) -> f64 {
    if visible {
        1.
    } else {
        0.
    }
}

fn local_center(
    id: &str,
    frame: &AnimationFrame<'_>,
    press: FramePress<'_>,
    previous_scale: f32,
) -> (f32, f32) {
    let bounds = |id: &str| -> Option<Rect> {
        let node = frame.dom.by_id(id)?;
        Some(frame.layout?.nodes.get(node.index())?.bounds)
    };
    let face = bounds(id)
        .filter(|&control| control == press.bounds)
        .and_then(|_| bounds(&format!("{id}-face")))
        .unwrap_or_else(|| {
            // A fast accepted release can rebuild the view before the first
            // frame. Match the earlier centered, pixel-rounded face.
            let hit = press.bounds;
            let side = (hit.width.min(hit.height) * previous_scale).round();
            Rect::new(
                hit.x + (hit.width - side) / 2.,
                hit.y + (hit.height - side) / 2.,
                side,
                side,
            )
        });
    let normalize = |point: f32, start: f32, extent: f32| {
        if extent > 0. && extent.is_finite() && point.is_finite() && start.is_finite() {
            ((point - start) / extent).clamp(0., 1.)
        } else {
            0.5
        }
    };
    (
        normalize(press.position.0, face.x, face.width),
        normalize(press.position.1, face.y, face.height),
    )
}

impl ExpressiveFabRipple {
    pub(super) fn contour(
        self,
        side: f32,
        corner_radius: f32,
        color: Color,
    ) -> Option<FillContour> {
        if color.3 == 0
            || side <= 0.
            || ![
                side,
                corner_radius,
                self.center.0,
                self.center.1,
                self.progress,
            ]
            .into_iter()
            .all(f32::is_finite)
        {
            return None;
        }
        let side = f64::from(side);
        let boundary = rounded_boundary(side, f64::from(corner_radius).clamp(0., side / 2.));
        let points = if self.progress >= 1. {
            boundary
        } else {
            let x = f64::from(self.center.0.clamp(0., 1.)) * side;
            let y = f64::from(self.center.1.clamp(0., 1.)) * side;
            let cover = x.max(side - x).hypot(y.max(side - y));
            let start = 0.3 * side;
            let radius = start + (cover - start) * f64::from(self.progress.max(0.));
            let circle = (0..64)
                .map(|index| {
                    let (sin, cos) = (TAU * f64::from(index) / 64.).sin_cos();
                    [x + radius * cos, y + radius * sin]
                })
                .collect();
            clip_convex(circle, &boundary)
        };
        FillContour::new(
            points
                .into_iter()
                .map(|point| point.map(|axis| axis.clamp(0., side) as f32)),
            color,
        )
    }
}

type Point = [f64; 2];

fn rounded_boundary(side: f64, radius: f64) -> Vec<Point> {
    if radius == 0. {
        return vec![[0., 0.], [side, 0.], [side, side], [0., side]];
    }
    let far = side - radius;
    [[far, radius], [far, far], [radius, far], [radius, radius]]
        .into_iter()
        .enumerate()
        .flat_map(|(corner, [x, y])| {
            (0..=12).map(move |step| {
                let angle = FRAC_PI_2 * (corner as f64 - 1. + f64::from(step) / 12.);
                let (sin, cos) = angle.sin_cos();
                [x + radius * cos, y + radius * sin]
            })
        })
        .collect()
}

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
