//! The Material 3 Expressive split button and its optional command menu.

use surface::{
    Align, Color, Corners, FillContour, Insets, Length, Node, Rect, Role, Style, TextStyle,
};
use surface_app::{CursorStyle, Key, Roving, View};

use crate::{
    motion::STATE_CHANGE,
    place_split_button_popup,
    shape::{self, LayoutDirection},
    split_button_motion::Interaction,
    typography, ButtonGroupAppearance, ButtonGroupSize, Elevation, SplitButtonMotion, Theme,
    VectorIcon,
};

const TOUCH: f32 = 48.;
const GAP: f32 = 2.;
const OPTICAL_CENTER: f32 = 0.11;
const MENU_PADDING: f32 = 8.;
const MENU_WIDTH: f32 = 240.;

/// The same size scale used for Material button groups.
pub type SplitButtonSize = ButtonGroupSize;
/// The supported Material container colors.
pub type SplitButtonAppearance = ButtonGroupAppearance;

/// Visual choices and caller-controlled state for one split button.
#[derive(Clone, Copy, Debug)]
pub struct SplitButtonOptions<'a> {
    pub id: &'a str,
    pub size: SplitButtonSize,
    pub appearance: SplitButtonAppearance,
    pub direction: LayoutDirection,
    /// Parent allocation in logical pixels. Supply at least 98 dp for two
    /// full 48 dp targets and the 2 dp space between them.
    pub available_width: f32,
    pub open: bool,
    pub leading_disabled: bool,
    pub trailing_disabled: bool,
    /// Alternate stable focus target if both halves are disabled or removed.
    pub restore_focus_to: Option<&'a str>,
}

/// Content of the leading action. Each form has a localized accessible label.
pub enum SplitButtonLeading<'a> {
    Text(&'a str),
    Icon { label: &'a str, icon: VectorIcon },
    TextAndIcon { label: &'a str, icon: VectorIcon },
}

pub struct SplitButtonContent<'a, M> {
    pub leading: SplitButtonLeading<'a>,
    /// Localized name of the trailing disclosure, for example Export options.
    pub trailing_label: &'a str,
    pub on_primary: M,
    pub on_toggle: M,
    /// Omit the menu to open a caller-provided card or other disclosure.
    pub menu: Option<SplitButtonMenu<'a, M>>,
}

/// Optional flat command menu. Handle dismissal and item messages by closing
/// the application-controlled open state.
pub struct SplitButtonMenu<'a, M> {
    pub label: &'a str,
    pub on_dismiss: M,
    pub items: Vec<SplitButtonMenuItem<'a, M>>,
}

pub struct SplitButtonMenuItem<'a, M> {
    pub id: &'a str,
    pub label: &'a str,
    pub icon: Option<VectorIcon>,
    pub disabled: bool,
    pub on_press: M,
}

impl Theme {
    /// Build two independently named buttons. Only the trailing action opens
    /// the menu. The application owns that state and retains one motion value
    /// per split button. Advance it with the runtime's frame input.
    ///
    /// The built-in menu manages initial focus and returns it on dismissal.
    /// For another kind of popup, use the trailing message and the measured
    /// anchor from [SplitButtonMotion::anchor].
    ///
    /// [Material 3 split button](https://m3.material.io/components/split-button/guidelines)
    pub fn split_button<M: Clone + 'static>(
        &self,
        options: SplitButtonOptions<'_>,
        content: SplitButtonContent<'_, M>,
        motion: &SplitButtonMotion,
    ) -> View<M> {
        let SplitButtonContent {
            leading,
            trailing_label,
            on_primary,
            on_toggle,
            menu,
        } = content;
        let tokens = Tokens::new(options.size);
        let primary_id = format!("{}-leading", options.id);
        let trailer_id = format!("{}-trailing", options.id);
        let (label, icon, show_text) = match &leading {
            SplitButtonLeading::Text(label) => (*label, None, true),
            SplitButtonLeading::Icon { label, icon } => (*label, Some(icon), false),
            SplitButtonLeading::TextAndIcon { label, icon } => (*label, Some(icon), true),
        };
        let widths = Widths::new(
            options.available_width,
            &tokens,
            LeadingMetrics {
                label: show_text.then_some(label),
                has_icon: icon.is_some(),
                label_width: motion.label_width(),
            },
        );
        let height = (motion.content_height() + 2. * widths.vertical_inset).max(tokens.height);
        let open = options.open && !options.trailing_disabled;
        let leading_shape = face_corners(
            options.direction,
            &tokens,
            false,
            false,
            motion.interaction(false),
        );
        let trailing_shape = face_corners(
            options.direction,
            &tokens,
            true,
            open,
            motion.interaction(true),
        );
        let leading_palette = FacePalette::new(self, options.appearance, options.leading_disabled);
        let trailing_palette =
            FacePalette::new(self, options.appearance, options.trailing_disabled);
        let leading_content = leading_content(
            options.id,
            options.direction,
            show_text.then_some(label),
            icon,
            &tokens,
            widths,
            leading_palette.ink,
        );
        let sign = if options.direction == LayoutDirection::RightToLeft {
            -1.
        } else {
            1.
        };
        let chevron = chevron(
            options.id,
            widths.chevron,
            motion.angle_degrees() * sign,
            trailing_palette.ink,
        );
        let trailer_pad = (widths.trailing - widths.chevron) / 2.;
        let leading_face = render_face(
            self,
            FaceOptions {
                id: &format!("{primary_id}-face"),
                width: widths.leading,
                height,
                padding: widths.leading_padding,
                corners: leading_shape,
                palette: leading_palette,
                outline: tokens.outline,
                direction: options.direction,
                open: false,
            },
            leading_content,
        );
        let trailing_face = render_face(
            self,
            FaceOptions {
                id: &format!("{trailer_id}-face"),
                width: widths.trailing,
                height,
                padding: (trailer_pad, trailer_pad),
                corners: trailing_shape,
                palette: trailing_palette,
                outline: tokens.outline,
                direction: options.direction,
                open,
            },
            chevron,
        );

        let leading = button(
            Node::new(Role::Button, label)
                .id(&primary_id)
                .disabled(options.leading_disabled),
            widths.leading,
            leading_face,
            on_primary,
            options.restore_focus_to,
        )
        .restore_focus_on_removal(&trailer_id);
        let leading = if let Some(fallback) = options.restore_focus_to {
            leading.restore_focus_in_subtree_on_removal(fallback)
        } else {
            leading
        };
        let trailer_node = Node::new(Role::Button, trailing_label)
            .id(&trailer_id)
            .disabled(options.trailing_disabled);
        let trailer_node = if menu.is_some() {
            trailer_node.menu_expanded(open)
        } else {
            trailer_node.disclosure_expanded(open)
        };
        let keyboard_toggle = on_toggle.clone();
        let trailing = button(
            trailer_node,
            widths.trailing,
            trailing_face,
            on_toggle,
            options.restore_focus_to,
        )
        .restore_focus_on_removal(&primary_id);
        let trailing = if let Some(fallback) = options.restore_focus_to {
            trailing.restore_focus_in_subtree_on_removal(fallback)
        } else {
            trailing
        };
        let trailing = if menu.is_some() && !open && !options.trailing_disabled {
            trailing.on_key(move |press| {
                (press.key == Key::ArrowDown && press.modifiers.is_empty() && !press.repeat)
                    .then(|| keyboard_toggle.clone())
            })
        } else {
            trailing
        };
        let width = widths.leading + GAP + widths.trailing;
        let row = View::new(directed(
            Node::new(Role::Group, "")
                .id(format!("{}-row", options.id))
                .styled(
                    Style::row()
                        .width(Length::Px(width))
                        .gap(GAP)
                        .align(Align::Center),
                ),
            options.direction,
        ))
        .children([leading, trailing]);
        let measurement = show_text.then(|| measure_label(options.id, label, tokens.font.clone()));
        let fallback = if options.leading_disabled {
            options.restore_focus_to.unwrap_or(&trailer_id)
        } else {
            &primary_id
        };
        let popup = menu
            .filter(|_| open)
            .and_then(|menu| render_menu(self, options, menu, motion, fallback, motion.anchor()?));
        View::new(
            Node::new(Role::Group, "")
                .id(options.id)
                .styled(Style::column().width(Length::Px(width))),
        )
        .children(std::iter::once(row).chain(measurement).chain(popup))
    }
}

fn button<M>(
    node: Node,
    width: f32,
    face: View<M>,
    message: M,
    restore_focus_to: Option<&str>,
) -> View<M> {
    let disabled = node.is_disabled();
    let mut style = Style::row()
        .width(Length::Px(width))
        .align(Align::Center)
        .justify(Align::Center);
    style.min_height = TOUCH;
    let button = View::new(node.styled(style).focus_color(Color::TRANSPARENT))
        .children([face.without_interaction(restore_focus_to)]);
    if disabled {
        button
    } else {
        button
            .on_press(message)
            .accessible_activation()
            .protect_composition_on_press()
            .cursor(CursorStyle::Pointer)
    }
}

struct Tokens {
    height: f32,
    waiting: f32,
    active: f32,
    padding: (f32, f32),
    chevron: f32,
    trailing_padding: f32,
    leading_icon: f32,
    icon_gap: f32,
    font: TextStyle,
    outline: f32,
}

impl Tokens {
    fn new(size: SplitButtonSize) -> Self {
        use ButtonGroupSize::*;
        let index = match size {
            ExtraSmall => 0,
            Small => 1,
            Medium => 2,
            Large => 3,
            ExtraLarge => 4,
        };
        let shared = size.tokens();
        Self {
            height: shared.height,
            waiting: [4., 4., 4., 8., 12.][index],
            active: [8., 12., 12., 20., 20.][index],
            padding: [(12., 10.), (16., 12.), (24., 24.), (48., 48.), (64., 64.)][index],
            chevron: [22., 22., 26., 38., 50.][index],
            trailing_padding: [13., 13., 15., 29., 43.][index],
            leading_icon: shared.text_icon,
            icon_gap: shared.icon_gap,
            font: shared.font,
            outline: [1., 1., 1., 2., 3.][index],
        }
    }
}

struct LeadingMetrics<'a> {
    label: Option<&'a str>,
    has_icon: bool,
    label_width: Option<f32>,
}

#[derive(Clone, Copy)]
struct Widths {
    leading: f32,
    trailing: f32,
    leading_padding: (f32, f32),
    label_width: f32,
    vertical_inset: f32,
    chevron: f32,
    stack: bool,
}

impl Widths {
    fn new(available_width: f32, tokens: &Tokens, metrics: LeadingMetrics<'_>) -> Self {
        let available = if available_width.is_finite() {
            available_width
        } else {
            0.
        };
        let available = available.max(TOUCH * 2. + GAP);
        let units = metrics.label.map_or(0., |label| {
            label
                .chars()
                .map(|c| if c.is_ascii() { 0.57 } else { 1. })
                .sum::<f32>()
        });
        let intrinsic = metrics.label.map_or(0., |_| {
            metrics.label_width.unwrap_or(units * tokens.font.font_size)
        });
        let min_label = intrinsic / units.max(1.);
        let icon = if metrics.has_icon {
            tokens.leading_icon
        } else {
            0.
        };
        let gap = if metrics.has_icon && metrics.label.is_some() {
            tokens.icon_gap
        } else {
            0.
        };
        let minimum_leading = TOUCH.max(min_label.max(icon));
        let trailing = (tokens.chevron + 2. * tokens.trailing_padding)
            .min((available - GAP - minimum_leading).max(TOUCH));
        let leading = (tokens.padding.0 + icon + gap + intrinsic + tokens.padding.1)
            .max(TOUCH)
            .min(available - GAP - trailing);
        // When an inline icon leaves just one large character per line,
        // stack it above the label if that admits a second character. Keep
        // the two distinct button faces in the same row.
        let inline_room = (leading - icon - gap).max(0.);
        let pair = intrinsic.min(2. * min_label + 0.25);
        let stack =
            gap > 0. && (inline_room < min_label || (inline_room < pair && leading >= pair));
        let min_content = if stack {
            pair.max(icon)
        } else {
            pair.min(inline_room) + icon + gap
        };
        let padding = (leading - min_content)
            .min(tokens.padding.0 + tokens.padding.1)
            .max(0.);
        let leading_padding = (
            padding * tokens.padding.0 / (tokens.padding.0 + tokens.padding.1),
            padding * tokens.padding.1 / (tokens.padding.0 + tokens.padding.1),
        );
        let label_width = (leading - padding - if stack { 0. } else { icon + gap })
            .min(intrinsic)
            .max(1.);
        let vertical_inset = if label_width < intrinsic {
            let content = if stack {
                label_width.max(icon)
            } else {
                label_width + icon + gap
            };
            wrapped_content_inset(leading, content, leading_padding, tokens)
        } else {
            0.
        };
        Self {
            leading,
            trailing,
            leading_padding,
            label_width,
            vertical_inset,
            chevron: tokens.chevron.min(trailing - 8.),
            stack,
        }
    }
}

fn wrapped_content_inset(
    width: f32,
    content_width: f32,
    (start, end): (f32, f32),
    tokens: &Tokens,
) -> f32 {
    // Use the largest possible outer radius, even while height is settling.
    // Optical centering always moves away from that outer edge.
    let outer_radius = width / 2.;
    let inner_radius = tokens.active.max(tokens.waiting).min(outer_radius);
    let max_shift = (OPTICAL_CENTER * (outer_radius - tokens.waiting).max(0.)).min(end);
    let spare = (width - content_width - start - end).max(0.) / 2.;
    let clearance = |radius: f32, edge_gap: f32| {
        let inset = (radius - edge_gap).clamp(0., radius);
        radius - (radius * radius - inset * inset).max(0.).sqrt()
    };
    let outer = clearance(outer_radius, start + spare);
    let inner = clearance(inner_radius, end + spare - max_shift);
    outer.max(inner).ceil() + 1.
}

fn face_corners(
    direction: LayoutDirection,
    tokens: &Tokens,
    trailing: bool,
    open: bool,
    interaction: Interaction,
) -> Corners {
    if trailing && open && !interaction.pressed {
        return shape::FULL;
    }
    let inner = if interaction.emphasized {
        tokens.active
    } else {
        tokens.waiting
    };
    let inner_is_left = trailing != (direction == LayoutDirection::RightToLeft);
    let (left, right) = if inner_is_left {
        (inner, Corners::FULL_RADIUS)
    } else {
        (Corners::FULL_RADIUS, inner)
    };
    Corners::new(left, right, right, left)
}

#[derive(Clone, Copy)]
struct FacePalette {
    fill: Color,
    ink: Color,
    border: Option<Color>,
    elevation: Elevation,
}

impl FacePalette {
    fn new(theme: &Theme, style: SplitButtonAppearance, disabled: bool) -> Self {
        let c = theme.colors;
        use ButtonGroupAppearance::*;
        let (fill, ink, border, elevation) = match style {
            Elevated => (c.surface_container_low, c.primary, None, Elevation::Level1),
            Filled => (c.primary, c.on_primary, None, Elevation::Level0),
            Tonal => (
                c.secondary_container,
                c.on_secondary_container,
                None,
                Elevation::Level0,
            ),
            Outlined => (
                Color::TRANSPARENT,
                c.on_surface_variant,
                Some(c.outline_variant),
                Elevation::Level0,
            ),
        };
        if disabled {
            return Self {
                fill: if border.is_some() {
                    Color::TRANSPARENT
                } else {
                    alpha(c.on_surface, 31)
                },
                ink: alpha(c.on_surface, 97),
                border: border.map(|_| alpha(c.on_surface, 31)),
                elevation: Elevation::Level0,
            };
        }
        Self {
            fill,
            ink,
            border,
            elevation,
        }
    }
}

struct FaceOptions<'a> {
    id: &'a str,
    width: f32,
    height: f32,
    padding: (f32, f32),
    corners: Corners,
    palette: FacePalette,
    outline: f32,
    direction: LayoutDirection,
    open: bool,
}

fn render_face<M>(theme: &Theme, options: FaceOptions<'_>, content: View<M>) -> View<M> {
    let FaceOptions {
        id,
        width,
        height,
        padding,
        corners,
        palette,
        outline,
        direction,
        open,
    } = options;
    let resolved = corners.resolve(width, height);
    let (left, right) = match direction {
        LayoutDirection::LeftToRight => padding,
        LayoutDirection::RightToLeft => (padding.1, padding.0),
    };
    let offset = (OPTICAL_CENTER * (resolved.top_left - resolved.top_right)).clamp(-left, right);
    let mut style = Style::row()
        .width(Length::Px(width))
        .justify(Align::Center)
        .align(Align::Center)
        .background(palette.fill)
        .state_layer(palette.ink)
        .corners(corners)
        .transition(STATE_CHANGE);
    style.min_height = height;
    if let Some(border) = palette.border {
        style = style.border(outline, border);
    }
    let state_layer = open.then(|| {
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-open-layer"))
                .styled(
                    Style::column()
                        .absolute(Insets::all(0.))
                        .background(alpha(palette.ink, 26))
                        .corners(corners)
                        .transition(STATE_CHANGE),
                ),
        )
    });
    View::new(
        Node::new(Role::Group, "")
            .id(id)
            .styled(style)
            .box_shadows(theme.elevation(palette.elevation))
            .focus_color(theme.colors.secondary)
            .focus_outline_from_parent(),
    )
    .children(state_layer.into_iter().chain([
        space(left + offset),
        content,
        space(right - offset),
    ]))
}

fn leading_content<M>(
    id: &str,
    direction: LayoutDirection,
    text: Option<&str>,
    icon: Option<&VectorIcon>,
    tokens: &Tokens,
    widths: Widths,
    ink: Color,
) -> View<M> {
    let style = if widths.stack {
        Style::column()
    } else {
        Style::row()
    };
    let mut style = style
        .gap(tokens.icon_gap)
        .align(Align::Center)
        .justify(Align::Center);
    style.max_width =
        Length::Px((widths.leading - widths.leading_padding.0 - widths.leading_padding.1).max(1.));
    let content = directed(
        Node::new(Role::Group, "")
            .id(format!("{id}-leading-content"))
            .styled(style),
        direction,
    );
    let label = text.map(|text| {
        let style = Style::column()
            .with_text_style(tokens.font.clone())
            .foreground(ink)
            .width(Length::Px(widths.label_width));
        View::new(
            Node::new(Role::Text, text)
                .id(format!("{id}-label"))
                .text(text)
                .styled(style),
        )
    });
    View::new(content).children(
        icon.map(|icon| icon_view(icon, tokens.leading_icon, ink))
            .into_iter()
            .chain(label),
    )
}

fn measure_label<M>(id: &str, label: &str, font: TextStyle) -> View<M> {
    // Measure the intrinsic label in the host shaper at the user's scale.
    // This is offscreen, decorative, and outside the normal flow.
    View::new(
        Node::new(Role::Group, "").styled(
            Style::column()
                .width(Length::Px(16384.))
                .align(Align::Start)
                .fixed(Insets {
                    left: Length::Px(-32768.),
                    top: Length::Px(0.),
                    ..Insets::default()
                }),
        ),
    )
    .children([View::new(
        Node::new(Role::Text, label)
            .id(format!("{id}-measure"))
            .text(label)
            .styled(Style::column().with_text_style(font)),
    )])
    .without_interaction(None)
}

fn chevron<M>(id: &str, size: f32, angle: f32, ink: Color) -> View<M> {
    let (sine, cosine) = angle.to_radians().sin_cos();
    let point = |(x, y): (f32, f32)| {
        let (x, y) = (x - 12., y - 12.);
        [
            (x * cosine - y * sine + 12.) * size / 24.,
            (x * sine + y * cosine + 12.) * size / 24.,
        ]
    };
    let points = [
        (6., 9.),
        (12., 15.),
        (18., 9.),
        (16.5, 7.5),
        (12., 12.),
        (7.5, 7.5),
    ]
    .map(point);
    let mut node = Node::new(Role::Group, "")
        .id(format!("{id}-chevron"))
        .styled(icon_style(size));
    if let Some(fill) = FillContour::new(points, ink) {
        node = node.fill_contour(fill);
    }
    View::new(node)
}

fn icon_view<M>(icon: &VectorIcon, size: f32, ink: Color) -> View<M> {
    let mut node = Node::new(Role::Group, "").styled(icon_style(size));
    if let Some(fill) = icon.contour_dp(size, ink) {
        node = node.fill_contour(fill);
    }
    View::new(node)
}

fn icon_style(size: f32) -> Style {
    Style::column()
        .width(Length::Px(size))
        .height(Length::Px(size))
}

fn space<M>(width: f32) -> View<M> {
    View::new(Node::new(Role::Group, "").styled(Style::column().width(Length::Px(width))))
}

fn directed(node: Node, direction: LayoutDirection) -> Node {
    if direction == LayoutDirection::RightToLeft {
        node.reverse_row()
    } else {
        node
    }
}

fn alpha(Color(r, g, b, _): Color, opacity: u8) -> Color {
    Color(r, g, b, opacity)
}

fn render_menu<M: Clone + 'static>(
    theme: &Theme,
    options: SplitButtonOptions<'_>,
    menu: SplitButtonMenu<'_, M>,
    motion: &SplitButtonMotion,
    fallback: &str,
    anchor: crate::SplitButtonAnchor,
) -> Option<View<M>> {
    let desired_height =
        (menu.items.len() as f32 * TOUCH + 2. * MENU_PADDING).max(motion.desired_menu_height());
    let position =
        place_split_button_popup(anchor, (MENU_WIDTH, desired_height), options.direction)?;
    let id = options.id;
    let content = View::new(
        Node::new(Role::Group, "")
            .id(format!("{id}-menu-content"))
            .styled(
                Style::column()
                    .width(Length::Percent(1.))
                    .padding(MENU_PADDING),
            ),
    )
    .children(
        menu.items
            .into_iter()
            .map(|item| menu_item(theme, options.direction, item)),
    );
    let panel = View::new(
        Node::new(Role::Menu, menu.label)
            .id(format!("{id}-menu"))
            .tab_index(-1)
            .styled(menu_style(theme, position))
            .box_shadows(theme.elevation(Elevation::Level2)),
    )
    .children([content])
    .roving(Roving::vertical(false))
    .accessibility_scroll_pages()
    .modal_overlay(menu.on_dismiss)
    .dismiss_modal_on_tab()
    .modal_focus_fallback(fallback);
    Some(if let Some(last) = options.restore_focus_to {
        panel.restore_focus_in_subtree_on_removal(last)
    } else {
        panel
    })
}

fn menu_style(theme: &Theme, rect: Rect) -> Style {
    let mut style = Style::column()
        .width(Length::Px(rect.width))
        .height(Length::Px(rect.height))
        .fixed(Insets {
            left: Length::Px(rect.x),
            top: Length::Px(rect.y),
            ..Insets::default()
        })
        .clip(true)
        .background(theme.colors.surface_container_high)
        .corners(shape::EXTRA_SMALL);
    style.z_index = 1001;
    style
}

fn menu_item<M>(
    theme: &Theme,
    direction: LayoutDirection,
    item: SplitButtonMenuItem<'_, M>,
) -> View<M> {
    let ink = if item.disabled {
        alpha(theme.colors.on_surface, 97)
    } else {
        theme.colors.on_surface
    };
    let mut style = Style::row()
        .width(Length::Percent(1.))
        .padding(8.)
        .gap(12.)
        .align(Align::Center)
        .state_layer(ink);
    style.min_height = TOUCH;
    let icon = item.icon.as_ref().map(|icon| icon_view(icon, 24., ink));
    let label = View::new(
        Node::new(Role::Text, item.label)
            .text(item.label)
            .styled(
                Style::column()
                    .with_text_style(typography::BODY_LARGE)
                    .foreground(ink),
            )
            .shrink_to_fit_width(),
    );
    let control = View::new(directed(
        Node::new(Role::MenuItem, item.label)
            .id(item.id)
            .disabled(item.disabled)
            .styled(style),
        direction,
    ))
    .children(
        icon.into_iter()
            .chain([label])
            .map(|view| view.without_interaction(None)),
    )
    .accessibility_scroll_into_view();
    if item.disabled {
        control
    } else {
        control.on_press(item.on_press).accessible_activation()
    }
}
