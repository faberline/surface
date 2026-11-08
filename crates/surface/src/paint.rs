//! Ordered, serializable drawing commands shared by every GPU backend.
use crate::{Color, ComputedDom, Corners, MicroDom, NodeId, Rect, Role, Style, TextStyle};
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
    /// A filled rectangle with a radius per corner.
    Shape {
        rect: Rect,
        clip: Rect,
        corners: Corners,
        color: Color,
    },
    /// An inside stroke that follows a per-corner shape.
    ShapeStroke {
        rect: Rect,
        clip: Rect,
        corners: Corners,
        width: f32,
        color: Color,
    },
    /// Text whose style is more than a size. Size-only text keeps the
    /// `Text` and `EditableText` forms above.
    StyledText {
        rect: Rect,
        clip: Rect,
        text: String,
        style: TextStyle,
        color: Color,
        scroll: (f32, f32),
        multiline: bool,
        editable: bool,
    },
}
impl PaintOp {
    pub fn rect(&self) -> Rect {
        match self {
            Self::Quad { rect, .. }
            | Self::Stroke { rect, .. }
            | Self::Text { rect, .. }
            | Self::EditableText { rect, .. }
            | Self::Shape { rect, .. }
            | Self::ShapeStroke { rect, .. }
            | Self::StyledText { rect, .. } => *rect,
        }
    }

    pub fn clip(&self) -> Rect {
        match self {
            Self::Quad { clip, .. }
            | Self::Stroke { clip, .. }
            | Self::Text { clip, .. }
            | Self::EditableText { clip, .. }
            | Self::Shape { clip, .. }
            | Self::ShapeStroke { clip, .. }
            | Self::StyledText { clip, .. } => *clip,
        }
    }
}

/// The outline of one node. A uniform shape keeps the plain radius ops.
#[derive(Clone, Copy)]
enum Outline {
    Radius(f32),
    Corners(Corners),
}
impl Outline {
    fn of(style: &Style, bounds: Rect) -> Self {
        match style.corners {
            // Legacy nodes are not resolved so their output never changes.
            None => Self::Radius(style.radius),
            Some(corners) => {
                let corners = corners.resolve(bounds.width, bounds.height);
                corners
                    .uniform()
                    .map_or(Self::Corners(corners), Self::Radius)
            }
        }
    }
    fn fill(self, rect: Rect, clip: Rect, color: Color) -> PaintOp {
        match self {
            Self::Radius(radius) => PaintOp::Quad {
                rect,
                clip,
                radius,
                color,
            },
            Self::Corners(corners) => PaintOp::Shape {
                rect,
                clip,
                corners,
                color,
            },
        }
    }
    fn stroke(self, rect: Rect, clip: Rect, width: f32, color: Color) -> PaintOp {
        match self {
            Self::Radius(radius) => PaintOp::Stroke {
                rect,
                clip,
                radius,
                width,
                color,
            },
            Self::Corners(corners) => PaintOp::ShapeStroke {
                rect,
                clip,
                corners,
                width,
                color,
            },
        }
    }
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
            let outline = Outline::of(s, g.bounds);
            if s.background.3 > 0 {
                ops.push(outline.fill(g.bounds, g.clip, s.background));
            }
            if s.border_width > 0. && s.border_color.3 > 0 {
                ops.push(outline.stroke(g.bounds, g.clip, s.border_width, s.border_color));
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
                    ops.push(outline.fill(
                        g.bounds,
                        g.clip,
                        Color(c.0, c.1, c.2, (c.3 as u16 * opacity / 255) as u8),
                    ));
                }
            }
            if focused == Some(*id) {
                if s.corners.is_none() {
                    // Legacy nodes keep the four square edge quads.
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
                } else {
                    // One inside stroke that follows the node shape.
                    ops.push(outline.stroke(g.bounds, g.clip, 2., s.foreground));
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
                let editable = n.role() == Role::TextInput;
                let text_style = s.text_style();
                if !text_style.is_legacy() {
                    let (scroll, multiline) = if editable {
                        (
                            decoration.map(|d| d.scroll).unwrap_or_default(),
                            decoration.is_some_and(|d| d.multiline),
                        )
                    } else {
                        ((0., 0.), true)
                    };
                    ops.push(PaintOp::StyledText {
                        rect,
                        clip,
                        text: text.to_owned(),
                        style: text_style,
                        color: s.foreground,
                        scroll,
                        multiline,
                        editable,
                    });
                } else if editable {
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

    /// The visible rectangles of the ops that differ from `previous`.
    pub fn damage(&self, previous: &DisplayList) -> Vec<Rect> {
        if self.viewport != previous.viewport {
            return vec![self.viewport];
        }
        let mut damage: Vec<Rect> = Vec::new();
        let n = self.ops.len().max(previous.ops.len());
        for i in 0..n {
            let a = self.ops.get(i);
            let b = previous.ops.get(i);
            if a == b {
                continue;
            }
            for op in [a, b].into_iter().flatten() {
                let visible = op.rect().intersect(op.clip());
                if visible.width > 0. && visible.height > 0. && !damage.contains(&visible) {
                    damage.push(visible);
                }
            }
        }
        damage
    }
}
