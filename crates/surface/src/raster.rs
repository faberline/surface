//! Live raster resources and their renderer-neutral paint geometry.

use std::{fmt, sync::Arc};

use crate::{Corners, Rect};

/// Immutable, straight-alpha sRGB RGBA8 pixels in top-to-bottom rows.
/// Clones share the original allocation. The image does not carry a GPU handle.
#[derive(Clone, PartialEq, Eq)]
pub struct RasterImage {
    width: u32,
    height: u32,
    rgba: Arc<[u8]>,
}

impl RasterImage {
    /// The largest supported width or height in source pixels.
    pub const MAX_SIDE: u32 = 4096;

    /// Create an image from tightly packed straight-alpha RGBA8 pixels.
    /// Reject a zero side, a side above `MAX_SIDE`, or a mismatched byte count.
    pub fn new(width: u32, height: u32, rgba: impl Into<Arc<[u8]>>) -> Option<Self> {
        if !(1..=Self::MAX_SIDE).contains(&width) || !(1..=Self::MAX_SIDE).contains(&height) {
            return None;
        }
        let expected = 4 * (width as usize) * (height as usize);
        let rgba = rgba.into();
        (rgba.len() == expected).then_some(Self {
            width,
            height,
            rgba,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn rgba8(&self) -> &[u8] {
        &self.rgba
    }

    /// Whether two images use the same allocation and pixel dimensions.
    pub fn shares_pixels(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && Arc::ptr_eq(&self.rgba, &other.rgba)
    }
}

impl fmt::Debug for RasterImage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RasterImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

/// An image draw with a centered crop, ancestor clip, and resolved corners.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterPaint {
    image: RasterImage,
    rect: Rect,
    clip: Rect,
    corners: Corners,
    source_uv: [f32; 4],
}

impl RasterPaint {
    /// Cover the destination with a centered, aspect-preserving crop. Reject
    /// invalid geometry or an image with no visible intersection.
    pub fn new(image: RasterImage, rect: Rect, clip: Rect, corners: Corners) -> Option<Self> {
        if !is_valid(rect) || !is_valid(clip) || !corners.is_valid() {
            return None;
        }
        let visible = rect.intersect(clip);
        if visible.width <= 0. || visible.height <= 0. {
            return None;
        }
        let source = f64::from(image.width) / f64::from(image.height);
        let destination = f64::from(rect.width) / f64::from(rect.height);
        let width = (destination / source).min(1.) as f32;
        let height = (source / destination).min(1.) as f32;
        Some(Self {
            image,
            rect,
            clip,
            corners: corners.resolve(rect.width, rect.height),
            source_uv: [(1. - width) / 2., (1. - height) / 2., width, height],
        })
    }

    pub fn image(&self) -> &RasterImage {
        &self.image
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn clip(&self) -> Rect {
        self.clip
    }

    pub fn corners(&self) -> Corners {
        self.corners
    }

    /// Normalized source `(x, y, width, height)` with the origin at the top left.
    pub fn source_uv(&self) -> [f32; 4] {
        self.source_uv
    }
}

fn is_valid(rect: Rect) -> bool {
    rect.width > 0.
        && rect.height > 0.
        && [rect.x, rect.y, rect.x + rect.width, rect.y + rect.height]
            .into_iter()
            .all(f32::is_finite)
}
