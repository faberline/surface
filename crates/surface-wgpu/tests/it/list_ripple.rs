use surface::{Align, Color, Corners, Length, Node, RasterImage, Rect, Role, Style};
use surface_app::{Application, Event, Runtime, View};
use surface_wgpu::{Capture, Renderer, TextSystem};

const WIDTH: u32 = 216;
const HEIGHT: u32 = 88;
const BACKDROP: Color = Color(28, 33, 42, 255);
const FACE: Color = Color(230, 226, 220, 255);
const PHOTO: Color = Color(200, 120, 50, 255);
const MARK: Color = Color(22, 150, 100, 255);
const FOCUS: Color = Color(36, 112, 204, 255);

#[derive(Default)]
struct MediaRow {
    activations: usize,
}

impl Application for MediaRow {
    type Message = ();

    fn update(&mut self, (): ()) {
        self.activations += 1;
    }

    fn view(&self) -> View<()> {
        let bytes: &[u8] = &[PHOTO.0, PHOTO.1, PHOTO.2, PHOTO.3];
        let image = View::new(
            Node::new(Role::Group, "")
                .id("photo")
                .styled(
                    Style::column()
                        .width(Length::Px(68.))
                        .height(Length::Px(48.)),
                )
                .raster_image(RasterImage::new(1, 1, bytes).unwrap()),
        )
        .without_interaction(None);
        let marker = View::new(
            Node::new(Role::Group, "").id("mark").styled(
                Style::column()
                    .width(Length::Px(18.))
                    .height(Length::Px(18.))
                    .background(MARK),
            ),
        )
        .without_interaction(None);
        let row = View::new(
            Node::new(Role::Button, "Open photo")
                .id("media")
                .styled(
                    Style::row()
                        .width(Length::Px(196.))
                        .height(Length::Px(64.))
                        .padding(8.)
                        .gap(12.)
                        .align(Align::Center)
                        .background(FACE)
                        .corners(Corners::new(16., 4., 16., 4.))
                        .state_layer(Color::rgb(32, 32, 32)),
                )
                .focus_color(FOCUS)
                .focus_outline_width(3.)
                .state_layer_alphas(20, 26, 26)
                .press_ripple(Some("photo".to_owned())),
        )
        .on_press(())
        .children([image, marker]);
        let scroller = View::new(
            Node::new(Role::Group, "Clipped list").id("clip").styled(
                Style::column()
                    .width(Length::Px(196.))
                    .height(Length::Px(50.))
                    .clip(true),
            ),
        )
        .children([row]);
        View::new(
            Node::new(Role::Window, "").styled(
                Style::column()
                    .width(Length::Px(WIDTH as f32))
                    .height(Length::Px(HEIGHT as f32))
                    .padding(10.)
                    .background(BACKDROP),
            ),
        )
        .children([scroller])
    }
}

fn bounds(runtime: &Runtime<MediaRow>, name: &str) -> Rect {
    let id = runtime.dom().by_id(name).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn sample(capture: &Capture, (x, y): (f32, f32)) -> [u8; 4] {
    let offset = (y as usize * capture.width as usize + x as usize) * 4;
    capture.rgba[offset..offset + 4].try_into().unwrap()
}

fn rgba(color: Color) -> [u8; 4] {
    [color.0, color.1, color.2, color.3]
}

#[test]
#[ignore = "requires a native GPU; run explicitly, never skip a missing adapter"]
fn gpu_offscreen_list_ripple_keeps_photo_focus_and_clip_in_order() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = Renderer::request(&instance, None)
            .await
            .expect("native GPU adapter is required");
        eprintln!("GPU evidence: {:?}", adapter.get_info());
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .unwrap();
        let mut runtime = Runtime::new(MediaRow::default()).unwrap();
        let mut text = TextSystem::default();
        let mut frame = |runtime: &mut Runtime<MediaRow>, milliseconds| {
            runtime.set_time(milliseconds);
            let painter = runtime
                .render_extended(WIDTH as f32, HEIGHT as f32, &mut text)
                .unwrap();
            renderer
                .capture_extended(&painter, WIDTH, HEIGHT, 1.)
                .unwrap()
        };

        frame(&mut runtime, 0.);
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        let focused = frame(&mut runtime, 0.);
        assert_eq!(runtime.focused(), Some("media"));
        let row = bounds(&runtime, "media");
        let photo = bounds(&runtime, "photo");
        let mark = bounds(&runtime, "mark");
        let origin = (photo.x + 18., photo.y + 18.);
        let far = (row.x + row.width - 20., origin.1);
        let focus = (row.x + 1., row.y + 32.);
        let outside_arc = (row.x + 1., row.y + 1.);
        let outside_clip = (origin.0, bounds(&runtime, "clip").y + 54.);
        let mark = (mark.x + 8., mark.y + 8.);
        assert_eq!(sample(&focused, origin), rgba(PHOTO));
        assert_eq!(sample(&focused, focus), rgba(FOCUS));

        let down = Event::PointerDown {
            x: origin.0,
            y: origin.1,
        };
        let up = Event::PointerUp {
            x: origin.0,
            y: origin.1,
        };
        runtime.dispatch(down.clone()).unwrap();
        let first = frame(&mut runtime, 0.);
        let early = frame(&mut runtime, 40.);
        assert_eq!(sample(&first, origin), rgba(PHOTO));
        assert_eq!(sample(&early, far), sample(&first, far));
        let early_photo = sample(&early, origin);
        assert!(early_photo[0] < PHOTO.0);
        assert!(runtime.needs_frame());
        assert_eq!(runtime.app().activations, 0);

        let full = frame(&mut runtime, 225.);
        let full_photo = sample(&full, origin);
        assert!(
            (i32::from(full_photo[0]) - 183).abs() <= 2,
            "{full_photo:?}"
        );
        assert!(full_photo[0] < early_photo[0]);
        assert!(sample(&full, far)[0] < sample(&early, far)[0]);
        for capture in [&first, &early, &full] {
            assert_eq!(sample(capture, mark), rgba(MARK));
            assert_eq!(sample(capture, focus), rgba(FOCUS));
            assert_eq!(sample(capture, outside_arc), rgba(BACKDROP));
            assert_eq!(sample(capture, outside_clip), rgba(BACKDROP));
        }
        assert!(!runtime.needs_frame());

        runtime.dispatch(up).unwrap();
        let released = frame(&mut runtime, 225.);
        let fade = frame(&mut runtime, 300.);
        let settled = frame(&mut runtime, 375.);
        assert_eq!(sample(&released, origin), full_photo);
        let fading_photo = sample(&fade, origin);
        assert!(full_photo[0] < fading_photo[0] && fading_photo[0] < PHOTO.0);
        assert_eq!(sample(&settled, origin), rgba(PHOTO));
        assert_eq!(runtime.app().activations, 1);
        assert!(!runtime.needs_frame());

        runtime.set_reduced_motion(true);
        runtime.dispatch(down).unwrap();
        let reduced = frame(&mut runtime, 380.);
        assert_eq!(sample(&reduced, origin), rgba(PHOTO));
        assert_eq!(sample(&reduced, far), sample(&full, far));
        assert!(!runtime.needs_frame());
        runtime.cancel_pointer().unwrap();
        eprintln!(
            "Ripple photo RGB at 0/40/225/300/375 ms: {:?} {early_photo:?} {full_photo:?} {fading_photo:?} {:?}",
            sample(&first, origin),
            sample(&settled, origin)
        );

        if let Ok(path) = std::env::var("SURFACE_GPU_EVIDENCE") {
            for (name, capture) in [
                ("early", &early),
                ("full", &full),
                ("fade", &fade),
                ("reduced", &reduced),
            ] {
                let path = std::path::Path::new(&path)
                    .with_file_name(format!("gpu-list-ripple-{name}.png"));
                let mut encoder =
                    png::Encoder::new(std::fs::File::create(path).unwrap(), WIDTH, HEIGHT);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&capture.rgba)
                    .unwrap();
            }
        }
    });
}
