use std::collections::BTreeMap;
use surface::{
    ComputedDom, DisplayList, ExtendedDisplayList, InteractionState, MicroDom, NodeId,
    TextDecoration,
};

/// Compile one completed runtime frame into the caller's display list format.
pub(crate) trait FramePaint {
    fn compile(
        dom: &MicroDom,
        layout: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
    ) -> Self;

    #[cfg(feature = "devtools")]
    fn len(&self) -> usize;
}

impl FramePaint for DisplayList {
    fn compile(
        dom: &MicroDom,
        layout: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
    ) -> Self {
        Self::compile_with_interaction(dom, layout, focused, decorations, interaction)
    }

    #[cfg(feature = "devtools")]
    fn len(&self) -> usize {
        self.ops.len()
    }
}

impl FramePaint for ExtendedDisplayList {
    fn compile(
        dom: &MicroDom,
        layout: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
    ) -> Self {
        Self::compile_with_interaction(dom, layout, focused, decorations, interaction)
    }

    #[cfg(feature = "devtools")]
    fn len(&self) -> usize {
        self.ops().len()
    }
}
