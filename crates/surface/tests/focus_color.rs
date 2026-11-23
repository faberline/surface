use surface::{
    Color, Corners, DisplayList, LayoutEngine, Length, MicroDom, Node, PaintOp, Role, Style,
    TextMeasurer,
};

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (100., 32.)
    }
}

#[test]
fn focus_color_changes_only_the_outline_for_square_and_shaped_controls() {
    let foreground = Color(20, 30, 40, 255);
    let indicator = Color(170, 10, 10, 255);
    let style = Style::default().foreground(foreground);
    for (style, edges) in [(style.clone(), 4), (style.corners(Corners::top(4.)), 1)] {
        let mut dom = MicroDom::new(Node::new(Role::Window, "Window"));
        let field = dom
            .append(
                dom.root(),
                Node::new(Role::TextInput, "Field")
                    .text("Draft")
                    .styled(style)
                    .focus_color(indicator),
            )
            .unwrap();
        let computed = LayoutEngine::default()
            .compute(&dom, 200., 100., &mut Metrics)
            .unwrap();
        let unfocused = DisplayList::compile(&dom, &computed, None);
        let focused = DisplayList::compile(&dom, &computed, Some(field));
        assert_eq!(focused.ops.len(), unfocused.ops.len() + edges);
        let indicator_ops = focused.ops.iter().filter(|op| {
            matches!(op, PaintOp::Quad { color, .. } | PaintOp::ShapeStroke { color, .. } if *color == indicator)
        });
        assert_eq!(indicator_ops.count(), edges);
        let text = focused.ops.last().unwrap();
        assert!(
            matches!(text, PaintOp::EditableText { text, color, .. } if text == "Draft" && *color == foreground)
        );
        assert_eq!(unfocused.ops.last(), Some(text));
    }
}

#[test]
fn transparent_foreground_keeps_legacy_focus_ops_but_transparent_override_hides_them() {
    let style = Style::default().foreground(Color::TRANSPARENT);
    for (style, edges) in [(style.clone(), 4), (style.corners(Corners::top(4.)), 1)] {
        let authored = Node::new(Role::Button, "Destination").styled(style);
        let legacy = MicroDom::new(authored.clone());
        let hidden = MicroDom::new(authored.focus_color(Color::TRANSPARENT));
        let mut engine = LayoutEngine::default();
        let layout = engine.compute(&legacy, 200., 100., &mut Metrics).unwrap();
        let focused = DisplayList::compile(&legacy, &layout, Some(legacy.root()));
        assert_eq!(focused.ops.len(), edges);
        assert!(focused.ops.iter().all(|op| matches!(
            op,
            PaintOp::Quad { color, .. } | PaintOp::ShapeStroke { color, .. }
                if *color == Color::TRANSPARENT
        )));
        let layout = engine.compute(&hidden, 200., 100., &mut Metrics).unwrap();
        assert!(DisplayList::compile(&hidden, &layout, Some(hidden.root()))
            .ops
            .is_empty());
    }
}

#[test]
fn custom_outline_is_an_inside_stroke_and_keeps_the_ancestor_clip() {
    let color = Color::rgb(170, 10, 10);
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "Window")
            .styled(Style::column().height(Length::Px(32.)).clip(true)),
    );
    let style = Style {
        min_height: 56.,
        ..Style::default().corners(Corners::top(12.))
    };
    let field = dom
        .append(
            dom.root(),
            Node::new(Role::Button, "Action")
                .styled(style)
                .focus_color(color)
                .focus_outline_width(3.),
        )
        .unwrap();
    let layout = LayoutEngine::default()
        .compute(&dom, 200., 100., &mut Metrics)
        .unwrap();
    let geometry = &layout.nodes[field.index()];
    assert_eq!(geometry.bounds.height, 56.);
    assert_eq!(geometry.clip.height, 32.);
    assert!(DisplayList::compile(&dom, &layout, None).ops.is_empty());
    assert_eq!(
        DisplayList::compile(&dom, &layout, Some(field)).ops,
        [PaintOp::ShapeStroke {
            rect: geometry.bounds,
            clip: geometry.clip,
            corners: Corners::top(12.),
            width: 3.,
            color,
        }]
    );
}

#[test]
fn absent_or_invalid_outline_width_keeps_legacy_focus_paint_and_json() {
    let legacy = Node::new(Role::Button, "Action").styled(Style {
        min_height: 56.,
        ..Style::default().radius(8.)
    });
    let json = serde_json::to_value(&legacy).unwrap();
    assert!(json.get("focus_outline_width").is_none());
    let restored: Node = serde_json::from_value(json.clone()).unwrap();
    for node in [
        restored,
        legacy.clone().focus_outline_width(f32::NAN),
        legacy.clone().focus_outline_width(-3.),
        legacy.clone().focus_outline_width(0.),
    ] {
        assert_eq!(serde_json::to_value(&node).unwrap(), json);
        let dom = MicroDom::new(node);
        let layout = LayoutEngine::default()
            .compute(&dom, 200., 100., &mut Metrics)
            .unwrap();
        let focused = DisplayList::compile(&dom, &layout, Some(dom.root()));
        assert_eq!(focused.ops.len(), 4);
        assert!(focused
            .ops
            .iter()
            .all(|op| matches!(op, PaintOp::Quad { radius, .. } if *radius == 0.)));
    }
    let json = serde_json::to_value(legacy.focus_outline_width(3.)).unwrap();
    assert_eq!(json["focus_outline_width"], 3.);
    let dom = MicroDom::new(serde_json::from_value(json).unwrap());
    let layout = LayoutEngine::default()
        .compute(&dom, 200., 100., &mut Metrics)
        .unwrap();
    assert!(matches!(
        DisplayList::compile(&dom, &layout, Some(dom.root())).ops.as_slice(),
        [PaintOp::Stroke { radius, width, .. }] if *radius == 8. && *width == 3.
    ));
}
