//! The rendered accessibility tree and checked input actions for platform hosts.
use crate::{
    controls::descendant, focus_scroll::can_reveal, AppError, Application, Binding, Event,
    KeyPress, Runtime, ScrollMetrics, TextOptions,
};
use std::{
    cell::{OnceCell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::LazyLock,
};
use surface::{scroll_extent, ComputedDom, ComputedNode, MicroDom, Node, NodeId, Rect, Role};

static EMPTY_ROOT: LazyLock<Node> = LazyLock::new(|| Node::new(Role::Group, ""));

/// Identity of a rendered accessibility tree, local to the UI thread.
/// A host must retire its ordinary adapter IDs when this changes. Leased
/// control IDs can survive with a fresh target from the new frame.
#[derive(Clone, Debug)]
pub struct AccessibilityFrameId(Rc<()>);

impl PartialEq for AccessibilityFrameId {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for AccessibilityFrameId {}

/// An opaque focus target issued from a rendered tree. Route native requests
/// on the Runtime's UI thread and carry this token from the tree where Focus
/// was advertised. Never resolve a delayed request using the current node ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessibilityFocusTarget {
    frame: AccessibilityFrameId,
    id: NodeId,
}

/// A checked reveal target in the currently rendered accessibility tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessibilityScrollTarget {
    frame: AccessibilityFrameId,
    id: NodeId,
    focus: Option<NodeId>,
}

/// Physical directions within a scroll viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// A checked page request from the tree where this direction was advertised.
/// Do not resolve delayed actions using the current node ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessibilityScrollPageTarget {
    frame: AccessibilityFrameId,
    id: NodeId,
    focus: Option<NodeId>,
    direction: AccessibilityScrollDirection,
}

pub(crate) enum PendingScroll {
    IntoView(AccessibilityScrollTarget),
    Page(AccessibilityScrollPageTarget),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityScrollResult {
    /// The scroll will be applied on the next render if still valid.
    Scheduled,
    /// The requested scroll is unavailable in the current rendered scope.
    Rejected,
    BlockedByComposition,
    BlockedByPointer,
    /// Earlier queued updates were applied. Publish the new tree.
    Rebuilt,
}

/// An opaque press target issued from a rendered tree. A host must route the
/// request on the UI thread using the target from the tree that offered it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessibilityActivationTarget {
    frame: AccessibilityFrameId,
    id: NodeId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityActivationResult {
    /// The press message was dispatched. Render and publish the new layout.
    Activated,
    /// The target or its rendered geometry is no longer current.
    Rejected,
    /// Activation would interrupt pending input method composition.
    BlockedByComposition,
    /// Activation would interrupt a pointer interaction.
    BlockedByPointer,
    /// Earlier queued updates were applied without activating this target.
    /// Render and publish the new layout before accepting another request.
    Rebuilt,
}

/// The result of requesting keyboard input focus through an accessibility host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityFocusResult {
    /// Focus moved. The host should render and publish the new layout.
    Moved,
    /// The target already has focus.
    Unchanged,
    /// The target or the rendered tree is no longer available.
    Rejected,
    /// Focus stayed on the current field to preserve its pending IME input.
    BlockedByComposition,
    /// Focus stayed in place until the current pointer interaction ends.
    BlockedByPointer,
    /// Earlier queued work was applied. Render and publish the new tree;
    /// the old focus request was not applied.
    Rebuilt,
}

#[derive(Default)]
pub(crate) struct Revision {
    id: OnceCell<AccessibilityFrameId>,
    issued_targets: RefCell<BTreeSet<NodeId>>,
    issued_scroll_targets: RefCell<BTreeSet<NodeId>>,
    issued_scroll_page_targets: RefCell<BTreeSet<NodeId>>,
    rendered: bool,
}

/// A borrowed view of the current accessibility tree and layout. During a
/// modal interaction its root is the modal, and its background is excluded.
/// Only committed text in the view is available here. Editor preedit, parked
/// fields and private interaction state are not exported.
pub struct AccessibilityFrame<'a, M> {
    revision: &'a Revision,
    dom: &'a MicroDom,
    layout: &'a ComputedDom,
    bindings: &'a BTreeMap<NodeId, Binding<M>>,
    scroll_metrics: &'a BTreeMap<String, ScrollMetrics>,
    modal: Option<NodeId>,
    focused: Option<NodeId>,
}

pub struct AccessibilityNode<'a> {
    pub node: &'a Node,
    pub layout: &'a ComputedNode,
    pub text_options: Option<&'a TextOptions>,
}

struct ScrollPageGeometry {
    offset: (f32, f32),
    max: (f32, f32),
    page: Rect,
}

impl ScrollPageGeometry {
    fn next_offset(&self, direction: AccessibilityScrollDirection) -> Option<(f32, f32)> {
        use AccessibilityScrollDirection::{Down, Left, Right, Up};
        let (x, y) = self.offset;
        let (max_x, max_y) = self.max;
        let next = match direction {
            Left if max_x > 0. && x > 0. => ((x - self.page.width).clamp(0., max_x), y),
            Right if x < max_x => ((x + self.page.width).clamp(0., max_x), y),
            Up if max_y > 0. && y > 0. => (x, (y - self.page.height).clamp(0., max_y)),
            Down if y < max_y => (x, (y + self.page.height).clamp(0., max_y)),
            _ => return None,
        };
        (next != self.offset).then_some(next)
    }
}

impl<M> AccessibilityFrame<'_, M> {
    /// Ordinary renders keep the same identity. Rebuilds, render failures and
    /// loss of visibility of any issued action target retire it.
    pub fn generation(&self) -> AccessibilityFrameId {
        self.revision
            .id
            .get_or_init(|| AccessibilityFrameId(Rc::new(())))
            .clone()
    }

    pub fn root(&self) -> NodeId {
        self.modal.unwrap_or_else(|| self.dom.root())
    }

    pub fn modal_root(&self) -> Option<NodeId> {
        self.modal
    }

    pub fn focused(&self) -> Option<NodeId> {
        self.focused
            .filter(|id| self.node(*id).is_some_and(|node| focusable(&node)))
    }

    pub fn children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let in_tree = self.node(id).is_some();
        self.dom
            .children(id)
            .filter(move |child| in_tree && self.accepts_interaction(*child))
    }

    fn accepts_interaction(&self, id: NodeId) -> bool {
        self.bindings
            .get(&id)
            .is_some_and(Binding::accepts_interaction)
    }

    pub fn node(&self, id: NodeId) -> Option<AccessibilityNode<'_>> {
        let node = self.dom.get(id).ok()?;
        let interactive = self.accepts_interaction(id);
        // Both platform trees need an application root even when its
        // contents only paint. Give them an empty semantic root in that case.
        if (!interactive && id != self.dom.root())
            || self
                .modal
                .is_some_and(|root| !descendant(self.dom, id, root))
        {
            return None;
        }
        let layout = self.layout.nodes.get(id.index())?;
        let text_options = if interactive && node.role() == Role::TextInput {
            self.bindings.get(&id).map(|binding| &binding.text_options)
        } else {
            None
        };
        Some(AccessibilityNode {
            node: if interactive { node } else { &EMPTY_ROOT },
            layout,
            text_options,
        })
    }

    /// Whether this named group uses roving focus within the current tree.
    /// This describes the group boundary rather than a focus or press action.
    pub fn is_named_roving_group(&self, id: NodeId) -> bool {
        self.node(id).is_some_and(|node| {
            node.node.role() == Role::Group && !node.node.name().trim().is_empty()
        }) && self
            .bindings
            .get(&id)
            .is_some_and(|binding| binding.accepts_interaction() && binding.roving.is_some())
    }

    /// Whether current visible geometry allows focus without issuing a target.
    pub fn accepts_focus(&self, id: NodeId) -> bool {
        self.node(id).is_some_and(|node| focusable(&node))
    }

    /// Focus is available to enabled controls whose bounds are not completely
    /// clipped. This test does not detect occlusion by other painted nodes.
    pub fn focus_target(&self, id: NodeId) -> Option<AccessibilityFocusTarget> {
        if !self.accepts_focus(id) {
            return None;
        }
        self.revision.issued_targets.borrow_mut().insert(id);
        Some(AccessibilityFocusTarget {
            frame: self.generation(),
            id,
        })
    }

    /// An explicitly enabled target that can be brought into view by its
    /// in-scope scroll ancestors. A fully clipped target can offer reveal
    /// even while focus and press actions are unavailable.
    pub fn scroll_target(&self, id: NodeId) -> Option<AccessibilityScrollTarget> {
        if !self.scroll_available(id) {
            return None;
        }
        self.revision.issued_scroll_targets.borrow_mut().insert(id);
        Some(AccessibilityScrollTarget {
            frame: self.generation(),
            id,
            focus: self.focused,
        })
    }

    fn scroll_available(&self, id: NodeId) -> bool {
        self.node(id)
            .is_some_and(|source| !source.node.is_disabled())
            && self
                .bindings
                .get(&id)
                .is_some_and(|binding| binding.accepts_interaction() && binding.accessible_scroll)
            && can_reveal(self.dom, self.layout, id, self.modal, |_, key| {
                let metrics = self.scroll_metrics.get(key)?;
                Some((
                    (metrics.content.0 - metrics.viewport.0).max(0.),
                    (metrics.content.1 - metrics.viewport.1).max(0.),
                ))
            })
    }

    /// Issue a relative page action for an opted-in viewport. The page uses
    /// the visible size on the requested axis. Advertise only directions
    /// that can move within the current scroll range.
    pub fn scroll_page_target(
        &self,
        id: NodeId,
        direction: AccessibilityScrollDirection,
    ) -> Option<AccessibilityScrollPageTarget> {
        self.scroll_page_geometry(id, self.layout)?
            .next_offset(direction)?;
        self.revision
            .issued_scroll_page_targets
            .borrow_mut()
            .insert(id);
        Some(AccessibilityScrollPageTarget {
            frame: self.generation(),
            id,
            focus: self.focused,
            direction,
        })
    }

    fn scroll_page_geometry(
        &self,
        id: NodeId,
        computed: &ComputedDom,
    ) -> Option<ScrollPageGeometry> {
        let node = self.node(id)?.node;
        let binding = self.bindings.get(&id)?;
        let style = node.style();
        if node.is_disabled()
            || !binding.accepts_interaction()
            || !style.clip
            || !binding.accessible_scroll_pages
        {
            return None;
        }
        let geometry = computed.nodes.get(id.index())?;
        let page = geometry.bounds.intersect(geometry.clip);
        if page.width <= 0. || page.height <= 0. {
            return None;
        }
        let (width, height) = scroll_extent(self.dom, computed, id)?;
        let max = (
            width - geometry.bounds.width,
            height - geometry.bounds.height,
        );
        if max.0 <= 0. && max.1 <= 0. {
            return None;
        }
        Some(ScrollPageGeometry {
            offset: (style.scroll_x, style.scroll_y),
            max,
            page,
        })
    }

    /// Offer a press action for a visible, enabled control in this scope.
    /// The app must opt in through `View::accessible_activation`.
    pub fn activation_target(&self, id: NodeId) -> Option<AccessibilityActivationTarget> {
        self.press_binding(id)?;
        self.revision.issued_targets.borrow_mut().insert(id);
        Some(AccessibilityActivationTarget {
            frame: self.generation(),
            id,
        })
    }

    fn press_binding(&self, id: NodeId) -> Option<(AccessibilityNode<'_>, &M)> {
        let node = self.node(id).filter(focusable)?;
        if !offers_press(node.node.role()) {
            return None;
        }
        let binding = self
            .bindings
            .get(&id)
            .filter(|b| b.accepts_interaction() && b.accessible_activation)?;
        Some((node, binding.activate.as_ref()?))
    }
}

fn offers_press(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::Checkbox
            | Role::Radio
            | Role::Switch
            | Role::Tab
            | Role::MenuItem
            | Role::ListItem
    )
}

fn focusable(node: &AccessibilityNode<'_>) -> bool {
    let visible = node.layout.bounds.intersect(node.layout.clip);
    node.node.is_focusable()
        && node.node.stable_id().is_some()
        && visible.width > 0.
        && visible.height > 0.
}

impl<A: Application> Runtime<A> {
    /// Borrow a frame after a successful render, when the current view has
    /// no pending messages and the window is not waiting to close.
    pub fn accessibility_frame(&self) -> Option<AccessibilityFrame<'_, A::Message>> {
        let frame = self.accessibility_frame_raw()?;
        (frame.revision.rendered && !self.queue.1.get() && self.queue.0.borrow().is_empty())
            .then_some(frame)
    }

    fn accessibility_frame_raw(&self) -> Option<AccessibilityFrame<'_, A::Message>> {
        Some(AccessibilityFrame {
            revision: self.accessibility.as_ref()?,
            dom: &self.dom,
            layout: self.computed.as_ref()?,
            bindings: &self.bindings,
            scroll_metrics: &self.scroll_state.metrics,
            modal: self.active_scope_root(),
            focused: self.focused_node(),
        })
    }

    pub(crate) fn accessibility_rendered(&mut self, success: bool) {
        let visibility_lost = self.accessibility_frame_raw().is_some_and(|frame| {
            frame
                .revision
                .issued_targets
                .borrow()
                .iter()
                .any(|id| !frame.node(*id).is_some_and(|node| focusable(&node)))
                || frame
                    .revision
                    .issued_scroll_targets
                    .borrow()
                    .iter()
                    .any(|id| !frame.scroll_available(*id))
                || frame
                    .revision
                    .issued_scroll_page_targets
                    .borrow()
                    .iter()
                    .any(|id| frame.scroll_page_geometry(*id, frame.layout).is_none())
        });
        if let Some(revision) = &mut self.accessibility {
            if !success || visibility_lost {
                *revision = Revision::default();
            }
            revision.rendered = success;
        }
    }

    /// Schedule an accessibility reveal without focusing or invoking the target.
    /// The next render cancels it if the view or input state was replaced.
    pub fn accessibility_scroll_into_view(
        &mut self,
        target: &AccessibilityScrollTarget,
    ) -> Result<AccessibilityScrollResult, AppError> {
        self.schedule_accessibility_scroll(PendingScroll::IntoView(target.clone()))
    }

    pub(crate) fn check_accessibility_reveal(
        &self,
        target: &AccessibilityScrollTarget,
        computed: &ComputedDom,
    ) -> Option<NodeId> {
        if !self.matches_accessibility_scroll(&target.frame, target.focus)
            || self.has_pending_composition()
            || self.accessibility_pointer_active()
            || !self.queue.2.borrow().is_empty()
            || !self
                .accessibility_frame()
                .is_some_and(|frame| frame.scroll_available(target.id))
        {
            return None;
        }
        can_reveal(
            &self.dom,
            computed,
            target.id,
            self.active_scope_root(),
            |id, _| {
                let size = computed.nodes[id.index()].bounds;
                let (width, height) = scroll_extent(&self.dom, computed, id)?;
                Some(((width - size.width).max(0.), (height - size.height).max(0.)))
            },
        )
        .then_some(target.id)
    }

    /// Schedule one page on the original viewport without focusing or
    /// activating it. The next render uses its current visible geometry.
    /// Only the last accessibility scroll request before a render is kept.
    pub fn accessibility_scroll_page(
        &mut self,
        target: &AccessibilityScrollPageTarget,
    ) -> Result<AccessibilityScrollResult, AppError> {
        self.schedule_accessibility_scroll(PendingScroll::Page(target.clone()))
    }

    fn schedule_accessibility_scroll(
        &mut self,
        request: PendingScroll,
    ) -> Result<AccessibilityScrollResult, AppError> {
        use AccessibilityScrollResult as Outcome;
        let (frame, focus) = match &request {
            PendingScroll::IntoView(target) => (&target.frame, target.focus),
            PendingScroll::Page(target) => (&target.frame, target.focus),
        };
        if !self.matches_accessibility_scroll(frame, focus) {
            return Ok(Outcome::Rejected);
        }
        if self.has_pending_composition() {
            return Ok(Outcome::BlockedByComposition);
        }
        if self.accessibility_pointer_active() {
            return Ok(Outcome::BlockedByPointer);
        }
        if !self.queue.0.borrow().is_empty() {
            self.flush()?;
            return Ok(Outcome::Rebuilt);
        }
        if !self.queue.2.borrow().is_empty()
            || !self
                .accessibility_frame()
                .is_some_and(|frame| match &request {
                    PendingScroll::IntoView(target) => frame.scroll_available(target.id),
                    PendingScroll::Page(target) => frame
                        .scroll_page_geometry(target.id, frame.layout)
                        .and_then(|geometry| geometry.next_offset(target.direction))
                        .is_some(),
                })
        {
            return Ok(Outcome::Rejected);
        }
        self.pending_accessibility_scroll = Some(request);
        Ok(Outcome::Scheduled)
    }

    pub(crate) fn apply_accessibility_scroll_page(
        &mut self,
        target: &AccessibilityScrollPageTarget,
        computed: &ComputedDom,
    ) -> Result<bool, AppError> {
        if !self.matches_accessibility_scroll(&target.frame, target.focus)
            || self.has_pending_composition()
            || self.accessibility_pointer_active()
            || !self.queue.2.borrow().is_empty()
        {
            return Ok(false);
        }
        let Some(geometry) = self
            .accessibility_frame()
            .and_then(|frame| frame.scroll_page_geometry(target.id, computed))
        else {
            return Ok(false);
        };
        let Some(offset) = geometry.next_offset(target.direction) else {
            return Ok(false);
        };
        let key = self
            .stable(target.id)
            .expect("accessibility scroller has ID");
        let mut style = self.dom.get(target.id)?.style().clone();
        style.scroll_x = offset.0;
        style.scroll_y = offset.1;
        self.dom.set_style(target.id, style)?;
        self.scroll.insert(key, offset);
        Ok(true)
    }

    fn matches_accessibility_scroll(
        &self,
        frame: &AccessibilityFrameId,
        focus: Option<NodeId>,
    ) -> bool {
        self.accessibility
            .as_ref()
            .and_then(|revision| revision.id.get())
            == Some(frame)
            && self.focused_node() == focus
            && !self.queue.1.get()
    }

    fn accessibility_pointer_active(&self) -> bool {
        self.pressed.is_some()
            || self.range_drag.is_some()
            || self.has_captured_drag()
            || self.text_drag.is_some()
            || self.modal_outside
    }

    /// Move input focus without invoking the control's action or changing text.
    /// Every request must refer to a target from the current rendered tree.
    pub fn accessibility_focus(
        &mut self,
        target: &AccessibilityFocusTarget,
    ) -> Result<AccessibilityFocusResult, AppError> {
        use AccessibilityFocusResult as Outcome;
        let current = self
            .accessibility
            .as_ref()
            .and_then(|revision| revision.id.get());
        if current != Some(&target.frame) || self.queue.1.get() {
            return Ok(Outcome::Rejected);
        }
        if self.focused_node() == Some(target.id) {
            return Ok(Outcome::Unchanged);
        }
        // An empty preedit can retain a replacement range until commit. Do
        // not flush queued updates during preedit: they could remove the field.
        if self
            .editors
            .values()
            .any(|editor| editor.composition().is_some())
        {
            return Ok(Outcome::BlockedByComposition);
        }
        if self.pressed.is_some()
            || self.range_drag.is_some()
            || self.has_captured_drag()
            || self.modal_outside
        {
            return Ok(Outcome::BlockedByPointer);
        }
        if !self.queue.0.borrow().is_empty() {
            self.flush()?;
            return Ok(Outcome::Rebuilt);
        }
        let Some(stable_id) = self.accessibility_frame().and_then(|frame| {
            let node = frame.node(target.id).filter(focusable)?;
            node.node.stable_id().map(str::to_owned)
        }) else {
            return Ok(Outcome::Rejected);
        };
        self.focused = Some(stable_id);
        self.computed = None;
        self.layout_focus = None;
        self.ime_area = None;
        self.text_drag = None;
        Ok(Outcome::Moved)
    }

    /// Route a keyboard event from a rendered semantic control to current input focus.
    ///
    /// A projected tree can still show the previous control as focused when
    /// keys arrive faster than the next frame. The source token authorizes
    /// delivery within its current tree and scope. It never moves focus.
    /// Use no fallback for a press that only the app can handle.
    pub fn accessibility_key(
        &mut self,
        source: &AccessibilityFocusTarget,
        press: &KeyPress,
        fallback: impl Into<Option<Event>>,
    ) -> Result<bool, AppError> {
        if self.queue.1.get()
            || !self.accessibility.as_ref().is_some_and(|revision| {
                revision.rendered
                    && revision.id.get() == Some(&source.frame)
                    && revision.issued_targets.borrow().contains(&source.id)
            })
        {
            return Ok(false);
        }
        let focusable = |id| self.in_scope(id) && self.dom.get(id).is_ok_and(Node::is_focusable);
        let Some(focused) = self.focused_node().filter(|&id| focusable(id)) else {
            return Ok(false);
        };
        if !focusable(source.id) {
            return Ok(false);
        }
        if self.has_pending_composition()
            || (source.id != focused
                && (self.pressed.is_some()
                    || self.range_drag.is_some()
                    || self.has_captured_drag()
                    || self.modal_outside))
        {
            return Ok(false);
        }
        if !self.queue.0.borrow().is_empty() {
            self.flush()?;
            return Ok(true);
        }
        if self.key(press)? {
            Ok(true)
        } else if let Some(event) = fallback.into() {
            self.dispatch(event)
        } else {
            Ok(false)
        }
    }

    /// Dispatch a press which the current rendered tree offered. The
    /// original press binding is checked before cloning its message and
    /// checked again before changing focus or dispatching it.
    pub fn accessibility_activate(
        &mut self,
        target: &AccessibilityActivationTarget,
    ) -> Result<AccessibilityActivationResult, AppError> {
        use AccessibilityActivationResult as Outcome;
        if let Some(outcome) = self.activation_blocked(target)? {
            return Ok(outcome);
        }
        let Some((stable_id, message)) = self.accessibility_frame().and_then(|frame| {
            let (node, message) = frame.press_binding(target.id)?;
            Some((node.node.stable_id()?.to_owned(), message.clone()))
        }) else {
            return Ok(Outcome::Rejected);
        };
        // A message's Clone implementation can enqueue updates or request
        // closure through a Queue. Never run the old press after those updates.
        if let Some(outcome) = self.activation_blocked(target)? {
            return Ok(outcome);
        }
        #[cfg(feature = "devtools")]
        self.trace_source(&stable_id, crate::trace::Handler::Press);
        self.focused = Some(stable_id);
        self.message(Some(message))?;
        Ok(Outcome::Activated)
    }

    fn activation_blocked(
        &mut self,
        target: &AccessibilityActivationTarget,
    ) -> Result<Option<AccessibilityActivationResult>, AppError> {
        use AccessibilityActivationResult as Outcome;
        let current = self
            .accessibility
            .as_ref()
            .and_then(|revision| revision.id.get());
        if current != Some(&target.frame) || self.queue.1.get() {
            return Ok(Some(Outcome::Rejected));
        }
        if self
            .editors
            .values()
            .any(|editor| editor.composition().is_some())
        {
            return Ok(Some(Outcome::BlockedByComposition));
        }
        if self.pressed.is_some()
            || self.range_drag.is_some()
            || self.has_captured_drag()
            || self.text_drag.is_some()
            || self.modal_outside
        {
            return Ok(Some(Outcome::BlockedByPointer));
        }
        if !self.queue.0.borrow().is_empty() {
            self.flush()?;
            return Ok(Some(Outcome::Rebuilt));
        }
        if !self.queue.2.borrow().is_empty() {
            return Ok(Some(Outcome::Rejected));
        }
        let offered = self
            .accessibility_frame()
            .is_some_and(|frame| frame.press_binding(target.id).is_some());
        Ok((!offered).then_some(Outcome::Rejected))
    }
}
