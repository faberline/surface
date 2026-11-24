//! Host clock and display preference handling.
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
            web_now_ms()
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn web_now_ms() -> Option<f64> {
    Some(web_sys::window()?.performance()?.now())
}

/// `SURFACE_REDUCED_MOTION=1` forces reduced motion on native hosts.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn reduced_motion_from_env() -> bool {
    std::env::var("SURFACE_REDUCED_MOTION").is_ok_and(|value| value == "1")
}

/// Apply the effective preference on the event-loop thread. A forced setting
/// must remain enabled even when the OS subsequently turns its setting off.
/// Return whether a redraw is needed.
#[cfg(any(target_os = "macos", all(test, not(target_arch = "wasm32"))))]
pub(crate) fn update_preference<A: surface_app::Application>(
    runtime: &mut surface_app::Runtime<A>,
    forced: bool,
    system: bool,
) -> bool {
    let reduced = forced || system;
    if runtime.reduced_motion() == reduced {
        return false;
    }
    runtime.set_reduced_motion(reduced);
    true
}

#[cfg(target_os = "macos")]
pub(crate) use macos::{refresh_display_options, DisplayOptionsWatch as MacDisplayOptionsWatch};

#[cfg(any(target_os = "macos", all(test, not(target_arch = "wasm32"))))]
fn apply_macos_display_options<A: surface_app::Application>(
    runtime: &mut surface_app::Runtime<A>,
    force_reduce_motion: bool,
    system_reduce_motion: bool,
    system_increase_contrast: bool,
) -> (bool, Option<surface_app::AppError>) {
    use surface_app::DisplayContrast;

    let motion_changed = update_preference(runtime, force_reduce_motion, system_reduce_motion);
    let contrast = if system_increase_contrast {
        DisplayContrast::Increased
    } else {
        DisplayContrast::Normal
    };
    let previous = runtime.display_contrast();
    match runtime.set_display_contrast(contrast) {
        Ok(contrast_changed) => (motion_changed || contrast_changed, None),
        // Preserve the redraw from motion. If a later reconciliation failed
        // after installing contrast, the installed appearance needs a frame.
        Err(error) => (
            motion_changed || runtime.display_contrast() != previous,
            Some(error),
        ),
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{apply_macos_display_options, reduced_motion_from_env};
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2::{rc::Retained, runtime::ProtocolObject};
    use objc2_app_kit::{NSWorkspace, NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification};
    use objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectProtocol};

    /// Read on the host event-loop thread.
    pub(crate) fn refresh_display_options<A: surface_app::Application>(
        runtime: &mut surface_app::Runtime<A>,
    ) -> bool {
        let workspace = NSWorkspace::sharedWorkspace();
        let (redraw, contrast_error) = apply_macos_display_options(
            runtime,
            reduced_motion_from_env(),
            workspace.accessibilityDisplayShouldReduceMotion(),
            workspace.accessibilityDisplayShouldIncreaseContrast(),
        );
        if let Some(error) = contrast_error {
            eprintln!("Surface macOS display contrast: {error}");
        }
        redraw
    }

    /// The workspace notification carries no preference values. The callback
    /// signals the event loop, which reads both current preferences there.
    pub(crate) struct DisplayOptionsWatch {
        center: Retained<NSNotificationCenter>,
        observer: Retained<ProtocolObject<dyn NSObjectProtocol>>,
    }

    impl DisplayOptionsWatch {
        pub(crate) fn new(notify: impl Fn() + Send + Sync + 'static) -> Self {
            Self::observe(NSWorkspace::sharedWorkspace().notificationCenter(), notify)
        }

        fn observe(
            center: Retained<NSNotificationCenter>,
            notify: impl Fn() + Send + Sync + 'static,
        ) -> Self {
            let callback = RcBlock::new(move |_note: NonNull<NSNotification>| notify());
            // SAFETY: The notification name is Apple's constant. There is no
            // sender filter. With no queue the block may run on any posting
            // thread; its captured callback is Send + Sync and only signals
            // the event loop. The center retains the block until removal.
            let observer = unsafe {
                center.addObserverForName_object_queue_usingBlock(
                    Some(NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification),
                    None,
                    None,
                    &callback,
                )
            };
            Self { center, observer }
        }
    }

    impl Drop for DisplayOptionsWatch {
        fn drop(&mut self) {
            // SAFETY: This is the token returned by this center. Keep it alive
            // until the registration has been removed.
            unsafe { self.center.removeObserver((*self.observer).as_ref()) };
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };

        #[test]
        fn observer_delivers_notifications_and_unregisters_on_drop() {
            let center = NSNotificationCenter::new();
            let calls = Arc::new(AtomicUsize::new(0));
            let notified = calls.clone();
            let watcher = DisplayOptionsWatch::observe(center.clone(), move || {
                notified.fetch_add(1, Ordering::SeqCst);
            });
            // SAFETY: Name is the same framework constant used to register,
            // and a nil sender is valid because no sender filter was installed.
            unsafe {
                center.postNotificationName_object(
                    NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
                    None,
                )
            };
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            drop(watcher);
            // SAFETY: As above; the owned observer is now unregistered.
            unsafe {
                center.postNotificationName_object(
                    NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
                    None,
                )
            };
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod preference_tests {
    use super::{apply_macos_display_options, update_preference};
    use surface::{Node, Role};
    use surface_app::{Application, DisplayContrast, Runtime, View};

    struct App;
    impl Application for App {
        type Message = ();
        fn update(&mut self, (): ()) {}
        fn view(&self) -> View<()> {
            View::new(Node::new(Role::Window, "test"))
        }
    }

    #[test]
    fn turning_reduce_on_during_transition_finishes_on_next_frame() {
        use surface::{Color, Easing, Length, PaintOp, Style, TextMeasurer, Transition};

        struct Animated(bool);
        impl Application for Animated {
            type Message = ();
            fn update(&mut self, (): ()) {
                self.0 = !self.0;
            }
            fn view(&self) -> View<()> {
                let red = if self.0 { 200 } else { 0 };
                View::new(
                    Node::new(Role::Button, "change").id("change").styled(
                        Style::default()
                            .width(Length::Px(100.))
                            .height(Length::Px(40.))
                            .background(Color::rgb(red, 0, 0))
                            .transition(Transition::new(100., Easing::Linear)),
                    ),
                )
                .on_press(())
            }
        }
        struct Metrics;
        impl TextMeasurer for Metrics {
            fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
                (text.chars().count() as f32 * size / 2., size * 1.5)
            }
        }
        let mut runtime = Runtime::new(Animated(false)).unwrap();
        runtime.set_time(0.);
        runtime.render(320., 240., &mut Metrics).unwrap();
        runtime.activate("change").unwrap();
        runtime.set_time(10.);
        runtime.render(320., 240., &mut Metrics).unwrap();
        assert!(runtime.needs_frame());
        assert!(update_preference(&mut runtime, false, true));
        let display = runtime.render(320., 240., &mut Metrics).unwrap();
        assert!(!runtime.needs_frame());
        assert!(display.ops.iter().any(|op| matches!(
            op,
            PaintOp::Quad { color, .. } | PaintOp::Shape { color, .. }
                if *color == Color::rgb(200, 0, 0)
        )));
    }

    #[test]
    fn effective_preference_tracks_system_and_force_without_unneeded_redraws() {
        let mut runtime = Runtime::new(App).unwrap();
        assert!(!runtime.reduced_motion());
        assert!(!update_preference(&mut runtime, false, false));
        assert!(update_preference(&mut runtime, false, true));
        assert!(runtime.reduced_motion());
        assert!(!update_preference(&mut runtime, false, true));
        assert!(update_preference(&mut runtime, false, false));
        assert!(!runtime.reduced_motion());
        assert!(update_preference(&mut runtime, true, false));
        assert!(!update_preference(&mut runtime, true, true));
        assert!(!update_preference(&mut runtime, true, false));
        assert!(runtime.reduced_motion());
    }

    #[derive(Default)]
    struct Appearance {
        contrast: DisplayContrast,
        reject_increased: bool,
    }

    impl Application for Appearance {
        type Message = ();

        fn update(&mut self, (): ()) {}

        fn display_contrast_changed(&mut self, contrast: DisplayContrast) -> bool {
            self.contrast = contrast;
            true
        }

        fn view(&self) -> View<()> {
            let root = View::new(Node::new(Role::Window, "test"));
            if self.reject_increased && self.contrast == DisplayContrast::Increased {
                let duplicate = || View::new(Node::new(Role::Group, "").id("duplicate"));
                return root.children([duplicate(), duplicate()]);
            }
            root
        }
    }

    #[test]
    fn mac_display_options_apply_both_preferences_and_coalesce_redraw() {
        let mut runtime = Runtime::new(Appearance::default()).unwrap();
        let (redraw, error) = apply_macos_display_options(&mut runtime, false, true, true);
        assert!(redraw);
        assert!(error.is_none());
        assert!(runtime.reduced_motion());
        assert_eq!(runtime.display_contrast(), DisplayContrast::Increased);

        let (redraw, error) = apply_macos_display_options(&mut runtime, false, true, true);
        assert!(!redraw);
        assert!(error.is_none());

        let (redraw, error) = apply_macos_display_options(&mut runtime, true, false, false);
        assert!(redraw);
        assert!(error.is_none());
        assert!(runtime.reduced_motion());
        assert_eq!(runtime.display_contrast(), DisplayContrast::Normal);
    }

    #[test]
    fn mac_display_options_keep_motion_changes_when_contrast_is_rejected() {
        let mut runtime = Runtime::new(Appearance {
            reject_increased: true,
            ..Appearance::default()
        })
        .unwrap();
        let (redraw, error) = apply_macos_display_options(&mut runtime, false, true, true);
        assert!(redraw);
        assert!(error.is_some());
        assert!(runtime.reduced_motion());
        assert_eq!(runtime.display_contrast(), DisplayContrast::Normal);
        assert_eq!(runtime.app().contrast, DisplayContrast::Normal);

        let (redraw, error) = apply_macos_display_options(&mut runtime, false, true, true);
        assert!(!redraw);
        assert!(error.is_some());
    }
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
