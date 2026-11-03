//! Thin desktop/browser window host. The application never sees platform events.
use std::{cell::RefCell, rc::Rc, sync::Arc};
use surface_app::{Application, Event, Runtime};
use surface_wgpu::Renderer;
#[cfg(target_arch = "wasm32")]
use winit::event_loop::EventLoopProxy;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
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
    #[cfg(target_arch = "wasm32")]
    Ready(Result<Gpu, HostError>),
}
struct Host<A: Application> {
    runtime: Rc<RefCell<Runtime<A>>>,
    options: Options,
    gpu: Option<Gpu>,
    window: Option<Arc<Window>>,
    #[cfg(target_arch = "wasm32")]
    proxy: EventLoopProxy<UserEvent>,
    pointer: (f32, f32),
    shift: bool,
    control: bool,
    frames: u32,
    error: Option<HostError>,
}
impl<A: Application> Host<A> {
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
        let result = self.runtime.borrow_mut().render(
            size.width as f32 / scale,
            size.height as f32 / scale,
            &mut gpu.renderer.text,
        );
        let list = match result {
            Ok(list) => list,
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
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
        window.set_ime_allowed(true);
        self.window = Some(window.clone());
        #[cfg(not(target_arch = "wasm32"))]
        match pollster::block_on(Gpu::new(window)) {
            Ok(gpu) => {
                self.gpu = Some(gpu);
                self.window.as_ref().unwrap().request_redraw();
            }
            Err(error) => self.fail(event_loop, error),
        }
        #[cfg(target_arch = "wasm32")]
        {
            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let result = Gpu::new(window).await;
                let _ = proxy.send_event(UserEvent::Ready(result));
            });
        }
    }
    #[cfg(target_arch = "wasm32")]
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Ready(Ok(gpu)) => {
                self.gpu = Some(gpu);
                if let (Some(window), Some(gpu)) = (&self.window, &mut self.gpu) {
                    let size = window.inner_size();
                    gpu.resize(size.width, size.height);
                    window.request_redraw();
                }
            }
            UserEvent::Ready(Err(error)) => self.fail(event_loop, error),
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
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
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.control = m.state().control_key() || m.state().super_key();
            }
            WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. } => {
                self.input(event_loop, Event::Cancel)
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key {
                    Key::Named(NamedKey::Tab) => self.input(
                        event_loop,
                        Event::FocusNext {
                            reverse: self.shift,
                        },
                    ),
                    Key::Named(NamedKey::Backspace) => self.input(event_loop, Event::Backspace),
                    Key::Named(NamedKey::Enter) => self.input(event_loop, Event::Activate),
                    Key::Named(NamedKey::Escape) => self.input(event_loop, Event::Cancel),
                    _ if !self.control => {
                        let text_field = {
                            let r = self.runtime.borrow();
                            r.focused()
                                .and_then(|id| r.dom().by_id(id))
                                .and_then(|id| r.dom().get(id).ok())
                                .is_some_and(|n| n.role() == surface::Role::TextInput)
                        };
                        if event.logical_key == Key::Named(NamedKey::Space) && !text_field {
                            self.input(event_loop, Event::Activate);
                        } else if let Some(text) = event.text {
                            self.input(event_loop, Event::Text(text.to_string()));
                        }
                    }
                    _ => {}
                }
            }
            // Basic committed input only. Full IME pre-edit and caret UI are not yet implemented.
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                self.input(event_loop, Event::Text(text))
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
    let host = Host {
        runtime,
        options,
        gpu: None,
        window: None,
        #[cfg(target_arch = "wasm32")]
        proxy: event_loop.create_proxy(),
        pointer: (0., 0.),
        shift: false,
        control: false,
        frames: 0,
        error: None,
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
