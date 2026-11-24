//! Compact Material 3 Expressive navigation with vertical destinations.

use surface::{Align, Color, Length, Node, Role, Style, TextAlignment};
use surface_app::{CursorStyle, View};

use crate::{
    adaptive::WindowWidthSizeClass, components::navigation_indicator, shape::LayoutDirection,
    typography, vector_icon::VectorIconSize, Elevation, NavigationRailItem, Theme,
};

const MIN_HEIGHT: f32 = 64.;
const INDICATOR_WIDTH: f32 = 56.;
const VERTICAL_PADDING: f32 = 6.;
const LABEL_GAP: f32 = 4.;
const LABEL_INSET: f32 = 4.;
const DISABLED_ALPHA: u8 = 97;

/// The stable identity and compact layout of a flexible navigation bar.
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

        let mut style = Style::row()
            .width(Length::Px(width))
            .align(Align::Stretch)
            .background(self.colors.surface_container);
        style.min_height = MIN_HEIGHT;
        let bar = Node::new(Role::Navigation, options.name)
            .id(options.id)
            .styled(style)
            .box_shadows(self.elevation(Elevation::Level2));
        let bar = match options.direction {
            LayoutDirection::LeftToRight => bar,
            LayoutDirection::RightToLeft => bar.reverse_row(),
        };
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

    fn flexible_navigation_destination<M>(
        &self,
        item: NavigationRailItem<'_, M>,
        width_fraction: f32,
        label_width: f32,
    ) -> View<M> {
        let state = item.state;
        let inactive = self.colors.on_surface_variant;
        let (icon_ink, label_ink) = if state.disabled {
            let Color(red, green, blue, _) = inactive;
            let disabled = Color(red, green, blue, DISABLED_ALPHA);
            (disabled, disabled)
        } else if state.selected {
            (self.colors.on_secondary_container, self.colors.secondary)
        } else {
            (inactive, inactive)
        };
        let caption = if state.selected {
            typography::LABEL_MEDIUM_EMPHASIZED
        } else {
            typography::LABEL_MEDIUM
        };
        let indicator = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-indicator", item.id))
                .styled(
                    navigation_indicator(self, INDICATOR_WIDTH, state.selected)
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
        let destination = View::new(
            Node::new(Role::Button, item.name())
                .id(item.id)
                .selected(state.selected)
                .current_page(state.selected)
                .disabled(state.disabled)
                .focus_color(self.colors.secondary)
                .focus_outline_width(3.)
                .styled(
                    Style::column()
                        .align(Align::Center)
                        .width(Length::Percent(width_fraction)),
                ),
        )
        .children([
            spacer(VERTICAL_PADDING),
            indicator,
            spacer(LABEL_GAP),
            label,
            spacer(VERTICAL_PADDING),
        ]);
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

fn spacer<M>(height: f32) -> View<M> {
    View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(height))))
        .without_interaction(None)
}
