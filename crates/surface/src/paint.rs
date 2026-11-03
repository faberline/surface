//! Ordered, serializable drawing commands shared by every GPU backend.
use crate::{Color, ComputedDom, MicroDom, NodeId, Rect};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PaintOp {
    Quad {
        rect: Rect,
        clip: Rect,
        radius: f32,
        color: Color,
    },
    Text {
        rect: Rect,
        clip: Rect,
        text: String,
        font_size: f32,
        color: Color,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DisplayList {
    pub viewport: Rect,
    pub ops: Vec<PaintOp>,
}
impl DisplayList {
    pub fn compile(dom: &MicroDom, computed: &ComputedDom, focused: Option<NodeId>) -> Self {
        let mut ops = Vec::new();
        for id in &computed.paint_order {
            let n = dom.get(*id).expect("computed node belongs to DOM");
            let g = &computed.nodes[id.index()];
            let s = n.style();
            if g.bounds.intersect(g.clip).width <= 0. || g.bounds.intersect(g.clip).height <= 0. {
                continue;
            }
            if s.background.3 > 0 {
                ops.push(PaintOp::Quad {
                    rect: g.bounds,
                    clip: g.clip,
                    radius: s.radius,
                    color: s.background,
                });
            }
            if focused == Some(*id) {
                let r = g.bounds;
                for line in [
                    Rect::new(r.x, r.y, r.width, 2.),
                    Rect::new(r.x, r.y + r.height - 2., r.width, 2.),
                    Rect::new(r.x, r.y, 2., r.height),
                    Rect::new(r.x + r.width - 2., r.y, 2., r.height),
                ] {
                    ops.push(PaintOp::Quad {
                        rect: line,
                        clip: g.clip,
                        radius: 0.,
                        color: s.foreground,
                    });
                }
            }
            if let Some(text) = n.text_value() {
                let rect = g.bounds.inset(s.padding);
                ops.push(PaintOp::Text {
                    rect,
                    clip: g.clip.intersect(rect),
                    text: text.to_owned(),
                    font_size: s.font_size,
                    color: s.foreground,
                });
            }
        }
        Self {
            viewport: computed.viewport,
            ops,
        }
    }
}
