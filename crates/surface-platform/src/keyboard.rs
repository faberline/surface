use crate::{
    key_route::{self, FieldAction, KeyStep},
    Host,
};
use surface_app::{Application, KeyPress};
use winit::{event::KeyEvent, event_loop::ActiveEventLoop};
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
        // The app's key handler runs before the field (#103).
        let press =
            key_route::from_winit(&event.logical_key).map(|key| KeyPress::new(key, self.mods));
        let step = key_route::route_key(&mut self.runtime.borrow_mut(), press, false);
        match step {
            KeyStep::Consumed => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                self.sync_text_input();
                return;
            }
            KeyStep::Fail(error) => return self.fail(event_loop, error),
            KeyStep::Pass => {}
        }
        let text = event.text.as_deref();
        match key_route::field_action(&event.logical_key, text, self.mods, input.as_ref()) {
            FieldAction::Dispatch(event) => self.input(event_loop, event),
            #[cfg(not(target_arch = "wasm32"))]
            FieldAction::Clipboard(command) => self.clipboard_command(event_loop, &command),
            #[cfg(target_arch = "wasm32")]
            FieldAction::Clipboard(_) => {}
            FieldAction::Nothing => {}
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
                self.input(event_loop, surface_app::Event::Text(text));
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
