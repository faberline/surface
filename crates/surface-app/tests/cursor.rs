//! Cursor choice (issue #121). Headless: a fixed-metric measurer, no window,
//! no real mouse. `Runtime::cursor_at` is plain data; the host maps it.
use surface::{Insets, Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, CursorStyle, Event, Runtime, View};

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.5, size * 1.5)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Open,
    Close,
    Text(String),
    Noop,
}

struct App {
    /// Attach the explicit cursors. Off gives the same tree without them.
    cursors: bool,
    modal: bool,
}
impl Application for App {
    type Message = Msg;
    fn update(&mut self, message: Msg) {
        match message {
            Msg::Open => self.modal = true,
            Msg::Close => self.modal = false,
            _ => {}
        }
    }
    fn view(&self) -> View<Msg> {
        let c = |view: View<Msg>, style: CursorStyle| {
            if self.cursors {
                view.cursor(style)
            } else {
                view
            }
        };
        let row = |role: Role, id: &'static str| {
            Node::new(role, id)
                .id(id)
                .styled(Style::default().height(Length::Px(40.)))
        };
        // Rows are 40 px tall and stacked, so row n covers y = 40n .. 40n + 40.
        let mut children = vec![
            // 0: enabled button with a Pointer.
            c(
                View::new(row(Role::Button, "go")).on_press(Msg::Open),
                CursorStyle::Pointer,
            ),
            // 1: text field with a Text cursor.
            c(
                View::new(row(Role::TextInput, "field")).on_input(Msg::Text),
                CursorStyle::Text,
            ),
            // 2: disabled button with a Pointer. It is not a candidate.
            c(
                View::new(row(Role::Button, "off").disabled(true)).on_press(Msg::Noop),
                CursorStyle::Pointer,
            ),
            // 3: a handle. Not focusable, so `hit_test` never returns it.
            c(
                View::new(row(Role::Separator, "handle")),
                CursorStyle::ResizeHorizontal,
            ),
            // 4: overlap. The child is painted after its parent and wins.
            c(
                View::new(
                    Node::new(Role::Group, "outer")
                        .id("outer")
                        .styled(Style::column().height(Length::Px(40.))),
                ),
                CursorStyle::Grab,
            )
            .children([c(
                View::new(row(Role::Button, "inner")).on_press(Msg::Noop),
                CursorStyle::NotAllowed,
            )]),
            // 5: focusable, no explicit cursor.
            View::new(row(Role::Button, "bare")).on_press(Msg::Noop),
            // 6: not focusable, no explicit cursor.
            View::new(row(Role::Group, "plain")),
        ];
        if self.modal {
            let mut style = Style::column().width(Length::Px(200.)).fixed(Insets {
                left: Length::Px(300.),
                top: Length::Px(300.),
                ..Default::default()
            });
            style.z_index = 1001;
            children.push(
                View::new(
                    Node::new(Role::Menu, "Dialog")
                        .id("dialog")
                        .tab_index(-1)
                        .styled(style),
                )
                .children([c(
                    View::new(row(Role::Button, "ok")).on_press(Msg::Close),
                    CursorStyle::Pointer,
                )])
                .modal(Msg::Close),
            );
        }
        View::new(Node::new(Role::Window, "App").styled(Style::column())).children(children)
    }
}

fn runtime(cursors: bool) -> Runtime<App> {
    let mut rt = Runtime::new(App {
        cursors,
        modal: false,
    })
    .unwrap();
    rt.render(640., 480., &mut Metrics).unwrap();
    rt
}
const ROW: f32 = 20.;
fn at(rt: &Runtime<App>, row: usize) -> CursorStyle {
    rt.cursor_at(20., row as f32 * 40. + ROW)
}

// Acceptance row 1.
#[test]
fn enabled_node_gives_its_explicit_cursor() {
    let rt = runtime(true);
    assert_eq!(at(&rt, 0), CursorStyle::Pointer);
    assert_eq!(at(&rt, 1), CursorStyle::Text);
}

#[test]
fn disabled_node_is_not_a_candidate() {
    let rt = runtime(true);
    assert_eq!(at(&rt, 2), CursorStyle::Default);
}

#[test]
fn non_focusable_handle_with_an_explicit_cursor_counts() {
    let rt = runtime(true);
    assert_eq!(at(&rt, 3), CursorStyle::ResizeHorizontal);
}

#[test]
fn last_painted_of_two_overlapping_nodes_wins() {
    let rt = runtime(true);
    assert_eq!(at(&rt, 4), CursorStyle::NotAllowed);
}

#[test]
fn focusable_without_a_cursor_and_empty_space_give_default() {
    let rt = runtime(true);
    // No policy by role: a bare Button is not a hand.
    assert_eq!(at(&rt, 5), CursorStyle::Default);
    assert_eq!(at(&rt, 6), CursorStyle::Default);
    // Below every row, and outside the viewport.
    assert_eq!(rt.cursor_at(20., 400.), CursorStyle::Default);
    assert_eq!(rt.cursor_at(-5., 20.), CursorStyle::Default);
    assert_eq!(rt.cursor_at(700., 20.), CursorStyle::Default);
}

#[test]
fn no_layout_yet_gives_default() {
    let rt = Runtime::new(App {
        cursors: true,
        modal: false,
    })
    .unwrap();
    assert_eq!(rt.cursor_at(20., 20.), CursorStyle::Default);
}

#[test]
fn nodes_outside_the_modal_scope_give_default() {
    let mut rt = runtime(true);
    rt.dispatch(Event::PointerDown { x: 20., y: 20. }).unwrap();
    rt.dispatch(Event::PointerUp { x: 20., y: 20. }).unwrap();
    rt.render(640., 480., &mut Metrics).unwrap();
    assert!(rt.app().modal, "the press opened the dialog");
    // The Pointer button and the Text field are behind the dialog.
    assert_eq!(rt.cursor_at(20., 20.), CursorStyle::Default);
    assert_eq!(rt.cursor_at(20., 60.), CursorStyle::Default);
    // The dialog's own button is in scope.
    assert_eq!(rt.cursor_at(320., 320.), CursorStyle::Pointer);
}

#[test]
fn cursor_at_does_not_change_state() {
    let rt = runtime(true);
    let before = rt.export_dom().unwrap();
    let _ = rt.cursor_at(20., 20.);
    assert_eq!(rt.export_dom().unwrap(), before);
}

#[test]
fn default_style_is_default() {
    assert_eq!(CursorStyle::default(), CursorStyle::Default);
}

// Acceptance row 2.
#[test]
fn explicit_cursors_change_no_output() {
    let mut with = Runtime::new(App {
        cursors: true,
        modal: false,
    })
    .unwrap();
    let mut without = Runtime::new(App {
        cursors: false,
        modal: false,
    })
    .unwrap();
    let list_with = with.render(640., 480., &mut Metrics).unwrap();
    let list_without = without.render(640., 480., &mut Metrics).unwrap();
    assert_eq!(with.export_dom().unwrap(), without.export_dom().unwrap());
    assert_eq!(with.computed(), without.computed());
    assert_eq!(list_with, list_without);
    // Negative: the cursors really differ, so the equality above means something.
    assert_ne!(with.cursor_at(20., 20.), without.cursor_at(20., 20.));
}
