use crate::Host;
use surface_app::{Application, EditKey, Event};
use winit::{
    event::KeyEvent,
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
};
impl<A: Application> Host<A> {
    pub(super) fn key_input(&mut self, event_loop: &ActiveEventLoop, event: KeyEvent) {
        #[cfg(target_arch = "wasm32")]
        if self.web_input.as_ref().is_some_and(|input| input.focused()) {
            return;
        }
        let input = self.runtime.borrow().input_snapshot();
        if input.as_ref().is_some_and(|i| i.composing) {
            return;
        }
        if self.control {
            if let Key::Character(key) = &event.logical_key {
                let edit = match key.to_lowercase().as_str() {
                    "a" => Some(EditKey::SelectAll),
                    "z" => Some(if self.shift {
                        EditKey::Redo
                    } else {
                        EditKey::Undo
                    }),
                    "y" => Some(EditKey::Redo),
                    _ => None,
                };
                if let Some(key) = edit {
                    self.input(
                        event_loop,
                        Event::Edit {
                            key,
                            extend: false,
                            by_word: false,
                        },
                    );
                    return;
                }
                #[cfg(not(target_arch = "wasm32"))]
                if matches!(key.to_lowercase().as_str(), "c" | "x" | "v") {
                    self.clipboard_command(event_loop, &key.to_lowercase());
                    return;
                }
            }
        }
        let key = match &event.logical_key {
            Key::Named(NamedKey::ArrowLeft) => Some(EditKey::Left),
            Key::Named(NamedKey::ArrowRight) => Some(EditKey::Right),
            Key::Named(NamedKey::ArrowUp) => Some(EditKey::Up),
            Key::Named(NamedKey::ArrowDown) => Some(EditKey::Down),
            Key::Named(NamedKey::Home) => Some(EditKey::Home),
            Key::Named(NamedKey::End) => Some(EditKey::End),
            Key::Named(NamedKey::Backspace) => Some(EditKey::Backspace),
            Key::Named(NamedKey::Delete) => Some(EditKey::Delete),
            _ => None,
        };
        if let Some(key) = key {
            self.input(
                event_loop,
                Event::Edit {
                    key,
                    extend: self.shift,
                    by_word: self.control,
                },
            );
            return;
        }
        match event.logical_key {
            Key::Named(NamedKey::Tab) => self.input(
                event_loop,
                Event::FocusNext {
                    reverse: self.shift,
                },
            ),
            Key::Named(NamedKey::Enter) if input.as_ref().is_some_and(|i| i.multiline) => {
                self.input(event_loop, Event::Text("\n".to_owned()))
            }
            Key::Named(NamedKey::Enter) => self.input(event_loop, Event::Activate),
            Key::Named(NamedKey::Escape) => self.input(event_loop, Event::Cancel),
            Key::Named(NamedKey::Space) if input.is_none() => {
                self.input(event_loop, Event::Activate)
            }
            _ if !self.control => {
                if let Some(text) = event.text {
                    self.input(event_loop, Event::Text(text.to_string()));
                }
            }
            _ => {}
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn clipboard_command(&mut self, event_loop: &ActiveEventLoop, command: &str) {
        let selected = self
            .runtime
            .borrow()
            .selected_text()
            .filter(|text| !text.is_empty());
        if self.clipboard.is_none() {
            self.clipboard = arboard::Clipboard::new().ok();
        }
        let Some(clipboard) = &mut self.clipboard else {
            return;
        };
        if command == "v" {
            if let Ok(text) = clipboard.get_text() {
                self.input(event_loop, Event::Text(text));
            }
        } else if let Some(text) = selected {
            if clipboard.set_text(text).is_ok() && command == "x" {
                let result = self.runtime.borrow_mut().cut_selection();
                if let Err(error) = result {
                    self.fail(event_loop, error);
                } else {
                    self.sync_text_input();
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
        }
    }
}
