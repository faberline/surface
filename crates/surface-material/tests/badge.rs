use surface::{Align, Length, Node, PaintOp, Role, Style, TextMeasurer};
use surface_app::{Application, Runtime, View};
use surface_material::{shape, typography, Theme};

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.6, 16.)
    }
}

struct App {
    dark: bool,
    name: &'static str,
    label: Option<&'static str>,
}
impl Application for App {
    type Message = ();
    fn update(&mut self, _: ()) {}
    fn view(&self) -> View<()> {
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        View::new(
            Node::new(Role::Window, "Test")
                .styled(Style::row().width(Length::Percent(1.)).align(Align::Start)),
        )
        .children([theme.badge("badge", self.name, self.label)])
    }
}

fn render(dark: bool, name: &'static str, label: Option<&'static str>) -> Runtime<App> {
    let mut app = Runtime::new(App { dark, name, label }).unwrap();
    app.render(800., 100., &mut Metrics).unwrap();
    app
}

#[test]
fn dot_has_size_color_and_a_readable_status_name() {
    for dark in [false, true] {
        let app = render(dark, "New activity", None);
        let id = app.dom().by_id("badge").unwrap();
        let badge = app.dom().get(id).unwrap();
        let bounds = app.computed().unwrap().nodes[id.index()].bounds;
        let frame = app.accessibility_frame().unwrap();
        assert_eq!(frame.node(id).unwrap().node.name(), "New activity");
        assert!(frame.children(id).next().is_none());
        assert!(frame.focus_target(id).is_none());
        assert!(frame.activation_target(id).is_none());
        let theme = if dark { Theme::dark() } else { Theme::light() };
        assert_eq!(badge.role(), Role::Text);
        assert_eq!(badge.name(), "New activity");
        assert_eq!(badge.style().background, theme.colors.error);
        assert_eq!(badge.style().corners, Some(shape::FULL));
        assert_eq!((bounds.width, bounds.height), (6., 6.));
        assert_eq!(app.dom().children(id).count(), 0);
    }
}

#[test]
fn text_badge_uses_readable_label_tokens_and_expands_for_long_text() {
    for dark in [false, true] {
        let mut app = render(dark, "125 unread messages", Some("125 unread"));
        let display = app.render(800., 100., &mut Metrics).unwrap();
        let id = app.dom().by_id("badge").unwrap();
        let badge = app.dom().get(id).unwrap();
        let bounds = app.computed().unwrap().nodes[id.index()].bounds;
        let theme = if dark { Theme::dark() } else { Theme::light() };
        assert_eq!(badge.name(), "125 unread messages");
        let frame = app.accessibility_frame().unwrap();
        assert_eq!(frame.node(id).unwrap().node.name(), "125 unread messages");
        assert!(frame.children(id).next().is_none());
        assert!(frame.focus_target(id).is_none());
        assert!(frame.activation_target(id).is_none());
        assert_eq!(badge.style().background, theme.colors.error);
        assert_eq!(badge.style().foreground, theme.colors.on_error);
        assert_eq!(bounds.height, 16.);
        let text = app.dom().children(id).nth(1).unwrap();
        let text_node = app.dom().get(text).unwrap();
        let text_bounds = app.computed().unwrap().nodes[text.index()].bounds;
        assert_eq!(text_node.text_value(), Some("125 unread"));
        assert_eq!(
            text_node.style().font_size,
            typography::LABEL_SMALL.font_size
        );
        assert_eq!(text_node.style().foreground, theme.colors.on_error);
        assert!(bounds.width >= text_bounds.width + 8.);
        assert!(
            text_bounds.x >= bounds.x
                && text_bounds.x + text_bounds.width <= bounds.x + bounds.width
        );
        assert!(display.ops.iter().any(|op| matches!(op,
            PaintOp::StyledText { text, rect, clip, .. } if text == "125 unread"
                && clip.x <= rect.x && clip.x + clip.width >= rect.x + rect.width)));
    }
}

#[test]
fn empty_label_is_a_dot_and_missing_names_have_nonempty_fallbacks() {
    let dot = render(false, "", Some(" \n\t"));
    let dot = dot.dom().get(dot.dom().by_id("badge").unwrap()).unwrap();
    assert_eq!(dot.name(), "Notification");
    assert_eq!(dot.style().width, Length::Px(6.));

    let text = render(false, " ", Some(" 9\u{2028} unlabeled second line "));
    let text = text.dom().get(text.dom().by_id("badge").unwrap()).unwrap();
    assert_eq!(text.name(), "9");
    assert_eq!(text.style().min_width, 16.);
}

#[test]
fn badges_keep_content_width_in_a_stretched_column() {
    struct ColumnApp;
    impl Application for ColumnApp {
        type Message = ();
        fn update(&mut self, _: ()) {}
        fn view(&self) -> View<()> {
            let t = Theme::light();
            View::new(
                Node::new(Role::Window, "Test").styled(Style::column().width(Length::Percent(1.))),
            )
            .children([
                t.badge("column-dot", "New", None),
                t.badge("column-number", "3 unread", Some("3")),
                t.badge("column-long", "99 or more unread", Some("99+")),
            ])
        }
    }
    let mut app = Runtime::new(ColumnApp).unwrap();
    app.render(320., 200., &mut Metrics).unwrap();
    let width = |id| {
        app.computed().unwrap().nodes[app.dom().by_id(id).unwrap().index()]
            .bounds
            .width
    };
    assert_eq!(width("column-dot"), 6.);
    assert!(width("column-number") >= 16. && width("column-number") < 40.);
    assert!(width("column-long") > width("column-number") && width("column-long") < 60.);
}
