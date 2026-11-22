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

/// A vertical gesture in logical pixels. Positive displacement and velocity
/// point down. Velocity is in logical pixels per second.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum VerticalDrag {
    Move { delta_y: f32 },
    End { delta_y: f32, velocity_y: f32 },
    Cancel,
}

/// Identity of one logical drag binding. Keep it across preview rebuilds.
/// Use a new session when a different operation reuses the same stable ID.
#[derive(Clone, Debug, Default)]
pub struct VerticalDragSession(Rc<()>);

impl VerticalDragSession {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PartialEq for VerticalDragSession {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for VerticalDragSession {}

#[derive(Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    const ALL: [Self; 2] = [Self::Horizontal, Self::Vertical];

    fn component(self, x: f32, y: f32) -> f32 {
        match self {
            Self::Horizontal => x,
            Self::Vertical => y,
        }
    }

    fn dominant(dx: f32, dy: f32) -> Option<Self> {
        if dx.abs() > dy.abs() {
            Some(Self::Horizontal)
        } else if dy.abs() > dx.abs() {
            Some(Self::Vertical)
        } else {
            None
        }
    }
}

enum Drag {
    Move { delta: f32 },
    End { delta: f32, velocity: f32 },
    Cancel,
}

struct Binding<M> {
    session: Rc<()>,
    update: Rc<dyn Fn(Drag, Rect) -> M>,
}

pub(crate) struct Bindings<M>([Option<Binding<M>>; 2]);

impl<M> Default for Bindings<M> {
    fn default() -> Self {
        Self([None, None])
    }
}

impl<M> Bindings<M> {
    pub(crate) fn set_horizontal(
        &mut self,
        session: HorizontalDragSession,
        update: impl Fn(HorizontalDrag, Rect) -> M + 'static,
    ) {
        self.0[Axis::Horizontal as usize] = Some(Binding {
            session: session.0,
            update: Rc::new(move |drag, bounds| {
                let event = match drag {
                    Drag::Move { delta } => HorizontalDrag::Move { delta_x: delta },
                    Drag::End { delta, velocity } => HorizontalDrag::End {
                        delta_x: delta,
                        velocity_x: velocity,
                    },
                    Drag::Cancel => HorizontalDrag::Cancel,
                };
                update(event, bounds)
            }),
        });
    }

    pub(crate) fn set_vertical(
        &mut self,
        session: VerticalDragSession,
        update: impl Fn(VerticalDrag, Rect) -> M + 'static,
    ) {
        self.0[Axis::Vertical as usize] = Some(Binding {
            session: session.0,
            update: Rc::new(move |drag, bounds| {
                let event = match drag {
                    Drag::Move { delta } => VerticalDrag::Move { delta_y: delta },
                    Drag::End { delta, velocity } => VerticalDrag::End {
                        delta_y: delta,
                        velocity_y: velocity,
                    },
                    Drag::Cancel => VerticalDrag::Cancel,
                };
                update(event, bounds)
            }),
        });
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.iter().all(Option::is_none)
    }

    fn get(&self, axis: Axis) -> Option<&Binding<M>> {
        self.0[axis as usize].as_ref()
    }
}

struct AxisMotion {
    session: Rc<()>,
    velocity: Velocity,
}

pub(crate) struct Tracking {
    id: String,
    role: Role,
    scope: Option<String>,
    start: (f32, f32),
    bounds: Rect,
    motion: [Option<AxisMotion>; 2],
    claimed: Option<Axis>,
}

impl<A: Application> Runtime<A> {
    pub(crate) fn in_drag_scope(&self, id: NodeId) -> bool {
        self.dom.get(id).is_ok_and(|node| !node.is_disabled())
            && (self.in_scope(id)
                || (self.has_interaction_binding(id)
                    && self
                        .active_scope_root()
                        .is_some_and(|root| self.modal_layers.contains(&(Some(id), root)))))
    }

    pub(crate) fn drag_candidate(&self, hit: Option<NodeId>, x: f32, y: f32) -> Option<NodeId> {
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
                .is_some_and(|binding| !binding.drag_bindings.is_empty())
                && self.in_drag_scope(id)
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

    pub(crate) fn begin_drag(&mut self, id: NodeId, x: f32, y: f32) {
        let node = self.dom.get(id).expect("drag candidate exists");
        let bindings = &self.bindings[&id].drag_bindings;
        let bounds = self
            .computed
            .as_ref()
            .expect("drag candidate has current geometry")
            .nodes[id.index()]
        .bounds;
        let now = self.input_time_ms();
        self.captured_drag = Some(Tracking {
            id: node
                .stable_id()
                .expect("captured drag requires a stable ID")
                .to_owned(),
            role: node.role(),
            scope: self.drag_scope().map(str::to_owned),
            start: (x, y),
            bounds,
            motion: Axis::ALL.map(|axis| {
                bindings.get(axis).map(|binding| AxisMotion {
                    session: binding.session.clone(),
                    velocity: Velocity::new(axis.component(x, y), now),
                })
            }),
            claimed: None,
        });
    }

    fn matching_drag_binding(
        &self,
        drag: &Tracking,
        axis: Axis,
    ) -> Option<(NodeId, &Binding<A::Message>)> {
        let motion = drag.motion[axis as usize].as_ref()?;
        let id = self.dom.by_id(&drag.id)?;
        let node = self.dom.get(id).ok()?;
        let binding = self.bindings.get(&id)?.drag_bindings.get(axis)?;
        (node.role() == drag.role && Rc::ptr_eq(&binding.session, &motion.session))
            .then_some((id, binding))
    }

    fn active_drag_binding(&self, drag: &Tracking, axis: Axis) -> Option<&Binding<A::Message>> {
        let (id, binding) = self.matching_drag_binding(drag, axis)?;
        (self.in_drag_scope(id) && self.drag_scope() == drag.scope.as_deref()).then_some(binding)
    }

    fn drag_still_active(&self, drag: &Tracking) -> bool {
        match drag.claimed {
            Some(axis) => self.active_drag_binding(drag, axis).is_some(),
            None => Axis::ALL
                .into_iter()
                .any(|axis| self.active_drag_binding(drag, axis).is_some()),
        }
    }

    pub(crate) fn update_drag(&mut self, x: f32, y: f32, release: bool) -> Result<bool, AppError> {
        let Some(mut drag) = self.captured_drag.take() else {
            return Ok(false);
        };
        let dx = x - drag.start.0;
        let dy = y - drag.start.1;
        if !dx.is_finite() || !dy.is_finite() || !self.drag_still_active(&drag) {
            return self.finish_drag_cancel(drag);
        }
        let now = self.input_time_ms();
        let axis = if let Some(axis) = drag.claimed {
            axis
        } else {
            if dx.abs().max(dy.abs()) < SLOP {
                if !release {
                    for axis in Axis::ALL {
                        if let Some(motion) = &mut drag.motion[axis as usize] {
                            motion.velocity.sample(axis.component(x, y), now);
                        }
                    }
                    self.captured_drag = Some(drag);
                }
                return Ok(false);
            }
            self.retire_drag_tap();
            let Some(axis) = Axis::dominant(dx, dy) else {
                return Ok(false);
            };
            axis
        };
        let Some(update) = self
            .active_drag_binding(&drag, axis)
            .map(|binding| binding.update.clone())
        else {
            return Ok(false);
        };
        if drag.claimed.is_none() {
            self.hovered = None;
            drag.claimed = Some(axis);
        }
        let velocity = drag.motion[axis as usize]
            .as_mut()
            .expect("matched drag has motion")
            .velocity
            .sample(axis.component(x, y), now);
        let delta = axis.component(dx, dy);
        let bounds = drag.bounds;
        let event = if release {
            Drag::End { delta, velocity }
        } else {
            self.captured_drag = Some(drag);
            Drag::Move { delta }
        };
        self.message(Some(update(event, bounds)))
    }

    fn retire_drag_tap(&mut self) {
        self.pressed = None;
        self.modal_outside = false;
    }

    pub(crate) fn discard_drag(&mut self) {
        if self.captured_drag.take().is_some() {
            self.retire_drag_tap();
        }
    }

    pub(crate) fn cancel_drag(&mut self) -> Result<bool, AppError> {
        let Some(drag) = self.captured_drag.take() else {
            return Ok(false);
        };
        self.finish_drag_cancel(drag)
    }

    fn finish_drag_cancel(&mut self, drag: Tracking) -> Result<bool, AppError> {
        self.retire_drag_tap();
        if let Some(axis) = drag.claimed {
            // A matching session can roll its preview back after leaving a
            // scope. A replacement must not receive the old gesture.
            if let Some(message) = self
                .matching_drag_binding(&drag, axis)
                .map(|(_, binding)| (binding.update)(Drag::Cancel, drag.bounds))
            {
                self.message(Some(message))?;
            }
        }
        Ok(true)
    }

    pub(crate) fn reconcile_drag_binding(&mut self) -> Result<(), AppError> {
        if self
            .captured_drag
            .as_ref()
            .is_some_and(|drag| !self.drag_still_active(drag))
        {
            self.cancel_drag()?;
        }
        Ok(())
    }
}
