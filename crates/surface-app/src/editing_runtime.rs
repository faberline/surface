use crate::{AppError, Application, EditKey, Event, ImeEvent, Runtime, TextEditor, TextSelection};
use std::collections::BTreeMap;
use surface::{
    ComputedDom, Length, MicroDom, NodeId, Rect, Role, TextDecoration, TextLayout, TextMeasurer,
    TextScaling,
};

const CARET_REVEAL_WIDTH: f32 = 2.;

fn right_alignment_space(content_width: f32, layout: &TextLayout) -> f32 {
    let text_right = layout.clusters.iter().fold(0_f32, |right, cluster| {
        right.max(cluster.bounds.x + cluster.bounds.width)
    });
    content_width - text_right - CARET_REVEAL_WIDTH
}

pub(crate) struct EditingGeometry {
    layout: TextLayout,
    origin: Rect,
    scroll: (f32, f32),
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
                editor.sync(value);
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
            (self.text_geometry.get(&id), self.editors.get_mut(&id))
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
        self.text_drag = Some(id);
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
        let handler = binding.input.clone();
        let editor = self
            .editors
            .get_mut(&stable)
            .expect("editor was reconciled");
        let up = matches!(
            event,
            Event::Edit {
                key: EditKey::Up,
                ..
            }
        );
        let changed = match event {
            Event::Text(text) => editor.insert(&text, &options),
            Event::Edit {
                key: EditKey::Up,
                extend,
                ..
            }
            | Event::Edit {
                key: EditKey::Down,
                extend,
                ..
            } => {
                if !editor.is_composing() {
                    if let Some(g) = self.text_geometry.get(&stable) {
                        let caret = g.layout.caret(editor.selection().cursor);
                        let dy = if up {
                            -g.layout.line_height
                        } else {
                            g.layout.line_height
                        };
                        let cursor = g
                            .layout
                            .hit(caret.x, caret.y + dy + g.layout.line_height / 2.);
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
            let mut origin = g
                .bounds
                .inset(node.style().padding)
                .inset_horizontal(content_inset);
            let content_width = origin.width;
            let text = editor.display_value();
            let text_style = node.style().text_style();
            let layout = measurer.text_layout_styled(
                &text,
                &text_style,
                if options.multiline {
                    Some(content_width)
                } else {
                    None
                },
            );
            let vertical_inset = if binding.center_single_line_text && !options.multiline {
                (origin.height - layout.line_height).max(0.) / 2.
            } else {
                0.
            };
            let right_aligned = binding.right_align_single_line_text && !options.multiline;
            let horizontal_space =
                right_aligned.then(|| right_alignment_space(content_width, &layout));
            let horizontal_inset = horizontal_space.unwrap_or_default().max(0.);
            origin.x += horizontal_inset;
            origin.width -= horizontal_inset;
            origin.y += vertical_inset;
            origin.height -= vertical_inset;
            let mut scroll = self
                .text_geometry
                .get(key)
                .map(|g| g.scroll)
                .unwrap_or_default();
            if horizontal_space.is_some_and(|space| space >= 0.) {
                scroll.0 = 0.;
            }
            if vertical_inset > 0. {
                scroll.1 = 0.;
            }
            let selection = editor.display_selection();
            let focused = self.focused.as_ref() == Some(key);
            if focused {
                if let Some(selection) = &selection {
                    let caret = layout.caret(selection.cursor);
                    if caret.x < scroll.0 {
                        scroll.0 = caret.x;
                    } else if caret.x + CARET_REVEAL_WIDTH > scroll.0 + origin.width {
                        scroll.0 = caret.x + CARET_REVEAL_WIDTH - origin.width;
                    }
                    if caret.y < scroll.1 {
                        scroll.1 = caret.y;
                    } else if caret.y + caret.height > scroll.1 + origin.height {
                        scroll.1 = caret.y + caret.height - origin.height;
                    }
                }
            }
            scroll.0 = scroll.0.max(0.);
            scroll.1 = scroll.1.max(0.);
            let translate = |r: Rect| {
                Rect::new(
                    origin.x + r.x - scroll.0,
                    origin.y + r.y - scroll.1,
                    r.width,
                    r.height,
                )
            };
            let placeholder = binding.text_input_placeholder.as_ref().filter(|_| {
                editor.value().is_empty()
                    && !editor.is_composing()
                    && (!binding.focused_placeholder || self.focused() == Some(key.as_str()))
            });
            let paint_scroll_x = if let (true, Some(hint)) = (right_aligned, placeholder) {
                let hint_layout = measurer.text_layout_styled(&hint.0, &text_style, None);
                -right_alignment_space(content_width, &hint_layout).max(0.)
            } else {
                scroll.0 - horizontal_inset
            };
            let mut decoration = TextDecoration {
                // Paint starts at the original content box. Signed scroll
                // places its run at the editor or placeholder origin.
                scroll: (paint_scroll_x, scroll.1 - vertical_inset),
                multiline: options.multiline,
                ..Default::default()
            };
            if focused {
                if let Some(selection) = selection {
                    decoration.selection = layout
                        .selection(selection.range())
                        .into_iter()
                        .map(translate)
                        .collect();
                    if selection.is_collapsed() {
                        decoration.caret = Some(translate(layout.caret(selection.cursor)));
                    }
                    self.ime_area = Some(
                        translate(layout.caret(selection.cursor))
                            .intersect(origin.intersect(g.clip)),
                    );
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
                },
            );
            decorations.insert(id, decoration);
            if content_inset > 0. {
                display.set_presented_text_input_horizontal_inset(id, content_inset)?;
            }
            if let Some(placeholder) = placeholder {
                display.set_presented_text_input_placeholder(id, placeholder.clone())?;
            }
        }
        Ok((display, decorations, computed))
    }
}
