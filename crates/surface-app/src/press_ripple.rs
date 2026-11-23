use surface::{ComputedDom, DomError, Easing, MicroDom, NodeId, Role};

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
    role: Role,
    sequence: u64,
    center: (f32, f32),
    size: (f32, f32),
    started_at: f64,
    released_at: Option<f64>,
}

impl PressRipple {
    pub(crate) fn needs_frame(&self) -> bool {
        self.request_next
    }

    fn cancel(&mut self) -> bool {
        self.request_next = false;
        self.wave.take().is_some()
    }

    fn observe(&mut self, observed: Option<(FramePress<'_>, Role)>, now: f64) {
        let Some((press, role)) = observed else {
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
            let bounds = press.bounds;
            if bounds.width <= 0. || bounds.height <= 0. {
                self.cancel();
                return;
            }
            self.wave = Some(Wave {
                owner: press.id.to_owned(),
                role,
                sequence: press.sequence,
                center: (
                    (press.position.0 - bounds.x).clamp(0., bounds.width),
                    (press.position.1 - bounds.y).clamp(0., bounds.height),
                ),
                size: (bounds.width, bounds.height),
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
    pub(crate) fn press_ripple_role(&self, id: NodeId) -> Option<Role> {
        let node = self.dom.get(id).ok()?;
        (node.has_press_ripple()
            && node.is_focusable()
            && self.in_scope(id)
            && self
                .bindings
                .get(&id)
                .is_some_and(|binding| binding.activate.is_some()))
        .then_some(node.role())
    }

    fn current_ripple_id(&self, owner: &str, role: Role) -> Option<NodeId> {
        self.dom
            .by_id(owner)
            .filter(|&id| self.press_ripple_role(id) == Some(role))
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
                .map(|wave| (wave.owner.as_str(), wave.role))
        });
        if owner.is_some_and(|(name, role)| self.current_ripple_id(name, role).is_none()) {
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
        let id = self
            .current_ripple_id(&wave.owner, wave.role)
            .filter(|&id| {
                computed.nodes.get(id.index()).is_some_and(|node| {
                    let bounds = node.bounds;
                    let visible = bounds.intersect(node.clip).intersect(computed.viewport);
                    (bounds.width, bounds.height) == wave.size
                        && visible.width > 0.
                        && visible.height > 0.
                })
            });
        if let Some(id) = id {
            self.press_ripple.present(id, now, display)?;
        } else {
            self.cancel_press_ripple();
        }
        Ok(())
    }
}
