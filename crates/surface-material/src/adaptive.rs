//! Material window size classes for adaptive layouts.
//!
//! Values use the available window's logical dimensions in density-independent
//! pixels (dp), not the physical display size or framebuffer pixels. The host
//! must convert its viewport to dp before classification. The classes describe
//! the current window, so an app should recompute them when it is resized.
//!
//! Breakpoints follow the current Android adaptive window size class table:
//! <https://developer.android.com/develop/adaptive-apps/guides/use-window-size-classes>.

/// Width class of the available application window, in ascending order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WindowWidthSizeClass {
    /// Less than 600 dp.
    Compact,
    /// At least 600 dp and less than 840 dp.
    Medium,
    /// At least 840 dp and less than 1200 dp.
    Expanded,
    /// At least 1200 dp and less than 1600 dp.
    Large,
    /// At least 1600 dp.
    ExtraLarge,
}

impl WindowWidthSizeClass {
    /// Classify a finite, nonnegative width in dp.
    ///
    /// Returns `None` for a negative or non-finite value. Zero is valid because
    /// a window can temporarily have no available space during layout.
    #[must_use]
    pub fn from_dp(width: f32) -> Option<Self> {
        if !width.is_finite() || width < 0. {
            return None;
        }
        Some(if width < 600. {
            Self::Compact
        } else if width < 840. {
            Self::Medium
        } else if width < 1200. {
            Self::Expanded
        } else if width < 1600. {
            Self::Large
        } else {
            Self::ExtraLarge
        })
    }
}
