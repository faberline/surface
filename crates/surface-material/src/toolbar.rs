//! Material 3 docked and floating toolbars for page actions.

use surface::{Align, Color, Length, Node, Role, Style, TextScaling};
use surface_app::{CursorStyle, Roving, View};

use crate::{motion::STATE_CHANGE, shape, shape::LayoutDirection, Elevation, Theme};

const CONTAINER_SIZE: f32 = 64.;
const TARGET_SIZE: f32 = 48.;
const BUTTON_SIZE: f32 = 40.;
const DOCKED_EDGE: f32 = 16.;
const FLOATING_PADDING: f32 = 8.;
const ACTION_GAP: f32 = 4.;
const FAB_GAP: f32 = 8.;

/// The position and orientation of the action surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarKind {
    /// A full-width horizontal surface. The caller places it at a layout edge.
    Docked,
    FloatingHorizontal,
    FloatingVertical,
}

/// The colors of the toolbar and its actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolbarPalette {
    #[default]
    Standard,
    Vibrant,
}

/// The caller's current toolbar visibility.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolbarVisibility {
    #[default]
    Expanded,
    /// Keep the floating toolbar's core or its adjacent FAB. A docked toolbar
    /// continues to show all of its controls.
    Collapsed,
    /// Remove the toolbar and its adjacent FAB from input, layout and paint.
    Hidden,
}

/// Placement, palette and focus options for a toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolbarOptions<'a> {
    /// Stable ID of the action surface and its focus group.
    pub id: &'a str,
    /// Accessible name of the group.
    pub name: &'a str,
    pub kind: ToolbarKind,
    pub palette: ToolbarPalette,
    pub direction: LayoutDirection,
    pub visibility: ToolbarVisibility,
    /// Focus target when the focused action disappears. A control may supply
    /// its own target, which is tried before this one.
    pub restore_focus_to: &'a str,
}

/// Actions in logical order. Supply overflow as a caller-controlled action.
pub struct ToolbarContent<M> {
    pub leading: Vec<View<M>>,
    pub core: Vec<View<M>>,
    pub trailing: Vec<View<M>>,
    /// Adjacent floating action button (FAB). Docked bars use an embedded
    /// primary action in the control lists. The FAB has its own Tab stop.
    pub fab: Option<View<M>>,
}

/// A named icon button in the toolbar's palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolbarButtonOptions<'a> {
    pub id: &'a str,
    pub label: &'a str,
    /// Visual glyph. It stays at a fixed 24 dp when text scale changes.
    pub icon: &'a str,
    pub palette: ToolbarPalette,
    /// `None` for a plain action. A Boolean marks a controlled toggle.
    pub selected: Option<bool>,
    pub disabled: bool,
}

impl Theme {
    /// Lay out Material 3 toolbar actions.
    ///
    /// A floating toolbar stays in the supplied layout. Place it with room
    /// for its shadow and at least 16 dp from the relevant viewport edges.
    /// Keep action IDs stable across visibility and palette changes.
    /// Tab reaches one action in the group. Arrow keys move focus without
    /// activating a control. Tab also reaches the optional adjacent FAB.
    ///
    /// The toolbar uses `{id}` for the named action group and `{id}-layout`
    /// for the outer layout. Put the action that opens an overflow menu into
    /// the corresponding action list. The caller owns the menu and messages.
    ///
    /// [Material 3 toolbars](https://m3.material.io/components/toolbars/overview)
    /// [MDC Floating Toolbar tokens](https://github.com/material-components/material-components-android/blob/66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad/lib/java/com/google/android/material/floatingtoolbar/res/values/tokens.xml)
    pub fn toolbar<M>(&self, options: ToolbarOptions<'_>, content: ToolbarContent<M>) -> View<M> {
        let ToolbarContent {
            leading,
            core,
            trailing,
            fab,
        } = content;
        let docked = options.kind == ToolbarKind::Docked;
        let vertical = options.kind == ToolbarKind::FloatingVertical;
        let mut outer_style = toolbar_axis(vertical).align(Align::Center).gap(FAB_GAP);
        if options.kind == ToolbarKind::FloatingHorizontal {
            // A start-, center- or end-aligned parent leaves an auto-width
            // child at its natural size. Bound the pair by that parent so
            // the action surface can shrink and wrap alongside the FAB.
            outer_style.max_width = Length::Percent(1.);
        }
        let layout = View::new(toolbar_direction(
            Node::new(Role::Group, "")
                .id(format!("{}-layout", options.id))
                .styled(if docked {
                    outer_style.width(Length::Percent(1.))
                } else {
                    outer_style
                }),
            options.direction,
        ));
        if options.visibility == ToolbarVisibility::Hidden {
            return layout;
        }
        let fab = if docked { None } else { fab };
        let show_all = docked || options.visibility == ToolbarVisibility::Expanded;
        let surface = (show_all || fab.is_none()).then(|| {
            let items: Vec<_> = leading
                .into_iter()
                .filter(|_| show_all)
                .chain(core)
                .chain(trailing.into_iter().filter(|_| show_all))
                .collect();
            self.toolbar_surface(options, items)
        });
        layout
            .children(surface.into_iter().chain(fab))
            .restore_focus_in_subtree_on_removal(options.restore_focus_to)
    }

    /// Create a toolbar action with a 48 dp hit target and the matching
    /// Material colors for a plain, toggled or disabled control.
    pub fn toolbar_button<M>(&self, options: ToolbarButtonOptions<'_>, on_press: M) -> View<M> {
        let tone = toolbar_colors(self, options.palette, options.selected == Some(true));
        let (background, ink) = if options.disabled {
            let Color(r, g, b, a) = self.colors.on_surface;
            (
                toolbar_colors(self, options.palette, false).0,
                Color(r, g, b, ((u16::from(a) * 38 + 50) / 100) as u8),
            )
        } else {
            tone
        };
        let touch_size = Length::Px(TARGET_SIZE);
        let visual_size = Length::Px(BUTTON_SIZE);
        let mut node = Node::new(Role::Button, options.label)
            .id(options.id)
            .disabled(options.disabled)
            .styled(
                Style::row()
                    .width(touch_size)
                    .height(touch_size)
                    .align(Align::Center)
                    .justify(Align::Center)
                    .corners(shape::FULL),
            )
            .focus_color(toolbar_colors(self, options.palette, false).1);
        if let Some(checked) = options.selected {
            node = node.checked(checked);
        }
        let icon = View::new(
            Node::new(Role::Text, "")
                .text(options.icon)
                .text_scaling(TextScaling::Fixed)
                .styled(Style::default().font_size(24.).foreground(ink)),
        );
        let visual = View::new(
            Node::new(Role::Group, "").styled(
                Style::row()
                    .width(visual_size)
                    .height(visual_size)
                    .align(Align::Center)
                    .justify(Align::Center)
                    .corners(shape::FULL)
                    .background(background)
                    .transition(STATE_CHANGE)
                    .state_layer(if options.disabled {
                        Color::TRANSPARENT
                    } else {
                        ink
                    }),
            ),
        )
        .children([icon]);
        let button = View::new(node).children([visual]);
        if options.disabled {
            button
        } else {
            button
                .on_press(on_press)
                .accessible_activation()
                .protect_composition_on_press()
                .cursor(CursorStyle::Pointer)
        }
    }

    fn toolbar_surface<M>(&self, options: ToolbarOptions<'_>, items: Vec<View<M>>) -> View<M> {
        let vertical = options.kind == ToolbarKind::FloatingVertical;
        let background = toolbar_colors(self, options.palette, false).0;
        let base = toolbar_axis(vertical)
            .align(Align::Center)
            .background(background);
        let (style, children, elevation) = if options.kind == ToolbarKind::Docked {
            let mut docked = base.width(Length::Percent(1.)).corners(shape::NONE);
            docked.min_height = CONTAINER_SIZE;
            let content = View::new(toolbar_direction(
                Node::new(Role::Group, "").styled(
                    Style::row()
                        .width(Length::Px(0.))
                        .grow(1.)
                        .align(Align::Center)
                        .gap(ACTION_GAP)
                        .wrap(true),
                ),
                options.direction,
            ))
            .children(items);
            let edge = || {
                View::new(
                    Node::new(Role::Group, "").styled(Style::row().width(Length::Px(DOCKED_EDGE))),
                )
            };
            (docked, vec![edge(), content, edge()], Elevation::Level0)
        } else {
            let mut floating = base
                .padding(FLOATING_PADDING)
                .gap(ACTION_GAP)
                .corners(shape::FULL);
            if vertical {
                floating.min_width = CONTAINER_SIZE;
            } else {
                floating.min_height = CONTAINER_SIZE;
                floating.max_width = Length::Percent(1.);
                floating.wrap = true;
            }
            (floating, items, Elevation::Level3)
        };
        View::new(toolbar_direction(
            Node::new(Role::Group, options.name)
                .id(options.id)
                .shrink_to_fit_width()
                .styled(style)
                .box_shadows(self.elevation(elevation)),
            options.direction,
        ))
        .children(children)
        .roving(if vertical {
            Roving::vertical(false)
        } else {
            Roving::horizontal(false)
        })
    }
}

fn toolbar_axis(vertical: bool) -> Style {
    if vertical {
        Style::column()
    } else {
        Style::row()
    }
}

fn toolbar_direction(node: Node, direction: LayoutDirection) -> Node {
    match direction {
        LayoutDirection::LeftToRight => node,
        LayoutDirection::RightToLeft => node.reverse_row(),
    }
}

fn toolbar_colors(theme: &Theme, palette: ToolbarPalette, selected: bool) -> (Color, Color) {
    let colors = theme.colors;
    match (palette, selected) {
        (ToolbarPalette::Standard, false) => (colors.surface_container, colors.on_surface_variant),
        (ToolbarPalette::Standard, true) => {
            (colors.secondary_container, colors.on_secondary_container)
        }
        (ToolbarPalette::Vibrant, false) => (colors.primary_container, colors.on_primary_container),
        (ToolbarPalette::Vibrant, true) => (colors.surface_container, colors.on_surface),
    }
}
