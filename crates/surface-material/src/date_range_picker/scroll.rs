//! Bounded month mounting and preservation of a measured month on a roll.
use surface::{scroll_extent, Rect};
use surface_app::{AnimationFrame, AnimationUpdate, Queue, ScrollTarget, VerticalScroll};

use crate::{DateBounds, YearMonth};

const MAX_MONTHS: usize = 5;
const PREFETCH: f32 = 48.;

#[derive(Clone, Copy)]
struct Anchor {
    month: YearMonth,
    top: f32,
    extra: f32,
}

#[derive(Clone, Copy, Default)]
struct Geometry {
    offset: (f32, f32),
    max: (f32, f32),
}

struct DragMotion {
    delta: f32,
    horizontal: bool,
    ended: bool,
}

pub(super) struct CalendarScroll {
    months: Vec<YearMonth>,
    anchor: Option<Anchor>,
    geometry: Option<Geometry>,
    request: Option<ScrollTarget>,
    roll: Option<Anchor>,
    wheel: Option<VerticalScroll>,
    drag_origin: Option<f32>,
    drag_motion: Option<DragMotion>,
    rebase_drag: Option<f32>,
    logical_start: bool,
}

impl CalendarScroll {
    pub(super) fn new(month: YearMonth, bounds: DateBounds) -> Self {
        let mut scroll = Self {
            months: Vec::with_capacity(MAX_MONTHS),
            anchor: None,
            geometry: None,
            request: None,
            roll: None,
            wheel: None,
            drag_origin: None,
            drag_motion: None,
            rebase_drag: None,
            logical_start: true,
        };
        scroll.reset(month, bounds);
        scroll
    }

    pub(super) fn months(&self) -> &[YearMonth] {
        &self.months
    }

    pub(super) fn reset(&mut self, month: YearMonth, bounds: DateBounds) {
        let month = month.clamp(bounds.min().year_month(), bounds.max().year_month());
        self.months.clear();
        self.months.extend(
            (-1..=1).filter_map(|step| month.step(step).filter(|m| bounds.contains_month(*m))),
        );
        self.geometry = None;
        self.wheel = None;
        self.cancel_drag();
        self.request = None;
        self.roll = None;
        self.logical_start = true;
        self.show_month(month);
    }

    pub(super) fn mount_for_focus(&mut self, month: YearMonth, bounds: DateBounds) {
        if !self.months.contains(&month) {
            self.reset(month, bounds);
        }
        // Key navigation already asks Runtime to reveal the target.
        self.anchor = None;
        self.logical_start = false;
    }

    fn show_month(&mut self, month: YearMonth) {
        self.anchor = Some(Anchor {
            month,
            top: 0.,
            extra: 0.,
        });
    }

    pub(super) fn browse_to(&mut self, month: YearMonth) {
        self.anchor = None;
        self.logical_start = false;
        self.roll = Some(Anchor {
            month,
            top: 0.,
            extra: 0.,
        });
    }

    pub(super) fn reset_input_scroll(&mut self) {
        self.geometry = None;
        self.cancel_drag();
        self.request = Some(ScrollTarget::Offset { x: 0., y: 0. });
    }

    pub(super) fn flush_requests<M>(&mut self, id: &str, queue: &Queue<M>) {
        let body = format!("{id}-body");
        if let Some(request) = self.request.take() {
            queue.scroll_to(&body, request);
        }
        if let Some(roll) = self.roll.take() {
            queue.scroll_to_child_top(&body, &month_id(id, roll.month), roll.top - roll.extra);
        }
    }

    pub(super) fn wheel(&mut self, wheel: VerticalScroll) {
        self.wheel = Some(wheel);
    }

    pub(super) fn drag(&mut self, delta: f32, horizontal: bool, ended: bool) {
        self.drag_motion = Some(DragMotion {
            delta,
            horizontal,
            ended,
        });
    }

    fn measure_drag(&mut self, geometry: Geometry) -> Option<f32> {
        let (x, y) = geometry.offset;
        if let (Some(origin), Some(previous)) = (&mut self.drag_origin, self.rebase_drag.take()) {
            *origin += y - previous;
        }
        let DragMotion {
            delta,
            horizontal,
            ended,
        } = self.drag_motion.take()?;
        let origin = if horizontal { x } else { y };
        let offset = *self.drag_origin.get_or_insert(origin) - delta;
        let (next_x, next_y) = if horizontal {
            (offset.clamp(0., geometry.max.0), y)
        } else {
            (x, offset.clamp(0., geometry.max.1))
        };
        self.request = Some(ScrollTarget::Offset {
            x: next_x,
            y: next_y,
        });
        if ended {
            self.drag_origin = None;
        }
        (!horizontal).then_some(offset - y)
    }

    pub(super) fn cancel_drag(&mut self) {
        self.drag_origin = None;
        self.drag_motion = None;
        self.rebase_drag = None;
    }

    pub(super) fn observe_editor(&mut self, id: &str, frame: &AnimationFrame<'_>) -> bool {
        if self.drag_motion.is_none() {
            return false;
        }
        let Some((_, geometry)) = body_geometry(id, frame) else {
            return frame.layout.is_none();
        };
        self.measure_drag(geometry);
        true
    }

    pub(super) fn observe(
        &mut self,
        id: &str,
        frame: &AnimationFrame<'_>,
        bounds: DateBounds,
        rtl: bool,
        focused: Option<&str>,
    ) -> (AnimationUpdate, Option<YearMonth>) {
        let Some(layout) = frame.layout else {
            return (next_frame(false), None);
        };
        let Some((body, geometry)) = body_geometry(id, frame) else {
            return (AnimationUpdate::default(), None);
        };
        let previous = self.geometry.replace(geometry);
        let rect = |month| {
            let key = month_id(id, month);
            frame
                .dom
                .by_id(&key)
                .map(|node| layout.nodes[node.index()].bounds)
        };
        let (x, y) = geometry.offset;
        let x = if std::mem::take(&mut self.logical_start) {
            if rtl {
                geometry.max.0
            } else {
                0.
            }
        } else if rtl
            && previous
                .is_some_and(|p| (p.offset.0 - p.max.0).abs() <= 0.5 && p.max.0 != geometry.max.0)
        {
            geometry.max.0
        } else {
            x
        };

        if let Some(anchor) = self.anchor.take() {
            let key = month_id(id, anchor.month);
            if let Some(month) = frame.dom.by_id(&key) {
                let top =
                    y + layout.nodes[month.index()].bounds.y - body.y - anchor.top + anchor.extra;
                // Keep Runtime's focus reveal when aligning the month.
                let focus = focused
                    .and_then(|id| frame.dom.by_id(id))
                    .filter(|node| {
                        std::iter::successors(Some(*node), |id| frame.dom.get(*id).ok()?.parent())
                            .any(|id| id == month)
                    })
                    .map(|node| layout.nodes[node.index()].bounds);
                let (x, top) = focus.map_or((x, top), |r| {
                    let left = geometry.offset.0 + r.x - body.x;
                    let x = x
                        .max(left + r.width - body.width)
                        .min(left)
                        .clamp(0., geometry.max.0);
                    let top = top.max(y + r.y + r.height - body.y - body.height);
                    (x, top)
                });
                let top = top.clamp(0., geometry.max.1);
                self.request = Some(ScrollTarget::Offset { x, y: top });
                return (next_frame(false), None);
            }
        }

        let rolled = self.rebase_drag.is_some();
        let drag = self.measure_drag(geometry);

        let Some((visible, visible_rect)) = self.months.iter().copied().find_map(|month| {
            let rect = rect(month)?;
            (rect.y + rect.height > body.y + 0.5 && rect.y < body.y + body.height)
                .then_some((month, rect))
        }) else {
            return (AnimationUpdate::default(), None);
        };

        let movement = previous.filter(|_| !rolled).map_or(0., |p| y - p.offset.1);
        let wheel = self.wheel.take();
        // Wheel and captured drags can give a direction even at the old limit.
        let direction = wheel.map_or(drag.unwrap_or(movement), |event| event.delta);
        let extra = wheel.map_or(drag.unwrap_or(0.), |event| {
            let end = (event.offset + event.delta).clamp(0., event.max_offset);
            event.delta - (end - event.offset)
        });
        let earlier = direction < -0.5 && y < PREFETCH;
        let later = direction > 0.5 && y + PREFETCH > geometry.max.1;
        if (earlier || later) && self.extend(visible, bounds, earlier) {
            self.roll = Some(Anchor {
                month: visible,
                top: visible_rect.y - body.y,
                extra,
            });
            self.rebase_drag = Some(y + extra);
            return (next_frame(true), Some(visible));
        }

        if self.request.is_some() {
            return (next_frame(false), Some(visible));
        }
        if x != geometry.offset.0 {
            self.request = Some(ScrollTarget::Offset { x, y });
            return (next_frame(false), Some(visible));
        }
        (AnimationUpdate::default(), Some(visible))
    }

    fn extend(&mut self, visible: YearMonth, bounds: DateBounds, earlier: bool) -> bool {
        let edge = if earlier {
            self.months.first()
        } else {
            self.months.last()
        };
        let Some(month) = edge
            .and_then(|month| month.step(if earlier { -1 } else { 1 }))
            .filter(|month| bounds.contains_month(*month))
        else {
            return false;
        };
        if self.months.len() == MAX_MONTHS {
            let remove = if earlier { MAX_MONTHS - 1 } else { 0 };
            if self.months[remove] == visible {
                return false;
            }
            self.months.remove(remove);
        }
        if earlier {
            self.months.insert(0, month);
        } else {
            self.months.push(month);
        }
        true
    }
}

fn body_geometry(id: &str, frame: &AnimationFrame<'_>) -> Option<(Rect, Geometry)> {
    let layout = frame.layout?;
    let body = frame.dom.by_id(&format!("{id}-body"))?;
    let style = frame.dom.get(body).ok()?.style();
    let bounds = layout.nodes[body.index()].bounds;
    let content = scroll_extent(frame.dom, layout, body).unwrap_or((bounds.width, bounds.height));
    let geometry = Geometry {
        offset: (style.scroll_x, style.scroll_y),
        max: (
            (content.0 - bounds.width).max(0.),
            (content.1 - bounds.height).max(0.),
        ),
    };
    Some((bounds, geometry))
}

fn next_frame(rebuild: bool) -> AnimationUpdate {
    AnimationUpdate {
        rebuild,
        request_next: true,
    }
}

pub(super) fn month_id(id: &str, month: YearMonth) -> String {
    format!("{id}-month-{:04}-{:02}", month.year(), month.month())
}
