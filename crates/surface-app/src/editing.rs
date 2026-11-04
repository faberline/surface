//! Controlled Unicode editing. Composition never changes committed app state.
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextSelection {
    pub anchor: usize,
    pub cursor: usize,
}
impl TextSelection {
    pub fn range(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }
    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.cursor
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Composition {
    pub replacement: Range<usize>,
    pub text: String,
    /// Byte offsets into preedit text. None means the IME hides its cursor.
    pub selection: Option<(usize, usize)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKey {
    Left,
    Right,
    Home,
    End,
    Up,
    Down,
    Backspace,
    Delete,
    SelectAll,
    Undo,
    Redo,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImeEvent {
    Begin,
    Preedit {
        text: String,
        selection: Option<(usize, usize)>,
    },
    Commit(String),
    Cancel,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextOptions {
    pub multiline: bool,
    pub read_only: bool,
    pub max_graphemes: Option<usize>,
}
#[derive(Clone, Debug)]
struct Checkpoint {
    value: String,
    selection: TextSelection,
}
#[derive(Clone, Debug)]
pub struct TextEditor {
    value: String,
    selection: TextSelection,
    composition: Option<Composition>,
    undo: Vec<Checkpoint>,
    redo: Vec<Checkpoint>,
}
fn boundary(text: &str, index: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .take_while(|i| *i <= index.min(text.len()))
        .last()
        .unwrap_or(0)
}
impl TextEditor {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let end = value.len();
        Self {
            value,
            selection: TextSelection {
                anchor: end,
                cursor: end,
            },
            composition: None,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
    pub fn value(&self) -> &str {
        &self.value
    }
    pub fn selection(&self) -> &TextSelection {
        &self.selection
    }
    pub fn composition(&self) -> Option<&Composition> {
        self.composition.as_ref()
    }
    pub fn is_composing(&self) -> bool {
        self.composition.is_some()
    }
    pub fn sync(&mut self, value: &str) {
        if self.value != value {
            self.value = value.to_owned();
            self.composition = None;
            self.undo.clear();
            self.redo.clear();
            self.select(self.selection.anchor, self.selection.cursor);
        }
    }
    pub fn select(&mut self, anchor: usize, cursor: usize) {
        self.selection = TextSelection {
            anchor: boundary(&self.value, anchor),
            cursor: boundary(&self.value, cursor),
        };
    }
    pub fn selected_text(&self) -> &str {
        &self.value[self.selection.range()]
    }
    fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            value: self.value.clone(),
            selection: self.selection.clone(),
        }
    }
    fn remember(&mut self) {
        if self.undo.len() == 100 {
            self.undo.remove(0);
        }
        self.undo.push(self.checkpoint());
        self.redo.clear();
    }
    pub fn insert(&mut self, text: &str, options: &TextOptions) -> bool {
        if options.read_only || self.is_composing() {
            return false;
        }
        self.replace(self.selection.range(), text, options)
    }
    fn replace(&mut self, range: Range<usize>, text: &str, options: &TextOptions) -> bool {
        if options.read_only {
            return false;
        }
        let mut input: String = text
            .chars()
            .filter(|c| !c.is_control() || (options.multiline && matches!(*c, '\n' | '\t')))
            .collect();
        if let Some(limit) = options.max_graphemes {
            let retained = self.value[..range.start].graphemes(true).count()
                + self.value[range.end..].graphemes(true).count();
            input = input
                .graphemes(true)
                .take(limit.saturating_sub(retained))
                .collect();
        }
        if range.is_empty() && input.is_empty() {
            return false;
        }
        let end = range.start + input.len();
        if self.value[range.clone()] == input {
            self.select(end, end);
            return false;
        }
        self.remember();
        self.value.replace_range(range, &input);
        self.select(end, end);
        true
    }
    pub fn replace_from_host(
        &mut self,
        value: &str,
        anchor: usize,
        cursor: usize,
        options: &TextOptions,
    ) -> bool {
        if self.is_composing() || options.read_only {
            return false;
        }
        let before = self.value.clone();
        if self.value != value {
            // Browser input already applied a selection-aware operation. Still
            // enforce the same field policy as native input.
            let range = 0..self.value.len();
            self.replace(range, value, options);
        }
        self.select(anchor, cursor);
        self.value != before
    }
    pub fn ime(&mut self, event: ImeEvent, options: &TextOptions) -> bool {
        if options.read_only {
            self.composition = None;
            return false;
        }
        match event {
            ImeEvent::Begin => {
                if self.composition.is_none() {
                    self.composition = Some(Composition {
                        replacement: self.selection.range(),
                        text: String::new(),
                        selection: None,
                    });
                }
                false
            }
            ImeEvent::Preedit { text, selection } => {
                if self.composition.is_none() {
                    self.ime(ImeEvent::Begin, options);
                }
                let selection = selection.map(|(a, b)| (boundary(&text, a), boundary(&text, b)));
                let composition = self.composition.as_mut().expect("composition was started");
                composition.text = text;
                composition.selection = selection;
                false
            }
            ImeEvent::Commit(text) => {
                let range = self
                    .composition
                    .take()
                    .map(|c| c.replacement)
                    .unwrap_or_else(|| self.selection.range());
                self.replace(range, &text, options)
            }
            ImeEvent::Cancel => {
                self.composition = None;
                false
            }
        }
    }
    pub fn display_value(&self) -> String {
        if let Some(c) = &self.composition {
            if !c.text.is_empty() {
                return format!(
                    "{}{}{}",
                    &self.value[..c.replacement.start],
                    c.text,
                    &self.value[c.replacement.end..]
                );
            }
        }
        self.value.clone()
    }
    pub fn display_selection(&self) -> Option<TextSelection> {
        if let Some(c) = &self.composition {
            return c.selection.map(|(a, b)| TextSelection {
                anchor: c.replacement.start + a,
                cursor: c.replacement.start + b,
            });
        }
        Some(self.selection.clone())
    }
    pub fn preedit_range(&self) -> Option<Range<usize>> {
        self.composition
            .as_ref()
            .filter(|c| !c.text.is_empty())
            .map(|c| c.replacement.start..c.replacement.start + c.text.len())
    }
    pub fn key(
        &mut self,
        key: EditKey,
        extend: bool,
        by_word: bool,
        options: &TextOptions,
    ) -> bool {
        if self.is_composing() {
            return false;
        }
        let cursor = self.selection.cursor;
        let previous = self.value[..cursor]
            .grapheme_indices(true)
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        let next = self.value[cursor..]
            .graphemes(true)
            .next()
            .map(|s| cursor + s.len())
            .unwrap_or(cursor);
        match key {
            EditKey::Undo | EditKey::Redo if !options.read_only => {
                let checkpoint = if key == EditKey::Undo {
                    self.undo.pop()
                } else {
                    self.redo.pop()
                };
                let Some(checkpoint) = checkpoint else {
                    return false;
                };
                let current = self.checkpoint();
                if key == EditKey::Undo {
                    self.redo.push(current);
                } else {
                    self.undo.push(current);
                }
                self.value = checkpoint.value;
                self.selection = checkpoint.selection;
                true
            }
            EditKey::Backspace | EditKey::Delete => {
                let selection = self.selection.range();
                let range = if !selection.is_empty() {
                    selection
                } else if key == EditKey::Backspace {
                    (if by_word {
                        self.value[..cursor]
                            .split_word_bound_indices()
                            .rfind(|(_, s)| !s.trim().is_empty())
                            .map(|(i, _)| i)
                            .unwrap_or(0)
                    } else {
                        previous
                    })..cursor
                } else {
                    cursor..(if by_word {
                        self.value[cursor..]
                            .split_word_bound_indices()
                            .find(|(_, s)| !s.trim().is_empty())
                            .map(|(i, s)| cursor + i + s.len())
                            .unwrap_or(self.value.len())
                    } else {
                        next
                    })
                };
                self.replace(range, "", options)
            }
            EditKey::SelectAll => {
                self.select(0, self.value.len());
                false
            }
            _ => {
                let range = self.selection.range();
                let target = match key {
                    EditKey::Left if !extend && !range.is_empty() => range.start,
                    EditKey::Right if !extend && !range.is_empty() => range.end,
                    EditKey::Left if by_word => self.value[..cursor]
                        .split_word_bound_indices()
                        .rfind(|(_, s)| !s.trim().is_empty())
                        .map(|(i, _)| i)
                        .unwrap_or(0),
                    EditKey::Right if by_word => self.value[cursor..]
                        .split_word_bound_indices()
                        .find(|(_, s)| !s.trim().is_empty())
                        .map(|(i, s)| cursor + i + s.len())
                        .unwrap_or(self.value.len()),
                    EditKey::Left => previous,
                    EditKey::Right => next,
                    EditKey::Home => self.value[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0),
                    EditKey::End => self.value[cursor..]
                        .find('\n')
                        .map(|i| cursor + i)
                        .unwrap_or(self.value.len()),
                    _ => return false,
                };
                self.select(
                    if extend {
                        self.selection.anchor
                    } else {
                        target
                    },
                    target,
                );
                false
            }
        }
    }
}

/// Browser selections use UTF-16 code units; runtime selections use UTF-8 bytes.
pub fn utf16_to_utf8(text: &str, index: usize) -> usize {
    let mut units = 0;
    for (byte, c) in text.char_indices() {
        if units + c.len_utf16() > index {
            return byte;
        }
        units += c.len_utf16();
    }
    text.len()
}
pub fn utf8_to_utf16(text: &str, index: usize) -> usize {
    text[..boundary(text, index)].encode_utf16().count()
}
