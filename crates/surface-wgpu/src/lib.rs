//! One ordered wgpu renderer for native GPU surfaces and browser WebGPU.
mod text;
use glyphon::{
    Cache, Resolution, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use std::borrow::Cow;
use surface::{Color, Corners, DisplayList, PaintOp, Rect, TextStyle};
pub use text::TextSystem;
use wgpu::util::DeviceExt;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("no compatible GPU adapter")]
    NoAdapter,
    #[error("GPU device request failed: {0}")]
    Device(#[from] wgpu::RequestDeviceError),
    #[error("text preparation failed: {0}")]
    Prepare(#[from] glyphon::PrepareError),
    #[error("text render failed: {0}")]
    Text(#[from] glyphon::RenderError),
    #[error("invalid render dimensions or scale")]
    Dimensions,
    #[error("capture of {width}x{height} exceeds the {max} pixel side limit")]
    CaptureTooLarge { width: u32, height: u32, max: u32 },
    #[error("capture readback failed: {0}")]
    Readback(String),
}
/// Largest width or height `Renderer::capture` accepts.
pub const MAX_CAPTURE_SIDE: u32 = 8192;
/// Tightly packed RGBA8 pixels, row 0 at the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
impl Capture {
    /// Reject a size `Renderer::capture` would refuse. Needs no GPU.
    pub fn check_size(width: u32, height: u32) -> Result<(), RenderError> {
        let _ = (width, height);
        unimplemented!("red commit: capture size check")
    }
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    local: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
    /// Top-left, top-right, bottom-right, bottom-left.
    radius: [f32; 4],
    stroke: f32,
}
const ATTRS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Float32x2,3=>Float32x4,4=>Float32x4,5=>Float32];
fn corner_array(c: &Corners) -> [f32; 4] {
    [c.top_left, c.top_right, c.bottom_right, c.bottom_left]
}
enum Draw {
    Quad { start: u32, clip: Rect },
    Text(usize),
}
/// The paint-time view shared by `Text`, `EditableText` and `StyledText`.
struct TextRun<'a> {
    rect: &'a Rect,
    clip: &'a Rect,
    text: &'a str,
    style: Cow<'a, TextStyle>,
    color: &'a Color,
    scroll: (f32, f32),
    wrap: bool,
}
fn text_run(op: &PaintOp) -> Option<TextRun<'_>> {
    match op {
        PaintOp::Quad { .. }
        | PaintOp::Stroke { .. }
        | PaintOp::Shape { .. }
        | PaintOp::ShapeStroke { .. } => None,
        PaintOp::Text {
            rect,
            clip,
            text,
            font_size,
            color,
        } => Some(TextRun {
            rect,
            clip,
            text,
            style: Cow::Owned(TextStyle::new(*font_size)),
            color,
            scroll: (0., 0.),
            wrap: true,
        }),
        PaintOp::EditableText {
            rect,
            clip,
            text,
            font_size,
            color,
            scroll,
            multiline,
        } => Some(TextRun {
            rect,
            clip,
            text,
            style: Cow::Owned(TextStyle::new(*font_size)),
            color,
            scroll: *scroll,
            wrap: *multiline,
        }),
        PaintOp::StyledText {
            rect,
            clip,
            text,
            style,
            color,
            scroll,
            multiline,
            ..
        } => Some(TextRun {
            rect,
            clip,
            text,
            style: Cow::Borrowed(style),
            color,
            scroll: *scroll,
            wrap: *multiline,
        }),
    }
}
pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub text: TextSystem,
    pipeline: wgpu::RenderPipeline,
    atlas: TextAtlas,
    viewport: Viewport,
    cache: SwashCache,
    text_renderers: Vec<TextRenderer>,
    srgb: bool,
}
impl Renderer {
    pub async fn request(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<wgpu::Adapter, RenderError> {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: surface,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::NoAdapter)
    }
    pub async fn new(
        adapter: &wgpu::Adapter,
        format: wgpu::TextureFormat,
    ) -> Result<Self, RenderError> {
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Surface"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Surface rounded rectangles"),
            source: wgpu::ShaderSource::Wgsl(include_str!("quad.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Surface quads"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &ATTRS,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        let atlas = TextAtlas::with_color_mode(
            &device,
            &queue,
            &cache,
            format,
            if format.is_srgb() {
                glyphon::ColorMode::Accurate
            } else {
                glyphon::ColorMode::Web
            },
        );
        Ok(Self {
            device,
            queue,
            text: TextSystem::default(),
            pipeline,
            atlas,
            viewport,
            cache: SwashCache::new(),
            text_renderers: Vec::new(),
            srgb: format.is_srgb(),
        })
    }
    /// Render offscreen in the renderer's own format and read back RGBA8.
    pub fn capture(
        &mut self,
        list: &DisplayList,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<Capture, RenderError> {
        let _ = (list, width, height, scale);
        unimplemented!("red commit: capture")
    }
    /// Paint every command in order. Text never jumps above later rectangles.
    pub fn render(
        &mut self,
        list: &DisplayList,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<(), RenderError> {
        if width == 0 || height == 0 || !scale.is_finite() || scale <= 0. {
            return Err(RenderError::Dimensions);
        }
        self.viewport
            .update(&self.queue, Resolution { width, height });
        let mut vertices = Vec::new();
        let mut draws = Vec::new();
        let mut buffers = Vec::new();
        for op in &list.ops {
            match op {
                PaintOp::Quad { .. }
                | PaintOp::Stroke { .. }
                | PaintOp::Shape { .. }
                | PaintOp::ShapeStroke { .. } => {
                    let (rect, clip, radii, stroke, color) = match op {
                        PaintOp::Quad {
                            rect,
                            clip,
                            radius,
                            color,
                        } => (rect, clip, [*radius; 4], 0., color),
                        PaintOp::Stroke {
                            rect,
                            clip,
                            radius,
                            width,
                            color,
                        } => (rect, clip, [*radius; 4], *width, color),
                        PaintOp::Shape {
                            rect,
                            clip,
                            corners,
                            color,
                        } => (rect, clip, corner_array(corners), 0., color),
                        PaintOp::ShapeStroke {
                            rect,
                            clip,
                            corners,
                            width,
                            color,
                        } => (rect, clip, corner_array(corners), *width, color),
                        _ => unreachable!("only rectangle ops reach this arm"),
                    };
                    // The shader clamps again. Clamping here keeps fully rounded
                    // radii (f32::MAX) out of vertex interpolation.
                    let limit = rect.width.min(rect.height) * 0.5;
                    let radius = radii.map(|r| r.min(limit));
                    let start = vertices.len() as u32;
                    for (u, v) in [(0., 0.), (1., 0.), (0., 1.), (0., 1.), (1., 0.), (1., 1.)] {
                        let x = (rect.x + u * rect.width) * scale;
                        let y = (rect.y + v * rect.height) * scale;
                        let channel = |byte: u8| {
                            let c = byte as f32 / 255.;
                            if !self.srgb {
                                c
                            } else if c <= 0.04045 {
                                c / 12.92
                            } else {
                                ((c + 0.055) / 1.055).powf(2.4)
                            }
                        };
                        vertices.push(Vertex {
                            position: [x / width as f32 * 2. - 1., 1. - y / height as f32 * 2.],
                            local: [u * rect.width, v * rect.height],
                            size: [rect.width, rect.height],
                            color: [
                                channel(color.0),
                                channel(color.1),
                                channel(color.2),
                                color.3 as f32 / 255.,
                            ],
                            radius,
                            stroke,
                        });
                    }
                    draws.push(Draw::Quad { start, clip: *clip });
                }
                PaintOp::Text { .. }
                | PaintOp::EditableText { .. }
                | PaintOp::StyledText { .. } => {
                    let Some(TextRun {
                        rect,
                        clip,
                        text,
                        style,
                        color,
                        scroll,
                        wrap,
                    }) = text_run(op)
                    else {
                        continue;
                    };
                    let index = buffers.len();
                    buffers.push(self.text.buffer(
                        text,
                        &style,
                        if wrap { Some(rect.width) } else { None },
                    ));
                    while self.text_renderers.len() <= index {
                        self.text_renderers.push(TextRenderer::new(
                            &mut self.atlas,
                            &self.device,
                            Default::default(),
                            None,
                        ));
                    }
                    self.text_renderers[index].prepare(
                        &self.device,
                        &self.queue,
                        &mut self.text.fonts,
                        &mut self.atlas,
                        &self.viewport,
                        [TextArea {
                            buffer: &buffers[index],
                            left: (rect.x - scroll.0) * scale,
                            top: (rect.y - scroll.1) * scale,
                            scale,
                            bounds: TextBounds {
                                left: (clip.x * scale).floor() as i32,
                                top: (clip.y * scale).floor() as i32,
                                right: ((clip.x + clip.width) * scale).ceil() as i32,
                                bottom: ((clip.y + clip.height) * scale).ceil() as i32,
                            },
                            default_color: glyphon::Color::rgba(color.0, color.1, color.2, color.3),
                            custom_glyphs: &[],
                        }],
                        &mut self.cache,
                    )?;
                    draws.push(Draw::Text(index));
                }
            }
        }
        // wgpu disallows an empty buffer, but a list containing only text is valid.
        let empty_vertex = Vertex::zeroed();
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Surface frame vertices"),
                contents: if vertices.is_empty() {
                    bytemuck::bytes_of(&empty_vertex)
                } else {
                    bytemuck::cast_slice(&vertices)
                },
                usage: wgpu::BufferUsages::VERTEX,
            });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Surface frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Surface paint"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            for draw in draws {
                match draw {
                    Draw::Quad { start, clip } => {
                        let left = (clip.x * scale).floor().max(0.).min(width as f32) as u32;
                        let top = (clip.y * scale).floor().max(0.).min(height as f32) as u32;
                        let right = ((clip.x + clip.width) * scale)
                            .ceil()
                            .max(left as f32)
                            .min(width as f32) as u32;
                        let bottom = ((clip.y + clip.height) * scale)
                            .ceil()
                            .max(top as f32)
                            .min(height as f32) as u32;
                        if right <= left || bottom <= top {
                            continue;
                        }
                        pass.set_scissor_rect(left, top, right - left, bottom - top);
                        pass.set_pipeline(&self.pipeline);
                        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        pass.draw(start..start + 6, 0..1);
                    }
                    Draw::Text(index) => {
                        pass.set_scissor_rect(0, 0, width, height);
                        self.text_renderers[index].render(
                            &self.atlas,
                            &self.viewport,
                            &mut pass,
                        )?;
                    }
                }
            }
        }
        self.queue.submit(Some(encoder.finish()));
        self.atlas.trim();
        Ok(())
    }
}
use bytemuck::Zeroable;
