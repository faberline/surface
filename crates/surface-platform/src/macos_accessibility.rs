//! Project the current application scope into the macOS accessibility tree.
use accesskit::{
    Action, ActionData, ActionRequest, AriaCurrent, Node, NodeId, Rect, Role, ScrollUnit, Toggled,
    TreeId, TreeInfo, TreeUpdate,
};
use std::collections::BTreeMap;
use surface::{NodeId as SurfaceId, Role as SurfaceRole};
use surface_app::{
    AccessibilityActivationResult, AccessibilityActivationTarget, AccessibilityFocusResult,
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
    reveal: Option<AccessibilityScrollTarget>,
    pages: BTreeMap<Action, AccessibilityScrollPageTarget>,
}

#[derive(Default)]
pub(crate) struct Tree {
    next_id: u64,
    generation: Option<AccessibilityFrameId>,
    ids: BTreeMap<SurfaceId, NodeId>,
    scroll_ids: BTreeMap<SurfaceId, NodeId>,
    targets: BTreeMap<NodeId, Targets>,
}

impl Tree {
    pub(crate) fn reset(&mut self) {
        self.generation = None;
        self.ids.clear();
        self.scroll_ids.clear();
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

        let mut ids = BTreeMap::new();
        let mut scroll_ids = BTreeMap::new();
        let mut targets = BTreeMap::new();
        let mut nodes = Vec::new();
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

            let mut node = describe(&source);
            let focus = frame.focus_target(id);
            let press = frame.activation_target(id);
            let native_id = if id == root && modal.is_none() {
                node.set_role(Role::Window);
                WINDOW
            } else if same_generation {
                self.ids
                    .get(&id)
                    .copied()
                    .unwrap_or_else(|| self.allocate())
            } else {
                self.allocate()
            };
            if modal == Some(id) {
                node.set_modal();
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
            if reveal.is_some() {
                node.add_action(Action::ScrollIntoView);
            }

            if let Some(bounds) = bounds {
                let pages: BTreeMap<_, _> = PAGE_ACTIONS
                    .into_iter()
                    .filter_map(|(action, direction)| {
                        frame
                            .scroll_page_target(id, direction)
                            .map(|target| (action, target))
                    })
                    .collect();
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
            nodes.push((native_id, node));
            ids.insert(id, native_id);
            targets.insert(
                native_id,
                Targets {
                    focus,
                    press,
                    reveal,
                    pages: BTreeMap::new(),
                },
            );
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
        match (request.action, request.data) {
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

fn describe(source: &AccessibilityNode<'_>) -> Node {
    let authored = source.node;
    let role = match authored.role() {
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
    };
    let mut node = Node::new(role);
    let label = match authored.role() {
        SurfaceRole::Text | SurfaceRole::Heading | SurfaceRole::Status => {
            authored.text_value().unwrap_or(authored.name())
        }
        _ => authored.name(),
    };
    if !label.is_empty() {
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
    if let Some(selected) = authored.is_selected() {
        node.set_selected(selected);
    }
    if authored.is_current_page() == Some(true) {
        node.set_aria_current(AriaCurrent::Page);
    }
    if let Some(expanded) = authored.expanded_state() {
        node.set_expanded(expanded);
    }
    if let Some(value) = authored.is_checked() {
        node.set_toggled(if authored.is_mixed() {
            Toggled::Mixed
        } else {
            value.into()
        });
    }
    if let Some(range) = authored.range_value() {
        node.set_numeric_value(range.value);
        node.set_min_numeric_value(range.min);
        node.set_max_numeric_value(range.max);
        node.set_numeric_value_step(range.step);
    }
    node
}
