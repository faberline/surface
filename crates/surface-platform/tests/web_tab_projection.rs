#[path = "../src/web_accessibility/tabs.rs"]
mod tabs;

use std::collections::BTreeSet;
use surface::{Length, Node, NodeId, Role, Style, TextMeasurer};
use surface_app::{Application, Runtime, View};

const FIRST: &str = "Inbox / 甲";
const SECOND: &str = "Inbox / 乙";
const FIRST_PANEL: &str = "Panel / 甲";
const SECOND_PANEL: &str = "Panel / 乙";

struct App(bool);

impl Application for App {
    type Message = bool;

    fn update(&mut self, second: bool) {
        self.0 = second;
    }

    fn view(&self) -> View<bool> {
        let tab = |id, panel, selected| {
            View::new(
                Node::new(Role::Tab, id)
                    .id(id)
                    .selected(selected)
                    .tab_controls(panel)
                    .styled(Style::column().height(Length::Px(42.))),
            )
        };
        let list = View::new(Node::new(Role::TabList, "Locations").id("list")).children([
            tab(FIRST, FIRST_PANEL, !self.0),
            tab(SECOND, SECOND_PANEL, self.0),
        ]);
        let (id, label) = if self.0 {
            (SECOND_PANEL, SECOND)
        } else {
            (FIRST_PANEL, FIRST)
        };
        let panel = View::new(
            Node::new(Role::Group, "")
                .id(id)
                .tab_panel_labelled_by("list", label)
                .styled(Style::column().height(Length::Px(60.))),
        )
        .children([View::new(
            Node::new(Role::Text, "").id("active-content").text(label),
        )]);
        View::new(Node::new(Role::Window, "Test")).children([list, panel])
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size, size)
    }
}

fn render(app: &mut Runtime<App>) {
    app.render(400., 300., &mut Metrics).unwrap();
}

fn projection(app: &Runtime<App>, omit: Option<NodeId>, occupied: &[String]) -> tabs::TabRelations {
    let visible: BTreeSet<_> = app
        .dom()
        .iter()
        .map(|(id, _)| id)
        .filter(|id| Some(*id) != omit)
        .collect();
    tabs::TabRelations::collect(&app.accessibility_frame().unwrap(), &visible, |id| {
        Ok::<_, ()>(!occupied.iter().any(|other| other == id))
    })
    .unwrap()
}

#[test]
fn both_idrefs_and_the_placeholder_keep_the_same_ids_when_selection_mounts_new_content() {
    let mut app = Runtime::new(App(false)).unwrap();
    render(&mut app);
    let first = projection(&app, None, &[]);
    let first_tab = app.dom().by_id(FIRST).unwrap();
    let second_tab = app.dom().by_id(SECOND).unwrap();
    let first_panel = app.dom().by_id(FIRST_PANEL).unwrap();
    assert!(app.dom().by_id(SECOND_PANEL).is_none());
    assert_eq!(first.panels.len(), 1);
    assert_eq!(
        first.panels[&first_panel].dom_id.as_ref(),
        Some(&first.controls[&first_tab])
    );
    assert_eq!(
        first.panels[&first_panel].labelled_by.as_ref(),
        Some(&first.tab_ids[&first_tab])
    );
    assert_eq!(
        first.placeholders,
        [(
            first.controls[&second_tab].clone(),
            first.tab_ids[&second_tab].clone()
        )]
    );
    let ids: BTreeSet<_> = first
        .controls
        .values()
        .chain(first.tab_ids.values())
        .collect();
    assert_eq!(ids.len(), 4);
    assert!(ids
        .iter()
        .all(|id| id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')));

    app.apply(true).unwrap();
    render(&mut app);
    let second = projection(&app, None, &[]);
    let second_panel = app.dom().by_id(SECOND_PANEL).unwrap();
    assert!(app.dom().by_id(FIRST_PANEL).is_none());
    assert_eq!(first.tab_ids, second.tab_ids);
    assert_eq!(first.controls, second.controls);
    assert_eq!(
        first.placeholders[0].0,
        *second.panels[&second_panel].dom_id.as_ref().unwrap()
    );
    assert_eq!(
        second.placeholders[0].0,
        *first.panels[&first_panel].dom_id.as_ref().unwrap()
    );
}

#[test]
fn clipped_endpoints_and_foreign_ids_never_produce_dangling_or_ambiguous_references() {
    let mut app = Runtime::new(App(false)).unwrap();
    render(&mut app);
    let first_tab = app.dom().by_id(FIRST).unwrap();
    let second_tab = app.dom().by_id(SECOND).unwrap();
    let first_panel = app.dom().by_id(FIRST_PANEL).unwrap();
    let baseline = projection(&app, None, &[]);

    let clipped_tab = projection(&app, Some(first_tab), &[]);
    assert!(!clipped_tab.controls.contains_key(&first_tab));
    assert_eq!(clipped_tab.panels[&first_panel].labelled_by, None);
    assert_eq!(clipped_tab.panels[&first_panel].fallback_label, FIRST);

    let clipped_panel = projection(&app, Some(first_panel), &[]);
    assert!(!clipped_panel.controls.contains_key(&first_tab));
    assert!(!clipped_panel.panels.contains_key(&first_panel));
    assert_eq!(clipped_panel.placeholders.len(), 1);

    let foreign = [
        baseline.controls[&first_tab].clone(),
        baseline.tab_ids[&second_tab].clone(),
    ];
    let blocked = projection(&app, None, &foreign);
    assert!(blocked.controls.is_empty());
    assert!(blocked.tab_ids.is_empty());
    assert!(blocked.placeholders.is_empty());
    assert_eq!(blocked.panels[&first_panel].dom_id, None);
    assert_eq!(blocked.panels[&first_panel].labelled_by, None);
}
