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
#[derive(Clone, Copy, Debug)]
pub enum ChipKind {
    Assist,
    Suggestion,
    Filter,
    Input,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FabSize {
    Small,
    Regular,
    Large,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FabColor {
    Surface,
    Primary,
    Secondary,
    Tertiary,
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
        let color = if state.disabled {
            self.outline
        } else {
            self.primary
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
                    .border(2., color)
                    .transition(STATE_CHANGE),
            )
            .children([box_view(
                Style::default()
                    .width(Length::Px(10.))
                    .height(Length::Px(10.))
                    .corners(shape::FULL)
                    .background(if state.selected {
                        color
                    } else {
                        Color::TRANSPARENT
                    })
                    .transition(STATE_CHANGE),
            )]),
            label(text, self.on_container, 16.),
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
