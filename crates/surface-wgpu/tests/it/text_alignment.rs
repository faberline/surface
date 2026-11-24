use surface::{
    Color, ExtendedDisplayList, Insets, InteractionState, LayoutEngine, Length, MicroDom, Node,
    Role, Style, TextAlignment,
};
use surface_wgpu::{Capture, Renderer};

const TEXT_ORIGINS: [u32; 3] = [8, 96, 184];
const TEXT_WIDTH: u32 = 80;

fn line_ink(capture: &Capture, origin: u32, row: u32, scale: u32) -> (u32, u32) {
    let mut left = u32::MAX;
    let mut right = 0;
    for y in (8 + row * 32) * scale..(8 + (row + 1) * 32) * scale {
        for x in origin * scale..(origin + TEXT_WIDTH) * scale {
            let offset = ((y * capture.width + x) * 4) as usize;
            if capture.rgba[offset] < 128 {
                left = left.min(x);
                right = right.max(x);
            }
        }
    }
    assert!(left < right, "No ink in visual row {row} at {origin}");
    (left, right)
}

fn save(capture: &Capture) {
    let Ok(path) = std::env::var("SURFACE_GPU_EVIDENCE") else {
        return;
    };
    let path = std::path::Path::new(&path).with_file_name("gpu-text-alignment.png");
    let mut encoder = png::Encoder::new(
        std::fs::File::create(path).unwrap(),
        capture.width,
        capture.height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&capture.rgba)
        .unwrap();
}

#[test]
#[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
fn gpu_offscreen_text_alignment_uses_each_visual_line() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None)
            .await
            .expect("native GPU adapter is required");
        eprintln!("GPU text alignment evidence: {:?}", adapter.get_info());
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .unwrap();

        let mut dom = MicroDom::new(
            Node::new(Role::Window, "").styled(
                Style::column()
                    .width(Length::Px(272.))
                    .height(Length::Px(112.))
                    .background(Color::rgb(255, 255, 255)),
            ),
        );
        let text = "編輯行程\n時間";
        assert_eq!(renderer.text.missing_glyphs(text), 0);
        for (origin, alignment) in TEXT_ORIGINS.into_iter().zip([
            TextAlignment::Left,
            TextAlignment::Center,
            TextAlignment::Right,
        ]) {
            dom.append(
                dom.root(),
                Node::new(Role::Heading, text)
                    .text(text)
                    .text_alignment(alignment)
                    .styled(
                        Style::column()
                            .width(Length::Px(TEXT_WIDTH as f32))
                            .height(Length::Px(96.))
                            .absolute(Insets {
                                left: Length::Px(origin as f32),
                                top: Length::Px(8.),
                                ..Insets::default()
                            })
                            .font_size(24.)
                            .line_height(32.)
                            .foreground(Color::rgb(0, 0, 0)),
                    ),
            )
            .unwrap();
        }
        let layout = LayoutEngine::default()
            .compute(&dom, 272., 112., &mut renderer.text)
            .unwrap();
        let frame = ExtendedDisplayList::compile_with_interaction(
            &dom,
            &layout,
            None,
            &Default::default(),
            InteractionState::default(),
        );
        for scale in [1, 2] {
            let capture = renderer
                .capture_extended(&frame, 272 * scale, 112 * scale, scale as f32)
                .unwrap();
            for row in 0..3 {
                let [left, center, right] =
                    TEXT_ORIGINS.map(|origin| line_ink(&capture, origin, row, scale));
                let left_gap = left.0 - TEXT_ORIGINS[0] * scale;
                let right_gap = (TEXT_ORIGINS[2] + TEXT_WIDTH) * scale - 1 - right.1;
                let center_twice = (2 * TEXT_ORIGINS[1] + TEXT_WIDTH - 1) * scale;
                assert!(
                    left_gap <= 4 * scale && right_gap <= 4 * scale,
                    "row {row} gaps {left_gap}, {right_gap} at {scale}x"
                );
                assert!(
                    (center.0 + center.1).abs_diff(center_twice) <= 6 * scale,
                    "row {row} center {center:?} at {scale}x"
                );
                assert!((left.1 - left.0).abs_diff(right.1 - right.0) <= 2 * scale);
            }
            let [long, short, medium] = [0, 1, 2].map(|row| {
                let (left, right) = line_ink(&capture, TEXT_ORIGINS[2], row, scale);
                right - left
            });
            assert!(
                long > medium && medium > short,
                "expected a soft wrap and another paragraph: {long}, {short}, {medium}"
            );
            if scale == 2 {
                save(&capture);
            }
        }
    });
}
