//! Scroll control: stick to bottom, scroll-to requests and read-only metrics.
use crate::{AppError, Application, Queue, Runtime};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use surface::{scroll_extent, ComputedDom, NodeId, Role, TextMeasurer};

/// Distance from the bottom, in pixels, that still counts as "at bottom".
const BOTTOM_TOLERANCE: f32 = 0.5;
const PENDING_WHEEL_LIMIT: usize = 32;

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

/// A vertical wheel or trackpad scroll in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerticalScroll {
    /// Positive moves down through the content. Negative moves up.
    pub delta: f32,
    /// The scroll offset before this event.
    pub offset: f32,
    /// The largest scroll offset at the current layout.
    pub max_offset: f32,
}

/// The amount a vertical scroll hook takes and the message it dispatches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerticalScrollResponse<M> {
    /// Use the sign of the offered delta. The runtime bounds this amount by
    /// that delta. A non-finite amount or one in the opposite direction takes
    /// no scroll from the view.
    pub consumed: f32,
    /// Dispatched after the remaining scroll is saved.
    pub message: M,
}

pub(crate) type VerticalScrollHandler<M> =
    Rc<dyn Fn(VerticalScroll, Option<&str>) -> Option<VerticalScrollResponse<M>>>;

/// A horizontal wheel or trackpad scroll in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizontalScroll {
    /// Positive scrolls to the physical right. Negative scrolls left.
    pub delta: f32,
    /// The runtime's horizontal offset before this event.
    pub offset: f32,
    /// The largest horizontal offset at the current layout.
    pub max_offset: f32,
    /// The monotonic host clock at dispatch, if the host set one.
    pub now_ms: Option<f64>,
}

/// The horizontal wheel result from an opted-in view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizontalScrollResponse<M> {
    /// Set the runtime offset to this absolute position, clamped to the
    /// scroll range. `None` leaves the offset with the application. The
    /// runtime ignores a non-finite value and still delivers the message.
    pub offset: Option<f32>,
    /// Dispatched after both wheel offsets are saved.
    pub message: M,
}

pub(crate) type HorizontalScrollHandler<M> =
    Rc<dyn Fn(HorizontalScroll, Option<&str>) -> Option<HorizontalScrollResponse<M>>>;

pub(crate) enum Request {
    Position(ScrollTarget),
    ChildTop { child: String, top: f32 },
}

pub(crate) type Pending = Rc<RefCell<Vec<(String, Request)>>>;

#[derive(Default)]
pub(crate) struct State {
    /// The largest vertical offset at the last render, for stick containers.
    last_max: BTreeMap<String, f32>,
    pub(crate) metrics: BTreeMap<String, ScrollMetrics>,
    wheel_burst: Option<WheelBurst>,
}

#[derive(Clone, Copy)]
struct WheelRoute {
    hit: NodeId,
    horizontal: Option<NodeId>,
    vertical: Option<NodeId>,
}

#[derive(PartialEq, Eq)]
struct WheelOwner {
    key: String,
    role: Role,
    enabled: bool,
    observes_horizontal: bool,
    observes_vertical: bool,
}

struct WheelSample {
    delta: (f32, f32),
    now_ms: Option<f64>,
}

struct WheelBurst {
    point: (f32, f32),
    scope: Option<String>,
    route: [Option<WheelOwner>; 3],
    pending: Vec<WheelSample>,
}

impl<M> Queue<M> {
    /// Ask a scroll container to move at the next `render`. It never fails:
    /// an unknown id, a node without `clip` or a non-finite offset is dropped.
    pub fn scroll_to(&self, id: &str, target: ScrollTarget) {
        self.2
            .borrow_mut()
            .push((id.to_owned(), Request::Position(target)));
    }

    /// Place a descendant's top at `top` pixels from the scroller's top edge.
    /// Use the next render's layout and preserve the horizontal offset. The
    /// vertical offset stays within the content bounds. Unknown IDs, targets
    /// that do not move with the scroller and non-finite positions are ignored.
    pub fn scroll_to_child_top(&self, scroller: &str, child: &str, top: f32) {
        if top.is_finite() {
            self.2.borrow_mut().push((
                scroller.to_owned(),
                Request::ChildTop {
                    child: child.to_owned(),
                    top,
                },
            ));
        }
    }
}

fn finite(target: ScrollTarget) -> bool {
    match target {
        ScrollTarget::Offset { x, y } => x.is_finite() && y.is_finite(),
        _ => true,
    }
}

impl<A: Application> Runtime<A> {
    pub(crate) fn dispatch_wheel(
        &mut self,
        point: (f32, f32),
        delta: (f32, f32),
    ) -> Result<bool, AppError> {
        if !delta.0.is_finite() || !delta.1.is_finite() {
            return Ok(false);
        }
        let sample = WheelSample {
            delta,
            now_ms: self.input_time_ms(),
        };
        let scope = self.active_scope_root().and_then(|id| self.stable(id));
        if self.computed.is_none() {
            return Ok(self.scroll_state.wheel_burst.as_mut().is_some_and(|burst| {
                if burst.point != point
                    || burst.scope != scope
                    || burst.pending.len() >= PENDING_WHEEL_LIMIT
                {
                    return false;
                }
                burst.pending.push(sample);
                true
            }));
        }
        self.discard_wheel_burst();
        let Some(route) = self.wheel_route(point) else {
            return Ok(false);
        };
        let owners = self.wheel_owners(route);
        let handled = self.dispatch_wheel_route(route, sample)?;
        if handled && scope == self.active_scope_root().and_then(|id| self.stable(id)) {
            self.scroll_state.wheel_burst = Some(WheelBurst {
                point,
                scope,
                route: owners,
                pending: Vec::new(),
            });
        }
        Ok(handled)
    }

    pub(crate) fn discard_wheel_burst(&mut self) {
        self.scroll_state.wheel_burst = None;
    }

    /// Only continue a previously verified wheel route. Rebuilds can move or
    /// replace its controls, so hit test current geometry before each sample.
    pub(crate) fn replay_wheel_burst(
        &mut self,
        width: f32,
        height: f32,
        measurer: &mut impl TextMeasurer,
    ) -> Result<(), AppError> {
        let Some(burst) = self.scroll_state.wheel_burst.take() else {
            return Ok(());
        };
        for sample in burst.pending {
            if self.queue.1.get()
                || !self.queue.0.borrow().is_empty()
                || burst.scope != self.active_scope_root().and_then(|id| self.stable(id))
            {
                break;
            }
            let scale = self.app.text_scale();
            let scale = if scale.is_finite() && scale > 0. {
                scale
            } else {
                1.
            };
            let (_, _, mut computed) = self.layout_editing(width, height, scale, measurer)?;
            let pending_scroll = !self.queue.2.borrow().is_empty();
            if pending_scroll && self.apply_scroll_control(&computed)? {
                (_, _, computed) = self.layout_editing(width, height, scale, measurer)?;
            }
            self.dom
                .apply_modal_layers(&mut computed.paint_order, &self.modal_layers);
            self.computed = Some(computed);
            let Some(route) = self
                .wheel_route(burst.point)
                .filter(|route| self.wheel_owners(*route) == burst.route)
            else {
                self.computed = None;
                break;
            };
            self.dispatch_wheel_route(route, sample)?;
        }
        Ok(())
    }

    fn wheel_route(&self, point: (f32, f32)) -> Option<WheelRoute> {
        let computed = self.computed.as_ref()?;
        let hit = computed
            .paint_order
            .iter()
            .rev()
            .copied()
            .find(|&id| self.scroll_region_at(computed, id, point))?;
        let horizontal = self
            .scroll_ancestors_at(computed, hit, point)
            .find(|id| self.bindings[id].horizontal_scroll.is_some());
        Some(WheelRoute {
            hit,
            horizontal,
            vertical: horizontal.and_then(|_| self.vertical_wheel_target(computed, hit, point)),
        })
    }

    fn wheel_owners(&self, route: WheelRoute) -> [Option<WheelOwner>; 3] {
        [Some(route.hit), route.horizontal, route.vertical].map(|id| {
            let id = id?;
            let node = self.dom.get(id).ok()?;
            let binding = self.bindings.get(&id)?;
            Some(WheelOwner {
                key: self.stable(id)?,
                role: node.role(),
                enabled: !node.is_disabled(),
                observes_horizontal: binding.horizontal_scroll.is_some(),
                observes_vertical: binding.vertical_scroll.is_some(),
            })
        })
    }

    fn dispatch_wheel_route(
        &mut self,
        route: WheelRoute,
        sample: WheelSample,
    ) -> Result<bool, AppError> {
        let target = route.hit;
        let delta = sample.delta;
        if let Some(horizontal) = route.horizontal {
            return self.dispatch_split_wheel(horizontal, route.vertical, sample);
        }

        let computed = self.computed.as_ref().expect("wheel has current geometry");
        let (max_x, max_y) = self.wheel_limits(computed, target);
        let scroll_x = self.dom.get(target)?.style().scroll_x;
        let (y, message) = self.vertical_wheel(target, delta.1, max_y);
        self.save_scroll_offsets(target, ((scroll_x + delta.0).clamp(0., max_x), y))?;
        self.computed = None;
        self.ime_area = None;
        self.message(message)?;
        Ok(true)
    }

    fn dispatch_split_wheel(
        &mut self,
        horizontal: NodeId,
        vertical: Option<NodeId>,
        sample: WheelSample,
    ) -> Result<bool, AppError> {
        let computed = self.computed.as_ref().expect("wheel has current geometry");
        let delta = sample.delta;
        let horizontal_result = (delta.0 != 0. && !self.dom.get(horizontal)?.is_disabled())
            .then(|| self.horizontal_wheel(computed, horizontal, delta.0, sample.now_ms));
        let vertical_result = if delta.1 != 0. {
            vertical.map(|id| {
                let (offset, message) =
                    self.vertical_wheel(id, delta.1, self.wheel_limits(computed, id).1);
                (id, offset, message)
            })
        } else {
            None
        };
        let handled = horizontal_result.is_some() || vertical_result.is_some();
        let mut messages = [None, None];
        if let Some((offset, message)) = horizontal_result {
            if let Some(x) = offset {
                let y = self.dom.get(horizontal)?.style().scroll_y;
                self.save_scroll_offsets(horizontal, (x, y))?;
            }
            messages[0] = message;
        }
        if let Some((id, y, message)) = vertical_result {
            let style = self.dom.get(id)?.style();
            if y != style.scroll_y {
                self.save_scroll_offsets(id, (style.scroll_x, y))?;
            }
            messages[1] = message;
        }
        if handled {
            self.computed = None;
            self.ime_area = None;
        }
        if messages.iter().any(Option::is_some) {
            for message in messages.into_iter().flatten() {
                self.queue.post(message);
            }
            self.flush()?;
        }
        Ok(handled)
    }

    pub(crate) fn scroll_region_at(
        &self,
        computed: &ComputedDom,
        id: NodeId,
        point: (f32, f32),
    ) -> bool {
        let node = self.dom.get(id).expect("valid wheel node");
        let geometry = &computed.nodes[id.index()];
        node.style().clip
            && self.in_scope(id)
            && node.stable_id().is_some()
            && geometry
                .bounds
                .intersect(geometry.clip)
                .contains(point.0, point.1)
    }

    pub(crate) fn scroll_ancestors_at<'a>(
        &'a self,
        computed: &'a ComputedDom,
        start: NodeId,
        point: (f32, f32),
    ) -> impl Iterator<Item = NodeId> + 'a {
        std::iter::successors(Some(start), |id| self.dom.get(*id).ok()?.parent())
            .filter(move |&id| self.scroll_region_at(computed, id, point))
    }

    pub(crate) fn vertical_wheel_target(
        &self,
        computed: &ComputedDom,
        target: NodeId,
        point: (f32, f32),
    ) -> Option<NodeId> {
        self.scroll_ancestors_at(computed, target, point)
            .find(|id| {
                let binding = &self.bindings[id];
                self.has_vertical_scroll_owner(*id)
                    || (binding.horizontal_scroll.is_none()
                        && self.wheel_limits(computed, *id).1 > 0.)
            })
    }

    pub(crate) fn has_vertical_scroll_owner(&self, id: NodeId) -> bool {
        self.bindings
            .get(&id)
            .is_some_and(|binding| binding.vertical_scroll.is_some())
            && self
                .dom
                .get(id)
                .is_ok_and(|node| node.style().clip && !node.is_disabled())
    }

    fn wheel_limits(&self, computed: &ComputedDom, id: NodeId) -> (f32, f32) {
        let bounds = computed.nodes[id.index()].bounds;
        let style = self.dom.get(id).expect("valid wheel node").style();
        let (width, height) =
            self.dom
                .children(id)
                .fold((bounds.width, bounds.height), |(width, height), child| {
                    let r = computed.nodes[child.index()].bounds;
                    (
                        width.max(r.x + r.width - bounds.x + style.scroll_x + style.padding),
                        height.max(r.y + r.height - bounds.y + style.scroll_y + style.padding),
                    )
                });
        (
            (width - bounds.width).max(0.),
            (height - bounds.height).max(0.),
        )
    }

    fn horizontal_wheel(
        &self,
        computed: &ComputedDom,
        id: NodeId,
        delta: f32,
        now_ms: Option<f64>,
    ) -> (Option<f32>, Option<A::Message>) {
        let offset = self.dom.get(id).expect("valid wheel node").style().scroll_x;
        let max_offset = self.wheel_limits(computed, id).0;
        let scroll = self.bindings[&id]
            .horizontal_scroll
            .as_ref()
            .expect("marked wheel node");
        let input = HorizontalScroll {
            delta,
            offset,
            max_offset,
            now_ms,
        };
        match scroll(input, self.focused()) {
            Some(response) => (
                response
                    .offset
                    .filter(|value| value.is_finite())
                    .map(|value| value.clamp(0., max_offset)),
                Some(response.message),
            ),
            None => (Some((offset + delta).clamp(0., max_offset)), None),
        }
    }

    fn vertical_wheel(&self, id: NodeId, delta: f32, max_offset: f32) -> (f32, Option<A::Message>) {
        let node = self.dom.get(id).expect("valid wheel node");
        let offset = node.style().scroll_y;
        let response = if delta != 0. && !node.is_disabled() {
            self.bindings[&id]
                .vertical_scroll
                .as_ref()
                .and_then(|scroll| {
                    scroll(
                        VerticalScroll {
                            delta,
                            offset,
                            max_offset,
                        },
                        self.focused(),
                    )
                })
        } else {
            None
        };
        let consumed = response
            .as_ref()
            .map(|response| response.consumed)
            .filter(|consumed| consumed.is_finite())
            .unwrap_or(0.)
            .clamp(delta.min(0.), delta.max(0.));
        (
            (offset + (delta - consumed)).clamp(0., max_offset),
            response.map(|response| response.message),
        )
    }

    pub(crate) fn save_scroll_offsets(
        &mut self,
        id: NodeId,
        offset: (f32, f32),
    ) -> Result<(), AppError> {
        let key = self.stable(id).expect("scroll node has ID");
        self.scroll.insert(key, offset);
        let mut style = self.dom.get(id)?.style().clone();
        style.scroll_x = offset.0;
        style.scroll_y = offset.1;
        self.dom.set_style(id, style)?;
        Ok(())
    }

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
            let style = self.dom.get(node)?.style().clone();
            let bounds = computed.nodes[node.index()].bounds;
            let content =
                scroll_extent(&self.dom, computed, node).unwrap_or((bounds.width, bounds.height));
            let max = (
                (content.0 - bounds.width).max(0.),
                (content.1 - bounds.height).max(0.),
            );
            let mut offset = (style.scroll_x, style.scroll_y);
            let mut requested = false;
            for (_, request) in requests.iter().filter(|(id, _)| *id == key) {
                offset = match request {
                    Request::Position(ScrollTarget::Top) => (offset.0, 0.),
                    Request::Position(ScrollTarget::Bottom) => (offset.0, max.1),
                    Request::Position(ScrollTarget::Offset { x, y }) => (*x, *y),
                    Request::ChildTop { child, top } => {
                        let Some(child) = self.dom.by_id(child) else {
                            continue;
                        };
                        if !self.scroll_ancestors(child).contains(&node) {
                            continue;
                        }
                        let child = computed.nodes[child.index()].bounds;
                        let y = style.scroll_y + (child.y - bounds.y) - top;
                        if !y.is_finite() {
                            continue;
                        }
                        (offset.0, y)
                    }
                };
                requested = true;
            }
            let tracked = stick || self.scroll.contains_key(&key) || requested;
            let mut stuck = false;
            if tracked {
                if stick {
                    stuck = if requested {
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
