use surface::{Color, ExtendedPaintOp, Insets, Length, Node, PaintTransform, Rect, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, Application, Event, Runtime, View};
use surface_material::{
    shape::{self, LayoutDirection},
    FabColor, FabSize, FullScreenDialogAnchor, FullScreenDialogMotion, FullScreenDialogOptions,
    FullScreenDialogTransition, Theme,
};
use surface_wgpu::{Capture, Renderer, TextSystem};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 400;
const SCALE: f32 = 2.;
const PAGE: Color = Color::rgb(24, 43, 71);
const PROBE: Color = Color::rgb(236, 34, 52);

struct Editor {
    theme: Theme,
    motion: FullScreenDialogMotion,
    anchor: Option<FullScreenDialogAnchor>,
    open: bool,
}

impl Application for Editor {
    type Message = bool;

    fn update(&mut self, open: bool) {
        self.open = open;
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        if frame
            .layout
            .is_some_and(|layout| layout.viewport == viewport())
        {
            self.anchor = FullScreenDialogAnchor::measured(
                &frame,
                "fab",
                shape::LARGE.top_left,
                self.theme.colors.primary_container,
            );
        }
        self.motion
            .advance(&frame, self.open, self.anchor, viewport())
    }

    fn view(&self) -> View<bool> {
        let fab = View::new(
            Node::new(Role::Group, "").styled(
                Style::row()
                    .width(Length::Px(56.))
                    .height(Length::Px(56.))
                    .fixed(Insets {
                        left: Length::Px(24.),
                        bottom: Length::Px(24.),
                        ..Insets::default()
                    }),
            ),
        )
        .children([self.theme.fab(
            "fab",
            "新增行程",
            "+",
            FabSize::Regular,
            FabColor::Primary,
            false,
            true,
        )]);
        let body = View::new(
            Node::new(Role::Group, "").id("probe").styled(
                Style::column()
                    .width(Length::Px(88.))
                    .height(Length::Px(44.))
                    .background(PROBE),
            ),
        );
        let dialog = self.theme.full_screen_dialog_with_motion(
            FullScreenDialogOptions {
                id: "edit",
                title: "編輯行程",
                body_label: "行程",
                close_label: "關閉",
                action_label: "儲存",
                action_disabled: false,
                initial_focus: Some("edit-close"),
                restore_focus_to: Some("fab"),
                viewport: viewport(),
                direction: LayoutDirection::RightToLeft,
            },
            body,
            false,
            false,
            FullScreenDialogTransition {
                open: self.open,
                anchor: self.anchor,
                motion: &self.motion,
            },
        );
        View::new(
            Node::new(Role::Window, "").styled(
                Style::column()
                    .width(Length::Percent(1.))
                    .height(Length::Percent(1.))
                    .background(PAGE),
            ),
        )
        .children(std::iter::once(fab).chain(dialog))
    }
}

fn viewport() -> Rect {
    Rect::new(0., 0., WIDTH as f32, HEIGHT as f32)
}

fn sample(image: &Capture, x: f32, y: f32) -> [u8; 4] {
    let x = (x * SCALE) as u32;
    let y = (y * SCALE) as u32;
    let offset = ((y * image.width + x) * 4) as usize;
    image.rgba[offset..offset + 4].try_into().unwrap()
}

fn save(image: &Capture, name: &str) {
    let Ok(path) = std::env::var("SURFACE_GPU_EVIDENCE") else {
        return;
    };
    let path =
        std::path::Path::new(&path).with_file_name(format!("gpu-full-screen-fab-{name}.png"));
    let mut encoder = png::Encoder::new(
        std::fs::File::create(path).unwrap(),
        image.width,
        image.height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&image.rgba)
        .unwrap();
}

fn bounds(runtime: &Runtime<Editor>, id: &str) -> Rect {
    let node = runtime.dom().by_id(id).unwrap();
    runtime.computed().unwrap().nodes[node.index()].bounds
}

#[test]
#[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
fn gpu_offscreen_full_screen_fab_moves_and_clips_real_dialog_pixels() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None)
            .await
            .expect("native GPU adapter is required");
        eprintln!("GPU Full-screen FAB evidence: {:?}", adapter.get_info());
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .unwrap();
        let mut text = TextSystem::default();
        let mut runtime = Runtime::new(Editor {
            theme: Theme::dark(),
            motion: FullScreenDialogMotion::default(),
            anchor: None,
            open: false,
        })
        .unwrap();
        let mut frame = |runtime: &mut Runtime<Editor>, now| {
            runtime.set_time(now);
            let display = runtime
                .render_extended(WIDTH as f32, HEIGHT as f32, &mut text)
                .unwrap();
            let transform = display.ops().iter().find_map(|op| match op {
                ExtendedPaintOp::BeginTransformGroup { transform } => Some(*transform),
                _ => None,
            });
            let capture = renderer
                .capture_extended(&display, WIDTH * 2, HEIGHT * 2, SCALE)
                .unwrap();
            (capture, transform)
        };
        let (closed, _) = frame(&mut runtime, 0.);
        frame(&mut runtime, 20.);
        let origin = bounds(&runtime, "fab");
        assert_eq!(origin, Rect::new(24., 320., 56., 56.));
        runtime.activate("fab").unwrap();
        let (first, start) = frame(&mut runtime, 100.);
        assert_eq!(start.unwrap().clip, origin);
        assert_eq!(first.rgba, closed.rgba);
        assert_eq!(runtime.focused(), Some("edit-close"));

        let (entering, active) = frame(&mut runtime, 170.);
        let PaintTransform {
            clip,
            destination,
            source,
            opacity,
            radius,
        } = active.unwrap();
        assert!(radius > 3. && clip.width > origin.width && clip.width < viewport().width);
        assert!((0. ..1.).contains(&opacity));
        let background = sample(&closed, 4., 4.);
        assert_eq!(sample(&entering, 4., 4.), background);
        assert_eq!(sample(&entering, clip.x + 0.5, clip.y + 0.5), background);
        let probe = bounds(&runtime, "probe");
        let scale = destination.width / source.width;
        let probe_x = destination.x + (probe.x + probe.width * 0.5 - source.x) * scale;
        let probe_y = destination.y + (probe.y + probe.height * 0.5 - source.y) * scale;
        assert!(clip.contains(probe_x, probe_y));
        let probe_pixel = sample(&entering, probe_x, probe_y);
        let near_probe = sample(
            &entering,
            probe_x - (probe.width * 0.5 + 7.) * scale,
            probe_y,
        );
        assert!(
            u16::from(probe_pixel[0]) > u16::from(near_probe[0]) + 25,
            "{probe_pixel:?} and {near_probe:?}"
        );
        save(&entering, "enter");

        let (open, complete) = frame(&mut runtime, 650.);
        assert!(complete.is_none());
        let probe = bounds(&runtime, "probe");
        assert_eq!(
            sample(
                &open,
                probe.x + probe.width * 0.5,
                probe.y + probe.height * 0.5
            ),
            [PROBE.0, PROBE.1, PROBE.2, 255],
        );
        save(&open, "settled");
        runtime.dispatch(Event::Cancel).unwrap();
        assert_eq!(runtime.focused(), Some("fab"));
        frame(&mut runtime, 700.);
        let (exiting, transition) = frame(&mut runtime, 770.);
        assert!(transition.is_some());
        let accessibility = runtime.accessibility_frame().unwrap();
        assert_eq!(accessibility.modal_root(), None);
        assert!(accessibility
            .node(runtime.dom().by_id("edit").unwrap())
            .is_none());
        assert_eq!(sample(&exiting, 4., 4.), background);
        save(&exiting, "exit");

        runtime.set_reduced_motion(true);
        let (done, transition) = frame(&mut runtime, 780.);
        assert!(transition.is_none());
        assert!(runtime.dom().by_id("edit").is_none());
        assert_eq!(sample(&done, 4., 4.), background);
    });
}
