use std::ops::Range;

use surface::{ExtendedPaintOp, PaintTransform, Rect};
use wgpu::util::DeviceExt;

use crate::{ExtendedRenderError, RasterSpace};

// The intermediate is in the same space and format as the target. It needs
// enough alpha precision to store the coverage from the ordinary draw lanes.
pub(super) fn supports_format(format: wgpu::TextureFormat) -> bool {
    use wgpu::TextureFormat::*;
    matches!(
        format,
        Rgba8Unorm | Rgba8UnormSrgb | Bgra8Unorm | Bgra8UnormSrgb | Rgba16Float
    )
}

pub(super) fn validate(ops: &[ExtendedPaintOp]) -> Result<(), ExtendedRenderError> {
    let mut open = false;
    for op in ops {
        match op {
            ExtendedPaintOp::BeginOpacityGroup { .. }
            | ExtendedPaintOp::BeginTransformGroup { .. } => {
                if open {
                    return Err(ExtendedRenderError::InvalidOpacityGroup(
                        "groups cannot nest",
                    ));
                }
                match op {
                    ExtendedPaintOp::BeginOpacityGroup { opacity }
                        if !(0. ..=1.).contains(opacity) =>
                    {
                        return Err(ExtendedRenderError::InvalidOpacityGroup(
                            "opacity must be finite and between zero and one",
                        ));
                    }
                    ExtendedPaintOp::BeginTransformGroup { transform } if !transform.is_valid() => {
                        return Err(ExtendedRenderError::InvalidPaintTransform(
                            "rectangles must have finite edges and positive area, radius must be finite and nonnegative, and opacity must be in 0..=1",
                        ));
                    }
                    _ => {}
                }
                open = true;
            }
            ExtendedPaintOp::EndOpacityGroup => {
                if !open {
                    return Err(ExtendedRenderError::InvalidOpacityGroup(
                        "end without a start",
                    ));
                }
                open = false;
            }
            _ => {}
        }
    }
    if open {
        return Err(ExtendedRenderError::InvalidOpacityGroup(
            "start without an end",
        ));
    }
    Ok(())
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    source: [f32; 4],
    destination: [f32; 4],
    clip: [f32; 4],
    radius: f32,
    opacity: f32,
}

const OPACITY_ATTRIBUTES: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32,
    offset: std::mem::offset_of!(Instance, opacity) as u64,
    shader_location: 0,
}];
const TRANSFORM_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x2];

#[derive(Clone, Copy)]
pub(super) struct Transform {
    instance: Instance,
    scissor: [u32; 4],
}

#[derive(Clone, Copy)]
pub(super) enum Effect {
    Opacity(f32),
    Transform(Transform),
}

impl Effect {
    pub(super) fn transform(
        transform: PaintTransform,
        space: RasterSpace,
    ) -> Result<Self, ExtendedRenderError> {
        let scale = |rect: Rect| {
            Rect::new(
                rect.x * space.scale,
                rect.y * space.scale,
                rect.width * space.scale,
                rect.height * space.scale,
            )
        };
        let radius = transform
            .radius
            .min(transform.clip.width.min(transform.clip.height) * 0.5);
        let device = PaintTransform {
            source: scale(transform.source),
            destination: scale(transform.destination),
            clip: scale(transform.clip),
            radius: radius * space.scale,
            opacity: transform.opacity,
        };
        if !device.is_valid() {
            return Err(ExtendedRenderError::InvalidPaintTransform(
                "scaled geometry cannot be represented in device pixels",
            ));
        }
        if device.opacity == 0. {
            return Ok(Self::Opacity(0.));
        }
        let viewport = Rect::new(0., 0., space.width as f32, space.height as f32);
        if device.source == device.destination
            && device.radius == 0.
            && device.source.intersect(viewport) == viewport
            && device.clip.intersect(viewport) == viewport
        {
            return Ok(Self::Opacity(device.opacity));
        }
        let bounds = device
            .destination
            .intersect(device.clip)
            .intersect(viewport);
        if bounds.width == 0. || bounds.height == 0. {
            return Ok(Self::Opacity(0.));
        }
        // Allow the antialias fringe outside the rectangles. The shader
        // evaluates actual fractional bounds and rounded intersections.
        let fringe = Rect::new(
            bounds.x - 0.5,
            bounds.y - 0.5,
            bounds.width + 1.,
            bounds.height + 1.,
        );
        let scissor = RasterSpace { scale: 1., ..space }
            .scissor(fringe)
            .expect("visible bounds and their fringe intersect the device viewport");
        let to_array = |rect: Rect| [rect.x, rect.y, rect.width, rect.height];
        Ok(Self::Transform(Transform {
            instance: Instance {
                source: to_array(device.source),
                destination: to_array(device.destination),
                clip: to_array(device.clip),
                radius: device.radius,
                opacity: device.opacity,
            },
            scissor,
        }))
    }

    pub(super) fn needs_surface(self) -> bool {
        match self {
            Self::Opacity(opacity) => opacity > 0. && opacity < 1.,
            Self::Transform(_) => true,
        }
    }

    fn instance(self) -> Instance {
        match self {
            Self::Transform(transform) => transform.instance,
            Self::Opacity(opacity) => Instance {
                source: [0.; 4],
                destination: [0.; 4],
                clip: [0.; 4],
                radius: 0.,
                opacity,
            },
        }
    }
}

pub(super) enum ActiveGroup {
    Direct,
    Hidden,
    Intermediate { start: usize, effect: Effect },
}

impl ActiveGroup {
    pub(super) fn begin(effect: Effect, start: usize) -> Self {
        match effect {
            Effect::Opacity(0.) => Self::Hidden,
            Effect::Opacity(1.) => Self::Direct,
            _ => Self::Intermediate { start, effect },
        }
    }

    pub(super) fn end(&mut self, end: usize, groups: &mut Vec<DrawGroup>) {
        if let Self::Intermediate { start, effect } = std::mem::replace(self, Self::Direct) {
            if start < end {
                groups.push(DrawGroup {
                    draws: start..end,
                    effect,
                });
            }
        }
    }
}

pub(super) struct DrawGroup {
    draws: Range<usize>,
    effect: Effect,
}

pub(super) enum Pass {
    Target {
        draws: Range<usize>,
        composite: Option<usize>,
    },
    Intermediate {
        draws: Range<usize>,
    },
}

pub(super) fn passes(groups: &[DrawGroup], draws: usize) -> impl Iterator<Item = Pass> + '_ {
    (0..=groups.len()).flat_map(move |index| {
        let previous = index.checked_sub(1);
        let start = previous.map_or(0, |prev| groups[prev].draws.end);
        let next = groups.get(index);
        let target = Pass::Target {
            draws: start..next.map_or(draws, |group| group.draws.start),
            composite: previous,
        };
        std::iter::once(target).chain(next.map(|group| Pass::Intermediate {
            draws: group.draws.clone(),
        }))
    })
}

struct Surface {
    width: u32,
    height: u32,
    view: wgpu::TextureView,
    binding: wgpu::BindGroup,
}

pub(super) struct Compositor {
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    opacity_pipeline: wgpu::RenderPipeline,
    transform_pipeline: Option<wgpu::RenderPipeline>,
    surface: Option<Surface>,
}

impl Compositor {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Surface opacity texture layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Surface opacity pipeline layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let opacity_pipeline = create_pipeline(
            device,
            &pipeline_layout,
            wgpu::include_wgsl!("opacity.wgsl"),
            &OPACITY_ATTRIBUTES,
            format,
        );
        Self {
            layout,
            pipeline_layout,
            opacity_pipeline,
            transform_pipeline: None,
            surface: None,
        }
    }

    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        groups: &[DrawGroup],
    ) -> Prepared<'_> {
        if self
            .surface
            .as_ref()
            .is_none_or(|s| (s.width, s.height) != (width, height))
        {
            let image = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Surface opacity intermediate"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = image.create_view(&wgpu::TextureViewDescriptor::default());
            let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Surface opacity intermediate binding"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                }],
            });
            self.surface = Some(Surface {
                width,
                height,
                view,
                binding,
            });
        }
        let values: Vec<_> = groups.iter().map(|group| group.effect.instance()).collect();
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Surface isolated paint instances"),
            contents: bytemuck::cast_slice(&values),
            usage: wgpu::BufferUsages::VERTEX,
        });
        if groups
            .iter()
            .any(|group| matches!(group.effect, Effect::Transform(_)))
        {
            self.transform_pipeline.get_or_insert_with(|| {
                create_pipeline(
                    device,
                    &self.pipeline_layout,
                    wgpu::include_wgsl!("isolated_transform.wgsl"),
                    &TRANSFORM_ATTRIBUTES,
                    format,
                )
            });
        }
        Prepared {
            surface: self
                .surface
                .as_ref()
                .expect("intermediate surface prepared"),
            opacity_pipeline: &self.opacity_pipeline,
            transform_pipeline: self.transform_pipeline.as_ref(),
            instances,
        }
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    source: wgpu::ShaderModuleDescriptor<'_>,
    attributes: &[wgpu::VertexAttribute],
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let label = source.label;
    let shader = device.create_shader_module(source);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label,
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: size_of::<Instance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes,
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

pub(super) struct Prepared<'a> {
    surface: &'a Surface,
    opacity_pipeline: &'a wgpu::RenderPipeline,
    transform_pipeline: Option<&'a wgpu::RenderPipeline>,
    instances: wgpu::Buffer,
}

impl Prepared<'_> {
    pub(super) fn view(&self) -> &wgpu::TextureView {
        &self.surface.view
    }

    pub(super) fn composite(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        index: usize,
        group: &DrawGroup,
    ) {
        let (pipeline, [x, y, width, height]) = match group.effect {
            Effect::Opacity(_) => (
                self.opacity_pipeline,
                [0, 0, self.surface.width, self.surface.height],
            ),
            Effect::Transform(transform) => (
                self.transform_pipeline
                    .expect("a transform pipeline was prepared"),
                transform.scissor,
            ),
        };
        let start = (index * size_of::<Instance>()) as u64;
        pass.set_scissor_rect(x, y, width, height);
        pass.set_pipeline(pipeline);
        pass.set_vertex_buffer(
            0,
            self.instances
                .slice(start..start + size_of::<Instance>() as u64),
        );
        pass.set_bind_group(0, &self.surface.binding, &[]);
        pass.draw(0..3, 0..1);
    }
}
