//! Orders key presses against modifier changes.
//!
//! The winit web backend sends a key press before the `ModifiersChanged`
//! event that belongs to it. This type holds one press until the modifiers
//! arrive, or until the batch ends.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Mods {
    pub shift: bool,
    pub control: bool,
}

#[derive(Debug)]
pub(crate) struct KeyOrder<K> {
    held: Option<K>,
    mods: Mods,
}

impl<K> KeyOrder<K> {
    pub fn new() -> Self {
        Self {
            held: None,
            mods: Mods::default(),
        }
    }
    /// Holds `key`. Returns the key that was held before, with the current modifiers.
    pub fn press(&mut self, key: K) -> Option<(K, Mods)> {
        let out = self.flush();
        self.held = Some(key);
        out
    }
    /// Stores the modifiers. Returns the held key with them.
    pub fn modifiers(&mut self, mods: Mods) -> Option<(K, Mods)> {
        self.mods = mods;
        self.flush()
    }
    /// Returns the held key with the current modifiers.
    pub fn flush(&mut self) -> Option<(K, Mods)> {
        self.held.take().map(|key| (key, self.mods))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const NONE: Mods = Mods {
        shift: false,
        control: false,
    };
    const SHIFT: Mods = Mods {
        shift: true,
        control: false,
    };
    #[test]
    fn press_then_modifiers_gives_key_with_new_modifiers() {
        let mut o = KeyOrder::new();
        assert_eq!(o.press('t'), None);
        assert_eq!(o.modifiers(SHIFT), Some(('t', SHIFT)));
        assert_eq!(o.flush(), None);
    }
    #[test]
    fn modifiers_before_press_apply_to_each_key() {
        let mut o = KeyOrder::new();
        assert_eq!(o.modifiers(SHIFT), None);
        assert_eq!(o.press('a'), None);
        assert_eq!(o.press('b'), Some(('a', SHIFT)));
        assert_eq!(o.flush(), Some(('b', SHIFT)));
    }
    #[test]
    fn press_then_modifiers_none_clears_shift() {
        let mut o = KeyOrder::new();
        o.modifiers(SHIFT);
        o.press('t');
        assert_eq!(o.modifiers(NONE), Some(('t', NONE)));
    }
    #[test]
    fn press_then_flush_uses_current_modifiers() {
        let mut o = KeyOrder::new();
        o.modifiers(SHIFT);
        o.press('t');
        assert_eq!(o.flush(), Some(('t', SHIFT)));
    }
    #[test]
    fn two_batches_in_a_row() {
        let mut o = KeyOrder::new();
        o.press('t');
        assert_eq!(o.modifiers(SHIFT), Some(('t', SHIFT)));
        o.press('u');
        assert_eq!(o.modifiers(NONE), Some(('u', NONE)));
        assert_eq!(o.flush(), None);
    }
    #[test]
    fn flush_with_nothing_held_gives_nothing() {
        let mut o: KeyOrder<char> = KeyOrder::new();
        assert_eq!(o.flush(), None);
    }
}
