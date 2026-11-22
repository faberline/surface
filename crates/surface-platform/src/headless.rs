use crate::{devtools::DevtoolsServer, screenshot, HostError};
use std::time::Instant;
use surface_app::{Application, Runtime};
use surface_devtools_protocol::{ErrorCode, Reply, PAGE_SCREENSHOT, VERSION};
use surface_wgpu::{Capture, Renderer};

/// Serve a native runtime through DevTools at a fixed logical viewport.
///
/// Requires `SURFACE_DEVTOOLS_SESSION` to name a new private session file.
/// Blocks the calling thread and uses the native GPU without creating a window
/// or an event loop. Frames follow requests and use Reduce Motion. Screenshots
/// use a base scale of one pixel per logical unit.
pub fn run_headless_runtime<A: Application>(
    mut runtime: Runtime<A>,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = DevtoolsServer::session_path()
        .ok_or_else(|| HostError::new("headless runtime requires SURFACE_DEVTOOLS_SESSION"))?;
    Capture::check_size(width, height)?;
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = pollster::block_on(Renderer::request(&instance, None))?;
    let mut renderer =
        pollster::block_on(Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm))?;

    runtime.set_reduced_motion(true);
    runtime.set_host_methods(&[PAGE_SCREENSHOT]);
    let start = Instant::now();
    let draw = |runtime: &mut Runtime<A>, renderer: &mut Renderer| {
        runtime.set_time(start.elapsed().as_secs_f64() * 1_000.);
        runtime.render_extended(width as f32, height as f32, &mut renderer.text)
    };
    draw(&mut runtime, &mut renderer)?;

    let (_server, requests) = DevtoolsServer::start_queued(path, runtime.app().title())?;
    for (request, reply) in requests {
        let id = request.id;
        if request.v != VERSION {
            reply.send(Reply::error(
                id,
                ErrorCode::UnsupportedVersion,
                format!("use protocol version {VERSION}"),
            ));
            continue;
        }
        let frame = match draw(&mut runtime, &mut renderer) {
            Ok(frame) => frame,
            Err(error) => {
                reply.send(Reply::error(id, ErrorCode::RenderFailed, error.to_string()));
                return Err(error.into());
            }
        };
        if request.method == PAGE_SCREENSHOT {
            let response = match screenshot::params(&request.params) {
                Ok(params) => {
                    screenshot::capture(&mut renderer, &frame, (width, height), 1., id, params)
                }
                Err(message) => Reply::error(id, ErrorCode::InvalidParams, message),
            };
            reply.send(response);
            continue;
        }
        let mutating = request.method.starts_with("Action.");
        let response = runtime.execute_devtools(request);
        if mutating && response.ok {
            if let Err(error) = draw(&mut runtime, &mut renderer) {
                reply.send(Reply::error(
                    id,
                    ErrorCode::RenderFailed,
                    "the renderer failed after handling the action; application state may have changed",
                ));
                return Err(error.into());
            }
        }
        reply.send(response);
    }
    Err(HostError::new("the headless DevTools server closed").into())
}
