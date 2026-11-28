//! A shared page and persistent navigation for width-based Material layouts.

use surface::{Align, Length, Node, Role, Style};
use surface_app::View;

use crate::{
    adaptive::WindowWidthSizeClass, shape::LayoutDirection, FlexibleNavigationBarOptions,
    NavigationRailItem, Theme, WideNavigationRailMode,
};

/// Stable identities and exact logical size of an adaptive navigation scaffold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveNavigationScaffoldOptions<'a> {
    /// Stable ID of the scaffold.
    pub id: &'a str,
    /// Stable ID of the navigation landmark in every presentation.
    pub navigation_id: &'a str,
    /// Accessible name of the navigation landmark.
    pub name: &'a str,
    pub available_width: f32,
    pub available_height: f32,
    pub direction: LayoutDirection,
}

impl Theme {
    /// Place one caller-owned page beside a leading rail or above a bottom bar.
    ///
    /// This width-only policy uses the vertical bar below 600 dp, the horizontal
    /// bar from 600 dp to below 840 dp, and the collapsed wide rail at 840 dp and
    /// above. The rail scrolls independently when its destinations do not fit.
    /// The rail uses one Tab stop with Up and Down to move focus. In a bar,
    /// Tab visits each enabled destination.
    ///
    /// Supply three to five destinations in reading order. Reuse their IDs and
    /// the page's IDs across resizes. The caller chooses the current page and
    /// handles activation. Each destination keeps its name and current-page
    /// state in every presentation.
    ///
    /// This reserves `{id}-content`, `{navigation_id}-scroll`, and
    /// `{navigation_id}-destinations`. Returns `None` for invalid dimensions,
    /// a height below the minimum 64 dp bar, or when the Compact bar cannot
    /// fit the requested count. Allow extra height for wrapped labels.
    pub fn adaptive_navigation_scaffold<'a, M: 'a>(
        &self,
        options: AdaptiveNavigationScaffoldOptions<'_>,
        page: View<M>,
        items: impl IntoIterator<Item = NavigationRailItem<'a, M>>,
    ) -> Option<View<M>> {
        let width_class = WindowWidthSizeClass::from_dp(options.available_width)?;
        if !options.available_height.is_finite() || options.available_height < 64. {
            return None;
        }
        let items: Vec<_> = items.into_iter().take(6).collect();
        if !(3..=5).contains(&items.len()) {
            return None;
        }

        let rail = width_class >= WindowWidthSizeClass::Expanded;
        let bar_options = FlexibleNavigationBarOptions {
            id: options.navigation_id,
            name: options.name,
            available_width: options.available_width,
            direction: options.direction,
        };
        let navigation = match width_class {
            WindowWidthSizeClass::Compact => self.flexible_navigation_bar(bar_options, items)?,
            WindowWidthSizeClass::Medium => {
                self.horizontal_flexible_navigation_bar(bar_options, items)?
            }
            _ => self.scaffold_wide_navigation_rail(
                options.navigation_id,
                options.name,
                items.into_iter().map(|item| {
                    self.wide_navigation_rail_item_with_icon(
                        WideNavigationRailMode::Collapsed,
                        options.direction,
                        item,
                    )
                    .suppress_repeated_activation()
                    .accessibility_scroll_into_view()
                }),
            ),
        };
        let style = if rail { Style::row() } else { Style::column() }
            .width(Length::Px(options.available_width))
            .height(Length::Px(options.available_height))
            .align(Align::Stretch);
        let mut shell = Node::new(Role::Group, "").id(options.id).styled(style);
        if rail && options.direction == LayoutDirection::RightToLeft {
            shell = shell.reverse_row();
        }
        let content_style = if rail {
            Style::column().width(Length::Px(0.))
        } else {
            Style::column().height(Length::Px(0.))
        };
        let content = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-content", options.id))
                .styled(content_style.grow(1.).clip(true)),
        )
        .children([page]);
        let children = if rail {
            [navigation, content]
        } else {
            [content, navigation]
        };
        Some(View::new(shell).children(children))
    }
}
