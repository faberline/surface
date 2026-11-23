//! Retained motion and touch ownership for the single-date calendar.

use surface::{scroll_extent, ComputedDom, MicroDom, NodeId};
use surface_app::{
    AnimationFrame, AnimationUpdate, HorizontalDrag, HorizontalDragSession, Queue, ScrollTarget,
    VerticalDrag, VerticalDragSession,
};

use super::{
    calendar_view::GRID_WIDTH,
    swipe_decision::{SwipeDecision, SwipeGesture, SwipeInput, SwipeViewport},
};
use crate::{
    motion::easing, shape::LayoutDirection, DateBounds, DatePickerConstraints, DatePickerEffect,
    DatePickerEvent, DatePickerLocale, DatePickerMode, DatePickerSession, StepDirection, YearMonth,
};

const SETTLE_MS: f64 = 180.;

/// An event from the swipe-capable single-date picker.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DatePickerSwipeInput {
    Date(DatePickerEvent),
    Horizontal { drag: HorizontalDrag, width: f32 },
    Vertical(VerticalDrag),
}

/// Keep one controller alongside each open single-date session.
#[derive(Default)]
pub struct DatePickerSwipe {
    horizontal: HorizontalDragSession,
    vertical: VerticalDragSession,
    pending: Vec<DatePickerSwipeInput>,
    geometry: Option<Geometry>,
    context: Option<Context>,
    gesture: Option<SwipeGesture>,
    vertical_origin: Option<f32>,
    preview: Option<PagePreview>,
    settle: Option<Settle>,
}

#[derive(Clone, Copy, PartialEq)]
struct Context {
    month: YearMonth,
    bounds: DateBounds,
    direction: LayoutDirection,
}

#[derive(Clone, Copy)]
struct Geometry {
    width: f32,
    max_x: f32,
    height: f32,
    max_y: f32,
}

#[derive(Clone, Copy)]
pub(super) struct PagePreview {
    pub(super) month: YearMonth,
    pub(super) pitch: f32,
    pub(super) translation_x: f32,
    pub(super) sign: f32,
}

#[derive(Clone, Copy)]
struct Settle {
    start_ms: f64,
    from: f32,
    target: f32,
    turn: Option<StepDirection>,
}

impl DatePickerSwipe {
    pub fn new() -> Self {
        Self::default()
    }

    /// The outgoing month's physical translation while a page is in motion.
    pub fn preview_offset(&self) -> Option<f32> {
        self.preview.map(|page| page.translation_x)
    }

    /// Forward an input from [crate::Theme::swipeable_date_picker].
    pub fn apply(
        &mut self,
        input: DatePickerSwipeInput,
        session: &mut DatePickerSession,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> DatePickerEffect {
        match input {
            DatePickerSwipeInput::Date(event) => {
                self.clear_motion();
                session.apply(event, constraints, locale)
            }
            _ if session.mode() == DatePickerMode::Calendar => {
                self.pending.push(input);
                DatePickerEffect::None
            }
            _ => DatePickerEffect::None,
        }
    }

    /// Call before painting from the application's animation frame hook.
    /// Use the queue received from [surface_app::Application::attach_queue].
    /// The frame can rebuild the view when a preview changes or a page turn finishes.
    pub fn observe<M>(
        &mut self,
        id: &str,
        frame: &AnimationFrame<'_>,
        session: &mut DatePickerSession,
        constraints: DatePickerConstraints<'_>,
        locale: &DatePickerLocale<'_>,
        queue: &Queue<M>,
    ) -> AnimationUpdate {
        if session.mode() != DatePickerMode::Calendar {
            self.clear_motion();
            self.geometry = None;
            self.context = None;
            return AnimationUpdate::default();
        }

        let context = Context {
            month: session.month(),
            bounds: constraints.bounds,
            direction: locale.direction,
        };
        let grid = format!("{id}-grid-scroll");
        let body = format!("{id}-body");
        let Some((grid_id, body_id)) = frame.dom.by_id(&grid).zip(frame.dom.by_id(&body)) else {
            return AnimationUpdate::default();
        };

        let mut rebuild = false;
        if self.context.is_some_and(|old| old != context) {
            rebuild |= self.preview.is_some();
            self.clear_motion();
        }
        self.context = Some(context);
        if let Some(layout) = frame.layout {
            let measured = Geometry::measure(frame.dom, layout, grid_id, body_id);
            if self
                .geometry
                .is_some_and(|old| old.width != measured.width || old.height != measured.height)
            {
                rebuild |= self.preview.is_some();
                self.clear_motion();
            }
            self.geometry = Some(measured);
        }
        let Some(geometry) = self.geometry else {
            return animation(rebuild, true);
        };
        let grid_style = frame.dom.get(grid_id).expect("valid grid ID").style();
        let body_style = frame.dom.get(body_id).expect("valid body ID").style();
        let mut x = grid_style.scroll_x;
        let mut y = body_style.scroll_y;
        let clock = frame
            .now_ms
            .filter(|now| now.is_finite() && !frame.reduced_motion);
        let mut turn = None;
        for input in std::mem::take(&mut self.pending) {
            match input {
                DatePickerSwipeInput::Horizontal { drag, width }
                    if self.settle.is_none() && (width - geometry.width).abs() < 0.5 =>
                {
                    let before = self.preview_offset();
                    turn = self.horizontal_drag(drag, width, &mut x, geometry, context, clock);
                    rebuild |= before != self.preview_offset() || self.settle.is_some();
                    if turn.is_some() {
                        break;
                    }
                }
                DatePickerSwipeInput::Vertical(drag) if self.settle.is_none() => {
                    self.vertical_drag(drag, &mut y, geometry.max_y);
                }
                _ => {}
            }
        }

        if x != grid_style.scroll_x {
            queue.scroll_to(
                &grid,
                ScrollTarget::Offset {
                    x,
                    y: grid_style.scroll_y,
                },
            );
        }
        if y != body_style.scroll_y {
            queue.scroll_to(
                &body,
                ScrollTarget::Offset {
                    x: body_style.scroll_x,
                    y,
                },
            );
        }

        if let Some(settle) = self.settle {
            let progress = clock.map_or(1., |now| {
                ((now - settle.start_ms) / SETTLE_MS).clamp(0., 1.) as f32
            });
            if progress >= 1. {
                self.preview = None;
                self.settle = None;
                turn = settle.turn;
                rebuild = true;
            } else if let Some(page) = &mut self.preview {
                let next = settle.from
                    + (settle.target - settle.from) * easing::EMPHASIZED.sample(progress);
                rebuild |= page.translation_x != next;
                page.translation_x = next;
            }
        } else if clock.is_none() && self.preview.take().is_some() {
            rebuild = true;
        }

        if let Some(direction) = turn {
            session.apply(DatePickerEvent::BrowseMonth(direction), constraints, locale);
            self.context = Some(Context {
                month: session.month(),
                ..context
            });
            rebuild = true;
        }
        animation(rebuild, self.settle.is_some() || frame.layout.is_none())
    }

    pub(super) fn horizontal_session(&self) -> HorizontalDragSession {
        self.horizontal.clone()
    }

    pub(super) fn vertical_session(&self) -> VerticalDragSession {
        self.vertical.clone()
    }

    pub(super) fn preview(&self) -> Option<PagePreview> {
        self.preview
    }

    pub(super) fn is_settling(&self) -> bool {
        self.settle.is_some()
    }

    fn clear_motion(&mut self) {
        self.pending.clear();
        self.gesture = None;
        self.vertical_origin = None;
        self.preview = None;
        self.settle = None;
    }

    fn horizontal_drag(
        &mut self,
        drag: HorizontalDrag,
        width: f32,
        x: &mut f32,
        geometry: Geometry,
        context: Context,
        clock: Option<f64>,
    ) -> Option<StepDirection> {
        let input = match drag {
            HorizontalDrag::Move { delta_x } => SwipeInput::Move { delta_x },
            HorizontalDrag::End {
                delta_x,
                velocity_x,
            } => SwipeInput::End {
                delta_x,
                velocity_x,
            },
            HorizontalDrag::Cancel => SwipeInput::Cancel,
            _ => return None,
        };
        let terminal = !matches!(input, SwipeInput::Move { .. });
        if self.gesture.is_none() && input != SwipeInput::Cancel {
            self.gesture = SwipeGesture::new(
                SwipeViewport {
                    width,
                    offset_x: *x,
                    max_offset_x: geometry.max_x,
                },
                context.direction,
                context.month,
                context.bounds,
                clock.is_some(),
            );
        }
        let decision = self.gesture.as_mut()?.apply(input);
        if terminal {
            self.gesture = None;
        }
        match decision {
            SwipeDecision::None => {}
            SwipeDecision::PanTo(offset) => *x = offset,
            SwipeDecision::Preview {
                month,
                translation_x,
                sign,
            } if clock.is_some() => {
                self.preview = Some(PagePreview {
                    month,
                    pitch: width.max(GRID_WIDTH),
                    translation_x,
                    sign,
                });
            }
            SwipeDecision::Preview { .. } => {}
            SwipeDecision::Restore => return self.start_settle(None, clock),
            SwipeDecision::Browse {
                from,
                to,
                direction,
            } => {
                if from == context.month && context.bounds.contains_month(to) {
                    return self.start_settle(Some(direction), clock);
                }
            }
        }
        None
    }

    fn start_settle(
        &mut self,
        turn: Option<StepDirection>,
        clock: Option<f64>,
    ) -> Option<StepDirection> {
        if let Some((start_ms, page)) = clock.zip(self.preview) {
            let target = turn.map_or(0., |_| page.pitch * page.sign);
            if target != page.translation_x {
                self.settle = Some(Settle {
                    start_ms,
                    from: page.translation_x,
                    target,
                    turn,
                });
                return None;
            }
        }
        self.preview = None;
        turn
    }

    fn vertical_drag(&mut self, drag: VerticalDrag, y: &mut f32, max_y: f32) {
        match drag {
            VerticalDrag::Move { delta_y } | VerticalDrag::End { delta_y, .. }
                if delta_y.is_finite() =>
            {
                *y = (*self.vertical_origin.get_or_insert(*y) - delta_y).clamp(0., max_y);
                if matches!(drag, VerticalDrag::End { .. }) {
                    self.vertical_origin = None;
                }
            }
            _ => self.vertical_origin = None,
        }
    }
}

impl Geometry {
    fn measure(dom: &MicroDom, layout: &ComputedDom, grid: NodeId, body: NodeId) -> Self {
        let grid_bounds = layout.nodes[grid.index()].bounds;
        let body_bounds = layout.nodes[body.index()].bounds;
        let max_x = scroll_extent(dom, layout, grid)
            .map_or(0., |(width, _)| (width - grid_bounds.width).max(0.));
        let max_y = scroll_extent(dom, layout, body)
            .map_or(0., |(_, height)| (height - body_bounds.height).max(0.));
        Self {
            width: grid_bounds.width,
            max_x,
            height: body_bounds.height,
            max_y,
        }
    }
}

fn animation(rebuild: bool, request_next: bool) -> AnimationUpdate {
    AnimationUpdate {
        rebuild,
        request_next,
    }
}
