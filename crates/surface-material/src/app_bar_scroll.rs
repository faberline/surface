use surface_app::{VerticalScroll, VerticalScrollResponse};

/// How a top app bar responds to vertical content scrolling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppBarScrollBehavior {
    /// Hold the bar in place and change its surface when content scrolls.
    #[default]
    Pinned,
    /// Collapse on forward scroll and expand as soon as scroll reverses.
    EnterAlways,
    /// Expand after reverse scroll reaches the start of the content.
    ExitUntilCollapsed,
}

/// One scroll update for a caller-controlled app bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppBarScrollUpdate {
    pub bar: AppBarScroll,
    /// Content offset after the bar and content share the gesture.
    pub content_offset: f32,
}

/// The last app bar state. The view can preserve it across scroll messages.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AppBarScroll {
    collapsed: f32,
    scrolled: bool,
}

impl AppBarScroll {
    /// Partial collapse from 0 (expanded) to 1 (collapsed).
    pub fn collapse(self) -> f32 {
        self.collapsed
    }

    /// Use the scrolled surface while content is offset or the bar is collapsed.
    pub fn scrolled(self) -> bool {
        self.scrolled
    }

    /// Share a wheel gesture between a bar and its content.
    ///
    /// Pass the returned response from the content's
    /// `View::on_vertical_scroll` handler. Store `update.bar` when the
    /// application receives the resulting message. Pass the component's
    /// collapse distance in logical pixels.
    pub fn apply<M>(
        self,
        gesture: VerticalScroll,
        behavior: AppBarScrollBehavior,
        distance: f32,
        message: impl FnOnce(AppBarScrollUpdate) -> M,
    ) -> Option<VerticalScrollResponse<M>> {
        let VerticalScroll {
            delta,
            offset,
            max_offset,
        } = gesture;
        if !delta.is_finite() || delta == 0. || !offset.is_finite() || !max_offset.is_finite() {
            return None;
        }
        let max_offset = max_offset.max(0.);
        let offset = offset.clamp(0., max_offset);
        let distance = if distance.is_finite() && behavior != AppBarScrollBehavior::Pinned {
            distance.max(0.)
        } else {
            0.
        };
        let current = self.collapsed * distance;
        let offered = if behavior == AppBarScrollBehavior::ExitUntilCollapsed && delta < 0. {
            (delta + offset).min(0.)
        } else {
            delta
        };
        let bar_offset = (current + offered).clamp(0., distance);
        let consumed = bar_offset - current;
        let content_offset = (offset + (delta - consumed)).clamp(0., max_offset);
        let bar = Self {
            collapsed: if distance == 0. {
                0.
            } else {
                bar_offset / distance
            },
            scrolled: bar_offset > 0. || content_offset > 0.,
        };
        Some(VerticalScrollResponse {
            consumed,
            message: message(AppBarScrollUpdate {
                bar,
                content_offset,
            }),
        })
    }

    /// Update the tint after programmatic scroll or keyboard focus reveal.
    pub fn with_content_offset(mut self, offset: f32) -> Self {
        self.scrolled = self.collapsed > 0. || (offset.is_finite() && offset > 0.);
        self
    }

    /// Start from an application-controlled collapse fraction and content offset.
    pub fn new(collapse: f32, content_offset: f32) -> Self {
        let collapsed = if collapse.is_finite() {
            collapse.clamp(0., 1.)
        } else {
            0.
        };
        Self {
            collapsed,
            scrolled: false,
        }
        .with_content_offset(content_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AppBarScrollBehavior::{EnterAlways, ExitUntilCollapsed, Pinned};

    fn scroll(
        bar: AppBarScroll,
        behavior: AppBarScrollBehavior,
        delta: f32,
        offset: f32,
    ) -> VerticalScrollResponse<AppBarScrollUpdate> {
        bar.apply(
            VerticalScroll {
                delta,
                offset,
                max_offset: 500.,
            },
            behavior,
            72.,
            |change| change,
        )
        .unwrap()
    }

    #[test]
    fn pinned_scroll_changes_surface_only() {
        let start = AppBarScroll::default();
        let forward = scroll(start, Pinned, 25., 0.);
        assert_eq!(forward.consumed, 0.);
        assert_eq!(forward.message.content_offset, 25.);
        assert_eq!(forward.message.bar.collapse(), 0.);
        assert!(forward.message.bar.scrolled());
        let back = scroll(forward.message.bar, Pinned, -25., 25.);
        assert_eq!(back.message.bar, start);
    }

    #[test]
    fn enter_always_consumes_before_content_in_both_directions() {
        let forward = scroll(AppBarScroll::default(), EnterAlways, 152., 0.);
        assert_eq!(forward.consumed, 72.);
        assert_eq!(forward.message.content_offset, 80.);
        assert_eq!(forward.message.bar.collapse(), 1.);
        let back = scroll(forward.message.bar, EnterAlways, -36., 80.);
        assert_eq!(back.consumed, -36.);
        assert_eq!(back.message.content_offset, 80.);
        assert_eq!(back.message.bar.collapse(), 0.5);
        let back = scroll(back.message.bar, EnterAlways, -48., 80.);
        assert_eq!(back.consumed, -36.);
        assert_eq!(back.message.content_offset, 68.);
        assert_eq!(back.message.bar.collapse(), 0.);
    }

    #[test]
    fn exit_until_collapsed_uses_only_reverse_delta_left_at_the_top() {
        let forward = scroll(AppBarScroll::default(), ExitUntilCollapsed, 152., 0.);
        let back = scroll(forward.message.bar, ExitUntilCollapsed, -50., 80.);
        assert_eq!(back.consumed, 0.);
        assert_eq!(back.message.content_offset, 30.);
        assert_eq!(back.message.bar.collapse(), 1.);
        let back = scroll(back.message.bar, ExitUntilCollapsed, -66., 30.);
        assert_eq!(back.consumed, -36.);
        assert_eq!(back.message.content_offset, 0.);
        assert_eq!(back.message.bar.collapse(), 0.5);
        assert!(back.message.bar.scrolled());
        let back = scroll(back.message.bar, ExitUntilCollapsed, -72., 0.);
        assert!(!back.message.bar.scrolled());
    }
}
