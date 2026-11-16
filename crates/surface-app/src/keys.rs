//! Keys an app sees in a `View::on_key` handler (issue #103).
//!
//! These are Surface's own types. Each host maps its platform key events
//! (winit, DOM) to them inside `surface-platform`.

/// The logical key of a press, after the keyboard layout is applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Space,
    /// A printable key, for example `"a"`, `"A"` or `"/"`.
    Character(String),
}

/// The modifier keys held during a press.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    /// Option on macOS.
    pub alt: bool,
    /// Command on macOS, the Windows key elsewhere.
    pub meta: bool,
}
impl Modifiers {
    /// True when no modifier is held.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// One key press delivered to a key handler.
///
/// A handler is never called while the focused field has an active IME
/// composition. The composition owns those keys: Enter commits the
/// candidate and never reaches the handler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyPress {
    pub key: Key,
    pub modifiers: Modifiers,
    /// True for an auto-repeat of a held key.
    pub repeat: bool,
}

/// Default high-level mappings for a chat-style text field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAction {
    /// Enter with no modifier.
    Submit,
    /// Shift+Enter with no other modifier.
    NewLine,
}

impl KeyPress {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            modifiers,
            repeat: false,
        }
    }
    /// This press with its auto-repeat flag set.
    pub fn with_repeat(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }
    /// The default mapping of this press, if any. Apps may ignore it and
    /// read `key` and `modifiers` directly.
    pub fn text_action(&self) -> Option<TextAction> {
        if self.key != Key::Enter {
            return None;
        }
        let shift_only = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        if self.modifiers.is_empty() {
            Some(TextAction::Submit)
        } else if self.modifiers == shift_only {
            Some(TextAction::NewLine)
        } else {
            None
        }
    }
}

/// A global shortcut: a key chord and the message it sends (issue #119).
///
/// `Key::Character` compares case-insensitively. Modifiers match exactly.
/// A chord with `alt` and a character key is not supported on macOS,
/// because Option changes the logical character. Named keys with `alt` work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shortcut<M> {
    pub key: Key,
    pub modifiers: Modifiers,
    pub message: M,
}
impl<M> Shortcut<M> {
    pub fn new(key: Key, modifiers: Modifiers, message: M) -> Self {
        Self {
            key,
            modifiers,
            message,
        }
    }
    pub(crate) fn matches(&self, press: &KeyPress) -> bool {
        let same_key = match (&self.key, &press.key) {
            (Key::Character(a), Key::Character(b)) => a.to_lowercase() == b.to_lowercase(),
            (a, b) => a == b,
        };
        same_key && self.modifiers == press.modifiers
    }
}

/// Characters that, with Control or Command, belong to a focused text
/// field: select all, copy, cut, paste, undo, redo.
pub(crate) const FIELD_CHORDS: [&str; 6] = ["a", "c", "x", "v", "z", "y"];

/// True when a text field owns this press, so no shortcut may match it:
/// a chord with no Control or Command, or one of `FIELD_CHORDS`.
pub(crate) fn field_owns(press: &KeyPress) -> bool {
    let command = press.modifiers.control || press.modifiers.meta;
    if !command {
        return true;
    }
    matches!(&press.key, Key::Character(c) if FIELD_CHORDS.contains(&c.to_lowercase().as_str()))
}
