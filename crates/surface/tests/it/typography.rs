//! Text styles: defaults, JSON compatibility, layout validation and measuring,
//! and the paint operations that carry a style to a renderer.
use surface::*;

const TEXT_KEYS: [&str; 4] = [
    "font_weight",
    "line_height",
    "letter_spacing",
    "font_family",
];

/// A size-only measurer, like every `TextMeasurer` written before this module.
#[derive(Default)]
struct SizeOnly;
impl TextMeasurer for SizeOnly {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.5, size * 1.5)
    }
}

/// A measurer that overrides `measure_text` and records what layout handed it.
#[derive(Default)]
struct Recorder {
    styles: Vec<TextStyle>,
}
impl TextMeasurer for Recorder {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        panic!("layout must measure through measure_text");
    }
    fn measure_text(&mut self, text: &str, style: &TextStyle, _: Option<f32>) -> (f32, f32) {
        self.styles.push(style.clone());
        (
            text.chars().count() as f32 * style.font_size * 0.5,
            style.line_height_px(),
        )
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn text_style_defaults_and_line_height() {
    let base = TextStyle::new(16.);
    assert_eq!(base.font_size, 16.);
    assert_eq!(base.line_height, None);
    assert_eq!(base.weight, 400);
    assert_eq!(base.letter_spacing, 0.);
    assert_eq!(base.family, FontFamily::Default);
    assert_eq!(FontFamily::default(), FontFamily::Default);
    assert_eq!(TextStyle::WEIGHT_REGULAR, 400);
    assert_eq!(TextStyle::WEIGHT_MEDIUM, 500);
    assert_eq!(TextStyle::WEIGHT_BOLD, 700);
    // No explicit line height means 1.5 times the size, as layout always did.
    assert_eq!(base.line_height_px(), 24.);
    assert_eq!(TextStyle::new(14.).line_height_px(), 21.);
    let tall = TextStyle {
        line_height: Some(40.),
        ..TextStyle::new(32.)
    };
    assert_eq!(tall.line_height_px(), 40.);
    assert!(base.is_legacy());
    assert!(!tall.is_legacy());
    // The contract case: size 16 with an explicit line height of 20.
    let lined = TextStyle {
        line_height: Some(20.),
        ..TextStyle::new(16.)
    };
    assert_eq!(lined.line_height_px(), 20.);
    assert!(!lined.is_legacy());
    for changed in [
        TextStyle {
            weight: 500,
            ..base.clone()
        },
        TextStyle {
            letter_spacing: 0.5,
            ..base.clone()
        },
        TextStyle {
            family: FontFamily::Named("Inter".to_owned()),
            ..base.clone()
        },
    ] {
        assert!(!changed.is_legacy(), "{changed:?}");
    }

    // Style carries the same attributes and defaults to the legacy text style.
    assert_eq!(Style::default().text_style(), TextStyle::new(16.));
    let style = Style::default()
        .font_size(20.)
        .font_weight(700)
        .line_height(28.)
        .letter_spacing(0.25)
        .font_family(FontFamily::Named("Inter".to_owned()));
    let expected = TextStyle {
        font_size: 20.,
        line_height: Some(28.),
        weight: 700,
        letter_spacing: 0.25,
        family: FontFamily::Named("Inter".to_owned()),
    };
    assert_eq!(style.text_style(), expected);
    assert_eq!(
        Style::default().with_text_style(expected.clone()),
        style,
        "with_text_style sets every text field"
    );
    assert_eq!(
        style.with_text_style(TextStyle::new(12.)).text_style(),
        TextStyle::new(12.),
        "with_text_style replaces the whole text style, not just some fields"
    );
}

#[test]
fn style_json_omits_default_text_fields_and_round_trips() {
    let value = serde_json::to_value(Style::default()).unwrap();
    let object = value.as_object().unwrap();
    for key in TEXT_KEYS {
        assert!(!object.contains_key(key), "{key} leaked into default JSON");
    }
    assert!(object.contains_key("font_size"));
    let default_json = serde_json::to_string(&Style::default()).unwrap();
    assert_eq!(
        serde_json::from_str::<Style>(&default_json).unwrap(),
        Style::default()
    );
    // Documents written before text styles existed read back as regular text.
    let legacy = serde_json::from_str::<Style>("{}").unwrap();
    assert_eq!(legacy.font_weight, 400);
    assert_eq!(legacy.line_height, None);
    assert_eq!(legacy.letter_spacing, 0.);
    assert_eq!(legacy.font_family, FontFamily::Default);
    assert_eq!(legacy.text_style(), TextStyle::new(legacy.font_size));

    // One non-default field adds exactly that key.
    for (style, key) in [
        (Style::default().font_weight(700), "font_weight"),
        (Style::default().line_height(30.), "line_height"),
        (Style::default().letter_spacing(-0.2), "letter_spacing"),
        (
            Style::default().font_family(FontFamily::Named("Inter".to_owned())),
            "font_family",
        ),
    ] {
        let value = serde_json::to_value(&style).unwrap();
        let object = value.as_object().unwrap();
        for other in TEXT_KEYS {
            assert_eq!(object.contains_key(other), other == key, "{key} / {other}");
        }
        assert_eq!(
            serde_json::from_value::<Style>(value).unwrap(),
            style,
            "{key} round trip"
        );
    }

    let styled = Style::default()
        .font_size(32.)
        .font_weight(500)
        .line_height(40.)
        .letter_spacing(0.5)
        .font_family(FontFamily::Named("Inter".to_owned()));
    let value = serde_json::to_value(&styled).unwrap();
    assert_eq!(value["font_weight"], 500);
    assert_eq!(value["line_height"], 40.0);
    assert_eq!(value["letter_spacing"], 0.5);
    assert_eq!(value["font_family"]["Named"], "Inter");
    assert_eq!(serde_json::from_value::<Style>(value).unwrap(), styled);

    // The contract case: a whole text style set at once, round-tripped through
    // both JSON forms, reads back as the same text style.
    let literal = TextStyle {
        font_size: 14.,
        line_height: Some(20.),
        weight: 500,
        letter_spacing: 0.1,
        family: FontFamily::Named("Tuffy".into()),
    };
    let style = Style::default().with_text_style(literal.clone());
    assert_eq!(style.text_style(), literal);
    let json = serde_json::to_string(&style).unwrap();
    let back = serde_json::from_str::<Style>(&json).unwrap();
    assert_eq!(back, style);
    assert_eq!(back.text_style(), literal);
    let back = serde_json::from_value::<Style>(serde_json::to_value(&style).unwrap()).unwrap();
    assert_eq!(back, style);
    assert_eq!(back.text_style(), literal);
}

#[test]
fn layout_rejects_invalid_text_style() {
    let compute = |style: Style| {
        let dom = MicroDom::new(Node::new(Role::Text, "Hi").text("Hi").styled(style));
        LayoutEngine::default().compute(&dom, 200., 100., &mut SizeOnly)
    };
    assert!(compute(Style::default()).is_ok());
    for ok in [
        Style::default().font_weight(1),
        Style::default().font_weight(1000),
        Style::default().line_height(0.5),
        Style::default().letter_spacing(-2.),
        Style::default().letter_spacing(3.),
    ] {
        assert!(compute(ok.clone()).is_ok(), "{ok:?}");
    }
    for bad in [
        Style::default().font_weight(0),
        Style::default().font_weight(1001),
        Style::default().line_height(0.),
        Style::default().line_height(-4.),
        Style::default().line_height(f32::NAN),
        Style::default().line_height(f32::INFINITY),
        Style::default().letter_spacing(f32::NAN),
        Style::default().letter_spacing(f32::INFINITY),
        Style::default().letter_spacing(f32::NEG_INFINITY),
    ] {
        assert!(
            matches!(compute(bad.clone()), Err(LayoutError::InvalidStyle(0))),
            "{bad:?}"
        );
    }
}

#[test]
fn default_measure_text_scales_legacy_height_by_line_height() {
    let mut measurer = SizeOnly;
    // The contract case: font 16 with a line height of 40 is 40 high, and a
    // legacy style measures exactly as `measure` does.
    let sixteen = measurer.measure("Hello", 16., None);
    assert_eq!(sixteen, (40., 24.));
    assert_eq!(
        measurer.measure_text(
            "Hello",
            &TextStyle {
                line_height: Some(40.),
                ..TextStyle::new(16.)
            },
            None
        ),
        (40., 40.)
    );
    assert_eq!(
        measurer.measure_text("Hello", &TextStyle::new(16.), None),
        sixteen
    );
    let legacy = measurer.measure("Hello", 32., None);
    assert_eq!(legacy, (80., 48.));
    // No line height: the legacy answer, unchanged.
    assert_eq!(
        measurer.measure_text("Hello", &TextStyle::new(32.), None),
        legacy
    );
    // A line height replaces the legacy 1.5 factor. Width is untouched.
    let style = |size: f32, line_height: f32| TextStyle {
        line_height: Some(line_height),
        ..TextStyle::new(size)
    };
    assert_eq!(
        measurer.measure_text("Hello", &style(32., 40.), None),
        (80., 40.)
    );
    assert_eq!(
        measurer.measure_text("Hello", &style(32., 64.), Some(10.)),
        (80., 64.)
    );
    assert_eq!(
        measurer.measure_text("Hello", &style(24., 32.), None),
        (60., 32.)
    );
    assert_eq!(
        measurer.measure_text("Hello", &style(11., 16.), None),
        (27.5, 16.)
    );
    // Weight, spacing and family do not change a size-only measurement.
    let heavy = TextStyle {
        weight: 700,
        letter_spacing: 1.,
        family: FontFamily::Named("Inter".to_owned()),
        ..TextStyle::new(32.)
    };
    assert_eq!(measurer.measure_text("Hello", &heavy, None), legacy);
}

#[test]
fn default_text_layout_styled_scales_lines() {
    let mut measurer = SizeOnly;
    let legacy = measurer.text_layout("ab\ncd", 16., None);
    assert_eq!(legacy.line_height, 24.);
    assert_eq!(legacy.height, 48.);
    // No line height: identical to the size-only layout.
    let same = measurer.text_layout_styled("ab\ncd", &TextStyle::new(16.), None);
    assert_eq!(same.line_height, legacy.line_height);
    assert_eq!(same.height, legacy.height);
    assert_eq!(same.text_len, legacy.text_len);
    assert_eq!(same.clusters.len(), legacy.clusters.len());
    for (new, old) in same.clusters.iter().zip(&legacy.clusters) {
        assert_eq!(new.bytes, old.bytes);
        assert_eq!(new.bounds, old.bounds);
        assert_eq!(new.rtl, old.rtl);
    }

    let style = TextStyle {
        line_height: Some(20.),
        ..TextStyle::new(16.)
    };
    let styled = measurer.text_layout_styled("ab\ncd", &style, None);
    assert_eq!(styled.clusters.len(), legacy.clusters.len());
    assert_eq!(styled.text_len, legacy.text_len);
    assert!(close(styled.line_height, 20.), "{}", styled.line_height);
    assert!(close(styled.height, 40.), "{}", styled.height);
    for (new, old) in styled.clusters.iter().zip(&legacy.clusters) {
        assert_eq!(new.bytes, old.bytes);
        assert_eq!(new.rtl, old.rtl);
        assert_eq!(new.bounds.x, old.bounds.x);
        assert_eq!(new.bounds.width, old.bounds.width);
        assert!(close(new.bounds.height, 20.), "{:?}", new.bounds);
        assert!(
            close(new.bounds.y, old.bounds.y * 20. / 24.),
            "{:?}",
            new.bounds
        );
    }
    // The second line starts one styled line height below the first.
    assert!(close(styled.clusters[2].bounds.y, 20.));
    assert!(close(styled.caret("ab\n".len()).height, 20.));

    // The contract case: two lines at size 16 and line height 30. The factor
    // 30 / 24 is exact in f32, so these values are exact too.
    let thirty = TextStyle {
        line_height: Some(30.),
        ..TextStyle::new(16.)
    };
    let layout = measurer.text_layout_styled("a\nb", &thirty, None);
    assert_eq!(layout.line_height, 30.);
    assert_eq!(layout.height, 60.);
    assert_eq!(layout.clusters.len(), 2);
    assert_eq!(layout.clusters[0].bounds.y, 0.);
    assert_eq!(layout.clusters[1].bounds.y, 30.);
    assert_eq!(layout.clusters[1].bounds.height, 30.);
}

#[test]
fn layout_measures_with_the_node_text_style() {
    let heading = TextStyle {
        font_size: 32.,
        line_height: Some(40.),
        weight: 500,
        letter_spacing: 0.25,
        family: FontFamily::Named("Inter".to_owned()),
    };
    let mut dom =
        MicroDom::new(Node::new(Role::Window, "App").styled(Style::column().align(Align::Start)));
    let title = dom
        .append(
            dom.root(),
            Node::new(Role::Heading, "Title")
                .id("title")
                .text("Title")
                .styled(Style::default().with_text_style(heading.clone())),
        )
        .unwrap();
    let plain = dom
        .append(
            dom.root(),
            Node::new(Role::Text, "Plain")
                .id("plain")
                .text("Plain")
                .styled(Style::default().font_size(20.)),
        )
        .unwrap();
    let mut engine = LayoutEngine::default();
    let mut recorder = Recorder::default();
    let computed = engine.compute(&dom, 400., 300., &mut recorder).unwrap();
    assert!(recorder.styles.contains(&heading));
    assert!(recorder.styles.contains(&TextStyle::new(20.)));
    assert_eq!(computed.nodes[title.index()].bounds.height, 40.);
    assert_eq!(computed.nodes[plain.index()].bounds.height, 30.);
    assert_eq!(computed.nodes[title.index()].bounds.width, 80.);

    // A frame with nothing changed reuses the measure.
    recorder.styles.clear();
    engine.compute(&dom, 400., 300., &mut recorder).unwrap();
    assert!(recorder.styles.is_empty(), "{:?}", recorder.styles);

    // A change to only the weight must measure again, with the new weight.
    let mut style = dom.get(title).unwrap().style().clone();
    style.font_weight = 700;
    dom.set_style(title, style).unwrap();
    engine.compute(&dom, 400., 300., &mut recorder).unwrap();
    // Taffy may ask for the same node more than once in a pass. Only the
    // changed node is measured, and always with its new style.
    assert!(!recorder.styles.is_empty());
    for measured in &recorder.styles {
        assert_eq!(measured.weight, 700, "{measured:?}");
        assert_eq!(measured.line_height, Some(40.), "{measured:?}");
        assert_eq!(measured.font_size, 32., "{measured:?}");
    }

    // The same holds for line height alone.
    let mut style = dom.get(title).unwrap().style().clone();
    style.line_height = Some(64.);
    dom.set_style(title, style).unwrap();
    let computed = engine.compute(&dom, 400., 300., &mut recorder).unwrap();
    assert_eq!(recorder.styles.last().unwrap().line_height, Some(64.));
    assert_eq!(computed.nodes[title.index()].bounds.height, 64.);

    // A size-only measurer still lays out a styled node by its line height.
    let computed = LayoutEngine::default()
        .compute(&dom, 400., 300., &mut SizeOnly)
        .unwrap();
    assert_eq!(computed.nodes[title.index()].bounds.height, 64.);
    assert_eq!(computed.nodes[plain.index()].bounds.height, 30.);

    // The contract case: a text node with weight 700 and line height 30 is
    // measured with exactly that style, and its height is the measurer result.
    let mut dom =
        MicroDom::new(Node::new(Role::Window, "App").styled(Style::column().align(Align::Start)));
    let bold = dom
        .append(
            dom.root(),
            Node::new(Role::Text, "Bold")
                .text("Bold")
                .styled(Style::default().font_weight(700).line_height(30.)),
        )
        .unwrap();
    let mut recorder = Recorder::default();
    let computed = LayoutEngine::default()
        .compute(&dom, 400., 300., &mut recorder)
        .unwrap();
    let expected = TextStyle {
        font_size: 16.,
        line_height: Some(30.),
        weight: 700,
        letter_spacing: 0.,
        family: FontFamily::Default,
    };
    assert!(!recorder.styles.is_empty());
    for measured in &recorder.styles {
        assert_eq!(*measured, expected);
    }
    assert_eq!(computed.nodes[bold.index()].bounds.height, 30.);
    assert_eq!(computed.nodes[bold.index()].bounds.width, 32.);
}

#[test]
fn paint_keeps_legacy_text_ops_and_emits_styled_text_otherwise() {
    let mut dom = MicroDom::new(Node::new(Role::Window, "App").styled(Style::column()));
    let mut add = |role, name: &str, style: Style| {
        dom.append(
            dom.root(),
            Node::new(role, name).id(name).text(name).styled(style),
        )
        .unwrap();
    };
    add(Role::Text, "plain", Style::default().font_size(20.));
    add(Role::TextInput, "field", Style::default().font_size(18.));
    add(Role::Text, "tall", Style::default().line_height(30.));
    add(
        Role::TextInput,
        "heavy",
        Style::default().font_weight(500).letter_spacing(0.5),
    );
    // The two cases the contract names: a plain text node at weight 500, and
    // a text input that only sets a line height.
    add(Role::Text, "medium", Style::default().font_weight(500));
    add(Role::TextInput, "lined", Style::default().line_height(30.));
    let computed = LayoutEngine::default()
        .compute(&dom, 400., 300., &mut SizeOnly)
        .unwrap();
    let list = DisplayList::compile(&dom, &computed, None);

    let find = |name: &str| {
        list.ops
            .iter()
            .filter(|op| match op {
                PaintOp::Text { text, .. }
                | PaintOp::EditableText { text, .. }
                | PaintOp::StyledText { text, .. } => text == name,
                _ => false,
            })
            .collect::<Vec<_>>()
    };
    // Size-only text keeps the original operations and field values.
    assert!(matches!(
        find("plain").as_slice(),
        [PaintOp::Text { font_size, .. }] if *font_size == 20.
    ));
    assert!(matches!(
        find("field").as_slice(),
        [PaintOp::EditableText { font_size, multiline: false, .. }] if *font_size == 18.
    ));
    // Anything beyond a size becomes a single StyledText with the full style.
    match find("tall").as_slice() {
        [PaintOp::StyledText {
            style,
            editable: false,
            multiline: true,
            scroll,
            ..
        }] => {
            assert_eq!(style.line_height, Some(30.));
            assert_eq!(style.font_size, 16.);
            assert_eq!(*scroll, (0., 0.));
        }
        other => panic!("expected one StyledText, got {other:?}"),
    }
    match find("heavy").as_slice() {
        [PaintOp::StyledText {
            style,
            editable: true,
            multiline: false,
            ..
        }] => {
            assert_eq!(style.weight, 500);
            assert_eq!(style.letter_spacing, 0.5);
        }
        other => panic!("expected one editable StyledText, got {other:?}"),
    }
    // Plain text at weight 500: a StyledText that is not editable, has no
    // scroll, and carries exactly the style the node holds.
    match find("medium").as_slice() {
        [PaintOp::StyledText {
            style,
            editable: false,
            scroll,
            ..
        }] => {
            assert_eq!(*style, Style::default().font_weight(500).text_style());
            assert_eq!(style.weight, 500);
            assert_eq!(*scroll, (0., 0.));
        }
        other => panic!("expected one plain StyledText, got {other:?}"),
    }
    // A text input with a line height is editable StyledText, not EditableText.
    match find("lined").as_slice() {
        [PaintOp::StyledText {
            style,
            editable: true,
            ..
        }] => {
            assert_eq!(*style, Style::default().line_height(30.).text_style());
            assert_eq!(style.line_height, Some(30.));
        }
        other => panic!("expected one editable StyledText, got {other:?}"),
    }

    // StyledText survives the JSON wire format.
    let json = serde_json::to_string(&list).unwrap();
    assert_eq!(serde_json::from_str::<DisplayList>(&json).unwrap(), list);

    // A document with only size-only text never mentions the new operation,
    // so display lists that predate typography serialize as they did.
    let mut legacy = MicroDom::new(Node::new(Role::Window, "App").styled(Style::column()));
    legacy
        .append(
            legacy.root(),
            Node::new(Role::Text, "plain")
                .text("plain")
                .styled(Style::default().font_size(20.)),
        )
        .unwrap();
    let computed = LayoutEngine::default()
        .compute(&legacy, 400., 300., &mut SizeOnly)
        .unwrap();
    let json = serde_json::to_string(&DisplayList::compile(&legacy, &computed, None)).unwrap();
    assert!(json.contains(r#""Text":{"#), "{json}");
    assert!(!json.contains("StyledText"), "{json}");
}
