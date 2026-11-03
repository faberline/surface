use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn contains(self, x: f32, y: f32) -> bool {
        self.width > 0.
            && self.height > 0.
            && x >= self.x
            && y >= self.y
            && x < self.x + self.width
            && y < self.y + self.height
    }
    pub fn intersect(self, rhs: Self) -> Self {
        let x = self.x.max(rhs.x);
        let y = self.y.max(rhs.y);
        Self::new(
            x,
            y,
            (self.x + self.width).min(rhs.x + rhs.width).max(x) - x,
            (self.y + self.height).min(rhs.y + rhs.height).max(y) - y,
        )
    }
    pub fn inset(self, value: f32) -> Self {
        Self::new(
            self.x + value,
            self.y + value,
            (self.width - 2. * value).max(0.),
            (self.height - 2. * value).max(0.),
        )
    }
}
