use glyphon::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Wrap};
use surface::TextMeasurer;

/// Shared deterministic fonts and shaping for layout and both renderer hosts.
pub struct TextSystem {
    pub(crate) fonts: FontSystem,
}
impl Default for TextSystem {
    fn default() -> Self {
        let mut db = glyphon::fontdb::Database::new();
        db.load_font_data(include_bytes!("../assets/Tuffy.ttf").to_vec());
        db.set_sans_serif_family("Tuffy");
        Self {
            fonts: FontSystem::new_with_locale_and_db("en-US".to_owned(), db),
        }
    }
}
impl TextSystem {
    /// Add font bytes for scripts outside the bundled font's coverage.
    pub fn add_font(&mut self, bytes: Vec<u8>) {
        self.fonts.db_mut().load_font_data(bytes);
    }
    pub(crate) fn buffer(&mut self, text: &str, size: f32, width: Option<f32>) -> Buffer {
        let mut buffer = Buffer::new(&mut self.fonts, Metrics::new(size, size * 1.5));
        buffer.set_size(&mut self.fonts, width.map(|w| w.max(0.01)), None);
        buffer.set_wrap(&mut self.fonts, Wrap::WordOrGlyph);
        buffer.set_text(
            &mut self.fonts,
            text,
            Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut self.fonts, false);
        buffer
    }
}
impl TextMeasurer for TextSystem {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let buffer = self.buffer(text, size, width);
        let (width, height) = buffer.layout_runs().fold((0f32, 0f32), |(w, h), run| {
            (w.max(run.line_w), h.max(run.line_top + run.line_height))
        });
        // Reserve the last fractional pixel before Taffy rounds node bounds.
        // Rounding down here can wrap a word during painting and clip it.
        (width.ceil(), height.ceil())
    }
}
