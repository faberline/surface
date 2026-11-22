use crate::{AnimationUpdate, AppError, Application, Event, Runtime};

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
