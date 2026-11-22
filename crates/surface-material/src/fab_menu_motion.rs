//! Measured anchors and retained motion for the FAB Menu.

use surface::{ComputedDom, MicroDom, Rect};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, FramePress, FramePressPhase};

use crate::fab_motion::{ExpressiveFabMotion, Interaction, Spring};

const MAX_ITEMS: usize = 6;
const SPATIAL: (f64, f64) = (800., 0.6);
const EFFECTS: (f64, f64) = (3800., 1.);
const STAGGER: (f64, f64) = (800., 1.);

/// The desired menu state for the next frame.
pub struct FabMenuMotionTarget<'a> {
    /// Stable ID of the original, unmoving FAB allocation.
    pub id: &'a str,
    pub open: bool,
    /// Action IDs in the authored order, from top to bottom.
    pub item_ids: &'a [&'a str],
}

/// The presented width and opacity of a floating action, each from 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FabMenuItemMotion {
    pub width: f32,
    pub opacity: f32,
}

/// Retain one instance next to the caller's controlled open state.
pub struct FabMenuMotion {
    open: bool,
    last_time: Option<f64>,
    morph: Spring,
    count: Option<Spring>,
    items: Vec<Item>,
    anchor: Option<Rect>,
    menu_height: Option<f32>,
    trigger: ExpressiveFabMotion,
}

struct Item {
    id: String,
    width: Spring,
    opacity: Spring,
    size: Option<(f32, f32)>,
    interaction: Interaction,
}

impl Default for FabMenuMotion {
    fn default() -> Self {
        Self::new(false)
    }
}

impl FabMenuMotion {
    /// Initialize at the caller's current open or closed endpoint.
    pub const fn new(open: bool) -> Self {
        Self {
            open,
            last_time: None,
            morph: Spring::new(endpoint(open)),
            count: None,
            items: Vec::new(),
            anchor: None,
            menu_height: None,
            trigger: ExpressiveFabMotion::new(),
        }
    }

    /// Interpolation from the original square (0) to the close face (1).
    pub fn progress(&self) -> f32 {
        unit(self.morph.position)
    }

    /// The current stagger value in units of items, from zero to the item count.
    pub fn visible_items(&self) -> f32 {
        self.count.map_or(0., |count| {
            count.position.clamp(0., self.items.len() as f64) as f32
        })
    }

    /// The presented action at the given authored index.
    pub fn item(&self, index: usize) -> FabMenuItemMotion {
        self.items
            .get(index)
            .map_or_else(|| item_endpoint(self.open), Item::presentation)
    }

    /// Whether any action still has paint while the caller closes the menu.
    /// The caller removes hit and accessibility targets as soon as it closes.
    pub fn items_present(&self) -> bool {
        self.items.iter().any(|item| {
            let pose = item.presentation();
            pose.width > 0. && pose.opacity > 0.
        })
    }

    /// The measured original slot in logical window coordinates.
    pub fn anchor(&self) -> Option<Rect> {
        self.anchor
    }

    /// Press, hover and focus motion for the same stable FAB trigger.
    pub fn trigger_motion(&self) -> &ExpressiveFabMotion {
        &self.trigger
    }

    pub(super) fn menu_height(&self) -> Option<f32> {
        self.menu_height
    }

    pub(super) fn item_size(&self, id: &str) -> Option<(f32, f32)> {
        self.items.iter().find(|item| item.id == id)?.size
    }

    pub(super) fn interaction(&self, id: &str) -> Interaction {
        self.items
            .iter()
            .find(|item| item.id == id)
            .map_or_else(Interaction::default, |item| item.interaction)
    }

    /// Advance from `Application::animation_frame_with_press_detail`.
    /// The original FAB allocation must keep the stable ID in `target`.
    pub fn advance(
        &mut self,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
        target: FabMenuMotionTarget<'_>,
    ) -> AnimationUpdate {
        let previous = self.presentation();
        let now = frame
            .now_ms
            .filter(|clock| clock.is_finite() && !frame.reduced_motion)
            .map(|clock| clock.max(self.last_time.unwrap_or(clock)));
        if let Some(now) = now {
            let elapsed = ((now - self.last_time.unwrap_or(now)) * 0.001).clamp(0., 2.);
            self.advance_retained(elapsed);
        }

        let item_ids = &target.item_ids[..target.item_ids.len().min(MAX_ITEMS)];
        let items_changed = self.compare_items(item_ids);
        if self.open != target.open || items_changed {
            // Keep the visible number on reversal. Restart the stagger
            // velocity so a close cannot schedule new item entrances.
            let count = self.visible_items();
            if let Some(gate) = &mut self.count {
                *gate = Spring::new(f64::from(count));
            }
        }
        self.open = target.open;
        if now.is_none() {
            self.snap();
        }
        self.last_time = now;

        let measurements_changed = self.measure(frame, target.id);
        let mut interaction_changed = false;
        for item in &mut self.items {
            let active = self.open;
            let next = Interaction {
                hovered: active && hovered == Some(item.id.as_str()),
                focused: active && input.focused == Some(item.id.as_str()),
                pressed: active
                    && press.is_some_and(|event| {
                        event.phase == FramePressPhase::Held && event.id == item.id
                    }),
            };
            interaction_changed |= next != item.interaction;
            item.interaction = next;
        }
        let trigger = self.trigger.advance(
            &format!("{}-trigger", target.id),
            frame,
            input,
            press,
            hovered,
            true,
        );
        let changed = self.presentation() != previous
            || items_changed
            || measurements_changed
            || interaction_changed
            || trigger.rebuild;
        AnimationUpdate {
            rebuild: changed,
            request_next: changed
                || trigger.request_next
                || self.in_motion()
                || (frame.layout.is_none() && frame.dom.by_id(target.id).is_some()),
        }
    }

    fn presentation(&self) -> (f32, f32, [FabMenuItemMotion; MAX_ITEMS]) {
        (
            self.progress(),
            self.visible_items(),
            std::array::from_fn(|index| self.item(index)),
        )
    }

    fn compare_items(&mut self, ids: &[&str]) -> bool {
        if self
            .items
            .iter()
            .map(|item| item.id.as_str())
            .eq(ids.iter().copied())
        {
            return false;
        }
        let mut prior = std::mem::take(&mut self.items);
        let initially_open = self.count.is_none() && self.open;
        self.items = ids
            .iter()
            .map(|&id| {
                prior.iter().position(|item| item.id == id).map_or_else(
                    || Item::new(id, initially_open),
                    |index| prior.swap_remove(index),
                )
            })
            .collect();
        if self.count.is_none() {
            let count = if initially_open {
                self.items.len() as f64
            } else {
                0.
            };
            self.count = Some(Spring::new(count));
        }
        true
    }

    fn advance_retained(&mut self, mut elapsed: f64) {
        spring_step(&mut self.morph, endpoint(self.open), elapsed, SPATIAL);
        let item_count = self.items.len();
        let Some(count) = &mut self.count else {
            return;
        };
        let goal = if self.open { item_count as f64 } else { 0. };
        while elapsed > 0. {
            // Resolve stagger gates even across a skipped frame. Each item
            // then progresses on its own width and opacity springs.
            let step = if count.resting_at(goal) {
                elapsed
            } else {
                elapsed.min(0.008)
            };
            let shown = count.position.round() as usize;
            for (index, item) in self.items.iter_mut().enumerate() {
                let goal = endpoint(item_count - index <= shown);
                spring_step(&mut item.width, goal, step, SPATIAL);
                spring_step(&mut item.opacity, goal, step, EFFECTS);
            }
            spring_step(count, goal, step, STAGGER);
            elapsed -= step;
        }
    }

    fn snap(&mut self) {
        let goal = endpoint(self.open);
        self.morph = Spring::new(goal);
        if self.count.is_some() {
            self.count = Some(Spring::new(goal * self.items.len() as f64));
        }
        for item in &mut self.items {
            item.width = Spring::new(goal);
            item.opacity = Spring::new(goal);
        }
    }

    fn in_motion(&self) -> bool {
        let goal = endpoint(self.open);
        !self.morph.resting_at(goal)
            || self
                .count
                .is_some_and(|count| !count.resting_at(goal * self.items.len() as f64))
            || self
                .items
                .iter()
                .any(|item| !item.width.resting_at(goal) || !item.opacity.resting_at(goal))
    }

    fn measure(&mut self, frame: &AnimationFrame<'_>, id: &str) -> bool {
        let Some(anchor) = frame.dom.by_id(id) else {
            return self.anchor.take().is_some() | self.menu_height.take().is_some();
        };
        let Some(layout) = frame.layout else {
            return false;
        };
        let bounds = layout.nodes.get(anchor.index()).and_then(|geometry| {
            let visible = geometry
                .bounds
                .intersect(geometry.clip)
                .intersect(layout.viewport);
            (positive(visible.width) && positive(visible.height)).then_some(geometry.bounds)
        });
        let mut changed = replace(&mut self.anchor, bounds);

        if let Some(height) = node_bounds(frame.dom, layout, &format!("{id}-menu-content"))
            .map(|bounds| bounds.height)
            .filter(|&height| positive(height))
        {
            changed |= replace(&mut self.menu_height, Some(height));
        }
        for item in &mut self.items {
            let measured = node_bounds(frame.dom, layout, &format!("{}-measure", item.id))
                .zip(node_bounds(
                    frame.dom,
                    layout,
                    &format!("{}-wrapped", item.id),
                ))
                .map(|(natural, wrapped)| (natural.width, wrapped.height))
                .filter(|&(width, height)| positive(width) && positive(height));
            changed |= replace(&mut item.size, measured);
        }
        changed
    }
}

impl Item {
    fn new(id: &str, open: bool) -> Self {
        let initial = endpoint(open);
        Self {
            id: id.to_owned(),
            width: Spring::new(initial),
            opacity: Spring::new(initial),
            size: None,
            interaction: Interaction::default(),
        }
    }

    fn presentation(&self) -> FabMenuItemMotion {
        FabMenuItemMotion {
            width: unit(self.width.position),
            opacity: unit(self.opacity.position),
        }
    }
}

fn item_endpoint(open: bool) -> FabMenuItemMotion {
    let value = endpoint(open) as f32;
    FabMenuItemMotion {
        width: value,
        opacity: value,
    }
}

fn node_bounds(dom: &MicroDom, layout: &ComputedDom, id: &str) -> Option<Rect> {
    Some(layout.nodes.get(dom.by_id(id)?.index())?.bounds)
}

fn spring_step(spring: &mut Spring, target: f64, seconds: f64, (stiffness, damping): (f64, f64)) {
    spring.advance(target, seconds, stiffness, damping);
}

fn replace<T: PartialEq>(slot: &mut T, next: T) -> bool {
    if *slot == next {
        false
    } else {
        *slot = next;
        true
    }
}

const fn endpoint(open: bool) -> f64 {
    if open {
        1.
    } else {
        0.
    }
}

fn unit(value: f64) -> f32 {
    value.clamp(0., 1.) as f32
}

fn positive(value: f32) -> bool {
    value.is_finite() && value > 0.
}
