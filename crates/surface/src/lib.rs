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

mod fill_contour;
mod geometry;
mod layout;
mod microdom;
mod motion;
mod paint;
mod path_geometry;
mod scroll_extent;
mod shadow;
mod shape;
mod style;
mod text_layout;
mod typography;
pub use fill_contour::{FillContour, PathFill};
pub use geometry::Rect;
pub use layout::{
    ComputedDom, ComputedNode, LayoutEngine, LayoutError, MeasuredGroupLayout, TextMeasurer,
};
pub use microdom::{
    DomError, DomSnapshot, GridRole, MicroDom, Node, NodeId, OverlayBoundary,
    OverlayHorizontalEdge, RangeValue, Role,
};
pub use motion::{Easing, Transition};
pub use paint::{
    DisplayList, ExtendedDisplayList, ExtendedPaintOp, InteractionState, PaintOp, TextDecoration,
};
pub use scroll_extent::scroll_extent;
pub use shadow::BoxShadow;
pub use shape::Corners;
pub use style::{Align, Color, Direction, Insets, Length, Position, Style};
pub use text_layout::{TextCluster, TextLayout};
pub use typography::{FontFamily, TextScaling, TextStyle};
