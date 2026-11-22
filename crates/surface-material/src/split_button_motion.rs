//! Split button feedback, measured label layout and chevron rotation.

use surface::Rect;
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput};

use crate::SplitButtonAnchor;

const TRAVEL_MS: f64 = 200.;

/// Retained presentation data for one split button.
pub struct SplitButtonMotion {
    open: bool,
    angle: f32,
    turn: Option<Turn>,
    anchor: Option<SplitButtonAnchor>,
    label_width: Option<f32>,
    content_height: f32,
    menu_height: f32,
    active: [Interaction; 2],
}

#[derive(Clone, Copy)]
struct Turn {
    start: f32,
    at: f64,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Interaction {
    pub emphasized: bool,
    pub pressed: bool,
}

impl Default for SplitButtonMotion {
    fn default() -> Self {
        Self::new(false)
    }
}

impl SplitButtonMotion {
    /// Initialize at the current application-controlled open state.
    pub fn new(open: bool) -> Self {
        Self {
            open,
            angle: endpoint(open),
            turn: None,
            anchor: None,
            label_width: None,
            content_height: 0.,
            menu_height: 0.,
            active: [Interaction::default(); 2],
        }
    }

    /// Current inward angle in degrees. Mirror its sign in a right-to-left
    /// layout. Reduce Motion always uses the correct endpoint.
    pub fn angle_degrees(&self) -> f32 {
        self.angle
    }

    /// Current measured anchor for a caller-controlled popup.
    pub fn anchor(&self) -> Option<SplitButtonAnchor> {
        self.anchor
    }

    /// Update from `Application::animation_frame_with_press`. Pass the
    /// runtime's accepted primary press and the current open state.
    pub fn advance(
        &mut self,
        id: &str,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        pressed: Option<&str>,
        open: bool,
    ) -> AnimationUpdate {
        let leading = format!("{id}-leading");
        let trailing = format!("{id}-trailing");
        let menu = format!("{id}-menu");
        let previous = (
            self.angle,
            self.anchor,
            self.label_width,
            self.content_height,
            self.menu_height,
            self.active,
        );
        let trailing_node = frame.dom.by_id(&trailing);
        let open = open
            && trailing_node
                .and_then(|id| frame.dom.get(id).ok())
                .is_none_or(|node| !node.is_disabled());
        self.advance_angle(frame.now_ms, frame.reduced_motion, open);

        let hovered = if frame.dom.by_id(&menu).is_none() && input.touch.is_none() {
            input
                .pointer
                .zip(frame.layout)
                .and_then(|((x, y), layout)| layout.hit_test(frame.dom, x, y))
        } else {
            None
        };
        self.active = [leading.as_str(), trailing.as_str()].map(|key| {
            let held = pressed == Some(key);
            let hovered = hovered.is_some() && hovered == frame.dom.by_id(key);
            Interaction {
                emphasized: held || hovered || input.focused == Some(key),
                pressed: held,
            }
        });
        if let Some(layout) = frame.layout {
            let get = |suffix: &str| {
                frame
                    .dom
                    .by_id(&format!("{id}-{suffix}"))
                    .and_then(|node| layout.nodes.get(node.index()))
            };
            let bounds = |suffix| get(suffix).map(|node| node.bounds);
            self.label_width = bounds("measure").map(|bounds| bounds.width);
            self.content_height = bounds("leading-content").map_or(0., |bounds| bounds.height);
            self.menu_height = bounds("menu-content").map_or(0., |bounds| bounds.height);
            self.anchor = get("leading-face")
                .zip(get("trailing-face"))
                .and_then(|(lead, tail)| {
                    let leading = lead.bounds.intersect(lead.clip);
                    let trailing = tail.bounds.intersect(tail.clip);
                    (leading.width > 0.
                        && leading.height > 0.
                        && trailing.width > 0.
                        && trailing.height > 0.)
                        .then(|| SplitButtonAnchor {
                            group: union(leading, trailing),
                            trailing,
                            viewport: layout.viewport,
                        })
                });
        }
        let changed = previous
            != (
                self.angle,
                self.anchor,
                self.label_width,
                self.content_height,
                self.menu_height,
                self.active,
            );
        AnimationUpdate {
            rebuild: changed,
            request_next: changed
                || self.turn.is_some()
                || (trailing_node.is_some() && frame.layout.is_none()),
        }
    }

    pub(super) fn label_width(&self) -> Option<f32> {
        self.label_width
    }

    pub(super) fn content_height(&self) -> f32 {
        self.content_height
    }

    pub(super) fn desired_menu_height(&self) -> f32 {
        self.menu_height
    }

    pub(super) fn interaction(&self, trailing: bool) -> Interaction {
        self.active[usize::from(trailing)]
    }

    fn advance_angle(&mut self, now_ms: Option<f64>, reduced_motion: bool, open: bool) {
        let Some(now) = now_ms.filter(|now| now.is_finite() && !reduced_motion) else {
            self.open = open;
            self.angle = endpoint(open);
            self.turn = None;
            return;
        };
        if let Some(turn) = self.turn {
            let progress = ((now - turn.at) / TRAVEL_MS).clamp(0., 1.) as f32;
            let offset = progress - 1.;
            // Android's pinned OvershootInterpolator with tension 1.
            let adjusted = 1. + offset * offset * (1. + 2. * offset);
            self.angle = turn.start + (endpoint(self.open) - turn.start) * adjusted;
            if progress == 1. {
                self.turn = None;
            }
        }
        if self.open != open {
            self.turn = Some(Turn {
                start: self.angle,
                at: now,
            });
            self.open = open;
        }
    }
}

fn endpoint(open: bool) -> f32 {
    if open {
        180.
    } else {
        0.
    }
}

fn union(a: Rect, b: Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect::new(
        x,
        y,
        (a.x + a.width).max(b.x + b.width) - x,
        (a.y + a.height).max(b.y + b.height) - y,
    )
}
