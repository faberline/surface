use crate::{
    accessibility::{invalid_editor_description, static_description_text},
    controls::descendant,
    AppError, Application, Runtime,
};
use surface::{scroll_extent, ComputedDom, MicroDom, NodeId, Position, Rect};

fn reveal_offset(target: Rect, viewport: Rect, offset: (f32, f32), max: (f32, f32)) -> (f32, f32) {
    if viewport.width <= 0. || viewport.height <= 0. {
        return offset;
    }
    let delta = |start: f32, size: f32, view_start: f32, view_size: f32| {
        if start < view_start || size > view_size {
            start - view_start
        } else if start + size > view_start + view_size {
            start + size - view_start - view_size
        } else {
            0.
        }
    };
    (
        (offset.0 + delta(target.x, target.width, viewport.x, viewport.width)).clamp(0., max.0),
        (offset.1 + delta(target.y, target.height, viewport.y, viewport.height)).clamp(0., max.1),
    )
}

fn enclosing_rect(first: Rect, second: Rect) -> Rect {
    let x = first.x.min(second.x);
    let y = first.y.min(second.y);
    let right = (first.x + first.width).max(second.x + second.width);
    let bottom = (first.y + first.height).max(second.y + second.height);
    Rect::new(x, y, right - x, bottom - y)
}

/// Test whether normal inner-to-outer reveal can bring any part of this
/// target into the viewport. Anonymous and out-of-scope clips stay fixed.
pub(crate) fn can_reveal(
    dom: &MicroDom,
    computed: &ComputedDom,
    target: NodeId,
    modal: Option<NodeId>,
    mut limits: impl FnMut(NodeId, &str) -> Option<(f32, f32)>,
) -> bool {
    let mut bounds = computed.nodes[target.index()].bounds;
    let mut visible = bounds;
    let mut current = target;
    let mut can_scroll = false;
    while let Ok(node) = dom.get(current) {
        if node.style().position == Position::Fixed {
            break;
        }
        let Some(parent) = node.parent() else { break };
        current = parent;
        let parent_node = dom.get(parent).expect("valid parent");
        let style = parent_node.style();
        if !style.clip {
            continue;
        }
        let clip = computed.nodes[parent.index()].bounds;
        if let Some(max) = parent_node
            .stable_id()
            .filter(|_| modal.is_none_or(|root| descendant(dom, parent, root)))
            .and_then(|key| limits(parent, key))
        {
            let viewport = clip.inset(style.padding);
            can_scroll |= viewport.width > 0. && viewport.height > 0. && (max.0 > 0. || max.1 > 0.);
            let next = reveal_offset(bounds, viewport, (style.scroll_x, style.scroll_y), max);
            let dx = style.scroll_x - next.0;
            let dy = style.scroll_y - next.1;
            bounds.x += dx;
            bounds.y += dy;
            visible.x += dx;
            visible.y += dy;
        }
        visible = visible.intersect(clip);
        if visible.width <= 0. || visible.height <= 0. {
            return false;
        }
    }
    let visible = visible.intersect(computed.viewport);
    can_scroll && visible.width > 0. && visible.height > 0.
}

impl<A: Application> Runtime<A> {
    pub(crate) fn scroll_ancestors(&self, focused: NodeId) -> Vec<NodeId> {
        let mut ancestors = Vec::new();
        let mut current = focused;
        while let Ok(node) = self.dom.get(current) {
            // A fixed layer is placed in the viewport, outside its former ancestors.
            if node.style().position == Position::Fixed {
                break;
            }
            let Some(parent) = node.parent() else { break };
            let node = self.dom.get(parent).expect("valid parent");
            if node.style().clip && node.stable_id().is_some() {
                ancestors.push(parent);
            }
            current = parent;
        }
        ancestors
    }
    pub(crate) fn reveal_in(
        &mut self,
        focused: NodeId,
        ancestor: NodeId,
        computed: &ComputedDom,
    ) -> Result<bool, AppError> {
        let geometry = &computed.nodes[ancestor.index()];
        let target = computed.nodes[focused.index()].bounds;
        let node = self.dom.get(ancestor)?;
        let style = node.style();
        // Reveal within the inner viewport even when an outer scroller hides it.
        // Outer ancestors are adjusted after this layout has been recomputed.
        let viewport = geometry.bounds.inset(style.padding);
        let room = geometry.bounds;
        let fits = |bounds: &Rect| bounds.width <= room.width && bounds.height <= room.height;
        let padded_target = target.inset(-style.padding);
        let outlined = self
            .dom
            .children(focused)
            .filter_map(|child| {
                let node = self.dom.get(child).ok()?;
                (node.outlines_parent_focus() && node.style().position != Position::Fixed)
                    .then_some(computed.nodes[child.index()].bounds)
            })
            .reduce(enclosing_rect)
            .map(|outline| enclosing_rect(padded_target, outline))
            .filter(fits);
        // Keep the whole focus indicator visible. Also show a mounted error
        // when the editor, outline and error can all fit at the same scroll offset.
        let with_validation = self
            .validation_bounds(focused, ancestor, computed)
            .map(|error| {
                enclosing_rect(
                    outlined.unwrap_or(padded_target),
                    error.inset(-style.padding),
                )
            })
            .filter(fits);
        let (target, viewport) = with_validation
            .or(outlined)
            .map_or((target, viewport), |bounds| (bounds, room));
        let extent = scroll_extent(&self.dom, computed, ancestor)
            .unwrap_or((geometry.bounds.width, geometry.bounds.height));
        let max = (
            (extent.0 - geometry.bounds.width).max(0.),
            (extent.1 - geometry.bounds.height).max(0.),
        );
        let offset = reveal_offset(target, viewport, (style.scroll_x, style.scroll_y), max);
        if offset == (style.scroll_x, style.scroll_y) {
            return Ok(false);
        }
        let key = node
            .stable_id()
            .expect("scroll ancestor has an ID")
            .to_owned();
        let mut style = style.clone();
        style.scroll_x = offset.0;
        style.scroll_y = offset.1;
        self.scroll.insert(key, offset);
        self.dom.set_style(ancestor, style)?;
        Ok(true)
    }

    fn validation_bounds(
        &self,
        focused: NodeId,
        ancestor: NodeId,
        computed: &ComputedDom,
    ) -> Option<Rect> {
        let key = invalid_editor_description(self.dom.get(focused).ok()?)?;
        let target = self.dom.by_id(key)?;
        static_description_text(self.dom.get(target).ok()?)?;
        if !self.in_scope(target)
            || !self.bindings.get(&target)?.accepts_interaction()
            || !self.scroll_ancestors(target).contains(&ancestor)
        {
            return None;
        }
        let bounds = computed.nodes[target.index()].bounds;
        (bounds.width > 0. && bounds.height > 0.).then_some(bounds)
    }
}
