//! The DOM hosts IME/clipboard input only. Rust owns the value and painted UI.
use crate::key_route;
use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
use surface_app::{
    utf16_to_utf8, utf8_to_utf16, Application, EditKey, Event, ImeEvent, InputSnapshot, Modifiers,
    Runtime,
};
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsCast};
use web_sys::{EventTarget, HtmlTextAreaElement, PointerEvent};
use winit::{platform::web::WindowExtWebSys, window::Window};

// Use the same fractional CSS offsets as winit's web pointer backend.
#[wasm_bindgen]
extern "C" {
    type PointerOffsets;
    #[wasm_bindgen(method, getter, js_name = offsetX)]
    fn offset_x(this: &PointerOffsets) -> f64;
    #[wasm_bindgen(method, getter, js_name = offsetY)]
    fn offset_y(this: &PointerOffsets) -> f64;
}

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
    press: Rc<RefCell<crate::press_guard::PressGuard>>,
    mirror: Rc<RefCell<InputMirror>>,
    listeners: Vec<Listener>,
}

#[derive(Default)]
struct InputMirror {
    field: Option<String>,
    state: Option<EditorState>,
}

impl InputMirror {
    fn owns(&self, snapshot: &InputSnapshot) -> bool {
        self.field.as_deref() == Some(snapshot.id.as_str())
    }

    fn runtime_unchanged(&self, snapshot: &InputSnapshot) -> bool {
        self.owns(snapshot)
            && self
                .state
                .as_ref()
                .is_some_and(|last| last.matches(snapshot))
    }
}

struct EditorState {
    value: String,
    anchor: usize,
    cursor: usize,
}

impl EditorState {
    fn from_snapshot(snapshot: &InputSnapshot) -> Self {
        Self {
            value: snapshot.value.clone(),
            anchor: snapshot.selection.anchor,
            cursor: snapshot.selection.cursor,
        }
    }

    fn matches(&self, snapshot: &InputSnapshot) -> bool {
        self.value == snapshot.value
            && self.anchor == snapshot.selection.anchor
            && self.cursor == snapshot.selection.cursor
    }
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
// HTML exposes both lone CR and CRLF as LF in a textarea's API value.
fn textarea_value(raw: &str) -> Cow<'_, str> {
    if raw.contains('\r') {
        Cow::Owned(raw.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(raw)
    }
}

fn host_units_to_raw_bytes(raw: &str, projected: &str, units: usize) -> usize {
    let mut byte = utf16_to_utf8(projected, units);
    for (cr, _) in raw.match_indices("\r\n") {
        if cr >= byte {
            break;
        }
        byte += 1;
    }
    byte
}

fn raw_bytes_to_host_units(raw: &str, byte: usize) -> usize {
    let removed = raw
        .match_indices("\r\n")
        .take_while(|(cr, _)| cr + 2 <= byte)
        .count();
    utf8_to_utf16(raw, byte) - removed
}

fn host_selection_units(area: &HtmlTextAreaElement) -> (usize, usize) {
    let start = area.selection_start().ok().flatten().unwrap_or(0) as usize;
    let end = area.selection_end().ok().flatten().unwrap_or(0) as usize;
    if area.selection_direction().ok().flatten().as_deref() == Some("backward") {
        (end, start)
    } else {
        (start, end)
    }
}

fn runtime_selection(
    area: &HtmlTextAreaElement,
    raw: &str,
    (anchor, cursor): (usize, usize),
) -> Option<(usize, usize)> {
    let value = area.value();
    if textarea_value(raw).as_ref() != value {
        return None;
    }
    Some((
        host_units_to_raw_bytes(raw, &value, anchor),
        host_units_to_raw_bytes(raw, &value, cursor),
    ))
}

fn forward_edit_selection<A: Application>(
    area: &HtmlTextAreaElement,
    runtime: &Rc<RefCell<Runtime<A>>>,
    window: &Window,
    units: (usize, usize),
) {
    let Some(snapshot) = runtime.borrow().input_snapshot() else {
        return;
    };
    if let Some((anchor, cursor)) = runtime_selection(area, &snapshot.value, units) {
        dispatch(runtime, window, Event::Select { anchor, cursor });
    }
}
fn sync_value<A: Application>(
    area: &HtmlTextAreaElement,
    runtime: &Rc<RefCell<Runtime<A>>>,
    mirror: &RefCell<InputMirror>,
) {
    let Some(snapshot) = runtime.borrow().input_snapshot() else {
        return;
    };
    if snapshot.composing || !mirror.borrow().owns(&snapshot) {
        return;
    }
    write_host_input(area, &snapshot, mirror, true);
}

fn write_host_input(
    area: &HtmlTextAreaElement,
    snapshot: &InputSnapshot,
    mirror: &RefCell<InputMirror>,
    force: bool,
) {
    let projected = textarea_value(&snapshot.value);
    if area.value() != projected.as_ref() {
        area.set_value(&projected);
    } else if !force && mirror.borrow().runtime_unchanged(snapshot) {
        // The browser may already have moved the selection on keydown. A
        // layout-only redraw must let select/keyup deliver that movement.
        return;
    }
    let backward = snapshot.selection.anchor > snapshot.selection.cursor;
    let start = raw_bytes_to_host_units(
        &snapshot.value,
        snapshot.selection.anchor.min(snapshot.selection.cursor),
    ) as u32;
    let end = raw_bytes_to_host_units(
        &snapshot.value,
        snapshot.selection.anchor.max(snapshot.selection.cursor),
    ) as u32;
    if area.selection_start().ok().flatten() != Some(start)
        || area.selection_end().ok().flatten() != Some(end)
        || (start != end
            && (area.selection_direction().ok().flatten().as_deref() == Some("backward"))
                != backward)
    {
        let _ = area.set_selection_range_with_direction(
            start,
            end,
            if backward { "backward" } else { "forward" },
        );
    }
    mirror.borrow_mut().state = Some(EditorState::from_snapshot(snapshot));
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
        let mirror = Rc::new(RefCell::new(InputMirror::default()));
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
            press: press.clone(),
            mirror: mirror.clone(),
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
            let mirror = mirror.clone();
            bridge.listen_with_capture(
                canvas.unchecked_into(),
                "pointerdown",
                true,
                move |event| {
                    press.borrow_mut().press();
                    if let Some(pointer) = event.dyn_ref::<PointerEvent>() {
                        let offsets: &PointerOffsets = pointer.unchecked_ref();
                        if pointer.button() == 0
                            && runtime.borrow().pointer_preserves_composition(
                                offsets.offset_x() as f32,
                                offsets.offset_y() as f32,
                            )
                        {
                            // Runtime rejects this down. Block the browser's
                            // focus default and winit's explicit canvas.focus().
                            event.prevent_default();
                            event.stop_immediate_propagation();
                            return;
                        }
                    }
                    // A canvas press gives Surface authority over its caret,
                    // even when it places the caret at the previous offset.
                    mirror.borrow_mut().state = None;
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
            let mirror = mirror.clone();
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
                    forward_edit_selection(&area, &runtime, &window, host_selection_units(&area));
                    dispatch(
                        &runtime,
                        &window,
                        Event::Edit {
                            key,
                            extend: false,
                            by_word: false,
                        },
                    );
                    sync_value(&area, &runtime, &mirror);
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
            let mirror = mirror.clone();
            bridge.listen(target.clone(), "compositionstart", move |event| {
                event.stop_propagation();
                mirror.borrow_mut().state = None;
                let units = host_selection_units(&area);
                forward_edit_selection(&area, &runtime, &window, units);
                base.set(units.0.min(units.1));
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
            let mirror = mirror.clone();
            bridge.listen(target.clone(), "input", move |event| {
                event.stop_propagation();
                if ignore_input.replace(false) {
                    window.request_redraw();
                    return;
                }
                if composing.get() {
                    let value = text.borrow().clone();
                    let projected = textarea_value(&value);
                    let a = host_units_to_raw_bytes(
                        &value,
                        &projected,
                        (area.selection_start().ok().flatten().unwrap_or(0) as usize)
                            .saturating_sub(base.get()),
                    );
                    let c = host_units_to_raw_bytes(
                        &value,
                        &projected,
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
                    let value = area.value();
                    // A real edit continues to submit the textarea API value.
                    let (anchor, cursor) = host_selection_units(&area);
                    let anchor = utf16_to_utf8(&value, anchor);
                    let cursor = utf16_to_utf8(&value, cursor);
                    if let Some(snapshot) = runtime.borrow().input_snapshot() {
                        let mut mirror = mirror.borrow_mut();
                        if mirror.owns(&snapshot) {
                            mirror.state = Some(EditorState {
                                value: value.clone(),
                                anchor,
                                cursor,
                            });
                        }
                    }
                    dispatch(
                        &runtime,
                        &window,
                        Event::InputValue {
                            value,
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
            let area = textarea.clone();
            let mirror = mirror.clone();
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
                let commit = !value.is_empty();
                dispatch_ime(
                    &runtime,
                    &window,
                    id.clone(),
                    if commit {
                        ImeEvent::Commit(value)
                    } else {
                        ImeEvent::Cancel
                    },
                );
                if commit {
                    let snapshot = runtime.borrow().input_snapshot();
                    let mut mirror = mirror.borrow_mut();
                    if let Some(snapshot) = snapshot.filter(|snapshot| {
                        !snapshot.composing
                            && id.as_deref() == Some(snapshot.id.as_str())
                            && mirror.owns(snapshot)
                            && textarea_value(&snapshot.value).as_ref() == area.value()
                    }) {
                        // The browser finished the same edit. The next select
                        // may arrive before a frame can acknowledge the commit.
                        mirror.state = Some(EditorState::from_snapshot(&snapshot));
                    }
                }
            })?;
        }
        for name in ["select", "keyup"] {
            let runtime = runtime.clone();
            let window = window.clone();
            let area = textarea.clone();
            let composing = composing.clone();
            let press = press.clone();
            let switching = switching.clone();
            let mirror = mirror.clone();
            bridge.listen(target.clone(), name, move |event| {
                event.stop_propagation();
                if composing.get() || switching.get() {
                    return;
                }
                // Safari resets the textarea selection to 0,0 when it takes focus
                // inside a canvas press. Surface already set the caret from the pointer.
                if name == "select" && !press.borrow().forwards_select() {
                    return;
                }
                let Some(snapshot) = runtime.borrow().input_snapshot() else {
                    return;
                };
                if snapshot.composing {
                    return;
                }
                let Some((anchor, cursor)) =
                    runtime_selection(&area, &snapshot.value, host_selection_units(&area))
                else {
                    return;
                };
                let state = EditorState {
                    value: snapshot.value.clone(),
                    anchor,
                    cursor,
                };
                let already_current = state.matches(&snapshot);
                {
                    let mut mirror = mirror.borrow_mut();
                    if !mirror.owns(&snapshot)
                        || (!already_current && !mirror.runtime_unchanged(&snapshot))
                    {
                        // An asynchronous select can arrive after Surface has
                        // already authored a newer selection or changed fields.
                        return;
                    }
                    mirror.state = Some(state);
                }
                if !already_current {
                    dispatch(&runtime, &window, Event::Select { anchor, cursor });
                }
            })?;
        }
        {
            let runtime = runtime.clone();
            let window = window.clone();
            let composing = composing.clone();
            let area = textarea.clone();
            let mirror = mirror.clone();
            bridge.listen(target.clone(), "keydown", move |event| {
                event.stop_propagation();
                let Some(key) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                    return;
                };
                // The app's key handler runs before the field (#103).
                let mods = Modifiers {
                    shift: key.shift_key(),
                    control: key.ctrl_key(),
                    alt: key.alt_key(),
                    meta: key.meta_key(),
                };
                let step = key_route::web_key(
                    &mut runtime.borrow_mut(),
                    &key.key(),
                    key.key_code(),
                    key.is_composing(),
                    composing.get(),
                    mods,
                    key.repeat(),
                );
                if let Some(error) = step.error {
                    web_sys::console::error_1(&error.into());
                }
                if step.prevent_default {
                    event.prevent_default();
                    window.request_redraw();
                }
                if let Some(action) = step.dispatch {
                    dispatch(&runtime, &window, action);
                }
                if step.sync {
                    sync_value(&area, &runtime, &mirror);
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
        if self.mirror.borrow().field != field {
            self.switching.set(true);
            if field.is_some() && self.focused() {
                // End the old browser composition before reusing this host.
                // A move back to canvas must blur and focus in one guarded step
                // below. Blurring here would leave the page body focused.
                let _ = self.textarea.blur();
            }
            self.composing.set(false);
            *self.mirror.borrow_mut() = InputMirror { field, state: None };
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
        let needs_focus = !self.focused();
        let mirror_value = !self.composing.get() && !snapshot.composing;
        if mirror_value {
            let pressing = !self.press.borrow().forwards_select();
            write_host_input(
                &self.textarea,
                &snapshot,
                &self.mirror,
                needs_focus || pressing,
            );
        } else {
            self.mirror.borrow_mut().state = None;
        }
        if needs_focus {
            self.switching.set(true);
            let _ = self.textarea.focus();
            if mirror_value {
                // The browser can reset its selection when focus returns from
                // a canvas press. Restore the Surface caret before yielding.
                write_host_input(&self.textarea, &snapshot, &self.mirror, true);
            }
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

#[cfg(test)]
mod newline_selection_tests {
    use super::{host_units_to_raw_bytes, raw_bytes_to_host_units, textarea_value};

    const RAW: &str = "A😃B\r\n中\rZ\r\n💡";

    #[test]
    fn first_line_arrow_left_still_targets_the_raw_value() {
        let value = textarea_value(RAW);
        assert_eq!(value, "A😃B\n中\nZ\n💡");
        let end_of_first_line = raw_bytes_to_host_units(RAW, "A😃B".len());
        assert_eq!(end_of_first_line, 4);
        assert_eq!(
            host_units_to_raw_bytes(RAW, &value, end_of_first_line - 1),
            "A😃".len()
        );
        assert_eq!(
            host_units_to_raw_bytes(RAW, &value, end_of_first_line + 1),
            "A😃B\r\n".len()
        );
    }

    #[test]
    fn maps_both_sides_of_normalized_breaks_and_surrogates() {
        let value = textarea_value(RAW);
        for (host_units, raw_bytes) in [
            (0, 0),
            (1, 1),
            (3, 5),
            (4, 6),
            (5, 8),
            (6, 11),
            (7, 12),
            (8, 13),
            (9, 15),
            (11, 19),
        ] {
            assert_eq!(host_units_to_raw_bytes(RAW, &value, host_units), raw_bytes);
            assert_eq!(raw_bytes_to_host_units(RAW, raw_bytes), host_units);
        }
        assert_eq!(host_units_to_raw_bytes(RAW, &value, 2), 1);
        assert_eq!(host_units_to_raw_bytes(RAW, &value, 10), 15);
        assert_eq!(raw_bytes_to_host_units(RAW, 7), 4);
    }
}
