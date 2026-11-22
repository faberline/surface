use surface::{
    Align, Color, LayoutEngine, LayoutError, Length, MeasuredGroupLayout, MicroDom, Node, NodeId,
    Rect, Role, Style, TextMeasurer,
};

struct Fonts;

impl TextMeasurer for Fonts {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.5, size * 1.5)
    }
}

#[derive(Debug, Clone, Copy)]
enum GroupLayout {
    Pair,
    InvalidSize,
    InvalidRect,
    MissingRect,
}

impl MeasuredGroupLayout for GroupLayout {
    fn measure(
        &self,
        children: &[&Node],
        available_width: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        if matches!(self, Self::InvalidSize) {
            return (f32::NAN, 32.);
        }
        let label = children[1];
        let (text_width, _) = fonts.measure_text(
            label.text_value().unwrap(),
            &label.style().text_style(),
            available_width,
        );
        let width = text_width + f32::from(label.style().foreground.0);
        (width.min(available_width.unwrap_or(width)), 32.)
    }

    fn place(&self, _: &[&Node], (width, _): (f32, f32), _: &mut dyn TextMeasurer) -> Vec<Rect> {
        let icon = Rect::new(-4., 2., 12., 12.);
        let label = Rect::new(12., 3., width, 16.);
        match self {
            Self::InvalidRect => vec![icon, Rect::new(0., 0., 12., -1.)],
            Self::MissingRect => vec![icon],
            _ => vec![icon, label],
        }
    }
}

fn scene(layout: Option<GroupLayout>) -> (MicroDom, [NodeId; 4]) {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "Window")
            .styled(Style::row().align(Align::Start).padding(10.).gap(8.)),
    );
    let owner = Node::new(Role::Button, "Destination")
        .id("destination")
        .styled(Style::row().clip(true));
    let owner = match layout {
        Some(layout) => owner.measured_group(layout),
        None => owner,
    };
    let group = dom.append(dom.root(), owner).unwrap();
    let icon = dom
        .append(
            group,
            Node::new(Role::Group, "").styled(
                Style::row()
                    .width(Length::Px(90.))
                    .height(Length::Px(90.))
                    .background(Color::rgb(10, 20, 30)),
            ),
        )
        .unwrap();
    let label = dom
        .append(
            group,
            Node::new(Role::Text, "").text("abcdef").styled(
                Style::default()
                    .font_size(10.)
                    .foreground(Color::rgb(20, 0, 0)),
            ),
        )
        .unwrap();
    let sibling = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "")
                .styled(Style::row().width(Length::Px(30.)).height(Length::Px(20.))),
        )
        .unwrap();
    (dom, [group, icon, label, sibling])
}

#[test]
fn measured_group_allocates_as_one_item_and_clips_its_placed_children() {
    let (dom, [group, icon, label, sibling]) = scene(Some(GroupLayout::Pair));
    let result = LayoutEngine::default()
        .compute(&dom, 180., 120., &mut Fonts)
        .unwrap();
    let clip = Rect::new(10., 10., 50., 32.);
    assert_eq!(result.nodes[group.index()].bounds, clip);
    assert_eq!(result.nodes[sibling.index()].bounds.x, 68.);
    assert_eq!(
        result.nodes[icon.index()].bounds,
        Rect::new(6., 12., 12., 12.)
    );
    assert_eq!(
        result.nodes[label.index()].bounds,
        Rect::new(22., 13., 50., 16.)
    );
    assert_eq!(result.nodes[icon.index()].clip, clip);
    assert_eq!(result.nodes[label.index()].clip, clip);
    assert_eq!(
        result.paint_order,
        vec![dom.root(), group, icon, label, sibling]
    );
    assert_eq!(result.hit_test(&dom, 8., 15.), None);
    assert_eq!(result.hit_test(&dom, 15., 15.), Some(group));
}

#[test]
fn retained_layout_reads_new_child_properties_and_can_switch_to_flow() {
    let (mut dom, [group, _, label, _]) = scene(Some(GroupLayout::Pair));
    let mut engine = LayoutEngine::default();
    engine.compute(&dom, 180., 120., &mut Fonts).unwrap();
    let style = dom.get(label).unwrap().style().clone();
    dom.set_style(label, style.foreground(Color::rgb(30, 0, 0)))
        .unwrap();
    let next = engine.compute(&dom, 180., 120., &mut Fonts).unwrap();
    assert_eq!(next.nodes[group.index()].bounds.width, 60.);

    let (flow, [group, icon, _, _]) = scene(None);
    let next = engine.compute(&flow, 180., 120., &mut Fonts).unwrap();
    assert_eq!(next.nodes[group.index()].bounds.height, 90.);
    assert_eq!(
        next.nodes[icon.index()].bounds,
        Rect::new(10., 10., 90., 90.)
    );
}

#[test]
fn invalid_group_measurement_placement_or_child_shape_returns_a_layout_error() {
    let mut engine = LayoutEngine::default();
    for mode in [
        GroupLayout::InvalidSize,
        GroupLayout::InvalidRect,
        GroupLayout::MissingRect,
    ] {
        let (dom, [group, _, _, _]) = scene(Some(mode));
        assert!(matches!(
            engine.compute(&dom, 180., 120., &mut Fonts),
            Err(LayoutError::InvalidStyle(index)) if index == group.index()
        ));
    }
    let (mut dom, [group, icon, _, _]) = scene(Some(GroupLayout::Pair));
    engine.compute(&dom, 180., 120., &mut Fonts).unwrap();
    dom.append(icon, Node::new(Role::Group, "")).unwrap();
    assert!(matches!(
        engine.compute(&dom, 180., 120., &mut Fonts),
        Err(LayoutError::InvalidStyle(index)) if index == group.index()
    ));
}

#[test]
fn measured_group_has_clone_identity_and_preserves_the_wire_shape() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<Node>();

    let plain = Node::new(Role::Group, "");
    let measured = plain.clone().measured_group(GroupLayout::Pair);
    assert_eq!(measured.clone(), measured);
    assert_ne!(plain.clone().measured_group(GroupLayout::Pair), measured);
    let json = serde_json::to_value(measured).unwrap();
    assert_eq!(json, serde_json::to_value(&plain).unwrap());
    assert_eq!(serde_json::from_value::<Node>(json).unwrap(), plain);
}
