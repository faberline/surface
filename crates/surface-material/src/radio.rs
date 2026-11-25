//! Material 3 radio buttons with caller-owned selection.

use surface::{
    Align, Color, Length, MeasuredGroupLayout, Node, Rect, Role, Style, TextAlignment,
    TextMeasurer, TextScaling,
};
use surface_app::{
    AnimationFrame, AnimationUpdate, CursorStyle, FrameInput, FramePress, FramePressPhase,
    ToggleMotion, ToggleSpring, View,
};

use crate::{
    motion::STATE_CHANGE,
    shape::{self, LayoutDirection},
    typography, ChoiceState, Theme,
};

const TARGET_SIZE: f32 = 48.;
const LAYER_SIZE: f32 = 40.;
const RING_SIZE: f32 = 20.;
const RING_STROKE: f32 = 2.;
const DOT_SIZE: f32 = 10.;
const FOCUS_GAP: f32 = 2.;
const FOCUS_STROKE: f32 = 3.;
const FOCUS_SIZE: f32 = RING_SIZE + 2. * (FOCUS_GAP + FOCUS_STROKE);
const DISABLED_ALPHA: u8 = 97;
const HOVER_ALPHA: u8 = 20;
const FOCUS_ALPHA: u8 = 26;
const PRESS_ALPHA: u8 = 26;

/// A named radio option. The adjacent label shares its hit target.
#[derive(Clone, Copy, Debug)]
pub struct RadioOptions<'a> {
    pub id: &'a str,
    pub label: &'a str,
    /// Override the accessible name. An omitted or blank name uses the label.
    pub name: Option<&'a str>,
    pub state: ChoiceState,
    pub direction: LayoutDirection,
    /// Scale the wrapping label while keeping the artwork and target fixed.
    pub text_scale: f32,
}

/// The layout of a radio group. A horizontal group wraps when space runs out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadioGroupLayout {
    #[default]
    Vertical,
    Horizontal,
}

/// The visible layout and accessible name of a controlled radio group.
#[derive(Clone, Copy, Debug)]
pub struct RadioGroupOptions<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub direction: LayoutDirection,
    pub layout: RadioGroupLayout,
}

/// Presented radio artwork. The checked state always comes from the caller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadioPose {
    /// The fraction of the selected dot diameter currently painted.
    pub dot_fraction: f32,
    pub hovered: bool,
    pub focused: bool,
    pub pressed: bool,
}

impl RadioPose {
    fn rest(state: ChoiceState) -> Self {
        Self {
            dot_fraction: if state.selected { 1. } else { 0. },
            hovered: false,
            focused: false,
            pressed: false,
        }
    }

    fn layer_alpha(self) -> u8 {
        if self.pressed {
            PRESS_ALPHA
        } else if self.focused {
            FOCUS_ALPHA
        } else if self.hovered {
            HOVER_ALPHA
        } else {
            0
        }
    }
}

/// Retain one controller for each stable radio ID.
#[derive(Default)]
pub struct RadioMotion {
    dot: Option<ToggleMotion>,
    shown: Option<RadioPose>,
    last_time: Option<f64>,
}

impl RadioMotion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pose(&self, options: RadioOptions<'_>) -> RadioPose {
        if options.state.disabled {
            RadioPose::rest(options.state)
        } else {
            self.shown.unwrap_or_else(|| RadioPose::rest(options.state))
        }
    }

    /// Advance from `Application::animation_frame_with_press_detail` with frame
    /// input observation enabled. An absent clock, disabled or unmounted
    /// option, or Reduce Motion settles the dot at the accepted endpoint.
    pub fn advance(
        &mut self,
        options: RadioOptions<'_>,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> AnimationUpdate {
        let previous = self.pose(options);
        let mut pose = RadioPose::rest(options.state);
        let enabled = !options.state.disabled
            && frame
                .dom
                .by_id(options.id)
                .and_then(|id| frame.dom.get(id).ok())
                .is_some_and(|node| node.role() == Role::Radio && !node.is_disabled());
        if enabled {
            pose.hovered = hovered == Some(options.id);
            pose.focused = input.focused == Some(options.id);
            pose.pressed = press.is_some_and(|event| {
                event.id == options.id && event.phase == FramePressPhase::Held
            });
        }
        let clock = frame
            .now_ms
            .filter(|time| time.is_finite() && enabled && !frame.reduced_motion)
            .map(|time| time.max(self.last_time.unwrap_or(time)));
        let request_next = if let Some(now) = clock {
            let selected = options.state.selected;
            let dot = self.dot.get_or_insert_with(|| ToggleMotion::new(selected));
            let (fraction, request_next) = dot.sample(
                selected,
                Some(now),
                false,
                ToggleSpring::StandardFastSpatial,
            );
            pose.dot_fraction = fraction;
            request_next
        } else {
            self.dot = None;
            false
        };
        self.shown = Some(pose);
        self.last_time = clock;
        AnimationUpdate {
            rebuild: pose != previous,
            request_next,
        }
    }
}

impl Theme {
    /// Build one controlled radio. An enabled unselected option sends the
    /// given message on activation. The selected option remains focusable.
    pub fn radio_with_options<M>(
        &self,
        options: RadioOptions<'_>,
        motion: &RadioMotion,
        message: M,
    ) -> View<M> {
        let name = options
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(options.label);
        let rtl = options.direction == LayoutDirection::RightToLeft;
        let gap = if options.label.trim().is_empty() {
            0.
        } else {
            8.
        };
        let mut row = Style::row().align(Align::Center).gap(gap);
        row.min_width = TARGET_SIZE;
        row.max_width = Length::Percent(1.);
        row.min_height = TARGET_SIZE;
        let mut node = Node::new(Role::Radio, name)
            .id(options.id)
            .checked(options.state.selected)
            .disabled(options.state.disabled)
            .focus_color(Color::TRANSPARENT)
            .styled(row);
        if rtl {
            node = node.reverse_row();
        }
        let view = View::new(node).children([
            self.radio_art(options, motion.pose(options)),
            self.radio_label(options, rtl),
        ]);
        if options.state.disabled {
            return view;
        }
        let view = view.cursor(CursorStyle::Pointer);
        if options.state.selected {
            view
        } else {
            view.on_press(message)
                .suppress_repeated_activation()
                .accessible_activation()
        }
    }

    /// Build a named, controlled group. Keep one selected child at most.
    /// The group may start with no selection. Arrows select enabled options
    /// and wrap. Enabled checked children retain the group's normal Tab entry.
    pub fn radio_group_with_options<M>(
        &self,
        options: RadioGroupOptions<'_>,
        children: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        let horizontal = options.layout == RadioGroupLayout::Horizontal;
        let rtl = options.direction == LayoutDirection::RightToLeft;
        let style = if horizontal {
            Style::row().wrap(true)
        } else {
            Style::column()
        };
        let mut group = Node::new(Role::RadioGroup, options.name)
            .id(options.id)
            .styled(style.gap(4.));
        if horizontal && rtl {
            group = group.reverse_row();
        }
        View::new(group).children(children).roving_radio_group(rtl)
    }

    fn radio_label<M>(&self, options: RadioOptions<'_>, rtl: bool) -> View<M> {
        let scale = if options.text_scale > 0. && (options.text_scale * 24.).is_finite() {
            options.text_scale
        } else {
            1.
        };
        let mut text = typography::BODY_LARGE;
        text.font_size *= scale;
        text.line_height = text.line_height.map(|height| height * scale);
        let color = if options.state.disabled {
            alpha(self.colors.on_surface, DISABLED_ALPHA)
        } else {
            self.colors.on_surface
        };
        View::new(
            Node::new(Role::Text, "")
                .id(format!("{}-label", options.id))
                .text(options.label)
                .text_scaling(TextScaling::Fixed)
                .text_alignment(if rtl {
                    TextAlignment::Right
                } else {
                    TextAlignment::Left
                })
                .styled(
                    Style::column()
                        .with_text_style(text)
                        .foreground(color)
                        .grow(1.),
                )
                .shrink_to_fit_width(),
        )
        .without_interaction(None)
    }

    fn radio_art<M>(&self, options: RadioOptions<'_>, pose: RadioPose) -> View<M> {
        let colors = &self.colors;
        let selected = options.state.selected;
        let disabled = options.state.disabled;
        let layer_alpha = pose.layer_alpha();
        let dot_color = if disabled {
            alpha(colors.on_surface, DISABLED_ALPHA)
        } else {
            colors.primary
        };
        let ring_color = if selected || disabled {
            dot_color
        } else if layer_alpha != 0 {
            colors.on_surface
        } else {
            colors.on_surface_variant
        };
        let layer_color = if selected != pose.pressed {
            colors.primary
        } else {
            colors.on_surface
        };
        let focus_color = if pose.focused {
            colors.secondary
        } else {
            Color::TRANSPARENT
        };
        let circle = || Style::column().corners(shape::FULL);
        let art = |suffix: &str, style| {
            View::new(
                Node::new(Role::Group, "")
                    .id(format!("{}-{suffix}", options.id))
                    .styled(style),
            )
        };
        let mut ring = circle().border(RING_STROKE, ring_color);
        if !disabled {
            ring = ring.transition(STATE_CHANGE);
        }
        View::new(
            Node::new(Role::Group, "")
                .styled(
                    Style::column()
                        .width(Length::Px(TARGET_SIZE))
                        .height(Length::Px(TARGET_SIZE)),
                )
                .measured_group(RadioArtwork {
                    dot_size: DOT_SIZE * pose.dot_fraction,
                }),
        )
        .children([
            art(
                "state-layer",
                circle().background(alpha(layer_color, layer_alpha)),
            ),
            art("ring", ring),
            art("dot", circle().background(dot_color)),
            art("focus-ring", circle().border(FOCUS_STROKE, focus_color)),
        ])
        .without_interaction(None)
    }
}

#[derive(Debug)]
struct RadioArtwork {
    dot_size: f32,
}

impl MeasuredGroupLayout for RadioArtwork {
    fn measure(&self, _: &[&Node], _: Option<f32>, _: &mut dyn TextMeasurer) -> (f32, f32) {
        (TARGET_SIZE, TARGET_SIZE)
    }

    fn place(
        &self,
        _: &[&Node],
        (width, height): (f32, f32),
        _: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        [LAYER_SIZE, RING_SIZE, self.dot_size, FOCUS_SIZE]
            .map(|size| Rect::new((width - size) / 2., (height - size) / 2., size, size))
            .to_vec()
    }
}

fn alpha(color: Color, opacity: u8) -> Color {
    Color(color.0, color.1, color.2, opacity.min(color.3))
}
