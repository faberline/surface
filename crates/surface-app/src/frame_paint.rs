use crate::DisplayContrast;
use std::collections::BTreeMap;
use surface::{
    ComputedDom, DisplayList, ExtendedDisplayList, InteractionState, MicroDom, NodeId,
    TextDecoration, TextSelectionColors,
};

pub(crate) fn selection_colors(contrast: DisplayContrast) -> Option<TextSelectionColors> {
    let DisplayContrast::Forced(colors) = contrast else {
        return None;
    };
    Some(TextSelectionColors {
        background: colors.highlight.background,
        foreground: colors.highlight.foreground,
    })
}

/// Compile one completed runtime frame into the caller's display list format.
pub(crate) trait FramePaint {
    fn compile(
        dom: &MicroDom,
        layout: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
        selection: Option<TextSelectionColors>,
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
        _selection: Option<TextSelectionColors>,
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
        selection: Option<TextSelectionColors>,
    ) -> Self {
        match selection {
            Some(colors) => Self::compile_with_selection_colors(
                dom,
                layout,
                focused,
                decorations,
                interaction,
                colors,
            ),
            None => Self::compile_with_interaction(dom, layout, focused, decorations, interaction),
        }
    }

    #[cfg(feature = "devtools")]
    fn len(&self) -> usize {
        self.ops().len()
    }
}
