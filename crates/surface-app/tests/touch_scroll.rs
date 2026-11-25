use surface::{Align, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityActivationResult, AccessibilityFocusResult, AccessibilityScrollResult,
    Application, Event, Runtime, ScrollTarget, VerticalDrag, VerticalDragSession,
    VerticalScrollResponse, View,
};

const ITEMS: [&str; 5] = ["walk", "tram", "bike", "boat", "rail"];

#[derive(Clone)]
enum Message {
    Toggle(usize),
    ExplicitDrag(VerticalDrag),
    EnableVerticalHook,
    VerticalWheel,
    Modal,
    RemoveOptIn,
}

struct App {
    rtl: bool,
    opt_in: bool,
    items: usize,
    vertical: bool,
    modal: bool,
    checked: [bool; 5],
    activations: usize,
    explicit_drag: Option<VerticalDragSession>,
    explicit_events: Vec<VerticalDrag>,
    custom_vertical_overflows: Option<bool>,
    custom_wheels: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            rtl: false,
            opt_in: true,
            items: 5,
            vertical: true,
            modal: false,
            checked: [false; 5],
            activations: 0,
            explicit_drag: None,
            explicit_events: Vec::new(),
            custom_vertical_overflows: None,
            custom_wheels: 0,
        }
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Toggle(i) => {
                self.checked[i] = !self.checked[i];
                self.activations += 1;
            }
            Message::Modal => self.modal = !self.modal,
            Message::ExplicitDrag(drag) => self.explicit_events.push(drag),
            Message::EnableVerticalHook => self.custom_vertical_overflows = Some(false),
            Message::VerticalWheel => self.custom_wheels += 1,
            Message::RemoveOptIn => self.opt_in = false,
        }
    }

    fn view(&self) -> View<Message> {
        let width = 128.;
        let content_width = (self.items as f32) * 64.;
        let mut rail = Style::row().height(Length::Px(64.));
        rail.min_width = content_width;
        let mut track = Node::new(Role::Group, "").styled(rail);
        if self.rtl {
            track = track.reverse_row();
        }
        let mut inner_style = Style::column()
            .align(Align::Start)
            .width(Length::Px(width))
            .height(Length::Px(64.))
            .clip(true);
        if self.rtl {
            inner_style.scroll_x = (content_width - width).max(0.);
        }
        let inner = View::new(
            Node::new(Role::Group, "Transport")
                .id("inner")
                .styled(inner_style),
        )
        .children([
            View::new(track).children(ITEMS[..self.items].iter().enumerate().map(|(i, id)| {
                let checkbox = Node::new(Role::Checkbox, *id)
                    .id(*id)
                    .checked(self.checked[i])
                    .styled(
                        Style::column()
                            .width(Length::Px(64.))
                            .height(Length::Px(64.)),
                    );
                View::new(checkbox)
                    .on_press(Message::Toggle(i))
                    .accessible_activation()
                    .accessibility_scroll_into_view()
            })),
        ])
        .on_horizontal_scroll(|_, _| None)
        .accessibility_scroll_pages();
        let inner = if self.opt_in {
            inner.horizontal_touch_scroll()
        } else {
            inner
        };
        let inner = if let Some(overflows) = self.custom_vertical_overflows {
            let child = View::new(
                Node::new(Role::Group, "")
                    .styled(Style::column().height(Length::Px(if overflows { 164. } else { 16. }))),
            );
            View::new(
                Node::new(Role::Group, "Custom scroller")
                    .id("middle")
                    .styled(
                        Style::column()
                            .align(Align::Start)
                            .width(Length::Px(160.))
                            .height(Length::Px(96.))
                            .clip(true),
                    ),
            )
            .children([inner, child])
            .on_vertical_scroll(|scroll| {
                Some(VerticalScrollResponse {
                    consumed: scroll.delta,
                    message: Message::VerticalWheel,
                })
            })
        } else {
            inner
        };
        let filler = View::new(
            Node::new(Role::Group, "")
                .styled(Style::column().height(Length::Px(if self.vertical { 300. } else { 16. }))),
        );
        let outer = View::new(
            Node::new(Role::Group, "Page").id("outer").styled(
                Style::column()
                    .align(Align::Start)
                    .width(Length::Px(160.))
                    .height(Length::Px(156.))
                    .clip(true),
            ),
        )
        .children([inner, filler]);
        let outer = if let Some(session) = &self.explicit_drag {
            outer.on_vertical_drag(session.clone(), Message::ExplicitDrag)
        } else {
            outer
        };
        let after = View::new(
            Node::new(Role::Button, "After").id("after").styled(
                Style::column()
                    .width(Length::Px(64.))
                    .height(Length::Px(44.)),
            ),
        );
        let modal = self.modal.then(|| {
            View::new(Node::new(Role::Dialog, "Other scope").id("modal"))
                .children([View::new(Node::new(Role::Button, "Close").id("close"))
                    .on_press(Message::Modal)])
                .modal(Message::Modal)
        });
        View::new(Node::new(Role::Window, "Demo").styled(Style::column().padding(12.).gap(12.)))
            .children([outer, after].into_iter().chain(modal))
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size, size)
    }
}

fn draw(runtime: &mut Runtime<App>) {
    runtime.render(360., 400., &mut Metrics).unwrap();
}

fn bounds(runtime: &Runtime<App>, id: &str) -> Rect {
    let id = runtime.dom().by_id(id).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn point(runtime: &Runtime<App>) -> (f32, f32) {
    let first = bounds(runtime, "walk").intersect(bounds(runtime, "inner"));
    (first.x + first.width / 2., first.y + first.height / 2.)
}

fn offset(runtime: &Runtime<App>, container: &str) -> (f32, f32) {
    runtime.scroll_metrics(container).unwrap().offset
}

fn down(runtime: &mut Runtime<App>, (x, y): (f32, f32)) {
    runtime.dispatch_touch(Event::PointerDown { x, y }).unwrap();
}

fn move_to(runtime: &mut Runtime<App>, (x, y): (f32, f32)) {
    runtime.dispatch_touch(Event::PointerMove { x, y }).unwrap();
}

fn up(runtime: &mut Runtime<App>, (x, y): (f32, f32)) {
    runtime.dispatch_touch(Event::PointerUp { x, y }).unwrap();
}

#[test]
fn touch_tap_stays_on_the_original_checkbox_and_claim_does_not_take_focus() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    draw(&mut runtime);
    runtime.activate("after").unwrap();
    draw(&mut runtime);
    let (x, y) = point(&runtime);
    down(&mut runtime, (x, y));
    assert!(runtime.has_touch_scroll_capture());
    assert_eq!(runtime.focused(), Some("after"));
    move_to(&mut runtime, (x - 3., y + 2.));
    assert_eq!(runtime.app().activations, 0);
    up(&mut runtime, (x - 3., y + 2.));
    assert!(!runtime.has_touch_scroll_capture());
    assert_eq!(runtime.focused(), Some("walk"));
    assert_eq!(runtime.app().activations, 1);
    assert!(runtime.app().checked[0]);
    draw(&mut runtime);
    assert_eq!(offset(&runtime, "inner"), (0., 0.));
}

#[test]
fn horizontal_touch_is_physical_in_both_directions_and_locks_its_axis() {
    for rtl in [false, true] {
        let mut runtime = Runtime::new(App {
            rtl,
            ..App::default()
        })
        .unwrap();
        draw(&mut runtime);
        let initial = offset(&runtime, "inner").0;
        let (x, y) = point(&runtime);
        let sign = if rtl { 1. } else { -1. };
        down(&mut runtime, (x, y));
        move_to(&mut runtime, (x + sign * 24., y + 2.));
        draw(&mut runtime);
        assert_eq!(offset(&runtime, "inner").0, initial - sign * 24.);
        assert_eq!(offset(&runtime, "outer").1, 0.);
        up(&mut runtime, (x + sign * 38., y - 120.));
        draw(&mut runtime);
        assert_eq!(offset(&runtime, "inner").0, initial - sign * 38.);
        assert_eq!(offset(&runtime, "outer").1, 0.);
        assert_eq!(runtime.app().activations, 0);
        assert_eq!(runtime.focused(), None);
    }
}

#[test]
fn vertical_touch_started_in_the_inner_strip_moves_only_the_parent() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    draw(&mut runtime);
    let (x, y) = point(&runtime);
    down(&mut runtime, (x, y));
    move_to(&mut runtime, (x - 1., y - 24.));
    draw(&mut runtime);
    assert_eq!(offset(&runtime, "inner"), (0., 0.));
    assert_eq!(offset(&runtime, "outer"), (0., 24.));
    up(&mut runtime, (x - 110., y - 31.));
    draw(&mut runtime);
    assert_eq!(offset(&runtime, "inner"), (0., 0.));
    assert_eq!(offset(&runtime, "outer"), (0., 31.));
    assert_eq!(runtime.app().activations, 0);
}

#[test]
fn nearest_custom_vertical_owner_blocks_touch_handoff_and_keeps_wheel() {
    for overflows in [false, true] {
        let mut runtime = Runtime::new(App {
            custom_vertical_overflows: Some(overflows),
            ..App::default()
        })
        .unwrap();
        draw(&mut runtime);
        let middle = runtime.scroll_metrics("middle").unwrap();
        assert_eq!(middle.content.1 > middle.viewport.1, overflows);
        assert!(
            runtime.scroll_metrics("outer").unwrap().content.1 > bounds(&runtime, "outer").height
        );
        let (x, y) = point(&runtime);
        down(&mut runtime, (x, y));
        assert!(runtime.has_touch_scroll_capture());
        move_to(&mut runtime, (x - 1., y - 20.));
        up(&mut runtime, (x, y));
        draw(&mut runtime);
        assert_eq!(offset(&runtime, "middle"), (0., 0.));
        assert_eq!(offset(&runtime, "outer"), (0., 0.));
        assert_eq!(runtime.app().custom_wheels, 0);
        assert_eq!(runtime.app().activations, 0);

        runtime
            .dispatch(Event::Scroll {
                x,
                y,
                delta_x: 0.,
                delta_y: 20.,
            })
            .unwrap();
        draw(&mut runtime);
        assert_eq!(runtime.app().custom_wheels, 1);
        assert_eq!(offset(&runtime, "middle"), (0., 0.));
        assert_eq!(offset(&runtime, "outer"), (0., 0.));

        down(&mut runtime, (x, y));
        up(&mut runtime, (x - 24., y));
        draw(&mut runtime);
        assert_eq!(offset(&runtime, "inner"), (24., 0.));
        assert_eq!(runtime.app().custom_wheels, 1);
        assert_eq!(runtime.app().activations, 0);
    }
}

#[test]
fn custom_owner_inserted_during_a_vertical_pan_stops_foreign_writes() {
    for cancel in [false, true] {
        let mut runtime = Runtime::new(App::default()).unwrap();
        draw(&mut runtime);
        let (x, y) = point(&runtime);
        down(&mut runtime, (x, y));
        move_to(&mut runtime, (x, y - 20.));
        draw(&mut runtime);
        assert_eq!(offset(&runtime, "outer"), (0., 20.));
        runtime.apply(Message::EnableVerticalHook).unwrap();
        draw(&mut runtime);
        if cancel {
            runtime.cancel_pointer().unwrap();
        } else {
            move_to(&mut runtime, (x, y - 40.));
        }
        up(&mut runtime, (x, y - 45.));
        draw(&mut runtime);
        assert_eq!(offset(&runtime, "middle"), (0., 0.));
        assert_eq!(offset(&runtime, "outer"), (0., 20.));
        assert_eq!(runtime.app().custom_wheels, 0);
        assert_eq!(runtime.app().activations, 0);
    }
}

#[test]
fn panning_past_existing_focus_does_not_reveal_it_until_requested() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    draw(&mut runtime);
    let walk = runtime.dom().by_id("walk").unwrap();
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(walk)
        .unwrap();
    assert_eq!(
        runtime.accessibility_focus(&target).unwrap(),
        AccessibilityFocusResult::Moved,
    );
    draw(&mut runtime);
    let (x, y) = point(&runtime);
    down(&mut runtime, (x, y));
    move_to(&mut runtime, (x - 88., y));
    draw(&mut runtime);
    assert_eq!(offset(&runtime, "inner"), (88., 0.));
    up(&mut runtime, (x - 112., y));
    draw(&mut runtime);
    assert_eq!(offset(&runtime, "inner"), (112., 0.));
    assert_eq!(runtime.focused(), Some("walk"));
    assert!(
        bounds(&runtime, "walk")
            .intersect(bounds(&runtime, "inner"))
            .width
            <= 0.
    );
    assert_eq!(runtime.app().activations, 0);
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .scroll_target(walk)
        .unwrap();
    assert_eq!(
        runtime.accessibility_scroll_into_view(&target).unwrap(),
        AccessibilityScrollResult::Scheduled,
    );
    draw(&mut runtime);
    assert_eq!(offset(&runtime, "inner"), (0., 0.));
    assert_eq!(runtime.focused(), Some("walk"));
}

#[test]
fn release_only_pans_but_diagonal_and_missing_vertical_extent_retire_taps() {
    let mut runtime = Runtime::new(App {
        vertical: false,
        ..App::default()
    })
    .unwrap();
    draw(&mut runtime);
    let (x, y) = point(&runtime);
    down(&mut runtime, (x, y));
    up(&mut runtime, (x - 20., y));
    draw(&mut runtime);
    assert_eq!(offset(&runtime, "inner"), (20., 0.));
    let view = bounds(&runtime, "inner");
    let (x, y) = (view.x + 18., view.y + 26.);
    for (dx, dy) in [(0., 18.), (8., 8.)] {
        down(&mut runtime, (x, y));
        move_to(&mut runtime, (x + dx, y + dy));
        up(&mut runtime, (x, y));
        draw(&mut runtime);
        assert_eq!(offset(&runtime, "inner"), (20., 0.));
        assert_eq!(offset(&runtime, "outer"), (0., 0.));
        assert_eq!(runtime.app().activations, 0);
    }
}

#[test]
fn cancel_and_matched_host_loss_restore_offset_and_retire_the_old_touch() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    draw(&mut runtime);
    let (x, y) = point(&runtime);
    for cancel in [
        Runtime::<App>::cancel_pointer,
        Runtime::<App>::cancel_touch_scroll_capture,
    ] {
        down(&mut runtime, (x, y));
        move_to(&mut runtime, (x - 20., y + 1.));
        draw(&mut runtime);
        assert_eq!(offset(&runtime, "inner"), (20., 0.));
        assert!(cancel(&mut runtime).unwrap());
        assert!(!runtime.has_touch_scroll_capture());
        up(&mut runtime, (x, y));
        draw(&mut runtime);
        assert_eq!(offset(&runtime, "inner"), (0., 0.));
        assert_eq!(runtime.app().activations, 0);
    }
    assert!(!runtime.cancel_touch_scroll_capture().unwrap());
    down(&mut runtime, (x, y));
    up(&mut runtime, (x, y));
    assert_eq!(runtime.app().activations, 1);
}

#[test]
fn programmatic_scroll_wins_and_ordinary_wheel_and_projected_reveal_still_work() {
    let mut runtime = Runtime::new(App::default()).unwrap();
    draw(&mut runtime);
    let (x, y) = point(&runtime);
    let rail = runtime.dom().by_id("rail").unwrap();
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .scroll_target(rail)
        .unwrap();
    down(&mut runtime, (x, y));
    move_to(&mut runtime, (x - 24., y));
    draw(&mut runtime);
    assert_ne!(
        runtime.accessibility_scroll_into_view(&target).unwrap(),
        AccessibilityScrollResult::Scheduled
    );
    assert!(runtime.scroll_to("inner", ScrollTarget::Offset { x: 40., y: 0. }));
    draw(&mut runtime);
    assert!(runtime.cancel_pointer().unwrap());
    draw(&mut runtime);
    assert_eq!(offset(&runtime, "inner"), (40., 0.));
    runtime
        .dispatch(Event::Scroll {
            x,
            y,
            delta_x: 12.,
            delta_y: 16.,
        })
        .unwrap();
    draw(&mut runtime);
    assert_eq!(offset(&runtime, "inner"), (52., 0.));
    assert_eq!(offset(&runtime, "outer"), (0., 16.));
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .scroll_target(rail)
        .unwrap();
    assert_eq!(
        runtime.accessibility_scroll_into_view(&target).unwrap(),
        AccessibilityScrollResult::Scheduled
    );
    draw(&mut runtime);
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(rail)
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Activated
    );
}

#[test]
fn scope_replacement_resize_and_opt_out_leave_no_stale_touch() {
    for replacement in [Some(Message::Modal), Some(Message::RemoveOptIn), None] {
        let mut runtime = Runtime::new(App::default()).unwrap();
        draw(&mut runtime);
        let (x, y) = point(&runtime);
        down(&mut runtime, (x, y));
        move_to(&mut runtime, (x - 20., y));
        draw(&mut runtime);
        if let Some(message) = replacement {
            runtime.apply(message).unwrap();
        } else {
            assert!(runtime.pointer_geometry_changed().unwrap());
        }
        assert!(!runtime.has_touch_scroll_capture());
        up(&mut runtime, (x, y));
        draw(&mut runtime);
        assert_eq!(runtime.app().activations, 0);
    }
}

#[test]
fn existing_explicit_drags_and_nonoverflowing_or_unopted_views_keep_the_old_route() {
    let app = App {
        explicit_drag: Some(VerticalDragSession::new()),
        ..App::default()
    };
    let mut explicit = Runtime::new(app).unwrap();
    draw(&mut explicit);
    let (x, y) = point(&explicit);
    down(&mut explicit, (x, y));
    assert!(!explicit.has_touch_scroll_capture());
    move_to(&mut explicit, (x, y - 24.));
    up(&mut explicit, (x, y - 28.));
    draw(&mut explicit);
    assert!(matches!(
        explicit.app().explicit_events.as_slice(),
        [
            VerticalDrag::Move { delta_y: -24. },
            VerticalDrag::End { delta_y: -28., .. }
        ]
    ));
    assert_eq!(offset(&explicit, "inner"), (0., 0.));
    assert_eq!(offset(&explicit, "outer"), (0., 0.));
    for app in [
        App {
            items: 2,
            ..App::default()
        },
        App {
            opt_in: false,
            ..App::default()
        },
    ] {
        let mut runtime = Runtime::new(app).unwrap();
        draw(&mut runtime);
        let (x, y) = point(&runtime);
        down(&mut runtime, (x, y));
        assert!(!runtime.has_touch_scroll_capture());
        assert_eq!(runtime.focused(), Some("walk"));
        up(&mut runtime, (x, y));
        assert_eq!(runtime.app().activations, 1);
    }
}
