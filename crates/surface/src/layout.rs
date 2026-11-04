//! Retained Taffy layout with renderer-supplied text measurement.
use crate::{Align, Direction, Length, MicroDom, NodeId, Rect, Style};
use taffy::prelude as t;

/// A renderer must use the same font metrics here and during text painting.
pub trait TextMeasurer {
    fn measure(&mut self, text: &str, font_size: f32, width: Option<f32>) -> (f32, f32);
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
    tree: t::TaffyTree<(String, f32)>,
    ids: Vec<t::NodeId>,
    previous: Vec<Input>,
}
impl Default for LayoutEngine {
    fn default() -> Self {
        Self {
            tree: t::TaffyTree::new(),
            ids: Vec::new(),
            previous: Vec::new(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct ComputedNode {
    pub id: NodeId,
    pub bounds: Rect,
    pub clip: Rect,
}
#[derive(Clone, Debug)]
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
    [s.width, s.height, s.max_width].into_iter().all(length)
        && [
            s.min_width,
            s.min_height,
            s.gap,
            s.padding,
            s.grow,
            s.radius,
            s.font_size,
        ]
        .into_iter()
        .all(|v| v.is_finite() && v >= 0.)
        && s.font_size > 0.
        && s.scroll_x.is_finite()
        && s.scroll_y.is_finite()
}
fn style(s: &Style) -> t::Style {
    t::Style {
        display: t::Display::Flex,
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
            height: t::Dimension::Auto,
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
            || self
                .previous
                .iter()
                .zip(&input)
                .any(|(a, b)| a.children != b.children);
        if topology_changed {
            self.tree = t::TaffyTree::new();
            self.ids.clear();
            for n in &input {
                self.ids.push(self.tree.new_leaf_with_context(
                    style(&n.style),
                    (n.text.clone().unwrap_or_default(), n.style.font_size),
                )?);
            }
            for (i, n) in input.iter().enumerate() {
                let children: Vec<_> = n.children.iter().map(|c| self.ids[c.index()]).collect();
                self.tree.set_children(self.ids[i], &children)?;
            }
        } else {
            for (i, (a, b)) in self.previous.iter().zip(&input).enumerate() {
                if style(&a.style) != style(&b.style) {
                    self.tree.set_style(self.ids[i], style(&b.style))?;
                }
                if a.text != b.text || a.style.font_size != b.style.font_size {
                    self.tree.set_node_context(
                        self.ids[i],
                        Some((b.text.clone().unwrap_or_default(), b.style.font_size)),
                    )?;
                }
            }
        }
        // Root auto size fills the host viewport. Explicit sizes are respected.
        let mut root_style = style(&input[0].style);
        if matches!(input[0].style.width, Length::Auto) {
            root_style.size.width = t::Dimension::Length(width);
        }
        if matches!(input[0].style.height, Length::Auto) {
            root_style.size.height = t::Dimension::Length(height);
        }
        if self.tree.style(self.ids[0])? != &root_style {
            self.tree.set_style(self.ids[0], root_style)?;
        }
        self.tree.compute_layout_with_measure(
            self.ids[0],
            t::Size {
                width: t::AvailableSpace::Definite(width),
                height: t::AvailableSpace::Definite(height),
            },
            |known, available, _, context, _| {
                let (text, size) = context
                    .map(|(text, size)| (text.as_str(), *size))
                    .unwrap_or(("", 16.));
                let limit = known.width.or(match available.width {
                    t::AvailableSpace::Definite(w) => Some(w),
                    _ => None,
                });
                let (w, h) = if text.is_empty() {
                    (0., 0.)
                } else {
                    measurer.measure(text, size, limit)
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
            let (x, y, clip) = if let Some(parent) = n.parent() {
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
        fn order(dom: &MicroDom, id: NodeId, out: &mut Vec<NodeId>) {
            out.push(id);
            let mut children: Vec<_> = dom.children(id).collect();
            children.sort_by_key(|id| dom.get(*id).expect("valid node").style().z_index);
            for child in children {
                order(dom, child, out);
            }
        }
        let mut paint_order = Vec::with_capacity(nodes.len());
        order(dom, dom.root(), &mut paint_order);
        Ok(ComputedDom {
            viewport,
            nodes,
            paint_order,
        })
    }
}
