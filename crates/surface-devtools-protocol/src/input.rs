//! Serde mirror of the input events of `surface_app::Event`. Type only.
//! Conversion lives with the executor, because it needs `surface-app`.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditKey {
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Backspace,
    Delete,
    SelectAll,
    Undo,
    Redo,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImeEvent {
    Begin,
    Preedit {
        text: String,
        selection: Option<(usize, usize)>,
    },
    Commit {
        text: String,
    },
    Cancel,
}

/// `Text(String)` and `Ime(ImeEvent)` become struct variants so the tag works.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputEvent {
    PointerDown {
        x: f32,
        y: f32,
    },
    PointerUp {
        x: f32,
        y: f32,
    },
    PointerMove {
        x: f32,
        y: f32,
    },
    PointerLeave,
    FocusNext {
        reverse: bool,
    },
    Activate,
    Text {
        text: String,
    },
    Backspace,
    Edit {
        key: EditKey,
        extend: bool,
        by_word: bool,
    },
    Ime {
        event: ImeEvent,
    },
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
