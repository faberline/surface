use surface::{Color, Corners, DisplayList, FontFamily, PaintOp, Rect, TextMeasurer, TextStyle};
use surface_wgpu::{Renderer, TextSystem};

/// A copy of the bundled font that claims weight 700 and half the units per em,
/// so every glyph is twice as wide. It is built in memory for this test only.
/// The real bundle has no bold face, so this is the only way to see whether a
/// requested weight reaches the shaper.
fn heavy_face() -> Vec<u8> {
    let mut bytes = include_bytes!("../assets/Tuffy.ttf").to_vec();
    let table = |bytes: &[u8], tag: &[u8; 4]| -> usize {
        let count = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
        (0..count)
            .map(|i| 12 + 16 * i)
            .find(|record| &bytes[*record..*record + 4] == tag)
            .map(|record| {
                u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap()) as usize
            })
            .expect("font table")
    };
    let os2 = table(&bytes, b"OS/2");
    bytes[os2 + 4..os2 + 6].copy_from_slice(&700u16.to_be_bytes());
    let head = table(&bytes, b"head");
    let units = u16::from_be_bytes([bytes[head + 18], bytes[head + 19]]);
    bytes[head + 18..head + 20].copy_from_slice(&(units / 2).to_be_bytes());
    bytes
}

#[test]
fn text_system_applies_line_height_and_weight() {
    let mut text = TextSystem::default();
    let style = |line_height: Option<f32>, weight: u16| TextStyle {
        line_height,
        weight,
        ..TextStyle::new(16.)
    };

    // Size-only measuring is exactly the styled measuring of a legacy style.
    let legacy = text.measure("Surface", 16., None);
    assert_eq!(
        text.measure_text("Surface", &style(None, 400), None),
        legacy
    );
    assert_eq!(legacy.1, 24., "no line height means 1.5 times the size");

    // The contract case: two lines at size 16 and line height 30 are 60 high.
    let two_lines = text.measure_text("A\nB", &style(Some(30.), 400), None);
    assert!((two_lines.1 - 60.).abs() <= 1., "{two_lines:?}");
    let layout = text.text_layout_styled("A\nB", &style(Some(30.), 400), None);
    assert_eq!(layout.line_height, 30.);
    assert!((layout.height - 60.).abs() <= 1., "{}", layout.height);
    // Weight 700 and the family Tuffy shape to finite, positive sizes.
    let bold = text.measure_text("A\nB", &style(None, 700), None);
    let tuffy = text.measure_text(
        "A\nB",
        &TextStyle {
            family: FontFamily::Named("Tuffy".to_owned()),
            ..TextStyle::new(16.)
        },
        None,
    );
    for (width, height) in [bold, tuffy] {
        assert!(width.is_finite() && width > 0., "{width}");
        assert!(height.is_finite() && height > 0., "{height}");
    }

    // One line is as tall as the requested line height, at any value.
    for line_height in [14., 20., 24., 40., 64.] {
        let (width, height) = text.measure_text("Surface", &style(Some(line_height), 400), None);
        assert_eq!(height, line_height);
        assert_eq!(width, legacy.0, "line height must not change the width");
    }
    // Wrapped lines stack by the styled line height.
    let wrapped = text.measure_text(
        "Surface Rust application",
        &style(Some(40.), 400),
        Some(70.),
    );
    assert!(wrapped.1 >= 80. && wrapped.1 % 40. == 0., "{wrapped:?}");
    let layout = text.text_layout_styled("ab\ncd", &style(Some(40.), 400), None);
    assert_eq!(layout.line_height, 40.);
    assert_eq!(layout.height, 80.);
    assert_eq!(layout.caret("ab\n".len()).y, 40.);
    assert_eq!(layout.caret("ab\n".len()).height, 40.);
    let tight = text.text_layout_styled("ab\ncd", &style(Some(18.), 400), None);
    assert_eq!(tight.line_height, 18.);
    assert_eq!(tight.caret("ab\n".len()).y, 18.);
    assert_eq!(
        text.text_layout("ab\ncd", 16., None).line_height,
        24.,
        "the size-only layout is unchanged"
    );

    // A zero or negative line height never reaches the shaper as a panic.
    let degenerate = text.measure_text("Surface", &style(Some(0.), 400), None);
    assert_eq!(degenerate.1, 24.);

    // Family: a name selects the face that carries it. Tuffy and Noto Sans CJK
    // TC are different typefaces, so the same text measures differently. The
    // default family draws regular text with Noto Sans CJK TC. Tuffy declares
    // weight 500, so asking cosmic-text for 400 by name would have missed it
    // and shaped with Noto. The bundled faces have one weight each, so a name
    // gives the same face at every weight.
    let tuffy_regular = in_family(&mut text, "Tuffy", 400);
    assert_ne!(
        tuffy_regular, legacy,
        "Named(\"Tuffy\") must change the face"
    );
    assert!(
        tuffy_regular.0 < legacy.0,
        "{tuffy_regular:?} vs {legacy:?}"
    );
    assert_eq!(tuffy_regular.1, 24.);
    let noto = in_family(&mut text, "Noto Sans CJK TC", 400);
    assert_eq!(
        noto, legacy,
        "the default family shapes with Noto Sans CJK TC"
    );
    for weight in [100, 500, 700, 900] {
        assert_eq!(in_family(&mut text, "Tuffy", weight), tuffy_regular);
        assert_eq!(in_family(&mut text, "Noto Sans CJK TC", weight), noto);
    }
    // A name that no face carries shapes like the default family.
    for weight in [400, 700] {
        assert_eq!(in_family(&mut text, "No Such Family", weight), legacy);
    }
    // Tuffy has no Han glyphs. Text it cannot draw still falls back to a face
    // that can, so a named family never leaves missing-glyph boxes.
    let han = text.measure("\u{4f60}\u{597d}", 16., None);
    let tuffy_han = TextStyle {
        family: FontFamily::Named("Tuffy".to_owned()),
        ..TextStyle::new(16.)
    };
    assert_eq!(text.measure_text("\u{4f60}\u{597d}", &tuffy_han, None), han);

    // Weight: before a heavier face exists every weight uses the only face.
    for weight in [100, 400, 500, 700, 900] {
        assert_eq!(
            text.measure_text("Surface", &style(None, weight), None),
            legacy,
            "weight {weight}"
        );
    }
    text.add_font(heavy_face());
    assert_eq!(
        text.measure_text("Surface", &style(None, 400), None),
        legacy,
        "regular text still picks the regular face"
    );
    let heavy = text.measure_text("Surface", &style(None, 700), None);
    assert!(
        heavy.0 > legacy.0 * 1.5,
        "weight 700 must pick the face that claims it: {heavy:?} vs {legacy:?}"
    );
    assert_eq!(heavy.1, 24.);
    // Each request takes the nearest weight that has its own face.
    assert_eq!(
        text.measure_text("Surface", &style(None, 500), None),
        legacy
    );
    assert_eq!(text.measure_text("Surface", &style(None, 600), None), heavy);
    assert_eq!(text.measure_text("Surface", &style(None, 900), None), heavy);
    // Weight and line height apply together.
    let both = text.measure_text("Surface", &style(Some(40.), 700), None);
    assert_eq!(both, (heavy.0, 40.));
    assert_eq!(
        text.text_layout_styled("Surface", &style(Some(40.), 700), None)
            .clusters
            .len(),
        7
    );

    // A named family now has two Tuffy faces: the bundled one at 500 and the
    // added one at 700. A request takes the nearest weight inside the family.
    let tuffy_at = |text: &mut TextSystem, weight| in_family(text, "Tuffy", weight);
    assert_eq!(tuffy_at(&mut text, 400), tuffy_regular);
    assert_eq!(tuffy_at(&mut text, 500), tuffy_regular);
    assert_eq!(
        tuffy_at(&mut text, 600),
        tuffy_regular,
        "a tie goes to the lighter face"
    );
    assert_eq!(tuffy_at(&mut text, 700), heavy);
    assert_eq!(tuffy_at(&mut text, 900), heavy);
    // The default family and a family it does not own are unaffected.
    assert_eq!(in_family(&mut text, "Noto Sans CJK TC", 700), legacy);
    assert_eq!(in_family(&mut text, "No Such Family", 400), legacy);
}

fn in_family(text: &mut TextSystem, name: &str, weight: u16) -> (f32, f32) {
    let style = TextStyle {
        weight,
        family: FontFamily::Named(name.to_owned()),
        ..TextStyle::new(16.)
    };
    text.measure_text("Surface", &style, None)
}

#[test]
fn chinese_font_and_shaped_editing_geometry_have_real_glyphs() {
    let mut text = TextSystem::default();
    assert_eq!(text.missing_glyphs("繁體中文简体汉语ㄅㄆㄇㄓㄨㄥ✓−×"), 0);
    let layout = text.text_layout("中文輸入", 18., None);
    assert_eq!(layout.clusters.len(), 4);
    for cluster in &layout.clusters {
        assert!(cluster.bounds.width > 10.);
    }
    let caret = layout.caret("中文".len());
    assert!(caret.x > 20.);
    assert_eq!(layout.hit(caret.x + 1., caret.y + 5.), "中文".len());
    assert!(!layout.selection(0.."中文".len()).is_empty());
    let wrapped = text.text_layout("中文\n輸入", 18., Some(100.));
    assert!(wrapped.caret("中文\n".len()).y > 0.);
}

#[test]
fn subset_font_keeps_the_chinese_text_the_tests_use() {
    let mut text = TextSystem::with_cjk_font(include_bytes!("../assets/NotoSansCJKtc-Subset.otf"));
    assert_eq!(
        text.missing_glyphs("繁體中文简体汉语ㄅㄆㄇㄓㄨㄥ✓−×，。「」、"),
        0
    );
    assert_eq!(text.missing_glyphs("ㄓㄨㄥ"), 0);
}

#[test]
fn subset_font_is_a_real_subset() {
    let mut text = TextSystem::with_cjk_font(include_bytes!("../assets/NotoSansCJKtc-Subset.otf"));
    assert_eq!(text.missing_glyphs("龘"), 1);
    assert!(include_bytes!("../assets/NotoSansCJKtc-Subset.otf").len() < 5_000_000);
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(TextSystem::default().missing_glyphs("龘"), 0);
}

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
                PaintOp::Stroke {
                    rect: Rect::new(228., 8., 20., 40.),
                    clip: viewport,
                    radius: 6.,
                    width: 2.,
                    color: Color::rgb(40, 80, 180),
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
        assert_eq!(pixel(238, 9), [40, 80, 180, 255], "rounded stroke edge");
        assert_eq!(
            pixel(238, 20),
            [254, 247, 255, 255],
            "stroke leaves its center clear"
        );
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

#[test]
#[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
fn gpu_offscreen_per_corner_shapes() {
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
            label: Some("Surface per-corner shapes"),
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
                // Square on the left side, rounded at the top-right and bottom-left.
                PaintOp::Shape {
                    rect: Rect::new(16., 16., 96., 96.),
                    clip: viewport,
                    corners: Corners::new(0., 48., 0., 48.),
                    color: Color::rgb(255, 0, 0),
                },
                // Rounded at the top-left and bottom-right, square elsewhere.
                PaintOp::ShapeStroke {
                    rect: Rect::new(144., 16., 96., 96.),
                    clip: viewport,
                    corners: Corners::new(32., 0., 32., 0.),
                    width: 4.,
                    color: Color::rgb(0, 0, 255),
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
        // The renderer clears the target to opaque black before the first op.
        let background = [0, 0, 0, 255];
        let red = [255, 0, 0, 255];
        let blue = [0, 0, 255, 255];
        assert_eq!(pixel(250, 120), background, "untouched target is cleared");
        assert_eq!(pixel(18, 18), red, "shape: square top-left corner");
        assert_eq!(
            pixel(109, 19),
            background,
            "shape: rounded top-right corner"
        );
        assert_eq!(pixel(64, 64), red, "shape: center is filled");
        assert_eq!(pixel(146, 64), blue, "stroke: left edge");
        assert_eq!(pixel(192, 64), background, "stroke: inside is clear");
        assert_eq!(
            pixel(147, 19),
            background,
            "stroke: rounded top-left corner"
        );
        assert_eq!(pixel(237, 18), blue, "stroke: square top-right corner");
        if let Ok(path) = std::env::var("SURFACE_GPU_EVIDENCE") {
            let path = std::path::Path::new(&path).with_file_name("gpu-shapes.png");
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

#[test]
#[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
fn gpu_offscreen_bottom_corners_are_not_swapped() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None)
            .await
            .expect("native GPU adapter is required");
        eprintln!("GPU evidence: {:?}", adapter.get_info());
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .unwrap();
        let viewport = Rect::new(0., 0., 256., 128.);
        let list = DisplayList {
            viewport,
            ops: vec![
                // Only the bottom-left corner is rounded.
                PaintOp::Shape {
                    rect: Rect::new(8., 32., 64., 64.),
                    clip: viewport,
                    corners: Corners::new(0., 0., 0., 32.),
                    color: Color::rgb(255, 0, 0),
                },
                // Only the bottom-right corner is rounded.
                PaintOp::Shape {
                    rect: Rect::new(96., 32., 64., 64.),
                    clip: viewport,
                    corners: Corners::new(0., 0., 32., 0.),
                    color: Color::rgb(255, 0, 0),
                },
                // A stroke with only the bottom-left corner rounded.
                PaintOp::ShapeStroke {
                    rect: Rect::new(184., 32., 64., 64.),
                    clip: viewport,
                    corners: Corners::new(0., 0., 0., 32.),
                    width: 4.,
                    color: Color::rgb(0, 0, 255),
                },
            ],
        };
        let capture = renderer.capture(&list, 256, 128, 1.).unwrap();
        assert_eq!((capture.width, capture.height), (256, 128));
        let pixels = &capture.rgba;
        let pixel = |x: usize, y: usize| {
            let offset = (y * 256 + x) * 4;
            [
                pixels[offset],
                pixels[offset + 1],
                pixels[offset + 2],
                pixels[offset + 3],
            ]
        };
        // The renderer clears the target to opaque black before the first op.
        let background = [0, 0, 0, 255];
        let red = [255, 0, 0, 255];
        let blue = [0, 0, 255, 255];
        assert_eq!(pixel(250, 120), background, "untouched target is cleared");
        assert_eq!(pixel(10, 93), background, "A: bottom-left is rounded");
        assert_eq!(pixel(69, 34), red, "A: top-right is square");
        assert_eq!(pixel(69, 93), red, "A: bottom-right is square");
        assert_eq!(pixel(157, 93), background, "B: bottom-right is rounded");
        assert_eq!(pixel(98, 93), red, "B: bottom-left is square");
        assert_eq!(pixel(98, 34), red, "B: top-left is square");
        assert_eq!(pixel(186, 93), background, "stroke: bottom-left is rounded");
        assert_eq!(pixel(245, 93), blue, "stroke: bottom-right is square");
        assert_eq!(pixel(186, 34), blue, "stroke: top-left is square");
        assert_eq!(pixel(245, 34), blue, "stroke: top-right is square");
    });
}

#[test]
fn capture_rejects_oversize_requests_without_a_gpu() {
    use surface_wgpu::{Capture, RenderError, MAX_CAPTURE_SIDE};
    assert!(Capture::check_size(1, 1).is_ok());
    assert!(Capture::check_size(MAX_CAPTURE_SIDE, MAX_CAPTURE_SIDE).is_ok());
    for (w, h) in [
        (MAX_CAPTURE_SIDE + 1, 1),
        (1, MAX_CAPTURE_SIDE + 1),
        (u32::MAX, u32::MAX),
    ] {
        match Capture::check_size(w, h) {
            Err(RenderError::CaptureTooLarge { width, height, max }) => {
                assert_eq!((width, height, max), (w, h, MAX_CAPTURE_SIDE));
            }
            other => panic!("{w}x{h} must be rejected, got {other:?}"),
        }
    }
    assert!(matches!(
        Capture::check_size(0, 10),
        Err(RenderError::Dimensions)
    ));
}

#[test]
#[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
fn gpu_offscreen_capture_reads_rgba_in_both_formats() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None)
            .await
            .expect("native GPU adapter is required");
        eprintln!("GPU evidence: {:?}", adapter.get_info());
        let viewport = Rect::new(0., 0., 70., 40.);
        // 70 px * 4 bytes = 280, so rows are not 256-byte aligned: padding must be removed.
        let list = DisplayList {
            viewport,
            ops: vec![
                PaintOp::Quad {
                    rect: Rect::new(0., 0., 35., 40.),
                    clip: viewport,
                    radius: 0.,
                    color: Color::rgb(255, 0, 0),
                },
                PaintOp::Quad {
                    rect: Rect::new(35., 0., 35., 20.),
                    clip: viewport,
                    radius: 0.,
                    color: Color::rgb(0, 0, 255),
                },
            ],
        };
        let formats = [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8Unorm,
        ];
        let mut tested = 0;
        let mut saved = None;
        for format in formats {
            let usages = adapter.get_texture_format_features(format).allowed_usages;
            if !usages
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC)
            {
                eprintln!("capture format {format:?} unsupported on this adapter");
                continue;
            }
            let mut renderer = Renderer::new(&adapter, format).await.unwrap();
            let capture = renderer.capture(&list, 70, 40, 1.).unwrap();
            assert_eq!((capture.width, capture.height), (70, 40), "{format:?}");
            assert_eq!(capture.rgba.len(), 70 * 40 * 4, "{format:?}: tight rows");
            let pixel = |x: usize, y: usize| {
                let o = (y * 70 + x) * 4;
                [
                    capture.rgba[o],
                    capture.rgba[o + 1],
                    capture.rgba[o + 2],
                    capture.rgba[o + 3],
                ]
            };
            assert_eq!(pixel(10, 10), [255, 0, 0, 255], "{format:?}: red as RGBA");
            assert_eq!(pixel(10, 39), [255, 0, 0, 255], "{format:?}: last row");
            assert_eq!(pixel(60, 10), [0, 0, 255, 255], "{format:?}: blue as RGBA");
            assert_eq!(pixel(69, 19), [0, 0, 255, 255], "{format:?}: last column");
            assert_eq!(pixel(60, 30), [0, 0, 0, 255], "{format:?}: cleared black");
            tested += 1;
            saved = Some(capture);
        }
        assert!(tested >= 1, "no capture format was usable");
        if let (Ok(path), Some(capture)) = (std::env::var("SURFACE_GPU_EVIDENCE"), saved) {
            let path = std::path::Path::new(&path).with_file_name("gpu-capture.png");
            let file = std::fs::File::create(path).unwrap();
            let mut png = png::Encoder::new(file, capture.width, capture.height);
            png.set_color(png::ColorType::Rgba);
            png.set_depth(png::BitDepth::Eight);
            png.write_header()
                .unwrap()
                .write_image_data(&capture.rgba)
                .unwrap();
        }
    });
}

struct MotionApp {
    on: bool,
}

impl surface_app::Application for MotionApp {
    type Message = ();
    fn update(&mut self, _: ()) {
        self.on = !self.on;
    }
    fn view(&self) -> surface_app::View<()> {
        use surface::{Easing, Length, Node, Role, Style, Transition};
        let mut style = Style::default()
            .width(Length::Px(200.))
            .height(Length::Px(120.))
            .background(if self.on {
                Color::rgb(255, 0, 0)
            } else {
                Color::rgb(0, 0, 255)
            })
            .corners(if self.on {
                Corners::all(40.)
            } else {
                Corners::all(0.)
            });
        style.transition = Some(Transition::new(200., Easing::Linear));
        surface_app::View::new(
            Node::new(Role::Window, "GPU motion")
                .id("root")
                .styled(Style::column().clip(true)),
        )
        .children([
            surface_app::View::new(Node::new(Role::Group, "box").id("box").styled(style)),
            surface_app::View::new(
                Node::new(Role::Button, "Toggle")
                    .id("toggle")
                    .text("Toggle")
                    .styled(Style::default().height(Length::Px(40.))),
            )
            .on_press(()),
        ])
    }
}

#[test]
#[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
fn gpu_offscreen_motion_animates_color_and_corners() {
    pollster::block_on(async {
        const W: u32 = 256;
        const H: u32 = 200;
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None)
            .await
            .expect("native GPU adapter is required");
        eprintln!("GPU evidence: {:?}", adapter.get_info());
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .unwrap();
        let size = wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        };
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Surface motion frames"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let mut text = TextSystem::default();
        let mut runtime = surface_app::Runtime::new(MotionApp { on: false }).unwrap();

        // Render one frame at the given host time and read the pixels back.
        let mut frame = |runtime: &mut surface_app::Runtime<MotionApp>, t: f64| -> Vec<u8> {
            runtime.set_time(t);
            let list = runtime.render(W as f32, H as f32, &mut text).unwrap();
            renderer.render(&list, &view, W, H, 1.).unwrap();
            let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Surface motion readback"),
                size: u64::from(W * H * 4),
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
                        bytes_per_row: Some(W * 4),
                        rows_per_image: Some(H),
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
            let pixels = buffer.slice(..).get_mapped_range().to_vec();
            pixels
        };
        let pixel = |pixels: &[u8], x: usize, y: usize| {
            let offset = (y * W as usize + x) * 4;
            [pixels[offset], pixels[offset + 1], pixels[offset + 2]]
        };
        let near = |got: [u8; 3], want: [u8; 3]| {
            got.iter()
                .zip(want)
                .all(|(g, w)| (i32::from(*g) - i32::from(w)).abs() <= 3)
        };

        let first = frame(&mut runtime, 0.);
        assert!(runtime.activate("toggle").unwrap());
        let start = frame(&mut runtime, 0.);
        let mid = frame(&mut runtime, 100.);
        assert!(runtime.needs_frame(), "the run is still going at t=100");
        let end = frame(&mut runtime, 200.);
        assert!(!runtime.needs_frame(), "the run is over at t=200");

        // Color: linear blue to red.
        assert!(near(pixel(&first, 100, 60), [0, 0, 255]));
        assert!(
            near(pixel(&start, 100, 60), [0, 0, 255]),
            "run starts shown"
        );
        assert!(
            near(pixel(&mid, 100, 60), [128, 0, 128]),
            "midpoint color: {:?}",
            pixel(&mid, 100, 60)
        );
        assert!(near(pixel(&end, 100, 60), [255, 0, 0]));

        // Corners: radius 0, 20 and 40. Probe (4,4) is covered only by radius 0.
        // Probe (8,8) is covered by radius 0 and 20 but not by 40.
        let dark = |p: [u8; 3]| p.iter().all(|c| *c < 40);
        assert!(!dark(pixel(&start, 4, 4)), "t=0 has square corners");
        assert!(!dark(pixel(&start, 8, 8)));
        assert!(
            dark(pixel(&mid, 4, 4)),
            "t=100 radius 20 leaves (4,4) empty: {:?}",
            pixel(&mid, 4, 4)
        );
        assert!(
            near(pixel(&mid, 8, 8), [128, 0, 128]),
            "t=100 radius 20 covers (8,8): {:?}",
            pixel(&mid, 8, 8)
        );
        assert!(dark(pixel(&end, 4, 4)));
        assert!(
            dark(pixel(&end, 8, 8)),
            "t=200 radius 40 leaves (8,8) empty"
        );

        if let Ok(path) = std::env::var("SURFACE_GPU_EVIDENCE") {
            let path = std::path::Path::new(&path).with_file_name("gpu-motion-midpoint.png");
            let file = std::fs::File::create(path).unwrap();
            let mut png = png::Encoder::new(file, W, H);
            png.set_color(png::ColorType::Rgba);
            png.set_depth(png::BitDepth::Eight);
            png.write_header().unwrap().write_image_data(&mid).unwrap();
        }
    });
}
