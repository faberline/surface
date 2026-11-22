//! Scroll control: stick to bottom, scroll-to requests and read-only metrics.
use crate::{AppError, Application, Queue, Runtime};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use surface::{scroll_extent, ComputedDom, NodeId};

/// Distance from the bottom, in pixels, that still counts as "at bottom".
const BOTTOM_TOLERANCE: f32 = 0.5;

/// Where `scroll_to` moves a scroll container. Offsets are clamped to the
/// content at the next `render`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollTarget {
    /// Vertical offset 0. The horizontal offset stays.
    Top,
    /// The largest vertical offset. The horizontal offset stays.
    Bottom,
    Offset {
        x: f32,
        y: f32,
    },
}

/// Scroll state of one container, as of the last `render`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollMetrics {
    pub offset: (f32, f32),
    /// Content size: at least the viewport size.
    pub content: (f32, f32),
    /// The container's own size.
    pub viewport: (f32, f32),
    /// The vertical offset is within half a pixel of the largest one.
    pub at_bottom: bool,
    /// The container follows new content (`View::stick_to_bottom`).
    pub stuck: bool,
}

pub(crate) type Pending = Rc<RefCell<Vec<(String, ScrollTarget)>>>;

#[derive(Default)]
pub(crate) struct State {
    /// The largest vertical offset at the last render, for stick containers.
    last_max: BTreeMap<String, f32>,
    pub(crate) metrics: BTreeMap<String, ScrollMetrics>,
}

impl<M> Queue<M> {
    /// Ask a scroll container to move at the next `render`. It never fails:
    /// an unknown id, a node without `clip` or a non-finite offset is dropped.
    pub fn scroll_to(&self, id: &str, target: ScrollTarget) {
        self.2.borrow_mut().push((id.to_owned(), target));
    }
}

fn finite(target: ScrollTarget) -> bool {
    match target {
        ScrollTarget::Offset { x, y } => x.is_finite() && y.is_finite(),
        _ => true,
    }
}

impl<A: Application> Runtime<A> {
    /// Move a scroll container at the next `render`. Returns `false`, and
    /// does nothing, for an unknown id, a node that is not a `clip` scroller
    /// or a non-finite offset.
    pub fn scroll_to(&mut self, id: &str, target: ScrollTarget) -> bool {
        let is_scroller = self
            .dom
            .by_id(id)
            .and_then(|node| self.dom.get(node).ok())
            .is_some_and(|node| node.style().clip);
        if !is_scroller || !finite(target) {
            return false;
        }
        self.queue.scroll_to(id, target);
        true
    }
    /// Scroll state as of the last `render`. `None` before the first render
    /// and for an id that is not a `clip` scroller.
    pub fn scroll_metrics(&self, id: &str) -> Option<ScrollMetrics> {
        self.scroll_state.metrics.get(id).copied()
    }
    /// Run once per frame after layout and the focus reveal. Applies pending
    /// requests, follows the bottom of `stick_to_bottom` containers and
    /// clamps tracked containers. Returns `true` if an offset changed and
    /// the frame needs another layout.
    pub(crate) fn apply_scroll_control(
        &mut self,
        computed: &ComputedDom,
    ) -> Result<bool, AppError> {
        let requests = std::mem::take(&mut *self.queue.2.borrow_mut());
        let mut metrics = BTreeMap::new();
        let mut last_max = BTreeMap::new();
        let mut changed = false;
        let scrollers: Vec<NodeId> = (0..self.dom.len())
            .map(|i| computed.nodes[i].id)
            .filter(|id| {
                self.dom
                    .get(*id)
                    .is_ok_and(|n| n.style().clip && n.stable_id().is_some())
            })
            .collect();
        for node in scrollers {
            let key = self.stable(node).expect("scroller has an ID");
            let stick = self.bindings.get(&node).is_some_and(|b| b.stick);
            let mine: Vec<ScrollTarget> = requests
                .iter()
                .filter(|(id, _)| *id == key)
                .map(|(_, target)| *target)
                .collect();
            let tracked = stick || self.scroll.contains_key(&key) || !mine.is_empty();
            let style = self.dom.get(node)?.style().clone();
            let bounds = computed.nodes[node.index()].bounds;
            let content =
                scroll_extent(&self.dom, computed, node).unwrap_or((bounds.width, bounds.height));
            let max = (
                (content.0 - bounds.width).max(0.),
                (content.1 - bounds.height).max(0.),
            );
            let mut offset = (style.scroll_x, style.scroll_y);
            let mut stuck = false;
            if tracked {
                for target in &mine {
                    offset = match *target {
                        ScrollTarget::Top => (offset.0, 0.),
                        ScrollTarget::Bottom => (offset.0, max.1),
                        ScrollTarget::Offset { x, y } => (x, y),
                    };
                }
                if stick {
                    stuck = if !mine.is_empty() {
                        offset.1 >= max.1 - BOTTOM_TOLERANCE
                    } else {
                        self.scroll_state
                            .last_max
                            .get(&key)
                            .is_none_or(|last| offset.1 >= last - BOTTOM_TOLERANCE)
                    };
                    if stuck {
                        offset.1 = max.1;
                    }
                    last_max.insert(key.clone(), max.1);
                }
                offset = (offset.0.clamp(0., max.0), offset.1.clamp(0., max.1));
                if offset != (style.scroll_x, style.scroll_y) {
                    let mut style = style;
                    style.scroll_x = offset.0;
                    style.scroll_y = offset.1;
                    self.dom.set_style(node, style)?;
                    self.scroll.insert(key.clone(), offset);
                    changed = true;
                }
            }
            metrics.insert(
                key,
                ScrollMetrics {
                    offset,
                    content,
                    viewport: (bounds.width, bounds.height),
                    at_bottom: offset.1 >= max.1 - BOTTOM_TOLERANCE,
                    stuck,
                },
            );
        }
        self.scroll_state.last_max = last_max;
        self.scroll_state.metrics = metrics;
        Ok(changed)
    }
}
