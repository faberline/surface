use std::{collections::HashSet, rc::Rc};

use surface::Rect;
use surface_app::{
    AnimationFrame, AnimationUpdate, PointDrag, PointDragSession, Queue, ScrollTarget, View,
};

use super::{
    can_step, geometry::Geometry, vertical_drag, Input, LayoutDirection, ListReorderEntry,
    ListReorderInput, ListReorderRequest, ListStyle, MoveDirection, ReorderPreview,
    ReorderableListOptions,
};

const EDGE_BAND: f32 = 40.;
const EDGE_SPEED: f32 = 640.;

#[derive(PartialEq)]
struct Context {
    list: String,
    scroller: String,
    style: ListStyle,
    direction: LayoutDirection,
    width: f32,
    revision: u64,
}

impl Context {
    fn new(options: ReorderableListOptions<'_>) -> Self {
        Self {
            list: options.id.to_owned(),
            scroller: options.scroller_id.to_owned(),
            style: options.style,
            direction: options.direction,
            width: options.available_width,
            revision: options.revision,
        }
    }

    fn matches(&self, options: ReorderableListOptions<'_>) -> bool {
        self.list == options.id
            && self.scroller == options.scroller_id
            && self.style == options.style
            && self.direction == options.direction
            && self.width == options.available_width
            && self.revision == options.revision
    }
}

#[derive(Clone)]
pub(super) struct Pointer {
    context: Rc<Context>,
    item: Rc<str>,
    event: PointDrag,
    geometry: Option<Rc<Geometry>>,
}

struct Active {
    context: Rc<Context>,
    item: Rc<str>,
    position: (f32, f32),
    grab_y: f32,
    visible: Rect,
    edge_time_ms: Option<f64>,
}

/// Retained pointer and preview state for the named reorderable list.
/// The application owns the order. Keep one instance per list.
#[derive(Default)]
pub struct ListReorderState {
    session: PointDragSession,
    active: Option<Active>,
    preview: Option<ReorderPreview>,
}

impl ListReorderState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drop any pending preview and retire a held grip. Returns whether the
    /// preview changed. On a route or model change, rebuild the list if mounted.
    pub fn cancel(&mut self) -> bool {
        self.session = PointDragSession::new();
        self.active = None;
        self.preview.take().is_some()
    }

    /// Map a view event to at most one proposed move. Apply accepted requests
    /// atomically, update the order revision, and announce the new position.
    /// The input order and options must describe the rows in the current view.
    pub fn update(
        &mut self,
        input: ListReorderInput,
        revision: u64,
        order: &[ListReorderEntry<'_>],
    ) -> Option<ListReorderRequest> {
        if !unique_order(order) {
            self.cancel();
            return None;
        }
        match input.0 {
            Input::Step {
                revision: authored,
                item,
                direction,
            } => {
                self.cancel();
                if authored != revision {
                    return None;
                }
                let index = order.iter().position(|row| row.id == item)?;
                if !can_step(order, index, direction) {
                    return None;
                }
                let before = match direction {
                    MoveDirection::Up => index - 1,
                    MoveDirection::Down => index + 2,
                };
                Some(request(order, index, before))
            }
            Input::Pointer(pointer) => self.pointer(pointer, revision, order),
        }
    }

    /// Advance preview and edge scrolling from the last available frame. Pass
    /// the app's queue to scroll near the boundary while a grip is held.
    /// Keep calling this hook while the named list is mounted.
    pub fn advance<M>(
        &mut self,
        options: ReorderableListOptions<'_>,
        order: &[ListReorderEntry<'_>],
        frame: &AnimationFrame<'_>,
        queue: Option<&Queue<M>>,
    ) -> AnimationUpdate {
        let Some(active) = &mut self.active else {
            return AnimationUpdate::default();
        };
        let index = order.iter().position(|row| row.id == active.item.as_ref());
        let Some(index) = index.filter(|index| {
            active.context.matches(options)
                && can_drag(order, *index)
                && frame.dom.by_id(&format!("{}-grip", active.item)).is_some()
        }) else {
            return AnimationUpdate {
                rebuild: self.cancel(),
                request_next: false,
            };
        };
        if frame.layout.is_none() {
            let request_next = queue.is_some() && edge_speed(active.visible, active.position) != 0.;
            return AnimationUpdate {
                rebuild: false,
                request_next,
            };
        }
        let Some((mut geometry, scroll)) = Geometry::frame(options, order, frame) else {
            return AnimationUpdate {
                rebuild: self.cancel(),
                request_next: false,
            };
        };
        let mut request_next = false;
        let now = frame.now_ms.filter(|time| time.is_finite());
        let speed = edge_speed(geometry.visible, active.position);
        if let (Some(queue), Some(now)) = (queue, now) {
            let seconds = active.edge_time_ms.map_or(0., |previous| {
                ((now - previous) / 1000.).clamp(0., 0.05) as f32
            });
            let offset = (scroll.y + speed * seconds).clamp(0., scroll.max_y);
            if offset != scroll.y {
                queue.scroll_to(
                    options.scroller_id,
                    ScrollTarget::Offset {
                        x: scroll.x,
                        y: offset,
                    },
                );
                geometry.scroll_by(offset - scroll.y);
            }
            request_next = (speed < 0. && offset > 0.) || (speed > 0. && offset < scroll.max_y);
        }
        active.edge_time_ms = now.filter(|_| request_next);
        active.visible = geometry.visible;
        let destination = geometry.destination(order, index, active.position);
        let preview = geometry.preview(index, active.position, active.grab_y, destination);
        let rebuild = self.preview.as_ref() != Some(&preview);
        self.preview = Some(preview);
        AnimationUpdate {
            rebuild,
            request_next,
        }
    }

    pub(super) fn preview(&self) -> Option<&ReorderPreview> {
        self.preview.as_ref()
    }

    pub(super) fn bind_grip<M: 'static>(
        &self,
        view: View<M>,
        options: ReorderableListOptions<'_>,
        item: &str,
        measured_ids: Rc<[String]>,
        on_input: Rc<dyn Fn(ListReorderInput) -> M>,
    ) -> View<M> {
        let context = Rc::new(Context::new(options));
        let item = Rc::<str>::from(item);
        view.on_point_drag_with_layout(
            self.session.clone(),
            measured_ids,
            move |event, _, layout| {
                if matches!(event, PointDrag::Tap { .. }) {
                    return None;
                }
                let geometry = match event {
                    PointDrag::Move { .. } | PointDrag::End { .. } => {
                        Geometry::captured(layout, &context.list, &context.scroller).map(Rc::new)
                    }
                    _ => None,
                };
                Some(on_input(ListReorderInput(Input::Pointer(Pointer {
                    context: context.clone(),
                    item: item.clone(),
                    event,
                    geometry,
                }))))
            },
        )
    }

    fn pointer(
        &mut self,
        pointer: Pointer,
        revision: u64,
        order: &[ListReorderEntry<'_>],
    ) -> Option<ListReorderRequest> {
        let (origin, position, end) = match pointer.event {
            PointDrag::Move { origin, position } => (origin, position, false),
            PointDrag::End { origin, position } => (origin, position, true),
            PointDrag::Cancel => {
                self.cancel();
                return None;
            }
            _ => return None,
        };
        let index = order.iter().position(|row| row.id == pointer.item.as_ref());
        let matching_contact = self
            .active
            .as_ref()
            .filter(|state| state.context == pointer.context && state.item == pointer.item);
        let Some((index, geometry)) =
            index
                .zip(pointer.geometry.as_deref())
                .filter(|(index, geometry)| {
                    pointer.context.revision == revision
                        && geometry.matches(order)
                        && can_drag(order, *index)
                        && (self.active.is_none() || matching_contact.is_some())
                })
        else {
            self.cancel();
            return None;
        };
        if matching_contact.is_none() && !vertical_drag(pointer.event) {
            return None;
        }
        let destination = geometry.destination(order, index, position);
        if end {
            self.cancel();
            return destination.map(|before| request(order, index, before));
        }
        let (grab_y, edge_time_ms) = matching_contact.map_or_else(
            || (geometry.grab_offset(index, origin.1), None),
            |state| (state.grab_y, state.edge_time_ms),
        );
        self.preview = Some(geometry.preview(index, position, grab_y, destination));
        self.active = Some(Active {
            context: pointer.context,
            item: pointer.item,
            position,
            grab_y,
            visible: geometry.visible,
            edge_time_ms,
        });
        None
    }
}

pub(super) fn can_drag(order: &[ListReorderEntry<'_>], index: usize) -> bool {
    can_step(order, index, MoveDirection::Up) || can_step(order, index, MoveDirection::Down)
}

pub(super) fn unique_order(order: &[ListReorderEntry<'_>]) -> bool {
    let mut seen = HashSet::with_capacity(order.len());
    order
        .iter()
        .all(|row| !row.id.is_empty() && seen.insert(row.id))
}

fn request(order: &[ListReorderEntry<'_>], index: usize, before: usize) -> ListReorderRequest {
    ListReorderRequest {
        item: order[index].id.to_owned(),
        before: order.get(before).map(|row| row.id.to_owned()),
    }
}

fn edge_speed(visible: Rect, position: (f32, f32)) -> f32 {
    if !visible.contains(position.0, position.1) {
        return 0.;
    }
    let band = EDGE_BAND.min(visible.height / 2.);
    let y = position.1 - visible.y;
    let top = (1. - y / band).clamp(0., 1.);
    let bottom = (1. - (visible.height - y) / band).clamp(0., 1.);
    (bottom - top) * EDGE_SPEED
}
