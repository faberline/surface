use surface::{GridRole, Insets, Length, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{Application, EditKey, Event, Runtime, View};

#[derive(Clone)]
enum Message {
    Activate(&'static str),
    Remove,
    Disable,
    Modal(bool),
}

#[derive(Default)]
struct App {
    rtl: bool,
    scrolled: bool,
    nested: bool,
    removed: bool,
    disabled: bool,
    explicit_fallback: bool,
    modal: bool,
    actions: Vec<&'static str>,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Activate(id) => self.actions.push(id),
            Message::Remove => self.removed = true,
            Message::Disable => self.disabled = true,
            Message::Modal(open) => self.modal = open,
        }
    }

    fn view(&self) -> View<Message> {
        let mut split = vec![cell("body-cell", action("body", 48., false))];
        if !self.removed {
            let remove = action("remove", 48., self.disabled);
            let remove = if self.explicit_fallback {
                remove.restore_focus_on_removal("after")
            } else {
                remove
            };
            split.push(cell("remove-cell", remove));
        }
        let mut first_row = vec![
            cell("a-cell", action("a", 54., false)),
            cell("blocked-cell", action("blocked", 40., true)),
        ];
        first_row.extend(split);
        first_row.push(cell("zero-cell", action("zero", 0., false)));
        if self.nested {
            let inner = View::new(
                Node::new(Role::Group, "Nested options")
                    .id("inner-grid")
                    .grid_role(GridRole::Grid),
            )
            .children([row(
                "inner-row",
                self.rtl,
                [cell("inner-cell", action("inner", 48., false))],
            )]);
            first_row.push(cell("nested-cell", inner));
        }
        let unreachable = View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Px(0.))
                    .height(Length::Px(0.))
                    .clip(true),
            ),
        )
        .children([View::new(
            Node::new(Role::Button, "Outside permanent clip")
                .id("hidden")
                .styled(
                    Style::column()
                        .width(Length::Px(40.))
                        .height(Length::Px(48.))
                        .absolute(Insets::all(0.)),
                ),
        )
        .on_press(Message::Activate("hidden"))]);

        let mut style = Style::column().width(Length::Px(300.));
        if self.scrolled {
            style = style.height(Length::Px(72.)).clip(true);
        }
        let grid = View::new(
            Node::new(Role::Group, "Chips")
                .id("chips")
                .grid_role(GridRole::Grid)
                .styled(style),
        )
        .children([
            row(
                "unreachable-row",
                self.rtl,
                [cell("unreachable-cell", unreachable)],
            ),
            row("first-row", self.rtl, first_row),
            row(
                "blocked-row",
                self.rtl,
                [cell("empty-cell", action("blocked-row-control", 60., true))],
            ),
            row(
                "last-row",
                self.rtl,
                [
                    cell("c-cell", action("c", 30., false)),
                    cell("d-cell", action("d", 108., false)),
                    cell("e-cell", action("e", 60., false)),
                ],
            ),
        ])
        .roving_grid();
        let before = action("before", 100., false);
        let after = action("after", 100., false);
        let extra = self.modal.then(|| {
            View::new(Node::new(Role::Dialog, "Modal").id("modal"))
                .children([action("modal-action", 96., false)])
                .modal(Message::Modal(false))
        });
        View::new(Node::new(Role::Window, "Test"))
            .children([before, grid, after].into_iter().chain(extra))
    }
}

fn action(id: &'static str, width: f32, disabled: bool) -> View<Message> {
    View::new(
        Node::new(Role::Button, id)
            .id(id)
            .disabled(disabled)
            .styled(
                Style::column()
                    .width(Length::Px(width))
                    .height(Length::Px(48.)),
            ),
    )
    .on_press(Message::Activate(id))
}

fn cell(id: &str, action: View<Message>) -> View<Message> {
    View::new(
        Node::new(Role::Group, "")
            .id(id)
            .grid_role(GridRole::GridCell),
    )
    .children([action])
}

fn row(id: &str, rtl: bool, cells: impl IntoIterator<Item = View<Message>>) -> View<Message> {
    let row = Node::new(Role::Group, "")
        .id(id)
        .grid_role(GridRole::Row)
        .styled(Style::row().width(Length::Px(300.)).gap(4.));
    let row = if rtl { row.reverse_row() } else { row };
    View::new(row).children(cells)
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size / 2., size * 1.5)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime.render(330., 390., &mut Metrics).unwrap();
}

fn open(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime);
    runtime
}

fn focus(runtime: &mut Runtime<App>, reverse: bool, expected: &str) {
    runtime.dispatch(Event::FocusNext { reverse }).unwrap();
    assert_eq!(runtime.focused(), Some(expected));
}

fn edit(runtime: &mut Runtime<App>, key: EditKey, expected: &str) {
    assert!(
        runtime
            .dispatch(Event::Edit {
                key,
                extend: false,
                by_word: false
            })
            .unwrap(),
        "{key:?} should belong to the grid"
    );
    assert_eq!(runtime.focused(), Some(expected), "{key:?}");
}

fn bounds(runtime: &Runtime<App>, id: &str) -> Rect {
    runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
}

#[test]
fn one_tab_stop_physical_rows_and_authored_row_edges_in_both_directions() {
    for rtl in [false, true] {
        let mut runtime = open(App {
            rtl,
            ..App::default()
        });
        let next = if rtl { EditKey::Left } else { EditKey::Right };
        let previous = if rtl { EditKey::Right } else { EditKey::Left };
        focus(&mut runtime, false, "before");
        focus(&mut runtime, false, "a");
        edit(&mut runtime, previous, "a");
        edit(&mut runtime, next, "body");
        edit(&mut runtime, EditKey::Down, "d");
        edit(&mut runtime, EditKey::End, "e");
        edit(&mut runtime, EditKey::Home, "c");
        edit(&mut runtime, EditKey::Up, "a");
        edit(&mut runtime, EditKey::End, "remove");
        edit(&mut runtime, next, "remove");
        edit(&mut runtime, EditKey::Down, "e");
        edit(&mut runtime, EditKey::Down, "e");
        edit(&mut runtime, EditKey::Up, "remove");
        assert!(runtime.app().actions.is_empty());
        runtime.dispatch(Event::Activate).unwrap();
        assert_eq!(runtime.app().actions, ["remove"]);
        render(&mut runtime);
        focus(&mut runtime, false, "after");
        focus(&mut runtime, true, "a");
        focus(&mut runtime, true, "before");
    }
}

#[test]
fn fully_clipped_revealable_cell_takes_focus_and_scrolls_but_permanently_hidden_cell_does_not() {
    let mut runtime = open(App {
        scrolled: true,
        ..App::default()
    });
    let initial = bounds(&runtime, "d").intersect(bounds(&runtime, "chips"));
    assert_eq!(initial.height, 0.);
    assert!(bounds(&runtime, "hidden").width > 0.);
    focus(&mut runtime, false, "before");
    focus(&mut runtime, false, "a");
    edit(&mut runtime, EditKey::Right, "body");
    edit(&mut runtime, EditKey::Down, "d");
    render(&mut runtime);
    assert_eq!(
        bounds(&runtime, "d").intersect(bounds(&runtime, "chips")),
        bounds(&runtime, "d")
    );
    assert!(runtime.scroll_metrics("chips").unwrap().offset.1 > 0.);
    assert!(runtime.app().actions.is_empty());
}

#[test]
fn nested_grid_actions_keep_their_own_focus_order() {
    let mut runtime = open(App {
        nested: true,
        ..App::default()
    });
    focus(&mut runtime, false, "before");
    focus(&mut runtime, false, "a");
    edit(&mut runtime, EditKey::End, "remove");
    edit(&mut runtime, EditKey::Right, "remove");
    focus(&mut runtime, false, "inner");
    assert!(!runtime
        .dispatch(Event::Edit {
            key: EditKey::Right,
            extend: false,
            by_word: false,
        })
        .unwrap());
    assert_eq!(runtime.focused(), Some("inner"));
}

#[test]
fn lost_or_disabled_focus_prefers_nearest_surviving_peer_after_layout() {
    for message in [Message::Remove, Message::Disable] {
        let mut runtime = open(App::default());
        focus(&mut runtime, false, "before");
        focus(&mut runtime, false, "a");
        edit(&mut runtime, EditKey::End, "remove");
        runtime.apply(message).unwrap();
        render(&mut runtime);
        assert_eq!(runtime.focused(), Some("body"));
        assert!(runtime.app().actions.is_empty());
    }
}

#[test]
fn explicit_fallback_and_new_modal_keep_priority_over_the_grid() {
    let mut runtime = open(App {
        explicit_fallback: true,
        ..App::default()
    });
    focus(&mut runtime, false, "before");
    focus(&mut runtime, false, "a");
    edit(&mut runtime, EditKey::End, "remove");
    runtime.apply(Message::Remove).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("after"));

    focus(&mut runtime, true, "a");
    runtime.apply(Message::Modal(true)).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("modal-action"));
    assert!(!runtime
        .dispatch(Event::Edit {
            key: EditKey::Down,
            extend: false,
            by_word: false,
        })
        .unwrap());
    assert_eq!(runtime.focused(), Some("modal-action"));
    runtime.dispatch(Event::Cancel).unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("a"));
    assert!(runtime.app().actions.is_empty());
}

#[test]
fn one_authored_row_can_reflow_into_different_measured_rows() {
    struct Wrap;

    impl Application for Wrap {
        type Message = Message;

        fn update(&mut self, _: Message) {
            panic!("grid navigation must not activate")
        }

        fn view(&self) -> View<Message> {
            View::new(
                Node::new(Role::Group, "Wrapped chips")
                    .id("wrap-grid")
                    .grid_role(GridRole::Grid)
                    .styled(Style::column().width(Length::Percent(1.))),
            )
            .roving_grid()
            .children([View::new(
                Node::new(Role::Group, "")
                    .id("wrap-row")
                    .grid_role(GridRole::Row)
                    .styled(Style::row().width(Length::Percent(1.)).wrap(true).gap(4.)),
            )
            .children([
                cell("wrap-a", action("a", 150., false)),
                cell("wrap-b", action("b", 95., false)),
                cell("wrap-c", action("c", 104., false)),
                cell("wrap-d", action("d", 90., false)),
            ])])
        }
    }

    let mut runtime = Runtime::new(Wrap).unwrap();
    let press = |runtime: &mut Runtime<Wrap>, key, expected: &str| {
        assert!(runtime
            .dispatch(Event::Edit {
                key,
                extend: false,
                by_word: false,
            })
            .unwrap());
        assert_eq!(runtime.focused(), Some(expected));
    };
    runtime.render(320., 260., &mut Metrics).unwrap();
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    press(&mut runtime, EditKey::Right, "b");
    press(&mut runtime, EditKey::Down, "d");
    press(&mut runtime, EditKey::Home, "c");
    runtime.render(220., 260., &mut Metrics).unwrap();
    assert_eq!(runtime.focused(), Some("c"));
    press(&mut runtime, EditKey::Home, "b");
    press(&mut runtime, EditKey::Down, "d");
    press(&mut runtime, EditKey::Up, "b");
    press(&mut runtime, EditKey::Up, "a");
    press(&mut runtime, EditKey::Right, "a");
}
