use std::f32::consts::{FRAC_PI_2, PI, TAU};

use surface::{Color, FillContour, Insets, Length, Node, Role, Style};
use surface_app::View;

use super::{ProgressIndicatorOptions, ProgressShape};
use crate::{motion::easing, Theme};

const FLAT_DIAMETER: f32 = 40.;
const WAVE_DIAMETER: f32 = 48.;
const STROKE: f32 = 4.;
const HALF_STROKE: f32 = STROKE / 2.;
const TRACK_GAP: f32 = 4.;
const WAVE_AMPLITUDE: f32 = 1.6;
const WAVE_LENGTH: f32 = 15.;
const CYCLE_MS: f64 = 6000.;
const WAVE_PERIOD_MS: f64 = 1000.;
const ROTATION_INTERVAL_MS: f64 = 1500.;
const ROTATION_STEP_MS: f64 = 300.;
const MIN_SWEEP_FRACTION: f32 = 0.1;
const MAX_SWEEP_FRACTION: f32 = 0.87;
const RING_SEGMENTS: usize = 108;
const CAP_SEGMENTS: usize = 8;

impl Theme {
    /// Build a circular progress indicator with application-owned progress.
    ///
    /// Flat indicators occupy 40 dp. Wavy indicators occupy 48 dp and
    /// leave room for the wave. Both use a 4 dp round stroke. Determinate
    /// progress starts at twelve o'clock and advances clockwise. The flat
    /// indeterminate variant uses a transparent track.
    ///
    /// Keep `elapsed_ms` on a monotonic clock to animate indeterminate
    /// sweeps or the wavy active stroke. With reduced motion, the indicator
    /// keeps a visible static frame without cycling.
    pub fn circular_progress_indicator<M>(&self, options: ProgressIndicatorOptions<'_>) -> View<M> {
        let fraction = options.fraction();
        let elapsed_ms = options.elapsed();
        let wavy = matches!(options.shape, ProgressShape::Wavy);
        let diameter = if wavy { WAVE_DIAMETER } else { FLAT_DIAMETER };
        let wave_room = if wavy { WAVE_AMPLITUDE } else { 0. };
        let ring = Ring {
            diameter,
            radius: diameter / 2. - HALF_STROKE - wave_room,
        };
        let (start, sweep) = fraction.map_or_else(
            || indeterminate_arc(elapsed_ms),
            |value| (-FRAC_PI_2, value * TAU),
        );
        let wave = (wavy && fraction.is_none_or(|value| value > 0.1 && value < 0.95))
            .then(|| Wave::at(ring.radius, elapsed_ms));
        let gap = if wavy {
            (TRACK_GAP + STROKE) / ring.radius
        } else {
            (TRACK_GAP + STROKE) / (diameter / 2.)
        }
        .min(sweep);
        let track = (wavy || fraction.is_some())
            .then(|| {
                ring.arc(
                    start + sweep + gap,
                    TAU - sweep - 2. * gap,
                    None,
                    self.colors.secondary_container,
                )
            })
            .flatten();
        let active = ring.arc(start, sweep, wave, self.colors.primary);
        options
            .view(square(diameter).clip(true))
            .children(track.into_iter().chain(active))
    }
}

fn indeterminate_arc(elapsed_ms: f64) -> (f32, f32) {
    let time = elapsed_ms % CYCLE_MS;
    // The Material cycle makes three full turns in six seconds. It also
    // adds an eased quarter turn at the start of each 1.5-second interval.
    let global = (3. * time / CYCLE_MS) as f32 * TAU;
    let quarter_turns = (time / ROTATION_INTERVAL_MS).floor() as f32;
    let extra = easing::EMPHASIZED_DECELERATE
        .sample(((time % ROTATION_INTERVAL_MS) / ROTATION_STEP_MS) as f32);
    let start = (global + (quarter_turns + extra) * FRAC_PI_2).rem_euclid(TAU);
    let phase = (2. * time / CYCLE_MS) as f32;
    let growth = if phase <= 1. {
        phase
    } else {
        1. - easing::STANDARD.sample(phase - 1.)
    };
    let sweep = MIN_SWEEP_FRACTION + (MAX_SWEEP_FRACTION - MIN_SWEEP_FRACTION) * growth;
    (start, sweep * TAU)
}

#[derive(Clone, Copy)]
struct Wave {
    cycles: f32,
    phase: f32,
}

impl Wave {
    fn at(radius: f32, elapsed_ms: f64) -> Self {
        Self {
            // Use an integer number of lobes so the wave stays continuous
            // through the seam, close to the preferred wavelength.
            cycles: (TAU * radius / WAVE_LENGTH).round(),
            phase: ((elapsed_ms % WAVE_PERIOD_MS) / WAVE_PERIOD_MS) as f32 * TAU,
        }
    }
}

struct Ring {
    radius: f32,
    diameter: f32,
}

impl Ring {
    fn arc<M>(&self, start: f32, sweep: f32, wave: Option<Wave>, color: Color) -> Option<View<M>> {
        if sweep <= 0. {
            return None;
        }
        // When the end caps meet, use a single closed band. Overlapping
        // round caps in one even-odd contour would cancel each other.
        let closed = (TAU - sweep) * self.radius < STROKE;
        let sweep = if closed { TAU } else { sweep };
        let segments = ((sweep / TAU * RING_SEGMENTS as f32).ceil() as usize).max(1);
        let first = self.section(start, wave);
        let sections = std::iter::once(first)
            .chain((1..=segments).map(|index| {
                if closed && index == segments {
                    first
                } else {
                    self.section(start + sweep * index as f32 / segments as f32, wave)
                }
            }))
            .collect::<Vec<_>>();

        let mut outline = Vec::with_capacity(2 * sections.len() + 2 * CAP_SEGMENTS);
        outline.extend(sections.iter().map(|section| section.point_at(0.)));
        if !closed {
            outline.extend(sections.last()?.cap(0.));
        }
        outline.extend(sections.iter().rev().map(|section| section.point_at(PI)));
        if !closed {
            outline.extend(first.cap(PI));
        }
        let contour = FillContour::new(outline, color)?;
        Some(
            View::new(
                Node::new(Role::Group, "")
                    .styled(square(self.diameter).absolute(Insets::all(0.)))
                    .fill_contour(contour),
            )
            .without_interaction(None),
        )
    }

    fn section(&self, angle: f32, wave: Option<Wave>) -> Section {
        let (y, x) = angle.sin_cos();
        let (displacement, slope) = wave.map_or((0., 0.), |wave| {
            let (s, c) = (angle * wave.cycles - wave.phase).sin_cos();
            (WAVE_AMPLITUDE * c, -WAVE_AMPLITUDE * wave.cycles * s)
        });
        let radius = self.radius + displacement;
        let dx = slope * x - radius * y;
        let dy = slope * y + radius * x;
        let speed = dx.hypot(dy);
        let center = self.diameter / 2.;
        Section {
            center: [center + radius * x, center + radius * y],
            tangent: [dx / speed, dy / speed],
        }
    }
}

#[derive(Clone, Copy)]
struct Section {
    center: [f32; 2],
    tangent: [f32; 2],
}

impl Section {
    fn point_at(self, angle: f32) -> [f32; 2] {
        let (along, perpendicular) = angle.sin_cos();
        let [dx, dy] = self.tangent;
        [
            self.center[0] + HALF_STROKE * (dy * perpendicular + dx * along),
            self.center[1] + HALF_STROKE * (-dx * perpendicular + dy * along),
        ]
    }

    fn cap(self, angle: f32) -> impl Iterator<Item = [f32; 2]> {
        (1..CAP_SEGMENTS)
            .map(move |index| self.point_at(angle + PI * index as f32 / CAP_SEGMENTS as f32))
    }
}

fn square(diameter: f32) -> Style {
    Style::column()
        .width(Length::Px(diameter))
        .height(Length::Px(diameter))
}
