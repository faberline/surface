//! The surface snapshot: a schema-versioned, flat, serializable capture of an
//! element tree, its builder and its queries.
//!
//! Node ids are structural paths: the root is `root`, a child is
//! `<parent>/<index>`, and a fragment's children are `<node>/fragment/<index>`
//! under the fragment's own parent. Fragments and `Empty` add no node.

use serde::{Deserialize, Serialize};

use crate::element::Element;
use crate::props::Props;
use crate::surface_node::{SurfaceNode, SurfaceNodeKind, SurfaceProps, SurfaceRect};

/// Serializable snapshot of a rendered surface tree.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SurfaceSnapshot {
    pub schema_version: u32,
    pub nodes: Vec<SurfaceNode>,
}

impl SurfaceSnapshot {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn from_element(root: &Element) -> Self {
        let mut snapshot = Self {
            schema_version: Self::SCHEMA_VERSION,
            nodes: Vec::new(),
        };
        push_snapshot_nodes(root, None, "root".to_string(), &mut snapshot);
        snapshot
    }

    pub fn get(&self, node_id: &str) -> Option<&SurfaceNode> {
        self.nodes.iter().find(|node| node.node_id == node_id)
    }

    pub fn find_by_semantic_id(&self, semantic_id: &str) -> Option<&SurfaceNode> {
        self.nodes
            .iter()
            .find(|node| node.semantic_id == semantic_id)
    }

    pub fn find_by_role<'a>(&'a self, role: &str) -> Vec<&'a SurfaceNode> {
        self.nodes
            .iter()
            .filter(|node| node.role.as_deref() == Some(role))
            .collect()
    }

    pub fn text_content(&self) -> String {
        self.nodes
            .iter()
            .filter_map(|node| node.text.as_deref())
            .collect()
    }

    pub fn set_bounds(&mut self, node_id: &str, bounds: SurfaceRect) -> bool {
        let Some(node) = self.nodes.iter_mut().find(|node| node.node_id == node_id) else {
            return false;
        };
        node.bounds = Some(bounds);
        true
    }
}

fn push_snapshot_nodes(
    element: &Element,
    parent_id: Option<String>,
    node_id: String,
    snapshot: &mut SurfaceSnapshot,
) {
    match element {
        Element::Intrinsic {
            tag,
            props,
            children,
        } => {
            let semantic_id = props.id().map_or_else(|| node_id.clone(), str::to_string);
            let name = accessible_name(tag, props, children);
            snapshot.nodes.push(SurfaceNode {
                node_id: node_id.clone(),
                semantic_id,
                parent_id,
                kind: SurfaceNodeKind::Element,
                tag: Some((*tag).to_string()),
                component: None,
                role: role_for(tag, props),
                name,
                text: None,
                props: SurfaceProps::from(props),
                bounds: None,
            });
            for (idx, child) in children.iter().enumerate() {
                push_snapshot_nodes(
                    child,
                    Some(node_id.clone()),
                    format!("{node_id}/{idx}"),
                    snapshot,
                );
            }
        }
        Element::Text(text) => {
            snapshot.nodes.push(SurfaceNode {
                node_id: node_id.clone(),
                semantic_id: node_id,
                parent_id,
                kind: SurfaceNodeKind::Text,
                tag: None,
                component: None,
                role: None,
                name: None,
                text: Some(text.clone()),
                props: SurfaceProps::default(),
                bounds: None,
            });
        }
        Element::Component(component) => {
            snapshot.nodes.push(SurfaceNode {
                node_id: node_id.clone(),
                semantic_id: node_id,
                parent_id,
                kind: SurfaceNodeKind::Component,
                tag: None,
                component: Some(component.name().to_string()),
                role: None,
                name: Some(component.name().to_string()),
                text: None,
                props: SurfaceProps::default(),
                bounds: None,
            });
        }
        Element::Fragment(children) => {
            for (idx, child) in children.iter().enumerate() {
                push_snapshot_nodes(
                    child,
                    parent_id.clone(),
                    format!("{node_id}/fragment/{idx}"),
                    snapshot,
                );
            }
        }
        Element::Empty => {}
    }
}

fn role_for(tag: &str, props: &Props) -> Option<String> {
    let role = match tag {
        "button" => "button",
        "input" if props.input_type() == Some("checkbox") => "checkbox",
        "input" | "textarea" => "textbox",
        "label" => "label",
        "main" => "main",
        "nav" => "navigation",
        "table" => "table",
        "tr" => "row",
        "td" => "cell",
        "th" => "columnheader",
        "ul" | "ol" => "list",
        "li" => "listitem",
        _ => return None,
    };
    Some(role.to_string())
}

fn accessible_name(tag: &str, props: &Props, children: &[Element]) -> Option<String> {
    if let Some(label) = props.aria_label().filter(|s| !s.is_empty()) {
        return Some(label.to_string());
    }
    match tag {
        "button" | "label" | "td" | "th" => {
            let text = children
                .iter()
                .map(Element::text_content)
                .collect::<String>();
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_string())
        }
        "input" | "textarea" => props
            .value()
            .filter(|s| !s.is_empty())
            .or_else(|| props.placeholder().filter(|s| !s.is_empty()))
            .map(str::to_string),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
