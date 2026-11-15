//! Paint attribution export, schema `surface.paint.v1` (feature `devtools`).
//! Each op of the last frame with the node that produced it.
use crate::{Application, Runtime};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use surface::{DisplayList, InteractionState, MicroDom, NodeId, PaintOp, Rect, TextDecoration};

/// The schema name in every export.
pub const PAINT_SCHEMA: &str = "surface.paint.v1";

/// The ops of the last frame, in paint order, each with its owner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaintExport {
    pub schema: String,
    pub viewport: Rect,
    pub ops: Vec<AttributedOp>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttributedOp {
    pub owner: PaintOwner,
    pub op: PaintOp,
}
/// `node` is the node index, the same index as in `DomExport`.
/// `id` is its stable id, or `None` for a node without one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaintOwner {
    pub node: u32,
    pub id: Option<String>,
}

/// The compile inputs of the last frame. `render` moves them here instead
/// of dropping them, so keeping a frame costs no allocation; the export
/// compiles them again with owners. Compile is pure, so the result is the
/// list `render` returned.
pub(crate) struct PaintFrame {
    pub(crate) dom: MicroDom,
    pub(crate) decorations: BTreeMap<NodeId, TextDecoration>,
    pub(crate) focused: Option<NodeId>,
    pub(crate) interaction: InteractionState,
}

impl<A: Application> Runtime<A> {
    /// The typed `surface.paint.v1` export of the last frame. `None` before
    /// the first frame and after a state, editing or scroll change, until
    /// the next `render`, like `DomExport::layout`.
    pub fn paint_snapshot(&self) -> Option<PaintExport> {
        let frame = self.paint_frame.as_ref()?;
        let computed = self.computed.as_ref()?;
        let (list, owners) = DisplayList::compile_attributed(
            &frame.dom,
            computed,
            frame.focused,
            &frame.decorations,
            frame.interaction,
        );
        let ops = list
            .ops
            .into_iter()
            .zip(owners)
            .map(|(op, owner)| AttributedOp {
                owner: PaintOwner {
                    node: owner.index() as u32,
                    id: frame
                        .dom
                        .get(owner)
                        .ok()
                        .and_then(|n| n.stable_id())
                        .map(str::to_owned),
                },
                op,
            })
            .collect();
        Some(PaintExport {
            schema: PAINT_SCHEMA.to_owned(),
            viewport: list.viewport,
            ops,
        })
    }
    /// `paint_snapshot` as pretty JSON. Without a current frame it returns
    /// an error value; it never panics.
    pub fn export_paint(&self) -> Result<String, serde_json::Error> {
        match self.paint_snapshot() {
            Some(paint) => serde_json::to_string_pretty(&paint),
            None => Err(serde::ser::Error::custom(
                "no current frame: call render before export_paint",
            )),
        }
    }
}
