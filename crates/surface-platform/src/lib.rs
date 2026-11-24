//! Thin desktop/browser window host. The application never sees platform events.
mod clipboard;
mod close;
#[cfg(not(target_arch = "wasm32"))]
mod cursor;
#[cfg(feature = "devtools")]
mod devtools;
#[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
mod headless;
#[cfg(any(target_arch = "wasm32", test))]
mod host_style;
#[cfg(not(target_arch = "wasm32"))]
mod ime_session;
mod key_order;
mod key_route;
mod keyboard;
#[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
mod macos_accessibility;
mod motion;
#[cfg(not(target_arch = "wasm32"))]
mod pointer_route;
#[cfg(any(target_arch = "wasm32", test))]
mod press_guard;
#[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
mod screenshot;
mod touch_route;
#[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
mod trace_file;
mod wake;
#[cfg(target_arch = "wasm32")]
mod web_accessibility;
#[cfg(any(target_arch = "wasm32", test))]
mod web_contrast;
#[cfg(target_arch = "wasm32")]
mod web_input;
#[cfg(any(target_arch = "wasm32", test))]
mod web_pointer_capture;
pub use clipboard::{Clipboard, ClipboardError};
#[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
pub use headless::run_headless_runtime;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use surface_app::{Application, Event, Runtime};
use surface_wgpu::Renderer;
pub use wake::WakeHandle;
#[cfg(target_arch = "wasm32")]
use winit::event::MouseButton;
use winit::event_loop::EventLoopProxy;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

fn drawable_size(window: &Window) -> (winit::dpi::PhysicalSize<u32>, f32) {
    let scale = window.scale_factor() as f32;
    #[cfg(not(target_arch = "wasm32"))]
    {
        (window.inner_size(), scale)
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        let canvas = window.canvas().expect("web window owns a canvas");
        // Use the CSS box as the logical viewport. Browser device-pixel
        // ResizeObserver reports can differ under emulation and page scaling.
        let size = winit::dpi::PhysicalSize::new(
            (canvas.client_width() as f32 * scale).round() as u32,
            (canvas.client_height() as f32 * scale).round() as u32,
        );
        if canvas.width() != size.width {
            canvas.set_width(size.width);
        }
        if canvas.height() != size.height {
            canvas.set_height(size.height);
        }
        (size, scale)
    }
}

/// Test seam for the native close smoke. Not part of the supported API.
/// 0 = no window was released, 1 = the window and gpu were gone before the app
/// state dropped, 2 = the window was still alive. Read it from the app's `Drop`.
#[doc(hidden)]
pub static WINDOW_RELEASED: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

#[derive(Clone, Debug)]
pub struct Options {
    pub width: u32,
    pub height: u32,
    pub close_after_frames: Option<u32>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            width: 960,
            height: 800,
            close_after_frames: None,
        }
    }
}
#[derive(Debug, thiserror::Error)]
#[error("Surface host: {0}")]
pub struct HostError(String);
impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
struct Gpu {
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    config: wgpu::SurfaceConfiguration,
}

#[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
struct PresentedFrame {
    list: surface::ExtendedDisplayList,
    size: (u32, u32),
    scale: f32,
}
impl Gpu {
    async fn new(window: Arc<Window>) -> Result<Self, HostError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| HostError(e.to_string()))?;
        let adapter = Renderer::request(&instance, Some(&surface))
            .await
            .map_err(|e| HostError(e.to_string()))?;
        let (size, _) = drawable_size(&window);
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| HostError("no surface configuration".to_owned()))?;
        // Prefer a non-sRGB target: Material color bytes then map directly to pixels.
        if let Some(format) = surface
            .get_capabilities(&adapter)
            .formats
            .iter()
            .find(|f| !f.is_srgb())
        {
            config.format = *format;
        }
        let renderer = Renderer::new(&adapter, config.format)
            .await
            .map_err(|e| HostError(e.to_string()))?;
        surface.configure(&renderer.device, &config);
        Ok(Self {
            surface,
            renderer,
            config,
        })
    }
    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.renderer.device, &self.config);
    }
}
/// On wasm32 `EventLoopProxy<UserEvent>` is not `Send`: `UserEvent` carries
/// wasm-only payloads that are not `Send` (`Ready`). The proxy's own waker is
/// thread-safe, and wasm32 here has one thread (no atomics). Sound only
/// because this wrapper sends the payload-free `UserEvent::Wake`. Never send
/// a variant with a payload through it.
struct SendProxy(EventLoopProxy<UserEvent>);
impl SendProxy {
    // A method, so a closure captures the whole wrapper, not its field.
    fn send_wake(&self) -> Result<(), winit::event_loop::EventLoopClosed<UserEvent>> {
        self.0.send_event(UserEvent::Wake)
    }
}
#[cfg(target_arch = "wasm32")]
unsafe impl Send for SendProxy {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for SendProxy {}
enum UserEvent {
    Wake,
    #[cfg(target_os = "macos")]
    MacDisplayOptionsChanged,
    #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
    AccessKit(accesskit_winit::Event),
    #[cfg(feature = "devtools")]
    Bridge {
        request: jet_bridge_protocol::Request,
        reply: devtools::Reply,
    },
    /// A devtools protocol request from the native loopback transport.
    #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
    /// Boxed, so `UserEvent` stays small (`clippy::result_large_err`).
    Devtools {
        request: Box<surface_devtools_protocol::Request>,
        reply: devtools::DevtoolsReply,
    },
    #[cfg(target_arch = "wasm32")]
    Ready(Result<Box<Gpu>, HostError>),
    #[cfg(target_arch = "wasm32")]
    VerifyWebFocus,
    #[cfg(target_arch = "wasm32")]
    WebDisplayContrast,
    #[cfg(target_arch = "wasm32")]
    WebAccessibility(web_accessibility::Action),
    #[cfg(target_arch = "wasm32")]
    WebPointerCaptureLost,
}
#[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
impl From<accesskit_winit::Event> for UserEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        Self::AccessKit(event)
    }
}
struct Host<A: Application> {
    runtime: Rc<RefCell<Runtime<A>>>,
    options: Options,
    gpu: Option<Gpu>,
    window: Option<Arc<Window>>,
    #[cfg_attr(
        not(any(target_arch = "wasm32", feature = "devtools")),
        allow(dead_code)
    )]
    proxy: EventLoopProxy<UserEvent>,
    wake: WakeHandle,
    /// Set once the loop began to exit; later close requests are ignored.
    closing: bool,
    #[cfg(feature = "devtools")]
    bridge: Option<devtools::Bridge>,
    #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
    devtools: Option<devtools::DevtoolsServer>,
    #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
    trace: Option<trace_file::TraceFile>,
    pointer: (f32, f32),
    touch: touch_route::TouchRoute,
    #[cfg(not(target_arch = "wasm32"))]
    cursor: surface_app::CursorStyle,
    mods: key_order::Mods,
    #[cfg(not(target_arch = "wasm32"))]
    route: pointer_route::PointerRoute,
    #[cfg(target_arch = "wasm32")]
    key_order: key_order::KeyOrder<winit::event::KeyEvent>,
    frames: u32,
    error: Option<HostError>,
    clock: motion::Clock,
    #[cfg(target_arch = "wasm32")]
    web_input: Option<web_input::WebInput<A>>,
    #[cfg(target_arch = "wasm32")]
    web_accessibility: Option<web_accessibility::WebAccessibility>,
    #[cfg(target_arch = "wasm32")]
    web_pointer_capture: Option<web_pointer_capture::WebPointerCapture>,
    #[cfg(target_arch = "wasm32")]
    reduced_motion: Option<motion::ReducedMotionWatch>,
    #[cfg(target_arch = "wasm32")]
    web_display_contrast: Option<web_contrast::DisplayContrastWatch>,
    #[cfg(target_os = "macos")]
    _mac_display_options: motion::MacDisplayOptionsWatch,
    #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
    accessibility_adapter: Option<accesskit_winit::Adapter>,
    #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
    accessibility_tree: macos_accessibility::Tree,
    #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
    accessibility_geometry: Option<(winit::dpi::PhysicalSize<u32>, f32)>,
    #[cfg(not(target_arch = "wasm32"))]
    clipboard: Option<arboard::Clipboard>,
    #[cfg(not(target_arch = "wasm32"))]
    ime: ime_session::ImeSession,
}
impl<A: Application> Host<A> {
    #[cfg(target_arch = "wasm32")]
    fn refresh_web_display_contrast(&mut self) {
        if self.closing {
            return;
        }
        let Some(watch) = &mut self.web_display_contrast else {
            return;
        };
        let rebuilt = watch.refresh(&mut self.runtime.borrow_mut());
        if rebuilt {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    /// Handles the held web key press with the modifiers seen so far.
    #[cfg(target_arch = "wasm32")]
    fn deliver_held_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        held: Option<(winit::event::KeyEvent, key_order::Mods)>,
    ) {
        if let Some((event, mods)) = held {
            self.mods = mods;
            self.key_input(event_loop, event);
        }
    }
    #[cfg(target_arch = "wasm32")]
    fn flush_held_key(&mut self, event_loop: &ActiveEventLoop) {
        let held = self.key_order.flush();
        self.deliver_held_key(event_loop, held);
    }
    #[cfg(feature = "devtools")]
    fn install_bridge(&mut self, event_loop: &ActiveEventLoop) {
        let result =
            devtools::Bridge::install(self.proxy.clone(), self.runtime.borrow().app().title());
        match result {
            Ok(bridge) => self.bridge = bridge,
            Err(error) => self.fail(event_loop, error),
        }
    }
    /// Starts the devtools protocol transport when `SURFACE_DEVTOOLS_SESSION`
    /// is set. It has its own listener, token and session file.
    #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
    fn install_devtools(&mut self, event_loop: &ActiveEventLoop) {
        if self.error.is_some() {
            return;
        }
        let result = devtools::DevtoolsServer::install(
            self.proxy.clone(),
            self.runtime.borrow().app().title(),
        );
        match result {
            Ok(server) => {
                let methods: &'static [&'static str] = if server.is_some()
                    && self
                        .gpu
                        .as_ref()
                        .is_some_and(|gpu| screenshot::supports_format(gpu.config.format))
                {
                    &[surface_devtools_protocol::PAGE_SCREENSHOT]
                } else {
                    &[]
                };
                self.runtime.borrow_mut().set_host_methods(methods);
                self.devtools = server;
            }
            Err(error) => self.fail(event_loop, error),
        }
    }
    /// Like `bridge_request`, on the devtools executor.
    #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
    fn devtools_request(
        &mut self,
        event_loop: &ActiveEventLoop,
        request: surface_devtools_protocol::Request,
        reply: devtools::DevtoolsReply,
    ) {
        use surface_devtools_protocol::{ErrorCode, Reply, PAGE_SCREENSHOT};
        if request.method == PAGE_SCREENSHOT {
            reply.send(self.screenshot(event_loop, request));
            return;
        }
        if self.gpu.is_none() || self.window.is_none() {
            reply.send(Reply::error(
                request.id,
                ErrorCode::NotReady,
                "the application renderer is not ready",
            ));
            return;
        }
        // Read queries must use current geometry. Actions then present their new state.
        self.draw(event_loop);
        if self.error.is_some() {
            reply.send(Reply::error(
                request.id,
                ErrorCode::RenderFailed,
                "the application renderer failed",
            ));
            return;
        }
        let mutating = request.method.starts_with("Action.");
        let id = request.id;
        let response = self.runtime.borrow_mut().execute_devtools(request);
        if mutating {
            self.draw(event_loop);
            if self.error.is_some() {
                reply.send(Reply::error(
                    id,
                    ErrorCode::RenderFailed,
                    "the renderer failed after handling the action; application state may have changed",
                ));
                return;
            }
        }
        reply.send(response);
    }

    #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
    fn screenshot(
        &mut self,
        event_loop: &ActiveEventLoop,
        request: surface_devtools_protocol::Request,
    ) -> surface_devtools_protocol::Reply {
        use surface_devtools_protocol::{ErrorCode, Reply, VERSION};

        if request.v != VERSION {
            return Reply::error(
                request.id,
                ErrorCode::UnsupportedVersion,
                format!("use protocol version {VERSION}"),
            );
        }
        let params = match screenshot::params(&request.params) {
            Ok(params) => params,
            Err(message) => return Reply::error(request.id, ErrorCode::InvalidParams, message),
        };
        let (window, gpu) =
            match screenshot::ready(self.window.as_ref(), self.gpu.as_ref(), request.id) {
                Ok(ready) => ready,
                Err(reply) => return reply,
            };
        if !screenshot::supports_format(gpu.config.format) {
            return Reply::error(
                request.id,
                ErrorCode::RenderFailed,
                "the window uses an unsupported screenshot color format",
            );
        }
        if window.is_minimized() == Some(true) || window.is_visible() == Some(false) {
            return Reply::error(request.id, ErrorCode::NotReady, "the window is not visible");
        }
        let mut presented = None;
        self.draw_frame(event_loop, Some(&mut presented));
        if self.error.is_some() {
            return Reply::error(
                request.id,
                ErrorCode::RenderFailed,
                "the application renderer failed",
            );
        }
        let (Some(gpu), Some(frame)) = (&mut self.gpu, presented) else {
            return Reply::error(
                request.id,
                ErrorCode::NotReady,
                "no frame was presented for capture",
            );
        };
        screenshot::capture(
            &mut gpu.renderer,
            &frame.list,
            frame.size,
            frame.scale,
            request.id,
            params,
        )
    }

    #[cfg(feature = "devtools")]
    fn bridge_request(
        &mut self,
        event_loop: &ActiveEventLoop,
        request: jet_bridge_protocol::Request,
        reply: devtools::Reply,
    ) {
        use jet_bridge_protocol::Response;
        if self.gpu.is_none() || self.window.is_none() {
            reply.send(Response::error(
                request.id,
                "not_ready",
                "the application renderer is not ready",
            ));
            return;
        }
        // Read queries must use current geometry. Actions then present their new state.
        self.draw(event_loop);
        if self.error.is_some() {
            reply.send(Response::error(
                request.id,
                "render_failed",
                "the application renderer failed",
            ));
            return;
        }
        let mutating = request.command.is_mutating();
        let id = request.id;
        let response = self.runtime.borrow_mut().execute_bridge(request);
        if mutating {
            self.draw(event_loop);
            if self.error.is_some() {
                reply.send(Response::error(id, "render_failed", "the renderer failed after handling the action; application state may have changed"));
                return;
            }
        }
        reply.send(response);
    }
    /// One coalesced wake: run the app hook, rebuild, redraw once.
    fn wake_app(&mut self, event_loop: &ActiveEventLoop) {
        // Clear first: a wake that arrives while the app runs asks again.
        self.wake.acknowledge();
        let result = self.runtime.borrow_mut().wake();
        if let Err(error) = result {
            self.fail(event_loop, error);
        } else if let Some(window) = &self.window {
            window.request_redraw();
        }
        // The app may have changed the focused field, as after an input.
        self.sync_text_input();
    }
    /// The user asked to close the window. The app may veto. On allow the
    /// loop exits; `run_with_wake` then drops the host and the app state.
    fn close_requested(&mut self, event_loop: &ActiveEventLoop) {
        let step = close::close_request_step(&mut self.runtime.borrow_mut(), &mut self.closing);
        match step {
            close::HostStep::Exit => event_loop.exit(),
            close::HostStep::Stay => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                self.sync_text_input();
            }
            close::HostStep::Fail(message) => self.fail(event_loop, message),
        }
    }
    /// Exit when the app queued a close command. Called from
    /// `about_to_wait`, so every path that runs `update` is covered.
    #[cfg(not(target_arch = "wasm32"))]
    fn exit_if_app_closed(&mut self, event_loop: &ActiveEventLoop) {
        if close::should_exit(&mut self.runtime.borrow_mut(), &mut self.closing) {
            event_loop.exit();
        }
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl std::fmt::Display) {
        let message = error.to_string();
        eprintln!("Surface error: {message}");
        #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
        if let Some(trace) = &mut self.trace {
            if let Some(recorder) = self.runtime.borrow().trace() {
                trace.error(recorder, &message);
            }
            trace.finish();
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(status) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("status"))
        {
            status.set_text_content(Some(&format!("Surface error: {message}")));
        }
        self.error = Some(HostError(message));
        event_loop.exit();
    }
    fn input(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        #[cfg(target_arch = "wasm32")]
        self.stop_following_web_focus_on_press(&event);
        self.dispatch_input(event_loop, |runtime| runtime.dispatch(event));
    }
    fn reset_pointer_route(&mut self) {
        self.touch.reset();
        #[cfg(target_arch = "wasm32")]
        if let Some(capture) = &self.web_pointer_capture {
            capture.reset();
        }
    }
    #[cfg(target_arch = "wasm32")]
    fn track_web_point_down(&self, touch: Option<u64>) {
        if let Some(capture) = &self.web_pointer_capture {
            capture.accept_down(touch, self.runtime.borrow().has_point_drag());
        }
    }
    #[cfg(target_arch = "wasm32")]
    fn cancel_lost_web_point(&mut self, event_loop: &ActiveEventLoop) {
        let loss = self
            .web_pointer_capture
            .as_ref()
            .and_then(web_pointer_capture::WebPointerCapture::take_loss);
        if let Some(pointer) = loss {
            if self.touch.retire_web_contact(pointer.touch_id()) {
                self.dispatch_input(event_loop, Runtime::cancel_point_drag);
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    fn stop_following_web_focus_on_press(&mut self, event: &Event) {
        if matches!(event, Event::PointerDown { .. }) {
            if let Some(tree) = &mut self.web_accessibility {
                tree.follow_focus(false);
            }
        }
    }
    fn dispatch_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        action: impl FnOnce(&mut Runtime<A>) -> Result<bool, surface_app::AppError>,
    ) {
        let result = {
            let mut runtime = self.runtime.borrow_mut();
            if let Some(now) = self.clock.now_ms() {
                runtime.set_time(now);
            }
            action(&mut runtime)
        };
        match result {
            Ok(true) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            Ok(false) => {}
            Err(error) => self.fail(event_loop, error),
        }
        self.sync_text_input();
        #[cfg(not(target_arch = "wasm32"))]
        self.sync_cursor();
    }
    #[cfg(target_arch = "wasm32")]
    fn verify_web_focus(&mut self, event_loop: &ActiveEventLoop) {
        use winit::platform::web::WindowExtWebSys;
        let canvas_focused = self
            .window
            .as_ref()
            .and_then(|window| window.canvas())
            .is_some_and(|canvas| {
                web_sys::window()
                    .and_then(|browser| browser.document())
                    .and_then(|document| document.active_element())
                    .is_some_and(|active| active == *canvas.as_ref())
            });
        if canvas_focused
            || self
                .web_input
                .as_ref()
                .is_some_and(|input| input.owns_focus())
            || self
                .web_accessibility
                .as_ref()
                .is_some_and(|tree| tree.owns_focus())
        {
            return;
        }
        self.reset_pointer_route();
        if let Some(tree) = &mut self.web_accessibility {
            tree.follow_focus(false);
        }
        self.input(event_loop, Event::Blur);
    }
    #[cfg(target_arch = "wasm32")]
    fn web_accessibility_action(
        &mut self,
        event_loop: &ActiveEventLoop,
        action: web_accessibility::Action,
    ) {
        use surface_app::{AccessibilityActivationResult, AccessibilityFocusResult};
        use web_accessibility::Action;
        if matches!(action, Action::Press(_))
            && self.runtime.borrow().accessibility_frame().is_none()
        {
            self.draw(event_loop);
            if self.closing || self.error.is_some() {
                return;
            }
        }
        let mut follow_focus = false;
        self.dispatch_input(event_loop, |runtime| match action {
            Action::Focus(target) => {
                let result = runtime.accessibility_focus(&target)?;
                follow_focus = matches!(
                    result,
                    AccessibilityFocusResult::Moved | AccessibilityFocusResult::Unchanged
                );
                Ok(matches!(
                    result,
                    AccessibilityFocusResult::Moved | AccessibilityFocusResult::Rebuilt
                ))
            }
            Action::Press(target) => {
                let result = runtime.accessibility_activate(&target)?;
                follow_focus = result == AccessibilityActivationResult::Activated;
                Ok(matches!(
                    result,
                    AccessibilityActivationResult::Activated
                        | AccessibilityActivationResult::Rebuilt
                ))
            }
            Action::Key(source, press, event) => runtime.accessibility_key(&source, &press, event),
        });
        if follow_focus {
            if let Some(tree) = &mut self.web_accessibility {
                tree.follow_focus(true);
            }
        }
    }
    #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
    fn publish_accessibility(&mut self, size: winit::dpi::PhysicalSize<u32>, scale: f32) {
        let Some(adapter) = &mut self.accessibility_adapter else {
            return;
        };
        let runtime = self.runtime.borrow();
        let Some(frame) = runtime.accessibility_frame() else {
            return;
        };
        adapter.update_if_active(|| {
            self.accessibility_geometry = Some((size, scale));
            self.accessibility_tree.update(frame, scale)
        });
    }

    #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
    fn accessibility_event(&mut self, event_loop: &ActiveEventLoop, event: accesskit_winit::Event) {
        use accesskit_winit::WindowEvent::{
            AccessibilityDeactivated, ActionRequested, InitialTreeRequested,
        };
        let Some(window) = self
            .window
            .as_ref()
            .filter(|window| window.id() == event.window_id)
        else {
            return;
        };
        if self.closing {
            return;
        }
        match event.window_event {
            InitialTreeRequested => {
                self.accessibility_tree.reset();
                self.accessibility_geometry = None;
                window.request_redraw();
            }
            ActionRequested(request) => {
                if self.accessibility_geometry != Some(drawable_size(window)) || !window.has_focus()
                {
                    return;
                }
                let result = self
                    .accessibility_tree
                    .action(&mut self.runtime.borrow_mut(), request);
                match result {
                    Ok(true) => {
                        window.request_redraw();
                        self.sync_text_input();
                    }
                    Ok(false) => {}
                    Err(error) => self.fail(event_loop, error),
                }
            }
            AccessibilityDeactivated => {
                self.accessibility_tree.reset();
                self.accessibility_geometry = None;
            }
        }
    }
    fn sync_text_input(&mut self) {
        #[cfg(target_arch = "wasm32")]
        if let Some(input) = &self.web_input {
            input.sync();
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(window) = &self.window {
            let input = self.runtime.borrow().input_snapshot();
            let target = input
                .as_ref()
                .filter(|i| !i.read_only)
                .map(|i| i.id.clone());
            let allowed = target.is_some();
            for allowed in self.ime.focus(target) {
                window.set_ime_allowed(allowed);
            }
            if let Some(caret) = input.and_then(|i| i.caret).filter(|_| allowed) {
                window.set_ime_cursor_area(
                    winit::dpi::LogicalPosition::new(caret.x, caret.y),
                    winit::dpi::LogicalSize::new(caret.width.max(1.), caret.height.max(1.)),
                );
            }
        }
    }
    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        self.draw_frame(
            event_loop,
            #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
            None,
        );
    }

    fn draw_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))] presented: Option<
            &mut Option<PresentedFrame>,
        >,
    ) {
        let (Some(window), Some(gpu)) = (&self.window, &mut self.gpu) else {
            return;
        };
        let (size, scale) = drawable_size(window);
        if size.width == 0 || size.height == 0 {
            return;
        }
        if gpu.config.width != size.width || gpu.config.height != size.height {
            gpu.resize(size.width, size.height);
        }
        let (result, animating) = {
            let mut runtime = self.runtime.borrow_mut();
            if let Some(now) = self.clock.now_ms() {
                runtime.set_time(now);
            }
            let result = runtime.render_extended(
                size.width as f32 / scale,
                size.height as f32 / scale,
                &mut gpu.renderer.text,
            );
            (result, runtime.needs_frame())
        };
        let list = match result {
            Ok(list) => list,
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
        // Keep drawing while a transition runs. winit maps this to
        // requestAnimationFrame on the web.
        if animating {
            window.request_redraw();
        }
        let frame = match gpu.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                gpu.resize(size.width, size.height);
                window.request_redraw();
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => {
                window.request_redraw();
                return;
            }
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
        let present_start = self.trace.as_ref().map(|t| t.clock.now_ms());
        if let Err(error) =
            gpu.renderer
                .render_extended(&list, &view, size.width, size.height, scale)
        {
            self.fail(event_loop, error);
            return;
        }
        frame.present();
        #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
        if let Some(presented) = presented {
            *presented = Some(PresentedFrame {
                list,
                size: (size.width, size.height),
                scale,
            });
        }
        self.frames += 1;
        #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
        if let (Some(trace), Some(start)) = (&mut self.trace, present_start) {
            let end = trace.clock.now_ms();
            let mut runtime = self.runtime.borrow_mut();
            if let Some(recorder) = runtime.trace_mut() {
                recorder.record_present(start, end - start);
                trace.drain(recorder);
            }
        }
        #[cfg(target_arch = "wasm32")]
        if self.frames == 1 {
            if let Some(status) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("status"))
            {
                status.set_text_content(Some("Ready · Rust / WASM / WebGPU"));
            }
        }
        if let Some(limit) = self.options.close_after_frames {
            if self.frames >= limit {
                event_loop.exit();
            } else {
                window.request_redraw();
            }
        }
        self.sync_text_input();
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            let canvas = self.window.as_ref().and_then(|window| window.canvas());
            if let (Some(tree), Some(canvas)) = (&mut self.web_accessibility, canvas) {
                let result = self
                    .runtime
                    .borrow()
                    .accessibility_frame()
                    .map(|frame| tree.publish(&frame, &canvas));
                if let Some(Err(error)) = result {
                    self.fail(
                        event_loop,
                        format!("browser accessibility bridge: {error:?}"),
                    );
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.sync_cursor();
        #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
        self.publish_accessibility(size, scale);
    }
}
impl<A: Application> ApplicationHandler<UserEvent> for Host<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(self.runtime.borrow().app().title())
            .with_inner_size(LogicalSize::new(self.options.width, self.options.height));
        #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
        let attributes = attributes.with_visible(false);
        #[cfg(target_arch = "wasm32")]
        let attributes = {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("surface"))
                .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok());
            let Some(canvas) = canvas else {
                self.fail(event_loop, "missing canvas#surface");
                return;
            };
            attributes
                .with_canvas(Some(canvas))
                .with_prevent_default(true)
        };
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
        self.window = Some(window.clone());
        #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
        {
            self.accessibility_adapter = Some(accesskit_winit::Adapter::with_event_loop_proxy(
                event_loop,
                &window,
                self.proxy.clone(),
            ));
            window.set_visible(true);
        }
        #[cfg(target_arch = "wasm32")]
        match web_input::WebInput::new(self.runtime.clone(), window.clone()) {
            Ok(input) => self.web_input = Some(input),
            Err(error) => {
                self.fail(event_loop, format!("browser input bridge: {error:?}"));
                return;
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            if let Some(canvas) = window.canvas() {
                match web_pointer_capture::WebPointerCapture::new(canvas, self.proxy.clone()) {
                    Ok(capture) => self.web_pointer_capture = Some(capture),
                    Err(error) => {
                        self.fail(event_loop, format!("browser pointer capture: {error:?}"));
                        return;
                    }
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        match web_accessibility::WebAccessibility::new(self.proxy.clone()) {
            Ok(tree) => self.web_accessibility = Some(tree),
            Err(error) => {
                self.fail(
                    event_loop,
                    format!("browser accessibility bridge: {error:?}"),
                );
                return;
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;

            self.reduced_motion =
                motion::ReducedMotionWatch::new(self.runtime.clone(), window.clone());
            if let Some(canvas) = window.canvas() {
                let proxy = self.proxy.clone();
                match web_contrast::DisplayContrastWatch::new(canvas, move || {
                    let _ = proxy.send_event(UserEvent::WebDisplayContrast);
                }) {
                    Ok(watch) => self.web_display_contrast = watch,
                    Err(error) => web_sys::console::warn_2(
                        &"Surface could not observe Web display contrast:".into(),
                        &error,
                    ),
                }
            }
            self.refresh_web_display_contrast();
        }
        #[cfg(not(target_arch = "wasm32"))]
        match pollster::block_on(Gpu::new(window)) {
            Ok(gpu) => {
                self.gpu = Some(gpu);
                #[cfg(feature = "devtools")]
                self.install_bridge(event_loop);
                #[cfg(feature = "devtools")]
                self.install_devtools(event_loop);
                self.window.as_ref().unwrap().request_redraw();
            }
            Err(error) => self.fail(event_loop, error),
        }
        #[cfg(target_arch = "wasm32")]
        {
            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let result = Gpu::new(window).await.map(Box::new);
                let _ = proxy.send_event(UserEvent::Ready(result));
            });
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Wake => self.wake_app(event_loop),
            #[cfg(target_arch = "wasm32")]
            UserEvent::WebDisplayContrast => self.refresh_web_display_contrast(),
            #[cfg(target_os = "macos")]
            UserEvent::MacDisplayOptionsChanged => {
                let changed = !self.closing
                    && motion::refresh_display_options(&mut self.runtime.borrow_mut());
                if changed {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
            UserEvent::AccessKit(event) => self.accessibility_event(event_loop, event),
            #[cfg(feature = "devtools")]
            UserEvent::Bridge { request, reply } => self.bridge_request(event_loop, request, reply),
            #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
            UserEvent::Devtools { request, reply } => {
                self.devtools_request(event_loop, *request, reply)
            }
            #[cfg(target_arch = "wasm32")]
            UserEvent::Ready(Ok(gpu)) => {
                self.gpu = Some(*gpu);
                #[cfg(feature = "devtools")]
                self.install_bridge(event_loop);
                if let (Some(window), Some(gpu)) = (&self.window, &mut self.gpu) {
                    let size = window.inner_size();
                    gpu.resize(size.width, size.height);
                    window.request_redraw();
                }
            }
            #[cfg(target_arch = "wasm32")]
            UserEvent::Ready(Err(error)) => self.fail(event_loop, error),
            #[cfg(target_arch = "wasm32")]
            UserEvent::VerifyWebFocus => self.verify_web_focus(event_loop),
            #[cfg(target_arch = "wasm32")]
            UserEvent::WebAccessibility(action) => {
                self.web_accessibility_action(event_loop, action)
            }
            #[cfg(target_arch = "wasm32")]
            UserEvent::WebPointerCaptureLost => self.cancel_lost_web_point(event_loop),
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        self.flush_held_key(event_loop);
        #[cfg(not(target_arch = "wasm32"))]
        self.exit_if_app_closed(event_loop);
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Chrome may deliver the next move or up immediately after loss.
        // Drain before winit can dispatch either for the old contact.
        #[cfg(target_arch = "wasm32")]
        self.cancel_lost_web_point(event_loop);
        #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
        if let (Some(window), Some(adapter)) = (
            self.window.as_ref().filter(|window| window.id() == _id),
            &mut self.accessibility_adapter,
        ) {
            adapter.process_event(window, &event);
        }
        // The web backend sends a key press before its modifier change.
        // Any other event first handles the held key, which keeps the order.
        #[cfg(target_arch = "wasm32")]
        if !matches!(
            event,
            WindowEvent::KeyboardInput { .. } | WindowEvent::ModifiersChanged(_)
        ) {
            self.flush_held_key(event_loop);
        }
        match event {
            WindowEvent::CloseRequested => self.close_requested(event_loop),
            WindowEvent::RedrawRequested => self.draw(event_loop),
            WindowEvent::Resized(size) => {
                self.reset_pointer_route();
                self.dispatch_input(event_loop, Runtime::pointer_geometry_changed);
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                self.reset_pointer_route();
                self.dispatch_input(event_loop, Runtime::pointer_geometry_changed);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.touch.active() {
                    return;
                }
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.);
                self.pointer = (position.x as f32 / scale, position.y as f32 / scale);
                self.input(
                    event_loop,
                    Event::PointerMove {
                        x: self.pointer.0,
                        y: self.pointer.1,
                    },
                );
            }
            #[cfg(not(target_arch = "wasm32"))]
            WindowEvent::MouseInput { state, button, .. } => {
                if self.touch.active() {
                    return;
                }
                let step = self.route.route_mouse(
                    pointer_route::from_winit_button(button),
                    state == ElementState::Pressed,
                    self.mods,
                    self.pointer,
                    cfg!(target_os = "macos"),
                );
                match step {
                    pointer_route::PointerStep::Primary(event)
                        if self.touch.mouse_primary(state) =>
                    {
                        self.input(event_loop, event);
                    }
                    pointer_route::PointerStep::Secondary(event) => self.input(event_loop, event),
                    _ => {}
                }
            }
            #[cfg(target_arch = "wasm32")]
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                if !self.touch.mouse_primary(state) {
                    return;
                }
                let (x, y) = self.pointer;
                let down = state == ElementState::Pressed;
                if !down {
                    if let Some(capture) = &self.web_pointer_capture {
                        capture.mouse_up();
                    }
                }
                self.input(
                    event_loop,
                    if down {
                        Event::PointerDown { x, y }
                    } else {
                        Event::PointerUp { x, y }
                    },
                );
                if down {
                    self.track_web_point_down(None);
                }
            }
            WindowEvent::Touch(touch) => {
                let scale = self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.);
                #[cfg(target_arch = "wasm32")]
                let touch_id = touch.id;
                match self.touch.route(touch, scale) {
                    touch_route::TouchStep::Pointer(event) => {
                        #[cfg(target_arch = "wasm32")]
                        self.stop_following_web_focus_on_press(&event);
                        #[cfg(target_arch = "wasm32")]
                        let down = matches!(event, Event::PointerDown { .. });
                        let released = matches!(event, Event::PointerUp { .. });
                        self.dispatch_input(event_loop, move |runtime| {
                            let changed = runtime.dispatch_touch(event)?;
                            let left = released && runtime.dispatch(Event::PointerLeave)?;
                            Ok(changed || left)
                        });
                        #[cfg(target_arch = "wasm32")]
                        if down {
                            self.track_web_point_down(Some(touch_id));
                        }
                    }
                    touch_route::TouchStep::Cancel => {
                        self.dispatch_input(event_loop, |runtime| {
                            let changed = runtime.cancel_pointer()?;
                            Ok(runtime.dispatch(Event::PointerLeave)? || changed)
                        });
                    }
                    touch_route::TouchStep::Ignore => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (-x * 32., -y * 32.),
                    MouseScrollDelta::PixelDelta(p) => {
                        let s = self
                            .window
                            .as_ref()
                            .map(|w| w.scale_factor() as f32)
                            .unwrap_or(1.);
                        (-p.x as f32 / s, -p.y as f32 / s)
                    }
                };
                self.input(
                    event_loop,
                    Event::Scroll {
                        x: self.pointer.0,
                        y: self.pointer.1,
                        delta_x: dx,
                        delta_y: dy,
                    },
                );
            }
            #[cfg(target_arch = "wasm32")]
            WindowEvent::ModifiersChanged(m) => {
                self.mods = key_route::from_winit_mods(m.state());
                let held = self.key_order.modifiers(self.mods);
                self.deliver_held_key(event_loop, held);
            }
            #[cfg(not(target_arch = "wasm32"))]
            WindowEvent::ModifiersChanged(m) => {
                self.mods = key_route::from_winit_mods(m.state());
            }
            WindowEvent::Focused(false) => {
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = self.proxy.send_event(UserEvent::VerifyWebFocus);
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.reset_pointer_route();
                    self.route = pointer_route::PointerRoute::default();
                    self.input(event_loop, Event::Blur);
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.input(event_loop, Event::PointerLeave);
                #[cfg(not(target_arch = "wasm32"))]
                self.apply_cursor(surface_app::CursorStyle::Default);
            }
            #[cfg(target_arch = "wasm32")]
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let held = self.key_order.press(event);
                self.deliver_held_key(event_loop, held);
            }
            #[cfg(not(target_arch = "wasm32"))]
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                self.key_input(event_loop, event);
            }
            #[cfg(not(target_arch = "wasm32"))]
            WindowEvent::Ime(event) => {
                if let Some(event) = self.ime.event(event) {
                    self.input(event_loop, event);
                }
            }
            _ => {}
        }
    }
}
pub fn run<A: Application>(app: A) -> Result<(), HostError> {
    let runtime = Runtime::new(app).map_err(|e| HostError(e.to_string()))?;
    run_runtime(Rc::new(RefCell::new(runtime)), Options::default())
}
pub fn run_runtime<A: Application>(
    runtime: Rc<RefCell<Runtime<A>>>,
    options: Options,
) -> Result<(), HostError> {
    run_with_wake(options, |_| Ok(runtime))
}
/// Like `run_runtime`, but first gives `build` a `WakeHandle` for the new
/// event loop. Keep the handle in your worker, wrap it in
/// `Arc<dyn Fn() + Send + Sync>`, and call it from any thread. The host then
/// calls `Application::wake` once per coalesced wake and redraws.
pub fn run_with_wake<A: Application>(
    options: Options,
    build: impl FnOnce(WakeHandle) -> Result<Rc<RefCell<Runtime<A>>>, HostError>,
) -> Result<(), HostError> {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(|e| HostError(e.to_string()))?;
    let proxy = SendProxy(event_loop.create_proxy());
    let wake = WakeHandle::new(move || {
        // The loop is gone after exit; a late wake is then ignored.
        let _ = proxy.send_wake();
    });
    let runtime = build(wake.clone())?;
    #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
    let trace = trace_file::TraceFile::from_env_value(std::env::var_os(trace_file::ENV))
        .map_err(HostError)?;
    #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
    if let Some(trace) = &trace {
        eprintln!("Surface trace: writing {}", trace.path().display());
        runtime
            .borrow_mut()
            .attach_trace(surface_app::trace::Recorder::new(trace.recorder_clock()));
    }
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos")))]
    if motion::reduced_motion_from_env() {
        runtime.borrow_mut().set_reduced_motion(true);
    }
    #[cfg(target_os = "macos")]
    let mac_display_options = {
        let proxy = event_loop.create_proxy();
        let watch = motion::MacDisplayOptionsWatch::new(move || {
            let _ = proxy.send_event(UserEvent::MacDisplayOptionsChanged);
        });
        motion::refresh_display_options(&mut runtime.borrow_mut());
        watch
    };
    let host = Host {
        runtime,
        options,
        gpu: None,
        window: None,
        proxy: event_loop.create_proxy(),
        wake,
        closing: false,
        #[cfg(feature = "devtools")]
        bridge: None,
        #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
        devtools: None,
        #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
        trace,
        pointer: (0., 0.),
        touch: touch_route::TouchRoute::default(),
        #[cfg(not(target_arch = "wasm32"))]
        cursor: surface_app::CursorStyle::Default,
        mods: key_order::Mods::default(),
        #[cfg(not(target_arch = "wasm32"))]
        route: pointer_route::PointerRoute::default(),
        #[cfg(target_arch = "wasm32")]
        key_order: key_order::KeyOrder::new(),
        frames: 0,
        error: None,
        clock: motion::Clock::new(),
        #[cfg(target_arch = "wasm32")]
        web_input: None,
        #[cfg(target_arch = "wasm32")]
        web_accessibility: None,
        #[cfg(target_arch = "wasm32")]
        web_pointer_capture: None,
        #[cfg(target_arch = "wasm32")]
        reduced_motion: None,
        #[cfg(target_arch = "wasm32")]
        web_display_contrast: None,
        #[cfg(target_os = "macos")]
        _mac_display_options: mac_display_options,
        #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
        accessibility_adapter: None,
        #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
        accessibility_tree: macos_accessibility::Tree::default(),
        #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
        accessibility_geometry: None,
        #[cfg(not(target_arch = "wasm32"))]
        clipboard: None,
        #[cfg(not(target_arch = "wasm32"))]
        ime: ime_session::ImeSession::default(),
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut host = host;
        let result = event_loop.run_app(&mut host);
        // Close the window before the app cleanup runs in `Drop`, so a slow
        // child stop or worker join does not freeze a visible window. This
        // runs on the `Err` path too, before `?` drops the runtime.
        let window = host.window.as_ref().map(Arc::downgrade);
        #[cfg(all(feature = "devtools", not(target_arch = "wasm32")))]
        if let Some(trace) = &mut host.trace {
            if let Some(recorder) = host.runtime.borrow().trace() {
                trace.drain(recorder);
            }
            trace.finish();
        }
        host.gpu = None;
        #[cfg(all(target_os = "macos", feature = "macos-accessibility"))]
        {
            host.accessibility_adapter = None;
        }
        host.window = None;
        if let Some(window) = window {
            let state = if window.strong_count() == 0 { 1 } else { 2 };
            WINDOW_RELEASED.store(state, std::sync::atomic::Ordering::SeqCst);
        }
        result.map_err(|e| HostError(e.to_string()))?;
        if let Some(error) = host.error {
            return Err(error);
        }
        if let Some(expected) = host.options.close_after_frames {
            if host.frames < expected {
                return Err(HostError(format!(
                    "smoke run closed after {} of {expected} frames",
                    host.frames
                )));
            }
            eprintln!("Surface smoke: presented {} native GPU frames", host.frames);
        }
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(host);
        Ok(())
    }
}
