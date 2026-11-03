use surface::{Color, DisplayList, PaintOp, Rect, TextMeasurer};
use surface_wgpu::{Renderer, TextSystem};

#[test]
fn bundled_font_measures_repeated_text_and_wraps_at_the_paint_width() {
    let mut text = TextSystem::default();
    let wide = text.measure("Surface Rust application", 16., Some(600.));
    assert!(wide.0 > 100. && wide.1 > 0.);
    assert_eq!(
        wide,
        text.measure("Surface Rust application", 16., Some(600.))
    );
    let narrow = text.measure("Surface Rust application", 16., Some(70.));
    assert!(narrow.0 <= 70.1);
    assert!(narrow.1 > wide.1);
    // Taffy snaps node bounds to logical pixels. A measured label must still
    // fit on its original line after that snap and paint-time shaping.
    let label = text.measure("Dark theme", 16., None);
    let painted = text.measure("Dark theme", 16., Some(label.0.round()));
    assert_eq!(
        label.1, painted.1,
        "pixel snapping must not hide the second word"
    );
}

#[test]
#[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
fn gpu_offscreen_paints_clips_rounding_text_and_order() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None)
            .await
            .expect("native GPU adapter is required");
        eprintln!("GPU evidence: {:?}", adapter.get_info());
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .unwrap();
        let size = wgpu::Extent3d {
            width: 256,
            height: 128,
            depth_or_array_layers: 1,
        };
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Surface offscreen evidence"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let viewport = Rect::new(0., 0., 256., 128.);
        let list = DisplayList {
            viewport,
            ops: vec![
                PaintOp::Quad {
                    rect: viewport,
                    clip: viewport,
                    radius: 0.,
                    color: Color::rgb(254, 247, 255),
                },
                PaintOp::Quad {
                    rect: Rect::new(16., 16., 96., 48.),
                    clip: viewport,
                    radius: 24.,
                    color: Color::rgb(103, 80, 164),
                },
                PaintOp::Quad {
                    rect: Rect::new(128., 16., 96., 48.),
                    clip: Rect::new(128., 16., 48., 48.),
                    radius: 0.,
                    color: Color::rgb(0, 128, 64),
                },
                PaintOp::Text {
                    rect: Rect::new(16., 80., 160., 32.),
                    clip: viewport,
                    text: "GPU Rust".to_owned(),
                    font_size: 20.,
                    color: Color::rgb(29, 27, 32),
                },
                // This later primitive must cover text, proving display-list order.
                PaintOp::Quad {
                    rect: Rect::new(90., 80., 86., 32.),
                    clip: viewport,
                    radius: 0.,
                    color: Color::rgb(254, 247, 255),
                },
            ],
        };
        renderer
            .render(
                &list,
                &texture.create_view(&Default::default()),
                256,
                128,
                1.,
            )
            .unwrap();
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Surface readback"),
            size: 256 * 128 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = renderer.device.create_command_encoder(&Default::default());
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
                    bytes_per_row: Some(256 * 4),
                    rows_per_image: Some(128),
                },
            },
            size,
        );
        renderer.queue.submit(Some(encoder.finish()));
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        renderer.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();
        let pixels = buffer.slice(..).get_mapped_range();
        let pixel = |x: usize, y: usize| {
            let offset = (y * 256 + x) * 4;
            [
                pixels[offset],
                pixels[offset + 1],
                pixels[offset + 2],
                pixels[offset + 3],
            ]
        };
        assert_eq!(pixel(64, 40), [103, 80, 164, 255]);
        assert_eq!(
            pixel(16, 16),
            [254, 247, 255, 255],
            "rounded corner is transparent"
        );
        assert_eq!(pixel(140, 30), [0, 128, 64, 255]);
        assert_eq!(
            pixel(200, 30),
            [254, 247, 255, 255],
            "clip rejects the right side"
        );
        let ink = (80..112)
            .flat_map(|y| (16..90).map(move |x| (x, y)))
            .filter(|(x, y)| pixel(*x, *y)[0] < 150)
            .count();
        assert!(
            ink > 80,
            "font rasterization must produce visible text: {ink}"
        );
        for y in 80..112 {
            for x in 90..176 {
                assert_eq!(pixel(x, y), [254, 247, 255, 255], "later quad covers text");
            }
        }
        if let Ok(path) = std::env::var("SURFACE_GPU_EVIDENCE") {
            let file = std::fs::File::create(path).unwrap();
            let mut png = png::Encoder::new(file, 256, 128);
            png.set_color(png::ColorType::Rgba);
            png.set_depth(png::BitDepth::Eight);
            png.write_header()
                .unwrap()
                .write_image_data(&pixels)
                .unwrap();
        }
    });
}
