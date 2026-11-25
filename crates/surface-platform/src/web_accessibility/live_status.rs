//! Deliver each visible passive status change through a primed live region.

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use surface::{NodeId, Role};
use surface_app::AccessibilityFrame;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{Document, Element, Window};

pub(super) fn message<'a, M>(
    frame: &'a AccessibilityFrame<'_, M>,
    id: NodeId,
) -> Option<(&'a str, &'a str, bool)> {
    let source = frame.node(id)?;
    if id == frame.root()
        || source.node.role() != Role::Status
        || source.node.is_focusable()
        || source.node.is_disabled()
        || frame.children(id).next().is_some()
    {
        return None;
    }
    let bounds = source.layout.bounds.intersect(source.layout.clip);
    if !(bounds.width > 0. && bounds.height > 0.) {
        return None;
    }
    let key = source.node.stable_id().filter(|key| !key.is_empty())?;
    let text = source.node.text_value().unwrap_or(source.node.name());
    Some((key, text, source.node.is_assertive_live_status()))
}

type Messages = BTreeMap<String, String>;

pub(super) struct LiveStatus {
    element: Element,
    scope: Option<Option<String>>,
    visible: Messages,
    pending: Rc<RefCell<Messages>>,
    window: Window,
    timeout: Rc<Cell<Option<i32>>>,
    deliver: Closure<dyn FnMut()>,
}

impl LiveStatus {
    pub(super) fn new(
        window: Window,
        document: &Document,
        assertive: bool,
    ) -> Result<Self, JsValue> {
        let element = document.create_element("div")?;
        element.set_attribute("aria-live", if assertive { "assertive" } else { "polite" })?;
        element.set_attribute("aria-atomic", "true")?;
        element.set_attribute("aria-relevant", "additions text")?;
        let pending = Rc::new(RefCell::new(Messages::new()));
        let timeout = Rc::new(Cell::new(None));
        let deliver = {
            let element = element.clone();
            let pending = Rc::clone(&pending);
            let timeout = Rc::clone(&timeout);
            Closure::new(move || {
                timeout.set(None);
                let messages = std::mem::take(&mut *pending.borrow_mut());
                if !messages.is_empty() {
                    let text = messages.into_values().collect::<Vec<_>>().join(" ");
                    element.set_text_content(Some(&text));
                }
            })
        };
        Ok(Self {
            element,
            scope: None,
            visible: Messages::new(),
            pending,
            window,
            timeout,
            deliver,
        })
    }

    pub(super) fn mount(&self, parent: &Element) -> Result<(), JsValue> {
        if self.element.parent_element().as_ref() != Some(parent) {
            self.element.set_text_content(None);
            parent.append_child(&self.element)?;
        }
        Ok(())
    }

    pub(super) fn publish<M>(
        &mut self,
        frame: &AccessibilityFrame<'_, M>,
        visible: &Messages,
        same_generation: bool,
    ) -> Result<(), JsValue> {
        let scope = frame
            .modal_root()
            .map(|id| frame.node(id).and_then(|node| node.node.stable_id()));
        let same_scope = match scope {
            None => self.scope.is_none(),
            Some(Some(key)) => self.scope.as_ref().and_then(Option::as_deref) == Some(key),
            Some(None) => same_generation && self.scope == Some(None),
        };
        if same_scope && self.visible == *visible {
            return Ok(());
        }
        self.element.set_text_content(None);
        let mut pending = self.pending.borrow_mut();
        if !same_scope {
            self.visible.clear();
            pending.clear();
        } else {
            pending.retain(|key, text| visible.get(key) == Some(text));
        }
        for (key, text) in visible {
            if self.visible.get(key) != Some(text) && !text.trim().is_empty() {
                pending.insert(key.clone(), text.clone());
            }
        }
        self.scope = scope.map(|key| key.map(str::to_owned));
        self.visible.clone_from(visible);
        if !pending.is_empty() && self.timeout.get().is_none() {
            // Establish the blank region before inserting the announcement.
            let id = self
                .window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    self.deliver.as_ref().unchecked_ref(),
                    0,
                )?;
            self.timeout.set(Some(id));
        }
        Ok(())
    }
}

impl Drop for LiveStatus {
    fn drop(&mut self) {
        if let Some(id) = self.timeout.get() {
            self.window.clear_timeout_with_handle(id);
        }
    }
}
