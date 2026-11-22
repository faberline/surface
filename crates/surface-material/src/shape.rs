//! Material 3 shape tokens, from androidx `ShapeTokens.kt` (androidx-main).
//!
//! The original start and end constants keep their left-to-right meaning.
//! Use the direction-aware helpers for layouts that mirror their logical edges.
use surface::Corners;

/// Direction of a component's logical start and end edges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutDirection {
    #[default]
    LeftToRight,
    RightToLeft,
}

pub const fn start(radius: f32, direction: LayoutDirection) -> Corners {
    match direction {
        LayoutDirection::LeftToRight => Corners::start(radius),
        LayoutDirection::RightToLeft => Corners::end(radius),
    }
}

pub const fn end(radius: f32, direction: LayoutDirection) -> Corners {
    match direction {
        LayoutDirection::LeftToRight => Corners::end(radius),
        LayoutDirection::RightToLeft => Corners::start(radius),
    }
}

pub const fn large_end(direction: LayoutDirection) -> Corners {
    end(16., direction)
}

pub const NONE: Corners = Corners::all(0.);
pub const EXTRA_SMALL: Corners = Corners::all(4.);
pub const EXTRA_SMALL_TOP: Corners = Corners::top(4.);
pub const SMALL: Corners = Corners::all(8.);
pub const MEDIUM: Corners = Corners::all(12.);
pub const LARGE: Corners = Corners::all(16.);
pub const LARGE_TOP: Corners = Corners::top(16.);
pub const LARGE_START: Corners = Corners::start(16.);
pub const LARGE_END: Corners = Corners::end(16.);
pub const LARGE_INCREASED: Corners = Corners::all(20.);
pub const EXTRA_LARGE: Corners = Corners::all(28.);
pub const EXTRA_LARGE_TOP: Corners = Corners::top(28.);
pub const EXTRA_LARGE_INCREASED: Corners = Corners::all(32.);
pub const EXTRA_EXTRA_LARGE: Corners = Corners::all(48.);
pub const FULL: Corners = Corners::full();
