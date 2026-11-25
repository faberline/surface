use serde::{Deserialize, Serialize};

use crate::{Color, PaintOp, Rect, TextAlignment};

/// The two colors painted together for an editable text selection.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextSelectionColors {
    pub background: Color,
    pub foreground: Color,
}

/// One shaped-as-usual text run with an optional second color inside its
/// selected regions. The renderer paints the selection background first
/// and uses the same run for all foreground regions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextSelectionPaint {
    run: PaintOp,
    colors: TextSelectionColors,
    selected: Vec<Rect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    alignment: Option<TextAlignment>,
}

/// One part of the editable run's physical scissor. Regions never share a
/// pixel. Selected regions carry the paired background and foreground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextSelectionRegion {
    /// Left, top, width and height in device pixels.
    pub scissor: [u32; 4],
    pub selected: bool,
}

impl TextSelectionPaint {
    pub(super) fn new(
        run: PaintOp,
        colors: TextSelectionColors,
        selected: Vec<Rect>,
        alignment: Option<TextAlignment>,
    ) -> Self {
        Self {
            run,
            colors,
            selected,
            alignment,
        }
    }

    /// The unchanged editable run. Use this rect, scroll, and font style to
    /// shape the whole input once. Its color is the unselected foreground.
    pub fn run(&self) -> &PaintOp {
        &self.run
    }

    pub fn colors(&self) -> TextSelectionColors {
        self.colors
    }

    /// The alignment of every visual line in the wrapped editable run.
    pub fn alignment(&self) -> Option<TextAlignment> {
        self.alignment
    }

    /// Partition the run's existing physical clip into disjoint regions.
    ///
    /// The outer clip follows the existing outward scissor. Internal edges
    /// assign pixels by sample center, so fractional scale cannot make both
    /// inks blend into the same device pixel. Use selected regions once for
    /// background paint, then every region once for foreground clipping.
    pub fn pixel_regions(&self, scale: f32, size: (u32, u32)) -> Vec<TextSelectionRegion> {
        if !scale.is_finite() || scale <= 0. {
            return Vec::new();
        }
        let Some(outer) = PixelRect::new(self.run.clip(), scale, size, f32::floor, f32::ceil)
        else {
            return Vec::new();
        };
        let sample_edge = |edge: f32| (edge - 0.5).ceil();
        let mut unselected = vec![outer];
        let mut selected = Vec::new();
        let mut remaining = Vec::new();
        for rect in self
            .selected
            .iter()
            .filter_map(|rect| PixelRect::new(*rect, scale, size, sample_edge, sample_edge))
        {
            for area in unselected.drain(..) {
                if let Some(cut) = area.intersect(rect) {
                    selected.push(cut.region(true));
                    remaining.extend(area.without(cut).into_iter().filter(PixelRect::has_area));
                } else {
                    remaining.push(area);
                }
            }
            std::mem::swap(&mut unselected, &mut remaining);
        }
        selected.extend(unselected.into_iter().map(|area| area.region(false)));
        selected
    }
}

pub(super) fn visible_selection(clip: Rect, rectangles: &[Rect]) -> Vec<Rect> {
    if !valid_rect(clip) {
        return Vec::new();
    }
    rectangles
        .iter()
        .copied()
        .filter(|rect| valid_rect(*rect) && valid_rect(rect.intersect(clip)))
        .collect()
}

/// Paint the visible parts of a decoration once, with their selected state.
/// Return false when no selection intersects it so the original draw stays.
pub(super) fn split_selected_underline(
    underline: Rect,
    clip: Rect,
    selection: &[Rect],
    mut paint: impl FnMut(Rect, bool),
) -> bool {
    if !valid_rect(underline) || !valid_rect(clip) {
        return false;
    }
    let visible = underline.intersect(clip);
    let mut selected = selection
        .iter()
        .copied()
        .filter(|rect| valid_rect(*rect))
        .map(|rect| rect.intersect(visible))
        .filter(|rect| valid_rect(*rect));
    let Some(first) = selected.next() else {
        return false;
    };
    let mut ordinary = vec![visible];
    let mut remaining = Vec::new();
    for selected in std::iter::once(first).chain(selected) {
        for area in ordinary.drain(..) {
            let cut = area.intersect(selected);
            if valid_rect(cut) {
                paint(cut, true);
                remaining.extend(
                    without_rect(area, cut)
                        .into_iter()
                        .filter(|rect| valid_rect(*rect)),
                );
            } else {
                remaining.push(area);
            }
        }
        std::mem::swap(&mut ordinary, &mut remaining);
        if ordinary.is_empty() {
            break;
        }
    }
    for area in ordinary {
        paint(area, false);
    }
    true
}

fn without_rect(area: Rect, cut: Rect) -> [Rect; 4] {
    let cut_right = cut.x + cut.width;
    let cut_bottom = cut.y + cut.height;
    [
        Rect::new(area.x, area.y, area.width, cut.y - area.y),
        Rect::new(
            area.x,
            cut_bottom,
            area.width,
            area.y + area.height - cut_bottom,
        ),
        Rect::new(area.x, cut.y, cut.x - area.x, cut.height),
        Rect::new(
            cut_right,
            cut.y,
            area.x + area.width - cut_right,
            cut.height,
        ),
    ]
}

fn valid_rect(rect: Rect) -> bool {
    rect.width > 0.
        && rect.height > 0.
        && rect.x.is_finite()
        && rect.y.is_finite()
        && (rect.x + rect.width).is_finite()
        && (rect.y + rect.height).is_finite()
}

#[derive(Clone, Copy)]
struct PixelRect {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

impl PixelRect {
    fn new(
        rect: Rect,
        scale: f32,
        (width, height): (u32, u32),
        start: fn(f32) -> f32,
        end: fn(f32) -> f32,
    ) -> Option<Self> {
        if !valid_rect(rect) {
            return None;
        }
        let pixel = |value: f32, limit: u32| value.max(0.).min(limit as f32) as u32;
        let bounds = Self {
            left: pixel(start(rect.x * scale), width),
            top: pixel(start(rect.y * scale), height),
            right: pixel(end((rect.x + rect.width) * scale), width),
            bottom: pixel(end((rect.y + rect.height) * scale), height),
        };
        bounds.has_area().then_some(bounds)
    }

    fn has_area(&self) -> bool {
        self.right > self.left && self.bottom > self.top
    }

    fn intersect(self, other: Self) -> Option<Self> {
        let bounds = Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        };
        bounds.has_area().then_some(bounds)
    }

    fn without(self, cut: Self) -> [Self; 4] {
        // The outside rows span the full width. The other two rectangles
        // occupy only the cut's row band, so corners never overlap.
        [
            Self {
                bottom: cut.top,
                ..self
            },
            Self {
                top: cut.bottom,
                ..self
            },
            Self {
                top: cut.top,
                bottom: cut.bottom,
                right: cut.left,
                ..self
            },
            Self {
                top: cut.top,
                bottom: cut.bottom,
                left: cut.right,
                ..self
            },
        ]
    }

    fn region(self, selected: bool) -> TextSelectionRegion {
        TextSelectionRegion {
            scissor: [
                self.left,
                self.top,
                self.right - self.left,
                self.bottom - self.top,
            ],
            selected,
        }
    }
}
