use crate::toolbar::ToolbarVisibility;
use surface_app::{VerticalScroll, VerticalScrollResponse};

const HIDE_DISTANCE: f32 = 40.;

/// How a toolbar follows the scroll of nearby content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolbarScrollBehavior {
    /// Keep the toolbar expanded.
    #[default]
    Pinned,
    /// Hide the toolbar after scrolling down through the content.
    HideOnScroll,
    /// Leave the caller's collapsed toolbar when scrolling down.
    CollapseOnScroll,
}

/// Toolbar visibility and the distance toward its next scroll transition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolbarScroll {
    visibility: ToolbarVisibility,
    forward_travel: f32,
}

impl Default for ToolbarScroll {
    fn default() -> Self {
        Self::new(ToolbarVisibility::Expanded)
    }
}

impl ToolbarScroll {
    /// Set an initial visibility without prior scroll travel.
    pub const fn new(visibility: ToolbarVisibility) -> Self {
        Self {
            visibility,
            forward_travel: 0.,
        }
    }

    pub const fn visibility(self) -> ToolbarVisibility {
        self.visibility
    }

    /// Set visibility and discard the previous scroll travel.
    pub fn with_visibility(mut self, visibility: ToolbarVisibility) -> Self {
        self.visibility = visibility;
        self.forward_travel = 0.;
        self
    }

    /// Follow the actual scroll travel left within the current content bounds.
    /// Forty logical pixels of uninterrupted down-scroll hide or collapse the
    /// toolbar. Up-scroll reveals it and resets that distance. The hook keeps
    /// the toolbar expanded while `keep_visible` is true, for example while a
    /// toolbar action has focus. Return the response from a scroll view's hook
    /// and store the new state in `Application::update`. The body keeps the
    /// full scroll delta.
    pub fn apply<M>(
        self,
        gesture: VerticalScroll,
        behavior: ToolbarScrollBehavior,
        keep_visible: bool,
        message: impl FnOnce(ToolbarScroll) -> M,
    ) -> Option<VerticalScrollResponse<M>> {
        let target = match behavior {
            ToolbarScrollBehavior::Pinned => ToolbarVisibility::Expanded,
            ToolbarScrollBehavior::HideOnScroll => ToolbarVisibility::Hidden,
            ToolbarScrollBehavior::CollapseOnScroll => ToolbarVisibility::Collapsed,
        };
        let next = if keep_visible || behavior == ToolbarScrollBehavior::Pinned {
            Self::default()
        } else {
            self.follow(gesture, target)?
        };
        (next != self).then(|| VerticalScrollResponse {
            consumed: 0.,
            message: message(next),
        })
    }

    fn follow(self, gesture: VerticalScroll, target: ToolbarVisibility) -> Option<Self> {
        let VerticalScroll {
            delta,
            offset,
            max_offset,
        } = gesture;
        if !delta.is_finite() || !offset.is_finite() || !max_offset.is_finite() {
            return None;
        }
        let max_offset = max_offset.max(0.);
        let offset = offset.clamp(0., max_offset);
        let travel = (offset + delta).clamp(0., max_offset) - offset;
        if travel < 0. {
            return Some(Self::default());
        }
        if travel == 0. || self.visibility == target {
            return None;
        }
        let forward_travel = self.forward_travel + travel;
        Some(if forward_travel >= HIDE_DISTANCE {
            Self::new(target)
        } else {
            Self {
                forward_travel,
                ..self
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ToolbarScrollBehavior::{CollapseOnScroll, HideOnScroll, Pinned};
    use ToolbarVisibility::{Collapsed, Expanded, Hidden};

    fn scroll(
        state: ToolbarScroll,
        behavior: ToolbarScrollBehavior,
        delta: f32,
        offset: f32,
        max_offset: f32,
        keep_visible: bool,
    ) -> Option<ToolbarScroll> {
        let response = state.apply(
            VerticalScroll {
                delta,
                offset,
                max_offset,
            },
            behavior,
            keep_visible,
            |new| new,
        )?;
        assert_eq!(response.consumed, 0.);
        Some(response.message)
    }

    #[test]
    fn actual_clamped_travel_counts_toward_hiding() {
        let state = scroll(
            ToolbarScroll::default(),
            HideOnScroll,
            80.,
            90.,
            100.,
            false,
        )
        .unwrap();
        assert_eq!(state.visibility(), Expanded);
        assert_eq!(scroll(state, HideOnScroll, 80., 100., 100., false), None);

        let state = scroll(state, HideOnScroll, 29., 0., 100., false).unwrap();
        assert_eq!(state.visibility(), Expanded);
        let state = scroll(state, HideOnScroll, 1., 29., 100., false).unwrap();
        assert_eq!(state.visibility(), Hidden);
        assert_eq!(scroll(state, HideOnScroll, -20., 0., 100., false), None);
    }

    #[test]
    fn reverse_scroll_reveals_and_resets_the_collapse_distance() {
        let state = scroll(
            ToolbarScroll::default(),
            CollapseOnScroll,
            39.,
            0.,
            200.,
            false,
        )
        .unwrap();
        let state = scroll(state, CollapseOnScroll, -1., 39., 200., false).unwrap();
        assert_eq!(state, ToolbarScroll::default());
        let state = scroll(state, CollapseOnScroll, 39., 38., 200., false).unwrap();
        assert_eq!(state.visibility(), Expanded);
        let state = scroll(state, CollapseOnScroll, 1., 77., 200., false).unwrap();
        assert_eq!(state.visibility(), Collapsed);
        let state = scroll(state, CollapseOnScroll, -1., 78., 200., false).unwrap();
        assert_eq!(state, ToolbarScroll::default());
    }

    #[test]
    fn focus_reveals_and_resets_travel_even_at_a_bound() {
        let hidden = ToolbarScroll::default().with_visibility(Hidden);
        let state = scroll(hidden, HideOnScroll, 100., 100., 100., true).unwrap();
        assert_eq!(state, ToolbarScroll::default());
        assert_eq!(scroll(state, HideOnScroll, 100., 0., 100., true), None);

        let state = scroll(state, HideOnScroll, 25., 0., 100., false).unwrap();
        let state = scroll(state, HideOnScroll, 100., 100., 100., true).unwrap();
        let state = scroll(state, HideOnScroll, 20., 0., 100., false).unwrap();
        assert_eq!(state.visibility(), Expanded);
    }

    #[test]
    fn pinned_and_invalid_scroll_cannot_hide_the_toolbar() {
        let state = ToolbarScroll::new(Collapsed);
        let state = scroll(state, Pinned, 20., 100., 100., false).unwrap();
        assert_eq!(state, ToolbarScroll::default());
        assert_eq!(scroll(state, Pinned, 80., 0., 100., false), None);
        assert_eq!(scroll(state, HideOnScroll, f32::NAN, 0., 100., false), None);
        assert_eq!(scroll(state, HideOnScroll, 80., 0., 0., false), None);
    }
}
