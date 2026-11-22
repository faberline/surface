//! Ordered, serializable drawing commands shared by every GPU backend.
use crate::shadow::has_area;
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
    fn from_corners(corners: Corners) -> Self {
        corners
            .uniform()
            .map_or(Self::Corners(corners), Self::Radius)
    }

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
/// Ordered drawing commands that retain vector fills.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExtendedPaintOp {
    Legacy(PaintOp),
    FillPath(crate::PathFill),
}

trait CompileOp: From<PaintOp> {
    fn push_fill(path: crate::PathFill, push: impl FnMut(Self));
}

impl CompileOp for PaintOp {
    fn push_fill(path: crate::PathFill, push: impl FnMut(Self)) {
        path.legacy_ops(push);
    }
}

impl From<PaintOp> for ExtendedPaintOp {
    fn from(op: PaintOp) -> Self {
        Self::Legacy(op)
    }
}

impl CompileOp for ExtendedPaintOp {
    fn push_fill(path: crate::PathFill, mut push: impl FnMut(Self)) {
        push(Self::FillPath(path));
    }
}

/// The ops of one compile and, for an attributed compile, their owners.
struct Ops<'a, Op> {
    ops: Vec<Op>,
    owners: Option<&'a mut Vec<NodeId>>,
}
impl<Op: CompileOp> Ops<'_, Op> {
    fn push(&mut self, owner: NodeId, op: PaintOp) {
        self.push_owned(owner, op.into());
    }
    fn push_fill(&mut self, owner: NodeId, path: crate::PathFill) {
        Op::push_fill(path, |op| self.push_owned(owner, op));
    }
    fn push_owned(&mut self, owner: NodeId, op: Op) {
        self.ops.push(op);
        if let Some(owners) = self.owners.as_deref_mut() {
            owners.push(owner);
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
        Self {
            viewport: computed.viewport,
            ops: compile_into(dom, computed, focused, decorations, interaction, None),
        }
    }
    /// The list `compile_with_interaction` returns, plus the node that
    /// produced each op: `owners.len() == ops.len()` and `owners[i]` owns
    /// `ops[i]`. For devtools and tests; the other `compile*` functions do
    /// not build the owner list.
    pub fn compile_attributed(
        dom: &MicroDom,
        computed: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
    ) -> (Self, Vec<NodeId>) {
        let mut owners = Vec::new();
        let list = Self {
            viewport: computed.viewport,
            ops: compile_into(
                dom,
                computed,
                focused,
                decorations,
                interaction,
                Some(&mut owners),
            ),
        };
        (list, owners)
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

/// A display list that retains filled vector contours.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtendedDisplayList {
    viewport: Rect,
    ops: Vec<ExtendedPaintOp>,
}

impl ExtendedDisplayList {
    pub fn new(viewport: Rect, ops: Vec<ExtendedPaintOp>) -> Self {
        Self { viewport, ops }
    }

    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    pub fn ops(&self) -> &[ExtendedPaintOp] {
        &self.ops
    }

    /// Append legacy operations without changing their paint order.
    pub fn extend_legacy(&mut self, ops: impl IntoIterator<Item = PaintOp>) {
        self.ops
            .extend(ops.into_iter().map(ExtendedPaintOp::Legacy));
    }

    pub fn compile_with_interaction(
        dom: &MicroDom,
        computed: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
    ) -> Self {
        Self::new(
            computed.viewport,
            compile_into(dom, computed, focused, decorations, interaction, None),
        )
    }

    pub fn compile_attributed(
        dom: &MicroDom,
        computed: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
    ) -> (Self, Vec<NodeId>) {
        let mut owners = Vec::new();
        let ops = compile_into(
            dom,
            computed,
            focused,
            decorations,
            interaction,
            Some(&mut owners),
        );
        (Self::new(computed.viewport, ops), owners)
    }

    pub fn into_ops(self) -> Vec<ExtendedPaintOp> {
        self.ops
    }
}

/// The one compile loop. `owners`, when present, gets the owner of each
/// op pushed.
fn compile_into<Op: CompileOp>(
    dom: &MicroDom,
    computed: &ComputedDom,
    focused: Option<NodeId>,
    decorations: &BTreeMap<NodeId, TextDecoration>,
    interaction: InteractionState,
    owners: Option<&mut Vec<NodeId>>,
) -> Vec<Op> {
    let mut ops = Ops {
        ops: Vec::new(),
        owners,
    };
    for id in &computed.paint_order {
        let n = dom.get(*id).expect("computed node belongs to DOM");
        let g = &computed.nodes[id.index()];
        let s = n.style();
        let visible = has_area(g.bounds.intersect(g.clip));
        for shadow in n.presented_box_shadow_layers(*id, focused, interaction) {
            shadow.contours(
                g.bounds,
                s.corner_radii(),
                g.clip,
                |rect, corners, color, stroke| {
                    let outline = Outline::from_corners(corners);
                    let op = match stroke {
                        Some(width) => outline.stroke(rect, g.clip, width, color),
                        None => outline.fill(rect, g.clip, color),
                    };
                    ops.push(*id, op);
                },
            );
        }
        let outline = Outline::of(s, g.bounds);
        if visible && s.background.3 > 0 {
            ops.push(*id, outline.fill(g.bounds, g.clip, s.background));
        }
        if let Some(path) = n
            .fill_contour
            .as_ref()
            .and_then(|fill| fill.resolve(g.bounds, g.clip))
        {
            ops.push_fill(*id, path);
        }
        if !visible {
            continue;
        }
        if s.border_width > 0. && s.border_color.3 > 0 {
            ops.push(
                *id,
                outline.stroke(g.bounds, g.clip, s.border_width, s.border_color),
            );
        }
        if s.state_layer.3 > 0 && !n.is_disabled() {
            let [hover_alpha, focus_alpha, press_alpha] = n.resolved_state_layer_alphas();
            let mut target = Some(*id);
            let mut opacity = 0;
            while let Some(id) = target {
                if interaction.pressed == Some(id) {
                    opacity = press_alpha;
                    break;
                }
                if focus_alpha > 0 && focused == Some(id) {
                    opacity = focus_alpha;
                } else if opacity == 0 && interaction.hovered == Some(id) {
                    opacity = hover_alpha;
                }
                target = dom.get(id).ok().and_then(|n| n.parent());
            }
            if opacity > 0 {
                let c = s.state_layer;
                ops.push(
                    *id,
                    outline.fill(
                        g.bounds,
                        g.clip,
                        Color(
                            c.0,
                            c.1,
                            c.2,
                            (u16::from(c.3) * u16::from(opacity) / 255) as u8,
                        ),
                    ),
                );
            }
        }
        let focus_color = n
            .focus_outline_color()
            .filter(|_| n.paints_focus_outline(*id, focused));
        if let Some(color) = focus_color {
            if s.corners.is_none() {
                // Legacy nodes keep the four square edge quads.
                let r = g.bounds;
                for line in [
                    Rect::new(r.x, r.y, r.width, 2.),
                    Rect::new(r.x, r.y + r.height - 2., r.width, 2.),
                    Rect::new(r.x, r.y, 2., r.height),
                    Rect::new(r.x + r.width - 2., r.y, 2., r.height),
                ] {
                    ops.push(
                        *id,
                        PaintOp::Quad {
                            rect: line,
                            clip: g.clip,
                            radius: 0.,
                            color,
                        },
                    );
                }
            } else {
                // One inside stroke that follows the node shape.
                ops.push(*id, outline.stroke(g.bounds, g.clip, 2., color));
            }
        }
        if let Some(text) = n.text_value() {
            let rect = g.bounds.inset(s.padding);
            let clip = g.clip.intersect(rect);
            let decoration = decorations.get(id);
            if let Some(d) = decoration {
                for r in &d.selection {
                    ops.push(
                        *id,
                        PaintOp::Quad {
                            rect: *r,
                            clip,
                            radius: 0.,
                            color: Color(103, 80, 164, 80),
                        },
                    );
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
                ops.push(
                    *id,
                    PaintOp::StyledText {
                        rect,
                        clip,
                        text: text.to_owned(),
                        style: text_style,
                        color: s.foreground,
                        scroll,
                        multiline,
                        editable,
                    },
                );
            } else if editable {
                ops.push(
                    *id,
                    PaintOp::EditableText {
                        rect,
                        clip,
                        text: text.to_owned(),
                        font_size: s.font_size,
                        color: s.foreground,
                        scroll: decoration.map(|d| d.scroll).unwrap_or_default(),
                        multiline: decoration.is_some_and(|d| d.multiline),
                    },
                );
            } else {
                ops.push(
                    *id,
                    PaintOp::Text {
                        rect,
                        clip,
                        text: text.to_owned(),
                        font_size: s.font_size,
                        color: s.foreground,
                    },
                );
            }
            if let Some(d) = decoration {
                for r in &d.underline {
                    ops.push(
                        *id,
                        PaintOp::Quad {
                            rect: *r,
                            clip,
                            radius: 0.,
                            color: s.foreground,
                        },
                    );
                }
                if let Some(caret) = d.caret {
                    ops.push(
                        *id,
                        PaintOp::Quad {
                            rect: caret,
                            clip,
                            radius: 0.,
                            color: s.foreground,
                        },
                    );
                }
            }
        }
    }
    ops.ops
}
