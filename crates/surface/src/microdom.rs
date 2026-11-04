//! An indexed semantic tree. No browser DOM objects or renderer handles live here.
use crate::Style;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(u32);
impl NodeId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Role {
    #[default]
    Group,
    Window,
    Heading,
    Text,
    Button,
    Checkbox,
    TextInput,
    Navigation,
    Radio,
    RadioGroup,
    Switch,
    Slider,
    Tab,
    TabList,
    Menu,
    MenuItem,
    Dialog,
    Progress,
    Status,
    Separator,
    Tooltip,
    List,
    ListItem,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RangeValue {
    pub min: f64,
    pub max: f64,
    pub value: f64,
    pub step: f64,
}
impl RangeValue {
    pub fn is_valid(self) -> bool {
        [self.min, self.max, self.value, self.step]
            .into_iter()
            .all(f64::is_finite)
            && self.min < self.max
            && (self.max - self.min).is_finite()
            && self.step > 0.
            && self.value >= self.min
            && self.value <= self.max
    }
    pub fn snap(self, value: f64) -> f64 {
        let value = value.clamp(self.min, self.max);
        let steps = (value - self.min) / self.step;
        if steps.is_finite() {
            (self.min + steps.round() * self.step).clamp(self.min, self.max)
        } else {
            value
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub(crate) stable_id: Option<String>,
    pub(crate) role: Role,
    pub(crate) name: String,
    pub(crate) text: Option<String>,
    pub(crate) disabled: bool,
    pub(crate) checked: Option<bool>,
    #[serde(default)]
    selected: Option<bool>,
    #[serde(default)]
    mixed: bool,
    #[serde(default)]
    range: Option<RangeValue>,
    #[serde(default)]
    tab_index: Option<i32>,
    pub(crate) style: Style,
    parent: Option<NodeId>,
    first_child: Option<NodeId>,
    last_child: Option<NodeId>,
    next_sibling: Option<NodeId>,
}
impl Node {
    pub fn new(role: Role, name: impl Into<String>) -> Self {
        Self {
            stable_id: None,
            role,
            name: name.into(),
            text: None,
            disabled: false,
            checked: None,
            selected: None,
            mixed: false,
            range: None,
            tab_index: None,
            style: Style::default(),
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
        }
    }
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.stable_id = Some(id.into());
        self
    }
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
    pub fn styled(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
    pub fn checked(mut self, value: bool) -> Self {
        self.checked = Some(value);
        self
    }
    pub fn selected(mut self, value: bool) -> Self {
        self.selected = Some(value);
        self
    }
    pub fn mixed(mut self, value: bool) -> Self {
        self.mixed = value;
        self
    }
    pub fn range(mut self, value: RangeValue) -> Self {
        self.range = Some(value);
        self
    }
    pub fn tab_index(mut self, value: i32) -> Self {
        self.tab_index = Some(value);
        self
    }
    pub fn is_selected(&self) -> Option<bool> {
        self.selected
    }
    pub fn is_mixed(&self) -> bool {
        self.mixed
    }
    pub fn range_value(&self) -> Option<RangeValue> {
        self.range
    }
    pub fn is_tab_stop(&self) -> bool {
        self.is_focusable() && self.tab_index.is_none_or(|v| v >= 0)
    }
    pub fn stable_id(&self) -> Option<&str> {
        self.stable_id.as_deref()
    }
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn text_value(&self) -> Option<&str> {
        self.text.as_deref()
    }
    pub fn style(&self) -> &Style {
        &self.style
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    pub fn is_checked(&self) -> Option<bool> {
        self.checked
    }
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }
    pub fn is_focusable(&self) -> bool {
        !self.disabled
            && (self.tab_index.is_some()
                || matches!(
                    self.role,
                    Role::Button
                        | Role::Checkbox
                        | Role::TextInput
                        | Role::Radio
                        | Role::Switch
                        | Role::Slider
                        | Role::Tab
                        | Role::MenuItem
                ))
    }
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DomError {
    #[error("duplicate stable ID: {0}")]
    DuplicateId(String),
    #[error("invalid node index: {0}")]
    InvalidNode(usize),
    #[error("MicroDOM exceeds the u32 node limit")]
    Capacity,
}
#[derive(Clone, Debug)]
pub struct MicroDom {
    nodes: Vec<Node>,
    ids: BTreeMap<String, NodeId>,
    semantics: BTreeMap<(Role, String), Vec<NodeId>>,
}
impl MicroDom {
    pub fn new(root: Node) -> Self {
        let mut dom = Self {
            nodes: Vec::new(),
            ids: BTreeMap::new(),
            semantics: BTreeMap::new(),
        };
        dom.insert(root).expect("a root cannot have duplicate IDs");
        dom
    }
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }
    fn insert(&mut self, mut node: Node) -> Result<NodeId, DomError> {
        if let Some(id) = &node.stable_id {
            if self.ids.contains_key(id) {
                return Err(DomError::DuplicateId(id.clone()));
            }
        }
        let id = NodeId(u32::try_from(self.nodes.len()).map_err(|_| DomError::Capacity)?);
        // Node values may be copied from a different tree. Never copy its links.
        node.parent = None;
        node.first_child = None;
        node.last_child = None;
        node.next_sibling = None;
        if let Some(stable_id) = &node.stable_id {
            self.ids.insert(stable_id.clone(), id);
        }
        self.semantics
            .entry((node.role, node.name.clone()))
            .or_default()
            .push(id);
        self.nodes.push(node);
        Ok(id)
    }
    pub fn append(&mut self, parent: NodeId, node: Node) -> Result<NodeId, DomError> {
        let last = self.get(parent)?.last_child;
        let id = self.insert(node)?;
        self.nodes[id.index()].parent = Some(parent);
        if let Some(last) = last {
            self.nodes[last.index()].next_sibling = Some(id);
        } else {
            self.nodes[parent.index()].first_child = Some(id);
        }
        self.nodes[parent.index()].last_child = Some(id);
        Ok(id)
    }
    pub fn get(&self, id: NodeId) -> Result<&Node, DomError> {
        self.nodes
            .get(id.index())
            .ok_or(DomError::InvalidNode(id.index()))
    }
    pub fn by_id(&self, id: &str) -> Option<NodeId> {
        self.ids.get(id).copied()
    }
    pub fn by_role_name(&self, role: Role, name: &str) -> &[NodeId] {
        self.semantics
            .get(&(role, name.to_owned()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (NodeId(i as u32), node))
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn children(&self, parent: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let first = self.nodes.get(parent.index()).and_then(|n| n.first_child);
        std::iter::successors(first, |id| self.nodes[id.index()].next_sibling)
    }
    pub fn set_style(&mut self, id: NodeId, style: Style) -> Result<(), DomError> {
        let n = self
            .nodes
            .get_mut(id.index())
            .ok_or(DomError::InvalidNode(id.index()))?;
        n.style = style;
        Ok(())
    }
    pub fn set_text(&mut self, id: NodeId, text: impl Into<String>) -> Result<(), DomError> {
        let n = self
            .nodes
            .get_mut(id.index())
            .ok_or(DomError::InvalidNode(id.index()))?;
        n.text = Some(text.into());
        Ok(())
    }
    pub fn snapshot(&self) -> DomSnapshot {
        DomSnapshot {
            version: 1,
            nodes: self.nodes.clone(),
        }
    }
    /// Viewport layers paint after flow content, in z-index order.
    pub fn paint_order(&self) -> Vec<NodeId> {
        fn visit(dom: &MicroDom, id: NodeId, out: &mut Vec<NodeId>) {
            out.push(id);
            let mut children: Vec<_> = dom
                .children(id)
                .filter(|id| {
                    dom.get(*id).expect("valid node").style().position != crate::Position::Fixed
                })
                .collect();
            children.sort_by_key(|id| dom.get(*id).expect("valid node").style().z_index);
            for id in children {
                visit(dom, id, out);
            }
        }
        let mut out = Vec::with_capacity(self.len());
        visit(self, self.root(), &mut out);
        let mut layers: Vec<_> = self
            .iter()
            .filter(|(id, n)| *id != self.root() && n.style().position == crate::Position::Fixed)
            .map(|(id, _)| id)
            .collect();
        layers.sort_by_key(|id| self.get(*id).expect("valid node").style().z_index);
        for id in layers {
            visit(self, id, &mut out);
        }
        out
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomSnapshot {
    pub version: u32,
    pub nodes: Vec<Node>,
}
