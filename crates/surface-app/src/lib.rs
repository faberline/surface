//! Platform-independent Rust application state and input routing.
mod accessibility;
#[cfg(feature = "bridge")]
mod bridge;
mod captured_drag;
mod context;
mod controls;
mod cursor;
#[cfg(feature = "devtools")]
mod devtools;
mod display_contrast;
mod drag_motion;
mod editing;
mod editing_runtime;
mod focus_scroll;
mod frame_input;
mod frame_paint;
mod keys;
mod motion;
#[cfg(feature = "devtools")]
mod paint_export;
mod press_ripple;
mod scroll_control;
mod snapshot;
#[cfg(feature = "devtools")]
pub mod trace;
pub use accessibility::{
    AccessibilityActivationResult, AccessibilityActivationTarget, AccessibilityAdjustment,
    AccessibilityAdjustmentResult, AccessibilityAdjustmentTarget, AccessibilityFocusResult,
    AccessibilityFocusTarget, AccessibilityFrame, AccessibilityFrameId, AccessibilityNode,
    AccessibilityScrollDirection, AccessibilityScrollPageTarget, AccessibilityScrollResult,
    AccessibilityScrollTarget,
};
pub use captured_drag::{
    HorizontalDrag, HorizontalDragSession, PointDrag, PointDragLayout, PointDragSession,
    PointDragTarget, VerticalDrag, VerticalDragSession,
};
pub use context::PointerPoint;
pub use controls::{FocusNavigation, Roving};
pub use cursor::CursorStyle;
pub use display_contrast::{DisplayContrast, SystemColorPair, SystemContrastColors};
pub use editing::{
    utf16_to_utf8, utf8_to_utf16, Composition, EditKey, ImeEvent, TextEditor, TextOptions,
    TextSelection,
};
pub use editing_runtime::InputSnapshot;
pub use frame_input::{
    FrameInput, FramePress, FramePressPhase, InputAnimationUpdate, TouchContact,
};
pub use keys::{Key, KeyPress, Modifiers, Shortcut, TextAction};
pub use motion::{SpatialMotion, ToggleMotion, ToggleSpring};
#[cfg(feature = "devtools")]
pub use paint_export::{AttributedOp, PaintExport, PaintOwner, PAINT_SCHEMA};
pub use scroll_control::{
    HorizontalScroll, HorizontalScrollResponse, ScrollMetrics, ScrollTarget, VerticalScroll,
    VerticalScrollResponse,
};
pub use snapshot::{DomExport, EditingSnapshot, InteractionSnapshot};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};
use surface::{
    ComputedDom, DisplayList, DomError, ExtendedDisplayList, FillContour, LayoutEngine,
    LayoutError, MicroDom, Node, NodeId, Rect, Role, TextMeasurer,
};

type ContourPainter = Box<dyn Fn() -> FillContour>;

/// One view of application state. Messages return through `Application::update`.
pub struct View<M> {
    node: Node,
    children: Vec<Self>,
    paint_contour: Option<ContourPainter>,
    activate: Option<M>,
    suppress_repeated_activation: bool,
    input: Option<Rc<dyn Fn(String) -> M>>,
    text_options: TextOptions,
    value: Option<Rc<dyn Fn(f64) -> M>>,
    accessible_step: Option<Rc<dyn Fn(bool) -> M>>,
    roving: Option<controls::RovingGroup>,
    dismiss: Option<M>,
    dismiss_on_tab: bool,
    modal_focus_fallback: Option<String>,
    modal_initial_focus: Option<String>,
    cancel_within: Option<M>,
    key: Option<KeyHandler<M>>,
    focus_shortcuts: Vec<Shortcut<String>>,
    key_focus_navigation: Option<KeyFocusNavigationHandler<M>>,
    focus_navigation: Option<controls::FocusNavigationHandler<M>>,
    context: Option<context::ContextHandler<M>>,
    stick: bool,
    vertical_scroll: Option<scroll_control::VerticalScrollHandler<M>>,
    horizontal_scroll: Option<scroll_control::HorizontalScrollHandler<M>>,
    cursor: Option<CursorStyle>,
    drag_bindings: captured_drag::Bindings<M>,
    accessible_activation: bool,
    description: Option<String>,
    focus_after_removal: Vec<String>,
    protect_composition_on_press: bool,
    focus_on_pointer_release: bool,
    accessible_scroll: bool,
    accessible_scroll_pages: bool,
    interaction_suppression: Option<Rc<InteractionSuppression>>,
    visual_overlay: bool,
    modal_layer: Option<ModalLayer>,
}
struct InteractionSuppression {
    focus_fallback: Option<String>,
}

#[derive(Clone, Copy)]
enum ModalLayer {
    BackdropParent,
    Panel,
}

/// A key handler. `Some(message)` consumes the press, `None` passes it on.
type KeyHandler<M> = Rc<dyn Fn(&KeyPress) -> Option<M>>;
type KeyFocusNavigationHandler<M> = Rc<dyn Fn(&KeyPress) -> Option<FocusNavigation<M>>>;
impl<M> View<M> {
    pub fn new(node: Node) -> Self {
        Self {
            node,
            children: Vec::new(),
            paint_contour: None,
            activate: None,
            suppress_repeated_activation: false,
            input: None,
            text_options: TextOptions::default(),
            value: None,
            accessible_step: None,
            roving: None,
            dismiss: None,
            dismiss_on_tab: false,
            modal_focus_fallback: None,
            modal_initial_focus: None,
            cancel_within: None,
            key: None,
            focus_shortcuts: Vec::new(),
            key_focus_navigation: None,
            focus_navigation: None,
            context: None,
            stick: false,
            vertical_scroll: None,
            horizontal_scroll: None,
            cursor: None,
            drag_bindings: captured_drag::Bindings::default(),
            accessible_activation: false,
            description: None,
            focus_after_removal: Vec::new(),
            protect_composition_on_press: false,
            focus_on_pointer_release: false,
            accessible_scroll: false,
            accessible_scroll_pages: false,
            interaction_suppression: None,
            visual_overlay: false,
            modal_layer: None,
        }
    }
    /// Paint a node-local contour from retained state on each rendered frame.
    /// The contour uses the extended display list and does not change layout,
    /// hit testing, or accessibility. Update the state during animation and
    /// request the next frame. Rebuild the view if any semantic state changes.
    pub fn paint_fill_contour(mut self, paint: impl Fn() -> FillContour + 'static) -> Self {
        self.paint_contour = Some(Box::new(paint));
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Self>) -> Self {
        self.children = children.into_iter().collect();
        self
    }
    pub fn on_press(mut self, message: M) -> Self {
        self.activate = Some(message);
        self
    }
    /// Ignore held Enter or Space repeats for this control's default press.
    /// Focused key handlers and scoped shortcuts take precedence. Text
    /// inputs keep their normal key behavior.
    pub fn suppress_repeated_activation(mut self) -> Self {
        self.suppress_repeated_activation = true;
        self
    }
    /// When the focused control is removed or disabled, move focus to an
    /// enabled target in the same scope. This does not press the target.
    pub fn restore_focus_on_removal(mut self, target: impl Into<String>) -> Self {
        self.focus_after_removal = vec![target.into()];
        self
    }

    /// Give focusable views in this subtree a fallback when removed or disabled.
    /// The runtime tries the view's own target first. It then tries this
    /// fallback if the first target cannot receive focus in the current scope.
    pub fn restore_focus_in_subtree_on_removal(mut self, target: impl Into<String>) -> Self {
        fn apply<M>(view: &mut View<M>, target: &str) {
            if view.node.is_focusable() && !view.focus_after_removal.iter().any(|id| id == target) {
                view.focus_after_removal.push(target.to_owned());
            }
            for child in &mut view.children {
                apply(child, target);
            }
        }
        apply(&mut self, &target.into());
        self
    }

    /// Keep this subtree in layout and paint while excluding it from
    /// pointer input, focus traversal and accessibility actions.
    ///
    /// When a focused descendant becomes unavailable, move focus to the
    /// given stable ID if it can receive focus in the current scope.
    /// An outer suppression governs any nested subtrees it covers.
    pub fn without_interaction(mut self, focus_fallback: Option<&str>) -> Self {
        self.interaction_suppression = Some(Rc::new(InteractionSuppression {
            focus_fallback: focus_fallback.map(str::to_owned),
        }));
        self
    }
    /// Paint this subtree above ordinary content and below modal paint layers.
    /// The overlay and its descendants stay out of input, focus and
    /// accessibility. This does not open a focus scope.
    pub fn visual_overlay(mut self) -> Self {
        self.visual_overlay = true;
        self.without_interaction(None)
    }
    /// Track horizontal motion after the pointer passes the drag threshold.
    pub fn on_horizontal_drag(
        self,
        session: HorizontalDragSession,
        update: impl Fn(HorizontalDrag) -> M + 'static,
    ) -> Self {
        self.on_horizontal_drag_with_bounds(session, move |drag, _| update(drag))
    }
    /// Track a horizontal drag and report the region's rendered bounds at
    /// the primary press. The original unclipped bounds stay fixed across
    /// moves, rebuilds, and release or cancellation. Use them when the
    /// gesture's anchor depends on the measured size of a moving region.
    pub fn on_horizontal_drag_with_bounds(
        mut self,
        session: HorizontalDragSession,
        update: impl Fn(HorizontalDrag, Rect) -> M + 'static,
    ) -> Self {
        self.drag_bindings.set_horizontal(session, update);
        self
    }

    /// Track vertical motion after the pointer passes the drag threshold.
    /// Use this on a dedicated handle to leave content scrolling separate.
    pub fn on_vertical_drag(
        self,
        session: VerticalDragSession,
        update: impl Fn(VerticalDrag) -> M + 'static,
    ) -> Self {
        self.on_vertical_drag_with_bounds(session, move |drag, _| update(drag))
    }

    /// Track a vertical drag and report the region's rendered bounds at the
    /// primary press. The original unclipped bounds stay fixed across moves,
    /// rebuilds, and release or cancellation. When the region has both bindings,
    /// the dominant axis at the drag threshold chooses one gesture.
    pub fn on_vertical_drag_with_bounds(
        mut self,
        session: VerticalDragSession,
        update: impl Fn(VerticalDrag, Rect) -> M + 'static,
    ) -> Self {
        self.drag_bindings.set_vertical(session, update);
        self
    }

    /// Capture a primary pointer's two coordinates in this view. A tap
    /// releases over the same visible owner. A drag beyond the threshold
    /// keeps the down-time bounds and can continue outside the view.
    /// This binding owns physical release and takes precedence over axis
    /// bindings on the same view. Point events preserve current focus.
    /// Returning `None` sends no message.
    pub fn on_point_drag_with_bounds(
        mut self,
        session: PointDragSession,
        update: impl Fn(PointDrag, Rect) -> Option<M> + 'static,
    ) -> Self {
        self.drag_bindings
            .set_point(session, None, move |event, bounds, _| update(event, bounds));
        self
    }

    /// Capture a point drag and measure selected stable IDs after each render.
    /// The update from pointer down receives the original owner bounds and
    /// the latest successfully rendered geometry. This covers a scroll between
    /// the last move and release, including when a rebuild has retired the
    /// live layout. Retain the session and change it to replace the operation.
    pub fn on_point_drag_with_layout(
        mut self,
        session: PointDragSession,
        ids: Rc<[String]>,
        update: impl Fn(PointDrag, Rect, &PointDragLayout) -> Option<M> + 'static,
    ) -> Self {
        self.drag_bindings
            .set_point(session, Some(ids), move |event, bounds, layout| {
                update(event, bounds, layout?)
            });
        self
    }

    /// Report whether this control opens or collapses a panel.
    pub fn disclosure_expanded(mut self, expanded: bool) -> Self {
        self.node = self.node.disclosure_expanded(expanded);
        self
    }
    /// Mark this button as a menu trigger and report whether its menu is open.
    pub fn menu_expanded(mut self, expanded: bool) -> Self {
        self.node = self.node.menu_expanded(expanded);
        self
    }
    /// Report current-page state without making the control a toggle.
    pub fn current_page(mut self, current: bool) -> Self {
        self.node = self.node.current_page(current);
        self
    }
    /// Let a platform accessibility action invoke this view's press binding.
    pub fn accessible_activation(mut self) -> Self {
        self.accessible_activation = true;
        self
    }
    /// Use a visible tooltip in the current scope to describe this view.
    /// The target ID refers to a `Role::Tooltip` view.
    pub fn described_by(mut self, target: impl Into<String>) -> Self {
        self.description = Some(target.into());
        self
    }
    /// Let a platform accessibility action reveal this view without pressing it.
    pub fn accessibility_scroll_into_view(mut self) -> Self {
        self.accessible_scroll = true;
        self
    }
    /// Let a platform accessibility action scroll this viewport by one page.
    pub fn accessibility_scroll_pages(mut self) -> Self {
        self.accessible_scroll_pages = true;
        self
    }
    /// Leave the current editor and its pending IME replacement in place
    /// when primary or secondary click, Tab navigation, or normal
    /// activation would enter this control during composition.
    ///
    /// Use this on controls that hide the editor. A press is ignored until
    /// the user finishes or cancels composition. If layout is invalidated,
    /// pointer presses wait for fresh geometry while an enabled protected
    /// control is in scope. Accessibility actions already protect
    /// composition without this option.
    pub fn protect_composition_on_press(mut self) -> Self {
        self.protect_composition_on_press = true;
        self
    }
    /// Keep focus in the current control until a pointer release activates
    /// this view. A point-drag view with this option takes focus on an accepted
    /// tap. A completed drag keeps its previous focus.
    pub fn focus_on_pointer_release(mut self) -> Self {
        self.focus_on_pointer_release = true;
        self
    }
    pub fn on_input(mut self, handler: impl Fn(String) -> M + 'static) -> Self {
        self.input = Some(Rc::new(handler));
        self
    }
    pub fn text_options(mut self, options: TextOptions) -> Self {
        self.text_options = options;
        self
    }
    /// Bind a slider value or an opted-in adjustable Group.
    pub fn on_value(mut self, handler: impl Fn(f64) -> M + 'static) -> Self {
        self.value = Some(Rc::new(handler));
        self
    }
    /// Handle native Increment (`true`) or Decrement (`false`) directly.
    ///
    /// Use this on an adjustable Group with [Self::on_value] to allow custom
    /// steps at its bounds. A current numeric value is required. SetValue
    /// still uses the authored numeric range and [Self::on_value].
    pub fn on_accessible_step(mut self, handler: impl Fn(bool) -> M + 'static) -> Self {
        self.accessible_step = Some(Rc::new(handler));
        self
    }
    pub fn roving(mut self, options: Roving) -> Self {
        self.roving = Some(controls::RovingGroup {
            options,
            first_enabled_on_entry: false,
            retain_focus: false,
            all_arrows: false,
            reveal_after_rebuild: false,
        });
        self
    }

    /// Enter this group at its first enabled tab stop in logical order.
    /// Either Tab direction enters at the same stop. The current member
    /// keeps the group's tab stop until focus moves out of the group.
    pub fn roving_first(mut self, options: Roving) -> Self {
        self.roving = Some(controls::RovingGroup {
            options,
            first_enabled_on_entry: true,
            retain_focus: false,
            all_arrows: false,
            reveal_after_rebuild: false,
        });
        self
    }

    /// Keep focus in this named roving group during a rebuild. If a focused
    /// member is removed or disabled, try its explicit fallback, then the
    /// selected or first enabled member of this group. A previously visible
    /// member that moves in the group's order is revealed again.
    pub fn roving_retained(mut self, options: Roving) -> Self {
        self.roving = Some(controls::RovingGroup {
            options,
            first_enabled_on_entry: false,
            retain_focus: true,
            all_arrows: false,
            reveal_after_rebuild: false,
        });
        self
    }

    /// Retain focus as with [`Self::roving_retained`] and keep a fully visible
    /// focused item in view when rows resize in flow. Once the user scrolls the
    /// item out of view, a layout rebuild alone leaves it there.
    pub fn roving_retained_layout(mut self, options: Roving) -> Self {
        self.roving = Some(controls::RovingGroup {
            options,
            first_enabled_on_entry: false,
            retain_focus: true,
            all_arrows: false,
            reveal_after_rebuild: true,
        });
        self
    }

    /// Give this named group one Tab stop at its first enabled member.
    /// Right and Down advance in authored order. Left and Up go back.
    /// Arrows wrap and move focus without activating a control. The order
    /// applies in either layout direction. Focus survives rebuilds as with
    /// [`Self::roving_retained`].
    pub fn roving_all_arrows(mut self) -> Self {
        self.roving = Some(controls::RovingGroup {
            options: Roving::vertical(false),
            first_enabled_on_entry: true,
            retain_focus: true,
            all_arrows: true,
            reveal_after_rebuild: false,
        });
        self
    }
    /// Handle key presses while this node has focus. The handler runs
    /// before the field's own key handling. Return `Some(message)` to
    /// consume the press: the field does not see it, and `message` runs.
    /// Return `None` to pass it to the field unchanged. The handler is not
    /// called during an IME composition. See `KeyPress`.
    pub fn on_key(mut self, handler: impl Fn(&KeyPress) -> Option<M> + 'static) -> Self {
        self.key = Some(Rc::new(handler));
        self
    }

    /// Move focus to a stable ID in this subtree when the chord is pressed.
    /// The focused node must be inside the subtree and the active modal.
    /// Text inputs and pending IME compositions keep their own keys. A
    /// focused `on_key` handler can consume the press first. An eligible
    /// repeated press is consumed without moving focus.
    pub fn focus_shortcut_within(
        mut self,
        key: Key,
        modifiers: Modifiers,
        target: impl Into<String>,
    ) -> Self {
        self.focus_shortcuts
            .push(Shortcut::new(key, modifiers, target.into()));
        self
    }

    /// Handle a key press from this focused non-editor control. The message
    /// may mount the named target. The runtime then focuses that target
    /// within the active scope without activating it.
    ///
    /// A focused `on_key` handler runs first. IME composition keeps its keys.
    /// If a repeated press matches, consume it without running the message.
    pub fn on_key_focus_navigation(
        mut self,
        handler: impl Fn(&KeyPress) -> Option<FocusNavigation<M>> + 'static,
    ) -> Self {
        self.key_focus_navigation = Some(Rc::new(handler));
        self
    }

    /// Handle navigation from this focused control. The message may mount
    /// the named target. The runtime moves focus after the resulting rebuild.
    pub fn on_focus_navigation(
        self,
        handler: impl Fn(EditKey) -> Option<FocusNavigation<M>> + 'static,
    ) -> Self {
        self.on_focus_navigation_with_extend(move |key, _| handler(key))
    }
    /// Handle a navigation key and its Shift modifier. The target can
    /// appear in the view after the message runs.
    pub fn on_focus_navigation_with_extend(
        mut self,
        handler: impl Fn(EditKey, bool) -> Option<FocusNavigation<M>> + 'static,
    ) -> Self {
        self.focus_navigation = Some(Rc::new(handler));
        self
    }
    /// Handle Escape when focus is in this subtree without opening a modal.
    /// A captured drag, IME composition or active modal takes precedence.
    pub fn on_cancel_within(mut self, message: M) -> Self {
        self.cancel_within = Some(message);
        self
    }
    /// Handle a secondary press (right click, or Control+click on macOS)
    /// on this node or any descendant that has no handler of its own. The
    /// handler gets the point in logical window pixels and returns the
    /// message to post, usually one that opens a menu there. A secondary
    /// press does not focus, press or move a text cursor. The web host
    /// sends no secondary press. See `docs/context-menu.md`.
    pub fn on_context_menu(mut self, handler: impl Fn(PointerPoint) -> M + 'static) -> Self {
        self.context = Some(Rc::new(handler));
        self
    }
    /// Keep a scroll container pinned to its bottom while it is at the bottom.
    /// It follows new content, stops when the user scrolls up (or a focus
    /// reveal moves it), and resumes when it is back at the bottom. The node
    /// needs `clip(true)` and a stable id. Without `clip` the flag is ignored.
    pub fn stick_to_bottom(mut self, on: bool) -> Self {
        self.stick = on;
        self
    }

    /// Let a clipped view with a stable ID take part of a vertical wheel or
    /// trackpad scroll. Return `None` to leave the whole delta with the view.
    /// Only the enabled scroll target in the current interaction scope receives
    /// the hook. It runs for nonzero finite vertical deltas, even at either
    /// end of the scroll range. Nested scroll targets do not bubble the event.
    pub fn on_vertical_scroll(
        self,
        handler: impl Fn(VerticalScroll) -> Option<VerticalScrollResponse<M>> + 'static,
    ) -> Self {
        self.on_vertical_scroll_with_focus(move |scroll, _| handler(scroll))
    }

    /// Like [`Self::on_vertical_scroll`], with the stable ID of the current
    /// focus target when there is one. The focus target may be outside the
    /// scroll view, for example in a toolbar beside it.
    pub fn on_vertical_scroll_with_focus(
        mut self,
        handler: impl Fn(VerticalScroll, Option<&str>) -> Option<VerticalScrollResponse<M>> + 'static,
    ) -> Self {
        self.vertical_scroll = Some(Rc::new(handler));
        self
    }

    /// Handle the horizontal wheel delta over this clipped view with a stable ID.
    /// The handler also receives the current focus ID, if there is one.
    /// Nested clip masks do not take horizontal input away from this handler.
    /// Vertical input can still reach a scrollable child or a clipped ancestor.
    /// Attach a vertical hook here to handle both axes on this view.
    /// Return `None` to use the runtime's ordinary horizontal scrolling.
    pub fn on_horizontal_scroll(
        mut self,
        handler: impl Fn(HorizontalScroll, Option<&str>) -> Option<HorizontalScrollResponse<M>>
            + 'static,
    ) -> Self {
        self.horizontal_scroll = Some(Rc::new(handler));
        self
    }
    /// The mouse cursor over this node. It does not change the DOM, layout
    /// or display list. Read it with `Runtime::cursor_at`.
    pub fn cursor(mut self, cursor: CursorStyle) -> Self {
        self.cursor = Some(cursor);
        self
    }
    /// Trap focus in this panel, dismiss on Escape/outside click, then restore focus.
    pub fn modal(mut self, dismiss: M) -> Self {
        self.dismiss = Some(dismiss);
        self
    }
    /// Open a modal without a backdrop, such as a fixed menu. This panel
    /// and its descendants form one layer above background content.
    pub fn modal_overlay(mut self, dismiss: M) -> Self {
        self.dismiss = Some(dismiss);
        self.modal_layer = Some(ModalLayer::Panel);
        self
    }
    /// Let Tab and Shift+Tab dismiss this modal menu. The same key continues
    /// traversal from the restored opener when only this scope closes.
    pub fn dismiss_modal_on_tab(mut self) -> Self {
        self.dismiss_on_tab = true;
        self
    }
    /// Preserve focus when this modal closes and its opener is gone. First
    /// try the previously focused control if it still exists in the scope
    /// below the modal, then try the given stable ID in that scope.
    pub fn modal_focus_fallback(mut self, target: impl Into<String>) -> Self {
        self.modal_focus_fallback = Some(target.into());
        self
    }
    /// Prefer a focusable descendant with this stable ID on modal entry.
    /// If it is unavailable, use the first reachable control.
    pub fn modal_initial_focus(mut self, target: impl Into<String>) -> Self {
        self.modal_initial_focus = Some(target.into());
        self
    }
    /// Wrap this modal panel in its backdrop. Keep owned popups inside the
    /// panel's view subtree so the runtime can resolve their current owner.
    /// The panel needs a stable ID for focus and dismissal.
    pub fn modal_with_backdrop(mut self, backdrop: Node, dismiss: M) -> Self {
        self.dismiss = Some(dismiss);
        self.modal_layer = Some(ModalLayer::BackdropParent);
        Self::new(backdrop).children([self])
    }
}
/// The app's answer to a close request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseDecision {
    Allow,
    Cancel,
}
/// Time and the last valid geometry available before drawing a frame.
pub struct AnimationFrame<'a> {
    pub now_ms: Option<f64>,
    pub reduced_motion: bool,
    pub dom: &'a MicroDom,
    pub layout: Option<&'a ComputedDom>,
}

/// What an application needs after advancing its animation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimationUpdate {
    pub rebuild: bool,
    pub request_next: bool,
}

pub trait Application: 'static {
    type Message: Clone + 'static;
    fn title(&self) -> &str {
        "Surface"
    }
    fn update(&mut self, message: Self::Message);
    /// Give the application its own UI-thread queue before its first view.
    /// Store a copy to enqueue work such as scroll requests from a frame hook.
    fn attach_queue(&mut self, _queue: Queue<Self::Message>) {}
    /// Called on the event loop after a `WakeHandle::wake` from any thread.
    /// Poll workers, channels and shared state here. The default does
    /// nothing. Many wakes can coalesce into one call, so drain everything
    /// that is ready. The runtime rebuilds the view after this returns.
    fn wake(&mut self) {}
    /// Called once when the user asks to close the window. Return
    /// `CloseDecision::Cancel` to keep the window, for example while a dialog
    /// is open or after a failed save. The default allows the close.
    fn close_requested(&mut self) -> CloseDecision {
        CloseDecision::Allow
    }

    /// Text scale applied during layout. A nonfinite or nonpositive value uses 1.
    fn text_scale(&self) -> f32 {
        1.
    }
    /// Report a change of the logical viewport before layout.
    /// Return true when the application's view must be rebuilt.
    fn viewport_changed(&mut self, _width: f32, _height: f32) -> bool {
        false
    }
    /// Report a host display-contrast preference. Store the presentation
    /// preference and return true if the view must be rebuilt. Keep stable
    /// control IDs and authored editor values across appearance changes to
    /// retain focus, drafts and composition.
    ///
    /// If the new view fails validation, the runtime calls this hook again
    /// with the last accepted preference. This hook should only change
    /// appearance state. `Normal` is the initial preference.
    fn display_contrast_changed(&mut self, _contrast: DisplayContrast) -> bool {
        false
    }
    /// Advance frame-driven application motion.
    fn animation_frame(&mut self, _frame: AnimationFrame<'_>) -> AnimationUpdate {
        AnimationUpdate::default()
    }
    /// Ask the host for a frame when a pointer moves over passive content.
    /// Override this together with [`Self::animation_frame_with_input`] when
    /// the current view uses pointer or touch position to draw.
    fn observes_frame_input(&self) -> bool {
        false
    }
    /// Advance motion with the latest input, including positions over
    /// non-focusable content. By default this calls [`Self::animation_frame`].
    fn animation_frame_with_input(
        &mut self,
        frame: AnimationFrame<'_>,
        _input: FrameInput<'_>,
    ) -> InputAnimationUpdate {
        InputAnimationUpdate {
            animation: self.animation_frame(frame),
            suppress_touch_activation: false,
        }
    }
    /// Observe the stable ID of the accepted primary mouse or touch press.
    /// It is present while the press is pending and clears after release,
    /// cancellation, or loss of the target. Events completed before the
    /// frame leave no pending press. The default calls
    /// [`Self::animation_frame_with_input`].
    fn animation_frame_with_press(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
        _pressed: Option<&str>,
    ) -> InputAnimationUpdate {
        self.animation_frame_with_input(frame, input)
    }
    /// Also observe the accepted down position, last press outcome, and live
    /// hovered target while [`Self::observes_frame_input`] returns true.
    /// Hover uses the current layout and active input scope.
    /// A release or cancellation is reported once, including a complete tap
    /// between two frames. If several presses arrive before a frame, only the
    /// latest is retained. The default keeps the existing pressed-ID callback.
    fn animation_frame_with_press_detail(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
        pressed: Option<&str>,
        _press: Option<FramePress<'_>>,
        _hovered: Option<&str>,
    ) -> InputAnimationUpdate {
        self.animation_frame_with_press(frame, input, pressed)
    }
    /// Handle Escape for transient application UI outside modal scopes.
    /// Return true when the runtime should rebuild the view.
    fn cancel_transient(&mut self) -> bool {
        false
    }
    fn view(&self) -> View<Self::Message>;
    /// Global keyboard shortcuts, read each time a key arrives. A shortcut
    /// sends its message whichever node has focus. The runtime does not
    /// apply modal scope: return the table that fits the current state.
    /// The default has none. See `Shortcut` and `Runtime::shortcut`.
    fn shortcuts(&self) -> Vec<Shortcut<Self::Message>> {
        Vec::new()
    }
    /// A short text for a message in a devtools trace. The default returns
    /// `None`. The runtime calls it only while a trace recorder is attached
    /// (feature `devtools`).
    fn describe(&self, message: &Self::Message) -> Option<String> {
        let _ = message;
        None
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    PointerDown {
        x: f32,
        y: f32,
    },
    /// A secondary press at a window point. See `View::on_context_menu`.
    SecondaryDown {
        x: f32,
        y: f32,
    },
    PointerUp {
        x: f32,
        y: f32,
    },
    FocusNext {
        reverse: bool,
    },
    Activate,
    Text(String),
    Backspace,
    PointerMove {
        x: f32,
        y: f32,
    },
    PointerLeave,
    Edit {
        key: EditKey,
        extend: bool,
        by_word: bool,
    },
    Ime(ImeEvent),
    ImeFor {
        id: String,
        event: ImeEvent,
    },
    InputValue {
        value: String,
        anchor: usize,
        cursor: usize,
    },
    Select {
        anchor: usize,
        cursor: usize,
    },
    Blur,
    Cancel,
    Scroll {
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
    },
    SetValue {
        id: String,
        value: f64,
    },
}
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Dom(#[from] DomError),
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error("interactive nodes need a stable ID: {0}")]
    MissingId(String),
    #[error("no enabled control with ID {0}")]
    MissingControl(String),
    #[error("writable text input needs an on_input handler: {0}")]
    MissingInputHandler(String),
    #[error("slider needs a finite range and on_value handler: {0}")]
    InvalidRange(String),
}
struct Binding<M> {
    activate: Option<M>,
    suppress_repeated_activation: bool,
    input: Option<Rc<dyn Fn(String) -> M>>,
    text_options: TextOptions,
    value: Option<Rc<dyn Fn(f64) -> M>>,
    accessible_step: Option<Rc<dyn Fn(bool) -> M>>,
    roving: Option<controls::RovingGroup>,
    dismiss: Option<M>,
    dismiss_on_tab: bool,
    modal_focus_fallback: Option<String>,
    modal_initial_focus: Option<String>,
    cancel_within: Option<M>,
    key: Option<KeyHandler<M>>,
    focus_shortcuts: Vec<Shortcut<String>>,
    key_focus_navigation: Option<KeyFocusNavigationHandler<M>>,
    focus_navigation: Option<controls::FocusNavigationHandler<M>>,
    context: Option<context::ContextHandler<M>>,
    stick: bool,
    vertical_scroll: Option<scroll_control::VerticalScrollHandler<M>>,
    horizontal_scroll: Option<scroll_control::HorizontalScrollHandler<M>>,
    cursor: Option<CursorStyle>,
    drag_bindings: captured_drag::Bindings<M>,
    accessible_activation: bool,
    description: Option<String>,
    focus_after_removal: Vec<String>,
    protect_composition_on_press: bool,
    focus_on_pointer_release: bool,
    accessible_scroll: bool,
    accessible_scroll_pages: bool,
    interaction_suppression: Option<Rc<InteractionSuppression>>,
    visual_overlay: bool,
    modal_layer: Option<ModalLayer>,
}

impl<M> Binding<M> {
    fn accepts_interaction(&self) -> bool {
        self.interaction_suppression.is_none()
    }
}

/// Messages posted from inside `update` or from other code on the UI thread.
/// Clone it freely. It is not `Send`; use `WakeHandle` across threads.
pub struct Queue<M>(
    Rc<RefCell<VecDeque<M>>>,
    Rc<Cell<bool>>,
    scroll_control::Pending,
);
impl<M> Clone for Queue<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), self.2.clone())
    }
}
impl<M> Queue<M> {
    /// Queue a message. It runs after the current `update` returns, in
    /// order, and never re-enters `update`. Outside an update it runs on the
    /// next `apply`, `wake` or `flush`.
    pub fn post(&self, message: M) {
        self.0.borrow_mut().push_back(message);
    }
    /// Ask the host to close the window. It does not call `close_requested`:
    /// the app already decided. The host ends the event loop after the
    /// current update, and the app state is then dropped normally. Called
    /// from inside `close_requested`, it takes effect at once, even if that
    /// hook returns `Cancel`.
    pub fn close(&self) {
        self.1.set(true);
    }
}

struct PreparedView<M> {
    dom: MicroDom,
    bindings: BTreeMap<NodeId, Binding<M>>,
    contour_painters: Vec<(NodeId, ContourPainter)>,
}

pub struct Runtime<A: Application> {
    queue: Queue<A::Message>,
    app: A,
    dom: MicroDom,
    bindings: BTreeMap<NodeId, Binding<A::Message>>,
    contour_painters: Vec<(NodeId, ContourPainter)>,
    layout: LayoutEngine,
    computed: Option<ComputedDom>,
    focused: Option<String>,
    pending_roving_focus: Option<String>,
    pressed: Option<String>,
    hovered: Option<String>,
    pointer_position: Option<(f32, f32)>,
    press_observation: frame_input::PressObservation,
    press_ripple: press_ripple::PressRipple,
    touch: Option<frame_input::ActiveTouch>,
    touch_cancelled: bool,
    scroll: BTreeMap<String, (f32, f32)>,
    scroll_state: scroll_control::State,
    editors: BTreeMap<String, TextEditor>,
    text_geometry: BTreeMap<String, editing_runtime::EditingGeometry>,
    ime_area: Option<surface::Rect>,
    text_drag: Option<String>,
    scopes: Vec<controls::FocusScope>,
    range_drag: Option<controls::RangeDrag>,
    captured_drag: Option<captured_drag::Tracking<A::Message>>,
    pointer_geometry_current: bool,
    modal_layers: Vec<(Option<NodeId>, NodeId)>,
    accessibility: Option<accessibility::Revision>,
    pending_accessibility_scroll: Option<accessibility::PendingScroll>,
    app_viewport: Option<(f32, f32)>,
    viewport_rebuild_pending: bool,
    display_contrast: DisplayContrast,
    animation_rebuild_pending: bool,
    animation_request_next: bool,
    layout_text_scale: f32,
    modal_outside: bool,
    layout_focus: Option<String>,
    layout_viewport: Option<(f32, f32)>,
    motion: motion::Motion,
    #[cfg(feature = "devtools")]
    trace: Option<trace::Recorder>,
    #[cfg(feature = "devtools")]
    host_methods: &'static [&'static str],
    #[cfg(feature = "devtools")]
    paint_frame: Option<paint_export::PaintFrame>,
}
impl<A: Application> Runtime<A> {
    pub fn new(app: A) -> Result<Self, AppError> {
        let mut runtime = Self {
            queue: Queue(Rc::default(), Rc::default(), Rc::default()),
            app,
            dom: MicroDom::new(Node::new(Role::Window, "Surface")),
            bindings: BTreeMap::new(),
            contour_painters: Vec::new(),
            layout: LayoutEngine::default(),
            computed: None,
            focused: None,
            pending_roving_focus: None,
            pressed: None,
            hovered: None,
            pointer_position: None,
            press_observation: frame_input::PressObservation::default(),
            press_ripple: press_ripple::PressRipple::default(),
            touch: None,
            touch_cancelled: false,
            scroll: BTreeMap::new(),
            scroll_state: scroll_control::State::default(),
            editors: BTreeMap::new(),
            text_geometry: BTreeMap::new(),
            ime_area: None,
            text_drag: None,
            scopes: Vec::new(),
            range_drag: None,
            captured_drag: None,
            pointer_geometry_current: false,
            modal_layers: Vec::new(),
            accessibility: None,
            pending_accessibility_scroll: None,
            app_viewport: None,
            viewport_rebuild_pending: false,
            display_contrast: DisplayContrast::Normal,
            animation_rebuild_pending: false,
            animation_request_next: false,
            layout_text_scale: 1.,
            modal_outside: false,
            layout_focus: None,
            layout_viewport: None,
            motion: motion::Motion::default(),
            #[cfg(feature = "devtools")]
            trace: None,
            #[cfg(feature = "devtools")]
            host_methods: &[],
            #[cfg(feature = "devtools")]
            paint_frame: None,
        };
        runtime.app.attach_queue(runtime.queue.clone());
        runtime.rebuild()?;
        Ok(runtime)
    }
    pub fn app(&self) -> &A {
        &self.app
    }
    pub fn dom(&self) -> &MicroDom {
        &self.dom
    }
    pub fn computed(&self) -> Option<&ComputedDom> {
        self.computed.as_ref()
    }
    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// Record a logical viewport change. This runs before frame layout.
    pub fn set_viewport(&mut self, width: f32, height: f32) -> Result<bool, AppError> {
        if !width.is_finite() || !height.is_finite() || width <= 0. || height <= 0. {
            return Ok(false);
        }
        let mut changed = false;
        if self.app_viewport != Some((width, height)) {
            changed = self.pointer_geometry_changed()?;
            self.viewport_rebuild_pending |= self.app.viewport_changed(width, height);
            self.app_viewport = Some((width, height));
        }
        if self.viewport_rebuild_pending {
            self.rebuild()?;
            self.viewport_rebuild_pending = false;
            changed = true;
        }
        Ok(changed)
    }

    /// The most recently accepted host preference, even for an app that
    /// chooses to keep its view. Starts at `Normal`.
    pub fn display_contrast(&self) -> DisplayContrast {
        self.display_contrast
    }

    /// Report a changed preference and rebuild only if the app asks for it.
    /// Return true when a new frame is needed, either for a rebuilt view or
    /// for a changed extended selection palette. Repeating an accepted
    /// preference does no work. If the new view fails
    /// validation, keep the previous view and preference so a host may retry.
    pub fn set_display_contrast(&mut self, contrast: DisplayContrast) -> Result<bool, AppError> {
        let previous = self.display_contrast;
        if contrast == previous {
            return Ok(false);
        }
        if !self.app.display_contrast_changed(contrast) {
            let redraw =
                frame_paint::selection_colors(previous) != frame_paint::selection_colors(contrast);
            self.display_contrast = contrast;
            return Ok(redraw);
        }
        let prepared = match self.prepare_view() {
            Ok(prepared) => prepared,
            Err(error) => {
                self.app.display_contrast_changed(previous);
                return Err(error);
            }
        };
        self.display_contrast = contrast;
        self.rebuild_prepared(Some(prepared))?;
        Ok(true)
    }

    /// Cancel a pointer sequence without invoking a destination or dismissal.
    pub fn cancel_pointer(&mut self) -> Result<bool, AppError> {
        let touch_changed = self.cancel_touch_contact();
        let changed = self.cancel_press_ripple()
            || touch_changed
            || self.pressed.is_some()
            || self.hovered.is_some()
            || self.range_drag.is_some()
            || self.text_drag.is_some()
            || self.modal_outside;
        self.clear_modal_pointer();
        self.cancel_drag().map(|drag| changed || drag)
    }

    /// Invalidate pointer geometry before the next resize or scale redraw.
    pub fn pointer_geometry_changed(&mut self) -> Result<bool, AppError> {
        self.pointer_geometry_current = false;
        self.discard_wheel_burst();
        let ripple_changed = self.cancel_press_ripple();
        let pointer_changed = self.pointer_position.take().is_some();
        let cancelled = if self.touch.is_some() {
            self.cancel_pointer()?
        } else {
            self.cancel_drag()?
        };
        Ok(cancelled || ripple_changed || (pointer_changed && self.app.observes_frame_input()))
    }

    fn has_captured_drag(&self) -> bool {
        self.captured_drag.is_some()
    }

    fn drag_scope(&self) -> Option<&str> {
        self.active_scope_root()
            .and_then(|id| self.dom.get(id).ok()?.stable_id())
    }

    fn protects_composition(&self, id: NodeId) -> bool {
        self.bindings
            .get(&id)
            .is_some_and(|b| b.protect_composition_on_press)
            && self.has_pending_composition()
    }
    fn has_pending_composition(&self) -> bool {
        self.editors
            .values()
            .any(|editor| editor.composition().is_some())
    }
    /// Whether a press at these logical coordinates is guarded while an
    /// IME replacement is pending. Hosts can consult this before a native
    /// focus transfer would end composition. This does not dispatch the press.
    pub fn pointer_preserves_composition(&self, x: f32, y: f32) -> bool {
        let hit = self.hit(x, y);
        self.protects_pointer_composition(hit)
            || (self.has_pending_composition() && self.drag_candidate(hit, x, y).is_some())
    }
    fn protects_pointer_composition(&self, hit: Option<NodeId>) -> bool {
        if let Some(id) = hit {
            return self.protects_composition(id);
        }
        // IME events invalidate the old text layout and hit geometry. A
        // press arriving before redraw might be on a protected control.
        self.computed.is_none()
            && self.has_pending_composition()
            && self.bindings.iter().any(|(id, binding)| {
                (binding.protect_composition_on_press
                    && self.in_scope(*id)
                    && self.dom.get(*id).is_ok_and(|n| n.is_focusable()))
                    || (!binding.drag_bindings.is_empty() && self.in_drag_scope(*id))
            })
    }
    /// A handle for posting messages that run after the current update.
    pub fn queue(&self) -> Queue<A::Message> {
        self.queue.clone()
    }
    /// Apply a message outside input dispatch, then run queued messages and
    /// rebuild once. Used for timers, async results and tests.
    pub fn apply(&mut self, message: A::Message) -> Result<(), AppError> {
        self.queue.post(message);
        self.flush()
    }
    /// Run the app's `wake` hook, then queued messages, then rebuild once.
    /// The platform host calls this once per coalesced wake.
    pub fn wake(&mut self) -> Result<(), AppError> {
        self.app.wake();
        self.flush()
    }
    /// The host calls this when the user asks to close the window. Runs the
    /// app's `close_requested` hook once, then queued messages, then
    /// rebuilds. Returns `true` when the window should close now.
    pub fn request_close(&mut self) -> Result<bool, AppError> {
        let decision = self.app.close_requested();
        // Run what the hook posted and rebuild, also after a veto.
        self.flush()?;
        let closed_by_app = self.take_close();
        Ok(decision == CloseDecision::Allow || closed_by_app)
    }
    /// True once, after the app called `Queue::close`. The host checks it
    /// after each input, wake or flush.
    pub fn take_close(&mut self) -> bool {
        self.queue.1.replace(false)
    }
    /// Run queued messages in order, including ones posted by `update`
    /// while this runs, then rebuild once.
    pub fn flush(&mut self) -> Result<(), AppError> {
        let result = self.run_queue();
        #[cfg(feature = "devtools")]
        self.trace_error(&result);
        result
    }
    fn run_queue(&mut self) -> Result<(), AppError> {
        loop {
            let next = self.queue.0.borrow_mut().pop_front();
            let Some(message) = next else { break };
            #[cfg(feature = "devtools")]
            let start = self.trace_message(&message);
            self.app.update(message);
            #[cfg(feature = "devtools")]
            self.trace_update(start);
        }
        self.rebuild()
    }
    fn rebuild(&mut self) -> Result<(), AppError> {
        self.rebuild_prepared(None)
    }

    fn rebuild_prepared(
        &mut self,
        prepared: Option<PreparedView<A::Message>>,
    ) -> Result<(), AppError> {
        self.discard_wheel_burst();
        if !self.app.observes_frame_input() {
            self.press_observation.clear_application();
        }
        #[cfg(feature = "devtools")]
        let start = self.trace_now();
        self.accessibility = None;
        let result = prepared
            .map_or_else(|| self.prepare_view(), Ok)
            .map(|view| self.install_view(view));
        if result.is_ok() {
            self.accessibility = Some(accessibility::Revision::default());
            self.animation_rebuild_pending = false;
            self.reconcile_press_ripple();
        } else {
            self.cancel_press_ripple();
            self.discard_drag();
            if self.cancel_touch_contact() || self.active_scope_root().is_some() {
                self.clear_modal_pointer();
            }
        }
        #[cfg(feature = "devtools")]
        self.trace_span(
            start,
            trace::Data::Rebuild {
                nodes: self.dom.len() as u32,
            },
        );
        result?;
        self.reconcile_drag_binding()
    }
    fn prepare_view(&self) -> Result<PreparedView<A::Message>, AppError> {
        fn add<M>(
            view: View<M>,
            dom: &mut MicroDom,
            parent: Option<NodeId>,
            bindings: &mut BTreeMap<NodeId, Binding<M>>,
            contour_painters: &mut Vec<(NodeId, ContourPainter)>,
            disabled: bool,
            inherited_suppression: Option<&Rc<InteractionSuppression>>,
        ) -> Result<(), AppError> {
            let disabled = disabled || view.node.is_disabled();
            let suppression = inherited_suppression.or(view.interaction_suppression.as_ref());
            let node = view.node.disabled(disabled);
            if (node.is_focusable()
                || view.roving.is_some()
                || view.dismiss.is_some()
                || view.stick
                || view.vertical_scroll.is_some()
                || view.horizontal_scroll.is_some()
                || view.accessible_scroll
                || view.accessible_scroll_pages
                || !view.drag_bindings.is_empty())
                && node.stable_id().is_none()
            {
                return Err(AppError::MissingId(node.name().to_owned()));
            }
            if node.role() == Role::Slider
                && (node.range_value().is_none_or(|v| !v.is_valid())
                    || (!disabled && view.value.is_none()))
            {
                return Err(AppError::InvalidRange(
                    node.stable_id().unwrap_or(node.name()).to_owned(),
                ));
            }
            if node.accessibility_adjustable().is_some() && !disabled && view.value.is_none() {
                return Err(AppError::InvalidRange(
                    node.stable_id().unwrap_or(node.name()).to_owned(),
                ));
            }
            if node.role() == Role::TextInput
                && !disabled
                && !view.text_options.read_only
                && view.input.is_none()
            {
                return Err(AppError::MissingInputHandler(
                    node.stable_id().unwrap_or(node.name()).to_owned(),
                ));
            }
            let id = if let Some(parent) = parent {
                dom.append(parent, node)?
            } else {
                dom.root()
            };
            if let Some(paint) = view.paint_contour {
                contour_painters.push((id, paint));
            }
            bindings.insert(
                id,
                Binding {
                    activate: view.activate,
                    suppress_repeated_activation: view.suppress_repeated_activation,
                    input: view.input,
                    text_options: view.text_options,
                    value: view.value,
                    accessible_step: view.accessible_step,
                    roving: view.roving,
                    dismiss: view.dismiss,
                    dismiss_on_tab: view.dismiss_on_tab,
                    modal_focus_fallback: view.modal_focus_fallback,
                    modal_initial_focus: view.modal_initial_focus,
                    cancel_within: view.cancel_within,
                    key: view.key,
                    focus_shortcuts: view.focus_shortcuts,
                    key_focus_navigation: view.key_focus_navigation,
                    focus_navigation: view.focus_navigation,
                    context: view.context,
                    stick: view.stick,
                    vertical_scroll: view.vertical_scroll,
                    horizontal_scroll: view.horizontal_scroll,
                    cursor: view.cursor,
                    drag_bindings: view.drag_bindings,
                    accessible_activation: view.accessible_activation,
                    description: view.description,
                    focus_after_removal: view.focus_after_removal,
                    protect_composition_on_press: view.protect_composition_on_press,
                    focus_on_pointer_release: view.focus_on_pointer_release,
                    accessible_scroll: view.accessible_scroll,
                    accessible_scroll_pages: view.accessible_scroll_pages,
                    visual_overlay: view.visual_overlay,
                    modal_layer: view.modal_layer,
                    interaction_suppression: suppression.cloned(),
                },
            );
            for child in view.children {
                add(
                    child,
                    dom,
                    Some(id),
                    bindings,
                    contour_painters,
                    disabled,
                    suppression,
                )?;
            }
            Ok(())
        }
        let view = self.app.view();
        let mut dom = MicroDom::new(view.node.clone());
        let mut bindings = BTreeMap::new();
        let mut contour_painters = Vec::new();
        add(
            view,
            &mut dom,
            None,
            &mut bindings,
            &mut contour_painters,
            false,
            None,
        )?;
        for (key, (x, y)) in &self.scroll {
            if let Some(id) = dom.by_id(key) {
                let mut style = dom.get(id)?.style().clone();
                style.scroll_x = *x;
                style.scroll_y = *y;
                dom.set_style(id, style)?;
            }
        }
        Ok(PreparedView {
            dom,
            bindings,
            contour_painters,
        })
    }

    fn install_view(&mut self, prepared: PreparedView<A::Message>) {
        let focus_after_removal = self
            .focused_node()
            .filter(|id| self.in_scope(*id))
            .and_then(|id| self.bindings.get(&id))
            .map(|binding| binding.focus_after_removal.clone())
            .unwrap_or_default();
        let retained_roving_focus = self.capture_retained_roving_focus();
        let pressed_role = self
            .pressed
            .as_deref()
            .and_then(|key| self.dom.by_id(key))
            .and_then(|id| self.dom.get(id).ok())
            .map(Node::role);
        self.dom = prepared.dom;
        self.bindings = prepared.bindings;
        self.contour_painters = prepared.contour_painters;
        self.reconcile_editors();
        self.computed = None;
        self.reconcile_controls(focus_after_removal, retained_roving_focus);
        if self.focused.as_ref().is_some_and(|key| {
            self.dom
                .by_id(key)
                .and_then(|id| self.dom.get(id).ok())
                .is_none_or(|n| !n.is_focusable())
        }) {
            self.focused = None;
        }
        self.hovered = self.hovered.take().filter(|key| {
            self.dom
                .by_id(key)
                .is_some_and(|id| self.has_interaction_binding(id))
        });
        self.pressed = self.pressed.take().filter(|key| {
            self.dom.by_id(key).is_some_and(|id| {
                self.in_scope(id)
                    && self
                        .dom
                        .get(id)
                        .is_ok_and(|node| node.is_focusable() && Some(node.role()) == pressed_role)
            })
        });
    }

    pub fn render(
        &mut self,
        width: f32,
        height: f32,
        measurer: &mut impl TextMeasurer,
    ) -> Result<DisplayList, AppError> {
        self.render_with(width, height, measurer)
    }

    /// Render one frame with extended geometry and shadows. The extended
    /// renderer uses the host's selection palette in forced colors.
    pub fn render_extended(
        &mut self,
        width: f32,
        height: f32,
        measurer: &mut impl TextMeasurer,
    ) -> Result<ExtendedDisplayList, AppError> {
        self.render_with(width, height, measurer)
    }

    fn render_with<P: frame_paint::FramePaint>(
        &mut self,
        width: f32,
        height: f32,
        measurer: &mut impl TextMeasurer,
    ) -> Result<P, AppError> {
        let result = self.render_frame(width, height, measurer);
        self.pointer_geometry_current = result.is_ok();
        if result.is_err() {
            // Roll back a live gesture without replacing the render error.
            let _ = self.cancel_pointer();
            self.discard_wheel_burst();
            self.animation_request_next = false;
            self.computed = None;
            self.ime_area = None;
            self.pending_accessibility_scroll = None;
        }
        self.accessibility_rendered(result.is_ok());
        #[cfg(feature = "devtools")]
        self.trace_error(&result);
        result
    }
    fn render_frame<P: frame_paint::FramePaint>(
        &mut self,
        width: f32,
        height: f32,
        measurer: &mut impl TextMeasurer,
    ) -> Result<P, AppError> {
        self.set_viewport(width, height)?;
        if width.is_finite() && height.is_finite() && width > 0. && height > 0. {
            self.replay_wheel_burst(width, height, measurer)?;
            self.animate_application()?;
        }
        let text_scale = self.app.text_scale();
        let text_scale = if text_scale.is_finite() && text_scale > 0. {
            text_scale
        } else {
            1.
        };

        #[cfg(feature = "devtools")]
        let start = self.trace_now();
        let (mut display_dom, mut decorations, mut computed) =
            self.layout_editing(width, height, text_scale, measurer)?;
        if self.resolve_pending_roving_focus(&computed)
            && self
                .focused
                .as_ref()
                .is_some_and(|key| self.editors.contains_key(key))
        {
            (display_dom, decorations, computed) =
                self.layout_editing(width, height, text_scale, measurer)?;
        }
        if self.focused != self.layout_focus
            || self.layout_viewport != Some((width, height))
            || self.layout_text_scale != text_scale
        {
            if let Some(focused) = self.focused_node() {
                for ancestor in self.scroll_ancestors(focused) {
                    if self.reveal_in(focused, ancestor, &computed)? {
                        (display_dom, decorations, computed) =
                            self.layout_editing(width, height, text_scale, measurer)?;
                    }
                }
            }
            self.layout_focus.clone_from(&self.focused);
            self.layout_viewport = Some((width, height));
            self.layout_text_scale = text_scale;
        }
        if let Some(pending) = self.pending_accessibility_scroll.take() {
            match pending {
                accessibility::PendingScroll::IntoView(target) => {
                    if let Some(target) = self.check_accessibility_reveal(&target, &computed) {
                        for ancestor in self.scroll_ancestors(target) {
                            if !self.in_scope(ancestor) {
                                break;
                            }
                            if self.reveal_in(target, ancestor, &computed)? {
                                (display_dom, decorations, computed) =
                                    self.layout_editing(width, height, text_scale, measurer)?;
                            }
                        }
                    }
                }
                accessibility::PendingScroll::Page(target) => {
                    if self.apply_accessibility_scroll_page(&target, &computed)? {
                        (display_dom, decorations, computed) =
                            self.layout_editing(width, height, text_scale, measurer)?;
                    }
                }
            }
        }
        if self.apply_scroll_control(&computed)? {
            (display_dom, decorations, computed) =
                self.layout_editing(width, height, text_scale, measurer)?;
        }
        self.dom
            .apply_modal_layers(&mut computed.paint_order, &self.modal_layers);
        #[cfg(feature = "devtools")]
        self.trace_span(
            start,
            trace::Data::Layout {
                nodes: computed.nodes.len() as u32,
            },
        );
        #[cfg(feature = "devtools")]
        let start = self.trace_now();
        let focused = self.focused.as_ref().and_then(|id| self.dom.by_id(id));
        let interaction = surface::InteractionState {
            hovered: self.hovered.as_ref().and_then(|id| self.dom.by_id(id)),
            pressed: self.pressed.as_ref().and_then(|id| self.dom.by_id(id)),
        };
        self.present_motion(&mut display_dom, &computed, focused, interaction)?;
        self.present_press_ripple(&mut display_dom, &computed)?;
        for (id, paint) in &self.contour_painters {
            display_dom.set_presented_fill_contour(*id, paint())?;
        }
        let list = P::compile(
            &display_dom,
            &computed,
            focused,
            &decorations,
            interaction,
            frame_paint::selection_colors(self.display_contrast),
        );
        #[cfg(feature = "devtools")]
        self.trace_paint(start, list.len());
        #[cfg(feature = "devtools")]
        {
            // Moved, not copied: keeping the frame allocates nothing.
            self.paint_frame = Some(paint_export::PaintFrame {
                dom: display_dom,
                decorations,
                focused,
                interaction,
            });
        }
        self.refresh_point_drag_layout(&computed);
        self.computed = Some(computed);
        Ok(list)
    }
    fn hit(&self, x: f32, y: f32) -> Option<NodeId> {
        self.computed
            .as_ref()?
            .hit_test_where(&self.dom, x, y, |id| self.in_scope(id))
    }
    fn stable(&self, id: NodeId) -> Option<String> {
        self.dom.get(id).ok()?.stable_id().map(str::to_owned)
    }
    fn focused_node(&self) -> Option<NodeId> {
        self.dom.by_id(self.focused.as_deref()?)
    }
    fn cancel_within_focus(&self) -> Option<A::Message> {
        if self.active_scope_root().is_some() {
            return None;
        }
        let mut id = self.focused_node().filter(|id| self.in_scope(*id))?;
        loop {
            if let Some(message) = self.bindings.get(&id)?.cancel_within.clone() {
                return Some(message);
            }
            id = self.dom.get(id).ok()?.parent()?;
        }
    }
    fn message(&mut self, message: Option<A::Message>) -> Result<bool, AppError> {
        if let Some(message) = message {
            self.queue.post(message);
            self.flush()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// The host calls this for each key press before the field handles it.
    /// Returns `true` when a focused handler, scoped shortcut or repeat guard
    /// consumes the press. During IME composition, the field keeps its keys.
    pub fn key(&mut self, press: &KeyPress) -> Result<bool, AppError> {
        #[cfg(feature = "devtools")]
        let span = self.trace_input("Key", None, None, None);
        let result = self.key_press(press);
        #[cfg(feature = "devtools")]
        self.trace_input_end(span, &result);
        result
    }
    fn key_press(&mut self, press: &KeyPress) -> Result<bool, AppError> {
        // The composition owns the keys. Enter commits the candidate.
        if self.editors.values().any(|editor| editor.is_composing()) {
            return Ok(false);
        }
        let Some(id) = self.focused_node() else {
            return Ok(false);
        };
        if let Some(message) = self
            .bindings
            .get(&id)
            .and_then(|binding| binding.key.clone())
            .and_then(|handler| handler(press))
        {
            #[cfg(feature = "devtools")]
            if let (Some(recorder), Some(focused)) = (&mut self.trace, &self.focused) {
                recorder.set_source(focused, trace::Handler::Key);
            }
            return self.message(Some(message));
        }
        if self.navigate_key_focus(id, press)? || self.focus_shortcut(id, press) {
            return Ok(true);
        }
        let modifiers = press.modifiers;
        Ok(press.repeat
            && matches!(press.key, Key::Enter | Key::Space)
            && !modifiers.control
            && !modifiers.alt
            && !modifiers.meta
            && !self.has_pending_composition()
            && self.in_scope(id)
            && self.bindings.get(&id).is_some_and(|binding| {
                binding.suppress_repeated_activation && binding.activate.is_some()
            })
            && self
                .dom
                .get(id)
                .is_ok_and(|node| node.is_focusable() && node.role() != Role::TextInput))
    }

    fn navigate_key_focus(&mut self, id: NodeId, press: &KeyPress) -> Result<bool, AppError> {
        let Some(handler) = self
            .bindings
            .get(&id)
            .and_then(|binding| binding.key_focus_navigation.as_ref())
        else {
            return Ok(false);
        };
        if self.has_pending_composition()
            || !self.in_scope(id)
            || !self
                .dom
                .get(id)
                .is_ok_and(|node| node.is_focusable() && node.role() != Role::TextInput)
        {
            return Ok(false);
        }
        let Some(navigation) = handler(press) else {
            return Ok(false);
        };
        if press.repeat {
            return Ok(true);
        }
        #[cfg(feature = "devtools")]
        if let (Some(recorder), Some(focused)) = (&mut self.trace, &self.focused) {
            recorder.set_source(focused, trace::Handler::Key);
        }
        self.apply_focus_navigation(navigation)
    }

    fn focus_shortcut(&mut self, source: NodeId, press: &KeyPress) -> bool {
        let Some(target) = self.focus_shortcut_target(source, press) else {
            return false;
        };
        if !press.repeat && self.focused.as_deref() != Some(target) {
            self.focused = Some(target.to_owned());
            self.computed = None;
            self.layout_focus = None;
            self.ime_area = None;
            self.text_drag = None;
        }
        true
    }

    fn focus_shortcut_target(&self, source: NodeId, press: &KeyPress) -> Option<&str> {
        let focused = self.dom.get(source).ok()?;
        if !focused.is_focusable()
            || focused.role() == Role::TextInput
            || self.has_pending_composition()
        {
            return None;
        }
        let mut owner = Some(source);
        while let Some(id) = owner.filter(|id| self.in_scope(*id)) {
            let binding = self.bindings.get(&id)?;
            if let Some(shortcut) = binding
                .focus_shortcuts
                .iter()
                .find(|chord| chord.matches(press))
            {
                let target = self.dom.by_id(&shortcut.message)?;
                return (self.in_scope(target)
                    && self.descendant(target, id)
                    && self.dom.get(target).is_ok_and(Node::is_focusable)
                    && self.has_tab_extent(target))
                .then_some(shortcut.message.as_str());
            }
            owner = self.dom.get(id).ok()?.parent();
        }
        None
    }
    /// The host calls this after `key` returns `false`. Sends the message
    /// of the first matching `Application::shortcuts` entry and returns
    /// `true`. Returns `false` during an IME composition, with no match,
    /// and for a chord a focused text field owns: any chord without Control
    /// or Command, and Cmd/Ctrl with A, C, X, V, Z or Y. A repeated press
    /// that matches is consumed and sends nothing.
    pub fn shortcut(&mut self, press: &KeyPress) -> Result<bool, AppError> {
        if self.editors.values().any(|editor| editor.is_composing()) {
            return Ok(false);
        }
        if self.input_snapshot().is_some() && keys::field_owns(press) {
            return Ok(false);
        }
        let Some(shortcut) = self.app.shortcuts().into_iter().find(|s| s.matches(press)) else {
            return Ok(false);
        };
        if press.repeat {
            return Ok(true);
        }
        self.message(Some(shortcut.message))
    }
    /// Semantic drivers and platform input use the same activation path.
    pub fn activate(&mut self, stable_id: &str) -> Result<bool, AppError> {
        #[cfg(feature = "devtools")]
        let span = self.trace_input("ActivateId", Some(stable_id), None, None);
        let result = self.activate_id(stable_id);
        #[cfg(feature = "devtools")]
        self.trace_input_end(span, &result);
        result
    }
    fn activate_id(&mut self, stable_id: &str) -> Result<bool, AppError> {
        let id = self
            .dom
            .by_id(stable_id)
            .filter(|id| self.in_scope(*id) && self.dom.get(*id).is_ok_and(|n| n.is_focusable()))
            .ok_or_else(|| AppError::MissingControl(stable_id.to_owned()))?;
        if self.protects_composition(id) {
            return Ok(false);
        }
        self.pending_roving_focus = None;
        let focus_changed = self.focused.as_deref() != Some(stable_id);
        if focus_changed {
            self.cancel_composition();
        }
        self.focused = Some(stable_id.to_owned());
        let message = self.bindings.get(&id).and_then(|b| b.activate.clone());
        #[cfg(feature = "devtools")]
        self.trace_source(stable_id, trace::Handler::Press);
        self.message(message)
            .map(|changed| changed || focus_changed)
    }
    pub fn dispatch(&mut self, event: Event) -> Result<bool, AppError> {
        self.dispatch_with(event, Self::dispatch_observed_event)
    }
    fn dispatch_with(
        &mut self,
        event: Event,
        dispatch: impl FnOnce(&mut Self, Event) -> Result<bool, AppError>,
    ) -> Result<bool, AppError> {
        if matches!(
            event,
            Event::PointerDown { .. } | Event::FocusNext { .. } | Event::Blur
        ) {
            self.pending_roving_focus = None;
        }
        #[cfg(feature = "devtools")]
        let span = self.trace_event(&event);
        let result = dispatch(self, event);
        #[cfg(feature = "devtools")]
        self.trace_input_end(span, &result);
        result
    }
    fn dispatch_event(
        &mut self,
        event: Event,
        suppress_touch_activation: bool,
    ) -> Result<bool, AppError> {
        match event {
            Event::PointerDown { x, y } => {
                if self.has_captured_drag() {
                    self.cancel_pointer()?;
                }
                let hit = self.hit(x, y);
                if self.protects_pointer_composition(hit) {
                    return Ok(false);
                }
                let observes_application = self.app.observes_frame_input();
                let ripple_role = hit.and_then(|id| self.press_ripple_role(id));
                let press_bounds = if observes_application || ripple_role.is_some() {
                    hit.and_then(|id| {
                        self.computed
                            .as_ref()?
                            .nodes
                            .get(id.index())
                            .map(|node| node.bounds)
                    })
                } else {
                    None
                };
                let drag = self.drag_candidate(hit, x, y);
                if let Some(id) = drag {
                    if self.has_pending_composition() {
                        return Ok(false);
                    }
                    self.begin_drag(id, x, y);
                } else {
                    self.cancel_composition();
                }
                self.cancel_press_ripple();
                if !observes_application {
                    self.press_observation.clear_application();
                }
                self.modal_outside = self.outside_scope(x, y, hit);
                if self.modal_outside {
                    self.pressed = None;
                    return Ok(true);
                }
                self.pressed = hit.and_then(|id| self.stable(id));
                if let Some((id, bounds)) = self.pressed.as_deref().zip(press_bounds) {
                    let time = self.input_time_ms();
                    self.press_observation.accept(
                        id,
                        (x, y),
                        bounds,
                        time,
                        observes_application,
                        ripple_role,
                    );
                }
                self.hovered = self.pressed.clone();
                let defer_focus = hit
                    .and_then(|id| self.bindings.get(&id))
                    .is_some_and(|binding| binding.focus_on_pointer_release);
                if drag.is_none() && !defer_focus {
                    // Blank clicks within a modal must not release its focus scope.
                    if self.pressed.is_some() || self.active_scope_root().is_none() {
                        self.focused = self.pressed.clone();
                    }
                    if self.start_range_drag() {
                        return self.range_pointer(x);
                    }
                    self.place_text_cursor(x, y, false);
                }
                Ok(true)
            }
            Event::SecondaryDown { x, y } => {
                if self.protects_pointer_composition(self.hit(x, y)) {
                    return Ok(false);
                }
                self.secondary_down(x, y)
            }
            Event::PointerUp { x, y } => {
                let drag_tap = self.captured_drag.is_some();
                if self.update_drag(x, y, true, suppress_touch_activation)? {
                    return Ok(true);
                }
                if self.range_drag.is_some() {
                    self.press_observation.release();
                    let result = self.range_pointer(x);
                    self.range_drag = None;
                    self.pressed = None;
                    return result;
                }
                if std::mem::take(&mut self.modal_outside) {
                    return if !suppress_touch_activation && self.outside_scope(x, y, self.hit(x, y))
                    {
                        self.dismiss_scope()
                    } else {
                        Ok(true)
                    };
                }
                self.text_drag = None;
                if suppress_touch_activation {
                    self.pressed = None;
                    return Ok(true);
                }
                let hit = self.hit(x, y).and_then(|id| self.stable(id));
                if let Some(pressed) = self
                    .pressed
                    .take()
                    .filter(|pressed| hit.as_ref() == Some(pressed))
                {
                    self.press_observation.release();
                    // An unclaimed gesture still acts as the original tap.
                    if drag_tap {
                        self.cancel_composition();
                    }
                    self.activate_id(&pressed).map(|_| true)
                } else {
                    Ok(true)
                }
            }
            Event::FocusNext { reverse } => {
                if self.has_captured_drag() {
                    self.cancel_pointer()?;
                }
                let collapsed = match self.dismiss_tab_scope()? {
                    controls::TabDismissal::Standard => None,
                    controls::TabDismissal::Resume { collapsed } => Some(collapsed),
                    controls::TabDismissal::Consumed => return Ok(true),
                };
                let controls: Vec<_> = self
                    .dom
                    .iter()
                    .filter_map(|(id, node)| {
                        let key = node.stable_id()?;
                        (node.is_tab_stop()
                            && self.has_tab_extent(id)
                            && !collapsed.as_ref().is_some_and(|ids| ids.contains(key))
                            && self.tab_stop(id, collapsed.as_ref()))
                        .then_some(id)
                    })
                    .collect();
                if controls.is_empty() {
                    self.cancel_composition();
                    self.focused = match &collapsed {
                        None => self.scope_entry(),
                        Some(ids) => self.focused.take().filter(|key| !ids.contains(key)),
                    };
                    return Ok(true);
                }
                let current = self.focused_node();
                let relative = || {
                    // A programmatically focused opener still anchors Tab
                    // on dismissal even when it is not itself a Tab stop.
                    let anchor = collapsed.as_ref().and(current)?;
                    if reverse {
                        controls.iter().rposition(|id| *id < anchor)
                    } else {
                        controls.iter().position(|id| *id > anchor)
                    }
                };
                let current = current.and_then(|id| controls.iter().position(|key| *key == id));
                let next = match (current, reverse) {
                    (Some(i), true) => (i + controls.len() - 1) % controls.len(),
                    (Some(i), false) => (i + 1) % controls.len(),
                    (None, true) => relative().unwrap_or(controls.len() - 1),
                    (None, false) => relative().unwrap_or(0),
                };
                let next = controls[next];
                if self.protects_composition(next) {
                    return Ok(false);
                }
                self.cancel_composition();
                self.focused = self.stable(next);
                Ok(true)
            }
            Event::Activate => {
                if let Some(id) = self.focused.clone() {
                    self.activate_id(&id)
                } else {
                    Ok(false)
                }
            }
            Event::Text(text) => self.editing_event(Event::Text(text)),
            Event::Backspace => self.editing_event(Event::Edit {
                key: EditKey::Backspace,
                extend: false,
                by_word: false,
            }),
            Event::PointerMove { x, y } => {
                if self.update_drag(x, y, false, false)? {
                    return Ok(true);
                }
                let hover = self.hit(x, y).and_then(|id| self.stable(id));
                let changed = self.hovered != hover;
                self.hovered = hover;
                if self.range_drag.is_some() {
                    return self.range_pointer(x);
                }
                self.place_text_cursor(x, y, true);
                Ok(changed || self.text_drag.is_some())
            }
            Event::PointerLeave => Ok(self.hovered.take().is_some()),
            Event::ImeFor { id, event } => {
                if self.focused.as_ref() != Some(&id) {
                    return Ok(false);
                }
                let cancelled = self.cancel_drag()?;
                self.editing_event(Event::Ime(event))
                    .map(|handled| handled || cancelled)
            }
            Event::Edit {
                key,
                extend,
                by_word,
            } => {
                if self
                    .focused_node()
                    .and_then(|id| self.dom.get(id).ok())
                    .is_some_and(|n| n.role() != Role::TextInput)
                {
                    self.navigate_control(key, extend)
                } else {
                    self.editing_event(Event::Edit {
                        key,
                        extend,
                        by_word,
                    })
                }
            }
            Event::SetValue { id, value } => self.set_value(&id, value),
            event @ Event::Ime(_) => {
                let cancelled = self.cancel_drag()?;
                self.editing_event(event)
                    .map(|handled| handled || cancelled)
            }
            event @ (Event::InputValue { .. } | Event::Select { .. }) => self.editing_event(event),
            Event::Blur => {
                let pointer = self.cancel_pointer()?;
                self.editing_event(Event::Blur)
                    .map(|changed| pointer || changed)
            }
            Event::Cancel => {
                let dragging = self.has_captured_drag();
                let pointer = self.cancel_pointer()?;
                if dragging {
                    return Ok(true);
                }
                let composing = self.has_pending_composition();
                self.cancel_composition();
                if composing {
                    return Ok(true);
                }
                if let Some(message) = self.cancel_within_focus() {
                    return self.message(Some(message));
                }
                if self.active_scope_root().is_none() && self.app.cancel_transient() {
                    self.flush()?;
                    return Ok(true);
                }
                self.dismiss_scope().map(|handled| handled || pointer)
            }
            Event::Scroll {
                x,
                y,
                delta_x,
                delta_y,
            } => self.dispatch_wheel((x, y), (delta_x, delta_y)),
        }
    }
}
