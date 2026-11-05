use surface::{FontFamily, NodeId, Role, TextMeasurer, TextStyle};
use surface_app::{Application, Runtime, View};
use surface_material::{typography::*, Theme};

#[test]
fn type_scale_matches_androidx_tokens() {
    // (role, size, line height, letter spacing, weight) from TypeScaleTokens.kt
    // in androidx-main. Weights: Regular 400, Medium 500, Bold 700.
    let scale: [(&str, TextStyle, f32, f32, f32, u16); 30] = [
        ("DisplayLarge", DISPLAY_LARGE, 57., 64., -0.2, 400),
        ("DisplayMedium", DISPLAY_MEDIUM, 45., 52., 0., 400),
        ("DisplaySmall", DISPLAY_SMALL, 36., 44., 0., 400),
        ("HeadlineLarge", HEADLINE_LARGE, 32., 40., 0., 400),
        ("HeadlineMedium", HEADLINE_MEDIUM, 28., 36., 0., 400),
        ("HeadlineSmall", HEADLINE_SMALL, 24., 32., 0., 400),
        ("TitleLarge", TITLE_LARGE, 22., 28., 0., 400),
        ("TitleMedium", TITLE_MEDIUM, 16., 24., 0.2, 500),
        ("TitleSmall", TITLE_SMALL, 14., 20., 0.1, 500),
        ("BodyLarge", BODY_LARGE, 16., 24., 0.5, 400),
        ("BodyMedium", BODY_MEDIUM, 14., 20., 0.2, 400),
        ("BodySmall", BODY_SMALL, 12., 16., 0.4, 400),
        ("LabelLarge", LABEL_LARGE, 14., 20., 0.1, 500),
        ("LabelMedium", LABEL_MEDIUM, 12., 16., 0.5, 500),
        ("LabelSmall", LABEL_SMALL, 11., 16., 0.5, 500),
        (
            "DisplayLargeEmphasized",
            DISPLAY_LARGE_EMPHASIZED,
            57.,
            64.,
            0.,
            500,
        ),
        (
            "DisplayMediumEmphasized",
            DISPLAY_MEDIUM_EMPHASIZED,
            45.,
            52.,
            0.,
            500,
        ),
        (
            "DisplaySmallEmphasized",
            DISPLAY_SMALL_EMPHASIZED,
            36.,
            44.,
            0.,
            500,
        ),
        (
            "HeadlineLargeEmphasized",
            HEADLINE_LARGE_EMPHASIZED,
            32.,
            40.,
            0.,
            500,
        ),
        (
            "HeadlineMediumEmphasized",
            HEADLINE_MEDIUM_EMPHASIZED,
            28.,
            36.,
            0.,
            500,
        ),
        (
            "HeadlineSmallEmphasized",
            HEADLINE_SMALL_EMPHASIZED,
            24.,
            32.,
            0.,
            500,
        ),
        (
            "TitleLargeEmphasized",
            TITLE_LARGE_EMPHASIZED,
            22.,
            28.,
            0.,
            500,
        ),
        (
            "TitleMediumEmphasized",
            TITLE_MEDIUM_EMPHASIZED,
            16.,
            24.,
            0.15,
            700,
        ),
        (
            "TitleSmallEmphasized",
            TITLE_SMALL_EMPHASIZED,
            14.,
            20.,
            0.1,
            700,
        ),
        (
            "BodyLargeEmphasized",
            BODY_LARGE_EMPHASIZED,
            16.,
            24.,
            0.15,
            500,
        ),
        (
            "BodyMediumEmphasized",
            BODY_MEDIUM_EMPHASIZED,
            14.,
            20.,
            0.25,
            500,
        ),
        (
            "BodySmallEmphasized",
            BODY_SMALL_EMPHASIZED,
            12.,
            16.,
            0.4,
            500,
        ),
        (
            "LabelLargeEmphasized",
            LABEL_LARGE_EMPHASIZED,
            14.,
            20.,
            0.1,
            700,
        ),
        (
            "LabelMediumEmphasized",
            LABEL_MEDIUM_EMPHASIZED,
            12.,
            16.,
            0.5,
            700,
        ),
        (
            "LabelSmallEmphasized",
            LABEL_SMALL_EMPHASIZED,
            11.,
            16.,
            0.5,
            700,
        ),
    ];
    for (name, style, size, line_height, spacing, weight) in scale {
        assert_eq!(style.font_size, size, "{name} size");
        assert_eq!(style.line_height, Some(line_height), "{name} line height");
        assert_eq!(style.letter_spacing, spacing, "{name} letter spacing");
        assert_eq!(style.weight, weight, "{name} weight");
        assert_eq!(style.family, FontFamily::Default, "{name} family");
        assert!(!style.is_legacy(), "{name} must paint as a styled run");
    }
    // The emphasized variants keep their base role's size and line height.
    let pairs = [
        (DISPLAY_LARGE, DISPLAY_LARGE_EMPHASIZED),
        (HEADLINE_SMALL, HEADLINE_SMALL_EMPHASIZED),
        (TITLE_MEDIUM, TITLE_MEDIUM_EMPHASIZED),
        (BODY_LARGE, BODY_LARGE_EMPHASIZED),
        (LABEL_SMALL, LABEL_SMALL_EMPHASIZED),
    ];
    for (base, emphasized) in pairs {
        assert_eq!(base.font_size, emphasized.font_size);
        assert_eq!(base.line_height, emphasized.line_height);
        assert!(emphasized.weight > base.weight);
    }
}

#[derive(Clone)]
enum Message {
    Nothing,
}
struct Gallery;
impl Application for Gallery {
    type Message = Message;
    fn update(&mut self, _: Message) {}
    fn view(&self) -> View<Message> {
        let theme = Theme::light();
        theme.page([
            theme.heading("title", "Headline"),
            theme.text("body", "Body copy"),
            theme.text_field("name", "Name", "value", |_| Message::Nothing),
            theme.button("save", "Save", Message::Nothing),
            theme.checkbox("agree", "Agree", true, Message::Nothing),
            theme.dialog(
                "confirm",
                "Dialog title",
                [theme.text("dialog-body", "Are you sure?")],
                [theme.button("ok", "OK", Message::Nothing)],
                Message::Nothing,
            ),
        ])
    }
}
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.5, size * 1.5)
    }
}

#[test]
fn theme_components_use_type_roles() {
    let mut runtime = Runtime::new(Gallery).unwrap();
    runtime.render(800., 900., &mut Metrics).unwrap();
    let dom = runtime.dom();
    let style = |id: NodeId| dom.get(id).unwrap().style().text_style();
    let by_id = |id: &str| dom.by_id(id).unwrap_or_else(|| panic!("missing {id}"));
    let label = |name: &str| {
        let ids = dom.by_role_name(Role::Text, name);
        assert_eq!(ids.len(), 1, "label {name}");
        ids[0]
    };

    assert_eq!(style(by_id("title")), HEADLINE_LARGE);
    assert_eq!(style(by_id("body")), BODY_LARGE);
    assert_eq!(style(by_id("dialog-body")), BODY_LARGE);
    assert_eq!(style(by_id("name")), BODY_LARGE);
    // Button labels are size 14: Label Large. Checkbox text is size 16: Body Large.
    assert_eq!(style(label("Save")), LABEL_LARGE);
    assert_eq!(style(label("OK")), LABEL_LARGE);
    assert_eq!(style(label("Agree")), BODY_LARGE);
    let dialog_title = dom.by_role_name(Role::Heading, "Dialog title");
    assert_eq!(dialog_title.len(), 1);
    assert_eq!(style(dialog_title[0]), HEADLINE_SMALL);
    // The checkbox mark is a glyph, not text in the type scale.
    assert_eq!(style(label("✓")), TextStyle::new(12.));
    assert!(style(label("✓")).is_legacy());

    // Line heights from the roles reach layout: the measurer above reports
    // 1.5 times the size, and the default measure_text rescales it.
    let computed = runtime.computed().unwrap();
    let height = |id: NodeId| {
        computed
            .nodes
            .iter()
            .find(|node| node.id == id)
            .unwrap()
            .bounds
            .height
    };
    assert_eq!(height(by_id("title")), 40.);
    assert_eq!(height(by_id("body")), 24.);
    assert_eq!(height(dialog_title[0]), 32.);
    assert_eq!(height(label("Save")), 20.);
}
