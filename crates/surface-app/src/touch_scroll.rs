use crate::{captured_drag::Axis, drag_motion::SLOP, AppError, Application, Runtime};
use surface::{ComputedDom, NodeId, Role};

impl Axis {
    fn value(self, pair: (f32, f32)) -> f32 {
        self.component(pair.0, pair.1)
    }

    fn offset(self, pair: (f32, f32), value: f32) -> (f32, f32) {
        match self {
            Self::Horizontal => (value, pair.1),
            Self::Vertical => (pair.0, value),
        }
    }
}

struct Target {
    key: String,
    role: Role,
    limit: f32,
    initial: f32,
    presented: f32,
}

struct EditorTarget {
    key: String,
    initial: (f32, bool),
    presented: (f32, bool),
}

enum Owner {
    Horizontal(Target),
    Multiline(EditorTarget),
}

enum Phase {
    Pending,
    Panning(Axis),
    Retired,
}

/// The ordinary pointer route owns a tap until this contact takes an axis.
pub(crate) struct TouchScroll {
    owner: Owner,
    vertical: Option<Target>,
    hit: Option<String>,
    scope: Option<String>,
    phase: Phase,
}

impl TouchScroll {
    fn target(&mut self, axis: Axis) -> Option<&mut Target> {
        match axis {
            Axis::Horizontal => match &mut self.owner {
                Owner::Horizontal(target) => Some(target),
                Owner::Multiline(_) => None,
            },
            Axis::Vertical => self.vertical.as_mut(),
        }
    }

    pub(crate) fn deferred_editor(&self) -> Option<&str> {
        match &self.owner {
            Owner::Multiline(editor) => Some(&editor.key),
            Owner::Horizontal(_) => None,
        }
    }
}

impl<A: Application> Runtime<A> {
    pub(crate) fn touch_scroll_at(&self, point: (f32, f32)) -> Option<TouchScroll> {
        if !self.pointer_geometry_current || self.has_pending_composition() {
            return None;
        }
        let hit = self.raw_hit(point.0, point.1);
        let layout = self.computed.as_ref()?;
        if self.horizontal_pair_at(hit, point.0, point.1).is_some()
            || self.drag_candidate(hit, point.0, point.1).is_some()
            || self.outside_scope(point.0, point.1, hit)
        {
            return None;
        }
        if let Some(id) = self.multiline_touch_at(layout, point) {
            let parent = self.dom.get(id).ok()?.parent();
            let vertical =
                parent.and_then(|parent| self.vertical_touch_target(layout, parent, id, point));
            if vertical.is_none() && self.multiline_scroll_at(layout, point).is_none() {
                return None;
            }
            let key = self.stable(id)?;
            let initial = self.multiline_scroll_position(id)?;
            return Some(TouchScroll {
                owner: Owner::Multiline(EditorTarget {
                    key,
                    initial,
                    presented: initial,
                }),
                vertical,
                hit: None,
                scope: self.drag_scope().map(str::to_owned),
                phase: Phase::Pending,
            });
        }
        let horizontal = |id| {
            if !self.bindings.get(&id)?.horizontal_touch_scroll {
                return None;
            }
            Some((
                id,
                self.touch_scroll_target(layout, id, point, Axis::Horizontal)?,
            ))
        };
        let (owner, horizontal) = if let Some(hit) = hit {
            self.scroll_ancestors_at(layout, hit, point)
                .find_map(horizontal)
        } else {
            layout
                .paint_order
                .iter()
                .rev()
                .copied()
                .find_map(horizontal)
        }?;
        if hit.is_some_and(|id| {
            self.dom
                .get(id)
                .is_ok_and(|node| matches!(node.role(), Role::TextInput | Role::Slider))
        }) {
            return None;
        }
        Some(TouchScroll {
            owner: Owner::Horizontal(horizontal),
            vertical: self.vertical_touch_target(layout, hit.unwrap_or(owner), owner, point),
            hit: hit.and_then(|id| self.stable(id)),
            scope: self.drag_scope().map(str::to_owned),
            phase: Phase::Pending,
        })
    }

    fn vertical_touch_target(
        &self,
        layout: &ComputedDom,
        hit: NodeId,
        owner: NodeId,
        point: (f32, f32),
    ) -> Option<Target> {
        // The nearest custom vertical owner reserves this route even when
        // its content fits. A touch pan neither runs a wheel handler nor
        // skips it to move a more distant ancestor.
        self.vertical_wheel_target(layout, hit, point)
            .filter(|&id| id != owner && self.descendant(owner, id))
            .filter(|&id| !self.has_vertical_scroll_owner(id))
            .and_then(|id| self.touch_scroll_target(layout, id, point, Axis::Vertical))
    }

    fn touch_scroll_target(
        &self,
        layout: &ComputedDom,
        id: NodeId,
        point: (f32, f32),
        axis: Axis,
    ) -> Option<Target> {
        if !self.scroll_region_at(layout, id, point) {
            return None;
        }
        let node = self.dom.get(id).ok()?;
        let key = node.stable_id()?;
        let metrics = self.scroll_metrics(key)?;
        let limit = axis.value(metrics.content) - axis.value(metrics.viewport);
        if node.is_disabled() || !limit.is_finite() || limit <= 0. {
            return None;
        }
        let initial = axis.value((node.style().scroll_x, node.style().scroll_y));
        Some(Target {
            key: key.to_owned(),
            role: node.role(),
            limit,
            initial,
            presented: initial,
        })
    }

    /// Tell the Web host whether to monitor capture loss for this touch.
    #[doc(hidden)]
    pub fn has_touch_scroll_capture(&self) -> bool {
        self.touch
            .as_ref()
            .is_some_and(|touch| touch.scroll.is_some())
    }

    /// Retire just an opted-in touch contact after host capture loss.
    #[doc(hidden)]
    pub fn cancel_touch_scroll_capture(&mut self) -> Result<bool, AppError> {
        if self.has_touch_scroll_capture() {
            self.cancel_pointer()
        } else {
            Ok(false)
        }
    }

    fn current_touch_scroll_owner(&self, target: &Target) -> Option<NodeId> {
        let id = self.dom.by_id(&target.key)?;
        self.dom.get(id).ok().filter(|node| {
            node.style().clip
                && node.role() == target.role
                && !node.is_disabled()
                && self.in_scope(id)
        })?;
        Some(id)
    }

    fn touch_owner(&self, owner: &Owner) -> Option<NodeId> {
        match owner {
            Owner::Horizontal(target) => {
                let id = self.current_touch_scroll_owner(target)?;
                self.bindings
                    .get(&id)?
                    .horizontal_touch_scroll
                    .then_some(id)
            }
            Owner::Multiline(editor) => {
                let id = self.dom.by_id(&editor.key)?;
                self.multiline_scroll_position(id)?;
                Some(id)
            }
        }
    }

    fn touch_scroll_in_scope(&self, scroll: &TouchScroll) -> bool {
        scroll.scope.as_deref() == self.drag_scope() && self.touch_owner(&scroll.owner).is_some()
    }

    fn valid_vertical_touch_owner(&self, scroll: &TouchScroll) -> bool {
        let Some(ancestor) = scroll
            .vertical
            .as_ref()
            .and_then(|target| self.current_touch_scroll_owner(target))
        else {
            return false;
        };
        let Some(owner) = self.touch_owner(&scroll.owner) else {
            return false;
        };
        let hit = match &scroll.hit {
            Some(key) => self
                .dom
                .by_id(key)
                .filter(|&hit| self.descendant(hit, owner)),
            None => Some(owner),
        };
        let Some(hit) = hit else {
            return false;
        };
        if !self.descendant(owner, ancestor) || self.has_vertical_scroll_owner(ancestor) {
            return false;
        }
        // A rebuild can insert a new scroll owner before the old target.
        std::iter::successors(Some(hit), |id| self.dom.get(*id).ok()?.parent())
            .take_while(|id| *id != ancestor)
            .all(|id| !self.has_vertical_scroll_owner(id))
    }

    pub(crate) fn reconcile_touch_scroll(&mut self) -> Result<(), AppError> {
        if self
            .touch
            .as_ref()
            .and_then(|touch| touch.scroll.as_ref())
            .is_some_and(|scroll| !self.touch_scroll_in_scope(scroll))
        {
            self.cancel_pointer()?;
        }
        Ok(())
    }

    pub(crate) fn cancel_touch_scroll(&mut self) -> Result<(), AppError> {
        let Some(mut scroll) = self.touch.as_mut().and_then(|touch| touch.scroll.take()) else {
            return Ok(());
        };
        if !self.touch_scroll_in_scope(&scroll) {
            return Ok(());
        }
        if let Phase::Panning(axis) = scroll.phase {
            let valid_vertical = self.valid_vertical_touch_owner(&scroll);
            if let Owner::Multiline(editor) = &scroll.owner {
                if let Some(id) = self.dom.by_id(&editor.key) {
                    if self.restore_multiline_scroll(id, editor.presented, editor.initial) {
                        self.computed = None;
                        self.ime_area = None;
                    }
                }
            }
            if matches!(axis, Axis::Vertical) && !valid_vertical {
                return Ok(());
            }
            if let Some(target) = scroll.target(axis) {
                self.set_touch_scroll_offset(target, axis, target.initial)?;
            }
        }
        Ok(())
    }

    /// Return None for a pending tap, or Some after taking the contact.
    /// A direct release can be the first sample to cross the threshold.
    pub(crate) fn advance_touch_scroll(
        &mut self,
        scroll: &mut TouchScroll,
        delta: (f32, f32),
    ) -> Result<Option<bool>, AppError> {
        let multiline = matches!(scroll.owner, Owner::Multiline(_));
        if !self.touch_scroll_in_scope(scroll)
            || (matches!(scroll.phase, Phase::Panning(Axis::Vertical))
                && !multiline
                && !self.valid_vertical_touch_owner(scroll))
            || !delta.0.is_finite()
            || !delta.1.is_finite()
        {
            self.clear_modal_pointer();
            self.cancel_press_ripple();
            scroll.phase = Phase::Retired;
            return Ok(Some(true));
        }
        let (axis, claimed_now) = match scroll.phase {
            Phase::Pending => {
                if delta.0.abs().max(delta.1.abs()) < SLOP {
                    return Ok(None);
                }
                self.clear_modal_pointer();
                self.cancel_press_ripple();
                let axis = match Axis::dominant(delta.0, delta.1) {
                    Some(Axis::Horizontal) if !multiline => Axis::Horizontal,
                    Some(Axis::Vertical)
                        if multiline || self.valid_vertical_touch_owner(scroll) =>
                    {
                        Axis::Vertical
                    }
                    _ => {
                        scroll.phase = Phase::Retired;
                        return Ok(Some(true));
                    }
                };
                scroll.phase = Phase::Panning(axis);
                (axis, true)
            }
            Phase::Panning(axis) => (axis, false),
            Phase::Retired => return Ok(Some(false)),
        };
        let changed = if multiline {
            self.pan_multiline(scroll, delta.1)?
        } else {
            let target = scroll.target(axis).expect("panning has an owner");
            let goal = (target.initial - axis.value(delta)).clamp(0., target.limit);
            self.set_touch_scroll_offset(target, axis, goal)?
        };
        Ok(Some(claimed_now || changed))
    }

    fn pan_multiline(&mut self, scroll: &mut TouchScroll, delta_y: f32) -> Result<bool, AppError> {
        let valid_vertical = self.valid_vertical_touch_owner(scroll);
        let Owner::Multiline(editor) = &mut scroll.owner else {
            unreachable!("a multiline pan has an editor");
        };
        let Some(id) = self.dom.by_id(&editor.key) else {
            return Ok(false);
        };
        if self.multiline_scroll_position(id) != Some(editor.presented) {
            return Ok(false);
        }
        let delta = (editor.initial.0 - editor.presented.0) - delta_y;
        let residual = self.scroll_multiline(id, delta);
        let mut changed = false;
        if let Some(position) = self.multiline_scroll_position(id) {
            changed = position != editor.presented;
            editor.presented = position;
        }
        if changed {
            self.computed = None;
            self.ime_area = None;
        }
        // Both previews derive from the contact origin. If the finger
        // reverses, it also undoes any residual given to the parent.
        if let Some(target) = scroll.vertical.as_mut().filter(|_| valid_vertical) {
            let goal = (target.initial + residual).clamp(0., target.limit);
            changed |= self.set_touch_scroll_offset(target, Axis::Vertical, goal)?;
        }
        Ok(changed)
    }

    fn set_touch_scroll_offset(
        &mut self,
        target: &mut Target,
        axis: Axis,
        goal: f32,
    ) -> Result<bool, AppError> {
        let Some(id) = self.current_touch_scroll_owner(target) else {
            return Ok(false);
        };
        let style = self.dom.get(id)?.style();
        let current = (style.scroll_x, style.scroll_y);
        // A queued scroll or different input can take ownership of the axis.
        // Leave that newer offset intact, even on cancellation.
        if axis.value(current) != target.presented || target.presented == goal {
            return Ok(false);
        }
        self.save_scroll_offsets(id, axis.offset(current, goal))?;
        target.presented = goal;
        self.computed = None;
        self.ime_area = None;
        Ok(true)
    }
}
