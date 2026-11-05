//! Material 3 shape tokens, from androidx `ShapeTokens.kt` (androidx-main).
//!
//! `start` and `end` assume left-to-right text. They are not mirrored for RTL.
use surface::Corners;

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
