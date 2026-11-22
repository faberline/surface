use std::{
    f32::consts::{FRAC_PI_2, PI},
    ops::Range,
};

use surface::{Color, FillContour, Insets, Length, Node, Role, Style};
use surface_app::View;

use crate::{motion::easing, shape, shape::LayoutDirection, Theme};

use super::{ProgressIndicatorOptions, ProgressShape};

const THICKNESS: f32 = 4.;
const TRACK_GAP: f32 = 4.;
const WAVE_HEIGHT: f32 = 10.;
const WAVE_AMPLITUDE: f32 = 3.;
const DETERMINATE_WAVELENGTH: f32 = 40.;
const INDETERMINATE_WAVELENGTH: f32 = 20.;
const MAX_WIDTH: f32 = 8192.;
const CYCLE_MS: f64 = 1750.;
const WAVE_PERIOD_MS: f64 = 1000.;

impl Theme {
    /// A Material 3 linear progress indicator. Pass the available logical
    /// width, with 240 dp as the default Material width. Widths clamp to
    /// 0–8192 dp. Non-finite widths give a zero-width indicator.
    ///
    /// Flat indicators are 4 dp high. Wavy indicators are 10 dp high.
    /// The fill and the determinate end marker follow the logical direction.
    pub fn linear_progress_indicator<M>(
        &self,
        options: ProgressIndicatorOptions<'_>,
        width: f32,
        direction: LayoutDirection,
    ) -> View<M> {
        let layout = LinearLayout {
            width: if width.is_finite() {
                width.clamp(0., MAX_WIDTH)
            } else {
                0.
            },
            height: match options.shape {
                ProgressShape::Flat => THICKNESS,
                ProgressShape::Wavy => WAVE_HEIGHT,
            },
            direction,
        };
        let fraction = options.fraction();
        let active = active_spans(layout.width, fraction, options);
        let wave = match options.shape {
            ProgressShape::Wavy if fraction.is_none_or(|value| value > 0.1 && value < 0.95) => {
                Some(Wave {
                    wavelength: if fraction.is_some() {
                        DETERMINATE_WAVELENGTH
                    } else {
                        INDETERMINATE_WAVELENGTH
                    },
                    phase: (options.elapsed() % WAVE_PERIOD_MS / WAVE_PERIOD_MS) as f32,
                })
            }
            _ => None,
        };

        let mut track_start = 0.;
        let mut pieces = Vec::new();
        for span in active.into_iter().filter(|span| !span.is_empty()) {
            push_pill(
                &mut pieces,
                layout,
                track_start..span.start - TRACK_GAP,
                THICKNESS,
                self.colors.secondary_container,
            );
            let gap_after = if fraction.is_some() {
                TRACK_GAP.min(span.end)
            } else {
                TRACK_GAP
            };
            track_start = (span.end + gap_after).min(layout.width);
            if let Some(wave) = wave.filter(|_| span.end - span.start > THICKNESS) {
                wave.push_stroke(&mut pieces, layout, span, self.primary);
            } else {
                push_pill(&mut pieces, layout, span, THICKNESS, self.primary);
            }
        }
        push_pill(
            &mut pieces,
            layout,
            track_start..layout.width,
            THICKNESS,
            self.colors.secondary_container,
        );
        if fraction.is_some() {
            let stop = (layout.width - track_start).min(THICKNESS);
            push_pill(
                &mut pieces,
                layout,
                layout.width - stop..layout.width,
                stop,
                self.primary,
            );
        }

        options
            .view(
                Style::default()
                    .width(Length::Px(layout.width))
                    .height(Length::Px(layout.height))
                    .clip(true),
            )
            .children(pieces)
    }
}

fn active_spans(
    width: f32,
    fraction: Option<f32>,
    options: ProgressIndicatorOptions<'_>,
) -> [Range<f32>; 2] {
    if let Some(fraction) = fraction {
        return [0.0..fraction * width, 0.0..0.];
    }
    if options.reduced_motion {
        return [width * 0.2..width * 0.6, 0.0..0.];
    }
    // AndroidX linear keyframes. Each head and tail has its own delay.
    let elapsed = options.elapsed() % CYCLE_MS;
    let position = |delay: f64, duration: f64| {
        width * easing::EMPHASIZED_ACCELERATE.sample(((elapsed - delay) / duration) as f32)
    };
    [
        position(900., 850.)..position(650., 850.),
        position(250., 1000.)..position(0., 1000.),
    ]
}

#[derive(Clone, Copy)]
struct LinearLayout {
    width: f32,
    height: f32,
    direction: LayoutDirection,
}

impl LinearLayout {
    fn physical_x(self, logical_x: f32) -> f32 {
        match self.direction {
            LayoutDirection::LeftToRight => logical_x,
            LayoutDirection::RightToLeft => self.width - logical_x,
        }
    }

    fn positioned(self, span: Range<f32>, height: f32) -> Style {
        Style::default()
            .width(Length::Px(span.end - span.start))
            .height(Length::Px(height))
            .absolute(Insets {
                left: Length::Px(self.physical_x(span.start).min(self.physical_x(span.end))),
                top: Length::Px((self.height - height) / 2.),
                ..Default::default()
            })
    }
}

fn push_pill<M>(
    pieces: &mut Vec<View<M>>,
    layout: LinearLayout,
    span: Range<f32>,
    height: f32,
    color: Color,
) {
    if !span.is_empty() {
        let style = layout
            .positioned(span, height)
            .corners(shape::FULL)
            .background(color);
        pieces.push(View::new(Node::new(Role::Group, "").styled(style)).without_interaction(None));
    }
}

#[derive(Clone, Copy)]
struct Wave {
    wavelength: f32,
    phase: f32,
}

impl Wave {
    // The Material centerline is made of alternating quadratic half waves.
    // Sample its tangent as well so the stroke keeps its 4 dp thickness.
    fn sample(self, x: f32) -> StrokeSample {
        let half_wave = (2. * (x / self.wavelength - self.phase)).rem_euclid(2.);
        let sign = if half_wave < 1. { 1. } else { -1. };
        let t = half_wave.fract();
        let y = WAVE_HEIGHT / 2. + sign * WAVE_AMPLITUDE * 4. * t * (1. - t);
        let slope = sign * WAVE_AMPLITUDE * 8. * (1. - 2. * t) / self.wavelength;
        StrokeSample {
            x,
            y,
            tangent: slope.atan(),
        }
    }

    fn push_stroke<M>(
        self,
        pieces: &mut Vec<View<M>>,
        layout: LinearLayout,
        span: Range<f32>,
        color: Color,
    ) {
        let start = span.start + THICKNESS / 2.;
        let end = span.end - THICKNESS / 2.;
        let length = end - start;
        // Keep a sub-wavelength sampling step on wide layouts. Split
        // only at a shared cross section so adjacent contours meet.
        const SAMPLE_STEP: f32 = 2.;
        const STEPS_PER_CONTOUR: usize = 118;
        let total_steps = (length / SAMPLE_STEP).ceil().max(1.) as usize;
        let contour_count = total_steps.div_ceil(STEPS_PER_CONTOUR);
        for contour in 0..contour_count {
            let from = contour * STEPS_PER_CONTOUR;
            let to = (from + STEPS_PER_CONTOUR).min(total_steps);
            let samples: Vec<_> = (from..=to)
                .map(|step| self.sample(start + length * (step as f32 / total_steps as f32)))
                .collect();
            let points = stroke_outline(&samples, from == 0, to == total_steps)
                .map(|[x, y]| [layout.physical_x(x), y]);
            if let Some(path) = FillContour::new(points, color) {
                let style = layout.positioned(0.0..layout.width, layout.height);
                pieces.push(
                    View::new(Node::new(Role::Group, "").styled(style).fill_contour(path))
                        .without_interaction(None),
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
struct StrokeSample {
    x: f32,
    y: f32,
    tangent: f32,
}

impl StrokeSample {
    fn edge(self, normal: f32) -> [f32; 2] {
        let (dy, dx) = (self.tangent + normal).sin_cos();
        [self.x + dx * THICKNESS / 2., self.y + dy * THICKNESS / 2.]
    }

    fn cap(self, normal: f32, draw: bool) -> impl Iterator<Item = [f32; 2]> {
        const CAP_STEPS: u8 = 8;
        (1..CAP_STEPS)
            .filter(move |_| draw)
            .map(move |step| self.edge(normal + PI * f32::from(step) / f32::from(CAP_STEPS)))
    }
}

fn stroke_outline(
    samples: &[StrokeSample],
    cap_start: bool,
    cap_end: bool,
) -> impl Iterator<Item = [f32; 2]> + '_ {
    samples
        .iter()
        .map(|sample| sample.edge(-FRAC_PI_2))
        .chain(samples[samples.len() - 1].cap(-FRAC_PI_2, cap_end))
        .chain(samples.iter().rev().map(|sample| sample.edge(FRAC_PI_2)))
        .chain(samples[0].cap(FRAC_PI_2, cap_start))
}
