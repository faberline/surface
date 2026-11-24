//! Material 3 checkboxes with caller-owned state and a retained vector mark.

use surface::{Align, Color, Corners, FillContour, Insets, Length, Node, Role, Style};
use surface::{TextAlignment, TextScaling};
use surface_app::{
    AnimationFrame, AnimationUpdate, CursorStyle, FrameInput, FramePress, FramePressPhase,
    ToggleMotion, ToggleSpring, View,
};

use crate::{
    motion::STATE_CHANGE,
    shape::{self, LayoutDirection},
    typography, CheckValue, Theme,
};

const BOX_SIZE: f32 = 18.;
const BOX_RADIUS: f32 = 2.;
const OUTLINE: f32 = 2.;
const LAYER_SIZE: f32 = 40.;
const FOCUS_STROKE: f32 = 3.;
const FOCUS_GAP: f32 = 2.;
const VISUAL_SIZE: f32 = LAYER_SIZE + 2. * (FOCUS_STROKE + FOCUS_GAP);
const MIN_TARGET: f32 = 48.;
const DISABLED_ALPHA: u8 = 97;
const HOVER_ALPHA: u8 = 20;
const FOCUS_ALPHA: u8 = 26;
const PRESS_ALPHA: u8 = 26;

/// One named, controlled checkbox. The adjacent label shares the hit target.
#[derive(Clone, Copy, Debug)]
pub struct CheckboxOptions<'a> {
    pub id: &'a str,
    pub label: &'a str,
    /// A name for assistive technology. An omitted or blank name uses the label.
    pub name: Option<&'a str>,
    pub value: CheckValue,
    pub disabled: bool,
    pub error: bool,
    /// Stable ID of visible supporting text rendered by the caller.
    /// Used when `error` is true.
    pub error_description: Option<&'a str>,
    pub direction: LayoutDirection,
    /// Scale the wrapping label. The artwork and minimum target stay fixed.
    pub text_scale: f32,
}

impl CheckboxOptions<'_> {
    fn selected(self) -> bool {
        self.value != CheckValue::Unchecked
    }
}

/// Presented vector mark and interaction. Checked state is always caller-owned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckboxPose {
    /// How much of the tick or dash path to draw, from 0 to 1.
    pub mark_progress: f32,
    /// 0 is a tick, 1 is a dash. Intermediate values move the elbow.
    pub mixed_fraction: f32,
    pub hovered: bool,
    pub focused: bool,
    pub pressed: bool,
}

impl CheckboxPose {
    fn rest(options: CheckboxOptions<'_>) -> Self {
        Self {
            mark_progress: if options.selected() { 1. } else { 0. },
            mixed_fraction: if options.value == CheckValue::Mixed {
                1.
            } else {
                0.
            },
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

/// Retain one controller for each stable Checkbox ID.
#[derive(Default)]
pub struct CheckboxMotion {
    tracks: Option<(ToggleMotion, ToggleMotion)>,
    shown: Option<CheckboxPose>,
    last_time: Option<f64>,
}

impl CheckboxMotion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pose(&self, options: CheckboxOptions<'_>) -> CheckboxPose {
        if options.disabled {
            CheckboxPose::rest(options)
        } else {
            self.shown.unwrap_or_else(|| CheckboxPose::rest(options))
        }
    }

    /// Advance in `Application::animation_frame_with_press_detail`, with
    /// `Application::observes_frame_input` enabled. Missing time, disabled or
    /// unmounted controls, and Reduce Motion use exact endpoints.
    pub fn advance(
        &mut self,
        options: CheckboxOptions<'_>,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> AnimationUpdate {
        let previous = self.pose(options);
        let mut pose = CheckboxPose::rest(options);
        let enabled = !options.disabled
            && frame.dom.by_id(options.id).is_some_and(|id| {
                frame
                    .dom
                    .get(id)
                    .is_ok_and(|node| node.role() == Role::Checkbox && !node.is_disabled())
            });
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
        let mut request_next = false;
        if let Some(now) = clock {
            let selected = options.selected();
            let indeterminate = options.value == CheckValue::Mixed;
            let (draw, shape) = self.tracks.get_or_insert_with(|| {
                (
                    ToggleMotion::new(selected),
                    ToggleMotion::new(indeterminate),
                )
            });
            let (mark_progress, drawing) = draw.sample(
                selected,
                Some(now),
                false,
                ToggleSpring::StandardDefaultSpatial,
            );
            // Changing from empty to a mixed selection starts with a dash.
            // Preserve the current shape while erasing a selected mark.
            if selected && previous.mark_progress == 0. {
                *shape = ToggleMotion::new(indeterminate);
            }
            let (mixed_fraction, shaping) = if selected {
                shape.sample(
                    indeterminate,
                    Some(now),
                    false,
                    ToggleSpring::StandardDefaultSpatial,
                )
            } else {
                (previous.mixed_fraction, false)
            };
            pose.mark_progress = mark_progress;
            pose.mixed_fraction = if mark_progress == 0. && !selected && !drawing {
                0.
            } else {
                mixed_fraction
            };
            request_next = drawing || shaping;
        } else {
            self.tracks = None;
        }
        self.shown = Some(pose);
        self.last_time = clock;
        AnimationUpdate {
            rebuild: pose != previous,
            request_next,
        }
    }
}

impl Theme {
    /// Build a Material 3 checkbox. Checked requests Unchecked, while
    /// Unchecked and Mixed request Checked. The caller accepts by changing
    /// its value. An enabled checkbox owns one pointer and keyboard target.
    pub fn checkbox_with_options<M>(
        &self,
        options: CheckboxOptions<'_>,
        motion: &CheckboxMotion,
        on_change: impl Fn(CheckValue) -> M,
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
        row.min_width = MIN_TARGET;
        row.min_height = MIN_TARGET;
        let mut root = Node::new(Role::Checkbox, name)
            .id(options.id)
            .checked(options.selected())
            .mixed(options.value == CheckValue::Mixed)
            .disabled(options.disabled)
            .invalid(options.error)
            .focus_color(Color::TRANSPARENT)
            .styled(row);
        if let Some(id) = options
            .error_description
            .filter(|id| options.error && !id.trim().is_empty())
        {
            root = root.described_by_text(id);
        }
        if rtl {
            root = root.reverse_row();
        }

        let scale = if options.text_scale > 0. && (options.text_scale * 24.).is_finite() {
            options.text_scale
        } else {
            1.
        };
        let mut typography = typography::BODY_LARGE;
        typography.font_size *= scale;
        typography.line_height = typography.line_height.map(|height| height * scale);
        let label_color = if options.disabled {
            alpha(self.colors.on_surface, DISABLED_ALPHA)
        } else {
            self.colors.on_surface
        };
        let label = View::new(
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
                        .with_text_style(typography)
                        .foreground(label_color)
                        .grow(1.),
                )
                .shrink_to_fit_width(),
        )
        .without_interaction(None);
        let view =
            View::new(root).children([self.checkbox_art(options, motion.pose(options)), label]);
        if options.disabled {
            view
        } else {
            let next = if options.value == CheckValue::Checked {
                CheckValue::Unchecked
            } else {
                CheckValue::Checked
            };
            view.on_press(on_change(next))
                .suppress_repeated_activation()
                .accessible_activation()
                .cursor(CursorStyle::Pointer)
        }
    }

    fn checkbox_art<M>(&self, options: CheckboxOptions<'_>, pose: CheckboxPose) -> View<M> {
        let colors = self.checkbox_colors(options, pose);
        let layer = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-state-layer", options.id))
                .styled(
                    centered_art(LAYER_SIZE)
                        .corners(shape::FULL)
                        .background(alpha(colors.layer, pose.layer_alpha())),
                ),
        );
        let mut box_style = centered_art(BOX_SIZE)
            .corners(Corners::all(BOX_RADIUS))
            .background(colors.fill)
            .border(
                if options.selected() { 0. } else { OUTLINE },
                colors.outline,
            );
        if !options.disabled {
            box_style = box_style.transition(STATE_CHANGE);
        }
        let container = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-box", options.id))
                .styled(box_style),
        );
        let mut mark = Node::new(Role::Group, "")
            .id(format!("{}-mark", options.id))
            .styled(centered_art(BOX_SIZE));
        if let Some(contour) = mark_contour(pose, colors.mark) {
            mark = mark.fill_contour(contour);
        }
        let ring = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-focus-ring", options.id))
                .styled(centered_art(VISUAL_SIZE).corners(shape::FULL).border(
                    FOCUS_STROKE,
                    if pose.focused {
                        self.colors.secondary
                    } else {
                        Color::TRANSPARENT
                    },
                )),
        );
        View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Px(VISUAL_SIZE))
                    .height(Length::Px(VISUAL_SIZE)),
            ),
        )
        .children([layer, container, View::new(mark), ring])
        .without_interaction(None)
    }

    fn checkbox_colors(&self, options: CheckboxOptions<'_>, pose: CheckboxPose) -> CheckboxColors {
        let colors = &self.colors;
        let (accent, unselected, mark, layer) = if options.disabled {
            let dim = alpha(colors.on_surface, DISABLED_ALPHA);
            (dim, dim, colors.surface, Color::TRANSPARENT)
        } else if options.error {
            (colors.error, colors.error, colors.on_error, colors.error)
        } else {
            let layer = if options.selected() != pose.pressed {
                colors.primary
            } else {
                colors.on_surface
            };
            let unselected = if pose.layer_alpha() == 0 {
                colors.on_surface_variant
            } else {
                colors.on_surface
            };
            (colors.primary, unselected, colors.on_primary, layer)
        };
        if options.selected() {
            CheckboxColors {
                fill: accent,
                outline: Color::TRANSPARENT,
                mark,
                layer,
            }
        } else {
            let departing_mark = if pose.mark_progress > 0. {
                mark
            } else {
                Color::TRANSPARENT
            };
            CheckboxColors {
                fill: Color::TRANSPARENT,
                outline: unselected,
                mark: departing_mark,
                layer,
            }
        }
    }
}

struct CheckboxColors {
    fill: Color,
    outline: Color,
    mark: Color,
    layer: Color,
}

fn centered_art(size: f32) -> Style {
    let inset = (VISUAL_SIZE - size) / 2.;
    Style::column()
        .width(Length::Px(size))
        .height(Length::Px(size))
        .absolute(Insets {
            left: Length::Px(inset),
            top: Length::Px(inset),
            ..Insets::default()
        })
}

fn alpha(color: Color, opacity: u8) -> Color {
    Color(color.0, color.1, color.2, opacity.min(color.3))
}

// Stroke the measured center path with one square-capped, 2 dp vector fill.
// The endpoints stay within the 18 dp box throughout the morph.
fn mark_contour(pose: CheckboxPose, color: Color) -> Option<FillContour> {
    if pose.mark_progress <= 0. || color.3 == 0 {
        return None;
    }
    let mixed = pose.mixed_fraction;
    let left = [4.5, 9.];
    let elbow = [7.2 + 1.8 * mixed, 11.7 - 2.7 * mixed];
    let right = [13.5, 5.4 + 3.6 * mixed];
    let (first, first_length) = tangent(left, elbow);
    let (second, second_length) = tangent(elbow, right);
    let offset = (first_length + second_length) * pose.mark_progress;
    let begin = along(left, first, -1.);
    let first_normal = [-first[1], first[0]];
    if offset <= first_length {
        let end = along(left, first, offset + 1.);
        return FillContour::new(
            [
                along(begin, first_normal, 1.),
                along(end, first_normal, 1.),
                along(end, first_normal, -1.),
                along(begin, first_normal, -1.),
            ],
            color,
        );
    }
    let second_normal = [-second[1], second[0]];
    let join = [
        first_normal[0] + second_normal[0],
        first_normal[1] + second_normal[1],
    ];
    let miter = 1. / (1. + first[0] * second[0] + first[1] * second[1]);
    let end = along(elbow, second, offset - first_length + 1.);
    FillContour::new(
        [
            along(begin, first_normal, 1.),
            along(elbow, join, miter),
            along(end, second_normal, 1.),
            along(end, second_normal, -1.),
            along(elbow, join, -miter),
            along(begin, first_normal, -1.),
        ],
        color,
    )
}

fn tangent(start: [f32; 2], end: [f32; 2]) -> ([f32; 2], f32) {
    let delta = [end[0] - start[0], end[1] - start[1]];
    let length = delta[0].hypot(delta[1]);
    ([delta[0] / length, delta[1] / length], length)
}

fn along(point: [f32; 2], direction: [f32; 2], distance: f32) -> [f32; 2] {
    [
        point[0] + direction[0] * distance,
        point[1] + direction[1] * distance,
    ]
}
