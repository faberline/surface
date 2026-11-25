//! Publish the canvas's current semantic tree to the browser.
mod live_status;
mod semantics;
mod tabs;

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

#[derive(Default, PartialEq, Eq)]
struct Relations {
    headings: BTreeMap<NodeId, NodeId>,
    references: BTreeMap<NodeId, NodeId>,
    text_targets: BTreeSet<NodeId>,
    clipped_text: BTreeMap<NodeId, String>,
    tabs: tabs::TabRelations,
}

impl Relations {
    fn target_id(&self, id: NodeId) -> String {
        let kind = if self.text_targets.contains(&id) {
            "text"
        } else {
            "tooltip"
        };
        format!("surface-accessibility-{kind}-{}", id.index())
    }

    fn described_by(&self, source: NodeId) -> Option<String> {
        self.references.get(&source).map(|id| self.target_id(*id))
    }

    fn heading_id(id: NodeId) -> String {
        format!("surface-accessibility-heading-{}", id.index())
    }
}

struct Published {
    generation: AccessibilityFrameId,
    visible: BTreeSet<NodeId>,
    relations: Relations,
    editor: Option<NodeId>,
    tree: Element,
}

pub(crate) struct WebAccessibility {
    root: Element,
    document: Document,
    targets: Rc<RefCell<BTreeMap<usize, Target>>>,
    published: Option<Published>,
    live_status: live_status::LiveStatus,
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
        let live_status = live_status::LiveStatus::new(browser, &document)?;
        live_status.mount(&root)?;
        let mut bridge = Self {
            root,
            document,
            targets: Rc::default(),
            published: None,
            live_status,
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
        let same_generation = self
            .published
            .as_ref()
            .is_some_and(|old| old.generation == generation);
        let mut visible = BTreeSet::new();
        collect_visible(frame, frame.root(), &mut visible);
        let tab_relations = tabs::TabRelations::collect(frame, &visible, |id| {
            // IDs inside our previous tree leave with that tree. Exclude
            // matches elsewhere so references cannot name foreign elements.
            let matches = self.document.query_selector_all(&format!("#{id}"))?;
            Ok::<_, JsValue>((0..matches.length()).all(|index| {
                matches.item(index).is_some_and(|node| {
                    self.published
                        .as_ref()
                        .is_some_and(|old| old.tree.contains(Some(&node)))
                })
            }))
        })?;
        let mut relations = Relations {
            tabs: tab_relations,
            ..Relations::default()
        };
        for source in &visible {
            if let Some(heading) = frame.dialog_heading_target(*source) {
                relations.headings.insert(*source, heading);
            }
            if let Some(target) = frame
                .description_target(*source)
                .filter(|id| visible.contains(id))
            {
                relations.references.insert(*source, target);
                if frame
                    .node(target)
                    .is_some_and(|node| node.node.role() == Role::Text)
                {
                    relations.text_targets.insert(target);
                }
            } else if let Some((target, text)) = frame.clipped_validation_text(*source) {
                relations.references.insert(*source, target);
                relations.text_targets.insert(target);
                relations.clipped_text.insert(target, text.to_owned());
            }
        }
        let statuses = visible
            .iter()
            .filter_map(|id| live_status::message(frame, *id))
            .map(|(key, text)| (key.to_owned(), text.to_owned()))
            .collect();
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
            old.generation != generation
                || old.visible != visible
                || old.relations != relations
                || old.editor != editor
        }) {
            let mut targets = BTreeMap::new();
            let tree = project(
                &self.document,
                frame,
                frame.root(),
                &visible,
                editor,
                &relations,
                &mut targets,
            )?;
            // A referenced hidden element still supplies its description.
            // Keep it separate from the focusable and readable projection.
            for (id, text) in &relations.clipped_text {
                let hidden = self.document.create_element("span")?;
                hidden.set_id(&relations.target_id(*id));
                hidden.set_attribute("hidden", "")?;
                hidden.set_text_content(Some(text));
                tree.append_child(&hidden)?;
            }
            for (panel_id, tab_id) in &relations.tabs.placeholders {
                let placeholder = self.document.create_element("div")?;
                placeholder.set_id(panel_id);
                placeholder.set_attribute("role", "tabpanel")?;
                placeholder.set_attribute("aria-labelledby", tab_id)?;
                placeholder.set_attribute("hidden", "")?;
                placeholder.set_attribute("aria-hidden", "true")?;
                tree.append_child(&placeholder)?;
            }
            // Put the live region inside the active modal. A browser may
            // prune accessibility content outside an aria-modal dialog.
            self.live_status.mount(if frame.modal_root().is_some() {
                &tree
            } else {
                &self.root
            })?;
            if let Some(old) = &self.published {
                self.root.replace_child(&tree, &old.tree)?;
            } else {
                self.root
                    .insert_before(&tree, self.root.first_child().as_ref())?;
            }
            *self.targets.borrow_mut() = targets;
            if let Some(textbox) = self.document.get_element_by_id(TEXT_INPUT) {
                if let Some(id) = editor.and_then(|id| relations.described_by(id)) {
                    textbox.set_attribute("aria-describedby", &id)?;
                } else {
                    textbox.remove_attribute("aria-describedby")?;
                }
                if editor
                    .and_then(|id| frame.node(id))
                    .is_some_and(|source| source.node.is_invalid())
                {
                    textbox.set_attribute("aria-invalid", "true")?;
                } else {
                    textbox.remove_attribute("aria-invalid")?;
                }
            }
            self.published = Some(Published {
                generation,
                visible,
                relations,
                editor,
                tree,
            });
        }
        self.live_status
            .publish(frame, &statuses, same_generation)?;
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
    relations: &Relations,
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
    if live_status::message(frame, id).is_some() {
        // Keep this status readable where it was authored. The persistent
        // region delivers text changes without re-announcing the new tree.
        element.set_attribute("aria-live", "off")?;
    }
    if let Some(stable_id) = source.node.stable_id() {
        element.set_attribute("data-surface-id", stable_id)?;
    }
    if source.node.role() == Role::Tooltip || relations.text_targets.contains(&id) {
        element.set_id(&relations.target_id(id));
    } else if source.node.role() == Role::Heading
        && relations.headings.values().any(|target| *target == id)
    {
        element.set_id(&Relations::heading_id(id));
    }
    if let Some(heading) = relations.headings.get(&id) {
        element.set_attribute("aria-labelledby", &Relations::heading_id(*heading))?;
        element.remove_attribute("aria-label")?;
    }
    if let Some(tab_id) = relations.tabs.tab_ids.get(&id) {
        element.set_id(tab_id);
    }
    if let Some(panel_id) = relations.tabs.controls.get(&id) {
        element.set_attribute("aria-controls", panel_id)?;
    }
    if let Some(panel) = relations.tabs.panels.get(&id) {
        element.set_attribute("role", "tabpanel")?;
        if let Some(panel_id) = &panel.dom_id {
            element.set_id(panel_id);
        }
        if let Some(label) = &panel.labelled_by {
            element.set_attribute("aria-labelledby", label)?;
            element.remove_attribute("aria-label")?;
        } else if source.node.name().trim().is_empty() {
            element.set_attribute("aria-label", &panel.fallback_label)?;
        }
    }
    let authored_description = relations.described_by(id);
    let description = if let Some(text) = source.node.web_adjustable_description_text() {
        // The referenced text stays hidden as a standalone item. Project it
        // first so the selected value precedes any authored hint.
        let value_id = format!("surface-accessibility-value-{}", id.index());
        let value = document.create_element("span")?;
        value.set_id(&value_id);
        value.set_attribute("hidden", "")?;
        value.set_text_content(Some(text));
        element.append_child(&value)?;
        Some(match authored_description {
            Some(hint) => format!("{value_id} {hint}"),
            None => value_id,
        })
    } else {
        authored_description
    };
    if let Some(description) = description {
        element.set_attribute("aria-describedby", &description)?;
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
        let child_element = project(document, frame, child, visible, editor, relations, targets)?;
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
