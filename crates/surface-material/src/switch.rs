//! Material 3 switches with independently controlled state and retained motion.

use surface::{Align, Color, Insets, Length, Node, Role, Style, TextAlignment, TextScaling};
use surface_app::{
    AnimationFrame, AnimationUpdate, CursorStyle, FrameInput, FramePress, FramePressPhase,
    SpatialMotion, View,
};

use crate::{
    motion::STATE_CHANGE,
    shape::{self, LayoutDirection},
    typography, ChoiceState, Elevation, Theme, VectorIcon,
};

const TRACK_WIDTH: f32 = 52.;
const TRACK_HEIGHT: f32 = 32.;
const OUTLINE: f32 = 2.;
const OFF_CENTER: f32 = 16.;
const ON_CENTER: f32 = TRACK_WIDTH - OFF_CENTER;
const OFF_DIAMETER: f32 = 16.;
const ON_DIAMETER: f32 = 24.;
const PRESSED_DIAMETER: f32 = 28.;
const ICON_SIZE: f32 = 16.;
const STATE_LAYER: f32 = 40.;
const FOCUS_GAP: f32 = 2.;
const FOCUS_STROKE: f32 = 3.;
const FOCUS_INSET: f32 = FOCUS_GAP + FOCUS_STROKE;
const VISUAL_WIDTH: f32 = TRACK_WIDTH + 2. * FOCUS_INSET;
const VISUAL_HEIGHT: f32 = TRACK_HEIGHT + 2. * FOCUS_INSET;
const DISABLED_TRACK_ALPHA: u8 = 31;
const DISABLED_MARK_ALPHA: u8 = 97;

/// Optional decorative artwork for the switch handle.
#[derive(Clone, Copy, Debug, Default)]
pub enum SwitchIcons<'a> {
    #[default]
    None,
    Selected(&'a VectorIcon),
    Both {
        unselected: &'a VectorIcon,
        selected: &'a VectorIcon,
    },
}

impl<'a> SwitchIcons<'a> {
    fn at(self, selected: bool) -> Option<&'a VectorIcon> {
        match (self, selected) {
            (Self::Both { selected, .. } | Self::Selected(selected), true) => Some(selected),
            (Self::Both { unselected, .. }, false) => Some(unselected),
            _ => None,
        }
    }
}

/// One controlled switch with a visible label and stable identity.
#[derive(Clone, Copy, Debug)]
pub struct SwitchOptions<'a> {
    pub id: &'a str,
    pub label: &'a str,
    /// A descriptive accessible name when the visible label is ambiguous.
    /// An omitted or blank name uses the visible label.
    pub name: Option<&'a str>,
    pub state: ChoiceState,
    pub icons: SwitchIcons<'a>,
    pub direction: LayoutDirection,
    /// Scale for the wrapping label. Use the application's current setting.
    pub text_scale: f32,
}

/// The current handle geometry and accepted interaction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwitchPose {
    /// Handle center measured from the logical start of the 52 dp track.
    pub center: f32,
    pub diameter: f32,
    pub hovered: bool,
    pub focused: bool,
    pub pressed: bool,
}

impl SwitchPose {
    fn rest(options: SwitchOptions<'_>) -> Self {
        let selected = options.state.selected;
        Self {
            center: if selected { ON_CENTER } else { OFF_CENTER },
            diameter: if selected || options.icons.at(false).is_some() {
                ON_DIAMETER
            } else {
                OFF_DIAMETER
            },
            hovered: false,
            focused: false,
            pressed: false,
        }
    }
}

/// Retain one instance per stable switch ID. The application owns the checked value.
#[derive(Default)]
pub struct SwitchMotion {
    tracks: Option<(SpatialMotion, SpatialMotion)>,
    shown: Option<SwitchPose>,
    last_time: Option<f64>,
}

impl SwitchMotion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pose(&self, options: SwitchOptions<'_>) -> SwitchPose {
        let shown = self.shown.unwrap_or_else(|| SwitchPose::rest(options));
        if options.state.disabled {
            SwitchPose::rest(options)
        } else {
            shown
        }
    }

    /// Advance in `Application::animation_frame_with_press_detail` when the
    /// application opts in through `Application::observes_frame_input`.
    /// The first frame, reduced motion and missing time use exact endpoints.
    pub fn advance(
        &mut self,
        options: SwitchOptions<'_>,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> AnimationUpdate {
        let previous = self.pose(options);
        let mut target = SwitchPose::rest(options);
        let enabled = !options.state.disabled
            && frame.dom.by_id(options.id).is_some_and(|id| {
                frame
                    .dom
                    .get(id)
                    .is_ok_and(|node| node.role() == Role::Switch && !node.is_disabled())
            });
        if enabled {
            target.hovered = hovered == Some(options.id);
            target.focused = input.focused == Some(options.id);
            target.pressed = press.is_some_and(|event| {
                event.id == options.id && event.phase == FramePressPhase::Held
            });
        }
        if target.pressed {
            target.diameter = PRESSED_DIAMETER;
        }
        let clock = frame
            .now_ms
            .filter(|time| time.is_finite() && enabled && !frame.reduced_motion)
            .map(|time| time.max(self.last_time.unwrap_or(time)));
        let mut active = false;
        if let Some(now) = clock {
            let (center, size) = self.tracks.get_or_insert_with(|| {
                (
                    SpatialMotion::new(f64::from(target.center)),
                    SpatialMotion::new(f64::from(target.diameter)),
                )
            });
            center.aim(f64::from(target.center), now);
            if target.pressed && !previous.pressed {
                *size = SpatialMotion::new(f64::from(PRESSED_DIAMETER));
            } else {
                size.aim(f64::from(target.diameter), now);
            }
            let (x, moving) = center.advance(now);
            let (diameter, resizing) = size.advance(now);
            target.center = (x as f32).clamp(OFF_CENTER, ON_CENTER);
            target.diameter = (diameter as f32).clamp(OFF_DIAMETER, PRESSED_DIAMETER);
            active = moving || resizing;
        } else {
            self.tracks = None;
        }
        self.shown = Some(target);
        self.last_time = clock;
        AnimationUpdate {
            rebuild: target != previous,
            request_next: active,
        }
    }
}

impl Theme {
    /// Build a binary switch with an independent layout and interaction target.
    ///
    /// The stable target covers the visible label and artwork. Enter, Space
    /// and a tap request the opposite checked value once. Update the options
    /// to accept the request. The component keeps the authored state until
    /// then. A disabled switch exposes its state without an activation action.
    pub fn switch_with_options<M>(
        &self,
        options: SwitchOptions<'_>,
        motion: &SwitchMotion,
        on_change: impl Fn(bool) -> M,
    ) -> View<M> {
        let name = options
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(options.label);
        let rtl = options.direction == LayoutDirection::RightToLeft;
        let gap = if options.label.trim().is_empty() {
            0.
        } else {
            12.
        };
        let mut row = Style::row().align(Align::Center).gap(gap);
        row.min_width = VISUAL_WIDTH;
        row.min_height = 48.;
        let mut root = Node::new(Role::Switch, name)
            .id(options.id)
            .checked(options.state.selected)
            .disabled(options.state.disabled)
            .focus_color(Color::TRANSPARENT)
            .styled(row);
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
        let label_color = if options.state.disabled {
            alpha(self.colors.on_surface, DISABLED_MARK_ALPHA)
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
            View::new(root).children([label, self.switch_art(options, motion.pose(options))]);
        if options.state.disabled {
            view
        } else {
            view.on_press(on_change(!options.state.selected))
                .suppress_repeated_activation()
                .accessible_activation()
                .cursor(CursorStyle::Pointer)
        }
    }

    fn switch_art<M>(&self, options: SwitchOptions<'_>, pose: SwitchPose) -> View<M> {
        let colors = self.switch_colors(options.state, pose);
        let center = if options.direction == LayoutDirection::RightToLeft {
            TRACK_WIDTH - pose.center
        } else {
            pose.center
        };
        let track = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-track", options.id))
                .styled(
                    art_at(FOCUS_INSET, FOCUS_INSET, TRACK_WIDTH, TRACK_HEIGHT)
                        .background(colors.track)
                        .border(OUTLINE, colors.outline)
                        .transition(STATE_CHANGE),
                ),
        );
        let state_layer = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-state-layer", options.id))
                .styled(centered_art(center, STATE_LAYER).state_layer(colors.layer))
                .state_layer_alphas(20, 26, 26),
        );
        let handle = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-handle", options.id))
                .styled(
                    centered_art(center, pose.diameter)
                        .background(colors.handle)
                        .transition(STATE_CHANGE),
                )
                .box_shadows(self.elevation(if options.state.disabled {
                    Elevation::Level0
                } else {
                    Elevation::Level1
                })),
        );
        let icon = options.icons.at(options.state.selected).and_then(|icon| {
            // During the first part of an icon-bearing transition the full
            // square may not yet fit in the circle. Keep the path inside it.
            let size = ICON_SIZE.min(pose.diameter * std::f32::consts::FRAC_1_SQRT_2);
            Some(View::new(
                Node::new(Role::Group, "")
                    .id(format!("{}-icon", options.id))
                    .styled(centered_art(center, size))
                    .fill_contour(icon.contour_dp(size, colors.icon)?),
            ))
        });
        let focus_ring = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-focus-ring", options.id))
                .styled(art_at(0., 0., VISUAL_WIDTH, VISUAL_HEIGHT).border(
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
                    .width(Length::Px(VISUAL_WIDTH))
                    .height(Length::Px(VISUAL_HEIGHT)),
            ),
        )
        .children(
            [track, state_layer, handle]
                .into_iter()
                .chain(icon)
                .chain([focus_ring]),
        )
        .without_interaction(None)
    }

    fn switch_colors(&self, state: ChoiceState, pose: SwitchPose) -> SwitchColors {
        let colors = &self.colors;
        match (state.disabled, state.selected) {
            (true, true) => SwitchColors {
                track: alpha(colors.on_surface, DISABLED_TRACK_ALPHA),
                outline: Color::TRANSPARENT,
                handle: colors.surface,
                icon: alpha(colors.on_surface, DISABLED_MARK_ALPHA),
                layer: Color::TRANSPARENT,
            },
            (true, false) => SwitchColors {
                track: alpha(colors.surface_container_highest, DISABLED_TRACK_ALPHA),
                outline: alpha(colors.on_surface, DISABLED_TRACK_ALPHA),
                handle: alpha(colors.on_surface, DISABLED_MARK_ALPHA),
                icon: alpha(colors.surface_container_highest, DISABLED_MARK_ALPHA),
                layer: Color::TRANSPARENT,
            },
            (false, selected) => {
                let interacting = pose.hovered || pose.focused || pose.pressed;
                if selected {
                    SwitchColors {
                        track: colors.primary,
                        outline: Color::TRANSPARENT,
                        handle: if interacting {
                            colors.primary_container
                        } else {
                            colors.on_primary
                        },
                        icon: colors.primary,
                        layer: colors.primary,
                    }
                } else {
                    SwitchColors {
                        track: colors.surface_container_highest,
                        outline: colors.outline,
                        handle: if interacting {
                            colors.on_surface_variant
                        } else {
                            colors.outline
                        },
                        icon: colors.surface_container_highest,
                        layer: colors.on_surface,
                    }
                }
            }
        }
    }
}

struct SwitchColors {
    track: Color,
    outline: Color,
    handle: Color,
    icon: Color,
    layer: Color,
}

fn centered_art(center: f32, size: f32) -> Style {
    art_at(
        center + FOCUS_INSET - size / 2.,
        VISUAL_HEIGHT / 2. - size / 2.,
        size,
        size,
    )
}

fn art_at(x: f32, y: f32, width: f32, height: f32) -> Style {
    Style::column()
        .width(Length::Px(width))
        .height(Length::Px(height))
        .absolute(Insets {
            left: Length::Px(x),
            top: Length::Px(y),
            ..Insets::default()
        })
        .corners(shape::FULL)
}

fn alpha(color: Color, opacity: u8) -> Color {
    Color(color.0, color.1, color.2, opacity)
}
