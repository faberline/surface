//! Plan tab ID references for the nodes included in the current Web projection.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use surface::NodeId;
use surface_app::AccessibilityFrame;

#[derive(Default, PartialEq, Eq)]
pub(super) struct TabRelations {
    pub(super) controls: BTreeMap<NodeId, String>,
    pub(super) tab_ids: BTreeMap<NodeId, String>,
    pub(super) panels: BTreeMap<NodeId, Panel>,
    pub(super) placeholders: Vec<(String, String)>,
}

#[derive(PartialEq, Eq)]
pub(super) struct Panel {
    pub(super) dom_id: Option<String>,
    pub(super) labelled_by: Option<String>,
    pub(super) fallback_label: String,
}

impl TabRelations {
    pub(super) fn collect<M, E>(
        frame: &AccessibilityFrame<'_, M>,
        visible: &BTreeSet<NodeId>,
        mut id_available: impl FnMut(&str) -> Result<bool, E>,
    ) -> Result<Self, E> {
        let mut relations = Self::default();
        for &id in visible {
            if let Some(tab) = frame.tab_panel_tab(id) {
                let source = frame.node(id).expect("resolved panel");
                let tab_node = frame.node(tab).expect("resolved tab");
                let panel = stable_id("tabpanel", source.node.stable_id().expect("panel ID"));
                let label = stable_id("tab", tab_node.node.stable_id().expect("tab ID"));
                let panel_available = id_available(&panel)?;
                let linked = panel_available && visible.contains(&tab) && id_available(&label)?;
                if linked {
                    relations.controls.insert(tab, panel.clone());
                    relations.tab_ids.insert(tab, label.clone());
                }
                relations.panels.insert(
                    id,
                    Panel {
                        dom_id: panel_available.then_some(panel),
                        labelled_by: linked.then_some(label),
                        fallback_label: tab_node.node.name().to_owned(),
                    },
                );
            } else if let Some(panel) = frame.lazy_tab_panel(id) {
                let tab_node = frame.node(id).expect("resolved tab");
                let panel = stable_id("tabpanel", panel);
                let label = stable_id("tab", tab_node.node.stable_id().expect("tab ID"));
                if id_available(&panel)? && id_available(&label)? {
                    relations.controls.insert(id, panel.clone());
                    relations.tab_ids.insert(id, label.clone());
                    relations.placeholders.push((panel, label));
                }
            }
        }
        Ok(relations)
    }
}

// Hex is injective for string bytes, including whitespace or control
// characters. The separate prefixes also keep a tab and a panel distinct.
fn stable_id(kind: &str, key: &str) -> String {
    let mut id = format!("surface-accessibility-{kind}-");
    for byte in key.bytes() {
        write!(id, "{byte:02x}").expect("writing to String");
    }
    id
}
