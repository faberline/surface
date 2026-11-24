//! Retained language-independent motion for the entering and exiting Search view.

use surface::Rect;
use surface_app::{AnimationFrame, AnimationUpdate, ToggleMotion, ToggleSpring};

use crate::search_layout::{SearchLayout, SearchPresentation, SearchStyle};

/// A frame target derived from the current Search options and safe geometry.
#[derive(Clone, Copy, Debug)]
pub struct SearchOverlayMotionTarget {
    pub(super) open: bool,
    pub(super) style: SearchStyle,
    pub(super) resting_bar: Option<Rect>,
    pub(super) destination: Option<SearchLayout>,
}

/// Retained motion for one caller-controlled Search overlay.
#[derive(Default)]
pub struct SearchOverlayMotion {
    spring: ToggleMotion,
    progress: f32,
    moving: bool,
}

impl SearchOverlayMotion {
    /// Begin at the specified resting state.
    pub fn new(open: bool) -> Self {
        Self {
            spring: ToggleMotion::new(open),
            progress: if open { 1. } else { 0. },
            moving: false,
        }
    }

    /// Current clamped fraction between the measured start and destination.
    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// Whether the spatial track is settling.
    pub fn is_animating(&self) -> bool {
        self.moving
    }

    /// Advance the retained track for the caller's current open state. Pass
    /// the same geometry to the builder. A missing or unfit anchor, missing
    /// clock, or Reduce Motion snaps to the target immediately.
    pub fn advance(
        &mut self,
        frame: &AnimationFrame<'_>,
        target: SearchOverlayMotionTarget,
    ) -> AnimationUpdate {
        let preset = match (
            target.style,
            target.destination.map(|layout| layout.presentation),
            target.open,
        ) {
            (SearchStyle::Contained, Some(SearchPresentation::Docked), true) => {
                ToggleSpring::StandardDefaultSpatial
            }
            (SearchStyle::Contained, _, _) => ToggleSpring::StandardFastSpatial,
            (SearchStyle::Divided, _, true) => ToggleSpring::StandardSlowSpatial,
            (SearchStyle::Divided, _, false) => ToggleSpring::StandardDefaultSpatial,
        };
        let (progress, moving) = self.spring.sample(
            target.open,
            frame.now_ms,
            frame.reduced_motion || target.resting_bar.is_none() || target.destination.is_none(),
            preset,
        );
        let rebuild = self.progress != progress;
        self.progress = progress;
        self.moving = moving;
        AnimationUpdate {
            rebuild,
            request_next: moving,
        }
    }
}
