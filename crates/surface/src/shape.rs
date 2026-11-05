//! Per-corner radii for rounded rectangles.
//!
//! Corner order is always top-left, top-right, bottom-right, bottom-left.
//! `start` and `end` assume left-to-right text and are not mirrored for RTL.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Corners {
    pub const ZERO: Corners = Corners::all(0.);
    /// Fully rounded. Finite so JSON round-trips exactly. `resolve` clamps it
    /// to half of the short side.
    pub const FULL_RADIUS: f32 = f32::MAX;

    pub const fn all(r: f32) -> Corners {
        Corners::new(r, r, r, r)
    }
    pub const fn new(
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) -> Corners {
        Corners {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }
    pub const fn top(r: f32) -> Corners {
        Corners::new(r, r, 0., 0.)
    }
    pub const fn bottom(r: f32) -> Corners {
        Corners::new(0., 0., r, r)
    }
    /// Left side in a left-to-right layout.
    pub const fn start(r: f32) -> Corners {
        Corners::new(r, 0., 0., r)
    }
    /// Right side in a left-to-right layout.
    pub const fn end(r: f32) -> Corners {
        Corners::new(0., r, r, 0.)
    }
    pub const fn full() -> Corners {
        Corners::all(Corners::FULL_RADIUS)
    }
    /// The shared radius when all four corners are exactly equal.
    pub fn uniform(&self) -> Option<f32> {
        (self.top_left == self.top_right
            && self.top_right == self.bottom_right
            && self.bottom_right == self.bottom_left)
            .then_some(self.top_left)
    }
    pub fn is_uniform(&self) -> bool {
        self.uniform().is_some()
    }
    /// Every corner is finite and not negative.
    pub fn is_valid(&self) -> bool {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
        .into_iter()
        .all(|r| r.is_finite() && r >= 0.)
    }
    /// Clamp each corner into `[0, min(width, height) / 2]`.
    pub fn resolve(&self, width: f32, height: f32) -> Corners {
        let max = width.max(0.).min(height.max(0.)) / 2.;
        let clamp = |r: f32| r.clamp(0., max);
        Corners::new(
            clamp(self.top_left),
            clamp(self.top_right),
            clamp(self.bottom_right),
            clamp(self.bottom_left),
        )
    }
}
