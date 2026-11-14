use crate::{
    key_route::{self, FieldAction, NativeStep},
    Host,
};
use surface_app::Application;
use winit::{event::KeyEvent, event_loop::ActiveEventLoop};
impl<A: Application> Host<A> {
    pub(super) fn key_input(&mut self, event_loop: &ActiveEventLoop, event: KeyEvent) {
        #[cfg(target_arch = "wasm32")]
        if self.web_input.as_ref().is_some_and(|input| input.focused()) {
            return;
        }
        // The app's key handler runs before the field (#103).
        let step = key_route::native_key(
            &mut self.runtime.borrow_mut(),
            &event.logical_key,
            event.text.as_deref(),
            self.mods,
        );
        match step {
            NativeStep::Ignore | NativeStep::Field(FieldAction::Nothing) => {}
            NativeStep::Consumed => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                self.sync_text_input();
            }
            NativeStep::Fail(error) => self.fail(event_loop, error),
            NativeStep::Field(FieldAction::Dispatch(event)) => self.input(event_loop, event),
            #[cfg(not(target_arch = "wasm32"))]
            NativeStep::Field(FieldAction::Clipboard(command)) => {
                self.clipboard_command(event_loop, &command)
            }
            #[cfg(target_arch = "wasm32")]
            NativeStep::Field(FieldAction::Clipboard(_)) => {}
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
