use surface::{
    Color, Corners, ExtendedDisplayList, ExtendedPaintOp, Insets, InteractionState, LayoutEngine,
    Length, MicroDom, Node, PaintOp, PathFill, RasterImage, RasterPaint, Rect, Role, Style,
    TextAlignment,
};
use surface_wgpu::{Capture, ExtendedRenderError, Renderer};

const WIDTH: u32 = 160;
const HEIGHT: u32 = 128;
const VIEWPORT: Rect = Rect {
    x: 0.,
    y: 0.,
    width: WIDTH as f32,
    height: HEIGHT as f32,
};
const BASE: Color = Color::rgb(28, 74, 93);
const PANEL: Color = Color::rgb(228, 104, 32);
const CHILD: Color = Color::rgb(43, 194, 158);
const SECOND: Color = Color::rgb(94, 218, 27);

fn quad(rect: Rect, color: Color) -> ExtendedPaintOp {
    PaintOp::Quad {
        rect,
        clip: VIEWPORT,
        radius: 0.,
        color,
    }
    .into()
}

fn aligned_label(renderer: &mut Renderer) -> ExtendedPaintOp {
    let mut dom = MicroDom::new(Node::new(Role::Window, ""));
    let clip = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Px(58.))
                    .height(Length::Px(23.))
                    .clip(true)
                    .absolute(Insets {
                        left: Length::Px(18.),
                        top: Length::Px(87.),
                        ..Insets::default()
                    }),
            ),
        )
        .unwrap();
    dom.append(
        clip,
        Node::new(Role::Heading, "")
            .text("Title\nNext")
            .text_alignment(TextAlignment::Right)
            .styled(
                Style::column()
                    .width(Length::Px(62.))
                    .height(Length::Px(40.))
                    .font_size(15.)
                    .line_height(20.)
                    .foreground(Color::rgb(8, 16, 25)),
            ),
    )
    .unwrap();
    let layout = LayoutEngine::default()
        .compute(&dom, VIEWPORT.width, VIEWPORT.height, &mut renderer.text)
        .unwrap();
    let mut ops = ExtendedDisplayList::compile_with_interaction(
        &dom,
        &layout,
        None,
        &Default::default(),
        InteractionState::default(),
    )
    .into_ops();
    assert_eq!(ops.len(), 1);
    let labeled = ops.pop().unwrap();
    assert!(matches!(&labeled, ExtendedPaintOp::AlignedText(text)
        if text.alignment() == TextAlignment::Right
        && text.run().clip() == Rect::new(18., 87., 58., 23.)));
    labeled
}

fn fixture(renderer: &mut Renderer) -> Vec<Vec<ExtendedPaintOp>> {
    let raster = RasterImage::new(1, 1, [237, 45, 35, 157].to_vec()).unwrap();
    let path = PathFill::new(
        [[36., 60.], [84., 60.], [84., 86.], [54., 82.]],
        Color(35, 62, 240, 197),
        Rect::new(45., 65., 30., 15.),
    )
    .unwrap();
    vec![
        vec![quad(VIEWPORT, BASE)],
        vec![
            quad(Rect::new(8., 8., 80., 110.), PANEL),
            quad(Rect::new(24., 24., 36., 30.), CHILD),
            ExtendedPaintOp::Raster(
                RasterPaint::new(
                    raster,
                    Rect::new(62., 32., 38., 21.),
                    Rect::new(66., 35., 22., 16.),
                    Corners::all(4.),
                )
                .unwrap(),
            ),
            ExtendedPaintOp::FillPath(path),
            aligned_label(renderer),
            // A fractional leaf over an otherwise transparent area.
            quad(Rect::new(89., 85., 14., 17.), Color(18, 201, 234, 122)),
        ],
        vec![
            // These two groups are adjacent. Most pixels are disjoint, so
            // a stale intermediate would be visible on the second composite.
            // This first square overlaps to make their order observable.
            quad(Rect::new(36., 31., 18., 13.), SECOND),
            quad(Rect::new(106., 63., 46., 49.), SECOND),
            quad(Rect::new(120., 78., 29., 24.), Color(120, 33, 199, 180)),
        ],
        vec![quad(
            Rect::new(20., 19., 11., 40.),
            Color::rgb(204, 224, 137),
        )],
    ]
}

fn display_list(parts: &[Vec<ExtendedPaintOp>], opacity: Option<f32>) -> ExtendedDisplayList {
    let mut ops = parts[0].clone();
    for group in &parts[1..3] {
        if let Some(opacity) = opacity {
            ops.push(ExtendedPaintOp::BeginOpacityGroup { opacity });
        }
        ops.extend_from_slice(group);
        if opacity.is_some() {
            ops.push(ExtendedPaintOp::EndOpacityGroup);
        }
    }
    ops.extend_from_slice(&parts[3]);
    ExtendedDisplayList::new(VIEWPORT, ops)
}

fn pixel(capture: &Capture, x: u32, y: u32) -> [u8; 4] {
    let start = ((y * capture.width + x) * 4) as usize;
    capture.rgba[start..start + 4].try_into().unwrap()
}

fn midpoint(base: u8, full: u8, srgb: bool) -> u8 {
    let decode = |n| {
        let v = f64::from(n) / 255.;
        if !srgb {
            v
        } else if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let linear = (decode(base) + decode(full)) / 2.;
    let encoded = if !srgb {
        linear
    } else if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1. / 2.4) - 0.055
    };
    (encoded * 255.).round() as u8
}

fn save(capture: &Capture) {
    if let Ok(path) = std::env::var("SURFACE_GPU_EVIDENCE") {
        let path = std::path::Path::new(&path).with_file_name("gpu-opacity-group.png");
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
}

#[test]
#[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
fn gpu_offscreen_opacity_groups_composite_once_and_preserve_draw_lanes() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None)
            .await
            .expect("native GPU adapter is required");
        eprintln!("GPU opacity evidence: {:?}", adapter.get_info());
        for format in [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ] {
            let mut renderer = Renderer::new(&adapter, format).await.unwrap();
            let parts = fixture(&mut renderer);
            let base = ExtendedDisplayList::new(
                VIEWPORT,
                parts[0].iter().chain(&parts[3]).cloned().collect(),
            );
            for scale in [1, 2] {
                let mut capture = |list: &ExtendedDisplayList| {
                    renderer
                        .capture_extended(list, WIDTH * scale, HEIGHT * scale, scale as f32)
                        .unwrap()
                };
                let backdrop = capture(&base);
                let opaque = capture(&display_list(&parts, None));
                let half = capture(&display_list(&parts, Some(0.5)));
                assert_eq!(capture(&display_list(&parts, Some(0.))).rgba, backdrop.rgba);
                assert_eq!(capture(&display_list(&parts, Some(1.))).rgba, opaque.rgba);
                assert_eq!(capture(&display_list(&parts, Some(0.5))).rgba, half.rgba);

                let opaque_at = |x, y| pixel(&opaque, x * scale, y * scale);
                let base_at = |x, y| pixel(&backdrop, x * scale, y * scale);
                let half_at = |x, y| pixel(&half, x * scale, y * scale);
                assert_eq!(opaque_at(34, 34), [CHILD.0, CHILD.1, CHILD.2, 255]);
                assert_eq!(opaque_at(40, 34), [SECOND.0, SECOND.1, SECOND.2, 255]);
                assert_eq!(opaque_at(65, 43), opaque_at(67, 55));
                assert_ne!(opaque_at(72, 43), opaque_at(65, 43), "raster lane");
                assert_eq!(opaque_at(78, 67), opaque_at(78, 59));
                assert_ne!(opaque_at(65, 71), opaque_at(78, 67), "path lane");
                assert_ne!(opaque_at(94, 92), base_at(94, 92));
                assert_eq!(
                    half_at(25, 35),
                    base_at(25, 35),
                    "last quad is above the group"
                );
                assert!(
                    (87 * scale..110 * scale).any(|y| {
                        (18 * scale..76 * scale).any(|x| pixel(&opaque, x, y)[0] < 180)
                    }),
                    "aligned text ink inside clip"
                );
                assert!(
                    (87 * scale..113 * scale).all(|y| {
                        (76 * scale..82 * scale).all(|x| pixel(&opaque, x, y)[0] == PANEL.0)
                    }),
                    "aligned text does not paint past clip"
                );

                // Direct reference paint determines the isolated foreground.
                // With two real partial ranges, this detects stale intermediate
                // pixels as well as accidental attenuation of each leaf.
                for y in 0..half.height {
                    for x in 0..half.width {
                        let destination = pixel(&backdrop, x, y);
                        let source = pixel(&opaque, x, y);
                        let actual = pixel(&half, x, y);
                        let overlap = (36 * scale..54 * scale).contains(&x)
                            && (31 * scale..44 * scale).contains(&y);
                        for channel in 0..3 {
                            let below = if overlap {
                                midpoint(
                                    destination[channel],
                                    [CHILD.0, CHILD.1, CHILD.2][channel],
                                    format.is_srgb(),
                                )
                            } else {
                                destination[channel]
                            };
                            let expected = midpoint(below, source[channel], format.is_srgb());
                            assert!(actual[channel].abs_diff(expected) <= 2,
                                "{format:?} {scale}x pixel ({x},{y}) channel {channel}: {actual:?}, expected {expected}");
                        }
                        assert_eq!(actual[3], 255);
                    }
                }
                if format.is_srgb() && scale == 2 {
                    save(&half);
                }
            }
        }
    });
}

#[test]
#[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
fn gpu_offscreen_opacity_groups_reject_invalid_ranges() {
    use ExtendedPaintOp::{BeginOpacityGroup as Begin, EndOpacityGroup as End};
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None).await.unwrap();
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .unwrap();
        for ops in [
            vec![End],
            vec![Begin { opacity: 0.5 }],
            vec![Begin { opacity: 0. }, Begin { opacity: 1. }, End, End],
            vec![Begin { opacity: -0.001 }, End],
            vec![Begin { opacity: 1.001 }, End],
            vec![Begin { opacity: f32::NAN }, End],
            vec![
                Begin {
                    opacity: f32::INFINITY,
                },
                End,
            ],
        ] {
            let list = ExtendedDisplayList::new(VIEWPORT, ops);
            assert!(matches!(
                renderer.capture_extended(&list, WIDTH, HEIGHT, 1.),
                Err(ExtendedRenderError::InvalidOpacityGroup(_))
            ));
        }
    });
}
