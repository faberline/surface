//! Width allocation and adjacent-item movement for Material button groups.

use crate::{
    button_group::{ButtonGroupItem, ButtonGroupOptions, ButtonGroupWidth},
    ButtonGroupKind, ButtonGroupMotion,
};

pub(super) const TOUCH: f32 = 48.;
const PRESS_GROWTH: f32 = 0.15;

#[derive(Clone, Copy)]
pub(super) struct Slot {
    pub hit: f32,
    pub margin: f32,
    pub gap_after: f32,
    pub min_label: f32,
    pub height: f32,
    min: f32,
    max: f32,
    flex: f32,
}

impl Slot {
    pub fn face(self) -> f32 {
        self.hit - 2. * self.margin
    }
}

pub(super) struct GroupLayout {
    pub slots: Vec<Slot>,
    pub visible_items: usize,
    pub overflow: bool,
    pub viewport_width: f32,
    pub content_width: f32,
    pub scroll: bool,
}

impl GroupLayout {
    pub fn new<M>(
        options: ButtonGroupOptions<'_>,
        items: &[ButtonGroupItem<'_, M>],
        motion: &ButtonGroupMotion,
        can_overflow: bool,
        overflow_id: &str,
    ) -> Self {
        let tokens = options.size.tokens();
        let connected = options.kind == ButtonGroupKind::Connected;
        let gap = if connected { 2. } else { tokens.standard_gap };
        let limit = if options.available_width.is_finite() && options.available_width > 0. {
            options.available_width
        } else {
            TOUCH
        };
        let mut slots: Vec<_> = items
            .iter()
            .map(|item| {
                let icon = item.icon.is_some();
                let icon_only = item.icon_only && icon;
                let mut min_label = 0.;
                let accessory = if icon && !icon_only {
                    tokens.text_icon + tokens.icon_gap
                } else {
                    0.
                };
                let intrinsic = if icon_only {
                    tokens.icon_widths[match item.width {
                        ButtonGroupWidth::Narrow => 0,
                        ButtonGroupWidth::Wide => 2,
                        _ => 1,
                    }]
                } else {
                    let label = motion.measured(item.id).unwrap_or_else(|| {
                        item.label
                            .chars()
                            .map(|c| if c.is_ascii() { 0.57 } else { 1. })
                            .sum::<f32>()
                            * tokens.font.font_size
                    });
                    let wide_char = !item.label.is_ascii();
                    min_label = label.min(tokens.font.font_size * if wide_char { 2. } else { 1. });
                    label + 2. * tokens.label_inset + accessory
                };
                let requested = match item.width {
                    ButtonGroupWidth::Fixed(width) if width.is_finite() && width > 0. => width,
                    _ => intrinsic,
                };
                let flex = match item.width {
                    ButtonGroupWidth::Flex(weight) if weight.is_finite() && weight > 0. => weight,
                    ButtonGroupWidth::Intrinsic if connected => 1.,
                    _ => 0.,
                };
                // XS/S standard icon faces can use their spacing for hit area.
                let floor = if connected || !icon_only {
                    TOUCH
                } else {
                    TOUCH - gap
                };
                let min = if icon_only {
                    floor.max(tokens.icon_only + 8.)
                } else {
                    floor.max(min_label + accessory)
                };
                let max = if icon_only {
                    tokens.icon_widths[2].max(min)
                } else {
                    f32::INFINITY
                };
                let mut slot = slot(requested.clamp(min, max), min, flex);
                slot.max = max + 2. * slot.margin;
                slot.min_label = min_label;
                slot.height = if connected {
                    tokens.height.max(motion.tallest_label())
                } else {
                    tokens.height
                };
                slot
            })
            .collect();
        let overflow_face = tokens.icon_widths[1].max(TOUCH - gap);
        let mut trigger = slot(overflow_face, overflow_face, 0.);
        trigger.height = tokens.height;
        let mut visible_items = items.len();
        if can_overflow && !connected && row_width(&slots, gap) > limit {
            // Only trailing commands are eligible. Toggle choices stay in
            // the group so their state is visible and reachable in context.
            while visible_items > 1 && !items[visible_items - 1].toggle {
                visible_items -= 1;
                slots.truncate(visible_items);
                slots.push(trigger);
                if row_width(&slots, gap) <= limit || items[visible_items - 1].toggle {
                    break;
                }
            }
        }
        let overflow = visible_items < items.len();
        assign_gaps(&mut slots, gap);
        let intrinsic = row_width(&slots, gap);
        if intrinsic < limit {
            stretch(&mut slots, limit - intrinsic);
        } else if intrinsic > limit {
            compress(&mut slots, intrinsic - limit);
        }

        if !connected {
            let (active, progress) = motion.presentation();
            let pressed = items[..visible_items]
                .iter()
                .position(|item| Some(item.id) == active && !item.disabled)
                .or_else(|| (overflow && active == Some(overflow_id)).then_some(visible_items));
            if let Some(pressed) = pressed {
                resize_neighbors(&mut slots, pressed, progress);
            }
        }
        let width: f32 = slots.iter().map(|slot| slot.hit + slot.gap_after).sum();
        Self {
            slots,
            visible_items,
            overflow,
            viewport_width: width.min(limit),
            content_width: width,
            scroll: width - limit > 0.01,
        }
    }
}

fn slot(face: f32, min: f32, flex: f32) -> Slot {
    let hit = face.max(TOUCH);
    Slot {
        hit,
        margin: (hit - face) / 2.,
        gap_after: 0.,
        min_label: 0.,
        height: 0.,
        // The hit areas are always disjoint, including during press.
        min: (min + hit - face).max(TOUCH),
        max: f32::INFINITY,
        flex,
    }
}

fn row_width(slots: &[Slot], gap: f32) -> f32 {
    slots.iter().map(|slot| slot.hit).sum::<f32>()
        + slots
            .windows(2)
            .map(|pair| pair_gap(pair[0], pair[1], gap))
            .sum::<f32>()
}

fn pair_gap(first: Slot, second: Slot, gap: f32) -> f32 {
    (gap - first.margin - second.margin).max(0.)
}

fn assign_gaps(slots: &mut [Slot], gap: f32) {
    for index in 1..slots.len() {
        slots[index - 1].gap_after = pair_gap(slots[index - 1], slots[index], gap);
    }
}

fn compress(slots: &mut [Slot], excess: f32) {
    let slack: f32 = slots.iter().map(|slot| slot.hit - slot.min).sum();
    if slack > 0. {
        let fraction = (excess / slack).min(1.);
        for slot in slots {
            slot.hit -= (slot.hit - slot.min) * fraction;
        }
    }
}

fn stretch(slots: &mut [Slot], mut remaining: f32) {
    for _ in 0..slots.len() {
        let weight: f32 = slots
            .iter()
            .filter(|slot| slot.max > slot.hit)
            .map(|slot| slot.flex)
            .sum();
        if weight == 0. || remaining < 0.01 {
            break;
        }
        let mut added = 0.;
        for slot in slots.iter_mut() {
            let growth = (remaining * (slot.flex / weight)).min(slot.max - slot.hit);
            slot.hit += growth;
            added += growth;
        }
        remaining -= added;
    }
}

fn resize_neighbors(slots: &mut [Slot], index: usize, progress: f32) {
    let left = index.checked_sub(1);
    let right = (index + 1 < slots.len()).then_some(index + 1);
    let capacity =
        |neighbor: Option<usize>| neighbor.map_or(0., |i| (slots[i].hit - slots[i].min).max(0.));
    let (left_capacity, right_capacity) = (capacity(left), capacity(right));
    let growth =
        (slots[index].face() * PRESS_GROWTH * progress).min(left_capacity + right_capacity);
    let right_loss = (growth * 0.5)
        .max(growth - left_capacity)
        .min(right_capacity);
    if let Some(left) = left {
        slots[left].hit -= growth - right_loss;
    }
    if let Some(right) = right {
        slots[right].hit -= right_loss;
    }
    slots[index].hit += growth;
}
