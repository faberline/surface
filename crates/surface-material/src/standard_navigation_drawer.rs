use surface::{Align, Length, Node, Role, Style, TextScaling};
use surface_app::{Roving, View};

use crate::{shape, shape::LayoutDirection, typography, Theme};

const SHEET_WIDTH: f32 = 360.;
const SHEET_PADDING: f32 = 12.;
const SHARED_WIDTH_FRACTION: f32 = 0.6;

/// Localized names and logical placement for a standard drawer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StandardNavigationDrawerOptions<'a> {
    /// Stable ID of the navigation sheet.
    pub id: &'a str,
    /// Accessible name of the navigation landmark.
    pub navigation_name: &'a str,
    /// An optional visible heading at the top of the sheet.
    pub headline: Option<&'a str>,
    /// Direction of the sheet and its headings. Match this in destination rows.
    pub direction: LayoutDirection,
}

/// The application's current state for a standard drawer in a shared layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandardNavigationDrawerMode<'a> {
    Permanent,
    Dismissible {
        open: bool,
        /// Stable ID of an enabled control to focus when a drawer control
        /// loses focus as the sheet closes.
        restore_focus_to: &'a str,
    },
}

impl Theme {
    /// Build a full-height, scrollable navigation sheet up to 360 dp wide.
    ///
    /// Place it at the logical start edge of the parent. Use
    /// [Theme::navigation_drawer_item_with_icon] for destinations. Lists may
    /// also contain dividers and [Theme::navigation_drawer_section_heading].
    /// Up and Down move among enabled destinations, and Tab leaves the group.
    ///
    /// [AndroidX navigation drawer tokens](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/NavigationDrawerTokens.kt)
    pub fn standard_navigation_drawer_sheet<M>(
        &self,
        options: StandardNavigationDrawerOptions<'_>,
        items: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        self.standard_drawer_sheet(options, items, Length::Percent(1.))
    }

    /// Lay out a sheet at logical start and the main content in the remaining
    /// space. On narrow layouts the sheet takes at most 60% of the width.
    /// When the drawer closes, the main content uses the full width.
    ///
    /// The application provides a control to toggle a dismissible drawer.
    /// Opening keeps focus in place. Closing restores focus from a removed
    /// destination to the supplied control. For a custom nested control, set
    /// [View::restore_focus_on_removal] on that control as well.
    /// The layout reserves `{id}-layout` and `{id}-content`.
    pub fn standard_navigation_drawer_layout<M>(
        &self,
        options: StandardNavigationDrawerOptions<'_>,
        mode: StandardNavigationDrawerMode<'_>,
        items: impl IntoIterator<Item = View<M>>,
        content: View<M>,
    ) -> View<M> {
        let content = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-content", options.id))
                .styled(Style::column().width(Length::Px(0.)).grow(1.).clip(true)),
        )
        .children([content]);
        let mut layout = Node::new(Role::Group, "")
            .id(format!("{}-layout", options.id))
            .styled(
                Style::row()
                    .width(Length::Percent(1.))
                    .height(Length::Percent(1.)),
            );
        if options.direction == LayoutDirection::RightToLeft {
            layout = layout.reverse_row();
        }
        let mut children = Vec::new();
        let max_width = Length::Percent(SHARED_WIDTH_FRACTION);
        match mode {
            StandardNavigationDrawerMode::Permanent => {
                children.push(self.standard_drawer_sheet(options, items, max_width));
            }
            StandardNavigationDrawerMode::Dismissible {
                open: true,
                restore_focus_to,
            } => {
                let items = items
                    .into_iter()
                    .map(|item| item.restore_focus_on_removal(restore_focus_to));
                children.push(self.standard_drawer_sheet(options, items, max_width));
            }
            StandardNavigationDrawerMode::Dismissible { open: false, .. } => {}
        }
        children.push(content);
        View::new(layout).children(children)
    }

    /// Add a title above a group of destinations in the sheet.
    pub fn navigation_drawer_section_heading<M>(
        &self,
        text: &str,
        direction: LayoutDirection,
    ) -> View<M> {
        let mut heading =
            Node::new(Role::Heading, text).styled(Style::row().align(Align::Center).padding(16.));
        if direction == LayoutDirection::RightToLeft {
            heading = heading.reverse_row();
        }
        let label = Node::new(Role::Text, "")
            .text(text)
            .shrink_to_fit_width()
            .text_scaling(TextScaling::Expand)
            .styled(
                Style::default()
                    .with_text_style(typography::TITLE_SMALL)
                    .foreground(self.colors.on_surface_variant),
            );
        View::new(heading).children([View::new(label).without_interaction(None)])
    }

    fn standard_drawer_sheet<M>(
        &self,
        options: StandardNavigationDrawerOptions<'_>,
        items: impl IntoIterator<Item = View<M>>,
        max_width: Length,
    ) -> View<M> {
        let mut sheet = Style::column()
            .width(Length::Px(SHEET_WIDTH))
            .height(Length::Percent(1.))
            .padding(SHEET_PADDING)
            .corners(shape::large_end(options.direction))
            .background(self.colors.surface)
            .clip(true);
        sheet.max_width = max_width;
        let heading = options
            .headline
            .map(|text| self.navigation_drawer_section_heading(text, options.direction));
        View::new(
            Node::new(Role::Navigation, options.navigation_name)
                .id(options.id)
                .styled(sheet),
        )
        .children(heading.into_iter().chain(items))
        .roving(Roving::vertical(false))
        .accessibility_scroll_pages()
    }
}
