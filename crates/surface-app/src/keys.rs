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
        Self { key, modifiers }
    }
    /// The default mapping of this press, if any. Apps may ignore it and
    /// read `key` and `modifiers` directly.
    pub fn text_action(&self) -> Option<TextAction> {
        None
    }
}
