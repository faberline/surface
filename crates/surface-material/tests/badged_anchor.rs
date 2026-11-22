use surface::{Align, DisplayList, Length, Node, PaintOp, Rect, Role, Style};
use surface_app::{Application, Event, Runtime, View};
use surface_material::{
    shape::LayoutDirection::{LeftToRight, RightToLeft},
    typography, BadgeContent, BadgedAnchorOptions, Theme,
};
use surface_wgpu::TextSystem;

struct App {
    content: BadgeContent<'static>,
    options: BadgedAnchorOptions,
    scale: f32,
    pressed: Vec<&'static str>,
}

#[derive(Clone)]
enum Message {
    Change(BadgeContent<'static>, f32),
    Press(&'static str),
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Change(content, scale) => {
                self.content = content;
                self.scale = scale;
            }
            Message::Press(id) => self.pressed.push(id),
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn view(&self) -> View<Message> {
        let button = |id, label| {
            View::new(
                Node::new(Role::Button, label)
                    .id(id)
                    .styled(Style::row().width(Length::Px(48.)).height(Length::Px(48.))),
            )
            .on_press(Message::Press(id))
            .accessible_activation()
        };
        let anchor_name = match self.content {
            BadgeContent::Hidden => "收件匣",
            _ => "收件匣，1234 封未讀訊息",
        };
        View::new(
            Node::new(Role::Window, "Badges")
                .styled(Style::column().padding(32.).align(Align::Stretch)),
        )
        .children([
            Theme::light().badged_anchor(button("inbox", anchor_name), self.content, self.options),
            button("next", "下一頁"),
        ])
    }
}

fn runtime(options: BadgedAnchorOptions) -> Runtime<App> {
    Runtime::new(App {
        content: BadgeContent::Hidden,
        options,
        scale: 1.,
        pressed: Vec::new(),
    })
    .unwrap()
}

fn bounds(app: &Runtime<App>, key: &str) -> Rect {
    app.computed().unwrap().nodes[app.dom().by_id(key).unwrap().index()].bounds
}

fn badge_bounds(display: &DisplayList) -> Option<Rect> {
    display.ops.iter().find_map(|op| match op {
        PaintOp::Shape { rect, color, .. } | PaintOp::Quad { rect, color, .. }
            if *color == Theme::light().colors.error =>
        {
            Some(*rect)
        }
        _ => None,
    })
}

#[test]
fn anchored_badge_follows_the_artwork_and_preserves_flow_in_both_directions() {
    let mut fonts = TextSystem::default();
    for direction in [LeftToRight, RightToLeft] {
        let mut app = runtime(BadgedAnchorOptions {
            direction,
            top_inset: 12.,
            end_inset: 12.,
        });
        let hidden = app.render(280., 240., &mut fonts).unwrap();
        let anchor = bounds(&app, "inbox");
        let next = bounds(&app, "next");
        assert!(badge_bounds(&hidden).is_none());
        assert_eq!((anchor.width, anchor.height), (48., 48.));

        for (content, overlap, lower_edge, height, text) in [
            (BadgeContent::Dot, 6., 6., 6., None),
            (BadgeContent::Label(" "), 6., 6., 6., None),
            (BadgeContent::Label("新\n未讀"), 12., 14., 16., Some("新")),
            (BadgeContent::Count(7), 12., 14., 16., Some("7")),
            (BadgeContent::Count(1234), 12., 14., 16., Some("999+")),
        ] {
            app.apply(Message::Change(content, 1.)).unwrap();
            let display = app.render(280., 240., &mut fonts).unwrap();
            let badge = badge_bounds(&display).unwrap();
            assert_eq!(bounds(&app, "inbox"), anchor);
            assert_eq!(bounds(&app, "next"), next);
            assert_eq!(badge.height, height);
            assert_eq!(badge.y + badge.height, anchor.y + 12. + lower_edge);
            if direction == LeftToRight {
                assert_eq!(badge.x, anchor.x + anchor.width - 12. - overlap);
            } else {
                assert_eq!(badge.x + badge.width, anchor.x + 12. + overlap);
            }
            if let Some(text) = text {
                assert!(display.ops.iter().any(|op| matches!(op,
                    PaintOp::StyledText { text: visible, .. } if visible == text
                )));
            }
        }
    }
}

#[test]
fn larger_localized_badge_text_stays_inside_the_face() {
    let options = BadgedAnchorOptions {
        top_inset: 12.,
        end_inset: 12.,
        ..BadgedAnchorOptions::default()
    };
    let mut fonts = TextSystem::default();
    let mut app = runtime(options);
    let label = BadgeContent::Label("未讀");
    app.apply(Message::Change(label, 1.)).unwrap();
    let base = app.render(280., 240., &mut fonts).unwrap();
    let badge_at_default = badge_bounds(&base).unwrap();
    let initial_anchor = bounds(&app, "inbox");

    app.apply(Message::Change(label, 2.)).unwrap();
    let display = app.render(280., 240., &mut fonts).unwrap();
    let badge = badge_bounds(&display).unwrap();
    assert_eq!(bounds(&app, "inbox"), initial_anchor);
    assert_eq!(
        badge.y + badge.height,
        badge_at_default.y + badge_at_default.height
    );
    assert!(badge.width > badge_at_default.width && badge.height > badge_at_default.height);
    assert!(display
        .ops
        .iter()
        .any(|op| matches!(op, PaintOp::StyledText {
        rect, clip, text, style, ..
    } if text == "未讀" && style.font_size == typography::LABEL_SMALL.font_size * 2.
        && rect.x >= badge.x && rect.x + rect.width <= badge.x + badge.width
        && rect.y >= badge.y && rect.y + rect.height <= badge.y + badge.height
        && clip.x <= rect.x && clip.x + clip.width >= rect.x + rect.width
        && clip.y <= rect.y && clip.y + clip.height >= rect.y + rect.height)));
}

#[test]
fn anchored_badge_is_decorative_and_keeps_the_same_anchor_focus_and_action() {
    let mut app = runtime(BadgedAnchorOptions::default());
    let mut fonts = TextSystem::default();
    app.render(280., 240., &mut fonts).unwrap();
    app.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(app.focused(), Some("inbox"));

    app.apply(Message::Change(BadgeContent::Count(1234), 1.))
        .unwrap();
    let display = app.render(280., 240., &mut fonts).unwrap();
    assert_eq!(app.focused(), Some("inbox"));
    let anchor_bounds = bounds(&app, "inbox");
    let badge = badge_bounds(&display).unwrap();
    let inbox = app.dom().by_id("inbox").unwrap();
    let frame = app.accessibility_frame().unwrap();
    assert_eq!(frame.focused(), Some(inbox));
    assert_eq!(
        frame.node(inbox).unwrap().node.name(),
        "收件匣，1234 封未讀訊息"
    );
    assert!(frame.activation_target(inbox).is_some());
    let decoration = app
        .dom()
        .iter()
        .find(|(_, node)| node.text_value() == Some("999+"))
        .unwrap()
        .0;
    assert!(frame.node(decoration).is_none());
    assert!(frame.activation_target(decoration).is_none());
    assert!(frame.focus_target(decoration).is_none());

    let y = anchor_bounds.y + 6.;
    assert!(badge.y <= y && y < badge.y + badge.height);
    let inside_overlay = anchor_bounds.x + anchor_bounds.width - 4.;
    let outside_target = anchor_bounds.x + anchor_bounds.width + 4.;
    assert!(badge.x <= inside_overlay && outside_target < badge.x + badge.width);
    for x in [inside_overlay, outside_target] {
        app.render(280., 240., &mut fonts).unwrap();
        app.dispatch(Event::PointerDown { x, y }).unwrap();
        app.dispatch(Event::PointerUp { x, y }).unwrap();
    }
    assert_eq!(app.app().pressed, ["inbox"]);

    app.render(280., 240., &mut fonts).unwrap();
    app.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(app.focused(), Some("inbox"));
    app.dispatch(Event::FocusNext { reverse: false }).unwrap();
    assert_eq!(app.focused(), Some("next"));
    app.dispatch(Event::FocusNext { reverse: true }).unwrap();
    assert_eq!(app.focused(), Some("inbox"));
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.app().pressed, ["inbox", "inbox"]);
}
