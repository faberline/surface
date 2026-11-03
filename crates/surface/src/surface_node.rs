//! The snapshot node types: the serializable wire format of one captured
//! element, text leaf or component.

use serde::{Deserialize, Serialize};

use crate::props::Props;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceNode {
    /// Stable structural path inside the rendered tree.
    pub node_id: String,
    /// Comparator/test alignment key. Uses `Props::id` when set,
    /// otherwise falls back to the structural path.
    pub semantic_id: String,
    pub parent_id: Option<String>,
    pub kind: SurfaceNodeKind,
    pub tag: Option<String>,
    pub component: Option<String>,
    pub role: Option<String>,
    pub name: Option<String>,
    pub text: Option<String>,
    pub props: SurfaceProps,
    pub bounds: Option<SurfaceRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceNodeKind {
    Element,
    Text,
    Component,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SurfaceProps {
    pub id: Option<String>,
    pub class_name: Option<String>,
    pub style: Option<String>,
    pub value: Option<String>,
    pub input_type: Option<String>,
    pub placeholder: Option<String>,
    pub checked: Option<bool>,
    pub aria_label: Option<String>,
    pub html_for: Option<String>,
    pub disabled: bool,
    pub has_on_click: bool,
    pub has_on_change: bool,
    pub has_on_checked_change: bool,
}

impl From<&Props> for SurfaceProps {
    fn from(props: &Props) -> Self {
        Self {
            id: props.id().map(str::to_string),
            class_name: props.class_name().map(str::to_string),
            style: props.style().map(str::to_string),
            value: props.value().map(str::to_string),
            input_type: props.input_type().map(str::to_string),
            placeholder: props.placeholder().map(str::to_string),
            checked: props.checked(),
            aria_label: props.aria_label().map(str::to_string),
            html_for: props.html_for().map(str::to_string),
            disabled: props.disabled(),
            has_on_click: props.on_click().is_some(),
            has_on_change: props.on_change().is_some(),
            has_on_checked_change: props.on_checked_change().is_some(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SurfaceRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
