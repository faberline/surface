//! Standard and connected Material 3 Expressive button groups.

use surface::{Align, Color, Corners, Insets, Length, Node, Rect, Role, Style, TextStyle};
use surface_app::{CursorStyle, Roving, View};

use crate::{
    button_group_layout::{GroupLayout, Slot, TOUCH},
    motion::STATE_CHANGE,
    shape::{self, LayoutDirection},
    typography, ButtonGroupMotion, Elevation, Theme, VectorIcon,
};

/// The arrangement and interaction of the items in one row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonGroupKind {
    #[default]
    Standard,
    Connected,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonGroupSize {
    ExtraSmall,
    #[default]
    Small,
    Medium,
    Large,
    ExtraLarge,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonGroupShape {
    #[default]
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonGroupAppearance {
    Elevated,
    #[default]
    Filled,
    Tonal,
    Outlined,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ButtonGroupWidth {
    #[default]
    Intrinsic,
    /// Use the outline of the narrow icon-only style.
    Narrow,
    /// Use the outline of the wide icon-only style.
    Wide,
    Fixed(f32),
    /// Distribute spare width by weight after accounting for intrinsic widths.
    Flex(f32),
}

/// The application supplies the current selection on each render.
#[derive(Clone, Copy, Debug, Default)]
pub enum ButtonGroupSelection<'a> {
    #[default]
    Commands,
    Single {
        selected: Option<&'a str>,
        required: bool,
    },
    Multiple {
        selected: &'a [&'a str],
        required: bool,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonGroupOptions<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub kind: ButtonGroupKind,
    pub size: ButtonGroupSize,
    pub shape: ButtonGroupShape,
    pub direction: LayoutDirection,
    pub selection: ButtonGroupSelection<'a>,
    /// Width allocated by the parent. A connected row fills this width
    /// when its items are flexible. Use the available width or a lower cap.
    pub available_width: f32,
    pub appearance: ButtonGroupAppearance,
}

pub struct ButtonGroupItem<'a, M> {
    pub id: &'a str,
    /// Localized accessible name, even for an icon-only item.
    pub label: &'a str,
    pub icon: Option<VectorIcon>,
    pub icon_only: bool,
    /// Override the default appearance in a standard group.
    pub appearance: Option<ButtonGroupAppearance>,
    pub width: ButtonGroupWidth,
    /// Standard groups can mix commands and independent toggle options.
    /// All members of connected groups are selection options.
    pub toggle: bool,
    pub disabled: bool,
    pub on_press: M,
}

/// Optional controlled menu for trailing commands in a standard group.
/// When a menu item sends its action, the application closes the menu.
/// Close it when a resize removes the trigger. The group preserves focus on
/// the command when it moves back into the row.
pub struct ButtonGroupOverflow<'a, M> {
    pub label: &'a str,
    pub open: bool,
    pub on_toggle: M,
    pub on_dismiss: M,
}

impl Theme {
    /// Arrange labeled and icon actions in one line. Connected items share
    /// an appearance and use logical end corners. The optional standard
    /// overflow menu holds trailing commands and leaves toggle items visible.
    /// If minimum sizes cannot fit, the row scrolls horizontally.
    ///
    /// Update [ButtonGroupMotion] before drawing frames for exact intrinsic
    /// label widths, press motion and the measured overflow menu anchor.
    /// Keep control IDs stable and unique. Keep selection and menu visibility
    /// in the application. A required selection suppresses a press that would
    /// clear it, while the selected control remains available to focus.
    /// Each button uses its ID for input and accessibility. The same ID
    /// follows a command into the menu.
    ///
    /// [Material 3 button groups](https://m3.material.io/components/button-groups/guidelines)
    pub fn button_group<'a, M: Clone>(
        &self,
        options: ButtonGroupOptions<'_>,
        items: impl IntoIterator<Item = ButtonGroupItem<'a, M>>,
        motion: &ButtonGroupMotion,
        overflow: Option<ButtonGroupOverflow<'_, M>>,
    ) -> View<M> {
        let items: Vec<_> = items.into_iter().collect();
        let id = options.id;
        let trigger_id = format!("{id}-overflow");
        let kind = options.kind;
        let is_toggle =
            |item: &ButtonGroupItem<'_, M>| kind == ButtonGroupKind::Connected || item.toggle;
        let selected_count = items
            .iter()
            .filter(|item| is_toggle(item) && options.selection.is_selected(item.id))
            .count();
        let layout = GroupLayout::new(options, &items, motion, overflow.is_some(), &trigger_id);
        let row = group_direction(
            Node::new(Role::Group, "").id(format!("{id}-row")).styled(
                Style::row()
                    .align(Align::Center)
                    .width(Length::Px(layout.content_width)),
            ),
            options.direction,
        );
        // The roving group owns visual direction and stable logical order.
        // Put the scrolling viewport above it so moving focus reveals items.
        let mut buttons = Vec::new();
        let first_enabled = items[..layout.visible_items]
            .iter()
            .find(|item| !item.disabled)
            .map(|item| item.id);
        let (visible, hidden) = items.split_at(layout.visible_items);
        for (index, (item, slot)) in visible.iter().zip(&layout.slots).enumerate() {
            let selected = is_toggle(item).then(|| options.selection.is_selected(item.id));
            let refuse_deselection =
                selected == Some(true) && options.selection.required() && selected_count <= 1;
            let corners = corners(
                options,
                motion.is_pressed(item.id),
                selected,
                index,
                layout.visible_items,
            );
            let button = group_button(
                self,
                options,
                item,
                *slot,
                ButtonState {
                    selected,
                    expanded: None,
                    corners,
                },
                (!item.disabled && !refuse_deselection).then(|| item.on_press.clone()),
            );
            let button = button.restore_focus_on_removal(&trigger_id);
            let fallback = visible
                .iter()
                .find(|other| !other.disabled && other.id != item.id);
            buttons.push(if let Some(fallback) = fallback {
                button.restore_focus_in_subtree_on_removal(fallback.id)
            } else {
                button
            });
            append_gap(&mut buttons, slot.gap_after);
        }

        let mut menu = None;
        if let Some(overflow) = overflow.filter(|_| layout.overflow) {
            let trigger = ButtonGroupItem {
                id: &trigger_id,
                label: overflow.label,
                icon: Some(more_icon()),
                icon_only: true,
                appearance: Some(ButtonGroupAppearance::Tonal),
                width: ButtonGroupWidth::Intrinsic,
                toggle: false,
                disabled: false,
                on_press: overflow.on_toggle,
            };
            let position = layout.visible_items;
            let mut button = group_button(
                self,
                options,
                &trigger,
                layout.slots[position],
                ButtonState {
                    selected: None,
                    expanded: Some(overflow.open),
                    corners: corners(
                        options,
                        motion.is_pressed(&trigger_id),
                        None,
                        position,
                        position + 1,
                    ),
                },
                Some(trigger.on_press.clone()),
            );
            if let Some(first) = first_enabled {
                button = button.restore_focus_on_removal(first);
            }
            buttons.push(button);
            if overflow.open {
                let fallback = first_enabled
                    .or_else(|| items.iter().find(|item| !item.disabled).map(|item| item.id))
                    .unwrap_or(&trigger_id);
                let panel = overflow_menu(
                    self,
                    options,
                    &trigger_id,
                    overflow.label,
                    overflow.on_dismiss,
                    motion,
                    hidden,
                );
                menu = Some(panel.modal_focus_fallback(fallback));
            }
        }
        let mut viewport_style = Style::column()
            .width(Length::Px(layout.viewport_width))
            .clip(layout.scroll);
        if layout.scroll && options.direction == LayoutDirection::RightToLeft {
            viewport_style.scroll_x = layout.content_width - layout.viewport_width;
        }
        let mut viewport = View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-viewport"))
                .styled(viewport_style),
        )
        .children([View::new(row)
            .children(buttons)
            .roving_first(Roving::horizontal(false))]);
        if layout.scroll {
            viewport = viewport
                .on_horizontal_scroll(|_, _| None)
                .accessibility_scroll_pages();
        }
        View::new(
            Node::new(Role::Group, options.name)
                .id(id)
                .styled(Style::column().width(Length::Px(layout.viewport_width))),
        )
        .children(
            [viewport, measure_labels(options, &items)]
                .into_iter()
                .chain(menu),
        )
    }
}

impl ButtonGroupSelection<'_> {
    fn is_selected(self, id: &str) -> bool {
        match self {
            Self::Commands => false,
            Self::Single { selected, .. } => selected == Some(id),
            Self::Multiple { selected, .. } => selected.contains(&id),
        }
    }

    fn required(self) -> bool {
        match self {
            Self::Commands => false,
            Self::Single { required, .. } | Self::Multiple { required, .. } => required,
        }
    }
}

pub(super) struct SizeTokens {
    pub height: f32,
    pub font: TextStyle,
    pub label_inset: f32,
    pub text_icon: f32,
    pub icon_only: f32,
    pub icon_gap: f32,
    pub icon_widths: [f32; 3],
    pub standard_gap: f32,
    square: f32,
    pressed: f32,
    connected_inner: f32,
    connected_outer_square: f32,
    connected_pressed: f32,
    outline: f32,
}

impl ButtonGroupSize {
    pub(super) fn tokens(self) -> SizeTokens {
        use ButtonGroupSize::*;
        let index = match self {
            ExtraSmall => 0,
            Small => 1,
            Medium => 2,
            Large => 3,
            ExtraLarge => 4,
        };
        SizeTokens {
            height: [32., 40., 56., 96., 136.][index],
            font: match self {
                ExtraSmall | Small => typography::LABEL_LARGE,
                Medium => typography::TITLE_MEDIUM,
                Large => typography::HEADLINE_SMALL,
                ExtraLarge => typography::HEADLINE_LARGE,
            },
            label_inset: [12., 16., 24., 48., 64.][index],
            text_icon: [20., 20., 24., 32., 40.][index],
            icon_only: [20., 24., 24., 32., 40.][index],
            icon_gap: [8., 8., 8., 12., 16.][index],
            icon_widths: [
                [28., 32., 40.],
                [32., 40., 52.],
                [48., 56., 72.],
                [64., 96., 128.],
                [104., 136., 184.],
            ][index],
            standard_gap: [18., 12., 8., 8., 8.][index],
            square: [12., 12., 16., 28., 28.][index],
            pressed: [8., 8., 12., 16., 16.][index],
            // The official XS token resolves to 8 dp. The published
            // illustration labels the XS square outer corner as 4 dp.
            connected_inner: [8., 8., 8., 16., 20.][index],
            connected_outer_square: [4., 8., 8., 16., 20.][index],
            connected_pressed: [4., 4., 4., 12., 16.][index],
            outline: [1., 1., 1., 2., 3.][index],
        }
    }
}

fn corners(
    options: ButtonGroupOptions<'_>,
    pressed: bool,
    selected: Option<bool>,
    index: usize,
    count: usize,
) -> Corners {
    let tokens = options.size.tokens();
    let round = options.shape == ButtonGroupShape::Round;
    let selected = selected == Some(true);
    if options.kind == ButtonGroupKind::Standard {
        return if pressed {
            Corners::all(tokens.pressed)
        } else if round != selected {
            shape::FULL
        } else {
            Corners::all(tokens.square)
        };
    }
    if selected && !pressed {
        return shape::FULL;
    }
    let outer = if round {
        Corners::FULL_RADIUS
    } else {
        tokens.connected_outer_square
    };
    let inner = if pressed {
        tokens.connected_pressed
    } else {
        tokens.connected_inner
    };
    let start = if index == 0 { outer } else { inner };
    let end = if index + 1 == count { outer } else { inner };
    let (left, right) = match options.direction {
        LayoutDirection::LeftToRight => (start, end),
        LayoutDirection::RightToLeft => (end, start),
    };
    Corners::new(left, right, right, left)
}

struct ButtonState {
    selected: Option<bool>,
    expanded: Option<bool>,
    corners: Corners,
}

fn group_button<M>(
    theme: &Theme,
    options: ButtonGroupOptions<'_>,
    item: &ButtonGroupItem<'_, M>,
    slot: Slot,
    state: ButtonState,
    message: Option<M>,
) -> View<M> {
    let tokens = options.size.tokens();
    let size = slot.face();
    let appearance = if options.kind == ButtonGroupKind::Connected {
        options.appearance
    } else {
        item.appearance.unwrap_or(options.appearance)
    };
    let (fill, ink, outline) = button_colors(theme, appearance, state.selected, item.disabled);
    let has_icon = item.icon.is_some();
    let icon_only = has_icon && item.icon_only;
    let icon_size = if icon_only {
        tokens.icon_only
    } else {
        tokens.text_icon
    };
    let label_space = if has_icon {
        icon_size + tokens.icon_gap
    } else {
        0.
    };
    let max_label = (size - label_space).max(1.);
    let inset = if icon_only {
        0.
    } else {
        tokens
            .label_inset
            .min(((max_label - slot.min_label) / 2.).max(0.))
    };
    let mut face = Style::row()
        .width(Length::Px(size))
        .gap(tokens.icon_gap)
        .align(Align::Center)
        .justify(Align::Center)
        .corners(state.corners)
        .background(fill)
        .state_layer(ink)
        .transition(STATE_CHANGE);
    face.min_height = slot.height;
    if let Some(outline) = outline {
        face = face.border(tokens.outline, outline);
    }
    let icon = item
        .icon
        .as_ref()
        .map(|icon| icon_view(icon, icon_size, ink));
    let label = (!icon_only).then(|| {
        let mut style = Style::column().with_text_style(tokens.font).foreground(ink);
        style.max_width = Length::Px((max_label - 2. * inset).max(1.));
        View::new(
            Node::new(Role::Text, item.label)
                .id(format!("{}-label", item.id))
                .text(item.label)
                .styled(style)
                .shrink_to_fit_width(),
        )
    });
    let mut face_node = group_direction(
        Node::new(Role::Group, "")
            .id(format!("{}-face", item.id))
            .styled(face)
            .focus_color(theme.colors.secondary)
            .focus_outline_from_parent(),
        options.direction,
    );
    if appearance == ButtonGroupAppearance::Elevated && !item.disabled {
        face_node = face_node.box_shadows(theme.elevation(Elevation::Level1));
    }
    let face = View::new(face_node)
        .children(icon.into_iter().chain(label))
        .without_interaction(None);
    let mut hit = Style::row()
        .width(Length::Px(slot.hit))
        .align(Align::Center)
        .justify(Align::Center);
    hit.min_height = TOUCH;
    let mut button = Node::new(Role::Button, item.label)
        .id(item.id)
        .styled(hit)
        .focus_color(Color::TRANSPARENT)
        .disabled(item.disabled);
    if let Some(checked) = state.selected {
        button = button.checked(checked);
    }
    if let Some(expanded) = state.expanded {
        button = button.menu_expanded(expanded);
    }
    let button = View::new(button)
        .children([face])
        .accessibility_scroll_into_view()
        .protect_composition_on_press();
    if let Some(message) = message {
        button
            .on_press(message)
            .accessible_activation()
            .cursor(CursorStyle::Pointer)
    } else {
        button
    }
}

fn group_direction(node: Node, direction: LayoutDirection) -> Node {
    if direction == LayoutDirection::RightToLeft {
        node.reverse_row()
    } else {
        node
    }
}

fn append_gap<M>(children: &mut Vec<View<M>>, gap: f32) {
    if gap > 0. {
        children.push(
            View::new(Node::new(Role::Group, "").styled(Style::column().width(Length::Px(gap))))
                .without_interaction(None),
        );
    }
}

fn button_colors(
    theme: &Theme,
    appearance: ButtonGroupAppearance,
    selected: Option<bool>,
    disabled: bool,
) -> (Color, Color, Option<Color>) {
    use ButtonGroupAppearance::*;
    let c = theme.colors;
    if disabled {
        let outline = (appearance == Outlined && selected != Some(true))
            .then(|| alpha(c.outline_variant, 31));
        return (
            if outline.is_some() {
                Color::TRANSPARENT
            } else {
                alpha(c.on_surface, 31)
            },
            alpha(c.on_surface, 97),
            outline,
        );
    }
    match (appearance, selected) {
        (Filled, Some(false)) => (c.surface_container, c.on_surface_variant, None),
        (Filled, _) | (Elevated, Some(true)) => (c.primary, c.on_primary, None),
        (Elevated, _) => (c.surface_container_low, c.primary, None),
        (Tonal, Some(true)) => (c.secondary, c.on_secondary, None),
        (Tonal, _) => (c.secondary_container, c.on_secondary_container, None),
        (Outlined, Some(true)) => (c.inverse_surface, c.inverse_on_surface, None),
        (Outlined, _) => (
            Color::TRANSPARENT,
            c.on_surface_variant,
            Some(c.outline_variant),
        ),
    }
}

fn alpha(Color(r, g, b, _): Color, opacity: u8) -> Color {
    Color(r, g, b, opacity)
}

fn icon_view<M>(icon: &VectorIcon, size: f32, ink: Color) -> View<M> {
    let size_px = Length::Px(size);
    let mut node =
        Node::new(Role::Group, "").styled(Style::column().width(size_px).height(size_px));
    if let Some(contour) = icon.contour_dp(size, ink) {
        node = node.fill_contour(contour);
    }
    View::new(node)
}

fn more_icon() -> VectorIcon {
    VectorIcon::from_contours([6., 12., 18.].map(|x| {
        [
            [x - 1.6, 12.],
            [x - 1., 10.7],
            [x + 1., 10.7],
            [x + 1.6, 12.],
            [x + 1., 13.3],
            [x - 1., 13.3],
        ]
    }))
    .expect("valid overflow icon")
}

fn measure_labels<M>(options: ButtonGroupOptions<'_>, items: &[ButtonGroupItem<'_, M>]) -> View<M> {
    // Fixed out of the viewport so these text measurements have no paint,
    // scroll extent, focus or accessible nodes.
    let style = Style::column()
        .align(Align::Start)
        .width(Length::Px(16384.))
        .fixed(Insets {
            left: Length::Px(-32768.),
            top: Length::Px(0.),
            ..Insets::default()
        });
    View::new(
        Node::new(Role::Group, "")
            .id(format!("{}-measure", options.id))
            .styled(style),
    )
    .children(
        items
            .iter()
            .filter(|item| !item.icon_only || item.icon.is_none())
            .map(|item| {
                View::new(
                    Node::new(Role::Text, item.label)
                        .id(format!("{}-measure", item.id))
                        .text(item.label)
                        .styled(Style::column().with_text_style(options.size.tokens().font)),
                )
            }),
    )
    .without_interaction(None)
}

fn overflow_menu<M: Clone>(
    theme: &Theme,
    options: ButtonGroupOptions<'_>,
    trigger_id: &str,
    label: &str,
    on_dismiss: M,
    motion: &ButtonGroupMotion,
    items: &[ButtonGroupItem<'_, M>],
) -> View<M> {
    let (anchor, viewport) = motion.menu_anchor();
    let viewport = if viewport.width > 0. && viewport.height > 0. {
        viewport
    } else {
        Rect::new(0., 0., options.available_width.max(48.), 360.)
    };
    let anchor = anchor.unwrap_or(Rect::new(viewport.x, viewport.y, 48., 48.));
    let width = 240_f32.min(viewport.width);
    let height = (items.len() as f32 * TOUCH + 16.).min(viewport.height);
    let right = viewport.x + viewport.width;
    let bottom = viewport.y + viewport.height;
    let x = match options.direction {
        LayoutDirection::LeftToRight => anchor.x + anchor.width - width,
        LayoutDirection::RightToLeft => anchor.x,
    }
    .clamp(viewport.x, right - width);
    let below = anchor.y + anchor.height + 4.;
    let y = if below + height <= bottom {
        below
    } else {
        anchor.y - height - 4.
    }
    .clamp(viewport.y, bottom - height);
    let mut style = Style::column()
        .width(Length::Px(width))
        .height(Length::Px(height))
        .padding(8.)
        .corners(shape::EXTRA_SMALL)
        .clip(true)
        .background(theme.colors.surface_container_high)
        .fixed(Insets {
            left: Length::Px(x),
            top: Length::Px(y),
            ..Insets::default()
        });
    style.z_index = 1001;
    View::new(
        Node::new(Role::Menu, label)
            .id(format!("{}-menu", options.id))
            .styled(style)
            .box_shadows(theme.elevation(Elevation::Level2))
            .tab_index(-1),
    )
    .children(items.iter().map(|item| {
        theme
            .menu_item(item.id, item.label, item.disabled, item.on_press.clone())
            .accessible_activation()
            .restore_focus_on_removal(trigger_id)
    }))
    .roving(Roving::vertical(false))
    .accessibility_scroll_pages()
    .modal(on_dismiss)
    .dismiss_modal_on_tab()
}
