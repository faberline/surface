//! Directional Material navigation drawer and destinations.

use surface::{Align, Color, FillContour, Insets, Length, Node, Role, Style, TextScaling};
use surface_app::{CursorStyle, HorizontalDrag, Roving, View};

mod swipe;
pub use swipe::NavigationDrawerSwipe;

use crate::{
    motion::STATE_CHANGE, shape, shape::LayoutDirection, typography, vector_icon::VectorIconSize,
    ChoiceState, Elevation, Theme, VectorIcon,
};

// See the pinned AndroidX NavigationDrawerTokens linked on the public builder.
const MAX_WIDTH: f32 = 360.;
const MIN_DESTINATION_HEIGHT: f32 = 56.;
// The pinned AndroidX ScrimTokens.ContainerOpacity is 32%.
const SCRIM_ALPHA: u8 = 82;
const SWIPE_EDGE: f32 = 24.;
const DISABLED_INK_ALPHA: u8 = 97;
const CLOSE_ICON: [[f32; 2]; 12] = [
    [5., 6.4],
    [6.4, 5.],
    [12., 10.6],
    [17.6, 5.],
    [19., 6.4],
    [13.4, 12.],
    [19., 17.6],
    [17.6, 19.],
    [12., 13.4],
    [6.4, 19.],
    [5., 17.6],
    [10.6, 12.],
];

/// A caller-controlled drawer destination with decorative vector art.
pub struct NavigationDrawerItem<'a, M> {
    pub id: &'a str,
    pub label: &'a str,
    /// The localized button name. A blank name falls back to `label`.
    pub accessible_name: &'a str,
    pub icon: &'a VectorIcon,
    pub direction: LayoutDirection,
    pub state: ChoiceState,
    pub message: M,
}

/// Localized names and placement for an open directional drawer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModalNavigationDrawerOptions<'a> {
    /// Stable ID of the dialog.
    pub id: &'a str,
    /// Visible title and accessible dialog name.
    pub name: &'a str,
    /// Localized name of the destination group.
    pub navigation_name: &'a str,
    /// Localized name of the Close button.
    pub close_name: &'a str,
    /// Logical start edge of the panel and direction of the header.
    pub direction: LayoutDirection,
}

impl Theme {
    /// Build one drawer destination with a 24 dp vector icon and a wrapping label.
    ///
    /// Match the drawer's direction. The artwork and label are visual parts
    /// of a single accessible button. The application owns selection.
    pub fn navigation_drawer_item_with_icon<M>(
        &self,
        item: NavigationDrawerItem<'_, M>,
    ) -> View<M> {
        let name = if item.accessible_name.trim().is_empty() {
            item.label
        } else {
            item.accessible_name
        };
        let ink = if item.state.disabled {
            let Color(r, g, b, _) = self.colors.on_surface;
            Color(r, g, b, DISABLED_INK_ALPHA)
        } else if item.state.selected {
            self.colors.on_secondary_container
        } else {
            self.colors.on_surface_variant
        };
        let fill = if item.state.selected {
            self.colors.secondary_container
        } else {
            Color::TRANSPARENT
        };
        let mut style = Style::row()
            .align(Align::Center)
            .gap(12.)
            .width(Length::Percent(1.))
            .padding(16.)
            .corners(shape::FULL)
            .background(fill)
            .state_layer(ink)
            .transition(STATE_CHANGE);
        style.min_height = MIN_DESTINATION_HEIGHT;
        let node = Node::new(Role::Button, name)
            .id(item.id)
            .selected(item.state.selected)
            .current_page(item.state.selected)
            .disabled(item.state.disabled)
            .text_scaling(TextScaling::Expand)
            .focus_color(self.colors.secondary)
            .styled(style);
        let label = Node::new(Role::Text, "")
            .text(item.label)
            .shrink_to_fit_width()
            .styled(
                Style::default()
                    .with_text_style(typography::LABEL_LARGE)
                    .foreground(ink),
            );
        let view = View::new(in_direction(node, item.direction))
            .children([
                item.icon
                    .view(VectorIconSize::Dp24, ink)
                    .without_interaction(None),
                View::new(label).without_interaction(None),
            ])
            .on_press(item.message)
            .accessible_activation()
            .accessibility_scroll_into_view();
        if item.state.disabled {
            view
        } else {
            view.cursor(CursorStyle::Pointer)
        }
    }

    /// A full-height drawer at the logical start of the viewport.
    ///
    /// Mount it after normal page content. Remove it when the application
    /// receives `dismiss` from Close, Escape or a click on the scrim. The
    /// drawer traps focus and the runtime restores the opener's focus.
    /// The Close button is the first Tab stop. The body scrolls separately
    /// and the arrow keys move among its enabled destinations.
    ///
    /// In addition to `id`, this reserves `{id}-layer`, `{id}-header`,
    /// `{id}-title`, `{id}-close` and `{id}-navigation`.
    ///
    /// [AndroidX navigation drawer tokens](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/NavigationDrawerTokens.kt)
    pub fn modal_navigation_drawer_with_direction<M: Clone>(
        &self,
        options: ModalNavigationDrawerOptions<'_>,
        items: impl IntoIterator<Item = View<M>>,
        dismiss: M,
    ) -> View<M> {
        self.directional_drawer_panel(options, items, dismiss.clone(), 0.)
            .modal_with_backdrop(self.directional_drawer_layer(options, 1.), dismiss)
    }

    /// Mount a directional modal drawer with optional pointer and touch swipes.
    ///
    /// Extend the page's children with these views after ordinary content.
    /// Keep an accessible control for opening the drawer. When closed, this
    /// reserves a 24 dp gesture region at the logical start of that page.
    /// The caller can use the static builder on screens with a competing
    /// edge gesture. The closed preview is visual only. Focus and modality
    /// change when the application applies the committed result from
    /// [NavigationDrawerSwipe::advance]. A closing drag keeps the focus
    /// scope and scrim until it settles. A drag that returns to the same
    /// endpoint leaves the app's open flag and focus unchanged.
    ///
    /// Also reserves the ID formed by appending `-swipe-edge` to the drawer ID.
    pub fn swipeable_modal_navigation_drawer<M: Clone>(
        &self,
        options: ModalNavigationDrawerOptions<'_>,
        items: impl IntoIterator<Item = View<M>>,
        dismiss: M,
        swipe: &NavigationDrawerSwipe,
        open: bool,
        on_drag: impl Fn(HorizontalDrag) -> M + 'static,
    ) -> impl Iterator<Item = View<M>> {
        let edge = if open || swipe.is_settling() || swipe.width() == 0. {
            None
        } else {
            let mut style = Style::default()
                .fixed(start_inset(options.direction, 0., true))
                .width(Length::Px(SWIPE_EDGE));
            style.max_width = Length::Percent(1.);
            style.z_index = 999;
            Some(View::new(
                Node::new(Role::Group, "")
                    .id(format!("{}-swipe-edge", options.id))
                    .styled(style),
            ))
        };
        let progress = swipe.progress(open);
        let layer = (open || progress > 0.).then(|| {
            let panel = self.directional_drawer_panel(
                options,
                items,
                dismiss.clone(),
                -(1. - progress) * swipe.width(),
            );
            let backdrop = self.directional_drawer_layer(options, progress);
            if open {
                panel.modal_with_backdrop(backdrop, dismiss)
            } else {
                View::new(backdrop).children([panel]).visual_overlay()
            }
        });
        let (edge, layer) = match (edge, layer) {
            (Some(edge), layer) => (
                Some(edge.on_horizontal_drag(swipe.session.clone(), on_drag)),
                layer,
            ),
            (None, Some(layer)) if open && !swipe.is_settling() && swipe.width() > 0. => (
                None,
                Some(layer.on_horizontal_drag(swipe.session.clone(), on_drag)),
            ),
            views => views,
        };
        [edge, layer].into_iter().flatten()
    }

    fn directional_drawer_panel<M: Clone>(
        &self,
        options: ModalNavigationDrawerOptions<'_>,
        items: impl IntoIterator<Item = View<M>>,
        dismiss: M,
        offset: f32,
    ) -> View<M> {
        let id = options.id;
        let title = View::new(
            Node::new(Role::Heading, options.name)
                .id(format!("{id}-title"))
                .text(options.name)
                .shrink_to_fit_width()
                .styled(
                    Style::default()
                        .with_text_style(typography::TITLE_SMALL)
                        .foreground(self.colors.on_surface_variant),
                ),
        );
        let close = self.drawer_close_button(options, dismiss);
        let header = Node::new(Role::Group, "")
            .id(format!("{id}-header"))
            .text_scaling(TextScaling::Expand)
            .styled(Style::row().align(Align::Center).padding(12.).gap(12.));
        let header = View::new(in_direction(header, options.direction)).children([close, title]);
        let navigation = View::new(
            Node::new(Role::Navigation, options.navigation_name)
                .id(format!("{id}-navigation"))
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Px(0.))
                        .grow(1.)
                        .padding(12.)
                        .clip(true),
                ),
        )
        .children(items)
        .roving(Roving::vertical(false))
        .accessibility_scroll_pages();
        let mut panel_style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Percent(1.))
            .corners(shape::large_end(options.direction))
            .background(self.colors.surface_container_low)
            .clip(true);
        panel_style.max_width = Length::Px(MAX_WIDTH);
        if offset < 0. {
            panel_style = panel_style.absolute(start_inset(options.direction, offset, false));
        }
        View::new(
            Node::new(Role::Dialog, options.name)
                .id(id)
                .styled(panel_style)
                .box_shadows(self.elevation(Elevation::Level1)),
        )
        .children([header, navigation])
    }

    fn drawer_close_button<M>(
        &self,
        options: ModalNavigationDrawerOptions<'_>,
        dismiss: M,
    ) -> View<M> {
        let ink = self.colors.on_surface_variant;
        let icon = Node::new(Role::Group, "")
            .fill_contour(FillContour::new(CLOSE_ICON, ink).expect("valid close icon contour"))
            .styled(Style::row().width(Length::Px(24.)).height(Length::Px(24.)));
        let state_layer = Node::new(Role::Group, "").styled(
            centered_square(40.)
                .corners(shape::FULL)
                .state_layer(ink)
                .transition(STATE_CHANGE),
        );
        View::new(
            Node::new(Role::Button, options.close_name)
                .id(format!("{}-close", options.id))
                .focus_color(self.colors.secondary)
                .styled(centered_square(48.).corners(shape::FULL)),
        )
        .children([View::new(state_layer)
            .children([View::new(icon)])
            .without_interaction(None)])
        .on_press(dismiss)
        .accessible_activation()
        .protect_composition_on_press()
        .cursor(CursorStyle::Pointer)
    }

    fn directional_drawer_layer(
        &self,
        options: ModalNavigationDrawerOptions<'_>,
        progress: f32,
    ) -> Node {
        let Color(red, green, blue, _) = self.colors.scrim;
        let mut layer = Style::row()
            .fixed(Insets::all(0.))
            .justify(match options.direction {
                LayoutDirection::LeftToRight => Align::Start,
                LayoutDirection::RightToLeft => Align::End,
            })
            .background(Color(
                red,
                green,
                blue,
                (f32::from(SCRIM_ALPHA) * progress).round() as u8,
            ));
        layer.z_index = 1000;
        Node::new(Role::Group, "")
            .id(format!("{}-layer", options.id))
            .styled(layer)
    }
}

fn centered_square(size: f32) -> Style {
    Style::row()
        .width(Length::Px(size))
        .height(Length::Px(size))
        .align(Align::Center)
        .justify(Align::Center)
}

fn start_inset(direction: LayoutDirection, offset: f32, full_height: bool) -> Insets {
    let mut inset = Insets {
        top: Length::Px(0.),
        ..Insets::default()
    };
    if full_height {
        inset.bottom = Length::Px(0.);
    }
    match direction {
        LayoutDirection::LeftToRight => inset.left = Length::Px(offset),
        LayoutDirection::RightToLeft => inset.right = Length::Px(offset),
    }
    inset
}

fn in_direction(node: Node, direction: LayoutDirection) -> Node {
    match direction {
        LayoutDirection::LeftToRight => node,
        LayoutDirection::RightToLeft => node.reverse_row(),
    }
}
