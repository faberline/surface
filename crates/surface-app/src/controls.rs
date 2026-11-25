use crate::{
    focus_scroll::can_reveal, grid_navigation, AccessibilityAdjustment, AppError, Application,
    EditKey, ModalDismissal, Runtime,
};
use std::{collections::BTreeSet, rc::Rc};
use surface::{
    scroll_extent, ComputedDom, Direction, GridRole, MicroDom, Node, NodeId, Position, RangeValue,
    Rect, Role,
};

/// Mount or update a named control, then focus it without activating it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusNavigation<M> {
    /// Stable ID of the control to focus after the rebuild.
    pub target: String,
    /// Message that runs before the runtime checks the target.
    pub message: M,
}

pub(crate) type FocusNavigationHandler<M> = Rc<dyn Fn(EditKey, bool) -> Option<FocusNavigation<M>>>;

/// One tab stop per group. Arrows move among its focusable controls.
/// Disabled controls are skipped unless the view opts in to disabled menu focus.
#[derive(Clone, Copy, Debug)]
pub struct Roving {
    pub direction: Direction,
    pub automatic: bool,
}
impl Roving {
    pub const fn horizontal(automatic: bool) -> Self {
        Self {
            direction: Direction::Row,
            automatic,
        }
    }
    pub const fn vertical(automatic: bool) -> Self {
        Self {
            direction: Direction::Column,
            automatic,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct RovingGroup {
    pub(crate) options: Roving,
    pub(crate) first_enabled_on_entry: bool,
    pub(crate) retain_focus: bool,
    pub(crate) arrows: RovingArrows,
    pub(crate) reveal_after_rebuild: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum RovingArrows {
    Axis,
    All,
    Radio { reverse_horizontal: bool },
    Grid,
}

impl RovingGroup {
    fn is_radio(self) -> bool {
        matches!(self.arrows, RovingArrows::Radio { .. })
    }

    fn is_grid(self) -> bool {
        matches!(self.arrows, RovingArrows::Grid)
    }
}

pub(crate) struct RetainedRovingFocus {
    group: String,
    visible_predecessors: Option<Vec<String>>,
    fully_visible: bool,
    grid_center: Option<grid_navigation::Center>,
}

pub(crate) struct PendingRovingFocus {
    group: String,
    grid_center: Option<grid_navigation::Center>,
}

pub(crate) struct FocusScope {
    id: String,
    restore: Option<String>,
    fallback: Option<String>,
}

pub(crate) enum ModalDismissInput {
    Escape,
    Outside,
}

pub(crate) enum TabDismissal {
    Standard,
    Resume { collapsed: BTreeSet<String> },
    Consumed,
}

pub(crate) struct RangeDrag {
    id: String,
    bounds: Rect,
    range: RangeValue,
    input: Option<crate::SliderInputOptions>,
}

impl RangeDrag {
    pub(crate) fn has_input_policy(&self) -> bool {
        self.input.is_some()
    }

    fn matches_range(&self, current: RangeValue) -> bool {
        current.min == self.range.min
            && current.max == self.range.max
            && current.step == self.range.step
    }
}

/// Normalize requests from the regular setter and native accessibility.
pub(crate) fn changed_slider_value(
    range: RangeValue,
    request: f64,
    input: Option<crate::SliderInputOptions>,
) -> Option<f64> {
    if !request.is_finite() {
        return None;
    }
    let value = match input {
        Some(input) => input.resolve_value(range, request)?,
        None => range.snap(request),
    };
    (value != range.value).then_some(value)
}

fn has_extent(layout: &ComputedDom, id: NodeId) -> bool {
    let bounds = layout.nodes[id.index()].bounds;
    bounds.width > 0. && bounds.height > 0.
}

pub(crate) fn descendant(dom: &MicroDom, mut id: NodeId, parent: NodeId) -> bool {
    loop {
        if id == parent {
            return true;
        }
        let Some(next) = dom.get(id).ok().and_then(Node::parent) else {
            return false;
        };
        id = next;
    }
}

impl<A: Application> Runtime<A> {
    pub(crate) fn apply_focus_navigation(
        &mut self,
        navigation: FocusNavigation<A::Message>,
    ) -> Result<bool, AppError> {
        self.apply(navigation.message)?;
        if self
            .dom
            .by_id(&navigation.target)
            .is_some_and(|target| self.in_scope(target) && self.focus_eligible(target))
        {
            self.focused = Some(navigation.target);
            self.layout_focus = None;
        }
        Ok(true)
    }

    pub(crate) fn descendant(&self, id: NodeId, parent: NodeId) -> bool {
        descendant(&self.dom, id, parent)
    }
    pub(crate) fn active_scope_root(&self) -> Option<NodeId> {
        self.scopes.last().and_then(|s| self.dom.by_id(&s.id))
    }
    pub(crate) fn has_interaction_binding(&self, id: NodeId) -> bool {
        self.bindings
            .get(&id)
            .is_some_and(super::Binding::accepts_interaction)
    }
    pub(crate) fn in_scope(&self, id: NodeId) -> bool {
        self.has_interaction_binding(id)
            && self
                .active_scope_root()
                .is_none_or(|root| self.descendant(id, root))
    }
    pub(crate) fn focus_eligible(&self, id: NodeId) -> bool {
        self.dom.get(id).is_ok_and(Node::is_focusable)
            || self
                .bindings
                .get(&id)
                .is_some_and(super::Binding::disabled_menu_focus)
    }

    pub(crate) fn tab_candidate(&self, id: NodeId, node: &Node) -> bool {
        node.is_tab_stop()
            || self
                .bindings
                .get(&id)
                .and_then(|binding| binding.disabled_menu_tab_stop)
                == Some(true)
    }

    fn group(&self, id: NodeId) -> Option<(NodeId, RovingGroup)> {
        let member = self.dom.get(id).ok()?;
        let mut parent = member.parent();
        let mut crossed_grid = member.accessibility_grid_role() == Some(GridRole::Grid);
        while let Some(id) = parent {
            if let Some(options) = self.bindings.get(&id).and_then(|b| b.roving) {
                return ((!options.is_radio() || member.role() == Role::Radio)
                    && (!options.is_grid() || !crossed_grid))
                    .then_some((id, options));
            }
            let node = self.dom.get(id).ok()?;
            crossed_grid |= node.accessibility_grid_role() == Some(GridRole::Grid);
            parent = node.parent();
        }
        None
    }
    fn members(
        &self,
        group: NodeId,
        collapsed: Option<&BTreeSet<String>>,
        layout: Option<&ComputedDom>,
    ) -> Vec<NodeId> {
        let roving = self.bindings[&group]
            .roving
            .expect("group has roving behavior");
        let measure_members = roving.first_enabled_on_entry || roving.is_radio();
        self.dom
            .iter()
            .filter(|(id, node)| {
                self.tab_candidate(*id, node)
                    && self.in_scope(*id)
                    && self.group(*id).is_some_and(|(parent, _)| parent == group)
                    && collapsed
                        .is_none_or(|ids| node.stable_id().is_none_or(|key| !ids.contains(key)))
                    && if roving.is_grid() {
                        !node.is_disabled()
                            && layout.is_none_or(|layout| self.grid_member_visible(layout, *id))
                    } else {
                        !measure_members || layout.is_none_or(|layout| has_extent(layout, *id))
                    }
            })
            .map(|(id, _)| id)
            .collect()
    }

    fn grid_member_visible(&self, layout: &ComputedDom, id: NodeId) -> bool {
        if !has_extent(layout, id) {
            return false;
        }
        let geometry = &layout.nodes[id.index()];
        let rect = geometry.bounds;
        if ![
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            rect.x + rect.width,
            rect.y + rect.height,
        ]
        .into_iter()
        .all(f32::is_finite)
        {
            return false;
        }
        let portion = rect.intersect(geometry.clip);
        (portion.width > 0. && portion.height > 0.)
            || can_reveal(
                &self.dom,
                layout,
                id,
                self.active_scope_root(),
                |ancestor, _| {
                    let view = layout.nodes[ancestor.index()].bounds;
                    let (width, height) = scroll_extent(&self.dom, layout, ancestor)?;
                    Some(((width - view.width).max(0.), (height - view.height).max(0.)))
                },
            )
    }

    fn grid_center(
        &self,
        layout: &ComputedDom,
        group: NodeId,
        member: NodeId,
    ) -> grid_navigation::Center {
        let (x, y) = grid_navigation::center(layout.nodes[member.index()].bounds);
        let group = layout.nodes[group.index()].bounds;
        (x - f64::from(group.x), y - f64::from(group.y))
    }

    fn roving_entry(
        &self,
        members: &[NodeId],
        options: RovingGroup,
        reverse: bool,
    ) -> Option<NodeId> {
        if !options.first_enabled_on_entry {
            let selected = members.iter().copied().find(|id| {
                self.dom
                    .get(*id)
                    .is_ok_and(|n| n.is_selected() == Some(true) || n.is_checked() == Some(true))
            });
            if selected.is_some() {
                return selected;
            }
        }
        if reverse && options.is_radio() {
            members.last().copied()
        } else {
            members.first().copied()
        }
    }

    // The runtime builds subtrees in preorder. Include stable predecessors
    // that do not take focus, since disabled rows can also move a member.
    fn preceding_roving_ids(&self, group: NodeId, focused: NodeId) -> impl Iterator<Item = &str> {
        let start = group.index() + 1;
        self.dom
            .iter()
            .skip(start)
            .take(focused.index() - start)
            .filter_map(|(_, node)| node.stable_id())
    }

    pub(crate) fn capture_retained_roving_focus(&self) -> Option<RetainedRovingFocus> {
        let focused = self.focused_node().filter(|id| self.in_scope(*id))?;
        let (group, options) = self.group(focused)?;
        if !options.retain_focus || !self.in_scope(group) {
            return None;
        }
        let (visible, fully_visible) = self.computed.as_ref().map_or((false, false), |layout| {
            let bounds = &layout.nodes[focused.index()];
            let portion = bounds.bounds.intersect(bounds.clip);
            let visible = portion.width > 0. && portion.height > 0.;
            (visible, visible && portion == bounds.bounds)
        });
        Some(RetainedRovingFocus {
            group: self.stable(group)?,
            visible_predecessors: visible.then(|| {
                self.preceding_roving_ids(group, focused)
                    .map(str::to_owned)
                    .collect()
            }),
            fully_visible,
            grid_center: self
                .computed
                .as_ref()
                .filter(|layout| options.is_grid() && self.grid_member_visible(layout, focused))
                .map(|layout| self.grid_center(layout, group, focused)),
        })
    }

    fn reconcile_retained_roving_focus(&mut self, retained: RetainedRovingFocus) {
        let Some(group) = self
            .dom
            .by_id(&retained.group)
            .filter(|id| self.in_scope(*id))
        else {
            return;
        };
        let Some(options) = self.bindings[&group]
            .roving
            .filter(|group| group.retain_focus)
        else {
            return;
        };
        if let Some(focused) = self
            .focused_node()
            .filter(|&id| self.in_scope(id) && self.focus_eligible(id))
        {
            if self.group(focused).is_some_and(|(owner, _)| owner == group)
                && ((options.reveal_after_rebuild && retained.fully_visible)
                    || retained.visible_predecessors.is_some_and(|before| {
                        self.preceding_roving_ids(group, focused)
                            .ne(before.iter().map(String::as_str))
                    }))
            {
                self.layout_focus = None;
            }
        } else if !matches!(options.arrows, RovingArrows::Axis) {
            // This group uses measured eligibility. Wait for the replacement
            // tree to lay out before picking a fallback.
            self.pending_roving_focus = Some(PendingRovingFocus {
                group: retained.group,
                grid_center: retained.grid_center,
            });
        } else if let Some(target) = self
            .roving_entry(
                &self.members(group, None, self.computed.as_ref()),
                options,
                false,
            )
            .and_then(|target| self.stable(target))
        {
            self.focused = Some(target);
            self.layout_focus = None;
        }
    }

    pub(crate) fn resolve_pending_roving_focus(&mut self, layout: &ComputedDom) -> bool {
        let Some(pending) = self.pending_roving_focus.take() else {
            return false;
        };
        if self.focused.is_some() {
            return false;
        }
        self.focused = self
            .measured_roving_group(&pending.group)
            .and_then(|(id, options)| {
                let members = self.members(id, None, Some(layout));
                if let Some((x, y)) = pending.grid_center.filter(|_| options.is_grid()) {
                    return members
                        .iter()
                        .copied()
                        .min_by(|a, b| {
                            let distance = |member| {
                                let (mx, my) = self.grid_center(layout, id, member);
                                (mx - x).hypot(my - y)
                            };
                            distance(*a)
                                .total_cmp(&distance(*b))
                                .then_with(|| a.index().cmp(&b.index()))
                        })
                        .and_then(|member| self.stable(member));
                }
                self.stable(self.roving_entry(&members, options, false)?)
            })
            .or_else(|| {
                self.active_scope_root()?;
                self.scope_entry_after_layout(Some(layout))
            });
        self.layout_focus = None;
        self.focused.is_some()
    }

    fn measured_roving_group(&self, key: &str) -> Option<(NodeId, RovingGroup)> {
        let id = self.dom.by_id(key).filter(|id| self.in_scope(*id))?;
        self.bindings[&id]
            .roving
            .filter(|group| !matches!(group.arrows, RovingArrows::Axis))
            .map(|group| (id, group))
    }

    pub(crate) fn tab_stop(
        &self,
        id: NodeId,
        collapsed: Option<&BTreeSet<String>>,
        reverse: bool,
    ) -> bool {
        self.tab_stop_in_layout(id, collapsed, self.computed.as_ref(), reverse)
    }

    fn tab_stop_in_layout(
        &self,
        id: NodeId,
        collapsed: Option<&BTreeSet<String>>,
        layout: Option<&ComputedDom>,
        reverse: bool,
    ) -> bool {
        if !self.in_scope(id) {
            return false;
        }
        let Some((group, options)) = self.group(id) else {
            return true;
        };
        let members = self.members(group, collapsed, layout);
        let current = self.focused_node().filter(|id| members.contains(id));
        current.or_else(|| self.roving_entry(&members, options, reverse)) == Some(id)
    }
    pub(crate) fn has_tab_extent(&self, id: NodeId) -> bool {
        self.computed
            .as_ref()
            .is_none_or(|layout| has_extent(layout, id))
    }
    pub(crate) fn scope_entry(&self) -> Option<String> {
        self.scope_entry_after_layout(None)
    }

    fn scope_entry_after_layout(&self, measured: Option<&ComputedDom>) -> Option<String> {
        if let Some(target) = self
            .active_scope_root()
            .and_then(|id| self.bindings.get(&id)?.modal_initial_focus.as_deref())
            .filter(|key| self.focusable_in_scope(key))
        {
            return Some(target.to_owned());
        }
        self.dom
            .iter()
            .find(|(id, n)| {
                self.tab_candidate(*id, n)
                    && measured.is_none_or(|layout| has_extent(layout, *id))
                    && self.tab_stop_in_layout(
                        *id,
                        None,
                        measured.or(self.computed.as_ref()),
                        false,
                    )
            })
            .and_then(|(id, _)| self.stable(id))
            .or_else(|| {
                self.scopes
                    .last()
                    .and_then(|s| self.dom.by_id(&s.id))
                    .filter(|id| self.focus_eligible(*id))
                    .and_then(|id| self.stable(id))
            })
    }
    fn focusable_in_scope(&self, key: &str) -> bool {
        self.dom
            .by_id(key)
            .is_some_and(|id| self.in_scope(id) && self.focus_eligible(id))
    }

    pub(crate) fn reconcile_controls(
        &mut self,
        focus_after_removal: Vec<String>,
        retained_roving_focus: Option<RetainedRovingFocus>,
    ) {
        let old_focus = self.focused.clone();
        // Bindings follow the current tree's preorder. Later sibling layers
        // cover earlier siblings. A nested layer always follows its parent.
        self.modal_layers.clear();
        self.modal_layers.extend(
            self.bindings
                .iter()
                .filter_map(|(&id, binding)| binding.visual_overlay.then_some((None, id))),
        );
        self.modal_layers
            .extend(self.bindings.iter().filter_map(|(&panel, binding)| {
                let layer = binding.modal_layer?;
                if !layer.is_retiring()
                    && (!binding.accepts_interaction() || binding.dismiss.is_none())
                {
                    return None;
                }
                let backdrop = if layer.has_backdrop() {
                    self.dom.get(panel).ok()?.parent()
                } else {
                    None
                };
                Some((backdrop, panel))
            }));
        let mut order = self.dom.paint_order();
        self.dom.apply_modal_layers(&mut order, &self.modal_layers);
        let panels: Vec<_> = order
            .into_iter()
            .filter(|id| {
                self.bindings
                    .get(id)
                    .is_some_and(|b| b.accepts_interaction() && b.dismiss.is_some())
            })
            .filter_map(|id| self.stable(id))
            .collect();
        let common = self
            .scopes
            .iter()
            .map(|s| &s.id)
            .zip(&panels)
            .take_while(|(a, b)| a == b)
            .count();
        let same_scope = common == self.scopes.len() && common == panels.len();
        if !same_scope {
            self.pending_roving_focus = None;
            self.layout_focus = None;
            self.cancel_press_ripple();
            self.clear_modal_pointer();
        }
        while self.scopes.len() > common {
            let scope = self.scopes.pop().expect("scope exists");
            self.focused = scope
                .restore
                .filter(|key| self.focusable_in_scope(key))
                .or_else(|| {
                    let fallback = scope.fallback?;
                    self.focused
                        .as_ref()
                        .filter(|key| self.focusable_in_scope(key))
                        .cloned()
                        .or_else(|| self.focusable_in_scope(&fallback).then_some(fallback))
                });
        }
        let fallback = |key: &str| {
            self.dom
                .by_id(key)
                .and_then(|id| self.bindings.get(&id)?.modal_focus_fallback.clone())
        };
        for scope in &mut self.scopes {
            scope.fallback = fallback(&scope.id);
        }
        for id in panels.into_iter().skip(common) {
            self.scopes.push(FocusScope {
                fallback: fallback(&id),
                id,
                restore: self.focused.clone(),
            });
            self.focused = self.scope_entry();
        }
        if same_scope {
            if let Some(suppression) = self
                .focused_node()
                .and_then(|id| self.bindings.get(&id)?.interaction_suppression.as_ref())
            {
                self.focused = suppression
                    .focus_fallback
                    .as_ref()
                    .filter(|key| self.focusable_in_scope(key))
                    .cloned();
                self.layout_focus = None;
            } else if let Some(target) = focus_after_removal.into_iter().find(|target| {
                self.focused_node()
                    .is_none_or(|id| !self.focus_eligible(id))
                    && self.focusable_in_scope(target)
            }) {
                self.focused = Some(target);
                self.layout_focus = None;
            } else if let Some(retained) = retained_roving_focus {
                self.reconcile_retained_roving_focus(retained);
            }
        }
        if self
            .pending_roving_focus
            .as_ref()
            .is_some_and(|pending| self.measured_roving_group(&pending.group).is_none())
        {
            self.pending_roving_focus = None;
        }
        if self.pending_roving_focus.is_none()
            && self.scopes.last().is_some()
            && self.focused_node().is_none_or(|id| !self.in_scope(id))
        {
            self.focused = self.scope_entry();
        }
        if self.focused != old_focus {
            self.cancel_composition();
            self.ime_area = None;
            self.text_drag = None;
        }
        if self
            .range_drag
            .as_ref()
            .is_some_and(|drag| !self.valid_range_drag(drag))
        {
            if self
                .range_drag
                .as_ref()
                .is_some_and(RangeDrag::has_input_policy)
            {
                self.pressed = None;
            }
            self.range_drag = None;
        }
    }
    fn valid_range_drag(&self, drag: &RangeDrag) -> bool {
        let Some(id) = self.dom.by_id(&drag.id).filter(|id| self.in_scope(*id)) else {
            return false;
        };
        let node = self.dom.get(id).expect("stable slider exists");
        if node.is_disabled() || node.role() != Role::Slider {
            return false;
        }
        !drag.has_input_policy()
            || (self.bindings.get(&id).is_some_and(|binding| {
                binding.value.is_some() && binding.slider_input == drag.input
            }) && node
                .range_value()
                .is_some_and(|range| drag.matches_range(range)))
    }
    pub(crate) fn clear_modal_pointer(&mut self) {
        self.modal_outside = false;
        self.pressed = None;
        self.hovered = None;
        self.text_drag = None;
        self.range_drag = None;
    }
    pub(crate) fn outside_scope(&self, x: f32, y: f32, hit: Option<NodeId>) -> bool {
        let Some(root) = self.active_scope_root() else {
            return false;
        };
        self.computed.as_ref().is_some_and(|layout| {
            let contains = |id: NodeId| {
                let geometry = &layout.nodes[id.index()];
                geometry.bounds.intersect(geometry.clip).contains(x, y)
            };
            let binding = &self.bindings[&root];
            if !binding.modal_descendants_only && contains(root) {
                return false;
            }
            if binding.modal_layer.is_none() {
                return true;
            }
            // A painted or scrollable positioned descendant is part of the
            // modal even at a blank point. A transparent spacer is not.
            hit.is_none()
                && !self.dom.iter().any(|(id, node)| {
                    let style = node.style();
                    style.position != Position::Flow
                        && (style.clip
                            || style.background.3 > 0
                            || (style.border_width > 0. && style.border_color.3 > 0))
                        && contains(id)
                        && self.descendant(id, root)
                })
        })
    }
    pub(crate) fn scopes_block_application_shortcuts(&self) -> bool {
        self.scopes.iter().any(|scope| {
            self.dom
                .by_id(&scope.id)
                .and_then(|id| self.bindings.get(&id))
                .is_some_and(|binding| binding.block_application_shortcuts)
        })
    }

    pub(crate) fn dismiss_scope(&mut self, input: ModalDismissInput) -> Result<bool, AppError> {
        let message = self
            .active_scope_root()
            .and_then(|id| self.bindings.get(&id))
            .filter(|binding| match binding.modal_dismissal {
                ModalDismissal::EscapeOrOutside => true,
                ModalDismissal::Escape => matches!(input, ModalDismissInput::Escape),
                ModalDismissal::ActionOnly => false,
            })
            .and_then(|binding| binding.dismiss.clone());
        self.message(message).map(|_| true)
    }
    /// Deliver the top scope's opt-in dismiss message. Consume the key if
    /// an IME composition owns it or the app does not return to the parent.
    pub(crate) fn dismiss_tab_scope(&mut self) -> Result<TabDismissal, AppError> {
        let Some(dismiss) = self
            .active_scope_root()
            .and_then(|id| self.bindings.get(&id))
            .filter(|binding| binding.dismiss_on_tab)
            .and_then(|binding| binding.dismiss.clone())
        else {
            return Ok(TabDismissal::Standard);
        };
        if self.has_pending_composition() {
            return Ok(TabDismissal::Consumed);
        }
        let containing_depth = self.scopes.len() - 1;
        let containing_id = self
            .scopes
            .iter()
            .rev()
            .nth(1)
            .map(|scope| scope.id.clone());
        // Closing the menu discards its layout. Keep the last known size
        // exclusions by stable ID until the containing scope lays out again.
        let collapsed = self
            .dom
            .iter()
            .filter(|(id, _)| !self.has_tab_extent(*id))
            .filter_map(|(_, node)| node.stable_id().map(str::to_owned))
            .collect();
        self.message(Some(dismiss))?;
        if self.scopes.len() == containing_depth
            && self.scopes.last().map(|scope| &scope.id) == containing_id.as_ref()
        {
            Ok(TabDismissal::Resume { collapsed })
        } else {
            Ok(TabDismissal::Consumed)
        }
    }
    pub(crate) fn navigate_control(
        &mut self,
        key: EditKey,
        extend: bool,
    ) -> Result<bool, AppError> {
        let Some(id) = self.focused_node() else {
            return Ok(false);
        };
        let node = self.dom.get(id)?;
        if let Some(range) = node.range_value().filter(|_| node.role() == Role::Slider) {
            if let Some(input) = self
                .bindings
                .get(&id)
                .and_then(|binding| binding.slider_input)
            {
                let Some(value) = input.key_value(range, key) else {
                    return Ok(false);
                };
                let id = self.stable(id).expect("slider has ID");
                return self.set_value(&id, value);
            }
            let value = match key {
                EditKey::Left | EditKey::Down => range.value - range.step,
                EditKey::Right | EditKey::Up => range.value + range.step,
                EditKey::Home => range.min,
                EditKey::End => range.max,
                EditKey::PageUp => range.value + ((range.max - range.min) / 10.).max(range.step),
                EditKey::PageDown => range.value - ((range.max - range.min) / 10.).max(range.step),
                _ => return Ok(false),
            };
            let id = self.stable(id).expect("slider has ID");
            return self.set_value(&id, value);
        }
        if self.in_scope(id) && node.is_focusable() && !self.has_pending_composition() {
            if let Some(navigation) = self.bindings[&id]
                .focus_navigation
                .as_ref()
                .and_then(|navigate| navigate(key, extend))
            {
                return self.apply_focus_navigation(navigation);
            }
            if let Some(range) = node.accessibility_adjustable() {
                let action = match key {
                    EditKey::Up => Some(AccessibilityAdjustment::Increment),
                    EditKey::Down => Some(AccessibilityAdjustment::Decrement),
                    EditKey::Home => Some(AccessibilityAdjustment::SetValue(range.min)),
                    EditKey::End => Some(AccessibilityAdjustment::SetValue(range.max)),
                    _ => None,
                };
                if let Some(action) = action {
                    let message = action.resolve(range).and_then(|value| {
                        self.bindings
                            .get(&id)?
                            .value
                            .as_ref()
                            .map(|handler| handler(value))
                    });
                    self.message(message)?;
                    return Ok(true);
                }
            }
        }
        let Some((group, roving)) = self
            .bindings
            .get(&id)
            .and_then(|b| b.roving.map(|options| (id, options)))
            .or_else(|| self.group(id))
        else {
            return Ok(false);
        };
        let options = roving.options;
        let members = self.members(group, None, self.computed.as_ref());
        if roving.is_grid() {
            if !self.in_scope(id) || !self.in_scope(group) || !grid_navigation::is_navigation(key) {
                return Ok(false);
            }
            if let Some(next) = self
                .computed
                .as_ref()
                .and_then(|layout| grid_navigation::target(key, id, &members, layout))
            {
                self.focused = self.stable(next);
            }
            return Ok(true);
        }
        if members.is_empty() {
            return Ok(false);
        }
        let index = members.iter().position(|node| *node == id);
        let reversed = self.dom.get(group)?.is_row_reversed();
        let key = match roving.arrows {
            RovingArrows::Axis if options.direction == Direction::Row && reversed => match key {
                EditKey::Left => EditKey::Right,
                EditKey::Right => EditKey::Left,
                _ => key,
            },
            RovingArrows::Axis => key,
            RovingArrows::Grid => unreachable!("grid navigation uses measured rows"),
            RovingArrows::All | RovingArrows::Radio { .. } => {
                let mirrored = matches!(
                    roving.arrows,
                    RovingArrows::Radio {
                        reverse_horizontal: true
                    }
                );
                match (key, mirrored) {
                    (EditKey::Left, false) | (EditKey::Right, true) => EditKey::Up,
                    (EditKey::Right, false) | (EditKey::Left, true) => EditKey::Down,
                    _ => key,
                }
            }
        };
        let next = match (key, options.direction) {
            (EditKey::Home, _) => 0,
            (EditKey::End, _) => members.len() - 1,
            (EditKey::Left, Direction::Row) | (EditKey::Up, Direction::Column) => index
                .map_or(members.len() - 1, |i| {
                    (i + members.len() - 1) % members.len()
                }),
            (EditKey::Right, Direction::Row) | (EditKey::Down, Direction::Column) => {
                index.map_or(0, |i| (i + 1) % members.len())
            }
            _ => return Ok(false),
        };
        let next = members[next];
        let target = self.stable(next).expect("control has ID");
        if options.automatic && !self.dom.get(next)?.is_disabled() {
            self.activate(&target)
                .map(|handled| handled || roving.is_radio())
        } else {
            self.focused = Some(target);
            Ok(true)
        }
    }
    /// The semantic driver, keyboard and pointer use the same controlled value path.
    pub fn set_value(&mut self, key: &str, value: f64) -> Result<bool, AppError> {
        if !value.is_finite() {
            return Ok(false);
        }
        let id = self
            .dom
            .by_id(key)
            .filter(|id| {
                self.in_scope(*id)
                    && self
                        .dom
                        .get(*id)
                        .is_ok_and(|n| n.role() == Role::Slider && !n.is_disabled())
            })
            .ok_or_else(|| AppError::MissingControl(key.to_owned()))?;
        let range = self.dom.get(id)?.range_value().expect("validated slider");
        let binding = self.bindings.get(&id);
        let input = binding.and_then(|binding| binding.slider_input);
        let Some(value) = changed_slider_value(range, value, input) else {
            return Ok(false);
        };
        let message = binding
            .and_then(|b| b.value.as_ref())
            .map(|handler| handler(value));
        self.message(message)
    }
    pub(crate) fn start_range_drag(&mut self) -> bool {
        let Some(id) = self.focused_node() else {
            return false;
        };
        let node = self.dom.get(id).expect("focused node exists");
        if node.role() != Role::Slider {
            return false;
        }
        let binding = self.bindings.get(&id);
        let input = binding.and_then(|binding| binding.slider_input);
        if input.is_some() && binding.is_none_or(|binding| binding.value.is_none()) {
            return false;
        }
        let Some(computed) = &self.computed else {
            return false;
        };
        let bounds = computed.nodes[id.index()]
            .bounds
            .inset(node.style().padding);
        self.range_drag = Some(RangeDrag {
            id: self.stable(id).expect("slider has ID"),
            bounds,
            range: node.range_value().expect("validated range"),
            input,
        });
        true
    }
    pub(crate) fn range_pointer(&mut self, x: f32) -> Result<bool, AppError> {
        let Some(drag) = &self.range_drag else {
            return Ok(false);
        };
        if !x.is_finite() || drag.bounds.width <= 0. {
            return Ok(false);
        }
        let value = if let Some(input) = drag.input {
            if self.computed.as_ref().is_some_and(|layout| {
                self.dom.by_id(&drag.id).is_none_or(|id| {
                    self.dom.get(id).is_ok_and(|node| {
                        layout.nodes[id.index()].bounds.inset(node.style().padding) != drag.bounds
                    })
                })
            }) {
                return self.cancel_pointer();
            }
            input.pointer_value(drag.range, drag.bounds, x)
        } else {
            drag.range.min
                + ((x - drag.bounds.x) / drag.bounds.width).clamp(0., 1.) as f64
                    * (drag.range.max - drag.range.min)
        };
        let id = drag.id.clone();
        self.set_value(&id, value).map(|_| true)
    }
}
