//! Frame-driven Material Expressive button group motion and measurements.

use std::collections::BTreeMap;

use surface::Rect;
use surface_app::{AnimationFrame, AnimationUpdate};

// Fast Spatial Expressive spring. Mass is 1 and time is in seconds.
const STIFFNESS: f64 = 800.;
const DAMPING_RATIO: f64 = 0.6;
const PULSE_MS: f64 = 75.;

/// Motion and measurements for one mounted button group. Keep this instance
/// across frames. Application selection and menu state remain independent.
#[derive(Default)]
pub struct ButtonGroupMotion {
    measured: BTreeMap<String, f32>,
    tallest_label: f32,
    trigger: Option<Rect>,
    viewport: Rect,
    active: Option<String>,
    held: Option<String>,
    pending_pulse: Option<String>,
    pulse_end: Option<f64>,
    segment: Spring,
    progress: f32,
}

impl ButtonGroupMotion {
    /// Animate an activation that has no held press, such as Enter or an
    /// accessibility action. Calling this at the end of a pointer press
    /// does not start a new pulse.
    pub fn pulse(&mut self, item_id: &str) {
        if self.held.as_deref() != Some(item_id) {
            self.pending_pulse = Some(item_id.to_owned());
        }
    }

    /// Track the accepted primary press and last rendered geometry. Use the
    /// stable button ID that the runtime provides in its animation frame hook.
    pub fn advance(
        &mut self,
        group_id: &str,
        frame: &AnimationFrame<'_>,
        pressed: Option<&str>,
    ) -> AnimationUpdate {
        let mut changed = self.measure(group_id, frame);
        let row = frame.dom.by_id(&format!("{group_id}-row"));
        let belongs = |id: &str| {
            row.is_some_and(|row| {
                frame
                    .dom
                    .by_id(id)
                    .and_then(|button| frame.dom.get(button).ok())
                    .is_some_and(|button| button.parent() == Some(row) && !button.is_disabled())
            })
        };
        let held = pressed.filter(|id| belongs(id));
        let pending = self.pending_pulse.take().filter(|id| belongs(id));
        self.held = held.map(str::to_owned);
        let prev = (self.active.clone(), self.progress);
        let now = frame.now_ms.filter(|time| time.is_finite());
        let pulse = if let Some(held) = held {
            self.pulse_end = None;
            Some(held.to_owned())
        } else if let Some(pending) = pending.filter(|_| !frame.reduced_motion) {
            self.pulse_end = now.map(|now| now + PULSE_MS);
            Some(pending)
        } else {
            let pulse = self
                .pulse_end
                .zip(now)
                .filter(|(end, now)| now < end && !frame.reduced_motion)
                .and(self.active.clone());
            if pulse.is_none() {
                self.pulse_end = None;
            }
            pulse
        };
        let mut running = false;
        if let Some(now) = now.filter(|_| !frame.reduced_motion) {
            if let Some(id) = pulse.as_deref() {
                if self.active.as_deref() != Some(id) {
                    self.segment = Spring::default();
                    self.active = Some(id.to_owned());
                }
            }
            self.segment.aim(if pulse.is_some() { 1. } else { 0. }, now);
            let (value, velocity) = self.segment.sample(now);
            running = (value - self.segment.target).abs() > 0.001 || velocity.abs() > 0.025;
            self.progress = if running {
                value.max(0.) as f32
            } else {
                self.segment.target as f32
            };
            if !running && pulse.is_none() {
                self.active = None;
            }
        } else {
            self.active = pulse;
            self.progress = 0.;
            self.segment = Spring::default();
            self.pulse_end = None;
        }
        changed |= prev != (self.active.clone(), self.progress);
        AnimationUpdate {
            rebuild: changed,
            request_next: changed
                || running
                || (row.is_some() && frame.layout.is_none())
                || self.pulse_end.zip(now).is_some_and(|(end, now)| now < end),
        }
    }

    pub(super) fn measured(&self, id: &str) -> Option<f32> {
        self.measured.get(&format!("{id}-measure")).copied()
    }

    pub(super) fn presentation(&self) -> (Option<&str>, f32) {
        (self.active.as_deref(), self.progress)
    }

    pub(super) fn is_pressed(&self, id: &str) -> bool {
        self.held.as_deref() == Some(id)
            || (self.pulse_end.is_some() && self.active.as_deref() == Some(id))
    }

    pub(super) fn tallest_label(&self) -> f32 {
        self.tallest_label
    }

    pub(super) fn menu_anchor(&self) -> (Option<Rect>, Rect) {
        (self.trigger, self.viewport)
    }

    fn measure(&mut self, id: &str, frame: &AnimationFrame<'_>) -> bool {
        let Some(layout) = frame.layout else {
            return false;
        };
        let sizes = frame
            .dom
            .by_id(&format!("{id}-measure"))
            .into_iter()
            .flat_map(|group| frame.dom.children(group))
            .filter_map(|child| {
                let id = frame.dom.get(child).ok()?.stable_id()?;
                let width = layout.nodes.get(child.index())?.bounds.width;
                (width > 0. && width.is_finite()).then_some((id.to_owned(), width))
            })
            .collect();
        let trigger = frame
            .dom
            .by_id(&format!("{id}-overflow"))
            .and_then(|node| layout.nodes.get(node.index()))
            .map(|node| node.bounds);
        let tallest_label = frame
            .dom
            .by_id(&format!("{id}-row"))
            .into_iter()
            .flat_map(|row| frame.dom.children(row))
            .filter_map(|button| {
                let id = frame.dom.get(button).ok()?.stable_id()?;
                let label = frame.dom.by_id(&format!("{id}-label"))?;
                Some(layout.nodes.get(label.index())?.bounds.height)
            })
            .fold(0., f32::max);
        let changed = self.measured != sizes
            || self.tallest_label != tallest_label
            || self.trigger != trigger
            || self.viewport != layout.viewport;
        self.measured = sizes;
        self.tallest_label = tallest_label;
        self.trigger = trigger;
        self.viewport = layout.viewport;
        changed
    }
}

#[derive(Default)]
struct Spring {
    target: f64,
    value: f64,
    velocity: f64,
    started_ms: f64,
}

impl Spring {
    fn aim(&mut self, target: f64, now: f64) {
        if target != self.target || now < self.started_ms {
            let (value, velocity) = self.sample(now);
            *self = Self {
                target,
                value,
                velocity,
                started_ms: now,
            };
        }
    }

    fn sample(&self, now: f64) -> (f64, f64) {
        let elapsed = (now - self.started_ms).max(0.) / 1000.;
        if elapsed > 1. {
            return (self.target, 0.);
        }
        let decay = DAMPING_RATIO * STIFFNESS.sqrt();
        let frequency = (STIFFNESS - decay * decay).sqrt();
        let offset = self.value - self.target;
        let sine_weight = (self.velocity + decay * offset) / frequency;
        let (sine, cosine) = (frequency * elapsed).sin_cos();
        let envelope = (-decay * elapsed).exp();
        let wave = offset * cosine + sine_weight * sine;
        let derivative = frequency * (-offset * sine + sine_weight * cosine);
        (
            self.target + envelope * wave,
            envelope * (derivative - decay * wave),
        )
    }
}
