use surface::{GridRole, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{AnimationFrame, AnimationUpdate, Application, Event, Queue, Runtime, View};
use surface_material::{
    shape::LayoutDirection, CivilDate, DateBounds, DateInputFormat, DateInputOrder,
    DateInputSeparator, DatePickerConstraints, DatePickerEffect, DatePickerLabels,
    DatePickerLayout, DatePickerLocale, DatePickerMode, DatePickerOptions, DatePickerSession,
    DatePickerSwipe, DatePickerSwipeInput, Theme, Weekday,
};

const PICKER: &str = "picker";
const GRID: &str = "picker-grid-scroll";
const CURRENT: &str = "picker-swipe-current";
const PREVIEW: &str = "picker-swipe-preview";
const SELECTED: &str = "picker-day-2024-02-28";
const NEXT: &str = "picker-day-2024-03-28";

const LABELS: DatePickerLabels<'static> = DatePickerLabels {
    title: "選擇日期",
    input_name: "日期",
    previous_month: "上個月",
    next_month: "下個月",
    choose_year: "選擇年份",
    edit_mode: "輸入日期",
    calendar_mode: "顯示行事曆",
    confirm: "確定",
    cancel: "取消",
    save: "儲存",
    close: "關閉",
};

fn date(month: u8, day: u8) -> CivilDate {
    CivilDate::new(2024, month, day).unwrap()
}

fn constraints() -> DatePickerConstraints<'static> {
    DatePickerConstraints {
        bounds: DateBounds::new(date(1, 1), date(12, 31)).unwrap(),
        allowed_date: None,
    }
}

fn locale(direction: LayoutDirection) -> DatePickerLocale<'static> {
    DatePickerLocale {
        direction,
        first_weekday: Weekday::Sunday,
        input_format: DateInputFormat {
            order: DateInputOrder::YearMonthDay,
            separator: DateInputSeparator::Slash,
        },
        format_hint: "YYYY/MM/DD",
        weekday_short: ["日", "一", "二", "三", "四", "五", "六"],
        weekday_full: ["日", "一", "二", "三", "四", "五", "六"],
        month_title: &|month| format!("{}年{}月", month.year(), month.month()),
        date_headline: &|value| value.to_string(),
        date_name: &|value| value.to_string(),
        error_text: &|error| format!("{error:?}"),
    }
}

#[derive(Clone)]
enum Message {
    Open,
    Picker(DatePickerSwipeInput),
}

struct Demo {
    direction: LayoutDirection,
    viewport: Rect,
    session: Option<DatePickerSession>,
    swipe: DatePickerSwipe,
    queue: Option<Queue<Message>>,
    commits: usize,
}

impl Application for Demo {
    type Message = Message;

    fn attach_queue(&mut self, queue: Queue<Message>) {
        self.queue = Some(queue);
    }

    fn update(&mut self, message: Message) {
        let locale = locale(self.direction);
        match message {
            Message::Open => {
                self.session = Some(DatePickerSession::new(
                    Some(date(2, 28)),
                    date(2, 29),
                    constraints(),
                    &locale,
                    DatePickerMode::Calendar,
                ));
                self.swipe = DatePickerSwipe::new();
            }
            Message::Picker(input) => {
                if let Some(session) = &mut self.session {
                    match self.swipe.apply(input, session, constraints(), &locale) {
                        DatePickerEffect::None => {}
                        DatePickerEffect::Commit(_) => {
                            self.commits += 1;
                            self.session = None;
                        }
                        DatePickerEffect::Dismiss => self.session = None,
                    }
                }
            }
        }
    }

    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        let Some(session) = &mut self.session else {
            return AnimationUpdate::default();
        };
        self.swipe.observe(
            PICKER,
            &frame,
            session,
            constraints(),
            &locale(self.direction),
            self.queue.as_ref().unwrap(),
        )
    }

    fn text_scale(&self) -> f32 {
        if self.viewport.width < 336. {
            2.
        } else {
            1.
        }
    }

    fn view(&self) -> View<Message> {
        let opener = View::new(
            Node::new(Role::Button, "開啟日期")
                .id("open")
                .styled(Style::row().height(Length::Px(48.))),
        )
        .on_press(Message::Open);
        let picker = self.session.as_ref().map(|session| {
            Theme::dark().swipeable_date_picker(
                DatePickerOptions {
                    id: PICKER,
                    today: date(2, 29),
                    constraints: constraints(),
                    locale: &locale(self.direction),
                    labels: &LABELS,
                    layout: DatePickerLayout::Adaptive,
                    viewport: self.viewport,
                    restore_focus_to: Some("open"),
                },
                session,
                &self.swipe,
                Message::Picker,
            )
        });
        View::new(Node::new(Role::Window, "Test")).children(std::iter::once(opener).chain(picker))
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, content: &str, size: f32, max_width: Option<f32>) -> (f32, f32) {
        let width = content.chars().count() as f32 * size * 0.82;
        let fitted = width.min(max_width.unwrap_or(width));
        let lines = (width / fitted.max(1.)).ceil().max(1.);
        (fitted, size * 1.5 * lines)
    }
}

fn render(runtime: &mut Runtime<Demo>, time: f64) {
    runtime.set_time(time);
    let viewport = runtime.app().viewport;
    runtime
        .render(viewport.width, viewport.height, &mut Metrics)
        .unwrap();
}

fn open(direction: LayoutDirection, width: f32, height: f32, reduce: bool) -> Runtime<Demo> {
    let mut runtime = Runtime::new(Demo {
        direction,
        viewport: Rect::new(0., 0., width, height),
        session: None,
        swipe: DatePickerSwipe::new(),
        queue: None,
        commits: 0,
    })
    .unwrap();
    runtime.set_reduced_motion(reduce);
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    runtime.apply(Message::Open).unwrap();
    render(&mut runtime, 0.);
    render(&mut runtime, 1.);
    assert_eq!(runtime.focused(), Some(SELECTED));
    runtime
}

fn bounds(runtime: &Runtime<Demo>, target: &str) -> Rect {
    let id = runtime.dom().by_id(target).unwrap();
    runtime.computed().unwrap().nodes[id.index()].bounds
}

fn origin(runtime: &Runtime<Demo>) -> (f32, f32) {
    let clip = bounds(runtime, GRID).intersect(bounds(runtime, "picker-body"));
    assert!(clip.width > 48. && clip.height > 48.);
    (clip.x + clip.width / 2., clip.y + clip.height / 2.)
}

fn begin(runtime: &mut Runtime<Demo>, origin: (f32, f32)) {
    let (x, y) = origin;
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
}

fn move_x(runtime: &mut Runtime<Demo>, origin: (f32, f32), delta: f32) {
    let (x, y) = origin;
    runtime
        .dispatch(Event::PointerMove { x: x + delta, y })
        .unwrap();
}

fn end_x(runtime: &mut Runtime<Demo>, origin: (f32, f32), delta: f32) {
    let (x, y) = origin;
    runtime
        .dispatch(Event::PointerUp { x: x + delta, y })
        .unwrap();
}

fn assert_month(runtime: &Runtime<Demo>, month: u8) {
    let app = runtime.app();
    let state = app.session.as_ref().unwrap();
    assert_eq!(state.month().month(), month);
    assert_eq!(state.draft(), Some(date(2, 28)));
    assert_eq!(app.commits, 0);
}

#[test]
fn physical_swipe_previews_a_passive_neighbor_then_keeps_day_focus_and_draft() {
    for (direction, sign) in [
        (LayoutDirection::LeftToRight, -1.),
        (LayoutDirection::RightToLeft, 1.),
    ] {
        let mut runtime = open(direction, 960., 720., false);
        let initial = bounds(&runtime, CURRENT);
        let contact = origin(&runtime);
        begin(&mut runtime, contact);
        move_x(&mut runtime, contact, sign * 70.);
        render(&mut runtime, 30.);
        assert_month(&runtime, 2);
        assert_eq!(runtime.focused(), Some(SELECTED));
        assert_eq!(bounds(&runtime, CURRENT).x, initial.x + sign * 70.);
        let neighbor = bounds(&runtime, PREVIEW);
        assert_eq!(neighbor.x, initial.x + sign * (70. - initial.width));
        assert!(neighbor.intersect(bounds(&runtime, GRID)).width > 0.);
        assert!(runtime.dom().by_id(NEXT).is_none());
        let frame = runtime.accessibility_frame().unwrap();
        let mut pending = vec![frame.root()];
        let mut grids = 0;
        while let Some(id) = pending.pop() {
            let node = frame.node(id).unwrap().node;
            grids += usize::from(node.accessibility_grid_role() == Some(GridRole::Grid));
            assert_ne!(node.stable_id(), Some(PREVIEW));
            pending.extend(frame.children(id));
        }
        assert_eq!(grids, 1);

        move_x(&mut runtime, contact, sign * 135.);
        render(&mut runtime, 160.);
        end_x(&mut runtime, contact, sign * 135.);
        render(&mut runtime, 180.);
        assert_month(&runtime, 2);
        render(&mut runtime, 400.);
        assert_month(&runtime, 3);
        assert_eq!(runtime.focused(), Some(NEXT));
        assert!(runtime.dom().by_id(PREVIEW).is_none());
        let target = bounds(&runtime, NEXT);
        assert_eq!(target.intersect(bounds(&runtime, GRID)).width, 48.);
        assert_eq!(
            target.intersect(bounds(&runtime, "picker-body")).height,
            48.
        );
    }
}

#[test]
fn escape_during_capture_restores_the_page_before_dismissing_the_modal() {
    let mut runtime = open(LayoutDirection::LeftToRight, 960., 720., false);
    let contact = origin(&runtime);
    begin(&mut runtime, contact);
    move_x(&mut runtime, contact, -75.);
    render(&mut runtime, 40.);
    assert!(runtime.dom().by_id(PREVIEW).is_some());
    runtime.dispatch(Event::Cancel).unwrap();
    render(&mut runtime, 65.);
    render(&mut runtime, 300.);
    assert_month(&runtime, 2);
    assert_eq!(runtime.focused(), Some(SELECTED));
    assert!(runtime.dom().by_id(PREVIEW).is_none());

    runtime.dispatch(Event::Cancel).unwrap();
    render(&mut runtime, 320.);
    assert!(runtime.app().session.is_none());
    assert_eq!(runtime.focused(), Some("open"));
}

#[test]
fn compact_pan_reaches_the_edge_before_a_new_rtl_gesture_pages() {
    let mut runtime = open(LayoutDirection::RightToLeft, 320., 568., true);
    let mut contact = origin(&runtime);
    let original = runtime.scroll_metrics(GRID).unwrap();
    assert_eq!(original.offset.0, 16.);

    // A rightward finger motion first pans to physical X=0.
    begin(&mut runtime, contact);
    move_x(&mut runtime, contact, 150.);
    render(&mut runtime, 40.);
    assert_eq!(runtime.scroll_metrics(GRID).unwrap().offset.0, 0.);
    end_x(&mut runtime, contact, 190.);
    render(&mut runtime, 60.);
    assert_month(&runtime, 2);
    assert!(runtime.dom().by_id(PREVIEW).is_none());

    contact = origin(&runtime);
    begin(&mut runtime, contact);
    // The threshold is one third of the 320 dp viewport.
    end_x(&mut runtime, contact, 110.);
    render(&mut runtime, 90.);
    assert_month(&runtime, 3);
    assert_eq!(runtime.focused(), Some(NEXT));
    assert!(runtime.dom().by_id(PREVIEW).is_none());
}

#[test]
fn compact_rtl_month_pages_meet_at_the_full_grid_edge() {
    let mut runtime = open(LayoutDirection::RightToLeft, 320., 568., false);
    let contact = origin(&runtime);
    begin(&mut runtime, contact);
    end_x(&mut runtime, contact, 150.);
    render(&mut runtime, 30.);
    assert_eq!(runtime.scroll_metrics(GRID).unwrap().offset.0, 0.);

    let contact = origin(&runtime);
    begin(&mut runtime, contact);
    move_x(&mut runtime, contact, 72.);
    render(&mut runtime, 60.);
    let current = bounds(&runtime, CURRENT);
    let neighbor = bounds(&runtime, PREVIEW);
    assert_eq!(bounds(&runtime, GRID).width, 320.);
    assert_eq!(current.width, 336.);
    assert_eq!(neighbor.width, current.width);
    assert!((current.x - neighbor.x - neighbor.width).abs() < 0.05);
    assert_eq!(current.intersect(neighbor).width, 0.);

    move_x(&mut runtime, contact, 348.);
    render(&mut runtime, 110.);
    assert_eq!(runtime.app().swipe.preview_offset(), Some(320.));
    end_x(&mut runtime, contact, 348.);
    render(&mut runtime, 130.);
    render(&mut runtime, 285.);
    assert!(runtime.app().swipe.preview_offset().unwrap() > 320.);
    render(&mut runtime, 330.);
    assert_month(&runtime, 3);
    assert!(runtime.dom().by_id(PREVIEW).is_none());
}

#[test]
fn short_compact_calendar_can_scroll_vertically_without_paging() {
    let mut runtime = open(LayoutDirection::LeftToRight, 320., 290., true);
    let before = runtime.scroll_metrics("picker-body").unwrap();
    let contact = origin(&runtime);
    begin(&mut runtime, contact);
    runtime
        .dispatch(Event::PointerUp {
            x: contact.0,
            y: contact.1 + 120.,
        })
        .unwrap();
    render(&mut runtime, 80.);
    assert!(runtime.scroll_metrics("picker-body").unwrap().offset.1 < before.offset.1);
    assert_month(&runtime, 2);
    assert!(runtime.dom().by_id(PREVIEW).is_none());
}
