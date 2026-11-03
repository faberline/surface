//! Renderer-neutral UI surface primitives.
//!
//! `surface` is deliberately below any renderer or framework runtime. It
//! owns the UI element tree shape that Jet WASM TSX/Vue/etc. adapters produce,
//! plus a serializable snapshot form that native desktop readers, renderers, tests,
//! and parity comparators can inspect without a browser or toolkit-private tree.

mod callback;
mod component;
mod element;
mod props;
mod snapshot;
mod surface_node;

pub use callback::Callback;
pub use component::{Component, ComponentFn};
pub use element::Element;
pub use props::Props;
pub use snapshot::SurfaceSnapshot;
pub use surface_node::{SurfaceNode, SurfaceNodeKind, SurfaceProps, SurfaceRect};

mod geometry;
mod layout;
mod microdom;
mod paint;
mod style;
pub use geometry::Rect;
pub use layout::{ComputedDom, ComputedNode, LayoutEngine, LayoutError, TextMeasurer};
pub use microdom::{DomError, DomSnapshot, MicroDom, Node, NodeId, Role};
pub use paint::{DisplayList, PaintOp};
pub use style::{Align, Color, Direction, Length, Style};
