use surface::{GridRole, Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Runtime, View};

const LIST: &str = "tabs";
const ACTIVE: &str = "messages";
const LAZY: &str = "activity";
const PANEL: &str = "messages-panel";
const LAZY_PANEL: &str = "activity-panel";

#[derive(Clone, Copy, Default)]
enum Case {
    #[default]
    Linked,
    MissingActive,
    WrongRole,
    WrongParent,
    WrongOwner,
    Nonreciprocal,
    UnnamedTab,
    UnnamedList,
    NearestList,
    DuplicateClaim,
    DuplicateLazyClaim,
    LazyIdTaken,
    InactiveMounted,
    Modal,
    ModalCrossing,
    ClippedTab,
    ConflictingPanelRole,
}

struct App(Case);

fn row() -> Style {
    Style::column().height(Length::Px(32.))
}

impl Application for App {
    type Message = ();

    fn update(&mut self, _: ()) {}

    fn view(&self) -> View<()> {
        let case = self.0;
        let active = View::new(
            Node::new(
                Role::Tab,
                if matches!(case, Case::UnnamedTab) {
                    " "
                } else {
                    "Messages"
                },
            )
            .id(ACTIVE)
            .selected(!matches!(case, Case::InactiveMounted))
            .tab_controls(if matches!(case, Case::Nonreciprocal) {
                "unrelated-panel"
            } else {
                PANEL
            })
            .styled(row()),
        )
        .accessibility_scroll_into_view();
        let active = if matches!(case, Case::NearestList) {
            View::new(Node::new(Role::TabList, "Nested").id("nested")).children([active])
        } else {
            active
        };
        let lazy = View::new(
            Node::new(Role::Tab, "Activity")
                .id(LAZY)
                .selected(false)
                .tab_controls(if matches!(case, Case::DuplicateClaim) {
                    PANEL
                } else {
                    LAZY_PANEL
                })
                .styled(row()),
        );
        let duplicate_lazy = matches!(case, Case::DuplicateLazyClaim).then(|| {
            View::new(
                Node::new(Role::Tab, "History")
                    .id("history")
                    .selected(false)
                    .tab_controls(LAZY_PANEL)
                    .styled(row()),
            )
        });
        let mut strip = Style::column();
        if matches!(case, Case::ClippedTab) {
            strip.height = Length::Px(32.);
            strip.clip = true;
        }
        let list = View::new(
            Node::new(
                Role::TabList,
                if matches!(case, Case::UnnamedList) {
                    " "
                } else {
                    "Destinations"
                },
            )
            .id(LIST)
            .styled(strip),
        )
        .children([View::new(Node::new(Role::Group, "Options"))
            .children([lazy, active].into_iter().chain(duplicate_lazy))]);

        let panel = (!matches!(case, Case::MissingActive)).then(|| {
            let node = Node::new(
                if matches!(case, Case::WrongRole) {
                    Role::Button
                } else {
                    Role::Group
                },
                "Messages panel",
            )
            .id(PANEL)
            .tab_index(0)
            .styled(Style::column().height(Length::Px(64.)))
            .tab_panel_labelled_by(
                if matches!(case, Case::WrongOwner) {
                    "other-list"
                } else {
                    LIST
                },
                ACTIVE,
            );
            let panel = View::new(if matches!(case, Case::ConflictingPanelRole) {
                node.grid_role(GridRole::Grid)
            } else {
                node
            });
            if matches!(case, Case::WrongParent) {
                View::new(Node::new(Role::Group, "Other content")).children([panel])
            } else {
                panel
            }
        });
        let taken = matches!(case, Case::LazyIdTaken)
            .then(|| View::new(Node::new(Role::Text, "Standalone information").id(LAZY_PANEL)));

        let root = View::new(Node::new(Role::Window, "Test"));
        match case {
            Case::Modal => {
                root.children([
                    View::new(Node::new(Role::Dialog, "Tabs dialog").id("dialog"))
                        .modal(())
                        .children(std::iter::once(list).chain(panel)),
                ])
            }
            Case::ModalCrossing => root.children(
                std::iter::once(
                    View::new(Node::new(Role::Dialog, "Tabs dialog").id("dialog"))
                        .modal(())
                        .children([list]),
                )
                .chain(panel),
            ),
            _ => root.children(std::iter::once(list).chain(panel).chain(taken)),
        }
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size, size)
    }
}

fn render(case: Case) -> Runtime<App> {
    let mut runtime = Runtime::new(App(case)).unwrap();
    runtime.render(320., 300., &mut Metrics).unwrap();
    runtime
}

#[test]
fn a_named_list_through_wrappers_resolves_mounted_and_lazy_panels() {
    for case in [Case::Linked, Case::Modal] {
        let runtime = render(case);
        let frame = runtime.accessibility_frame().unwrap();
        let id = |key| runtime.dom().by_id(key).unwrap();
        assert_eq!(frame.tab_panel_tab(id(PANEL)), Some(id(ACTIVE)));
        assert!(frame.focus_target(id(PANEL)).is_some());
        assert_eq!(frame.lazy_tab_panel(id(ACTIVE)), None);
        assert_eq!(frame.lazy_tab_panel(id(LAZY)), Some(LAZY_PANEL));
        assert!(runtime.dom().by_id(LAZY_PANEL).is_none());
    }
}

#[test]
fn a_mounted_role_conflict_or_unscoped_link_is_rejected() {
    for case in [
        Case::WrongRole,
        Case::WrongParent,
        Case::WrongOwner,
        Case::Nonreciprocal,
        Case::UnnamedTab,
        Case::UnnamedList,
        Case::NearestList,
        Case::DuplicateClaim,
        Case::InactiveMounted,
        Case::ModalCrossing,
        Case::ConflictingPanelRole,
    ] {
        let runtime = render(case);
        let frame = runtime.accessibility_frame().unwrap();
        let panel = runtime.dom().by_id(PANEL).unwrap();
        let active = runtime.dom().by_id(ACTIVE).unwrap();
        assert_eq!(frame.tab_panel_tab(panel), None);
        assert_eq!(frame.lazy_tab_panel(active), None);
    }
}

#[test]
fn a_lazy_id_must_be_unique_and_cannot_shadow_any_mounted_node() {
    for case in [
        Case::DuplicateLazyClaim,
        Case::LazyIdTaken,
        Case::UnnamedList,
    ] {
        let runtime = render(case);
        let frame = runtime.accessibility_frame().unwrap();
        assert_eq!(
            frame.lazy_tab_panel(runtime.dom().by_id(LAZY).unwrap()),
            None
        );
    }
    let runtime = render(Case::MissingActive);
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(
        frame.lazy_tab_panel(runtime.dom().by_id(ACTIVE).unwrap()),
        None
    );
    assert_eq!(
        frame.lazy_tab_panel(runtime.dom().by_id(LAZY).unwrap()),
        Some(LAZY_PANEL)
    );
}

#[test]
fn clipped_tab_retains_the_panel_identity_and_its_explicit_reveal_action() {
    let runtime = render(Case::ClippedTab);
    let frame = runtime.accessibility_frame().unwrap();
    let active = runtime.dom().by_id(ACTIVE).unwrap();
    let panel = runtime.dom().by_id(PANEL).unwrap();
    assert_eq!(frame.tab_panel_tab(panel), Some(active));
    assert!(frame.focus_target(active).is_none());
    assert!(frame.scroll_target(active).is_some());
}
