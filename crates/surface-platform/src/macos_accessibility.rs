//! Project the current application scope into the macOS accessibility tree.
use accesskit::{
    Action, ActionData, ActionRequest, AriaCurrent, Live, Node, NodeId, Rect, Role, ScrollUnit,
    Toggled, TreeId, TreeInfo, TreeUpdate,
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
                node.set_live(Live::Polite);
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

#[cfg(test)]
mod tests {
    use super::*;
    use surface::{Length, Node as SurfaceNode, Style, TextMeasurer};
    use surface_app::{Event, View};

    #[derive(Clone, Copy, Default)]
    struct App {
        notice: Option<&'static str>,
        modal: bool,
        focusable_notice: bool,
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
            .accessible_activation();
            let status = self.notice.map(|message| {
                let mut node = SurfaceNode::new(SurfaceRole::Status, message)
                    .id("status")
                    .text(message);
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
            View::new(SurfaceNode::new(SurfaceRole::Window, "Desktop").styled(Style::column()))
                .children(std::iter::once(button).chain(status).chain(modal))
        }
    }

    struct Metrics;

    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
            (text.chars().count() as f32 * size, size)
        }
    }

    fn update(runtime: &mut Runtime<App>, tree: &mut Tree) -> TreeUpdate {
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
