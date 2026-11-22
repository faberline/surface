use std::collections::BTreeMap;
use surface::{
    Color, DisplayList, Insets, InteractionState, LayoutEngine, Length, MicroDom, Node, NodeId,
    Rect, Role, Style, TextMeasurer,
};

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn fixed(bounds: Rect, z_index: i32, color: Color) -> Style {
    let mut style = Style::default()
        .width(Length::Px(bounds.width))
        .height(Length::Px(bounds.height))
        .fixed(Insets {
            left: Length::Px(bounds.x),
            top: Length::Px(bounds.y),
            ..Insets::default()
        })
        .background(color);
    style.z_index = z_index;
    style
}

fn add(dom: &mut MicroDom, parent: NodeId, role: Role, style: Style) -> NodeId {
    dom.append(parent, Node::new(role, "").styled(style))
        .unwrap()
}

#[test]
fn owned_fixed_popups_paint_after_their_panel_and_all_background_layers() {
    let mut dom = MicroDom::new(Node::new(Role::Window, "App"));
    let root = dom.root();
    let viewport = Rect::new(0., 0., 240., 180.);
    let ink = Color::rgb(81, 120, 143);
    let low = add(&mut dom, root, Role::Button, fixed(viewport, i32::MIN, ink));
    let backdrop = add(
        &mut dom,
        root,
        Role::Group,
        fixed(viewport, 50, Color(0, 0, 0, 80)),
    );
    let panel = add(
        &mut dom,
        backdrop,
        Role::Dialog,
        fixed(Rect::new(40., 30., 120., 80.), 50, ink).clip(true),
    );
    let body = add(
        &mut dom,
        panel,
        Role::Button,
        Style::default().height(Length::Px(80.)).background(ink),
    );
    let slightly_outside = Rect::new(140., 40., 48., 40.);
    let first = add(
        &mut dom,
        panel,
        Role::Button,
        fixed(slightly_outside, i32::MIN, ink),
    );
    let second = add(
        &mut dom,
        panel,
        Role::Button,
        fixed(slightly_outside, i32::MIN, ink),
    );
    let higher = add(
        &mut dom,
        panel,
        Role::Button,
        fixed(Rect::new(145., 50., 8., 8.), 85, ink),
    );
    // A sibling of the panel is outside the modal, even if it is a child
    // of the explicitly registered backdrop.
    let equal = add(&mut dom, backdrop, Role::Button, fixed(viewport, 50, ink));
    let high = add(&mut dom, root, Role::Button, fixed(viewport, i32::MAX, ink));

    let mut layout = LayoutEngine::default()
        .compute(&dom, viewport.width, viewport.height, &mut Metrics)
        .unwrap();
    let legacy_order = dom.paint_order();
    assert_eq!(layout.hit_test(&dom, 60., 50.), Some(high));
    dom.apply_modal_layers(&mut layout.paint_order, &[(Some(backdrop), panel)]);

    assert_eq!(
        layout.paint_order,
        [root, low, equal, high, backdrop, panel, body, first, second, higher]
    );
    let (_, owners) = DisplayList::compile_attributed(
        &dom,
        &layout,
        None,
        &BTreeMap::new(),
        InteractionState::default(),
    );
    assert_eq!(&owners, &layout.paint_order[1..]);
    assert_eq!(layout.hit_test(&dom, 60., 50.), Some(body));
    assert_eq!(layout.hit_test(&dom, 170., 50.), Some(second));
    assert_eq!(layout.hit_test(&dom, 146., 51.), Some(higher));
    assert_eq!(
        layout.hit_test_where(&dom, 146., 51., |id| id == first || id == second),
        Some(second)
    );
    assert_eq!(dom.paint_order(), legacy_order);
}

#[test]
fn nested_and_sibling_modals_use_the_supplied_stack_order() {
    let mut dom = MicroDom::new(Node::new(Role::Window, "App"));
    let root = dom.root();
    let viewport = Rect::new(0., 0., 240., 180.);
    let ink = Color::rgb(81, 120, 143);
    let outer_backdrop = add(&mut dom, root, Role::Group, fixed(viewport, 20, ink));
    let outer = add(
        &mut dom,
        outer_backdrop,
        Role::Dialog,
        fixed(viewport, 20, ink),
    );
    let inner_backdrop = add(&mut dom, outer, Role::Group, fixed(viewport, -20, ink));
    let inner = add(
        &mut dom,
        inner_backdrop,
        Role::Dialog,
        fixed(viewport, -20, ink),
    );
    let outer_action = add(
        &mut dom,
        outer,
        Role::Button,
        fixed(viewport, i32::MAX, ink),
    );
    let inner_action = add(
        &mut dom,
        inner,
        Role::Button,
        fixed(viewport, i32::MIN, ink),
    );
    let sibling = add(&mut dom, root, Role::Menu, fixed(viewport, i32::MIN, ink));
    let sibling_action = add(
        &mut dom,
        sibling,
        Role::MenuItem,
        fixed(viewport, i32::MIN, ink),
    );
    let mut layout = LayoutEngine::default()
        .compute(&dom, viewport.width, viewport.height, &mut Metrics)
        .unwrap();
    dom.apply_modal_layers(
        &mut layout.paint_order,
        &[
            (Some(outer_backdrop), outer),
            (Some(inner_backdrop), inner),
            (None, sibling),
        ],
    );
    assert_eq!(
        layout.paint_order,
        [
            root,
            outer_backdrop,
            outer,
            outer_action,
            inner_backdrop,
            inner,
            inner_action,
            sibling,
            sibling_action,
        ]
    );
    assert_eq!(layout.hit_test(&dom, 60., 50.), Some(sibling_action));
    assert_eq!(
        layout.hit_test_where(&dom, 60., 50., |id| id == inner_action),
        Some(inner_action)
    );
}

#[test]
fn empty_or_invalid_layers_keep_legacy_order() {
    let mut dom = MicroDom::new(Node::new(Role::Window, "App"));
    let root = dom.root();
    let group = add(&mut dom, root, Role::Group, Style::default());
    let panel = add(&mut dom, group, Role::Dialog, Style::default());
    let sibling = add(&mut dom, root, Role::Group, Style::default());
    let mut order = dom.paint_order();
    let legacy = order.clone();
    dom.apply_modal_layers(&mut order, &[]);
    assert_eq!(order, legacy);
    dom.apply_modal_layers(&mut order, &[(Some(sibling), panel), (Some(root), panel)]);
    assert_eq!(order, legacy);
}
