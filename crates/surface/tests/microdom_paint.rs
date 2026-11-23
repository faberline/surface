use surface::*;

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

fn fixture() -> (MicroDom, NodeId, NodeId) {
    fixture_with_second_background(Color::rgb(103, 80, 164))
}

fn fixture_with_second_background(second_background: Color) -> (MicroDom, NodeId, NodeId) {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "App").styled(Style::column().padding(10.).gap(10.).clip(true)),
    );
    let first = dom
        .append(
            dom.root(),
            Node::new(Role::Button, "One")
                .id("first")
                .text("One")
                .styled(
                    Style::default()
                        .padding(4.)
                        .background(Color::rgb(103, 80, 164))
                        .radius(12.),
                ),
        )
        .unwrap();
    let second = dom
        .append(
            dom.root(),
            Node::new(Role::Button, "Two")
                .id("second")
                .text("Two")
                .styled(
                    Style::default()
                        .padding(4.)
                        .background(second_background)
                        .radius(12.),
                ),
        )
        .unwrap();
    (dom, first, second)
}

fn compile_at(dom: &MicroDom, width: f32, height: f32) -> DisplayList {
    let computed = LayoutEngine::default()
        .compute(dom, width, height, &mut Metrics)
        .unwrap();
    DisplayList::compile(dom, &computed, None)
}

fn compile(dom: &MicroDom) -> DisplayList {
    compile_at(dom, 300., 200.)
}

fn bounds(dom: &MicroDom, id: NodeId) -> Rect {
    LayoutEngine::default()
        .compute(dom, 300., 200., &mut Metrics)
        .unwrap()
        .nodes[id.index()]
    .bounds
}

#[test]
fn identical_trees_give_byte_stable_output() {
    let (a, _, _) = fixture();
    let (b, _, _) = fixture();
    assert_eq!(
        serde_json::to_vec(&compile(&a)).unwrap(),
        serde_json::to_vec(&compile(&b)).unwrap()
    );
}

#[test]
fn ops_follow_paint_order() {
    let (dom, first, second) = fixture();
    let list = compile(&dom);
    let first_bounds = bounds(&dom, first);
    let second_bounds = bounds(&dom, second);
    let first_quad = list
        .ops
        .iter()
        .position(|op| matches!(op, PaintOp::Quad { rect, .. } if *rect == first_bounds))
        .unwrap();
    let first_text = list
        .ops
        .iter()
        .position(|op| matches!(op, PaintOp::Text { text, .. } if text == "One"))
        .unwrap();
    let second_quad = list
        .ops
        .iter()
        .position(|op| matches!(op, PaintOp::Quad { rect, .. } if *rect == second_bounds))
        .unwrap();
    let second_text = list
        .ops
        .iter()
        .position(|op| matches!(op, PaintOp::Text { text, .. } if text == "Two"))
        .unwrap();
    assert!(first_quad < first_text);
    assert!(first_text < second_quad);
    assert!(second_quad < second_text);
}

#[test]
fn interaction_corners_follow_press_focus_and_hover_without_moving_the_face() {
    let style = Style::default()
        .height(Length::Px(56.))
        .corners(Corners::all(4.))
        .background(Color::rgb(10, 20, 30))
        .state_layer(Color::rgb(30, 40, 50))
        .border(1., Color::rgb(50, 60, 70));
    let node = Node::new(Role::Button, "Option")
        .styled(style)
        .focus_outline_width(3.)
        .state_layer_alphas(20, 26, 26)
        .interaction_corners(
            Some(Corners::all(12.)),
            Some(Corners::all(18.)),
            Some(Corners::all(16.)),
        );
    for (disabled, hovered, focused, pressed, radius, layers) in [
        (false, false, false, false, 4., 2),
        (false, true, false, false, 12., 3),
        (false, true, true, false, 16., 4),
        (false, true, true, true, 18., 4),
        (true, true, false, true, 4., 2),
    ] {
        let dom = MicroDom::new(node.clone().disabled(disabled));
        let id = dom.root();
        let layout = LayoutEngine::default()
            .compute(&dom, 200., 100., &mut Metrics)
            .unwrap();
        let painted = DisplayList::compile_with_interaction(
            &dom,
            &layout,
            focused.then_some(id),
            &Default::default(),
            InteractionState {
                hovered: hovered.then_some(id),
                pressed: pressed.then_some(id),
            },
        );
        assert_eq!(painted.ops.len(), layers);
        for op in painted.ops {
            assert_eq!(op.rect(), layout.nodes[id.index()].bounds);
            assert_eq!(op.clip(), layout.nodes[id.index()].clip);
            assert!(matches!(
                op,
                PaintOp::Quad { radius: painted, .. } | PaintOp::Stroke { radius: painted, .. }
                    if painted == radius
            ));
        }
    }
}

#[test]
fn nested_clips_are_carried_on_every_op() {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "App").styled(
            Style::default()
                .width(Length::Px(100.))
                .height(Length::Px(100.))
                .clip(true),
        ),
    );
    let group = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "Group").styled(
                Style::default()
                    .width(Length::Px(40.))
                    .height(Length::Px(40.))
                    .clip(true),
            ),
        )
        .unwrap();
    dom.append(
        group,
        Node::new(Role::Button, "Hi").text("Hi").styled(
            Style::default()
                .width(Length::Px(40.))
                .height(Length::Px(120.))
                .background(Color::rgb(1, 2, 3)),
        ),
    )
    .unwrap();
    let list = compile(&dom);
    let limit = Rect::new(0., 0., 40., 40.);
    assert!(!list.ops.is_empty());
    for op in &list.ops {
        assert_eq!(op.clip().intersect(limit), op.clip());
    }
}

#[test]
fn text_is_a_reference_not_glyphs() {
    let (dom, _, _) = fixture();
    let list = compile(&dom);
    let font_size = Style::default().font_size;
    assert!(list.ops.iter().any(|op| matches!(
        op,
        PaintOp::Text { text, font_size: size, .. } if text == "One" && *size == font_size
    )));
    let json = serde_json::to_string(&list).unwrap();
    assert!(json.contains("\"One\""));
}

#[test]
fn no_gpu_or_browser_types_in_output() {
    let (dom, _, _) = fixture();
    let json = serde_json::to_string(&compile(&dom)).unwrap();
    for banned in ["wgpu", "Wgpu", "web_sys", "HtmlElement"] {
        assert!(!json.contains(banned), "output contains {banned}");
    }
}

#[test]
fn damage_is_empty_for_identical_lists() {
    let (dom, _, _) = fixture();
    assert!(compile(&dom).damage(&compile(&dom)).is_empty());
}

#[test]
fn damage_is_bounded_to_the_changed_node() {
    let (mut dom, first, second) = fixture();
    let before = compile(&dom);
    dom.set_style(
        second,
        Style::default()
            .padding(4.)
            .background(Color::rgb(200, 0, 0))
            .radius(12.),
    )
    .unwrap();
    let after = compile(&dom);
    let damage = after.damage(&before);
    assert!(!damage.is_empty());
    let second_bounds = bounds(&dom, second);
    let first_bounds = bounds(&dom, first);
    for rect in &damage {
        assert_eq!(rect.intersect(second_bounds), *rect);
        let overlap = rect.intersect(first_bounds);
        assert_eq!(overlap.width * overlap.height, 0.);
    }
}

#[test]
fn damage_covers_removed_and_added_ops() {
    let (mut dom, _, _) = fixture();
    let a = compile(&dom);
    let third = dom
        .append(
            dom.root(),
            Node::new(Role::Button, "Three")
                .id("third")
                .text("Three")
                .styled(Style::default().padding(4.)),
        )
        .unwrap();
    let b = compile(&dom);
    let third_bounds = bounds(&dom, third);
    let added = b.damage(&a);
    assert!(!added.is_empty());
    for rect in &added {
        assert_eq!(rect.intersect(third_bounds), *rect);
    }
    let removed = a.damage(&b);
    assert!(!removed.is_empty());
    for rect in &removed {
        assert_eq!(rect.intersect(third_bounds), *rect);
    }
}

#[test]
fn damage_is_the_viewport_when_the_viewport_changes() {
    let (dom, _, _) = fixture();
    let a = compile_at(&dom, 300., 200.);
    let b = compile_at(&dom, 320., 200.);
    assert_eq!(b.damage(&a), vec![b.viewport]);
}

#[test]
fn paint_op_accessors_return_rect_and_clip() {
    let (dom, _, _) = fixture();
    let list = compile(&dom);
    for op in &list.ops {
        match op {
            PaintOp::Quad { rect, clip, .. } => {
                assert_eq!(op.rect(), *rect);
                assert_eq!(op.clip(), *clip);
            }
            _ => assert!(op.rect().width >= 0.),
        }
    }
}
