use std::collections::BTreeMap;

use surface::{
    Color, DisplayList, ExtendedDisplayList, ExtendedPaintOp, Insets, InteractionState,
    LayoutEngine, Length, MicroDom, Node, PaintOp, Rect, Role, Style, TextAlignment,
    TextDecoration, TextMeasurer, TextSelectionColors, TextSelectionRegion, TextStyle,
};

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.len() as f32 * size / 2., size * 1.5)
    }
}

#[test]
fn wrapped_alignment_is_opt_in_and_preserves_legacy_selection_serialization() {
    let mut dom = MicroDom::new(Node::new(Role::Window, ""));
    let field = dom
        .append(
            dom.root(),
            Node::new(Role::TextInput, "Wrapped").text("ABC").styled(
                Style::column()
                    .width(Length::Px(50.))
                    .height(Length::Px(40.))
                    .font_weight(500),
            ),
        )
        .unwrap();
    let authored = serde_json::to_value(dom.get(field).unwrap()).unwrap();
    let layout = LayoutEngine::default()
        .compute(&dom, 50., 40., &mut Metrics)
        .unwrap();
    let compile = |dom: &MicroDom, multiline| {
        ExtendedDisplayList::compile_with_selection_colors(
            dom,
            &layout,
            None,
            &BTreeMap::from([(
                field,
                TextDecoration {
                    selection: vec![Rect::new(12., 0., 30., 10.)],
                    multiline,
                    ..TextDecoration::default()
                },
            )]),
            InteractionState::default(),
            TextSelectionColors {
                background: Color::rgb(0, 220, 245),
                foreground: Color::rgb(0, 0, 0),
            },
        )
    };
    let baseline = compile(&dom, true);
    let [ExtendedPaintOp::TextSelection(legacy)] = baseline.ops() else {
        panic!("one selection operation");
    };
    let legacy = serde_json::to_value(legacy).unwrap();
    assert!(legacy.get("alignment").is_none());
    dom.set_presented_text_input_alignment(field, Some(TextAlignment::Right))
        .unwrap();
    assert_eq!(
        serde_json::to_value(dom.get(field).unwrap()).unwrap(),
        authored
    );
    let aligned = compile(&dom, true);
    let [ExtendedPaintOp::TextSelection(aligned)] = aligned.ops() else {
        panic!("one aligned selection operation");
    };
    assert_eq!(aligned.alignment(), Some(TextAlignment::Right));
    let mut aligned = serde_json::to_value(aligned).unwrap();
    assert_eq!(
        aligned.as_object_mut().unwrap().remove("alignment"),
        Some(serde_json::json!("Right"))
    );
    assert_eq!(aligned, legacy);
    let single_line = compile(&dom, false);
    let [ExtendedPaintOp::TextSelection(paint)] = single_line.ops() else {
        panic!("one single-line selection operation");
    };
    assert_eq!(paint.alignment(), None);

    let measured = Metrics.text_layout_aligned_styled(
        "AB\nC",
        &TextStyle::new(16.),
        50.,
        TextAlignment::Right,
    );
    assert_eq!(measured.caret(0).x, 34.);
    assert_eq!(measured.caret(3).x, 42.);
    assert_eq!(measured.hit(43., 27.), 3);
}

#[test]
fn text_input_caret_override_changes_only_the_caret_in_normal_paint() {
    let foreground = Color::rgb(18, 28, 39);
    let accent = Color::rgb(187, 32, 55);
    let colors = TextSelectionColors {
        background: Color::rgb(20, 40, 60),
        foreground: Color::rgb(232, 240, 248),
    };
    let node = Node::new(Role::TextInput, "Field").text("入力").styled(
        Style::column()
            .width(Length::Px(90.))
            .height(Length::Px(48.))
            .foreground(foreground),
    );
    let legacy = serde_json::to_value(&node).unwrap();
    assert!(legacy.get("text_input_caret_color").is_none());
    let decoded: Node = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), legacy);
    let accented = node.clone().text_input_caret_color(accent);
    let encoded = serde_json::to_value(&accented).unwrap();
    assert_eq!(
        encoded["text_input_caret_color"],
        serde_json::json!([187, 32, 55, 255])
    );
    let accented: Node = serde_json::from_value(encoded).unwrap();
    assert!(
        serde_json::to_value(Node::new(Role::Text, "").text_input_caret_color(accent))
            .unwrap()
            .get("text_input_caret_color")
            .is_none()
    );

    let caret = Rect::new(25., 1., 1., 16.);
    let render = |field| {
        let mut dom = MicroDom::new(Node::new(Role::Window, ""));
        let id = dom.append(dom.root(), field).unwrap();
        let computed = LayoutEngine::default()
            .compute(&dom, 120., 90., &mut Metrics)
            .unwrap();
        let decorations = BTreeMap::from([(
            id,
            TextDecoration {
                caret: Some(caret),
                selection: vec![Rect::new(0., 1., 24., 16.)],
                underline: vec![Rect::new(0., 16., 26., 1.)],
                ..TextDecoration::default()
            },
        )]);
        let interaction = InteractionState::default();
        let normal =
            DisplayList::compile_with_interaction(&dom, &computed, None, &decorations, interaction);
        let forced = ExtendedDisplayList::compile_with_selection_colors(
            &dom,
            &computed,
            None,
            &decorations,
            interaction,
            colors,
        );
        (normal, forced)
    };
    let (ordinary, forced) = render(node);
    let (custom, custom_forced) = render(accented);
    let (last, other) = ordinary.ops.split_last().unwrap();
    let (custom_last, custom_other) = custom.ops.split_last().unwrap();
    assert_eq!(other, custom_other);
    let PaintOp::Quad {
        rect,
        clip,
        radius,
        color,
    } = last
    else {
        panic!("caret is a quad");
    };
    assert_eq!((*rect, *radius, *color), (caret, 0., foreground));
    assert_eq!(
        *custom_last,
        PaintOp::Quad {
            rect: caret,
            clip: *clip,
            radius: 0.,
            color: accent
        }
    );
    assert!(forced.ops().iter().any(|op| matches!(op,
        ExtendedPaintOp::TextSelection(paint) if paint.colors() == colors)));
    assert_eq!(
        custom_forced.ops(),
        forced.ops(),
        "forced text and caret paint take precedence"
    );
}

fn contains(region: TextSelectionRegion, x: u32, y: u32) -> bool {
    let [left, top, width, height] = region.scissor;
    x >= left && x < left + width && y >= top && y < top + height
}

fn assert_partition(regions: &[TextSelectionRegion], clip: Rect, selected: &[Rect], scale: f32) {
    assert!(regions.iter().any(|region| region.selected));
    assert!(regions.iter().any(|region| !region.selected));
    for y in 0..96 {
        for x in 0..96 {
            let px = x as f32;
            let py = y as f32;
            let visible = px >= (clip.x * scale).floor()
                && px < ((clip.x + clip.width) * scale).ceil()
                && py >= (clip.y * scale).floor()
                && py < ((clip.y + clip.height) * scale).ceil();
            let mut hits = regions
                .iter()
                .copied()
                .filter(|region| contains(*region, x, y));
            if let Some(hit) = hits.next() {
                assert!(visible, "paint outside the inherited clip at {x},{y}");
                assert!(hits.next().is_none(), "double paint at {x},{y}");
                let sample = ((px + 0.5) / scale, (py + 0.5) / scale);
                assert_eq!(
                    hit.selected,
                    selected
                        .iter()
                        .any(|rect| rect.contains(sample.0, sample.1)),
                    "wrong ink at {x},{y} and scale {scale}",
                );
            } else {
                assert!(!visible, "missing ordinary or selected ink at {x},{y}");
            }
        }
    }
}

#[test]
fn paired_selection_preserves_one_editable_run_and_disjoint_fractional_pixel_regions() {
    let ink = Color::rgb(12, 45, 67);
    let colors = TextSelectionColors {
        background: Color(0, 230, 255, 204),
        foreground: Color::rgb(0, 0, 0),
    };

    for styled in [false, true] {
        let mut dom = MicroDom::new(
            Node::new(Role::Window, "").styled(
                Style::column()
                    .width(Length::Px(40.))
                    .height(Length::Px(29.))
                    .clip(true),
            ),
        );
        let style = Style::column()
            .width(Length::Px(54.))
            .height(Length::Px(40.))
            .padding(4.)
            .foreground(ink)
            .absolute(Insets {
                left: Length::Px(5.),
                top: Length::Px(6.),
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
                Node::new(Role::TextInput, "Editor")
                    .text("abc אבג\n第二行")
                    .styled(style),
            )
            .unwrap();
        let layout = LayoutEngine::default()
            .compute(&dom, 48., 48., &mut Metrics)
            .unwrap();
        let geometry = &layout.nodes[field.index()];
        let text_rect = geometry.bounds.inset(4.);
        let clip = geometry.clip.intersect(text_rect);
        assert!(clip.width < text_rect.width && clip.height < text_rect.height);
        let (x, y) = (clip.x, clip.y);
        let selection = vec![
            Rect::new(x - 5.125, y + 0.125, 19., 8.),
            Rect::new(x + 9.125, y + 5.125, 27., 9.),
            Rect::new(x + 100., y, 10., 8.),
        ];
        let decorations = BTreeMap::from([(
            field,
            TextDecoration {
                selection: selection.clone(),
                underline: vec![Rect::new(x + 2., y + 15., 5., 1.)],
                caret: Some(Rect::new(x + 20., y + 10., 1., 8.)),
                scroll: (4.25, 2.5),
                multiline: true,
            },
        )]);
        let state = InteractionState::default();
        let normal =
            DisplayList::compile_with_interaction(&dom, &layout, None, &decorations, state);
        assert_eq!(normal.ops.len(), 6);
        for brush in &normal.ops[..3] {
            assert!(
                matches!(brush, PaintOp::Quad { color, .. } if *color == Color(103, 80, 164, 80))
            );
        }

        let (list, owners) = ExtendedDisplayList::compile_attributed_with_selection_colors(
            &dom,
            &layout,
            None,
            &decorations,
            state,
            colors,
        );
        let [ExtendedPaintOp::TextSelection(paint), ExtendedPaintOp::Legacy(underline), ExtendedPaintOp::Legacy(caret)] =
            list.ops()
        else {
            panic!("one selected run plus its original decorations")
        };
        assert_eq!(owners, vec![field; 3]);
        assert_eq!(paint.run(), &normal.ops[3]);
        assert_eq!(paint.colors(), colors);
        assert_eq!(underline, &normal.ops[4]);
        assert_eq!(caret, &normal.ops[5]);
        assert_eq!(
            matches!(paint.run(), PaintOp::StyledText { editable: true, .. }),
            styled
        );

        for scale in [1., 1.25, 1.5, 2.] {
            assert_partition(
                &paint.pixel_regions(scale, (96, 96)),
                clip,
                &selection,
                scale,
            );
        }
        assert!(paint.pixel_regions(f32::NAN, (96, 96)).is_empty());
        assert!(paint.pixel_regions(1., (0, 0)).is_empty());
    }
}

#[test]
fn overlapping_forced_selection_uses_highlight_text_for_each_clipped_preedit_underline_region() {
    let normal_ink = Color::rgb(0, 0, 0);
    let selected_ink = Color::rgb(255, 255, 255);
    let colors = TextSelectionColors {
        background: normal_ink,
        foreground: selected_ink,
    };
    let mut dom = MicroDom::new(
        Node::new(Role::Window, "").styled(
            Style::column()
                .width(Length::Px(32.))
                .height(Length::Px(24.))
                .clip(true),
        ),
    );
    let field = dom
        .append(
            dom.root(),
            Node::new(Role::TextInput, "Editor").text("ㄊㄞˊ北").styled(
                Style::column()
                    .width(Length::Px(48.))
                    .height(Length::Px(32.))
                    .padding(2.)
                    .foreground(normal_ink)
                    .absolute(Insets {
                        left: Length::Px(3.),
                        top: Length::Px(5.),
                        ..Insets::default()
                    }),
            ),
        )
        .unwrap();
    let layout = LayoutEngine::default()
        .compute(&dom, 60., 48., &mut Metrics)
        .unwrap();
    let geometry = &layout.nodes[field.index()];
    let text_rect = geometry.bounds.inset(2.);
    let clip = geometry.clip.intersect(text_rect);
    assert!(clip.width < text_rect.width && clip.height < text_rect.height);
    let bottom = clip.y + clip.height;
    let underline = Rect::new(clip.x - 2., bottom - 2., clip.width + 4., 4.);
    let selection = vec![
        Rect::new(clip.x + 4.25, bottom - 3., 10., 4.),
        Rect::new(clip.x + 10.125, bottom - 1., 10., 2.5),
        Rect::new(clip.x + 40., bottom - 1., 4., 1.),
    ];
    let decorations = BTreeMap::from([(
        field,
        TextDecoration {
            selection: selection.clone(),
            underline: vec![underline],
            ..TextDecoration::default()
        },
    )]);
    let interaction = InteractionState::default();
    let normal =
        DisplayList::compile_with_interaction(&dom, &layout, None, &decorations, interaction);
    assert_eq!(
        normal.ops.last(),
        Some(&PaintOp::Quad {
            rect: underline,
            clip,
            radius: 0.,
            color: normal_ink
        }),
    );

    let (paint, owners) = ExtendedDisplayList::compile_attributed_with_selection_colors(
        &dom,
        &layout,
        None,
        &decorations,
        interaction,
        colors,
    );
    let [ExtendedPaintOp::TextSelection(_), draws @ ..] = paint.ops() else {
        panic!("the selected run precedes the preedit underline");
    };
    assert_eq!(owners, vec![field; 1 + draws.len()]);
    let fragments: Vec<_> = draws
        .iter()
        .map(|draw| {
            let ExtendedPaintOp::Legacy(PaintOp::Quad {
                rect,
                clip: draw_clip,
                radius,
                color,
            }) = draw
            else {
                panic!("expected a rectangular underline fragment");
            };
            assert_eq!(*draw_clip, clip);
            assert_eq!(*radius, 0.);
            (*rect, *color)
        })
        .collect();
    assert!(
        fragments.iter().any(|(_, ink)| *ink == selected_ink),
        "preedit underline needs HighlightText over the dark selection"
    );
    assert!(fragments.iter().any(|(_, ink)| *ink == normal_ink));

    let visible = underline.intersect(clip);
    for (rect, _) in &fragments {
        assert_eq!(rect.intersect(visible), *rect);
    }
    for row in ((bottom - 3.) * 8.) as i32..((bottom + 2.) * 8.) as i32 {
        for column in ((clip.x - 2.) * 8.) as i32..((clip.x + clip.width + 2.) * 8.) as i32 {
            let (x, y) = (column as f32 / 8., row as f32 / 8.);
            let mut hits = fragments.iter().filter(|(rect, _)| rect.contains(x, y));
            let expected = visible.contains(x, y).then(|| {
                if selection.iter().any(|rect| rect.contains(x, y)) {
                    selected_ink
                } else {
                    normal_ink
                }
            });
            assert_eq!(
                hits.next().map(|(_, ink)| *ink),
                expected,
                "wrong ink at {x},{y}"
            );
            assert!(hits.next().is_none(), "repeated underline paint at {x},{y}");
        }
    }
}
