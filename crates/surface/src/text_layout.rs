//! Font-independent geometry used by text paint, pointer hit testing and IME.
use crate::Rect;
use std::ops::Range;
#[derive(Clone, Debug)]
pub struct TextCluster {
    pub bytes: Range<usize>,
    pub bounds: Rect,
    pub rtl: bool,
}
#[derive(Clone, Debug)]
pub struct TextLayout {
    pub clusters: Vec<TextCluster>,
    pub line_height: f32,
    pub text_len: usize,
    pub height: f32,
}
impl TextLayout {
    pub fn caret(&self, byte: usize) -> Rect {
        let byte = byte.min(self.text_len);
        if let Some(c) = self.clusters.iter().find(|c| c.bytes.start == byte) {
            return Rect::new(
                if c.rtl {
                    c.bounds.x + c.bounds.width
                } else {
                    c.bounds.x
                },
                c.bounds.y,
                1.,
                c.bounds.height,
            );
        }
        if let Some(c) = self.clusters.iter().rev().find(|c| c.bytes.end <= byte) {
            return Rect::new(
                if c.rtl {
                    c.bounds.x
                } else {
                    c.bounds.x + c.bounds.width
                },
                c.bounds.y,
                1.,
                c.bounds.height,
            );
        }
        Rect::new(0., 0., 1., self.line_height)
    }
    pub fn hit(&self, x: f32, y: f32) -> usize {
        let Some(c) = self.clusters.iter().min_by(|a, b| {
            let distance = |r: Rect| {
                let dy = if y < r.y {
                    r.y - y
                } else if y > r.y + r.height {
                    y - r.y - r.height
                } else {
                    0.
                };
                let dx = if x < r.x {
                    r.x - x
                } else if x > r.x + r.width {
                    x - r.x - r.width
                } else {
                    0.
                };
                dy * 10000. + dx
            };
            distance(a.bounds).total_cmp(&distance(b.bounds))
        }) else {
            return 0;
        };
        let left = x < c.bounds.x + c.bounds.width / 2.;
        if left != c.rtl {
            c.bytes.start
        } else {
            c.bytes.end
        }
    }
    pub fn selection(&self, range: Range<usize>) -> Vec<Rect> {
        let mut rects: Vec<_> = self
            .clusters
            .iter()
            .filter(|c| c.bytes.start < range.end && c.bytes.end > range.start)
            .map(|c| c.bounds)
            .collect();
        rects.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
        let mut merged: Vec<Rect> = Vec::new();
        for r in rects {
            if let Some(last) = merged.last_mut() {
                if (last.y - r.y).abs() < 0.5 && r.x <= last.x + last.width + 1. {
                    last.width = (r.x + r.width).max(last.x + last.width) - last.x;
                    continue;
                }
            }
            merged.push(r);
        }
        merged
    }
}
