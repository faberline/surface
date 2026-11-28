use std::cell::Cell;
use std::num::NonZeroU16;

use surface::{
    Color, ExtendedDisplayList, ExtendedPaintOp, Node, PaintOp, PathFill, Rect, Role, Style,
    TextAlignment, TextMeasurer,
};
use surface_app::{
    Application, DisplayContrast, EditKey, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime,
    SystemColorPair, SystemContrastColors, View,
};
use surface_material::{
    shape::LayoutDirection, MultilineTextFieldSize, OutlinedTextFieldOptions, TextFieldAdornments,
    TextFieldAffix, TextFieldAppearance, TextFieldCounterOptions, TextFieldEditability,
    TextFieldTrailingAction, Theme, VectorIcon,
};
use surface_wgpu::TextSystem;

const ID: &str = "project-name";
const LABEL: &str = "專案名稱";
const LONG_LABEL: &str = "輸入團隊成員在共用專案清單中容易辨認的完整名稱";
const HINT: &str = "輸入名稱";
const SUPPORT: &str = "請輸入中文或英文名稱。";
const ERROR: &str = "請輸入有效的專案名稱。";
const COUNT_ERROR: &str = "名稱超過字數上限。";
const COUNTER: &str = "project-name-counter";
const AFTER: &str = "after-field";

#[derive(Clone)]
enum Message {
    Edit(String),
    Submit,
    Action,
    ChangeAppearance,
    SetInvalid(bool),
    SetEditability(TextFieldEditability),
}

struct Form {
    label: &'static str,
    hint: &'static str,
    value: String,
    dark: bool,
    rtl: bool,
    scale: f32,
    invalid: bool,
    editability: TextFieldEditability,
    edits: Vec<String>,
    submits: Vec<String>,
    actions: usize,
    adorned: Option<TextFieldAppearance>,
    character_limit: Option<usize>,
    multiline: Option<MultilineTextFieldSize>,
    field_contrast: Option<DisplayContrast>,
    trailing_disabled: bool,
    builds: Cell<usize>,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            label: LABEL,
            hint: HINT,
            value: String::new(),
            dark: false,
            rtl: false,
            scale: 1.,
            invalid: false,
            editability: TextFieldEditability::Editable,
            edits: Vec::new(),
            submits: Vec::new(),
            actions: 0,
            adorned: None,
            character_limit: None,
            multiline: None,
            field_contrast: None,
            trailing_disabled: false,
            builds: Cell::new(0),
        }
    }
}

impl Application for Form {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Edit(value) => {
                self.edits.push(value.clone());
                self.value = value;
            }
            Message::Submit => self.submits.push(self.value.clone()),
            Message::Action => self.actions += 1,
            Message::ChangeAppearance => {
                self.dark = !self.dark;
                self.rtl = !self.rtl;
                self.scale = 2.;
                self.invalid = !self.invalid;
            }
            Message::SetInvalid(invalid) => self.invalid = invalid,
            Message::SetEditability(editability) => self.editability = editability,
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn display_contrast_changed(&mut self, contrast: DisplayContrast) -> bool {
        if let Some(current) = &mut self.field_contrast {
            *current = contrast;
            return true;
        }
        false
    }

    fn view(&self) -> View<Message> {
        self.builds.set(self.builds.get() + 1);
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let mut options = OutlinedTextFieldOptions::new(ID, self.label, &self.value);
        options.placeholder = Some(self.hint);
        options.supporting_text = Some(SUPPORT);
        options.error = self.invalid.then_some(ERROR);
        options.editability = self.editability;
        options.direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        options.text_scale = self.scale;
        let on_submit = || Some(Message::Submit);
        let field = if let Some(appearance) = self.adorned {
            let icon = VectorIcon::new([[4., 4.], [20., 4.], [12., 20.]]).unwrap();
            let adornments = TextFieldAdornments {
                leading_icon: Some(&icon),
                trailing_action: Some(TextFieldTrailingAction {
                    icon: &icon,
                    name: "套用金額",
                    disabled: self.trailing_disabled,
                    on_press: Message::Action,
                }),
                prefix: Some(TextFieldAffix {
                    visual: "€",
                    spoken_name: "歐元",
                }),
                suffix: Some(TextFieldAffix {
                    visual: "/月",
                    spoken_name: "每月",
                }),
            };
            let counter = self
                .character_limit
                .map(|max_graphemes| TextFieldCounterOptions {
                    max_graphemes,
                    over_limit_error: COUNT_ERROR,
                    describe: &|count, max| format!("已輸入{count}個字元，上限{max}個。"),
                });
            match (self.field_contrast, self.multiline, counter) {
                (Some(contrast), Some(size), counter) => theme.text_fields_for(contrast).multiline(
                    appearance,
                    options,
                    size,
                    adornments,
                    counter,
                    Message::Edit,
                ),
                (Some(contrast), None, counter) => theme.text_fields_for(contrast).single_line(
                    appearance,
                    options,
                    adornments,
                    counter,
                    Message::Edit,
                    on_submit,
                ),
                (None, Some(size), counter) => theme.multiline_text_field(
                    appearance,
                    options,
                    size,
                    adornments,
                    counter,
                    Message::Edit,
                ),
                (None, None, Some(counter)) => theme.text_field_with_counter(
                    appearance,
                    options,
                    adornments,
                    counter,
                    Message::Edit,
                    on_submit,
                ),
                (None, None, None) => theme.text_field_with_adornments(
                    appearance,
                    options,
                    adornments,
                    Message::Edit,
                    on_submit,
                ),
            }
        } else {
            theme.outlined_text_field_with_submit(options, Message::Edit, on_submit)
        };
        View::new(Node::new(Role::Window, "Form").styled(Style::column().padding(12.).gap(8.)))
            .children([
                field.expect("valid Material field"),
                theme.button(AFTER, "Next", Message::ChangeAppearance),
            ])
    }
}

fn render(runtime: &mut Runtime<Form>, width: f32, text: &mut TextSystem) -> ExtendedDisplayList {
    runtime.render_extended(width, 440., text).unwrap()
}

fn bounds(runtime: &Runtime<Form>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn find_outline(list: &ExtendedDisplayList, color: Color, face: Rect) -> &PathFill {
    list.ops()
        .iter()
        .find_map(|op| match op {
            ExtendedPaintOp::FillPath(path) if path.color() == color && path.rect() == face => {
                Some(path)
            }
            _ => None,
        })
        .expect("inside outline contour at the editor bounds")
}

// The renderer uses even-odd fill. Sample between edges, away from vertices.
fn covers(path: &PathFill, x: f32, y: f32) -> bool {
    let points = path.points();
    path.clip().contains(x, y)
        && points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .filter(|(a, b)| {
                (a[1] > y) != (b[1] > y) && x < a[0] + (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1])
            })
            .count()
            % 2
            == 1
}

fn bottom_boundary(list: &ExtendedDisplayList, face: Rect) -> &PathFill {
    let mut paths = list.ops().iter().filter_map(|op| match op {
        ExtendedPaintOp::FillPath(path)
            if covers(path, face.x + face.width / 2., face.y + face.height - 0.25) =>
        {
            Some(path)
        }
        _ => None,
    });
    let path = paths.next().expect("boundary covers the bottom edge");
    assert!(paths.next().is_none(), "exactly one path covers that edge");
    path
}

fn assert_bottom_width(path: &PathFill, face: Rect, width: f32) {
    let x = face.x + face.width / 2.;
    let inner = face.y + face.height - width;
    assert!(covers(path, x, inner + 0.012), "stroke width under {width}");
    assert!(!covers(path, x, inner - 0.012), "stroke width over {width}");
}

fn text_run<'a>(list: &'a ExtendedDisplayList, text: &str) -> Option<(&'a PaintOp, TextAlignment)> {
    list.ops().iter().find_map(|op| {
        let (run, alignment) = match op {
            ExtendedPaintOp::Legacy(run) => (run, TextAlignment::Left),
            ExtendedPaintOp::AlignedText(run) => (run.run(), run.alignment()),
            _ => return None,
        };
        matches!(run, PaintOp::StyledText { text: value, .. } if value == text)
            .then_some((run, alignment))
    })
}

fn tab(runtime: &mut Runtime<Form>) {
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
}

fn enter() -> KeyPress {
    KeyPress::new(Key::Enter, Modifiers::default())
}

const BODY: &str = "project-name-value-box";
const TRAILING: &str = "project-name-trailing-action";
const HELP: &str = "project-name-description";

fn label_pose(list: &ExtendedDisplayList, label: &str) -> (Rect, f32) {
    let mut labels = list.ops().iter().filter_map(|op| {
        let run = match op {
            ExtendedPaintOp::Legacy(run) => run,
            ExtendedPaintOp::AlignedText(run) => run.run(),
            _ => return None,
        };
        match run {
            PaintOp::StyledText {
                text, rect, style, ..
            } if text == label => Some((*rect, style.font_size)),
            _ => None,
        }
    });
    let pose = labels.next().expect("one painted label");
    assert!(
        labels.next().is_none(),
        "the transition paints only one label"
    );
    pose
}

fn label_color(list: &ExtendedDisplayList, label: &str) -> Color {
    let mut colors = list.ops().iter().filter_map(|op| {
        let run = match op {
            ExtendedPaintOp::Legacy(run) => run,
            ExtendedPaintOp::AlignedText(run) => run.run(),
            _ => return None,
        };
        match run {
            PaintOp::StyledText { text, color, .. } if text == label && color.3 > 0 => Some(*color),
            _ => None,
        }
    });
    let color = colors.next().expect("expected visible field text");
    assert!(
        colors.next().is_none(),
        "expected one visible run for {label}"
    );
    color
}

fn hint_alpha(list: &ExtendedDisplayList) -> Option<u8> {
    let (PaintOp::StyledText { color, .. }, _) = text_run(list, HINT)? else {
        unreachable!("a hint uses styled text");
    };
    Some(color.3)
}

fn force_contrast(disabled_text: Option<Color>) -> DisplayContrast {
    let pair = SystemColorPair {
        background: Color::rgb(255, 255, 255),
        foreground: Color::rgb(0, 0, 0),
    };
    let colors = SystemContrastColors::new(pair, pair, pair);
    DisplayContrast::Forced(disabled_text.map_or(colors, |ink| colors.with_disabled_text(ink)))
}

fn assert_glyph_ink(list: &ExtendedDisplayList, bounds: Rect, ink: Color) {
    assert!(list.ops().iter().any(|op| matches!(
        op,
        ExtendedPaintOp::FillPath(path) if path.color() == ink
            && covers(path, bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.)
    )));
}

#[test]
fn system_disabled_ink_reaches_both_field_appearances_and_row_layouts() {
    const SYSTEM_INK: Color = Color(57, 142, 91, 173);
    const VALUE: &str = "數值";
    let colors = Theme::light().colors;
    let Color(r, g, b, _) = colors.on_surface;
    let mut fonts = TextSystem::default();
    for (appearance, border) in [
        (TextFieldAppearance::Outlined, "project-name-outline"),
        (TextFieldAppearance::Filled, "project-name-indicator"),
    ] {
        for multiline in [
            None,
            Some(MultilineTextFieldSize::FixedRows(
                NonZeroU16::new(2).unwrap(),
            )),
        ] {
            let mut runtime = Runtime::new(Form {
                value: VALUE.into(),
                adorned: Some(appearance),
                character_limit: Some(8),
                multiline,
                invalid: true,
                editability: TextFieldEditability::Disabled,
                field_contrast: Some(DisplayContrast::Normal),
                ..Form::default()
            })
            .unwrap();
            runtime.set_display_contrast(force_contrast(None)).unwrap();
            let fallback = render(&mut runtime, 400., &mut fonts);
            assert_eq!(label_color(&fallback, VALUE), Color(r, g, b, 97));
            let face = bounds(&runtime, border);
            let editor = bounds(&runtime, ID);
            runtime
                .dispatch(Event::PointerMove {
                    x: editor.x + editor.width / 2.,
                    y: editor.y + editor.height / 2.,
                })
                .unwrap();
            runtime
                .set_display_contrast(force_contrast(Some(SYSTEM_INK)))
                .unwrap();
            let forced = render(&mut runtime, 400., &mut fonts);
            for text in [LABEL, VALUE, ERROR, "2/8", "€", "/月"] {
                assert_eq!(label_color(&forced, text), SYSTEM_INK, "{text}");
            }
            for icon in ["project-name-leading-icon", "project-name-trailing-icon"] {
                assert_glyph_ink(&forced, bounds(&runtime, icon), SYSTEM_INK);
            }
            assert_eq!(bounds(&runtime, border), face);
            assert_eq!(bounds(&runtime, ID), editor);
            let before = bottom_boundary(&fallback, face);
            let after = bottom_boundary(&forced, face);
            assert_eq!(after.points(), before.points());
            assert_eq!(after.color(), before.color());
            let id = runtime.dom().by_id(ID).unwrap();
            let frame = runtime.accessibility_frame().unwrap();
            let semantic = frame.node(id).unwrap().node;
            assert!(semantic.is_disabled());
            assert!(!semantic.is_invalid());
            assert_eq!(semantic.text_value(), Some(VALUE));
            assert!(!frame.accepts_focus(id));
        }
    }
}

#[test]
fn system_disabled_ink_reaches_an_empty_hint_and_an_independently_disabled_action() {
    const SYSTEM_INK: Color = Color::rgb(57, 142, 91);
    const VALUE: &str = "數值";
    let colors = Theme::light().colors;
    let mut fonts = TextSystem::default();
    for appearance in [TextFieldAppearance::Outlined, TextFieldAppearance::Filled] {
        let mut disabled = Runtime::new(Form {
            adorned: Some(appearance),
            label: LONG_LABEL,
            editability: TextFieldEditability::Disabled,
            rtl: true,
            scale: 2.,
            field_contrast: Some(DisplayContrast::Normal),
            ..Form::default()
        })
        .unwrap();
        disabled
            .set_display_contrast(force_contrast(Some(SYSTEM_INK)))
            .unwrap();
        let compact = render(&mut disabled, 320., &mut fonts);
        assert_eq!(label_color(&compact, HINT), SYSTEM_INK);
        assert_eq!(label_color(&compact, LONG_LABEL), SYSTEM_INK);
        let hint = disabled
            .dom()
            .by_id("project-name-disabled-placeholder")
            .unwrap();
        assert!(disabled.accessibility_frame().unwrap().node(hint).is_none());

        let mut read_only = Runtime::new(Form {
            value: VALUE.into(),
            adorned: Some(appearance),
            trailing_disabled: true,
            editability: TextFieldEditability::ReadOnly,
            invalid: true,
            field_contrast: Some(DisplayContrast::Normal),
            ..Form::default()
        })
        .unwrap();
        render(&mut read_only, 400., &mut fonts);
        tab(&mut read_only);
        assert_eq!(read_only.focused(), Some(ID));
        read_only
            .set_display_contrast(force_contrast(Some(SYSTEM_INK)))
            .unwrap();
        render(&mut read_only, 400., &mut fonts);
        let action = bounds(&read_only, TRAILING);
        read_only
            .dispatch(Event::PointerMove {
                x: action.x + action.width / 2.,
                y: action.y + action.height / 2.,
            })
            .unwrap();
        let focused = render(&mut read_only, 400., &mut fonts);
        assert_eq!(label_color(&focused, VALUE), colors.on_surface);
        assert_eq!(label_color(&focused, LABEL), colors.error);
        assert_glyph_ink(
            &focused,
            bounds(&read_only, "project-name-trailing-icon"),
            SYSTEM_INK,
        );
        assert_glyph_ink(
            &focused,
            bounds(&read_only, "project-name-leading-icon"),
            colors.on_surface_variant,
        );
        let editor = read_only.dom().by_id(ID).unwrap();
        let action = read_only.dom().by_id(TRAILING).unwrap();
        let frame = read_only.accessibility_frame().unwrap();
        assert!(frame.node(editor).unwrap().text_options.unwrap().read_only);
        assert!(frame.node(editor).unwrap().node.is_invalid());
        assert!(frame.node(action).unwrap().node.is_disabled());
        assert!(!frame.accepts_focus(action));
        assert_eq!(read_only.focused(), Some(ID));
    }
}

#[test]
fn expanding_label_hints_fade_without_overlapping_or_painting_during_ime() {
    for appearance in [TextFieldAppearance::Outlined, TextFieldAppearance::Filled] {
        let mut runtime = Runtime::new(Form {
            adorned: Some(appearance),
            dark: true,
            rtl: true,
            scale: 2.,
            ..Form::default()
        })
        .unwrap();
        let mut fonts = TextSystem::default();
        runtime.set_time(0.);
        assert_eq!(hint_alpha(&render(&mut runtime, 400., &mut fonts)), None);
        let editor = bounds(&runtime, ID);
        tab(&mut runtime);
        assert_eq!(hint_alpha(&render(&mut runtime, 400., &mut fonts)), None);

        runtime.set_time(8.);
        // The slow spring has positive alpha. The label still covers the
        // right-aligned hint, including the part left of the caret.
        assert_eq!(hint_alpha(&render(&mut runtime, 400., &mut fonts)), None);
        runtime.set_time(180.);
        assert_eq!(
            hint_alpha(&render(&mut runtime, 400., &mut fonts)),
            Some(245)
        );
        assert_eq!(bounds(&runtime, ID), editor);
        let id = runtime.dom().by_id(ID).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        let input = frame.node(id).unwrap().node;
        assert_eq!(input.name(), LABEL);
        assert_eq!(input.text_value(), Some(""));

        runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
        assert!(runtime.input_snapshot().unwrap().composing);
        assert_eq!(hint_alpha(&render(&mut runtime, 400., &mut fonts)), None);
        runtime.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();
        assert_eq!(
            hint_alpha(&render(&mut runtime, 400., &mut fonts)),
            Some(245)
        );

        runtime.set_reduced_motion(true);
        assert_eq!(
            hint_alpha(&render(&mut runtime, 400., &mut fonts)),
            Some(255)
        );
        tab(&mut runtime);
        assert_eq!(hint_alpha(&render(&mut runtime, 400., &mut fonts)), None);
    }
}

#[test]
fn above_label_shows_empty_idle_hint_and_fades_after_unfocused_clear() {
    for appearance in [TextFieldAppearance::Outlined, TextFieldAppearance::Filled] {
        let mut runtime = Runtime::new(Form {
            label: LONG_LABEL,
            adorned: Some(appearance),
            dark: true,
            rtl: true,
            scale: 2.,
            ..Form::default()
        })
        .unwrap();
        let mut fonts = TextSystem::default();
        runtime.set_time(0.);
        let idle = render(&mut runtime, 320., &mut fonts);
        let pose = label_pose(&idle, LONG_LABEL);
        assert_eq!(runtime.focused(), None);
        assert_eq!(hint_alpha(&idle), Some(255));

        runtime.apply(Message::Edit("已填".into())).unwrap();
        assert_eq!(hint_alpha(&render(&mut runtime, 320., &mut fonts)), None);
        runtime.set_time(400.);
        render(&mut runtime, 320., &mut fonts);
        runtime.apply(Message::Edit(String::new())).unwrap();
        assert_eq!(hint_alpha(&render(&mut runtime, 320., &mut fonts)), None);
        runtime.set_time(444.);
        let rising = render(&mut runtime, 320., &mut fonts);
        assert_eq!(hint_alpha(&rising), Some(90));
        assert_eq!(label_pose(&rising, LONG_LABEL), pose);

        runtime
            .apply(Message::SetEditability(TextFieldEditability::Disabled))
            .unwrap();
        let disabled = render(&mut runtime, 320., &mut fonts);
        assert_eq!(hint_alpha(&disabled), Some(97));
        let id = runtime.dom().by_id(ID).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        let input = frame.node(id).unwrap().node;
        assert_eq!(input.text_value(), Some(""));
        assert!(frame
            .node(
                runtime
                    .dom()
                    .by_id("project-name-disabled-placeholder")
                    .unwrap()
            )
            .is_none());
    }
}

#[test]
fn disabled_above_label_hint_keeps_single_line_clipping() {
    const OVERFLOW_HINT: &str = "Monitor Keyboard";
    let visible_clusters = |list: &ExtendedDisplayList, fonts: &mut TextSystem| {
        let Some((
            PaintOp::StyledText {
                rect,
                clip,
                text,
                style,
                scroll,
                multiline,
                ..
            },
            alignment,
        )) = text_run(list, OVERFLOW_HINT)
        else {
            panic!("an empty above-label field paints its hint");
        };
        assert!(
            fonts.measure_text("Keyboard", style, None).0 <= clip.width,
            "the second word alone fits in the visible row"
        );
        // Shape with the same width and origin as the display run. A true
        // line break can move a whole word out of the one visible row.
        let layout = if *multiline {
            fonts.text_layout_aligned_styled(text, style, rect.width, alignment)
        } else {
            fonts.text_layout_styled(text, style, None)
        };
        layout
            .clusters
            .into_iter()
            .filter_map(|cluster| {
                let bounds = cluster.bounds;
                let visible = Rect::new(
                    rect.x - scroll.0 + bounds.x,
                    rect.y - scroll.1 + bounds.y,
                    bounds.width,
                    bounds.height,
                )
                .intersect(*clip);
                (visible.width > 0. && visible.height > 0.).then_some(cluster.bytes.start)
            })
            .collect::<Vec<_>>()
    };

    for (appearance, rtl) in [
        (TextFieldAppearance::Outlined, false),
        (TextFieldAppearance::Filled, false),
        (TextFieldAppearance::Outlined, true),
        (TextFieldAppearance::Filled, true),
    ] {
        let mut fonts = TextSystem::default();
        let mut runtime = Runtime::new(Form {
            label: LONG_LABEL,
            hint: OVERFLOW_HINT,
            rtl,
            adorned: Some(appearance),
            ..Form::default()
        })
        .unwrap();
        runtime.set_time(0.);
        let idle = render(&mut runtime, 260., &mut fonts);
        assert_eq!(bounds(&runtime, "project-name-floating-label").width, 0.);
        let field = bounds(&runtime, ID);
        let expected = visible_clusters(&idle, &mut fonts);
        if rtl {
            assert!(
                expected.contains(&(OVERFLOW_HINT.len() - 1)) && !expected.contains(&0),
                "the RTL value area keeps the physical-right end: {expected:?}"
            );
        } else {
            assert!(
                expected.contains(&"Monitor ".len())
                    && !expected.contains(&(OVERFLOW_HINT.len() - 1)),
                "the LTR value area clips the end of the second word: {expected:?}"
            );
        }

        runtime.set_time(16.);
        runtime
            .apply(Message::SetEditability(TextFieldEditability::Disabled))
            .unwrap();
        let disabled = render(&mut runtime, 260., &mut fonts);
        assert_eq!(bounds(&runtime, ID), field);
        assert_eq!(
            visible_clusters(&disabled, &mut fonts),
            expected,
            "{appearance:?} rtl={rtl} disabled hint preserves the visible part of the same line"
        );

        runtime.set_time(32.);
        runtime
            .apply(Message::SetEditability(TextFieldEditability::Editable))
            .unwrap();
        let enabled = render(&mut runtime, 260., &mut fonts);
        assert_eq!(bounds(&runtime, ID), field);
        assert_eq!(visible_clusters(&enabled, &mut fonts), expected);
    }
}

#[test]
fn floating_label_reverses_from_the_same_pose_and_reduced_motion_settles_it() {
    for appearance in [TextFieldAppearance::Outlined, TextFieldAppearance::Filled] {
        let mut runtime = Runtime::new(Form {
            adorned: Some(appearance),
            ..Form::default()
        })
        .unwrap();
        let mut fonts = TextSystem::default();
        runtime.set_time(0.);
        let start = render(&mut runtime, 400., &mut fonts);
        let resting = label_pose(&start, LABEL);
        let editor = bounds(&runtime, ID);
        let target = bounds(&runtime, "project-name-floating-label");
        let action = bounds(&runtime, TRAILING);

        tab(&mut runtime);
        let first = render(&mut runtime, 400., &mut fonts);
        assert_eq!(label_pose(&first, LABEL), resting);
        assert!(
            text_run(&first, HINT).is_none(),
            "the resting label still owns the value row"
        );
        assert!(runtime.needs_frame());
        runtime.set_time(27.);
        let rising = render(&mut runtime, 400., &mut fonts);
        let moving = label_pose(&rising, LABEL);
        assert!(moving.0.y < resting.0.y && moving.0.y > target.y);
        assert!(moving.1 < resting.1 && moving.1 > 12.);
        assert_eq!(bounds(&runtime, ID), editor);
        assert_eq!(bounds(&runtime, TRAILING), action);
        assert_eq!(bounds(&runtime, "project-name-floating-label"), target);
        let frame = runtime.accessibility_frame().unwrap();
        assert_eq!(
            frame
                .node(runtime.dom().by_id(ID).unwrap())
                .unwrap()
                .node
                .name(),
            LABEL
        );
        assert!(frame
            .node(runtime.dom().by_id("project-name-floating-label").unwrap())
            .is_none());

        tab(&mut runtime);
        let reversal = render(&mut runtime, 400., &mut fonts);
        assert_eq!(label_pose(&reversal, LABEL), moving);
        runtime.set_time(78.);
        let falling = render(&mut runtime, 400., &mut fonts);
        assert!(label_pose(&falling, LABEL).0.y > moving.0.y);

        runtime
            .dispatch(Event::FocusNext { reverse: true })
            .unwrap();
        let _ = render(&mut runtime, 400., &mut fonts);
        runtime.set_reduced_motion(true);
        let reduced = render(&mut runtime, 400., &mut fonts);
        assert_eq!(label_pose(&reduced, LABEL), (target, 12.));
        assert!(text_run(&reduced, HINT).is_some());
        assert!(!runtime.needs_frame());
    }
}

#[test]
fn outlined_label_and_early_notch_share_one_frame_and_large_rtl_keeps_fallback() {
    let mut fonts = TextSystem::default();
    let mut runtime = Runtime::new(Form::default()).unwrap();
    runtime.set_time(0.);
    let resting = label_pose(&render(&mut runtime, 360., &mut fonts), LABEL);
    tab(&mut runtime);
    render(&mut runtime, 360., &mut fonts);
    runtime.set_time(8.);
    let frame = render(&mut runtime, 360., &mut fonts);
    let face = bounds(&runtime, "project-name-outline");
    let left = bounds(&runtime, "project-name-floating-label").x;
    let outline = bottom_boundary(&frame, face);
    assert!(label_pose(&frame, LABEL).0.y < resting.0.y);
    assert!(!covers(outline, left, face.y + 0.1), "start of the cutout");
    assert!(covers(outline, left, face.y + 0.95), "lower stroke remains");
    assert!(!covers(outline, left, face.y + 1.25));
    assert!(
        covers(outline, face.x + face.width / 2., face.y + 0.1),
        "outside the cutout"
    );

    runtime.dispatch(Event::Text("中".into())).unwrap();
    let edited = render(&mut runtime, 360., &mut fonts);
    assert!(
        text_run(&edited, "中").is_some(),
        "input can paint during motion"
    );
    assert!(text_run(&edited, HINT).is_none());

    let mut short_rtl = Runtime::new(Form {
        dark: true,
        rtl: true,
        scale: 2.,
        adorned: Some(TextFieldAppearance::Outlined),
        ..Form::default()
    })
    .unwrap();
    short_rtl.set_time(0.);
    let start = label_pose(&render(&mut short_rtl, 320., &mut fonts), LABEL).0;
    let end = bounds(&short_rtl, "project-name-floating-label");
    tab(&mut short_rtl);
    render(&mut short_rtl, 320., &mut fonts);
    short_rtl.set_time(26.);
    let (middle, font) = label_pose(&render(&mut short_rtl, 320., &mut fonts), LABEL);
    assert!(middle.x + middle.width > start.x + start.width);
    assert!(middle.x + middle.width < end.x + end.width);
    assert!(font > 24. && font < 32.);

    let mut rtl = Runtime::new(Form {
        label: LONG_LABEL,
        dark: true,
        rtl: true,
        scale: 2.,
        ..Form::default()
    })
    .unwrap();
    rtl.set_time(0.);
    let before = label_pose(&render(&mut rtl, 320., &mut fonts), LONG_LABEL);
    tab(&mut rtl);
    render(&mut rtl, 320., &mut fonts);
    rtl.set_time(31.);
    let focused = render(&mut rtl, 320., &mut fonts);
    assert_eq!(label_pose(&focused, LONG_LABEL), before);
    let face = bounds(&rtl, "project-name-outline");
    let outline = bottom_boundary(&focused, face);
    assert!(covers(outline, face.x + face.width / 2., face.y + 1.));
    assert!(rtl.needs_frame(), "the fallback still animates label color");
    let ink = label_color(&focused, LONG_LABEL);
    let widened = render(&mut rtl, 1000., &mut fonts);
    assert_eq!(label_color(&widened, LONG_LABEL), ink);
    assert_eq!(
        label_pose(&widened, LONG_LABEL).0,
        bounds(&rtl, "project-name-floating-label")
    );
    let narrowed = render(&mut rtl, 320., &mut fonts);
    assert_eq!(label_pose(&narrowed, LONG_LABEL), before);
    assert_eq!(label_color(&narrowed, LONG_LABEL), ink);
    rtl.set_time(350.);
    let settled = render(&mut rtl, 320., &mut fonts);
    assert_eq!(label_pose(&settled, LONG_LABEL), before);
    assert_eq!(
        label_color(&settled, LONG_LABEL),
        Theme::dark().colors.primary
    );
    assert!(!rtl.needs_frame());
    rtl.apply(Message::ChangeAppearance).unwrap();
    let light = render(&mut rtl, 320., &mut fonts);
    assert_eq!(label_color(&light, LONG_LABEL), Theme::light().colors.error);
    assert_eq!(
        bottom_boundary(&light, bounds(&rtl, "project-name-outline")).color(),
        Theme::light().colors.error
    );
    // The separate Next button can still animate its own theme colors.
    rtl.set_time(750.);
    let light = render(&mut rtl, 320., &mut fonts);
    assert_eq!(label_color(&light, LONG_LABEL), Theme::light().colors.error);
    assert!(!rtl.needs_frame());
}

#[test]
fn field_boundaries_follow_distinct_width_curves_and_reverse_from_the_presented_pose() {
    let colors = Theme::light().colors;
    // Independent analytic Standard motion samples at 24 and 48 ms.
    for (appearance, boundary, idle_ink, at_24, at_48) in [
        (
            TextFieldAppearance::Outlined,
            "project-name-outline",
            colors.outline,
            1.870_553,
            2.589_262,
        ),
        (
            TextFieldAppearance::Filled,
            "project-name-indicator",
            colors.on_surface_variant,
            1.473_985,
            2.139_934,
        ),
    ] {
        let mut fonts = TextSystem::default();
        let mut runtime = Runtime::new(Form {
            adorned: Some(appearance),
            value: "欄位內容".into(),
            ..Form::default()
        })
        .unwrap();
        runtime.set_time(0.);
        let idle = render(&mut runtime, 400., &mut fonts);
        let face = bounds(&runtime, boundary);
        let editor_bounds = bounds(&runtime, ID);
        let editor_id = runtime.dom().by_id(ID).unwrap();
        let idle_path = bottom_boundary(&idle, face);
        assert_eq!(idle_path.color(), idle_ink);
        assert_bottom_width(idle_path, face, 1.);

        tab(&mut runtime);
        let first = render(&mut runtime, 400., &mut fonts);
        let first_path = bottom_boundary(&first, face);
        assert_eq!(first_path.points(), idle_path.points());
        assert_eq!(first_path.color(), idle_ink);
        assert!(runtime.needs_frame());
        runtime.set_time(24.);
        let rising = render(&mut runtime, 400., &mut fonts);
        let rising_path = bottom_boundary(&rising, face);
        assert_bottom_width(rising_path, face, at_24);
        assert_ne!(rising_path.color(), idle_ink);
        assert_ne!(rising_path.color(), colors.primary);
        assert_eq!(bounds(&runtime, ID), editor_bounds);
        assert_eq!(runtime.dom().by_id(ID), Some(editor_id));
        assert_eq!(runtime.focused(), Some(ID));

        runtime.set_time(48.);
        let later = render(&mut runtime, 400., &mut fonts);
        let later_path = bottom_boundary(&later, face);
        assert_bottom_width(later_path, face, at_48);
        assert_ne!(later_path.color(), rising_path.color());
        tab(&mut runtime);
        let reversal = render(&mut runtime, 400., &mut fonts);
        let reversal_path = bottom_boundary(&reversal, face);
        assert_eq!(reversal_path.points(), later_path.points());
        assert_eq!(reversal_path.color(), later_path.color());
        runtime.set_time(100.);
        let falling = render(&mut runtime, 400., &mut fonts);
        let falling_path = bottom_boundary(&falling, face);
        assert!(covers(
            falling_path,
            face.x + 30.,
            face.y + face.height - 1.02
        ));
        assert!(!covers(
            falling_path,
            face.x + 30.,
            face.y + face.height - at_48
        ));
        runtime.set_time(650.);
        let settled = render(&mut runtime, 400., &mut fonts);
        let settled_path = bottom_boundary(&settled, face);
        assert_eq!(settled_path.points(), idle_path.points());
        assert_eq!(settled_path.color(), idle_ink);
        assert!(!runtime.needs_frame());
    }
}

#[test]
fn field_boundaries_use_hover_and_error_ink_and_snap_on_disable_or_reduced_motion() {
    let colors = Theme::light().colors;
    for (appearance, boundary, disabled_alpha) in [
        (TextFieldAppearance::Outlined, "project-name-outline", 31),
        (TextFieldAppearance::Filled, "project-name-indicator", 97),
    ] {
        let mut fonts = TextSystem::default();
        let mut runtime = Runtime::new(Form {
            adorned: Some(appearance),
            ..Form::default()
        })
        .unwrap();
        runtime.set_time(0.);
        render(&mut runtime, 400., &mut fonts);
        let face = bounds(&runtime, boundary);
        runtime
            .dispatch(Event::PointerMove {
                x: face.x + 8.,
                y: face.y + face.height / 2.,
            })
            .unwrap();
        render(&mut runtime, 400., &mut fonts);
        runtime.set_time(350.);
        let hover = render(&mut runtime, 400., &mut fonts);
        let hover_path = bottom_boundary(&hover, face);
        assert_eq!(hover_path.color(), colors.on_surface);
        assert_bottom_width(hover_path, face, 1.);

        runtime.apply(Message::SetInvalid(true)).unwrap();
        let error_start = render(&mut runtime, 400., &mut fonts);
        assert_eq!(
            bottom_boundary(&error_start, face).color(),
            colors.on_surface
        );
        runtime.set_time(410.);
        let error_middle = render(&mut runtime, 400., &mut fonts);
        let error_ink = bottom_boundary(&error_middle, face).color();
        assert_ne!(error_ink, colors.on_surface);
        assert_ne!(error_ink, colors.on_error_container);
        runtime.set_time(650.);
        let error_hover = render(&mut runtime, 400., &mut fonts);
        assert_eq!(
            bottom_boundary(&error_hover, face).color(),
            colors.on_error_container
        );

        tab(&mut runtime);
        render(&mut runtime, 400., &mut fonts);
        runtime.set_time(674.);
        let moving = render(&mut runtime, 400., &mut fonts);
        assert_ne!(bottom_boundary(&moving, face).color(), colors.error);
        runtime
            .apply(Message::SetEditability(TextFieldEditability::Disabled))
            .unwrap();
        let disabled = render(&mut runtime, 400., &mut fonts);
        let disabled_path = bottom_boundary(&disabled, face);
        let Color(r, g, b, _) = colors.on_surface;
        assert_eq!(disabled_path.color(), Color(r, g, b, disabled_alpha));
        assert_bottom_width(disabled_path, face, 1.);
        assert_ne!(runtime.focused(), Some(ID));
        assert!(!runtime
            .accessibility_frame()
            .unwrap()
            .node(runtime.dom().by_id(ID).unwrap())
            .unwrap()
            .node
            .is_invalid());

        let mut reduced = Runtime::new(Form {
            adorned: Some(appearance),
            invalid: true,
            ..Form::default()
        })
        .unwrap();
        reduced.set_time(0.);
        reduced.set_reduced_motion(true);
        render(&mut reduced, 400., &mut fonts);
        tab(&mut reduced);
        let focused = render(&mut reduced, 400., &mut fonts);
        let path = bottom_boundary(&focused, face);
        assert_eq!(path.color(), colors.error);
        assert_bottom_width(path, face, 3.);
        tab(&mut reduced);
        let blurred = render(&mut reduced, 400., &mut fonts);
        assert_bottom_width(bottom_boundary(&blurred, face), face, 1.);
        assert!(!reduced.needs_frame());
    }
}

#[test]
fn field_boundary_uses_enabled_ink_on_first_frame_after_reenable() {
    let colors = Theme::light().colors;
    for (appearance, boundary, idle_ink) in [
        (
            TextFieldAppearance::Outlined,
            "project-name-outline",
            colors.outline,
        ),
        (
            TextFieldAppearance::Filled,
            "project-name-indicator",
            colors.on_surface_variant,
        ),
    ] {
        for invalid in [false, true] {
            let mut fonts = TextSystem::default();
            let mut runtime = Runtime::new(Form {
                adorned: Some(appearance),
                invalid,
                ..Form::default()
            })
            .unwrap();
            runtime.set_time(0.);
            render(&mut runtime, 400., &mut fonts);
            let face = bounds(&runtime, boundary);
            runtime
                .apply(Message::SetEditability(TextFieldEditability::Disabled))
                .unwrap();
            let disabled = render(&mut runtime, 400., &mut fonts);
            let disabled_ink = bottom_boundary(&disabled, face).color();
            runtime.set_time(16.);
            runtime
                .apply(Message::SetEditability(TextFieldEditability::Editable))
                .unwrap();
            let enabled = render(&mut runtime, 400., &mut fonts);
            let expected = if invalid { colors.error } else { idle_ink };
            assert_ne!(disabled_ink, expected);
            assert_eq!(
                bottom_boundary(&enabled, face).color(),
                expected,
                "{appearance:?}, invalid={invalid}: enabled ink on first frame"
            );
            assert_bottom_width(bottom_boundary(&enabled, face), face, 1.);
        }
    }
}

#[test]
fn label_color_retargets_on_focus_and_error_without_changing_its_pose() {
    let colors = Theme::light().colors;
    for appearance in [TextFieldAppearance::Outlined, TextFieldAppearance::Filled] {
        let mut fonts = TextSystem::default();
        let mut runtime = Runtime::new(Form {
            adorned: Some(appearance),
            value: "欄位內容".into(),
            ..Form::default()
        })
        .unwrap();
        runtime.set_time(0.);
        let idle = render(&mut runtime, 400., &mut fonts);
        let pose = label_pose(&idle, LABEL);
        assert_eq!(label_color(&idle, LABEL), colors.on_surface_variant);

        tab(&mut runtime);
        let focus = render(&mut runtime, 400., &mut fonts);
        assert_eq!(label_color(&focus, LABEL), colors.on_surface_variant);
        runtime.set_time(24.);
        let rising = render(&mut runtime, 400., &mut fonts);
        let ink = label_color(&rising, LABEL);
        assert_ne!(ink, colors.on_surface_variant);
        assert_ne!(ink, colors.primary);
        assert_eq!(label_pose(&rising, LABEL), pose);
        assert!(runtime.needs_frame());

        runtime.apply(Message::SetInvalid(true)).unwrap();
        let retargeted = render(&mut runtime, 400., &mut fonts);
        assert_eq!(label_color(&retargeted, LABEL), ink);
        runtime.set_time(46.);
        let invalid = render(&mut runtime, 400., &mut fonts);
        assert_ne!(label_color(&invalid, LABEL), colors.error);
        assert_ne!(label_color(&invalid, LABEL), ink);
        assert_eq!(label_pose(&invalid, LABEL), pose);
        let editor = runtime.dom().by_id(ID).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        let input = frame.node(editor).unwrap().node;
        assert!(input.is_invalid());
        assert_eq!(input.name(), LABEL);
        assert_eq!(input.text_value(), Some("欄位內容"));

        runtime.set_reduced_motion(true);
        let reduced = render(&mut runtime, 400., &mut fonts);
        assert_eq!(label_color(&reduced, LABEL), colors.error);
        assert!(!runtime.needs_frame());
        runtime.set_reduced_motion(false);
        runtime
            .apply(Message::SetEditability(TextFieldEditability::Disabled))
            .unwrap();
        let disabled = render(&mut runtime, 400., &mut fonts);
        let Color(r, g, b, _) = colors.on_surface;
        assert_eq!(label_color(&disabled, LABEL), Color(r, g, b, 97));
        assert!(!runtime.needs_frame());

        runtime.set_time(62.);
        runtime
            .apply(Message::SetEditability(TextFieldEditability::Editable))
            .unwrap();
        let first_enabled_frame = render(&mut runtime, 400., &mut fonts);
        assert_eq!(
            label_color(&first_enabled_frame, LABEL),
            colors.error,
            "{appearance:?} label uses enabled ink on the first frame"
        );
        assert!(!runtime.needs_frame());
    }
}

#[test]
fn growing_multiline_preserves_first_row_and_measures_preedit_and_committed_lines() {
    for (appearance, boundary) in [
        (TextFieldAppearance::Outlined, "project-name-outline"),
        (TextFieldAppearance::Filled, "project-name-indicator"),
    ] {
        let mut fonts = TextSystem::default();
        let mut single = Runtime::new(Form {
            adorned: Some(appearance),
            ..Form::default()
        })
        .unwrap();
        render(&mut single, 400., &mut fonts);
        let first_row = bounds(&single, BODY);
        let baseline_face = bounds(&single, boundary);
        let mut runtime = Runtime::new(Form {
            adorned: Some(appearance),
            multiline: Some(MultilineTextFieldSize::Grow),
            character_limit: Some(8),
            ..Form::default()
        })
        .unwrap();
        render(&mut runtime, 400., &mut fonts);
        assert_eq!(bounds(&runtime, BODY), first_row);
        assert_eq!(bounds(&runtime, boundary), baseline_face);
        let first_action = bounds(&runtime, TRAILING);
        let next_y = bounds(&runtime, AFTER).y;
        let editor = runtime.dom().by_id(ID).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        assert!(frame.node(editor).unwrap().text_options.unwrap().multiline);
        tab(&mut runtime);
        assert_eq!(runtime.focused(), Some(ID));
        assert!(
            !runtime.key(&enter()).unwrap(),
            "Enter has no submit binding"
        );
        runtime.dispatch(Event::Text("中\n".into())).unwrap();
        let appended = render(&mut runtime, 400., &mut fonts);
        assert_eq!(runtime.app().value, "中\n");
        assert_eq!(runtime.app().edits, ["中\n"]);
        assert!(runtime.app().submits.is_empty());
        assert!(text_run(&appended, "2/8").is_some());
        assert_eq!(bounds(&runtime, BODY).y, first_row.y);
        assert_eq!(bounds(&runtime, BODY).height, 2. * first_row.height);
        assert_eq!(
            bounds(&runtime, boundary).height,
            baseline_face.height + first_row.height
        );
        assert_eq!(bounds(&runtime, AFTER).y, next_y + first_row.height);
        assert_eq!(bounds(&runtime, TRAILING), first_action);
        let caret = runtime.input_snapshot().unwrap().caret.unwrap();
        assert_eq!(caret.y, first_row.y + first_row.height);

        runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
        runtime
            .dispatch(Event::Ime(ImeEvent::Preedit {
                text: "文".repeat(60),
                selection: None,
            }))
            .unwrap();
        let marked = render(&mut runtime, 400., &mut fonts);
        assert!(bounds(&runtime, BODY).height > 2. * first_row.height);
        assert!(text_run(&marked, "2/8").is_some());
        assert_eq!(runtime.app().edits, ["中\n"]);
        runtime.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();
        render(&mut runtime, 400., &mut fonts);
        assert_eq!(bounds(&runtime, BODY).height, 2. * first_row.height);
        assert_eq!(runtime.focused(), Some(ID));
        assert_eq!(
            runtime.input_snapshot().unwrap().selection.cursor,
            "中\n".len()
        );
    }
}

#[test]
fn fixed_rows_clip_body_without_moving_adornments_or_footer() {
    for appearance in [TextFieldAppearance::Outlined, TextFieldAppearance::Filled] {
        for rows in [1, 3] {
            let mut fonts = TextSystem::default();
            let mut runtime = Runtime::new(Form {
                adorned: Some(appearance),
                multiline: Some(MultilineTextFieldSize::FixedRows(
                    NonZeroU16::new(rows).unwrap(),
                )),
                character_limit: Some(100),
                ..Form::default()
            })
            .unwrap();
            render(&mut runtime, 400., &mut fonts);
            let body = bounds(&runtime, BODY);
            let footer = bounds(&runtime, HELP);
            let action = bounds(&runtime, TRAILING);
            let field = bounds(&runtime, ID);
            assert_eq!(body.height, f32::from(rows) * 24.);
            assert!(footer.y >= field.y + field.height + 4.);
            assert_eq!(field.intersect(action).width, 0.);
            tab(&mut runtime);
            let value = "一\n二\n三\n四\n五\n末";
            runtime.dispatch(Event::Text(value.into())).unwrap();
            let display = render(&mut runtime, 400., &mut fonts);
            assert_eq!(bounds(&runtime, BODY), body);
            let updated_footer = bounds(&runtime, HELP);
            assert_eq!(
                (updated_footer.y, updated_footer.height),
                (footer.y, footer.height)
            );
            assert_eq!(bounds(&runtime, TRAILING), action);
            assert_eq!(bounds(&runtime, ID), field);
            let (run, alignment) = text_run(&display, value).unwrap();
            assert_eq!(alignment, TextAlignment::Left);
            assert!(matches!(run, PaintOp::StyledText {
                rect, clip, multiline: true, editable: true, scroll: (_, offset), ..
            } if *rect == body && *clip == body && *offset == (6. - f32::from(rows)) * 24.));
            let caret = runtime.input_snapshot().unwrap().caret.unwrap();
            assert_eq!(caret.y + caret.height, body.y + body.height);
            tab(&mut runtime);
            assert_eq!(runtime.focused(), Some(TRAILING));
        }
    }
}

#[test]
fn narrow_rtl_multiline_relocates_action_and_keeps_disabled_and_read_only_values() {
    const VALUE: &str = "輸入\n收尾";
    for appearance in [TextFieldAppearance::Outlined, TextFieldAppearance::Filled] {
        let mut fonts = TextSystem::default();
        let mut runtime = Runtime::new(Form {
            label: LONG_LABEL,
            value: VALUE.into(),
            adorned: Some(appearance),
            multiline: Some(MultilineTextFieldSize::FixedRows(
                NonZeroU16::new(3).unwrap(),
            )),
            character_limit: Some(4),
            editability: TextFieldEditability::ReadOnly,
            rtl: true,
            dark: true,
            scale: 2.,
            ..Form::default()
        })
        .unwrap();
        let idle = render(&mut runtime, 136., &mut fonts);
        let body = bounds(&runtime, BODY);
        let field = bounds(&runtime, ID);
        let help = bounds(&runtime, HELP);
        let count = bounds(&runtime, COUNTER);
        let action = bounds(&runtime, TRAILING);
        assert_eq!(field.width, 112.);
        assert_eq!(body.height, 3. * 48.);
        assert!(field.y > bounds(&runtime, "project-name-above-label").y);
        assert!(count.y >= help.y + help.height + 4.);
        assert!(count.x < body.x + body.width / 2.);
        assert!(action.y >= count.y + count.height + 4.);
        assert_eq!(action.x, field.x);
        assert!(matches!(text_run(&idle, VALUE), Some((
            PaintOp::StyledText { rect, multiline: true, editable: true, .. },
            TextAlignment::Right
        )) if *rect == body));
        tab(&mut runtime);
        runtime.dispatch(Event::Text("\n".into())).unwrap();
        assert_eq!(runtime.app().value, VALUE);
        assert!(runtime.app().edits.is_empty() && runtime.app().submits.is_empty());

        runtime
            .apply(Message::SetEditability(TextFieldEditability::Disabled))
            .unwrap();
        let disabled = render(&mut runtime, 136., &mut fonts);
        let (disabled_rect, disabled_clip) = disabled
            .ops()
            .iter()
            .find_map(|op| match op {
                ExtendedPaintOp::AlignedText(paint)
                    if paint.alignment() == TextAlignment::Right =>
                {
                    match paint.run() {
                        PaintOp::StyledText {
                            rect,
                            clip,
                            text,
                            editable: false,
                            ..
                        } if text == VALUE => Some((*rect, *clip)),
                        _ => None,
                    }
                }
                _ => None,
            })
            .expect("right-aligned passive value");
        assert_eq!(disabled_rect, body);
        assert_eq!(disabled_clip, body.intersect(disabled.viewport()));
        let editor = runtime.dom().by_id(ID).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        let source = frame.node(editor).unwrap();
        assert!(source.node.is_disabled());
        assert!(!source.node.is_invalid());
        assert_eq!(source.node.text_value(), Some(VALUE));
        assert!(frame.focus_target(editor).is_none());
    }
}

#[test]
fn counter_counts_committed_graphemes_and_exposes_one_soft_limit_error() {
    let initial = "e\u{301}👩‍💻";
    let mut runtime = Runtime::new(Form {
        value: initial.into(),
        adorned: Some(TextFieldAppearance::Outlined),
        character_limit: Some(2),
        ..Form::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    let idle = render(&mut runtime, 400., &mut text);
    assert!(text_run(&idle, "2/2").is_some());
    let editor_id = runtime.dom().by_id(ID).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(!frame.node(editor_id).unwrap().node.is_invalid());
    let descriptions: Vec<_> = frame
        .description_parts(editor_id)
        .into_iter()
        .map(|(target, _)| {
            let node = frame.node(target).unwrap().node;
            assert_eq!(node.role(), Role::Text);
            assert!(frame.focus_target(target).is_none());
            node.text_value().unwrap()
        })
        .collect();
    assert_eq!(
        descriptions,
        ["歐元", "每月", "已輸入2個字元，上限2個。", SUPPORT]
    );

    tab(&mut runtime);
    runtime
        .dispatch(Event::Edit {
            key: EditKey::End,
            extend: false,
            by_word: false,
        })
        .unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄓㄨㄥ".into(),
            selection: None,
        }))
        .unwrap();
    let composing = render(&mut runtime, 400., &mut text);
    assert!(text_run(&composing, "2/2").is_some());
    assert!(!runtime.key(&enter()).unwrap());
    assert!(runtime.app().edits.is_empty() && runtime.app().submits.is_empty());
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("中".into())))
        .unwrap();
    let overflow = render(&mut runtime, 400., &mut text);
    assert!(text_run(&overflow, "3/2").is_some());
    assert_eq!(runtime.app().value, format!("{initial}中"));
    assert_eq!(runtime.app().edits, [format!("{initial}中")]);
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.node(editor_id).unwrap().node.is_invalid());
    let status_id = runtime
        .dom()
        .by_id("project-name-validation-status")
        .unwrap();
    assert_eq!(
        frame.node(status_id).unwrap().node.text_value(),
        Some(COUNT_ERROR)
    );
    assert!(runtime.key(&enter()).unwrap());
    assert_eq!(runtime.app().submits, [format!("{initial}中")]);

    runtime.apply(Message::ChangeAppearance).unwrap();
    render(&mut runtime, 400., &mut text);
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(
        frame.node(status_id).unwrap().node.text_value(),
        Some(ERROR)
    );
    assert_eq!(runtime.focused(), Some(ID));
}

#[test]
fn counter_and_support_share_wide_rows_and_stack_in_compact_rtl() {
    let mut text = TextSystem::default();
    for appearance in [TextFieldAppearance::Outlined, TextFieldAppearance::Filled] {
        let mut runtime = Runtime::new(Form {
            adorned: Some(appearance),
            character_limit: Some(0),
            ..Form::default()
        })
        .unwrap();
        let wide = render(&mut runtime, 400., &mut text);
        assert!(text_run(&wide, "0/0").is_some());
        let counter = bounds(&runtime, COUNTER);
        let support = bounds(&runtime, "project-name-description");
        assert_eq!(support.y, counter.y);
        assert_eq!(counter.x - support.x - support.width, 16.);
        assert_eq!(counter.x + counter.width, 372.);

        runtime.apply(Message::ChangeAppearance).unwrap();
        let manual_error = render(&mut runtime, 320., &mut text);
        assert!(matches!(
            text_run(&manual_error, "0/0"),
            Some((PaintOp::StyledText { color, .. }, _))
                if *color == Theme::dark().colors.on_surface_variant
        ));
        runtime.apply(Message::Edit("中".into())).unwrap();
        let compact = render(&mut runtime, 112., &mut text);
        let counter = bounds(&runtime, COUNTER);
        let support = bounds(&runtime, "project-name-description");
        assert_eq!(counter.y, support.y + support.height + 4.);
        assert_eq!(counter.x, support.x);
        assert!(counter.x + counter.width <= support.x + support.width);
        assert!(matches!(
            text_run(&compact, "1/0"),
            Some((PaintOp::StyledText { color, .. }, TextAlignment::Left))
                if *color == Theme::dark().colors.error
        ));

        runtime
            .apply(Message::SetEditability(TextFieldEditability::Disabled))
            .unwrap();
        let disabled = render(&mut runtime, 112., &mut text);
        let Color(r, g, b, _) = Theme::dark().colors.on_surface;
        assert!(matches!(
            text_run(&disabled, "1/0"),
            Some((PaintOp::StyledText { color, .. }, _)) if *color == Color(r, g, b, 97)
        ));
    }
}

#[test]
fn adornments_keep_one_editor_and_a_separate_action_across_both_variants() {
    const ACTION: &str = "project-name-trailing-action";
    const PREFIX: &str = "project-name-prefix-visual";
    const SUFFIX: &str = "project-name-suffix-visual";
    const VALUE: &str = "project-name-value-box";
    let mut text = TextSystem::default();
    for (appearance, border) in [
        (TextFieldAppearance::Outlined, "project-name-outline"),
        (TextFieldAppearance::Filled, "project-name-indicator"),
    ] {
        let mut runtime = Runtime::new(Form {
            adorned: Some(appearance),
            ..Form::default()
        })
        .unwrap();
        let idle = render(&mut runtime, 340., &mut text);
        let face = bounds(&runtime, border);
        let editor = bounds(&runtime, ID);
        let action = bounds(&runtime, ACTION);
        let leading = bounds(&runtime, "project-name-leading-icon");
        let trailing = bounds(&runtime, "project-name-trailing-icon");
        assert_eq!((action.width, action.height), (48., 48.));
        assert_eq!(action.x, editor.x + editor.width);
        assert_eq!(action.x + action.width, face.x + face.width);
        assert_eq!(
            (leading.x - face.x, leading.width, trailing.width),
            (12., 24., 24.)
        );
        assert_eq!(trailing.x - action.x, 12.);
        assert!(text_run(&idle, "€").is_none());

        let editor_id = runtime.dom().by_id(ID).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        let description: Vec<_> = frame
            .description_parts(editor_id)
            .into_iter()
            .map(|(id, _)| frame.node(id).unwrap().node.text_value().unwrap())
            .collect();
        assert_eq!(description, ["歐元", "每月", SUPPORT]);

        runtime
            .dispatch(Event::PointerMove {
                x: action.x + 24.,
                y: action.y + 24.,
            })
            .unwrap();
        let hover = render(&mut runtime, 340., &mut text);
        let stroke_bounds = if appearance == TextFieldAppearance::Filled {
            Rect::new(face.x, face.y + face.height - 1., face.width, 1.)
        } else {
            face
        };
        let shape = find_outline(&hover, Theme::light().colors.on_surface, stroke_bounds);
        assert!(covers(
            shape,
            face.x + face.width / 2.,
            face.y + face.height - 0.5
        ));
        runtime
            .dispatch(Event::PointerDown {
                x: leading.x + 12.,
                y: leading.y + 12.,
            })
            .unwrap();
        runtime
            .dispatch(Event::PointerUp {
                x: leading.x + 12.,
                y: leading.y + 12.,
            })
            .unwrap();
        assert_eq!(runtime.focused(), Some(ID));
        let focused = render(&mut runtime, 340., &mut text);
        assert!(text_run(&focused, "€").is_some());
        let prefix = bounds(&runtime, PREFIX);
        let suffix = bounds(&runtime, SUFFIX);
        let value = bounds(&runtime, VALUE);
        assert_eq!(prefix.x - face.x, 52.);
        assert!((value.x - (prefix.x + prefix.width) - 2.).abs() < 0.1);
        assert!((suffix.x - (value.x + value.width) - 2.).abs() < 0.1);

        runtime.dispatch(Event::Text("24".into())).unwrap();
        assert!(runtime.key(&enter()).unwrap());
        tab(&mut runtime);
        assert_eq!(runtime.focused(), Some(ACTION));
        assert!(!runtime.key(&enter()).unwrap());
        assert!(runtime.dispatch(Event::Activate).unwrap());
        assert_eq!(runtime.app().submits, ["24"]);
        assert_eq!(runtime.app().edits, ["24"]);
        assert_eq!(runtime.app().actions, 1);

        runtime.apply(Message::ChangeAppearance).unwrap();
        let narrow = render(&mut runtime, 112., &mut text);
        let face = bounds(&runtime, border);
        let action = bounds(&runtime, ACTION);
        assert!(action.y > face.y + face.height);
        assert_eq!(action.x, face.x);
        assert_eq!(action.width, 48.);
        assert_eq!(bounds(&runtime, "project-name-leading-icon").width, 0.);
        assert!(!narrow.ops().iter().any(|op| matches!(
            op,
            ExtendedPaintOp::FillPath(path)
                if path.color() == Theme::dark().colors.on_surface_variant
                    && path.rect().width == 16.
                    && path.rect().height == 16.
        )));
        assert!(bounds(&runtime, VALUE).width > 0.);
        assert!(text_run(&narrow, "每月").is_some());
        assert_eq!(runtime.focused(), Some(ACTION));
    }
}

#[test]
fn focus_repaints_one_semantic_editor_and_cuts_a_transparent_three_dp_outline() {
    let mut runtime = Runtime::new(Form::default()).unwrap();
    let mut text = TextSystem::default();
    let idle = render(&mut runtime, 400., &mut text);
    let face = bounds(&runtime, ID);
    assert_eq!(face.height, 56.);
    let label = bounds(&runtime, "project-name-floating-label");
    let notch_x = label.x + label.width / 2.;
    let colors = Theme::light().colors;
    let outline = find_outline(&idle, colors.outline, face);
    assert!(covers(outline, notch_x, face.y + 0.5));
    assert!(!covers(outline, notch_x, face.y + 1.5));
    assert_eq!(
        text_run(&idle, LABEL).unwrap().0.rect(),
        bounds(&runtime, "project-name-inline-label")
    );
    assert!(text_run(&idle, HINT).is_none());

    let builds = runtime.app().builds.get();
    tab(&mut runtime);
    let focus = render(&mut runtime, 400., &mut text);
    assert_eq!(runtime.focused(), Some(ID));
    assert_eq!(runtime.app().builds.get(), builds);
    assert_eq!(bounds(&runtime, ID), face);
    assert_eq!(text_run(&focus, LABEL).unwrap().0.rect(), label);
    assert!(text_run(&focus, HINT).is_some());
    let outline = find_outline(&focus, colors.primary, face);
    assert!(!covers(outline, notch_x, face.y + 0.5));
    assert!(covers(outline, label.x - 5., face.y + 2.5));
    assert!(covers(outline, label.x + label.width + 5., face.y + 2.5));
    assert!(!covers(outline, label.x - 5., face.y + 3.5));
    assert!(!focus.ops().iter().any(|op| matches!(
        op,
        ExtendedPaintOp::Legacy(PaintOp::Quad { rect, color, .. } | PaintOp::Shape { rect, color, .. })
            if color.3 > 0 && rect.contains(notch_x, face.y + 0.5)
    )));

    let frame = runtime.accessibility_frame().unwrap();
    let editor = runtime.dom().by_id(ID).unwrap();
    let shell = runtime.dom().by_id("project-name-field").unwrap();
    let description = runtime.dom().by_id("project-name-description").unwrap();
    let status = runtime
        .dom()
        .by_id("project-name-validation-status")
        .unwrap();
    assert_eq!(
        frame.children(shell).collect::<Vec<_>>(),
        [editor, description, status]
    );
    let live = frame.node(status).unwrap().node;
    assert_eq!(live.role(), Role::Status);
    assert!(live.is_assertive_live_status());
    assert_eq!(live.text_value(), Some(""));
    assert!(frame.focus_target(status).is_none());
    let projected = frame.node(editor).unwrap().node;
    assert_eq!(projected.name(), LABEL);
    assert_eq!(projected.role(), Role::TextInput);
    assert!(!projected.is_invalid());
    assert_eq!(frame.description_target(editor), Some(description));
    assert_eq!(
        frame.node(description).unwrap().node.text_value(),
        Some(SUPPORT)
    );
    assert_eq!(
        bounds(&runtime, "project-name-description").y,
        face.y + face.height + 4.
    );

    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄓ".into(),
            selection: None,
        }))
        .unwrap();
    let composing = render(&mut runtime, 400., &mut text);
    assert_eq!(runtime.input_snapshot().unwrap().value, "");
    assert!(text_run(&composing, HINT).is_none());
    runtime.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();
    assert!(text_run(&render(&mut runtime, 400., &mut text), HINT).is_some());
    assert_eq!(runtime.app().builds.get(), builds);
}

#[test]
fn same_id_theme_scale_and_validation_rebuild_preserve_backward_selection_and_preedit() {
    let mut runtime = Runtime::new(Form {
        value: "後界".into(),
        ..Form::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    render(&mut runtime, 500., &mut text);
    tab(&mut runtime);
    runtime
        .dispatch(Event::Select {
            anchor: 6,
            cursor: 3,
        })
        .unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Preedit {
            text: "ㄨㄣ".into(),
            selection: None,
        }))
        .unwrap();
    runtime.apply(Message::ChangeAppearance).unwrap();
    let paint = render(&mut runtime, 320., &mut text);

    let input = runtime.input_snapshot().unwrap();
    assert_eq!((input.id.as_str(), input.value.as_str()), (ID, "後界"));
    assert_eq!((input.selection.anchor, input.selection.cursor), (6, 3));
    assert!(input.composing);
    assert_eq!(
        runtime.editor(ID).unwrap().composition().unwrap().text,
        "ㄨㄣ"
    );
    assert!(text_run(&paint, HINT).is_none());
    let frame = runtime.accessibility_frame().unwrap();
    let editor = runtime.dom().by_id(ID).unwrap();
    assert!(frame.node(editor).unwrap().node.is_invalid());
    let description = frame.description_target(editor).unwrap();
    assert_eq!(
        frame.node(description).unwrap().node.text_value(),
        Some(ERROR)
    );
    let status = runtime
        .dom()
        .by_id("project-name-validation-status")
        .unwrap();
    let live = frame.node(status).unwrap().node;
    assert!(live.is_assertive_live_status());
    assert_eq!(live.text_value(), Some(ERROR));
    find_outline(&paint, Theme::dark().colors.error, bounds(&runtime, ID));

    assert!(!runtime.key(&enter()).unwrap());
    assert!(runtime.app().submits.is_empty());
    assert!(runtime.app().edits.is_empty());
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("文".into())))
        .unwrap();
    assert_eq!(runtime.app().edits, ["後文"]);
    assert!(!runtime.key(&enter().with_repeat(true)).unwrap());
    let shifted = KeyPress::new(
        Key::Enter,
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
    );
    assert!(!runtime.key(&shifted).unwrap());
    assert!(runtime.key(&enter()).unwrap());
    assert_eq!(runtime.app().submits, ["後文"]);
    assert_eq!(runtime.app().edits, ["後文"]);
}

#[test]
fn compact_rtl_long_label_wraps_above_a_closed_outline() {
    let mut runtime = Runtime::new(Form {
        label: LONG_LABEL,
        value: "共用名稱".into(),
        scale: 2.,
        rtl: true,
        dark: true,
        ..Form::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    render(&mut runtime, 320., &mut text);
    tab(&mut runtime);
    let paint = render(&mut runtime, 320., &mut text);
    let face = bounds(&runtime, ID);
    let above = bounds(&runtime, "project-name-above-label");
    assert_eq!(face.width, 296.);
    assert!(face.height >= 56.);
    assert_eq!(face.y - above.y - above.height, 4.);
    assert!(above.height > 32.);
    assert_eq!(above.x, face.x + 16.);
    assert_eq!(above.x + above.width, face.x + face.width - 16.);
    let (caption, alignment) = text_run(&paint, LONG_LABEL).unwrap();
    assert_eq!(caption.rect(), above);
    assert_eq!(alignment, TextAlignment::Right);
    let outline = find_outline(&paint, Theme::dark().colors.primary, face);
    for fraction in [0.1, 0.5, 0.9] {
        assert!(covers(
            outline,
            face.x + face.width * fraction,
            face.y + 1.5
        ));
    }
    let caret = runtime.input_snapshot().unwrap().caret.unwrap();
    assert!(caret.x > face.x + face.width / 2.);
    assert!(caret.x < face.x + face.width - 16.);
}

#[test]
fn read_only_selects_without_edits_and_disabled_leaves_the_tab_order() {
    let mut runtime = Runtime::new(Form {
        value: "既有專案".into(),
        invalid: true,
        editability: TextFieldEditability::ReadOnly,
        ..Form::default()
    })
    .unwrap();
    let mut text = TextSystem::default();
    render(&mut runtime, 320., &mut text);
    tab(&mut runtime);
    render(&mut runtime, 320., &mut text);
    let editor = runtime.dom().by_id(ID).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert!(frame.node(editor).unwrap().text_options.unwrap().read_only);
    assert!(frame.node(editor).unwrap().node.is_invalid());
    assert!(frame.accepts_focus(editor));
    runtime
        .dispatch(Event::Select {
            anchor: 12,
            cursor: 0,
        })
        .unwrap();
    assert_eq!(runtime.selected_text().as_deref(), Some("既有專案"));
    assert!(!runtime.key(&enter()).unwrap());
    runtime.dispatch(Event::Text("更改".into())).unwrap();
    runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("更".into())))
        .unwrap();
    assert_eq!(runtime.selected_text().as_deref(), Some("既有專案"));
    assert!(runtime.app().edits.is_empty());
    assert!(runtime.app().submits.is_empty());

    runtime
        .apply(Message::SetEditability(TextFieldEditability::Disabled))
        .unwrap();
    render(&mut runtime, 320., &mut text);
    let editor = runtime.dom().by_id(ID).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    let node = frame.node(editor).unwrap().node;
    assert!(node.is_disabled());
    assert!(!node.is_invalid());
    assert!(frame.description_target(editor).is_none());
    assert!(!frame.accepts_focus(editor));
    assert!(runtime.editor(ID).is_none());
    tab(&mut runtime);
    assert_eq!(runtime.focused(), Some(AFTER));
}
