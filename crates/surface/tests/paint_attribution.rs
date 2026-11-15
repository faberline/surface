//! Paint attribution (issue #128, O3): every op has an owner node.
//! Rows 1 and 2 of the acceptance table.
use std::collections::BTreeMap;
use surface::*;

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size * 0.5, size * 1.5)
    }
}

const ROLES: [Role; 23] = [
    Role::Group,
    Role::Window,
    Role::Heading,
    Role::Text,
    Role::Button,
    Role::Checkbox,
    Role::TextInput,
    Role::Navigation,
    Role::Radio,
    Role::RadioGroup,
    Role::Switch,
    Role::Slider,
    Role::Tab,
    Role::TabList,
    Role::Menu,
    Role::MenuItem,
    Role::Dialog,
    Role::Progress,
    Role::Status,
    Role::Separator,
    Role::Tooltip,
    Role::List,
    Role::ListItem,
];

/// One node per role, each in four style variants, so every op branch of
/// the compile loop runs: fill, border, state layer, legacy and shaped
/// focus, plain, styled and editable text.
fn tree() -> (MicroDom, NodeId) {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "App").styled(Style::column().padding(4.).gap(2.).clip(true)),
    );
    let shape = Corners {
        top_left: 12.,
        top_right: 0.,
        bottom_right: 12.,
        bottom_left: 0.,
    };
    let row = dom
        .append(
            dom.root(),
            Node::new(Role::Group, "Row").styled(Style::row().gap(2.).wrap(true)),
        )
        .unwrap();
    for (i, role) in ROLES.into_iter().enumerate() {
        for variant in 0..4 {
            let mut style = Style::default()
                .padding(2.)
                .background(Color::rgb(103, 80, 164))
                .foreground(Color::rgb(255, 255, 255));
            match variant {
                0 => style = style.radius(8.),
                1 => style = style.border(1., Color::rgb(0, 0, 0)).corners(shape),
                2 => {
                    style = style
                        .border(2., Color::rgb(0, 0, 0))
                        .state_layer(Color::rgb(0, 0, 0))
                        .font_weight(600)
                }
                _ => style = style.state_layer(Color::rgb(0, 0, 0)).corners(shape),
            }
            let node = Node::new(role, format!("{role:?}"))
                .id(format!("n{i}-{variant}"))
                .text(format!("{role:?} {variant}"))
                .styled(style);
            dom.append(row, node).unwrap();
        }
    }
    let field = dom
        .append(
            dom.root(),
            Node::new(Role::TextInput, "Field")
                .id("field")
                .text("hello world")
                .styled(
                    Style::default()
                        .padding(4.)
                        .background(Color::rgb(240, 240, 240))
                        .border(1., Color::rgb(0, 0, 0))
                        .height(Length::Px(40.)),
                ),
        )
        .unwrap();
    (dom, field)
}

fn computed(dom: &MicroDom) -> ComputedDom {
    LayoutEngine::default()
        .compute(dom, 1200., 900., &mut Metrics)
        .unwrap()
}

fn decorations(field: NodeId, computed: &ComputedDom) -> BTreeMap<NodeId, TextDecoration> {
    let b = computed.nodes[field.index()].bounds;
    let mut map = BTreeMap::new();
    map.insert(
        field,
        TextDecoration {
            selection: vec![
                Rect::new(b.x + 4., b.y + 4., 20., 14.),
                Rect::new(b.x + 4., b.y + 18., 30., 14.),
            ],
            underline: vec![Rect::new(b.x + 4., b.y + 30., 20., 1.)],
            caret: Some(Rect::new(b.x + 24., b.y + 4., 1., 14.)),
            scroll: (0., 0.),
            multiline: false,
        },
    );
    map
}

/// Every focus target and interaction state the tree supports.
fn cases(dom: &MicroDom, field: NodeId) -> Vec<(Option<NodeId>, InteractionState)> {
    let ids: Vec<NodeId> = dom.iter().map(|(id, _)| id).collect();
    let mut cases = vec![(None, InteractionState::default())];
    for &id in &ids {
        cases.push((Some(id), InteractionState::default()));
        cases.push((
            None,
            InteractionState {
                hovered: Some(id),
                pressed: None,
            },
        ));
        cases.push((
            Some(id),
            InteractionState {
                hovered: Some(id),
                pressed: Some(id),
            },
        ));
    }
    cases.push((
        Some(field),
        InteractionState {
            hovered: Some(field),
            pressed: None,
        },
    ));
    cases
}

// Row 1: owners.len() == ops.len(), owners follow paint order, and each
// text op is owned by the node that carries that text.
#[test]
fn owner_list_matches_ops_for_every_control_state() {
    let (dom, field) = tree();
    let computed = computed(&dom);
    let decorations = decorations(field, &computed);
    let rank: BTreeMap<NodeId, usize> = computed
        .paint_order
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();
    let mut kinds = BTreeMap::new();
    for (focused, interaction) in cases(&dom, field) {
        let (list, owners) =
            DisplayList::compile_attributed(&dom, &computed, focused, &decorations, interaction);
        assert_eq!(owners.len(), list.ops.len(), "focused {focused:?}");
        for pair in owners.windows(2) {
            assert!(
                rank[&pair[0]] <= rank[&pair[1]],
                "owners follow paint order"
            );
        }
        for (op, owner) in list.ops.iter().zip(&owners) {
            let node = dom.get(*owner).unwrap();
            let kind = match op {
                PaintOp::Quad { .. } => "Quad",
                PaintOp::Stroke { .. } => "Stroke",
                PaintOp::Text { text, .. } => {
                    assert_eq!(node.text_value(), Some(text.as_str()));
                    "Text"
                }
                PaintOp::EditableText { text, .. } => {
                    assert_eq!(node.text_value(), Some(text.as_str()));
                    "EditableText"
                }
                PaintOp::Shape { .. } => "Shape",
                PaintOp::ShapeStroke { .. } => "ShapeStroke",
                PaintOp::StyledText { text, .. } => {
                    assert_eq!(node.text_value(), Some(text.as_str()));
                    "StyledText"
                }
            };
            *kinds.entry(kind).or_insert(0usize) += 1;
        }
    }
    // Every op kind was produced at least once, so every branch was covered.
    assert_eq!(
        kinds.keys().copied().collect::<Vec<_>>(),
        [
            "EditableText",
            "Quad",
            "Shape",
            "ShapeStroke",
            "Stroke",
            "StyledText",
            "Text"
        ]
    );
}

// Row 1, text field part: selection quads, text, underline and caret all
// belong to the field.
#[test]
fn text_field_decorations_belong_to_the_field() {
    let (dom, field) = tree();
    let computed = computed(&dom);
    let decorations = decorations(field, &computed);
    let (list, owners) = DisplayList::compile_attributed(
        &dom,
        &computed,
        Some(field),
        &decorations,
        InteractionState::default(),
    );
    assert_eq!(owners.len(), list.ops.len());
    let field_ops: Vec<&PaintOp> = list
        .ops
        .iter()
        .zip(&owners)
        .filter(|(_, owner)| **owner == field)
        .map(|(op, _)| op)
        .collect();
    // fill, border, 4 focus quads, 2 selection quads, text, underline, caret
    assert_eq!(field_ops.len(), 11, "{field_ops:#?}");
    let d = &decorations[&field];
    for rect in d.selection.iter().chain(&d.underline).chain(d.caret.iter()) {
        assert!(
            field_ops.iter().any(|op| op.rect() == *rect),
            "decoration {rect:?} owned by the field"
        );
    }
    assert!(field_ops
        .iter()
        .any(|op| matches!(op, PaintOp::EditableText { text, .. } if text == "hello world")));
}

// Row 2: the attributed list is the list `compile_with_interaction` returns.
#[test]
fn attributed_list_equals_compile_with_interaction() {
    let (dom, field) = tree();
    let computed = computed(&dom);
    let decorations = decorations(field, &computed);
    for (focused, interaction) in cases(&dom, field) {
        let plain = DisplayList::compile_with_interaction(
            &dom,
            &computed,
            focused,
            &decorations,
            interaction,
        );
        let (attributed, _) =
            DisplayList::compile_attributed(&dom, &computed, focused, &decorations, interaction);
        assert_eq!(attributed.ops, plain.ops);
        assert_eq!(attributed, plain);
        assert_eq!(
            serde_json::to_string(&attributed).unwrap(),
            serde_json::to_string(&plain).unwrap()
        );
    }
}

// The other entry points are unchanged by the shared inner function.
#[test]
fn compile_and_compile_with_text_still_match() {
    let (dom, field) = tree();
    let computed = computed(&dom);
    let decorations = decorations(field, &computed);
    let (a, _) = DisplayList::compile_attributed(
        &dom,
        &computed,
        Some(field),
        &BTreeMap::new(),
        InteractionState::default(),
    );
    assert_eq!(a, DisplayList::compile(&dom, &computed, Some(field)));
    let (b, _) = DisplayList::compile_attributed(
        &dom,
        &computed,
        None,
        &decorations,
        InteractionState::default(),
    );
    assert_eq!(
        b,
        DisplayList::compile_with_text(&dom, &computed, None, &decorations)
    );
}
