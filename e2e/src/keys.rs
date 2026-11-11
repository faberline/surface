//! Key definitions for CDP `Input.dispatchKeyEvent`.
//!
//! This mirrors what the old Playwright scripts sent: the US layout key,
//! code, Windows key code and text, the modifier bitmask, and the macOS
//! editing commands Chrome needs for keys such as Meta+A.

use anyhow::{bail, Result};

/// CDP modifier bits.
pub const ALT: i64 = 1;
pub const CONTROL: i64 = 2;
pub const META: i64 = 4;
pub const SHIFT: i64 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    pub key: String,
    pub code: String,
    pub key_code: i64,
    pub text: String,
    pub location: i64,
}

/// One parsed combination such as `Shift+Tab` or `Meta+A`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chord {
    /// Modifier keys in press order.
    pub modifiers: Vec<Key>,
    pub key: Key,
}

impl Chord {
    pub fn parse(combo: &str) -> Result<Self> {
        let parts: Vec<&str> = if combo == "+" {
            vec!["+"]
        } else {
            combo.split('+').collect()
        };
        let Some((last, mods)) = parts.split_last() else {
            bail!("empty key combination");
        };
        let modifiers = mods.iter().map(|m| key(m)).collect::<Result<Vec<_>>>()?;
        for m in &modifiers {
            if modifier_bit(&m.key) == 0 {
                bail!("`{}` is not a modifier in `{combo}`", m.key);
            }
        }
        Ok(Self {
            modifiers,
            key: key(last)?,
        })
    }
}

pub fn modifier_bit(key: &str) -> i64 {
    match key {
        "Alt" => ALT,
        "Control" => CONTROL,
        "Meta" => META,
        "Shift" => SHIFT,
        _ => 0,
    }
}

/// The platform's primary shortcut modifier, as the old scripts chose it.
pub fn primary() -> &'static str {
    if cfg!(target_os = "macos") {
        "Meta"
    } else {
        "Control"
    }
}

pub fn key(name: &str) -> Result<Key> {
    let named = |key: &str, code: &str, key_code: i64, text: &str, location: i64| Key {
        key: key.into(),
        code: code.into(),
        key_code,
        text: text.into(),
        location,
    };
    Ok(match name {
        "Tab" => named("Tab", "Tab", 9, "", 0),
        "Enter" => named("Enter", "Enter", 13, "\r", 0),
        "Backspace" => named("Backspace", "Backspace", 8, "", 0),
        "Delete" => named("Delete", "Delete", 46, "", 0),
        "Escape" => named("Escape", "Escape", 27, "", 0),
        "Space" | " " => named(" ", "Space", 32, " ", 0),
        "ArrowLeft" => named("ArrowLeft", "ArrowLeft", 37, "", 0),
        "ArrowUp" => named("ArrowUp", "ArrowUp", 38, "", 0),
        "ArrowRight" => named("ArrowRight", "ArrowRight", 39, "", 0),
        "ArrowDown" => named("ArrowDown", "ArrowDown", 40, "", 0),
        "Home" => named("Home", "Home", 36, "", 0),
        "End" => named("End", "End", 35, "", 0),
        "Shift" => named("Shift", "ShiftLeft", 16, "", 1),
        "Control" => named("Control", "ControlLeft", 17, "", 1),
        "Alt" => named("Alt", "AltLeft", 18, "", 1),
        "Meta" => named("Meta", "MetaLeft", 91, "", 1),
        single => {
            let mut chars = single.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else {
                bail!("unknown key `{name}`");
            };
            let text = c.to_string();
            if c.is_ascii_alphabetic() {
                let upper = c.to_ascii_uppercase();
                Key {
                    key: text.clone(),
                    code: format!("Key{upper}"),
                    key_code: upper as i64,
                    text,
                    location: 0,
                }
            } else if c.is_ascii_digit() {
                Key {
                    key: text.clone(),
                    code: format!("Digit{c}"),
                    key_code: c as i64,
                    text,
                    location: 0,
                }
            } else {
                bail!("unknown key `{name}`");
            }
        }
    })
}

/// macOS editing commands for a key, as Playwright sends them. Commands
/// that insert text are left out because the key event inserts it.
pub fn mac_commands(code: &str, modifiers: i64) -> Vec<&'static str> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }
    let mut shortcut = String::new();
    for (bit, name) in [
        (SHIFT, "Shift"),
        (CONTROL, "Control"),
        (ALT, "Alt"),
        (META, "Meta"),
    ] {
        if modifiers & bit != 0 {
            shortcut.push_str(name);
            shortcut.push('+');
        }
    }
    shortcut.push_str(code);
    let command = match shortcut.as_str() {
        "Backspace" | "Shift+Backspace" => "deleteBackward",
        "Delete" => "deleteForward",
        "Escape" | "Shift+Escape" => "cancelOperation",
        "ArrowUp" => "moveUp",
        "ArrowDown" => "moveDown",
        "ArrowLeft" => "moveLeft",
        "ArrowRight" => "moveRight",
        "Home" => "scrollToBeginningOfDocument",
        "End" => "scrollToEndOfDocument",
        "Meta+KeyA" => "selectAll",
        "Meta+KeyZ" => "undo",
        "Shift+Meta+KeyZ" => "redo",
        _ => return Vec::new(),
    };
    vec![command]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modifier_chords() {
        let chord = Chord::parse("Shift+Tab").unwrap();
        assert_eq!(
            chord
                .modifiers
                .iter()
                .map(|m| modifier_bit(&m.key))
                .sum::<i64>(),
            SHIFT
        );
        assert_eq!(chord.key.key_code, 9);
        let chord = Chord::parse("Meta+Shift+Z").unwrap();
        assert_eq!(
            chord
                .modifiers
                .iter()
                .map(|m| modifier_bit(&m.key))
                .sum::<i64>(),
            META | SHIFT
        );
        assert_eq!(chord.key.code, "KeyZ");
        assert_eq!(chord.key.key, "Z");
    }

    #[test]
    fn rejects_unknown_keys_and_non_modifiers() {
        assert!(Chord::parse("Hyper").is_err());
        assert!(Chord::parse("Tab+A").is_err());
    }

    #[test]
    fn printable_keys_carry_text() {
        assert_eq!(key("Enter").unwrap().text, "\r");
        assert_eq!(key("Space").unwrap().text, " ");
        assert_eq!(key("c").unwrap().key_code, 67);
        assert_eq!(key("Tab").unwrap().text, "");
    }

    #[test]
    fn mac_commands_follow_playwright() {
        if cfg!(target_os = "macos") {
            assert_eq!(mac_commands("KeyA", META), vec!["selectAll"]);
            assert_eq!(mac_commands("KeyZ", META | SHIFT), vec!["redo"]);
            assert!(mac_commands("Enter", 0).is_empty());
            assert!(mac_commands("Tab", SHIFT).is_empty());
        } else {
            assert!(mac_commands("KeyA", META).is_empty());
        }
    }
}
