use crate::{AppError, Application, Runtime};
use surface::{ComputedDom, NodeId, Position};

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
        if viewport.width <= 0. || viewport.height <= 0. {
            return Ok(false);
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
        let dx = delta(target.x, target.width, viewport.x, viewport.width);
        let dy = delta(target.y, target.height, viewport.y, viewport.height);
        let extent = self
            .dom
            .children(ancestor)
            .filter(|child| {
                self.dom
                    .get(*child)
                    .is_ok_and(|node| node.style().position != Position::Fixed)
            })
            .fold(
                (geometry.bounds.width, geometry.bounds.height),
                |(width, height), child| {
                    let bounds = computed.nodes[child.index()].bounds;
                    (
                        width.max(
                            bounds.x + bounds.width - geometry.bounds.x
                                + style.scroll_x
                                + style.padding,
                        ),
                        height.max(
                            bounds.y + bounds.height - geometry.bounds.y
                                + style.scroll_y
                                + style.padding,
                        ),
                    )
                },
            );
        let offset = (
            (style.scroll_x + dx).clamp(0., (extent.0 - geometry.bounds.width).max(0.)),
            (style.scroll_y + dy).clamp(0., (extent.1 - geometry.bounds.height).max(0.)),
        );
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
}
