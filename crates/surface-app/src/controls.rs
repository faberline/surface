use crate::{AppError, Application, EditKey, Runtime};
use surface::{Direction, NodeId, RangeValue, Rect, Role};

/// One tab stop per group. Arrows move within it and skip disabled controls.
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
pub(crate) struct FocusScope {
    id: String,
    restore: Option<String>,
}
pub(crate) struct RangeDrag {
    id: String,
    bounds: Rect,
    range: RangeValue,
}

impl<A: Application> Runtime<A> {
    fn descendant(&self, mut id: NodeId, parent: NodeId) -> bool {
        loop {
            if id == parent {
                return true;
            }
            let Some(next) = self.dom.get(id).ok().and_then(|n| n.parent()) else {
                return false;
            };
            id = next;
        }
    }
    pub(crate) fn in_scope(&self, id: NodeId) -> bool {
        self.scopes
            .last()
            .and_then(|s| self.dom.by_id(&s.id))
            .is_none_or(|root| self.descendant(id, root))
    }
    fn group(&self, id: NodeId) -> Option<(NodeId, Roving)> {
        let mut parent = self.dom.get(id).ok()?.parent();
        while let Some(id) = parent {
            if let Some(options) = self.bindings.get(&id).and_then(|b| b.roving) {
                return Some((id, options));
            }
            parent = self.dom.get(id).ok()?.parent();
        }
        None
    }
    fn members(&self, group: NodeId) -> Vec<NodeId> {
        self.dom
            .iter()
            .filter(|(id, node)| {
                node.is_tab_stop()
                    && self.in_scope(*id)
                    && self.group(*id).is_some_and(|(parent, _)| parent == group)
            })
            .map(|(id, _)| id)
            .collect()
    }
    pub(crate) fn tab_stop(&self, id: NodeId) -> bool {
        if !self.in_scope(id) {
            return false;
        }
        let Some((group, _)) = self.group(id) else {
            return true;
        };
        let members = self.members(group);
        let current = self.focused_node().filter(|id| members.contains(id));
        let selected = members.iter().copied().find(|id| {
            self.dom
                .get(*id)
                .is_ok_and(|n| n.is_selected() == Some(true) || n.is_checked() == Some(true))
        });
        current.or(selected).or_else(|| members.first().copied()) == Some(id)
    }
    pub(crate) fn scope_entry(&self) -> Option<String> {
        self.dom
            .iter()
            .find(|(id, n)| n.is_tab_stop() && self.tab_stop(*id))
            .and_then(|(id, _)| self.stable(id))
            .or_else(|| {
                self.scopes
                    .last()
                    .and_then(|s| self.dom.by_id(&s.id))
                    .filter(|id| self.dom.get(*id).is_ok_and(|n| n.is_focusable()))
                    .and_then(|id| self.stable(id))
            })
    }
    pub(crate) fn reconcile_controls(&mut self) {
        let old_focus = self.focused.clone();
        let panels: Vec<_> = self
            .dom
            .paint_order()
            .into_iter()
            .filter(|id| self.bindings.get(id).is_some_and(|b| b.dismiss.is_some()))
            .filter_map(|id| self.stable(id))
            .collect();
        let common = self
            .scopes
            .iter()
            .map(|s| &s.id)
            .zip(&panels)
            .take_while(|(a, b)| a == b)
            .count();
        while self.scopes.len() > common {
            let scope = self.scopes.pop().expect("scope exists");
            self.focused = scope.restore.filter(|key| {
                self.dom.by_id(key).is_some_and(|id| {
                    self.in_scope(id) && self.dom.get(id).is_ok_and(|n| n.is_focusable())
                })
            });
        }
        for id in panels.into_iter().skip(common) {
            self.scopes.push(FocusScope {
                id,
                restore: self.focused.clone(),
            });
            self.focused = self.scope_entry();
        }
        if self.scopes.last().is_some() && self.focused_node().is_none_or(|id| !self.in_scope(id)) {
            self.focused = self.scope_entry();
        }
        if self.focused != old_focus {
            self.cancel_composition();
            self.ime_area = None;
            self.text_drag = None;
        }
        if self.range_drag.as_ref().is_some_and(|drag| {
            self.dom.by_id(&drag.id).is_none_or(|id| {
                !self.in_scope(id)
                    || self
                        .dom
                        .get(id)
                        .is_ok_and(|n| n.is_disabled() || n.role() != Role::Slider)
            })
        }) {
            self.range_drag = None;
        }
    }
    pub(crate) fn outside_scope(&self, x: f32, y: f32) -> bool {
        let Some(id) = self.scopes.last().and_then(|s| self.dom.by_id(&s.id)) else {
            return false;
        };
        self.computed.as_ref().is_some_and(|c| {
            let g = &c.nodes[id.index()];
            !g.bounds.intersect(g.clip).contains(x, y)
        })
    }
    pub(crate) fn dismiss_scope(&mut self) -> Result<bool, AppError> {
        let message = self
            .scopes
            .last()
            .and_then(|s| self.dom.by_id(&s.id))
            .and_then(|id| self.bindings.get(&id))
            .and_then(|b| b.dismiss.clone());
        self.message(message).map(|_| true)
    }
    pub(crate) fn navigate_control(&mut self, key: EditKey) -> Result<bool, AppError> {
        let Some(id) = self.focused_node() else {
            return Ok(false);
        };
        let node = self.dom.get(id)?;
        if let Some(range) = node.range_value().filter(|_| node.role() == Role::Slider) {
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
        let Some((group, options)) = self
            .bindings
            .get(&id)
            .and_then(|b| b.roving.map(|options| (id, options)))
            .or_else(|| self.group(id))
        else {
            return Ok(false);
        };
        let members = self.members(group);
        if members.is_empty() {
            return Ok(false);
        }
        let index = members.iter().position(|node| *node == id);
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
        let target = self.stable(members[next]).expect("control has ID");
        if options.automatic {
            self.activate(&target)
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
        let value = range.snap(value);
        if value == range.value {
            return Ok(false);
        }
        let message = self
            .bindings
            .get(&id)
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
        let value = drag.range.min
            + ((x - drag.bounds.x) / drag.bounds.width).clamp(0., 1.) as f64
                * (drag.range.max - drag.range.min);
        let id = drag.id.clone();
        self.set_value(&id, value).map(|_| true)
    }
}
