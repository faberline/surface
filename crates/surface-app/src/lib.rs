//! Platform-independent Rust application state and input routing.
#[cfg(feature = "bridge")]
mod bridge;
mod controls;
mod editing;
mod editing_runtime;
mod focus_scroll;
mod keys;
mod motion;
mod snapshot;
pub use controls::Roving;
pub use editing::{
    utf16_to_utf8, utf8_to_utf16, Composition, EditKey, ImeEvent, TextEditor, TextOptions,
    TextSelection,
};
pub use editing_runtime::InputSnapshot;
pub use keys::{Key, KeyPress, Modifiers, TextAction};
pub use snapshot::{DomExport, EditingSnapshot, InteractionSnapshot};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};
use surface::{
    ComputedDom, DisplayList, DomError, LayoutEngine, LayoutError, MicroDom, Node, NodeId, Role,
    TextMeasurer,
};

/// One view of application state. Messages return through `Application::update`.
pub struct View<M> {
    node: Node,
    children: Vec<Self>,
    activate: Option<M>,
    input: Option<Rc<dyn Fn(String) -> M>>,
    text_options: TextOptions,
    value: Option<Rc<dyn Fn(f64) -> M>>,
    roving: Option<Roving>,
    dismiss: Option<M>,
    key: Option<KeyHandler<M>>,
}
/// A key handler. `Some(message)` consumes the press, `None` passes it on.
type KeyHandler<M> = Rc<dyn Fn(&KeyPress) -> Option<M>>;
impl<M> View<M> {
    pub fn new(node: Node) -> Self {
        Self {
            node,
            children: Vec::new(),
            activate: None,
            input: None,
            text_options: TextOptions::default(),
            value: None,
            roving: None,
            dismiss: None,
            key: None,
        }
    }
    pub fn children(mut self, children: impl IntoIterator<Item = Self>) -> Self {
        self.children = children.into_iter().collect();
        self
    }
    pub fn on_press(mut self, message: M) -> Self {
        self.activate = Some(message);
        self
    }
    pub fn on_input(mut self, handler: impl Fn(String) -> M + 'static) -> Self {
        self.input = Some(Rc::new(handler));
        self
    }
    pub fn text_options(mut self, options: TextOptions) -> Self {
        self.text_options = options;
        self
    }
    pub fn on_value(mut self, handler: impl Fn(f64) -> M + 'static) -> Self {
        self.value = Some(Rc::new(handler));
        self
    }
    pub fn roving(mut self, options: Roving) -> Self {
        self.roving = Some(options);
        self
    }
    /// Handle key presses while this node has focus. The handler runs
    /// before the field's own key handling. Return `Some(message)` to
    /// consume the press: the field does not see it, and `message` runs.
    /// Return `None` to pass it to the field unchanged. The handler is not
    /// called during an IME composition. See `KeyPress`.
    pub fn on_key(mut self, handler: impl Fn(&KeyPress) -> Option<M> + 'static) -> Self {
        self.key = Some(Rc::new(handler));
        self
    }
    /// Trap focus in this panel, dismiss on Escape/outside click, then restore focus.
    pub fn modal(mut self, dismiss: M) -> Self {
        self.dismiss = Some(dismiss);
        self
    }
}
/// The app's answer to a close request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseDecision {
    Allow,
    Cancel,
}
pub trait Application: 'static {
    type Message: Clone + 'static;
    fn title(&self) -> &str {
        "Surface"
    }
    fn update(&mut self, message: Self::Message);
    /// Called on the event loop after a `WakeHandle::wake` from any thread.
    /// Poll workers, channels and shared state here. The default does
    /// nothing. Many wakes can coalesce into one call, so drain everything
    /// that is ready. The runtime rebuilds the view after this returns.
    fn wake(&mut self) {}
    /// Called once when the user asks to close the window. Return
    /// `CloseDecision::Cancel` to keep the window, for example while a dialog
    /// is open or after a failed save. The default allows the close.
    fn close_requested(&mut self) -> CloseDecision {
        CloseDecision::Allow
    }
    fn view(&self) -> View<Self::Message>;
}
#[derive(Clone, Debug)]
pub enum Event {
    PointerDown {
        x: f32,
        y: f32,
    },
    PointerUp {
        x: f32,
        y: f32,
    },
    FocusNext {
        reverse: bool,
    },
    Activate,
    Text(String),
    Backspace,
    PointerMove {
        x: f32,
        y: f32,
    },
    PointerLeave,
    Edit {
        key: EditKey,
        extend: bool,
        by_word: bool,
    },
    Ime(ImeEvent),
    ImeFor {
        id: String,
        event: ImeEvent,
    },
    InputValue {
        value: String,
        anchor: usize,
        cursor: usize,
    },
    Select {
        anchor: usize,
        cursor: usize,
    },
    Blur,
    Cancel,
    Scroll {
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
    },
    SetValue {
        id: String,
        value: f64,
    },
}
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Dom(#[from] DomError),
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error("interactive nodes need a stable ID: {0}")]
    MissingId(String),
    #[error("no enabled control with ID {0}")]
    MissingControl(String),
    #[error("writable text input needs an on_input handler: {0}")]
    MissingInputHandler(String),
    #[error("slider needs a finite range and on_value handler: {0}")]
    InvalidRange(String),
}
struct Binding<M> {
    activate: Option<M>,
    input: Option<Rc<dyn Fn(String) -> M>>,
    text_options: TextOptions,
    value: Option<Rc<dyn Fn(f64) -> M>>,
    roving: Option<Roving>,
    dismiss: Option<M>,
    key: Option<KeyHandler<M>>,
}

/// Messages posted from inside `update` or from other code on the UI thread.
/// Clone it freely. It is not `Send`; use `WakeHandle` across threads.
pub struct Queue<M>(Rc<RefCell<VecDeque<M>>>, Rc<Cell<bool>>);
impl<M> Clone for Queue<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}
impl<M> Queue<M> {
    /// Queue a message. It runs after the current `update` returns, in
    /// order, and never re-enters `update`. Outside an update it runs on the
    /// next `apply`, `wake` or `flush`.
    pub fn post(&self, message: M) {
        self.0.borrow_mut().push_back(message);
    }
    /// Ask the host to close the window. It does not call `close_requested`:
    /// the app already decided. The host ends the event loop after the
    /// current update, and the app state is then dropped normally. Called
    /// from inside `close_requested`, it takes effect at once, even if that
    /// hook returns `Cancel`.
    pub fn close(&self) {
        self.1.set(true);
    }
}

pub struct Runtime<A: Application> {
    queue: Queue<A::Message>,
    app: A,
    dom: MicroDom,
    bindings: BTreeMap<NodeId, Binding<A::Message>>,
    layout: LayoutEngine,
    computed: Option<ComputedDom>,
    focused: Option<String>,
    pressed: Option<String>,
    hovered: Option<String>,
    scroll: BTreeMap<String, (f32, f32)>,
    editors: BTreeMap<String, TextEditor>,
    text_geometry: BTreeMap<String, editing_runtime::EditingGeometry>,
    ime_area: Option<surface::Rect>,
    text_drag: Option<String>,
    scopes: Vec<controls::FocusScope>,
    range_drag: Option<controls::RangeDrag>,
    modal_outside: bool,
    layout_focus: Option<String>,
    layout_viewport: Option<(f32, f32)>,
    motion: motion::Motion,
}
impl<A: Application> Runtime<A> {
    pub fn new(app: A) -> Result<Self, AppError> {
        let mut runtime = Self {
            queue: Queue(Rc::default(), Rc::default()),
            app,
            dom: MicroDom::new(Node::new(Role::Window, "Surface")),
            bindings: BTreeMap::new(),
            layout: LayoutEngine::default(),
            computed: None,
            focused: None,
            pressed: None,
            hovered: None,
            scroll: BTreeMap::new(),
            editors: BTreeMap::new(),
            text_geometry: BTreeMap::new(),
            ime_area: None,
            text_drag: None,
            scopes: Vec::new(),
            range_drag: None,
            modal_outside: false,
            layout_focus: None,
            layout_viewport: None,
            motion: motion::Motion::default(),
        };
        runtime.rebuild()?;
        Ok(runtime)
    }
    pub fn app(&self) -> &A {
        &self.app
    }
    pub fn dom(&self) -> &MicroDom {
        &self.dom
    }
    pub fn computed(&self) -> Option<&ComputedDom> {
        self.computed.as_ref()
    }
    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }
    /// A handle for posting messages that run after the current update.
    pub fn queue(&self) -> Queue<A::Message> {
        self.queue.clone()
    }
    /// Apply a message outside input dispatch, then run queued messages and
    /// rebuild once. Used for timers, async results and tests.
    pub fn apply(&mut self, message: A::Message) -> Result<(), AppError> {
        self.queue.post(message);
        self.flush()
    }
    /// Run the app's `wake` hook, then queued messages, then rebuild once.
    /// The platform host calls this once per coalesced wake.
    pub fn wake(&mut self) -> Result<(), AppError> {
        self.app.wake();
        self.flush()
    }
    /// The host calls this when the user asks to close the window. Runs the
    /// app's `close_requested` hook once, then queued messages, then
    /// rebuilds. Returns `true` when the window should close now.
    pub fn request_close(&mut self) -> Result<bool, AppError> {
        let decision = self.app.close_requested();
        // Run what the hook posted and rebuild, also after a veto.
        self.flush()?;
        let closed_by_app = self.take_close();
        Ok(decision == CloseDecision::Allow || closed_by_app)
    }
    /// True once, after the app called `Queue::close`. The host checks it
    /// after each input, wake or flush.
    pub fn take_close(&mut self) -> bool {
        self.queue.1.replace(false)
    }
    /// Run queued messages in order, including ones posted by `update`
    /// while this runs, then rebuild once.
    pub fn flush(&mut self) -> Result<(), AppError> {
        loop {
            let next = self.queue.0.borrow_mut().pop_front();
            let Some(message) = next else { break };
            self.app.update(message);
        }
        self.rebuild()
    }
    fn rebuild(&mut self) -> Result<(), AppError> {
        fn add<M>(
            view: View<M>,
            dom: &mut MicroDom,
            parent: Option<NodeId>,
            bindings: &mut BTreeMap<NodeId, Binding<M>>,
            disabled: bool,
        ) -> Result<(), AppError> {
            let disabled = disabled || view.node.is_disabled();
            let node = view.node.disabled(disabled);
            if (node.is_focusable() || view.roving.is_some() || view.dismiss.is_some())
                && node.stable_id().is_none()
            {
                return Err(AppError::MissingId(node.name().to_owned()));
            }
            if node.role() == Role::Slider
                && (node.range_value().is_none_or(|v| !v.is_valid())
                    || (!disabled && view.value.is_none()))
            {
                return Err(AppError::InvalidRange(
                    node.stable_id().unwrap_or(node.name()).to_owned(),
                ));
            }
            if node.role() == Role::TextInput
                && !disabled
                && !view.text_options.read_only
                && view.input.is_none()
            {
                return Err(AppError::MissingInputHandler(
                    node.stable_id().unwrap_or(node.name()).to_owned(),
                ));
            }
            let id = if let Some(parent) = parent {
                dom.append(parent, node)?
            } else {
                dom.root()
            };
            bindings.insert(
                id,
                Binding {
                    activate: view.activate,
                    input: view.input,
                    text_options: view.text_options,
                    value: view.value,
                    roving: view.roving,
                    dismiss: view.dismiss,
                    key: view.key,
                },
            );
            for child in view.children {
                add(child, dom, Some(id), bindings, disabled)?;
            }
            Ok(())
        }
        let view = self.app.view();
        let mut dom = MicroDom::new(view.node.clone());
        let mut bindings = BTreeMap::new();
        add(view, &mut dom, None, &mut bindings, false)?;
        for (key, (x, y)) in &self.scroll {
            if let Some(id) = dom.by_id(key) {
                let mut style = dom.get(id)?.style().clone();
                style.scroll_x = *x;
                style.scroll_y = *y;
                dom.set_style(id, style)?;
            }
        }
        self.dom = dom;
        self.bindings = bindings;
        self.reconcile_editors();
        if self.focused.as_ref().is_some_and(|key| {
            self.dom
                .by_id(key)
                .and_then(|id| self.dom.get(id).ok())
                .is_none_or(|n| !n.is_focusable())
        }) {
            self.focused = None;
        }
        self.computed = None;
        self.reconcile_controls();
        self.hovered = self
            .hovered
            .take()
            .filter(|key| self.dom.by_id(key).is_some());
        self.pressed = self.pressed.take().filter(|key| {
            self.dom
                .by_id(key)
                .is_some_and(|id| self.dom.get(id).is_ok_and(|n| n.is_focusable()))
        });
        Ok(())
    }
    pub fn render(
        &mut self,
        width: f32,
        height: f32,
        measurer: &mut impl TextMeasurer,
    ) -> Result<DisplayList, AppError> {
        let (mut display_dom, mut decorations, mut computed) =
            self.layout_editing(width, height, measurer)?;
        if self.focused != self.layout_focus || self.layout_viewport != Some((width, height)) {
            if let Some(focused) = self.focused_node() {
                for ancestor in self.scroll_ancestors(focused) {
                    if self.reveal_in(focused, ancestor, &computed)? {
                        (display_dom, decorations, computed) =
                            self.layout_editing(width, height, measurer)?;
                    }
                }
            }
            self.layout_focus.clone_from(&self.focused);
            self.layout_viewport = Some((width, height));
        }
        self.present_motion(&mut display_dom, &computed)?;
        let focused = self.focused.as_ref().and_then(|id| self.dom.by_id(id));
        let interaction = surface::InteractionState {
            hovered: self.hovered.as_ref().and_then(|id| self.dom.by_id(id)),
            pressed: self.pressed.as_ref().and_then(|id| self.dom.by_id(id)),
        };
        let list = DisplayList::compile_with_interaction(
            &display_dom,
            &computed,
            focused,
            &decorations,
            interaction,
        );
        self.computed = Some(computed);
        Ok(list)
    }
    fn hit(&self, x: f32, y: f32) -> Option<NodeId> {
        self.computed
            .as_ref()?
            .hit_test(&self.dom, x, y)
            .filter(|id| self.in_scope(*id))
    }
    fn stable(&self, id: NodeId) -> Option<String> {
        self.dom.get(id).ok()?.stable_id().map(str::to_owned)
    }
    fn focused_node(&self) -> Option<NodeId> {
        self.dom.by_id(self.focused.as_deref()?)
    }
    fn message(&mut self, message: Option<A::Message>) -> Result<bool, AppError> {
        if let Some(message) = message {
            self.queue.post(message);
            self.flush()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// The host calls this for each key press before the field's own key
    /// handling. Returns `true` when the focused node's `on_key` handler
    /// consumed the press; the host then skips the field. Returns `false`
    /// with no handler, when the handler passes, or during an IME
    /// composition, where the handler is not called.
    pub fn key(&mut self, press: &KeyPress) -> Result<bool, AppError> {
        let _ = press;
        Ok(false)
    }
    /// Semantic drivers and platform input use the same activation path.
    pub fn activate(&mut self, stable_id: &str) -> Result<bool, AppError> {
        let id = self
            .dom
            .by_id(stable_id)
            .filter(|id| self.in_scope(*id) && self.dom.get(*id).is_ok_and(|n| n.is_focusable()))
            .ok_or_else(|| AppError::MissingControl(stable_id.to_owned()))?;
        let focus_changed = self.focused.as_deref() != Some(stable_id);
        if focus_changed {
            self.cancel_composition();
        }
        self.focused = Some(stable_id.to_owned());
        let message = self.bindings.get(&id).and_then(|b| b.activate.clone());
        self.message(message)
            .map(|changed| changed || focus_changed)
    }
    pub fn dispatch(&mut self, event: Event) -> Result<bool, AppError> {
        match event {
            Event::PointerDown { x, y } => {
                self.cancel_composition();
                self.modal_outside = self.outside_scope(x, y);
                if self.modal_outside {
                    self.pressed = None;
                    return Ok(true);
                }
                self.pressed = self.hit(x, y).and_then(|id| self.stable(id));
                self.hovered = self.pressed.clone();
                self.focused = self.pressed.clone();
                if self.start_range_drag() {
                    return self.range_pointer(x);
                }
                self.place_text_cursor(x, y, false);
                Ok(true)
            }
            Event::PointerUp { x, y } => {
                if self.range_drag.is_some() {
                    let result = self.range_pointer(x);
                    self.range_drag = None;
                    self.pressed = None;
                    return result;
                }
                if std::mem::take(&mut self.modal_outside) {
                    return if self.outside_scope(x, y) {
                        self.dismiss_scope()
                    } else {
                        Ok(true)
                    };
                }
                self.text_drag = None;
                let hit = self.hit(x, y).and_then(|id| self.stable(id));
                if let Some(pressed) = self
                    .pressed
                    .take()
                    .filter(|pressed| hit.as_ref() == Some(pressed))
                {
                    self.activate(&pressed).map(|_| true)
                } else {
                    Ok(true)
                }
            }
            Event::FocusNext { reverse } => {
                self.cancel_composition();
                let controls: Vec<_> = self
                    .dom
                    .iter()
                    .filter(|(id, n)| {
                        self.tab_stop(*id)
                            && n.is_tab_stop()
                            && self.computed.as_ref().is_none_or(|c| {
                                let g = &c.nodes[id.index()];
                                g.bounds.width > 0. && g.bounds.height > 0.
                            })
                    })
                    .filter_map(|(id, _)| self.stable(id))
                    .collect();
                if controls.is_empty() {
                    self.focused = self.scope_entry();
                    return Ok(true);
                }
                let current = self
                    .focused
                    .as_ref()
                    .and_then(|id| controls.iter().position(|x| x == id));
                let next = match (current, reverse) {
                    (Some(i), true) => (i + controls.len() - 1) % controls.len(),
                    (Some(i), false) => (i + 1) % controls.len(),
                    (None, true) => controls.len() - 1,
                    (None, false) => 0,
                };
                self.focused = Some(controls[next].clone());
                Ok(true)
            }
            Event::Activate => {
                if let Some(id) = self.focused.clone() {
                    self.activate(&id)
                } else {
                    Ok(false)
                }
            }
            Event::Text(text) => self.editing_event(Event::Text(text)),
            Event::Backspace => self.editing_event(Event::Edit {
                key: EditKey::Backspace,
                extend: false,
                by_word: false,
            }),
            Event::PointerMove { x, y } => {
                let hover = self.hit(x, y).and_then(|id| self.stable(id));
                let changed = self.hovered != hover;
                self.hovered = hover;
                if self.range_drag.is_some() {
                    return self.range_pointer(x);
                }
                self.place_text_cursor(x, y, true);
                Ok(changed || self.text_drag.is_some())
            }
            Event::PointerLeave => Ok(self.hovered.take().is_some()),
            Event::ImeFor { id, event } => {
                if self.focused.as_ref() != Some(&id) {
                    return Ok(false);
                }
                self.editing_event(Event::Ime(event))
            }
            Event::Edit {
                key,
                extend,
                by_word,
            } => {
                if self
                    .focused_node()
                    .and_then(|id| self.dom.get(id).ok())
                    .is_some_and(|n| n.role() != Role::TextInput)
                {
                    self.navigate_control(key)
                } else {
                    self.editing_event(Event::Edit {
                        key,
                        extend,
                        by_word,
                    })
                }
            }
            Event::SetValue { id, value } => self.set_value(&id, value),
            event @ (Event::Ime(_)
            | Event::InputValue { .. }
            | Event::Select { .. }
            | Event::Blur) => self.editing_event(event),
            Event::Cancel => {
                self.pressed = None;
                self.text_drag = None;
                self.range_drag = None;
                self.modal_outside = false;
                let composing = self.editors.values().any(|e| e.is_composing());
                self.cancel_composition();
                if composing {
                    Ok(true)
                } else {
                    self.dismiss_scope()
                }
            }
            Event::Scroll {
                x,
                y,
                delta_x,
                delta_y,
            } => {
                if !delta_x.is_finite() || !delta_y.is_finite() {
                    return Ok(false);
                }
                let Some(computed) = &self.computed else {
                    return Ok(false);
                };
                let target = computed
                    .paint_order
                    .iter()
                    .rev()
                    .find(|id| {
                        let n = self.dom.get(**id).expect("valid node");
                        let g = &computed.nodes[id.index()];
                        n.style().clip
                            && self.in_scope(**id)
                            && n.stable_id().is_some()
                            && g.bounds.intersect(g.clip).contains(x, y)
                    })
                    .copied();
                let Some(id) = target else { return Ok(false) };
                let g = &computed.nodes[id.index()];
                let s = self.dom.get(id)?.style();
                let extent = self.dom.children(id).fold(
                    (g.bounds.width, g.bounds.height),
                    |(w, h), child| {
                        let r = computed.nodes[child.index()].bounds;
                        (
                            w.max(r.x + r.width - g.bounds.x + s.scroll_x + s.padding),
                            h.max(r.y + r.height - g.bounds.y + s.scroll_y + s.padding),
                        )
                    },
                );
                let offset = (
                    (s.scroll_x + delta_x).clamp(0., (extent.0 - g.bounds.width).max(0.)),
                    (s.scroll_y + delta_y).clamp(0., (extent.1 - g.bounds.height).max(0.)),
                );
                let key = self.stable(id).expect("scroll node has ID");
                self.scroll.insert(key, offset);
                let mut style = s.clone();
                style.scroll_x = offset.0;
                style.scroll_y = offset.1;
                self.dom.set_style(id, style)?;
                self.computed = None;
                self.ime_area = None;
                Ok(true)
            }
        }
    }
}
