//! Material 3 primary and secondary tabs with caller-owned selection.

use std::collections::BTreeSet;

use surface::{
    Align, Color, Corners, Insets, Length, Node, Role, Style, TextAlignment, TextScaling,
};
use surface_app::{CursorStyle, Roving, View};

use crate::{shape::LayoutDirection, typography, Theme, VectorIcon};

mod motion;
pub use motion::{TabBarInput, TabBarMotion};

const MIN_TARGET: f32 = 48.;
const SCROLL_CELL_MIN: f32 = 90.;
const SCROLL_EDGE: f32 = 52.;
const ICON_SIZE: f32 = 24.;
const HOVER_ALPHA: u8 = 20;
const INTERACTION_ALPHA: u8 = 26;

/// A Material 3 tab strip's location in the page hierarchy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabBarKind {
    #[default]
    Primary,
    Secondary,
}

impl TabBarKind {
    fn indicator_height(self) -> f32 {
        match self {
            Self::Primary => 3.,
            Self::Secondary => 2.,
        }
    }
}

/// A single row of equal cells or a clipped row of natural-width cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabBarLayout {
    #[default]
    Fixed,
    Scrollable,
}

/// Choose whether keyboard navigation selects the newly focused tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabActivation {
    #[default]
    Manual,
    /// Use when the caller can display the destination immediately.
    Automatic,
}

/// View settings. Pass the same values to the retained motion controller.
#[derive(Clone, Copy, Debug)]
pub struct TabBarOptions<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub kind: TabBarKind,
    pub layout: TabBarLayout,
    pub direction: LayoutDirection,
    pub available_width: f32,
    pub text_scale: f32,
    /// The accepted tab key. `None` leaves all choices unselected.
    pub selected: Option<&'a str>,
    pub activation: TabActivation,
}

/// One labeled tab, its optional icon, and its controlled selection request.
pub struct TabItem<'a, M> {
    pub id: &'a str,
    pub label: &'a str,
    pub name: Option<&'a str>,
    pub icon: Option<VectorIcon>,
    pub disabled: bool,
    pub panel_id: &'a str,
    pub on_select: M,
}

/// Where Tab goes after leaving the single tab bar entry stop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabPanelEntry {
    /// Put the panel itself in the Tab order when it starts with static content.
    #[default]
    Panel,
    /// Let eligible controls in the panel receive Tab directly.
    Contents,
}

impl Theme {
    /// Build a named Material tab bar with one caller-controlled selection.
    ///
    /// Keep the motion controller across frames and apply this view's
    /// [TabBarInput] messages to it. Call [TabBarMotion::advance] on frames.
    /// Build only the active panel with [Self::tab_panel] next to the bar.
    /// Return `None` for invalid keys, missing choices, and fixed layouts
    /// that cannot assign a 48 dp target to every choice. Allow extra height
    /// for labels which wrap at the caller's text scale.
    pub fn tab_bar<'a, M>(
        &self,
        options: TabBarOptions<'a>,
        items: impl IntoIterator<Item = TabItem<'a, M>>,
        motion: &TabBarMotion,
        on_input: impl Fn(TabBarInput) -> M + 'static,
    ) -> Option<View<M>> {
        let items: Vec<_> = items.into_iter().collect();
        if !valid_items(options, &items) {
            return None;
        }
        let width = options.available_width;
        let scrollable = options.layout == TabBarLayout::Scrollable;
        let fixed_width = width / items.len() as f32;
        if !scrollable && fixed_width < MIN_TARGET {
            return None;
        }
        let has_icon = items.iter().any(|item| item.icon.is_some());
        let strip_height = if has_icon && options.kind == TabBarKind::Primary {
            64.
        } else {
            MIN_TARGET
        };
        let mut strip_children = Vec::with_capacity(items.len() + 3);
        if scrollable {
            strip_children.push(spacer());
        }
        for item in items {
            strip_children.push(self.tab_cell(options, item, motion, fixed_width, strip_height));
        }
        if scrollable {
            strip_children.push(spacer());
        }
        strip_children.push(
            View::new(
                Node::new(Role::Group, "")
                    .id(member(options.id, "indicator"))
                    .styled(bottom_overlay(options.kind.indicator_height())),
            )
            .paint_fill_contour(motion.painter(options.kind, self.colors.primary))
            .without_interaction(None),
        );
        let mut strip_style = Style::row().align(Align::Stretch);
        strip_style.min_width = width;
        if !scrollable {
            strip_style.width = Length::Px(width);
        }
        let strip = View::new(logical_node(
            Node::new(Role::Group, "")
                .id(member(options.id, "strip"))
                .styled(strip_style),
            options.direction,
        ))
        .children(strip_children)
        .roving_retained_layout(Roving::horizontal(
            options.activation == TabActivation::Automatic,
        ));
        let scroll = View::new(
            Node::new(Role::Group, "")
                .id(member(options.id, "scroll"))
                .styled(
                    Style::column()
                        .align(Align::Start)
                        .width(Length::Px(width))
                        .clip(true),
                ),
        )
        .children([strip]);
        let scroll = if scrollable {
            scroll
                .accessibility_scroll_pages()
                .on_horizontal_drag(motion.drag_session(), move |drag| {
                    on_input(TabBarInput::Drag(drag))
                })
        } else {
            scroll
        };
        let divider = View::new(
            Node::new(Role::Group, "")
                .id(member(options.id, "divider"))
                .styled(bottom_overlay(1.).background(self.colors.surface_variant)),
        )
        .without_interaction(None);
        Some(
            View::new(
                Node::new(Role::TabList, options.name)
                    .id(options.id)
                    .styled(
                        Style::column()
                            .width(Length::Px(width))
                            .background(self.colors.surface),
                    ),
            )
            .children([divider, scroll]),
        )
    }

    /// Mount the active content beside its tab bar. The tab supplies the
    /// panel's accessible name. Style the content in its supplied view.
    pub fn tab_panel<M>(
        &self,
        id: &str,
        tablist_id: &str,
        tab_id: &str,
        entry: TabPanelEntry,
        content: View<M>,
    ) -> View<M> {
        let panel = Node::new(Role::Group, "")
            .id(id)
            .tab_panel_labelled_by(tablist_id, tab_id)
            .styled(Style::column().width(Length::Percent(1.)))
            .focus_color(self.colors.secondary)
            .focus_outline_width(3.);
        let panel = match entry {
            TabPanelEntry::Panel => panel.tab_index(0),
            TabPanelEntry::Contents => panel,
        };
        View::new(panel).children([content])
    }

    fn tab_cell<M>(
        &self,
        options: TabBarOptions<'_>,
        item: TabItem<'_, M>,
        motion: &TabBarMotion,
        fixed_width: f32,
        strip_height: f32,
    ) -> View<M> {
        let selected = options.selected == Some(item.id);
        let name = item
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(item.label);
        let on = if options.kind == TabBarKind::Primary {
            self.colors.primary
        } else {
            self.colors.on_surface
        };
        let ink = if item.disabled {
            with_alpha(self.colors.on_surface, 97)
        } else if selected {
            on
        } else if motion.is_interacting(item.id) {
            self.colors.on_surface
        } else {
            self.colors.on_surface_variant
        };
        let layer = if selected { on } else { self.colors.on_surface };
        let content = tab_content(options, &item, ink);
        let mut style = Style::column()
            .padding(8.)
            .align(Align::Center)
            .justify(Align::Center)
            .state_layer(layer);
        style.min_height = strip_height;
        match options.layout {
            TabBarLayout::Fixed => style.width = Length::Px(fixed_width),
            TabBarLayout::Scrollable => {
                style.min_width = SCROLL_CELL_MIN.min(options.available_width);
                style.max_width = Length::Px(options.available_width);
            }
        }
        let focus_outline = View::new(
            Node::new(Role::Group, "")
                .id(member(item.id, "focus-ring"))
                .styled(
                    Style::column()
                        .corners(Corners::all(4.))
                        .absolute(Insets::all(0.)),
                )
                .focus_outline_from_parent()
                .focus_color(self.colors.secondary)
                .focus_outline_width(3.),
        )
        .without_interaction(None);
        let tab = Node::new(Role::Tab, name)
            .id(item.id)
            .tab_controls(item.panel_id)
            .selected(selected)
            .disabled(item.disabled)
            .styled(style)
            .focus_color(Color::TRANSPARENT)
            .state_layer_alphas(HOVER_ALPHA, INTERACTION_ALPHA, INTERACTION_ALPHA);
        let tab = if item.disabled {
            View::new(tab)
        } else {
            View::new(tab.press_state_layer_ink(on).press_ripple_on_focusable())
                .focus_on_pointer_release()
                .cursor(CursorStyle::Pointer)
        }
        .children([content, focus_outline]);
        if item.disabled {
            tab
        } else {
            let tab = tab.protect_composition_on_press();
            let tab = if options.layout == TabBarLayout::Scrollable {
                tab.accessibility_scroll_into_view()
            } else {
                tab
            };
            if selected {
                tab
            } else {
                tab.on_press(item.on_select)
                    .accessible_activation()
                    .suppress_repeated_activation()
            }
        }
    }
}

fn tab_content<M>(options: TabBarOptions<'_>, item: &TabItem<'_, M>, ink: Color) -> View<M> {
    let scale = if options.text_scale > 0. && (options.text_scale * 20.).is_finite() {
        options.text_scale
    } else {
        1.
    };
    let mut typography = typography::TITLE_SMALL;
    typography.font_size *= scale;
    typography.line_height = typography.line_height.map(|height| height * scale);
    typography.letter_spacing *= scale;
    let label = View::new(
        Node::new(Role::Text, "")
            .id(member(item.id, "label"))
            .text(item.label)
            .text_alignment(TextAlignment::Center)
            .styled(Style::column().with_text_style(typography).foreground(ink))
            .text_scaling(TextScaling::Fixed)
            .shrink_to_fit_width(),
    );
    let icon = item.icon.as_ref().map(|icon| {
        View::new(
            Node::new(Role::Group, "")
                .id(member(item.id, "icon"))
                .styled(
                    Style::column()
                        .width(Length::Px(ICON_SIZE))
                        .height(Length::Px(ICON_SIZE)),
                )
                .fill_contour(
                    icon.contour_dp(ICON_SIZE, ink)
                        .expect("validated vector icon"),
                ),
        )
    });
    let mut style = match options.kind {
        TabBarKind::Primary => Style::column().gap(4.),
        TabBarKind::Secondary => Style::row().gap(8.),
    };
    style.max_width = Length::Percent(1.);
    let node = Node::new(Role::Group, "")
        .id(member(item.id, "content"))
        .styled(style.align(Align::Center).justify(Align::Center))
        .shrink_to_fit_width();
    View::new(logical_node(node, options.direction))
        .children(icon.into_iter().chain([label]))
        .without_interaction(None)
}

fn valid_items<M>(options: TabBarOptions<'_>, items: &[TabItem<'_, M>]) -> bool {
    if options.id.trim().is_empty()
        || options.name.trim().is_empty()
        || !options.available_width.is_finite()
        || options.available_width < MIN_TARGET
        || items.is_empty()
        || options
            .selected
            .is_some_and(|id| !items.iter().any(|item| item.id == id))
    {
        return false;
    }
    let mut keys: BTreeSet<_> = ["scroll", "strip", "divider", "indicator"]
        .into_iter()
        .map(|suffix| member(options.id, suffix))
        .chain([options.id.to_owned()])
        .collect();
    items.iter().all(|item| {
        !item.id.trim().is_empty()
            && !item.panel_id.trim().is_empty()
            && !item.label.trim().is_empty()
            && keys.insert(item.id.to_owned())
            && keys.insert(item.panel_id.to_owned())
            && ["content", "label", "focus-ring"]
                .into_iter()
                .chain(item.icon.as_ref().map(|_| "icon"))
                .all(|suffix| keys.insert(member(item.id, suffix)))
    })
}

fn spacer<M>() -> View<M> {
    View::new(Node::new(Role::Group, "").styled(Style::column().width(Length::Px(SCROLL_EDGE))))
        .without_interaction(None)
}

fn with_alpha(color: Color, alpha: u8) -> Color {
    Color(color.0, color.1, color.2, alpha.min(color.3))
}

fn logical_node(node: Node, direction: LayoutDirection) -> Node {
    match direction {
        LayoutDirection::LeftToRight => node,
        LayoutDirection::RightToLeft => node.reverse_row(),
    }
}

fn bottom_overlay(height: f32) -> Style {
    Style::column().height(Length::Px(height)).absolute(Insets {
        left: Length::Px(0.),
        right: Length::Px(0.),
        bottom: Length::Px(0.),
        ..Insets::default()
    })
}

fn member(id: &str, suffix: &str) -> String {
    format!("{id}-{suffix}")
}
