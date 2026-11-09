//! CSS for the hidden IME textarea. One line box equals the caret line.
pub(crate) const BASE_CSS: &str = "position:fixed;opacity:0;width:320px;padding:0;margin:0;border:0;resize:none;pointer-events:none;overflow:hidden;white-space:pre;z-index:-1;";

/// `left`/`top` are CSS pixels (canvas rect + caret x/y). `height` is the caret height.
pub(crate) fn caret_css(left: f64, top: f64, height: f32) -> String {
    let h = height.max(1.0);
    format!("{BASE_CSS}left:{left}px;top:{top}px;height:{h}px;line-height:{h}px;font-size:{h}px;")
}

#[cfg(test)]
mod tests {
    use super::caret_css;

    /// The value of one CSS property in a declaration list.
    fn prop(css: &str, name: &str) -> String {
        css.split(';')
            .filter_map(|part| part.split_once(':'))
            .find(|(key, _)| key.trim() == name)
            .map(|(_, value)| value.trim().to_owned())
            .unwrap_or_else(|| panic!("missing css property {name} in {css}"))
    }

    #[test]
    fn top_and_left_follow_the_caret() {
        let css = caret_css(10.5, 200., 24.);
        assert_eq!(prop(&css, "left"), "10.5px");
        assert_eq!(prop(&css, "top"), "200px");
    }

    #[test]
    fn line_box_equals_the_caret_height_for_two_heights() {
        for h in [24., 40.] {
            let css = caret_css(0., 0., h);
            let expected = format!("{h}px");
            assert_eq!(prop(&css, "height"), expected);
            assert_eq!(prop(&css, "line-height"), expected);
            assert_eq!(prop(&css, "font-size"), expected);
        }
    }

    #[test]
    fn height_never_collapses_below_one_pixel() {
        let css = caret_css(0., 0., 0.);
        assert_eq!(prop(&css, "height"), "1px");
    }

    #[test]
    fn host_stays_invisible_and_ignores_pointer_input() {
        let css = caret_css(0., 0., 24.);
        assert_eq!(prop(&css, "opacity"), "0");
        assert_eq!(prop(&css, "pointer-events"), "none");
        assert_eq!(prop(&css, "position"), "fixed");
        assert_eq!(prop(&css, "white-space"), "pre");
    }
}
