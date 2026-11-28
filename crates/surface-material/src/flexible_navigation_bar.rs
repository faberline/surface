//! Compact and medium Material 3 Expressive navigation bars.

use surface::{
    Align, Color, Length, MeasuredGroupLayout, Node, Rect, Role, Style, TextAlignment,
    TextMeasurer, TextStyle,
};
use surface_app::{CursorStyle, View};

use crate::{
    adaptive::WindowWidthSizeClass,
    components::navigation_indicator,
    motion::STATE_CHANGE,
    shape::{self, LayoutDirection},
    typography,
    vector_icon::VectorIconSize,
    ChoiceState, Elevation, NavigationRailItem, Theme,
};

const MIN_HEIGHT: f32 = 64.;
const INDICATOR_WIDTH: f32 = 56.;
const VERTICAL_PADDING: f32 = 6.;
const LABEL_GAP: f32 = 4.;
const LABEL_INSET: f32 = 4.;
const DISABLED_ALPHA: u8 = 97;
const HORIZONTAL_CELL_WIDTH: f32 = 112.;
const HORIZONTAL_ICON_SIZE: f32 = 24.;
const HORIZONTAL_PILL_INSET: f32 = 16.;
const HORIZONTAL_PILL_Y_PADDING: f32 = 8.;
const HORIZONTAL_BAR_Y_PADDING: f32 = 12.;
const HORIZONTAL_ART_WIDTH: f32 = 2. * HORIZONTAL_PILL_INSET + HORIZONTAL_ICON_SIZE + LABEL_GAP;

/// The stable identity and layout of a flexible navigation bar.
pub struct FlexibleNavigationBarOptions<'a> {
    pub id: &'a str,
    /// The localized accessible name for the navigation landmark.
    pub name: &'a str,
    /// The exact width reserved for the bar, in logical pixels.
    pub available_width: f32,
    pub direction: LayoutDirection,
}

impl Theme {
    /// Build a compact Expressive bar with vertical icon and label stacks.
    ///
    /// Supply three to five destinations in reading order and update the
    /// current page when a destination sends its message. Tab visits each
    /// enabled destination. A press sends its message without changing the
    /// caller's state directly. The visible label wraps and grows with text.
    /// The caller positions the bar within the viewport.
    ///
    /// Returns `None` outside compact width or when the width cannot contain
    /// one 56 dp indicator per destination. Use a different layout in those
    /// cases.
    pub fn flexible_navigation_bar<'a, M: 'a>(
        &self,
        options: FlexibleNavigationBarOptions<'_>,
        items: impl IntoIterator<Item = NavigationRailItem<'a, M>>,
    ) -> Option<View<M>> {
        let width = options.available_width;
        if WindowWidthSizeClass::from_dp(width) != Some(WindowWidthSizeClass::Compact) {
            return None;
        }
        let items: Vec<_> = items.into_iter().take(6).collect();
        if !(3..=5).contains(&items.len()) || width < INDICATOR_WIDTH * items.len() as f32 {
            return None;
        }

        let bar = self.flexible_bar_node(&options, Align::Start);
        let fraction = 1. / items.len() as f32;
        let label_width = width * fraction - 2. * LABEL_INSET;
        Some(
            View::new(bar).children(
                items
                    .into_iter()
                    .map(|item| self.flexible_navigation_destination(item, fraction, label_width)),
            ),
        )
    }

    /// Build a centered Expressive bar with logical-leading icons in medium windows.
    ///
    /// Supply three to five destinations in reading order and let the caller
    /// own the current page. Each enabled destination has its own Tab stop.
    /// The 112 dp hit cells stay fixed as the outer margins grow. A long label
    /// wraps within its pill and can increase the height of the bar.
    /// Returns `None` below 600 dp, at 840 dp or above, or for an invalid item count.
    pub fn horizontal_flexible_navigation_bar<'a, M: 'a>(
        &self,
        options: FlexibleNavigationBarOptions<'_>,
        items: impl IntoIterator<Item = NavigationRailItem<'a, M>>,
    ) -> Option<View<M>> {
        if WindowWidthSizeClass::from_dp(options.available_width)
            != Some(WindowWidthSizeClass::Medium)
        {
            return None;
        }
        let items: Vec<_> = items.into_iter().take(6).collect();
        if !(3..=5).contains(&items.len()) {
            return None;
        }
        let bar = self.flexible_bar_node(&options, Align::Center);
        Some(
            View::new(bar).children(
                items
                    .into_iter()
                    .map(|item| self.horizontal_flexible_destination(item, options.direction)),
            ),
        )
    }

    fn flexible_bar_node(
        &self,
        options: &FlexibleNavigationBarOptions<'_>,
        justify: Align,
    ) -> Node {
        let mut style = Style::row()
            .width(Length::Px(options.available_width))
            .align(Align::Stretch)
            .justify(justify)
            .background(self.colors.surface_container);
        style.min_height = MIN_HEIGHT;
        physical_row(
            Node::new(Role::Navigation, options.name)
                .id(options.id)
                .styled(style)
                .box_shadows(self.elevation(Elevation::Level2)),
            options.direction,
        )
    }

    fn flexible_navigation_destination<M>(
        &self,
        item: NavigationRailItem<'_, M>,
        width_fraction: f32,
        label_width: f32,
    ) -> View<M> {
        let (icon_ink, label_ink, caption) =
            self.flexible_destination_ink(item.state, self.colors.secondary);
        let indicator = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-indicator", item.id))
                .styled(
                    navigation_indicator(self, INDICATOR_WIDTH, item.state.selected)
                        .state_layer(self.colors.on_secondary_container),
                )
                .state_layer_alphas(20, 26, 26),
        )
        .children([item.icon.view(VectorIconSize::Dp24, icon_ink)])
        .without_interaction(None);
        let label = View::new(
            Node::new(Role::Text, "")
                .id(format!("{}-label", item.id))
                .text(item.label)
                .text_alignment(TextAlignment::Center)
                .styled(
                    Style::column()
                        .width(Length::Px(label_width))
                        .with_text_style(caption)
                        .foreground(label_ink),
                ),
        )
        .without_interaction(None);
        self.flexible_destination_button(
            item,
            Style::column()
                .align(Align::Center)
                .width(Length::Percent(width_fraction)),
            [
                spacer(VERTICAL_PADDING),
                indicator,
                spacer(LABEL_GAP),
                label,
                spacer(VERTICAL_PADDING),
            ],
        )
    }

    fn horizontal_flexible_destination<M>(
        &self,
        item: NavigationRailItem<'_, M>,
        direction: LayoutDirection,
    ) -> View<M> {
        let (icon_ink, label_ink, caption) =
            self.flexible_destination_ink(item.state, self.colors.on_secondary_container);
        let fill = if item.state.selected {
            self.colors.secondary_container
        } else {
            Color::TRANSPARENT
        };
        let icon = View::new(
            Node::new(Role::Group, "")
                .styled(
                    Style::row()
                        .width(Length::Px(HORIZONTAL_ICON_SIZE))
                        .height(Length::Px(HORIZONTAL_ICON_SIZE))
                        .clip(true),
                )
                .fill_contour(
                    item.icon
                        .contour_dp(HORIZONTAL_ICON_SIZE, icon_ink)
                        .expect("validated icon contour"),
                ),
        );
        let label = View::new(
            Node::new(Role::Text, "")
                .id(format!("{}-label", item.id))
                .text(item.label)
                .text_alignment(match direction {
                    LayoutDirection::LeftToRight => TextAlignment::Left,
                    LayoutDirection::RightToLeft => TextAlignment::Right,
                })
                .styled(
                    Style::column()
                        .with_text_style(caption)
                        .foreground(label_ink),
                ),
        );
        let indicator = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-indicator", item.id))
                .styled(
                    Style::row()
                        .corners(shape::FULL)
                        .background(fill)
                        .state_layer(self.colors.on_secondary_container)
                        .transition(STATE_CHANGE),
                )
                .state_layer_alphas(20, 26, 26)
                .measured_group(HorizontalPill { direction }),
        )
        .children([icon, label])
        .without_interaction(None);
        self.flexible_destination_button(
            item,
            Style::column()
                .align(Align::Center)
                .justify(Align::Center)
                .width(Length::Px(HORIZONTAL_CELL_WIDTH)),
            [
                spacer(HORIZONTAL_BAR_Y_PADDING),
                indicator,
                spacer(HORIZONTAL_BAR_Y_PADDING),
            ],
        )
    }

    fn flexible_destination_ink(
        &self,
        state: ChoiceState,
        active_label: Color,
    ) -> (Color, Color, TextStyle) {
        let inactive = self.colors.on_surface_variant;
        let (icon, label) = if state.disabled {
            let Color(red, green, blue, _) = inactive;
            let disabled = Color(red, green, blue, DISABLED_ALPHA);
            (disabled, disabled)
        } else if state.selected {
            (self.colors.on_secondary_container, active_label)
        } else {
            (inactive, inactive)
        };
        let caption = if state.selected {
            typography::LABEL_MEDIUM_EMPHASIZED
        } else {
            typography::LABEL_MEDIUM
        };
        (icon, label, caption)
    }

    fn flexible_destination_button<M>(
        &self,
        item: NavigationRailItem<'_, M>,
        style: Style,
        children: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        let state = item.state;
        let destination = View::new(
            Node::new(Role::Button, item.name())
                .id(item.id)
                .selected(state.selected)
                .current_page(state.selected)
                .disabled(state.disabled)
                .focus_color(self.colors.secondary)
                .focus_outline_width(3.)
                .styled(style),
        )
        .children(children);
        if state.disabled {
            destination
        } else {
            destination
                .on_press(item.message)
                .suppress_repeated_activation()
                .accessible_activation()
                .cursor(CursorStyle::Pointer)
        }
    }
}

fn physical_row(node: Node, direction: LayoutDirection) -> Node {
    match direction {
        LayoutDirection::LeftToRight => node,
        LayoutDirection::RightToLeft => node.reverse_row(),
    }
}

#[derive(Debug)]
struct HorizontalPill {
    direction: LayoutDirection,
}

impl MeasuredGroupLayout for HorizontalPill {
    fn measure(
        &self,
        children: &[&Node],
        _available_width: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let label = children[1];
        let text = label.text_value().unwrap_or_default();
        let styles = both_weights(label);
        let text_width = styles
            .iter()
            .map(|style| fonts.measure_text(text, style, None).0)
            .fold(0., f32::max)
            .ceil()
            .min(HORIZONTAL_CELL_WIDTH - HORIZONTAL_ART_WIDTH);
        let text_height = label_height(text, &styles, text_width, fonts);
        (
            HORIZONTAL_ART_WIDTH + text_width,
            HORIZONTAL_ICON_SIZE.max(text_height) + 2. * HORIZONTAL_PILL_Y_PADDING,
        )
    }

    fn place(
        &self,
        children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let label = children[1];
        let text_width = (width - HORIZONTAL_ART_WIDTH).max(0.);
        let text_height = label_height(
            label.text_value().unwrap_or_default(),
            &both_weights(label),
            text_width,
            fonts,
        );
        let (icon_x, label_x) = match self.direction {
            LayoutDirection::LeftToRight => (
                HORIZONTAL_PILL_INSET,
                HORIZONTAL_PILL_INSET + HORIZONTAL_ICON_SIZE + LABEL_GAP,
            ),
            LayoutDirection::RightToLeft => (
                width - HORIZONTAL_PILL_INSET - HORIZONTAL_ICON_SIZE,
                HORIZONTAL_PILL_INSET,
            ),
        };
        vec![
            Rect::new(
                icon_x,
                (height - HORIZONTAL_ICON_SIZE) / 2.,
                HORIZONTAL_ICON_SIZE,
                HORIZONTAL_ICON_SIZE,
            ),
            Rect::new(
                label_x,
                (height - text_height) / 2.,
                text_width,
                text_height,
            ),
        ]
    }
}

fn both_weights(label: &Node) -> [TextStyle; 2] {
    let scaled = label.style().text_style();
    // Reserve space for either weight so choosing a destination cannot reflow it.
    [
        typography::LABEL_MEDIUM.weight,
        typography::LABEL_MEDIUM_EMPHASIZED.weight,
    ]
    .map(|weight| TextStyle {
        weight,
        ..scaled.clone()
    })
}

fn label_height(
    text: &str,
    styles: &[TextStyle; 2],
    width: f32,
    fonts: &mut dyn TextMeasurer,
) -> f32 {
    styles
        .iter()
        .map(|style| fonts.measure_text(text, style, Some(width.max(1.))).1)
        .fold(0., f32::max)
        .ceil()
}

fn spacer<M>(height: f32) -> View<M> {
    View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(height))))
        .without_interaction(None)
}
