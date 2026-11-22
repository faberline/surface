//! Retained width, label and visibility motion for the Extended FAB.

use surface::{Role, Transition};
use surface_app::{
    AnimationFrame, AnimationUpdate, FrameInput, VerticalScroll, VerticalScrollResponse,
};

use crate::{
    motion::{duration, easing},
    ToolbarScroll, ToolbarScrollBehavior, ToolbarVisibility,
};

const SIZE: Transition = Transition::new(duration::MEDIUM4, easing::EMPHASIZED);
const LABEL_IN: Transition = Transition::new(duration::SHORT4, easing::EMPHASIZED);
const LABEL_OUT: Transition = Transition::new(duration::SHORT2, easing::EMPHASIZED);
const SHOW: Transition = Transition::new(duration::LONG2, easing::EMPHASIZED);
const FADE_OUT: Transition = Transition::new(duration::SHORT3, easing::EMPHASIZED_ACCELERATE);
const SCALE_OUT: Transition = Transition::new(duration::MEDIUM1, easing::EMPHASIZED_ACCELERATE);
const HIDDEN_SCALE: f32 = 0.4;

/// Retained animation and measurements for one Extended FAB. Keep this
/// state alongside the application's expanded and visible flags.
pub struct ExtendedFabMotion {
    expansion: Channel,
    label_opacity: Channel,
    visibility_scale: Channel,
    visibility_opacity: Channel,
    last_time: Option<f64>,
    label_size: Option<(f32, f32)>,
    interaction: Interaction,
}

#[derive(Clone, Copy)]
struct Channel {
    value: f32,
    target: f32,
    segment: Option<Segment>,
}

#[derive(Clone, Copy)]
struct Segment {
    from: f32,
    starts_at: f64,
    transition: Transition,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Interaction {
    pub hovered: bool,
    pub focused: bool,
    pub pressed: bool,
}

impl Default for ExtendedFabMotion {
    fn default() -> Self {
        Self::new(true, true)
    }
}

impl ExtendedFabMotion {
    /// Initialize without animating from a different state.
    pub const fn new(expanded: bool, visible: bool) -> Self {
        Self {
            expansion: Channel::new(endpoint(expanded)),
            label_opacity: Channel::new(endpoint(expanded)),
            visibility_scale: Channel::new(if visible { 1. } else { HIDDEN_SCALE }),
            visibility_opacity: Channel::new(endpoint(visible)),
            last_time: None,
            label_size: None,
            interaction: Interaction {
                hovered: false,
                focused: false,
                pressed: false,
            },
        }
    }

    /// Current interpolation of size and insets, from collapsed (0) to
    /// expanded (1). The view uses its measured expanded size.
    pub fn expansion(&self) -> f32 {
        self.expansion.value
    }

    /// Current label opacity, independent of the width interpolation.
    pub fn label_opacity(&self) -> f32 {
        self.label_opacity.value
    }

    /// Scale used for appearance and removal, from 0.4 to 1.
    pub fn visibility_scale(&self) -> f32 {
        self.visibility_scale.value
    }

    /// Current opacity used for appearance and removal.
    pub fn visibility_opacity(&self) -> f32 {
        self.visibility_opacity.value
    }

    pub(super) fn label_size(&self) -> Option<(f32, f32)> {
        self.label_size
    }

    pub(super) fn interaction(&self) -> Interaction {
        self.interaction
    }

    /// Advance from `Application::animation_frame_with_press` using the
    /// button's stable ID, the accepted press, and the current application
    /// state. Pass `true` for expansion when the action has no icon.
    pub fn advance(
        &mut self,
        id: &str,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        pressed: Option<&str>,
        expanded: bool,
        visible: bool,
    ) -> AnimationUpdate {
        let previous = self.visual_state();
        let now = frame
            .now_ms
            .filter(|time| time.is_finite() && !frame.reduced_motion)
            .map(|time| time.max(self.last_time.unwrap_or(time)));
        self.last_time = now;

        self.expansion.advance(now, endpoint(expanded), SIZE, 0.);
        let (fade, delay) = if expanded {
            (LABEL_IN, duration::SHORT2)
        } else {
            (LABEL_OUT, duration::SHORT1)
        };
        self.label_opacity
            .advance(now, endpoint(expanded), fade, f64::from(delay));
        let (scale, opacity) = if visible {
            (SHOW, SHOW)
        } else {
            (SCALE_OUT, FADE_OUT)
        };
        self.visibility_scale
            .advance(now, if visible { 1. } else { HIDDEN_SCALE }, scale, 0.);
        self.visibility_opacity
            .advance(now, endpoint(visible), opacity, 0.);

        let measure = frame.dom.by_id(&format!("{id}-measure"));
        let wrapped = frame.dom.by_id(&format!("{id}-measure-wrapped"));
        if let Some(layout) = frame.layout {
            self.label_size = measure.zip(wrapped).and_then(|(natural, wrapped)| {
                let width = layout.nodes.get(natural.index())?.bounds.width;
                let height = layout.nodes.get(wrapped.index())?.bounds.height;
                (width.is_finite() && height.is_finite() && width > 0. && height > 0.)
                    .then_some((width, height))
            });
        }

        self.interaction = frame
            .dom
            .by_id(id)
            .filter(|&node| {
                visible
                    && frame
                        .dom
                        .get(node)
                        .is_ok_and(|button| button.role() == Role::Button && !button.is_disabled())
            })
            .map(|node| Interaction {
                hovered: input.touch.is_none()
                    && input
                        .pointer
                        .zip(frame.layout)
                        .is_some_and(|((x, y), layout)| {
                            layout.hit_test(frame.dom, x, y) == Some(node)
                        }),
                focused: input.focused == Some(id),
                pressed: pressed == Some(id),
            })
            .unwrap_or_default();

        let changed = previous != self.visual_state();
        AnimationUpdate {
            rebuild: changed,
            request_next: changed
                || (measure.is_some() && wrapped.is_some() && frame.layout.is_none())
                || self.expansion.segment.is_some()
                || self.label_opacity.segment.is_some()
                || self.visibility_scale.segment.is_some()
                || self.visibility_opacity.segment.is_some(),
        }
    }

    fn visual_state(&self) -> ([f32; 4], Option<(f32, f32)>, Interaction) {
        (
            [
                self.expansion(),
                self.label_opacity(),
                self.visibility_scale(),
                self.visibility_opacity(),
            ],
            self.label_size,
            self.interaction,
        )
    }
}

impl Channel {
    const fn new(value: f32) -> Self {
        Self {
            value,
            target: value,
            segment: None,
        }
    }

    fn advance(&mut self, now: Option<f64>, target: f32, transition: Transition, delay: f64) {
        let Some(now) = now else {
            *self = Self::new(target);
            return;
        };
        if let Some(segment) = self.segment {
            let progress = (now - segment.starts_at) / f64::from(segment.transition.duration_ms);
            if progress >= 1. {
                self.value = self.target;
                self.segment = None;
            } else {
                let progress = segment.transition.easing.sample(progress as f32);
                self.value = segment.from + (self.target - segment.from) * progress;
            }
        }
        if target != self.target {
            self.target = target;
            self.segment = (self.value != target).then_some(Segment {
                from: self.value,
                starts_at: now + delay,
                transition,
            });
        }
    }
}

const fn endpoint(enabled: bool) -> f32 {
    if enabled {
        1.
    } else {
        0.
    }
}

/// Application-owned expansion state for an Extended FAB that follows a
/// vertical scroll view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtendedFabScroll(ToolbarScroll);

impl Default for ExtendedFabScroll {
    fn default() -> Self {
        Self::new(true)
    }
}

impl ExtendedFabScroll {
    /// Set the initial expansion state without any prior scroll travel.
    pub const fn new(expanded: bool) -> Self {
        Self(ToolbarScroll::new(expansion_visibility(expanded)))
    }

    /// Whether the action is extended.
    pub const fn expanded(self) -> bool {
        matches!(self.0.visibility(), ToolbarVisibility::Expanded)
    }

    /// Change the expansion state and reset accumulated scroll travel.
    pub fn with_expanded(self, expanded: bool) -> Self {
        Self(self.0.with_visibility(expansion_visibility(expanded)))
    }

    /// Observe vertical wheel or trackpad travel, leaving the body its full
    /// delta. Collapse after 40 logical pixels of down-scroll that actually
    /// move the body. Reverse travel or `keep_expanded` restores expansion.
    /// Store the supplied state in the application's update handler.
    pub fn apply<M>(
        self,
        gesture: VerticalScroll,
        keep_expanded: bool,
        message: impl FnOnce(Self) -> M,
    ) -> Option<VerticalScrollResponse<M>> {
        self.0.apply(
            gesture,
            ToolbarScrollBehavior::CollapseOnScroll,
            keep_expanded,
            |scroll| message(Self(scroll)),
        )
    }
}

const fn expansion_visibility(expanded: bool) -> ToolbarVisibility {
    if expanded {
        ToolbarVisibility::Expanded
    } else {
        ToolbarVisibility::Collapsed
    }
}
