use surface::{Color, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, Application, Runtime, View};
use surface_material::{LoadingIndicatorMotion, LoadingIndicatorOptions, LoadingKind, Theme};
use surface_wgpu::{Capture, Renderer, TextSystem};

struct Loading {
    theme: Theme,
    size: f32,
    motion: LoadingIndicatorMotion,
}

impl Application for Loading {
    type Message = ();

    fn update(&mut self, (): ()) {}

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        self.motion.advance(&frame, true)
    }

    fn view(&self) -> View<()> {
        let indicator = |id, name, kind| {
            self.theme.loading_indicator_with_motion(
                LoadingIndicatorOptions {
                    id,
                    name,
                    kind,
                    size: self.size,
                },
                &self.motion,
            )
        };
        View::new(
            Node::new(Role::Window, "").styled(
                Style::row()
                    .width(Length::Percent(1.))
                    .height(Length::Percent(1.))
                    .padding(12.)
                    .gap(12.)
                    .background(self.theme.colors.surface),
            ),
        )
        .children([
            indicator("default", "Loading results", LoadingKind::Default),
            indicator("contained", "Reading cache", LoadingKind::Contained),
        ])
    }
}

fn bounds(runtime: &Runtime<Loading>, id: &str) -> Rect {
    let node = runtime.dom().by_id(id).unwrap();
    runtime.computed().unwrap().nodes[node.index()].bounds
}

fn rgba(color: Color) -> [u8; 4] {
    [color.0, color.1, color.2, color.3]
}

fn sample(capture: &Capture, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * capture.width + x) * 4) as usize;
    capture.rgba[offset..offset + 4].try_into().unwrap()
}

fn mask(capture: &Capture, rect: Rect, ink: Color) -> Vec<(u32, u32)> {
    let x = rect.x.round() as u32;
    let y = rect.y.round() as u32;
    let ink = rgba(ink);
    (0..rect.height.round() as u32)
        .flat_map(|row| (0..rect.width.round() as u32).map(move |column| (column, row)))
        .filter(|&(column, row)| sample(capture, x + column, y + row) == ink)
        .collect()
}

fn save(capture: &Capture, name: &str) {
    if let Ok(path) = std::env::var("SURFACE_GPU_EVIDENCE") {
        let path = std::path::Path::new(&path).with_file_name(format!("gpu-loading-{name}.png"));
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
fn gpu_offscreen_loading_indicator_paints_each_shape_and_reduced_motion() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None)
            .await
            .expect("native GPU adapter is required");
        eprintln!("GPU evidence: {:?}", adapter.get_info());
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .unwrap();
        let mut text = TextSystem::default();
        let mut frame = |runtime: &mut Runtime<Loading>, ms, width, height| {
            runtime.set_time(ms);
            let painter = runtime
                .render_extended(width as f32, height as f32, &mut text)
                .unwrap();
            renderer
                .capture_extended(&painter, width, height, 1.)
                .unwrap()
        };
        let mut runtime = Runtime::new(Loading {
            theme: Theme::light(),
            size: 48.,
            motion: LoadingIndicatorMotion::default(),
        })
        .unwrap();
        let light = Theme::light().colors;
        let first = frame(&mut runtime, 0., 132, 72);
        let default = bounds(&runtime, "default");
        let contained = bounds(&runtime, "contained");
        for rect in [default, contained] {
            assert_eq!((rect.width, rect.height), (48., 48.));
        }
        assert_eq!(
            bounds(&runtime, "default-shape").width,
            38.,
            "Material 3 active box"
        );
        assert_eq!(sample(&first, 36, 14), rgba(light.surface));
        assert_eq!(sample(&first, 96, 14), rgba(light.primary_container));
        let first_mask = mask(&first, default, light.primary);
        let middle = frame(&mut runtime, 90., 132, 72);
        let middle_mask = mask(&middle, default, light.primary);
        let mut masks = vec![first_mask];
        for index in 1..7 {
            let capture = frame(&mut runtime, f64::from(index) * 650., 132, 72);
            assert_eq!(bounds(&runtime, "default"), default);
            assert_eq!(bounds(&runtime, "contained"), contained);
            let ink = mask(&capture, default, light.primary);
            assert!(!masks.contains(&ink), "duplicate shape at {index}");
            assert!(!mask(&capture, contained, light.on_primary_container).is_empty());
            masks.push(ink);
        }
        assert!(
            !masks.contains(&middle_mask),
            "the morph must have intermediate paint"
        );
        for ink in masks.iter().chain([&middle_mask]) {
            assert!(ink.len() > 120, "indicator should remain visible");
            assert!(ink
                .iter()
                .all(|&(x, y)| (5..43).contains(&x) && (5..43).contains(&y)));
        }
        assert!(runtime.needs_frame());
        eprintln!(
            "Seven distinct native masks, pixel counts: {:?}",
            masks.iter().map(Vec::len).collect::<Vec<_>>()
        );

        runtime.set_reduced_motion(true);
        let reduced = frame(&mut runtime, 4100., 132, 72);
        assert_eq!(reduced.rgba, first.rgba);
        assert_eq!(frame(&mut runtime, 9000., 132, 72).rgba, reduced.rgba);
        assert!(!runtime.needs_frame());

        let dark = Theme::dark().colors;
        let mut large = Runtime::new(Loading {
            theme: Theme::dark(),
            size: 240.,
            motion: LoadingIndicatorMotion::default(),
        })
        .unwrap();
        let large_first = frame(&mut large, 0., 516, 264);
        assert_eq!(bounds(&large, "contained-shape").width, 190.);
        let large_contained = bounds(&large, "contained");
        let large_ink = mask(&large_first, large_contained, dark.on_primary_container);
        assert!(large_ink.len() > 3_000);
        assert!(large_ink
            .iter()
            .all(|&(x, y)| (25..215).contains(&x) && (25..215).contains(&y)));
        assert_eq!(sample(&large_first, 384, 16), rgba(dark.primary_container));
        save(&first, "light-first");
        save(&middle, "light-middle");
        save(&large_first, "dark-large");
    });
}
