//! State for an inline numeric Scroll Field.

use surface_app::{AnimationUpdate, PointDragSession};

use crate::fab_motion::Spring;

const ASCII_DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
// Keep the fractional position precise even for an unusually long drag.
const MAX_DRAG_PHASE: f64 = (1_u64 << 40) as f64;

/// A finite range with at least two reachable values and a positive step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollFieldRange {
    min: i32,
    max: i32,
    step: i32,
}

impl ScrollFieldRange {
    /// Return no range if the bounds are reversed, equal, or misaligned.
    pub fn new(min: i32, max: i32, step: i32) -> Option<Self> {
        let span = i64::from(max) - i64::from(min);
        (span > 0 && step > 0 && span % i64::from(step) == 0).then_some(Self { min, max, step })
    }

    pub fn min(self) -> i32 {
        self.min
    }

    pub fn max(self) -> i32 {
        self.max
    }

    pub fn step(self) -> i32 {
        self.step
    }

    pub fn contains(self, value: i32) -> bool {
        (self.min..=self.max).contains(&value)
            && (i64::from(value) - i64::from(self.min)) % i64::from(self.step) == 0
    }

    fn index(self, value: i32) -> i64 {
        (i64::from(value) - i64::from(self.min)) / i64::from(self.step)
    }

    pub(crate) fn value_at(self, index: i64) -> Option<i32> {
        (0..=self.index(self.max))
            .contains(&index)
            .then(|| (i64::from(self.min) + index * i64::from(self.step)) as i32)
    }

    fn clamp_offset(self, offset: f64) -> f64 {
        offset.clamp(0., self.index(self.max) as f64)
    }

    fn step_value(self, value: i32, step: ScrollFieldStep) -> i32 {
        let index = self.index(value);
        match step {
            ScrollFieldStep::Increase => self.value_at(index + 1).unwrap_or(self.max),
            ScrollFieldStep::Decrease => self.value_at(index - 1).unwrap_or(self.min),
            ScrollFieldStep::First => self.min,
            ScrollFieldStep::Last => self.max,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollFieldMode {
    Scroll,
    Input,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollFieldError {
    Empty,
    InvalidNumber,
    OutOfRange,
    OffStep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollFieldStep {
    Increase,
    Decrease,
    First,
    Last,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollFieldDrag {
    /// Travel from pointer down in logical pixels. Positive is down.
    Move {
        delta_y: f32,
    },
    /// Final travel from pointer down, even if no move was delivered.
    End {
        delta_y: f32,
    },
    /// An accepted tap in the measured track.
    Tap {
        value: i32,
    },
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScrollFieldEvent {
    Edit,
    SetText(String),
    Submit,
    Cancel,
    SetValue(i32),
    Step(ScrollFieldStep),
    Drag {
        session: PointDragSession,
        sample: ScrollFieldDrag,
        row_height: f32,
    },
}

#[derive(Clone, Copy)]
enum Motion {
    Idle,
    Held {
        origin: f64,
        row_height: f64,
    },
    Settling {
        target: f64,
        spring: Spring,
        last_ms: Option<f64>,
    },
}

/// Retained draft, accepted value and visible offset for one field.
pub struct ScrollFieldState {
    range: ScrollFieldRange,
    value: i32,
    digits: [char; 10],
    mode: ScrollFieldMode,
    scroll_fits: bool,
    looping: bool,
    raw: String,
    error: Option<ScrollFieldError>,
    offset: f64,
    motion: Motion,
    session: PointDragSession,
}

impl ScrollFieldState {
    pub fn new(range: ScrollFieldRange, value: i32) -> Option<Self> {
        range.contains(value).then(|| Self {
            range,
            value,
            digits: ASCII_DIGITS,
            mode: ScrollFieldMode::Scroll,
            scroll_fits: true,
            looping: false,
            raw: value.to_string(),
            error: None,
            offset: range.index(value) as f64,
            motion: Motion::Idle,
            session: PointDragSession::new(),
        })
    }

    /// Let the wheel continue through both ends of its authored range.
    /// The authored bounds still apply to direct text and absolute values.
    pub fn with_looping(mut self) -> Self {
        self.looping = true;
        self
    }

    pub fn looping(&self) -> bool {
        self.looping
    }

    /// Select one distinct glyph per digit. ASCII and full-width digits
    /// remain accepted regardless of how the value is displayed.
    pub fn with_digits(mut self, digits: [char; 10]) -> Option<Self> {
        for (index, digit) in digits.iter().copied().enumerate() {
            if digit.is_whitespace()
                || digit.is_control()
                || matches!(digit, '+' | '＋' | '-' | '−' | '－')
                || digits[..index].contains(&digit)
                || standard_digit(digit).is_some_and(|value| value as usize != index)
            {
                return None;
            }
        }
        self.digits = digits;
        self.restore_raw();
        Some(self)
    }

    pub fn range(&self) -> ScrollFieldRange {
        self.range
    }

    /// The accepted value. A held gesture and an input draft leave it unchanged.
    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn mode(&self) -> ScrollFieldMode {
        if self.scroll_fits {
            self.mode
        } else {
            ScrollFieldMode::Input
        }
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Error from the last failed submission, cleared when the draft is corrected.
    pub fn error(&self) -> Option<ScrollFieldError> {
        self.error
    }

    pub fn format_value(&self, value: i32) -> String {
        value
            .to_string()
            .chars()
            .map(|ch| standard_digit(ch).map_or(ch, |digit| self.digits[digit as usize]))
            .collect()
    }

    /// Fractional row index from the minimum. The looping wheel uses a signed
    /// index while moving and returns to its canonical index at rest.
    pub fn offset(&self) -> f64 {
        self.offset
    }

    /// The value closest to the visible center.
    pub fn preview(&self) -> i32 {
        self.value_at(self.offset.round() as i64)
            .expect("offset stays within the range")
    }

    pub(crate) fn value_at(&self, index: i64) -> Option<i32> {
        let index = if self.looping {
            index.rem_euclid(self.period())
        } else {
            index
        };
        self.range.value_at(index)
    }

    pub fn is_dragging(&self) -> bool {
        matches!(self.motion, Motion::Held { .. })
    }

    pub fn settling(&self) -> bool {
        matches!(self.motion, Motion::Settling { .. })
    }

    /// Retain this identity in point-capture bindings across preview renders.
    pub fn drag_session(&self) -> PointDragSession {
        self.session.clone()
    }

    /// Report whether the scroll presentation fits. A mounted editor stays
    /// active through resize until the user confirms, cancels or changes mode.
    pub fn update_fit(&mut self, scroll_fits: bool) -> bool {
        if self.scroll_fits == scroll_fits {
            return false;
        }
        self.mode = self.mode();
        self.scroll_fits = scroll_fits;
        self.reset_motion();
        true
    }

    /// Invalidate captured geometry even if the field still fits.
    /// A true result needs a view rebuild to install the fresh binding.
    pub fn invalidate_layout(&mut self) -> bool {
        if self.mode() != ScrollFieldMode::Scroll {
            return false;
        }
        self.reset_motion();
        true
    }

    /// Apply a new external value and discard any draft or captured motion.
    /// An invalid or unchanged value leaves the field as it is.
    pub fn sync_value(&mut self, value: i32) -> bool {
        if value == self.value || !self.range.contains(value) {
            return false;
        }
        self.value = value;
        self.restore_raw();
        self.reset_motion();
        true
    }

    /// Apply one input event. A returned value is a changed, accepted value
    /// for the caller to commit. A valid submission can close the editor
    /// without changing the value.
    pub fn apply(&mut self, event: ScrollFieldEvent, reduced_motion: bool) -> Option<i32> {
        match (self.mode(), event) {
            (ScrollFieldMode::Scroll, ScrollFieldEvent::Edit) => {
                self.mode = ScrollFieldMode::Input;
                self.reset_motion();
            }
            (ScrollFieldMode::Input, ScrollFieldEvent::SetText(raw)) => {
                if raw != self.raw {
                    self.mode = ScrollFieldMode::Input;
                    self.raw = raw;
                    if self.error.is_some() {
                        self.error = self.parse().err();
                    }
                }
            }
            (ScrollFieldMode::Input, ScrollFieldEvent::Submit) => match self.parse() {
                Ok(value) => {
                    self.mode = ScrollFieldMode::Scroll;
                    return self.accept(value, true);
                }
                Err(error) => self.error = Some(error),
            },
            (ScrollFieldMode::Input, ScrollFieldEvent::Cancel) => {
                self.mode = ScrollFieldMode::Scroll;
                self.restore_raw();
            }
            (ScrollFieldMode::Scroll, ScrollFieldEvent::Cancel) => self.cancel_drag(reduced_motion),
            (ScrollFieldMode::Scroll, ScrollFieldEvent::SetValue(value))
                if self.range.contains(value) =>
            {
                return self.accept(value, reduced_motion);
            }
            (ScrollFieldMode::Scroll, ScrollFieldEvent::Step(step)) => {
                return self.step(step, reduced_motion);
            }
            (
                ScrollFieldMode::Scroll,
                ScrollFieldEvent::Drag {
                    session,
                    sample,
                    row_height,
                },
            ) if session == self.session => {
                return self.drag(sample, row_height, reduced_motion);
            }
            _ => {}
        }
        None
    }

    /// Sample the Expressive Fast Spatial spring. A held preview follows
    /// the pointer directly. An absent clock or Reduce Motion snaps to rest.
    pub fn advance(&mut self, now_ms: Option<f64>, reduced_motion: bool) -> AnimationUpdate {
        let Motion::Settling {
            target,
            mut spring,
            last_ms,
        } = self.motion
        else {
            return AnimationUpdate::default();
        };
        let before = self.offset;
        if let Some(now) = now_ms.filter(|now| now.is_finite() && !reduced_motion) {
            let now = last_ms.map_or(now, |last| now.max(last));
            if let Some(last) = last_ms {
                let seconds = (now - last) / 1000.;
                if seconds >= 2. {
                    spring = Spring::new(target);
                } else {
                    spring.advance(target, seconds, 800., 0.6);
                }
            }
            if spring.resting_at(target) {
                self.finish_settle();
            } else {
                self.offset = if self.looping {
                    spring.position
                } else {
                    self.range.clamp_offset(spring.position)
                };
                self.motion = Motion::Settling {
                    target,
                    spring,
                    last_ms: Some(now),
                };
            }
        } else {
            self.finish_settle();
        }
        AnimationUpdate {
            rebuild: self.offset != before,
            request_next: self.settling(),
        }
    }

    fn step(&mut self, step: ScrollFieldStep, reduced_motion: bool) -> Option<i32> {
        let direction = match step {
            ScrollFieldStep::Increase => 1.,
            ScrollFieldStep::Decrease => -1.,
            ScrollFieldStep::First | ScrollFieldStep::Last => 0.,
        };
        if self.looping && direction != 0. {
            let current = match self.motion {
                Motion::Settling { target, .. } => target,
                _ => self.nearest_phase(self.value),
            };
            let target = current + direction;
            let value = self.value_at(target as i64).expect("looping row exists");
            self.accept_at(value, target, reduced_motion)
        } else {
            self.accept(self.range.step_value(self.value, step), reduced_motion)
        }
    }

    fn drag(
        &mut self,
        sample: ScrollFieldDrag,
        row_height: f32,
        reduced_motion: bool,
    ) -> Option<i32> {
        let baseline = match self.motion {
            Motion::Held { origin, row_height } => Some((origin, row_height)),
            _ if row_height.is_finite() && row_height > 0. => {
                Some((self.offset, f64::from(row_height)))
            }
            _ => None,
        };
        let position = |delta_y| {
            let (origin, row_height) = baseline?;
            let offset = self.drag_offset(origin, row_height, delta_y)?;
            Some((origin, row_height, offset))
        };
        match sample {
            ScrollFieldDrag::Move { delta_y } => {
                if let Some((origin, row_height, offset)) = position(delta_y) {
                    self.offset = offset;
                    self.motion = Motion::Held { origin, row_height };
                }
                None
            }
            ScrollFieldDrag::End { delta_y } => {
                if let Some((_, _, offset)) = position(delta_y) {
                    self.offset = offset;
                    let target = offset.round();
                    self.motion = Motion::Idle;
                    self.accept_at(self.preview(), target, reduced_motion)
                } else {
                    self.cancel_drag(reduced_motion);
                    None
                }
            }
            ScrollFieldDrag::Tap { value } if !self.is_dragging() && self.range.contains(value) => {
                self.accept(value, reduced_motion)
            }
            ScrollFieldDrag::Tap { .. } | ScrollFieldDrag::Cancel => {
                self.cancel_drag(reduced_motion);
                None
            }
        }
    }

    fn drag_offset(&self, origin: f64, row_height: f64, delta_y: f32) -> Option<f64> {
        if !delta_y.is_finite() {
            return None;
        }
        let offset = origin + f64::from(delta_y) / row_height;
        if self.looping {
            (offset.abs() <= MAX_DRAG_PHASE).then_some(offset)
        } else {
            Some(self.range.clamp_offset(offset))
        }
    }

    fn cancel_drag(&mut self, reduced_motion: bool) {
        self.session = PointDragSession::new();
        if self.is_dragging() {
            self.settle_to(self.nearest_phase(self.value), reduced_motion);
        }
    }

    fn accept(&mut self, value: i32, reduced_motion: bool) -> Option<i32> {
        self.accept_at(value, self.nearest_phase(value), reduced_motion)
    }

    fn accept_at(&mut self, value: i32, target: f64, reduced_motion: bool) -> Option<i32> {
        let changed = value != self.value;
        self.value = value;
        self.restore_raw();
        self.session = PointDragSession::new();
        self.settle_to(target, reduced_motion);
        changed.then_some(value)
    }

    fn period(&self) -> i64 {
        self.range.index(self.range.max) + 1
    }

    fn nearest_phase(&self, value: i32) -> f64 {
        let index = self.range.index(value) as f64;
        if self.looping {
            let period = self.period() as f64;
            index + ((self.offset - index) / period).round() * period
        } else {
            index
        }
    }

    fn rest_offset(&self) -> f64 {
        self.range.index(self.value) as f64
    }

    fn reset_motion(&mut self) {
        self.finish_settle();
        self.session = PointDragSession::new();
    }

    fn finish_settle(&mut self) {
        self.offset = self.rest_offset();
        self.motion = Motion::Idle;
    }

    fn settle_to(&mut self, target: f64, reduced_motion: bool) {
        if reduced_motion {
            self.finish_settle();
            return;
        }
        self.motion = match self.motion {
            Motion::Settling {
                spring, last_ms, ..
            } => Motion::Settling {
                target,
                spring,
                last_ms,
            },
            _ if self.offset != target => Motion::Settling {
                target,
                spring: Spring::new(self.offset),
                last_ms: None,
            },
            _ => {
                self.finish_settle();
                return;
            }
        };
    }

    fn restore_raw(&mut self) {
        self.raw = self.format_value(self.value);
        self.error = None;
    }

    fn parse(&self) -> Result<i32, ScrollFieldError> {
        let text = self.raw.trim();
        if text.is_empty() {
            return Err(ScrollFieldError::Empty);
        }
        let (text, negative) = if let Some(magnitude) = text.strip_prefix(['-', '−', '－']) {
            (magnitude, true)
        } else {
            (text.strip_prefix(['+', '＋']).unwrap_or(text), false)
        };
        if text.is_empty() {
            return Err(ScrollFieldError::InvalidNumber);
        }
        let mut magnitude = Some(0_i64);
        for ch in text.chars() {
            let digit = standard_digit(ch)
                .or_else(|| {
                    self.digits
                        .iter()
                        .position(|digit| *digit == ch)
                        .map(|i| i as u8)
                })
                .ok_or(ScrollFieldError::InvalidNumber)?;
            magnitude =
                magnitude.and_then(|amount| amount.checked_mul(10)?.checked_add(i64::from(digit)));
        }
        let magnitude = magnitude.ok_or(ScrollFieldError::OutOfRange)?;
        let value = i32::try_from(if negative { -magnitude } else { magnitude })
            .map_err(|_| ScrollFieldError::OutOfRange)?;
        if !(self.range.min..=self.range.max).contains(&value) {
            return Err(ScrollFieldError::OutOfRange);
        }
        if !self.range.contains(value) {
            return Err(ScrollFieldError::OffStep);
        }
        Ok(value)
    }
}

fn standard_digit(ch: char) -> Option<u8> {
    match ch {
        '0'..='9' => Some((ch as u32 - '0' as u32) as u8),
        '０'..='９' => Some((ch as u32 - '０' as u32) as u8),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ScrollFieldState {
        ScrollFieldState::new(ScrollFieldRange::new(10, 100, 10).unwrap(), 30).unwrap()
    }

    fn drag(
        session: &PointDragSession,
        sample: ScrollFieldDrag,
        row_height: f32,
    ) -> ScrollFieldEvent {
        ScrollFieldEvent::Drag {
            session: session.clone(),
            sample,
            row_height,
        }
    }

    fn input(state: &mut ScrollFieldState, raw: &str) {
        assert_eq!(
            state.apply(ScrollFieldEvent::SetText(raw.into()), false),
            None
        );
    }

    #[test]
    fn range_checks_bounds_step_and_extreme_indices() {
        assert!(ScrollFieldRange::new(-50, 50, -5).is_none());
        assert!(ScrollFieldRange::new(-50, 50, 0).is_none());
        assert!(ScrollFieldRange::new(18, 18, 1).is_none());
        assert!(ScrollFieldRange::new(30, 10, 10).is_none());
        assert!(ScrollFieldRange::new(-50, 49, 5).is_none());
        let range = ScrollFieldRange::new(i32::MIN, i32::MAX, 1).unwrap();
        assert!(range.contains(i32::MAX));
        assert!(range.contains(i32::MIN));
        let mut field = ScrollFieldState::new(range, i32::MIN).unwrap();
        assert_eq!(field.offset(), 0.);
        assert_eq!(field.preview(), i32::MIN);
        assert_eq!(
            field.apply(ScrollFieldEvent::Step(ScrollFieldStep::Decrease), true),
            None
        );
        assert_eq!(
            field.apply(ScrollFieldEvent::Step(ScrollFieldStep::Last), true),
            Some(i32::MAX)
        );
        assert_eq!(field.offset(), 4_294_967_295.);
        assert_eq!(field.preview(), i32::MAX);
        assert_eq!(
            field.apply(ScrollFieldEvent::Step(ScrollFieldStep::Increase), true),
            None
        );
        assert_eq!(
            field.apply(ScrollFieldEvent::Step(ScrollFieldStep::Decrease), true),
            Some(i32::MAX - 1)
        );
        assert_eq!(range.value_at(-1), None);
        assert_eq!(range.value_at(4_294_967_296), None);
        assert!(ScrollFieldState::new(ScrollFieldRange::new(0, 90, 10).unwrap(), 25).is_none());
    }

    #[test]
    fn editor_rejects_bad_drafts_and_commits_reachable_values_once() {
        let mut field = state();
        field.apply(ScrollFieldEvent::Edit, false);
        for (text, error) in [
            ("   ", ScrollFieldError::Empty),
            ("4x", ScrollFieldError::InvalidNumber),
            ("-", ScrollFieldError::InvalidNumber),
            ("2147483648", ScrollFieldError::OutOfRange),
            ("999999999999999999999999999", ScrollFieldError::OutOfRange),
            ("-20", ScrollFieldError::OutOfRange),
            ("75", ScrollFieldError::OffStep),
        ] {
            input(&mut field, text);
            assert_eq!(field.apply(ScrollFieldEvent::Submit, false), None);
            assert_eq!(field.mode(), ScrollFieldMode::Input);
            assert_eq!(field.raw(), text);
            assert_eq!(field.error(), Some(error));
            assert_eq!(field.value(), 30);
        }
        input(&mut field, " ＋５０ ");
        assert_eq!(field.error(), None);
        assert_eq!(field.apply(ScrollFieldEvent::Submit, false), Some(50));
        assert_eq!(
            (field.raw(), field.mode(), field.offset()),
            ("50", ScrollFieldMode::Scroll, 4.)
        );
        assert_eq!(field.apply(ScrollFieldEvent::Submit, false), None);
        field.apply(ScrollFieldEvent::Edit, false);
        assert_eq!(field.apply(ScrollFieldEvent::Submit, false), None);
        assert_eq!(field.mode(), ScrollFieldMode::Scroll);
    }

    #[test]
    fn localized_digits_display_and_parse_signed_values() {
        let locale = ['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩'];
        let range = ScrollFieldRange::new(i32::MIN, i32::MAX, 1).unwrap();
        let mut field = ScrollFieldState::new(range, -50)
            .unwrap()
            .with_digits(locale)
            .unwrap();
        assert_eq!(field.raw(), "-٥٠");
        field.apply(ScrollFieldEvent::Edit, false);
        input(&mut field, "−٢١４７４۸");
        assert_eq!(field.apply(ScrollFieldEvent::Submit, false), None);
        assert_eq!(field.error(), Some(ScrollFieldError::InvalidNumber));
        input(&mut field, "－٢١４７４８３٦٤٨");
        assert_eq!(field.apply(ScrollFieldEvent::Submit, false), Some(i32::MIN));
        assert_eq!(field.raw(), "-٢١٤٧٤٨٣٦٤٨");
        let mut invalid = locale;
        invalid[2] = '１';
        assert!(state().with_digits(invalid).is_none());
        invalid[2] = '−';
        assert!(state().with_digits(invalid).is_none());
        invalid[2] = locale[0];
        assert!(state().with_digits(invalid).is_none());
    }

    #[test]
    fn fallback_is_an_editor_and_preserves_a_draft_after_resize() {
        let mut field = state();
        assert!(field.update_fit(false));
        assert_eq!(field.mode(), ScrollFieldMode::Input);
        assert!(!field.update_fit(false));
        input(&mut field, "42");
        field.apply(ScrollFieldEvent::Submit, false);
        assert!(field.update_fit(true));
        assert_eq!(field.mode(), ScrollFieldMode::Input);
        assert_eq!(field.raw(), "42");
        assert_eq!(field.error(), Some(ScrollFieldError::OffStep));
        assert_eq!(field.apply(ScrollFieldEvent::Cancel, false), None);
        assert_eq!((field.value(), field.raw()), (30, "30"));
        assert_eq!(field.mode(), ScrollFieldMode::Scroll);

        field.update_fit(false);
        input(&mut field, "80");
        assert_eq!(field.apply(ScrollFieldEvent::Submit, false), Some(80));
        assert_eq!(field.mode(), ScrollFieldMode::Input);
        assert!(field.update_fit(true));
        assert_eq!(field.mode(), ScrollFieldMode::Input);
        assert_eq!(field.raw(), "80");
        assert_eq!(field.apply(ScrollFieldEvent::Cancel, false), None);
        assert_eq!(field.mode(), ScrollFieldMode::Scroll);
        input(&mut field, "60");
        assert_eq!(field.raw(), "80", "old editor messages are ignored");
    }

    #[test]
    fn held_preview_uses_its_down_time_pitch_and_accepts_the_terminal_position() {
        let mut field = state();
        let session = field.drag_session();
        let moving = drag(&session, ScrollFieldDrag::Move { delta_y: 36. }, 48.);
        assert_eq!(field.apply(moving, false), None);
        assert!(field.is_dragging());
        assert_eq!(
            (field.value(), field.preview(), field.offset()),
            (30, 40, 2.75)
        );
        assert!(!field.advance(Some(100.), false).request_next);
        let ending = drag(&session, ScrollFieldDrag::End { delta_y: 83. }, 100.);
        assert_eq!(field.apply(ending.clone(), false), Some(50));
        assert!(field.settling());
        assert_eq!(field.value(), 50);
        assert_eq!(field.apply(ending, false), None);
        assert_ne!(field.drag_session(), session);
        assert!(!field.advance(None, false).request_next);
        assert_eq!(field.offset(), 4.);

        let session = field.drag_session();
        let release_only = drag(&session, ScrollFieldDrag::End { delta_y: -66. }, 48.);
        assert_eq!(field.apply(release_only, true), Some(40));
        assert_eq!((field.value(), field.offset()), (40, 3.));
    }

    #[test]
    fn cancel_invalid_terminal_and_external_changes_retire_the_capture() {
        let mut field = state();
        let session = field.drag_session();
        field.apply(
            drag(&session, ScrollFieldDrag::Move { delta_y: 80. }, 48.),
            false,
        );
        field.apply(drag(&session, ScrollFieldDrag::Cancel, 48.), false);
        assert_eq!(field.value(), 30);
        assert!(field.settling());
        field.advance(None, false);
        let stale = drag(&session, ScrollFieldDrag::End { delta_y: 180. }, 48.);
        assert_eq!(field.apply(stale, true), None);
        let session = field.drag_session();
        field.apply(
            drag(&session, ScrollFieldDrag::Move { delta_y: 80. }, 48.),
            false,
        );
        let invalid = drag(&session, ScrollFieldDrag::End { delta_y: f32::NAN }, 48.);
        assert_eq!(field.apply(invalid, true), None);
        assert_eq!((field.value(), field.offset()), (30, 2.));

        let session = field.drag_session();
        let stale = drag(&session, ScrollFieldDrag::End { delta_y: 180. }, 48.);
        assert!(field.invalidate_layout());
        assert_eq!(field.apply(stale, true), None);
        let session = field.drag_session();
        assert!(!field.sync_value(45));
        assert!(field.sync_value(60));
        assert_eq!(
            field.apply(
                drag(&session, ScrollFieldDrag::Tap { value: 70 }, 48.),
                true
            ),
            None
        );
        assert_eq!((field.value(), field.preview()), (60, 60));
    }

    #[test]
    fn expressive_spring_overshoots_and_reduced_or_missing_time_snaps() {
        let mut field = state();
        assert_eq!(field.apply(ScrollFieldEvent::SetValue(50), false), Some(50));
        assert_eq!((field.value(), field.preview()), (50, 30));
        assert!(field.advance(Some(1000.), false).request_next);
        let frame = field.advance(Some(1050.), false);
        assert!(frame.rebuild && frame.request_next);
        assert!((field.offset() - 3.054734999).abs() < 0.00001);
        field.advance(Some(1140.), false);
        assert!((field.offset() - 4.189459771).abs() < 0.00001);
        let last = field.offset();
        assert!(!field.advance(Some(1100.), false).rebuild);
        assert_eq!(field.offset(), last);
        let snap = field.advance(Some(1150.), true);
        assert!(snap.rebuild && !snap.request_next);
        assert_eq!(field.offset(), 4.);

        assert_eq!(field.apply(ScrollFieldEvent::SetValue(90), false), Some(90));
        let no_clock = field.advance(None, false);
        assert!(no_clock.rebuild && !no_clock.request_next);
        assert_eq!(field.offset(), 8.);
        assert_eq!(
            field.apply(ScrollFieldEvent::Step(ScrollFieldStep::First), true),
            Some(10)
        );
        assert!(!field.settling());
        assert_eq!(field.offset(), 0.);
    }

    #[test]
    fn neighboring_taps_stay_bounded_and_do_not_loop() {
        let mut field = state();
        assert_eq!(
            field.apply(ScrollFieldEvent::SetValue(100), true),
            Some(100)
        );
        let session = field.drag_session();
        assert_eq!(
            field.apply(
                drag(&session, ScrollFieldDrag::Tap { value: 110 }, 48.),
                true
            ),
            None
        );
        let session = field.drag_session();
        assert_eq!(
            field.apply(
                drag(&session, ScrollFieldDrag::Tap { value: 90 }, 48.),
                true
            ),
            Some(90)
        );
        assert_eq!(
            field.apply(
                drag(&session, ScrollFieldDrag::Tap { value: 80 }, 48.),
                true
            ),
            None
        );
        assert_eq!((field.value(), field.preview()), (90, 90));
    }

    #[test]
    fn looping_steps_travel_in_order_and_rebase_at_rest() {
        let mut field = ScrollFieldState::new(ScrollFieldRange::new(10, 40, 10).unwrap(), 40)
            .unwrap()
            .with_looping();
        assert!(field.looping());
        assert_eq!(field.value_at(4), Some(10));
        assert_eq!(field.value_at(-1), Some(40));
        assert_eq!(
            field.apply(ScrollFieldEvent::Step(ScrollFieldStep::Increase), false),
            Some(10)
        );
        assert_eq!(field.offset(), 3.);
        assert!(field.advance(Some(1000.), false).request_next);
        assert!(field.advance(Some(1050.), false).rebuild);
        assert!((3.5..4.).contains(&field.offset()));
        assert_eq!(field.preview(), 10);
        assert!(!field.advance(Some(3050.), false).request_next);
        assert_eq!(field.offset(), 0.);

        assert_eq!(
            field.apply(ScrollFieldEvent::Step(ScrollFieldStep::Decrease), false),
            Some(40)
        );
        field.advance(Some(4000.), false);
        field.advance(Some(4050.), false);
        assert!((-1. ..-0.5).contains(&field.offset()));
        assert_eq!(field.preview(), 40);
        assert!(!field.advance(None, false).request_next);
        assert_eq!(field.offset(), 3.);
    }

    #[test]
    fn two_item_loop_keeps_both_step_directions_and_pending_targets() {
        for (step, sign) in [
            (ScrollFieldStep::Increase, 1.),
            (ScrollFieldStep::Decrease, -1.),
        ] {
            let range = ScrollFieldRange::new(10, 20, 10).unwrap();
            let mut field = ScrollFieldState::new(range, 10).unwrap().with_looping();
            assert_eq!(field.value_at(-1), field.value_at(1));
            assert_eq!(field.apply(ScrollFieldEvent::Step(step), false), Some(20));
            assert_eq!(field.apply(ScrollFieldEvent::Step(step), false), Some(10));
            field.advance(Some(500.), false);
            field.advance(Some(550.), false);
            assert!((1. ..2.).contains(&(field.offset() * sign)));
            assert_eq!(
                field.preview(),
                20,
                "the other item remains in the signed path"
            );
            assert!(field.advance(Some(555.), true).rebuild);
            assert_eq!(
                (field.value(), field.offset(), field.settling()),
                (10, 0., false)
            );
        }
    }

    #[test]
    fn looping_drag_uses_signed_phase_across_laps_and_accepts_only_release() {
        let mut field = ScrollFieldState::new(ScrollFieldRange::new(10, 40, 10).unwrap(), 20)
            .unwrap()
            .with_looping();
        let capture = field.drag_session();
        let move_sample = drag(&capture, ScrollFieldDrag::Move { delta_y: 400. }, 48.);
        assert_eq!(field.apply(move_sample, false), None);
        assert_eq!(field.value(), 20);
        assert_eq!(
            field.preview(),
            20,
            "two laps do not change the center value"
        );
        assert!((9. ..10.).contains(&field.offset()));

        let end = drag(&capture, ScrollFieldDrag::End { delta_y: 425. }, 200.);
        assert_eq!(field.apply(end.clone(), false), Some(30));
        assert_eq!(field.apply(end, false), None);
        assert!(field.settling());
        assert!((9. ..10.).contains(&field.offset()));
        field.advance(Some(600.), false);
        field.advance(Some(650.), false);
        assert!((9. ..10.).contains(&field.offset()));
        assert_eq!(field.preview(), 30);
        assert!(!field.advance(None, false).request_next);
        assert_eq!(field.offset(), 2.);

        let capture = field.drag_session();
        let release_only = drag(&capture, ScrollFieldDrag::End { delta_y: -211. }, 48.);
        assert_eq!(field.apply(release_only, true), None);
        assert_eq!((field.value(), field.offset()), (30, 2.));
        let capture = field.drag_session();
        let release_only = drag(&capture, ScrollFieldDrag::End { delta_y: -250. }, 48.);
        assert_eq!(field.apply(release_only, true), Some(20));
        assert_eq!((field.value(), field.offset()), (20, 1.));
    }

    #[test]
    fn looping_cancel_returns_to_a_nearby_equivalent_phase_and_retires_capture() {
        let mut field = ScrollFieldState::new(ScrollFieldRange::new(10, 40, 10).unwrap(), 30)
            .unwrap()
            .with_looping();
        let capture = field.drag_session();
        let preview = drag(&capture, ScrollFieldDrag::Move { delta_y: -450. }, 48.);
        assert_eq!(field.apply(preview, false), None);
        assert_eq!(field.offset(), -7.375);
        assert_eq!(field.value(), 30);
        assert_eq!(field.preview(), 20);
        assert_eq!(
            field.apply(drag(&capture, ScrollFieldDrag::Cancel, 48.), false),
            None
        );
        field.advance(Some(250.), false);
        field.advance(Some(300.), false);
        assert!((-7. ..-6.).contains(&field.offset()));
        assert_ne!(field.drag_session(), capture);
        assert_eq!(
            field.apply(
                drag(&capture, ScrollFieldDrag::End { delta_y: 14. }, 48.),
                true
            ),
            None
        );
        assert!(!field.advance(Some(310.), true).request_next);
        assert_eq!((field.value(), field.offset()), (30, 2.));
    }

    #[test]
    fn looping_rejects_imprecise_motion_and_preserves_authored_input_bounds() {
        let range = ScrollFieldRange::new(i32::MIN, i32::MAX, 1).unwrap();
        let mut extremes = ScrollFieldState::new(range, i32::MAX)
            .unwrap()
            .with_looping();
        assert_eq!(extremes.value_at(i64::MIN), Some(i32::MIN));
        assert_eq!(extremes.value_at(i64::MAX), Some(i32::MAX));
        assert_eq!(
            extremes.apply(ScrollFieldEvent::Step(ScrollFieldStep::Increase), true),
            Some(i32::MIN)
        );
        assert_eq!(extremes.offset(), 0.);

        let mut field = state().with_looping();
        let capture = field.drag_session();
        let invalid = drag(&capture, ScrollFieldDrag::Move { delta_y: f32::MAX }, 48.);
        assert_eq!(field.apply(invalid, false), None);
        assert!(!field.is_dragging());
        assert_eq!(field.offset(), 2.);
        field.apply(
            drag(&capture, ScrollFieldDrag::Move { delta_y: 35. }, 48.),
            false,
        );
        assert_eq!(
            field.apply(
                drag(&capture, ScrollFieldDrag::End { delta_y: -f32::MAX }, 48.),
                true
            ),
            None
        );
        assert_eq!((field.value(), field.offset()), (30, 2.));
        assert_eq!(field.apply(ScrollFieldEvent::SetValue(0), true), None);
        field.apply(ScrollFieldEvent::Edit, false);
        input(&mut field, "110");
        assert_eq!(field.apply(ScrollFieldEvent::Submit, false), None);
        assert_eq!(field.error(), Some(ScrollFieldError::OutOfRange));
        field.apply(ScrollFieldEvent::Cancel, false);
        assert_eq!(
            field.apply(ScrollFieldEvent::Step(ScrollFieldStep::First), true),
            Some(10)
        );
        assert_eq!(
            field.apply(ScrollFieldEvent::Step(ScrollFieldStep::Last), true),
            Some(100)
        );
    }
}
