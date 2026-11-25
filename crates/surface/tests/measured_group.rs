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

#[derive(Debug)]
struct NestedLayout {
    permits_nested: bool,
}

impl MeasuredGroupLayout for NestedLayout {
    fn places_measured_groups(&self) -> bool {
        self.permits_nested
    }

    fn measure(&self, _: &[&Node], _: Option<f32>, _: &mut dyn TextMeasurer) -> (f32, f32) {
        (80., 48.)
    }

    fn place(&self, _: &[&Node], _: (f32, f32), _: &mut dyn TextMeasurer) -> Vec<Rect> {
        vec![Rect::new(12., 4., 40., 24.), Rect::new(6., 8., 36., 8.)]
    }
}

fn nested_scene(
    allow: bool,
    child_layout: Option<GroupLayout>,
    child_position: surface::Position,
) -> (MicroDom, [NodeId; 6]) {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "Window")
            .styled(Style::row().align(Align::Start).padding(10.).gap(8.)),
    );
    let outer = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "Measured owner")
                .styled(Style::row().clip(true))
                .measured_group(NestedLayout {
                    permits_nested: allow,
                }),
        )
        .unwrap();
    let mut child_style = Style::row().clip(true);
    child_style.position = child_position;
    child_style.z_index = 1;
    let child = Node::new(Role::Button, "Measured child")
        .id("measured-child")
        .styled(child_style)
        .slide_horizontally(0.25);
    let child = if let Some(layout) = child_layout {
        child.measured_group(layout)
    } else {
        child
    };
    let inner = dom.append(outer, child).unwrap();
    let art = dom.append(inner, Node::new(Role::Group, "")).unwrap();
    let label = dom
        .append(inner, Node::new(Role::Text, "").text("value"))
        .unwrap();
    let backdrop = dom.append(outer, Node::new(Role::Group, "")).unwrap();
    let neighbor = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "")
                .styled(Style::row().width(Length::Px(30.)).height(Length::Px(20.))),
        )
        .unwrap();
    (dom, [outer, inner, art, label, backdrop, neighbor])
}

#[test]
fn opted_in_nested_measured_child_uses_placed_size_transform_clip_and_paint_order() {
    let (dom, [outer, inner, art, label, backdrop, neighbor]) =
        nested_scene(true, Some(GroupLayout::Pair), surface::Position::Flow);
    let computed = LayoutEngine::default()
        .compute(&dom, 180., 120., &mut Fonts)
        .unwrap();
    let bounds = |id: NodeId| computed.nodes[id.index()].bounds;
    assert_eq!(bounds(outer), Rect::new(10., 10., 80., 48.));
    assert_eq!(bounds(inner), Rect::new(32., 14., 40., 24.));
    assert_eq!(bounds(art), Rect::new(28., 16., 12., 12.));
    assert_eq!(bounds(label), Rect::new(44., 17., 40., 16.));
    assert_eq!(bounds(backdrop), Rect::new(16., 18., 36., 8.));
    assert_eq!(bounds(neighbor).x, 98.);
    assert_eq!(computed.nodes[art.index()].clip, bounds(inner));
    assert_eq!(computed.nodes[label.index()].clip, bounds(inner));
    assert_eq!(
        computed.paint_order,
        vec![dom.root(), outer, backdrop, inner, art, label, neighbor]
    );
    assert_eq!(computed.hit_test(&dom, 29., 19.), None);
    assert_eq!(computed.hit_test(&dom, 35., 19.), Some(inner));
}

#[test]
fn nested_opt_in_preserves_legacy_rejection_and_checks_each_level() {
    let mut engine = LayoutEngine::default();
    for (allow, child, position) in [
        (false, Some(GroupLayout::Pair), surface::Position::Flow),
        (true, None, surface::Position::Flow),
        (true, Some(GroupLayout::Pair), surface::Position::Absolute),
        (true, Some(GroupLayout::Pair), surface::Position::Fixed),
    ] {
        let (dom, [outer, ..]) = nested_scene(allow, child, position);
        assert!(matches!(
            engine.compute(&dom, 180., 120., &mut Fonts),
            Err(LayoutError::InvalidStyle(index)) if index == outer.index()
        ));
    }

    let (dom, [_, inner, ..]) = nested_scene(
        true,
        Some(GroupLayout::InvalidRect),
        surface::Position::Flow,
    );
    assert!(matches!(
        engine.compute(&dom, 180., 120., &mut Fonts),
        Err(LayoutError::InvalidStyle(index)) if index == inner.index()
    ));
    let (mut dom, [_, inner, art, ..]) =
        nested_scene(true, Some(GroupLayout::Pair), surface::Position::Flow);
    dom.append(art, Node::new(Role::Group, "")).unwrap();
    assert!(matches!(
        engine.compute(&dom, 180., 120., &mut Fonts),
        Err(LayoutError::InvalidStyle(index)) if index == inner.index()
    ));
}
