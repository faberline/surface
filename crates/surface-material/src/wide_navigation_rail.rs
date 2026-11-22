use surface::{Align, Color, Insets, Length, Node, Role, Style, TextScaling, TextStyle};
use surface_app::{CursorStyle, HorizontalDrag, Roving, View};

mod item_motion;
mod motion;
mod swipe;
pub use motion::{
    WideNavigationRailMotion, WideNavigationRailMotionFrame, WideNavigationRailMotionKind,
};
pub use swipe::WideNavigationRailSwipe;

use crate::{
    shape, shape::LayoutDirection, typography, vector_icon::VectorIconSize, Elevation,
    IconButtonKind, NavigationRailItem, Theme,
};

// Pinned AndroidX WideNavigationRail and NavigationRail token files use
// these endpoint measurements. The horizontal inset is the item's outer
// padding. Its indicator has separate padding around the icon and label.
const COLLAPSED_WIDTH: f32 = 96.;
const EXPANDED_MIN_WIDTH: f32 = 220.;
const EXPANDED_MAX_WIDTH: f32 = 360.;
const TOP_SPACE: f32 = 44.;
const HEADER_SPACE: f32 = 40.;
const COLLAPSED_ITEM_GAP: f32 = 4.;
const COLLAPSED_ITEM_MIN_HEIGHT: f32 = 64.;
const COLLAPSED_INDICATOR_WIDTH: f32 = 56.;
const COLLAPSED_INDICATOR_HEIGHT: f32 = 32.;
const COLLAPSED_ICON_LABEL_GAP: f32 = 4.;
const EXPANDED_ITEM_INSET: f32 = 20.;
const EXPANDED_INDICATOR_PADDING: f32 = 16.;
const EXPANDED_ICON_LABEL_GAP: f32 = 8.;
const DISABLED_INK_ALPHA: u8 = 97;
const SCRIM_ALPHA: u8 = 82;

/// The current layout of an app-controlled, nonmodal wide navigation rail.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WideNavigationRailMode {
    #[default]
    Collapsed,
    Expanded,
}

/// Localized names and placement for an expanded modal wide navigation rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModalWideNavigationRailOptions<'a> {
    /// Stable ID of the dialog.
    pub id: &'a str,
    /// Visible title and accessible dialog name.
    pub name: &'a str,
    /// Accessible name of the destination group.
    pub navigation_name: &'a str,
    /// Accessible name of the Close button.
    pub close_name: &'a str,
    /// Direction of the page and destination rows.
    pub direction: LayoutDirection,
}

/// The app's collapsed layout for an animated modal wide navigation rail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModalWideNavigationRailCollapse {
    /// Leave a 96 dp rail in the page and grow the overlay from that width.
    Retain,
    /// Slide the standalone panel off the logical start edge when closed.
    Hide {
        /// Current viewport width in logical pixels.
        viewport_width: f32,
    },
}

/// The current frame of a standalone modal rail with swipe enabled.
pub struct ModalWideNavigationRailSwipeOptions<'a> {
    /// Whether the panel owns interaction and focus.
    pub open: bool,
    /// Ordinary modal motion before a drag and after independent dismissal.
    pub frame: WideNavigationRailMotionFrame,
    /// Gesture state retained by the app across rebuilds.
    pub swipe: &'a WideNavigationRailSwipe,
}

struct AnimatedModalRail {
    open: bool,
    collapse: ModalWideNavigationRailCollapse,
    frame: WideNavigationRailMotionFrame,
}

struct ModalRailLayout {
    width_fraction: f32,
    hidden_fraction: f32,
    available_width: f32,
}

impl Theme {
    /// Build a nonmodal rail with a fixed collapsed width and a content-fit
    /// expanded width. The app controls the mode. Place the rail at the logical
    /// start of a row with the growing page content beside it.
    ///
    /// The optional header can hold the app's accessible expand button. The
    /// header is a separate Tab stop from the arrow-key destination group.
    /// This builder reserves `{id}-destinations`.
    ///
    /// [AndroidX wide navigation rail](https://github.com/androidx/androidx/blob/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/WideNavigationRail.kt)
    pub fn wide_navigation_rail<M>(
        &self,
        id: &str,
        name: &str,
        mode: WideNavigationRailMode,
        header: Option<View<M>>,
        items: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        let rail = match mode {
            WideNavigationRailMode::Collapsed => 0.,
            WideNavigationRailMode::Expanded => 1.,
        };
        self.animated_wide_navigation_rail(
            id,
            name,
            WideNavigationRailMotionFrame {
                rail,
                ..WideNavigationRailMotionFrame::default()
            },
            header,
            items,
        )
    }

    /// Build the inline rail at its current motion fraction. The rail's layout
    /// width and item gap follow the transition, so a growing sibling reflows
    /// alongside it. Keep the header and destination IDs stable in both modes.
    /// Select the destination layout from the sampled fraction so changing
    /// the target alone keeps the current item geometry.
    pub fn animated_wide_navigation_rail<M>(
        &self,
        id: &str,
        name: &str,
        frame: WideNavigationRailMotionFrame,
        header: Option<View<M>>,
        items: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        let progress = unit_fraction(frame.rail);
        let moving = progress > 0. && progress < 1.;
        let mut style = Style::column().background(self.colors.surface).clip(moving);
        rail_width(&mut style, progress);
        let item_gap = COLLAPSED_ITEM_GAP * (1. - progress);
        let mut destinations_style = Style::column().gap(item_gap);
        if moving {
            // Measure wrapped rows at the same width cap as the moving rail.
            destinations_style.max_width = style.max_width;
        }
        let destinations = View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-destinations"))
                .styled(destinations_style),
        )
        .children(items)
        .roving(Roving::vertical(false));
        let mut children = vec![space(Style::column().height(Length::Px(TOP_SPACE)))];
        if let Some(header) = header {
            children.extend([
                header,
                space(Style::column().height(Length::Px(HEADER_SPACE))),
            ]);
        }
        children.push(destinations);
        View::new(Node::new(Role::Navigation, name).id(id).styled(style)).children(children)
    }

    /// Build the expanded modal wide navigation rail at the logical start.
    ///
    /// Mount it after the page content while the caller's state is expanded.
    /// The caller can keep a 96 dp collapsed rail in the page or open the
    /// dialog from a separate control. Give simultaneously mounted inline
    /// and modal destinations distinct IDs. Build modal items in
    /// [WideNavigationRailMode::Expanded] with the same direction.
    ///
    /// Close, Escape and a press on the scrim send `dismiss`. Remove the
    /// modal on dismissal to restore the opener's focus. The Close header
    /// stays above the scrollable destinations. The pinned reference also
    /// animates the width and scrim and supports predictive back and a
    /// standalone swipe to dismiss. Use
    /// [Self::swipeable_modal_wide_navigation_rail] for standalone swipe.
    ///
    /// In addition to `id`, this reserves `{id}-layer`, `{id}-header`,
    /// `{id}-title`, `{id}-close` and `{id}-navigation`.
    ///
    /// [AndroidX modal wide navigation rail](https://github.com/androidx/androidx/blob/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/WideNavigationRail.kt#L575)
    /// · [Expanded rail tokens](https://github.com/androidx/androidx/blob/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/NavigationRailExpandedTokens.kt)
    pub fn modal_wide_navigation_rail<M: Clone>(
        &self,
        options: ModalWideNavigationRailOptions<'_>,
        items: impl IntoIterator<Item = View<M>>,
        dismiss: M,
    ) -> View<M> {
        self.modal_wide_rail_panel(
            options,
            options.id,
            items,
            dismiss.clone(),
            ModalRailLayout {
                width_fraction: 1.,
                hidden_fraction: 0.,
                available_width: EXPANDED_MAX_WIDTH,
            },
        )
        .modal_with_backdrop(
            self.navigation_modal_layer(options.id, options.direction, 1.),
            dismiss,
        )
    }

    /// Build the current animated modal layer. `open` controls focus and
    /// interaction immediately. After dismissal, the remaining frame is
    /// painted without actions or accessibility until the motion ends.
    /// The app owns the collapsed rail or opener behind the layer.
    pub fn animated_modal_wide_navigation_rail<M: Clone>(
        &self,
        options: ModalWideNavigationRailOptions<'_>,
        items: impl IntoIterator<Item = View<M>>,
        dismiss: M,
        open: bool,
        collapse: ModalWideNavigationRailCollapse,
        frame: WideNavigationRailMotionFrame,
    ) -> Option<View<M>> {
        self.animated_modal_wide_rail_layer(
            options,
            items,
            dismiss,
            AnimatedModalRail {
                open,
                collapse,
                frame,
            },
            |panel| panel,
        )
    }

    /// Build a standalone modal rail that can be dragged toward the logical
    /// start edge. The panel and scrim follow the same gesture fraction.
    /// The panel keeps its focus scope until the app commits the release.
    ///
    /// Send the physical drag and its measured panel width to
    /// [WideNavigationRailSwipe::apply]. Use
    /// [Self::animated_modal_wide_navigation_rail] for the retained inline rail.
    pub fn swipeable_modal_wide_navigation_rail<M: Clone>(
        &self,
        options: ModalWideNavigationRailOptions<'_>,
        items: impl IntoIterator<Item = View<M>>,
        dismiss: M,
        state: ModalWideNavigationRailSwipeOptions<'_>,
        on_drag: impl Fn(HorizontalDrag, f32) -> M + 'static,
    ) -> Option<View<M>> {
        let ModalWideNavigationRailSwipeOptions {
            open,
            mut frame,
            swipe,
        } = state;
        let viewport_width = swipe.viewport_width();
        let progress = if open { swipe.progress() } else { None };
        let can_drag = open
            && viewport_width > 0.
            && !swipe.is_settling()
            && (unit_fraction(frame.slide) == 1. || progress.is_some());
        if let Some(progress) = progress {
            frame.slide = progress;
            frame.scrim = progress;
        }
        self.animated_modal_wide_rail_layer(
            options,
            items,
            dismiss,
            AnimatedModalRail {
                open,
                collapse: ModalWideNavigationRailCollapse::Hide { viewport_width },
                frame,
            },
            |panel| {
                if can_drag {
                    panel.on_horizontal_drag_with_bounds(
                        swipe.session.clone(),
                        move |drag, bounds| on_drag(drag, bounds.width),
                    )
                } else {
                    panel
                }
            },
        )
    }

    fn animated_modal_wide_rail_layer<M: Clone>(
        &self,
        options: ModalWideNavigationRailOptions<'_>,
        items: impl IntoIterator<Item = View<M>>,
        dismiss: M,
        state: AnimatedModalRail,
        decorate_panel: impl FnOnce(View<M>) -> View<M>,
    ) -> Option<View<M>> {
        let AnimatedModalRail {
            open,
            collapse,
            frame,
        } = state;
        let scrim = unit_fraction(frame.scrim);
        let (width, slide, available) = match collapse {
            ModalWideNavigationRailCollapse::Retain => {
                (unit_fraction(frame.rail), 1., EXPANDED_MAX_WIDTH)
            }
            ModalWideNavigationRailCollapse::Hide { viewport_width } => {
                let available = if viewport_width.is_finite() && viewport_width > 0. {
                    viewport_width.min(EXPANDED_MAX_WIDTH)
                } else {
                    EXPANDED_MAX_WIDTH
                };
                (1., unit_fraction(frame.slide), available)
            }
        };
        let visible = match collapse {
            ModalWideNavigationRailCollapse::Retain => width > 0.,
            ModalWideNavigationRailCollapse::Hide { .. } => slide > 0.,
        };
        if !open && !visible && scrim == 0. {
            return None;
        }
        let closing_id;
        let panel_id = if open {
            options.id
        } else {
            closing_id = format!("{}-closing", options.id);
            &closing_id
        };
        let panel = decorate_panel(self.modal_wide_rail_panel(
            options,
            panel_id,
            items,
            dismiss.clone(),
            ModalRailLayout {
                width_fraction: width,
                hidden_fraction: 1. - slide,
                available_width: available,
            },
        ));
        let backdrop = self.navigation_modal_layer(panel_id, options.direction, scrim);
        Some(if open {
            panel.modal_with_backdrop(backdrop, dismiss)
        } else {
            View::new(backdrop).children([panel]).visual_overlay()
        })
    }

    fn modal_wide_rail_panel<M>(
        &self,
        options: ModalWideNavigationRailOptions<'_>,
        panel_id: &str,
        items: impl IntoIterator<Item = View<M>>,
        dismiss: M,
        layout: ModalRailLayout,
    ) -> View<M> {
        let header = self.navigation_modal_header(
            panel_id,
            options.name,
            options.close_name,
            options.direction,
            dismiss,
        );
        // Retiring the focus scope keeps the same destination scroll position.
        let navigation = View::new(
            Node::new(Role::Navigation, options.navigation_name)
                .id(format!("{}-navigation", options.id))
                .styled(Style::column().height(Length::Px(0.)).grow(1.).clip(true)),
        )
        .children(items.into_iter().map(View::accessibility_scroll_into_view))
        .roving(Roving::vertical(false))
        .accessibility_scroll_pages();
        let mut style = Style::column()
            .height(Length::Percent(1.))
            .corners(shape::large_end(options.direction))
            .background(self.colors.surface_container)
            .clip(true);
        rail_width(&mut style, layout.width_fraction);
        style.min_width = style.min_width.min(layout.available_width);
        if let Length::Px(maximum) = style.max_width {
            style.max_width = Length::Px(maximum.min(layout.available_width));
        }
        let offset = match options.direction {
            LayoutDirection::LeftToRight => -layout.hidden_fraction,
            LayoutDirection::RightToLeft => layout.hidden_fraction,
        };
        // Keep the token spacing on tall panels and give the destinations
        // more room when the header takes up a short panel.
        let inset = |max_height| {
            let mut style = Style::column().height(Length::Percent(0.1));
            style.max_height = Length::Px(max_height);
            space(style)
        };
        View::new(
            Node::new(Role::Dialog, options.name)
                .id(panel_id)
                .box_shadows(self.elevation(Elevation::Level2))
                .slide_horizontally(offset)
                .shrink_to_fit_width()
                .styled(style),
        )
        .children([inset(TOP_SPACE), header, inset(HEADER_SPACE), navigation])
    }

    fn navigation_modal_header<M>(
        &self,
        id: &str,
        name: &str,
        close_name: &str,
        direction: LayoutDirection,
        dismiss: M,
    ) -> View<M> {
        let title = View::new(
            Node::new(Role::Heading, name)
                .id(format!("{id}-title"))
                .text(name)
                .shrink_to_fit_width()
                .styled(
                    Style::default()
                        .with_text_style(typography::TITLE_SMALL)
                        .foreground(self.colors.on_surface_variant),
                ),
        );
        let close = self
            .icon_button_with_text_scaling(
                &format!("{id}-close"),
                close_name,
                ("×", TextScaling::Fixed),
                IconButtonKind::Standard,
                None,
                false,
                dismiss,
            )
            .accessible_activation()
            .protect_composition_on_press();
        let header = Node::new(Role::Group, "")
            .id(format!("{id}-header"))
            .text_scaling(TextScaling::Expand)
            .styled(Style::row().align(Align::Center).padding(12.).gap(12.));
        View::new(in_direction(header, direction)).children([close, title])
    }

    fn navigation_modal_layer(&self, id: &str, direction: LayoutDirection, progress: f32) -> Node {
        let Color(red, green, blue, _) = self.colors.scrim;
        let mut layer = Style::row()
            .fixed(Insets::all(0.))
            .justify(match direction {
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
            .id(format!("{id}-layer"))
            .styled(layer)
    }

    /// Build a wide rail destination with a localizable accessible name.
    /// Use the rail's mode and the page's direction. Reuse the same item ID
    /// in both modes so focus follows the destination across the rebuild.
    pub fn wide_navigation_rail_item_with_icon<M>(
        &self,
        mode: WideNavigationRailMode,
        direction: LayoutDirection,
        item: NavigationRailItem<'_, M>,
    ) -> View<M> {
        let ItemAppearance {
            owner,
            indicator,
            indicator_style,
            ink,
            compact_ink,
        } = self.wide_rail_item_appearance(&item);
        let label_ink = match mode {
            WideNavigationRailMode::Collapsed => compact_ink,
            WideNavigationRailMode::Expanded => ink,
        };
        let icon = item.icon.view(VectorIconSize::Dp24, ink);
        let view = match mode {
            WideNavigationRailMode::Collapsed => {
                let mut row_style = Style::column()
                    .align(Align::Center)
                    .justify(Align::Center)
                    .gap(COLLAPSED_ICON_LABEL_GAP);
                row_style.min_height = COLLAPSED_ITEM_MIN_HEIGHT;
                let indicator = View::new(
                    indicator.styled(
                        indicator_style
                            .justify(Align::Center)
                            .width(Length::Px(COLLAPSED_INDICATOR_WIDTH))
                            .height(Length::Px(COLLAPSED_INDICATOR_HEIGHT)),
                    ),
                )
                .children([icon])
                .without_interaction(None);
                let label = label(
                    item.label,
                    typography::LABEL_MEDIUM,
                    label_ink,
                    Length::Auto,
                );
                View::new(owner.styled(row_style)).children([indicator, label])
            }
            WideNavigationRailMode::Expanded => {
                let label_width = EXPANDED_MAX_WIDTH
                    - 2. * (EXPANDED_ITEM_INSET + EXPANDED_INDICATOR_PADDING)
                    - VectorIconSize::Dp24.dp()
                    - EXPANDED_ICON_LABEL_GAP;
                let indicator = View::new(in_direction(
                    indicator.shrink_to_fit_width().styled(
                        indicator_style
                            .gap(EXPANDED_ICON_LABEL_GAP)
                            .padding(EXPANDED_INDICATOR_PADDING),
                    ),
                    direction,
                ))
                .children([
                    icon,
                    label(
                        item.label,
                        typography::LABEL_LARGE,
                        label_ink,
                        Length::Px(label_width),
                    ),
                ])
                .without_interaction(None);
                let inset = || space(Style::row().width(Length::Px(EXPANDED_ITEM_INSET)));
                let owner =
                    in_direction(owner.styled(Style::row().align(Align::Center)), direction);
                View::new(owner).children([inset(), indicator, inset()])
            }
        }
        .on_press(item.message)
        .accessible_activation();
        if item.state.disabled {
            view
        } else {
            view.cursor(CursorStyle::Pointer)
        }
    }

    fn wide_rail_item_appearance<M>(&self, item: &NavigationRailItem<'_, M>) -> ItemAppearance {
        let (ink, compact_ink) = if item.state.disabled {
            let Color(r, g, b, _) = self.colors.on_surface_variant;
            let disabled = Color(r, g, b, DISABLED_INK_ALPHA);
            (disabled, disabled)
        } else if item.state.selected {
            (self.colors.on_secondary_container, self.colors.secondary)
        } else {
            (
                self.colors.on_surface_variant,
                self.colors.on_surface_variant,
            )
        };
        let fill = if item.state.selected {
            self.colors.secondary_container
        } else {
            Color::TRANSPARENT
        };
        let indicator = Node::new(Role::Group, "")
            .focus_outline_from_parent()
            .focus_color(self.colors.secondary);
        let indicator_style = Style::row()
            .align(Align::Center)
            .corners(shape::FULL)
            .background(fill)
            .state_layer(self.colors.on_secondary_container);
        let owner = Node::new(Role::Button, item.name())
            .id(item.id)
            .selected(item.state.selected)
            .current_page(item.state.selected)
            .disabled(item.state.disabled)
            .focus_color(Color::TRANSPARENT);
        ItemAppearance {
            owner,
            indicator,
            indicator_style,
            ink,
            compact_ink,
        }
    }
}

struct ItemAppearance {
    owner: Node,
    indicator: Node,
    indicator_style: Style,
    ink: Color,
    compact_ink: Color,
}

fn unit_fraction(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0., 1.)
    } else {
        0.
    }
}

fn rail_width(style: &mut Style, progress: f32) {
    if progress == 0. {
        style.width = Length::Px(COLLAPSED_WIDTH);
    } else {
        style.min_width = COLLAPSED_WIDTH + (EXPANDED_MIN_WIDTH - COLLAPSED_WIDTH) * progress;
        style.max_width =
            Length::Px(COLLAPSED_WIDTH + (EXPANDED_MAX_WIDTH - COLLAPSED_WIDTH) * progress);
    }
}

fn space<M>(style: Style) -> View<M> {
    View::new(Node::new(Role::Group, "").styled(style)).without_interaction(None)
}

fn label<M>(text: &str, typography: TextStyle, ink: Color, max_width: Length) -> View<M> {
    let mut style = Style::default().with_text_style(typography).foreground(ink);
    // Bound expanded text during the intrinsic pass. The fixed collapsed rail
    // already supplies its width when measuring the label.
    style.max_width = max_width;
    View::new(
        Node::new(Role::Text, "")
            .text(text)
            .shrink_to_fit_width()
            .styled(style),
    )
    .without_interaction(None)
}

fn in_direction(node: Node, direction: LayoutDirection) -> Node {
    match direction {
        LayoutDirection::LeftToRight => node,
        LayoutDirection::RightToLeft => node.reverse_row(),
    }
}
