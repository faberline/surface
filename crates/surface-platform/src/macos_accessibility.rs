//! Project the current application scope into the macOS accessibility tree.
use accesskit::{
    Action, ActionData, ActionRequest, AriaCurrent, HasPopup, Invalid, Live, Node, NodeId,
    Orientation, Rect, Role, ScrollUnit, Toggled, TreeId, TreeInfo, TreeUpdate,
};
use std::collections::BTreeMap;
use surface::{GridRole, NodeId as SurfaceId, Role as SurfaceRole};
use surface_app::{
    AccessibilityActivationResult, AccessibilityActivationTarget, AccessibilityAdjustment,
    AccessibilityAdjustmentResult, AccessibilityAdjustmentTarget, AccessibilityFocusResult,
    AccessibilityFocusTarget, AccessibilityFrame, AccessibilityFrameId, AccessibilityNode,
    AccessibilityScrollDirection, AccessibilityScrollPageTarget, AccessibilityScrollResult,
    AccessibilityScrollTarget, AppError, Application, Runtime,
};

const WINDOW: NodeId = NodeId(0);
const PAGE_ACTIONS: [(Action, AccessibilityScrollDirection); 2] = [
    (Action::ScrollUp, AccessibilityScrollDirection::Up),
    (Action::ScrollDown, AccessibilityScrollDirection::Down),
];

#[derive(Default)]
struct Targets {
    focus: Option<AccessibilityFocusTarget>,
    press: Option<AccessibilityActivationTarget>,
    adjustment: Option<AccessibilityAdjustmentTarget>,
    reveal: Option<AccessibilityScrollTarget>,
    pages: BTreeMap<Action, AccessibilityScrollPageTarget>,
}

#[derive(Default)]
pub(crate) struct Tree {
    next_id: u64,
    generation: Option<AccessibilityFrameId>,
    ids: BTreeMap<SurfaceId, NodeId>,
    scroll_ids: BTreeMap<SurfaceId, NodeId>,
    status_ids: BTreeMap<String, NodeId>,
    modal_status_scope: Option<Option<String>>,
    targets: BTreeMap<NodeId, Targets>,
}

impl Tree {
    pub(crate) fn reset(&mut self) {
        self.generation = None;
        self.ids.clear();
        self.scroll_ids.clear();
        self.status_ids.clear();
        self.modal_status_scope = None;
        self.targets.clear();
    }

    fn allocate(&mut self) -> NodeId {
        self.next_id += 1;
        NodeId(self.next_id)
    }

    pub(crate) fn update<M>(&mut self, frame: AccessibilityFrame<'_, M>, scale: f32) -> TreeUpdate {
        let generation = frame.generation();
        let same_generation = self.generation.as_ref() == Some(&generation);
        let root = frame.root();
        let modal = frame.modal_root();
        let status_scope = modal.map(|id| frame.node(id).and_then(|node| node.node.stable_id()));
        let same_status_scope = match status_scope {
            None => self.modal_status_scope.is_none(),
            Some(Some(key)) => {
                self.modal_status_scope.as_ref().and_then(Option::as_deref) == Some(key)
            }
            Some(None) => same_generation && self.modal_status_scope == Some(None),
        };

        let mut ids = BTreeMap::new();
        let mut scroll_ids = BTreeMap::new();
        let mut status_ids = BTreeMap::new();
        let mut targets = BTreeMap::new();
        let mut nodes = Vec::new();
        let mut descriptions = Vec::new();
        let mut tab_panels = Vec::new();
        let mut tab_positions = BTreeMap::new();
        let mut stack = vec![(root, false)];
        while let Some((id, visited)) = stack.pop() {
            if !visited {
                stack.push((id, true));
                stack.extend(frame.children(id).map(|child| (child, false)));
                continue;
            }
            let Some(source) = frame.node(id) else {
                continue;
            };
            let mut children: Vec<_> = frame
                .children(id)
                .filter_map(|child| scroll_ids.get(&child).or_else(|| ids.get(&child)).copied())
                .collect();
            let bounds = bounds(source.layout.bounds.intersect(source.layout.clip), scale);
            let reveal = frame.scroll_target(id);
            if id != root && bounds.is_none() && children.is_empty() && reveal.is_none() {
                continue;
            }

            let mut node = describe(&source, frame.slider_adjustment_bounds(id));
            let focus = frame.focus_target(id);
            let press = frame.activation_target(id);
            let adjustment = frame.adjustment_target(id);
            let pages: BTreeMap<_, _> = PAGE_ACTIONS
                .into_iter()
                .filter_map(|(action, direction)| {
                    bounds?;
                    frame
                        .scroll_page_target(id, direction)
                        .map(|target| (action, target))
                })
                .collect();
            // AccessKit announces a live Status on insertion and on label
            // changes. Retain only visible, passive IDs across rebuilds.
            let stable_key = source.node.stable_id();
            let status_key = stable_key.filter(|key| {
                id != root
                    && node.role() == Role::Status
                    && !key.is_empty()
                    && !source.node.is_disabled()
                    && bounds.is_some()
                    && frame.children(id).next().is_none()
                    && focus.is_none()
                    && press.is_none()
                    && reveal.is_none()
                    && pages.is_empty()
            });
            let native_id = if id == root && modal.is_none() {
                node.set_role(Role::Window);
                WINDOW
            } else if let Some(key) = status_key {
                let native_id = self
                    .status_ids
                    .get(key)
                    .copied()
                    .filter(|_| same_status_scope)
                    .unwrap_or_else(|| self.allocate());
                node.set_live(if source.node.is_assertive_live_status() {
                    Live::Assertive
                } else {
                    Live::Polite
                });
                status_ids.insert(key.to_owned(), native_id);
                native_id
            } else if same_generation
                && !stable_key.is_some_and(|key| self.status_ids.contains_key(key))
            {
                self.ids
                    .get(&id)
                    .copied()
                    .unwrap_or_else(|| self.allocate())
            } else {
                self.allocate()
            };
            if modal == Some(id) {
                if source.node.is_alert_dialog() {
                    node.set_role(Role::AlertDialog);
                }
                node.set_modal();
            }
            if let Some(heading) = frame
                .dialog_heading_target(id)
                .and_then(|target| ids.get(&target))
            {
                node.set_labelled_by([*heading]);
                node.clear_label();
            }
            if let Some(bounds) = bounds {
                node.set_bounds(bounds);
            }
            if focus.is_some() {
                node.add_action(Action::Focus);
            }
            if press.is_some() {
                node.add_action(Action::Click);
            }
            if adjustment.is_some() {
                node.add_action(Action::SetValue);
                if frame.supports_adjustment_step(id, true) {
                    node.add_action(Action::Increment);
                }
                if frame.supports_adjustment_step(id, false) {
                    node.add_action(Action::Decrement);
                }
            }
            if reveal.is_some() {
                node.add_action(Action::ScrollIntoView);
            }

            if let Some(bounds) = bounds {
                if !pages.is_empty() {
                    let area_id = if same_generation {
                        self.scroll_ids
                            .get(&id)
                            .copied()
                            .unwrap_or_else(|| self.allocate())
                    } else {
                        self.allocate()
                    };
                    let mut area = Node::new(Role::ScrollView);
                    area.set_bounds(bounds);
                    area.set_children(if id == root {
                        std::mem::replace(&mut children, vec![area_id])
                    } else {
                        vec![native_id]
                    });
                    for action in pages.keys() {
                        area.add_action(*action);
                    }
                    nodes.push((area_id, area));
                    scroll_ids.insert(id, area_id);
                    targets.insert(
                        area_id,
                        Targets {
                            pages,
                            ..Targets::default()
                        },
                    );
                }
            }

            node.set_children(children);
            let parts = frame.description_parts(id);
            if !parts.is_empty() {
                descriptions.push((nodes.len(), parts));
            }
            if let Some(tab) = frame.tab_panel_tab(id) {
                node.set_role(Role::TabPanel);
                if source.node.name().trim().is_empty() {
                    node.set_label(frame.node(tab).expect("resolved tab").node.name());
                }
                tab_panels.push((nodes.len(), tab));
            }
            if source.node.tab_controls_target().is_some() {
                tab_positions.insert(id, nodes.len());
            }
            nodes.push((native_id, node));
            ids.insert(id, native_id);
            targets.insert(
                native_id,
                Targets {
                    focus,
                    press,
                    adjustment,
                    reveal,
                    pages: BTreeMap::new(),
                },
            );
        }
        // Both native IDs must exist before linking siblings.
        for (index, tab) in tab_panels {
            if let Some(&tab_index) = tab_positions.get(&tab) {
                let tab_id = nodes[tab_index].0;
                let panel_id = nodes[index].0;
                nodes[index].1.set_labelled_by([tab_id]);
                nodes[index].1.clear_label();
                nodes[tab_index].1.set_controls([panel_id]);
            }
        }
        for (index, parts) in descriptions {
            let mut references = Vec::new();
            let mut help = Vec::new();
            for (description, clipped) in parts {
                if let Some(text) = clipped {
                    // AXHelp can include the clipped validation without a
                    // reference to an unpublished accessibility node.
                    help.push(text);
                } else if let (Some(id), Some(target)) =
                    (ids.get(&description), frame.node(description))
                {
                    references.push(*id);
                    let text = target.node.text_value().unwrap_or(target.node.name());
                    if !text.trim().is_empty() {
                        help.push(text);
                    }
                }
            }
            let anchor = &mut nodes[index].1;
            if !references.is_empty() {
                anchor.set_described_by(references);
            }
            // The adapter reads AXHelp directly, with no text computed from
            // the ID references. Keep the authored order here too.
            if !help.is_empty() {
                anchor.set_description(help.join(" "));
            }
        }
        if modal.is_some() {
            let mut window = Node::new(Role::Window);
            if let Some(root) = ids.get(&root) {
                window.set_children(vec![*root]);
            }
            nodes.push((WINDOW, window));
        }
        let focus = frame
            .focused()
            .and_then(|id| ids.get(&id))
            .copied()
            .unwrap_or(WINDOW);
        self.generation = Some(generation);
        self.ids = ids;
        self.scroll_ids = scroll_ids;
        self.status_ids = status_ids;
        self.modal_status_scope = status_scope.map(|scope| scope.map(str::to_owned));
        self.targets = targets;
        TreeUpdate {
            nodes,
            tree: Some(TreeInfo::new(WINDOW)),
            tree_id: TreeId::ROOT,
            focus,
        }
    }

    pub(crate) fn action<A: Application>(
        &self,
        runtime: &mut Runtime<A>,
        request: ActionRequest,
    ) -> Result<bool, AppError> {
        if request.target_tree != TreeId::ROOT {
            return Ok(false);
        }
        let Some(target) = self.targets.get(&request.target_node) else {
            return Ok(false);
        };
        let scroll_applied = |result| {
            matches!(
                result,
                AccessibilityScrollResult::Scheduled | AccessibilityScrollResult::Rebuilt
            )
        };
        let adjust = |runtime: &mut Runtime<A>, action| {
            let Some(target) = &target.adjustment else {
                return Ok(false);
            };
            Ok(matches!(
                runtime.accessibility_adjust(target, action)?,
                AccessibilityAdjustmentResult::Adjusted | AccessibilityAdjustmentResult::Rebuilt
            ))
        };
        match (request.action, request.data) {
            (Action::Increment, None) => adjust(runtime, AccessibilityAdjustment::Increment),
            (Action::Decrement, None) => adjust(runtime, AccessibilityAdjustment::Decrement),
            (Action::SetValue, Some(ActionData::NumericValue(value))) => {
                adjust(runtime, AccessibilityAdjustment::SetValue(value))
            }
            (Action::SetValue, Some(ActionData::Value(text))) => match text.trim().parse::<f64>() {
                Ok(value) => adjust(runtime, AccessibilityAdjustment::SetValue(value)),
                Err(_) => Ok(false),
            },
            (Action::Focus, None) => {
                let Some(focus) = &target.focus else {
                    return Ok(false);
                };
                Ok(matches!(
                    runtime.accessibility_focus(focus)?,
                    AccessibilityFocusResult::Moved | AccessibilityFocusResult::Rebuilt
                ))
            }
            (Action::Click, None) => {
                let Some(press) = &target.press else {
                    return Ok(false);
                };
                Ok(matches!(
                    runtime.accessibility_activate(press)?,
                    AccessibilityActivationResult::Activated
                        | AccessibilityActivationResult::Rebuilt
                ))
            }
            (Action::ScrollIntoView, None) => {
                let Some(reveal) = &target.reveal else {
                    return Ok(false);
                };
                Ok(scroll_applied(
                    runtime.accessibility_scroll_into_view(reveal)?,
                ))
            }
            (action, Some(ActionData::ScrollUnit(ScrollUnit::Page))) => {
                let Some(page) = target.pages.get(&action) else {
                    return Ok(false);
                };
                Ok(scroll_applied(runtime.accessibility_scroll_page(page)?))
            }
            _ => Ok(false),
        }
    }
}

fn bounds(rect: surface::Rect, scale: f32) -> Option<Rect> {
    let [x, y, width, height] = [rect.x, rect.y, rect.width, rect.height].map(f64::from);
    if width <= 0. || height <= 0. || ![x, y, width, height].into_iter().all(f64::is_finite) {
        return None;
    }
    let scale = f64::from(scale);
    Some(Rect::new(
        x * scale,
        y * scale,
        (x + width) * scale,
        (y + height) * scale,
    ))
}

fn describe(source: &AccessibilityNode<'_>, slider_limits: Option<(f64, f64)>) -> Node {
    let authored = source.node;
    let role = match authored.accessibility_grid_role() {
        Some(GridRole::Grid) => Role::Grid,
        Some(GridRole::Row) => Role::Row,
        Some(GridRole::ColumnHeader) => Role::ColumnHeader,
        Some(GridRole::GridCell) => Role::GridCell,
        None => match authored.role() {
            SurfaceRole::Group if authored.accessibility_adjustable().is_some() => Role::SpinButton,
            SurfaceRole::Group if authored.name().is_empty() => Role::GenericContainer,
            SurfaceRole::Group => Role::Group,
            SurfaceRole::Window => Role::Window,
            SurfaceRole::Heading => Role::Heading,
            SurfaceRole::Text => Role::Label,
            SurfaceRole::Button => Role::Button,
            SurfaceRole::Checkbox => Role::CheckBox,
            SurfaceRole::TextInput if source.text_options.is_some_and(|text| text.multiline) => {
                Role::MultilineTextInput
            }
            SurfaceRole::TextInput => Role::TextInput,
            SurfaceRole::Navigation => Role::Navigation,
            SurfaceRole::Radio => Role::RadioButton,
            SurfaceRole::RadioGroup => Role::RadioGroup,
            SurfaceRole::Switch => Role::Switch,
            SurfaceRole::Slider => Role::Slider,
            SurfaceRole::Tab => Role::Tab,
            SurfaceRole::TabList => Role::TabList,
            SurfaceRole::Menu => Role::Menu,
            SurfaceRole::MenuItem if authored.is_checked().is_some() => Role::MenuItemCheckBox,
            SurfaceRole::MenuItem => Role::MenuItem,
            SurfaceRole::Dialog => Role::Dialog,
            SurfaceRole::Progress => Role::ProgressIndicator,
            SurfaceRole::Status => Role::Status,
            SurfaceRole::Separator => Role::GenericContainer,
            SurfaceRole::Tooltip => Role::Tooltip,
            SurfaceRole::List => Role::List,
            SurfaceRole::ListItem => Role::ListItem,
        },
    };
    let mut node = Node::new(role);
    if role == Role::Slider && authored.is_vertical_slider() {
        node.set_orientation(Orientation::Vertical);
    }
    let label = match authored.role() {
        SurfaceRole::Text | SurfaceRole::Heading | SurfaceRole::Status | SurfaceRole::Tooltip => {
            authored.text_value().unwrap_or(authored.name())
        }
        _ => authored.name(),
    };
    if !label.is_empty() && (!authored.is_assertive_live_status() || !label.trim().is_empty()) {
        node.set_label(label);
    }
    if authored.role() == SurfaceRole::TextInput {
        if let Some(text) = authored.text_value() {
            node.set_value(text);
        }
        if source.text_options.is_some_and(|text| text.read_only) {
            node.set_read_only();
        }
    }
    if authored.is_disabled() {
        node.set_disabled();
    }
    if authored.is_invalid() {
        node.set_invalid(Invalid::True);
    }
    if let Some(selected) = authored.is_selected() {
        node.set_selected(selected);
    }
    if authored.is_grid_multiselectable() {
        node.set_multiselectable();
    }
    if authored.is_current_date() {
        node.set_aria_current(AriaCurrent::Date);
    } else if role != Role::GridCell && authored.is_current_page() == Some(true) {
        node.set_aria_current(AriaCurrent::Page);
    }
    if let Some(expanded) = authored.expanded_state() {
        node.set_expanded(expanded);
    }
    if authored.has_menu_popup() {
        node.set_has_popup(HasPopup::Menu);
    }
    if let Some(value) = authored.is_checked() {
        node.set_toggled(if authored.is_mixed() {
            Toggled::Mixed
        } else {
            value.into()
        });
    }
    if let Some(range) = authored.accessibility_adjustable() {
        node.set_min_numeric_value(range.min);
        node.set_max_numeric_value(range.max);
        node.set_numeric_value_step(range.step);
        if let Some(value) = range.value {
            node.set_numeric_value(value);
        }
        if let Some(text) = &range.value_text {
            node.set_value(text.as_str());
        }
    } else if let Some(range) = authored.range_value() {
        node.set_numeric_value(range.value);
        let (min, max) = if role == Role::Slider {
            slider_limits.unwrap_or((range.min, range.max))
        } else {
            (range.min, range.max)
        };
        node.set_min_numeric_value(min);
        node.set_max_numeric_value(max);
        node.set_numeric_value_step(range.step);
        if let Some(text) = authored.slider_value_text().filter(|_| range.is_valid()) {
            node.set_value(text);
        }
    }
    node
}

#[cfg(test)]
mod tests {
    use super::*;
    use surface::{Length, Node as SurfaceNode, Style, TextMeasurer};
    use surface_app::{Event, View};

    #[derive(Clone, Copy, Default)]
    struct App {
        notice: Option<&'static str>,
        assertive_notice: bool,
        tooltip: bool,
        modal: bool,
        focusable_notice: bool,
        menu_expanded: Option<bool>,
        invalid_editor: Option<bool>,
    }

    impl Application for App {
        type Message = Self;

        fn update(&mut self, state: Self) {
            *self = state;
        }

        fn view(&self) -> View<Self> {
            let button = View::new(
                SurfaceNode::new(SurfaceRole::Button, "Action")
                    .id("action")
                    .text("Action"),
            )
            .on_press(*self)
            .accessible_activation()
            .described_by("action-tip");
            let menu_controls = self.menu_expanded.map(|expanded| {
                [
                    View::new(
                        SurfaceNode::new(SurfaceRole::Button, "Commands")
                            .id("commands")
                            .text("Commands"),
                    )
                    .menu_expanded(expanded),
                    View::new(
                        SurfaceNode::new(SurfaceRole::Button, "Details")
                            .id("details")
                            .text("Details"),
                    )
                    .disclosure_expanded(expanded),
                ]
            });
            let status = self.notice.map(|message| {
                let mut node = SurfaceNode::new(SurfaceRole::Status, message)
                    .id("status")
                    .text(message);
                if self.assertive_notice {
                    node = node
                        .assertive_live_status()
                        .styled(Style::column().height(Length::Px(20.)));
                }
                if self.focusable_notice {
                    node = node.tab_index(0);
                }
                View::new(node)
            });
            let modal = self.modal.then(|| {
                View::new(
                    SurfaceNode::new(SurfaceRole::Dialog, "Settings")
                        .id("settings")
                        .styled(Style::column().height(Length::Px(60.))),
                )
                .modal(Self {
                    modal: false,
                    ..*self
                })
            });
            let tooltip = self.tooltip.then(|| {
                View::new(
                    SurfaceNode::new(SurfaceRole::Tooltip, "")
                        .id("action-tip")
                        .text("Open the item"),
                )
            });
            let editor = self.invalid_editor.map(|invalid| {
                View::new(
                    SurfaceNode::new(SurfaceRole::TextInput, "Quantity")
                        .id("quantity")
                        .text("15")
                        .invalid(invalid),
                )
                .on_input(|_| Self::default())
            });
            View::new(SurfaceNode::new(SurfaceRole::Window, "Desktop").styled(Style::column()))
                .children(
                    std::iter::once(button)
                        .chain(menu_controls.into_iter().flatten())
                        .chain(status)
                        .chain(modal)
                        .chain(tooltip)
                        .chain(editor),
                )
        }
    }

    #[test]
    fn native_tabs_link_only_the_mounted_panel_to_the_projected_tab() {
        #[derive(Clone, Copy)]
        struct Tabs {
            other: bool,
            clipped: bool,
            reveal: bool,
        }

        impl Application for Tabs {
            type Message = Self;

            fn update(&mut self, model: Self) {
                *self = model;
            }

            fn view(&self) -> View<Self> {
                let tab = |key, name, panel, selected| {
                    let tab = View::new(
                        SurfaceNode::new(SurfaceRole::Tab, name)
                            .id(key)
                            .selected(selected)
                            .tab_controls(panel)
                            .styled(Style::column().height(Length::Px(32.))),
                    );
                    if self.reveal {
                        tab.accessibility_scroll_into_view()
                    } else {
                        tab
                    }
                };
                let mut style = Style::column();
                if self.clipped {
                    style.height = Length::Px(32.);
                    style.clip = true;
                }
                let list = View::new(
                    SurfaceNode::new(SurfaceRole::TabList, "Destinations")
                        .id("list")
                        .styled(style),
                )
                .children([
                    tab("history", "History", "history-panel", self.other),
                    tab("messages", "Messages", "messages-panel", !self.other),
                ]);
                let (panel, label) = if self.other {
                    ("history-panel", "history")
                } else {
                    ("messages-panel", "messages")
                };
                let panel = View::new(
                    SurfaceNode::new(SurfaceRole::Group, "")
                        .id(panel)
                        .tab_panel_labelled_by("list", label)
                        .styled(Style::column().height(Length::Px(64.))),
                );
                View::new(SurfaceNode::new(SurfaceRole::Window, "Tabs")).children([panel, list])
            }
        }

        let initial = Tabs {
            other: false,
            clipped: false,
            reveal: false,
        };
        let mut runtime = Runtime::new(initial).unwrap();
        let mut tree = Tree::default();
        for other in [false, true] {
            runtime.apply(Tabs { other, ..initial }).unwrap();
            let frame = update(&mut runtime, &mut tree);
            let (chosen, inactive) = if other {
                ("History", "Messages")
            } else {
                ("Messages", "History")
            };
            let (tab_id, tab) = named(&frame, chosen);
            let (_, inactive) = named(&frame, inactive);
            let mut panels = frame
                .nodes
                .iter()
                .filter(|(_, node)| node.role() == Role::TabPanel);
            let (panel_id, panel) = panels.next().unwrap();
            assert!(panels.next().is_none());
            assert_eq!(tab.role(), Role::Tab);
            assert_eq!(tab.is_selected(), Some(true));
            assert_eq!(tab.controls(), [*panel_id]);
            assert_eq!(panel.labelled_by(), [tab_id]);
            assert!(inactive.controls().is_empty());
        }
        for reveal in [false, true] {
            runtime
                .apply(Tabs {
                    clipped: true,
                    reveal,
                    ..initial
                })
                .unwrap();
            let frame = update(&mut runtime, &mut tree);
            let (panel_id, panel) = frame
                .nodes
                .iter()
                .find(|(_, node)| node.role() == Role::TabPanel)
                .unwrap();
            if reveal {
                let (tab_id, tab) = named(&frame, "Messages");
                assert!(tab.supports_action(Action::ScrollIntoView));
                assert!(!tab.supports_action(Action::Focus));
                assert_eq!(tab.controls(), [*panel_id]);
                assert_eq!(panel.labelled_by(), [tab_id]);
            } else {
                assert_eq!(panel.label(), Some("Messages"));
                assert!(panel.labelled_by().is_empty());
            }
        }
    }

    struct Metrics;

    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
            (text.chars().count() as f32 * size, size)
        }
    }

    fn update<A: Application>(runtime: &mut Runtime<A>, tree: &mut Tree) -> TreeUpdate {
        runtime.render(320., 240., &mut Metrics).unwrap();
        tree.update(runtime.accessibility_frame().unwrap(), 1.)
    }

    fn named<'a>(update: &'a TreeUpdate, name: &str) -> (NodeId, &'a Node) {
        update
            .nodes
            .iter()
            .find_map(|(id, node)| (node.label() == Some(name)).then_some((*id, node)))
            .unwrap()
    }

    #[test]
    fn disabled_menu_item_projects_focus_separately_from_press() {
        #[derive(Default)]
        struct Menu(usize);

        impl Application for Menu {
            type Message = ();

            fn update(&mut self, (): ()) {
                self.0 += 1;
            }

            fn view(&self) -> View<()> {
                let item = |name, disabled| {
                    View::new(
                        SurfaceNode::new(SurfaceRole::MenuItem, name)
                            .id(name)
                            .disabled(disabled)
                            .styled(Style::row().height(Length::Px(48.))),
                    )
                    .on_press(())
                    .accessible_activation()
                };
                View::new(SurfaceNode::new(SurfaceRole::Window, "Desktop")).children([View::new(
                    SurfaceNode::new(SurfaceRole::Menu, "Edit").styled(Style::column()),
                )
                .children([
                    item("Copy", false),
                    item("Paste", true).focus_disabled_menu_item(),
                    item("Undo", true),
                ])])
            }
        }

        let mut runtime = Runtime::new(Menu::default()).unwrap();
        let mut tree = Tree::default();
        let projected = update(&mut runtime, &mut tree);
        let (paste_id, paste) = named(&projected, "Paste");
        assert_eq!(paste.role(), Role::MenuItem);
        assert!(paste.is_disabled());
        assert!(paste.supports_action(Action::Focus));
        assert!(!paste.supports_action(Action::Click));
        let undo = named(&projected, "Undo").1;
        assert!(undo.is_disabled());
        assert!(!undo.supports_action(Action::Focus));
        assert!(!undo.supports_action(Action::Click));
        let request = |action, target_node| ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node,
            data: None,
        };
        assert!(tree
            .action(&mut runtime, request(Action::Focus, paste_id))
            .unwrap());
        assert_eq!(runtime.focused(), Some("Paste"));
        let focused = update(&mut runtime, &mut tree);
        assert_eq!(focused.focus, paste_id);
        assert!(!tree
            .action(&mut runtime, request(Action::Click, paste_id))
            .unwrap());
        assert_eq!(runtime.app().0, 0);
        let copy = named(&focused, "Copy").0;
        assert!(tree
            .action(&mut runtime, request(Action::Click, copy))
            .unwrap());
        assert_eq!(runtime.app().0, 1);
    }

    #[test]
    fn visible_alert_heading_and_description_reach_accesskit_and_rebuild_safely() {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Mode {
            Ordinary,
            Visible,
            Clipped,
            Removed,
            Modeless,
        }

        struct Dialog(Mode);

        impl Application for Dialog {
            type Message = Mode;

            fn update(&mut self, mode: Mode) {
                self.0 = mode;
            }

            fn view(&self) -> View<Mode> {
                let box_at = |height| Style::column().height(Length::Px(height));
                let heading = || {
                    View::new(
                        SurfaceNode::new(SurfaceRole::Heading, "Published heading")
                            .id("heading")
                            .text("Published heading")
                            .styled(box_at(22.)),
                    )
                };
                let description = View::new(
                    SurfaceNode::new(SurfaceRole::Text, "")
                        .id("message")
                        .text("This action removes the draft.")
                        .styled(box_at(22.)),
                );
                let title_view = match self.0 {
                    Mode::Visible => Some(heading()),
                    Mode::Clipped => Some(
                        View::new(
                            SurfaceNode::new(SurfaceRole::Group, "")
                                .id("clip")
                                .styled(box_at(24.).clip(true)),
                        )
                        .children([
                            View::new(SurfaceNode::new(SurfaceRole::Group, "").styled(box_at(50.))),
                            heading(),
                        ]),
                    ),
                    _ => None,
                };
                let mut authored = SurfaceNode::new(SurfaceRole::Dialog, "Fallback name")
                    .id("dialog")
                    .styled(box_at(120.))
                    .described_by_text("message");
                if self.0 != Mode::Ordinary {
                    authored = authored
                        .alert_dialog()
                        .dialog_labelled_by_heading("heading");
                }
                let dialog =
                    View::new(authored).children(title_view.into_iter().chain([description]));
                let (dialog, earlier_heading) = if self.0 == Mode::Modeless {
                    (dialog, Some(heading()))
                } else {
                    (dialog.modal(Mode::Ordinary), None)
                };
                View::new(SurfaceNode::new(SurfaceRole::Window, "Application"))
                    .children(earlier_heading.into_iter().chain([dialog]))
            }
        }

        let mut runtime = Runtime::new(Dialog(Mode::Ordinary)).unwrap();
        let mut tree = Tree::default();
        let ordinary = update(&mut runtime, &mut tree);
        let (_, dialog) = named(&ordinary, "Fallback name");
        assert_eq!(dialog.role(), Role::Dialog);
        assert!(dialog.is_modal());
        assert!(dialog.labelled_by().is_empty());

        runtime.apply(Mode::Visible).unwrap();
        let alert = update(&mut runtime, &mut tree);
        let (heading, _) = named(&alert, "Published heading");
        let (message, _) = named(&alert, "This action removes the draft.");
        let (_, dialog) = alert
            .nodes
            .iter()
            .find(|(_, node)| node.role() == Role::AlertDialog)
            .unwrap();
        assert!(dialog.is_modal());
        assert_eq!(dialog.labelled_by(), &[heading]);
        assert!(dialog.label().is_none());
        assert_eq!(dialog.described_by(), &[message]);
        assert_eq!(dialog.description(), Some("This action removes the draft."));

        for mode in [Mode::Clipped, Mode::Removed, Mode::Modeless] {
            runtime.apply(mode).unwrap();
            if mode == Mode::Clipped {
                runtime.render(320., 240., &mut Metrics).unwrap();
                let heading = runtime.dom().by_id("heading").unwrap();
                let frame = runtime.accessibility_frame().unwrap();
                let layout = frame.node(heading).unwrap().layout;
                assert_eq!(layout.bounds.intersect(layout.clip).height, 0.);
            }
            let next = update(&mut runtime, &mut tree);
            let (_, dialog) = named(&next, "Fallback name");
            let modal = mode != Mode::Modeless;
            assert_eq!(dialog.is_modal(), modal);
            assert_eq!(
                dialog.role(),
                if modal {
                    Role::AlertDialog
                } else {
                    Role::Dialog
                }
            );
            assert!(dialog.labelled_by().is_empty());
        }
    }

    #[test]
    fn inline_list_disclosure_projects_state_and_retires_removed_child_actions() {
        #[derive(Clone, Copy)]
        enum Message {
            Open(bool),
            ChildPress,
        }

        #[derive(Default)]
        struct Disclosure {
            open: bool,
            child_presses: usize,
        }

        impl Application for Disclosure {
            type Message = Message;

            fn update(&mut self, message: Message) {
                match message {
                    Message::Open(open) => self.open = open,
                    Message::ChildPress => self.child_presses += 1,
                }
            }

            fn view(&self) -> View<Message> {
                let header = View::new(
                    SurfaceNode::new(SurfaceRole::Button, "Projects")
                        .id("projects")
                        .styled(Style::column().height(Length::Px(56.))),
                )
                .disclosure_expanded(self.open)
                .on_press(Message::Open(!self.open))
                .accessible_activation();
                let children = self.open.then(|| {
                    let notes = View::new(
                        SurfaceNode::new(SurfaceRole::Button, "Notes")
                            .id("notes")
                            .styled(Style::column().height(Length::Px(56.))),
                    )
                    .on_press(Message::ChildPress)
                    .accessible_activation();
                    let row =
                        View::new(SurfaceNode::new(SurfaceRole::ListItem, "")).children([notes]);
                    let nested = View::new(SurfaceNode::new(SurfaceRole::List, "")).children([row]);
                    View::new(SurfaceNode::new(SurfaceRole::Group, ""))
                        .children([nested])
                        .restore_focus_in_subtree_on_removal("projects")
                });
                let branch = View::new(SurfaceNode::new(SurfaceRole::ListItem, ""))
                    .children(std::iter::once(header).chain(children));
                let list =
                    View::new(SurfaceNode::new(SurfaceRole::List, "Work")).children([branch]);
                View::new(SurfaceNode::new(SurfaceRole::Window, "Window")).children([list])
            }
        }

        let mut runtime = Runtime::new(Disclosure::default()).unwrap();
        let mut tree = Tree::default();
        let request = |action, target_node| ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node,
            data: None,
        };
        let frame = update(&mut runtime, &mut tree);
        let (header_id, header) = named(&frame, "Projects");
        assert_eq!(header.role(), Role::Button);
        assert_eq!(header.is_expanded(), Some(false));
        assert_eq!(header.has_popup(), None);
        assert!(frame
            .nodes
            .iter()
            .all(|(_, node)| node.label() != Some("Notes")));
        assert!(tree
            .action(&mut runtime, request(Action::Focus, header_id))
            .unwrap());
        assert!(!runtime.app().open);

        let frame = update(&mut runtime, &mut tree);
        assert!(tree
            .action(
                &mut runtime,
                request(Action::Click, named(&frame, "Projects").0)
            )
            .unwrap());
        let frame = update(&mut runtime, &mut tree);
        let list = named(&frame, "Work").1;
        let (header_id, header) = named(&frame, "Projects");
        let (notes_id, notes) = named(&frame, "Notes");
        let projected = |key| &frame.nodes.iter().find(|(id, _)| *id == key).unwrap().1;
        assert_eq!(list.role(), Role::List);
        assert_eq!(list.children().len(), 1);
        let row = projected(list.children()[0]);
        assert_eq!(row.role(), Role::ListItem);
        assert_eq!(row.children().len(), 2);
        assert_eq!(row.children()[0], header_id);
        let body = projected(row.children()[1]);
        let nested = projected(body.children()[0]);
        assert_eq!(nested.role(), Role::List);
        let nested_row = projected(nested.children()[0]);
        assert_eq!(nested_row.role(), Role::ListItem);
        assert_eq!(nested_row.children(), [notes_id]);
        assert_eq!(header.is_expanded(), Some(true));
        assert_eq!(header.has_popup(), None);
        assert!(notes.supports_action(Action::Focus));
        assert!(notes.supports_action(Action::Click));
        assert!(tree
            .action(&mut runtime, request(Action::Focus, notes_id))
            .unwrap());
        assert_eq!(runtime.focused(), Some("notes"));
        assert_eq!(runtime.app().child_presses, 0);

        runtime.apply(Message::Open(false)).unwrap();
        assert_eq!(runtime.focused(), Some("projects"));
        assert!(!tree
            .action(&mut runtime, request(Action::Click, notes_id))
            .unwrap());
        let frame = update(&mut runtime, &mut tree);
        assert_eq!(named(&frame, "Projects").1.is_expanded(), Some(false));
        assert!(frame
            .nodes
            .iter()
            .all(|(_, node)| node.label() != Some("Notes")));
        assert_eq!(runtime.app().child_presses, 0);
    }

    #[test]
    fn multi_action_list_projects_sibling_buttons_and_routes_their_actions_independently() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum Command {
            Open,
            Pin,
            Remove,
        }

        #[derive(Default)]
        struct Bookmarks {
            pinned: bool,
            presses: Vec<Command>,
        }

        impl Application for Bookmarks {
            type Message = Command;

            fn update(&mut self, command: Command) {
                self.presses.push(command);
                if command == Command::Pin {
                    self.pinned = !self.pinned;
                }
            }

            fn view(&self) -> View<Command> {
                let button = |id, name, command, checked, disabled| {
                    let mut node = SurfaceNode::new(SurfaceRole::Button, name)
                        .id(id)
                        .disabled(disabled)
                        .styled(Style::row().width(Length::Px(64.)).height(Length::Px(48.)));
                    if let Some(checked) = checked {
                        node = node.checked(checked);
                    }
                    View::new(node).on_press(command).accessible_activation()
                };
                let row = View::new(
                    SurfaceNode::new(SurfaceRole::ListItem, "").styled(Style::row().gap(4.)),
                )
                .children([
                    button("open", "Open bookmark", Command::Open, None, false),
                    button(
                        "pin",
                        "Pin bookmark",
                        Command::Pin,
                        Some(self.pinned),
                        false,
                    ),
                    button("remove", "Remove bookmark", Command::Remove, None, true),
                ]);
                let list =
                    View::new(SurfaceNode::new(SurfaceRole::List, "Bookmarks")).children([row]);
                View::new(SurfaceNode::new(SurfaceRole::Window, "Window")).children([list])
            }
        }

        let mut runtime = Runtime::new(Bookmarks::default()).unwrap();
        let mut tree = Tree::default();
        let frame = update(&mut runtime, &mut tree);
        let list = named(&frame, "Bookmarks").1;
        let (open_id, open) = named(&frame, "Open bookmark");
        let (pin_id, pin) = named(&frame, "Pin bookmark");
        let (remove_id, remove) = named(&frame, "Remove bookmark");
        assert_eq!(list.role(), Role::List);
        assert_eq!(list.children().len(), 1);
        let row_id = list.children()[0];
        let row = &frame.nodes.iter().find(|(id, _)| *id == row_id).unwrap().1;
        assert_eq!(row.role(), Role::ListItem);
        assert_eq!(row.children(), [open_id, pin_id, remove_id]);
        assert!(!row.supports_action(Action::Focus));
        assert!(!row.supports_action(Action::Click));
        for button in [open, pin] {
            assert_eq!(button.role(), Role::Button);
            assert!(button.supports_action(Action::Focus));
            assert!(button.supports_action(Action::Click));
        }
        assert_eq!(open.toggled(), None);
        assert_eq!(pin.toggled(), Some(Toggled::False));
        assert!(remove.is_disabled());
        assert!(!remove.supports_action(Action::Focus));
        assert!(!remove.supports_action(Action::Click));

        let request = |action, target_node| ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node,
            data: None,
        };
        assert!(!tree
            .action(&mut runtime, request(Action::Click, remove_id))
            .unwrap());
        assert!(tree
            .action(&mut runtime, request(Action::Focus, pin_id))
            .unwrap());
        assert_eq!(runtime.focused(), Some("pin"));
        assert!(runtime.app().presses.is_empty());
        let frame = update(&mut runtime, &mut tree);
        assert!(tree
            .action(
                &mut runtime,
                request(Action::Click, named(&frame, "Open bookmark").0)
            )
            .unwrap());
        assert_eq!(runtime.focused(), Some("open"));
        assert_eq!(runtime.app().presses, [Command::Open]);
        let frame = update(&mut runtime, &mut tree);
        assert!(tree
            .action(
                &mut runtime,
                request(Action::Click, named(&frame, "Pin bookmark").0)
            )
            .unwrap());
        assert_eq!(runtime.focused(), Some("pin"));
        assert_eq!(runtime.app().presses, [Command::Open, Command::Pin]);
        let frame = update(&mut runtime, &mut tree);
        assert_eq!(
            named(&frame, "Pin bookmark").1.toggled(),
            Some(Toggled::True)
        );
    }

    #[test]
    fn web_listbox_nodes_keep_native_radio_roles_values_and_actions() {
        struct Choices {
            web_listbox: bool,
            selected: usize,
            presses: usize,
        }
        impl Application for Choices {
            type Message = usize;

            fn update(&mut self, selected: usize) {
                self.selected = selected;
                self.presses += 1;
            }

            fn view(&self) -> View<usize> {
                let project = |node: SurfaceNode| {
                    if self.web_listbox {
                        node.web_listbox_semantics()
                    } else {
                        node
                    }
                };
                let group = View::new(project(
                    SurfaceNode::new(SurfaceRole::RadioGroup, "Delivery")
                        .id("delivery")
                        .styled(Style::column()),
                ))
                .children(
                    ["Standard", "Express", "Unavailable"]
                        .into_iter()
                        .enumerate()
                        .map(|(index, name)| {
                            View::new(project(
                                SurfaceNode::new(SurfaceRole::Radio, name)
                                    .id(name)
                                    .text(name)
                                    .checked(self.selected == index)
                                    .disabled(index == 2),
                            ))
                            .on_press(index)
                            .accessible_activation()
                        }),
                );
                View::new(SurfaceNode::new(SurfaceRole::Window, "Window")).children([group])
            }
        }

        for web_listbox in [false, true] {
            let mut runtime = Runtime::new(Choices {
                web_listbox,
                selected: 0,
                presses: 0,
            })
            .unwrap();
            let mut tree = Tree::default();
            let frame = update(&mut runtime, &mut tree);
            let group = named(&frame, "Delivery").1;
            let (standard_id, standard) = named(&frame, "Standard");
            let (express_id, express) = named(&frame, "Express");
            let (disabled_id, disabled) = named(&frame, "Unavailable");
            assert_eq!(group.role(), Role::RadioGroup);
            assert_eq!(group.children(), [standard_id, express_id, disabled_id]);
            assert_eq!(standard.role(), Role::RadioButton);
            assert_eq!(standard.toggled(), Some(Toggled::True));
            assert_eq!(express.toggled(), Some(Toggled::False));
            assert!(express.supports_action(Action::Focus));
            assert!(express.supports_action(Action::Click));
            assert!(disabled.is_disabled());
            assert!(!disabled.supports_action(Action::Focus));
            assert!(!disabled.supports_action(Action::Click));
            let action = |action, target_node| ActionRequest {
                action,
                target_tree: TreeId::ROOT,
                target_node,
                data: None,
            };
            assert!(tree
                .action(&mut runtime, action(Action::Focus, express_id))
                .unwrap());
            assert_eq!(runtime.focused(), Some("Express"));
            assert_eq!(runtime.app().selected, 0);
            assert_eq!(runtime.app().presses, 0);

            let frame = update(&mut runtime, &mut tree);
            let (express_id, _) = named(&frame, "Express");
            assert!(tree
                .action(&mut runtime, action(Action::Click, express_id))
                .unwrap());
            assert_eq!(runtime.app().selected, 1);
            assert_eq!(runtime.app().presses, 1);
            let frame = update(&mut runtime, &mut tree);
            assert_eq!(named(&frame, "Standard").1.toggled(), Some(Toggled::False));
            assert_eq!(named(&frame, "Express").1.toggled(), Some(Toggled::True));
        }
    }

    #[test]
    fn web_multiselect_listbox_keeps_native_checkbox_roles_and_independent_actions() {
        struct Drinks(u8);

        impl Application for Drinks {
            type Message = u8;

            fn update(&mut self, drink: u8) {
                self.0 ^= drink;
            }

            fn view(&self) -> View<u8> {
                let group = View::new(
                    SurfaceNode::new(SurfaceRole::Group, "Drinks")
                        .id("drinks")
                        .web_multiselect_listbox_semantics()
                        .styled(Style::column()),
                )
                .children(
                    [
                        ("Coffee", 1, false, false),
                        ("Tea", 2, false, false),
                        ("Water", 4, true, false),
                        ("Mixed", 8, false, true),
                    ]
                    .map(|(name, drink, disabled, mixed)| {
                        View::new(
                            SurfaceNode::new(SurfaceRole::Checkbox, name)
                                .id(name)
                                .text(name)
                                .checked(self.0 & drink != 0)
                                .mixed(mixed)
                                .disabled(disabled)
                                .web_multiselect_listbox_semantics(),
                        )
                        .on_press(drink)
                        .accessible_activation()
                    }),
                );
                View::new(SurfaceNode::new(SurfaceRole::Window, "Window")).children([group])
            }
        }

        let mut runtime = Runtime::new(Drinks(1)).unwrap();
        let mut tree = Tree::default();
        let frame = update(&mut runtime, &mut tree);
        let group = named(&frame, "Drinks").1;
        let (coffee_id, coffee) = named(&frame, "Coffee");
        let (tea_id, tea) = named(&frame, "Tea");
        let (water_id, water) = named(&frame, "Water");
        let (mixed_id, mixed) = named(&frame, "Mixed");
        assert_eq!(group.role(), Role::Group);
        assert_eq!(group.children(), [coffee_id, tea_id, water_id, mixed_id]);
        for (checkbox, toggled) in [
            (coffee, Toggled::True),
            (tea, Toggled::False),
            (water, Toggled::False),
            (mixed, Toggled::Mixed),
        ] {
            assert_eq!(checkbox.role(), Role::CheckBox);
            assert_eq!(checkbox.toggled(), Some(toggled));
            assert_eq!(checkbox.is_selected(), None);
        }
        assert!(tea.supports_action(Action::Focus));
        assert!(tea.supports_action(Action::Click));
        assert!(water.is_disabled());
        assert!(!water.supports_action(Action::Focus));
        assert!(!water.supports_action(Action::Click));

        let action = |action, target_node| ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node,
            data: None,
        };
        assert!(tree
            .action(&mut runtime, action(Action::Focus, tea_id))
            .unwrap());
        assert_eq!(runtime.focused(), Some("Tea"));
        assert_eq!(runtime.app().0, 1);
        let frame = update(&mut runtime, &mut tree);
        assert!(tree
            .action(&mut runtime, action(Action::Click, named(&frame, "Tea").0))
            .unwrap());
        assert_eq!(runtime.app().0, 3);
        let frame = update(&mut runtime, &mut tree);
        for label in ["Coffee", "Tea"] {
            assert_eq!(named(&frame, label).1.toggled(), Some(Toggled::True));
        }
    }

    #[test]
    fn adjustable_projection_sets_spinbutton_value_and_routes_current_actions() {
        struct Adjustable {
            value: Option<f64>,
            disabled: bool,
        }
        impl Application for Adjustable {
            type Message = f64;

            fn update(&mut self, value: f64) {
                self.value = Some(value);
            }

            fn view(&self) -> View<Self::Message> {
                let number = View::new(
                    SurfaceNode::new(SurfaceRole::Group, "Minutes")
                        .id("minutes")
                        .disabled(self.disabled)
                        .adjustable(surface::AdjustableValue {
                            min: 0.,
                            max: 59.,
                            step: 1.,
                            value: self.value,
                            value_text: Some(
                                if self.value.is_some() {
                                    "０７"
                                } else {
                                    "未設定"
                                }
                                .into(),
                            ),
                        })
                        .styled(Style::row().width(Length::Px(120.)).height(Length::Px(48.))),
                )
                .on_value(|value| value);
                View::new(SurfaceNode::new(SurfaceRole::Window, "Test")).children([number])
            }
        }

        let mut rt = Runtime::new(Adjustable {
            value: None,
            disabled: false,
        })
        .unwrap();
        let mut tree = Tree::default();
        let rendered = update(&mut rt, &mut tree);
        let (first_id, minute) = named(&rendered, "Minutes");
        assert_eq!(minute.role(), Role::SpinButton);
        assert_eq!(minute.numeric_value(), None);
        assert_eq!(minute.min_numeric_value(), Some(0.));
        assert_eq!(minute.max_numeric_value(), Some(59.));
        assert_eq!(minute.value(), Some("未設定"));
        for action in [
            Action::Increment,
            Action::Decrement,
            Action::SetValue,
            Action::Focus,
        ] {
            assert!(minute.supports_action(action));
        }
        assert!(!minute.supports_action(Action::Click));

        let request = |target_node, action, data| ActionRequest {
            target_tree: TreeId::ROOT,
            target_node,
            action,
            data,
        };
        assert!(tree
            .action(&mut rt, request(first_id, Action::Increment, None))
            .unwrap());
        assert_eq!(rt.app().value, Some(0.));
        assert!(!tree
            .action(&mut rt, request(first_id, Action::Increment, None))
            .unwrap());

        let rendered = update(&mut rt, &mut tree);
        let (id, minute) = named(&rendered, "Minutes");
        assert_ne!(id, first_id);
        assert!(minute.supports_action(Action::Increment));
        assert!(!minute.supports_action(Action::Decrement));
        assert!(tree
            .action(
                &mut rt,
                request(id, Action::SetValue, Some(ActionData::NumericValue(7.)))
            )
            .unwrap());
        assert_eq!(rt.app().value, Some(7.));

        let rendered = update(&mut rt, &mut tree);
        let (id, minute) = named(&rendered, "Minutes");
        assert_eq!(minute.numeric_value(), Some(7.));
        assert_eq!(minute.value(), Some("０７"));
        assert!(tree
            .action(&mut rt, request(id, Action::Decrement, None))
            .unwrap());
        assert_eq!(rt.app().value, Some(6.));

        let mut disabled = Runtime::new(Adjustable {
            value: Some(6.),
            disabled: true,
        })
        .unwrap();
        let rendered = update(&mut disabled, &mut tree);
        let (id, minute) = named(&rendered, "Minutes");
        assert!(minute.is_disabled());
        assert!(!minute.supports_action(Action::Increment));
        assert!(!minute.supports_action(Action::Decrement));
        assert!(!minute.supports_action(Action::SetValue));
        assert!(!tree
            .action(&mut disabled, request(id, Action::Increment, None))
            .unwrap());
        assert_eq!(disabled.app().value, Some(6.));
    }

    #[test]
    fn slider_projection_keeps_number_and_description_and_routes_token_actions() {
        #[derive(Clone, Copy)]
        enum Message {
            Set(f64),
            Disable,
        }

        struct Slider {
            value: f64,
            disabled: bool,
        }
        impl Application for Slider {
            type Message = Message;

            fn update(&mut self, message: Message) {
                match message {
                    Message::Set(value) => self.value = value,
                    Message::Disable => self.disabled = true,
                }
            }

            fn view(&self) -> View<Message> {
                let range = surface::RangeValue {
                    min: 0.,
                    max: 10.,
                    value: self.value,
                    step: 2.,
                };
                let node = |role, name, id| {
                    SurfaceNode::new(role, name)
                        .id(id)
                        .range(range)
                        .styled(Style::row().width(Length::Px(120.)).height(Length::Px(48.)))
                };
                let current = View::new(
                    node(SurfaceRole::Slider, "Volume", "volume")
                        .with_slider_value_text(format!("{} 音量", self.value))
                        .disabled(self.disabled),
                )
                .on_value(Message::Set)
                .accessible_slider_adjustment();
                let legacy =
                    View::new(node(SurfaceRole::Slider, "Legacy", "legacy")).on_value(Message::Set);
                let unbound =
                    View::new(node(SurfaceRole::Slider, "Unbound", "unbound").disabled(true))
                        .accessible_slider_adjustment();
                let progress = View::new(
                    node(SurfaceRole::Progress, "Progress", "progress")
                        .with_slider_value_text("Only for Slider"),
                );
                View::new(SurfaceNode::new(SurfaceRole::Window, "Test").styled(Style::column()))
                    .children([current, legacy, unbound, progress])
            }
        }

        let mut runtime = Runtime::new(Slider {
            value: 4.,
            disabled: false,
        })
        .unwrap();
        let mut tree = Tree::default();
        let actions = [Action::SetValue, Action::Increment, Action::Decrement];
        let request = |target_node, action, data| ActionRequest {
            target_tree: TreeId::ROOT,
            target_node,
            action,
            data,
        };

        for (current, action, data, expected) in [
            (4., Action::Increment, None, 6.),
            (6., Action::Decrement, None, 4.),
            (
                4.,
                Action::SetValue,
                Some(ActionData::NumericValue(9.)),
                10.,
            ),
            (
                10.,
                Action::SetValue,
                Some(ActionData::Value(" 3 ".into())),
                4.,
            ),
        ] {
            let projection = update(&mut runtime, &mut tree);
            let (id, slider) = named(&projection, "Volume");
            assert_eq!(slider.role(), Role::Slider);
            assert_eq!(slider.numeric_value(), Some(current));
            assert_eq!(slider.min_numeric_value(), Some(0.));
            assert_eq!(slider.max_numeric_value(), Some(10.));
            assert_eq!(slider.numeric_value_step(), Some(2.));
            let value_text = format!("{current} 音量");
            assert_eq!(slider.value(), Some(value_text.as_str()));
            assert!(slider.supports_action(Action::Focus));
            assert!(!slider.supports_action(Action::Click));
            assert!(slider.supports_action(Action::SetValue));
            assert!(slider.supports_action(Action::Decrement));
            assert_eq!(slider.supports_action(Action::Increment), current < 10.);
            assert!(tree
                .action(&mut runtime, request(id, action, data.clone()))
                .unwrap());
            assert_eq!(runtime.app().value, expected);
            assert!(!tree
                .action(&mut runtime, request(id, action, data))
                .unwrap());
        }
        assert_eq!(runtime.focused(), None);

        let projection = update(&mut runtime, &mut tree);
        for name in ["Legacy", "Unbound", "Progress"] {
            let (id, node) = named(&projection, name);
            assert_eq!(node.numeric_value(), Some(4.));
            assert_eq!(node.value(), None);
            for action in actions {
                assert!(!node.supports_action(action));
            }
            assert!(!tree
                .action(&mut runtime, request(id, Action::Increment, None))
                .unwrap());
        }
        let (old_id, _) = named(&projection, "Volume");
        runtime.apply(Message::Disable).unwrap();
        assert!(!tree
            .action(&mut runtime, request(old_id, Action::Increment, None))
            .unwrap());
        let projection = update(&mut runtime, &mut tree);
        let (id, disabled) = named(&projection, "Volume");
        assert!(disabled.is_disabled());
        assert_eq!(disabled.value(), Some("4 音量"));
        for action in actions {
            assert!(!disabled.supports_action(action));
        }
        assert!(!tree
            .action(&mut runtime, request(id, Action::Increment, None))
            .unwrap());
        assert_eq!(runtime.app().value, 4.);
    }

    #[test]
    fn vertical_orientation_is_descriptive_only_for_explicit_sliders() {
        struct Sliders;

        impl Application for Sliders {
            type Message = f64;

            fn update(&mut self, _: f64) {}

            fn view(&self) -> View<f64> {
                use surface_app::{SliderInputOptions, SliderScale};

                let node = |role, id| {
                    SurfaceNode::new(role, id)
                        .id(id)
                        .range(surface::RangeValue {
                            min: 0.,
                            max: 10.,
                            value: 4.,
                            step: 2.,
                        })
                        .styled(Style::row().width(Length::Px(48.)).height(Length::Px(120.)))
                };
                let scale = SliderScale::Continuous;
                let vertical = View::new(node(SurfaceRole::Slider, "vertical"))
                    .on_value(|value| value)
                    .with_vertical_slider_input(scale)
                    .accessible_slider_adjustment();
                let disabled = View::new(node(SurfaceRole::Slider, "disabled").disabled(true))
                    .with_vertical_slider_input(scale)
                    .accessible_slider_adjustment();
                let horizontal = View::new(node(SurfaceRole::Slider, "horizontal"))
                    .on_value(|value| value)
                    .with_slider_input(SliderInputOptions {
                        scale,
                        reverse_horizontal: true,
                    });
                let legacy = View::new(node(SurfaceRole::Slider, "legacy")).on_value(|value| value);
                let progress = View::new(node(SurfaceRole::Progress, "progress"))
                    .with_vertical_slider_input(scale);
                View::new(SurfaceNode::new(SurfaceRole::Window, "Sliders").styled(Style::row()))
                    .children([vertical, disabled, horizontal, legacy, progress])
            }
        }

        let mut runtime = Runtime::new(Sliders).unwrap();
        let mut tree = Tree::default();
        let projection = update(&mut runtime, &mut tree);
        for (name, role, orientation) in [
            ("vertical", Role::Slider, Some(Orientation::Vertical)),
            ("disabled", Role::Slider, Some(Orientation::Vertical)),
            ("horizontal", Role::Slider, None),
            ("legacy", Role::Slider, None),
            ("progress", Role::ProgressIndicator, None),
        ] {
            let (_, node) = named(&projection, name);
            assert_eq!(node.role(), role);
            assert_eq!(node.orientation(), orientation);
            assert_eq!(node.numeric_value(), Some(4.));
            assert_eq!(node.is_disabled(), name == "disabled");
            for action in [
                Action::Focus,
                Action::SetValue,
                Action::Increment,
                Action::Decrement,
            ] {
                let expected = name == "vertical"
                    || (matches!(name, "horizontal" | "legacy") && action == Action::Focus);
                assert_eq!(node.supports_action(action), expected, "{name}: {action:?}");
            }
        }
    }

    #[test]
    fn slider_pair_projects_peer_limits_and_keeps_locked_thumbs_named() {
        use surface::RangeValue;
        use surface_app::{SliderInputOptions, SliderScale};

        #[derive(Clone, Copy)]
        enum Message {
            Lower(f64),
            Upper(f64),
            Collapse,
            Disable,
        }

        struct Pair {
            lower: f64,
            upper: f64,
            disabled: bool,
        }

        impl Application for Pair {
            type Message = Message;

            fn update(&mut self, message: Message) {
                match message {
                    Message::Lower(value) => self.lower = value,
                    Message::Upper(value) => self.upper = value,
                    Message::Collapse => {
                        self.lower = 0.;
                        self.upper = 0.;
                    }
                    Message::Disable => self.disabled = true,
                }
            }

            fn view(&self) -> View<Message> {
                let thumb = |name, id, value, on_value: fn(f64) -> Message| {
                    let view = View::new(
                        SurfaceNode::new(SurfaceRole::Slider, name)
                            .id(id)
                            .range(RangeValue {
                                min: 0.,
                                max: 100.,
                                value,
                                step: 5.,
                            })
                            .disabled(self.disabled)
                            .styled(Style::row().width(Length::Px(48.)).height(Length::Px(48.))),
                    )
                    .with_slider_input(SliderInputOptions {
                        scale: SliderScale::Continuous,
                        reverse_horizontal: false,
                    })
                    .accessible_slider_adjustment();
                    if self.disabled {
                        view
                    } else {
                        view.on_value(on_value)
                    }
                };
                let rail = View::new(
                    SurfaceNode::new(SurfaceRole::Group, "")
                        .id("rail")
                        .styled(Style::row().width(Length::Px(180.)).height(Length::Px(48.))),
                )
                .without_interaction(None);
                let pair = View::new(
                    SurfaceNode::new(SurfaceRole::Group, "Price range")
                        .id("price")
                        .disabled(self.disabled)
                        .styled(Style::row()),
                )
                .children([
                    rail,
                    thumb("Lower price", "lower", self.lower, Message::Lower),
                    thumb("Upper price", "upper", self.upper, Message::Upper),
                ])
                .with_horizontal_slider_pair("rail", "lower", "upper");
                View::new(SurfaceNode::new(SurfaceRole::Window, "Test").styled(Style::column()))
                    .children([pair])
            }
        }

        let mut runtime = Runtime::new(Pair {
            lower: 25.,
            upper: 75.,
            disabled: false,
        })
        .unwrap();
        let mut tree = Tree::default();
        let projection = update(&mut runtime, &mut tree);
        let (lower_id, lower) = named(&projection, "Lower price");
        let (_, upper) = named(&projection, "Upper price");
        let range = |node: &Node| {
            assert_eq!(node.role(), Role::Slider);
            [
                node.min_numeric_value().unwrap(),
                node.numeric_value().unwrap(),
                node.max_numeric_value().unwrap(),
            ]
        };
        assert_eq!(range(lower), [0., 25., 75.]);
        assert_eq!(range(upper), [25., 75., 100.]);

        assert!(tree
            .action(
                &mut runtime,
                ActionRequest {
                    target_tree: TreeId::ROOT,
                    target_node: lower_id,
                    action: Action::SetValue,
                    data: Some(ActionData::NumericValue(100.)),
                },
            )
            .unwrap());
        let projection = update(&mut runtime, &mut tree);
        let (_, lower) = named(&projection, "Lower price");
        let (_, upper) = named(&projection, "Upper price");
        assert_eq!(range(lower), [0., 75., 75.]);
        assert_eq!(range(upper), [75., 75., 100.]);
        assert!(!lower.supports_action(Action::Increment));
        assert!(!upper.supports_action(Action::Decrement));

        runtime.apply(Message::Collapse).unwrap();
        let projection = update(&mut runtime, &mut tree);
        let (_, lower) = named(&projection, "Lower price");
        let (_, upper) = named(&projection, "Upper price");
        assert_eq!(range(lower), [0., 0., 0.]);
        assert_eq!(range(upper), [0., 0., 100.]);
        assert!(lower.supports_action(Action::Focus));
        assert!(upper.supports_action(Action::Focus));
        assert!(!lower.supports_action(Action::Increment));
        assert!(!lower.supports_action(Action::Decrement));
        assert!(upper.supports_action(Action::Increment));

        runtime.apply(Message::Disable).unwrap();
        let projection = update(&mut runtime, &mut tree);
        let (_, lower) = named(&projection, "Lower price");
        let (_, upper) = named(&projection, "Upper price");
        assert_eq!(range(lower), [0., 0., 0.]);
        assert_eq!(range(upper), [0., 0., 100.]);
        for node in [lower, upper] {
            assert!(node.is_disabled());
            assert!(!node.supports_action(Action::Focus));
            assert!(!node.supports_action(Action::SetValue));
        }
    }

    #[test]
    fn cyclic_adjustable_projects_and_routes_both_directions_at_bounds() {
        #[derive(Clone, Copy)]
        enum Message {
            Set(f64),
            Step(bool),
        }

        struct Stations {
            ordinal: f64,
            steps: Vec<bool>,
        }

        impl Application for Stations {
            type Message = Message;

            fn update(&mut self, message: Message) {
                self.ordinal = match message {
                    Message::Set(value) => value,
                    Message::Step(increment) => {
                        self.steps.push(increment);
                        3. - self.ordinal
                    }
                };
            }

            fn view(&self) -> View<Message> {
                let field = View::new(
                    SurfaceNode::new(SurfaceRole::Group, "Station")
                        .id("station")
                        .adjustable(surface::AdjustableValue {
                            min: 1.,
                            max: 2.,
                            step: 1.,
                            value: Some(self.ordinal),
                            value_text: Some(
                                if self.ordinal == 1. {
                                    "北站"
                                } else {
                                    "南站"
                                }
                                .into(),
                            ),
                        })
                        .styled(Style::row().width(Length::Px(120.)).height(Length::Px(48.))),
                )
                .on_value(Message::Set)
                .on_accessible_step(Message::Step);
                View::new(SurfaceNode::new(SurfaceRole::Window, "Test")).children([field])
            }
        }

        let mut runtime = Runtime::new(Stations {
            ordinal: 2.,
            steps: Vec::new(),
        })
        .unwrap();
        let mut tree = Tree::default();
        let request = |target_node, action, data| ActionRequest {
            target_tree: TreeId::ROOT,
            target_node,
            action,
            data,
        };
        for (at, name, action, expected) in [
            (2., "南站", Action::Increment, 1.),
            (1., "北站", Action::Decrement, 2.),
        ] {
            let rendered = update(&mut runtime, &mut tree);
            let (id, station) = named(&rendered, "Station");
            assert_eq!(station.role(), Role::SpinButton);
            assert_eq!(station.min_numeric_value(), Some(1.));
            assert_eq!(station.max_numeric_value(), Some(2.));
            assert_eq!(station.numeric_value(), Some(at));
            assert_eq!(station.value(), Some(name));
            for action in [Action::Increment, Action::Decrement, Action::SetValue] {
                assert!(station.supports_action(action));
            }
            assert!(tree
                .action(&mut runtime, request(id, action, None))
                .unwrap());
            assert_eq!(runtime.app().ordinal, expected);
            assert!(!tree
                .action(&mut runtime, request(id, action, None))
                .unwrap());
        }
        assert_eq!(runtime.app().steps, [true, false]);
        assert_eq!(runtime.focused(), None);

        let rendered = update(&mut runtime, &mut tree);
        let (id, _) = named(&rendered, "Station");
        let set = |value| request(id, Action::SetValue, Some(ActionData::NumericValue(value)));
        assert!(!tree.action(&mut runtime, set(f64::NAN)).unwrap());
        assert!(!tree.action(&mut runtime, set(90.)).unwrap());
        assert!(tree.action(&mut runtime, set(-90.)).unwrap());
        assert_eq!(runtime.app().ordinal, 1.);
        assert_eq!(runtime.app().steps, [true, false]);
    }

    struct Calendar(bool);

    impl Application for Calendar {
        type Message = bool;

        fn update(&mut self, selected: bool) {
            self.0 = selected;
        }

        fn view(&self) -> View<bool> {
            let grid = View::new(
                SurfaceNode::new(SurfaceRole::Group, "November 2026")
                    .grid_role(GridRole::Grid)
                    .styled(Style::column()),
            )
            .children([
                View::new(
                    SurfaceNode::new(SurfaceRole::Group, "Weekdays").grid_role(GridRole::Row),
                )
                .children([View::new(
                    SurfaceNode::new(SurfaceRole::Group, "Monday")
                        .grid_role(GridRole::ColumnHeader)
                        .text("Mo"),
                )]),
                View::new(SurfaceNode::new(SurfaceRole::Group, "Week 4").grid_role(GridRole::Row))
                    .children([
                        View::new(
                            SurfaceNode::new(SurfaceRole::Button, "Monday, November 23, 2026")
                                .id("today")
                                .grid_role(GridRole::GridCell)
                                .text("23")
                                .selected(self.0)
                                .current_date(true),
                        )
                        .on_press(true)
                        .accessible_activation(),
                        View::new(
                            SurfaceNode::new(SurfaceRole::Button, "Tuesday, November 24, 2026")
                                .id("unavailable")
                                .grid_role(GridRole::GridCell)
                                .text("24")
                                .disabled(true),
                        )
                        .on_press(true)
                        .accessible_activation(),
                    ]),
            ]);
            View::new(SurfaceNode::new(SurfaceRole::Window, "Dates").styled(Style::column()))
                .children([
                    grid,
                    View::new(
                        SurfaceNode::new(SurfaceRole::TextInput, "Due date")
                            .id("due-date")
                            .text("2026-11-23")
                            .described_by_text("format"),
                    )
                    .on_input(|_| false),
                    View::new(
                        SurfaceNode::new(SurfaceRole::Text, "")
                            .id("format")
                            .text("YYYY-MM-DD"),
                    ),
                    View::new(
                        SurfaceNode::new(SurfaceRole::Button, "Schedule")
                            .id("schedule")
                            .text("Schedule")
                            .current_page(true),
                    ),
                ])
        }
    }

    #[test]
    fn accesskit_calendar_projects_date_roles_state_and_editor_help() {
        let mut runtime = Runtime::new(Calendar(false)).unwrap();
        let mut tree = Tree::default();
        let frame = update(&mut runtime, &mut tree);
        let grid = named(&frame, "November 2026").1;
        let (header_row_id, header_row) = named(&frame, "Weekdays");
        let (row_id, row) = named(&frame, "Week 4");
        let (column_id, column) = named(&frame, "Monday");
        let (today_id, today) = named(&frame, "Monday, November 23, 2026");
        let (disabled_id, disabled) = named(&frame, "Tuesday, November 24, 2026");
        assert_eq!(grid.role(), Role::Grid);
        assert!(!grid.is_multiselectable());
        assert_eq!(row.role(), Role::Row);
        assert_eq!(column.role(), Role::ColumnHeader);
        assert_eq!(grid.children(), [header_row_id, row_id]);
        assert_eq!(header_row.children(), [column_id]);
        assert_eq!(row.children(), [today_id, disabled_id]);
        assert_eq!(today.role(), Role::GridCell);
        assert_eq!(today.aria_current(), Some(AriaCurrent::Date));
        assert_eq!(today.is_selected(), Some(false));
        assert!(today.supports_action(Action::Focus));
        assert!(today.supports_action(Action::Click));
        assert_eq!(disabled.is_selected(), None);
        assert!(!disabled.supports_action(Action::Focus));
        assert!(!disabled.supports_action(Action::Click));

        let format = named(&frame, "YYYY-MM-DD").0;
        let field = named(&frame, "Due date").1;
        assert_eq!(field.described_by(), [format]);
        assert_eq!(field.description(), Some("YYYY-MM-DD"));
        assert_eq!(
            named(&frame, "Schedule").1.aria_current(),
            Some(AriaCurrent::Page)
        );

        assert!(tree
            .action(
                &mut runtime,
                ActionRequest {
                    action: Action::Click,
                    target_tree: TreeId::ROOT,
                    target_node: today_id,
                    data: None,
                }
            )
            .unwrap());
        let frame = update(&mut runtime, &mut tree);
        let (current, selected) = named(&frame, "Monday, November 23, 2026");
        assert_eq!(frame.focus, current);
        assert_eq!(selected.is_selected(), Some(true));
        assert_eq!(selected.aria_current(), Some(AriaCurrent::Date));
    }

    #[test]
    fn accesskit_range_grid_projects_both_selected_cells() {
        struct RangeCalendar;

        impl Application for RangeCalendar {
            type Message = ();

            fn update(&mut self, (): ()) {}

            fn view(&self) -> View<()> {
                let cell = |name, selected, disabled| {
                    let mut node = SurfaceNode::new(SurfaceRole::Button, name)
                        .id(name)
                        .text(name)
                        .grid_role(GridRole::GridCell)
                        .disabled(disabled);
                    if let Some(selected) = selected {
                        node = node.selected(selected);
                    }
                    View::new(node).on_press(()).accessible_activation()
                };
                let row = |name| {
                    View::new(SurfaceNode::new(SurfaceRole::Group, name).grid_role(GridRole::Row))
                };
                View::new(SurfaceNode::new(SurfaceRole::Window, "Dates")).children([View::new(
                    SurfaceNode::new(SurfaceRole::Group, "November 2026")
                        .grid_role(GridRole::Grid)
                        .grid_multiselectable(true)
                        .styled(Style::column()),
                )
                .children([
                    row("Week 3").children([
                        cell("Start", Some(true), false),
                        cell("Interior", Some(false), false),
                    ]),
                    row("Week 4").children([
                        cell("Unavailable", None, true),
                        cell("End", Some(true), false),
                    ]),
                ])])
            }
        }

        let mut runtime = Runtime::new(RangeCalendar).unwrap();
        let mut tree = Tree::default();
        let frame = update(&mut runtime, &mut tree);
        let grid = named(&frame, "November 2026").1;
        assert_eq!(grid.role(), Role::Grid);
        assert!(grid.is_multiselectable());
        assert_eq!(
            grid.children(),
            [named(&frame, "Week 3").0, named(&frame, "Week 4").0]
        );
        for (name, selected) in [
            ("Start", Some(true)),
            ("End", Some(true)),
            ("Interior", Some(false)),
            ("Unavailable", None),
        ] {
            let day = named(&frame, name).1;
            assert_eq!(day.role(), Role::GridCell);
            assert_eq!(day.is_selected(), selected);
            assert!(!day.is_multiselectable());
        }
    }

    #[test]
    fn accesskit_split_chip_keeps_two_named_actions_in_separate_grid_cells() {
        #[derive(Default)]
        struct Recipients {
            edits: usize,
            removed: bool,
        }
        #[derive(Clone)]
        enum Message {
            Edit,
            Remove,
        }
        impl Application for Recipients {
            type Message = Message;

            fn update(&mut self, message: Message) {
                match message {
                    Message::Edit => self.edits += 1,
                    Message::Remove => self.removed = true,
                }
            }

            fn view(&self) -> View<Message> {
                let cell = |id, name, message| {
                    View::new(
                        SurfaceNode::new(SurfaceRole::Group, "").grid_role(GridRole::GridCell),
                    )
                    .children([View::new(
                        SurfaceNode::new(SurfaceRole::Button, name)
                            .id(id)
                            .styled(Style::row().width(Length::Px(48.)).height(Length::Px(48.))),
                    )
                    .on_press(message)
                    .accessible_activation()])
                };
                let chip = (!self.removed).then(|| {
                    View::new(SurfaceNode::new(SurfaceRole::Group, "").styled(Style::row()))
                        .children([
                            cell("edit", "小陳", Message::Edit),
                            cell("remove", "移除小陳", Message::Remove),
                        ])
                });
                let row =
                    View::new(SurfaceNode::new(SurfaceRole::Group, "").grid_role(GridRole::Row))
                        .children(chip);
                let grid = View::new(
                    SurfaceNode::new(SurfaceRole::Group, "收件人")
                        .id("recipients")
                        .grid_role(GridRole::Grid),
                )
                .children([row])
                .roving_grid();
                View::new(SurfaceNode::new(SurfaceRole::Window, "Chips")).children([grid])
            }
        }

        let mut runtime = Runtime::new(Recipients::default()).unwrap();
        let mut tree = Tree::default();
        let frame = update(&mut runtime, &mut tree);
        let grid = named(&frame, "收件人").1;
        assert_eq!(grid.role(), Role::Grid);
        let (edit_id, edit) = named(&frame, "小陳");
        let (remove_id, remove) = named(&frame, "移除小陳");
        assert_eq!(edit.role(), Role::Button);
        assert_eq!(remove.role(), Role::Button);
        assert!(edit.supports_action(Action::Click));
        assert!(remove.supports_action(Action::Click));
        let cells: Vec<_> = frame
            .nodes
            .iter()
            .filter_map(|(_, node)| (node.role() == Role::GridCell).then_some(node.children()))
            .collect();
        assert_eq!(cells.len(), 2);
        assert!(cells.contains(&&[edit_id][..]));
        assert!(cells.contains(&&[remove_id][..]));

        let click = |target_node| ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node,
            data: None,
        };
        assert!(tree.action(&mut runtime, click(edit_id)).unwrap());
        assert_eq!(runtime.app().edits, 1);
        assert!(!runtime.app().removed);
        let frame = update(&mut runtime, &mut tree);
        let remove = named(&frame, "移除小陳").0;
        assert!(tree.action(&mut runtime, click(remove)).unwrap());
        assert_eq!(runtime.app().edits, 1);
        assert!(runtime.app().removed);
        let frame = update(&mut runtime, &mut tree);
        assert!(frame
            .nodes
            .iter()
            .all(|(_, node)| node.role() != Role::GridCell));
        assert!(!tree.action(&mut runtime, click(remove)).unwrap());
    }

    #[test]
    fn an_accesskit_menu_trigger_reports_popup_and_expansion() {
        let mut runtime = Runtime::new(App::default()).unwrap();
        let mut tree = Tree::default();
        for open in [false, true] {
            runtime
                .apply(App {
                    menu_expanded: Some(open),
                    ..App::default()
                })
                .unwrap();
            let view = update(&mut runtime, &mut tree);
            let trigger = named(&view, "Commands").1;
            assert_eq!(trigger.role(), Role::Button);
            assert_eq!(trigger.is_expanded(), Some(open));
            assert_eq!(trigger.has_popup(), Some(HasPopup::Menu));
            let disclosure = named(&view, "Details").1;
            assert_eq!(disclosure.is_expanded(), Some(open));
            assert_eq!(disclosure.has_popup(), None);
        }
    }

    #[test]
    fn editor_validation_reaches_accesskit_and_clears_on_correction() {
        let mut runtime = Runtime::new(App::default()).unwrap();
        let mut tree = Tree::default();
        for invalid in [true, false] {
            runtime
                .apply(App {
                    invalid_editor: Some(invalid),
                    ..App::default()
                })
                .unwrap();
            let view = update(&mut runtime, &mut tree);
            let field = named(&view, "Quantity").1;
            assert_eq!(field.role(), Role::TextInput);
            assert_eq!(field.invalid(), invalid.then_some(Invalid::True));
        }
    }

    #[test]
    fn multiline_text_projects_value_description_and_focus_by_mode() {
        use surface_app::TextOptions;

        const VALUE: &str = "第一行\nsecond a\u{301}\n";
        const NAMES: [&str; 3] = ["多行備註", "唯讀內容", "停用內容"];
        const PREFIX: &str = "井字號";
        const HELP: &str = "可輸入多行文字。";

        struct Form;

        impl Application for Form {
            type Message = ();

            fn update(&mut self, _: ()) {}

            fn view(&self) -> View<()> {
                let editor = |id, name, read_only, disabled| {
                    View::new(
                        SurfaceNode::new(SurfaceRole::TextInput, name)
                            .id(id)
                            .text(VALUE)
                            .disabled(disabled)
                            .described_by_text("help")
                            .described_by_texts(["prefix"])
                            .styled(Style::column().height(Length::Px(44.))),
                    )
                    .text_options(TextOptions {
                        multiline: true,
                        read_only,
                        ..Default::default()
                    })
                    .on_input(|_| ())
                    .accessibility_scroll_into_view()
                };
                let description = |id, content| {
                    View::new(SurfaceNode::new(SurfaceRole::Text, "").id(id).text(content))
                };
                View::new(SurfaceNode::new(SurfaceRole::Window, "Notes")).children([
                    editor("notes", NAMES[0], false, false),
                    editor("readonly", NAMES[1], true, false),
                    editor("disabled", NAMES[2], false, true),
                    description("prefix", PREFIX),
                    description("help", HELP),
                ])
            }
        }

        let mut runtime = Runtime::new(Form).unwrap();
        let mut tree = Tree::default();
        let frame = update(&mut runtime, &mut tree);
        let prefix = named(&frame, PREFIX).0;
        let help = named(&frame, HELP).0;
        let fields = NAMES.map(|name| named(&frame, name));
        assert_eq!(
            frame
                .nodes
                .iter()
                .filter(|(_, node)| node.role() == Role::MultilineTextInput)
                .count(),
            fields.len()
        );
        for (_, field) in fields {
            assert_eq!(field.role(), Role::MultilineTextInput);
            assert_eq!(field.value(), Some(VALUE));
        }
        for (_, field) in &fields[..2] {
            assert_eq!(field.described_by(), [prefix, help]);
            assert_eq!(field.description(), Some("井字號 可輸入多行文字。"));
        }
        assert!(!fields[0].1.is_read_only());
        assert!(fields[1].1.is_read_only());
        assert!(fields[2].1.is_disabled());
        assert!(fields[2].1.described_by().is_empty());
        assert!(!fields[2].1.supports_action(Action::Focus));

        for expected in [fields[0].0, fields[1].0, fields[0].0] {
            runtime
                .dispatch(Event::FocusNext { reverse: false })
                .unwrap();
            assert_eq!(update(&mut runtime, &mut tree).focus, expected);
        }
    }

    #[test]
    fn clipped_validation_text_maps_to_accesskit_help_without_a_dangling_relation() {
        use surface_app::ScrollTarget;

        const ERROR: &str = "請輸入偶數";

        struct Validation(bool);

        impl Application for Validation {
            type Message = bool;

            fn update(&mut self, invalid: bool) {
                self.0 = invalid;
            }

            fn view(&self) -> View<bool> {
                let sized = |height| Style::column().height(Length::Px(height));
                let editor = View::new(
                    SurfaceNode::new(SurfaceRole::TextInput, "Quantity")
                        .id("quantity")
                        .text(if self.0 { "15" } else { "16" })
                        .invalid(self.0)
                        .described_by_text("validation-error")
                        .styled(sized(48.)),
                )
                .on_input(|_| false);
                let scroller = View::new(
                    SurfaceNode::new(SurfaceRole::Group, "Validation details")
                        .id("validation-scroll")
                        .styled(sized(48.).clip(true)),
                )
                .children([
                    View::new(
                        SurfaceNode::new(SurfaceRole::Text, "")
                            .id("validation-error")
                            .text(ERROR)
                            .styled(sized(20.)),
                    ),
                    View::new(SurfaceNode::new(SurfaceRole::Group, "").styled(sized(96.))),
                ]);
                let dialog = View::new(
                    SurfaceNode::new(SurfaceRole::Dialog, "Edit quantity")
                        .id("validation-dialog")
                        .styled(Style::column()),
                )
                .children([editor, scroller])
                .modal(false)
                .modal_initial_focus("quantity");
                View::new(SurfaceNode::new(SurfaceRole::Window, "Application")).children([dialog])
            }
        }

        let mut runtime = Runtime::new(Validation(true)).unwrap();
        let mut tree = Tree::default();
        let visible = update(&mut runtime, &mut tree);
        let (visible_error, _) = named(&visible, ERROR);
        let (_, visible_editor) = named(&visible, "Quantity");
        assert_eq!(visible_editor.described_by(), &[visible_error]);
        assert_eq!(visible_editor.description(), Some(ERROR));

        assert!(runtime.scroll_to("validation-scroll", ScrollTarget::Bottom));
        let clipped = update(&mut runtime, &mut tree);
        let target = runtime.dom().by_id("validation-error").unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        let error = frame
            .node(target)
            .expect("clipped error is still mounted in scope");
        let bounds = error.layout.bounds;
        let visible_bounds = bounds.intersect(error.layout.clip);
        assert!(bounds.width > 0. && bounds.height > 0.);
        assert!(visible_bounds.width <= 0. || visible_bounds.height <= 0.);
        assert!(!clipped
            .nodes
            .iter()
            .any(|(_, node)| node.label() == Some(ERROR)));
        let (editor_id, editor) = named(&clipped, "Quantity");
        assert_eq!(editor_id, clipped.focus);
        assert!(editor.supports_action(Action::Focus));
        assert_eq!(editor.invalid(), Some(Invalid::True));
        assert!(editor.described_by().is_empty());
        assert_eq!(editor.description(), Some(ERROR));

        runtime.apply(false).unwrap();
        assert!(runtime.scroll_to("validation-scroll", ScrollTarget::Bottom));
        let corrected = update(&mut runtime, &mut tree);
        let (_, editor) = named(&corrected, "Quantity");
        assert_eq!(editor.invalid(), None);
        assert!(editor.described_by().is_empty());
        assert_eq!(editor.description(), None);
    }

    #[test]
    fn tooltip_description_resolves_to_the_published_accesskit_node() {
        let mut runtime = Runtime::new(App {
            tooltip: true,
            ..App::default()
        })
        .unwrap();
        let mut tree = Tree::default();
        let shown = update(&mut runtime, &mut tree);
        let (id, tooltip) = named(&shown, "Open the item");
        assert_eq!(tooltip.role(), Role::Tooltip);
        assert_eq!(named(&shown, "Action").1.described_by(), &[id]);
        assert_eq!(
            named(&shown, "Action").1.description(),
            Some("Open the item")
        );
        assert!(!tooltip.supports_action(Action::Focus));
        assert!(!tooltip.supports_action(Action::Click));

        runtime.apply(App::default()).unwrap();
        let closed = update(&mut runtime, &mut tree);
        assert!(named(&closed, "Action").1.described_by().is_empty());
        assert_eq!(named(&closed, "Action").1.description(), None);
        assert!(!closed
            .nodes
            .iter()
            .any(|(_, node)| node.role() == Role::Tooltip));
    }

    #[test]
    fn ordered_descriptions_keep_clipped_validation_in_ax_help_without_an_id() {
        use surface::Insets;

        #[derive(Clone, Copy)]
        struct Amount {
            clipped: bool,
        }
        impl Application for Amount {
            type Message = Self;
            fn update(&mut self, message: Self) {
                *self = message;
            }
            fn view(&self) -> View<Self> {
                let error_style = if self.clipped {
                    Style::default().width(Length::Px(100.)).fixed(Insets {
                        left: Length::Px(-220.),
                        ..Default::default()
                    })
                } else {
                    Style::default()
                };
                let editor = View::new(
                    SurfaceNode::new(SurfaceRole::TextInput, "Amount")
                        .id("amount")
                        .text("12")
                        .invalid(true)
                        .described_by_texts(["prefix", "suffix", "prefix", "error"])
                        .described_by_text("error"),
                )
                .on_input(|_| Amount { clipped: false });
                View::new(SurfaceNode::new(SurfaceRole::Window, "App").styled(Style::column()))
                    .children([
                        editor,
                        View::new(
                            SurfaceNode::new(SurfaceRole::Text, "")
                                .id("suffix")
                                .text("公斤"),
                        ),
                        View::new(
                            SurfaceNode::new(SurfaceRole::Text, "")
                                .id("prefix")
                                .text("歐元"),
                        ),
                        View::new(
                            SurfaceNode::new(SurfaceRole::Text, "")
                                .id("error")
                                .text("Use an integer.")
                                .styled(error_style),
                        ),
                    ])
            }
        }

        let mut runtime = Runtime::new(Amount { clipped: false }).unwrap();
        let mut tree = Tree::default();
        for clipped in [false, true] {
            runtime.apply(Amount { clipped }).unwrap();
            let view = update(&mut runtime, &mut tree);
            let (_, editor) = named(&view, "Amount");
            let mut expected = vec![named(&view, "歐元").0, named(&view, "公斤").0];
            if clipped {
                assert!(view
                    .nodes
                    .iter()
                    .all(|(_, node)| node.label() != Some("Use an integer.")));
            } else {
                expected.push(named(&view, "Use an integer.").0);
            }
            assert_eq!(editor.described_by(), expected);
            assert_eq!(editor.description(), Some("歐元 公斤 Use an integer."));
        }
    }

    #[test]
    fn live_status_keeps_its_passive_identity_without_retaining_an_action() {
        let initial = App {
            notice: Some("Saved"),
            ..App::default()
        };
        let mut runtime = Runtime::new(initial).unwrap();
        let mut tree = Tree::default();
        runtime.render(320., 240., &mut Metrics).unwrap();
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        let first = update(&mut runtime, &mut tree);
        let (status_id, status) = named(&first, "Saved");
        assert_eq!(status.role(), Role::Status);
        assert_eq!(status.live(), Some(Live::Polite));
        assert!(!status.supports_action(Action::Focus));
        assert!(!status.supports_action(Action::Click));
        let old_action = named(&first, "Action").0;
        assert_eq!(first.focus, old_action);

        runtime.apply(initial).unwrap();
        let unchanged = update(&mut runtime, &mut tree);
        assert_eq!(named(&unchanged, "Saved").0, status_id);
        assert_eq!(unchanged.focus, named(&unchanged, "Action").0);
        assert_ne!(unchanged.focus, old_action);
        assert!(!tree
            .action(
                &mut runtime,
                ActionRequest {
                    action: Action::Click,
                    target_tree: TreeId::ROOT,
                    target_node: old_action,
                    data: None,
                }
            )
            .unwrap());

        runtime
            .apply(App {
                notice: Some("Updated"),
                ..initial
            })
            .unwrap();
        let changed = update(&mut runtime, &mut tree);
        assert_eq!(named(&changed, "Updated").0, status_id);
        assert_eq!(named(&changed, "Updated").1.live(), Some(Live::Polite));
        assert_eq!(changed.focus, named(&changed, "Action").0);

        runtime
            .apply(App {
                focusable_notice: true,
                ..initial
            })
            .unwrap();
        let active = update(&mut runtime, &mut tree);
        let (active_id, active_node) = named(&active, "Saved");
        assert_ne!(active_id, status_id);
        assert!(active_node.supports_action(Action::Focus));
        assert_eq!(active_node.live(), None);
    }

    #[test]
    fn assertive_status_keeps_the_blank_region_and_only_updates_current_scope() {
        let blank = App {
            notice: Some(""),
            assertive_notice: true,
            ..App::default()
        };
        let mut runtime = Runtime::new(blank).unwrap();
        let mut tree = Tree::default();
        let initial = update(&mut runtime, &mut tree);
        let (id, status) = initial
            .nodes
            .iter()
            .find(|(_, node)| node.role() == Role::Status)
            .expect("the blank live status has a stable identity");
        let id = *id;
        assert_eq!(status.live(), Some(Live::Assertive));
        assert_eq!(status.label(), None);
        assert!(!status.supports_action(Action::Focus));
        assert!(!status.supports_action(Action::Click));

        for notice in ["請輸入有效的專案名稱。", "Enter a valid project name."] {
            let show = App {
                notice: Some(notice),
                ..blank
            };
            runtime.apply(show).unwrap();
            let first = update(&mut runtime, &mut tree);
            let (changed_id, changed) = named(&first, notice);
            assert_eq!(changed_id, id);
            assert_eq!(changed.live(), Some(Live::Assertive));
            runtime.apply(show).unwrap();
            let rebuilt = update(&mut runtime, &mut tree);
            let (rebuilt_id, rebuilt_node) = named(&rebuilt, notice);
            assert_eq!(rebuilt_id, id);
            assert_eq!(rebuilt_node, changed);
        }

        runtime.apply(blank).unwrap();
        let cleared = update(&mut runtime, &mut tree);
        let cleared_status = &cleared.nodes.iter().find(|(key, _)| *key == id).unwrap().1;
        assert_eq!(cleared_status.live(), Some(Live::Assertive));
        assert_eq!(cleared_status.label(), None);

        let show = App {
            notice: Some("請輸入名稱。"),
            ..blank
        };
        runtime
            .apply(App {
                modal: true,
                ..show
            })
            .unwrap();
        let scoped = update(&mut runtime, &mut tree);
        assert!(scoped.nodes.iter().all(|(_, node)| node.live().is_none()));
        runtime.apply(show).unwrap();
        let restored = update(&mut runtime, &mut tree);
        let (restored_id, restored_status) = named(&restored, "請輸入名稱。");
        assert_ne!(restored_id, id);
        assert_eq!(restored_status.live(), Some(Live::Assertive));
    }

    #[test]
    fn hidden_and_out_of_scope_statuses_start_a_new_live_identity_on_return() {
        let shown = App {
            notice: Some("Saved"),
            ..App::default()
        };
        let mut runtime = Runtime::new(shown).unwrap();
        let mut tree = Tree::default();
        let first = named(&update(&mut runtime, &mut tree), "Saved").0;

        runtime.apply(App::default()).unwrap();
        let hidden = update(&mut runtime, &mut tree);
        assert!(hidden.nodes.iter().all(|(_, node)| node.live().is_none()));
        runtime.apply(shown).unwrap();
        let restored = named(&update(&mut runtime, &mut tree), "Saved").0;
        assert_ne!(restored, first);

        runtime
            .apply(App {
                modal: true,
                ..shown
            })
            .unwrap();
        let scoped = update(&mut runtime, &mut tree);
        assert!(scoped.nodes.iter().all(|(_, node)| node.live().is_none()));
        runtime.apply(shown).unwrap();
        let after_modal = named(&update(&mut runtime, &mut tree), "Saved").0;
        assert_ne!(after_modal, restored);
    }
}
