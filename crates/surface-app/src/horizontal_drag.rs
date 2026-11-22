use crate::{
    drag_motion::{Velocity, SLOP},
    AppError, Application, Runtime,
};
use std::rc::Rc;
use surface::{NodeId, Rect, Role};

/// A horizontal gesture in logical pixels. Positive displacement and velocity
/// point toward the physical right. Velocity is in logical pixels per second.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum HorizontalDrag {
    Move { delta_x: f32 },
    End { delta_x: f32, velocity_x: f32 },
    Cancel,
}

/// Identity of one logical drag binding. Keep it across preview rebuilds.
/// Use a new session when a different operation reuses the same stable ID.
#[derive(Clone, Debug, Default)]
pub struct HorizontalDragSession(Rc<()>);

impl HorizontalDragSession {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PartialEq for HorizontalDragSession {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for HorizontalDragSession {}

pub(crate) struct Binding<M> {
    pub(crate) session: HorizontalDragSession,
    pub(crate) update: Rc<dyn Fn(HorizontalDrag, Rect) -> M>,
}

pub(crate) struct Tracking {
    id: String,
    role: Role,
    scope: Option<String>,
    session: HorizontalDragSession,
    start: (f32, f32),
    bounds: Rect,
    velocity: Velocity,
    claimed: bool,
}

impl<A: Application> Runtime<A> {
    pub(crate) fn in_horizontal_drag_scope(&self, id: NodeId) -> bool {
        self.dom.get(id).is_ok_and(|node| !node.is_disabled())
            && (self.in_scope(id)
                || (self.has_interaction_binding(id)
                    && self
                        .active_scope_root()
                        .is_some_and(|root| self.modal_layers.contains(&(Some(id), root)))))
    }

    pub(crate) fn horizontal_drag_candidate(
        &self,
        hit: Option<NodeId>,
        x: f32,
        y: f32,
    ) -> Option<NodeId> {
        if !self.pointer_geometry_current || !x.is_finite() || !y.is_finite() {
            return None;
        }
        // Text and sliders retain their existing pointer interactions.
        if hit.is_some_and(|id| {
            self.dom
                .get(id)
                .is_ok_and(|node| matches!(node.role(), Role::TextInput | Role::Slider))
        }) {
            return None;
        }
        let layout = self.computed.as_ref()?;
        let contains = |id: NodeId| {
            self.bindings
                .get(&id)
                .is_some_and(|binding| binding.horizontal_drag.is_some())
                && self.in_horizontal_drag_scope(id)
                && layout.nodes.get(id.index()).is_some_and(|geometry| {
                    let visible = geometry
                        .bounds
                        .intersect(geometry.clip)
                        .intersect(layout.viewport);
                    visible.width > 0. && visible.height > 0. && visible.contains(x, y)
                })
        };
        for id in layout.paint_order.iter().rev().copied() {
            if contains(id) {
                return Some(id);
            }
            if Some(id) == hit {
                // An overlapping region below the hit control cannot steal
                // it. An opted-in ancestor can negotiate with its own child.
                let mut parent = self.dom.get(id).ok()?.parent();
                while let Some(id) = parent {
                    if contains(id) {
                        return Some(id);
                    }
                    parent = self.dom.get(id).ok()?.parent();
                }
                break;
            }
        }
        None
    }

    pub(crate) fn begin_horizontal_drag(&mut self, id: NodeId, x: f32, y: f32) {
        let node = self.dom.get(id).expect("drag candidate exists");
        let binding = self.bindings[&id]
            .horizontal_drag
            .as_ref()
            .expect("drag candidate has binding");
        let bounds = self
            .computed
            .as_ref()
            .expect("drag candidate has current geometry")
            .nodes[id.index()]
        .bounds;
        self.horizontal_drag = Some(Tracking {
            id: node
                .stable_id()
                .expect("horizontal drag requires a stable ID")
                .to_owned(),
            role: node.role(),
            scope: self.drag_scope().map(str::to_owned),
            session: binding.session.clone(),
            start: (x, y),
            bounds,
            velocity: Velocity::new(x, self.input_time_ms()),
            claimed: false,
        });
    }

    fn horizontal_drag_binding(&self, drag: &Tracking) -> Option<(NodeId, &Binding<A::Message>)> {
        let id = self.dom.by_id(&drag.id)?;
        let node = self.dom.get(id).ok()?;
        let binding = self.bindings.get(&id)?.horizontal_drag.as_ref()?;
        (node.role() == drag.role && binding.session == drag.session).then_some((id, binding))
    }

    fn active_horizontal_drag_binding(&self, drag: &Tracking) -> Option<&Binding<A::Message>> {
        let (id, binding) = self.horizontal_drag_binding(drag)?;
        (self.in_horizontal_drag_scope(id) && self.drag_scope() == drag.scope.as_deref())
            .then_some(binding)
    }

    pub(crate) fn update_horizontal_drag(
        &mut self,
        x: f32,
        y: f32,
        release: bool,
    ) -> Result<bool, AppError> {
        let Some(mut drag) = self.horizontal_drag.take() else {
            return Ok(false);
        };
        let dx = x - drag.start.0;
        let dy = y - drag.start.1;
        if !dx.is_finite() || !dy.is_finite() {
            return self.finish_horizontal_cancel(drag);
        }
        let Some(update) = self
            .active_horizontal_drag_binding(&drag)
            .map(|binding| binding.update.clone())
        else {
            return self.finish_horizontal_cancel(drag);
        };
        if !drag.claimed {
            if dx.abs().max(dy.abs()) < SLOP {
                if !release {
                    drag.velocity.sample(x, self.input_time_ms());
                    self.horizontal_drag = Some(drag);
                }
                return Ok(false);
            }
            self.retire_horizontal_tap();
            if dx.abs() <= dy.abs() {
                return Ok(false);
            }
            self.hovered = None;
            drag.claimed = true;
        }
        let velocity_x = drag.velocity.sample(x, self.input_time_ms());
        let bounds = drag.bounds;
        let event = if release {
            HorizontalDrag::End {
                delta_x: dx,
                velocity_x,
            }
        } else {
            self.horizontal_drag = Some(drag);
            HorizontalDrag::Move { delta_x: dx }
        };
        self.message(Some(update(event, bounds)))
    }

    fn retire_horizontal_tap(&mut self) {
        self.pressed = None;
        self.modal_outside = false;
    }

    pub(crate) fn discard_horizontal_drag(&mut self) {
        if self.horizontal_drag.take().is_some() {
            self.retire_horizontal_tap();
        }
    }

    pub(crate) fn cancel_horizontal_drag(&mut self) -> Result<bool, AppError> {
        let Some(drag) = self.horizontal_drag.take() else {
            return Ok(false);
        };
        self.finish_horizontal_cancel(drag)
    }

    fn finish_horizontal_cancel(&mut self, drag: Tracking) -> Result<bool, AppError> {
        self.retire_horizontal_tap();
        if drag.claimed {
            // A matching session can roll its preview back after leaving a
            // scope. A replacement must not receive the old gesture.
            if let Some(message) = self
                .horizontal_drag_binding(&drag)
                .map(|(_, binding)| (binding.update)(HorizontalDrag::Cancel, drag.bounds))
            {
                self.message(Some(message))?;
            }
        }
        Ok(true)
    }

    pub(crate) fn reconcile_horizontal_drag_binding(&mut self) -> Result<(), AppError> {
        if self
            .horizontal_drag
            .as_ref()
            .is_some_and(|drag| self.active_horizontal_drag_binding(drag).is_none())
        {
            self.cancel_horizontal_drag()?;
        }
        Ok(())
    }
}
