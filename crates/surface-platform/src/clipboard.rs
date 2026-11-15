//! App clipboard: write and read plain text without a host or a focused field.
use std::sync::{Arc, Mutex};

/// Why a clipboard call failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClipboardError {
    /// This host has no clipboard path.
    #[error("the clipboard is not supported on this host")]
    Unsupported,
    /// The clipboard could not be opened or used.
    #[error("the clipboard is unavailable: {0}")]
    Unavailable(String),
}

trait Backend: Send {
    fn set(&mut self, text: &str) -> Result<(), ClipboardError>;
    fn get(&mut self) -> Result<Option<String>, ClipboardError>;
}

#[derive(Default)]
struct Memory(Option<String>);

impl Backend for Memory {
    fn set(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.0 = Some(text.to_owned());
        Ok(())
    }
    fn get(&mut self) -> Result<Option<String>, ClipboardError> {
        Ok(self.0.clone())
    }
}

/// The OS clipboard through arboard. It opens on first use and stays open.
/// A failure to open is an error from the call, never a panic.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct System(Option<arboard::Clipboard>);

#[cfg(not(target_arch = "wasm32"))]
impl System {
    fn open(&mut self) -> Result<&mut arboard::Clipboard, ClipboardError> {
        if self.0.is_none() {
            let opened = arboard::Clipboard::new()
                .map_err(|error| ClipboardError::Unavailable(error.to_string()))?;
            self.0 = Some(opened);
        }
        Ok(self.0.as_mut().expect("opened above"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Backend for System {
    fn set(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.open()?
            .set_text(text)
            .map_err(|error| ClipboardError::Unavailable(error.to_string()))
    }
    fn get(&mut self) -> Result<Option<String>, ClipboardError> {
        match self.open()?.get_text() {
            Ok(text) => Ok(Some(text)),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(error) => Err(ClipboardError::Unavailable(error.to_string())),
        }
    }
}

/// The web path is out of scope: every call returns `Unsupported`.
#[cfg(target_arch = "wasm32")]
struct Unsupported;

#[cfg(target_arch = "wasm32")]
impl Backend for Unsupported {
    fn set(&mut self, _text: &str) -> Result<(), ClipboardError> {
        Err(ClipboardError::Unsupported)
    }
    fn get(&mut self) -> Result<Option<String>, ClipboardError> {
        Err(ClipboardError::Unsupported)
    }
}

/// Handle to a plain-text clipboard. `Clone + Send + Sync`.
///
/// Create it once, keep it in the app state and call `set_text` from
/// `update` or from a worker thread. It needs no host.
#[derive(Clone)]
pub struct Clipboard(Arc<Mutex<Box<dyn Backend>>>);

impl Clipboard {
    fn with_backend(backend: impl Backend + 'static) -> Self {
        Self(Arc::new(Mutex::new(Box::new(backend))))
    }
    /// The OS clipboard. On wasm32 every call returns `Unsupported`.
    pub fn system() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        return Self::with_backend(System::default());
        #[cfg(target_arch = "wasm32")]
        return Self::with_backend(Unsupported);
    }
    /// An in-memory clipboard for tests and headless hosts.
    pub fn memory() -> Self {
        Self::with_backend(Memory::default())
    }
    /// Replace the clipboard text.
    pub fn set_text(&self, text: &str) -> Result<(), ClipboardError> {
        self.lock().set(text)
    }
    /// Read the clipboard text. `None` when it holds no text.
    pub fn get_text(&self) -> Result<Option<String>, ClipboardError> {
        self.lock().get()
    }
    /// A panic in another caller must not disable the clipboard.
    fn lock(&self) -> std::sync::MutexGuard<'_, Box<dyn Backend>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    struct Failing;
    impl Backend for Failing {
        fn set(&mut self, _: &str) -> Result<(), ClipboardError> {
            Err(ClipboardError::Unavailable("no pasteboard".into()))
        }
        fn get(&mut self) -> Result<Option<String>, ClipboardError> {
            Err(ClipboardError::Unavailable("no pasteboard".into()))
        }
    }

    #[test]
    fn memory_set_then_get() {
        let clipboard = Clipboard::memory();
        assert_eq!(clipboard.get_text(), Ok(None));
        clipboard.set_text("hello").unwrap();
        assert_eq!(clipboard.get_text(), Ok(Some("hello".to_string())));
    }

    #[test]
    fn set_replaces_previous() {
        let clipboard = Clipboard::memory();
        clipboard.set_text("a").unwrap();
        clipboard.set_text("b").unwrap();
        assert_eq!(clipboard.get_text(), Ok(Some("b".to_string())));
    }

    #[test]
    fn clone_shares_state() {
        let one = Clipboard::memory();
        let two = one.clone();
        one.set_text("shared").unwrap();
        assert_eq!(two.get_text(), Ok(Some("shared".to_string())));
    }

    #[test]
    fn unicode_and_empty_round_trip() {
        let clipboard = Clipboard::memory();
        let text = "中文 e\u{301} 🙂\nline2";
        clipboard.set_text(text).unwrap();
        assert_eq!(clipboard.get_text(), Ok(Some(text.to_string())));
        clipboard.set_text("").unwrap();
        assert_eq!(clipboard.get_text(), Ok(Some(String::new())));
    }

    #[test]
    fn error_from_backend_is_returned() {
        let clipboard = Clipboard::with_backend(Failing);
        assert!(matches!(
            clipboard.set_text("x"),
            Err(ClipboardError::Unavailable(_))
        ));
        assert!(matches!(
            clipboard.get_text(),
            Err(ClipboardError::Unavailable(_))
        ));
    }

    #[test]
    fn handle_is_send_sync_clone() {
        fn assert_traits<T: Clone + Send + Sync + 'static>() {}
        assert_traits::<Clipboard>();
    }

    #[test]
    fn cross_thread() {
        let clipboard = Clipboard::memory();
        let worker = clipboard.clone();
        thread::spawn(move || worker.set_text("from worker").unwrap())
            .join()
            .unwrap();
        assert_eq!(clipboard.get_text(), Ok(Some("from worker".to_string())));
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn system_is_unsupported_on_wasm() {
        let clipboard = Clipboard::system();
        assert_eq!(clipboard.set_text("x"), Err(ClipboardError::Unsupported));
        assert_eq!(clipboard.get_text(), Err(ClipboardError::Unsupported));
    }

    /// Hand check only. It writes the real system clipboard.
    #[test]
    #[ignore]
    fn system_set_text_hand_check() {
        Clipboard::system().set_text("surface-108-check").unwrap();
    }
}
