use crate::{Application, ContourPainter, Runtime};
use surface::{Color, ComputedDom, DomError, FillContour, MicroDom, NodeId, Rect, Role};

/// The final rendered state of a named text input in logical window pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextInputPaintState {
    pub is_focused: bool,
    pub is_hovered: bool,
    /// An active IME session, even when the current preedit is empty.
    pub is_composing: bool,
    /// Whether the committed Runtime editor value is empty.
    /// A disabled editor uses its authored value.
    pub is_empty: bool,
    pub bounds: Rect,
    pub clip: Rect,
}

pub(crate) type TextInputContour =
    Box<dyn Fn(TextInputPaintState, Rect, Option<Rect>) -> Option<FillContour>>;

pub(crate) enum Effect {
    Visibility(Box<dyn Fn(TextInputPaintState) -> bool>),
    Foreground(Box<dyn Fn(TextInputPaintState) -> Option<Color>>),
    Contour {
        reference: Option<String>,
        paint: TextInputContour,
    },
}

pub(crate) struct Binding {
    pub(crate) editor: String,
    pub(crate) effect: Effect,
}

#[derive(Default)]
pub(crate) struct PaintBindings {
    pub(crate) retained_contours: Vec<(NodeId, ContourPainter)>,
    pub(crate) text_inputs: Vec<(NodeId, Binding)>,
}

pub(crate) enum ResolvedPaint {
    Foreground(Color),
    Contour(FillContour),
}

impl ResolvedPaint {
    pub(crate) fn apply(self, display: &mut MicroDom, id: NodeId) -> Result<(), DomError> {
        match self {
            Self::Foreground(color) => {
                let style = display.get(id)?.style().clone().foreground(color);
                display.set_style(id, style)
            }
            Self::Contour(contour) => display.set_presented_fill_contour(id, contour),
        }
    }
}

impl<A: Application> Runtime<A> {
    fn editor_paint_state(
        &self,
        editor: &str,
        visual: NodeId,
        computed: &ComputedDom,
        hovered: Option<NodeId>,
    ) -> Option<TextInputPaintState> {
        let id = self.dom.by_id(editor)?;
        let node = self.dom.get(id).ok()?;
        let shell = node.parent()?;
        if node.role() != Role::TextInput || self.dom.get(visual).ok()?.parent() != Some(shell) {
            return None;
        }
        let active = !node.is_disabled() && self.in_scope(id);
        let buffer = self.editor(editor);
        let geometry = &computed.nodes[id.index()];
        let action = self
            .bindings
            .get(&id)
            .and_then(|binding| self.dom.by_id(binding.text_input_hover_action.as_deref()?));
        let is_hovered = active
            && hovered.is_some_and(|target| {
                if target == id {
                    return true;
                }
                if action != Some(target) {
                    return false;
                }
                let Ok(peer) = self.dom.get(target) else {
                    return false;
                };
                let rect = computed.nodes[target.index()].bounds;
                peer.role() == Role::Button
                    && peer.parent() == Some(shell)
                    && rect.y >= geometry.bounds.y
                    && rect.y + rect.height <= geometry.bounds.y + geometry.bounds.height
            });
        Some(TextInputPaintState {
            is_focused: active && self.focused() == Some(editor),
            is_hovered,
            is_composing: active && buffer.is_some_and(crate::TextEditor::is_composing),
            is_empty: buffer.map_or_else(
                || node.text_value().unwrap_or_default().is_empty(),
                |buffer| buffer.value().is_empty(),
            ),
            bounds: geometry.bounds,
            clip: geometry.clip,
        })
    }

    /// Resolve the passive branches before assigning modal paint ranges.
    /// Keep every authored node and layout rectangle, including hidden art.
    pub(crate) fn resolve_text_input_paint(
        &self,
        computed: &mut ComputedDom,
    ) -> Vec<(NodeId, ResolvedPaint)> {
        let bindings = &self.paint_bindings.text_inputs;
        if bindings.is_empty() {
            return Vec::new();
        }
        let hovered = self
            .pointer_position
            .and_then(|(x, y)| computed.hit_test_where(&self.dom, x, y, |id| self.in_scope(id)));
        let mut resolved = Vec::new();
        let mut hidden = Vec::new();
        for (id, binding) in bindings {
            let Some(state) = self.editor_paint_state(&binding.editor, *id, computed, hovered)
            else {
                hidden.push(*id);
                continue;
            };
            let paint = match &binding.effect {
                Effect::Visibility(predicate) => {
                    if !predicate(state) {
                        hidden.push(*id);
                    }
                    continue;
                }
                Effect::Foreground(color) => color(state).map(ResolvedPaint::Foreground),
                Effect::Contour { reference, paint } => {
                    let shell = self.dom.get(*id).ok().and_then(surface::Node::parent);
                    let reference = reference
                        .as_ref()
                        .and_then(|key| self.dom.by_id(key))
                        .filter(|reference| {
                            self.dom
                                .get(*reference)
                                .ok()
                                .and_then(surface::Node::parent)
                                == shell
                        })
                        .map(|reference| computed.nodes[reference.index()].bounds);
                    let decoration = computed.nodes[id.index()].bounds;
                    paint(state, decoration, reference).map(ResolvedPaint::Contour)
                }
            };
            if let Some(paint) = paint {
                resolved.push((*id, paint));
            } else {
                hidden.push(*id);
            }
        }
        if !hidden.is_empty() {
            let mut mask = vec![false; self.dom.len()];
            for id in hidden {
                mask[id.index()] = true;
            }
            for (id, node) in self.dom.iter() {
                if node.parent().is_some_and(|parent| mask[parent.index()]) {
                    mask[id.index()] = true;
                }
            }
            computed.paint_order.retain(|id| !mask[id.index()]);
        }
        resolved
    }
}
