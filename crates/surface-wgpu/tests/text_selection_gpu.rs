use std::collections::BTreeMap;

use surface::{
    Color, DisplayList, ExtendedDisplayList, ExtendedPaintOp, Insets, InteractionState,
    LayoutEngine, Length, MicroDom, Node, PaintOp, Rect, Role, Style, TextDecoration,
    TextSelectionColors,
};
use surface_wgpu::{Capture, Renderer};

const PAPER: Color = Color::rgb(22, 31, 43);
const INK: Color = Color(255, 214, 106, 192);
const COLORS: TextSelectionColors = TextSelectionColors {
    background: Color(0, 230, 255, 204),
    foreground: Color(18, 30, 50, 198),
};

fn fill(viewport: Rect, color: Color) -> PaintOp {
    PaintOp::Quad {
        rect: viewport.inset(-2.),
        clip: viewport,
        radius: 0.,
        color,
    }
}

fn fixture(renderer: &mut Renderer, styled: bool) -> (ExtendedDisplayList, PaintOp, Vec<Rect>) {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "").styled(
            Style::column()
                .width(Length::Px(68.))
                .height(Length::Px(48.))
                .clip(true),
        ),
    );
    let style = Style::column()
        .width(Length::Px(96.))
        .height(Length::Px(61.))
        .padding(3.)
        .font_size(26.)
        .foreground(INK)
        .absolute(Insets {
            left: Length::Px(6.),
            top: Length::Px(5.),
            ..Insets::default()
        });
    let style = if styled {
        style.font_weight(600)
    } else {
        style
    };
    let field = dom
        .append(
            dom.root(),
            Node::new(Role::TextInput, "")
                .text("WMMWMW 比較\nMWMW 差分")
                .styled(style),
        )
        .unwrap();
    let layout = LayoutEngine::default()
        .compute(&dom, 96., 64., &mut renderer.text)
        .unwrap();
    let shape = &layout.nodes[field.index()];
    let clip = shape.bounds.inset(3.).intersect(shape.clip);
    assert!(clip.width < shape.bounds.inset(3.).width);
    let (x, y) = (clip.x, clip.y);
    let cuts = vec![
        Rect::new(x - 3.17, y + 4.35, 27.1, 20.),
        Rect::new(x + 19.3, y + 8.1, 15.2, 18.8),
        Rect::new(x + clip.width - 9.3, y + 8.8, 14., 9.4),
    ];
    let decoration = TextDecoration {
        selection: cuts.clone(),
        scroll: (3.25, 1.5),
        multiline: styled,
        ..Default::default()
    };
    let (result, _) = ExtendedDisplayList::compile_attributed_with_selection_colors(
        &dom,
        &layout,
        None,
        &BTreeMap::from([(field, decoration)]),
        InteractionState::default(),
        COLORS,
    );
    let [ExtendedPaintOp::TextSelection(selection)] = result.ops() else {
        panic!("one paired editable run");
    };
    let run = selection.run().clone();
    let viewport = result.viewport();
    let mut ops = vec![ExtendedPaintOp::Legacy(fill(viewport, PAPER))];
    ops.extend(result.into_ops());
    (ExtendedDisplayList::new(viewport, ops), run, cuts)
}

fn pixel(frame: &Capture, x: u32, y: u32) -> &[u8] {
    let p = (y * frame.width + x) as usize * 4;
    &frame.rgba[p..p + 4]
}

#[test]
#[ignore = "requires a real GPU adapter"]
fn paired_selection_matches_single_draw_references_at_fractional_clip_edges() {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = Renderer::request(&instance, None)
            .await
            .expect("GPU adapter");
        eprintln!("GPU evidence: {:?}", adapter.get_info());
        for format in [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ] {
            let mut renderer = Renderer::new(&adapter, format).await.unwrap();
            for styled in [false, true] {
                let (selection, ordinary_run, cuts) = fixture(&mut renderer, styled);
                let viewport = selection.viewport();
                let clip = ordinary_run.clip();
                let mut selected_run = ordinary_run.clone();
                match &mut selected_run {
                    PaintOp::EditableText { color, .. } | PaintOp::StyledText { color, .. } => {
                        *color = COLORS.foreground
                    }
                    _ => panic!("editable text"),
                }
                let normal = DisplayList {
                    viewport,
                    ops: vec![fill(viewport, PAPER), ordinary_run],
                };
                let highlight = DisplayList {
                    viewport,
                    ops: vec![
                        fill(viewport, PAPER),
                        fill(viewport, COLORS.background),
                        selected_run,
                    ],
                };
                for scale in [1.25, 1.5] {
                    let width = (viewport.width * scale) as u32;
                    let height = (viewport.height * scale) as u32;
                    let actual = renderer
                        .capture_extended(&selection, width, height, scale)
                        .unwrap();
                    let normal = renderer.capture(&normal, width, height, scale).unwrap();
                    let highlight = renderer.capture(&highlight, width, height, scale).unwrap();
                    let mut drawn = [0; 2];

                    for y in 0..height {
                        for x in 0..width {
                            let (px, py) = (x as f32, y as f32);
                            let clip_contains = px >= (clip.x * scale).floor()
                                && px < ((clip.x + clip.width) * scale).ceil()
                                && py >= (clip.y * scale).floor()
                                && py < ((clip.y + clip.height) * scale).ceil();
                            let selected = clip_contains
                                && cuts.iter().any(|cut| {
                                    cut.contains((px + 0.5) / scale, (py + 0.5) / scale)
                                });
                            let reference = if selected { &highlight } else { &normal };
                            let expected = pixel(reference, x, y);
                            let found = pixel(&actual, x, y);
                            assert_eq!(
                                found, expected,
                                "different single-draw coverage at {x},{y} {format:?} scale={scale} styled={styled}",
                            );
                            if expected != pixel(reference, 0, 0) {
                                drawn[usize::from(selected)] += 1;
                            }
                        }
                    }
                    assert!(
                        drawn[0] > 10 && drawn[1] > 10,
                        "both inks must actually paint"
                    );
                }
            }
        }
    });
}
