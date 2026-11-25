use surface::{Align, Color, Corners, Insets, Length, Node, Role, Style};
use surface_app::{CursorStyle, View};

use crate::{IconButtonKind, Theme, VectorIcon};

const MIN_TARGET: f32 = 48.;
const FOCUS_STROKE: f32 = 3.;
const FOCUS_OUTSET: f32 = 2. + FOCUS_STROKE;
// Material 3 Expressive Fast Spatial.
const CORNER_STIFFNESS: f32 = 800.;
const CORNER_DAMPING_RATIO: f32 = 0.6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonSize {
    ExtraSmall,
    #[default]
    Small,
    Medium,
    Large,
    ExtraLarge,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonWidth {
    Narrow,
    #[default]
    Default,
    Wide,
}

/// The resting shape of a command or an off toggle. An on toggle inverts it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonShape {
    #[default]
    Round,
    Square,
}

/// Vector artwork for a command or a caller-controlled toggle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialIcon<'a> {
    Command(&'a VectorIcon),
    Toggle {
        checked: bool,
        off: &'a VectorIcon,
        on: &'a VectorIcon,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct IconButtonOptions<'a> {
    pub id: &'a str,
    /// A localized action name that stays the same for both toggle states.
    pub name: &'a str,
    /// The stable ID of a visible tooltip or passive text description.
    pub description: Option<&'a str>,
    pub kind: IconButtonKind,
    pub size: IconButtonSize,
    pub width: IconButtonWidth,
    pub shape: IconButtonShape,
    pub disabled: bool,
}

impl Theme {
    /// Build a standalone Material 3 icon button with one named action.
    ///
    /// Use a unique, stable ID and reserve its `-face`, `-icon` and `-focus`
    /// child IDs. Leave room around the button for its focus outline. The
    /// application updates a toggle's checked value when it handles the
    /// message. Returns `None` for an empty ID or name, or artwork that
    /// cannot be drawn at the token size.
    pub fn icon_button_with_options<M>(
        &self,
        options: IconButtonOptions<'_>,
        icon: MaterialIcon<'_>,
        message: M,
    ) -> Option<View<M>> {
        if options.id.trim().is_empty() || options.name.trim().is_empty() {
            return None;
        }
        let (asset, checked) = match icon {
            MaterialIcon::Command(asset) => (asset, None),
            MaterialIcon::Toggle { checked, off, on } => {
                (if checked { on } else { off }, Some(checked))
            }
        };
        let tokens = options.size.tokens();
        let [narrow, default, wide] = tokens.widths;
        let width = match options.width {
            IconButtonWidth::Narrow => narrow,
            IconButtonWidth::Default => default,
            IconButtonWidth::Wide => wide,
        };
        let height = tokens.height;
        let target_width = width.max(MIN_TARGET);
        let target_height = height.max(MIN_TARGET);
        let palette = Palette::resolve(self, options.kind, checked, options.disabled);
        let radius = if (options.shape == IconButtonShape::Round) != (checked == Some(true)) {
            width.min(height) / 2.
        } else {
            tokens.square_radius
        };

        let pressed_corners =
            |outset| (!options.disabled).then_some(Corners::all(tokens.pressed_radius + outset));

        let icon_view = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-icon", options.id))
                .styled(centered_box(tokens.icon, tokens.icon))
                .fill_contour(asset.contour_dp(tokens.icon, palette.icon)?),
        );
        let face_id = format!("{}-face", options.id);
        let face = View::new(
            Node::new(Role::Group, "")
                .id(&face_id)
                .styled(
                    centered_box(width, height)
                        .corners(Corners::all(radius))
                        .background(palette.fill)
                        .border(
                            palette.outline.map_or(0., |_| tokens.outline),
                            palette.outline.unwrap_or(Color::TRANSPARENT),
                        )
                        .state_layer(if options.disabled {
                            Color::TRANSPARENT
                        } else {
                            palette.icon
                        }),
                )
                // Keep the pressed target with the passive face. The parent
                // owns input and will supply its pressed state here.
                .interaction_corners(None, pressed_corners(0.), None)
                .interaction_corners_from_parent()
                .animate_interaction_corners_with_spring(CORNER_STIFFNESS, CORNER_DAMPING_RATIO)
                .state_layer_alphas(20, 26, 26),
        )
        .children([icon_view])
        .without_interaction(None);
        let focus = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-focus", options.id))
                .styled(
                    centered_box(width + 2. * FOCUS_OUTSET, height + 2. * FOCUS_OUTSET)
                        .absolute(Insets {
                            left: Length::Px((target_width - width) / 2. - FOCUS_OUTSET),
                            top: Length::Px((target_height - height) / 2. - FOCUS_OUTSET),
                            ..Insets::default()
                        })
                        .corners(Corners::all(radius + FOCUS_OUTSET)),
                )
                // The focus outline uses the same target plus its outset.
                .interaction_corners(None, pressed_corners(FOCUS_OUTSET), None)
                .interaction_corners_from_parent()
                .animate_interaction_corners_with_spring(CORNER_STIFFNESS, CORNER_DAMPING_RATIO)
                .focus_outline_from_parent()
                .focus_color(self.colors.secondary)
                .focus_outline_width(FOCUS_STROKE),
        )
        .without_interaction(None);

        let mut action = Node::new(Role::Button, options.name)
            .id(options.id)
            .styled(centered_box(target_width, target_height))
            .disabled(options.disabled)
            .focus_color(Color::TRANSPARENT)
            .press_ripple_to(face_id);
        if let Some(value) = checked {
            action = action.checked(value);
        }
        if let Some(target) = options.description {
            action = action.described_by_text(target);
        }
        let mut view = View::new(action).children([face, focus]);
        if options.disabled {
            return Some(view);
        }
        if let Some(target) = options.description {
            view = view.described_by(target);
        }
        Some(
            view.on_press(message)
                .accessible_activation()
                .accessibility_scroll_into_view()
                .focus_on_pointer_release()
                .protect_composition_on_press()
                .suppress_repeated_activation()
                .cursor(CursorStyle::Pointer),
        )
    }
}

fn centered_box(width: f32, height: f32) -> Style {
    let mut style = Style::row()
        .width(Length::Px(width))
        .height(Length::Px(height))
        .align(Align::Center)
        .justify(Align::Center);
    style.min_width = width;
    style.min_height = height;
    style
}

struct SizeTokens {
    height: f32,
    icon: f32,
    widths: [f32; 3],
    square_radius: f32,
    pressed_radius: f32,
    outline: f32,
}

impl IconButtonSize {
    fn tokens(self) -> SizeTokens {
        let (height, icon, widths, square_radius, pressed_radius, outline) = match self {
            Self::ExtraSmall => (32., 20., [28., 32., 40.], 12., 8., 1.),
            Self::Small => (40., 24., [32., 40., 52.], 12., 8., 1.),
            Self::Medium => (56., 24., [48., 56., 72.], 16., 12., 1.),
            Self::Large => (96., 32., [64., 96., 128.], 28., 16., 2.),
            Self::ExtraLarge => (136., 40., [104., 136., 184.], 28., 16., 3.),
        };
        SizeTokens {
            height,
            icon,
            widths,
            square_radius,
            pressed_radius,
            outline,
        }
    }
}

struct Palette {
    fill: Color,
    icon: Color,
    outline: Option<Color>,
}

impl Palette {
    fn resolve(theme: &Theme, kind: IconButtonKind, checked: Option<bool>, disabled: bool) -> Self {
        use IconButtonKind::{Filled, Outlined, Standard, Tonal};
        let colors = theme.colors;
        let transparent = Color::TRANSPARENT;
        let (fill, icon) = match (kind, checked) {
            (Filled, Some(false)) => (colors.surface_container, colors.on_surface_variant),
            (Filled, _) => (colors.primary, colors.on_primary),
            (Tonal, Some(true)) => (colors.secondary, colors.on_secondary),
            (Tonal, _) => (colors.secondary_container, colors.on_secondary_container),
            (Outlined, Some(true)) => (colors.inverse_surface, colors.inverse_on_surface),
            (Standard, Some(true)) => (transparent, colors.primary),
            (Outlined | Standard, _) => (transparent, colors.on_surface_variant),
        };
        let outline = (kind == Outlined && checked != Some(true)).then_some(colors.outline_variant);
        if disabled {
            let Color(r, g, b, _) = colors.on_surface;
            Self {
                fill: if fill == transparent {
                    transparent
                } else {
                    Color(r, g, b, 26)
                },
                icon: Color(r, g, b, 97),
                outline,
            }
        } else {
            Self {
                fill,
                icon,
                outline,
            }
        }
    }
}
