//! Material 3 Expressive square floating action buttons.

use surface::{Align, Color, Corners, Insets, Length, Node, Rect, Role, Style};
use surface_app::{CursorStyle, View};

use crate::{shape, Elevation, ExpressiveFabMotion, Theme, VectorIcon};

pub use crate::extended_fab::{
    ExtendedFabAppearance as ExpressiveFabAppearance,
    ExtendedFabElevation as ExpressiveFabElevation,
};

const FOCUS_GAP: f32 = 2.;
const FOCUS_WIDTH: f32 = 3.;
const HOVER_OPACITY: f32 = 0.08;
const PRESS_OPACITY: f32 = 0.10;
const FOCUS_OPACITY: f32 = 0.10;

/// Current Expressive size tokens for a square icon-only FAB.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExpressiveFabSize {
    /// The 56 dp FAB.
    #[default]
    Regular,
    Medium,
    Large,
}

/// Application-controlled options for an icon-only primary action.
#[derive(Clone, Copy, Debug)]
pub struct ExpressiveFabOptions<'a> {
    pub id: &'a str,
    /// A localized name that describes what the action does.
    pub name: &'a str,
    pub size: ExpressiveFabSize,
    pub appearance: ExpressiveFabAppearance,
    pub elevation: ExpressiveFabElevation,
    /// If false, immediately remove the view from input, focus and accessibility.
    pub visible: bool,
    pub disabled: bool,
    /// An enabled action that can receive focus if this FAB becomes unavailable.
    pub restore_focus_to: Option<&'a str>,
}

pub struct ExpressiveFabContent<M> {
    /// A filled glyph. The button supplies the accessible name.
    pub icon: VectorIcon,
    pub on_press: M,
}

impl Theme {
    /// Build one named action with the Expressive square FAB tokens.
    ///
    /// Keep one motion value per stable ID and advance it with the view's
    /// visibility target and the accepted input from
    /// [surface_app::Application::animation_frame_with_press_detail].
    /// Initialize it with [ExpressiveFabMotion::with_visibility] when the
    /// action starts hidden.
    /// Keep this view mounted during an exit so its last pose can fade.
    ///
    /// The parent controls placement and reserves space for the outer shadow
    /// and focus indicator. Add a plain tooltip with [View::described_by]
    /// for a Web icon label, or mark the returned view with
    /// [View::menu_expanded] when it opens a menu.
    ///
    /// [Material 3 FAB](https://m3.material.io/components/floating-action-button/specs)
    pub fn expressive_fab<M>(
        &self,
        options: ExpressiveFabOptions<'_>,
        content: ExpressiveFabContent<M>,
        motion: &ExpressiveFabMotion,
    ) -> View<M> {
        let pose = FabPose::from(options.size);
        let face = fab_face(self, options, pose, &content.icon, motion, None);
        fab_action(
            options,
            pose.side,
            face,
            content.on_press,
            motion.visibility_opacity() > 0.,
        )
    }
}

#[derive(Clone, Copy)]
pub(super) struct FabPose {
    pub(super) side: f32,
    pub(super) radius: f32,
    pub(super) icon: f32,
}

impl From<ExpressiveFabSize> for FabPose {
    fn from(size: ExpressiveFabSize) -> Self {
        match size {
            ExpressiveFabSize::Regular => Self {
                side: 56.,
                radius: shape::LARGE.top_left,
                icon: 24.,
            },
            ExpressiveFabSize::Medium => Self {
                side: 80.,
                radius: shape::LARGE_INCREASED.top_left,
                icon: 28.,
            },
            ExpressiveFabSize::Large => Self {
                side: 96.,
                radius: shape::EXTRA_LARGE.top_left,
                icon: 36.,
            },
        }
    }
}

impl FabPose {
    fn scaled(self, scale: f32) -> Self {
        Self {
            side: (self.side * scale).round(),
            radius: self.radius * scale,
            icon: self.icon * scale,
        }
    }
}

pub(super) fn fab_action<M>(
    options: ExpressiveFabOptions<'_>,
    side: f32,
    face: View<M>,
    action: M,
    painted: bool,
) -> View<M> {
    let available = options.visible && painted;
    let mut hit = square(side).align(Align::Center).justify(Align::Center);
    hit.min_width = side;
    hit.min_height = side;
    let control = View::new(
        Node::new(Role::Button, options.name)
            .id(options.id)
            .styled(hit)
            .focus_color(Color::TRANSPARENT)
            .disabled(options.disabled || !available),
    )
    .children([face.without_interaction(None)]);
    let control = match options.restore_focus_to {
        Some(fallback) => control.restore_focus_on_removal(fallback),
        None => control,
    };
    if !available {
        control.without_interaction(options.restore_focus_to)
    } else if options.disabled {
        control
    } else {
        control
            .on_press(action)
            .accessible_activation()
            .protect_composition_on_press()
            .cursor(CursorStyle::Pointer)
    }
}

pub(super) fn fab_face<M>(
    theme: &Theme,
    options: ExpressiveFabOptions<'_>,
    pose: FabPose,
    icon: &VectorIcon,
    motion: &ExpressiveFabMotion,
    colors: Option<(Color, Color)>,
) -> View<M> {
    let mode = motion.interaction();
    let opacity = motion.visibility_opacity();
    let pose = pose.scaled(motion.visibility_scale());
    let active = options.visible && !options.disabled;
    let focus = active && mode.focused;
    let hover = active && mode.hovered && !mode.pressed;
    let level = match (options.disabled, options.elevation, hover) {
        (true, _, _) => Elevation::Level0,
        (false, ExpressiveFabElevation::Standard, true) => Elevation::Level4,
        (false, ExpressiveFabElevation::Standard, false) => Elevation::Level3,
        (false, ExpressiveFabElevation::Lowered, true) => Elevation::Level2,
        (false, ExpressiveFabElevation::Lowered, false) => Elevation::Level1,
    };
    let (fill, ink) = colors.unwrap_or_else(|| fab_colors(theme, options));
    let corners = Corners::all(pose.radius);
    let shadows = theme.elevation(level).map(|mut shadow| {
        shadow.color = tint(shadow.color, opacity);
        shadow
    });
    let background = Node::new(Role::Group, "")
        .id(format!("{}-face", options.id))
        .styled(
            square(pose.side)
                .corners(corners)
                .background(tint(fill, opacity)),
        )
        .box_shadows(shadows);
    let face_rect = Rect::new(0., 0., pose.side, pose.side);
    let ripple_state = motion.ripple().filter(|_| active);
    let ripple = ripple_state.as_ref().and_then(|ripple| {
        let color = tint(ink, PRESS_OPACITY * ripple.opacity * opacity);
        let contour = ripple.contour(pose.side, pose.radius, color)?;
        Some(View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-ripple", options.id))
                .styled(placed(face_rect))
                .fill_contour(contour),
        ))
    });
    let layer_opacity = if !active || ripple_state.is_some() {
        0.
    } else if mode.pressed {
        PRESS_OPACITY
    } else if focus {
        FOCUS_OPACITY
    } else if hover {
        HOVER_OPACITY
    } else {
        0.
    };
    let layer = (layer_opacity > 0. && opacity > 0.).then(|| {
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-state-layer", options.id))
                .styled(
                    placed(face_rect)
                        .corners(corners)
                        .background(tint(ink, layer_opacity * opacity)),
                ),
        )
    });
    let icon = icon
        .contour_dp(pose.icon, tint(ink, opacity))
        .map(|contour| {
            let origin = (pose.side - pose.icon) / 2.;
            View::new(
                Node::new(Role::Group, "")
                    .id(format!("{}-icon", options.id))
                    .styled(placed(Rect::new(origin, origin, pose.icon, pose.icon)))
                    .fill_contour(contour),
            )
        });
    let indicator = (focus && opacity > 0.).then(|| {
        let outset = FOCUS_GAP + FOCUS_WIDTH;
        let side = pose.side + 2. * outset;
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-focus-ring", options.id))
                .styled(
                    placed(Rect::new(-outset, -outset, side, side))
                        .corners(Corners::all(pose.radius + outset))
                        .border(FOCUS_WIDTH, tint(theme.colors.secondary, opacity)),
                ),
        )
    });
    View::new(background).children(layer.into_iter().chain(ripple).chain(icon).chain(indicator))
}

pub(super) fn fab_colors(theme: &Theme, options: ExpressiveFabOptions<'_>) -> (Color, Color) {
    let c = theme.colors;
    if options.disabled {
        return (tint(c.on_surface, 0.12), tint(c.on_surface, 0.38));
    }
    match options.appearance {
        ExpressiveFabAppearance::TonalPrimary => (c.primary_container, c.on_primary_container),
        ExpressiveFabAppearance::TonalSecondary => {
            (c.secondary_container, c.on_secondary_container)
        }
        ExpressiveFabAppearance::TonalTertiary => (c.tertiary_container, c.on_tertiary_container),
        ExpressiveFabAppearance::Primary => (c.primary, c.on_primary),
        ExpressiveFabAppearance::Secondary => (c.secondary, c.on_secondary),
        ExpressiveFabAppearance::Tertiary => (c.tertiary, c.on_tertiary),
    }
}

fn square(side: f32) -> Style {
    Style::row()
        .width(Length::Px(side))
        .height(Length::Px(side))
}

fn placed(rect: Rect) -> Style {
    Style::row()
        .width(Length::Px(rect.width))
        .height(Length::Px(rect.height))
        .absolute(Insets {
            left: Length::Px(rect.x),
            top: Length::Px(rect.y),
            ..Insets::default()
        })
}

fn tint(Color(r, g, b, a): Color, opacity: f32) -> Color {
    Color(r, g, b, (f32::from(a) * opacity).round() as u8)
}
