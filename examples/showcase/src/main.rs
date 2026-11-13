use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};
use surface_app::{Application, Queue, Runtime, View};
use surface_showcase::{Message, Showcase};

// Counts of the smoke app's `Drop`, read after `run_with_wake` returns.
static DROPS: AtomicU32 = AtomicU32::new(0);
static WINDOW_GONE_AT_DROP: AtomicBool = AtomicBool::new(false);

/// `--smoke-close`: the showcase plus a close command from `wake`.
struct CloseSmoke {
    inner: Showcase,
    queue: Rc<RefCell<Option<Queue<Message>>>>,
    window_released: Arc<AtomicBool>,
}
impl Application for CloseSmoke {
    type Message = Message;
    fn title(&self) -> &str {
        self.inner.title()
    }
    fn update(&mut self, message: Message) {
        self.inner.update(message)
    }
    fn wake(&mut self) {
        if let Some(queue) = self.queue.borrow().as_ref() {
            queue.close();
        }
    }
    fn view(&self) -> View<Message> {
        self.inner.view()
    }
}
impl Drop for CloseSmoke {
    fn drop(&mut self) {
        WINDOW_GONE_AT_DROP.store(
            self.window_released.load(Ordering::SeqCst),
            Ordering::SeqCst,
        );
        DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

fn smoke_close() -> Result<(), Box<dyn std::error::Error>> {
    let released = Arc::new(AtomicBool::new(false));
    let options = surface_platform::Options {
        window_released: Some(released.clone()),
        ..Default::default()
    };
    surface_platform::run_with_wake(options, |wake| {
        let queue = Rc::new(RefCell::new(None));
        let app = CloseSmoke {
            inner: Showcase::default(),
            queue: queue.clone(),
            window_released: released,
        };
        let runtime =
            Runtime::new(app).map_err(|e| surface_platform::HostError::new(e.to_string()))?;
        *queue.borrow_mut() = Some(runtime.queue());
        // Let the window present a few frames, then ask to close from `wake`.
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1500));
            wake.wake();
        });
        Ok(Rc::new(RefCell::new(runtime)))
    })?;
    let drops = DROPS.load(Ordering::SeqCst);
    let window_gone = WINDOW_GONE_AT_DROP.load(Ordering::SeqCst);
    eprintln!("Surface close smoke: run_with_wake returned Ok; app Drop ran {drops} time(s); window and gpu dropped before app state: {window_gone}");
    if drops != 1 {
        return Err(format!("app Drop ran {drops} times, expected 1").into());
    }
    if !window_gone {
        return Err("app state dropped while the window was still alive".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--smoke-close") {
        return smoke_close();
    }
    let runtime = surface_app::Runtime::new(surface_showcase::Showcase::default())?;
    let mut options = surface_platform::Options::default();
    if std::env::args().any(|a| a == "--smoke") {
        options.close_after_frames = Some(2);
    }
    surface_platform::run_runtime(std::rc::Rc::new(std::cell::RefCell::new(runtime)), options)?;
    Ok(())
}
