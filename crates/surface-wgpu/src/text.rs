use glyphon::fontdb::{Stretch, Style};
use glyphon::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Weight, Wrap};
use std::sync::Arc;
use surface::{FontFamily, Rect, TextCluster, TextLayout, TextMeasurer, TextStyle};

/// The family that the sans-serif alias names. `FontFamily::Default` shapes
/// with it, and `add_font` only widens the default family's weights for faces
/// that carry this name.
const DEFAULT_FAMILY: &str = "Tuffy";
#[cfg(target_arch = "wasm32")]
const CJK_FONT: &[u8] = include_bytes!("../assets/NotoSansCJKtc-Subset.otf");
#[cfg(not(target_arch = "wasm32"))]
const CJK_FONT: &[u8] = include_bytes!("../assets/NotoSansCJKtc-Regular.otf");

/// Shared deterministic fonts and shaping for layout and both renderer hosts.
///
/// Line height, weight and family reach the shaper. Letter spacing is part of
/// `TextStyle` but cosmic-text 0.12 cannot apply it, so it does not change
/// measurement or painting.
///
/// Only regular faces are bundled, so a Medium or Bold request renders with a
/// regular face. How a request reaches a face follows from how cosmic-text
/// 0.12.1 matches faces. `FontSystem::get_font_matches` keeps every face with
/// the right style and stretch and orders them by weight distance. Family
/// lookup then uses only faces at distance zero: `default_font_match_key`, the
/// script fallbacks and the common fallbacks all filter on
/// `font_weight_diff == 0`. Only the last resort, which ignores the family
/// name, takes a nearer weight. So a family is honored only when its face is
/// asked for by the exact weight the face declares. This type resolves the
/// weight itself, so that choice is explicit and deterministic.
///
/// * `FontFamily::Named(name)` takes the faces that carry `name` and asks for
///   the declared weight nearest to the request, preferring the lighter one on
///   a tie. `Named("Tuffy")` therefore shapes with Tuffy at any weight. Tuffy
///   declares weight 500, so a plain request for 400 would have missed it and
///   shaped with Noto Sans CJK TC. A name that no face carries shapes like the
///   default family.
/// * `FontFamily::Default` keeps the shaping that text had before text styles
///   existed: regular text uses Noto Sans CJK TC for Latin and CJK alike. The
///   bundled faces count as regular here. Tuffy declares weight 500, but it is
///   a separate typeface from Noto, not Noto's Medium. Honoring that number
///   would turn every Medium and Bold role into a switch of typeface. A face
///   added with `add_font` joins the default family when it carries the name
///   Tuffy, and then a request for its weight selects it.
pub struct TextSystem {
    pub(crate) fonts: FontSystem,
    /// Sorted weights that the default family can draw. The bundled faces
    /// count as regular, so this starts as `[400]`.
    weights: Vec<u16>,
}
impl TextSystem {
    /// Build the default font database with `cjk` as the CJK face, without copying it.
    pub fn with_cjk_font(cjk: &'static [u8]) -> Self {
        let mut db = glyphon::fontdb::Database::new();
        db.load_font_data(include_bytes!("../assets/Tuffy.ttf").to_vec());
        db.load_font_source(glyphon::fontdb::Source::Binary(Arc::new(cjk)));
        db.set_sans_serif_family(DEFAULT_FAMILY);
        Self {
            fonts: FontSystem::new_with_locale_and_db("zh-TW".to_owned(), db),
            weights: vec![TextStyle::WEIGHT_REGULAR],
        }
    }
}
impl Default for TextSystem {
    fn default() -> Self {
        Self::with_cjk_font(CJK_FONT)
    }
}
/// cosmic-text rejects a zero line height. Layout validation already refuses
/// one, but direct callers get the default instead of a panic.
fn usable_line_height(style: &TextStyle) -> f32 {
    let line_height = style.line_height_px();
    if line_height.is_finite() && line_height > 0. {
        line_height
    } else {
        style.font_size * 1.5
    }
}
/// The weight nearest to `requested`. A tie goes to the lighter weight.
fn nearest(weights: impl Iterator<Item = u16>, requested: u16) -> Option<u16> {
    weights.min_by_key(|weight| (weight.abs_diff(requested), *weight))
}
impl TextSystem {
    /// Add font bytes for scripts outside the bundled font's coverage, or a
    /// heavier face. A face reaches text by name with `FontFamily::Named`, and
    /// as glyph fallback. A face that carries the default family's name also
    /// adds the weight it declares to the weights that default text can
    /// request.
    pub fn add_font(&mut self, bytes: Vec<u8>) {
        let db = self.fonts.db_mut();
        let ids = db.load_font_source(glyphon::fontdb::Source::Binary(Arc::new(bytes)));
        let added = ids
            .iter()
            .filter_map(|id| db.face(*id))
            .filter(|face| face.families.iter().any(|(name, _)| name == DEFAULT_FAMILY))
            .map(|face| face.weight.0);
        self.weights.extend(added.collect::<Vec<_>>());
        self.weights.sort_unstable();
        self.weights.dedup();
    }
    pub fn missing_glyphs(&mut self, text: &str) -> usize {
        self.buffer(text, &TextStyle::new(16.), None)
            .layout_runs()
            .flat_map(|r| r.glyphs)
            .filter(|g| g.glyph_id == 0)
            .count()
    }
    /// The family and the exact face weight to hand to the shaper. See the
    /// type documentation for why the weight is resolved here.
    fn resolve<'a>(&self, family: &'a FontFamily, requested: u16) -> (Family<'a>, u16) {
        let default_weight = || {
            nearest(self.weights.iter().copied(), requested).unwrap_or(TextStyle::WEIGHT_REGULAR)
        };
        match family {
            FontFamily::Default => (Family::SansSerif, default_weight()),
            FontFamily::Named(name) => {
                // cosmic-text only matches faces of normal style and stretch.
                let own = self
                    .fonts
                    .db()
                    .faces()
                    .filter(|face| face.style == Style::Normal && face.stretch == Stretch::Normal)
                    .filter(|face| face.families.iter().any(|(family, _)| family == name))
                    .map(|face| face.weight.0);
                (
                    Family::Name(name),
                    nearest(own, requested).unwrap_or_else(default_weight),
                )
            }
        }
    }
    /// Shape `text` with the size, line height, weight and family of `style`.
    pub(crate) fn buffer(&mut self, text: &str, style: &TextStyle, width: Option<f32>) -> Buffer {
        let (family, weight) = self.resolve(&style.family, style.weight);
        let mut buffer = Buffer::new(
            &mut self.fonts,
            Metrics::new(style.font_size, usable_line_height(style)),
        );
        buffer.set_size(&mut self.fonts, width.map(|w| w.max(0.01)), None);
        buffer.set_wrap(&mut self.fonts, Wrap::WordOrGlyph);
        buffer.set_text(
            &mut self.fonts,
            text,
            Attrs::new().family(family).weight(Weight(weight)),
            Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut self.fonts, false);
        buffer
    }
}
impl TextMeasurer for TextSystem {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        self.measure_text(text, &TextStyle::new(size), width)
    }
    fn measure_text(&mut self, text: &str, style: &TextStyle, width: Option<f32>) -> (f32, f32) {
        let buffer = self.buffer(text, style, width);
        let (width, height) = buffer.layout_runs().fold((0f32, 0f32), |(w, h), run| {
            (w.max(run.line_w), h.max(run.line_top + run.line_height))
        });
        // Reserve the last fractional pixel before Taffy rounds node bounds.
        // Rounding down here can wrap a word during painting and clip it.
        (width.ceil(), height.ceil())
    }
    fn text_layout(&mut self, text: &str, size: f32, width: Option<f32>) -> TextLayout {
        self.text_layout_styled(text, &TextStyle::new(size), width)
    }
    fn text_layout_styled(
        &mut self,
        text: &str,
        style: &TextStyle,
        width: Option<f32>,
    ) -> TextLayout {
        let buffer = self.buffer(text, style, width);
        let mut starts = vec![0];
        for (i, c) in text.char_indices() {
            if c == '\n' {
                starts.push(i + 1);
            }
        }
        let mut clusters = Vec::new();
        let line_height = usable_line_height(style);
        let mut height = line_height;
        for run in buffer.layout_runs() {
            let base = starts.get(run.line_i).copied().unwrap_or(text.len());
            height = height.max(run.line_top + run.line_height);
            if run.glyphs.is_empty() {
                clusters.push(TextCluster {
                    bytes: base..base,
                    bounds: Rect::new(0., run.line_top, 0., run.line_height),
                    rtl: run.rtl,
                });
            }
            for glyph in run.glyphs {
                clusters.push(TextCluster {
                    bytes: base + glyph.start..base + glyph.end,
                    bounds: Rect::new(glyph.x, run.line_top, glyph.w, run.line_height),
                    rtl: glyph.level.is_rtl(),
                });
            }
        }
        TextLayout {
            clusters,
            line_height,
            text_len: text.len(),
            height,
        }
    }
}
