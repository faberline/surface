//! Material motion tokens from androidx `MotionTokens.kt` (androidx-main).
//!
//! Durations are in milliseconds. Easings are CSS-style cubic Beziers.
//! Springs are not implemented.
use surface::Transition;

pub mod duration {
    pub const SHORT1: f32 = 50.;
    pub const SHORT2: f32 = 100.;
    pub const SHORT3: f32 = 150.;
    pub const SHORT4: f32 = 200.;
    pub const MEDIUM1: f32 = 250.;
    pub const MEDIUM2: f32 = 300.;
    pub const MEDIUM3: f32 = 350.;
    pub const MEDIUM4: f32 = 400.;
    pub const LONG1: f32 = 450.;
    pub const LONG2: f32 = 500.;
    pub const LONG3: f32 = 550.;
    pub const LONG4: f32 = 600.;
    pub const EXTRA_LONG1: f32 = 700.;
    pub const EXTRA_LONG2: f32 = 800.;
    pub const EXTRA_LONG3: f32 = 900.;
    pub const EXTRA_LONG4: f32 = 1000.;
}

pub mod easing {
    use surface::Easing;

    /// Material 3 emphasized easing. androidx uses a path here. The cubic is
    /// an approximation of it, so this matches the androidx cubic constant.
    pub const EMPHASIZED: Easing = Easing::CubicBezier(0.2, 0., 0., 1.);
    pub const EMPHASIZED_DECELERATE: Easing = Easing::CubicBezier(0.05, 0.7, 0.1, 1.);
    pub const EMPHASIZED_ACCELERATE: Easing = Easing::CubicBezier(0.3, 0., 0.8, 0.15);
    pub const STANDARD: Easing = Easing::CubicBezier(0.2, 0., 0., 1.);
    pub const STANDARD_DECELERATE: Easing = Easing::CubicBezier(0., 0., 0., 1.);
    pub const STANDARD_ACCELERATE: Easing = Easing::CubicBezier(0.3, 0., 1., 1.);
    pub const LEGACY: Easing = Easing::CubicBezier(0.4, 0., 0.2, 1.);
    pub const LEGACY_DECELERATE: Easing = Easing::CubicBezier(0., 0., 0.2, 1.);
    pub const LEGACY_ACCELERATE: Easing = Easing::CubicBezier(0.4, 0., 1., 1.);
    pub const LINEAR: Easing = Easing::Linear;
}

/// The transition used when a component changes state. This is a Surface
/// choice, not an official Material mapping.
pub const STATE_CHANGE: Transition = Transition::new(duration::SHORT4, easing::STANDARD);
