//! Material 3 Expressive extended floating action buttons.

use surface::{Align, Color, Corners, Insets, Length, Node, Rect, Role, Style, TextStyle};
use surface_app::{CursorStyle, View};

use crate::{shape::LayoutDirection, typography, Elevation, ExtendedFabMotion, Theme, VectorIcon};

const MIN_TARGET: f32 = 48.;
const FOCUS_WIDTH: f32 = 3.;
const FOCUS_GAP: f32 = 2.;

/// The three current Expressive extended FAB sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExtendedFabSize {
    #[default]
    Small,
    Medium,
    Large,
}

/// The six Material 3 Expressive extended FAB palettes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExtendedFabAppearance {
    #[default]
    TonalPrimary,
    TonalSecondary,
    TonalTertiary,
    Primary,
    Secondary,
    Tertiary,
}

/// The standard or lowered Material 3 shadow treatment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExtendedFabElevation {
    #[default]
    Standard,
    Lowered,
}

/// Controlled state and parent allocation for one extended FAB.
#[derive(Clone, Copy, Debug)]
pub struct ExtendedFabOptions<'a> {
    pub id: &'a str,
    pub label: &'a str,
    /// Optional longer accessible name. It starts with the displayed label.
    pub name: Option<&'a str>,
    pub size: ExtendedFabSize,
    pub appearance: ExtendedFabAppearance,
    pub elevation: ExtendedFabElevation,
    pub direction: LayoutDirection,
    /// Available logical width. Allow space for the outside focus ring.
    pub available_width: f32,
    pub visible: bool,
    pub disabled: bool,
    /// A stable, enabled action to focus if this button becomes unavailable.
    pub restore_focus_to: Option<&'a str>,
}

pub struct ExtendedFabContent<M> {
    /// A text-only FAB keeps its label when the application requests collapse.
    pub icon: Option<VectorIcon>,
    pub on_press: M,
}

impl Theme {
    /// Build a primary action that retains one stable button and name through
    /// expansion and collapse. Keep one motion value per mounted FAB and
    /// update it with the expanded target, visibility and accepted press
    /// input in [ExtendedFabMotion::advance].
    ///
    /// Put the view where the action belongs. For example, use a trailing
    /// overlay above the navigation bar or a slot in a navigation rail.
    /// Keep the view mounted while a visibility transition is running.
    ///
    /// [Material 3 extended FAB](https://m3.material.io/components/extended-fab/guidelines)
    pub fn extended_fab<M>(
        &self,
        options: ExtendedFabOptions<'_>,
        content: ExtendedFabContent<M>,
        motion: &ExtendedFabMotion,
    ) -> View<M> {
        let tokens = Tokens::new(options.size);
        let has_icon = content.icon.is_some();
        let layout = FabLayout::new(options, &tokens, has_icon, motion.label_size());
        let frame = layout.present(&tokens, options, has_icon, motion);
        let name = options
            .name
            .filter(|name| name.starts_with(options.label))
            .unwrap_or(options.label);
        let mut hit = Style::row()
            .width(Length::Px(frame.face.width.max(MIN_TARGET)))
            .height(Length::Px(frame.face.height.max(MIN_TARGET)))
            .align(Align::Center)
            .justify(Align::Center);
        hit.min_width = MIN_TARGET;
        hit.min_height = MIN_TARGET;
        let button = View::new(
            Node::new(Role::Button, name)
                .id(options.id)
                .styled(hit)
                .focus_color(Color::TRANSPARENT)
                .disabled(options.disabled || !options.visible),
        )
        .children([
            fab_face(self, options, &tokens, frame, content.icon.as_ref(), motion)
                .without_interaction(None),
            measure_labels(options, &tokens, layout.label_width),
        ]);
        let button = if let Some(fallback) = options.restore_focus_to {
            button.restore_focus_on_removal(fallback)
        } else {
            button
        };
        if !options.visible {
            button.without_interaction(options.restore_focus_to)
        } else if options.disabled {
            button
        } else {
            button
                .on_press(content.on_press)
                .accessible_activation()
                .protect_composition_on_press()
                .cursor(CursorStyle::Pointer)
        }
    }
}

struct Tokens {
    size: f32,
    radius: f32,
    icon: f32,
    inset: f32,
    gap: f32,
    font: TextStyle,
}

impl Tokens {
    fn new(size: ExtendedFabSize) -> Self {
        match size {
            ExtendedFabSize::Small => Self {
                size: 56.,
                radius: 16.,
                icon: 24.,
                inset: 16.,
                gap: 8.,
                font: typography::TITLE_MEDIUM,
            },
            ExtendedFabSize::Medium => Self {
                size: 80.,
                radius: 20.,
                icon: 28.,
                inset: 26.,
                gap: 12.,
                font: typography::TITLE_LARGE,
            },
            ExtendedFabSize::Large => Self {
                size: 96.,
                radius: 28.,
                icon: 36.,
                inset: 28.,
                gap: 16.,
                font: typography::HEADLINE_SMALL,
            },
        }
    }
}

struct FabLayout {
    collapsed_width: f32,
    width: f32,
    height: f32,
    label_width: f32,
    label_height: f32,
    x: f32,
    stack: bool,
}

struct Frame {
    face: Rect,
    radius: f32,
    icon: Rect,
    label: Rect,
    label_clip: Rect,
    scale: f32,
    opacity: f32,
    label_opacity: f32,
}

impl FabLayout {
    fn new(
        options: ExtendedFabOptions<'_>,
        tokens: &Tokens,
        has_icon: bool,
        measured: Option<(f32, f32)>,
    ) -> Self {
        let limit = if options.available_width.is_finite() {
            options.available_width.max(MIN_TARGET)
        } else {
            tokens.size
        };
        let collapsed_width = tokens.size.min(limit);
        let units = options
            .label
            .chars()
            .map(|c| if c.is_ascii() { 0.57 } else { 1. })
            .sum::<f32>();
        let natural = measured.map_or(units * tokens.font.font_size, |(width, _)| width);
        let icon = if has_icon { tokens.icon } else { 0. };
        let gap = if has_icon { tokens.gap } else { 0. };
        let accessory = icon + gap;
        let preferred = natural + accessory + 2. * tokens.inset;
        let width = preferred.clamp(collapsed_width, limit);
        let glyph = natural / units.max(1.);
        let pair = natural.min(glyph * 2. + 0.25);
        let inline_room = (width - accessory).max(0.);
        let stack = has_icon && (inline_room < glyph || (inline_room < pair && width >= pair));
        let min_content = if stack {
            pair.max(icon)
        } else {
            pair.min(inline_room) + accessory
        };
        let inset = ((width - min_content) / 2.).clamp(0., tokens.inset);
        let label_width = (width - 2. * inset - if stack { 0. } else { accessory })
            .min(natural)
            .max(1.);
        let text_height = tokens
            .font
            .line_height
            .unwrap_or(tokens.font.font_size * 1.5);
        let label_height = measured.map_or(text_height, |(_, height)| height);
        let content_width = if stack {
            icon.max(label_width)
        } else {
            accessory + label_width
        };
        let content_height = if stack {
            icon + gap + label_height
        } else {
            icon.max(label_height)
        };
        let x = ((width - content_width) / 2.).max(0.);
        let clearance = corner_clearance(tokens.radius.min(width / 2.), x);
        let height = tokens.size.max(content_height + 2. * clearance);
        Self {
            collapsed_width,
            width,
            height,
            label_width,
            label_height,
            x,
            stack,
        }
    }

    fn present(
        &self,
        tokens: &Tokens,
        options: ExtendedFabOptions<'_>,
        has_icon: bool,
        motion: &ExtendedFabMotion,
    ) -> Frame {
        let expanded = if has_icon { motion.expansion() } else { 1. };
        let opacity = motion.visibility_opacity();
        let label_opacity = opacity * if has_icon { motion.label_opacity() } else { 1. };
        let scale = motion.visibility_scale();
        let width = lerp(self.collapsed_width, self.width, expanded);
        let height = lerp(tokens.size, self.height, expanded);
        let radius = tokens.radius.min(width.min(height) / 2.);
        let center_icon = (width - tokens.icon) / 2.;
        let icon_x = if self.stack {
            center_icon
        } else {
            lerp(center_icon, self.x, expanded)
        };
        let stack_offset = (self.label_height + tokens.gap) * expanded / 2.;
        let icon_y = if self.stack {
            ((height - tokens.icon) / 2. - stack_offset).max(0.)
        } else {
            (height - tokens.icon) / 2.
        };
        let label_x = if !has_icon || self.stack {
            (width - self.label_width) / 2.
        } else {
            icon_x + tokens.icon + tokens.gap
        };
        let label_y = if self.stack {
            icon_y + tokens.icon + tokens.gap
        } else {
            (height - self.label_height) / 2.
        };
        let icon = mirror(
            Rect::new(icon_x, icon_y, tokens.icon, tokens.icon),
            width,
            options.direction,
        );
        let label = mirror(
            Rect::new(label_x, label_y, self.label_width, self.label_height),
            width,
            options.direction,
        );
        // Clip only the revealing area during the morph. Text keeps the
        // same wrapping width while the face grows around it.
        let mut label_clip = label.intersect(Rect::new(0., 0., width, height));
        let edges = label_clip.x.min(width - label_clip.x - label_clip.width);
        let clearance = corner_clearance(radius * scale, edges * scale) / scale;
        label_clip = label_clip.intersect(Rect::new(0., clearance, width, height - 2. * clearance));
        Frame {
            face: Rect::new(0., 0., width * scale, height * scale),
            radius: radius * scale,
            icon: scaled(icon, scale),
            label: scaled(label, scale),
            label_clip: scaled(label_clip, scale),
            scale,
            opacity,
            label_opacity,
        }
    }
}

fn corner_clearance(radius: f32, edge_gap: f32) -> f32 {
    // Layout rounds both the face and clip edges. Reserve a pixel before
    // computing the arc so the rounded clip stays inside the contour.
    let inset = (radius - (edge_gap - 1.).max(0.)).clamp(0., radius);
    if inset == 0. {
        0.
    } else {
        (radius - (radius * radius - inset * inset).max(0.).sqrt()).ceil() + 1.
    }
}

fn mirror(rect: Rect, width: f32, direction: LayoutDirection) -> Rect {
    match direction {
        LayoutDirection::LeftToRight => rect,
        LayoutDirection::RightToLeft => {
            Rect::new(width - rect.x - rect.width, rect.y, rect.width, rect.height)
        }
    }
}

fn scaled(rect: Rect, factor: f32) -> Rect {
    Rect::new(
        rect.x * factor,
        rect.y * factor,
        rect.width * factor,
        rect.height * factor,
    )
}

fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

fn fab_face<M>(
    theme: &Theme,
    options: ExtendedFabOptions<'_>,
    tokens: &Tokens,
    frame: Frame,
    icon: Option<&VectorIcon>,
    motion: &ExtendedFabMotion,
) -> View<M> {
    let c = theme.colors;
    let (fill, ink) = if options.disabled {
        (opacity(c.on_surface, 0.12), opacity(c.on_surface, 0.38))
    } else {
        match options.appearance {
            ExtendedFabAppearance::TonalPrimary => (c.primary_container, c.on_primary_container),
            ExtendedFabAppearance::TonalSecondary => {
                (c.secondary_container, c.on_secondary_container)
            }
            ExtendedFabAppearance::TonalTertiary => (c.tertiary_container, c.on_tertiary_container),
            ExtendedFabAppearance::Primary => (c.primary, c.on_primary),
            ExtendedFabAppearance::Secondary => (c.secondary, c.on_secondary),
            ExtendedFabAppearance::Tertiary => (c.tertiary, c.on_tertiary),
        }
    };
    let interaction = motion.interaction();
    let active = options.visible && !options.disabled;
    let focus = active && interaction.focused;
    let layer_alpha = if !active {
        0.
    } else if interaction.pressed || interaction.focused {
        0.10
    } else if interaction.hovered {
        0.08
    } else {
        0.
    };
    let hovered = active && interaction.hovered && !interaction.pressed;
    let level = match (options.disabled, options.elevation, hovered) {
        (true, _, _) => Elevation::Level0,
        (false, ExtendedFabElevation::Standard, true) => Elevation::Level4,
        (false, ExtendedFabElevation::Standard, false) => Elevation::Level3,
        (false, ExtendedFabElevation::Lowered, true) => Elevation::Level2,
        (false, ExtendedFabElevation::Lowered, false) => Elevation::Level1,
    };
    let shadows = theme.elevation(level).map(|mut shadow| {
        shadow.color = opacity(shadow.color, frame.opacity);
        shadow
    });
    let corners = Corners::all(frame.radius);
    let face = Node::new(Role::Group, "")
        .id(format!("{}-face", options.id))
        .styled(
            Style::column()
                .width(Length::Px(frame.face.width))
                .height(Length::Px(frame.face.height))
                .corners(corners)
                .background(opacity(fill, frame.opacity)),
        )
        .box_shadows(shadows);
    let layer = (layer_alpha > 0. && frame.opacity > 0.).then(|| {
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-state-layer", options.id))
                .styled(
                    placed(frame.face)
                        .corners(corners)
                        .background(opacity(ink, layer_alpha * frame.opacity)),
                ),
        )
    });
    let icon = icon.and_then(|icon| {
        let contour = icon.contour_dp(frame.icon.width, opacity(ink, frame.opacity))?;
        Some(View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-icon", options.id))
                .styled(placed(frame.icon))
                .fill_contour(contour),
        ))
    });
    let label = label_view(options, tokens, &frame, ink);
    let indicator = (focus && frame.opacity > 0.).then(|| {
        let outset = FOCUS_GAP + FOCUS_WIDTH;
        let size = frame.face.width + 2. * outset;
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-focus-ring", options.id))
                .styled(
                    placed(Rect::new(
                        -outset,
                        -outset,
                        size,
                        frame.face.height + 2. * outset,
                    ))
                    .corners(Corners::all(frame.radius + outset))
                    .border(FOCUS_WIDTH, opacity(c.secondary, frame.opacity)),
                ),
        )
    });
    View::new(face).children(layer.into_iter().chain(icon).chain(label).chain(indicator))
}

fn label_view<M>(
    options: ExtendedFabOptions<'_>,
    tokens: &Tokens,
    frame: &Frame,
    ink: Color,
) -> Option<View<M>> {
    let clip = frame.label_clip;
    if frame.label_opacity <= 0. || clip.width <= 0. || clip.height <= 0. {
        return None;
    }
    let mut font = tokens.font.clone();
    font.font_size *= frame.scale;
    font.line_height = font
        .line_height
        .map(|line_height| line_height * frame.scale);
    let rect = Rect::new(
        frame.label.x - clip.x,
        frame.label.y - clip.y,
        frame.label.width,
        frame.label.height,
    );
    let label = View::new(
        Node::new(Role::Text, options.label)
            .id(format!("{}-label", options.id))
            .text(options.label)
            .styled(
                placed(rect)
                    .with_text_style(font)
                    .foreground(opacity(ink, frame.label_opacity)),
            ),
    );
    Some(View::new(Node::new(Role::Group, "").styled(placed(clip).clip(true))).children([label]))
}

fn measure_labels<M>(options: ExtendedFabOptions<'_>, tokens: &Tokens, width: f32) -> View<M> {
    let style = Style::column()
        .width(Length::Px(16384.))
        .align(Align::Start)
        .fixed(Insets {
            left: Length::Px(-32768.),
            top: Length::Px(0.),
            ..Insets::default()
        });
    let measure = |suffix, width| {
        View::new(
            Node::new(Role::Text, options.label)
                .id(format!("{}-{suffix}", options.id))
                .text(options.label)
                .styled(
                    Style::column()
                        .width(width)
                        .with_text_style(tokens.font.clone()),
                ),
        )
    };
    View::new(Node::new(Role::Group, "").styled(style))
        .children([
            measure("measure", Length::Auto),
            measure("measure-wrapped", Length::Px(width)),
        ])
        .without_interaction(None)
}

fn placed(rect: Rect) -> Style {
    Style::column()
        .width(Length::Px(rect.width))
        .height(Length::Px(rect.height))
        .absolute(Insets {
            left: Length::Px(rect.x),
            top: Length::Px(rect.y),
            ..Insets::default()
        })
}

fn opacity(Color(r, g, b, a): Color, value: f32) -> Color {
    Color(r, g, b, (f32::from(a) * value).round() as u8)
}
