use crate::{motion::STATE_CHANGE, shape, tokens::blend, typography, Theme};
use surface::{Align, Color, Corners, Insets, Length, Node, RangeValue, Role, Style};
use surface_app::{Roving, View};

#[derive(Clone, Copy, Debug)]
pub enum ButtonKind {
    Filled,
    Tonal,
    Outlined,
    Text,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ChoiceState {
    pub selected: bool,
    pub disabled: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckValue {
    Unchecked,
    Checked,
    Mixed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentPosition {
    First,
    Middle,
    Last,
    Only,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconButtonKind {
    Standard,
    Filled,
    Tonal,
    Outlined,
}
#[derive(Clone, Copy, Debug)]
pub enum ChipKind {
    Assist,
    Suggestion,
    Filter,
    Input,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DividerKind {
    Full,
    Inset,
    MiddleInset,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FabSize {
    Small,
    Regular,
    Large,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardKind {
    Elevated,
    Filled,
    Outlined,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FabColor {
    Surface,
    Primary,
    Secondary,
    Tertiary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadingKind {
    Default,
    Contained,
}

/// Corner radius of the 38 px indicator for each of the seven shape steps.
/// Even steps are circles (radius 19); odd steps are rounder squares.
fn loading_corners(phase: f32) -> Corners {
    const RADII: [f32; 7] = [19., 12., 19., 8., 19., 14., 19.];
    let phase = if phase.is_finite() {
        phase.clamp(0., 1.)
    } else {
        0.
    };
    let step = ((phase * 7.0).floor() as usize).min(6);
    Corners::all(RADII[step])
}

/// Labels at the two sizes Material names map to their type roles. Any other
/// size stays a plain size-only style, like the checkbox mark glyph.
fn label<M>(text: &str, color: Color, size: f32) -> View<M> {
    let style = if size == 14. {
        Style::default().with_text_style(typography::LABEL_LARGE)
    } else if size == 16. {
        Style::default().with_text_style(typography::BODY_LARGE)
    } else {
        Style::default().font_size(size)
    };
    View::new(
        Node::new(Role::Text, text)
            .text(text)
            .styled(style.foreground(color)),
    )
}
/// The navigation-bar destination label, at the Material LABEL_MEDIUM role.
/// Kept apart from `label`: the checkbox mark also uses a 12 dp size-only style.
fn nav_label<M>(text: &str, color: Color) -> View<M> {
    View::new(
        Node::new(Role::Text, text).text(text).styled(
            Style::default()
                .with_text_style(typography::LABEL_MEDIUM)
                .foreground(color),
        ),
    )
}
fn box_view<M>(style: Style) -> View<M> {
    View::new(Node::new(Role::Group, "").styled(style))
}
fn segment_corners(position: SegmentPosition) -> Corners {
    match position {
        SegmentPosition::First => Corners::start(20.),
        SegmentPosition::Last => Corners::end(20.),
        SegmentPosition::Middle => shape::NONE,
        SegmentPosition::Only => shape::FULL,
    }
}
fn offset(left: Length, top: Length) -> Insets {
    Insets {
        left,
        top,
        ..Default::default()
    }
}

impl Theme {
    pub fn button_with<M>(
        &self,
        id: &str,
        text: &str,
        kind: ButtonKind,
        disabled: bool,
        message: M,
    ) -> View<M> {
        let (background, foreground) = match kind {
            ButtonKind::Filled => (self.primary, self.on_primary),
            ButtonKind::Tonal => (self.tonal, self.on_tonal),
            ButtonKind::Outlined | ButtonKind::Text => (Color::TRANSPARENT, self.primary),
        };
        let mut visual = Style::row()
            .align(Align::Center)
            .justify(Align::Center)
            .padding(8.)
            .height(Length::Px(40.))
            .corners(shape::FULL)
            .state_layer(foreground)
            .background(if disabled { self.disabled } else { background })
            .transition(STATE_CHANGE);
        visual.grow = 1.;
        if matches!(kind, ButtonKind::Outlined) {
            visual = visual.border(1., self.outline);
        }
        let mut touch = Style::row()
            .padding(4.)
            .height(Length::Px(48.))
            .align(Align::Center)
            // The focus ring lands on this node. It follows the pill.
            .foreground(self.primary)
            .corners(shape::FULL);
        touch.min_width = 80.;
        View::new(
            Node::new(Role::Button, text)
                .id(id)
                .disabled(disabled)
                .styled(touch),
        )
        .children([box_view(visual).children([label(
            text,
            if disabled { self.outline } else { foreground },
            14.,
        )])])
        .on_press(message)
    }
    /// A floating action button. `name` is the accessible name; `icon` is a
    /// short text glyph (there is no icon system yet).
    #[allow(clippy::too_many_arguments)]
    pub fn fab<M>(
        &self,
        id: &str,
        name: &str,
        icon: &str,
        size: FabSize,
        color: FabColor,
        disabled: bool,
        message: M,
    ) -> View<M> {
        let (container, corner, icon_size) = match size {
            FabSize::Small => (40., shape::MEDIUM, 24.),
            FabSize::Regular => (56., shape::LARGE, 24.),
            FabSize::Large => (96., shape::EXTRA_LARGE, 36.),
        };
        let (background, foreground) = match color {
            FabColor::Surface => (self.colors.surface_container_high, self.colors.primary),
            FabColor::Primary => (
                self.colors.primary_container,
                self.colors.on_primary_container,
            ),
            FabColor::Secondary => (
                self.colors.secondary_container,
                self.colors.on_secondary_container,
            ),
            FabColor::Tertiary => (
                self.colors.tertiary_container,
                self.colors.on_tertiary_container,
            ),
        };
        let visual = Style::row()
            .align(Align::Center)
            .justify(Align::Center)
            .width(Length::Px(container))
            .height(Length::Px(container))
            .corners(corner)
            .state_layer(foreground)
            .background(if disabled { self.disabled } else { background })
            .transition(STATE_CHANGE);
        // The touch target is at least 48 px, even for the small size.
        let touch_size = container.max(48.);
        let touch = Style::row()
            .align(Align::Center)
            .justify(Align::Center)
            .width(Length::Px(touch_size))
            .height(Length::Px(touch_size))
            // The focus ring lands on this node. It follows the round shape.
            .foreground(self.primary)
            .corners(corner);
        View::new(
            Node::new(Role::Button, name)
                .id(id)
                .disabled(disabled)
                .styled(touch),
        )
        .children([box_view(visual).children([label(
            icon,
            if disabled { self.outline } else { foreground },
            icon_size,
        )])])
        .on_press(message)
    }
    /// Indeterminate loading indicator. The application owns `phase` (0 to 1)
    /// and changes it over time. It draws the shape cycle by corner radius;
    /// it does not rotate or morph polygons yet.
    pub fn loading_indicator<M>(
        &self,
        id: &str,
        name: &str,
        kind: LoadingKind,
        phase: f32,
    ) -> View<M> {
        let indicator = match kind {
            LoadingKind::Default => self.colors.primary,
            LoadingKind::Contained => self.colors.on_primary_container,
        };
        let mut outer = Style::row()
            .align(Align::Center)
            .justify(Align::Center)
            .width(Length::Px(48.))
            .height(Length::Px(48.));
        if kind == LoadingKind::Contained {
            outer = outer
                .corners(shape::FULL)
                .background(self.colors.primary_container);
        }
        let shape = Style::default()
            .width(Length::Px(38.))
            .height(Length::Px(38.))
            .corners(loading_corners(phase))
            .background(indicator);
        View::new(Node::new(Role::Progress, name).id(id).styled(outer)).children([box_view(shape)])
    }
    // The spec fixes this signature; the clippy gate needs the lint allowed here.
    #[allow(clippy::too_many_arguments)]
    pub fn icon_button<M>(
        &self,
        id: &str,
        name: &str,
        icon: &str,
        kind: IconButtonKind,
        selected: Option<bool>,
        disabled: bool,
        message: M,
    ) -> View<M> {
        let on = selected == Some(true);
        let toggle = selected.is_some();
        let c = self.colors;
        let (background, foreground, bordered) = match (kind, toggle, on) {
            (IconButtonKind::Standard, _, true) => (Color::TRANSPARENT, c.primary, false),
            (IconButtonKind::Standard, _, false) => {
                (Color::TRANSPARENT, c.on_surface_variant, false)
            }
            (IconButtonKind::Filled, true, false) => {
                (c.surface_container_highest, c.primary, false)
            }
            (IconButtonKind::Filled, _, _) => (c.primary, c.on_primary, false),
            (IconButtonKind::Tonal, true, false) => {
                (c.surface_container_highest, c.on_surface_variant, false)
            }
            (IconButtonKind::Tonal, _, _) => {
                (c.secondary_container, c.on_secondary_container, false)
            }
            (IconButtonKind::Outlined, _, true) => (c.inverse_surface, c.inverse_on_surface, false),
            (IconButtonKind::Outlined, _, false) => {
                (Color::TRANSPARENT, c.on_surface_variant, true)
            }
        };
        let mut visual = Style::row()
            .align(Align::Center)
            .justify(Align::Center)
            .width(Length::Px(40.))
            .height(Length::Px(40.))
            .corners(shape::FULL)
            .state_layer(foreground)
            .background(if disabled { self.disabled } else { background })
            .transition(STATE_CHANGE);
        if bordered {
            visual = visual.border(1., self.outline);
        }
        let touch = Style::row()
            .width(Length::Px(48.))
            .height(Length::Px(48.))
            .align(Align::Center)
            .justify(Align::Center)
            // The focus ring lands on this node. It follows the circle.
            .foreground(self.primary)
            .corners(shape::FULL);
        let mut node = Node::new(Role::Button, name)
            .id(id)
            .disabled(disabled)
            .styled(touch);
        if let Some(v) = selected {
            node = node.selected(v);
        }
        View::new(node)
            .children([box_view(visual).children([label(
                icon,
                if disabled { self.outline } else { foreground },
                24.,
            )])])
            .on_press(message)
    }

    pub fn checkbox_with<M>(
        &self,
        id: &str,
        text: &str,
        value: CheckValue,
        disabled: bool,
        message: M,
    ) -> View<M> {
        let checked = value != CheckValue::Unchecked;
        let color = if disabled { self.outline } else { self.primary };
        let mark = match value {
            CheckValue::Checked => "✓",
            CheckValue::Mixed => "−",
            _ => "",
        };
        let visual = Style::row()
            .align(Align::Center)
            .justify(Align::Center)
            .width(Length::Px(20.))
            .height(Length::Px(20.))
            // CheckboxTokens ContainerShape is 2dp.
            .corners(Corners::all(2.))
            .border(2., color)
            .background(if checked { color } else { Color::TRANSPARENT })
            .transition(STATE_CHANGE);
        View::new(
            Node::new(Role::Checkbox, text)
                .id(id)
                .checked(checked)
                .mixed(value == CheckValue::Mixed)
                .disabled(disabled)
                .styled(self.selection_style()),
        )
        .children([
            box_view(visual).children([label(mark, self.on_primary, 12.)]),
            label(text, self.on_container, 16.),
        ])
        .on_press(message)
    }
    pub fn radio<M>(&self, id: &str, text: &str, state: ChoiceState, message: M) -> View<M> {
        let ring = if state.disabled {
            self.outline
        } else if state.selected {
            self.primary
        } else {
            self.colors.on_surface_variant
        };
        let dot = if state.disabled {
            self.outline
        } else {
            self.primary
        };
        let label_color = if state.disabled {
            self.outline
        } else {
            self.on_container
        };
        View::new(
            Node::new(Role::Radio, text)
                .id(id)
                .checked(state.selected)
                .disabled(state.disabled)
                .styled(self.selection_style()),
        )
        .children([
            box_view(
                Style::default()
                    .width(Length::Px(20.))
                    .height(Length::Px(20.))
                    .padding(5.)
                    .corners(shape::FULL)
                    .border(2., ring)
                    .transition(STATE_CHANGE),
            )
            .children([box_view(
                Style::default()
                    .width(Length::Px(10.))
                    .height(Length::Px(10.))
                    .corners(shape::FULL)
                    .background(if state.selected {
                        dot
                    } else {
                        Color::TRANSPARENT
                    })
                    .transition(STATE_CHANGE),
            )]),
            label(text, label_color, 16.),
        ])
        .on_press(message)
    }
    pub fn radio_group<M>(
        &self,
        id: &str,
        name: &str,
        children: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        View::new(
            Node::new(Role::RadioGroup, name)
                .id(id)
                .styled(Style::column().gap(4.)),
        )
        .children(children)
        .roving(Roving::vertical(true))
    }
    pub fn switch<M>(&self, id: &str, text: &str, state: ChoiceState, message: M) -> View<M> {
        let color = if state.disabled {
            self.outline
        } else {
            self.primary
        };
        let track = Style::row()
            .width(Length::Px(52.))
            .height(Length::Px(32.))
            .padding(4.)
            .align(Align::Center)
            .justify(if state.selected {
                Align::End
            } else {
                Align::Start
            })
            .corners(shape::FULL)
            .border(2., color)
            .background(if state.selected {
                color
            } else {
                self.input_container
            })
            .transition(STATE_CHANGE);
        View::new(
            Node::new(Role::Switch, text)
                .id(id)
                .checked(state.selected)
                .disabled(state.disabled)
                .styled(self.selection_style()),
        )
        .children([
            box_view(track).children([box_view(
                Style::default()
                    .width(Length::Px(24.))
                    .height(Length::Px(24.))
                    .corners(shape::FULL)
                    .background(if state.selected {
                        self.on_primary
                    } else {
                        self.outline
                    })
                    .transition(STATE_CHANGE),
            )]),
            label(text, self.on_container, 16.),
        ])
        .on_press(message)
    }
    fn selection_style(&self) -> Style {
        let mut style = Style::row()
            .padding(12.)
            .gap(12.)
            .align(Align::Center)
            .foreground(self.primary);
        style.min_height = 48.;
        style
    }
    pub fn segmented_button<M>(
        &self,
        id: &str,
        text: &str,
        state: ChoiceState,
        message: M,
    ) -> View<M> {
        self.choice(id, text, state, Role::Button, message)
    }
    pub fn segmented_buttons<M>(
        &self,
        id: &str,
        name: &str,
        children: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        View::new(
            Node::new(Role::Group, name)
                .id(id)
                .styled(Style::row().gap(0.).wrap(true)),
        )
        .children(children)
        .roving(Roving::horizontal(false))
    }
    pub fn segment<M>(
        &self,
        id: &str,
        text: &str,
        position: SegmentPosition,
        state: ChoiceState,
        message: M,
    ) -> View<M> {
        let (background, foreground) = if state.selected {
            (self.tonal, self.on_tonal)
        } else {
            (Color::TRANSPARENT, self.on_container)
        };
        let mut children = Vec::new();
        if state.selected {
            children.push(label("✓", foreground, 18.));
        }
        children.push(label(
            text,
            if state.disabled {
                self.outline
            } else {
                foreground
            },
            14.,
        ));
        View::new(
            Node::new(Role::Button, text)
                .id(id)
                .selected(state.selected)
                .disabled(state.disabled)
                .styled(
                    Style::row()
                        .padding(12.)
                        .height(Length::Px(40.))
                        .gap(8.)
                        .align(Align::Center)
                        .justify(Align::Center)
                        .background(if state.disabled {
                            self.disabled
                        } else {
                            background
                        })
                        .foreground(self.primary)
                        .state_layer(foreground)
                        .border(1., self.outline)
                        .corners(segment_corners(position))
                        .transition(STATE_CHANGE),
                ),
        )
        .children(children)
        .on_press(message)
    }

    fn choice<M>(
        &self,
        id: &str,
        text: &str,
        state: ChoiceState,
        role: Role,
        message: M,
    ) -> View<M> {
        let (background, foreground) = if state.selected {
            (self.tonal, self.on_tonal)
        } else {
            (Color::TRANSPARENT, self.on_container)
        };
        View::new(
            Node::new(role, text)
                .id(id)
                .selected(state.selected)
                .disabled(state.disabled)
                .styled(
                    Style::row()
                        .padding(12.)
                        .height(Length::Px(48.))
                        .align(Align::Center)
                        .justify(Align::Center)
                        .background(if state.disabled {
                            self.disabled
                        } else {
                            background
                        })
                        .foreground(self.primary)
                        .state_layer(foreground)
                        .border(1., self.outline)
                        .corners(shape::FULL)
                        .transition(STATE_CHANGE),
                ),
        )
        .children([label(text, foreground, 14.)])
        .on_press(message)
    }
    pub fn tab<M>(&self, id: &str, text: &str, state: ChoiceState, message: M) -> View<M> {
        let foreground = if state.disabled {
            self.outline
        } else if state.selected {
            self.primary
        } else {
            self.colors.on_surface_variant
        };
        let mut children = vec![label(text, foreground, 14.)];
        if state.selected {
            children.push(box_view(
                Style::default()
                    .height(Length::Px(3.))
                    // PrimaryNavigationTabTokens ActiveIndicatorShape is 3dp.
                    .corners(Corners::all(3.))
                    .background(self.primary)
                    .transition(STATE_CHANGE)
                    .absolute(Insets {
                        left: Length::Px(12.),
                        right: Length::Px(12.),
                        bottom: Length::Px(0.),
                        ..Default::default()
                    }),
            ));
        }
        View::new(
            Node::new(Role::Tab, text)
                .id(id)
                .selected(state.selected)
                .disabled(state.disabled)
                .styled(
                    Style::row()
                        .height(Length::Px(48.))
                        .padding(12.)
                        .align(Align::Center)
                        .justify(Align::Center)
                        .foreground(self.primary)
                        .state_layer(foreground),
                ),
        )
        .children(children)
        .on_press(message)
    }
    pub fn tabs<M>(
        &self,
        id: &str,
        name: &str,
        children: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        View::new(
            Node::new(Role::TabList, name)
                .id(id)
                .styled(Style::row().gap(4.).wrap(true)),
        )
        .children(children)
        .roving(Roving::horizontal(false))
    }
    pub fn navigation_bar_item<M>(
        &self,
        id: &str,
        name: &str,
        icon: &str,
        state: ChoiceState,
        message: M,
    ) -> View<M> {
        let (icon_color, label_color) = if state.disabled {
            (self.outline, self.outline)
        } else if state.selected {
            (self.colors.on_secondary_container, self.colors.on_surface)
        } else {
            (
                self.colors.on_surface_variant,
                self.colors.on_surface_variant,
            )
        };
        let indicator = box_view(
            Style::row()
                .align(Align::Center)
                .justify(Align::Center)
                .width(Length::Px(64.))
                .height(Length::Px(32.))
                .corners(shape::FULL)
                .background(if state.selected {
                    self.colors.secondary_container
                } else {
                    Color::TRANSPARENT
                })
                .transition(STATE_CHANGE),
        )
        .children([label(icon, icon_color, 24.)]);
        View::new(
            Node::new(Role::Button, name)
                .id(id)
                .selected(state.selected)
                .disabled(state.disabled)
                .styled(
                    Style::column()
                        .align(Align::Center)
                        .justify(Align::Center)
                        .gap(4.)
                        .height(Length::Px(80.))
                        .grow(1.)
                        .foreground(self.primary)
                        .state_layer(label_color),
                ),
        )
        .children([indicator, nav_label(name, label_color)])
        .on_press(message)
    }
    pub fn navigation_bar<M>(
        &self,
        id: &str,
        name: &str,
        items: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        View::new(
            Node::new(Role::Navigation, name).id(id).styled(
                Style::row()
                    .align(Align::Center)
                    .width(Length::Percent(1.))
                    .height(Length::Px(80.))
                    .background(self.colors.surface_container),
            ),
        )
        .children(items)
        .roving(Roving::horizontal(false))
    }
    pub fn navigation_rail_item<M>(
        &self,
        id: &str,
        name: &str,
        icon: &str,
        state: ChoiceState,
        message: M,
    ) -> View<M> {
        let (icon_color, label_color) = if state.disabled {
            (self.outline, self.outline)
        } else if state.selected {
            (self.colors.on_secondary_container, self.colors.on_surface)
        } else {
            (
                self.colors.on_surface_variant,
                self.colors.on_surface_variant,
            )
        };
        let indicator = box_view(
            Style::row()
                .align(Align::Center)
                .justify(Align::Center)
                .width(Length::Px(56.))
                .height(Length::Px(32.))
                .corners(shape::FULL)
                .background(if state.selected {
                    self.colors.secondary_container
                } else {
                    Color::TRANSPARENT
                })
                .transition(STATE_CHANGE),
        )
        .children([label(icon, icon_color, 24.)]);
        View::new(
            Node::new(Role::Button, name)
                .id(id)
                .selected(state.selected)
                .disabled(state.disabled)
                .styled(
                    Style::column()
                        .align(Align::Center)
                        .justify(Align::Center)
                        .gap(4.)
                        .width(Length::Percent(1.))
                        .height(Length::Px(64.))
                        .padding(4.)
                        .foreground(self.primary)
                        .state_layer(label_color),
                ),
        )
        .children([indicator, nav_label(name, label_color)])
        .on_press(message)
    }
    pub fn navigation_rail<M>(
        &self,
        id: &str,
        name: &str,
        items: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        View::new(
            Node::new(Role::Navigation, name).id(id).styled(
                Style::column()
                    .align(Align::Center)
                    .gap(12.)
                    .padding(12.)
                    .width(Length::Px(80.))
                    .background(self.colors.surface),
            ),
        )
        .children(items)
        .roving(Roving::vertical(false))
    }
    pub fn navigation_drawer_item<M>(
        &self,
        id: &str,
        name: &str,
        icon: &str,
        state: ChoiceState,
        message: M,
    ) -> View<M> {
        let color = if state.disabled {
            self.outline
        } else if state.selected {
            self.colors.on_secondary_container
        } else {
            self.colors.on_surface_variant
        };
        View::new(
            Node::new(Role::Button, name)
                .id(id)
                .selected(state.selected)
                .disabled(state.disabled)
                .styled(
                    Style::row()
                        .align(Align::Center)
                        .gap(12.)
                        .height(Length::Px(56.))
                        .width(Length::Percent(1.))
                        .padding(16.)
                        .corners(shape::FULL)
                        .background(if state.selected {
                            self.colors.secondary_container
                        } else {
                            Color::TRANSPARENT
                        })
                        .foreground(self.primary)
                        .state_layer(color)
                        .transition(STATE_CHANGE),
                ),
        )
        .children([label(icon, color, 24.), label(name, color, 14.)])
        .on_press(message)
    }
    pub fn navigation_drawer<M>(
        &self,
        id: &str,
        name: &str,
        items: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        View::new(
            Node::new(Role::Navigation, name)
                .id(id)
                .styled(self.drawer_style()),
        )
        .children(items)
        .roving(Roving::vertical(false))
    }
    pub fn modal_navigation_drawer<M>(
        &self,
        id: &str,
        name: &str,
        items: impl IntoIterator<Item = View<M>>,
        dismiss: M,
    ) -> View<M> {
        let mut style = self.drawer_style();
        style.height = Length::Percent(1.);
        style.max_width = Length::Percent(1.);
        let panel = View::new(
            Node::new(Role::Navigation, name)
                .id(id)
                .tab_index(-1)
                .styled(style),
        )
        .children(items)
        .roving(Roving::vertical(false))
        .modal(dismiss);
        let mut layer = Style::row()
            .fixed(Insets::all(0.))
            .background(Color(0, 0, 0, 80));
        layer.z_index = 1000;
        View::new(
            Node::new(Role::Group, "Drawer overlay")
                .id(format!("{id}-layer"))
                .styled(layer),
        )
        .children([panel])
    }
    fn drawer_style(&self) -> Style {
        Style::column()
            .width(Length::Px(360.))
            .padding(12.)
            .gap(0.)
            .corners(shape::EXTRA_LARGE)
            .background(self.colors.surface)
    }
    pub fn chip<M>(
        &self,
        id: &str,
        text: &str,
        kind: ChipKind,
        state: ChoiceState,
        message: M,
        remove: Option<M>,
    ) -> View<M> {
        let mut children = vec![label(
            text,
            if state.selected {
                self.on_tonal
            } else {
                self.on_container
            },
            14.,
        )];
        if matches!(kind, ChipKind::Input) {
            if let Some(message) = remove {
                children.push(
                    View::new(
                        Node::new(Role::Button, format!("Remove {text}"))
                            .id(format!("{id}-remove"))
                            .text("×")
                            .styled(
                                Style::default()
                                    .width(Length::Px(24.))
                                    .height(Length::Px(24.))
                                    .foreground(self.on_container),
                            ),
                    )
                    .on_press(message),
                );
            }
        }
        View::new(
            Node::new(Role::Button, text)
                .id(id)
                .selected(state.selected)
                .disabled(state.disabled)
                .styled(
                    Style::row()
                        .padding(8.)
                        .gap(8.)
                        .align(Align::Center)
                        .corners(shape::SMALL)
                        .foreground(self.primary)
                        .border(1., self.outline)
                        .background(if state.selected {
                            self.tonal
                        } else {
                            Color::TRANSPARENT
                        })
                        .transition(STATE_CHANGE),
                ),
        )
        .children(children)
        .on_press(message)
    }
    pub fn slider<M>(
        &self,
        id: &str,
        name: &str,
        range: RangeValue,
        disabled: bool,
        handler: impl Fn(f64) -> M + 'static,
    ) -> View<M> {
        let fraction = if range.is_valid() {
            ((range.value - range.min) / (range.max - range.min)) as f32
        } else {
            0.
        };
        let color = if disabled { self.outline } else { self.primary };
        let track = box_view(
            Style::default()
                .width(Length::Percent(1.))
                .height(Length::Px(24.)),
        )
        .children([
            box_view(
                Style::default()
                    .height(Length::Px(4.))
                    .corners(shape::FULL)
                    .background(self.input_container)
                    .absolute(Insets {
                        left: Length::Px(0.),
                        right: Length::Px(0.),
                        top: Length::Px(10.),
                        ..Default::default()
                    }),
            ),
            box_view(
                Style::default()
                    .width(Length::Percent(fraction))
                    .height(Length::Px(4.))
                    .corners(shape::FULL)
                    .background(color)
                    .transition(STATE_CHANGE)
                    .absolute(offset(Length::Px(0.), Length::Px(10.))),
            ),
            box_view(
                Style::default()
                    .width(Length::Px(0.))
                    .height(Length::Px(24.))
                    .absolute(offset(Length::Percent(fraction), Length::Px(0.))),
            )
            .children([box_view(
                Style::default()
                    .width(Length::Px(20.))
                    .height(Length::Px(20.))
                    .corners(shape::FULL)
                    .background(color)
                    .transition(STATE_CHANGE)
                    .absolute(offset(Length::Px(-10.), Length::Px(2.))),
            )]),
        ]);
        View::new(
            Node::new(Role::Slider, name)
                .id(id)
                .range(range)
                .disabled(disabled)
                .styled(
                    Style::default()
                        .width(Length::Percent(1.))
                        .height(Length::Px(48.))
                        .padding(12.)
                        .foreground(color),
                ),
        )
        .children([track])
        .on_value(handler)
    }
    /// Linear progress: determinate when `value` is `Some`, otherwise an
    /// indeterminate bar whose position follows the application's `phase`.
    pub fn linear_progress<M>(
        &self,
        id: &str,
        name: &str,
        value: Option<f32>,
        phase: f32,
    ) -> View<M> {
        let clamp = |v: f32| if v.is_finite() { v.clamp(0., 1.) } else { 0. };
        let (range, bar) = match value {
            Some(v) => {
                let f = clamp(v);
                (
                    Some(RangeValue {
                        min: 0.,
                        max: 1.,
                        value: f as f64,
                        step: 0.01,
                    }),
                    Style::default()
                        .width(Length::Percent(f))
                        .height(Length::Px(4.))
                        .corners(shape::FULL)
                        .background(self.primary)
                        .absolute(offset(Length::Px(0.), Length::Px(0.))),
                )
            }
            None => {
                let p = clamp(phase);
                (
                    None,
                    Style::default()
                        .width(Length::Percent(0.4))
                        .height(Length::Px(4.))
                        .corners(shape::FULL)
                        .background(self.primary)
                        .absolute(offset(Length::Percent(p * 0.6), Length::Px(0.))),
                )
            }
        };
        let track = box_view(
            Style::default()
                .width(Length::Percent(1.))
                .height(Length::Px(4.)),
        )
        .children([
            box_view(
                Style::default()
                    .width(Length::Percent(1.))
                    .height(Length::Px(4.))
                    .corners(shape::FULL)
                    .background(self.colors.secondary_container)
                    .absolute(offset(Length::Px(0.), Length::Px(0.))),
            ),
            box_view(bar),
        ]);
        let mut node = Node::new(Role::Progress, name).id(id);
        if let Some(range) = range {
            node = node.range(range);
        }
        View::new(
            node.styled(
                Style::default()
                    .width(Length::Px(240.))
                    .height(Length::Px(4.)),
            ),
        )
        .children([track])
    }
    pub fn dialog<M>(
        &self,
        id: &str,
        title: &str,
        body: impl IntoIterator<Item = View<M>>,
        actions: impl IntoIterator<Item = View<M>>,
        dismiss: M,
    ) -> View<M> {
        let mut content = vec![View::new(
            Node::new(Role::Heading, title).text(title).styled(
                Style::default()
                    .with_text_style(typography::HEADLINE_SMALL)
                    .foreground(self.on_container),
            ),
        )];
        content.extend(body);
        content.push(
            View::new(
                Node::new(Role::Group, "Dialog actions")
                    .styled(Style::row().gap(8.).justify(Align::End).wrap(true)),
            )
            .children(actions),
        );
        let mut style = Style::column()
            .width(Length::Percent(1.))
            .padding(24.)
            .gap(16.)
            .corners(shape::EXTRA_LARGE)
            .background(self.container)
            .clip(true);
        style.max_width = Length::Px(560.);
        style.max_height = Length::Percent(1.);
        let panel = View::new(
            Node::new(Role::Dialog, title)
                .id(id)
                .tab_index(-1)
                .styled(style),
        )
        .children(content)
        .modal(dismiss);
        let mut layer = Style::default()
            .fixed(Insets::all(0.))
            .padding(24.)
            .align(Align::Center)
            .justify(Align::Center)
            .background(Color(0, 0, 0, 80));
        layer.z_index = 1000;
        View::new(
            Node::new(Role::Group, "Dialog overlay")
                .id(format!("{id}-layer"))
                .styled(layer),
        )
        .children([panel])
    }
    /// Basic dialog with an optional icon, a headline, optional supporting
    /// text, extra content and actions, using the M3 dialog tokens.
    #[allow(clippy::too_many_arguments)]
    pub fn dialog_with<M>(
        &self,
        id: &str,
        icon: Option<&str>,
        headline: &str,
        supporting: &str,
        extra: impl IntoIterator<Item = View<M>>,
        actions: impl IntoIterator<Item = View<M>>,
        dismiss: M,
    ) -> View<M> {
        let mut content = Vec::new();
        if let Some(glyph) = icon {
            content.push(
                View::new(
                    Node::new(Role::Group, "Dialog icon")
                        .id(format!("{id}-icon"))
                        .styled(Style::default().align(Align::Center)),
                )
                .children([label(glyph, self.colors.secondary, 24.)]),
            );
        }
        content.push(View::new(
            Node::new(Role::Heading, headline)
                .id(format!("{id}-headline"))
                .text(headline)
                .styled(
                    Style::default()
                        .with_text_style(typography::HEADLINE_SMALL)
                        .foreground(self.colors.on_surface),
                ),
        ));
        if !supporting.is_empty() {
            content.push(View::new(
                Node::new(Role::Text, supporting)
                    .id(format!("{id}-supporting"))
                    .text(supporting)
                    .styled(
                        Style::default()
                            .with_text_style(typography::BODY_MEDIUM)
                            .foreground(self.colors.on_surface_variant),
                    ),
            ));
        }
        content.extend(extra);
        content.push(
            View::new(
                Node::new(Role::Group, "Dialog actions")
                    .styled(Style::row().gap(8.).justify(Align::End).wrap(true)),
            )
            .children(actions),
        );
        let mut style = Style::column()
            .width(Length::Percent(1.))
            .padding(24.)
            .gap(16.)
            .corners(shape::EXTRA_LARGE)
            .background(self.colors.surface_container_high)
            .clip(true);
        style.max_width = Length::Px(560.);
        style.max_height = Length::Percent(1.);
        if icon.is_some() {
            style.align = Align::Center;
        }
        let panel = View::new(
            Node::new(Role::Dialog, headline)
                .id(id)
                .tab_index(-1)
                .styled(style),
        )
        .children(content)
        .modal(dismiss);
        let sc = self.colors.scrim;
        let mut layer = Style::default()
            .fixed(Insets::all(0.))
            .padding(24.)
            .align(Align::Center)
            .justify(Align::Center)
            .background(Color(sc.0, sc.1, sc.2, 82));
        layer.z_index = 1000;
        View::new(
            Node::new(Role::Group, "Dialog overlay")
                .id(format!("{id}-layer"))
                .styled(layer),
        )
        .children([panel])
    }
    pub fn bottom_sheet<M>(
        &self,
        id: &str,
        name: &str,
        content: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        self.sheet_panel(id, name, Role::Group, content, None)
    }
    pub fn modal_bottom_sheet<M>(
        &self,
        id: &str,
        name: &str,
        content: impl IntoIterator<Item = View<M>>,
        dismiss: M,
    ) -> View<M> {
        let panel = self.sheet_panel(id, name, Role::Dialog, content, Some(dismiss));
        let mut layer = Style::default()
            .fixed(Insets::all(0.))
            .align(Align::Center)
            .justify(Align::End)
            .background(Color(0, 0, 0, 80));
        layer.z_index = 1000;
        View::new(
            Node::new(Role::Group, "Sheet overlay")
                .id(format!("{id}-layer"))
                .styled(layer),
        )
        .children([panel])
    }
    fn sheet_panel<M>(
        &self,
        id: &str,
        name: &str,
        role: Role,
        content: impl IntoIterator<Item = View<M>>,
        dismiss: Option<M>,
    ) -> View<M> {
        let mut style = Style::column()
            .align(Align::Center)
            .width(Length::Percent(1.))
            .padding(16.)
            .gap(16.)
            .corners(shape::EXTRA_LARGE_TOP)
            .background(self.colors.surface_container_low)
            .clip(true);
        style.max_width = Length::Px(640.);
        let mut children = vec![box_view(
            Style::default()
                .width(Length::Px(32.))
                .height(Length::Px(4.))
                .corners(Corners::all(2.))
                .background(self.colors.on_surface_variant),
        )];
        children.extend(content);
        let panel =
            View::new(Node::new(role, name).id(id).tab_index(-1).styled(style)).children(children);
        match dismiss {
            Some(dismiss) => panel.modal(dismiss),
            None => panel,
        }
    }
    pub fn menu_item<M>(&self, id: &str, text: &str, disabled: bool, message: M) -> View<M> {
        View::new(
            Node::new(Role::MenuItem, text)
                .id(id)
                .disabled(disabled)
                .styled(
                    Style::row()
                        .padding(12.)
                        .height(Length::Px(48.))
                        .align(Align::Center)
                        .foreground(self.primary),
                ),
        )
        .children([label(
            text,
            if disabled {
                self.outline
            } else {
                self.on_container
            },
            14.,
        )])
        .on_press(message)
    }
    pub fn menu_item_with<M>(
        &self,
        id: &str,
        text: &str,
        leading: Option<&str>,
        trailing: Option<&str>,
        state: ChoiceState,
        message: M,
    ) -> View<M> {
        let text_color = if state.disabled {
            self.outline
        } else if state.selected {
            self.colors.on_secondary_container
        } else {
            self.on_container
        };
        let muted = if state.disabled {
            self.outline
        } else if state.selected {
            self.colors.on_secondary_container
        } else {
            self.colors.on_surface_variant
        };
        let mut children = Vec::new();
        if let Some(glyph) = leading {
            children.push(label(glyph, muted, 24.));
        }
        children.push(label(text, text_color, 14.));
        if let Some(shortcut) = trailing {
            children.push(box_view(Style::default().grow(1.)));
            children.push(label(shortcut, muted, 14.));
        }
        View::new(
            Node::new(Role::MenuItem, text)
                .id(id)
                .selected(state.selected)
                .disabled(state.disabled)
                .styled(
                    Style::row()
                        .align(Align::Center)
                        .gap(12.)
                        .padding(12.)
                        .height(Length::Px(48.))
                        .width(Length::Percent(1.))
                        .background(if state.selected {
                            self.colors.secondary_container
                        } else {
                            Color::TRANSPARENT
                        })
                        .foreground(self.primary)
                        .state_layer(text_color)
                        .transition(STATE_CHANGE),
                ),
        )
        .children(children)
        .on_press(message)
    }
    pub fn menu_at<M>(
        &self,
        id: &str,
        name: &str,
        position: (f32, f32),
        items: impl IntoIterator<Item = View<M>>,
        dismiss: M,
    ) -> View<M> {
        let mut style = Style::column()
            .width(Length::Px(240.))
            .padding(8.)
            .corners(shape::EXTRA_SMALL)
            .background(self.colors.surface_container_high)
            .clip(true)
            .fixed(offset(Length::Px(position.0), Length::Px(position.1)));
        style.z_index = 1001;
        View::new(
            Node::new(Role::Menu, name)
                .id(id)
                .tab_index(-1)
                .styled(style),
        )
        .children(items)
        .roving(Roving::vertical(false))
        .modal(dismiss)
    }
    pub fn divider<M>(&self, id: &str) -> View<M> {
        View::new(
            Node::new(Role::Separator, "Divider").id(id).styled(
                Style::default()
                    .height(Length::Px(1.))
                    .background(self.colors.outline_variant),
            ),
        )
    }
    pub fn divider_with<M>(&self, id: &str, kind: DividerKind) -> View<M> {
        match kind {
            DividerKind::Full => self.divider(id),
            DividerKind::Inset | DividerKind::MiddleInset => {
                let spacer = || {
                    View::new(
                        Node::new(Role::Group, "Divider spacer")
                            .styled(Style::default().width(Length::Px(16.))),
                    )
                };
                let line = View::new(
                    Node::new(Role::Separator, "Divider").id(id).styled(
                        Style::default()
                            .height(Length::Px(1.))
                            .grow(1.)
                            .background(self.colors.outline_variant),
                    ),
                );
                let mut items = vec![spacer(), line];
                if kind == DividerKind::MiddleInset {
                    items.push(spacer());
                }
                View::new(
                    Node::new(Role::Group, "Divider inset")
                        .id(format!("{id}-inset"))
                        .styled(Style::row().width(Length::Percent(1.))),
                )
                .children(items)
            }
        }
    }
    pub fn vertical_divider<M>(&self, id: &str) -> View<M> {
        View::new(
            Node::new(Role::Separator, "Vertical divider")
                .id(id)
                .styled(
                    Style::default()
                        .width(Length::Px(1.))
                        .height(Length::Percent(1.))
                        .background(self.colors.outline_variant),
                ),
        )
    }
    pub fn search_bar<M>(
        &self,
        id: &str,
        name: &str,
        query: &str,
        leading: &str,
        on_input: impl Fn(String) -> M + 'static,
        clear: M,
    ) -> View<M> {
        let input = View::new(
            Node::new(Role::TextInput, name)
                .id(format!("{id}-input"))
                .text(query)
                .styled(
                    Style::default()
                        .with_text_style(typography::BODY_LARGE)
                        .grow(1.)
                        .foreground(self.colors.on_surface),
                ),
        )
        .on_input(on_input);
        let mut children = vec![label(leading, self.colors.on_surface_variant, 24.), input];
        if !query.is_empty() {
            children.push(
                View::new(
                    Node::new(Role::Button, "Clear")
                        .id(format!("{id}-clear"))
                        .styled(
                            Style::row()
                                .align(Align::Center)
                                .justify(Align::Center)
                                .width(Length::Px(40.))
                                .height(Length::Px(40.))
                                .corners(shape::FULL)
                                .foreground(self.primary)
                                .state_layer(self.colors.on_surface_variant)
                                .transition(STATE_CHANGE),
                        ),
                )
                .children([label("×", self.colors.on_surface_variant, 24.)])
                .on_press(clear),
            );
        }
        View::new(
            Node::new(Role::Group, name).id(id).styled(
                Style::row()
                    .align(Align::Center)
                    .gap(16.)
                    .padding(16.)
                    .height(Length::Px(56.))
                    .width(Length::Percent(1.))
                    .corners(shape::FULL)
                    .background(self.colors.surface_container_high),
            ),
        )
        .children(children)
    }
    pub fn outlined_card<M>(
        &self,
        id: &str,
        children: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        View::new(
            Node::new(Role::Group, "Card").id(id).styled(
                Style::column()
                    .padding(24.)
                    .gap(16.)
                    .corners(shape::MEDIUM)
                    .background(self.background)
                    .border(1., self.outline),
            ),
        )
        .children(children)
    }
    /// A card with one of the three M3 container kinds. `name` is the accessible name.
    pub fn card_with<M>(
        &self,
        id: &str,
        name: &str,
        kind: CardKind,
        children: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        View::new(
            Node::new(Role::Group, name)
                .id(id)
                .styled(self.card_style(kind, false)),
        )
        .children(children)
    }
    /// A pressable card. It is a button with focus, press and disabled states.
    #[allow(clippy::too_many_arguments)]
    pub fn action_card<M>(
        &self,
        id: &str,
        name: &str,
        kind: CardKind,
        disabled: bool,
        message: M,
        children: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        View::new(
            Node::new(Role::Button, name)
                .id(id)
                .disabled(disabled)
                .styled(self.card_style(kind, disabled)),
        )
        .children(children)
        .on_press(message)
    }
    fn card_style(&self, kind: CardKind, disabled: bool) -> Style {
        let style = Style::column()
            .padding(16.)
            .gap(8.)
            .corners(shape::MEDIUM)
            .foreground(self.colors.on_surface)
            .state_layer(self.colors.on_surface);
        let style = match kind {
            CardKind::Elevated => style.background(self.colors.surface_container_low),
            CardKind::Filled => style.background(self.colors.surface_container_highest),
            CardKind::Outlined => style
                .background(self.colors.surface)
                .border(1., self.colors.outline_variant),
        };
        if disabled {
            style.background(self.disabled).foreground(self.outline)
        } else {
            style
        }
    }
    /// A transient interaction layer over an opaque component surface.
    pub fn state_layer(
        &self,
        surface: Color,
        foreground: Color,
        pressed: bool,
        hovered: bool,
    ) -> Color {
        blend(
            surface,
            foreground,
            if pressed {
                0.12
            } else if hovered {
                0.08
            } else {
                0.
            },
        )
    }
}
