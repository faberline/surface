//! The live element tree that framework runtimes render and renderers paint.

use crate::callback::Callback;
use crate::component::Component;
use crate::props::Props;
use crate::snapshot::SurfaceSnapshot;

/// A rendered element tree shared by framework runtimes.
#[derive(Clone)]
// Preserve Core/Jet's public struct-variant API during consumer migration.
// New applications use the compact indexed MicroDom instead.
#[allow(clippy::large_enum_variant)]
pub enum Element {
    /// An intrinsic renderer-neutral node: kind/tag + props + children.
    Intrinsic {
        tag: &'static str,
        props: Props,
        children: Vec<Element>,
    },
    /// A text leaf.
    Text(String),
    /// A component invocation; framework runtimes expand this before rendering.
    Component(Component),
    /// An empty/noop element. Used for conditional null/undefined in JSX.
    Empty,
    /// Transparent container for a dynamic list of elements.
    Fragment(Vec<Element>),
}

impl Element {
    pub fn intrinsic(tag: &'static str, props: Props, children: Vec<Element>) -> Self {
        Self::Intrinsic {
            tag,
            props,
            children,
        }
    }

    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into())
    }

    pub fn from_number<N: std::fmt::Display>(n: N) -> Self {
        Self::Text(n.to_string())
    }

    /// Depth-first collect an `on_click` callback by semantic id.
    pub fn find_on_click(&self, target_id: &str) -> Option<Callback<()>> {
        match self {
            Element::Intrinsic {
                props, children, ..
            } => {
                if props.id() == Some(target_id) {
                    return props.on_click().cloned();
                }
                children.iter().find_map(|c| c.find_on_click(target_id))
            }
            Element::Fragment(children) => children.iter().find_map(|c| c.find_on_click(target_id)),
            Element::Component(_) | Element::Text(_) | Element::Empty => None,
        }
    }

    /// Concatenate all text descendants in order.
    pub fn text_content(&self) -> String {
        match self {
            Element::Text(s) => s.clone(),
            Element::Intrinsic { children, .. } | Element::Fragment(children) => {
                children.iter().map(Element::text_content).collect()
            }
            Element::Component(_) | Element::Empty => String::new(),
        }
    }

    /// Capture a deterministic semantic surface snapshot.
    pub fn surface_snapshot(&self) -> SurfaceSnapshot {
        SurfaceSnapshot::from_element(self)
    }
}
