use std::collections::{BTreeMap, BTreeSet};

use surface::{NodeId, Role};
use surface_app::AccessibilityFrame;

#[derive(Default, PartialEq, Eq)]
pub(super) struct DescriptionRelations {
    references: BTreeMap<NodeId, Vec<NodeId>>,
    pub(super) text_targets: BTreeSet<NodeId>,
    pub(super) clipped_text: BTreeMap<NodeId, String>,
}

impl DescriptionRelations {
    pub(super) fn collect<M>(
        frame: &AccessibilityFrame<'_, M>,
        visible: &BTreeSet<NodeId>,
    ) -> Self {
        let mut relations = Self::default();
        for &source in visible {
            let mut targets = Vec::new();
            for (target, clipped) in frame.description_parts(source) {
                if let Some(text) = clipped {
                    relations.clipped_text.insert(target, text.to_owned());
                } else if !visible.contains(&target) {
                    continue;
                }
                if frame
                    .node(target)
                    .is_some_and(|node| node.node.role() == Role::Text)
                {
                    relations.text_targets.insert(target);
                }
                targets.push(target);
            }
            if !targets.is_empty() {
                relations.references.insert(source, targets);
            }
        }
        relations
    }

    pub(super) fn target_id(&self, id: NodeId) -> String {
        let kind = if self.text_targets.contains(&id) {
            "text"
        } else {
            "tooltip"
        };
        format!("surface-accessibility-{kind}-{}", id.index())
    }

    pub(super) fn described_by(&self, source: NodeId) -> Option<String> {
        Some(
            self.references
                .get(&source)?
                .iter()
                .map(|id| self.target_id(*id))
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}
