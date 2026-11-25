//! Match browser capture loss to the contact that started an opted-in drag.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Pointer {
    MouseLike(i32),
    Touch(i32),
}

impl Pointer {
    pub(crate) fn touch_id(self) -> Option<u64> {
        match self {
            Self::MouseLike(_) => None,
            Self::Touch(id) => Some(id as u64),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DragOwner {
    Point,
    SliderPair,
    TouchScroll,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Contact {
    pub(crate) pointer: Pointer,
    pub(crate) owner: DragOwner,
}

#[derive(Default)]
struct Capture {
    down: Option<Pointer>,
    held: Option<Contact>,
    lost: Option<Contact>,
}

impl Capture {
    fn pointer_down(&mut self, pointer: Pointer) {
        self.down = Some(pointer);
    }

    /// The capture-phase listener sees the DOM ID first. Winit then tells the
    /// host whether that down reached the runtime and began a tracked drag.
    fn accept_down(&mut self, touch: Option<u64>, owner: Option<DragOwner>) {
        let down = self
            .down
            .take()
            .filter(|pointer| pointer.touch_id() == touch);
        self.held = down
            .zip(owner)
            .map(|(pointer, owner)| Contact { pointer, owner });
    }

    fn pointer_end(&mut self, pointer: Pointer) {
        if self.held.is_some_and(|contact| contact.pointer == pointer) {
            self.held = None;
        }
        if self.down == Some(pointer) {
            self.down = None;
        }
    }

    fn mouse_up(&mut self) {
        // Winit can report the primary button up during a multi-button chord,
        // before the browser sends pointerup for the last released button.
        if let Some(Contact {
            pointer: pointer @ Pointer::MouseLike(_),
            ..
        }) = self.held
        {
            self.pointer_end(pointer);
        }
    }

    fn capture_lost(&mut self, pointer: Pointer) -> bool {
        if self.held.is_none_or(|contact| contact.pointer != pointer) {
            return false;
        }
        self.lost = self.held.take();
        true
    }

    fn take_loss(&mut self) -> Option<Contact> {
        self.lost.take()
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use browser::WebPointerCapture;

#[cfg(target_arch = "wasm32")]
mod browser {
    use super::{Capture, Contact, DragOwner, Pointer};
    use crate::UserEvent;
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::{closure::Closure, JsCast, JsValue};
    use web_sys::{AddEventListenerOptions, EventTarget, HtmlCanvasElement, PointerEvent};
    use winit::event_loop::EventLoopProxy;

    pub(crate) struct WebPointerCapture {
        state: Rc<RefCell<Capture>>,
        _listeners: Vec<Listener>,
    }

    impl WebPointerCapture {
        pub(crate) fn new(
            canvas: HtmlCanvasElement,
            proxy: EventLoopProxy<UserEvent>,
        ) -> Result<Self, JsValue> {
            let document: EventTarget = canvas
                .owner_document()
                .ok_or_else(|| JsValue::from_str("canvas has no document"))?
                .into();
            let canvas: EventTarget = canvas.into();
            let state = Rc::new(RefCell::new(Capture::default()));
            let down = {
                let state = state.clone();
                Listener::new(&canvas, "pointerdown", move |event, pointer| {
                    if event.button() == 0 {
                        state.borrow_mut().pointer_down(pointer);
                    }
                })?
            };
            let mut listeners = vec![down];
            for name in ["pointerup", "pointercancel"] {
                let state = state.clone();
                listeners.push(Listener::new(&document, name, move |_, pointer| {
                    state.borrow_mut().pointer_end(pointer);
                })?);
            }
            let loss = {
                let state = state.clone();
                Listener::new(&canvas, "lostpointercapture", move |_, pointer| {
                    let changed = state.borrow_mut().capture_lost(pointer);
                    if changed {
                        // Store the loss now. The next winit input may be
                        // dispatched before the asynchronous user event.
                        let _ = proxy.send_event(UserEvent::WebPointerCaptureLost);
                    }
                })?
            };
            listeners.push(loss);
            Ok(Self {
                state,
                _listeners: listeners,
            })
        }

        pub(crate) fn accept_down(&self, touch: Option<u64>, owner: Option<DragOwner>) {
            self.state.borrow_mut().accept_down(touch, owner);
        }

        pub(crate) fn mouse_up(&self) {
            self.state.borrow_mut().mouse_up();
        }

        pub(crate) fn take_loss(&self) -> Option<Contact> {
            self.state.borrow_mut().take_loss()
        }

        pub(crate) fn reset(&self) {
            *self.state.borrow_mut() = Capture::default();
        }
    }

    struct Listener {
        target: EventTarget,
        name: &'static str,
        handler: Closure<dyn FnMut(PointerEvent)>,
    }

    impl Listener {
        fn new(
            target: &EventTarget,
            name: &'static str,
            mut observe: impl FnMut(PointerEvent, Pointer) + 'static,
        ) -> Result<Self, JsValue> {
            let handler = Closure::new(move |event: PointerEvent| {
                if !event.is_trusted() {
                    return;
                }
                let id = event.pointer_id();
                let pointer = match event.pointer_type().as_str() {
                    "touch" => Pointer::Touch(id),
                    "mouse" | "pen" => Pointer::MouseLike(id),
                    _ => return,
                };
                observe(event, pointer);
            });
            let options = AddEventListenerOptions::new();
            options.set_capture(true);
            options.set_passive(true);
            target.add_event_listener_with_callback_and_add_event_listener_options(
                name,
                handler.as_ref().unchecked_ref(),
                &options,
            )?;
            Ok(Self {
                target: target.clone(),
                name,
                handler,
            })
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            let _ = self.target.remove_event_listener_with_callback_and_bool(
                self.name,
                self.handler.as_ref().unchecked_ref(),
                true,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Capture, Contact, DragOwner, Pointer};

    #[test]
    fn normal_up_or_cancel_clears_the_contact_before_loss() {
        let mut capture = Capture::default();
        for owner in [DragOwner::Point, DragOwner::SliderPair] {
            for pointer in [Pointer::MouseLike(1), Pointer::Touch(2)] {
                capture.pointer_down(pointer);
                capture.accept_down(pointer.touch_id(), Some(owner));
                capture.pointer_end(pointer);
                assert!(!capture.capture_lost(pointer));
                assert_eq!(capture.take_loss(), None);
            }
        }
    }

    #[test]
    fn touch_scroll_arms_only_the_matched_touch_and_delivers_loss_once() {
        let mut capture = Capture::default();
        let pointer = Pointer::Touch(7);
        capture.pointer_down(pointer);
        capture.accept_down(Some(8), Some(DragOwner::TouchScroll));
        assert!(!capture.capture_lost(pointer));
        capture.pointer_down(pointer);
        capture.accept_down(Some(7), Some(DragOwner::TouchScroll));
        assert!(!capture.capture_lost(Pointer::MouseLike(7)));
        assert!(capture.capture_lost(pointer));
        assert_eq!(
            capture.take_loss(),
            Some(Contact {
                pointer,
                owner: DragOwner::TouchScroll
            }),
        );
        assert_eq!(capture.take_loss(), None);
    }

    #[test]
    fn explicit_loss_is_retained_before_late_up_and_drains_once() {
        let mut capture = Capture::default();
        let mouse = Pointer::MouseLike(1);
        capture.pointer_down(mouse);
        capture.accept_down(None, Some(DragOwner::Point));
        assert!(capture.capture_lost(mouse));
        capture.pointer_end(mouse);
        assert!(!capture.capture_lost(mouse));
        assert_eq!(
            capture.take_loss(),
            Some(Contact {
                pointer: mouse,
                owner: DragOwner::Point
            })
        );
        assert_eq!(capture.take_loss(), None);
    }

    #[test]
    fn foreign_contact_can_neither_replace_nor_cancel_the_held_drag() {
        let mut capture = Capture::default();
        let first = Pointer::Touch(2);
        let foreign = Pointer::Touch(3);
        capture.pointer_down(first);
        capture.accept_down(Some(2), Some(DragOwner::SliderPair));
        capture.pointer_down(foreign);
        assert!(!capture.capture_lost(foreign));
        assert!(!capture.capture_lost(Pointer::MouseLike(2)));
        capture.pointer_end(foreign);
        assert_eq!(capture.take_loss(), None);
        assert!(capture.capture_lost(first));
        assert_eq!(
            capture.take_loss(),
            Some(Contact {
                pointer: first,
                owner: DragOwner::SliderPair
            })
        );
    }

    #[test]
    fn untracked_drag_or_unmatched_down_does_not_arm_loss() {
        let mut capture = Capture::default();
        let first = Pointer::Touch(2);
        capture.pointer_down(first);
        capture.accept_down(Some(2), None);
        assert!(!capture.capture_lost(first));
        capture.pointer_down(first);
        capture.accept_down(Some(3), Some(DragOwner::Point));
        assert!(!capture.capture_lost(first));
        capture.pointer_down(first);
        capture.accept_down(None, Some(DragOwner::SliderPair));
        assert!(!capture.capture_lost(first));
    }

    #[test]
    fn primary_button_release_ends_tracking_even_during_a_mouse_chord() {
        let mut capture = Capture::default();
        let mouse = Pointer::MouseLike(1);
        for owner in [DragOwner::Point, DragOwner::SliderPair] {
            capture.pointer_down(mouse);
            capture.accept_down(None, Some(owner));
            capture.mouse_up();
            assert!(!capture.capture_lost(mouse));
        }
    }

    #[test]
    fn loss_retains_its_owner_kind_before_the_next_contact() {
        let mut capture = Capture::default();
        let mouse = Pointer::MouseLike(1);
        capture.pointer_down(mouse);
        capture.accept_down(None, Some(DragOwner::SliderPair));
        assert!(capture.capture_lost(mouse));
        capture.pointer_end(mouse);
        capture.pointer_down(mouse);
        assert_eq!(
            capture.take_loss(),
            Some(Contact {
                pointer: mouse,
                owner: DragOwner::SliderPair
            })
        );
        capture.accept_down(None, Some(DragOwner::Point));
        assert!(capture.capture_lost(mouse));
        assert_eq!(
            capture.take_loss(),
            Some(Contact {
                pointer: mouse,
                owner: DragOwner::Point
            })
        );
        assert_eq!(capture.take_loss(), None);
    }
}
