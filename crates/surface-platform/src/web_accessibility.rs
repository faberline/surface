//! Publish the canvas's current semantic tree to the browser.
mod semantics;

use crate::{key_route, UserEvent};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use surface::{NodeId, Role};
use surface_app::{
    AccessibilityActivationTarget, AccessibilityFocusTarget, AccessibilityFrame,
    AccessibilityFrameId, EditKey, Event, KeyPress, Modifiers,
};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{Document, Element, HtmlCanvasElement, HtmlElement, KeyboardEvent};
use winit::event_loop::EventLoopProxy;

const ROOT: &str = "surface-accessibility";
const TEXT_INPUT: &str = "surface-text-input";
type EventListener = Closure<dyn FnMut(web_sys::Event)>;

pub(crate) fn contains(element: &Element) -> bool {
    element
        .closest("#surface-accessibility")
        .ok()
        .flatten()
        .is_some()
}

pub(crate) enum Action {
    Focus(AccessibilityFocusTarget),
    Press(AccessibilityActivationTarget),
    Key(AccessibilityFocusTarget, KeyPress, Option<Event>),
}

struct Target {
    element: Element,
    focus: AccessibilityFocusTarget,
    press: Option<AccessibilityActivationTarget>,
}

struct Published {
    generation: AccessibilityFrameId,
    visible: BTreeSet<NodeId>,
    editor: Option<NodeId>,
}

pub(crate) struct WebAccessibility {
    root: Element,
    document: Document,
    targets: Rc<RefCell<BTreeMap<usize, Target>>>,
    published: Option<Published>,
    follow_focus: bool,
    projecting_focus: Rc<RefCell<Option<Element>>>,
    listeners: Vec<(&'static str, EventListener)>,
}

impl WebAccessibility {
    pub(crate) fn new(proxy: EventLoopProxy<UserEvent>) -> Result<Self, JsValue> {
        let browser = web_sys::window()
            .ok_or_else(|| JsValue::from_str("Surface requires a browser window"))?;
        let document = browser
            .document()
            .ok_or_else(|| JsValue::from_str("Surface requires a browser document"))?;
        let root = document.create_element("div")?;
        root.set_id(ROOT);
        // Keep the semantic tree available without covering the canvas.
        root.set_attribute(
            "style",
            concat!(
                "position:fixed;top:0;left:0;width:1px;height:1px;overflow:hidden;",
                "opacity:0;white-space:nowrap;pointer-events:none",
            ),
        )?;
        let mut bridge = Self {
            root,
            document,
            targets: Rc::default(),
            published: None,
            follow_focus: false,
            projecting_focus: Rc::default(),
            listeners: Vec::new(),
        };
        for name in ["focusin", "click", "keydown", "focusout"] {
            let proxy = proxy.clone();
            let targets = Rc::clone(&bridge.targets);
            let document = bridge.document.clone();
            let projecting_focus = Rc::clone(&bridge.projecting_focus);
            let callback = EventListener::new(move |event: web_sys::Event| {
                if name == "focusout" {
                    let _ = proxy.send_event(UserEvent::VerifyWebFocus);
                    return;
                }
                let Some(element) = event
                    .target()
                    .and_then(|target| target.dyn_into::<Element>().ok())
                    .and_then(|element| {
                        element
                            .closest("[data-surface-id], [data-surface-node]")
                            .ok()
                            .flatten()
                    })
                else {
                    return;
                };
                if (name == "focusin" && projecting_focus.borrow().as_ref() == Some(&element))
                    || (name == "keydown" && document.active_element().as_ref() != Some(&element))
                {
                    return;
                }
                let Some(id) = element
                    .get_attribute("data-surface-node")
                    .and_then(|id| id.parse().ok())
                else {
                    return;
                };
                let action = {
                    let targets = targets.borrow();
                    let Some(target) = targets.get(&id).filter(|target| target.element == element)
                    else {
                        return;
                    };
                    match name {
                        "focusin" => Some(Action::Focus(target.focus.clone())),
                        "click" => target.press.clone().map(Action::Press),
                        "keydown" => event
                            .dyn_ref::<KeyboardEvent>()
                            .and_then(projected_key)
                            .map(|(press, event)| Action::Key(target.focus.clone(), press, event)),
                        _ => None,
                    }
                };
                if let Some(action) = action {
                    if !matches!(action, Action::Key(_, _, None)) {
                        event.prevent_default();
                    }
                    event.stop_propagation();
                    let _ = proxy.send_event(UserEvent::WebAccessibility(action));
                }
            });
            bridge
                .root
                .add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())?;
            bridge.listeners.push((name, callback));
        }
        bridge
            .document
            .body()
            .ok_or_else(|| JsValue::from_str("Surface requires a browser body"))?
            .append_child(&bridge.root)?;
        Ok(bridge)
    }

    pub(crate) fn owns_focus(&self) -> bool {
        self.document
            .active_element()
            .is_some_and(|element| contains(&element))
    }

    pub(crate) fn follow_focus(&mut self, follow: bool) {
        self.follow_focus = follow;
    }

    pub(crate) fn publish<M>(
        &mut self,
        frame: &AccessibilityFrame<'_, M>,
        canvas: &HtmlCanvasElement,
    ) -> Result<(), JsValue> {
        let generation = frame.generation();
        let mut visible = BTreeSet::new();
        collect_visible(frame, frame.root(), &mut visible);
        let focused = frame.focused();
        let editor = focused.filter(|id| {
            frame
                .node(*id)
                .is_some_and(|source| source.node.role() == Role::TextInput)
        });
        let keep_focus = self.owns_focus()
            || (self.follow_focus
                && self.document.active_element().as_ref() == Some(canvas.unchecked_ref()));
        if self.published.as_ref().is_none_or(|old| {
            old.generation != generation || old.visible != visible || old.editor != editor
        }) {
            let mut targets = BTreeMap::new();
            let tree = project(
                &self.document,
                frame,
                frame.root(),
                &visible,
                editor,
                &mut targets,
            )?;
            self.root.set_text_content(None);
            self.root.append_child(&tree)?;
            *self.targets.borrow_mut() = targets;
            self.published = Some(Published {
                generation,
                visible,
                editor,
            });
        }
        if keep_focus {
            let destination = focused
                .and_then(|id| {
                    self.targets
                        .borrow()
                        .get(&id.index())
                        .map(|entry| entry.element.clone())
                })
                .or_else(|| editor.and_then(|_| self.document.get_element_by_id(TEXT_INPUT)));
            if let Some(destination) = destination {
                if self.document.active_element().as_ref() != Some(&destination) {
                    let control = destination.clone().dyn_into::<HtmlElement>()?;
                    // This is the browser's reflection of focus the Runtime
                    // already selected. A queued echo could undo later input.
                    self.projecting_focus.replace(Some(destination));
                    let result = control.focus();
                    self.projecting_focus.take();
                    result?;
                }
            } else if self.owns_focus()
                || self.document.active_element().as_ref() != Some(canvas.unchecked_ref())
            {
                canvas.focus()?;
            }
        }
        Ok(())
    }
}

impl Drop for WebAccessibility {
    fn drop(&mut self) {
        for (name, callback) in &self.listeners {
            let _ = self
                .root
                .remove_event_listener_with_callback(name, callback.as_ref().unchecked_ref());
        }
        self.root.remove();
    }
}

fn collect_visible<M>(
    frame: &AccessibilityFrame<'_, M>,
    id: NodeId,
    visible: &mut BTreeSet<NodeId>,
) -> bool {
    let Some(source) = frame.node(id) else {
        return false;
    };
    let bounds = source.layout.bounds.intersect(source.layout.clip);
    let mut show = (bounds.width > 0. && bounds.height > 0.) || id == frame.root();
    for child in frame.children(id) {
        show |= collect_visible(frame, child, visible);
    }
    if show {
        visible.insert(id);
    }
    show
}

fn project<M>(
    document: &Document,
    frame: &AccessibilityFrame<'_, M>,
    id: NodeId,
    visible: &BTreeSet<NodeId>,
    editor: Option<NodeId>,
    targets: &mut BTreeMap<usize, Target>,
) -> Result<Element, JsValue> {
    let source = frame.node(id).expect("visible nodes belong to the frame");
    let children: Vec<_> = frame
        .children(id)
        .filter(|id| visible.contains(id))
        .collect();
    let element = semantics::element(
        document,
        &source,
        frame.modal_root() == Some(id),
        children.is_empty(),
    )?;
    if let Some(stable_id) = source.node.stable_id() {
        element.set_attribute("data-surface-id", stable_id)?;
    }
    if let Some(focus) = frame.focus_target(id) {
        let press = frame.activation_target(id);
        element.set_attribute("data-surface-node", &id.index().to_string())?;
        element.set_attribute("tabindex", "-1")?;
        if press.is_some() {
            element.set_attribute("data-surface-action", "press")?;
        }
        targets.insert(
            id.index(),
            Target {
                element: element.clone(),
                focus,
                press,
            },
        );
    }
    for child in children {
        if editor == Some(child) {
            element.set_attribute("aria-owns", TEXT_INPUT)?;
            continue;
        }
        let child_element = project(document, frame, child, visible, editor, targets)?;
        element.append_child(&child_element)?;
    }
    Ok(element)
}

fn projected_key(event: &KeyboardEvent) -> Option<(KeyPress, Option<Event>)> {
    if key_route::dom_composing(event.is_composing(), event.key_code(), false) {
        return None;
    }
    let key = event.key();
    let modifiers = Modifiers {
        shift: event.shift_key(),
        control: event.ctrl_key(),
        alt: event.alt_key(),
        meta: event.meta_key(),
    };
    let action = if modifiers.control || modifiers.alt || modifiers.meta {
        None
    } else {
        match key.as_str() {
            "Tab" => Some(Event::FocusNext {
                reverse: modifiers.shift,
            }),
            "Escape" => Some(Event::Cancel),
            "Enter" | " " => Some(Event::Activate),
            name => {
                let edit = match name {
                    "ArrowLeft" => Some(EditKey::Left),
                    "ArrowRight" => Some(EditKey::Right),
                    "ArrowUp" => Some(EditKey::Up),
                    "ArrowDown" => Some(EditKey::Down),
                    "Home" => Some(EditKey::Home),
                    "End" => Some(EditKey::End),
                    "PageUp" => Some(EditKey::PageUp),
                    "PageDown" => Some(EditKey::PageDown),
                    _ => None,
                };
                edit.map(|key| Event::Edit {
                    key,
                    extend: modifiers.shift,
                    by_word: false,
                })
            }
        }
    };
    let press = KeyPress::new(key_route::from_dom(&key)?, modifiers).with_repeat(event.repeat());
    Some((press, action))
}
