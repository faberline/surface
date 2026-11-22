use surface::{
    Color, Corners, ExtendedDisplayList, ExtendedPaintOp, Node, PaintOp, Rect, Role, Style,
    TextMeasurer,
};
use surface_app::{
    AnimationFrame, Application, EditKey, Event, FrameInput, InputAnimationUpdate, Runtime, View,
};
use surface_material::{
    place_split_button_popup, shape::LayoutDirection, SplitButtonAnchor, SplitButtonAppearance,
    SplitButtonContent, SplitButtonLeading, SplitButtonMenu, SplitButtonMenuItem,
    SplitButtonMotion, SplitButtonOptions, SplitButtonSize, Theme, VectorIcon,
};

#[derive(Clone, Copy)]
enum Message {
    Primary,
    Toggle,
    Dismiss,
    Choice(usize),
    DisableLeading,
    DisableTrailing,
    Outside,
}

#[derive(Clone, Copy)]
enum Form {
    Text,
    Icon,
    Both,
}

struct Example {
    size: SplitButtonSize,
    appearance: SplitButtonAppearance,
    direction: LayoutDirection,
    form: Form,
    width: f32,
    scale: f32,
    dark: bool,
    built_in: bool,
    open: bool,
    leading_disabled: bool,
    trailing_disabled: bool,
    primary_hits: usize,
    choices: Vec<usize>,
    dismissals: usize,
    motion: SplitButtonMotion,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            size: SplitButtonSize::Medium,
            appearance: SplitButtonAppearance::Filled,
            direction: LayoutDirection::LeftToRight,
            form: Form::Text,
            width: 588.,
            scale: 1.,
            dark: false,
            built_in: true,
            open: false,
            leading_disabled: false,
            trailing_disabled: false,
            primary_hits: 0,
            choices: Vec::new(),
            dismissals: 0,
            motion: SplitButtonMotion::default(),
        }
    }
}

impl Application for Example {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Primary => self.primary_hits += 1,
            Message::Toggle => self.open = !self.open,
            Message::Dismiss => {
                self.open = false;
                self.dismissals += 1;
            }
            Message::Choice(index) => {
                self.choices.push(index);
                self.open = false;
            }
            Message::DisableLeading => self.leading_disabled = true,
            Message::DisableTrailing => {
                self.trailing_disabled = true;
                self.open = false;
            }
            Message::Outside => {}
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn observes_frame_input(&self) -> bool {
        true
    }

    fn animation_frame_with_press(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
        pressed: Option<&str>,
    ) -> InputAnimationUpdate {
        InputAnimationUpdate {
            animation: self
                .motion
                .advance("split", &frame, &input, pressed, self.open),
            suppress_touch_activation: false,
        }
    }

    fn view(&self) -> View<Message> {
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let label = "匯出完整報告";
        let icon = || VectorIcon::new([[2., 2.], [22., 2.], [12., 22.]]).unwrap();
        let leading = match self.form {
            Form::Text => SplitButtonLeading::Text(label),
            Form::Icon => SplitButtonLeading::Icon {
                label,
                icon: icon(),
            },
            Form::Both => SplitButtonLeading::TextAndIcon {
                label,
                icon: icon(),
            },
        };
        let buttons = theme.split_button(
            SplitButtonOptions {
                id: "split",
                size: self.size,
                appearance: self.appearance,
                direction: self.direction,
                available_width: self.width,
                open: self.open,
                leading_disabled: self.leading_disabled,
                trailing_disabled: self.trailing_disabled,
                restore_focus_to: Some("before"),
            },
            SplitButtonContent {
                leading,
                trailing_label: "匯出選項",
                on_primary: Message::Primary,
                on_toggle: Message::Toggle,
                menu: self.built_in.then(|| SplitButtonMenu {
                    label: "報告格式",
                    on_dismiss: Message::Dismiss,
                    items: ["pdf", "csv", "png"]
                        .into_iter()
                        .enumerate()
                        .map(|(index, id)| SplitButtonMenuItem {
                            id,
                            label: id,
                            icon: None,
                            disabled: index == 1,
                            on_press: Message::Choice(index),
                        })
                        .collect(),
                }),
            },
            &self.motion,
        );
        View::new(Node::new(Role::Window, "Demo").styled(Style::column().padding(16.).gap(8.)))
            .children([
                theme.button("before", "Before", Message::Outside),
                buttons,
                theme.button("after", "After", Message::Outside),
            ])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, limit: Option<f32>) -> (f32, f32) {
        let count = text.chars().count().max(1);
        let per_line = limit.map_or(count, |limit| ((limit / size).floor() as usize).max(1));
        (
            count.min(per_line) as f32 * size,
            count.div_ceil(per_line) as f32 * size * 1.5,
        )
    }
}

fn paint(runtime: &mut Runtime<Example>, time: f64) -> ExtendedDisplayList {
    runtime.set_time(time);
    let viewport = runtime.app().width + 32.;
    runtime
        .render_extended(viewport, 760., &mut Metrics)
        .unwrap()
}

fn node<'a>(runtime: &'a Runtime<Example>, id: &str) -> &'a Node {
    runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
}

fn rect(runtime: &Runtime<Example>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

fn near(a: f32, b: f32) {
    assert!((a - b).abs() <= 0.35, "{a} != {b}");
}

fn center(bounds: Rect) -> (f32, f32) {
    (bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.)
}

fn assert_inside_rounded(face: Rect, corners: Corners, content: Rect) {
    let corners = corners.resolve(face.width, face.height);
    let (cx, cy) = center(face);
    for x in [content.x, content.x + content.width] {
        for y in [content.y, content.y + content.height] {
            let (radius, dx, dy) = match (x <= cx, y <= cy) {
                (true, true) => (corners.top_left, x - face.x, y - face.y),
                (false, true) => (corners.top_right, face.x + face.width - x, y - face.y),
                (true, false) => (corners.bottom_left, x - face.x, face.y + face.height - y),
                (false, false) => (
                    corners.bottom_right,
                    face.x + face.width - x,
                    face.y + face.height - y,
                ),
            };
            assert!(dx >= -0.01 && dy >= -0.01);
            let from_arc = (radius - dx).max(0.).hypot((radius - dy).max(0.));
            assert!(
                from_arc <= radius + 0.01,
                "content {content:?} crosses the face contour at ({x}, {y}) in {face:?}"
            );
        }
    }
}

fn tab(runtime: &mut Runtime<Example>, reverse: bool) {
    runtime.dispatch(Event::FocusNext { reverse }).unwrap();
}

fn key(runtime: &mut Runtime<Example>, key: EditKey) {
    runtime
        .dispatch(Event::Edit {
            key,
            extend: false,
            by_word: false,
        })
        .unwrap();
}

#[test]
fn split_tokens_cover_all_sizes_and_appearances() {
    use surface_material::{ButtonGroupAppearance::*, ButtonGroupSize::*};
    for dark in [false, true] {
        let theme = if dark { Theme::dark() } else { Theme::light() };
        for (index, size) in [ExtraSmall, Small, Medium, Large, ExtraLarge]
            .into_iter()
            .enumerate()
        {
            for (appearance, fill) in [
                (Elevated, theme.colors.surface_container_low),
                (Filled, theme.colors.primary),
                (Tonal, theme.colors.secondary_container),
                (Outlined, Color::TRANSPARENT),
            ] {
                let mut app = Runtime::new(Example {
                    size,
                    appearance,
                    dark,
                    form: Form::Icon,
                    ..Example::default()
                })
                .unwrap();
                paint(&mut app, 0.);
                paint(&mut app, 16.);
                let leading = rect(&app, "split-leading-face");
                let trailing = rect(&app, "split-trailing-face");
                near(leading.height, [32., 40., 56., 96., 136.][index]);
                near(trailing.height, leading.height);
                near(trailing.width, [48., 48., 56., 96., 136.][index]);
                near(trailing.x - leading.x - leading.width, 2.);
                near(
                    rect(&app, "split-chevron").width,
                    [22., 22., 26., 38., 50.][index],
                );
                assert!(rect(&app, "split-leading").height >= 48.);
                assert!(rect(&app, "split-trailing").height >= 48.);
                assert_eq!(node(&app, "split-leading").name(), "匯出完整報告");
                let face = node(&app, "split-leading-face");
                let radius = [4., 4., 4., 8., 12.][index];
                assert_eq!(
                    face.style().corners,
                    Some(Corners::new(
                        Corners::FULL_RADIUS,
                        radius,
                        radius,
                        Corners::FULL_RADIUS
                    ))
                );
                assert_eq!(face.style().background, fill);
                if appearance == Elevated {
                    assert_eq!(
                        face.box_shadow_layers(),
                        theme.elevation(surface_material::Elevation::Level1)
                    );
                }
                if appearance == Outlined {
                    near(face.style().border_width, [1., 1., 1., 2., 3.][index]);
                }
            }
        }
    }
}

#[test]
fn split_xl_keeps_both_faces_and_the_icon_when_rtl_text_wraps_at_200_percent() {
    let mut app = Runtime::new(Example {
        size: SplitButtonSize::ExtraLarge,
        direction: LayoutDirection::RightToLeft,
        form: Form::Both,
        width: 288.,
        scale: 2.,
        dark: true,
        ..Example::default()
    })
    .unwrap();
    paint(&mut app, 0.);
    paint(&mut app, 16.);
    paint(&mut app, 32.);
    paint(&mut app, 48.);
    let display = paint(&mut app, 64.);
    let row = rect(&app, "split-row");
    let leading = rect(&app, "split-leading-face");
    let trailing = rect(&app, "split-trailing-face");
    let label = rect(&app, "split-label");
    near(row.width, 288.);
    near(trailing.width, 136.);
    near(leading.x - trailing.x - trailing.width, 2.);
    near(leading.height, trailing.height);
    near(leading.y, trailing.y);
    assert!(
        label.width >= 128.,
        "label {label:?}, probe {:?}, content {:?}",
        rect(&app, "split-measure"),
        rect(&app, "split-leading-content")
    );
    assert!(label.height > 136.);
    let corners = node(&app, "split-leading-face").style().corners.unwrap();
    assert_inside_rounded(leading, corners, label);
    assert_eq!(row, row.intersect(Rect::new(16., row.y, 288., row.height)));
    let inside = |path: &surface::PathFill, face: Rect| {
        face.contains(
            path.rect().x + path.rect().width / 2.,
            path.rect().y + path.rect().height / 2.,
        )
    };
    let icon = display
        .ops()
        .iter()
        .find_map(|op| match op {
            ExtendedPaintOp::FillPath(path) if inside(path, leading) => Some(path.rect()),
            _ => None,
        })
        .unwrap();
    assert_inside_rounded(leading, corners, icon);
    assert!(display.ops().iter().any(|op| matches!(op, ExtendedPaintOp::Legacy(PaintOp::StyledText { text, style, .. }) if text == "匯出完整報告" && style.font_size == 64.)));

    tab(&mut app, false);
    tab(&mut app, false);
    assert_eq!(app.focused(), Some("split-leading"));
    app.apply(Message::DisableLeading).unwrap();
    assert_eq!(app.focused(), Some("split-trailing"));
    app.apply(Message::DisableTrailing).unwrap();
    assert_eq!(app.focused(), Some("before"));
    tab(&mut app, false);
    assert_eq!(app.focused(), Some("after"));
}

#[test]
fn split_rounded_faces_contain_wrapped_text_only_and_inline_icon_content() {
    for (form, width) in [(Form::Text, 288.), (Form::Both, 520.)] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let mut app = Runtime::new(Example {
                size: SplitButtonSize::ExtraLarge,
                direction,
                form,
                width,
                scale: 2.,
                ..Example::default()
            })
            .unwrap();
            for time in [0., 16., 32., 48., 64.] {
                paint(&mut app, time);
            }
            let face = rect(&app, "split-leading-face");
            let content = rect(&app, "split-leading-content");
            let label = rect(&app, "split-label");
            let corners = node(&app, "split-leading-face").style().corners.unwrap();
            assert!(label.height > 80.);
            near(content.height, label.height);
            near(face.height, rect(&app, "split-trailing-face").height);
            assert_inside_rounded(face, corners, content);
        }
    }
}

#[test]
fn menu_opens_without_primary_action_and_returns_focus_on_escape_tab_and_selection() {
    let mut app = Runtime::new(Example::default()).unwrap();
    paint(&mut app, 0.);
    paint(&mut app, 16.);
    tab(&mut app, false);
    tab(&mut app, false);
    assert_eq!(app.focused(), Some("split-leading"));
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.app().primary_hits, 1);
    assert!(!app.app().open);
    paint(&mut app, 32.);
    tab(&mut app, false);
    assert_eq!(app.focused(), Some("split-trailing"));
    assert!(node(&app, "split-trailing").has_menu_popup());
    assert_eq!(node(&app, "split-trailing").expanded_state(), Some(false));
    app.dispatch(Event::Activate).unwrap();
    paint(&mut app, 48.);
    assert!(app.app().open);
    assert_eq!(app.app().primary_hits, 1);
    assert_eq!(app.focused(), Some("pdf"));
    let anchor = rect(&app, "split-trailing-face");
    let menu = rect(&app, "split-menu");
    near(menu.x, anchor.x);
    near(menu.y - (anchor.y + anchor.height), 4.);
    let frame = app.accessibility_frame().unwrap();
    assert_eq!(frame.node(frame.root()).unwrap().node.role(), Role::Menu);
    assert!(frame
        .activation_target(app.dom().by_id("csv").unwrap())
        .is_none());
    key(&mut app, EditKey::Down);
    assert_eq!(app.focused(), Some("png"));
    app.dispatch(Event::Cancel).unwrap();
    assert_eq!(app.focused(), Some("split-trailing"));
    assert_eq!(app.app().dismissals, 1);

    app.dispatch(Event::Activate).unwrap();
    tab(&mut app, false);
    assert_eq!(app.focused(), Some("after"));
    assert!(!app.app().open);
    tab(&mut app, true);
    app.dispatch(Event::Activate).unwrap();
    tab(&mut app, true);
    assert_eq!(app.focused(), Some("split-leading"));
    assert_eq!(app.app().dismissals, 3);

    tab(&mut app, false);
    app.dispatch(Event::Activate).unwrap();
    key(&mut app, EditKey::Down);
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.app().choices, [2]);
    assert_eq!(app.focused(), Some("split-trailing"));

    app.dispatch(Event::Activate).unwrap();
    paint(&mut app, 64.);
    app.dispatch_touch(Event::PointerDown { x: 1., y: 1. })
        .unwrap();
    app.dispatch_touch(Event::PointerUp { x: 1., y: 1. })
        .unwrap();
    assert_eq!(app.app().dismissals, 4);
    assert_eq!(app.focused(), Some("split-trailing"));
    app.dispatch(Event::Activate).unwrap();
    assert_eq!(app.focused(), Some("pdf"));
    app.apply(Message::DisableTrailing).unwrap();
    assert_eq!(app.focused(), Some("split-leading"));
}

#[test]
fn split_state_layers_optical_centering_and_rotation_use_the_actual_trailing_action() {
    use LayoutDirection::*;
    for direction in [LeftToRight, RightToLeft] {
        let mut app = Runtime::new(Example {
            direction,
            built_in: false,
            ..Example::default()
        })
        .unwrap();
        paint(&mut app, 0.);
        paint(&mut app, 16.);
        let trailer = rect(&app, "split-trailing-face");
        let initial = center(rect(&app, "split-chevron")).0 - center(trailer).0;
        assert!(if direction == LeftToRight {
            initial < 0.
        } else {
            initial > 0.
        });
        assert!(!node(&app, "split-trailing").has_menu_popup());
        let resting_fill = node(&app, "split-trailing-face").style().background;
        let (x, y) = center(trailer);
        app.dispatch(Event::PointerDown { x, y }).unwrap();
        paint(&mut app, 32.);
        let part = node(&app, "split-trailing-face").style().corners.unwrap();
        near(
            if direction == LeftToRight {
                part.top_left
            } else {
                part.top_right
            },
            12.,
        );
        app.dispatch(Event::PointerUp { x, y }).unwrap();
        paint(&mut app, 48.);
        let midway = paint(&mut app, 96.);
        let angle = app.app().motion.angle_degrees();
        assert!(angle > 0. && angle < 180.);
        let chevron = rect(&app, "split-chevron");
        let chevron_path = midway
            .ops()
            .iter()
            .find_map(|op| match op {
                ExtendedPaintOp::FillPath(path)
                    if chevron.contains(path.rect().x, path.rect().y) =>
                {
                    Some(path)
                }
                _ => None,
            })
            .unwrap();
        let physical_angle = if direction == LeftToRight {
            angle
        } else {
            -angle
        };
        let (sine, cosine) = physical_angle.to_radians().sin_cos();
        let (cx, cy) = center(chevron);
        let scale = chevron.width / 24.;
        near(
            chevron_path.points()[0][0],
            cx + (-6. * cosine + 3. * sine) * scale,
        );
        near(
            chevron_path.points()[0][1],
            cy + (-6. * sine - 3. * cosine) * scale,
        );
        paint(&mut app, 300.);
        near(app.app().motion.angle_degrees(), 180.);
        assert_eq!(
            node(&app, "split-trailing-face").style().background,
            resting_fill
        );
        assert_eq!(
            node(&app, "split-trailing-face-open-layer")
                .style()
                .background
                .3,
            26
        );
        assert_eq!(
            node(&app, "split-trailing-face").style().corners,
            Some(Corners::full())
        );
        near(center(rect(&app, "split-chevron")).0, center(trailer).0);
        assert_eq!(node(&app, "split-trailing").expanded_state(), Some(true));
        app.set_reduced_motion(true);
        app.apply(Message::Toggle).unwrap();
        paint(&mut app, 316.);
        near(app.app().motion.angle_degrees(), 0.);
        assert!(app.dom().by_id("split-trailing-face-open-layer").is_none());
    }
}

#[test]
fn popup_alignment_flips_and_preserves_room_in_a_short_viewport() {
    let a = SplitButtonAnchor {
        group: Rect::new(40., 250., 120., 48.),
        trailing: Rect::new(112., 250., 48., 48.),
        viewport: Rect::new(0., 0., 280., 320.),
    };
    let rtl = place_split_button_popup(a, (120., 160.), LayoutDirection::RightToLeft).unwrap();
    assert_eq!(rtl, Rect::new(40., 86., 120., 160.));
    let ltr = place_split_button_popup(a, (160., 400.), LayoutDirection::LeftToRight).unwrap();
    assert_eq!(ltr, Rect::new(112., 0., 160., 246.));
    assert!(place_split_button_popup(a, (f32::NAN, 160.), LayoutDirection::LeftToRight).is_none());
}
