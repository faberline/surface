//! Read-only, versioned capture of the current UI. Contains no event closures.
use crate::{Application, Runtime};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use surface::{ComputedDom, DomSnapshot};

/// A portable capture for debug tools, semantic tests and remote inspectors.
/// `layout` is absent before the first frame and after a layout invalidation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomExport {
    pub version: u32,
    pub dom: DomSnapshot,
    pub layout: Option<ComputedDom>,
    pub interaction: InteractionSnapshot,
    pub text_inputs: BTreeMap<String, EditingSnapshot>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionSnapshot {
    pub focused: Option<String>,
    pub hovered: Option<String>,
    pub pressed: Option<String>,
}
/// Selection offsets are UTF-8 byte positions on grapheme boundaries.
/// Preedit text is a display value, separate from committed application state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditingSnapshot {
    pub value: String,
    pub display_value: String,
    pub selection_anchor: usize,
    pub selection_cursor: usize,
    pub composing: bool,
    pub preedit_range: Option<std::ops::Range<usize>>,
}
impl<A: Application> Runtime<A> {
    /// Capture current semantic state without updating the app or forcing a render.
    pub fn snapshot(&self) -> DomExport {
        DomExport {
            version: 1,
            dom: self.dom.snapshot(),
            layout: self.computed.clone(),
            interaction: InteractionSnapshot {
                focused: self.focused.clone(),
                hovered: self.hovered.clone(),
                pressed: self.pressed.clone(),
            },
            text_inputs: self
                .editors
                .iter()
                .map(|(id, editor)| {
                    (
                        id.clone(),
                        EditingSnapshot {
                            value: editor.value().to_owned(),
                            display_value: editor.display_value(),
                            selection_anchor: editor.selection().anchor,
                            selection_cursor: editor.selection().cursor,
                            composing: editor.is_composing(),
                            preedit_range: editor.preedit_range(),
                        },
                    )
                })
                .collect(),
        }
    }
    /// Export the current DOM, optional layout and interaction state as pretty JSON.
    pub fn export_dom(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.snapshot())
    }
}
