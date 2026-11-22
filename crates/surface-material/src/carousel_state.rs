//! Scroll and gesture state for Material carousels.

use surface_app::{
    AnimationUpdate, HorizontalDrag, HorizontalDragSession, VerticalDrag, VerticalDragSession,
};

use crate::{
    motion::{duration, easing},
    CarouselLayout,
};

const WHEEL_IDLE_MS: f64 = 90.;
const FLING_THRESHOLD: f32 = 500.;
const FLING_SECONDS: f32 = 0.16;

/// Input offered to the application that owns a carousel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CarouselInput {
    HorizontalDrag(HorizontalDrag),
    VerticalDrag(VerticalDrag),
    /// Physical wheel delta along the carousel axis, in logical pixels.
    Wheel(f32),
    /// Scroll to a requested item without invoking its action.
    GoTo(usize),
    /// Reveal an item immediately so Runtime can move keyboard focus to it.
    Focus(usize),
}

#[derive(Clone, Copy)]
struct Origin {
    offset: f32,
    index: Option<usize>,
}

#[derive(Clone, Copy)]
struct Settle {
    origin: f32,
    target: f32,
    index: Option<usize>,
    started: Option<f64>,
}

#[derive(Clone, Copy, Default)]
enum Phase {
    #[default]
    Idle,
    Dragging(Origin),
    Wheel {
        started: Option<f64>,
        index: Option<usize>,
    },
    Settling(Settle),
}

/// Caller-owned carousel motion. Keep one instance per mounted carousel.
#[derive(Default)]
pub struct CarouselState {
    offset: f32,
    index: Option<usize>,
    horizontal: HorizontalDragSession,
    vertical: VerticalDragSession,
    phase: Phase,
}

impl CarouselState {
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// Logical current item. Explicit focus and navigation retain the
    /// requested item even when several free-scroll anchors share an offset.
    pub fn active_index(&self, layout: &CarouselLayout) -> Option<usize> {
        self.index
            .filter(|index| layout.item_offset(*index) == Some(self.offset))
            .or_else(|| layout.nearest_index(self.offset))
    }

    /// The pending item when settling, otherwise the currently visible item.
    /// Use this for repeated previous or next commands during animation.
    pub fn target_index(&self, layout: &CarouselLayout) -> Option<usize> {
        match self.phase {
            Phase::Settling(settle) => settle.index,
            _ => None,
        }
        .filter(|index| layout.item_offset(*index).is_some())
        .or_else(|| self.active_index(layout))
    }

    pub fn dragging(&self) -> bool {
        matches!(self.phase, Phase::Dragging(_))
    }

    pub(crate) fn horizontal_session(&self) -> HorizontalDragSession {
        self.horizontal.clone()
    }

    pub(crate) fn vertical_session(&self) -> VerticalDragSession {
        self.vertical.clone()
    }

    /// Apply one wheel, drag or navigation request.
    pub fn apply(&mut self, input: CarouselInput, layout: &CarouselLayout) -> bool {
        match input {
            CarouselInput::HorizontalDrag(drag) => match drag {
                HorizontalDrag::Move { delta_x } => self.move_drag(delta_x, layout),
                HorizontalDrag::End {
                    delta_x,
                    velocity_x,
                } => self.end_drag(delta_x, velocity_x, layout),
                HorizontalDrag::Cancel => self.cancel_drag(layout),
                _ => false,
            },
            CarouselInput::VerticalDrag(drag) => match drag {
                VerticalDrag::Move { delta_y } => self.move_drag(delta_y, layout),
                VerticalDrag::End {
                    delta_y,
                    velocity_y,
                } => self.end_drag(delta_y, velocity_y, layout),
                VerticalDrag::Cancel => self.cancel_drag(layout),
                _ => false,
            },
            CarouselInput::Wheel(delta) if delta.is_finite() && delta != 0. => {
                let index = match self.phase {
                    Phase::Wheel { index, .. } => index,
                    _ => self.active_index(layout),
                };
                let offset = layout.clamp(self.offset + delta * layout.physical_sign());
                self.offset = self.clamp_step(offset, index, layout);
                self.index = None;
                if layout.kind.contained() {
                    self.phase = Phase::Wheel {
                        started: None,
                        index,
                    };
                } else {
                    self.phase = Phase::Idle;
                }
                true
            }
            CarouselInput::GoTo(index) => self.go_to(index, layout, false),
            CarouselInput::Focus(index) => self.go_to(index, layout, true),
            _ => false,
        }
    }

    /// Advance wheel settling or inertial drag motion on the host clock.
    /// Reduce Motion applies the final position on the next frame.
    pub fn advance(
        &mut self,
        now_ms: Option<f64>,
        reduced_motion: bool,
        layout: &CarouselLayout,
    ) -> AnimationUpdate {
        let prior = self.offset;
        let prior_index = self.active_index(layout);
        let now_ms = now_ms.filter(|time| time.is_finite());
        if let Phase::Wheel { started, index } = self.phase {
            let waiting = if let Some(now) = now_ms {
                let then = started.unwrap_or(now);
                self.phase = Phase::Wheel {
                    started: Some(then),
                    index,
                };
                now - then < WHEEL_IDLE_MS
            } else {
                false
            };
            if !waiting {
                if let Some(index) = layout.nearest_index(self.offset) {
                    self.go_to(index, layout, reduced_motion);
                } else {
                    self.phase = Phase::Idle;
                }
            }
        }
        match self.phase {
            Phase::Settling(mut settle) => {
                let target = settle
                    .index
                    .and_then(|index| layout.item_offset(index))
                    .unwrap_or_else(|| layout.clamp(settle.target));
                if let Some(now) = now_ms.filter(|_| !reduced_motion && settle.origin != target) {
                    let started = *settle.started.get_or_insert(now);
                    let fraction = ((now - started) / f64::from(duration::MEDIUM2)).clamp(0., 1.);
                    if fraction < 1. {
                        let eased = easing::EMPHASIZED.sample(fraction as f32);
                        self.offset =
                            layout.clamp(settle.origin + (target - settle.origin) * eased);
                        self.phase = Phase::Settling(settle);
                    } else {
                        self.finish(target, settle.index);
                    }
                } else {
                    self.finish(target, settle.index);
                }
            }
            Phase::Idle => {
                self.offset = self
                    .index
                    .and_then(|index| layout.item_offset(index))
                    .unwrap_or_else(|| layout.clamp(self.offset));
            }
            _ => self.offset = layout.clamp(self.offset),
        }
        AnimationUpdate {
            rebuild: prior != self.offset || prior_index != self.active_index(layout),
            request_next: matches!(self.phase, Phase::Wheel { .. } | Phase::Settling(_)),
        }
    }

    fn go_to(&mut self, index: usize, layout: &CarouselLayout, immediate: bool) -> bool {
        let Some(target) = layout.item_offset(index) else {
            return false;
        };
        if immediate {
            self.finish(target, Some(index));
        } else {
            self.settle(target, Some(index));
        }
        true
    }

    fn move_drag(&mut self, delta: f32, layout: &CarouselLayout) -> bool {
        if !delta.is_finite() {
            return self.cancel_drag(layout);
        }
        let origin = self.drag_origin(layout);
        self.offset = self.drag_preview(origin, delta, layout);
        self.index = None;
        self.phase = Phase::Dragging(origin);
        true
    }

    fn end_drag(&mut self, delta: f32, velocity: f32, layout: &CarouselLayout) -> bool {
        if !delta.is_finite() {
            return self.cancel_drag(layout);
        }
        let origin = self.drag_origin(layout);
        self.offset = self.drag_preview(origin, delta, layout);
        let projected_delta = if velocity.is_finite() && velocity.abs() >= FLING_THRESHOLD {
            delta + velocity * FLING_SECONDS
        } else {
            delta
        };
        let projected = self.drag_preview(origin, projected_delta, layout);
        if layout.kind.contained() {
            if let Some(index) = layout.nearest_index(projected) {
                self.go_to(index, layout, false);
            } else {
                self.finish(0., None);
            }
        } else {
            self.settle(projected, None);
        }
        true
    }

    fn cancel_drag(&mut self, layout: &CarouselLayout) -> bool {
        let Phase::Dragging(origin) = self.phase else {
            return false;
        };
        self.settle(layout.clamp(origin.offset), origin.index);
        true
    }

    fn drag_origin(&self, layout: &CarouselLayout) -> Origin {
        match self.phase {
            Phase::Dragging(origin) => origin,
            _ => Origin {
                offset: layout.clamp(self.offset),
                index: self.index,
            },
        }
    }

    fn drag_preview(&self, origin: Origin, delta: f32, layout: &CarouselLayout) -> f32 {
        let offset = layout.clamp(origin.offset - delta * layout.physical_sign());
        let origin_index = origin.index.or_else(|| layout.nearest_index(origin.offset));
        self.clamp_step(offset, origin_index, layout)
    }

    fn clamp_step(&self, offset: f32, origin_index: Option<usize>, layout: &CarouselLayout) -> f32 {
        if let Some(index) = origin_index.filter(|_| layout.kind.single_step()) {
            let low = layout.item_offset(index.saturating_sub(1)).unwrap_or(0.);
            let high = layout
                .item_offset(index.saturating_add(1))
                .unwrap_or(layout.max_offset());
            offset.clamp(low.min(high), high)
        } else {
            offset
        }
    }

    fn settle(&mut self, target: f32, index: Option<usize>) {
        if self.offset == target {
            self.finish(target, index);
        } else {
            self.index = None;
            self.phase = Phase::Settling(Settle {
                origin: self.offset,
                target,
                index,
                started: None,
            });
        }
    }

    fn finish(&mut self, target: f32, index: Option<usize>) {
        self.offset = target;
        self.index = index;
        self.phase = Phase::Idle;
    }
}
