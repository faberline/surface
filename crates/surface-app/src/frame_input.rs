use crate::press_ripple::PressRippleContact;
use crate::{AnimationUpdate, AppError, Application, Event, Runtime};
use surface::Rect;

/// A held primary touch contact in logical window coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchContact {
    pub origin: (f32, f32),
    pub position: (f32, f32),
    /// The monotonic host clock at the press, if the host set one.
    pub started_ms: Option<f64>,
}

/// The latest input available before an animation frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameInput<'a> {
    /// Mouse position in logical window coordinates, including passive content.
    /// A touch contact uses `touch` instead.
    pub pointer: Option<(f32, f32)>,
    /// The stable ID of the current focus target, if any.
    pub focused: Option<&'a str>,
    pub touch: Option<TouchContact>,
    /// A held contact was cancelled since the previous frame. A normal
    /// release does not set this flag.
    pub touch_cancelled: bool,
}

/// Stage of a primary pointer press accepted by the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramePressPhase {
    /// The primary pointer is still held on this accepted sequence.
    Held,
    /// The matching release completed the primary pointer interaction.
    Released,
    /// The press lost its target or ended without completing the interaction.
    Cancelled,
}

/// The last accepted mouse or touch press. Positions and bounds are the
/// logical window geometry at pointer down, even if the pointer has moved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FramePress<'a> {
    pub id: &'a str,
    pub position: (f32, f32),
    pub bounds: Rect,
    /// The host's last supplied time when the press was accepted.
    pub started_ms: Option<f64>,
    /// Distinguishes successive presses, including on the same stable ID.
    pub sequence: u64,
    pub phase: FramePressPhase,
}

#[derive(Default)]
pub(crate) struct PressObservation {
    sequence: u64,
    current: Option<ObservedPress>,
}

struct ObservedPress {
    id: String,
    position: (f32, f32),
    bounds: Rect,
    started_ms: Option<f64>,
    phase: FramePressPhase,
    application: bool,
    ripple: Option<PressRippleContact>,
}

impl ObservedPress {
    fn frame(&self, sequence: u64) -> FramePress<'_> {
        FramePress {
            id: &self.id,
            position: self.position,
            bounds: self.bounds,
            started_ms: self.started_ms,
            sequence,
            phase: self.phase,
        }
    }
}

impl PressObservation {
    pub(crate) fn clear_application(&mut self) {
        if let Some(press) = &mut self.current {
            press.application = false;
        }
        self.discard_if_unused();
    }

    pub(crate) fn clear_ripple(&mut self) {
        if let Some(press) = &mut self.current {
            press.ripple = None;
        }
        self.discard_if_unused();
    }

    fn discard_if_unused(&mut self) {
        if self
            .current
            .as_ref()
            .is_some_and(|p| !p.application && p.ripple.is_none())
        {
            self.current = None;
        }
    }

    pub(crate) fn accept(
        &mut self,
        id: &str,
        position: (f32, f32),
        bounds: Rect,
        started_ms: Option<f64>,
        application: bool,
        ripple: Option<PressRippleContact>,
    ) {
        self.sequence = self.sequence.wrapping_add(1);
        self.current = Some(ObservedPress {
            id: id.to_owned(),
            position,
            bounds,
            started_ms,
            phase: FramePressPhase::Held,
            application,
            ripple,
        });
    }

    pub(crate) fn release(&mut self) {
        if let Some(press) = self
            .current
            .as_mut()
            .filter(|press| press.phase == FramePressPhase::Held)
        {
            press.phase = FramePressPhase::Released;
        }
    }

    fn observe(&mut self, pressed: Option<&str>) -> Option<&ObservedPress> {
        let press = self.current.as_mut()?;
        if press.phase == FramePressPhase::Held && pressed != Some(press.id.as_str()) {
            press.phase = FramePressPhase::Cancelled;
        }
        Some(press)
    }

    pub(crate) fn application_frame(&mut self, pressed: Option<&str>) -> Option<FramePress<'_>> {
        self.current.as_ref().filter(|press| press.application)?;
        let sequence = self.sequence;
        Some(self.observe(pressed)?.frame(sequence))
    }

    pub(crate) fn ripple_frame(
        &mut self,
        pressed: Option<&str>,
    ) -> Option<(FramePress<'_>, &PressRippleContact)> {
        self.current.as_ref()?.ripple.as_ref()?;
        let sequence = self.sequence;
        let press = self.observe(pressed)?;
        Some((press.frame(sequence), press.ripple.as_ref()?))
    }

    pub(crate) fn ripple_owner(&self) -> Option<(&str, &PressRippleContact)> {
        let press = self.current.as_ref()?;
        Some((&press.id, press.ripple.as_ref()?))
    }

    pub(crate) fn finish_application_frame(&mut self) {
        if self.is_terminal() {
            self.clear_application();
        }
    }

    pub(crate) fn finish_ripple_frame(&mut self) {
        if self.is_terminal() {
            self.clear_ripple();
        }
    }

    fn is_terminal(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(|press| press.phase != FramePressPhase::Held)
    }
}

/// What the application needs after observing frame input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputAnimationUpdate {
    pub animation: AnimationUpdate,
    /// Prevent a tap on release of the current touch, for example after a
    /// long press. The runtime keeps this decision until that contact ends.
    pub suppress_touch_activation: bool,
}

pub(crate) struct ActiveTouch {
    pub(crate) contact: TouchContact,
    pub(crate) suppress_activation: bool,
}

impl<A: Application> Runtime<A> {
    /// Dispatch a primary touch through the normal pointer controls. Pass
    /// `PointerDown`, `PointerMove`, or `PointerUp` in logical coordinates.
    /// The host chooses one contact and calls [`Self::cancel_pointer`] if it
    /// is interrupted. Other events and moves or releases without a matching
    /// press are ignored.
    pub fn dispatch_touch(&mut self, event: Event) -> Result<bool, AppError> {
        self.dispatch_with(event, Self::dispatch_contact_event)
    }

    pub(crate) fn cancel_touch_contact(&mut self) -> bool {
        let changed = self.touch.take().is_some();
        self.touch_cancelled |= changed;
        changed
    }

    pub(crate) fn dispatch_observed_event(&mut self, event: Event) -> Result<bool, AppError> {
        let mut cancelled = false;
        let pointer_changed = match event {
            Event::PointerDown { x, y }
            | Event::PointerMove { x, y }
            | Event::PointerUp { x, y }
            | Event::SecondaryDown { x, y } => {
                if self.touch.is_some() {
                    cancelled = self.cancel_pointer()?;
                }
                let next = (x.is_finite() && y.is_finite()).then_some((x, y));
                std::mem::replace(&mut self.pointer_position, next) != next
            }
            Event::PointerLeave | Event::Blur => self.pointer_position.take().is_some(),
            _ => false,
        };
        let changed = self.dispatch_event(event, false)?;
        Ok(changed || cancelled || (pointer_changed && self.app.observes_frame_input()))
    }

    fn dispatch_contact_event(&mut self, event: Event) -> Result<bool, AppError> {
        let (x, y) = match event {
            Event::PointerDown { x, y }
            | Event::PointerMove { x, y }
            | Event::PointerUp { x, y } => (x, y),
            _ => return Ok(false),
        };
        if !x.is_finite() || !y.is_finite() {
            return if matches!(event, Event::PointerUp { .. }) && self.touch.is_some() {
                self.cancel_pointer()
            } else {
                Ok(false)
            };
        }
        let point = (x, y);
        let (changed, observed) = match event {
            Event::PointerDown { .. } => {
                if self.touch.is_some() {
                    self.cancel_pointer()?;
                }
                self.pointer_position = None;
                let changed = self.dispatch_event(event, false)?;
                self.touch = Some(ActiveTouch {
                    contact: TouchContact {
                        origin: point,
                        position: point,
                        started_ms: self.input_time_ms(),
                    },
                    suppress_activation: false,
                });
                (changed, true)
            }
            Event::PointerMove { .. } => {
                let Some(touch) = &mut self.touch else {
                    return Ok(false);
                };
                let observed = std::mem::replace(&mut touch.contact.position, point) != point;
                (self.dispatch_event(event, false)?, observed)
            }
            Event::PointerUp { .. } => {
                let Some(touch) = self.touch.take() else {
                    return Ok(false);
                };
                let changed = self.dispatch_event(event, touch.suppress_activation)?;
                (changed, true)
            }
            _ => unreachable!("other events cannot supply touch coordinates"),
        };
        Ok(changed || (observed && self.app.observes_frame_input()))
    }
}
