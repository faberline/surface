use crate::{Application, ContourPainter, Runtime, ToggleMotion, ToggleSpring};
use std::collections::BTreeMap;
use surface::{
    Color, ComputedDom, DomError, FillContour, MicroDom, NodeId, Rect, Role, TextAlignment,
    TextMeasurer, TextStyle,
};

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

/// Paint geometry shared by the label and the outline of a text field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextInputLabelMotion {
    pub progress: f32,
    pub bounds: Rect,
    pub floated_bounds: Rect,
}

pub(crate) type TextInputLabelContour =
    Box<dyn Fn(TextInputPaintState, Rect, Option<TextInputLabelMotion>) -> Option<FillContour>>;

pub(crate) enum Effect {
    Visibility(Box<dyn Fn(TextInputPaintState) -> bool>),
    Foreground(Box<dyn Fn(TextInputPaintState) -> Option<Color>>),
    Contour {
        reference: Option<String>,
        paint: TextInputContour,
    },
    Label {
        resting: String,
        alignment: TextAlignment,
        color: Box<dyn Fn(TextInputPaintState) -> Color>,
    },
    LabelContour {
        label: String,
        paint: TextInputLabelContour,
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

#[derive(Default)]
pub(crate) struct LabelMotions {
    tracks: BTreeMap<(String, String), ToggleMotion>,
    pub(crate) running: bool,
}

#[derive(Default)]
struct EditorPaint {
    paints: Vec<(NodeId, ResolvedPaint)>,
    hidden: Vec<NodeId>,
    labels: BTreeMap<(NodeId, NodeId), TextInputLabelMotion>,
}

pub(crate) enum ResolvedPaint {
    Foreground(Color),
    Contour(FillContour),
    Label {
        bounds: Rect,
        style: TextStyle,
        color: Color,
    },
    ClearPlaceholder,
}

impl ResolvedPaint {
    pub(crate) fn apply(self, display: &mut MicroDom, id: NodeId) -> Result<(), DomError> {
        match self {
            Self::Foreground(color) => {
                let style = display.get(id)?.style().clone().foreground(color);
                display.set_style(id, style)
            }
            Self::Contour(contour) => display.set_presented_fill_contour(id, contour),
            Self::Label {
                bounds,
                style,
                color,
            } => {
                let style = display
                    .get(id)?
                    .style()
                    .clone()
                    .with_text_style(style)
                    .foreground(color);
                display.set_style(id, style)?;
                display.set_presented_text_paint_bounds(id, bounds)
            }
            Self::ClearPlaceholder => display.clear_presented_text_input_placeholder(id),
        }
    }
}

fn interpolate(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

fn intermediate_label(
    fonts: &mut impl TextMeasurer,
    text: &str,
    from: TextStyle,
    to: TextStyle,
    endpoints: [Rect; 2],
    alignment: TextAlignment,
    progress: f32,
) -> (Rect, TextStyle) {
    let mix = |a, b| interpolate(a, b, progress);
    let style = TextStyle {
        font_size: mix(from.font_size, to.font_size),
        line_height: Some(mix(from.line_height_px(), to.line_height_px())),
        weight: mix(from.weight.into(), to.weight.into()).round() as u16,
        letter_spacing: mix(from.letter_spacing, to.letter_spacing),
        family: to.family,
    };
    let (width, height) = fonts.measure_text(text, &style, None);
    let factor = match alignment {
        TextAlignment::Left => 0.,
        TextAlignment::Center => 0.5,
        TextAlignment::Right => 1.,
    };
    let [resting, floating] = endpoints;
    let anchor = |rect: Rect| rect.x + factor * rect.width;
    let x = mix(anchor(resting), anchor(floating)) - factor * width;
    let y = mix(resting.y, floating.y);
    (Rect::new(x, y, width, height), style)
}

fn has_area(rect: Rect) -> bool {
    rect.width > 0. && rect.height > 0.
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

    fn resolve_text_input_labels(
        &self,
        computed: &ComputedDom,
        hovered: Option<NodeId>,
        fonts: &mut impl TextMeasurer,
        mut previous: LabelMotions,
        frame: &mut EditorPaint,
    ) -> LabelMotions {
        let mut next = LabelMotions::default();
        for (label, binding) in &self.paint_bindings.text_inputs {
            let Effect::Label {
                resting,
                alignment,
                color,
            } = &binding.effect
            else {
                continue;
            };
            let Some((editor, resting, key, text)) =
                self.label_nodes(*label, &binding.editor, resting)
            else {
                frame.hidden.push(*label);
                continue;
            };
            let resting_geometry = &computed.nodes[resting.index()];
            let floating_geometry = &computed.nodes[label.index()];
            let from = resting_geometry.bounds;
            let to = floating_geometry.bounds;
            if !has_area(from) || !has_area(to) {
                frame.hidden.extend([resting, *label]);
                continue;
            }
            let Some(state) = self.editor_paint_state(&binding.editor, *label, computed, hovered)
            else {
                frame.hidden.push(*label);
                continue;
            };
            let target = state.is_focused || state.is_composing || !state.is_empty;
            let visible = has_area(from.intersect(resting_geometry.clip))
                || has_area(to.intersect(floating_geometry.clip));
            let active =
                self.in_scope(editor) && self.dom.get(editor).is_ok_and(|node| !node.is_disabled());
            let (key, mut track) = previous
                .tracks
                .remove_entry(&key)
                .unwrap_or_else(|| (key, ToggleMotion::new(target)));
            let snap = self.reduced_motion() || !active || !visible;
            let (progress, running) = track.sample(
                target,
                self.input_time_ms(),
                snap,
                ToggleSpring::StandardFastSpatial,
            );
            next.running |= running;
            next.tracks.insert(key, track);

            let ink = color(state);
            let bounds = if progress == 0. {
                frame.hidden.push(*label);
                frame.paints.push((resting, ResolvedPaint::Foreground(ink)));
                from
            } else if progress == 1. {
                frame.hidden.push(resting);
                frame.paints.push((*label, ResolvedPaint::Foreground(ink)));
                to
            } else {
                frame.hidden.push(resting);
                let (bounds, style) = intermediate_label(
                    fonts,
                    text,
                    self.dom
                        .get(resting)
                        .expect("validated resting label")
                        .style()
                        .text_style(),
                    self.dom
                        .get(*label)
                        .expect("validated floating label")
                        .style()
                        .text_style(),
                    [from, to],
                    *alignment,
                    progress,
                );
                frame.paints.push((
                    *label,
                    ResolvedPaint::Label {
                        bounds,
                        style,
                        color: ink,
                    },
                ));
                bounds
            };
            if progress < 1.
                && state.is_focused
                && state.is_empty
                && !state.is_composing
                && self
                    .first_text_line(&binding.editor)
                    .is_some_and(|line| has_area(line.intersect(bounds)))
            {
                frame.paints.push((editor, ResolvedPaint::ClearPlaceholder));
            }
            frame.labels.insert(
                (editor, *label),
                TextInputLabelMotion {
                    progress,
                    bounds,
                    floated_bounds: to,
                },
            );
        }
        next
    }

    fn label_nodes(
        &self,
        label: NodeId,
        editor: &str,
        resting: &str,
    ) -> Option<(NodeId, NodeId, (String, String), &str)> {
        let editor_id = self.dom.by_id(editor)?;
        let input = self.dom.get(editor_id).ok()?;
        let parent = input.parent()?;
        let resting_id = self.dom.by_id(resting)?;
        let resting = self.dom.get(resting_id).ok()?;
        let floating = self.dom.get(label).ok()?;
        let name = floating.stable_id()?;
        let text = floating.text_value()?;
        (input.role() == Role::TextInput
            && label != resting_id
            && resting.text_value() == Some(text)
            && [(label, floating), (resting_id, resting)]
                .into_iter()
                .all(|(id, node)| {
                    node.role() == Role::Text
                        && node.parent() == Some(parent)
                        && self.dom.children(id).next().is_none()
                        && !self.has_interaction_binding(id)
                }))
        .then(|| {
            (
                editor_id,
                resting_id,
                (editor.to_owned(), name.to_owned()),
                text,
            )
        })
    }

    /// Resolve the passive branches before assigning modal paint ranges.
    /// Keep every authored node and layout rectangle, including hidden art.
    pub(crate) fn resolve_text_input_paint(
        &mut self,
        computed: &mut ComputedDom,
        fonts: &mut impl TextMeasurer,
    ) -> Vec<(NodeId, ResolvedPaint)> {
        if self.paint_bindings.text_inputs.is_empty() {
            self.editor_label_motion = LabelMotions::default();
            return Vec::new();
        }
        let hovered = self
            .pointer_position
            .and_then(|(x, y)| computed.hit_test_where(&self.dom, x, y, |id| self.in_scope(id)));
        let previous = std::mem::take(&mut self.editor_label_motion);
        let mut frame = EditorPaint::default();
        self.editor_label_motion =
            self.resolve_text_input_labels(computed, hovered, fonts, previous, &mut frame);
        for (id, binding) in &self.paint_bindings.text_inputs {
            if matches!(binding.effect, Effect::Label { .. }) {
                continue;
            }
            let Some(state) = self.editor_paint_state(&binding.editor, *id, computed, hovered)
            else {
                frame.hidden.push(*id);
                continue;
            };
            let paint = match &binding.effect {
                Effect::Visibility(predicate) => {
                    if !predicate(state) {
                        frame.hidden.push(*id);
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
                Effect::LabelContour { label, paint } => {
                    let pose = self
                        .dom
                        .by_id(&binding.editor)
                        .zip(self.dom.by_id(label))
                        .and_then(|key| frame.labels.get(&key).copied());
                    paint(state, computed.nodes[id.index()].bounds, pose)
                        .map(ResolvedPaint::Contour)
                }
                Effect::Label { .. } => unreachable!("label paint was resolved before contours"),
            };
            if let Some(paint) = paint {
                frame.paints.push((*id, paint));
            } else {
                frame.hidden.push(*id);
            }
        }
        if !frame.hidden.is_empty() {
            let mut mask = vec![false; self.dom.len()];
            for id in frame.hidden {
                mask[id.index()] = true;
            }
            for (id, node) in self.dom.iter() {
                if node.parent().is_some_and(|parent| mask[parent.index()]) {
                    mask[id.index()] = true;
                }
            }
            computed.paint_order.retain(|id| !mask[id.index()]);
        }
        frame.paints
    }
}
