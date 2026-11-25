//! Ordered, serializable drawing commands shared by every GPU backend.
mod press_ripple;
mod text_selection;

pub use text_selection::{TextSelectionColors, TextSelectionPaint, TextSelectionRegion};

use crate::shadow::has_area;
use crate::{Color, ComputedDom, Corners, MicroDom, NodeId, PathFill, Rect, Role, TextStyle};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Range};
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

    fn resolve(radius: f32, corners: Option<Corners>, bounds: Rect) -> Self {
        // Preserve the legacy radius in the paint command.
        corners.map_or(Self::Radius(radius), |corners| {
            Self::from_corners(corners.resolve(bounds.width, bounds.height))
        })
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
/// Physical horizontal alignment of each visual line in a static text run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// Static text with an explicit alignment. The original run retains its
/// bounds, clip, wrapping and style.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AlignedTextPaint {
    run: PaintOp,
    alignment: TextAlignment,
}

impl AlignedTextPaint {
    pub fn run(&self) -> &PaintOp {
        &self.run
    }

    pub fn alignment(&self) -> TextAlignment {
        self.alignment
    }
}

/// Map an isolated paint group from a source rectangle to a destination.
///
/// All geometry uses logical pixels, in the same coordinates as the display
/// list. The source crops the original group paint. The destination is masked
/// to the independent rounded `clip`. The radius is capped at half the clip's
/// shorter side. Opacity applies once to the whole transformed group.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaintTransform {
    pub source: Rect,
    pub destination: Rect,
    pub clip: Rect,
    pub radius: f32,
    pub opacity: f32,
}

impl PaintTransform {
    /// All rectangles must have finite edges and positive area. Opacity must
    /// be in `0..=1` and radius must be finite and nonnegative.
    pub fn is_valid(self) -> bool {
        let valid_rect = |rect: Rect| {
            let far_x = rect.x + rect.width;
            let far_y = rect.y + rect.height;
            rect.x.is_finite()
                && rect.y.is_finite()
                && far_x.is_finite()
                && far_y.is_finite()
                && far_x > rect.x
                && far_y > rect.y
        };
        valid_rect(self.source)
            && valid_rect(self.destination)
            && valid_rect(self.clip)
            && self.radius.is_finite()
            && self.radius >= 0.
            && (0. ..=1.).contains(&self.opacity)
    }
}

/// The effect applied to one isolated range in the final paint order.
#[derive(Clone, Copy, Debug, PartialEq)]
#[doc(hidden)]
pub enum PaintGroupEffect {
    Opacity(f32),
    Transform(PaintTransform),
}

impl PaintGroupEffect {
    fn is_valid(self) -> bool {
        match self {
            Self::Opacity(opacity) => (0. ..=1.).contains(&opacity),
            Self::Transform(transform) => transform.is_valid(),
        }
    }
}

/// Ordered drawing commands that retain vector fills and live raster images.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExtendedPaintOp {
    Legacy(PaintOp),
    FillPath(crate::PathFill),
    AlignedText(AlignedTextPaint),
    /// The two-color paint for one editable text selection.
    TextSelection(TextSelectionPaint),
    /// A live resource. Use a screenshot to export its rendered pixels.
    /// Serializing an extended list that contains this operation returns an error.
    #[serde(skip)]
    Raster(crate::RasterPaint),
    /// Start an isolated group. Its complete paint is composited once with
    /// an opacity between zero and one. End it with `EndOpacityGroup`.
    /// Group ranges cannot nest.
    BeginOpacityGroup {
        opacity: f32,
    },
    EndOpacityGroup,
    /// Start a transformed isolated group. End it with `EndOpacityGroup`.
    /// Group ranges cannot nest.
    BeginTransformGroup {
        transform: PaintTransform,
    },
}

trait CompileOp: From<PaintOp> {
    const PRESS_RIPPLE: bool = false;

    fn push_fill(path: crate::PathFill, push: impl FnMut(Self));

    fn with_aligned_text(run: PaintOp, _alignment: TextAlignment) -> Self {
        run.into()
    }

    fn with_text_selection(
        run: PaintOp,
        _colors: TextSelectionColors,
        _selected: Vec<Rect>,
    ) -> Self {
        run.into()
    }

    fn push_raster(
        _image: &crate::RasterImage,
        _rect: Rect,
        _clip: Rect,
        _corners: Corners,
        _push: impl FnMut(Self),
    ) {
    }

    fn begin_paint_group(_effect: PaintGroupEffect) -> Option<Self> {
        None
    }

    fn end_paint_group() -> Option<Self> {
        None
    }
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
    const PRESS_RIPPLE: bool = true;

    fn push_fill(path: crate::PathFill, mut push: impl FnMut(Self)) {
        push(Self::FillPath(path));
    }

    fn with_aligned_text(run: PaintOp, alignment: TextAlignment) -> Self {
        Self::AlignedText(AlignedTextPaint { run, alignment })
    }

    fn with_text_selection(run: PaintOp, colors: TextSelectionColors, selected: Vec<Rect>) -> Self {
        Self::TextSelection(TextSelectionPaint::new(run, colors, selected))
    }

    fn push_raster(
        image: &crate::RasterImage,
        rect: Rect,
        clip: Rect,
        corners: Corners,
        mut push: impl FnMut(Self),
    ) {
        if let Some(paint) = crate::RasterPaint::new(image.clone(), rect, clip, corners) {
            push(Self::Raster(paint));
        }
    }

    fn begin_paint_group(effect: PaintGroupEffect) -> Option<Self> {
        Some(match effect {
            PaintGroupEffect::Opacity(opacity) => Self::BeginOpacityGroup { opacity },
            PaintGroupEffect::Transform(transform) => Self::BeginTransformGroup { transform },
        })
    }

    fn end_paint_group() -> Option<Self> {
        Some(Self::EndOpacityGroup)
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

struct FocusPaint {
    rect: Rect,
    clip: Rect,
    color: Color,
    stroke: Option<(Outline, f32)>,
}

impl FocusPaint {
    fn push<Op: CompileOp>(self, owner: NodeId, ops: &mut Ops<'_, Op>) {
        if let Some((outline, width)) = self.stroke {
            ops.push(
                owner,
                outline.stroke(self.rect, self.clip, width, self.color),
            );
        } else {
            // Legacy nodes keep the four square edge quads.
            let r = self.rect;
            for rect in [
                Rect::new(r.x, r.y, r.width, 2.),
                Rect::new(r.x, r.y + r.height - 2., r.width, 2.),
                Rect::new(r.x, r.y, 2., r.height),
                Rect::new(r.x + r.width - 2., r.y, 2., r.height),
            ] {
                ops.push(
                    owner,
                    PaintOp::Quad {
                        rect,
                        clip: self.clip,
                        radius: 0.,
                        color: self.color,
                    },
                );
            }
        }
    }
}

struct FeedbackPaint {
    owner: NodeId,
    wave: Option<PathFill>,
    focus: Option<FocusPaint>,
}

impl FeedbackPaint {
    fn push<Op: CompileOp>(self, ops: &mut Ops<'_, Op>) {
        if let Some(wave) = self.wave {
            ops.push_fill(self.owner, wave);
        }
        if let Some(focus) = self.focus {
            focus.push(self.owner, ops);
        }
    }
}

fn raster_checkpoint(
    dom: &MicroDom,
    owner: NodeId,
    name: &str,
    remaining_paint_order: &[NodeId],
) -> Option<NodeId> {
    let id = dom.by_id(name)?;
    let node = dom.get(id).ok()?;
    if node.role() != Role::Group
        || node.is_focusable()
        || node.text_value().is_some()
        || node.raster.is_none()
        || dom.children(id).next().is_some()
        || !remaining_paint_order.contains(&id)
    {
        return None;
    }
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if ancestor == owner {
            return Some(id);
        }
        parent = dom.get(ancestor).ok()?.parent();
    }
    None
}

fn state_ink(color: Color, opacity: u8) -> Color {
    Color(
        color.0,
        color.1,
        color.2,
        (u16::from(color.3) * u16::from(opacity) / 255) as u8,
    )
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
            ops: compile_into(
                dom,
                computed,
                focused,
                decorations,
                interaction,
                None,
                CompileOptions::default(),
            ),
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
                CompileOptions::default(),
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

/// A display list that retains filled vector contours and live raster images.
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
            compile_into(
                dom,
                computed,
                focused,
                decorations,
                interaction,
                None,
                CompileOptions::default(),
            ),
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
            CompileOptions::default(),
        );
        (Self::new(computed.viewport, ops), owners)
    }

    /// Compile a frame with paired background and foreground colors for
    /// editable selection, without allocating an attribution list.
    /// The backend must support [ExtendedPaintOp::TextSelection].
    pub fn compile_with_selection_colors(
        dom: &MicroDom,
        computed: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
        colors: TextSelectionColors,
    ) -> Self {
        Self::new(
            computed.viewport,
            compile_into(
                dom,
                computed,
                focused,
                decorations,
                interaction,
                None,
                CompileOptions {
                    selection_colors: Some(colors),
                    ..CompileOptions::default()
                },
            ),
        )
    }

    /// Compile attributed paint with a paired background and foreground for
    /// editable selection. The backend must support [ExtendedPaintOp::TextSelection].
    /// Ordinary compile methods keep their original selection paint.
    pub fn compile_attributed_with_selection_colors(
        dom: &MicroDom,
        computed: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
        colors: TextSelectionColors,
    ) -> (Self, Vec<NodeId>) {
        let mut owners = Vec::new();
        let ops = compile_into(
            dom,
            computed,
            focused,
            decorations,
            interaction,
            Some(&mut owners),
            CompileOptions {
                selection_colors: Some(colors),
                ..CompileOptions::default()
            },
        );
        (Self::new(computed.viewport, ops), owners)
    }

    /// Compile isolated opacity over nonempty, disjoint ranges of nodes in
    /// the final paint order. The ranges must be in order. Invalid ranges
    /// leave the entire frame ungrouped.
    #[doc(hidden)]
    pub fn compile_with_opacity_groups(
        dom: &MicroDom,
        computed: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
        selection_colors: Option<TextSelectionColors>,
        groups: &[(Range<usize>, f32)],
    ) -> Self {
        let groups: Vec<_> = groups
            .iter()
            .map(|(range, opacity)| (range.clone(), PaintGroupEffect::Opacity(*opacity)))
            .collect();
        Self::compile_with_paint_groups(
            dom,
            computed,
            focused,
            decorations,
            interaction,
            selection_colors,
            &groups,
        )
    }

    /// Compile isolated effects over disjoint, ordered ranges of painted
    /// nodes. A range must be nonempty and lie inside the final paint order.
    /// Invalid inputs leave the entire frame ungrouped.
    #[doc(hidden)]
    pub fn compile_with_paint_groups(
        dom: &MicroDom,
        computed: &ComputedDom,
        focused: Option<NodeId>,
        decorations: &BTreeMap<NodeId, TextDecoration>,
        interaction: InteractionState,
        selection_colors: Option<TextSelectionColors>,
        groups: &[(Range<usize>, PaintGroupEffect)],
    ) -> Self {
        let mut previous_end = 0;
        let valid = groups.iter().all(|(range, effect)| {
            let valid = range.start >= previous_end
                && range.start < range.end
                && range.end <= computed.paint_order.len()
                && effect.is_valid();
            previous_end = range.end;
            valid
        });
        let paint_groups = if valid { groups } else { &[] };
        Self::new(
            computed.viewport,
            compile_into(
                dom,
                computed,
                focused,
                decorations,
                interaction,
                None,
                CompileOptions {
                    selection_colors,
                    paint_groups,
                },
            ),
        )
    }

    pub fn into_ops(self) -> Vec<ExtendedPaintOp> {
        self.ops
    }
}

#[derive(Default)]
struct CompileOptions<'a> {
    selection_colors: Option<TextSelectionColors>,
    paint_groups: &'a [(Range<usize>, PaintGroupEffect)],
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
    options: CompileOptions<'_>,
) -> Vec<Op> {
    let mut ops = Ops {
        ops: Vec::new(),
        owners,
    };
    let mut deferred: Option<(NodeId, FeedbackPaint)> = None;
    let mut pending_groups = options.paint_groups.iter();
    let mut next_group = pending_groups.next();
    let mut active_group: Option<(usize, NodeId)> = None;
    for (index, id) in computed.paint_order.iter().enumerate() {
        if let Some((_, panel)) = active_group.filter(|(end, _)| *end == index) {
            if let Some(op) = Op::end_paint_group() {
                ops.push_owned(panel, op);
            }
            active_group = None;
        }
        if let Some((range, effect)) = next_group.filter(|(range, _)| range.start == index) {
            if let Some(op) = Op::begin_paint_group(*effect) {
                ops.push_owned(*id, op);
            }
            active_group = Some((range.end, *id));
            next_group = pending_groups.next();
        }
        let n = dom.get(*id).expect("computed node belongs to DOM");
        let g = &computed.nodes[id.index()];
        let s = n.style();
        let visible = has_area(g.bounds.intersect(g.clip));
        let corners = n
            .presented_interaction_corners(*id, focused, interaction)
            .or(s.corners);
        let outline = Outline::resolve(s.radius, corners, g.bounds);
        for shadow in n.presented_box_shadow_layers(*id, focused, interaction) {
            shadow.contours(
                g.bounds,
                corners.unwrap_or(Corners::all(s.radius)),
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
        if let Some(image) = n.raster.as_ref().filter(|_| visible) {
            Op::push_raster(
                image,
                g.bounds,
                g.clip,
                corners.unwrap_or(Corners::all(s.radius)),
                |op| {
                    ops.push_owned(*id, op);
                },
            );
        }
        // A clipped checkpoint has no raster draw. It still fixes the order
        // between the owner, the remaining copy and the checkbox.
        if deferred
            .as_ref()
            .is_some_and(|(checkpoint, _)| checkpoint == id)
        {
            if let Some((_, feedback)) = deferred.take() {
                feedback.push(&mut ops);
            }
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
        let active_wave = n.press_ripple.as_deref().and_then(|ripple| {
            (Op::PRESS_RIPPLE && !n.is_disabled())
                .then_some((ripple.presented?, ripple.after.as_deref()))
        });
        let [hover_alpha, focus_alpha, press_alpha] = n.resolved_state_layer_alphas();
        let press_ink = n.resolved_press_state_layer_ink();
        if (s.state_layer.3 > 0 || press_ink.3 > 0) && !n.is_disabled() {
            let mut target = Some(*id);
            let mut ink = s.state_layer;
            let mut opacity = 0;
            while let Some(ancestor) = target {
                if interaction.pressed == Some(ancestor) {
                    // The held wave provides the pressed ink, including on
                    // its first transparent frame. Avoid doubling it with
                    // the hovered or focused fill during the same contact.
                    ink = press_ink;
                    opacity = if ancestor == *id && active_wave.is_some() {
                        0
                    } else {
                        press_alpha
                    };
                    break;
                }
                if focus_alpha > 0 && focused == Some(ancestor) {
                    opacity = focus_alpha;
                } else if opacity == 0 && interaction.hovered == Some(ancestor) {
                    opacity = hover_alpha;
                }
                target = dom.get(ancestor).ok().and_then(|n| n.parent());
            }
            if opacity > 0 && ink.3 > 0 {
                ops.push(*id, outline.fill(g.bounds, g.clip, state_ink(ink, opacity)));
            }
        }
        let focus_color = n
            .focus_outline_color()
            .filter(|_| n.paints_focus_outline(*id, focused));
        let focus = focus_color.map(|color| {
            let width = n.resolved_focus_outline_width();
            FocusPaint {
                rect: g.bounds,
                clip: g.clip,
                color,
                stroke: (corners.is_some() || width.is_some())
                    .then_some((outline, width.unwrap_or(2.))),
            }
        });
        if let Some((wave, after)) = active_wave {
            let mut ink = state_ink(press_ink, press_alpha);
            ink.3 = (f32::from(ink.3) * wave.opacity).round() as u8;
            let wave = press_ripple::contour(
                wave,
                g.bounds,
                g.clip,
                corners.unwrap_or(Corners::all(s.radius)),
                ink,
            );
            let feedback = FeedbackPaint {
                owner: *id,
                wave,
                focus,
            };
            let checkpoint = after.filter(|_| deferred.is_none()).and_then(|name| {
                // Deferred feedback belongs in its owner's isolated range.
                // Do not move it across a later begin or end marker.
                let limit = active_group
                    .map(|(end, _)| end)
                    .or_else(|| next_group.map(|(range, _)| range.start))
                    .unwrap_or(computed.paint_order.len());
                raster_checkpoint(dom, *id, name, &computed.paint_order[index + 1..limit])
            });
            if let Some(checkpoint) = checkpoint {
                deferred = Some((checkpoint, feedback));
            } else {
                feedback.push(&mut ops);
            }
        } else if let Some(focus) = focus {
            focus.push(*id, &mut ops);
        }
        let placeholder = n.text_input_placeholder();
        let text_run = placeholder.or_else(|| n.text_value().map(|text| (text, s.foreground)));
        if let Some((text, text_color)) = text_run {
            let rect = g
                .bounds
                .inset(s.padding)
                .inset_horizontal(n.presented_text_input_horizontal_inset);
            let clip = g.clip.intersect(rect);
            let decoration = decorations.get(id);
            let editable = n.role() == Role::TextInput;
            let selection_colors = options.selection_colors.filter(|_| editable);
            if let (None, Some(d)) = (selection_colors, decoration) {
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
            let text_style = s.text_style();
            let (scroll_x, scroll_y) = decoration.map(|d| d.scroll).unwrap_or_default();
            let scroll = if placeholder.is_some() {
                // A cleared field can retain scroll from its previous value.
                // Keep negative insets used for single-line alignment.
                (scroll_x.min(0.), scroll_y.min(0.))
            } else {
                (scroll_x, scroll_y)
            };
            let run = if !text_style.is_legacy() {
                let (scroll, multiline) = if editable {
                    (scroll, decoration.is_some_and(|d| d.multiline))
                } else {
                    ((0., 0.), true)
                };
                PaintOp::StyledText {
                    rect,
                    clip,
                    text: text.to_owned(),
                    style: text_style,
                    color: text_color,
                    scroll,
                    multiline,
                    editable,
                }
            } else if editable {
                PaintOp::EditableText {
                    rect,
                    clip,
                    text: text.to_owned(),
                    font_size: s.font_size,
                    color: text_color,
                    scroll,
                    multiline: decoration.is_some_and(|d| d.multiline),
                }
            } else {
                PaintOp::Text {
                    rect,
                    clip,
                    text: text.to_owned(),
                    font_size: s.font_size,
                    color: s.foreground,
                }
            };
            let run = match (n.read_only_text_alignment(), selection_colors, decoration) {
                (Some(alignment), _, _) => Op::with_aligned_text(run, alignment),
                (None, Some(colors), Some(decoration)) => {
                    let selected = text_selection::visible_selection(clip, &decoration.selection);
                    if selected.is_empty() {
                        run.into()
                    } else {
                        Op::with_text_selection(run, colors, selected)
                    }
                }
                _ => run.into(),
            };
            ops.push_owned(*id, run);
            if let Some(d) = decoration {
                for r in &d.underline {
                    let mut paint = |rect, color| {
                        ops.push(
                            *id,
                            PaintOp::Quad {
                                rect,
                                clip,
                                radius: 0.,
                                color,
                            },
                        );
                    };
                    if selection_colors.is_some_and(|colors| {
                        text_selection::split_selected_underline(
                            *r,
                            clip,
                            &d.selection,
                            |part, selected| {
                                paint(
                                    part,
                                    if selected {
                                        colors.foreground
                                    } else {
                                        s.foreground
                                    },
                                );
                            },
                        )
                    }) {
                        continue;
                    }
                    paint(*r, s.foreground);
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
    if let Some((_, panel)) = active_group {
        if let Some(op) = Op::end_paint_group() {
            ops.push_owned(panel, op);
        }
    }
    ops.ops
}
