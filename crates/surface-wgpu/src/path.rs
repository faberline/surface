use std::num::NonZeroU64;

use bytemuck::Zeroable;
use surface::{FillContour, PathFill};
use wgpu::util::DeviceExt;

// Each 2 KiB dynamic binding fits downlevel uniform-buffer limits and needs
// no fragment storage feature. The offset advances by a whole point block.
const POINT_PAIRS: usize = FillContour::MAX_POINTS / 2;
const PATHS_PER_BATCH: usize = 256;
const POINT_BYTES: u64 = std::mem::size_of::<PointBlock>() as u64;
const _: () = {
    assert!(POINT_PAIRS == 128); // Keep the WGSL uniform array in sync.
    assert!(POINT_BYTES.is_multiple_of(
        wgpu::Limits::downlevel_defaults().min_uniform_buffer_offset_alignment as u64
    ));
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PointBlock {
    // Uniform arrays use a 16-byte stride. Pack two points per slot.
    pairs: [[f32; 4]; POINT_PAIRS],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    clip_rect: [f32; 4],
    color: [f32; 4],
    point_count: u32,
}

const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Uint32];

#[derive(Default)]
pub(super) struct Frame {
    instances: Vec<Instance>,
    points: Vec<PointBlock>,
}

impl Frame {
    pub(super) fn push(
        &mut self,
        path: &PathFill,
        space: super::RasterSpace,
        srgb: bool,
    ) -> Option<u32> {
        let (rect, color, positions) = (path.rect(), path.color(), path.points());
        if color.3 == 0 {
            return None;
        }
        let [x, y, w, h] = space.scissor(path.clip())?;
        let to_pixel = |value: f32| f64::from(value) * f64::from(space.scale);
        // Include the half-pixel antialias fringe and trim the bounding quad
        // to the existing device-pixel scissor before shading.
        let left = (to_pixel(rect.x) - 0.5).max(f64::from(x));
        let top = (to_pixel(rect.y) - 0.5).max(f64::from(y));
        let right = (to_pixel(rect.x) + to_pixel(rect.width) + 0.5).min(f64::from(x + w));
        let bottom = (to_pixel(rect.y) + to_pixel(rect.height) + 0.5).min(f64::from(y + h));
        if right <= left || bottom <= top {
            return None;
        }

        // Points and the fragment position share device-pixel coordinates.
        let mut points = PointBlock::zeroed();
        for (i, &[x, y]) in positions.iter().enumerate() {
            let x = to_pixel(x) as f32;
            let y = to_pixel(y) as f32;
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let at = 2 * (i % 2);
            points.pairs[i / 2][at..at + 2].copy_from_slice(&[x, y]);
        }
        let index = self.instances.len() as u32;
        let ndc_x = |x| (x / f64::from(space.width) * 2. - 1.) as f32;
        let ndc_y = |y| (1. - y / f64::from(space.height) * 2.) as f32;
        self.instances.push(Instance {
            clip_rect: [ndc_x(left), ndc_y(top), ndc_x(right), ndc_y(bottom)],
            color: super::color_channels(color, srgb),
            point_count: positions.len() as u32,
        });
        self.points.push(points);
        Some(index)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub(super) fn upload(&self, device: &wgpu::Device, pipeline: &Pipeline) -> FrameBuffers {
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Surface path instances"),
            contents: bytemuck::cast_slice(&self.instances),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let points = self
            .points
            .chunks(PATHS_PER_BATCH)
            .map(|chunk| {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Surface path point batch"),
                    contents: bytemuck::cast_slice(chunk),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Surface path point batch"),
                    layout: &pipeline.points_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &buffer,
                            offset: 0,
                            size: NonZeroU64::new(POINT_BYTES),
                        }),
                    }],
                })
            })
            .collect();
        FrameBuffers { instances, points }
    }
}

pub(super) struct FrameBuffers {
    instances: wgpu::Buffer,
    points: Vec<wgpu::BindGroup>,
}

impl FrameBuffers {
    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, pipeline: &Pipeline, index: u32) {
        let i = index as usize;
        let batch = &self.points[i / PATHS_PER_BATCH];
        let offset = (i % PATHS_PER_BATCH) as u32 * POINT_BYTES as u32;
        pass.set_pipeline(&pipeline.render);
        pass.set_bind_group(0, batch, &[offset]);
        pass.set_vertex_buffer(0, self.instances.slice(..));
        pass.draw(0..6, index..index + 1);
    }
}

pub(super) struct Pipeline {
    render: wgpu::RenderPipeline,
    points_layout: wgpu::BindGroupLayout,
}

impl Pipeline {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Surface paths"),
            source: wgpu::ShaderSource::Wgsl(include_str!("path.wgsl").into()),
        });
        let points_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Surface path points"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(POINT_BYTES),
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Surface paths"),
            bind_group_layouts: &[&points_layout],
            push_constant_ranges: &[],
        });
        let render = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Surface paths"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &ATTRIBUTES,
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
        Self {
            render,
            points_layout,
        }
    }
}
