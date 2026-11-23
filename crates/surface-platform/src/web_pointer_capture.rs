//! Match browser capture loss to the contact that started a point drag.

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

#[derive(Default)]
struct Capture {
    down: Option<Pointer>,
    held: Option<Pointer>,
    lost: Option<Pointer>,
}

impl Capture {
    fn pointer_down(&mut self, pointer: Pointer) {
        self.down = Some(pointer);
    }

    /// The capture-phase listener sees the DOM ID first. Winit then tells the
    /// host whether that down reached the runtime and began a point drag.
    fn accept_down(&mut self, touch: Option<u64>, point_drag: bool) {
        self.held = self
            .down
            .take()
            .filter(|pointer| point_drag && pointer.touch_id() == touch);
    }

    fn pointer_end(&mut self, pointer: Pointer) {
        if self.held == Some(pointer) {
            self.held = None;
        }
        if self.down == Some(pointer) {
            self.down = None;
        }
    }

    fn mouse_up(&mut self) {
        // Winit can report the primary button up during a multi-button chord,
        // before the browser sends pointerup for the last released button.
        if let Some(pointer @ Pointer::MouseLike(_)) = self.held {
            self.pointer_end(pointer);
        }
    }

    fn capture_lost(&mut self, pointer: Pointer) -> bool {
        if self.held != Some(pointer) {
            return false;
        }
        self.lost = self.held.take();
        true
    }

    fn take_loss(&mut self) -> Option<Pointer> {
        self.lost.take()
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use browser::WebPointerCapture;

#[cfg(target_arch = "wasm32")]
mod browser {
    use super::{Capture, Pointer};
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

        pub(crate) fn accept_down(&self, touch: Option<u64>, point_drag: bool) {
            self.state.borrow_mut().accept_down(touch, point_drag);
        }

        pub(crate) fn mouse_up(&self) {
            self.state.borrow_mut().mouse_up();
        }

        pub(crate) fn take_loss(&self) -> Option<Pointer> {
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
    use super::{Capture, Pointer};

    #[test]
    fn normal_up_or_cancel_clears_the_contact_before_loss() {
        let mut capture = Capture::default();
        for pointer in [Pointer::MouseLike(1), Pointer::Touch(2)] {
            capture.pointer_down(pointer);
            capture.accept_down(pointer.touch_id(), true);
            capture.pointer_end(pointer);
            assert!(!capture.capture_lost(pointer));
            assert_eq!(capture.take_loss(), None);
        }
    }

    #[test]
    fn explicit_loss_is_retained_before_late_up_and_drains_once() {
        let mut capture = Capture::default();
        let mouse = Pointer::MouseLike(1);
        capture.pointer_down(mouse);
        capture.accept_down(None, true);
        assert!(capture.capture_lost(mouse));
        capture.pointer_end(mouse);
        assert!(!capture.capture_lost(mouse));
        assert_eq!(capture.take_loss(), Some(mouse));
        assert_eq!(capture.take_loss(), None);
    }

    #[test]
    fn foreign_contact_can_neither_replace_nor_cancel_the_held_drag() {
        let mut capture = Capture::default();
        let first = Pointer::Touch(2);
        let foreign = Pointer::Touch(3);
        capture.pointer_down(first);
        capture.accept_down(Some(2), true);
        capture.pointer_down(foreign);
        assert!(!capture.capture_lost(foreign));
        capture.pointer_end(foreign);
        assert_eq!(capture.take_loss(), None);
        assert!(capture.capture_lost(first));
        assert_eq!(capture.take_loss(), Some(first));
    }

    #[test]
    fn only_a_point_drag_arms_the_exact_browser_contact() {
        let mut capture = Capture::default();
        let first = Pointer::Touch(2);
        capture.pointer_down(first);
        capture.accept_down(Some(2), false);
        assert!(!capture.capture_lost(first));
        capture.pointer_down(first);
        capture.accept_down(Some(3), true);
        assert!(!capture.capture_lost(first));
        capture.pointer_down(first);
        capture.accept_down(None, true);
        assert!(!capture.capture_lost(first));
    }

    #[test]
    fn primary_button_release_ends_tracking_even_during_a_mouse_chord() {
        let mut capture = Capture::default();
        let mouse = Pointer::MouseLike(1);
        capture.pointer_down(mouse);
        capture.accept_down(None, true);
        capture.mouse_up();
        assert!(!capture.capture_lost(mouse));
    }
}
