use surface_app::{
    AnimationUpdate, HorizontalDrag, HorizontalDragSession, ToggleMotion, ToggleSpring,
};

use crate::shape::LayoutDirection;

// AndroidX AnchoredDraggable uses logical dp per second for this threshold.
const FLING_SPEED: f32 = 125.;

struct Settle {
    from: f32,
    target: f32,
    velocity: f32,
    motion: Option<ToggleMotion>,
}

/// Swipe state for one standalone modal wide navigation rail.
///
/// The app owns the open flag. Pass viewport updates to [Self::set_viewport],
/// route drag messages to [Self::apply], and sample [Self::advance] on the
/// animation clock. Call [Self::reset] before independent changes to the open
/// flag, layout direction or collapse mode.
#[derive(Default)]
pub struct WideNavigationRailSwipe {
    viewport: (f32, f32),
    pub(super) session: HorizontalDragSession,
    preview: Option<f32>,
    settle: Option<Settle>,
}

impl WideNavigationRailSwipe {
    /// Store the logical viewport size and report whether to rebuild the view.
    /// A resize interrupts the current gesture because its anchors may change.
    pub fn set_viewport(&mut self, width: f32, height: f32) -> bool {
        let viewport = if width.is_finite() && height.is_finite() && width > 0. && height > 0. {
            (width, height)
        } else {
            (0., 0.)
        };
        if self.viewport == viewport {
            return false;
        }
        self.viewport = viewport;
        self.reset();
        true
    }

    /// Discard the current gesture and release its pointer ownership.
    pub fn reset(&mut self) {
        self.session = HorizontalDragSession::default();
        self.preview = None;
        self.settle = None;
    }

    /// The visible fraction of the panel and scrim during a drag or release.
    /// The ordinary modal motion controls the frame when this returns `None`.
    pub fn progress(&self) -> Option<f32> {
        self.preview
    }

    /// Whether a release or cancellation is settling to an anchor.
    pub fn is_settling(&self) -> bool {
        self.settle.is_some()
    }

    pub(super) fn viewport_width(&self) -> f32 {
        self.viewport.0
    }

    /// Apply physical pointer travel using the panel width captured by the
    /// view at press time. Pass the direction used to build the panel.
    pub fn apply(&mut self, drag: HorizontalDrag, panel_width: f32, direction: LayoutDirection) {
        if self.is_settling() || !panel_width.is_finite() || panel_width <= 0. {
            return;
        }
        let sign = match direction {
            LayoutDirection::LeftToRight => 1.,
            LayoutDirection::RightToLeft => -1.,
        };
        let position = |dx: f32| (1. + sign * dx / panel_width).clamp(0., 1.);
        match drag {
            HorizontalDrag::Move { delta_x } if delta_x.is_finite() => {
                self.preview = Some(position(delta_x));
            }
            HorizontalDrag::End {
                delta_x,
                velocity_x,
            } if delta_x.is_finite() => {
                let from = position(delta_x);
                let velocity = sign * velocity_x;
                self.settle_to(from, release_anchor(from, velocity), velocity / panel_width);
            }
            HorizontalDrag::Cancel | HorizontalDrag::End { .. } => {
                if let Some(from) = self.preview {
                    self.settle_to(from, 1., 0.);
                }
            }
            _ => {}
        }
    }

    /// Advance release motion using the host clock. A `true` dismissal result
    /// commits once. Set the app's open flag and ordinary modal motion to the
    /// closed endpoint in the same update. A missing clock or Reduce Motion
    /// finishes the settlement immediately.
    pub fn advance(
        &mut self,
        now_ms: Option<f64>,
        reduced_motion: bool,
    ) -> (AnimationUpdate, bool) {
        let Some(settle) = &mut self.settle else {
            return (AnimationUpdate::default(), false);
        };
        if let Some(now_ms) =
            now_ms.filter(|now| now.is_finite() && !reduced_motion && settle.from != settle.target)
        {
            let spring = ToggleSpring::StandardDefaultSpatial;
            let open = settle.target == 1.;
            let motion = settle.motion.get_or_insert_with(|| {
                ToggleMotion::start_from(open, settle.from, settle.velocity, now_ms, spring)
            });
            let (progress, request_next) = motion.sample(open, Some(now_ms), false, spring);
            if request_next {
                let rebuild = self.preview != Some(progress);
                self.preview = Some(progress);
                return (
                    AnimationUpdate {
                        rebuild,
                        request_next,
                    },
                    false,
                );
            }
        }
        let dismiss = settle.target == 0.;
        self.reset();
        (
            AnimationUpdate {
                rebuild: true,
                request_next: false,
            },
            dismiss,
        )
    }

    fn settle_to(&mut self, from: f32, target: f32, velocity: f32) {
        self.preview = Some(from);
        self.settle = Some(Settle {
            from,
            target,
            velocity: if velocity.is_finite() { velocity } else { 0. },
            motion: None,
        });
    }
}

fn release_anchor(position: f32, velocity: f32) -> f32 {
    if position == 0. || position == 1. {
        return position;
    }
    let velocity = if velocity.is_finite() { velocity } else { 0. };
    let close = if velocity.abs() >= FLING_SPEED {
        velocity < 0.
    } else {
        position < 0.5 || (position == 0.5 && velocity < 0.)
    };
    if close {
        0.
    } else {
        1.
    }
}
