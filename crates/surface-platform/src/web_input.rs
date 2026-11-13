//! The DOM hosts IME/clipboard input only. Rust owns the value and painted UI.
use crate::key_route::{self, KeyStep};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
use surface_app::{
    utf16_to_utf8, utf8_to_utf16, Application, EditKey, Event, ImeEvent, KeyPress, Modifiers,
    Runtime,
};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{EventTarget, HtmlTextAreaElement};
use winit::{platform::web::WindowExtWebSys, window::Window};

struct Listener {
    target: EventTarget,
    name: &'static str,
    capture: bool,
    callback: Closure<dyn FnMut(web_sys::Event)>,
}
pub(crate) struct WebInput<A: Application> {
    textarea: HtmlTextAreaElement,
    runtime: Rc<RefCell<Runtime<A>>>,
    window: Arc<Window>,
    composing: Rc<Cell<bool>>,
    switching: Rc<Cell<bool>>,
    field: RefCell<Option<String>>,
    listeners: Vec<Listener>,
}
fn dispatch<A: Application>(runtime: &Rc<RefCell<Runtime<A>>>, window: &Window, event: Event) {
    if let Err(error) = runtime.borrow_mut().dispatch(event) {
        web_sys::console::error_1(&error.to_string().into());
    }
    window.request_redraw();
}
fn dispatch_ime<A: Application>(
    runtime: &Rc<RefCell<Runtime<A>>>,
    window: &Window,
    owner: Option<String>,
    event: ImeEvent,
) {
    if let Some(id) = owner {
        dispatch(runtime, window, Event::ImeFor { id, event });
    }
}
fn host_selection(area: &HtmlTextAreaElement) -> (usize, usize) {
    let value = area.value();
    let start = utf16_to_utf8(
        &value,
        area.selection_start().ok().flatten().unwrap_or(0) as usize,
    );
    let end = utf16_to_utf8(
        &value,
        area.selection_end().ok().flatten().unwrap_or(0) as usize,
    );
    if area.selection_direction().ok().flatten().as_deref() == Some("backward") {
        (end, start)
    } else {
        (start, end)
    }
}
fn sync_value<A: Application>(area: &HtmlTextAreaElement, runtime: &Rc<RefCell<Runtime<A>>>) {
    let Some(snapshot) = runtime.borrow().input_snapshot() else {
        return;
    };
    if snapshot.composing {
        return;
    }
    if area.value() != snapshot.value {
        area.set_value(&snapshot.value);
    }
    let start = utf8_to_utf16(
        &snapshot.value,
        snapshot.selection.anchor.min(snapshot.selection.cursor),
    ) as u32;
    let end = utf8_to_utf16(
        &snapshot.value,
        snapshot.selection.anchor.max(snapshot.selection.cursor),
    ) as u32;
    if area.selection_start().ok().flatten() != Some(start)
        || area.selection_end().ok().flatten() != Some(end)
    {
        let _ = area.set_selection_range_with_direction(
            start,
            end,
            if snapshot.selection.anchor > snapshot.selection.cursor {
                "backward"
            } else {
                "forward"
            },
        );
    }
}
impl<A: Application> WebInput<A> {
    pub fn new(
        runtime: Rc<RefCell<Runtime<A>>>,
        window: Arc<Window>,
    ) -> Result<Self, wasm_bindgen::JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();
        let textarea = document
            .create_element("textarea")?
            .dyn_into::<HtmlTextAreaElement>()?;
        textarea.set_id("surface-text-input");
        textarea.set_tab_index(-1);
        textarea.set_spellcheck(false);
        textarea.set_attribute("autocomplete", "off")?;
        textarea.set_attribute("autocapitalize", "off")?;
        textarea
            .style()
            .set_css_text(&crate::host_style::caret_css(0., 0., 24.));
        document.body().unwrap().append_child(&textarea)?;
        let composing = Rc::new(Cell::new(false));
        let switching = Rc::new(Cell::new(false));
        let press = Rc::new(RefCell::new(crate::press_guard::PressGuard::default()));
        let text = Rc::new(RefCell::new(String::new()));
        let base = Rc::new(Cell::new(0usize));
        let owner = Rc::new(RefCell::new(None::<String>));
        let ignore_input = Rc::new(Cell::new(false));
        let mut bridge = Self {
            textarea: textarea.clone(),
            runtime: runtime.clone(),
            window: window.clone(),
            composing: composing.clone(),
            switching: switching.clone(),
            field: RefCell::new(None),
            listeners: Vec::new(),
        };
        let target: EventTarget = textarea.clone().unchecked_into();
        if let Some(canvas) = window.canvas() {
            let runtime = runtime.clone();
            let window = window.clone();
            let composing = composing.clone();
            let owner = owner.clone();
            let ignore_input = ignore_input.clone();
            let press = press.clone();
            bridge.listen_with_capture(
                canvas.unchecked_into(),
                "pointerdown",
                true,
                move |_| {
                    press.borrow_mut().press();
                    if composing.replace(false) {
                        // Browser focus defaults can commit before winit delivers
                        // the Rust pointer event. Cancel before that default runs.
                        let id = owner.borrow_mut().take();
                        dispatch_ime(&runtime, &window, id, ImeEvent::Cancel);
                        ignore_input.set(true);
                    }
                },
            )?;
        }
        {
            let runtime = runtime.clone();
            let window = window.clone();
            let area = textarea.clone();
            let composing = composing.clone();
            let ignore_input = ignore_input.clone();
            bridge.listen(target.clone(), "beforeinput", move |event| {
                if composing.get() {
                    return;
                }
                let Some(input) = event.dyn_ref::<web_sys::InputEvent>() else {
                    return;
                };
                if input.is_composing() {
                    return;
                }
                // Cancellation need not send a final input event. Do not
                // swallow the next ordinary keystroke with its stale guard.
                ignore_input.set(false);
                let key = match input.input_type().as_str() {
                    "deleteContentBackward" => Some(EditKey::Backspace),
                    "deleteContentForward" => Some(EditKey::Delete),
                    _ => None,
                };
                if let Some(key) = key {
                    event.prevent_default();
                    event.stop_propagation();
                    let (anchor, cursor) = host_selection(&area);
                    dispatch(&runtime, &window, Event::Select { anchor, cursor });
                    dispatch(
                        &runtime,
                        &window,
                        Event::Edit {
                            key,
                            extend: false,
                            by_word: false,
                        },
                    );
                    sync_value(&area, &runtime);
                }
            })?;
        }
        {
            let runtime = runtime.clone();
            let window = window.clone();
            let area = textarea.clone();
            let composing = composing.clone();
            let base = base.clone();
            let owner = owner.clone();
            let ignore_input = ignore_input.clone();
            bridge.listen(target.clone(), "compositionstart", move |event| {
                event.stop_propagation();
                let (a, c) = host_selection(&area);
                dispatch(
                    &runtime,
                    &window,
                    Event::Select {
                        anchor: a,
                        cursor: c,
                    },
                );
                base.set(utf8_to_utf16(&area.value(), a.min(c)));
                composing.set(true);
                *owner.borrow_mut() = runtime.borrow().input_snapshot().map(|s| s.id);
                ignore_input.set(false);
                dispatch_ime(&runtime, &window, owner.borrow().clone(), ImeEvent::Begin);
            })?;
        }
        {
            let runtime = runtime.clone();
            let window = window.clone();
            let text = text.clone();
            let composing = composing.clone();
            let owner = owner.clone();
            bridge.listen(target.clone(), "compositionupdate", move |event| {
                event.stop_propagation();
                let Some(event) = event.dyn_ref::<web_sys::CompositionEvent>() else {
                    return;
                };
                let value = event.data().unwrap_or_default();
                *text.borrow_mut() = value.clone();
                composing.set(true);
                let end = value.len();
                dispatch_ime(
                    &runtime,
                    &window,
                    owner.borrow().clone(),
                    ImeEvent::Preedit {
                        text: value,
                        selection: Some((end, end)),
                    },
                );
            })?;
        }
        {
            let runtime = runtime.clone();
            let window = window.clone();
            let area = textarea.clone();
            let text = text.clone();
            let composing = composing.clone();
            let base = base.clone();
            let owner = owner.clone();
            let ignore_input = ignore_input.clone();
            bridge.listen(target.clone(), "input", move |event| {
                event.stop_propagation();
                if ignore_input.replace(false) {
                    window.request_redraw();
                    return;
                }
                if composing.get() {
                    let value = text.borrow().clone();
                    let a = utf16_to_utf8(
                        &value,
                        (area.selection_start().ok().flatten().unwrap_or(0) as usize)
                            .saturating_sub(base.get()),
                    );
                    let c = utf16_to_utf8(
                        &value,
                        (area.selection_end().ok().flatten().unwrap_or(0) as usize)
                            .saturating_sub(base.get()),
                    );
                    dispatch_ime(
                        &runtime,
                        &window,
                        owner.borrow().clone(),
                        ImeEvent::Preedit {
                            text: value,
                            selection: Some((a, c)),
                        },
                    );
                } else {
                    let (anchor, cursor) = host_selection(&area);
                    dispatch(
                        &runtime,
                        &window,
                        Event::InputValue {
                            value: area.value(),
                            anchor,
                            cursor,
                        },
                    );
                }
            })?;
        }
        {
            let runtime = runtime.clone();
            let window = window.clone();
            let composing = composing.clone();
            let text = text.clone();
            let owner = owner.clone();
            let ignore_input = ignore_input.clone();
            bridge.listen(target.clone(), "compositionend", move |event| {
                event.stop_propagation();
                let Some(event) = event.dyn_ref::<web_sys::CompositionEvent>() else {
                    return;
                };
                composing.set(false);
                text.borrow_mut().clear();
                let value = event.data().unwrap_or_default();
                let id = owner.borrow_mut().take();
                if id.as_deref() != runtime.borrow().focused() {
                    ignore_input.set(true);
                    window.request_redraw();
                    return;
                }
                dispatch_ime(
                    &runtime,
                    &window,
                    id,
                    if value.is_empty() {
                        ImeEvent::Cancel
                    } else {
                        ImeEvent::Commit(value)
                    },
                );
            })?;
        }
        for name in ["select", "keyup"] {
            let runtime = runtime.clone();
            let window = window.clone();
            let area = textarea.clone();
            let composing = composing.clone();
            let press = press.clone();
            bridge.listen(target.clone(), name, move |event| {
                event.stop_propagation();
                if composing.get() {
                    return;
                }
                // Safari resets the textarea selection to 0,0 when it takes focus
                // inside a canvas press. Surface already set the caret from the pointer.
                if name == "select" && !press.borrow().forwards_select() {
                    return;
                }
                let (anchor, cursor) = host_selection(&area);
                dispatch(&runtime, &window, Event::Select { anchor, cursor });
            })?;
        }
        {
            let runtime = runtime.clone();
            let window = window.clone();
            let composing = composing.clone();
            let area = textarea.clone();
            bridge.listen(target.clone(), "keydown", move |event| {
                event.stop_propagation();
                let Some(key) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                    return;
                };
                if key.is_composing() || composing.get() {
                    return;
                }
                // The app's key handler runs before the field (#103).
                let mods = Modifiers {
                    shift: key.shift_key(),
                    control: key.ctrl_key(),
                    alt: key.alt_key(),
                    meta: key.meta_key(),
                };
                let press = key_route::from_dom(&key.key()).map(|k| KeyPress::new(k, mods));
                let ime = key_route::dom_composing(key.is_composing(), key.key_code(), false);
                let step = key_route::route_key(&mut runtime.borrow_mut(), press, ime);
                match step {
                    KeyStep::Consumed => {
                        event.prevent_default();
                        window.request_redraw();
                        sync_value(&area, &runtime);
                        return;
                    }
                    KeyStep::Fail(error) => {
                        web_sys::console::error_1(&error.into());
                        return;
                    }
                    KeyStep::Pass => {}
                }
                if (key.meta_key() || key.ctrl_key())
                    && matches!(key.key().to_lowercase().as_str(), "z" | "y")
                {
                    event.prevent_default();
                    let redo = key.shift_key() || key.key().eq_ignore_ascii_case("y");
                    dispatch(
                        &runtime,
                        &window,
                        Event::Edit {
                            key: if redo { EditKey::Redo } else { EditKey::Undo },
                            extend: false,
                            by_word: false,
                        },
                    );
                    sync_value(&area, &runtime);
                    return;
                }
                let action = match key.key().as_str() {
                    "Tab" => Some(Event::FocusNext {
                        reverse: key.shift_key(),
                    }),
                    "Escape" => Some(Event::Cancel),
                    "Enter"
                        if runtime
                            .borrow()
                            .input_snapshot()
                            .is_some_and(|s| !s.multiline) =>
                    {
                        Some(Event::Activate)
                    }
                    _ => None,
                };
                if let Some(action) = action {
                    event.prevent_default();
                    dispatch(&runtime, &window, action);
                }
            })?;
        }
        {
            let runtime = runtime.clone();
            let window = window.clone();
            let composing = composing.clone();
            let switching = switching.clone();
            let press = press.clone();
            bridge.listen(target, "blur", move |_| {
                if switching.get() {
                    return;
                }
                // The browser blurs the textarea inside a canvas press (Safari).
                // Dropping that blur keeps the press that PointerUp activates.
                if !press.borrow().forwards_blur() {
                    return;
                }
                composing.set(false);
                dispatch(&runtime, &window, Event::Blur);
            })?;
        }
        {
            let composing = composing.clone();
            let runtime = runtime.clone();
            let window = window.clone();
            bridge.listen(
                web_sys::window().unwrap().unchecked_into(),
                "blur",
                move |_| {
                    composing.set(false);
                    dispatch(&runtime, &window, Event::Blur);
                },
            )?;
        }
        for name in ["pointerup", "pointercancel"] {
            let press = press.clone();
            bridge.listen_with_capture(
                web_sys::window().unwrap().unchecked_into(),
                name,
                true,
                move |_| press.borrow_mut().release(),
            )?;
        }
        Ok(bridge)
    }
    fn listen(
        &mut self,
        target: EventTarget,
        name: &'static str,
        callback: impl FnMut(web_sys::Event) + 'static,
    ) -> Result<(), wasm_bindgen::JsValue> {
        self.listen_with_capture(target, name, false, callback)
    }
    fn listen_with_capture(
        &mut self,
        target: EventTarget,
        name: &'static str,
        capture: bool,
        callback: impl FnMut(web_sys::Event) + 'static,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let callback = Closure::wrap(Box::new(callback) as Box<dyn FnMut(web_sys::Event)>);
        target.add_event_listener_with_callback_and_bool(
            name,
            callback.as_ref().unchecked_ref(),
            capture,
        )?;
        self.listeners.push(Listener {
            target,
            name,
            capture,
            callback,
        });
        Ok(())
    }
    pub fn focused(&self) -> bool {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .is_some_and(|e| e.id() == self.textarea.id())
    }
    pub fn owns_focus(&self) -> bool {
        self.switching.get() || self.focused()
    }
    pub fn sync(&self) {
        let snapshot = self.runtime.borrow().input_snapshot();
        let field = snapshot.as_ref().map(|s| s.id.clone());
        if *self.field.borrow() != field {
            self.switching.set(true);
            if field.is_some() && self.focused() {
                // End the old browser composition before reusing this host.
                // A move back to canvas must blur and focus in one guarded step
                // below. Blurring here would leave the page body focused.
                let _ = self.textarea.blur();
            }
            self.composing.set(false);
            *self.field.borrow_mut() = field;
            self.switching.set(false);
        }
        let Some(snapshot) = snapshot else {
            if self.focused() {
                self.switching.set(true);
                let _ = self.textarea.blur();
                if let Some(canvas) = self.window.canvas() {
                    let _ = canvas.focus();
                }
                self.switching.set(false);
            }
            self.composing.set(false);
            return;
        };
        self.textarea.set_read_only(snapshot.read_only);
        let _ = self.textarea.set_attribute("aria-label", &snapshot.label);
        if let (Some(caret), Some(canvas)) = (snapshot.caret, self.window.canvas()) {
            let bounds = canvas.get_bounding_client_rect();
            self.textarea
                .style()
                .set_css_text(&crate::host_style::caret_css(
                    bounds.left() + caret.x as f64,
                    bounds.top() + caret.y as f64,
                    caret.height,
                ));
        }
        if !self.composing.get() && !snapshot.composing {
            if self.textarea.value() != snapshot.value {
                self.textarea.set_value(&snapshot.value);
            }
            let start = utf8_to_utf16(
                &snapshot.value,
                snapshot.selection.anchor.min(snapshot.selection.cursor),
            ) as u32;
            let end = utf8_to_utf16(
                &snapshot.value,
                snapshot.selection.anchor.max(snapshot.selection.cursor),
            ) as u32;
            if self.textarea.selection_start().ok().flatten() != Some(start)
                || self.textarea.selection_end().ok().flatten() != Some(end)
            {
                let _ = self.textarea.set_selection_range_with_direction(
                    start,
                    end,
                    if snapshot.selection.anchor > snapshot.selection.cursor {
                        "backward"
                    } else {
                        "forward"
                    },
                );
            }
        }
        if !self.focused() {
            self.switching.set(true);
            let _ = self.textarea.focus();
            self.switching.set(false);
        }
    }
}
impl<A: Application> Drop for WebInput<A> {
    fn drop(&mut self) {
        for listener in &self.listeners {
            let _ = listener
                .target
                .remove_event_listener_with_callback_and_bool(
                    listener.name,
                    listener.callback.as_ref().unchecked_ref(),
                    listener.capture,
                );
        }
        self.textarea.remove();
    }
}
