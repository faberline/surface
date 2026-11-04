//! Ordered, serializable drawing commands shared by every GPU backend.
use crate::{Color, ComputedDom, MicroDom, NodeId, Rect, Role};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default)]
pub struct TextDecoration {
    pub selection: Vec<Rect>,
    pub underline: Vec<Rect>,
    pub caret: Option<Rect>,
    pub scroll: (f32, f32),
    pub multiline: bool,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct InteractionState {
    pub hovered: Option<NodeId>,
    pub pressed: Option<NodeId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PaintOp {
    Quad {
        rect: Rect,
        clip: Rect,
        radius: f32,
        color: Color,
    },
    Stroke {
        rect: Rect,
        clip: Rect,
        radius: f32,
        width: f32,
        color: Color,
    },
    Text {
        rect: Rect,
        clip: Rect,
        text: String,
        font_size: f32,
        color: Color,
    },
    EditableText {
        rect: Rect,
        clip: Rect,
        text: String,
        font_size: f32,
        color: Color,
        scroll: (f32, f32),
        multiline: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DisplayList {
    pub viewport: Rect,
    pub ops: Vec<PaintOp>,
}
impl DisplayList {
    pub fn compile(dom: &MicroDom, computed: &ComputedDom, focused: Option<NodeId>) -> Self {
        Self::compile_with_text(dom, computed, focused, &BTreeMap::new())
    }
    pub fn compile_with_text(
        dom: &MicroDom,
        computed: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
    ) -> Self {
        Self::compile_with_interaction(
            dom,
            computed,
            focused,
            decorations,
            InteractionState::default(),
        )
    }
    pub fn compile_with_interaction(
        dom: &MicroDom,
        computed: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
    ) -> Self {
        let mut ops = Vec::new();
        for id in &computed.paint_order {
            let n = dom.get(*id).expect("computed node belongs to DOM");
            let g = &computed.nodes[id.index()];
            let s = n.style();
            if g.bounds.intersect(g.clip).width <= 0. || g.bounds.intersect(g.clip).height <= 0. {
                continue;
            }
            if s.background.3 > 0 {
                ops.push(PaintOp::Quad {
                    rect: g.bounds,
                    clip: g.clip,
                    radius: s.radius,
                    color: s.background,
                });
            }
            if s.border_width > 0. && s.border_color.3 > 0 {
                ops.push(PaintOp::Stroke {
                    rect: g.bounds,
                    clip: g.clip,
                    radius: s.radius,
                    width: s.border_width,
                    color: s.border_color,
                });
            }
            if s.state_layer.3 > 0 && !n.is_disabled() {
                let mut target = Some(*id);
                let mut opacity = 0;
                while let Some(id) = target {
                    if interaction.pressed == Some(id) {
                        opacity = 31;
                        break;
                    }
                    if interaction.hovered == Some(id) {
                        opacity = 20;
                    }
                    target = dom.get(id).ok().and_then(|n| n.parent());
                }
                if opacity > 0 {
                    let c = s.state_layer;
                    ops.push(PaintOp::Quad {
                        rect: g.bounds,
                        clip: g.clip,
                        radius: s.radius,
                        color: Color(c.0, c.1, c.2, (c.3 as u16 * opacity / 255) as u8),
                    });
                }
            }
            if focused == Some(*id) {
                let r = g.bounds;
                for line in [
                    Rect::new(r.x, r.y, r.width, 2.),
                    Rect::new(r.x, r.y + r.height - 2., r.width, 2.),
                    Rect::new(r.x, r.y, 2., r.height),
                    Rect::new(r.x + r.width - 2., r.y, 2., r.height),
                ] {
                    ops.push(PaintOp::Quad {
                        rect: line,
                        clip: g.clip,
                        radius: 0.,
                        color: s.foreground,
                    });
                }
            }
            if let Some(text) = n.text_value() {
                let rect = g.bounds.inset(s.padding);
                let clip = g.clip.intersect(rect);
                let decoration = decorations.get(id);
                if let Some(d) = decoration {
                    for r in &d.selection {
                        ops.push(PaintOp::Quad {
                            rect: *r,
                            clip,
                            radius: 0.,
                            color: Color(103, 80, 164, 80),
                        });
                    }
                }
                if n.role() == Role::TextInput {
                    ops.push(PaintOp::EditableText {
                        rect,
                        clip,
                        text: text.to_owned(),
                        font_size: s.font_size,
                        color: s.foreground,
                        scroll: decoration.map(|d| d.scroll).unwrap_or_default(),
                        multiline: decoration.is_some_and(|d| d.multiline),
                    });
                } else {
                    ops.push(PaintOp::Text {
                        rect,
                        clip,
                        text: text.to_owned(),
                        font_size: s.font_size,
                        color: s.foreground,
                    });
                }
                if let Some(d) = decoration {
                    for r in &d.underline {
                        ops.push(PaintOp::Quad {
                            rect: *r,
                            clip,
                            radius: 0.,
                            color: s.foreground,
                        });
                    }
                    if let Some(caret) = d.caret {
                        ops.push(PaintOp::Quad {
                            rect: caret,
                            clip,
                            radius: 0.,
                            color: s.foreground,
                        });
                    }
                }
            }
        }
        Self {
            viewport: computed.viewport,
            ops,
        }
    }
}
