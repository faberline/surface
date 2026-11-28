use crate::{
    color_spring::ColorSpring,
    placeholder_motion::{PlaceholderMotion, PlaceholderPhase},
    Application, ContourPainter, PlaceholderPresentation, Runtime, ToggleMotion, ToggleSpring,
};
use std::{collections::BTreeMap, sync::Arc};
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

/// Motion for the paint of a text-input border or bottom indicator.
pub struct TextInputBoundaryMotion<'a> {
    /// The unfocused and focused widths, in that order, in logical pixels.
    pub widths: [f32; 2],
    pub width_spring: ToggleSpring,
    /// The theme's boundary colors. Changing the palette settles the ink.
    pub palette: &'a [Color],
}

type BoundaryContour =
    Box<dyn Fn(Rect, Option<TextInputLabelMotion>, f32, Color) -> Option<FillContour>>;

pub(crate) struct Boundary {
    pub(crate) label: Option<String>,
    pub(crate) widths: [f32; 2],
    pub(crate) width_spring: ToggleSpring,
    pub(crate) palette: Box<[Color]>,
    pub(crate) color: Box<dyn Fn(TextInputPaintState) -> Color>,
    pub(crate) paint: BoundaryContour,
}

pub(crate) struct LabelColorMotion {
    pub(crate) fallback: String,
    pub(crate) palette: Box<[Color]>,
}

pub(crate) enum Effect {
    Visibility(Box<dyn Fn(TextInputPaintState) -> bool>),
    Foreground(Box<dyn Fn(TextInputPaintState) -> Option<Color>>),
    Contour {
        reference: Option<String>,
        paint: TextInputContour,
    },
    Label {
        resting: String,
        color_motion: Option<LabelColorMotion>,
        alignment: TextAlignment,
        color: Box<dyn Fn(TextInputPaintState) -> Color>,
    },
    LabelContour {
        label: String,
        paint: TextInputLabelContour,
    },
    Boundary(Boundary),
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
pub(crate) struct EditorMotions {
    labels: BTreeMap<(String, String), LabelTrack>,
    boundaries: BTreeMap<(String, String), BoundaryTrack>,
    pub(crate) running: bool,
}

struct LabelTrack {
    spatial: ToggleMotion,
    color: Option<PaintColorTrack>,
    placeholder: Option<PlaceholderMotion>,
}

struct PaintColorTrack {
    spring: ColorSpring,
    palette: Box<[Color]>,
}

impl PaintColorTrack {
    fn new(ink: Color, palette: &[Color]) -> Self {
        Self {
            spring: ColorSpring::new(ink),
            palette: palette.into(),
        }
    }

    fn sample(
        &mut self,
        ink: Color,
        palette: &[Color],
        now: Option<f64>,
        snap: bool,
    ) -> (Color, bool) {
        let palette_changed = self.palette.as_ref() != palette;
        if palette_changed {
            self.palette = palette.into();
        }
        self.spring.sample(ink, now, snap || palette_changed)
    }
}

struct BoundaryTrack {
    width: ToggleMotion,
    width_spec: ([f32; 2], ToggleSpring),
    color: Option<PaintColorTrack>,
}

impl BoundaryTrack {
    fn new(boundary: &Boundary, focused: bool, ink: Color) -> Self {
        Self {
            width: ToggleMotion::new(focused),
            width_spec: (boundary.widths, boundary.width_spring),
            color: Some(PaintColorTrack::new(ink, &boundary.palette)),
        }
    }

    fn sample(
        &mut self,
        boundary: &Boundary,
        focused: bool,
        ink: Color,
        now: Option<f64>,
        snap: bool,
        enabled: bool,
    ) -> (f32, Color, bool) {
        let width_spec = (boundary.widths, boundary.width_spring);
        let width_changed = self.width_spec != width_spec;
        self.width_spec = width_spec;
        let (progress, width_running) =
            self.width
                .sample(focused, now, snap || width_changed, boundary.width_spring);
        let (ink, ink_running) = if enabled {
            self.color
                .get_or_insert_with(|| PaintColorTrack::new(ink, &boundary.palette))
                .sample(ink, &boundary.palette, now, snap)
        } else {
            self.color = None;
            (ink, false)
        };
        let [idle, focus] = boundary.widths;
        let width = match progress {
            0. => idle,
            1. => focus,
            _ => interpolate(idle, focus, progress),
        };
        (width, ink, width_running || ink_running)
    }
}

#[derive(Default)]
struct EditorPaint {
    paints: Vec<(NodeId, ResolvedPaint)>,
    hidden: Vec<NodeId>,
    labels: BTreeMap<(NodeId, NodeId), TextInputLabelMotion>,
}

impl EditorPaint {
    fn place_hint(&mut self, editor: NodeId, hint: Option<(Rect, ResolvedPaint)>, label: Rect) {
        if let Some((bounds, paint)) = hint {
            if !has_area(bounds.intersect(label)) {
                self.paints.push((editor, paint));
            }
        }
    }
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
    Placeholder(Arc<(String, Color)>, Color),
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
            Self::Placeholder(content, ink) => {
                display.set_presented_text_input_placeholder_ink(id, content, ink)
            }
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
        previous: &mut EditorMotions,
        frame: &mut EditorPaint,
    ) -> EditorMotions {
        let mut next = EditorMotions::default();
        for (label, binding) in &self.paint_bindings.text_inputs {
            let Effect::Label {
                resting,
                color_motion,
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
            let has_endpoints = has_area(from) && has_area(to);
            let editor_node = self.dom.get(editor).expect("validated text input");
            let parent = editor_node.parent();
            let fallback = color_motion
                .as_ref()
                .and_then(|motion| self.dom.by_id(&motion.fallback))
                .filter(|id| *id != resting && *id != *label && self.label_art(*id, parent, text));
            if !has_endpoints && fallback.is_none() {
                frame.hidden.extend([resting, *label]);
                continue;
            }
            let Some(state) = self.editor_paint_state(&binding.editor, *label, computed, hovered)
            else {
                frame.hidden.push(*label);
                continue;
            };
            let target = state.is_focused || state.is_composing || !state.is_empty;
            let visible = if has_endpoints {
                has_area(from.intersect(resting_geometry.clip))
                    || has_area(to.intersect(floating_geometry.clip))
            } else {
                fallback.is_some_and(|id| {
                    let geometry = &computed.nodes[id.index()];
                    has_area(geometry.bounds.intersect(geometry.clip))
                })
            };
            let enabled = !editor_node.is_disabled();
            let active = enabled && self.in_scope(editor);
            let (key, mut track) = previous.labels.remove_entry(&key).unwrap_or_else(|| {
                (
                    key,
                    LabelTrack {
                        spatial: ToggleMotion::new(target),
                        color: None,
                        placeholder: None,
                    },
                )
            });
            let snap = self.reduced_motion() || !active || !visible;
            let now = self.input_time_ms();
            let (progress, spatial_running) = track.spatial.sample(
                target,
                now,
                snap || !has_endpoints,
                ToggleSpring::StandardFastSpatial,
            );
            let target_color = color(state);
            // Never carry muted disabled ink into the first enabled frame.
            let animated_color = color_motion
                .as_ref()
                .filter(|_| enabled && fallback.is_some());
            let (ink, color_running) = if let Some(motion) = animated_color {
                track
                    .color
                    .get_or_insert_with(|| PaintColorTrack::new(target_color, &motion.palette))
                    .sample(target_color, &motion.palette, now, snap)
            } else {
                track.color = None;
                (target_color, false)
            };
            let (hint, hint_running) = self.resolve_hint(
                editor,
                &binding.editor,
                state,
                has_endpoints,
                active,
                &mut track.placeholder,
            );
            next.running |= spatial_running || color_running || hint_running;
            next.labels.insert(key, track);

            if let Some(fallback) = fallback {
                if !has_endpoints {
                    frame.hidden.extend([resting, *label]);
                    frame
                        .paints
                        .push((fallback, ResolvedPaint::Foreground(ink)));
                    frame.place_hint(editor, hint, computed.nodes[fallback.index()].bounds);
                    continue;
                }
                frame.hidden.push(fallback);
            }
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
            frame.place_hint(editor, hint, bounds);
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

    fn resolve_hint(
        &self,
        editor: NodeId,
        key: &str,
        state: TextInputPaintState,
        expands: bool,
        active: bool,
        track: &mut Option<PlaceholderMotion>,
    ) -> (Option<(Rect, ResolvedPaint)>, bool) {
        let content = self.bindings.get(&editor).and_then(|binding| {
            (binding.placeholder_presentation == PlaceholderPresentation::Animated)
                .then_some(binding.text_input_placeholder.as_ref())
                .flatten()
        });
        let Some(content) = content else {
            *track = None;
            return (None, false);
        };
        let phase = if state.is_focused {
            PlaceholderPhase::Focused
        } else if state.is_empty {
            PlaceholderPhase::UnfocusedEmpty
        } else {
            PlaceholderPhase::UnfocusedNotEmpty
        };
        let visible = active && has_area(state.bounds.intersect(state.clip));
        let (opacity, running) = track
            .get_or_insert_with(|| PlaceholderMotion::new(phase, expands))
            .sample(
                phase,
                expands,
                self.input_time_ms(),
                self.reduced_motion() || !visible,
            );
        // Keep the phase current while text or preedit owns the viewport.
        let Color(red, green, blue, alpha) = content.1;
        let alpha = (f32::from(alpha) * opacity).round() as u8;
        let hint = if visible && state.is_empty && !state.is_composing && alpha > 0 {
            self.text_input_hint_bounds(key).map(|bounds| {
                (
                    bounds,
                    ResolvedPaint::Placeholder(content.clone(), Color(red, green, blue, alpha)),
                )
            })
        } else {
            None
        };
        (hint, running)
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
        let floating = self.dom.get(label).ok()?;
        let name = floating.stable_id()?;
        let text = floating.text_value()?;
        (input.role() == Role::TextInput
            && label != resting_id
            && [label, resting_id]
                .into_iter()
                .all(|id| self.label_art(id, Some(parent), text)))
        .then(|| {
            (
                editor_id,
                resting_id,
                (editor.to_owned(), name.to_owned()),
                text,
            )
        })
    }

    fn label_art(&self, id: NodeId, parent: Option<NodeId>, text: &str) -> bool {
        self.dom.get(id).is_ok_and(|node| {
            node.role() == Role::Text
                && node.parent() == parent
                && node.text_value() == Some(text)
                && self.dom.children(id).next().is_none()
                && !self.has_interaction_binding(id)
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
            self.editor_motion = EditorMotions::default();
            return Vec::new();
        }
        let hovered = self
            .pointer_position
            .and_then(|(x, y)| computed.hit_test_where(&self.dom, x, y, |id| self.in_scope(id)));
        let mut previous = std::mem::take(&mut self.editor_motion);
        let mut frame = EditorPaint::default();
        let mut next =
            self.resolve_text_input_labels(computed, hovered, fonts, &mut previous, &mut frame);
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
                Effect::Boundary(boundary) => {
                    let editor = self.dom.by_id(&binding.editor);
                    let geometry = &computed.nodes[id.index()];
                    let pose = editor
                        .zip(
                            boundary
                                .label
                                .as_deref()
                                .and_then(|label| self.dom.by_id(label)),
                        )
                        .and_then(|key| frame.labels.get(&key).copied());
                    let ink = (boundary.color)(state);
                    let (width, ink) = if let Some(name) =
                        self.dom.get(*id).ok().and_then(surface::Node::stable_id)
                    {
                        let key = (binding.editor.clone(), name.to_owned());
                        let mut track = previous
                            .boundaries
                            .remove(&key)
                            .unwrap_or_else(|| BoundaryTrack::new(boundary, state.is_focused, ink));
                        let enabled = editor.is_some_and(|editor| {
                            self.dom.get(editor).is_ok_and(|node| !node.is_disabled())
                        });
                        let active = enabled && editor.is_some_and(|editor| self.in_scope(editor));
                        let snap = self.reduced_motion()
                            || !active
                            || !has_area(geometry.bounds.intersect(geometry.clip));
                        let (width, ink, running) = track.sample(
                            boundary,
                            state.is_focused,
                            ink,
                            self.input_time_ms(),
                            snap,
                            enabled,
                        );
                        next.running |= running;
                        next.boundaries.insert(key, track);
                        (width, ink)
                    } else {
                        (boundary.widths[usize::from(state.is_focused)], ink)
                    };
                    (boundary.paint)(geometry.bounds, pose, width, ink).map(ResolvedPaint::Contour)
                }
                Effect::Label { .. } => unreachable!("label paint was resolved before contours"),
            };
            if let Some(paint) = paint {
                frame.paints.push((*id, paint));
            } else {
                frame.hidden.push(*id);
            }
        }
        self.editor_motion = next;
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
