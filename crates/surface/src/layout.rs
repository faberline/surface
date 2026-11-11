//! Retained Taffy layout with renderer-supplied text measurement.
use crate::{Align, Direction, Length, MicroDom, NodeId, Position, Rect, Style, TextStyle};
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
    children: Vec<NodeId>,
}
pub struct LayoutEngine {
    tree: t::TaffyTree<(String, TextStyle)>,
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
        self.paint_order.iter().rev().copied().find(|id| {
            let n = &self.nodes[id.index()];
            dom.get(*id).is_ok_and(|node| node.is_focusable())
                && n.bounds.contains(x, y)
                && n.clip.contains(x, y)
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
fn style(s: &Style) -> t::Style {
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
        flex_shrink: 0.,
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
                children: dom.children(id).collect(),
            })
            .collect();
        for (i, n) in input.iter().enumerate() {
            if !valid(&n.style) {
                return Err(LayoutError::InvalidStyle(i));
            }
        }
        let topology_changed = self.previous.len() != input.len()
            || self.previous.iter().zip(&input).any(|(a, b)| {
                a.children != b.children
                    || (a.style.position == Position::Fixed)
                        != (b.style.position == Position::Fixed)
            });
        if topology_changed {
            self.tree = t::TaffyTree::new();
            self.ids.clear();
            self.viewport = Some(self.tree.new_leaf(t::Style::default())?);
            for n in &input {
                self.ids.push(self.tree.new_leaf_with_context(
                    style(&n.style),
                    (n.text.clone().unwrap_or_default(), n.style.text_style()),
                )?);
            }
            for (i, n) in input.iter().enumerate() {
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
                if style(&a.style) != style(&b.style) {
                    self.tree.set_style(self.ids[i], style(&b.style))?;
                }
                if a.text != b.text || !a.style.same_text_style(&b.style) {
                    self.tree.set_node_context(
                        self.ids[i],
                        Some((b.text.clone().unwrap_or_default(), b.style.text_style())),
                    )?;
                }
            }
        }
        // Root auto size fills the host viewport. Explicit sizes are respected.
        let mut root_style = style(&input[0].style);
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
                    Some((text, style)) if !text.is_empty() => {
                        measurer.measure_text(text, style, limit)
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
        let viewport = Rect::new(0., 0., width, height);
        let mut nodes = Vec::with_capacity(dom.len());
        for (id, n) in dom.iter() {
            let layout = self.tree.layout(self.ids[id.index()])?;
            let (x, y, clip) = if n.style().position == Position::Fixed {
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
            nodes.push(ComputedNode {
                id,
                bounds: Rect::new(
                    x + layout.location.x,
                    y + layout.location.y,
                    layout.size.width,
                    layout.size.height,
                ),
                clip,
            });
        }
        let paint_order = dom.paint_order();
        Ok(ComputedDom {
            viewport,
            nodes,
            paint_order,
        })
    }
}
