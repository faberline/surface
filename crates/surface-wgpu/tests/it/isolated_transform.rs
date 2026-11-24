use surface::{Color, ExtendedDisplayList, ExtendedPaintOp, PaintOp, PaintTransform, Rect};
use surface_wgpu::{Capture, ExtendedRenderError, Renderer};

const WIDTH: u32 = 144;
const HEIGHT: u32 = 104;
const VIEWPORT: Rect = Rect {
    x: 0.,
    y: 0.,
    width: WIDTH as f32,
    height: HEIGHT as f32,
};
const BASE: Color = Color::rgb(16, 40, 73);
const GREEN: Color = Color::rgb(36, 216, 95);
const RED: Color = Color::rgb(233, 69, 45);
const BLUE: Color = Color::rgb(60, 193, 226);
const GOLD: Color = Color::rgb(236, 171, 42);
const WHITE: Color = Color::rgb(255, 255, 255);
const MAGENTA: Color = Color::rgb(250, 16, 227);

fn quad(rect: Rect, color: Color) -> ExtendedPaintOp {
    PaintOp::Quad {
        rect,
        clip: VIEWPORT,
        radius: 0.,
        color,
    }
    .into()
}

fn transform() -> PaintTransform {
    PaintTransform {
        source: Rect::new(12., 14., 32., 20.),
        destination: Rect::new(52., 28., 64., 40.),
        clip: Rect::new(60., 31., 46., 34.),
        radius: 8.,
        opacity: 0.625,
    }
}

fn source_paint() -> Vec<ExtendedPaintOp> {
    vec![
        quad(Rect::new(11., 14., 1., 20.), MAGENTA),
        quad(Rect::new(44., 14., 1., 20.), MAGENTA),
        quad(Rect::new(12., 14., 16., 16.), GREEN),
        quad(Rect::new(28., 14., 16., 16.), RED),
        quad(
            Rect::new(12., 30., 32., 4.),
            Color(BLUE.0, BLUE.1, BLUE.2, 128),
        ),
    ]
}

fn scene(transform: PaintTransform) -> ExtendedDisplayList {
    let mut ops = vec![
        quad(VIEWPORT, BASE),
        ExtendedPaintOp::BeginOpacityGroup { opacity: 0.5 },
        quad(Rect::new(4., 61., 14., 11.), GOLD),
        ExtendedPaintOp::EndOpacityGroup,
        ExtendedPaintOp::BeginTransformGroup { transform },
    ];
    ops.extend(source_paint());
    ops.extend([
        ExtendedPaintOp::EndOpacityGroup,
        quad(Rect::new(98., 46., 5., 7.), WHITE),
    ]);
    ExtendedDisplayList::new(VIEWPORT, ops)
}

fn pixel(capture: &Capture, x: u32, y: u32) -> [u8; 4] {
    let byte = ((y * capture.width + x) * 4) as usize;
    capture.rgba[byte..byte + 4].try_into().unwrap()
}

fn rgba(color: Color) -> [u8; 4] {
    [color.0, color.1, color.2, color.3]
}

fn blend(under: Color, over: Color, alpha: f32) -> [u8; 4] {
    let a = rgba(under);
    let b = rgba(over);
    std::array::from_fn(|index| {
        (f32::from(a[index]) * (1. - alpha) + f32::from(b[index]) * alpha).round() as u8
    })
}

fn near(actual: [u8; 4], expected: [u8; 4]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(channel, target)| channel.abs_diff(target) <= 2),
        "actual {actual:?}, expected {expected:?}"
    );
}

fn save(capture: &Capture, name: &str) {
    let Ok(path) = std::env::var("SURFACE_GPU_EVIDENCE") else {
        return;
    };
    let path = std::path::Path::new(&path).with_file_name(format!("gpu-transform-{name}.png"));
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
fn gpu_offscreen_transform_groups_scale_crop_round_and_composite_in_device_pixels() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None).await.unwrap();
        eprintln!("GPU transformed-group evidence: {:?}", adapter.get_info());
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .unwrap();
        for scale in [1, 2] {
            let mut render = |list: &ExtendedDisplayList| {
                renderer
                    .capture_extended(list, WIDTH * scale, HEIGHT * scale, scale as f32)
                    .unwrap()
            };
            let transform = transform();
            let rounded = render(&scene(transform));
            let sample = |x, y| pixel(&rounded, x * scale, y * scale);
            // This old isolated opacity group is adjacent in draw order.
            near(sample(10, 66), blend(BASE, GOLD, 0.5));
            // Source pixels are mapped away. The following ordinary draw
            // stays on the real target and covers a section of the group.
            assert_eq!(sample(18, 20), rgba(BASE));
            assert_eq!(sample(100, 49), rgba(WHITE));
            near(sample(72, 42), blend(BASE, GREEN, transform.opacity));
            near(sample(93, 42), blend(BASE, RED, transform.opacity));
            near(
                sample(74, 63),
                blend(BASE, BLUE, transform.opacity * 128. / 255.),
            );
            assert_eq!(sample(60, 31), rgba(BASE), "rounded corner");
            assert_eq!(sample(57, 49), rgba(BASE), "independent clip");
            assert_eq!(sample(80, 66), rgba(BASE), "clip shorter than destination");
            let nominal = blend(BASE, GREEN, transform.opacity)[1];
            assert!(
                (60 * scale..68 * scale).any(|x| {
                    (31 * scale..39 * scale).any(|y| {
                        let green = pixel(&rounded, x, y)[1];
                        green > BASE.1 + 4 && green < nominal - 4
                    })
                }),
                "rounded clip edge must antialias at {scale}x"
            );

            let cropped = render(&scene(PaintTransform {
                clip: transform.destination,
                radius: 0.,
                opacity: 1.,
                ..transform
            }));
            // A mapping that sampled outside the source rect would mix the
            // purple trim into these edge samples.
            assert_eq!(pixel(&cropped, 52 * scale, 40 * scale), rgba(GREEN));
            assert_eq!(pixel(&cropped, 116 * scale - 1, 40 * scale), rgba(RED));
            near(
                pixel(&cropped, 84 * scale - 1, 40 * scale),
                blend(GREEN, RED, 0.25),
            );
            near(
                pixel(&cropped, 84 * scale, 40 * scale),
                blend(GREEN, RED, 0.75),
            );
            assert_eq!(pixel(&cropped, 52 * scale - 1, 40 * scale), rgba(BASE));
            assert_eq!(pixel(&cropped, 116 * scale, 40 * scale), rgba(BASE));

            let offscreen_source = PaintTransform {
                source: Rect::new(-4., -3., 12., 9.),
                destination: Rect::new(20., 75., 24., 18.),
                clip: VIEWPORT,
                radius: 0.,
                opacity: 1.,
            };
            let outside = render(&ExtendedDisplayList::new(
                VIEWPORT,
                vec![
                    quad(VIEWPORT, BASE),
                    ExtendedPaintOp::BeginTransformGroup {
                        transform: offscreen_source,
                    },
                    quad(VIEWPORT, GREEN),
                    ExtendedPaintOp::EndOpacityGroup,
                ],
            ));
            assert_eq!(pixel(&outside, 24 * scale, 87 * scale), rgba(BASE));
            assert_eq!(pixel(&outside, 38 * scale, 78 * scale), rgba(BASE));
            assert_eq!(pixel(&outside, 38 * scale, 88 * scale), rgba(GREEN));
            assert_eq!(render(&scene(transform)).rgba, rounded.rgba);
            if scale == 2 {
                save(&rounded, "rounded");
                save(&cropped, "crop");
            }
        }
    });
}

#[test]
#[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
fn gpu_offscreen_transform_identity_matches_direct_and_opacity_and_rejects_invalid_groups() {
    use ExtendedPaintOp::{
        BeginOpacityGroup as BeginOpacity, BeginTransformGroup as Begin, EndOpacityGroup as End,
    };
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None).await.unwrap();
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Bgra8UnormSrgb)
            .await
            .unwrap();
        let identity = PaintTransform {
            source: VIEWPORT,
            destination: VIEWPORT,
            clip: VIEWPORT,
            radius: 0.,
            opacity: 1.,
        };
        let base = quad(VIEWPORT, BASE);
        let leaf = quad(Rect::new(10., 11., 40., 20.), GREEN);
        let scale = 2.;
        let mut render = |ops| {
            renderer.capture_extended(
                &ExtendedDisplayList::new(VIEWPORT, ops),
                WIDTH * 2,
                HEIGHT * 2,
                scale,
            )
        };
        for opacity in [0., 0.5, 1.] {
            let transformed = render(vec![
                base.clone(),
                Begin {
                    transform: PaintTransform {
                        opacity,
                        ..identity
                    },
                },
                leaf.clone(),
                End,
            ])
            .unwrap();
            let ordinary = render(vec![
                base.clone(),
                BeginOpacity { opacity },
                leaf.clone(),
                End,
            ])
            .unwrap();
            assert_eq!(transformed.rgba, ordinary.rgba);
        }
        let direct = render(vec![base.clone(), leaf.clone()]).unwrap();
        let identity = render(vec![
            base.clone(),
            Begin {
                transform: identity,
            },
            leaf,
            End,
        ])
        .unwrap();
        assert_eq!(identity.rgba, direct.rgba);

        let full = transform();
        let no_intersection = PaintTransform {
            clip: Rect::new(0., 0., 5., 5.),
            ..full
        };
        assert_eq!(
            render(vec![
                base.clone(),
                Begin {
                    transform: no_intersection
                },
                quad(VIEWPORT, RED),
                End
            ])
            .unwrap()
            .rgba,
            render(vec![base]).unwrap().rgba,
        );
        for bad in [
            PaintTransform {
                radius: -1.,
                ..full
            },
            PaintTransform {
                opacity: 1.25,
                ..full
            },
            PaintTransform {
                opacity: f32::NAN,
                ..full
            },
            PaintTransform {
                source: Rect::new(1., 1., 0., 5.),
                opacity: 0.,
                ..full
            },
            PaintTransform {
                clip: Rect::new(f32::NAN, 1., 2., 5.),
                ..full
            },
            PaintTransform {
                destination: Rect::new(1., 1., f32::MAX, 8.),
                ..full
            },
        ] {
            assert!(matches!(
                render(vec![Begin { transform: bad }, End]),
                Err(ExtendedRenderError::InvalidPaintTransform(_))
            ));
        }
        for ops in [
            vec![Begin { transform: full }],
            vec![
                BeginOpacity { opacity: 0. },
                Begin { transform: full },
                End,
                End,
            ],
            vec![
                Begin { transform: full },
                BeginOpacity { opacity: 1. },
                End,
                End,
            ],
            vec![Begin { transform: full }, End, End],
        ] {
            assert!(matches!(
                render(ops),
                Err(ExtendedRenderError::InvalidOpacityGroup(_))
            ));
        }
    });
}
