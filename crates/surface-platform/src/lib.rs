//! Thin desktop/browser window host. The application never sees platform events.
#[cfg(feature = "devtools")]
mod devtools;
#[cfg(any(target_arch = "wasm32", test))]
mod host_style;
#[cfg(not(target_arch = "wasm32"))]
mod ime_session;
mod key_order;
mod keyboard;
mod motion;
mod wake;
#[cfg(any(target_arch = "wasm32", test))]
mod press_guard;
#[cfg(target_arch = "wasm32")]
mod web_input;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use surface_app::{Application, Event, Runtime};
use surface_wgpu::Renderer;
#[cfg(any(target_arch = "wasm32", feature = "devtools"))]
use winit::event_loop::EventLoopProxy;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
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
struct Gpu {
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    config: wgpu::SurfaceConfiguration,
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
enum UserEvent {
    #[cfg(feature = "devtools")]
    Bridge {
        request: jet_bridge_protocol::Request,
        reply: devtools::Reply,
    },
    #[cfg(target_arch = "wasm32")]
    Ready(Result<Box<Gpu>, HostError>),
}
struct Host<A: Application> {
    runtime: Rc<RefCell<Runtime<A>>>,
    options: Options,
    gpu: Option<Gpu>,
    window: Option<Arc<Window>>,
    #[cfg(any(target_arch = "wasm32", feature = "devtools"))]
    proxy: EventLoopProxy<UserEvent>,
    #[cfg(feature = "devtools")]
    bridge: Option<devtools::Bridge>,
    pointer: (f32, f32),
    shift: bool,
    control: bool,
    #[cfg(target_arch = "wasm32")]
    key_order: key_order::KeyOrder<winit::event::KeyEvent>,
    frames: u32,
    error: Option<HostError>,
    clock: motion::Clock,
    #[cfg(target_arch = "wasm32")]
    web_input: Option<web_input::WebInput<A>>,
    #[cfg(target_arch = "wasm32")]
    reduced_motion: Option<motion::ReducedMotionWatch>,
    #[cfg(not(target_arch = "wasm32"))]
    clipboard: Option<arboard::Clipboard>,
    #[cfg(not(target_arch = "wasm32"))]
    ime: ime_session::ImeSession,
}
impl<A: Application> Host<A> {
    /// Handles the held web key press with the modifiers seen so far.
    #[cfg(target_arch = "wasm32")]
    fn deliver_held_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        held: Option<(winit::event::KeyEvent, key_order::Mods)>,
    ) {
        if let Some((event, mods)) = held {
            self.shift = mods.shift;
            self.control = mods.control;
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
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl std::fmt::Display) {
        let message = error.to_string();
        eprintln!("Surface error: {message}");
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
        let result = self.runtime.borrow_mut().dispatch(event);
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
            let result = runtime.render(
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
        if let Err(error) = gpu
            .renderer
            .render(&list, &view, size.width, size.height, scale)
        {
            self.fail(event_loop, error);
            return;
        }
        frame.present();
        self.frames += 1;
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
            self.reduced_motion =
                motion::ReducedMotionWatch::new(self.runtime.clone(), window.clone());
        }
        #[cfg(not(target_arch = "wasm32"))]
        match pollster::block_on(Gpu::new(window)) {
            Ok(gpu) => {
                self.gpu = Some(gpu);
                #[cfg(feature = "devtools")]
                self.install_bridge(event_loop);
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
    #[cfg(any(target_arch = "wasm32", feature = "devtools"))]
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            #[cfg(feature = "devtools")]
            UserEvent::Bridge { request, reply } => self.bridge_request(event_loop, request, reply),
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
        }
    }
    #[cfg(target_arch = "wasm32")]
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.flush_held_key(event_loop);
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
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
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.draw(event_loop),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
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
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let (x, y) = self.pointer;
                self.input(
                    event_loop,
                    if state == ElementState::Pressed {
                        Event::PointerDown { x, y }
                    } else {
                        Event::PointerUp { x, y }
                    },
                );
            }
            WindowEvent::Touch(touch) => {
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.);
                let (x, y) = (
                    touch.location.x as f32 / scale,
                    touch.location.y as f32 / scale,
                );
                match touch.phase {
                    winit::event::TouchPhase::Started => {
                        self.input(event_loop, Event::PointerDown { x, y })
                    }
                    winit::event::TouchPhase::Ended => {
                        self.input(event_loop, Event::PointerUp { x, y })
                    }
                    winit::event::TouchPhase::Cancelled => self.input(event_loop, Event::Cancel),
                    _ => {}
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
                let state = m.state();
                let held = self.key_order.modifiers(key_order::Mods {
                    shift: state.shift_key(),
                    control: state.control_key() || state.super_key(),
                });
                self.shift = state.shift_key();
                self.control = state.control_key() || state.super_key();
                self.deliver_held_key(event_loop, held);
            }
            #[cfg(not(target_arch = "wasm32"))]
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.control = m.state().control_key() || m.state().super_key();
            }
            WindowEvent::Focused(false) => {
                #[cfg(target_arch = "wasm32")]
                // Canvas -> input host keeps focus inside this application.
                // The canvas blur can run before the textarea's focus event.
                if self
                    .web_input
                    .as_ref()
                    .is_some_and(|input| input.owns_focus())
                {
                    return;
                }
                self.input(event_loop, Event::Blur)
            }
            WindowEvent::CursorLeft { .. } => self.input(event_loop, Event::PointerLeave),
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
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(|e| HostError(e.to_string()))?;
    #[cfg(not(target_arch = "wasm32"))]
    if motion::reduced_motion_from_env() {
        runtime.borrow_mut().set_reduced_motion(true);
    }
    let host = Host {
        runtime,
        options,
        gpu: None,
        window: None,
        #[cfg(any(target_arch = "wasm32", feature = "devtools"))]
        proxy: event_loop.create_proxy(),
        #[cfg(feature = "devtools")]
        bridge: None,
        pointer: (0., 0.),
        shift: false,
        control: false,
        #[cfg(target_arch = "wasm32")]
        key_order: key_order::KeyOrder::new(),
        frames: 0,
        error: None,
        clock: motion::Clock::new(),
        #[cfg(target_arch = "wasm32")]
        web_input: None,
        #[cfg(target_arch = "wasm32")]
        reduced_motion: None,
        #[cfg(not(target_arch = "wasm32"))]
        clipboard: None,
        #[cfg(not(target_arch = "wasm32"))]
        ime: ime_session::ImeSession::default(),
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut host = host;
        event_loop
            .run_app(&mut host)
            .map_err(|e| HostError(e.to_string()))?;
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
