//! Baseline Material 3 menu for commands launched by a button.

use surface::{Align, Color, Insets, Length, Node, Rect, Role, Style, TextAlignment};
use surface_app::{AnimationFrame, AnimationUpdate, CursorStyle, Roving, View};

use crate::{
    menu_layout::{place_menu, MenuAnchor, MIN_ITEM_HEIGHT},
    shape::{self, LayoutDirection},
    typography, Elevation, Theme, VectorIcon,
};

const OUTER_PADDING: f32 = 8.;
const ITEM_SPACE: f32 = 12.;
const ICON_SIZE: f32 = 24.;
const SEPARATOR_HEIGHT: f32 = 1.;
const SEPARATOR_SPACE: f32 = 8.;

/// The stable identities, measured width and controlled state of a button menu.
pub struct CommandMenuOptions<'a> {
    /// Stable ID of the named menu panel.
    pub id: &'a str,
    /// Localized accessible name of the menu.
    pub label: &'a str,
    /// Stable ID already assigned to the caller's trigger button.
    pub trigger_id: &'a str,
    pub open: bool,
    pub direction: LayoutDirection,
    /// Preferred width, normally between 112 and 280 dp.
    pub preferred_width: f32,
    /// Alternate control to focus if the invoking control goes away.
    pub restore_focus_to: Option<&'a str>,
}

/// One command or a passive, semantic separator between command groups.
pub enum CommandMenuEntry<'a, M> {
    Item(CommandMenuItem<'a, M>),
    Separator { id: &'a str },
}

/// The row's label also names its single accessible action. Icons are passive.
pub struct CommandMenuItem<'a, M> {
    pub id: &'a str,
    pub label: &'a str,
    pub leading_icon: Option<VectorIcon>,
    pub trailing_icon: Option<VectorIcon>,
    pub disabled: bool,
    pub on_press: M,
}

/// Retain one measurement value per menu in the application.
#[derive(Default, Clone, Copy, PartialEq)]
pub struct CommandMenuMeasurement {
    anchor: Option<MenuAnchor>,
    content_height: Option<f32>,
}

impl CommandMenuMeasurement {
    /// Read the clipped trigger bounds and the full laid out menu contents.
    /// Use the result from the application's frame callback so the first
    /// valid measurement and later changes can update the popup placement.
    pub fn advance(
        &mut self,
        menu_id: &str,
        trigger_id: &str,
        frame: &AnimationFrame<'_>,
    ) -> AnimationUpdate {
        let previous = *self;
        if let Some(layout) = frame.layout {
            self.anchor = MenuAnchor::from_frame(frame, trigger_id);
            self.content_height = frame
                .dom
                .by_id(&content_id(menu_id))
                .and_then(|id| layout.nodes.get(id.index()))
                .map(|node| node.bounds.height)
                .filter(|height| height.is_finite() && *height > 0.);
        }
        let changed = previous != *self;
        AnimationUpdate {
            rebuild: changed,
            request_next: changed
                || (frame.layout.is_none() && frame.dom.by_id(trigger_id).is_some()),
        }
    }

    /// The visible portion of the trigger and its viewport, if measured.
    pub fn anchor(&self) -> Option<MenuAnchor> {
        self.anchor
    }

    /// Natural height before the viewport clips the currently mounted panel.
    pub fn content_height(&self) -> Option<f32> {
        self.content_height
    }
}

impl Theme {
    /// Build an anchored, scrollable command menu next to a caller-authored
    /// button. The returned group occupies the button's slot in the layout.
    /// The application changes `open` when the button, dismissal or command
    /// sends a message. This builder exposes the trigger as expanded only
    /// after it has a visible anchor and there is at least one command.
    ///
    /// [Material 3 menu specifications](https://m3.material.io/components/menus/specs)
    pub fn command_menu<'a, M>(
        &self,
        options: CommandMenuOptions<'_>,
        trigger: View<M>,
        entries: impl IntoIterator<Item = CommandMenuEntry<'a, M>>,
        on_dismiss: M,
        measurement: &CommandMenuMeasurement,
    ) -> View<M>
    where
        M: 'a,
    {
        let scope = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-scope", options.id))
                .styled(Style::column()),
        );
        let Some(anchor) = measurement.anchor.filter(|_| options.open) else {
            return scope.children([trigger.menu_expanded(false)]);
        };
        let entries: Vec<_> = entries.into_iter().collect();
        let first = entries.iter().find_map(|entry| match entry {
            CommandMenuEntry::Item(item) => Some(item.id),
            CommandMenuEntry::Separator { .. } => None,
        });
        let Some(first) = first else {
            return scope.children([trigger.menu_expanded(false)]);
        };
        let minimum_height = 2. * OUTER_PADDING
            + entries
                .iter()
                .map(|entry| match entry {
                    CommandMenuEntry::Item(_) => MIN_ITEM_HEIGHT,
                    CommandMenuEntry::Separator { .. } => 2. * SEPARATOR_SPACE + SEPARATOR_HEIGHT,
                })
                .sum::<f32>();
        let desired_height = measurement.content_height.unwrap_or(0.).max(minimum_height);
        let Some(layout) = place_menu(
            anchor,
            (options.preferred_width, desired_height),
            options.direction,
        ) else {
            return scope.children([trigger.menu_expanded(false)]);
        };
        let menu = self.command_menu_panel(&options, entries, layout.bounds);
        scope
            .children([trigger.menu_expanded(true), menu])
            .modal_overlay_descendants(on_dismiss)
            .modal_initial_focus(first)
            .modal_focus_fallback(options.restore_focus_to.unwrap_or(options.trigger_id))
            .dismiss_modal_on_tab()
    }

    fn command_menu_panel<'a, M>(
        &self,
        options: &CommandMenuOptions<'_>,
        entries: impl IntoIterator<Item = CommandMenuEntry<'a, M>>,
        bounds: Rect,
    ) -> View<M>
    where
        M: 'a,
    {
        let contents = View::new(
            Node::new(Role::Group, "")
                .id(content_id(options.id))
                .styled(Style::column().width(Length::Percent(1.))),
        )
        .children(
            std::iter::once(spacer(OUTER_PADDING))
                .chain(entries.into_iter().map(|entry| match entry {
                    CommandMenuEntry::Item(item) => self.command_menu_item(item, options.direction),
                    CommandMenuEntry::Separator { id } => self.command_menu_separator(id),
                }))
                .chain(std::iter::once(spacer(OUTER_PADDING))),
        );
        let style = Style::column()
            .width(Length::Px(bounds.width))
            .height(Length::Px(bounds.height))
            .fixed(Insets {
                left: Length::Px(bounds.x),
                top: Length::Px(bounds.y),
                ..Insets::default()
            })
            .clip(true)
            .background(self.colors.surface_container)
            .corners(shape::EXTRA_SMALL);
        View::new(
            Node::new(Role::Menu, options.label)
                .id(options.id)
                .styled(style)
                .box_shadows(self.elevation(Elevation::Level2)),
        )
        .children([contents])
        .roving(Roving::vertical(false))
        .accessibility_scroll_pages()
    }

    fn command_menu_item<M>(
        &self,
        item: CommandMenuItem<'_, M>,
        direction: LayoutDirection,
    ) -> View<M> {
        let label_ink = self.colors.on_surface;
        let (label_ink, icon_ink) = if item.disabled {
            let Color(r, g, b, _) = label_ink;
            (Color(r, g, b, 97), Color(r, g, b, 97))
        } else {
            (label_ink, self.colors.on_surface_variant)
        };
        let mut style = Style::row()
            .width(Length::Percent(1.))
            .padding(ITEM_SPACE)
            .gap(ITEM_SPACE)
            .align(Align::Center)
            .state_layer(self.colors.on_surface);
        style.min_height = MIN_ITEM_HEIGHT;
        let control = Node::new(Role::MenuItem, item.label)
            .id(item.id)
            .disabled(item.disabled)
            .focus_color(self.colors.secondary)
            .styled(style);
        let (control, alignment) = match direction {
            LayoutDirection::LeftToRight => (control, TextAlignment::Left),
            LayoutDirection::RightToLeft => (control.reverse_row(), TextAlignment::Right),
        };
        let text = View::new(
            Node::new(Role::Text, "")
                .id(format!("{}-label", item.id))
                .text(item.label)
                .text_alignment(alignment)
                .shrink_to_fit_width()
                .styled(
                    Style::column()
                        .grow(1.)
                        .with_text_style(typography::BODY_LARGE)
                        .foreground(label_ink),
                ),
        )
        .without_interaction(None);
        let leading = item
            .leading_icon
            .map(|icon| icon_view(format!("{}-leading", item.id), icon, icon_ink));
        let trailing = item
            .trailing_icon
            .map(|icon| icon_view(format!("{}-trailing", item.id), icon, icon_ink));
        let control = View::new(control)
            .children(leading.into_iter().chain([text]).chain(trailing))
            .accessibility_scroll_into_view();
        if item.disabled {
            control.focus_disabled_menu_item()
        } else {
            control
                .on_press(item.on_press)
                .accessible_activation()
                .suppress_repeated_activation()
                .cursor(CursorStyle::Pointer)
        }
    }

    fn command_menu_separator<M>(&self, id: &str) -> View<M> {
        let line = View::new(
            Node::new(Role::Separator, "").id(id).styled(
                Style::column()
                    .height(Length::Px(SEPARATOR_HEIGHT))
                    .background(self.colors.outline_variant),
            ),
        );
        View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .height(Length::Px(2. * SEPARATOR_SPACE + SEPARATOR_HEIGHT))
                    .justify(Align::Center),
            ),
        )
        .children([line])
    }
}

fn content_id(menu_id: &str) -> String {
    format!("{menu_id}-content")
}

fn spacer<M>(height: f32) -> View<M> {
    View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(height))))
}

fn icon_view<M>(id: String, icon: VectorIcon, ink: Color) -> View<M> {
    View::new(
        Node::new(Role::Group, "")
            .id(id)
            .styled(
                Style::column()
                    .width(Length::Px(ICON_SIZE))
                    .height(Length::Px(ICON_SIZE)),
            )
            .fill_contour(
                icon.contour_dp(ICON_SIZE, ink)
                    .expect("validated icon contour"),
            ),
    )
    .without_interaction(None)
}
