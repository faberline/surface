//! Cross-thread wake. Many calls coalesce into one pending event.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

struct Inner {
    pending: AtomicBool,
    notify: Box<dyn Fn() + Send + Sync>,
}

/// Asks the event loop for one `Application::wake` and one redraw.
///
/// `Clone + Send + Sync`: call it from any thread, on native and on the web.
/// It is cheap. While a wake is pending, further calls only read an atomic
/// flag. The host clears the flag before it calls `wake`, so a call made
/// during `wake` schedules one more. A call is never lost.
#[derive(Clone)]
pub struct WakeHandle(Arc<Inner>);

impl WakeHandle {
    pub(crate) fn new(notify: impl Fn() + Send + Sync + 'static) -> Self {
        Self(Arc::new(Inner {
            pending: AtomicBool::new(false),
            notify: Box::new(notify),
        }))
    }
    /// Request one update on the event loop. Safe from any thread.
    pub fn wake(&self) {
        if !self.0.pending.swap(true, Ordering::AcqRel) {
            (self.0.notify)();
        }
    }
    /// The host calls this when it takes the pending wake, before it runs
    /// the app.
    pub(crate) fn acknowledge(&self) {
        self.0.pending.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    };
    use std::{thread, time::Duration};
    use surface::{Node, Role};
    use surface_app::{Application, Runtime, View};

    #[test]
    fn handle_is_clone_send_sync_and_fits_a_wake_closure() {
        fn assert_traits<T: Clone + Send + Sync + 'static>() {}
        assert_traits::<WakeHandle>();
        let handle = WakeHandle::new(|| {});
        let _closure: Arc<dyn Fn() + Send + Sync> = Arc::new(move || handle.wake());
    }

    #[test]
    fn many_wakes_from_many_threads_queue_one_event() {
        let (tx, rx) = mpsc::channel();
        let handle = WakeHandle::new(move || tx.send(()).unwrap());
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let handle = handle.clone();
                thread::spawn(move || (0..5000).for_each(|_| handle.wake()))
            })
            .collect();
        workers.into_iter().for_each(|w| w.join().unwrap());
        assert_eq!(rx.try_iter().count(), 1, "20000 wakes, one pending event");
        handle.acknowledge();
        handle.wake();
        assert_eq!(rx.try_iter().count(), 1, "a wake after delivery is new");
    }

    #[derive(Clone)]
    enum Msg {}
    struct Polling {
        produced: Arc<AtomicUsize>,
        seen: usize,
        updates: usize,
    }
    impl Application for Polling {
        type Message = Msg;
        fn update(&mut self, message: Msg) {
            match message {}
        }
        fn wake(&mut self) {
            self.seen = self.produced.load(Ordering::SeqCst);
            self.updates += 1;
        }
        fn view(&self) -> View<Msg> {
            View::new(Node::new(Role::Text, format!("seen={}", self.seen)))
        }
    }

    #[test]
    fn worker_thread_wakes_many_times_and_update_runs_far_fewer() {
        const WAKES: usize = 20_000;
        let produced = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();
        let handle = WakeHandle::new(move || tx.send(()).unwrap());
        let mut runtime = Runtime::new(Polling {
            produced: produced.clone(),
            seen: 0,
            updates: 0,
        })
        .unwrap();
        let worker = {
            let handle = handle.clone();
            thread::spawn(move || {
                for _ in 0..WAKES {
                    produced.fetch_add(1, Ordering::SeqCst);
                    handle.wake();
                }
            })
        };
        // This thread plays the event loop.
        while rx.recv_timeout(Duration::from_millis(500)).is_ok() {
            handle.acknowledge();
            runtime.wake().unwrap();
        }
        worker.join().unwrap();
        let app = runtime.app();
        assert!(app.updates >= 1, "update never ran");
        assert_eq!(app.seen, WAKES, "the last wake must not be lost");
        assert!(
            app.updates * 10 < WAKES,
            "{} updates for {WAKES} wakes",
            app.updates
        );
    }
}
