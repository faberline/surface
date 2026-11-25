//! One ordered wgpu renderer for native GPU surfaces and browser WebGPU.
mod opacity;
mod path;
mod raster;
mod text;
use glyphon::{
    Cache, Resolution, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use std::borrow::Cow;
use surface::{
    Color, Corners, DisplayList, ExtendedDisplayList, ExtendedPaintOp, PaintOp, PathFill,
    RasterPaint, Rect, TextAlignment, TextSelectionPaint, TextStyle,
};
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
/// Error returned by the renderer's extended display-list entry points.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExtendedRenderError {
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error("this renderer does not support an operation in the extended display list")]
    UnsupportedOperation,
    #[error("invalid opacity group: {0}")]
    InvalidOpacityGroup(&'static str),
    #[error("isolated paint groups are not supported on target format {0:?}")]
    OpacityTargetFormat(wgpu::TextureFormat),
    #[error("invalid paint transform: {0}")]
    InvalidPaintTransform(&'static str),
    #[error("raster image of {width}x{height} exceeds the device's {max} pixel side limit")]
    RasterTooLarge { width: u32, height: u32, max: u32 },
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
        if width == 0 || height == 0 {
            return Err(RenderError::Dimensions);
        }
        if width > MAX_CAPTURE_SIDE || height > MAX_CAPTURE_SIDE {
            return Err(RenderError::CaptureTooLarge {
                width,
                height,
                max: MAX_CAPTURE_SIDE,
            });
        }
        Ok(())
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
const QUAD_UV: [(f32, f32); 6] = [(0., 0.), (1., 0.), (0., 1.), (0., 1.), (1., 0.), (1., 1.)];

fn corner_array(c: &Corners) -> [f32; 4] {
    [c.top_left, c.top_right, c.bottom_right, c.bottom_left]
}
fn color_channels(color: Color, srgb: bool) -> [f32; 4] {
    let channel = |byte: u8| {
        let c = byte as f32 / 255.;
        if !srgb {
            c
        } else if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [
        channel(color.0),
        channel(color.1),
        channel(color.2),
        color.3 as f32 / 255.,
    ]
}

#[derive(Clone, Copy)]
struct RasterSpace {
    width: u32,
    height: u32,
    scale: f32,
}

impl RasterSpace {
    fn scissor(self, clip: Rect) -> Option<[u32; 4]> {
        let left = (clip.x * self.scale).floor().max(0.).min(self.width as f32) as u32;
        let top = (clip.y * self.scale)
            .floor()
            .max(0.)
            .min(self.height as f32) as u32;
        let right = ((clip.x + clip.width) * self.scale)
            .ceil()
            .max(left as f32)
            .min(self.width as f32) as u32;
        let bottom = ((clip.y + clip.height) * self.scale)
            .ceil()
            .max(top as f32)
            .min(self.height as f32) as u32;
        (right > left && bottom > top).then(|| [left, top, right - left, bottom - top])
    }

    fn text_bounds(self, [left, top, width, height]: [u32; 4], origin_y: f32) -> TextBounds {
        let clip_top = top as f32;
        let clip_bottom = (top + height) as f32;
        // Glyphon compares a physical text origin plus unscaled line offsets
        // to these bounds before clipping its physical glyphs. Expand the
        // bounds in both spaces so visible runs survive that first check.
        let run_edge = |edge| origin_y + (edge - origin_y) / self.scale;
        TextBounds {
            left: left as i32,
            right: (left + width) as i32,
            top: clip_top.min(run_edge(clip_top)).floor() as i32,
            bottom: clip_bottom.max(run_edge(clip_bottom)).ceil() as i32,
        }
    }
}

enum RenderOp<'a> {
    Paint(&'a PaintOp, Option<TextAlignment>),
    BeginGroup(opacity::Effect),
    EndOpacityGroup,
    Fill(&'a PathFill),
    Raster(&'a RasterPaint),
    TextSelection(&'a TextSelectionPaint),
}

#[derive(Clone, Copy)]
enum Draw {
    Quad { start: u32, scissor: [u32; 4] },
    Text { index: usize, scissor: [u32; 4] },
    Fill { index: u32, clip: Rect },
    Raster { index: u32, clip: Rect },
}
/// A solid quad covering the render target. Everything samples the
/// interior of the quad shader, so the integer scissors own the edges
/// without introducing antialiasing at a boundary between two regions.
fn solid_selection_quad(vertices: &mut Vec<Vertex>, color: Color, srgb: bool) -> u32 {
    let start = vertices.len() as u32;
    let color = color_channels(color, srgb);
    vertices.extend(QUAD_UV.map(|(u, v)| Vertex {
        position: [u * 2. - 1., 1. - v * 2.],
        local: [0.5; 2],
        size: [1.; 2],
        color,
        radius: [0.; 4],
        stroke: 0.,
    }));
    start
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
    path_pipeline: Option<path::Pipeline>,
    raster_pipeline: Option<raster::Pipeline>,
    opacity_compositor: Option<opacity::Compositor>,
    atlas: TextAtlas,
    viewport: Viewport,
    cache: SwashCache,
    text_renderers: Vec<TextRenderer>,
    srgb: bool,
    format: wgpu::TextureFormat,
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
            path_pipeline: None,
            raster_pipeline: None,
            opacity_compositor: None,
            atlas,
            viewport,
            cache: SwashCache::new(),
            text_renderers: Vec::new(),
            srgb: format.is_srgb(),
            format,
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
        self.capture_with(width, height, |renderer, target| {
            renderer.render(list, target, width, height, scale)
        })
    }

    /// Render an extended frame offscreen in the renderer's own format and
    /// read back RGBA8.
    pub fn capture_extended(
        &mut self,
        list: &ExtendedDisplayList,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<Capture, ExtendedRenderError> {
        self.capture_with(width, height, |renderer, target| {
            renderer.render_extended(list, target, width, height, scale)
        })
    }

    fn capture_with<E: From<RenderError>>(
        &mut self,
        width: u32,
        height: u32,
        draw: impl FnOnce(&mut Self, &wgpu::TextureView) -> Result<(), E>,
    ) -> Result<Capture, E> {
        Capture::check_size(width, height)?;
        let limit = self.device.limits().max_texture_dimension_2d;
        if width > limit || height > limit {
            return Err(RenderError::CaptureTooLarge {
                width,
                height,
                max: limit,
            }
            .into());
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Surface capture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        draw(self, &texture.create_view(&Default::default()))?;
        let row = width as usize * 4;
        let padded = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Surface capture readback"),
            size: (padded * height as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: Default::default(),
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: Some(height),
                },
            },
            size,
        );
        self.queue.submit(Some(encoder.finish()));
        let (tx, rx) = std::sync::mpsc::channel();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| RenderError::Readback(e.to_string()))?
            .map_err(|e| RenderError::Readback(e.to_string()))?;
        let mapped = buffer.slice(..).get_mapped_range();
        let bgra = matches!(
            self.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let mut rgba = Vec::with_capacity(row * height as usize);
        for line in mapped.chunks_exact(padded).take(height as usize) {
            rgba.extend_from_slice(&line[..row]);
        }
        drop(mapped);
        buffer.unmap();
        if bgra {
            for px in rgba.chunks_exact_mut(4) {
                px.swap(0, 2);
            }
        }
        Ok(Capture {
            width,
            height,
            rgba,
        })
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
        self.render_ops(
            list.ops.iter().map(|op| Ok(RenderOp::Paint(op, None))),
            target,
            width,
            height,
            scale,
        )
    }

    /// Paint ordered commands, including vector fills, rasters and isolated groups.
    pub fn render_extended(
        &mut self,
        list: &ExtendedDisplayList,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<(), ExtendedRenderError> {
        opacity::validate(list.ops())?;
        let format = self.format;
        let supports_isolation = opacity::supports_format(format);
        let space = RasterSpace {
            width,
            height,
            scale,
        };
        let begin = |effect: opacity::Effect| {
            if effect.needs_surface() && !supports_isolation {
                return Err(ExtendedRenderError::OpacityTargetFormat(format));
            }
            Ok(RenderOp::BeginGroup(effect))
        };
        let max = self.device.limits().max_texture_dimension_2d;
        self.render_ops(
            list.ops().iter().map(|op| match op {
                ExtendedPaintOp::Legacy(op) => Ok(RenderOp::Paint(op, None)),
                ExtendedPaintOp::AlignedText(text)
                    if matches!(
                        text.run(),
                        PaintOp::Text { .. }
                            | PaintOp::StyledText {
                                multiline: true,
                                ..
                            }
                    ) =>
                {
                    Ok(RenderOp::Paint(text.run(), Some(text.alignment())))
                }
                ExtendedPaintOp::FillPath(fill) => Ok(RenderOp::Fill(fill)),
                ExtendedPaintOp::BeginOpacityGroup { opacity } => {
                    begin(opacity::Effect::Opacity(*opacity))
                }
                ExtendedPaintOp::BeginTransformGroup { transform } => {
                    begin(opacity::Effect::transform(*transform, space)?)
                }
                ExtendedPaintOp::EndOpacityGroup => Ok(RenderOp::EndOpacityGroup),
                ExtendedPaintOp::Raster(paint) => {
                    let width = paint.image().width();
                    let height = paint.image().height();
                    if width > max || height > max {
                        Err(ExtendedRenderError::RasterTooLarge { width, height, max })
                    } else {
                        Ok(RenderOp::Raster(paint))
                    }
                }
                ExtendedPaintOp::TextSelection(selection)
                    if matches!(
                        selection.run(),
                        PaintOp::EditableText { .. } | PaintOp::StyledText { editable: true, .. }
                    ) && (selection.alignment().is_none()
                        || matches!(
                            selection.run(),
                            PaintOp::StyledText {
                                multiline: true,
                                ..
                            }
                        )) =>
                {
                    Ok(RenderOp::TextSelection(selection))
                }
                _ => Err(ExtendedRenderError::UnsupportedOperation),
            }),
            target,
            width,
            height,
            scale,
        )
    }

    fn render_ops<'a, E: From<RenderError>>(
        &mut self,
        ops: impl IntoIterator<Item = Result<RenderOp<'a>, E>>,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<(), E> {
        if width == 0 || height == 0 || !scale.is_finite() || scale <= 0. {
            return Err(RenderError::Dimensions.into());
        }
        self.viewport
            .update(&self.queue, Resolution { width, height });
        let mut vertices = Vec::new();
        let mut draws = Vec::new();
        let space = RasterSpace {
            width,
            height,
            scale,
        };
        let mut paths = path::Frame::default();
        let mut rasters = raster::Frame::default();
        let mut next_text_renderer = 0;
        let mut active_group = opacity::ActiveGroup::Direct;
        let mut groups = Vec::new();
        for op in ops {
            let (op, alignment) = match op? {
                RenderOp::BeginGroup(effect) => {
                    active_group = opacity::ActiveGroup::begin(effect, draws.len());
                    continue;
                }
                RenderOp::EndOpacityGroup => {
                    active_group.end(draws.len(), &mut groups);
                    continue;
                }
                _ if matches!(active_group, opacity::ActiveGroup::Hidden) => continue,
                RenderOp::Paint(op, alignment) => (op, alignment),
                RenderOp::TextSelection(selection) => {
                    let regions = selection.pixel_regions(scale, (width, height));
                    if regions.is_empty() {
                        continue;
                    }
                    let colors = selection.colors();
                    if colors.background.3 != 0 && regions.iter().any(|region| region.selected) {
                        let start =
                            solid_selection_quad(&mut vertices, colors.background, self.srgb);
                        draws.extend(regions.iter().filter(|region| region.selected).map(
                            |region| Draw::Quad {
                                start,
                                scissor: region.scissor,
                            },
                        ));
                    }
                    let run = text_run(selection.run()).expect("validated editable text run");
                    let ordinary = *run.color;
                    self.prepare_text_run(
                        space,
                        &run,
                        selection.alignment(),
                        regions.into_iter().map(|region| {
                            let ink = if region.selected {
                                colors.foreground
                            } else {
                                ordinary
                            };
                            (region.scissor, ink)
                        }),
                        &mut next_text_renderer,
                        &mut draws,
                    )?;
                    continue;
                }
                RenderOp::Fill(fill) => {
                    if let Some(index) = paths.push(fill, space, self.srgb) {
                        draws.push(Draw::Fill {
                            index,
                            clip: fill.clip(),
                        });
                    }
                    continue;
                }
                RenderOp::Raster(paint) => {
                    let pipeline = self
                        .raster_pipeline
                        .get_or_insert_with(|| raster::Pipeline::new(&self.device, self.format));
                    if let Some(index) =
                        rasters.push(paint, space, pipeline, &self.device, &self.queue)
                    {
                        draws.push(Draw::Raster {
                            index,
                            clip: paint.clip(),
                        });
                    }
                    continue;
                }
            };
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
                    let Some(scissor) = space.scissor(*clip) else {
                        continue;
                    };
                    // The shader clamps again. Clamping here keeps fully rounded
                    // radii (f32::MAX) out of vertex interpolation.
                    let limit = rect.width.min(rect.height) * 0.5;
                    let radius = radii.map(|r| r.min(limit));
                    let start = vertices.len() as u32;
                    for (u, v) in QUAD_UV {
                        let x = (rect.x + u * rect.width) * scale;
                        let y = (rect.y + v * rect.height) * scale;
                        vertices.push(Vertex {
                            position: [x / width as f32 * 2. - 1., 1. - y / height as f32 * 2.],
                            local: [u * rect.width, v * rect.height],
                            size: [rect.width, rect.height],
                            color: color_channels(*color, self.srgb),
                            radius,
                            stroke,
                        });
                    }
                    draws.push(Draw::Quad { start, scissor });
                }
                PaintOp::Text { .. }
                | PaintOp::EditableText { .. }
                | PaintOp::StyledText { .. } => {
                    let Some(run) = text_run(op) else {
                        continue;
                    };
                    let Some(scissor) = space.scissor(*run.clip) else {
                        continue;
                    };
                    self.prepare_text_run(
                        space,
                        &run,
                        alignment,
                        [(scissor, *run.color)],
                        &mut next_text_renderer,
                        &mut draws,
                    )?;
                }
            }
        }
        // wgpu disallows an empty buffer, but a list containing only text is valid.
        let path_buffers = (!paths.is_empty()).then(|| {
            let pipeline = self
                .path_pipeline
                .get_or_insert_with(|| path::Pipeline::new(&self.device, self.format));
            (paths.upload(&self.device, pipeline), pipeline)
        });
        let raster_buffers = rasters.upload(&self.device);
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
        let compositor = if groups.is_empty() {
            None
        } else {
            let max = self.device.limits().max_texture_dimension_2d;
            if width > max || height > max {
                return Err(RenderError::Dimensions.into());
            }
            Some(
                self.opacity_compositor
                    .get_or_insert_with(|| opacity::Compositor::new(&self.device, self.format))
                    .prepare(&self.device, self.format, width, height, &groups),
            )
        };
        for stage in opacity::passes(&groups, draws.len()) {
            let (view, load, range, composite) = match stage {
                opacity::Pass::Target { draws, composite } => (
                    target,
                    if composite.is_some() {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                    },
                    draws,
                    composite,
                ),
                opacity::Pass::Intermediate { draws } => (
                    compositor.as_ref().expect("group surface prepared").view(),
                    wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    draws,
                    None,
                ),
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Surface paint"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some(index) = composite {
                compositor
                    .as_ref()
                    .expect("group surface prepared")
                    .composite(&mut pass, index, &groups[index]);
            }
            for &draw in &draws[range] {
                match draw {
                    Draw::Quad {
                        start,
                        scissor: [left, top, width, height],
                    } => {
                        pass.set_scissor_rect(left, top, width, height);
                        pass.set_pipeline(&self.pipeline);
                        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        pass.draw(start..start + 6, 0..1);
                    }
                    Draw::Fill { index, clip } => {
                        let Some([left, top, width, height]) = space.scissor(clip) else {
                            continue;
                        };
                        if let Some((buffers, pipeline)) = &path_buffers {
                            pass.set_scissor_rect(left, top, width, height);
                            buffers.draw(&mut pass, pipeline, index);
                        }
                    }
                    Draw::Raster { index, clip } => {
                        let Some([left, top, width, height]) = space.scissor(clip) else {
                            continue;
                        };
                        if let (Some(buffers), Some(pipeline)) =
                            (&raster_buffers, &self.raster_pipeline)
                        {
                            pass.set_scissor_rect(left, top, width, height);
                            buffers.draw(&mut pass, pipeline, index);
                        }
                    }
                    Draw::Text {
                        index,
                        scissor: [left, top, width, height],
                    } => {
                        // Clip to the true physical bounds after Glyphon's
                        // expanded run and glyph selection.
                        pass.set_scissor_rect(left, top, width, height);
                        self.text_renderers[index]
                            .render(&self.atlas, &self.viewport, &mut pass)
                            .map_err(RenderError::from)?;
                    }
                }
            }
        }
        self.queue.submit(Some(encoder.finish()));
        self.atlas.trim();
        Ok(())
    }

    fn prepare_text_run(
        &mut self,
        space: RasterSpace,
        run: &TextRun<'_>,
        alignment: Option<TextAlignment>,
        regions: impl IntoIterator<Item = ([u32; 4], Color)>,
        next_renderer: &mut usize,
        draws: &mut Vec<Draw>,
    ) -> Result<(), RenderError> {
        // Shape the complete run before splitting ink. Glyphon keeps its
        // prepared vertices, so every region can borrow the same buffer.
        let buffer = self.text.buffer_with_alignment(
            run.text,
            &run.style,
            run.wrap.then_some(run.rect.width),
            alignment,
        );
        let left = (run.rect.x - run.scroll.0) * space.scale;
        let top = (run.rect.y - run.scroll.1) * space.scale;
        for (scissor, color) in regions {
            let index = *next_renderer;
            *next_renderer += 1;
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
                    buffer: &buffer,
                    left,
                    top,
                    scale: space.scale,
                    bounds: space.text_bounds(scissor, top),
                    default_color: glyphon::Color::rgba(color.0, color.1, color.2, color.3),
                    custom_glyphs: &[],
                }],
                &mut self.cache,
            )?;
            draws.push(Draw::Text { index, scissor });
        }
        Ok(())
    }
}
use bytemuck::Zeroable;
