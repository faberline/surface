use surface::{ComputedDom, DomError, Easing, MicroDom, Node, NodeId, Rect, Role};

use crate::{Application, FramePress, FramePressPhase, Runtime};

const GROW_MS: f64 = 225.;
const FADE_IN_MS: f64 = 75.;
const FADE_OUT_MS: f64 = 150.;
const GROW: Easing = Easing::CubicBezier(0.4, 0., 0.2, 1.);

#[derive(Default)]
pub(crate) struct PressRipple {
    wave: Option<Wave>,
    request_next: bool,
}

struct Wave {
    owner: String,
    contact: PressRippleContact,
    sequence: u64,
    center: (f32, f32),
    started_at: f64,
    released_at: Option<f64>,
}

#[derive(Clone)]
pub(crate) struct PressRippleContact {
    role: Role,
    child: Option<String>,
    bounds: Rect,
    offset: (f32, f32),
}

fn paint_bounds(computed: &ComputedDom, id: NodeId) -> Option<Rect> {
    let node = computed.nodes.get(id.index())?;
    let visible = node
        .bounds
        .intersect(node.clip)
        .intersect(computed.viewport);
    (visible.width > 0. && visible.height > 0.).then_some(node.bounds)
}

impl PressRipple {
    pub(crate) fn needs_frame(&self) -> bool {
        self.request_next
    }

    fn cancel(&mut self) -> bool {
        self.request_next = false;
        self.wave.take().is_some()
    }

    fn observe(&mut self, observed: Option<(FramePress<'_>, &PressRippleContact)>, now: f64) {
        let Some((press, contact)) = observed else {
            if self
                .wave
                .as_ref()
                .is_some_and(|wave| wave.released_at.is_none())
            {
                self.cancel();
            }
            return;
        };
        if press.phase == FramePressPhase::Cancelled {
            self.cancel();
            return;
        }
        if self
            .wave
            .as_ref()
            .is_none_or(|wave| wave.sequence != press.sequence)
        {
            let bounds = contact.bounds;
            self.wave = Some(Wave {
                owner: press.id.to_owned(),
                contact: contact.clone(),
                sequence: press.sequence,
                center: (
                    (press.position.0 - bounds.x).clamp(0., bounds.width),
                    (press.position.1 - bounds.y).clamp(0., bounds.height),
                ),
                // The host clock at pointer down can be from an idle frame.
                started_at: now,
                released_at: None,
            });
        }
        if press.phase == FramePressPhase::Released {
            if let Some(wave) = &mut self.wave {
                wave.released_at.get_or_insert(now);
            }
        }
    }

    fn present(&mut self, id: NodeId, now: f64, display: &mut MicroDom) -> Result<(), DomError> {
        let Some(wave) = &self.wave else {
            return Ok(());
        };
        let elapsed = now - wave.started_at;
        let opacity = if let Some(released) = wave.released_at {
            let fade_start = released.max(wave.started_at + GROW_MS);
            (1. - (now - fade_start) / FADE_OUT_MS).clamp(0., 1.)
        } else {
            (elapsed / FADE_IN_MS).clamp(0., 1.)
        };
        if wave.released_at.is_some() && opacity == 0. {
            self.cancel();
            return Ok(());
        }

        let progress = GROW.sample((elapsed / GROW_MS).clamp(0., 1.) as f32);
        // Even the transparent first sample replaces the steady press tint.
        display.set_presented_press_ripple(id, wave.center, progress, opacity as f32)?;
        self.request_next = wave.released_at.is_some() || elapsed < GROW_MS;
        Ok(())
    }
}

impl<A: Application> Runtime<A> {
    fn press_ripple_owner(&self, id: NodeId) -> Option<&Node> {
        let node = self.dom.get(id).ok()?;
        (node.has_press_ripple()
            && node.is_focusable()
            && self.in_scope(id)
            && (node.allows_unbound_press_ripple()
                || self
                    .bindings
                    .get(&id)
                    .is_some_and(|binding| binding.activate.is_some())))
        .then_some(node)
    }

    pub(crate) fn press_ripple_contact(&self, owner: NodeId) -> Option<PressRippleContact> {
        let node = self.press_ripple_owner(owner)?;
        let target = self.dom.press_ripple_paint_target(owner)?;
        let computed = self.computed.as_ref()?;
        let bounds = paint_bounds(computed, target)?;
        let owner_bounds = paint_bounds(computed, owner)?;
        Some(PressRippleContact {
            role: node.role(),
            child: node.press_ripple_target_name().map(str::to_owned),
            bounds,
            offset: (bounds.x - owner_bounds.x, bounds.y - owner_bounds.y),
        })
    }

    fn current_ripple_ids(
        &self,
        owner: &str,
        contact: &PressRippleContact,
    ) -> Option<(NodeId, NodeId)> {
        let owner = self.dom.by_id(owner)?;
        let node = self.press_ripple_owner(owner)?;
        if node.role() != contact.role
            || node.press_ripple_target_name() != contact.child.as_deref()
        {
            return None;
        }
        Some((owner, self.dom.press_ripple_paint_target(owner)?))
    }

    pub(crate) fn cancel_press_ripple(&mut self) -> bool {
        self.press_observation.clear_ripple();
        self.press_ripple.cancel()
    }

    pub(crate) fn reconcile_press_ripple(&mut self) {
        let owner = self.press_observation.ripple_owner().or_else(|| {
            self.press_ripple
                .wave
                .as_ref()
                .map(|wave| (wave.owner.as_str(), &wave.contact))
        });
        if owner.is_some_and(|(name, contact)| self.current_ripple_ids(name, contact).is_none()) {
            self.cancel_press_ripple();
        }
    }

    pub(crate) fn present_press_ripple(
        &mut self,
        display: &mut MicroDom,
        computed: &ComputedDom,
    ) -> Result<(), DomError> {
        let Some(now) = self.input_time_ms().filter(|_| !self.reduced_motion()) else {
            self.cancel_press_ripple();
            return Ok(());
        };
        let press = self.press_observation.ripple_frame(self.pressed.as_deref());
        self.press_ripple.observe(press, now);
        self.press_observation.finish_ripple_frame();
        let Some(wave) = &self.press_ripple.wave else {
            return Ok(());
        };
        let ids = self
            .current_ripple_ids(&wave.owner, &wave.contact)
            .filter(|&(owner, target)| {
                let visible = paint_bounds(computed, target).zip(paint_bounds(computed, owner));
                visible.is_some_and(|(bounds, owner_bounds)| {
                    (bounds.width, bounds.height)
                        == (wave.contact.bounds.width, wave.contact.bounds.height)
                        && (bounds.x - owner_bounds.x - wave.contact.offset.0).abs() <= 0.01
                        && (bounds.y - owner_bounds.y - wave.contact.offset.1).abs() <= 0.01
                })
            });
        if let Some((owner, _)) = ids {
            self.press_ripple.present(owner, now, display)?;
        } else {
            self.cancel_press_ripple();
        }
        Ok(())
    }
}
