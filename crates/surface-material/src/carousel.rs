//! Material 3 carousels with controlled scroll and stable item actions.

use std::rc::Rc;

use surface::{Align, Insets, Length, Node, Rect, Role, Style};
use surface_app::{
    CursorStyle, EditKey, FocusNavigation, HorizontalScrollResponse, VerticalScrollResponse, View,
};

use crate::{
    shape, shape::LayoutDirection, CarouselInput, CarouselLayout, CarouselSlot, CarouselState,
    Theme,
};

const MIN_ACTION_SIZE: f32 = 48.;

/// A Material 3 carousel arrangement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CarouselKind {
    #[default]
    MultiBrowse,
    /// Equal-size cards that flow past the viewport edge.
    Uncontained,
    /// Free scroll with each item's aspect ratio clamped to 9:16 through 16:9.
    UncontainedMultiAspectRatio,
    HeroStart,
    HeroCenter,
    /// One edge-to-edge card per page. Use the vertical orientation in portrait.
    FullScreen,
}

impl CarouselKind {
    pub(crate) fn contained(self) -> bool {
        !matches!(self, Self::Uncontained | Self::UncontainedMultiAspectRatio)
    }

    pub(crate) fn single_step(self) -> bool {
        matches!(self, Self::HeroStart | Self::HeroCenter | Self::FullScreen)
    }
}

/// The carousel's scroll axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CarouselOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Viewport and appearance choices. Use the same values for view and layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarouselOptions<'a> {
    /// Stable ID of the clipped viewport and scroll input target.
    pub id: &'a str,
    pub name: &'a str,
    pub kind: CarouselKind,
    pub orientation: CarouselOrientation,
    pub direction: LayoutDirection,
    /// Allocated viewport width and height in logical pixels.
    pub viewport: (f32, f32),
    /// Desired fully visible item size along the scroll axis.
    pub preferred_item_extent: f32,
    pub reduced_motion: bool,
}

/// One carousel action, with caller-owned passive artwork and text.
pub struct CarouselItem<M> {
    pub id: String,
    /// Localized accessible item name, including its position or other context.
    pub label: String,
    /// Width divided by height. Used in the mixed-aspect arrangement.
    pub aspect_ratio: f32,
    pub content: View<M>,
    pub on_activate: M,
    pub disabled: bool,
}

impl Theme {
    /// Build a carousel using its current viewport geometry and state.
    ///
    /// Return the input messages to [CarouselState::apply] and call
    /// [CarouselState::advance] on application frames. Keep the same state
    /// and item IDs across rebuilds. Keyboard navigation reveals the item
    /// without activating it. Use passive item content and a localized
    /// label. In a vertically scrolling page, give horizontal carousels an
    /// external Show All action that goes to the full vertical list.
    ///
    /// [Material 3 carousels](https://m3.material.io/components/carousel/overview)
    pub fn carousel<M>(
        &self,
        options: CarouselOptions<'_>,
        layout: &CarouselLayout,
        state: &CarouselState,
        items: Vec<CarouselItem<M>>,
        on_input: impl Fn(CarouselInput) -> M + 'static,
    ) -> View<M> {
        let (width, height) = layout.viewport();
        let viewport = Style::column()
            .width(Length::Px(width))
            .height(Length::Px(height))
            .clip(true);
        let view = View::new(
            Node::new(Role::Group, options.name)
                .id(options.id)
                .styled(viewport),
        );
        let slots = layout.slots(state.offset());
        let keys: Rc<Vec<_>> = Rc::new(
            items
                .iter()
                .take(layout.item_count())
                .map(|item| (!item.disabled).then(|| item.id.clone()))
                .collect(),
        );
        let landing = state
            .active_index(layout)
            .into_iter()
            .chain(0..keys.len())
            .find_map(|index| {
                slots
                    .get(index)
                    .filter(|slot| eligible(**slot))
                    .and_then(|_| keys.get(index)?.as_deref())
            });
        let callback = Rc::new(on_input);
        let on_key = callback.clone();
        let orientation = layout.orientation;
        let sign = layout.physical_sign();
        let page = layout.page_size();
        let children = items
            .into_iter()
            .zip(slots)
            .enumerate()
            .map(|(index, (item, slot))| {
                let enabled = eligible(slot);
                let card = self.carousel_item(item, slot, layout.kind);
                if enabled {
                    let callback = on_key.clone();
                    let keys = keys.clone();
                    card.on_focus_navigation(move |key| {
                        let target = navigation_target(key, index, &keys, orientation, sign, page)?;
                        Some(FocusNavigation {
                            target: keys[target].as_ref()?.clone(),
                            message: callback(CarouselInput::Focus(target)),
                        })
                    })
                } else {
                    card.without_interaction(landing)
                }
            });
        let view = view.children(children);
        if layout.item_count() < 2 {
            return view;
        }
        match layout.orientation {
            CarouselOrientation::Horizontal => {
                let wheel = callback.clone();
                view.on_horizontal_drag(state.horizontal_session(), move |drag| {
                    callback(CarouselInput::HorizontalDrag(drag))
                })
                .on_horizontal_scroll(move |scroll, _focus| {
                    Some(HorizontalScrollResponse {
                        offset: None,
                        message: wheel(CarouselInput::Wheel(scroll.delta)),
                    })
                })
            }
            CarouselOrientation::Vertical => {
                let wheel = callback.clone();
                view.on_vertical_drag(state.vertical_session(), move |drag| {
                    callback(CarouselInput::VerticalDrag(drag))
                })
                .on_vertical_scroll(move |scroll| {
                    Some(VerticalScrollResponse {
                        consumed: scroll.delta,
                        message: wheel(CarouselInput::Wheel(scroll.delta)),
                    })
                })
            }
        }
    }

    fn carousel_item<M>(
        &self,
        item: CarouselItem<M>,
        slot: CarouselSlot,
        kind: CarouselKind,
    ) -> View<M> {
        let visible = slot.bounds.width > 0. && slot.bounds.height > 0.;
        let bounds = if visible {
            slot.bounds
        } else {
            Rect::default()
        };
        let mut style = rect_style(bounds)
            .background(self.colors.surface_container)
            .state_layer(self.colors.on_surface);
        if kind != CarouselKind::FullScreen {
            style = style.corners(shape::EXTRA_LARGE);
        }
        let card = View::new(
            Node::new(Role::Button, item.label)
                .id(item.id)
                .disabled(item.disabled)
                .styled(style)
                .focus_color(self.colors.primary),
        );
        let card = if visible {
            let relative = Rect::new(
                slot.artwork.x - bounds.x,
                slot.artwork.y - bounds.y,
                slot.artwork.width,
                slot.artwork.height,
            );
            let art = View::new(
                Node::new(Role::Group, "").styled(rect_style(relative).align(Align::Stretch)),
            )
            .children([item.content])
            .without_interaction(None);
            let mask = View::new(
                Node::new(Role::Group, "").styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.))
                        .clip(true),
                ),
            )
            .children([art]);
            card.children([mask])
        } else {
            card
        };
        if item.disabled {
            card
        } else {
            card.on_press(item.on_activate)
                .accessible_activation()
                .protect_composition_on_press()
                .cursor(CursorStyle::Pointer)
        }
    }
}

fn rect_style(rect: Rect) -> Style {
    Style::column()
        .width(Length::Px(rect.width))
        .height(Length::Px(rect.height))
        .absolute(Insets {
            left: Length::Px(rect.x),
            top: Length::Px(rect.y),
            ..Insets::default()
        })
}

fn eligible(slot: CarouselSlot) -> bool {
    let bounds = slot.bounds;
    bounds.width > 0.
        && bounds.height > 0.
        && bounds.width + 0.01 >= MIN_ACTION_SIZE.min(slot.artwork.width)
        && bounds.height + 0.01 >= MIN_ACTION_SIZE.min(slot.artwork.height)
}

fn navigation_target(
    key: EditKey,
    index: usize,
    keys: &[Option<String>],
    orientation: CarouselOrientation,
    sign: f32,
    page: usize,
) -> Option<usize> {
    let last = keys.len().checked_sub(1)?;
    let forward = match (key, orientation) {
        (EditKey::Right, CarouselOrientation::Horizontal) => sign > 0.,
        (EditKey::Left, CarouselOrientation::Horizontal) => sign < 0.,
        (EditKey::Down, CarouselOrientation::Vertical) | (EditKey::PageDown, _) => true,
        (EditKey::Up, CarouselOrientation::Vertical) | (EditKey::PageUp, _) => false,
        (EditKey::Home, _) => return keys.iter().position(Option::is_some),
        (EditKey::End, _) => return keys.iter().rposition(Option::is_some),
        _ => return None,
    };
    let step = if matches!(key, EditKey::PageDown | EditKey::PageUp) {
        page
    } else {
        1
    };
    let proposed = if forward {
        index.saturating_add(step).min(last)
    } else {
        index.saturating_sub(step)
    };
    if forward {
        (proposed..=last)
            .find(|&index| keys[index].is_some())
            .or_else(|| {
                (index + 1..proposed)
                    .rev()
                    .find(|&index| keys[index].is_some())
            })
    } else {
        (0..=proposed)
            .rev()
            .find(|&index| keys[index].is_some())
            .or_else(|| (proposed + 1..index).find(|&index| keys[index].is_some()))
    }
}
