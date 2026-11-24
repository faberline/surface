use surface::{Color, ExtendedPaintOp, Length, Node, Rect, Role, Style};
use surface_app::{
    AnimationFrame, AnimationUpdate, Application, Event, ModalDismissal, Runtime, View,
};
use surface_material::{
    shape::LayoutDirection, BasicDialogContent, BasicDialogMotion, BasicDialogOptions, ButtonKind,
    Theme,
};
use surface_wgpu::{Capture, Renderer, TextSystem};

const WIDTH: u32 = 560;
const HEIGHT: u32 = 420;
const CHILD: Color = Color::rgb(236, 123, 47);

struct Basic {
    theme: Theme,
    motion: BasicDialogMotion,
    open: bool,
}

impl Application for Basic {
    type Message = bool;

    fn update(&mut self, open: bool) {
        self.open = open;
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        self.motion.advance(&frame, self.open)
    }

    fn view(&self) -> View<bool> {
        let dialog = self.theme.basic_dialog_with_motion(
            BasicDialogOptions {
                id: "basic",
                label: "行程通知",
                heading: Some("行程通知"),
                supporting: Some("提醒時間"),
                icon: None,
                window_size: (WIDTH as f32, HEIGHT as f32),
                preferred_width: 360.,
                direction: LayoutDirection::RightToLeft,
                dismissal: ModalDismissal::EscapeOrOutside,
                alert: false,
                initial_focus: Some("basic-confirm"),
                restore_focus_to: Some("opener"),
            },
            BasicDialogContent {
                extra: vec![View::new(
                    Node::new(Role::Group, "").id("color-card").styled(
                        Style::column()
                            .width(Length::Px(94.))
                            .height(Length::Px(38.))
                            .background(CHILD),
                    ),
                )],
                confirm: ("知道了", false),
                cancel: None,
                on_dismiss: false,
            },
            self.open,
            &self.motion,
        );
        View::new(
            Node::new(Role::Window, "").styled(
                Style::column()
                    .width(Length::Percent(1.))
                    .height(Length::Percent(1.))
                    .padding(16.)
                    .background(self.theme.colors.surface),
            ),
        )
        .children(
            std::iter::once(self.theme.button_with(
                "opener",
                "行程",
                ButtonKind::Text,
                false,
                true,
            ))
            .chain(dialog),
        )
    }
}

fn sample(capture: &Capture, x: u32, y: u32) -> [u8; 4] {
    let start = ((y * capture.width + x) * 4) as usize;
    capture.rgba[start..start + 4].try_into().unwrap()
}

fn blend(base: [u8; 4], ink: Color, opacity: f32) -> [u8; 4] {
    let channel =
        |under, over| (f32::from(under) * (1. - opacity) + f32::from(over) * opacity).round() as u8;
    [
        channel(base[0], ink.0),
        channel(base[1], ink.1),
        channel(base[2], ink.2),
        255,
    ]
}

fn near(actual: [u8; 4], reference: [u8; 4]) {
    assert!(
        actual
            .into_iter()
            .zip(reference)
            .all(|(byte, expected)| byte.abs_diff(expected) <= 2),
        "actual {actual:?}, expected {reference:?}"
    );
}

fn center(runtime: &Runtime<Basic>, id: &str) -> (u32, u32) {
    let node = runtime.dom().by_id(id).unwrap();
    let Rect {
        x,
        y,
        width,
        height,
    } = runtime.computed().unwrap().nodes[node.index()].bounds;
    ((x + width * 0.5) as u32, (y + height * 0.5) as u32)
}

fn save(capture: &Capture, name: &str) {
    let Ok(path) = std::env::var("SURFACE_GPU_EVIDENCE") else {
        return;
    };
    let path = std::path::Path::new(&path).with_file_name(format!("gpu-basic-dialog-{name}.png"));
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
fn gpu_offscreen_basic_dialog_motion_blends_scrim_and_isolated_panel() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None)
            .await
            .expect("native GPU adapter is required");
        eprintln!("GPU Basic motion evidence: {:?}", adapter.get_info());
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .unwrap();
        let mut text = TextSystem::default();
        let theme = Theme::light();
        let scheme = theme.colors;
        let mut runtime = Runtime::new(Basic {
            theme,
            motion: BasicDialogMotion::default(),
            open: false,
        })
        .unwrap();
        let mut frame = |runtime: &mut Runtime<Basic>, now_ms| {
            runtime.set_time(now_ms);
            let display = runtime
                .render_extended(WIDTH as f32, HEIGHT as f32, &mut text)
                .unwrap();
            let alpha = display.ops().iter().find_map(|op| match op {
                ExtendedPaintOp::BeginOpacityGroup { opacity } => Some(*opacity),
                _ => None,
            });
            (
                renderer
                    .capture_extended(&display, WIDTH, HEIGHT, 1.)
                    .unwrap(),
                alpha,
            )
        };
        let (closed, _) = frame(&mut runtime, 0.);
        runtime.activate("opener").unwrap();
        let (first, start) = frame(&mut runtime, 60.);
        assert_eq!(start, Some(0.));
        assert_eq!(runtime.focused(), Some("basic-confirm"));
        assert_eq!(first.rgba, closed.rgba);
        let (opening, alpha) = frame(&mut runtime, 100.);
        let entering = alpha.unwrap();
        let (x, y) = center(&runtime, "color-card");
        let outer = sample(&closed, 2, 2);
        let verify_pixels = |capture: &Capture, progress: f32| {
            let scrim = (82. * progress).round() / 255.;
            let background = blend(outer, scheme.scrim, scrim);
            near(sample(capture, 2, 2), background);
            near(sample(capture, x, y), blend(background, CHILD, progress));
        };
        verify_pixels(&opening, entering);
        save(&opening, "enter");
        let (settled, alpha) = frame(&mut runtime, 500.);
        assert_eq!(alpha, None);
        verify_pixels(&settled, 1.);
        save(&settled, "settled");

        runtime.dispatch(Event::Cancel).unwrap();
        assert_eq!(runtime.focused(), Some("opener"));
        assert!(runtime.activate("basic-confirm").is_err());
        frame(&mut runtime, 600.);
        let (exiting, alpha) = frame(&mut runtime, 640.);
        let leaving = alpha.unwrap();
        assert!((0. ..1.).contains(&leaving));
        verify_pixels(&exiting, leaving);
        let retired = runtime.dom().by_id("basic").unwrap();
        assert!(runtime
            .accessibility_frame()
            .unwrap()
            .node(retired)
            .is_none());
        save(&exiting, "exit");

        runtime.set_reduced_motion(true);
        let (done, alpha) = frame(&mut runtime, 641.);
        assert_eq!(alpha, None);
        assert!(runtime.dom().by_id("basic").is_none());
        assert!(!runtime.needs_frame());
        near(sample(&done, x, y), outer);
        near(sample(&done, 2, 2), outer);
    });
}
