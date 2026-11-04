//! Platform-independent Rust application state and input routing.
mod editing;
mod editing_runtime;
pub use editing::{
    utf16_to_utf8, utf8_to_utf16, Composition, EditKey, ImeEvent, TextEditor, TextOptions,
    TextSelection,
};
pub use editing_runtime::InputSnapshot;
use std::{collections::BTreeMap, rc::Rc};
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
}
impl<M> View<M> {
    pub fn new(node: Node) -> Self {
        Self {
            node,
            children: Vec::new(),
            activate: None,
            input: None,
            text_options: TextOptions::default(),
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
}
pub trait Application: 'static {
    type Message: Clone + 'static;
    fn title(&self) -> &str {
        "Surface"
    }
    fn update(&mut self, message: Self::Message);
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
}
struct Binding<M> {
    activate: Option<M>,
    input: Option<Rc<dyn Fn(String) -> M>>,
    text_options: TextOptions,
}

pub struct Runtime<A: Application> {
    app: A,
    dom: MicroDom,
    bindings: BTreeMap<NodeId, Binding<A::Message>>,
    layout: LayoutEngine,
    computed: Option<ComputedDom>,
    focused: Option<String>,
    pressed: Option<String>,
    scroll: BTreeMap<String, (f32, f32)>,
    editors: BTreeMap<String, TextEditor>,
    text_geometry: BTreeMap<String, editing_runtime::EditingGeometry>,
    ime_area: Option<surface::Rect>,
    text_drag: Option<String>,
}
impl<A: Application> Runtime<A> {
    pub fn new(app: A) -> Result<Self, AppError> {
        let mut runtime = Self {
            app,
            dom: MicroDom::new(Node::new(Role::Window, "Surface")),
            bindings: BTreeMap::new(),
            layout: LayoutEngine::default(),
            computed: None,
            focused: None,
            pressed: None,
            scroll: BTreeMap::new(),
            editors: BTreeMap::new(),
            text_geometry: BTreeMap::new(),
            ime_area: None,
            text_drag: None,
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
            if node.is_focusable() && node.stable_id().is_none() {
                return Err(AppError::MissingId(node.name().to_owned()));
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
        Ok(())
    }
    pub fn render(
        &mut self,
        width: f32,
        height: f32,
        measurer: &mut impl TextMeasurer,
    ) -> Result<DisplayList, AppError> {
        let (display_dom, decorations, computed) = self.layout_editing(width, height, measurer)?;
        let focused = self.focused.as_ref().and_then(|id| self.dom.by_id(id));
        let list = DisplayList::compile_with_text(&display_dom, &computed, focused, &decorations);
        self.computed = Some(computed);
        Ok(list)
    }
    fn hit(&self, x: f32, y: f32) -> Option<NodeId> {
        self.computed.as_ref()?.hit_test(&self.dom, x, y)
    }
    fn stable(&self, id: NodeId) -> Option<String> {
        self.dom.get(id).ok()?.stable_id().map(str::to_owned)
    }
    fn focused_node(&self) -> Option<NodeId> {
        self.dom.by_id(self.focused.as_deref()?)
    }
    fn message(&mut self, message: Option<A::Message>) -> Result<bool, AppError> {
        if let Some(message) = message {
            self.app.update(message);
            self.rebuild()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// Semantic drivers and platform input use the same activation path.
    pub fn activate(&mut self, stable_id: &str) -> Result<bool, AppError> {
        if self.focused.as_deref() != Some(stable_id) {
            self.cancel_composition();
        }
        let id = self
            .dom
            .by_id(stable_id)
            .filter(|id| self.dom.get(*id).is_ok_and(|n| n.is_focusable()))
            .ok_or_else(|| AppError::MissingControl(stable_id.to_owned()))?;
        self.focused = Some(stable_id.to_owned());
        let message = self.bindings.get(&id).and_then(|b| b.activate.clone());
        self.message(message)
    }
    pub fn dispatch(&mut self, event: Event) -> Result<bool, AppError> {
        match event {
            Event::PointerDown { x, y } => {
                self.cancel_composition();
                self.pressed = self.hit(x, y).and_then(|id| self.stable(id));
                self.focused = self.pressed.clone();
                self.place_text_cursor(x, y, false);
                Ok(true)
            }
            Event::PointerUp { x, y } => {
                self.text_drag = None;
                let hit = self.hit(x, y).and_then(|id| self.stable(id));
                if let Some(pressed) = self
                    .pressed
                    .take()
                    .filter(|pressed| hit.as_ref() == Some(pressed))
                {
                    self.activate(&pressed)
                } else {
                    Ok(false)
                }
            }
            Event::FocusNext { reverse } => {
                self.cancel_composition();
                let controls: Vec<_> = self
                    .dom
                    .iter()
                    .filter(|(id, n)| {
                        n.is_focusable()
                            && self.computed.as_ref().is_none_or(|c| {
                                let g = &c.nodes[id.index()];
                                let r = g.bounds.intersect(g.clip);
                                r.width > 0. && r.height > 0.
                            })
                    })
                    .filter_map(|(id, _)| self.stable(id))
                    .collect();
                if controls.is_empty() {
                    self.focused = None;
                    return Ok(false);
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
                self.place_text_cursor(x, y, true);
                Ok(self.text_drag.is_some())
            }
            Event::ImeFor { id, event } => {
                if self.focused.as_ref() != Some(&id) {
                    return Ok(false);
                }
                self.editing_event(Event::Ime(event))
            }
            event @ (Event::Edit { .. }
            | Event::Ime(_)
            | Event::InputValue { .. }
            | Event::Select { .. }
            | Event::Blur) => self.editing_event(event),
            Event::Cancel => {
                self.pressed = None;
                self.text_drag = None;
                self.cancel_composition();
                Ok(true)
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
                Ok(true)
            }
        }
    }
}
