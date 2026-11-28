use crate::{
    AppError, Application, EditKey, Event, ImeEvent, PlaceholderPresentation, Runtime,
    SingleLineVerticalPlacement, TextEditor, TextSelection,
};
use std::collections::BTreeMap;
use surface::{
    ComputedDom, Length, MicroDom, NodeId, Rect, Role, TextAlignment, TextDecoration, TextLayout,
    TextMeasurer, TextScaling,
};

const CARET_REVEAL_WIDTH: f32 = 2.;

fn right_alignment_space(content_width: f32, layout: &TextLayout) -> f32 {
    let text_right = layout.clusters.iter().fold(0_f32, |right, cluster| {
        right.max(cluster.bounds.x + cluster.bounds.width)
    });
    content_width - text_right - CARET_REVEAL_WIDTH
}

fn hint_bounds(
    layout: &TextLayout,
    viewport: Rect,
    clip: Rect,
    scroll: (f32, f32),
) -> Option<Rect> {
    let clip = clip.intersect(viewport);
    let x = viewport.x - scroll.0;
    let y = viewport.y - scroll.1.min(0.);
    layout
        .clusters
        .iter()
        .map(|cluster| {
            let bounds = cluster.bounds;
            Rect::new(bounds.x + x, bounds.y + y, bounds.width, bounds.height).intersect(clip)
        })
        .filter(|bounds| bounds.width > 0. && bounds.height > 0.)
        .reduce(|a, b| {
            let x = a.x.min(b.x);
            let y = a.y.min(b.y);
            Rect::new(
                x,
                y,
                (a.x + a.width).max(b.x + b.width) - x,
                (a.y + a.height).max(b.y + b.height) - y,
            )
        })
}

pub(crate) struct EditingGeometry {
    layout: TextLayout,
    origin: Rect,
    scroll: (f32, f32),
    multiline: bool,
    focused: bool,
    manual_scroll: bool,
    vertical_goal: Option<f32>,
    caret: Option<Rect>,
    hint_bounds: Option<Rect>,
}

impl EditingGeometry {
    fn vertical_limit(&self) -> f32 {
        (self.layout.height - self.origin.height).max(0.)
    }
}
#[derive(Clone, Debug)]
pub struct InputSnapshot {
    pub id: String,
    pub label: String,
    pub value: String,
    pub selection: TextSelection,
    pub composing: bool,
    pub caret: Option<Rect>,
    pub multiline: bool,
    pub read_only: bool,
}
impl<A: Application> Runtime<A> {
    /// Whether the focused editor uses Surface's wrapped visual-line keys.
    pub fn focused_uses_multiline_viewport(&self) -> bool {
        self.focused_node().is_some_and(|id| {
            self.in_scope(id)
                && self
                    .dom
                    .get(id)
                    .is_ok_and(|node| node.role() == Role::TextInput && !node.is_disabled())
                && self.bindings.get(&id).is_some_and(|binding| {
                    binding.text_options.multiline && binding.multiline_text_viewport.is_some()
                })
        })
    }

    pub fn input_snapshot(&self) -> Option<InputSnapshot> {
        let id = self.focused.as_ref()?;
        let node_id = self.dom.by_id(id)?;
        let node = self.dom.get(node_id).ok()?;
        if node.role() != Role::TextInput || node.is_disabled() {
            return None;
        }
        let editor = self.editors.get(id)?;
        let options = &self.bindings.get(&node_id)?.text_options;
        Some(InputSnapshot {
            id: id.clone(),
            label: node.name().to_owned(),
            value: editor.value().to_owned(),
            selection: editor.selection().clone(),
            composing: editor.is_composing(),
            caret: self.ime_area,
            multiline: options.multiline,
            read_only: options.read_only,
        })
    }
    pub fn editor(&self, id: &str) -> Option<&TextEditor> {
        self.editors.get(id)
    }
    pub fn selected_text(&self) -> Option<String> {
        let editor = self.editors.get(self.focused.as_ref()?)?;
        (!editor.is_composing()).then(|| editor.selected_text().to_owned())
    }
    pub fn cut_selection(&mut self) -> Result<bool, AppError> {
        let Some(snapshot) = self.input_snapshot() else {
            return Ok(false);
        };
        if snapshot.selection.is_collapsed() {
            return Ok(false);
        }
        self.dispatch(Event::Edit {
            key: EditKey::Backspace,
            extend: false,
            by_word: false,
        })
    }
    pub(crate) fn reconcile_editors(&mut self) {
        self.editors.retain(|id, _| {
            self.dom
                .by_id(id)
                .and_then(|id| self.dom.get(id).ok())
                .is_some_and(|n| n.role() == Role::TextInput && !n.is_disabled())
        });
        self.text_geometry
            .retain(|id, geometry| !geometry.multiline || self.editors.contains_key(id));
        for (node_id, node) in self
            .dom
            .iter()
            .filter(|(_, n)| n.role() == Role::TextInput && !n.is_disabled())
        {
            if let Some(id) = node.stable_id() {
                let value = node.text_value().unwrap_or_default();
                let editor = self
                    .editors
                    .entry(id.to_owned())
                    .or_insert_with(|| TextEditor::new(value));
                let options = &self.bindings[&node_id].text_options;
                if options.read_only && editor.composition().is_some() {
                    editor.ime(ImeEvent::Cancel, options);
                    if self.focused.as_deref() == Some(id) {
                        self.ime_area = None;
                    }
                }
                if value != editor.value() {
                    if let Some(geometry) = self.text_geometry.get_mut(id) {
                        geometry.manual_scroll = false;
                        geometry.vertical_goal = None;
                    }
                    editor.sync(value);
                }
            }
        }
    }
    pub(crate) fn cancel_composition(&mut self) {
        if self.editors.values().any(|editor| editor.is_composing()) {
            self.computed = None;
        }
        self.ime_area = None;
        for editor in self.editors.values_mut() {
            editor.ime(ImeEvent::Cancel, &Default::default());
        }
    }
    pub(crate) fn place_text_cursor(&mut self, x: f32, y: f32, drag: bool) {
        let Some(id) = self.focused.clone() else {
            return;
        };
        if drag && self.text_drag.as_ref() != Some(&id) {
            return;
        }
        let (Some(geometry), Some(editor)) =
            (self.text_geometry.get_mut(&id), self.editors.get_mut(&id))
        else {
            return;
        };
        if editor.is_composing() {
            return;
        }
        let cursor = geometry.layout.hit(
            x - geometry.origin.x + geometry.scroll.0,
            y - geometry.origin.y + geometry.scroll.1,
        );
        editor.select(
            if drag {
                editor.selection().anchor
            } else {
                cursor
            },
            cursor,
        );
        geometry.manual_scroll = false;
        geometry.vertical_goal = None;
        self.text_drag = Some(id);
    }

    pub(crate) fn multiline_scroll_at(
        &self,
        computed: &ComputedDom,
        point: (f32, f32),
    ) -> Option<NodeId> {
        self.multiline_viewport_at(computed, point).filter(|&id| {
            self.multiline_scroll_geometry(id)
                .is_some_and(|geometry| geometry.vertical_limit() > 0.)
        })
    }

    pub(crate) fn multiline_touch_at(
        &self,
        computed: &ComputedDom,
        point: (f32, f32),
    ) -> Option<NodeId> {
        self.multiline_viewport_at(computed, point)
    }

    fn multiline_viewport_at(&self, computed: &ComputedDom, point: (f32, f32)) -> Option<NodeId> {
        let id = self.raw_hit(point.0, point.1)?;
        let geometry = self.multiline_scroll_geometry(id)?;
        let clip = computed.nodes[id.index()].clip;
        geometry
            .origin
            .intersect(clip)
            .contains(point.0, point.1)
            .then_some(id)
    }

    fn multiline_scroll_geometry(&self, id: NodeId) -> Option<&EditingGeometry> {
        let node = self.dom.get(id).ok()?;
        let binding = self.bindings.get(&id)?;
        if node.role() != Role::TextInput
            || node.is_disabled()
            || !self.in_scope(id)
            || !binding.text_options.multiline
            || binding.multiline_text_viewport.is_none()
        {
            return None;
        }
        let key = node.stable_id()?;
        let geometry = self.text_geometry.get(key)?;
        (geometry.multiline && !self.editors.get(key)?.is_composing()).then_some(geometry)
    }

    pub(crate) fn multiline_scroll_position(&self, id: NodeId) -> Option<(f32, bool)> {
        let geometry = self.multiline_scroll_geometry(id)?;
        Some((geometry.scroll.1, geometry.manual_scroll))
    }

    pub(crate) fn restore_multiline_scroll(
        &mut self,
        id: NodeId,
        expected: (f32, bool),
        prior: (f32, bool),
    ) -> bool {
        if self.multiline_scroll_position(id) != Some(expected) {
            return false;
        }
        let key = self.dom.get(id).ok().and_then(|node| node.stable_id());
        let Some(geometry) = key.and_then(|key| self.text_geometry.get_mut(key)) else {
            return false;
        };
        let restored = (prior.0.clamp(0., geometry.vertical_limit()), prior.1);
        (geometry.scroll.1, geometry.manual_scroll) = restored;
        restored != expected
    }

    /// Move the inner text and return the part available to the ancestor.
    pub(crate) fn scroll_multiline(&mut self, id: NodeId, delta: f32) -> f32 {
        let Some(key) = self.dom.get(id).ok().and_then(|node| node.stable_id()) else {
            return delta;
        };
        let Some(geometry) = self.text_geometry.get_mut(key) else {
            return delta;
        };
        let offset = geometry.scroll.1;
        let limit = geometry.vertical_limit();
        let consumed = delta.clamp(-offset, (limit - offset).max(0.));
        if consumed != 0. {
            geometry.scroll.1 = (offset + consumed).clamp(0., limit);
            geometry.manual_scroll = true;
        }
        delta - consumed
    }

    pub(crate) fn multiline_caret(&self, key: &str) -> Option<Rect> {
        self.text_geometry.get(key)?.caret
    }

    pub(crate) fn first_text_line(&self, key: &str) -> Option<Rect> {
        let geometry = self.text_geometry.get(key)?;
        let mut line = geometry.origin;
        line.height = line.height.min(geometry.layout.line_height);
        Some(line)
    }

    pub(crate) fn text_input_hint_bounds(&self, key: &str) -> Option<Rect> {
        self.text_geometry.get(key)?.hint_bounds
    }

    pub(crate) fn editing_event(&mut self, event: Event) -> Result<bool, AppError> {
        if matches!(event, Event::Blur) {
            self.cancel_composition();
            self.focused = None;
            self.pressed = None;
            self.text_drag = None;
            self.range_drag = None;
            self.hovered = None;
            self.modal_outside = false;
            self.ime_area = None;
            return Ok(true);
        }
        let Some(id) = self.focused_node() else {
            return Ok(false);
        };
        let node = self.dom.get(id)?;
        if node.role() != Role::TextInput || node.is_disabled() {
            return Ok(false);
        }
        // A capture must never attach geometry from the previous preedit frame
        // to the new editor state. The next render supplies matching geometry.
        self.computed = None;
        self.ime_area = None;
        let stable = node.stable_id().expect("input needs ID").to_owned();
        let binding = self.bindings.get(&id).expect("bound node");
        let options = binding.text_options.clone();
        let visual_alignment = binding
            .multiline_text_viewport
            .as_ref()
            .filter(|_| options.multiline)
            .map(|(_, alignment)| *alignment);
        let handler = binding.input.clone();
        let editor = self
            .editors
            .get_mut(&stable)
            .expect("editor was reconciled");
        let reveal = matches!(event, Event::Edit { .. } | Event::Ime(_));
        let previous_selection = visual_alignment.map(|_| editor.selection().clone());
        let mut vertical = false;
        let changed = match event {
            Event::Text(text) => editor.insert(&text, &options),
            Event::Edit {
                key: key @ (EditKey::Up | EditKey::Down | EditKey::PageUp | EditKey::PageDown),
                extend,
                ..
            } if visual_alignment.is_some() || matches!(key, EditKey::Up | EditKey::Down) => {
                if !editor.is_composing() {
                    if let Some(g) = self.text_geometry.get_mut(&stable) {
                        let caret = g.layout.caret(editor.selection().cursor);
                        let height = g.layout.line_height;
                        let lines = if matches!(key, EditKey::PageUp | EditKey::PageDown) {
                            (g.origin.height / height).floor().max(1.)
                        } else {
                            1.
                        };
                        let dy = if matches!(key, EditKey::Up | EditKey::PageUp) {
                            -lines * height
                        } else {
                            lines * height
                        };
                        let x = if visual_alignment.is_some() {
                            vertical = true;
                            *g.vertical_goal.get_or_insert(caret.x)
                        } else {
                            caret.x
                        };
                        let cursor = g.layout.hit(x, caret.y + dy + caret.height / 2.);
                        editor.select(
                            if extend {
                                editor.selection().anchor
                            } else {
                                cursor
                            },
                            cursor,
                        );
                    }
                }
                false
            }
            Event::Edit {
                key: key @ (EditKey::Home | EditKey::End),
                extend,
                by_word: false,
            } if visual_alignment.is_some() && self.text_geometry.contains_key(&stable) => {
                if !editor.is_composing() {
                    if let Some(geometry) = self.text_geometry.get(&stable) {
                        let caret = geometry.layout.caret(editor.selection().cursor);
                        let rtl = visual_alignment == Some(TextAlignment::Right);
                        let right_edge = rtl != (key == EditKey::End);
                        let x = if right_edge {
                            geometry.origin.width + 1.
                        } else {
                            -1.
                        };
                        let cursor = geometry.layout.hit(x, caret.y + caret.height / 2.);
                        editor.select(
                            if extend {
                                editor.selection().anchor
                            } else {
                                cursor
                            },
                            cursor,
                        );
                    }
                }
                false
            }
            Event::Edit {
                key,
                extend,
                by_word,
            } => editor.key(key, extend, by_word, &options),
            Event::Ime(event) => editor.ime(event, &options),
            Event::InputValue {
                value,
                anchor,
                cursor,
            } => editor.replace_from_host(&value, anchor, cursor, &options),
            Event::Select { anchor, cursor } => {
                if !editor.is_composing() {
                    editor.select(anchor, cursor);
                }
                false
            }
            _ => return Ok(false),
        };
        if reveal || changed || previous_selection.is_some_and(|old| &old != editor.selection()) {
            if let Some(geometry) = self.text_geometry.get_mut(&stable) {
                geometry.manual_scroll = false;
                if !vertical {
                    geometry.vertical_goal = None;
                }
            }
            if visual_alignment.is_some() {
                self.pending_multiline_caret_reveal = Some(stable);
            }
        }
        let value = editor.value().to_owned();
        if changed {
            if let Some(handler) = handler {
                self.message(Some(handler(value)))?;
            }
        }
        Ok(true)
    }
    pub(crate) fn layout_editing(
        &mut self,
        width: f32,
        height: f32,
        text_scale: f32,
        measurer: &mut impl TextMeasurer,
    ) -> Result<(MicroDom, BTreeMap<NodeId, TextDecoration>, ComputedDom), AppError> {
        let mut display = self.dom.clone();
        for (key, editor) in &self.editors {
            if let Some(id) = display.by_id(key) {
                display.set_text(id, editor.display_value())?;
            }
        }
        if text_scale != 1. {
            for (id, node) in self.dom.iter() {
                let scaling = node.text_scaling_mode();
                if scaling == TextScaling::Fixed {
                    continue;
                }
                let has_text = node.text_value().is_some() || node.role() == Role::TextInput;
                let expand = scaling == TextScaling::Expand && text_scale > 1.;
                if !has_text && !expand {
                    continue;
                }
                let mut style = node.style().clone();
                if has_text {
                    let text_style = style.text_style().scaled(text_scale);
                    if expand && node.role() == Role::TextInput {
                        style.min_height = style.min_height.max(
                            text_style.line_height_px() + 2. * (style.padding + style.border_width),
                        );
                    }
                    style = style.with_text_style(text_style);
                }
                if expand {
                    if let Length::Px(height) = style.height {
                        style.min_height = style.min_height.max(height);
                        style.height = Length::Auto;
                    }
                }
                display.set_style(id, style)?;
            }
        }
        let computed = self.layout.compute(&display, width, height, measurer)?;
        let mut decorations = BTreeMap::new();
        self.ime_area = None;
        for (key, editor) in &self.editors {
            let Some(id) = display.by_id(key) else {
                continue;
            };
            let node = display.get(id)?;
            let binding = &self.bindings[&id];
            let options = &binding.text_options;
            let g = &computed.nodes[id.index()];
            let content_inset = if options.multiline {
                0.
            } else {
                binding.single_line_text_horizontal_inset
            };
            let padded = g.bounds.inset(node.style().padding);
            let multiline_viewport = binding
                .multiline_text_viewport
                .as_ref()
                .filter(|_| options.multiline);
            let alignment = multiline_viewport.map(|(_, alignment)| *alignment);
            let reference = if options.multiline {
                multiline_viewport.map(|(sibling, _)| sibling.as_str())
            } else {
                binding.single_line_text_horizontal_bounds.as_deref()
            };
            let measured = reference
                .and_then(|key| display.by_id(key))
                .filter(|sibling| {
                    *sibling != id
                        && node.parent().is_some()
                        && display.get(*sibling).is_ok_and(|reference| {
                            reference.role() == Role::Group
                                && reference.parent() == node.parent()
                                && !reference.is_focusable()
                        })
                        && !self.has_interaction_binding(*sibling)
                })
                .map(|sibling| computed.nodes[sibling.index()].bounds)
                .filter(|bounds| {
                    bounds.x.is_finite()
                        && bounds.width.is_finite()
                        && bounds.width > 0.
                        && bounds.x >= padded.x
                        && bounds.x + bounds.width <= padded.x + padded.width
                        && (!options.multiline
                            || (bounds.y.is_finite()
                                && bounds.height.is_finite()
                                && bounds.height > 0.
                                && bounds.y >= padded.y
                                && bounds.y + bounds.height <= padded.y + padded.height))
                });
            let mut origin = padded.inset_horizontal(content_inset);
            let measured_viewport = measured.filter(|_| options.multiline);
            let measured_horizontal = measured.filter(|_| !options.multiline);
            if let Some(viewport) = measured_viewport {
                origin = viewport;
            } else if let Some(bounds) = measured_horizontal {
                origin.x = bounds.x;
                origin.width = bounds.width;
            }
            let content_width = origin.width;
            let text = editor.display_value();
            let text_style = node.style().text_style();
            let layout = if let Some(alignment) = alignment {
                measurer.text_layout_aligned_styled(&text, &text_style, content_width, alignment)
            } else {
                measurer.text_layout_styled(
                    &text,
                    &text_style,
                    options.multiline.then_some(content_width),
                )
            };
            let placement = if options.multiline {
                SingleLineVerticalPlacement::Top
            } else {
                binding.single_line_vertical_placement
            };
            let extra_height = (origin.height - layout.line_height).max(0.);
            let (top_inset, center_inset) = match placement {
                SingleLineVerticalPlacement::Top => (0., 0.),
                SingleLineVerticalPlacement::Center => (0., extra_height / 2.),
                SingleLineVerticalPlacement::Inset(inset) => (inset.min(extra_height), 0.),
            };
            // Keep the unshifted paint viewport. An empty right-aligned
            // editor's caret width does not describe the hint's extent.
            let hint_viewport = Rect::new(
                origin.x,
                origin.y + top_inset,
                origin.width,
                origin.height - top_inset,
            );
            let right_aligned = binding.right_align_single_line_text && !options.multiline;
            let horizontal_space =
                right_aligned.then(|| right_alignment_space(content_width, &layout));
            let horizontal_inset = horizontal_space.unwrap_or_default().max(0.);
            origin.x += horizontal_inset;
            origin.width -= horizontal_inset;
            origin.y += top_inset + center_inset;
            origin.height -= top_inset + center_inset;
            let previous = self.text_geometry.get(key);
            let mut scroll = previous.map(|g| g.scroll).unwrap_or_default();
            if alignment.is_some() || horizontal_space.is_some_and(|space| space >= 0.) {
                scroll.0 = 0.;
            }
            if center_inset > 0. || matches!(placement, SingleLineVerticalPlacement::Inset(_)) {
                scroll.1 = 0.;
            }
            let selection = editor.display_selection();
            let focused = self.focused.as_ref() == Some(key);
            let same_measure = |geometry: &EditingGeometry| {
                geometry.origin.width == origin.width
                    && geometry.origin.height == origin.height
                    && geometry.layout.line_height == layout.line_height
            };
            let manual_scroll = alignment.is_some()
                && previous
                    .is_some_and(|g| g.manual_scroll && (g.focused || !focused) && same_measure(g));
            let vertical_goal = previous
                .filter(|g| g.multiline && g.focused && focused && same_measure(g))
                .and_then(|g| g.vertical_goal);
            if focused && !manual_scroll {
                if let Some(selection) = &selection {
                    let caret = layout.caret(selection.cursor);
                    if alignment.is_none() {
                        if caret.x < scroll.0 {
                            scroll.0 = caret.x;
                        } else if caret.x + CARET_REVEAL_WIDTH > scroll.0 + origin.width {
                            scroll.0 = caret.x + CARET_REVEAL_WIDTH - origin.width;
                        }
                    }
                    if caret.y < scroll.1 {
                        scroll.1 = caret.y;
                    } else if caret.y + caret.height > scroll.1 + origin.height {
                        scroll.1 = caret.y + caret.height - origin.height;
                    }
                }
            }
            scroll.0 = scroll.0.max(0.);
            scroll.1 = if alignment.is_some() {
                scroll.1.clamp(0., (layout.height - origin.height).max(0.))
            } else {
                scroll.1.max(0.)
            };
            let translate = |r: Rect| {
                Rect::new(
                    origin.x + r.x - scroll.0,
                    origin.y + r.y - scroll.1,
                    r.width,
                    r.height,
                )
            };
            let caret_position = |r| {
                let mut caret = translate(r);
                if alignment.is_some() {
                    let right = (origin.x + origin.width - caret.width).max(origin.x);
                    caret.x = caret.x.clamp(origin.x, right);
                }
                caret
            };
            let placeholder = binding.text_input_placeholder.as_ref().filter(|_| {
                editor.value().is_empty()
                    && !editor.is_composing()
                    && (binding.placeholder_presentation != PlaceholderPresentation::Focused
                        || focused)
            });
            let animated = binding.placeholder_presentation == PlaceholderPresentation::Animated;
            let hint_layout = placeholder
                .filter(|_| right_aligned || animated)
                .map(|hint| {
                    if let Some(alignment) = alignment {
                        measurer.text_layout_aligned_styled(
                            &hint.0,
                            &text_style,
                            content_width,
                            alignment,
                        )
                    } else {
                        measurer.text_layout_styled(
                            &hint.0,
                            &text_style,
                            options.multiline.then_some(content_width),
                        )
                    }
                });
            let paint_scroll_x = match &hint_layout {
                Some(layout) if right_aligned => {
                    let space = right_alignment_space(content_width, layout);
                    if animated && space < -CARET_REVEAL_WIDTH {
                        // An overflowing right-aligned hint starts at the
                        // right edge of the clip. Short hints keep caret room.
                        -space - CARET_REVEAL_WIDTH
                    } else {
                        -space.max(0.)
                    }
                }
                Some(_) if animated => 0.,
                _ => scroll.0 - horizontal_inset,
            };
            let mut decoration = TextDecoration {
                // Paint already uses the fixed top inset. Signed scroll
                // places centered and right-aligned text in that paint box.
                scroll: (paint_scroll_x, scroll.1 - center_inset),
                multiline: options.multiline,
                ..Default::default()
            };
            let mut multiline_caret = None;
            if focused {
                if let Some(selection) = selection {
                    decoration.selection = layout
                        .selection(selection.range())
                        .into_iter()
                        .map(translate)
                        .collect();
                    let caret = caret_position(layout.caret(selection.cursor));
                    if selection.is_collapsed() {
                        decoration.caret = Some(caret);
                    }
                    let visible = caret.intersect(origin.intersect(g.clip));
                    if !manual_scroll || (visible.width > 0. && visible.height > 0.) {
                        self.ime_area = Some(visible);
                    }
                    if alignment.is_some() && !manual_scroll {
                        let caret = caret.intersect(origin);
                        if caret.width > 0. && caret.height > 0. {
                            multiline_caret = Some(caret);
                        }
                    }
                }
                if let Some(preedit) = editor.preedit_range() {
                    decoration.underline = layout
                        .selection(preedit)
                        .into_iter()
                        .map(|r| translate(Rect::new(r.x, r.y + r.height - 2., r.width, 1.)))
                        .collect();
                }
            }
            self.text_geometry.insert(
                key.clone(),
                EditingGeometry {
                    layout,
                    origin,
                    scroll,
                    multiline: alignment.is_some(),
                    focused,
                    manual_scroll,
                    vertical_goal,
                    caret: multiline_caret,
                    hint_bounds: hint_layout
                        .as_ref()
                        .filter(|_| animated)
                        .and_then(|layout| {
                            hint_bounds(layout, hint_viewport, g.clip, decoration.scroll)
                        }),
                },
            );
            decorations.insert(id, decoration);
            if let Some(viewport) = measured_viewport {
                display.set_presented_text_input_viewport(
                    id,
                    Rect::new(
                        viewport.x - padded.x,
                        viewport.y - padded.y,
                        viewport.width,
                        viewport.height,
                    ),
                )?;
            } else if let Some(bounds) = measured_horizontal {
                display.set_presented_text_input_horizontal_bounds(
                    id,
                    bounds.x - padded.x,
                    bounds.width,
                )?;
            } else if content_inset > 0. {
                display.set_presented_text_input_horizontal_inset(id, content_inset)?;
            }
            if top_inset > 0. {
                display.set_presented_text_input_top_inset(id, top_inset)?;
            }
            if let Some(placeholder) = placeholder.filter(|_| !animated) {
                display.set_presented_text_input_placeholder(id, placeholder.clone())?;
            }
            if alignment.is_some() {
                display.set_presented_text_input_alignment(id, alignment)?;
            }
        }
        Ok((display, decorations, computed))
    }
}
