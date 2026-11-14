//! Host decisions for the app key handler (issue #103). Pure, so they run
//! without a window. Both hosts map their platform key to a `KeyPress`
//! here, ask `route_key`, and run the field's own key handling only on
//! `KeyStep::Pass`.
use surface_app::{Application, EditKey, Event, InputSnapshot, Key, KeyPress, Modifiers, Runtime};
use winit::keyboard::{Key as WinitKey, ModifiersState, NamedKey};

/// What the host does with a key press after asking the app.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum KeyStep {
    /// The app's handler consumed the press. Skip the field.
    Consumed,
    /// Give the press to the field, as without a handler.
    Pass,
    Fail(String),
}

/// Ask the focused node's handler first. `composing` is the host's own
/// composition state (the platform IME session or the DOM composition
/// flags). During a composition the handler is not called; the IME owns
/// the key, and Enter commits the candidate.
pub(crate) fn route_key<A: Application>(
    runtime: &mut Runtime<A>,
    press: Option<KeyPress>,
    composing: bool,
) -> KeyStep {
    let Some(press) = press.filter(|_| !composing) else {
        return KeyStep::Pass;
    };
    match runtime.key(&press) {
        Ok(true) => KeyStep::Consumed,
        Ok(false) => KeyStep::Pass,
        Err(error) => KeyStep::Fail(error.to_string()),
    }
}

/// Map winit modifiers. Command is `meta`, Option is `alt`.
pub(crate) fn from_winit_mods(state: ModifiersState) -> Modifiers {
    Modifiers {
        shift: state.shift_key(),
        control: state.control_key(),
        alt: state.alt_key(),
        meta: state.super_key(),
    }
}

/// Map a winit logical key. Keys with no Surface name return `None` and
/// skip the handler.
pub(crate) fn from_winit(key: &WinitKey) -> Option<Key> {
    match key {
        WinitKey::Named(named) => from_named(*named),
        WinitKey::Character(text) => Some(Key::Character(text.to_string())),
        _ => None,
    }
}

fn from_named(key: NamedKey) -> Option<Key> {
    Some(match key {
        NamedKey::Enter => Key::Enter,
        NamedKey::Escape => Key::Escape,
        NamedKey::Tab => Key::Tab,
        NamedKey::Backspace => Key::Backspace,
        NamedKey::Delete => Key::Delete,
        NamedKey::ArrowUp => Key::ArrowUp,
        NamedKey::ArrowDown => Key::ArrowDown,
        NamedKey::ArrowLeft => Key::ArrowLeft,
        NamedKey::ArrowRight => Key::ArrowRight,
        NamedKey::Home => Key::Home,
        NamedKey::End => Key::End,
        NamedKey::PageUp => Key::PageUp,
        NamedKey::PageDown => Key::PageDown,
        NamedKey::Space => Key::Space,
        _ => return None,
    })
}

/// Map a DOM `KeyboardEvent.key`.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn from_dom(key: &str) -> Option<Key> {
    Some(match key {
        "Enter" => Key::Enter,
        "Escape" => Key::Escape,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "ArrowUp" => Key::ArrowUp,
        "ArrowDown" => Key::ArrowDown,
        "ArrowLeft" => Key::ArrowLeft,
        "ArrowRight" => Key::ArrowRight,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        " " => Key::Space,
        // Named keys are longer than one character: "Shift", "Process".
        text if text.chars().count() == 1 => Key::Character(text.to_owned()),
        _ => return None,
    })
}

/// True when a DOM keydown belongs to an IME composition. Safari sends the
/// Enter that commits a candidate after `compositionend`, with
/// `isComposing` false and `keyCode` 229, so the code is checked too.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn dom_composing(is_composing: bool, key_code: u32, host_composing: bool) -> bool {
    is_composing || key_code == 229 || host_composing
}

/// The native field's own handling of a key press: what the host did
/// before #103, unchanged.
#[derive(Debug)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) enum FieldAction {
    Dispatch(Event),
    /// Copy, cut or paste: `"c"`, `"x"` or `"v"`.
    Clipboard(String),
    Nothing,
}

/// `command` is Control or Command, the shortcut modifier.
pub(crate) fn field_action(
    key: &WinitKey,
    text: Option<&str>,
    mods: Modifiers,
    input: Option<&InputSnapshot>,
) -> FieldAction {
    let shift = mods.shift;
    let command = mods.control || mods.meta;
    if command {
        if let WinitKey::Character(key) = key {
            let edit = match key.to_lowercase().as_str() {
                "a" => Some(EditKey::SelectAll),
                "z" => Some(if shift { EditKey::Redo } else { EditKey::Undo }),
                "y" => Some(EditKey::Redo),
                _ => None,
            };
            if let Some(key) = edit {
                return FieldAction::Dispatch(Event::Edit {
                    key,
                    extend: false,
                    by_word: false,
                });
            }
            #[cfg(not(target_arch = "wasm32"))]
            if matches!(key.to_lowercase().as_str(), "c" | "x" | "v") {
                return FieldAction::Clipboard(key.to_lowercase());
            }
        }
    }
    let edit = match key {
        WinitKey::Named(NamedKey::ArrowLeft) => Some(EditKey::Left),
        WinitKey::Named(NamedKey::ArrowRight) => Some(EditKey::Right),
        WinitKey::Named(NamedKey::ArrowUp) => Some(EditKey::Up),
        WinitKey::Named(NamedKey::ArrowDown) => Some(EditKey::Down),
        WinitKey::Named(NamedKey::Home) => Some(EditKey::Home),
        WinitKey::Named(NamedKey::End) => Some(EditKey::End),
        WinitKey::Named(NamedKey::PageUp) => Some(EditKey::PageUp),
        WinitKey::Named(NamedKey::PageDown) => Some(EditKey::PageDown),
        WinitKey::Named(NamedKey::Backspace) => Some(EditKey::Backspace),
        WinitKey::Named(NamedKey::Delete) => Some(EditKey::Delete),
        _ => None,
    };
    if let Some(key) = edit {
        return FieldAction::Dispatch(Event::Edit {
            key,
            extend: shift,
            by_word: command,
        });
    }
    let event = match key {
        WinitKey::Named(NamedKey::Tab) => Some(Event::FocusNext { reverse: shift }),
        WinitKey::Named(NamedKey::Enter) if input.is_some_and(|i| i.multiline) => {
            Some(Event::Text("\n".to_owned()))
        }
        WinitKey::Named(NamedKey::Enter) => Some(Event::Activate),
        WinitKey::Named(NamedKey::Escape) => Some(Event::Cancel),
        WinitKey::Named(NamedKey::Space) if input.is_none() => Some(Event::Activate),
        _ if !command => text.map(|text| Event::Text(text.to_owned())),
        _ => None,
    };
    event.map_or(FieldAction::Nothing, FieldAction::Dispatch)
}

/// What the native host does with one key press. `key_input` only applies it.
#[derive(Debug)]
pub(crate) enum NativeStep {
    /// The focused field is composing. The IME owns the key.
    Ignore,
    /// The app's handler consumed the press: redraw and sync the IME area.
    Consumed,
    /// The field's own handling, as without a handler.
    Field(FieldAction),
    Fail(String),
}

/// The native host's whole key decision (#103): the composition check, the
/// app's handler, then the field.
pub(crate) fn native_key<A: Application>(
    runtime: &mut Runtime<A>,
    key: &WinitKey,
    text: Option<&str>,
    mods: Modifiers,
) -> NativeStep {
    let input = runtime.input_snapshot();
    if input.as_ref().is_some_and(|i| i.composing) {
        return NativeStep::Ignore;
    }
    let press = from_winit(key).map(|k| KeyPress::new(k, mods));
    match route_key(runtime, press, false) {
        KeyStep::Consumed => return NativeStep::Consumed,
        KeyStep::Fail(error) => return NativeStep::Fail(error),
        KeyStep::Pass => {}
    }
    NativeStep::Field(field_action(key, text, mods, input.as_ref()))
}

/// What the web textarea's `keydown` listener does. The listener only
/// applies it, in this order: `preventDefault`, dispatch, value sync.
#[derive(Debug, Default)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) struct WebStep {
    pub prevent_default: bool,
    pub dispatch: Option<Event>,
    /// Write the runtime value and selection back to the textarea.
    pub sync: bool,
    pub error: Option<String>,
}

/// The web host's whole keydown decision (#103). `key` is
/// `KeyboardEvent.key`. `bridge_composing` is the bridge's own composition
/// flag. A passed key keeps the browser default unless the field maps it,
/// so Shift+Enter in a multiline field inserts the newline in the browser.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn web_key<A: Application>(
    runtime: &mut Runtime<A>,
    key: &str,
    key_code: u32,
    is_composing: bool,
    bridge_composing: bool,
    mods: Modifiers,
) -> WebStep {
    if is_composing || bridge_composing {
        return WebStep::default();
    }
    let press = from_dom(key).map(|k| KeyPress::new(k, mods));
    let ime = dom_composing(is_composing, key_code, bridge_composing);
    match route_key(runtime, press, ime) {
        KeyStep::Consumed => {
            return WebStep {
                prevent_default: true,
                sync: true,
                ..WebStep::default()
            }
        }
        KeyStep::Fail(error) => {
            return WebStep {
                error: Some(error),
                ..WebStep::default()
            }
        }
        KeyStep::Pass => {}
    }
    if (mods.meta || mods.control) && matches!(key.to_lowercase().as_str(), "z" | "y") {
        let redo = mods.shift || key.eq_ignore_ascii_case("y");
        return WebStep {
            prevent_default: true,
            dispatch: Some(Event::Edit {
                key: if redo { EditKey::Redo } else { EditKey::Undo },
                extend: false,
                by_word: false,
            }),
            sync: true,
            error: None,
        };
    }
    let dispatch = match key {
        "Tab" => Some(Event::FocusNext {
            reverse: mods.shift,
        }),
        "Escape" => Some(Event::Cancel),
        "Enter" if runtime.input_snapshot().is_some_and(|s| !s.multiline) => Some(Event::Activate),
        _ => None,
    };
    WebStep {
        prevent_default: dispatch.is_some(),
        dispatch,
        sync: false,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    use surface::{Node, Role};
    use surface_app::{ImeEvent, TextAction, TextOptions, View};

    #[derive(Clone)]
    enum Msg {
        Text(String),
        Send,
    }
    struct App {
        draft: String,
        multiline: bool,
        handler: bool,
        sent: u32,
        calls: Rc<Cell<u32>>,
    }
    impl Application for App {
        type Message = Msg;
        fn update(&mut self, message: Msg) {
            match message {
                Msg::Text(text) => self.draft = text,
                Msg::Send => self.sent += 1,
            }
        }
        fn view(&self) -> View<Msg> {
            let field = View::new(
                Node::new(Role::TextInput, "Message")
                    .id("chat")
                    .text(&self.draft),
            )
            .text_options(TextOptions {
                multiline: self.multiline,
                ..TextOptions::default()
            })
            .on_input(Msg::Text);
            if !self.handler {
                return field;
            }
            let calls = self.calls.clone();
            field.on_key(move |press| {
                calls.set(calls.get() + 1);
                (press.text_action() == Some(TextAction::Submit)).then_some(Msg::Send)
            })
        }
    }
    fn runtime(multiline: bool, handler: bool) -> Runtime<App> {
        let mut rt = Runtime::new(App {
            draft: String::new(),
            multiline,
            handler,
            sent: 0,
            calls: Rc::default(),
        })
        .unwrap();
        rt.activate("chat").unwrap();
        rt.dispatch(Event::Text("hi".into())).unwrap();
        rt
    }
    const NONE: Modifiers = Modifiers {
        shift: false,
        control: false,
        alt: false,
        meta: false,
    };
    const SHIFT: Modifiers = Modifiers {
        shift: true,
        ..NONE
    };
    const ENTER: WinitKey = WinitKey::Named(NamedKey::Enter);

    /// Run `native_key` and apply its step, as `key_input` does.
    fn native_press(
        rt: &mut Runtime<App>,
        key: &WinitKey,
        text: Option<&str>,
        mods: Modifiers,
    ) -> String {
        let step = native_key(rt, key, text, mods);
        let shown = format!("{step:?}");
        match step {
            NativeStep::Field(FieldAction::Dispatch(event)) => {
                rt.dispatch(event).unwrap();
            }
            NativeStep::Fail(error) => panic!("{error}"),
            _ => {}
        }
        shown
    }
    /// Run `web_key` and apply its step, as the keydown listener does. The
    /// browser default for a passed key is not simulated.
    fn web_press(
        rt: &mut Runtime<App>,
        key: &str,
        key_code: u32,
        ime: bool,
        mods: Modifiers,
    ) -> WebStep {
        let step = web_key(rt, key, key_code, ime, false, mods);
        if let Some(event) = step.dispatch.clone() {
            rt.dispatch(event).unwrap();
        }
        assert!(step.error.is_none(), "{:?}", step.error);
        step
    }

    #[test]
    fn native_enter_is_consumed_by_the_handler_and_inserts_no_newline() {
        let mut rt = runtime(true, true);
        assert_eq!(native_press(&mut rt, &ENTER, Some("\r"), NONE), "Consumed");
        assert_eq!(rt.app().sent, 1);
        assert_eq!(rt.app().draft, "hi");
    }

    #[test]
    fn native_shift_enter_passes_and_the_field_inserts_a_newline() {
        let mut rt = runtime(true, true);
        native_press(&mut rt, &ENTER, Some("\r"), SHIFT);
        assert_eq!(rt.app().sent, 0);
        assert_eq!(rt.app().draft, "hi\n");
        assert_eq!(rt.app().calls.get(), 1);
    }

    #[test]
    fn native_enter_during_composition_skips_the_handler() {
        let mut rt = runtime(true, true);
        rt.dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄓ".into(),
            selection: None,
        }))
        .unwrap();
        assert_eq!(native_press(&mut rt, &ENTER, Some("\r"), NONE), "Ignore");
        assert_eq!(rt.app().calls.get(), 0);
        assert_eq!(rt.app().sent, 0);
        assert_eq!(rt.app().draft, "hi", "no newline during composition");
    }

    #[test]
    fn the_host_composition_flag_alone_skips_the_handler() {
        // The DOM can report a composition that the runtime has not seen
        // yet (Safari's Enter after compositionend). The host flag wins.
        let mut rt = runtime(true, true);
        assert!(!rt.input_snapshot().unwrap().composing);
        assert_eq!(
            route_key(&mut rt, Some(KeyPress::new(Key::Enter, NONE)), true),
            KeyStep::Pass
        );
        assert_eq!(rt.app().calls.get(), 0);
    }

    #[test]
    fn an_unmapped_key_skips_the_handler() {
        let mut rt = runtime(true, true);
        assert_eq!(route_key(&mut rt, None, false), KeyStep::Pass);
        assert_eq!(rt.app().calls.get(), 0);
    }

    #[test]
    fn native_with_no_handler_keeps_the_old_field_behavior() {
        let mut rt = runtime(true, false);
        native_press(&mut rt, &ENTER, Some("\r"), NONE);
        assert_eq!(rt.app().draft, "hi\n");
        let mut rt = runtime(false, false);
        native_press(&mut rt, &ENTER, Some("\r"), NONE);
        assert_eq!(rt.app().draft, "hi", "single-line Enter activates");
    }

    #[test]
    fn web_enter_is_consumed_prevents_the_default_and_syncs_the_value() {
        let mut rt = runtime(true, true);
        let step = web_press(&mut rt, "Enter", 13, false, NONE);
        assert!(
            step.prevent_default,
            "the browser must not insert a newline"
        );
        assert!(step.sync);
        assert!(step.dispatch.is_none());
        assert_eq!(rt.app().sent, 1);
        assert_eq!(rt.app().draft, "hi");
    }

    #[test]
    fn web_shift_enter_passes_and_keeps_the_browser_newline() {
        let mut rt = runtime(true, true);
        let step = web_press(&mut rt, "Enter", 13, false, SHIFT);
        assert!(!step.prevent_default, "the browser inserts the newline");
        assert!(step.dispatch.is_none());
        assert_eq!(rt.app().calls.get(), 1, "the handler saw Shift+Enter");
        assert_eq!(rt.app().sent, 0);
    }

    #[test]
    fn web_safari_late_enter_with_key_code_229_skips_the_handler() {
        let mut rt = runtime(true, true);
        let step = web_press(&mut rt, "Enter", 229, false, NONE);
        assert_eq!(rt.app().calls.get(), 0);
        assert_eq!(rt.app().sent, 0);
        assert!(
            !step.prevent_default,
            "the old multiline path: browser default"
        );
        // Single line: the old path activates, as before #103.
        let mut rt = runtime(false, true);
        let step = web_press(&mut rt, "Enter", 229, false, NONE);
        assert_eq!(rt.app().calls.get(), 0);
        assert_eq!(format!("{:?}", step.dispatch), "Some(Activate)");
    }

    #[test]
    fn web_composition_flags_skip_the_handler_and_the_field() {
        let mut rt = runtime(true, true);
        let step = web_press(&mut rt, "Enter", 13, true, NONE);
        assert!(!step.prevent_default && step.dispatch.is_none() && !step.sync);
        let step = web_key(&mut rt, "Enter", 13, false, true, NONE);
        assert!(!step.prevent_default && step.dispatch.is_none() && !step.sync);
        assert_eq!(rt.app().calls.get(), 0);
    }

    #[test]
    fn web_with_no_handler_keeps_the_old_keydown_table() {
        let cmd = Modifiers { meta: true, ..NONE };
        let show = |multiline: bool, key: &str, mods: Modifiers| {
            let mut rt = runtime(multiline, false);
            let s = web_key(&mut rt, key, 0, false, false, mods);
            format!("{} {:?} {}", s.prevent_default, s.dispatch, s.sync)
        };
        let cases = [
            (show(true, "Enter", NONE), "false None false"),
            (show(true, "Enter", SHIFT), "false None false"),
            (show(false, "Enter", NONE), "true Some(Activate) false"),
            (show(true, "Escape", NONE), "true Some(Cancel) false"),
            (
                show(true, "Tab", SHIFT),
                "true Some(FocusNext { reverse: true }) false",
            ),
            (
                show(true, "z", cmd),
                "true Some(Edit { key: Undo, extend: false, by_word: false }) true",
            ),
            (
                show(true, "Z", Modifiers { shift: true, ..cmd }),
                "true Some(Edit { key: Redo, extend: false, by_word: false }) true",
            ),
            (
                show(
                    true,
                    "y",
                    Modifiers {
                        control: true,
                        ..NONE
                    },
                ),
                "true Some(Edit { key: Redo, extend: false, by_word: false }) true",
            ),
            (show(true, "ArrowUp", NONE), "false None false"),
            (show(true, "a", NONE), "false None false"),
        ];
        for (got, want) in cases {
            assert_eq!(got, want);
        }
    }

    #[test]
    fn field_actions_match_the_pre_103_key_table() {
        let multi = InputSnapshot {
            id: "chat".into(),
            label: "Message".into(),
            value: String::new(),
            selection: surface_app::TextSelection {
                anchor: 0,
                cursor: 0,
            },
            composing: false,
            caret: None,
            multiline: true,
            read_only: false,
        };
        let cmd = Modifiers { meta: true, ..NONE };
        let ctrl = Modifiers {
            control: true,
            ..NONE
        };
        let show = |key: WinitKey, text: Option<&str>, mods, input: Option<&InputSnapshot>| {
            format!("{:?}", field_action(&key, text, mods, input))
        };
        let ch = |s: &str| WinitKey::Character(s.into());
        let named = WinitKey::Named;
        let cases = [
            (
                show(ENTER, None, NONE, Some(&multi)),
                r#"Dispatch(Text("\n"))"#,
            ),
            (
                show(ENTER, None, SHIFT, Some(&multi)),
                r#"Dispatch(Text("\n"))"#,
            ),
            (show(ENTER, None, NONE, None), "Dispatch(Activate)"),
            (
                show(named(NamedKey::Escape), None, NONE, Some(&multi)),
                "Dispatch(Cancel)",
            ),
            (
                show(named(NamedKey::Tab), None, SHIFT, None),
                "Dispatch(FocusNext { reverse: true })",
            ),
            (
                show(named(NamedKey::ArrowUp), None, SHIFT, Some(&multi)),
                "Dispatch(Edit { key: Up, extend: true, by_word: false })",
            ),
            (
                show(named(NamedKey::ArrowLeft), None, cmd, Some(&multi)),
                "Dispatch(Edit { key: Left, extend: false, by_word: true })",
            ),
            (
                show(named(NamedKey::Space), Some(" "), NONE, None),
                "Dispatch(Activate)",
            ),
            (
                show(named(NamedKey::Space), Some(" "), NONE, Some(&multi)),
                r#"Dispatch(Text(" "))"#,
            ),
            (
                show(ch("z"), Some("z"), Modifiers { shift: true, ..cmd }, None),
                "Dispatch(Edit { key: Redo, extend: false, by_word: false })",
            ),
            (
                show(ch("a"), Some("a"), ctrl, None),
                "Dispatch(Edit { key: SelectAll, extend: false, by_word: false })",
            ),
            (show(ch("b"), Some("b"), cmd, Some(&multi)), "Nothing"),
            (
                show(
                    ch("é"),
                    Some("é"),
                    Modifiers { alt: true, ..NONE },
                    Some(&multi),
                ),
                r#"Dispatch(Text("é"))"#,
            ),
        ];
        for (got, want) in cases {
            assert_eq!(got, want);
        }
        #[cfg(not(target_arch = "wasm32"))]
        assert_eq!(show(ch("v"), Some("v"), cmd, None), r#"Clipboard("v")"#);
    }

    #[test]
    fn winit_keys_map_to_surface_keys() {
        assert_eq!(from_winit(&ENTER), Some(Key::Enter));
        assert_eq!(
            from_winit(&WinitKey::Named(NamedKey::Escape)),
            Some(Key::Escape)
        );
        assert_eq!(
            from_winit(&WinitKey::Named(NamedKey::ArrowUp)),
            Some(Key::ArrowUp)
        );
        assert_eq!(
            from_winit(&WinitKey::Named(NamedKey::ArrowDown)),
            Some(Key::ArrowDown)
        );
        assert_eq!(
            from_winit(&WinitKey::Named(NamedKey::Space)),
            Some(Key::Space)
        );
        assert_eq!(
            from_winit(&WinitKey::Character("/".into())),
            Some(Key::Character("/".into()))
        );
        assert_eq!(from_winit(&WinitKey::Named(NamedKey::F1)), None);
        assert_eq!(from_winit(&WinitKey::Named(NamedKey::Shift)), None);
    }

    #[test]
    fn dom_keys_map_to_surface_keys() {
        assert_eq!(from_dom("Enter"), Some(Key::Enter));
        assert_eq!(from_dom("Escape"), Some(Key::Escape));
        assert_eq!(from_dom("ArrowUp"), Some(Key::ArrowUp));
        assert_eq!(from_dom("ArrowDown"), Some(Key::ArrowDown));
        assert_eq!(from_dom(" "), Some(Key::Space));
        assert_eq!(from_dom("a"), Some(Key::Character("a".into())));
        assert_eq!(from_dom("中"), Some(Key::Character("中".into())));
        for other in ["Shift", "Meta", "Process", "Unidentified", "Dead", "F1"] {
            assert_eq!(from_dom(other), None, "{other}");
        }
    }

    #[test]
    fn dom_composition_includes_safaris_late_enter() {
        assert!(!dom_composing(false, 13, false));
        assert!(dom_composing(true, 13, false));
        assert!(dom_composing(false, 13, true));
        assert!(dom_composing(false, 229, false), "Safari: keyCode 229");
    }
}
