use surface::{
    Color, Corners, Length, MeasuredGroupLayout, Node, Rect, Role, Style, TextAlignment,
    TextMeasurer, TextScaling, TextStyle,
};
use surface_app::{CursorStyle, View};

use crate::{shape::LayoutDirection, typography, Elevation, Theme, VectorIcon};

const MIN_TARGET: f32 = 48.;
const FOCUS_WIDTH: f32 = 3.;
const FOCUS_OUTSET: f32 = 2. + FOCUS_WIDTH;
const HOVER: u8 = 20;
const ACTIVE: u8 = 26;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonAppearance {
    Elevated,
    #[default]
    Filled,
    Tonal,
    Outlined,
    Text,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    ExtraSmall,
    #[default]
    Small,
    Medium,
    Large,
    ExtraLarge,
}

/// The shape of a command or an off toggle. An on toggle uses the other shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonShape {
    #[default]
    Round,
    Square,
}

/// A decorative icon at the logical start or end of the text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonIcon<'a> {
    Leading(&'a VectorIcon),
    Trailing(&'a VectorIcon),
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonOptions<'a> {
    pub id: &'a str,
    pub label: &'a str,
    /// A stable localized name. A blank name uses the visible label.
    pub accessible_name: Option<&'a str>,
    pub appearance: ButtonAppearance,
    pub size: ButtonSize,
    pub shape: ButtonShape,
    pub direction: LayoutDirection,
    pub disabled: bool,
    /// The current application-owned toggle state. Text buttons are commands.
    pub checked: Option<bool>,
    pub icon: Option<ButtonIcon<'a>>,
    /// Available width in dp. Reserve space around it for the focus outline.
    pub max_width: f32,
    /// Current text scale. The button measures and paints with the same scale.
    pub text_scale: f32,
}

impl<'a> ButtonOptions<'a> {
    pub fn new(id: &'a str, label: &'a str, max_width: f32) -> Self {
        Self {
            id,
            label,
            accessible_name: None,
            appearance: ButtonAppearance::Filled,
            size: ButtonSize::Small,
            shape: ButtonShape::Round,
            direction: LayoutDirection::LeftToRight,
            disabled: false,
            checked: None,
            icon: None,
            max_width,
            text_scale: 1.,
        }
    }
}

impl Theme {
    /// Build a Material 3 button with one named action and optional vector art.
    ///
    /// Each activation emits the supplied message once. Apply a toggle's next
    /// state in the application. A text appearance supports commands only.
    /// Use a unique stable ID and leave 5 dp around the face for focus.
    /// Returns `None` for blank content or an invalid width, scale or mode.
    pub fn button_with_options<M>(
        &self,
        options: ButtonOptions<'_>,
        message: M,
    ) -> Option<View<M>> {
        if options.id.trim().is_empty()
            || options.label.trim().is_empty()
            || !options.max_width.is_finite()
            || options.max_width < MIN_TARGET
            || !options.text_scale.is_finite()
            || options.text_scale <= 0.
            || (options.appearance == ButtonAppearance::Text && options.checked.is_some())
        {
            return None;
        }
        let tokens = options.size.tokens();
        let font = tokens.font.clone().scaled(options.text_scale);
        if !font.font_size.is_finite() || !font.line_height.is_some_and(f32::is_finite) {
            return None;
        }
        let layout = Layout {
            label: options.label.into(),
            tokens: tokens.clone(),
            font: font.clone(),
            icon_side: options.icon.map(IconSide::from),
            direction: options.direction,
            max_width: options.max_width,
        };
        let palette = Palette::new(self, options);
        let name = options
            .accessible_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(options.label);
        let face_id = format!("{}-face", options.id);
        let mut action_style = Style::column();
        action_style.min_width = MIN_TARGET;
        action_style.min_height = MIN_TARGET;
        action_style.max_width = Length::Px(options.max_width);
        let mut action = Node::new(Role::Button, name)
            .id(options.id)
            .styled(action_style)
            .focus_color(Color::TRANSPARENT)
            .disabled(options.disabled)
            .press_ripple_to(&face_id)
            .measured_group(ButtonLayout(layout.clone()));
        if let Some(checked) = options.checked {
            action = action.checked(checked);
        }
        let label = View::new(
            Node::new(Role::Text, "")
                .id(format!("{}-label", options.id))
                .text(options.label)
                .text_scaling(TextScaling::Fixed)
                .text_alignment(TextAlignment::Center)
                .styled(
                    Style::column()
                        .foreground(palette.ink)
                        .with_text_style(font),
                ),
        );
        let icon = match options.icon {
            Some(ButtonIcon::Leading(art) | ButtonIcon::Trailing(art)) => Some(View::new(
                Node::new(Role::Group, "")
                    .id(format!("{}-icon", options.id))
                    .fill_contour(art.contour_dp(tokens.icon, palette.ink)?),
            )),
            None => None,
        };
        let round = (options.shape == ButtonShape::Round) != (options.checked == Some(true));
        let corners = |outset| {
            if round {
                Corners::full()
            } else {
                Corners::all(tokens.square + outset)
            }
        };
        let press = |outset| (!options.disabled).then_some(Corners::all(tokens.pressed + outset));
        let face = View::new(
            Node::new(Role::Group, "")
                .id(&face_id)
                .styled(
                    Style::column()
                        .corners(corners(0.))
                        .background(palette.fill)
                        .border(
                            palette.outline.map_or(0., |_| tokens.outline),
                            palette.outline.unwrap_or(Color::TRANSPARENT),
                        )
                        .state_layer(palette.state_ink),
                )
                .state_layer_alphas(HOVER, ACTIVE, ACTIVE)
                .box_shadows(self.elevation(palette.elevation))
                .interaction_box_shadows(Some(self.elevation(palette.hover_elevation)), None, None)
                .interaction_box_shadows_from_parent()
                .interaction_corners(None, press(0.), None)
                .interaction_corners_from_parent()
                .animate_interaction_corners_with_spring(800., 0.6)
                .measured_group(layout),
        )
        .children(std::iter::once(label).chain(icon))
        .without_interaction(None);
        let focus = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-focus", options.id))
                .styled(Style::column().corners(corners(FOCUS_OUTSET)))
                .focus_color(self.colors.secondary)
                .focus_outline_width(FOCUS_WIDTH)
                .focus_outline_from_parent()
                .interaction_corners(None, press(FOCUS_OUTSET), None)
                .interaction_corners_from_parent()
                .animate_interaction_corners_with_spring(800., 0.6),
        )
        .without_interaction(None);
        let button = View::new(action).children([face, focus]);
        if options.disabled {
            return Some(button);
        }
        Some(
            button
                .on_press(message)
                .accessible_activation()
                .accessibility_scroll_into_view()
                .focus_on_pointer_release()
                .protect_composition_on_press()
                .suppress_repeated_activation()
                .cursor(CursorStyle::Pointer),
        )
    }
}

#[derive(Clone, Debug)]
struct Tokens {
    height: f32,
    side: f32,
    icon: f32,
    gap: f32,
    square: f32,
    pressed: f32,
    outline: f32,
    font: TextStyle,
}

impl ButtonSize {
    fn tokens(self) -> Tokens {
        // Google M3 Button Size 399c, DSDB 41.1.12. The glyph gap of
        // a standalone extra-small button is 4 dp.
        let (height, side, icon, gap, square, pressed, outline, font) = match self {
            Self::ExtraSmall => (32., 12., 20., 4., 12., 8., 1., typography::LABEL_LARGE),
            Self::Small => (40., 16., 20., 8., 12., 8., 1., typography::LABEL_LARGE),
            Self::Medium => (56., 24., 24., 8., 16., 12., 1., typography::TITLE_MEDIUM),
            Self::Large => (96., 48., 32., 12., 28., 16., 2., typography::HEADLINE_SMALL),
            Self::ExtraLarge => (
                136.,
                64.,
                40.,
                16.,
                28.,
                16.,
                3.,
                typography::HEADLINE_LARGE,
            ),
        };
        Tokens {
            height,
            side,
            icon,
            gap,
            square,
            pressed,
            outline,
            font,
        }
    }
}

struct Palette {
    fill: Color,
    ink: Color,
    outline: Option<Color>,
    state_ink: Color,
    elevation: Elevation,
    hover_elevation: Elevation,
}

impl Palette {
    fn new(theme: &Theme, options: ButtonOptions<'_>) -> Self {
        use ButtonAppearance::{Elevated, Filled, Outlined, Text, Tonal};
        let c = theme.colors;
        let outline = (options.appearance == Outlined && options.checked != Some(true))
            .then_some(c.outline_variant);
        if options.disabled {
            let Color(r, g, b, _) = c.on_surface;
            return Self {
                fill: Color(r, g, b, 26),
                ink: Color(r, g, b, 97),
                outline,
                state_ink: Color::TRANSPARENT,
                elevation: Elevation::Level0,
                hover_elevation: Elevation::Level0,
            };
        }
        let (fill, ink) = match (options.appearance, options.checked) {
            (Elevated, Some(true)) | (Filled, None | Some(true)) => (c.primary, c.on_primary),
            (Elevated, _) => (c.surface_container_low, c.primary),
            (Filled, Some(false)) => (c.surface_container, c.on_surface_variant),
            (Tonal, Some(true)) => (c.secondary, c.on_secondary),
            (Tonal, _) => (c.secondary_container, c.on_secondary_container),
            (Outlined, Some(true)) => (c.inverse_surface, c.inverse_on_surface),
            (Outlined, _) => (Color::TRANSPARENT, c.on_surface_variant),
            (Text, _) => (Color::TRANSPARENT, c.primary),
        };
        let (elevation, hover_elevation) = match options.appearance {
            Elevated => (Elevation::Level1, Elevation::Level2),
            Filled | Tonal => (Elevation::Level0, Elevation::Level1),
            Outlined | Text => (Elevation::Level0, Elevation::Level0),
        };
        Self {
            fill,
            ink,
            outline,
            state_ink: ink,
            elevation,
            hover_elevation,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum IconSide {
    Leading,
    Trailing,
}

impl From<ButtonIcon<'_>> for IconSide {
    fn from(icon: ButtonIcon<'_>) -> Self {
        match icon {
            ButtonIcon::Leading(_) => Self::Leading,
            ButtonIcon::Trailing(_) => Self::Trailing,
        }
    }
}

#[derive(Clone, Debug)]
struct Layout {
    label: String,
    tokens: Tokens,
    font: TextStyle,
    icon_side: Option<IconSide>,
    direction: LayoutDirection,
    max_width: f32,
}

struct Content {
    width: f32,
    height: f32,
    label_width: f32,
    label_height: f32,
    icon: f32,
    extra: f32,
    vertical: bool,
}

impl Layout {
    fn content(&self, available: f32, fonts: &mut dyn TextMeasurer) -> Content {
        let natural = fonts.measure_text(&self.label, &self.font, None).0.max(1.);
        let minimum_label = natural.min(self.font.font_size.max(1.));
        let icon = self.icon_side.map_or(0., |_| self.tokens.icon);
        let gap = if icon > 0. { self.tokens.gap } else { 0. };
        let extra = icon + gap;
        let width = (natural + extra + 2. * self.tokens.side)
            .ceil()
            .min(available);
        let vertical = icon > 0. && width < extra + minimum_label;
        let usable = if vertical { width } else { width - extra };
        let side = ((usable - natural.min(2. * minimum_label)) / 2.).clamp(0., self.tokens.side);
        let label_width = natural.min((usable - 2. * side).max(1.));
        let label_height = fonts
            .measure_text(&self.label, &self.font, Some(label_width))
            .1
            .ceil();
        let content_height = if vertical {
            label_height + extra
        } else {
            label_height.max(icon)
        };
        let rule_inset = ((self.tokens.height
            - self
                .tokens
                .font
                .line_height
                .unwrap_or(self.tokens.font.font_size)
                .max(icon))
            / 2.)
            .max(4.);
        Content {
            width,
            height: self.tokens.height.max(content_height + 2. * rule_inset),
            label_width,
            label_height,
            icon,
            extra,
            vertical,
        }
    }

    fn art_left(&self) -> bool {
        matches!(
            (self.icon_side, self.direction),
            (Some(IconSide::Leading), LayoutDirection::LeftToRight)
                | (Some(IconSide::Trailing), LayoutDirection::RightToLeft)
        )
    }
}

impl MeasuredGroupLayout for Layout {
    fn measure(
        &self,
        _children: &[&Node],
        available_width: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let content = self.content(available_width.unwrap_or(self.max_width), fonts);
        (content.width, content.height)
    }

    fn place(
        &self,
        _children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let content = self.content(width, fonts);
        let label_width = content.label_width;
        let label_height = content.label_height;
        let icon = content.icon;
        let (label, artwork) = if content.vertical {
            let y = (height - label_height - content.extra) / 2.;
            let after = matches!(self.icon_side, Some(IconSide::Trailing));
            let label_y = if after { y } else { y + content.extra };
            let art_y = if after {
                y + label_height + self.tokens.gap
            } else {
                y
            };
            (
                Rect::new(
                    (width - label_width) / 2.,
                    label_y,
                    label_width,
                    label_height,
                ),
                Rect::new((width - icon) / 2., art_y, icon, icon),
            )
        } else {
            let start = (width - label_width - content.extra) / 2.;
            let (label_x, icon_x) = if self.art_left() {
                (start + content.extra, start)
            } else {
                (start, start + label_width + self.tokens.gap)
            };
            (
                Rect::new(
                    label_x,
                    (height - label_height) / 2.,
                    label_width,
                    label_height,
                ),
                Rect::new(icon_x, (height - icon) / 2., icon, icon),
            )
        };
        let mut result = vec![label];
        if self.icon_side.is_some() {
            result.push(artwork);
        }
        result
    }
}

#[derive(Debug)]
struct ButtonLayout(Layout);

impl MeasuredGroupLayout for ButtonLayout {
    fn places_measured_groups(&self) -> bool {
        true
    }

    fn measure(
        &self,
        _children: &[&Node],
        available_width: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let limit = available_width
            .unwrap_or(self.0.max_width)
            .min(self.0.max_width);
        let content = self.0.content(limit.max(MIN_TARGET), fonts);
        (
            content.width.max(MIN_TARGET),
            content.height.max(MIN_TARGET),
        )
    }

    fn place(
        &self,
        _children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let content = self.0.content(width, fonts);
        let x = (width - content.width) / 2.;
        let y = (height - content.height) / 2.;
        vec![
            Rect::new(x, y, content.width, content.height),
            Rect::new(
                x - FOCUS_OUTSET,
                y - FOCUS_OUTSET,
                content.width + 2. * FOCUS_OUTSET,
                content.height + 2. * FOCUS_OUTSET,
            ),
        ]
    }
}
