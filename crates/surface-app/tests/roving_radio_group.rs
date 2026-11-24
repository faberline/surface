use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{
    AccessibilityActivationResult, AccessibilityFocusResult, Application, EditKey, Event, Roving,
    Runtime, View,
};

const ROWS: &[&str] = &["a", "b", "c", "d"];

#[derive(Clone, Copy)]
struct Choices {
    rows: &'static [&'static str],
    checked: Option<&'static str>,
    disabled: &'static [&'static str],
    collapsed: Option<&'static str>,
}

impl Default for Choices {
    fn default() -> Self {
        Self {
            rows: ROWS,
            checked: Some("b"),
            disabled: &["c"],
            collapsed: None,
        }
    }
}

#[derive(Clone)]
enum Message {
    Select(&'static str),
    Replace(Choices),
}

struct App {
    choices: Choices,
    radio_policy: Option<bool>,
    requests: Vec<&'static str>,
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        match message {
            Message::Select(id) => {
                self.requests.push(id);
                self.choices.checked = Some(id);
            }
            Message::Replace(choices) => self.choices = choices,
        }
    }

    fn view(&self) -> View<Message> {
        let choices = self.choices;
        let rows = choices.rows.iter().map(|&id| {
            let checked = choices.checked == Some(id);
            let disabled = choices.disabled.contains(&id);
            let height = if choices.collapsed == Some(id) {
                0.
            } else {
                40.
            };
            let row = View::new(
                Node::new(Role::Radio, id)
                    .id(id)
                    .checked(checked)
                    .disabled(disabled)
                    .styled(Style::column().height(Length::Px(height))),
            );
            if disabled || checked {
                row
            } else {
                row.on_press(Message::Select(id)).accessible_activation()
            }
        });
        let group = View::new(
            Node::new(Role::RadioGroup, "Delivery")
                .id("delivery")
                .styled(Style::column().height(Length::Px(90.)).clip(true)),
        )
        .children(rows);
        let group = match self.radio_policy {
            Some(mirrored) => group.roving_radio_group(mirrored),
            None => group.roving(Roving::vertical(true)),
        };
        let button = |id| {
            View::new(
                Node::new(Role::Button, id)
                    .id(id)
                    .styled(Style::column().height(Length::Px(40.))),
            )
        };
        View::new(Node::new(Role::Window, "Test")).children([
            button("before"),
            group,
            button("after"),
        ])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, _: &str, _: f32, _: Option<f32>) -> (f32, f32) {
        (0., 0.)
    }
}

fn render(runtime: &mut Runtime<App>) {
    runtime.render(240., 220., &mut Metrics).unwrap();
}

fn fixture(radio_policy: Option<bool>, choices: Choices) -> Runtime<App> {
    let mut runtime = Runtime::new(App {
        choices,
        radio_policy,
        requests: Vec::new(),
    })
    .unwrap();
    render(&mut runtime);
    runtime
}

fn dispatch(runtime: &mut Runtime<App>, event: Event, focus: &str) {
    assert!(runtime.dispatch(event).unwrap());
    render(runtime);
    assert_eq!(runtime.focused(), Some(focus));
}

fn tab(runtime: &mut Runtime<App>, reverse: bool, focus: &str) {
    dispatch(runtime, Event::FocusNext { reverse }, focus);
}

fn edit(key: EditKey) -> Event {
    Event::Edit {
        key,
        extend: false,
        by_word: false,
    }
}

fn assert_entry(runtime: &mut Runtime<App>, forward: &str, reverse: &str) {
    tab(runtime, false, "before");
    tab(runtime, false, forward);
    tab(runtime, false, "after");
    tab(runtime, true, reverse);
    tab(runtime, true, "before");
    assert!(runtime.app().requests.is_empty());
}

#[test]
fn an_enabled_checked_member_takes_one_tab_stop_in_both_directions() {
    for mirrored in [false, true] {
        let mut runtime = fixture(Some(mirrored), Choices::default());
        assert_entry(&mut runtime, "b", "b");
        assert_eq!(runtime.app().choices.checked, Some("b"));
    }
}

#[test]
fn no_eligible_checked_member_uses_first_forward_and_last_backward() {
    for choices in [
        Choices {
            checked: None,
            ..Choices::default()
        },
        Choices {
            disabled: &["b", "c"],
            ..Choices::default()
        },
        Choices {
            collapsed: Some("b"),
            ..Choices::default()
        },
    ] {
        let mut runtime = fixture(Some(false), choices);
        assert_entry(&mut runtime, "a", "d");
        assert_eq!(runtime.app().choices.checked, choices.checked);
        assert!(runtime.scroll_metrics("delivery").is_some());
    }
}

#[test]
fn an_all_disabled_group_yields_to_both_neighboring_controls() {
    let mut runtime = fixture(
        Some(false),
        Choices {
            disabled: ROWS,
            ..Choices::default()
        },
    );
    tab(&mut runtime, false, "before");
    tab(&mut runtime, false, "after");
    tab(&mut runtime, true, "before");
    assert!(runtime.app().requests.is_empty());
}

#[test]
fn each_arrow_selects_once_and_wraps_past_disabled_rows_with_rtl_mirroring() {
    for (mirrored, forward, backward) in [
        (false, EditKey::Right, EditKey::Left),
        (true, EditKey::Left, EditKey::Right),
    ] {
        let mut runtime = fixture(Some(mirrored), Choices::default());
        tab(&mut runtime, false, "before");
        tab(&mut runtime, false, "b");
        let mut requests = Vec::new();
        for (key, target) in [
            (forward, "d"),
            (EditKey::Down, "a"),
            (backward, "d"),
            (EditKey::Up, "b"),
        ] {
            dispatch(&mut runtime, edit(key), target);
            requests.push(target);
            assert_eq!(runtime.app().requests, requests);
            assert_eq!(runtime.app().choices.checked, Some(target));
            let node = runtime.dom().by_id(target).unwrap();
            assert_eq!(runtime.accessibility_frame().unwrap().focused(), Some(node));
        }
        assert!(!runtime.dispatch(Event::Activate).unwrap());
        assert_eq!(runtime.app().requests, requests);
    }
}

#[test]
fn a_single_checked_option_consumes_arrows_without_an_extra_request() {
    let mut runtime = fixture(
        Some(false),
        Choices {
            disabled: &["a", "c", "d"],
            ..Choices::default()
        },
    );
    tab(&mut runtime, false, "before");
    tab(&mut runtime, false, "b");
    dispatch(&mut runtime, edit(EditKey::Right), "b");
    assert!(runtime.app().requests.is_empty());
}

#[test]
fn a_removed_or_disabled_focused_member_uses_the_current_group_without_activation() {
    for (choices, fallback) in [
        (
            Choices {
                rows: &["a", "c", "d"],
                checked: Some("d"),
                ..Choices::default()
            },
            Some("d"),
        ),
        (
            Choices {
                disabled: &["b", "c"],
                ..Choices::default()
            },
            Some("a"),
        ),
        (
            Choices {
                disabled: ROWS,
                ..Choices::default()
            },
            None,
        ),
    ] {
        let mut runtime = fixture(Some(false), Choices::default());
        tab(&mut runtime, false, "before");
        tab(&mut runtime, false, "b");
        runtime.apply(Message::Replace(choices)).unwrap();
        render(&mut runtime);
        assert_eq!(runtime.focused(), fallback);
        assert_eq!(runtime.app().choices.checked, choices.checked);
        assert!(runtime.app().requests.is_empty());
    }
}

#[test]
fn a_projected_focus_action_is_distinct_from_selection() {
    let mut runtime = fixture(Some(false), Choices::default());
    let id = runtime.dom().by_id("a").unwrap();
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .focus_target(id)
        .unwrap();
    assert_eq!(
        runtime.accessibility_focus(&target).unwrap(),
        AccessibilityFocusResult::Moved,
    );
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("a"));
    assert_eq!(runtime.app().choices.checked, Some("b"));
    assert!(runtime.app().requests.is_empty());
    let target = runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id)
        .unwrap();
    assert_eq!(
        runtime.accessibility_activate(&target).unwrap(),
        AccessibilityActivationResult::Activated,
    );
    render(&mut runtime);
    assert_eq!(runtime.app().requests, ["a"]);
    assert_eq!(runtime.app().choices.checked, Some("a"));
    assert!(runtime
        .accessibility_frame()
        .unwrap()
        .activation_target(id)
        .is_none());
}

#[test]
fn legacy_vertical_roving_keeps_first_reverse_entry_and_axis_only_keys() {
    let mut runtime = fixture(
        None,
        Choices {
            checked: None,
            ..Choices::default()
        },
    );
    assert_entry(&mut runtime, "a", "a");
    tab(&mut runtime, false, "a");
    assert!(!runtime.dispatch(edit(EditKey::Right)).unwrap());
    assert_eq!(runtime.focused(), Some("a"));
    assert!(runtime.app().requests.is_empty());
    dispatch(&mut runtime, edit(EditKey::Down), "b");
    assert_eq!(runtime.app().requests, ["b"]);
}
