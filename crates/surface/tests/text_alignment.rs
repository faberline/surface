use std::collections::BTreeMap;

use surface::{
    Color, DisplayList, ExtendedDisplayList, ExtendedPaintOp, InteractionState, LayoutEngine,
    Length, MicroDom, Node, PaintOp, Rect, Role, Style, TextAlignment, TextDecoration,
    TextMeasurer, TextSelectionColors,
};

struct Fonts;

impl TextMeasurer for Fonts {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size / 2., size * 1.5)
    }
}

#[test]
fn text_alignment_node_wire_is_optional_and_text_inputs_keep_their_default() {
    let heading = Node::new(Role::Heading, "Dialog").text("Title");
    let default = serde_json::to_value(&heading).unwrap();
    assert!(default.get("text_alignment").is_none());
    assert_eq!(
        serde_json::from_value::<Node>(default.clone()).unwrap(),
        heading
    );

    let aligned = heading.text_alignment(TextAlignment::Right);
    let mut wire = serde_json::to_value(&aligned).unwrap();
    assert_eq!(
        serde_json::from_value::<Node>(wire.clone()).unwrap(),
        aligned
    );
    assert_eq!(
        wire.as_object_mut().unwrap().remove("text_alignment"),
        Some("Right".into())
    );
    assert_eq!(wire, default);

    let editor = Node::new(Role::TextInput, "Editor").text("A draft");
    assert_eq!(editor.clone().text_alignment(TextAlignment::Center), editor);
}

#[test]
fn extended_alignment_keeps_static_run_clip_and_owner_and_leaves_editable_selection() {
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "").styled(
            Style::column()
                .width(Length::Px(80.))
                .height(Length::Px(60.))
                .clip(true),
        ),
    );
    let size = Style::column()
        .width(Length::Px(90.))
        .height(Length::Px(18.))
        .padding(1.);
    let title = dom
        .append(
            dom.root(),
            Node::new(Role::Heading, "Title")
                .id("title")
                .text("Title")
                .styled(size.clone())
                .text_alignment(TextAlignment::Right),
        )
        .unwrap();
    let description = dom
        .append(
            dom.root(),
            Node::new(Role::Text, "Supporting")
                .id("supporting")
                .text("Supporting")
                .styled(size.clone().line_height(17.))
                .text_alignment(TextAlignment::Center),
        )
        .unwrap();
    let mut edited_wire = serde_json::to_value(
        Node::new(Role::TextInput, "Editor")
            .id("editor")
            .text("Draft")
            .styled(size),
    )
    .unwrap();
    edited_wire["text_alignment"] = "Right".into();
    let edited = dom
        .append(dom.root(), serde_json::from_value(edited_wire).unwrap())
        .unwrap();
    let layout = LayoutEngine::default()
        .compute(&dom, 100., 80., &mut Fonts)
        .unwrap();
    let editor_bounds = layout.nodes[edited.index()].bounds;
    let decorations = BTreeMap::from([(
        edited,
        TextDecoration {
            selection: vec![Rect::new(1., editor_bounds.y + 1., 8., 12.)],
            ..TextDecoration::default()
        },
    )]);
    let interaction = InteractionState::default();
    let (legacy, legacy_owners) =
        DisplayList::compile_attributed(&dom, &layout, None, &decorations, interaction);
    let (extended, owners) =
        ExtendedDisplayList::compile_attributed(&dom, &layout, None, &decorations, interaction);
    assert_eq!(owners, legacy_owners);
    for (index, (op, source)) in extended.ops().iter().zip(&owners).enumerate() {
        if *source == title || *source == description {
            let ExtendedPaintOp::AlignedText(aligned) = op else {
                panic!("missing alignment for {source:?}");
            };
            let mode = if *source == title {
                TextAlignment::Right
            } else {
                TextAlignment::Center
            };
            assert_eq!(aligned.alignment(), mode);
            assert_eq!(aligned.run(), &legacy.ops[index]);
            assert_eq!(aligned.run().clip().width, 79.);
        } else {
            assert_eq!(*op, ExtendedPaintOp::Legacy(legacy.ops[index].clone()));
        }
    }
    assert!(matches!(legacy.ops[0], PaintOp::Text { .. }));
    assert!(matches!(
        legacy.ops[1],
        PaintOp::StyledText {
            editable: false,
            ..
        }
    ));
    assert_eq!(
        serde_json::from_str::<ExtendedDisplayList>(&serde_json::to_string(&extended).unwrap())
            .unwrap(),
        extended,
    );
    let from_ops = ExtendedDisplayList::new(extended.viewport(), extended.clone().into_ops());
    assert_eq!(from_ops, extended);

    let (forced, owners) = ExtendedDisplayList::compile_attributed_with_selection_colors(
        &dom,
        &layout,
        None,
        &decorations,
        interaction,
        TextSelectionColors {
            background: Color::rgb(30, 64, 80),
            foreground: Color::rgb(255, 255, 255),
        },
    );
    assert_eq!(owners, [title, description, edited]);
    assert_eq!(forced.ops()[..2], extended.ops()[..2]);
    assert!(
        matches!(&forced.ops()[2], ExtendedPaintOp::TextSelection(selection)
        if matches!(selection.run(), PaintOp::EditableText { .. }))
    );
}
