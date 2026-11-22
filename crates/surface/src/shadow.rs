use crate::{Color, Corners, Rect};
use serde::{Deserialize, Serialize};

/// A drop shadow modeled as a rounded silhouette behind its caster.
/// Dimensions use logical pixels. A translucent caster can show the
/// shadow through its fill.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
}

impl BoxShadow {
    pub const NONE: Self = Self::new(0., 0., 0., 0., Color::TRANSPARENT);

    /// Positive spread expands the hard silhouette; positive blur softens
    /// the edge. A zero blur gives a sharp shadow.
    pub const fn new(offset_x: f32, offset_y: f32, blur: f32, spread: f32, color: Color) -> Self {
        Self {
            offset_x,
            offset_y,
            blur,
            spread,
            color,
        }
    }

    pub(crate) fn contours(
        self,
        caster: Rect,
        corners: Corners,
        clip: Rect,
        mut emit: impl FnMut(Rect, Corners, Color, Option<f32>),
    ) {
        if self.color.3 == 0
            || self.blur < 0.
            || [self.offset_x, self.offset_y, self.blur, self.spread]
                .into_iter()
                .any(|v| !v.is_finite())
            || !drawable(caster)
        {
            return;
        }
        let mut shadow = caster.inset(-self.spread);
        shadow.x += self.offset_x;
        shadow.y += self.offset_y;
        if !drawable(shadow) {
            return;
        }
        let outer = shadow.inset(-self.blur);
        if !drawable(outer) || !has_area(outer.intersect(clip)) {
            return;
        }

        let corners = corners.resolve(caster.width, caster.height);
        let spread_radius = |r: f32| {
            if r > 0. {
                (r + self.spread).max(0.)
            } else {
                0.
            }
        };
        let spread_corners = Corners::new(
            spread_radius(corners.top_left),
            spread_radius(corners.top_right),
            spread_radius(corners.bottom_right),
            spread_radius(corners.bottom_left),
        );
        if self.blur == 0. {
            emit(
                shadow,
                spread_corners.resolve(shadow.width, shadow.height),
                self.color,
                None,
            );
            return;
        }

        // Sample the falloff with adjacent strokes, each at its absolute
        // opacity. Overlapping fills make each extra opacity increment
        // vulnerable to rounding on dark RGBA8 targets.
        let steps = (self.blur * 2.).ceil().clamp(4., 24.) as u32;
        for step in 0..=steps {
            let position = step as f32 / steps as f32;
            let outset = self.blur * (1. - 2. * position);
            let rect = shadow.inset(-outset);
            if !drawable(rect) || !has_area(rect.intersect(clip)) {
                // The remaining contours are entirely inside this one.
                break;
            }
            let edge = ((step as f32 + 0.5) / steps as f32).min(1.);
            let coverage = edge * edge * (3. - 2. * edge);
            let alpha = (f32::from(self.color.3) * coverage).round() as u8;
            if alpha == 0 {
                continue;
            }
            let corners = Corners::new(
                spread_corners.top_left + outset,
                spread_corners.top_right + outset,
                spread_corners.bottom_right + outset,
                spread_corners.bottom_left + outset,
            )
            .resolve(rect.width, rect.height);
            emit(
                rect,
                corners,
                Color(self.color.0, self.color.1, self.color.2, alpha),
                (step < steps).then_some(self.blur * (2. / steps as f32)),
            );
        }
    }
}

impl Default for BoxShadow {
    fn default() -> Self {
        Self::NONE
    }
}

pub(crate) fn has_area(rect: Rect) -> bool {
    rect.width > 0. && rect.height > 0.
}

fn drawable(rect: Rect) -> bool {
    has_area(rect)
        && [rect.x, rect.y, rect.x + rect.width, rect.y + rect.height]
            .into_iter()
            .all(f32::is_finite)
}
