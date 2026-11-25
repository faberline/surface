//! Retained Taffy layout with renderer-supplied text measurement.
use crate::{
    microdom::AbsoluteOverlayFit, Align, Direction, Length, MicroDom, Node, NodeId,
    OverlayBoundary, OverlayHorizontalEdge, Position, Rect, Style, TextAlignment, TextStyle,
};
use std::sync::Arc;
use taffy::prelude as t;

/// A renderer must use the same font metrics here and during text painting.
pub trait TextMeasurer {
    /// With a style `line_height`, the default `measure_text` assumes this
    /// returns a height for a 1.5 x `font_size` line. Override `measure_text`
    /// if your measurer uses another line height.
    fn measure(&mut self, text: &str, font_size: f32, width: Option<f32>) -> (f32, f32);
    /// Measure with a full text style. The default keeps size-only measurers
    /// working: it scales the measured height to the requested line height.
    fn measure_text(&mut self, text: &str, style: &TextStyle, width: Option<f32>) -> (f32, f32) {
        let (w, h) = self.measure(text, style.font_size, width);
        match style.line_height {
            // Multiply before dividing so exact ratios stay exact in f32.
            Some(lh) => (w, h * lh / (style.font_size * 1.5)),
            None => (w, h),
        }
    }
    /// Cluster geometry with a full text style. The default scales every
    /// vertical value of the size-only layout to the requested line height.
    fn text_layout_styled(
        &mut self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
    ) -> crate::TextLayout {
        let mut layout = self.text_layout(text, style.font_size, width);
        if let Some(k) = style.line_height_scale() {
            for cluster in &mut layout.clusters {
                cluster.bounds.y *= k;
                cluster.bounds.height *= k;
            }
            layout.line_height *= k;
            layout.height *= k;
        }
        layout
    }
    /// Wrapped editor geometry with an explicit physical alignment per line.
    /// Override when the renderer's shaper can align its own glyphs.
    fn text_layout_aligned_styled(
        &mut self,
        text: &str,
        style: &TextStyle,
        width: f32,
        alignment: TextAlignment,
    ) -> crate::TextLayout {
        let mut layout = self.text_layout_styled(text, style, Some(width));
        for line in layout
            .clusters
            .chunk_by_mut(|left, right| left.bounds.y == right.bounds.y)
        {
            let (start, end) = line.iter().fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(start, end), cluster| {
                    let bounds = cluster.bounds;
                    (start.min(bounds.x), end.max(bounds.x + bounds.width))
                },
            );
            let space = (width - (end - start).max(1.)).max(0.);
            let target = match alignment {
                TextAlignment::Left => 0.,
                TextAlignment::Center => space / 2.,
                TextAlignment::Right => space,
            };
            for cluster in line {
                cluster.bounds.x += target - start;
            }
        }
        layout
    }
    fn text_layout(&mut self, text: &str, font_size: f32, width: Option<f32>) -> crate::TextLayout {
        let line_height = font_size * 1.5;
        let mut clusters = Vec::new();
        let mut x = 0.;
        let mut y = 0.;
        for (start, c) in text.char_indices() {
            if c == '\n' {
                x = 0.;
                y += line_height;
                continue;
            }
            let (w, _) = self.measure(&c.to_string(), font_size, None);
            if width.is_some_and(|limit| x + w > limit) && x > 0. {
                x = 0.;
                y += line_height;
            }
            clusters.push(crate::TextCluster {
                bytes: start..start + c.len_utf8(),
                bounds: Rect::new(x, y, w, line_height),
                rtl: false,
            });
            x += w;
        }
        crate::TextLayout {
            clusters,
            line_height,
            text_len: text.len(),
            height: y + line_height,
        }
    }
}
/// Measure a group as a single layout item and place its direct flow children.
///
/// The children keep their authored semantics and paint order. The group
/// owns placement and the usual ancestor and group clipping applies. Measured
/// sizes and placed rectangles must be finite and have nonnegative dimensions.
pub trait MeasuredGroupLayout: std::fmt::Debug + Send + Sync {
    /// Allow direct children that have their own measured layout.
    ///
    /// Each child places its descendants within the size given by this
    /// group. Normal flow containers with descendants and out-of-flow
    /// children remain unsupported.
    fn places_measured_groups(&self) -> bool {
        false
    }

    /// Return the group's desired content size within the available width.
    fn measure(
        &self,
        children: &[&Node],
        available_width: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32);

    /// Return one group-relative rectangle per child in authored order.
    /// `size` is the group's final outer size, including any style padding.
    fn place(
        &self,
        children: &[&Node],
        size: (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect>;
}

#[derive(Clone, Debug)]
pub(crate) struct MeasuredGroup(pub(crate) Arc<dyn MeasuredGroupLayout>);

impl PartialEq for MeasuredGroup {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("viewport must be finite and positive")]
    InvalidViewport,
    #[error("invalid layout value on node {0}")]
    InvalidStyle(usize),
    #[error(transparent)]
    Taffy(#[from] taffy::TaffyError),
}
#[derive(Clone, Debug, PartialEq)]
struct Input {
    text: Option<String>,
    style: Style,
    shrink: bool,
    reverse_row: bool,
    children: Vec<NodeId>,
    measured: bool,
}

impl Input {
    fn taffy_style(&self) -> t::Style {
        style(&self.style, self.shrink, self.reverse_row)
    }
}
enum MeasureContext {
    Text(String, TextStyle),
    Group(NodeId),
}

fn valid_size(width: f32, height: f32) -> bool {
    width.is_finite() && height.is_finite() && width >= 0. && height >= 0.
}

fn valid_placement(rect: Rect) -> bool {
    valid_size(rect.width, rect.height)
        && rect.x.is_finite()
        && rect.y.is_finite()
        && (rect.x + rect.width).is_finite()
        && (rect.y + rect.height).is_finite()
}

fn measured_children(dom: &MicroDom, id: NodeId) -> Vec<&Node> {
    dom.children(id)
        .map(|child| dom.get(child).expect("valid measured group child"))
        .collect()
}

pub struct LayoutEngine {
    tree: t::TaffyTree<MeasureContext>,
    ids: Vec<t::NodeId>,
    previous: Vec<Input>,
    viewport: Option<t::NodeId>,
}
impl Default for LayoutEngine {
    fn default() -> Self {
        Self {
            tree: t::TaffyTree::new(),
            ids: Vec::new(),
            previous: Vec::new(),
            viewport: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ComputedNode {
    pub id: NodeId,
    pub bounds: Rect,
    pub clip: Rect,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ComputedDom {
    pub viewport: Rect,
    pub nodes: Vec<ComputedNode>,
    pub paint_order: Vec<NodeId>,
}
impl ComputedDom {
    /// Last-painted visible enabled control wins. Containers do not steal input.
    pub fn hit_test(&self, dom: &MicroDom, x: f32, y: f32) -> Option<NodeId> {
        self.hit_test_where(dom, x, y, |_| true)
    }

    /// Find the last-painted visible enabled control accepted by `eligible`.
    pub fn hit_test_where(
        &self,
        dom: &MicroDom,
        x: f32,
        y: f32,
        mut eligible: impl FnMut(NodeId) -> bool,
    ) -> Option<NodeId> {
        self.paint_order.iter().rev().copied().find(|id| {
            let n = &self.nodes[id.index()];
            dom.get(*id).is_ok_and(|node| node.is_focusable())
                && n.bounds.contains(x, y)
                && n.clip.contains(x, y)
                && eligible(*id)
        })
    }

    /// Nodes whose bounds or clip differ from an earlier frame, or that the
    /// earlier frame did not have. Index order.
    pub fn changed_since(&self, previous: &ComputedDom) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| {
                previous
                    .nodes
                    .get(node.id.index())
                    .is_none_or(|old| old.bounds != node.bounds || old.clip != node.clip)
            })
            .map(|node| node.id)
            .collect()
    }
}

fn overlay_owner(dom: &MicroDom, id: NodeId, owner_id: &str) -> Option<NodeId> {
    let owner = dom.by_id(owner_id)?;
    if owner >= id {
        return None;
    }
    let owner_node = dom.get(owner).ok()?;
    let mut branch = id;
    loop {
        if branch == owner {
            return Some(owner);
        }
        let node = dom.get(branch).ok()?;
        if node.style().position == Position::Fixed {
            return None;
        }
        let parent = node.parent()?;
        if Some(parent) == owner_node.parent()
            && owner < branch
            && owner_node.style().position != Position::Fixed
        {
            return Some(owner);
        }
        branch = parent;
    }
}

fn fit_overlay(
    bounds: Rect,
    clip: Rect,
    owner: &ComputedNode,
    fit: &AbsoluteOverlayFit,
) -> Option<Rect> {
    let visible_owner = owner.bounds.intersect(owner.clip).intersect(clip);
    if visible_owner.width <= 0. || visible_owner.height <= 0. {
        return None;
    }
    let fit_to_owner = fit.boundary == OverlayBoundary::Owner;
    let horizontal = if fit_to_owner && bounds.width <= visible_owner.width {
        visible_owner
    } else {
        clip
    };
    let top = if fit_to_owner && bounds.height <= visible_owner.height {
        visible_owner.y
    } else {
        clip.y
    };
    let x = match fit.edge {
        OverlayHorizontalEdge::Left => bounds.x.max(horizontal.x),
        OverlayHorizontalEdge::Right => {
            bounds.x.min(horizontal.x + horizontal.width - bounds.width)
        }
    };
    Some(Rect::new(x, bounds.y.max(top), bounds.width, bounds.height))
}

fn dimension(value: Length) -> t::Dimension {
    match value {
        Length::Auto => t::Dimension::Auto,
        Length::Px(v) => t::Dimension::Length(v),
        Length::Percent(v) => t::Dimension::Percent(v),
    }
}
fn valid(s: &Style) -> bool {
    let length = |v| match v {
        Length::Auto => true,
        Length::Px(v) | Length::Percent(v) => v.is_finite() && v >= 0.,
    };
    [s.width, s.height, s.max_width, s.max_height]
        .into_iter()
        .all(length)
        && [
            s.min_width,
            s.min_height,
            s.gap,
            s.padding,
            s.grow,
            s.radius,
            s.font_size,
            s.border_width,
        ]
        .into_iter()
        .all(|v| v.is_finite() && v >= 0.)
        && s.font_size > 0.
        && s.corners.is_none_or(|c| c.is_valid())
        && (1..=1000).contains(&s.font_weight)
        && s.line_height.is_none_or(|v| v.is_finite() && v > 0.)
        && s.letter_spacing.is_finite()
        && s.scroll_x.is_finite()
        && s.scroll_y.is_finite()
        && s.transition.is_none_or(|t| t.is_valid())
        && [s.inset.left, s.inset.top, s.inset.right, s.inset.bottom]
            .into_iter()
            .all(|v| match v {
                Length::Auto => true,
                Length::Px(v) | Length::Percent(v) => v.is_finite(),
            })
}
fn style(s: &Style, shrink: bool, reverse_row: bool) -> t::Style {
    let offset = |v| match v {
        Length::Auto => t::LengthPercentageAuto::Auto,
        Length::Px(v) => t::LengthPercentageAuto::Length(v),
        Length::Percent(v) => t::LengthPercentageAuto::Percent(v),
    };
    t::Style {
        display: t::Display::Flex,
        position: match s.position {
            Position::Flow => t::Position::Relative,
            Position::Absolute | Position::Fixed => t::Position::Absolute,
        },
        inset: t::Rect {
            left: offset(s.inset.left),
            top: offset(s.inset.top),
            right: offset(s.inset.right),
            bottom: offset(s.inset.bottom),
        },
        justify_content: Some(match s.justify {
            Align::Center => t::JustifyContent::Center,
            Align::End => t::JustifyContent::FlexEnd,
            _ => t::JustifyContent::FlexStart,
        }),
        size: t::Size {
            width: dimension(s.width),
            height: dimension(s.height),
        },
        min_size: t::Size {
            width: t::Dimension::Length(s.min_width),
            height: t::Dimension::Length(s.min_height),
        },
        max_size: t::Size {
            width: dimension(s.max_width),
            height: dimension(s.max_height),
        },
        flex_direction: match s.direction {
            Direction::Row if reverse_row => t::FlexDirection::RowReverse,
            Direction::Row => t::FlexDirection::Row,
            Direction::Column => t::FlexDirection::Column,
        },
        flex_wrap: if s.wrap {
            t::FlexWrap::Wrap
        } else {
            t::FlexWrap::NoWrap
        },
        align_items: Some(match s.align {
            Align::Start => t::AlignItems::FlexStart,
            Align::Center => t::AlignItems::Center,
            Align::End => t::AlignItems::FlexEnd,
            Align::Stretch => t::AlignItems::Stretch,
        }),
        gap: t::Size {
            width: t::LengthPercentage::Length(s.gap),
            height: t::LengthPercentage::Length(s.gap),
        },
        padding: t::Rect::length(s.padding),
        flex_grow: s.grow,
        flex_shrink: if shrink { 1. } else { 0. },
        ..Default::default()
    }
}
impl LayoutEngine {
    pub fn compute(
        &mut self,
        dom: &MicroDom,
        width: f32,
        height: f32,
        measurer: &mut impl TextMeasurer,
    ) -> Result<ComputedDom, LayoutError> {
        if !width.is_finite() || !height.is_finite() || width <= 0. || height <= 0. {
            return Err(LayoutError::InvalidViewport);
        }
        let input: Vec<_> = dom
            .iter()
            .map(|(id, n)| Input {
                text: n.text_value().map(str::to_owned),
                style: n.style().clone(),
                reverse_row: n.is_row_reversed(),
                shrink: n.parent().and_then(|id| dom.get(id).ok()).is_some_and(
                    |parent| match parent.style().direction {
                        Direction::Row => n.can_shrink_width(),
                        Direction::Column => n.can_shrink_height(),
                    },
                ),
                children: dom.children(id).collect(),
                measured: n.measured_group_layout().is_some(),
            })
            .collect();
        let mut has_measured_groups = false;
        let mut has_overlay_fits = false;
        for ((id, node), n) in dom.iter().zip(&input) {
            let i = id.index();
            has_measured_groups |= n.measured;
            has_overlay_fits |= node.absolute_overlay_fit().is_some();
            if !valid(&n.style) {
                return Err(LayoutError::InvalidStyle(i));
            }
            if let Some(group) = node.measured_group_layout() {
                let nested = group.places_measured_groups();
                if n.children.iter().any(|id| {
                    let child = &input[id.index()];
                    child.style.position != Position::Flow
                        || if child.measured {
                            !nested
                        } else {
                            !child.children.is_empty()
                        }
                }) {
                    return Err(LayoutError::InvalidStyle(i));
                }
            }
        }
        let topology_changed = self.previous.len() != input.len()
            || self.previous.iter().zip(&input).any(|(a, b)| {
                a.children != b.children
                    || a.measured != b.measured
                    || (a.style.position == Position::Fixed)
                        != (b.style.position == Position::Fixed)
            });
        if topology_changed {
            self.tree = t::TaffyTree::new();
            self.ids.clear();
            self.viewport = Some(self.tree.new_leaf(t::Style::default())?);
            for ((id, _), n) in dom.iter().zip(&input) {
                let context = if n.measured {
                    MeasureContext::Group(id)
                } else {
                    MeasureContext::Text(n.text.clone().unwrap_or_default(), n.style.text_style())
                };
                self.ids
                    .push(self.tree.new_leaf_with_context(n.taffy_style(), context)?);
            }
            for (i, n) in input.iter().enumerate() {
                if n.measured {
                    continue;
                }
                let children: Vec<_> = n
                    .children
                    .iter()
                    .filter(|c| input[c.index()].style.position != Position::Fixed)
                    .map(|c| self.ids[c.index()])
                    .collect();
                self.tree.set_children(self.ids[i], &children)?;
            }
            let children: Vec<_> = std::iter::once(self.ids[0])
                .chain(
                    input
                        .iter()
                        .enumerate()
                        .skip(1)
                        .filter(|(_, n)| n.style.position == Position::Fixed)
                        .map(|(i, _)| self.ids[i]),
                )
                .collect();
            self.tree
                .set_children(self.viewport.expect("viewport exists"), &children)?;
        } else {
            for (i, (a, b)) in self.previous.iter().zip(&input).enumerate() {
                if a.taffy_style() != b.taffy_style() {
                    self.tree.set_style(self.ids[i], b.taffy_style())?;
                }
                if b.measured {
                    // The layout may read properties beyond the text and
                    // flex inputs, such as the child's icon or paint style.
                    self.tree.mark_dirty(self.ids[i])?;
                } else if a.text != b.text || !a.style.same_text_style(&b.style) {
                    self.tree.set_node_context(
                        self.ids[i],
                        Some(MeasureContext::Text(
                            b.text.clone().unwrap_or_default(),
                            b.style.text_style(),
                        )),
                    )?;
                }
            }
        }
        // Root auto size fills the host viewport. Explicit sizes are respected.
        let mut root_style = input[0].taffy_style();
        // The viewport wrapper only positions scene roots. Keeping the normal
        // root absolute avoids a second flex intrinsic-measurement pass and
        // preserves caches for unchanged siblings inside that root.
        root_style.position = t::Position::Absolute;
        root_style.inset = t::Rect {
            left: t::LengthPercentageAuto::Length(0.),
            top: t::LengthPercentageAuto::Length(0.),
            right: t::LengthPercentageAuto::Auto,
            bottom: t::LengthPercentageAuto::Auto,
        };
        if matches!(input[0].style.width, Length::Auto) {
            root_style.size.width = t::Dimension::Length(width);
        }
        if matches!(input[0].style.height, Length::Auto) {
            root_style.size.height = t::Dimension::Length(height);
        }
        if self.tree.style(self.ids[0])? != &root_style {
            self.tree.set_style(self.ids[0], root_style)?;
        }
        let viewport_id = self.viewport.expect("viewport exists");
        let viewport_style = t::Style {
            size: t::Size {
                width: t::Dimension::Length(width),
                height: t::Dimension::Length(height),
            },
            ..Default::default()
        };
        if self.tree.style(viewport_id)? != &viewport_style {
            self.tree.set_style(viewport_id, viewport_style)?;
        }
        let mut invalid_measurement = None;
        self.tree.compute_layout_with_measure(
            viewport_id,
            t::Size {
                width: t::AvailableSpace::Definite(width),
                height: t::AvailableSpace::Definite(height),
            },
            |known, available, _, context, _| {
                let limit = known.width.or(match available.width {
                    t::AvailableSpace::Definite(w) => Some(w),
                    _ => None,
                });
                let (w, h) = match context {
                    Some(MeasureContext::Text(text, style)) if !text.is_empty() => {
                        measurer.measure_text(text, style, limit)
                    }
                    Some(MeasureContext::Group(id)) => {
                        let group = dom.get(*id).expect("valid measured group");
                        let layout = group.measured_group_layout().expect("measured group");
                        let children = measured_children(dom, *id);
                        let limit = limit.filter(|width| width.is_finite() && *width >= 0.);
                        let (w, h) = layout.measure(&children, limit, measurer);
                        if !valid_size(w, h) {
                            invalid_measurement.get_or_insert(id.index());
                            return t::Size::ZERO;
                        }
                        (w, h)
                    }
                    _ => (0., 0.),
                };
                t::Size {
                    width: known.width.unwrap_or(w),
                    height: known.height.unwrap_or(h),
                }
            },
        )?;
        self.previous = input;
        if let Some(index) = invalid_measurement {
            return Err(LayoutError::InvalidStyle(index));
        }
        let viewport = Rect::new(0., 0., width, height);
        let mut nodes = Vec::with_capacity(dom.len());
        let mut placements = Vec::<Option<Rect>>::new();
        if has_measured_groups {
            placements.resize(dom.len(), None);
        }
        let mut hidden_overlays = has_overlay_fits.then(|| Vec::with_capacity(dom.len()));
        for (id, n) in dom.iter() {
            let placement = placements.get(id.index()).copied().flatten();
            let layout = self.tree.layout(self.ids[id.index()])?;
            let (x, y, mut clip) = if n.style().position == Position::Fixed {
                (0., 0., viewport)
            } else if let Some(parent) = n.parent() {
                let p: &ComputedNode = &nodes[parent.index()];
                let ps = dom.get(parent).expect("valid parent").style();
                (
                    p.bounds.x - ps.scroll_x,
                    p.bounds.y - ps.scroll_y,
                    if ps.clip {
                        p.clip.intersect(p.bounds)
                    } else {
                        p.clip
                    },
                )
            } else {
                (0., 0., viewport)
            };
            let (local_y, height) = placement
                .map_or((layout.location.y, layout.size.height), |rect| {
                    (rect.y, rect.height)
                });
            let (local_x, width) = placement
                .map_or((layout.location.x, layout.size.width), |rect| {
                    (rect.x, rect.width)
                });
            let mut bounds = Rect::new(
                x + local_x + n.horizontal_slide() * width,
                y + local_y,
                width,
                height,
            );
            if let Some(hidden_overlays) = &mut hidden_overlays {
                let mut hidden = n
                    .parent()
                    .is_some_and(|parent| hidden_overlays[parent.index()]);
                if let Some(fit) = n.absolute_overlay_fit().filter(|_| !hidden) {
                    let fitted = overlay_owner(dom, id, &fit.owner_id)
                        .and_then(|owner| nodes.get(owner.index()))
                        .and_then(|owner| fit_overlay(bounds, clip, owner, fit));
                    if let Some(fitted) = fitted {
                        bounds = fitted;
                    } else {
                        hidden = true;
                    }
                }
                if hidden {
                    // Keep the suppression through fixed descendants, which
                    // would otherwise start a fresh clip at the viewport.
                    clip = Rect::default();
                }
                hidden_overlays.push(hidden);
            }
            if placement.is_some() && !valid_placement(bounds) {
                return Err(LayoutError::InvalidStyle(
                    n.parent().expect("placed child has a group").index(),
                ));
            }
            if let Some(layout) = n.measured_group_layout() {
                if !valid_placement(bounds) {
                    return Err(LayoutError::InvalidStyle(id.index()));
                }
                let children = measured_children(dom, id);
                let rects = layout.place(&children, (bounds.width, bounds.height), measurer);
                if rects.len() != children.len() || rects.iter().any(|rect| !valid_placement(*rect))
                {
                    return Err(LayoutError::InvalidStyle(id.index()));
                }
                for (child, rect) in dom.children(id).zip(rects) {
                    placements[child.index()] = Some(rect);
                }
            }
            nodes.push(ComputedNode { id, bounds, clip });
        }
        let paint_order = dom.paint_order();
        Ok(ComputedDom {
            viewport,
            nodes,
            paint_order,
        })
    }
}
