use surface_app::{AnimationUpdate, HorizontalDrag, HorizontalDragSession};

use super::MAX_WIDTH;
use crate::{motion, shape::LayoutDirection};

// NavigationDrawer.kt at the builder's AndroidX pin supplies the fallback
// tween duration and the two thresholds used by its anchored drag.
const SETTLE_MS: f64 = 256.;
const FLING_SPEED: f32 = 400.;

#[derive(Clone, Copy)]
struct Drag {
    width: f32,
    direction: LayoutDirection,
    open: bool,
}

impl Drag {
    fn sign(self) -> f32 {
        match self.direction {
            LayoutDirection::LeftToRight => 1.,
            LayoutDirection::RightToLeft => -1.,
        }
    }

    fn progress(self, delta_x: f32) -> f32 {
        (endpoint(self.open) + self.sign() * delta_x / self.width).clamp(0., 1.)
    }

    fn target(self, progress: f32, velocity_x: f32) -> bool {
        let velocity = self.sign() * velocity_x;
        if velocity.is_finite() && velocity.abs() >= FLING_SPEED {
            return velocity > 0.;
        }
        progress > 0.5 || (progress == 0.5 && !self.open)
    }
}

struct Settle {
    from: f32,
    open: bool,
    start_ms: Option<f64>,
}

/// The visual and gesture state of an optionally swipeable modal drawer.
///
/// Keep one state per drawer. The application owns its open flag. Call
/// [Self::set_viewport] on viewport changes and [Self::reset] before changing
/// direction, closing the containing view, or setting the open flag directly.
#[derive(Default)]
pub struct NavigationDrawerSwipe {
    viewport: (f32, f32),
    pub(super) session: HorizontalDragSession,
    drag: Option<Drag>,
    preview: Option<f32>,
    settle: Option<Settle>,
}

impl NavigationDrawerSwipe {
    /// Update the logical viewport size. Returns whether to rebuild the view.
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

    /// Discard an interrupted drag or animation and renew gesture ownership.
    pub fn reset(&mut self) {
        self.session = HorizontalDragSession::default();
        self.drag = None;
        self.preview = None;
        self.settle = None;
    }

    /// The visible fraction of the drawer, from zero to one.
    pub fn progress(&self, open: bool) -> f32 {
        self.preview.unwrap_or(endpoint(open))
    }

    /// Whether a release is still moving to an endpoint.
    pub fn is_settling(&self) -> bool {
        self.settle.is_some()
    }

    pub(super) fn width(&self) -> f32 {
        self.viewport.0.min(MAX_WIDTH)
    }

    /// Apply physical pointer travel. Pass the open flag and direction used to
    /// build the drawer. The first event captures these and the drawer width
    /// until release or cancellation. After release, call [Self::advance]
    /// from the application animation frame.
    pub fn apply(&mut self, drag: HorizontalDrag, open: bool, direction: LayoutDirection) {
        let width = self.width();
        if width == 0. || self.is_settling() {
            return;
        }
        let anchor = self.drag.unwrap_or(Drag {
            width,
            direction,
            open,
        });
        match drag {
            HorizontalDrag::Move { delta_x } if delta_x.is_finite() => {
                self.drag = Some(anchor);
                self.preview = Some(anchor.progress(delta_x));
            }
            HorizontalDrag::End {
                delta_x,
                velocity_x,
            } if delta_x.is_finite() => {
                let from = anchor.progress(delta_x);
                self.drag = Some(anchor);
                self.preview = Some(from);
                self.settle = Some(Settle {
                    from,
                    open: anchor.target(from, velocity_x),
                    start_ms: None,
                });
            }
            _ => {
                self.drag = None;
                self.preview = None;
            }
        }
    }

    /// Advance release motion using the host animation clock. Apply the
    /// returned open flag immediately if present and rebuild as requested.
    /// The flag is present only once, when the open state changes.
    /// Missing clocks and Reduce Motion finish the release immediately.
    pub fn advance(
        &mut self,
        now_ms: Option<f64>,
        reduced_motion: bool,
    ) -> (AnimationUpdate, Option<bool>) {
        let Some(settle) = &mut self.settle else {
            return (AnimationUpdate::default(), None);
        };
        let target = endpoint(settle.open);
        if let Some(now) = now_ms.filter(|now| now.is_finite() && !reduced_motion) {
            let start = *settle.start_ms.get_or_insert(now);
            let elapsed = (now - start).max(0.);
            if elapsed < SETTLE_MS && settle.from != target {
                let t = motion::easing::STANDARD_DECELERATE.sample((elapsed / SETTLE_MS) as f32);
                let progress = Some(settle.from + (target - settle.from) * t.clamp(0., 1.));
                let rebuild = self.preview != progress;
                self.preview = progress;
                return (
                    AnimationUpdate {
                        rebuild,
                        request_next: true,
                    },
                    None,
                );
            }
        }
        let open = (self.drag.is_some_and(|drag| drag.open != settle.open)).then_some(settle.open);
        self.reset();
        (
            AnimationUpdate {
                rebuild: true,
                request_next: false,
            },
            open,
        )
    }
}

fn endpoint(open: bool) -> f32 {
    if open {
        1.
    } else {
        0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LayoutDirection::{LeftToRight, RightToLeft};

    #[test]
    fn swipe_uses_logical_direction_and_clamps_to_the_drawer_width() {
        let mut swipe = NavigationDrawerSwipe::default();
        assert!(swipe.set_viewport(800., 600.));
        swipe.apply(HorizontalDrag::Move { delta_x: 90. }, false, LeftToRight);
        assert_eq!(swipe.progress(false), 0.25);
        // A rebuild can't reinterpret the captured direction mid-gesture.
        swipe.apply(HorizontalDrag::Move { delta_x: 180. }, true, RightToLeft);
        assert_eq!(swipe.progress(false), 0.5);
        swipe.apply(HorizontalDrag::Move { delta_x: 450. }, false, LeftToRight);
        assert_eq!(swipe.progress(false), 1.);
        swipe.reset();
        swipe.apply(HorizontalDrag::Move { delta_x: 90. }, true, RightToLeft);
        assert_eq!(swipe.progress(true), 0.75);
        swipe.apply(HorizontalDrag::Cancel, true, RightToLeft);
        assert_eq!(swipe.progress(true), 1.);
        assert!(!swipe.is_settling());
    }

    #[test]
    fn release_uses_half_width_or_fling_and_commits_once() {
        let mut swipe = NavigationDrawerSwipe::default();
        swipe.set_viewport(320., 240.);
        swipe.apply(HorizontalDrag::Move { delta_x: 160. }, false, LeftToRight);
        swipe.apply(
            HorizontalDrag::End {
                delta_x: 160.,
                velocity_x: 0.,
            },
            false,
            LeftToRight,
        );
        assert_eq!(swipe.advance(Some(0.), false).1, None);
        let (mid, commit) = swipe.advance(Some(100.), false);
        assert!(mid.rebuild && mid.request_next);
        assert_eq!(commit, None);
        assert!(swipe.progress(false) > 0.5 && swipe.progress(false) < 1.);
        let (end, commit) = swipe.advance(Some(SETTLE_MS), false);
        assert!(end.rebuild && !end.request_next);
        assert_eq!(commit, Some(true));
        assert_eq!(swipe.advance(Some(SETTLE_MS + 1.), false).1, None);

        swipe.apply(HorizontalDrag::Move { delta_x: 32. }, true, RightToLeft);
        swipe.apply(
            HorizontalDrag::End {
                delta_x: 32.,
                velocity_x: FLING_SPEED,
            },
            true,
            RightToLeft,
        );
        assert_eq!(swipe.advance(Some(0.), true).1, Some(false));
    }

    #[test]
    fn resize_discards_pending_release_and_renews_binding() {
        let mut swipe = NavigationDrawerSwipe::default();
        assert!(!swipe.set_viewport(f32::NAN, 240.));
        assert!(swipe.set_viewport(320., 240.));
        swipe.apply(HorizontalDrag::Move { delta_x: 148. }, false, LeftToRight);
        swipe.apply(
            HorizontalDrag::End {
                delta_x: 148.,
                velocity_x: 0.,
            },
            false,
            LeftToRight,
        );
        assert_eq!(swipe.advance(None, false).1, None);
        swipe.apply(HorizontalDrag::Move { delta_x: 200. }, false, LeftToRight);
        assert!(!swipe.set_viewport(320., 240.));
        assert_eq!(swipe.progress(false), 0.625);
        swipe.apply(
            HorizontalDrag::End {
                delta_x: 200.,
                velocity_x: 900.,
            },
            false,
            LeftToRight,
        );
        assert!(swipe.is_settling());
        let previous = swipe.session.clone();
        assert!(swipe.set_viewport(320., 250.));
        assert_ne!(swipe.session, previous);
        assert_eq!(swipe.advance(None, false).1, None);
        assert_eq!(swipe.progress(false), 0.);
    }
}
