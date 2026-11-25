use std::collections::BTreeMap;

use surface::{MicroDom, Node, NodeId, RangeValue, Rect, Role};

use crate::{AppError, Application, Binding, Runtime, SliderInputOptions, SliderScale};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct HorizontalPair {
    pub(crate) rail: String,
    pub(crate) lower: String,
    pub(crate) upper: String,
}

pub(crate) struct PairPress {
    pub(crate) owner: NodeId,
    pub(crate) thumb: NodeId,
    pub(crate) rail: Rect,
    pub(crate) ambiguous: bool,
}

pub(crate) const PAIR_SLOP: f32 = 4.;

impl<A: Application> Runtime<A> {
    pub(crate) fn horizontal_pair_at(
        &self,
        hit: Option<NodeId>,
        x: f32,
        y: f32,
    ) -> Option<PairPress> {
        if !self.pointer_geometry_current || ![x, y].into_iter().all(f32::is_finite) {
            return None;
        }
        let layout = self.computed.as_ref()?;
        let candidate = |owner: NodeId| {
            let binding = self.bindings.get(&owner)?;
            let spec = binding.slider_pair.as_ref()?;
            let owner_box = layout.nodes.get(owner.index())?;
            let visible = owner_box
                .bounds
                .intersect(owner_box.clip)
                .intersect(layout.viewport);
            if !self.in_scope(owner)
                || self.dom.get(owner).ok()?.is_disabled()
                || visible.width <= 0.
                || visible.height <= 0.
                || !visible.contains(x, y)
            {
                return None;
            }
            let lower = self.dom.by_id(&spec.lower)?;
            let upper = self.dom.by_id(&spec.upper)?;
            if hit.is_some_and(|id| id != lower && id != upper) {
                return None;
            }
            let rail_id = self.dom.by_id(&spec.rail)?;
            let rail = layout.nodes.get(rail_id.index())?.bounds;
            if rail.width <= 0. || rail.height <= 0. {
                return None;
            }
            let input = self.bindings.get(&lower)?.slider_input?;
            let center = |id: NodeId| {
                let node = self.dom.get(id).ok()?;
                let bounds = layout.nodes.get(id.index())?;
                let visible = bounds
                    .bounds
                    .intersect(bounds.clip)
                    .intersect(layout.viewport);
                if !self.in_scope(id)
                    || !node.is_focusable()
                    || visible.width <= 0.
                    || visible.height <= 0.
                {
                    return None;
                }
                let fraction = input.value_fraction(node.range_value()?)? as f32;
                Some(
                    rail.x
                        + rail.width
                            * if input.reverse_horizontal {
                                1. - fraction
                            } else {
                                fraction
                            },
                )
            };
            let (thumb, ambiguous) = match (center(lower), center(upper)) {
                (Some(l), Some(u)) if l == u && (x - l).abs() > PAIR_SLOP => {
                    let toward_maximum = (x > l) != input.reverse_horizontal;
                    (if toward_maximum { upper } else { lower }, false)
                }
                (Some(l), Some(u)) => {
                    let left = (x - l).abs();
                    let right = (x - u).abs();
                    let chosen = if left < right
                        || (left == right && self.focused.as_deref() != Some(spec.upper.as_str()))
                    {
                        lower
                    } else {
                        upper
                    };
                    (chosen, left <= PAIR_SLOP && right <= PAIR_SLOP)
                }
                (Some(_), None) => (lower, false),
                (None, Some(_)) => (upper, false),
                (None, None) => return None,
            };
            Some(PairPress {
                owner,
                thumb,
                rail,
                ambiguous,
            })
        };
        if let Some(hit) = hit {
            // A regular control painted above a pair owns its own hit.
            return candidate(self.dom.get(hit).ok()?.parent()?);
        }
        layout
            .paint_order
            .iter()
            .rev()
            .find_map(|&owner| candidate(owner))
    }
}

/// Derive both limits from one validated pair on each controlled rebuild.
pub(crate) fn bind_limits<M>(
    dom: &MicroDom,
    bindings: &mut BTreeMap<NodeId, Binding<M>>,
) -> Result<(), AppError> {
    let mut limits = Vec::new();
    for (&owner, binding) in bindings.iter() {
        let Some(pair) = &binding.slider_pair else {
            continue;
        };
        let node = dom.get(owner)?;
        let invalid = || AppError::InvalidRange(node.stable_id().unwrap_or(node.name()).to_owned());
        let child = |id: &str| -> Option<(NodeId, &Node)> {
            let id = dom.by_id(id)?;
            let node = dom.get(id).ok()?;
            (node.parent() == Some(owner)).then_some((id, node))
        };
        let (rail_id, rail) = child(&pair.rail).ok_or_else(invalid)?;
        let (lower_id, lower) = child(&pair.lower).ok_or_else(invalid)?;
        let (upper_id, upper) = child(&pair.upper).ok_or_else(invalid)?;
        if node.role() != Role::Group
            || node.name().trim().is_empty()
            || node.stable_id().is_none_or(|id| id.trim().is_empty())
            || binding.roving.is_some()
            || !is_passive_rail(dom, rail_id, rail, &bindings[&rail_id])
            || lower_id >= upper_id
            || !is_thumb(lower, node.is_disabled())
            || !is_thumb(upper, node.is_disabled())
            || lower.name().trim() == upper.name().trim()
        {
            return Err(invalid());
        }
        let lower_range = lower
            .range_value()
            .filter(|r| r.is_valid())
            .ok_or_else(invalid)?;
        let upper_range = upper
            .range_value()
            .filter(|r| r.is_valid())
            .ok_or_else(invalid)?;
        let input = bindings[&lower_id].slider_input.ok_or_else(invalid)?;
        if bindings[&upper_id].slider_input != Some(input)
            || lower_range.min != upper_range.min
            || lower_range.max != upper_range.max
            || lower_range.step != upper_range.step
            || lower_range.value > upper_range.value
            || !has_valid_grid(input, lower_range)
            || input.resolve_value(lower_range, lower_range.value) != Some(lower_range.value)
            || input.resolve_value(upper_range, upper_range.value) != Some(upper_range.value)
        {
            return Err(invalid());
        }
        limits.extend([
            (lower_id, (lower_range.min, upper_range.value)),
            (upper_id, (lower_range.value, upper_range.max)),
        ]);
    }
    for (id, range) in limits {
        bindings
            .get_mut(&id)
            .expect("pair child was validated")
            .slider_limits = Some(range);
    }
    Ok(())
}

fn is_passive_rail<M>(dom: &MicroDom, id: NodeId, node: &Node, binding: &Binding<M>) -> bool {
    node.role() == Role::Group
        && !node.clone().disabled(false).is_focusable()
        && dom.children(id).next().is_none()
        && binding.activate.is_none()
        && binding.value.is_none()
        && binding.context.is_none()
        && binding.dismiss.is_none()
        && binding.vertical_scroll.is_none()
        && binding.horizontal_scroll.is_none()
        && binding.drag_bindings.is_empty()
}

fn is_thumb(node: &Node, disabled: bool) -> bool {
    node.role() == Role::Slider
        && !node.is_vertical_slider()
        && !node.name().trim().is_empty()
        && node.is_disabled() == disabled
        && (disabled || node.is_tab_stop())
}

fn has_valid_grid(input: SliderInputOptions, range: RangeValue) -> bool {
    let SliderScale::Discrete(intervals) = input.scale else {
        return true;
    };
    let count = f64::from(intervals.get());
    let span = range.max - range.min;
    if range.step != span / count {
        return false;
    }
    // At very small magnitudes relative to the endpoints, test that two
    // neighboring stops still map to different representable values.
    let edge_resolution = (range.min.next_up() - range.min).max(range.max - range.max.next_down());
    if range.step > edge_resolution {
        return true;
    }
    let mut previous = range.min;
    for index in 1..=intervals.get() {
        let fraction = f64::from(index) / count;
        let value = if index == intervals.get() {
            range.max
        } else {
            range.min + span * fraction
        };
        if value <= previous
            || input.value_fraction(RangeValue { value, ..range }) != Some(fraction)
        {
            return false;
        }
        previous = value;
    }
    true
}
