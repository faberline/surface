use surface::{
    Color, Corners, DisplayList, LayoutEngine, MicroDom, Node, PaintOp, Role, Style, TextMeasurer,
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
