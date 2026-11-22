//! Presented style values that follow targets over time.
//!
//! The semantic DOM always holds target styles. This module only edits the
//! display copy that `render` compiles, so exports and the bridge never see
//! a value that is still in motion.
//!
//! Elements are tracked by stable ID. An element without one is tracked by its
//! position under the nearest stable ancestor, so animate only elements that
//! have a stable ID. If siblings without IDs are inserted or reordered, a node
//! can inherit the track of the node that held its position and fade from its
//! values. When no element has a transition and nothing runs, tracks are
//! dropped. Then `Motion::present` does no allocation or track work. It still
//! scans the nodes once per call for a transition. While every track is
//! settled and no target has moved, `present` only compares the tree with a
//! saved copy and allocates nothing. `render` itself still lays out and clones
//! the DOM. Because tracks are dropped then, a transition added in the same
//! update as its first value change does not animate that first change. Later
//! changes animate.
use crate::{Application, Runtime};
use std::collections::BTreeMap;
use surface::{Color, ComputedDom, Corners, DomError, Easing, MicroDom, NodeId, Style, Transition};

#[derive(Clone, Copy, PartialEq)]
struct Value([f32; 4]);

impl Value {
    fn color(color: Color) -> Self {
        Self([color.0, color.1, color.2, color.3].map(f32::from))
    }
    fn to_color(self) -> Color {
        let [r, g, b, a] = self.0.map(|v| v.round().clamp(0., 255.) as u8);
        Color(r, g, b, a)
    }
}

#[derive(Clone, Copy)]
struct Run {
    from: Value,
    start: f64,
    duration: f64,
    easing: Easing,
}

/// One animated color of one element: four channels in 0-255 sRGB, rounded.
#[derive(Clone, Copy)]
struct Channel {
    target: Value,
    shown: Value,
    run: Option<Run>,
}

impl Channel {
    fn new(target: Value) -> Self {
        Self {
            target,
            shown: target,
            run: None,
        }
    }
    /// Follow `target` at `now`. Returns true while a transition still runs.
    fn update(&mut self, target: Value, transition: Option<Transition>, now: f64) -> bool {
        if self.target != target {
            self.target = target;
            match transition.filter(|t| t.is_valid() && t.duration_ms > 0.) {
                Some(t) => {
                    self.run = Some(Run {
                        from: self.shown,
                        start: now,
                        duration: f64::from(t.duration_ms),
                        easing: t.easing,
                    });
                }
                None => {
                    self.run = None;
                    self.shown = target;
                }
            }
        }
        let Some(run) = self.run else {
            return false;
        };
        let progress = ((now - run.start) / run.duration).clamp(0., 1.);
        if progress >= 1. {
            self.run = None;
            self.shown = self.target;
            return false;
        }
        let eased = run.easing.sample(progress as f32);
        let mut value = [0.; 4];
        for (i, v) in value.iter_mut().enumerate() {
            let (from, to) = (run.from.0[i], self.target.0[i]);
            *v = from + (to - from) * eased;
        }
        self.shown = Value(value.map(|v| v.round().clamp(0., 255.)));
        true
    }
    /// Jump to `target` and cancel any running transition.
    fn settle(&mut self, target: Value) {
        self.target = target;
        self.shown = target;
        self.run = None;
    }
}

/// The corners of one element. `target` is the unresolved style value, so a
/// resize of a `FULL` shape is not a change. `shown` is resolved and only
/// meaningful while `run` is set.
#[derive(Clone, Copy)]
struct CornerChannel {
    target: Corners,
    shown: Corners,
    run: Option<Run>,
}

fn corner_values(c: Corners) -> [f32; 4] {
    [c.top_left, c.top_right, c.bottom_right, c.bottom_left]
}

impl CornerChannel {
    fn new(target: Corners) -> Self {
        Self {
            target,
            shown: target,
            run: None,
        }
    }
    /// Follow `target` inside `width` x `height`. Returns true while a run is active.
    fn update(
        &mut self,
        target: Corners,
        transition: Option<Transition>,
        now: f64,
        (width, height): (f32, f32),
    ) -> bool {
        if self.target != target {
            let from = match self.run {
                Some(_) => self.shown,
                None => self.target.resolve(width, height),
            };
            self.target = target;
            self.run = transition
                .filter(|t| t.is_valid() && t.duration_ms > 0.)
                .map(|t| Run {
                    from: Value(corner_values(from)),
                    start: now,
                    duration: f64::from(t.duration_ms),
                    easing: t.easing,
                });
        }
        let Some(run) = self.run else {
            return false;
        };
        let progress = ((now - run.start) / run.duration).clamp(0., 1.);
        if progress >= 1. {
            self.run = None;
            return false;
        }
        let eased = run.easing.sample(progress as f32);
        let to = corner_values(target.resolve(width, height));
        let max = width.max(0.).min(height.max(0.)) / 2.;
        let mut value = [0.; 4];
        for (i, v) in value.iter_mut().enumerate() {
            let at = run.from.0[i] + (to[i] - run.from.0[i]) * eased;
            *v = if at.is_finite() {
                at.clamp(0., max)
            } else {
                0.
            };
        }
        let [top_left, top_right, bottom_right, bottom_left] = value;
        self.shown = Corners::new(top_left, top_right, bottom_right, bottom_left);
        true
    }
    /// Jump to `target` and cancel any running transition.
    fn settle(&mut self, target: Corners) {
        self.target = target;
        self.run = None;
    }
}

const COLORS: usize = 4;

/// All animated properties of one element.
struct Track {
    colors: [Channel; COLORS],
    corners: CornerChannel,
}

impl Track {
    fn targets(style: &Style) -> [Value; COLORS] {
        [
            Value::color(style.background),
            Value::color(style.foreground),
            Value::color(style.border_color),
            Value::color(style.state_layer),
        ]
    }
    fn new(style: &Style) -> Self {
        Self {
            colors: Self::targets(style).map(Channel::new),
            corners: CornerChannel::new(style.corner_radii()),
        }
    }
    fn settled(&self) -> bool {
        self.colors.iter().all(|c| c.shown == c.target) && self.corners.run.is_none()
    }
    /// Write the presented values into a copy of the target style.
    fn present(&self, style: &mut Style) {
        let [background, foreground, border, layer] = &self.colors;
        style.background = background.shown.to_color();
        style.foreground = foreground.shown.to_color();
        style.border_color = border.shown.to_color();
        style.state_layer = layer.shown.to_color();
        if self.corners.run.is_some() {
            style.corners = Some(self.corners.shown);
        }
    }
}

/// The element key: its stable ID, or the nearest ancestor's stable ID (empty
/// at the root) and the child-index path from that ancestor, like `save/0`.
fn keys(dom: &MicroDom) -> Vec<(NodeId, String)> {
    let mut out = Vec::with_capacity(dom.len());
    let mut stack = vec![(dom.root(), String::new(), String::new())];
    while let Some((id, anchor, path)) = stack.pop() {
        let Ok(node) = dom.get(id) else { continue };
        let (anchor, path) = match node.stable_id() {
            Some(stable) => (stable.to_owned(), String::new()),
            None => (anchor, path),
        };
        // The prefixes and NUL separators keep a stable ID such as `save/0`
        // apart from the first unstable child of `save`.
        out.push((
            id,
            match node.stable_id() {
                Some(stable) => format!("s\0{stable}"),
                None => format!("p\0{anchor}\0{path}"),
            },
        ));
        let children: Vec<_> = dom.children(id).collect();
        for (index, child) in children.into_iter().enumerate().rev() {
            let path = if path.is_empty() {
                index.to_string()
            } else {
                format!("{path}/{index}")
            };
            stack.push((child, anchor.clone(), path));
        }
    }
    out
}

#[derive(Default)]
pub(crate) struct Motion {
    /// Host clock in milliseconds. Motion is off until it is first set.
    now: Option<f64>,
    reduced: bool,
    running: bool,
    tracks: BTreeMap<String, Track>,
    /// The tree as the last full pass saw it. Only kept while nothing runs.
    seen: Vec<Seen>,
    /// Full passes made so far. Tests use it to see that a still tree is skipped.
    #[cfg(test)]
    passes: usize,
}

/// What decides the keys and targets of one node.
struct Seen {
    id: NodeId,
    stable: Option<String>,
    children: Vec<NodeId>,
    targets: [Value; COLORS],
    corners: Corners,
}

impl Seen {
    fn of(dom: &MicroDom, id: NodeId, node: &surface::Node) -> Self {
        let style = node.style();
        Self {
            id,
            stable: node.stable_id().map(str::to_owned),
            children: dom.children(id).collect(),
            targets: Track::targets(style),
            corners: style.corner_radii(),
        }
    }
    /// The same check as `of`, without allocating.
    fn matches(&self, dom: &MicroDom, id: NodeId, node: &surface::Node) -> bool {
        let style = node.style();
        self.id == id
            && self.stable.as_deref() == node.stable_id()
            && self.targets == Track::targets(style)
            && self.corners == style.corner_radii()
            && dom.children(id).eq(self.children.iter().copied())
    }
}

impl Motion {
    /// Update the display copy of the DOM with presented values.
    fn present(
        &mut self,
        dom: &MicroDom,
        display: &mut MicroDom,
        computed: &ComputedDom,
    ) -> Result<(), DomError> {
        let Some(now) = self.now else {
            self.tracks.clear();
            self.seen.clear();
            self.running = false;
            return Ok(());
        };
        if !self.running {
            if !dom
                .iter()
                .any(|(_, node)| node.style().transition.is_some())
            {
                self.tracks.clear();
                self.seen.clear();
                return Ok(());
            }
            // Every track is settled and no target moved, so the display
            // copy needs no edit. Skip the keys, tracks and clones.
            if self.seen.len() == dom.len()
                && dom
                    .iter()
                    .zip(&self.seen)
                    .all(|((id, node), seen)| seen.matches(dom, id, node))
            {
                return Ok(());
            }
        }
        #[cfg(test)]
        {
            self.passes += 1;
        }
        let mut previous = std::mem::take(&mut self.tracks);
        let mut running = false;
        for (id, key) in keys(dom) {
            let style = dom.get(id)?.style();
            // A key seen for the first time starts at its target.
            let mut track = previous.remove(&key).unwrap_or_else(|| Track::new(style));
            for (channel, target) in track.colors.iter_mut().zip(Track::targets(style)) {
                if self.reduced {
                    channel.settle(target);
                } else {
                    running |= channel.update(target, style.transition, now);
                }
            }
            let bounds = computed.nodes.get(id.index()).map(|n| n.bounds);
            let size = bounds.map_or((0., 0.), |b| (b.width, b.height));
            if self.reduced {
                track.corners.settle(style.corner_radii());
            } else {
                running |= track
                    .corners
                    .update(style.corner_radii(), style.transition, now, size);
            }
            if !track.settled() {
                let mut presented = style.clone();
                track.present(&mut presented);
                display.set_style(id, presented)?;
            }
            self.tracks.insert(key, track);
        }
        self.running = running;
        self.seen = if running {
            Vec::new()
        } else {
            dom.iter()
                .map(|(id, node)| Seen::of(dom, id, node))
                .collect()
        };
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ScalarSpring {
    start: f64,
    rate: f64,
    damping_ratio: f64,
    initial: [f64; 2],
}

impl ScalarSpring {
    const THRESHOLD: f64 = 0.01;

    pub(crate) fn new(stiffness: f32, damping_ratio: f32, initial: [f64; 2], start: f64) -> Self {
        Self {
            start,
            rate: f64::from(stiffness).sqrt(),
            damping_ratio: f64::from(damping_ratio),
            initial,
        }
    }

    pub(crate) fn sample(self, now: f64) -> Option<[f64; 2]> {
        let seconds = ((now - self.start) / 1000.).max(0.);
        let [position, velocity] = self.initial;
        let offset = position - 1.;
        if self.damping_ratio < 1. {
            let damping = self.rate * self.damping_ratio;
            let frequency = self.rate * (1. - self.damping_ratio * self.damping_ratio).sqrt();
            let sine_weight = (velocity + damping * offset) / frequency;
            // The envelope bounds every later oscillation. A crossing of
            // the target itself does not end an underdamped spring.
            let envelope = offset.hypot(sine_weight);
            let duration = ((envelope / Self::THRESHOLD).ln() / damping * 1000.).floor() / 1000.;
            if seconds >= duration {
                return None;
            }
            let (sin, cos) = (frequency * seconds).sin_cos();
            let displacement = offset * cos + sine_weight * sin;
            let slope = frequency * (-offset * sin + sine_weight * cos);
            let decay = (-damping * seconds).exp();
            return Some([
                1. + displacement * decay,
                (slope - damping * displacement) * decay,
            ]);
        }
        let phase = seconds * self.rate;
        if phase > 100. {
            return None;
        }
        let slope = velocity + self.rate * offset;
        let decay = (-phase).exp();
        let pose = [
            1. + (offset + slope * seconds) * decay,
            (velocity - slope * phase) * decay,
        ];
        ((1. - pose[0]).abs() > Self::THRESHOLD || pose[1].abs() > Self::THRESHOLD * self.rate)
            .then_some(pose)
    }
}

const STANDARD_FAST_SPATIAL: (f32, f32) = (1400., 0.9);

/// Presets from the Material 3 Standard motion scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToggleSpring {
    StandardDefaultSpatial,
    StandardFastSpatial,
    StandardDefaultEffects,
}

impl ToggleSpring {
    fn parameters(self) -> (f32, f32) {
        match self {
            Self::StandardDefaultSpatial => (700., 0.9),
            Self::StandardFastSpatial => STANDARD_FAST_SPATIAL,
            Self::StandardDefaultEffects => (1600., 1.),
        }
    }
}

/// Sample an app-controlled transition from its current position and velocity.
/// Use the same preset for this track throughout its lifetime.
#[derive(Default)]
pub struct ToggleMotion(BinarySpring);

impl ToggleMotion {
    pub fn new(target: bool) -> Self {
        let mut motion = Self::default();
        motion.0.snap(target);
        motion
    }

    /// Start settling from a gesture's current fraction and velocity.
    /// Progress is clamped to `0..=1`. Express velocity in fractions per
    /// second, with a positive value moving toward 1. Sample with the same
    /// preset to continue the motion. Nonfinite input snaps to the target.
    pub fn start_from(
        target: bool,
        progress: f32,
        velocity_per_second: f32,
        now_ms: f64,
        spring: ToggleSpring,
    ) -> Self {
        let mut motion = Self::new(target);
        if progress.is_finite() && velocity_per_second.is_finite() && now_ms.is_finite() {
            motion.0.start(
                target,
                [
                    f64::from(progress.clamp(0., 1.)),
                    f64::from(velocity_per_second),
                ],
                now_ms,
                spring.parameters(),
            );
        }
        motion
    }

    /// Return the current fraction in 0..=1 and whether to request another frame.
    /// Reduced Motion and frames without a finite clock snap to the target.
    pub fn sample(
        &mut self,
        target: bool,
        now_ms: Option<f64>,
        reduced_motion: bool,
        spring: ToggleSpring,
    ) -> (f32, bool) {
        self.sample_parameters(target, now_ms, reduced_motion, spring.parameters())
    }

    fn sample_parameters(
        &mut self,
        target: bool,
        now_ms: Option<f64>,
        reduced_motion: bool,
        parameters: (f32, f32),
    ) -> (f32, bool) {
        let Some(now_ms) = now_ms.filter(|time| time.is_finite() && !reduced_motion) else {
            self.0.snap(target);
            return (if target { 1. } else { 0. }, false);
        };
        self.0.aim(target, now_ms, parameters);
        self.0.finish(now_ms);
        (
            self.0.pose(now_ms)[0].clamp(0., 1.) as f32,
            self.0.running(),
        )
    }
}

#[derive(Default)]
pub(crate) struct BinarySpring {
    target: bool,
    spring: Option<ScalarSpring>,
}

impl BinarySpring {
    pub(crate) fn pose(&self, now: f64) -> [f64; 2] {
        let [position, velocity] = self
            .spring
            .and_then(|spring| spring.sample(now))
            .unwrap_or([1., 0.]);
        if self.target {
            [position, velocity]
        } else {
            [1. - position, -velocity]
        }
    }

    pub(crate) fn aim(&mut self, target: bool, now: f64, parameters: (f32, f32)) {
        if self.target == target {
            return;
        }
        self.start(target, self.pose(now), now, parameters);
    }

    fn start(
        &mut self,
        target: bool,
        [position, velocity]: [f64; 2],
        now: f64,
        (stiffness, damping): (f32, f32),
    ) {
        let initial = if target {
            [position, velocity]
        } else {
            [1. - position, -velocity]
        };
        self.target = target;
        self.spring = Some(ScalarSpring::new(stiffness, damping, initial, now));
    }

    pub(crate) fn snap(&mut self, target: bool) {
        self.target = target;
        self.spring = None;
    }

    pub(crate) fn finish(&mut self, now: f64) {
        self.spring = self.spring.filter(|spring| spring.sample(now).is_some());
    }

    pub(crate) fn running(&self) -> bool {
        self.spring.is_some()
    }
}

impl<A: Application> Runtime<A> {
    /// Set the host's monotonic clock in milliseconds.
    ///
    /// Motion is off until this has been called once. Values that are not
    /// finite, or that are earlier than the current time, are ignored. The
    /// clock never moves back. If the host restarts its clock, make a new
    /// `Runtime` or keep feeding times above the old maximum.
    pub fn set_time(&mut self, now_ms: f64) {
        if now_ms.is_finite() && self.motion.now.is_none_or(|now| now_ms >= now) {
            self.motion.now = Some(now_ms);
        }
    }
    /// Apply style changes immediately. Running transitions finish at the next render.
    pub fn set_reduced_motion(&mut self, reduce: bool) {
        self.motion.reduced = reduce;
    }
    pub fn reduced_motion(&self) -> bool {
        self.motion.reduced
    }
    /// True while a transition was still running at the last `render`.
    ///
    /// Hosts should render again on the next frame while this is true.
    pub fn needs_frame(&self) -> bool {
        self.animation_request_next || (self.motion.running && !self.motion.reduced)
    }
    pub(crate) fn input_time_ms(&self) -> Option<f64> {
        self.motion.now
    }

    pub(crate) fn animate_application(&mut self) -> Result<(), crate::AppError> {
        let frame = crate::AnimationFrame {
            now_ms: self.motion.now,
            reduced_motion: self.motion.reduced,
            dom: &self.dom,
            layout: self.computed.as_ref(),
        };
        let input = crate::FrameInput {
            pointer: self.pointer_position,
            focused: self.focused.as_deref(),
            touch: self.touch.as_ref().map(|touch| touch.contact),
            touch_cancelled: std::mem::take(&mut self.touch_cancelled),
        };
        let update = self.app.animation_frame_with_input(frame, input);
        if let Some(touch) = &mut self.touch {
            touch.suppress_activation |= update.suppress_touch_activation;
        }
        self.animation_request_next = update.animation.request_next;
        self.animation_rebuild_pending |= update.animation.rebuild;
        if self.animation_rebuild_pending {
            self.rebuild()?;
        }
        Ok(())
    }
    /// Replace animated values in the display copy with presented ones.
    /// Layout has already used the target styles.
    pub(crate) fn present_motion(
        &mut self,
        display: &mut MicroDom,
        computed: &ComputedDom,
    ) -> Result<(), DomError> {
        self.motion.present(&self.dom, display, computed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::View;
    use surface::{Length, Node, Role, TextMeasurer};

    struct Metrics;
    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
            (text.chars().count() as f32 * size / 2., size * 1.5)
        }
    }

    struct App {
        dark: bool,
        transition: bool,
    }
    impl Application for App {
        type Message = ();
        fn update(&mut self, _: ()) {
            self.dark = !self.dark;
        }
        fn view(&self) -> View<()> {
            let mut style = Style::default()
                .width(Length::Px(100.))
                .height(Length::Px(40.))
                .background(Color::rgb(if self.dark { 200 } else { 0 }, 0, 0));
            if self.transition {
                style.transition = Some(Transition::new(100., Easing::Linear));
            }
            View::new(Node::new(Role::Button, "box").id("box").styled(style)).on_press(())
        }
    }

    fn render(runtime: &mut Runtime<App>) {
        runtime.render(320., 240., &mut Metrics).unwrap();
    }

    fn settled_runtime(reduced: bool) -> Runtime<App> {
        let mut runtime = Runtime::new(App {
            dark: false,
            transition: true,
        })
        .unwrap();
        runtime.set_reduced_motion(reduced);
        runtime.set_time(0.);
        render(&mut runtime);
        runtime
    }

    #[test]
    fn a_still_tree_with_a_transition_skips_the_full_pass() {
        for reduced in [false, true] {
            let mut runtime = settled_runtime(reduced);
            render(&mut runtime);
            let passes = runtime.motion.passes;
            for t in [10., 20., 30.] {
                runtime.set_time(t);
                render(&mut runtime);
            }
            assert_eq!(
                runtime.motion.passes, passes,
                "reduced={reduced}: a still tree made a full pass"
            );
        }
    }

    #[test]
    fn a_target_change_makes_a_full_pass_again() {
        let mut runtime = settled_runtime(false);
        render(&mut runtime);
        let passes = runtime.motion.passes;
        runtime.activate("box").unwrap();
        runtime.set_time(10.);
        render(&mut runtime);
        assert!(runtime.motion.passes > passes, "the change was skipped");
        assert!(runtime.needs_frame());
    }
}
