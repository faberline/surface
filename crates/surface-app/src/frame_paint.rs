use crate::DisplayContrast;
use std::{collections::BTreeMap, ops::Range};
use surface::{
    ComputedDom, DisplayList, ExtendedDisplayList, InteractionState, MicroDom, NodeId,
    PaintGroupEffect, PaintTransform, Rect, TextDecoration, TextSelectionColors,
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

pub(crate) fn modal_panel_effect(
    viewport: Rect,
    opacity: Option<f32>,
    transform: Option<PaintTransform>,
) -> Option<PaintGroupEffect> {
    let mut opacity = opacity
        .filter(|value| value.is_finite())
        .unwrap_or(1.)
        .clamp(0., 1.);
    if let Some(mut transform) = transform.filter(|value| value.is_valid()) {
        opacity *= transform.opacity;
        let neutral_geometry = transform.source == transform.destination
            && transform.radius == 0.
            && transform.source.intersect(viewport) == viewport
            && transform.clip.intersect(viewport) == viewport;
        if !neutral_geometry {
            transform.opacity = opacity;
            return Some(PaintGroupEffect::Transform(transform));
        }
    }
    (opacity < 1.).then_some(PaintGroupEffect::Opacity(opacity))
}

/// Compile one completed runtime frame into the caller's display list format.
pub(crate) trait FramePaint {
    const PANEL_EFFECTS: bool = false;

    fn compile(
        dom: &MicroDom,
        layout: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
        selection: Option<TextSelectionColors>,
        panel_effects: &[(Range<usize>, PaintGroupEffect)],
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
        _panel_effects: &[(Range<usize>, PaintGroupEffect)],
    ) -> Self {
        Self::compile_with_interaction(dom, layout, focused, decorations, interaction)
    }

    #[cfg(feature = "devtools")]
    fn len(&self) -> usize {
        self.ops.len()
    }
}

impl FramePaint for ExtendedDisplayList {
    const PANEL_EFFECTS: bool = true;

    fn compile(
        dom: &MicroDom,
        layout: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
        selection: Option<TextSelectionColors>,
        panel_effects: &[(Range<usize>, PaintGroupEffect)],
    ) -> Self {
        if !panel_effects.is_empty() {
            return Self::compile_with_paint_groups(
                dom,
                layout,
                focused,
                decorations,
                interaction,
                selection,
                panel_effects,
            );
        }
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
