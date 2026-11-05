//! Host clock and reduced-motion preference for runtime motion.
//!
//! The runtime never reads a clock. The host sets it before each render and
//! keeps asking for frames while `Runtime::needs_frame` is true.

/// A monotonic clock in milliseconds.
pub(crate) struct Clock {
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
}

impl Clock {
    pub(crate) fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            start: std::time::Instant::now(),
        }
    }
    /// Milliseconds since the clock started. `None` when the platform has no
    /// clock, which leaves motion off so transitions can never stall.
    pub(crate) fn now_ms(&self) -> Option<f64> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Some(self.start.elapsed().as_secs_f64() * 1000.)
        }
        #[cfg(target_arch = "wasm32")]
        {
            Some(web_sys::window()?.performance()?.now())
        }
    }
}

/// Native hosts have no OS query yet. `SURFACE_REDUCED_MOTION=1` turns it on.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn reduced_motion_from_env() -> bool {
    std::env::var("SURFACE_REDUCED_MOTION").is_ok_and(|value| value == "1")
}

#[cfg(target_arch = "wasm32")]
pub(crate) use web::ReducedMotionWatch;

#[cfg(target_arch = "wasm32")]
mod web {
    use std::{cell::RefCell, rc::Rc, sync::Arc};
    use surface_app::{Application, Runtime};
    use wasm_bindgen::{closure::Closure, JsCast};
    use winit::window::Window;

    const QUERY: &str = "(prefers-reduced-motion: reduce)";

    /// Follows `prefers-reduced-motion` for the lifetime of the host.
    pub(crate) struct ReducedMotionWatch {
        query: web_sys::MediaQueryList,
        callback: Closure<dyn FnMut(web_sys::MediaQueryListEvent)>,
    }

    impl ReducedMotionWatch {
        /// Read the current preference and listen for changes. Returns `None`
        /// when the browser has no `matchMedia`, which leaves motion on.
        pub(crate) fn new<A: Application>(
            runtime: Rc<RefCell<Runtime<A>>>,
            window: Arc<Window>,
        ) -> Option<Self> {
            let query = web_sys::window()?.match_media(QUERY).ok()??;
            runtime.borrow_mut().set_reduced_motion(query.matches());
            let callback = Closure::wrap(Box::new(move |event: web_sys::MediaQueryListEvent| {
                runtime.borrow_mut().set_reduced_motion(event.matches());
                window.request_redraw();
            })
                as Box<dyn FnMut(web_sys::MediaQueryListEvent)>);
            query
                .add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())
                .ok()?;
            Some(Self { query, callback })
        }
    }

    impl Drop for ReducedMotionWatch {
        fn drop(&mut self) {
            let _ = self.query.remove_event_listener_with_callback(
                "change",
                self.callback.as_ref().unchecked_ref(),
            );
        }
    }
}
