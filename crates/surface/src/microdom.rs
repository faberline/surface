//! An indexed semantic tree. No browser DOM objects or renderer handles live here.
use crate::{AdjustableValue, BoxShadow, Color, Corners, Direction, Style, TextScaling};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(u32);
impl NodeId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Role {
    #[default]
    Group,
    Window,
    Heading,
    Text,
    Button,
    Checkbox,
    TextInput,
    Navigation,
    Radio,
    RadioGroup,
    Switch,
    Slider,
    Tab,
    TabList,
    Menu,
    MenuItem,
    Dialog,
    Progress,
    Status,
    Separator,
    Tooltip,
    List,
    ListItem,
}

/// An accessibility role for a grid or one of its parts. The node's base
/// [`Role`] still determines input and focus behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridRole {
    Grid,
    Row,
    ColumnHeader,
    GridCell,
}

impl GridRole {
    fn applies_to(self, role: Role) -> bool {
        role == Role::Group || (self == Self::GridCell && role == Role::Button)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RangeValue {
    pub min: f64,
    pub max: f64,
    pub value: f64,
    pub step: f64,
}
impl RangeValue {
    pub fn is_valid(self) -> bool {
        [self.min, self.max, self.value, self.step]
            .into_iter()
            .all(f64::is_finite)
            && self.min < self.max
            && (self.max - self.min).is_finite()
            && self.step > 0.
            && self.value >= self.min
            && self.value <= self.max
    }
    pub fn snap(self, value: f64) -> f64 {
        let value = value.clamp(self.min, self.max);
        let steps = (value - self.min) / self.step;
        if steps.is_finite() {
            (self.min + steps.round() * self.step).clamp(self.min, self.max)
        } else {
            value
        }
    }
}
/// Physical edge to clear when positioning an absolute decoration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayHorizontalEdge {
    Left,
    Right,
}

/// Bounds used to keep an absolute decoration clear of the top and one edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayBoundary {
    /// Use the inherited clip, including the viewport.
    Clip,
    /// Use the visible owner on each axis where the overlay fits. Otherwise,
    /// fall back to the inherited clip for that axis.
    Owner,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AbsoluteOverlayFit {
    pub(crate) owner_id: String,
    pub(crate) edge: OverlayHorizontalEdge,
    pub(crate) boundary: OverlayBoundary,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PressRipple {
    pub(crate) after: Option<String>,
    pub(crate) presented: Option<PresentedPressRipple>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PresentedPressRipple {
    pub(crate) center: (f32, f32),
    pub(crate) progress: f32,
    pub(crate) opacity: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    #[serde(default, skip_serializing_if = "no_box_shadows")]
    box_shadows: [BoxShadow; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interaction_box_shadows: Option<Box<InteractionBoxShadows>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interaction_corners: Option<Box<[Option<Corners>; 3]>>,
    #[serde(skip)]
    animate_interaction_corners: bool,
    #[serde(skip)]
    pub(crate) press_ripple: Option<Arc<PressRipple>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) fill_contour: Option<crate::FillContour>,
    #[serde(skip)]
    pub(crate) raster: Option<crate::RasterImage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    current_page: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    grid_role: Option<GridRole>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    web_listbox: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    web_multiselect_listbox: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    grid_multiselectable: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    current_date: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    described_by_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expanded: Option<bool>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    disclosure: bool,
    #[serde(skip)]
    horizontal_slide: f32,
    #[serde(skip)]
    measured_group: Option<crate::layout::MeasuredGroup>,
    #[serde(skip)]
    absolute_overlay_fit: Option<AbsoluteOverlayFit>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    shrink_to_fit_width: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    shrink_to_fit_height: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    reverse_row: bool,
    #[serde(default, skip_serializing_if = "TextScaling::is_default")]
    text_scaling: TextScaling,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    focus_color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    focus_outline_width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    state_layer_alphas: Option<[u8; 3]>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    focus_outline_from_parent: bool,
    pub(crate) stable_id: Option<String>,
    pub(crate) role: Role,
    pub(crate) name: String,
    pub(crate) text: Option<String>,
    pub(crate) disabled: bool,
    pub(crate) checked: Option<bool>,
    #[serde(default)]
    selected: Option<bool>,
    #[serde(default)]
    mixed: bool,
    #[serde(default)]
    range: Option<RangeValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    adjustable: Option<Box<AdjustableValue>>,
    #[serde(default)]
    tab_index: Option<i32>,
    pub(crate) style: Style,
    parent: Option<NodeId>,
    first_child: Option<NodeId>,
    last_child: Option<NodeId>,
    next_sibling: Option<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct InteractionBoxShadows {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hovered: Option<[BoxShadow; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pressed: Option<[BoxShadow; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    focused: Option<[BoxShadow; 2]>,
}

fn no_box_shadows(layers: &[BoxShadow; 2]) -> bool {
    *layers == [BoxShadow::NONE; 2]
}

impl Node {
    pub fn new(role: Role, name: impl Into<String>) -> Self {
        Self {
            stable_id: None,
            box_shadows: [BoxShadow::NONE; 2],
            interaction_box_shadows: None,
            interaction_corners: None,
            animate_interaction_corners: false,
            press_ripple: None,
            fill_contour: None,
            raster: None,
            current_page: None,
            grid_role: None,
            web_listbox: false,
            web_multiselect_listbox: false,
            grid_multiselectable: false,
            current_date: false,
            described_by_text: None,
            expanded: None,
            disclosure: false,
            horizontal_slide: 0.,
            measured_group: None,
            absolute_overlay_fit: None,
            shrink_to_fit_width: false,
            shrink_to_fit_height: false,
            reverse_row: false,
            text_scaling: TextScaling::default(),
            focus_color: None,
            focus_outline_width: None,
            state_layer_alphas: None,
            focus_outline_from_parent: false,
            role,
            name: name.into(),
            text: None,
            disabled: false,
            checked: None,
            selected: None,
            mixed: false,
            range: None,
            adjustable: None,
            tab_index: None,
            style: Style::default(),
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
        }
    }
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.stable_id = Some(id.into());
        self
    }
    /// Set two drop shadows in back-to-front order. Shadows affect paint only.
    pub fn box_shadows(mut self, layers: [BoxShadow; 2]) -> Self {
        self.box_shadows = layers;
        self
    }
    /// Return the authored drop shadows in paint order.
    pub fn box_shadow_layers(&self) -> [BoxShadow; 2] {
        self.box_shadows
    }
    /// Set two paint-only shadow layers for each interaction state. Pressed
    /// wins over hovered, then focused. A missing override uses the base
    /// shadows, as does a disabled node. Pass two [`BoxShadow::NONE`] layers
    /// to remove the shadow in a state.
    pub fn interaction_box_shadows(
        mut self,
        hovered: Option<[BoxShadow; 2]>,
        pressed: Option<[BoxShadow; 2]>,
        focused: Option<[BoxShadow; 2]>,
    ) -> Self {
        self.interaction_box_shadows =
            (hovered.is_some() || pressed.is_some() || focused.is_some()).then(|| {
                Box::new(InteractionBoxShadows {
                    hovered,
                    pressed,
                    focused,
                })
            });
        self
    }

    pub(crate) fn presented_box_shadow_layers(
        &self,
        id: NodeId,
        focused: Option<NodeId>,
        interaction: crate::InteractionState,
    ) -> [BoxShadow; 2] {
        let Some(layers) = self
            .interaction_box_shadows
            .as_deref()
            .filter(|_| !self.disabled)
        else {
            return self.box_shadows;
        };
        let active = if interaction.pressed == Some(id) {
            layers.pressed
        } else if interaction.hovered == Some(id) {
            layers.hovered
        } else if focused == Some(id) {
            layers.focused
        } else {
            None
        };
        active.unwrap_or(self.box_shadows)
    }
    /// Replace the painted corners of this node while it is hovered, pressed,
    /// or focused. Press wins over focus, then hover. A disabled node or state
    /// without valid override radii uses the style's corners. This changes
    /// paint immediately and keeps the same layout and hit area.
    pub fn interaction_corners(
        mut self,
        hovered: Option<Corners>,
        pressed: Option<Corners>,
        focused: Option<Corners>,
    ) -> Self {
        let overrides = [hovered, pressed, focused].map(|radii| radii.filter(Corners::is_valid));
        self.interaction_corners = overrides
            .iter()
            .any(Option::is_some)
            .then(|| Box::new(overrides));
        self
    }

    /// Let the runtime animate changes between the interaction corners and
    /// the style's resting corners using the style's transition.
    ///
    /// This affects paint only. It needs a stable node ID and a runtime frame
    /// clock. Direct paint compilation and serialized snapshots keep the
    /// immediate interaction behavior.
    pub fn animate_interaction_corners(mut self) -> Self {
        self.animate_interaction_corners = true;
        self
    }

    /// Enable bounded feedback for an accepted pointer press on this node.
    ///
    /// The extended renderer clips the ripple to the node's current outline.
    /// Set `after` to the stable ID of a passive raster descendant to draw
    /// the wave and focus outline above its image. Layout, input and snapshots
    /// keep their original form.
    pub fn press_ripple(mut self, after: Option<String>) -> Self {
        self.press_ripple = Some(Arc::new(PressRipple {
            after,
            presented: None,
        }));
        self
    }

    /// Whether this live node requests pointer ripple feedback.
    pub fn has_press_ripple(&self) -> bool {
        self.press_ripple.is_some()
    }

    /// Return the effective corner target for an opted-in runtime track.
    #[doc(hidden)]
    pub fn animated_interaction_corner_target(
        &self,
        id: NodeId,
        focus: Option<NodeId>,
        interaction: crate::InteractionState,
    ) -> Option<Corners> {
        (self.animate_interaction_corners && self.stable_id().is_some()).then(|| {
            self.presented_interaction_corners(id, focus, interaction)
                .unwrap_or_else(|| self.style.corner_radii())
        })
    }
    pub(crate) fn presented_interaction_corners(
        &self,
        id: NodeId,
        focus: Option<NodeId>,
        interaction: crate::InteractionState,
    ) -> Option<Corners> {
        let &[hovered, pressed, focused] = self
            .interaction_corners
            .as_deref()
            .filter(|_| !self.disabled)?;
        let active = if interaction.pressed == Some(id) {
            pressed
        } else if focus == Some(id) {
            focused
        } else if interaction.hovered == Some(id) {
            hovered
        } else {
            None
        };
        active.filter(Corners::is_valid)
    }
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
    /// Draw one closed fill relative to the node's origin after its paths.
    /// The ancestor clip applies. The fill adds no layout or input area and
    /// does not create a semantic child.
    pub fn fill_contour(mut self, contour: crate::FillContour) -> Self {
        self.fill_contour = Some(contour);
        self
    }

    /// Paint image pixels across this node with a centered aspect crop.
    /// Size the node through its style. The image adds no layout or semantic
    /// child. The extended renderer draws it with the node's corners and
    /// ancestor clip. Serialized DOM snapshots omit the live image resource.
    pub fn raster_image(mut self, image: crate::RasterImage) -> Self {
        self.raster = Some(image);
        self
    }
    /// Paint this node's focus outline while its immediate parent is focused.
    /// The parent's own outline can be hidden with a transparent focus color.
    /// This marker does not give the visual node keyboard or platform focus.
    pub fn focus_outline_from_parent(mut self) -> Self {
        self.focus_outline_from_parent = true;
        self
    }
    pub(crate) fn paints_focus_outline(&self, id: NodeId, focused: Option<NodeId>) -> bool {
        focused.is_some_and(|focus| {
            focus == id || (self.focus_outline_from_parent && self.parent() == Some(focus))
        })
    }
    /// Lay out a row from right to left. Its children keep their logical
    /// source order for accessibility and focus navigation.
    pub fn reverse_row(mut self) -> Self {
        self.reverse_row = true;
        self
    }
    pub fn is_row_reversed(&self) -> bool {
        self.reverse_row && self.style.direction == Direction::Row
    }
    /// Offset the computed subtree horizontally by a fraction of this node's
    /// measured width. Positive values move it right. Sibling positions and
    /// measured sizes stay the same. Paint and input use the moved geometry.
    pub fn slide_horizontally(mut self, fraction: f32) -> Self {
        if fraction.is_finite() {
            self.horizontal_slide = fraction.clamp(-1., 1.);
        }
        self
    }
    pub(crate) fn horizontal_slide(&self) -> f32 {
        self.horizontal_slide
    }
    /// Measure this node as a single item and place its direct decorative leaves.
    ///
    /// Children must be flow leaves. Each receives a rectangle relative to
    /// this group. The callback is runtime-only and is not serialized.
    pub fn measured_group(mut self, layout: impl crate::MeasuredGroupLayout + 'static) -> Self {
        self.measured_group = Some(crate::layout::MeasuredGroup(Arc::new(layout)));
        self
    }
    pub(crate) fn measured_group_layout(&self) -> Option<&dyn crate::MeasuredGroupLayout> {
        self.measured_group.as_ref().map(|group| group.0.as_ref())
    }
    /// Move an absolute decoration clear of the top and a physical edge.
    ///
    /// Measure the node first, then translate the subtree without resizing
    /// it or increasing its parent's scroll extent. An oversized decoration
    /// can still clip at its opposite edges. Keep decorative children passive.
    ///
    /// The named owner must be an ancestor or a preceding sibling of this
    /// node or one of its ancestors in the same viewport layer. Hide the
    /// decoration when the owner's bounds are outside the inherited clip.
    /// This setting applies only to absolute positioning and stays off wire.
    pub fn fit_absolute_overlay(
        mut self,
        owner_id: impl Into<String>,
        edge: OverlayHorizontalEdge,
        boundary: OverlayBoundary,
    ) -> Self {
        self.absolute_overlay_fit = Some(AbsoluteOverlayFit {
            owner_id: owner_id.into(),
            edge,
            boundary,
        });
        self
    }
    pub(crate) fn absolute_overlay_fit(&self) -> Option<&AbsoluteOverlayFit> {
        self.absolute_overlay_fit
            .as_ref()
            .filter(|_| self.style.position == crate::Position::Absolute)
    }
    /// Let this child give up width in a flex row when space runs out.
    /// A column keeps its usual height behavior. A child with space
    /// available keeps its natural width or its specified width.
    pub fn shrink_to_fit_width(mut self) -> Self {
        self.shrink_to_fit_width = true;
        self
    }
    pub fn can_shrink_width(&self) -> bool {
        self.shrink_to_fit_width
    }
    /// Let this child give up height in a flex column when space runs out.
    /// Its children keep their own layout rules.
    pub fn shrink_to_fit_height(mut self) -> Self {
        self.shrink_to_fit_height = true;
        self
    }
    pub fn can_shrink_height(&self) -> bool {
        self.shrink_to_fit_height
    }
    pub fn text_scaling(mut self, scaling: TextScaling) -> Self {
        self.text_scaling = scaling;
        self
    }
    pub fn text_scaling_mode(&self) -> TextScaling {
        self.text_scaling
    }
    /// Set the focus outline color without changing the node's text color.
    /// When omitted, the outline uses the foreground color.
    /// A transparent focus color hides the outline.
    pub fn focus_color(mut self, color: Color) -> Self {
        self.focus_color = Some(color);
        self
    }
    pub(crate) fn focus_outline_color(&self) -> Option<Color> {
        match self.focus_color {
            Some(color) => (color.3 > 0).then_some(color),
            None => Some(self.style.foreground),
        }
    }
    /// Paint an inside focus stroke of this width along the node's shape.
    /// The width must be finite and positive. Other values restore the default.
    pub fn focus_outline_width(mut self, width: f32) -> Self {
        self.focus_outline_width = valid_outline_width(width).then_some(width);
        self
    }
    pub(crate) fn resolved_focus_outline_width(&self) -> Option<f32> {
        self.focus_outline_width
            .filter(|&width| valid_outline_width(width))
    }
    /// Set the state layer opacity on the 0–255 scale. This does not change
    /// the focus outline.
    pub fn state_layer_alphas(mut self, hover: u8, focus: u8, press: u8) -> Self {
        self.state_layer_alphas = Some([hover, focus, press]);
        self
    }
    pub(crate) fn resolved_state_layer_alphas(&self) -> [u8; 3] {
        self.state_layer_alphas.unwrap_or([20, 0, 31])
    }
    /// Mark whether this navigation destination represents the current page.
    pub fn current_page(mut self, value: bool) -> Self {
        self.current_page = Some(value);
        self
    }
    /// The current-page state if the destination declares it.
    /// `None` means the destination does not declare this state.
    pub fn is_current_page(&self) -> Option<bool> {
        self.current_page
    }
    /// Give a group a grid role, or give a button the grid cell role. Use the
    /// button for a cell that can receive focus and activate.
    pub fn grid_role(mut self, role: GridRole) -> Self {
        self.grid_role = role.applies_to(self.role).then_some(role);
        self
    }
    pub fn accessibility_grid_role(&self) -> Option<GridRole> {
        self.grid_role.filter(|role| role.applies_to(self.role))
    }
    /// Expose a RadioGroup as a listbox or a Radio as an option on Web.
    /// A Radio's checked value supplies the option's selected state.
    /// The base role still controls keyboard and native accessibility.
    pub fn web_listbox_semantics(mut self) -> Self {
        self.web_listbox = matches!(self.role, Role::RadioGroup | Role::Radio);
        self
    }
    pub fn uses_web_listbox_semantics(&self) -> bool {
        self.web_listbox && matches!(self.role, Role::RadioGroup | Role::Radio)
    }
    /// Expose a Group as a multi-select listbox or a Checkbox as an option
    /// on Web. Set this on the listbox and on each option. Use checked values
    /// to express binary option selection.
    /// The base role and mixed state still apply to native accessibility.
    pub fn web_multiselect_listbox_semantics(mut self) -> Self {
        self.web_multiselect_listbox = matches!(self.role, Role::Group | Role::Checkbox);
        self
    }
    pub fn uses_web_multiselect_listbox_semantics(&self) -> bool {
        self.web_multiselect_listbox && matches!(self.role, Role::Group | Role::Checkbox)
    }
    /// Report that this grid can have multiple selected cells. Set selection
    /// on each cell separately. This property only applies to [`GridRole::Grid`].
    pub fn grid_multiselectable(mut self, value: bool) -> Self {
        self.grid_multiselectable = value;
        self
    }
    pub fn is_grid_multiselectable(&self) -> bool {
        self.grid_multiselectable && self.accessibility_grid_role() == Some(GridRole::Grid)
    }
    /// Mark a grid cell as today's date, independently of selection and focus.
    pub fn current_date(mut self, value: bool) -> Self {
        self.current_date = value;
        self
    }
    pub fn is_current_date(&self) -> bool {
        self.current_date && self.accessibility_grid_role() == Some(GridRole::GridCell)
    }
    /// Describe this node with a visible, nonfocusable text node in its scope.
    /// Supply the stable ID of that node.
    pub fn described_by_text(mut self, target: impl Into<String>) -> Self {
        self.described_by_text = Some(target.into());
        self
    }
    pub fn description_text_target(&self) -> Option<&str> {
        self.described_by_text.as_deref()
    }
    /// Set the open or closed state of a button that controls a disclosure.
    pub fn disclosure_expanded(mut self, value: bool) -> Self {
        self.expanded = (self.role == Role::Button).then_some(value);
        self.disclosure = self.expanded.is_some();
        self
    }
    /// Mark this button as a menu trigger and report whether its menu is open.
    pub fn menu_expanded(mut self, value: bool) -> Self {
        self.expanded = (self.role == Role::Button).then_some(value);
        self.disclosure = false;
        self
    }
    /// The open or closed state of a button, if declared.
    pub fn expanded_state(&self) -> Option<bool> {
        if self.role == Role::Button {
            self.expanded
        } else {
            None
        }
    }
    /// Whether this button opens a menu.
    pub fn has_menu_popup(&self) -> bool {
        !self.disclosure && self.expanded_state().is_some()
    }
    pub fn styled(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
    pub fn checked(mut self, value: bool) -> Self {
        self.checked = Some(value);
        self
    }
    pub fn selected(mut self, value: bool) -> Self {
        self.selected = Some(value);
        self
    }
    pub fn mixed(mut self, value: bool) -> Self {
        self.mixed = value;
        self
    }
    pub fn range(mut self, value: RangeValue) -> Self {
        self.range = Some(value);
        self
    }
    /// Expose this named Group as an adjustable control in accessibility
    /// adapters. Slider geometry, pointer hit testing and legacy ranges are
    /// unchanged. Grid roles keep priority over this opt-in.
    pub fn adjustable(mut self, value: AdjustableValue) -> Self {
        self.adjustable = Some(Box::new(value));
        self
    }
    pub fn accessibility_adjustable(&self) -> Option<&AdjustableValue> {
        self.adjustable.as_deref().filter(|value| {
            self.role == Role::Group && self.accessibility_grid_role().is_none() && value.is_valid()
        })
    }
    pub fn tab_index(mut self, value: i32) -> Self {
        self.tab_index = Some(value);
        self
    }
    pub fn is_selected(&self) -> Option<bool> {
        self.selected
    }
    pub fn is_mixed(&self) -> bool {
        self.mixed
    }
    pub fn range_value(&self) -> Option<RangeValue> {
        self.range
    }
    pub fn is_tab_stop(&self) -> bool {
        self.is_focusable() && self.tab_index.is_none_or(|v| v >= 0)
    }
    pub fn stable_id(&self) -> Option<&str> {
        self.stable_id.as_deref()
    }
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn text_value(&self) -> Option<&str> {
        self.text.as_deref()
    }
    pub fn style(&self) -> &Style {
        &self.style
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    pub fn is_checked(&self) -> Option<bool> {
        self.checked
    }
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }
    pub fn is_focusable(&self) -> bool {
        !self.disabled
            && (self.tab_index.is_some()
                || self.accessibility_adjustable().is_some()
                || matches!(
                    self.role,
                    Role::Button
                        | Role::Checkbox
                        | Role::TextInput
                        | Role::Radio
                        | Role::Switch
                        | Role::Slider
                        | Role::Tab
                        | Role::MenuItem
                ))
    }
}

fn valid_outline_width(width: f32) -> bool {
    width.is_finite() && width > 0.
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DomError {
    #[error("duplicate stable ID: {0}")]
    DuplicateId(String),
    #[error("invalid node index: {0}")]
    InvalidNode(usize),
    #[error("MicroDOM exceeds the u32 node limit")]
    Capacity,
    #[error("unsupported MicroDOM snapshot version: {0}")]
    UnsupportedVersion(u32),
    #[error("MicroDOM snapshot has no nodes")]
    EmptySnapshot,
    #[error("invalid MicroDOM tree links at node {0}")]
    InvalidTree(usize),
}
#[derive(Clone, Debug)]
pub struct MicroDom {
    nodes: Vec<Node>,
    ids: BTreeMap<String, NodeId>,
    semantics: BTreeMap<(Role, String), Vec<NodeId>>,
}
impl MicroDom {
    pub fn new(root: Node) -> Self {
        let mut dom = Self {
            nodes: Vec::new(),
            ids: BTreeMap::new(),
            semantics: BTreeMap::new(),
        };
        dom.insert(root).expect("a root cannot have duplicate IDs");
        dom
    }
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }
    fn insert(&mut self, mut node: Node) -> Result<NodeId, DomError> {
        if let Some(id) = &node.stable_id {
            if self.ids.contains_key(id) {
                return Err(DomError::DuplicateId(id.clone()));
            }
        }
        let id = NodeId(u32::try_from(self.nodes.len()).map_err(|_| DomError::Capacity)?);
        // Node values may be copied from a different tree. Never copy its links.
        node.parent = None;
        node.first_child = None;
        node.last_child = None;
        node.next_sibling = None;
        if let Some(stable_id) = &node.stable_id {
            self.ids.insert(stable_id.clone(), id);
        }
        self.semantics
            .entry((node.role, node.name.clone()))
            .or_default()
            .push(id);
        self.nodes.push(node);
        Ok(id)
    }
    pub fn append(&mut self, parent: NodeId, node: Node) -> Result<NodeId, DomError> {
        let last = self.get(parent)?.last_child;
        let id = self.insert(node)?;
        self.nodes[id.index()].parent = Some(parent);
        if let Some(last) = last {
            self.nodes[last.index()].next_sibling = Some(id);
        } else {
            self.nodes[parent.index()].first_child = Some(id);
        }
        self.nodes[parent.index()].last_child = Some(id);
        Ok(id)
    }
    pub fn get(&self, id: NodeId) -> Result<&Node, DomError> {
        self.nodes
            .get(id.index())
            .ok_or(DomError::InvalidNode(id.index()))
    }
    pub fn by_id(&self, id: &str) -> Option<NodeId> {
        self.ids.get(id).copied()
    }
    pub fn by_role_name(&self, role: Role, name: &str) -> &[NodeId] {
        self.semantics
            .get(&(role, name.to_owned()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (NodeId(i as u32), node))
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn children(&self, parent: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let first = self.nodes.get(parent.index()).and_then(|n| n.first_child);
        std::iter::successors(first, |id| self.nodes[id.index()].next_sibling)
    }
    pub fn set_style(&mut self, id: NodeId, style: Style) -> Result<(), DomError> {
        let n = self
            .nodes
            .get_mut(id.index())
            .ok_or(DomError::InvalidNode(id.index()))?;
        n.style = style;
        Ok(())
    }

    /// Apply one valid presented outline to the display copy before paint.
    /// Ignore invalid radii. The semantic DOM keeps the authored style and
    /// interaction targets.
    #[doc(hidden)]
    pub fn set_presented_interaction_corners(
        &mut self,
        id: NodeId,
        corners: Corners,
    ) -> Result<(), DomError> {
        let node = self
            .nodes
            .get_mut(id.index())
            .ok_or(DomError::InvalidNode(id.index()))?;
        if corners.is_valid() {
            node.style.corners = Some(corners);
            node.interaction_corners = None;
        }
        Ok(())
    }

    /// Set the active ripple on a display copy for the extended renderer.
    /// The center is in physical logical pixels relative to the node.
    /// Finite progress and opacity clamp to 0–1. Zero opacity still replaces
    /// the held press fill. An invalid sample clears the copy's wave.
    #[doc(hidden)]
    pub fn set_presented_press_ripple(
        &mut self,
        id: NodeId,
        center_local: (f32, f32),
        progress: f32,
        opacity: f32,
    ) -> Result<(), DomError> {
        let node = self
            .nodes
            .get_mut(id.index())
            .ok_or(DomError::InvalidNode(id.index()))?;
        if let Some(ripple) = &mut node.press_ripple {
            let presented = [center_local.0, center_local.1, progress, opacity]
                .into_iter()
                .all(f32::is_finite)
                .then(|| PresentedPressRipple {
                    center: center_local,
                    progress: progress.clamp(0., 1.),
                    opacity: opacity.clamp(0., 1.),
                });
            Arc::make_mut(ripple).presented = presented;
        }
        Ok(())
    }

    /// Set a node-local contour on the display copy. This does not affect layout.
    #[doc(hidden)]
    pub fn set_presented_fill_contour(
        &mut self,
        id: NodeId,
        contour: crate::FillContour,
    ) -> Result<(), DomError> {
        let node = self
            .nodes
            .get_mut(id.index())
            .ok_or(DomError::InvalidNode(id.index()))?;
        node.fill_contour = Some(contour);
        Ok(())
    }

    pub fn set_text(&mut self, id: NodeId, text: impl Into<String>) -> Result<(), DomError> {
        let n = self
            .nodes
            .get_mut(id.index())
            .ok_or(DomError::InvalidNode(id.index()))?;
        n.text = Some(text.into());
        Ok(())
    }
    pub fn snapshot_version(&self) -> u32 {
        Self::version_for_nodes(&self.nodes)
    }
    fn version_for_nodes(nodes: &[Node]) -> u32 {
        if nodes.iter().any(|node| node.fill_contour.is_some()) {
            // This is the fill contour version used by the extended scene format.
            4
        } else {
            1
        }
    }
    pub fn snapshot(&self) -> DomSnapshot {
        DomSnapshot {
            version: self.snapshot_version(),
            nodes: self.nodes.clone(),
        }
    }
    pub fn from_snapshot(snapshot: DomSnapshot) -> Result<MicroDom, DomError> {
        if snapshot.version != Self::version_for_nodes(&snapshot.nodes) {
            return Err(DomError::UnsupportedVersion(snapshot.version));
        }
        let nodes = snapshot.nodes;
        if nodes.is_empty() {
            return Err(DomError::EmptySnapshot);
        }
        let len = nodes.len();
        u32::try_from(len).map_err(|_| DomError::Capacity)?;
        for node in &nodes {
            for link in [
                node.parent,
                node.first_child,
                node.last_child,
                node.next_sibling,
            ]
            .into_iter()
            .flatten()
            {
                if link.index() >= len {
                    return Err(DomError::InvalidNode(link.index()));
                }
            }
        }
        let mut ids = BTreeMap::new();
        let mut semantics: BTreeMap<(Role, String), Vec<NodeId>> = BTreeMap::new();
        for (i, node) in nodes.iter().enumerate() {
            let id = NodeId(i as u32);
            if let Some(stable_id) = &node.stable_id {
                if ids.contains_key(stable_id) {
                    return Err(DomError::DuplicateId(stable_id.clone()));
                }
                ids.insert(stable_id.clone(), id);
            }
            semantics
                .entry((node.role, node.name.clone()))
                .or_default()
                .push(id);
        }
        if nodes[0].parent.is_some() {
            return Err(DomError::InvalidTree(0));
        }
        let mut visited = vec![false; len];
        visited[0] = true;
        for p in 0..len {
            let mut last_seen = None;
            let mut cur = nodes[p].first_child;
            while let Some(c) = cur {
                let ci = c.index();
                if nodes[ci].parent != Some(NodeId(p as u32)) || visited[ci] {
                    return Err(DomError::InvalidTree(ci));
                }
                visited[ci] = true;
                last_seen = Some(c);
                cur = nodes[ci].next_sibling;
            }
            if last_seen != nodes[p].last_child {
                return Err(DomError::InvalidTree(p));
            }
        }
        if let Some(i) = visited.iter().position(|seen| !seen) {
            return Err(DomError::InvalidTree(i));
        }
        Ok(MicroDom {
            nodes,
            ids,
            semantics,
        })
    }
    /// Viewport layers paint after flow content, in z-index order.
    pub fn paint_order(&self) -> Vec<NodeId> {
        fn visit(dom: &MicroDom, id: NodeId, out: &mut Vec<NodeId>) {
            out.push(id);
            let mut children: Vec<_> = dom
                .children(id)
                .filter(|id| {
                    dom.get(*id).expect("valid node").style().position != crate::Position::Fixed
                })
                .collect();
            children.sort_by_key(|id| dom.get(*id).expect("valid node").style().z_index);
            for id in children {
                visit(dom, id, out);
            }
        }
        let mut out = Vec::with_capacity(self.len());
        visit(self, self.root(), &mut out);
        let mut layers: Vec<_> = self
            .iter()
            .filter(|(id, n)| *id != self.root() && n.style().position == crate::Position::Fixed)
            .map(|(id, _)| id)
            .collect();
        layers.sort_by_key(|id| self.get(*id).expect("valid node").style().z_index);
        for id in layers {
            visit(self, id, &mut out);
        }
        out
    }

    /// Rearrange this DOM's paint order around explicit modal layers. Supply
    /// `(backdrop, panel)` pairs from back to front. A backdrop, if present,
    /// must be the panel's immediate parent. Invalid or reused endpoints are
    /// ignored.
    ///
    /// All background nodes keep their relative order and paint first. Each
    /// layer paints its backdrop, panel subtree, then the fixed descendants
    /// of its panel in their existing order. A nested layer is painted
    /// separately. Other children of a backdrop do not belong to its layer.
    pub fn apply_modal_layers(
        &self,
        paint_order: &mut Vec<NodeId>,
        layers: &[(Option<NodeId>, NodeId)],
    ) {
        if layers.is_empty() {
            return;
        }

        #[derive(Clone, Copy)]
        enum PaintPass {
            Backdrop(usize),
            Panel(usize),
            Flow(usize),
            Fixed(usize),
        }
        #[derive(Default)]
        struct Layer {
            backdrop: Option<NodeId>,
            panel: Option<NodeId>,
            flow: Vec<NodeId>,
            fixed: Vec<NodeId>,
        }

        let mut passes = vec![None; self.len()];
        let mut ordered = Vec::with_capacity(layers.len());
        for &(backdrop, panel) in layers {
            let Ok(node) = self.get(panel) else { continue };
            if passes[panel.index()].is_some()
                || backdrop
                    .is_some_and(|id| node.parent() != Some(id) || passes[id.index()].is_some())
            {
                continue;
            }
            let index = ordered.len();
            passes[panel.index()] = Some(PaintPass::Panel(index));
            if let Some(backdrop) = backdrop {
                passes[backdrop.index()] = Some(PaintPass::Backdrop(index));
            }
            ordered.push(Layer::default());
        }
        if ordered.is_empty() {
            return;
        }

        fn assign(
            dom: &MicroDom,
            id: NodeId,
            inherited: Option<PaintPass>,
            passes: &mut [Option<PaintPass>],
        ) {
            let explicit = passes[id.index()];
            let effective = match (explicit, inherited) {
                (Some(PaintPass::Panel(index)), _) => Some(PaintPass::Flow(index)),
                (_, Some(PaintPass::Flow(index)))
                    if dom.get(id).expect("valid node").style().position
                        == crate::Position::Fixed =>
                {
                    Some(PaintPass::Fixed(index))
                }
                (_, inherited) => inherited,
            };
            passes[id.index()] = explicit.or(effective);
            for child in dom.children(id) {
                assign(dom, child, effective, passes);
            }
        }
        assign(self, self.root(), None, &mut passes);

        paint_order.retain(|&id| {
            let Some(pass) = passes[id.index()] else {
                return true;
            };
            match pass {
                PaintPass::Backdrop(index) => ordered[index].backdrop = Some(id),
                PaintPass::Panel(index) => ordered[index].panel = Some(id),
                PaintPass::Flow(index) => ordered[index].flow.push(id),
                PaintPass::Fixed(index) => ordered[index].fixed.push(id),
            }
            false
        });
        for layer in ordered {
            paint_order.extend(layer.backdrop);
            paint_order.extend(layer.panel);
            paint_order.extend(layer.flow);
            paint_order.extend(layer.fixed);
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomSnapshot {
    pub version: u32,
    pub nodes: Vec<Node>,
}
