use surface::{Color, ExtendedPaintOp, Length, Node, PaintOp, Rect, Role, Style, TextMeasurer};
use surface_app::{AccessibilityActivationResult, Application, Event, Runtime, View};
use surface_material::{
    shape, shape::LayoutDirection, typography, Elevation, SnackbarOptions, Theme,
};

#[derive(Clone)]
enum Message {
    Show,
    Undo,
    Close,
}

struct Notice {
    theme: Theme,
    text: &'static str,
    action: Option<&'static str>,
    dismiss: bool,
    visible: bool,
    scale: f32,
    layout: SnackbarOptions<'static>,
    undid: usize,
    closed: usize,
}

impl Default for Notice {
    fn default() -> Self {
        Self {
            theme: Theme::light(),
            text: "Saved",
            action: Some("Undo"),
            dismiss: true,
            visible: true,
            scale: 1.,
            layout: SnackbarOptions {
                restore_focus_to: Some("open"),
                ..SnackbarOptions::default()
            },
            undid: 0,
            closed: 0,
        }
    }
}

impl Application for Notice {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Show => self.visible = true,
            Message::Undo => self.undid += 1,
            Message::Close => {
                self.closed += 1;
                self.visible = false;
            }
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn view(&self) -> View<Message> {
        let mut children = vec![self.theme.button("open", "Show notice", Message::Show)];
        if self.visible {
            children.push(self.theme.snackbar_with(
                "notice",
                self.text,
                self.action.map(|label| (label, Message::Undo)),
                self.dismiss.then_some(("Dismiss notice", Message::Close)),
                self.layout,
            ));
        }
        View::new(
            Node::new(Role::Window, "Test").styled(Style::column().width(Length::Percent(1.))),
        )
        .children(children)
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let glyph_width = size * 0.75;
        let glyphs_per_line = width.map_or(usize::MAX, |w| ((w / glyph_width) as usize).max(1));
        let mut widest = 0;
        let mut rows = 0;
        for line in text.split('\n') {
            let glyphs = line.chars().count();
            widest = widest.max(glyphs.min(glyphs_per_line));
            rows += glyphs.div_ceil(glyphs_per_line).max(1);
        }
        let text_width = widest as f32 * glyph_width;
        (text_width, rows as f32 * size * 1.5)
    }
}

fn bounds(rt: &Runtime<Notice>, id: &str) -> Rect {
    rt.computed().unwrap().nodes[rt.dom().by_id(id).unwrap().index()].bounds
}

fn node<'a>(rt: &'a Runtime<Notice>, id: &str) -> &'a Node {
    rt.dom().get(rt.dom().by_id(id).unwrap()).unwrap()
}

#[test]
fn message_only_uses_inverse_tokens_and_material_line_heights() {
    for theme in [Theme::light(), Theme::dark()] {
        let mut rt = Runtime::new(Notice {
            theme,
            action: None,
            dismiss: false,
            ..Notice::default()
        })
        .unwrap();
        rt.render(900., 380., &mut Metrics).unwrap();

        let panel = node(&rt, "notice");
        let status = node(&rt, "notice-text");
        assert_eq!(bounds(&rt, "notice").width, 600.);
        assert_eq!(bounds(&rt, "notice").height, 48.);
        assert_eq!(panel.style().background, theme.colors.inverse_surface);
        assert_eq!(panel.style().corners, Some(shape::EXTRA_SMALL));
        assert_eq!(
            panel.box_shadow_layers(),
            theme.elevation(Elevation::Level3)
        );
        assert_eq!(status.role(), Role::Status);
        assert_eq!(status.text_value(), Some("Saved"));
        assert_eq!(status.style().foreground, theme.colors.inverse_on_surface);
        assert_eq!(status.style().text_style(), typography::BODY_MEDIUM);
        assert!(rt.dom().by_id("notice-action").is_none());
        assert!(rt.dom().by_id("notice-dismiss").is_none());
    }

    let mut rt = Runtime::new(Notice {
        text: "First line\nSecond line",
        action: None,
        dismiss: false,
        ..Notice::default()
    })
    .unwrap();
    rt.render(320., 380., &mut Metrics).unwrap();
    assert_eq!(bounds(&rt, "notice").height, 68.);
}

#[test]
fn short_actions_fit_inline_with_centred_close_vector_and_hit_targets() {
    let mut rt = Runtime::new(Notice::default()).unwrap();
    let paint = rt.render_extended(528., 350., &mut Metrics).unwrap();
    let panel = bounds(&rt, "notice");
    let message = bounds(&rt, "notice-text");
    let action = bounds(&rt, "notice-action");
    let close = bounds(&rt, "notice-dismiss");
    assert_eq!(panel.height, 48.);
    assert_eq!(action.y, panel.y);
    assert_eq!(close.y, panel.y);
    assert_eq!(close.width, 48.);
    assert_eq!(action.height, 48.);
    assert!(action.width >= 48.);
    assert!(message.x + message.width < action.x);
    assert_eq!(action.x + action.width, close.x);

    let theme = Theme::light();
    assert_eq!(node(&rt, "notice-action").name(), "Undo");
    assert_eq!(node(&rt, "notice-dismiss").name(), "Dismiss notice");
    assert_eq!(
        node(&rt, "notice-action").style().foreground,
        theme.colors.inverse_primary
    );
    assert_eq!(
        node(&rt, "notice-dismiss").style().state_layer,
        theme.colors.inverse_on_surface
    );
    let close_ink = Rect::new(close.x + 17., close.y + 17., 14., 14.);
    assert!(paint.ops().iter().any(|op| matches!(
        op, ExtendedPaintOp::FillPath(path)
        if path.rect() == close_ink && path.color() == theme.colors.inverse_on_surface
    )));
}

#[test]
fn narrow_scaled_chinese_wraps_both_labels_and_mirrors_the_controls() {
    fn render(direction: LayoutDirection) -> Runtime<Notice> {
        let mut rt = Runtime::new(Notice {
            text: "項目已封存，可以在封存項目中找到這份記錄。",
            action: Some("撤銷封存並移回目前的資料夾"),
            scale: 2.,
            layout: SnackbarOptions {
                direction,
                ..SnackbarOptions::default()
            },
            ..Notice::default()
        })
        .unwrap();
        let paint = rt.render(320., 440., &mut Metrics).unwrap();
        let panel = bounds(&rt, "notice");
        let text = bounds(&rt, "notice-text");
        let action = bounds(&rt, "notice-action");
        let close = bounds(&rt, "notice-dismiss");
        assert_eq!(panel.width, 320.);
        assert!(text.height > 40.);
        assert!(action.y >= text.y + text.height);
        assert!(action.height >= 96.);
        assert_eq!(close.height, 48.);
        assert_eq!(action.intersect(panel), action);
        assert_eq!(close.intersect(panel), close);
        assert_eq!(action.intersect(close).width, 0.);
        assert!(paint.ops.iter().any(|op| matches!(
            op, PaintOp::StyledText { rect, text, style, .. }
            if text == "撤銷封存並移回目前的資料夾" && style.font_size == 28.
                && rect.height > 40. && rect.intersect(action) == *rect
        )));
        rt
    }

    let ltr = render(LayoutDirection::LeftToRight);
    let rtl = render(LayoutDirection::RightToLeft);
    for id in ["notice-text", "notice-action", "notice-dismiss"] {
        let ltr = bounds(&ltr, id);
        let rtl = bounds(&rtl, id);
        assert_eq!(
            rtl,
            Rect::new(320. - ltr.x - ltr.width, ltr.y, ltr.width, ltr.height)
        );
    }
}

#[test]
fn explicitly_stacked_action_and_close_only_are_independent_options() {
    let mut rt = Runtime::new(Notice {
        layout: SnackbarOptions {
            action_on_new_line: true,
            ..SnackbarOptions::default()
        },
        dismiss: false,
        ..Notice::default()
    })
    .unwrap();
    rt.render(600., 300., &mut Metrics).unwrap();
    assert_eq!(
        bounds(&rt, "notice-action").y,
        bounds(&rt, "notice").y + 48.
    );
    assert!(rt.dom().by_id("notice-dismiss").is_none());

    let mut rt = Runtime::new(Notice {
        action: None,
        ..Notice::default()
    })
    .unwrap();
    rt.render(320., 300., &mut Metrics).unwrap();
    assert!(rt.dom().by_id("notice-action").is_none());
    assert_eq!(bounds(&rt, "notice-dismiss").y, bounds(&rt, "notice").y);
}

#[test]
fn opening_keeps_focus_and_dismissal_restores_it_from_a_control() {
    let mut rt = Runtime::new(Notice {
        visible: false,
        ..Notice::default()
    })
    .unwrap();
    rt.render(320., 340., &mut Metrics).unwrap();
    rt.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(rt.focused(), Some("open"));
    rt.dispatch(Event::Activate).unwrap();
    assert_eq!(rt.focused(), Some("open"));
    rt.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(rt.focused(), Some("notice-action"));
    let paint = rt.render(320., 340., &mut Metrics).unwrap();
    let action = bounds(&rt, "notice-action");
    let ink = Theme::light().colors.inverse_primary;
    assert!(paint.ops.iter().any(|op| matches!(
        op, PaintOp::Stroke { rect, color, width, .. }
        if *rect == action && *width == 2. && *color == ink
    )));
    rt.dispatch(Event::Activate).unwrap();
    assert_eq!(rt.app().undid, 1);
    rt.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(rt.focused(), Some("notice-dismiss"));
    rt.dispatch(Event::Activate).unwrap();
    assert_eq!(rt.app().closed, 1);
    assert_eq!(rt.focused(), Some("open"));
}

#[test]
fn accessible_presses_are_separate_and_retire_with_their_notice() {
    let mut rt = Runtime::new(Notice::default()).unwrap();
    rt.render(320., 340., &mut Metrics).unwrap();
    let frame = rt.accessibility_frame().unwrap();
    let id = |k| rt.dom().by_id(k).unwrap();
    assert!(frame.activation_target(id("notice-text")).is_none());
    assert!(frame.focus_target(id("notice-text")).is_none());
    let action = frame.activation_target(id("notice-action")).unwrap();

    assert_eq!(
        rt.accessibility_activate(&action).unwrap(),
        AccessibilityActivationResult::Activated
    );
    assert_eq!((rt.app().undid, rt.app().closed), (1, 0));

    rt.render(320., 340., &mut Metrics).unwrap();
    let close = rt
        .accessibility_frame()
        .unwrap()
        .activation_target(rt.dom().by_id("notice-dismiss").unwrap())
        .unwrap();
    assert_eq!(
        rt.accessibility_activate(&close).unwrap(),
        AccessibilityActivationResult::Activated
    );
    rt.render(320., 340., &mut Metrics).unwrap();
    assert_eq!(
        rt.accessibility_activate(&close).unwrap(),
        AccessibilityActivationResult::Rejected
    );
    assert_eq!((rt.app().undid, rt.app().closed), (1, 1));
}

#[test]
fn pressed_close_paints_its_own_state_and_ignores_cancelled_input() {
    let mut rt = Runtime::new(Notice::default()).unwrap();
    rt.render(500., 340., &mut Metrics).unwrap();
    let close = bounds(&rt, "notice-dismiss");
    let x = close.x + close.width / 2.;
    let y = close.y + close.height / 2.;
    let Color(r, g, b, _) = Theme::light().colors.inverse_on_surface;
    let has_close_layer = |paint: surface::DisplayList, alpha| {
        paint.ops.iter().any(|op| {
            matches!(
                op, PaintOp::Quad { rect, color, .. }
                if *rect == close && *color == Color(r, g, b, alpha)
            )
        })
    };
    rt.dispatch(Event::PointerMove { x, y }).unwrap();
    assert!(has_close_layer(
        rt.render(500., 340., &mut Metrics).unwrap(),
        20
    ));
    rt.dispatch(Event::PointerDown { x, y }).unwrap();
    assert!(has_close_layer(
        rt.render(500., 340., &mut Metrics).unwrap(),
        31
    ));
    rt.dispatch(Event::Cancel).unwrap();
    rt.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!(rt.app().closed, 0);
    rt.dispatch(Event::PointerDown { x, y }).unwrap();
    rt.dispatch(Event::PointerUp { x, y }).unwrap();
    assert_eq!((rt.app().undid, rt.app().closed), (0, 1));
}

#[test]
fn narrow_large_text_keeps_all_shaped_glyph_rows_inside_their_bounds() {
    let label = "撤銷封存並移回目前的資料夾";
    let mut rt = Runtime::new(Notice {
        text: "這個項目已封存。你可以從已封存項目取回這份記錄。",
        action: Some(label),
        scale: 2.,
        ..Notice::default()
    })
    .unwrap();
    let mut fonts = surface_wgpu::TextSystem::default();
    let list = rt.render(320., 400., &mut fonts).unwrap();
    let panel = bounds(&rt, "notice");
    let text = bounds(&rt, "notice-text");
    let action = bounds(&rt, "notice-action");
    let mut checked = 0;
    for op in &list.ops {
        let PaintOp::StyledText {
            rect, text, style, ..
        } = op
        else {
            continue;
        };
        if text == label || text == rt.app().text {
            let (_, shaped_height) = fonts.measure_text(text, style, Some(rect.width));
            assert!(rect.height >= shaped_height, "{text} was clipped");
            assert!(shaped_height >= 2. * style.line_height_px());
            assert_eq!(rect.intersect(panel), *rect);
            checked += 1;
        }
    }
    assert_eq!(checked, 2);
    assert!(action.y >= text.y + text.height);
}
