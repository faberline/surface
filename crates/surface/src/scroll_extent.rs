use crate::{ComputedDom, MicroDom, NodeId, Position};

/// The scrollable content size of a node, in pixels: the far edge of its
/// in-flow children relative to the node, plus the node's own scroll offset
/// and padding, and at least the node's own size.
///
/// `Fixed` children are placed in the viewport, so they do not count. The
/// result does not depend on the current offset. Returns `None` if `id` is
/// not a node of `dom` or `computed` does not cover it.
pub fn scroll_extent(dom: &MicroDom, computed: &ComputedDom, id: NodeId) -> Option<(f32, f32)> {
    let style = dom.get(id).ok()?.style();
    let own = computed.nodes.get(id.index())?.bounds;
    let mut extent = (own.width, own.height);
    for child in dom.children(id) {
        if dom.get(child).ok()?.style().position == Position::Fixed {
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
