use std::collections::VecDeque;

use surface::{RasterImage, RasterPaint};
use wgpu::util::DeviceExt;

use super::{corner_array, RasterSpace};

const CACHE_ENTRIES: usize = 64;
const CACHE_BYTES: usize = 8 * 1024 * 1024;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    destination: [f32; 4],
    size: [f32; 2],
    corners: [f32; 4],
    source_uv: [f32; 4],
}

const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4];

impl Instance {
    fn new(paint: &RasterPaint, space: RasterSpace) -> Option<Self> {
        let rect = paint.rect();
        let visible = rect.intersect(paint.clip());
        if visible.width <= 0. || visible.height <= 0. {
            return None;
        }
        space.scissor(visible)?;
        let x = |value| value * space.scale / space.width as f32 * 2. - 1.;
        let y = |value| 1. - value * space.scale / space.height as f32 * 2.;
        let destination = [
            x(rect.x),
            y(rect.y),
            x(rect.x + rect.width),
            y(rect.y + rect.height),
        ];
        destination
            .iter()
            .all(|value| value.is_finite())
            .then(|| Self {
                destination,
                size: [rect.width, rect.height],
                corners: corner_array(&paint.corners()),
                source_uv: paint.source_uv(),
            })
    }
}

#[derive(Default)]
pub(super) struct Frame {
    instances: Vec<Instance>,
    textures: Vec<wgpu::BindGroup>,
    oversized: Vec<CachedTexture>,
}

impl Frame {
    pub(super) fn push(
        &mut self,
        paint: &RasterPaint,
        space: RasterSpace,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<u32> {
        let instance = Instance::new(paint, space)?;
        let index = self.instances.len() as u32;
        let texture = self.binding(pipeline, device, queue, paint.image());
        self.instances.push(instance);
        self.textures.push(texture);
        Some(index)
    }

    fn binding(
        &mut self,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &RasterImage,
    ) -> wgpu::BindGroup {
        if image.rgba8().len() <= CACHE_BYTES {
            return pipeline.binding(device, queue, image);
        }
        if let Some(entry) = self
            .oversized
            .iter()
            .find(|entry| entry.image.shares_pixels(image))
        {
            return entry.binding.clone();
        }
        // Share larger assets during one frame without admitting them to the LRU.
        let binding = pipeline.upload(device, queue, image);
        self.oversized.push(CachedTexture {
            image: image.clone(),
            binding: binding.clone(),
        });
        binding
    }

    pub(super) fn upload(self, device: &wgpu::Device) -> Option<FrameBuffers> {
        (!self.instances.is_empty()).then(|| FrameBuffers {
            instances: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Surface raster instances"),
                contents: bytemuck::cast_slice(&self.instances),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            textures: self.textures,
        })
    }
}

pub(super) struct FrameBuffers {
    instances: wgpu::Buffer,
    // Keep bindings alive even if this frame evicts a texture from the cache.
    textures: Vec<wgpu::BindGroup>,
}

impl FrameBuffers {
    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, pipeline: &Pipeline, index: u32) {
        pass.set_pipeline(&pipeline.render);
        pass.set_bind_group(0, &self.textures[index as usize], &[]);
        pass.set_vertex_buffer(0, self.instances.slice(..));
        pass.draw(0..6, index..index + 1);
    }
}

struct CachedTexture {
    image: RasterImage,
    binding: wgpu::BindGroup,
}

pub(super) struct Pipeline {
    render: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
    source_format: wgpu::TextureFormat,
    cache: VecDeque<CachedTexture>,
    cached_bytes: usize,
}

impl Pipeline {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Surface raster images"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raster.wgsl").into()),
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Surface raster texture"),
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Surface raster images"),
            bind_group_layouts: &[&texture_layout],
            push_constant_ranges: &[],
        });
        let render = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Surface raster images"),
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
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
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
            texture_layout,
            source_format: if format.is_srgb() {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            },
            cache: VecDeque::new(),
            cached_bytes: 0,
        }
    }

    fn binding(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &RasterImage,
    ) -> wgpu::BindGroup {
        if let Some(index) = self
            .cache
            .iter()
            .position(|entry| entry.image.shares_pixels(image))
        {
            let entry = self.cache.remove(index).expect("cache index exists");
            let binding = entry.binding.clone();
            self.cache.push_back(entry);
            return binding;
        }

        let binding = self.upload(device, queue, image);
        let bytes = image.rgba8().len();
        debug_assert!(bytes <= CACHE_BYTES);
        while self.cache.len() >= CACHE_ENTRIES || self.cached_bytes + bytes > CACHE_BYTES {
            let entry = self.cache.pop_front().expect("cache exceeds limits");
            self.cached_bytes -= entry.image.rgba8().len();
        }
        self.cached_bytes += bytes;
        self.cache.push_back(CachedTexture {
            image: image.clone(),
            binding: binding.clone(),
        });
        binding
    }

    fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &RasterImage,
    ) -> wgpu::BindGroup {
        let size = wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Surface raster image"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.source_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            image.rgba8(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.width * 4),
                rows_per_image: Some(size.height),
            },
            size,
        );
        let view = texture.create_view(&Default::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Surface raster texture"),
            layout: &self.texture_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        })
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::Renderer;

    #[test]
    #[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
    fn gpu_offscreen_raster_cache_reuses_shared_pixels_and_evicts_by_count_and_bytes() {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::PRIMARY,
                ..Default::default()
            });
            let adapter = Renderer::request(&instance, None)
                .await
                .expect("native GPU adapter is required");
            let renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
                .await
                .unwrap();
            eprintln!("GPU evidence: {:?}", adapter.get_info());
            let mut pipeline = Pipeline::new(&renderer.device, wgpu::TextureFormat::Rgba8Unorm);
            let bind = |pipeline: &mut Pipeline, image: &RasterImage| {
                pipeline.binding(&renderer.device, &renderer.queue, image)
            };

            let first = RasterImage::new(1, 1, vec![20, 80, 120, 255]).unwrap();
            let first_binding = bind(&mut pipeline, &first);
            assert_eq!(first_binding, bind(&mut pipeline, &first.clone()));
            for value in 0..CACHE_ENTRIES as u8 {
                let image = RasterImage::new(1, 1, vec![value, 0, 0, 255]).unwrap();
                bind(&mut pipeline, &image);
            }
            assert_eq!(pipeline.cache.len(), CACHE_ENTRIES);
            assert_eq!(pipeline.cached_bytes, CACHE_ENTRIES * 4);
            assert_ne!(first_binding, bind(&mut pipeline, &first));

            let a = RasterImage::new(1024, 1024, vec![0; CACHE_BYTES / 2]).unwrap();
            let b = RasterImage::new(1024, 1024, vec![255; CACHE_BYTES / 2]).unwrap();
            let a_binding = bind(&mut pipeline, &a);
            let b_binding = bind(&mut pipeline, &b);
            assert_eq!(pipeline.cache.len(), 2);
            assert_eq!(pipeline.cached_bytes, CACHE_BYTES);
            assert_eq!(a_binding, bind(&mut pipeline, &a));
            bind(&mut pipeline, &first);
            // Accessing A made B the least recently used texture.
            assert_eq!(a_binding, bind(&mut pipeline, &a));
            assert_ne!(b_binding, bind(&mut pipeline, &b));

            let oversized = RasterImage::new(2049, 1024, vec![0; 2049 * 1024 * 4]).unwrap();
            let mut frame = Frame::default();
            let frame_binding =
                |frame: &mut Frame, pipeline: &mut Pipeline, asset: &RasterImage| {
                    frame.binding(pipeline, &renderer.device, &renderer.queue, asset)
                };
            let binding = frame_binding(&mut frame, &mut pipeline, &oversized);
            assert_eq!(
                binding,
                frame_binding(&mut frame, &mut pipeline, &oversized.clone())
            );
            assert_eq!(frame.oversized.len(), 1);
            assert_ne!(
                binding,
                frame_binding(&mut Frame::default(), &mut pipeline, &oversized)
            );
            assert_eq!(pipeline.cache.len(), 2);
            assert_eq!(pipeline.cached_bytes, CACHE_BYTES);
            eprintln!("Raster cache: 64 bindings and 8 MiB cap, oversized binding reused within one frame");
            renderer.queue.submit([]);
            renderer.device.poll(wgpu::Maintain::Wait);
        });
    }
}
