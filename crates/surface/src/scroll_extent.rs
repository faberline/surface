use crate::{ComputedDom, MicroDom, NodeId, Position};

/// The scrollable content size of a node, in pixels: the far edge of its
/// content children relative to the node, plus the node's own scroll offset
/// and padding, and at least the node's own size.
///
/// Fixed children and fitted absolute decorations do not count. The result
/// does not depend on the current offset. Returns `None` if `id` is
/// not a node of `dom` or `computed` does not cover it.
pub fn scroll_extent(dom: &MicroDom, computed: &ComputedDom, id: NodeId) -> Option<(f32, f32)> {
    let style = dom.get(id).ok()?.style();
    let own = computed.nodes.get(id.index())?.bounds;
    let mut extent = (own.width, own.height);
    for child in dom.children(id) {
        let child_node = dom.get(child).ok()?;
        if child_node.style().position == Position::Fixed
            || child_node.absolute_overlay_fit().is_some()
        {
            continue;
        }
        let r = computed.nodes.get(child.index())?.bounds;
        extent.0 = extent
            .0
            .max(r.x + r.width - own.x + style.scroll_x + style.padding);
        extent.1 = extent
            .1
            .max(r.y + r.height - own.y + style.scroll_y + style.padding);
    }
    Some(extent)
}
