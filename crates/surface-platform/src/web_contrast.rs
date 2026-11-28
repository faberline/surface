//! Resolve browser contrast preferences for application-owned canvas paint.

use surface::Color;
use surface_app::{DisplayContrast, SystemColorPair, SystemContrastColors};

fn preference<E>(
    forced: bool,
    increased: bool,
    read_colors: impl FnOnce() -> Result<SystemContrastColors, E>,
) -> Result<DisplayContrast, E> {
    if forced {
        read_colors().map(DisplayContrast::Forced)
    } else if increased {
        Ok(DisplayContrast::Increased)
    } else {
        Ok(DisplayContrast::Normal)
    }
}

fn css_color(css: &str) -> Option<Color> {
    let (function, arguments) = css.trim().split_once('(')?;
    let arguments = arguments.strip_suffix(')')?;
    if arguments.contains(',') {
        if !matches!(function, "rgb" | "rgba") {
            return None;
        }
        let mut components = arguments.split(',').map(str::trim);
        let [r, g, b] = read_rgb(&mut components, 255.)?;
        let alpha = components.next().map_or(Some(255), |a| byte(a, 1.))?;
        return components.next().is_none().then_some(Color(r, g, b, alpha));
    }

    let (rgb, alpha) = arguments
        .split_once('/')
        .map_or((arguments, None), |(rgb, alpha)| (rgb, Some(alpha.trim())));
    let mut components = rgb.split_ascii_whitespace();
    let max = match function {
        "rgb" | "rgba" => 255.,
        "color" if components.next()? == "srgb" => 1.,
        _ => return None,
    };
    let [r, g, b] = read_rgb(&mut components, max)?;
    let alpha = alpha.map_or(Some(255), |a| byte(a, 1.))?;
    components.next().is_none().then_some(Color(r, g, b, alpha))
}

fn with_field_css(
    colors: SystemContrastColors,
    background: Option<&str>,
    foreground: Option<&str>,
) -> SystemContrastColors {
    let pair = background
        .and_then(css_color)
        .zip(foreground.and_then(css_color));
    pair.map_or(colors, |(background, foreground)| {
        colors.with_field(SystemColorPair {
            background,
            foreground,
        })
    })
}

fn read_rgb<'a>(components: &mut impl Iterator<Item = &'a str>, max: f32) -> Option<[u8; 3]> {
    Some([
        byte(components.next()?, max)?,
        byte(components.next()?, max)?,
        byte(components.next()?, max)?,
    ])
}

fn byte(component: &str, max: f32) -> Option<u8> {
    let (number, max) = component
        .strip_suffix('%')
        .map_or((component, max), |number| (number, 100.));
    let number: f32 = number.parse().ok()?;
    (0. ..=max)
        .contains(&number)
        .then(|| (number / max * 255.).round() as u8)
}

#[cfg(target_arch = "wasm32")]
pub(crate) use browser::DisplayContrastWatch;

#[cfg(target_arch = "wasm32")]
mod browser {
    use std::rc::Rc;

    use surface_app::{Application, Runtime, SystemColorPair};
    use wasm_bindgen::{closure::Closure, JsCast, JsValue};
    use web_sys::{
        Document, Event, EventTarget, HtmlCanvasElement, HtmlElement, MediaQueryList, Window,
    };

    use super::{css_color, preference, with_field_css, SystemContrastColors};

    const FORCED: &str = "(forced-colors: active)";
    const MORE: &str = "(prefers-contrast: more)";
    const DARK: &str = "(prefers-color-scheme: dark)";
    const RESAMPLE_MS: i32 = 1000;
    const PAIRS: [(&str, &str, &str); 3] = [
        ("canvas", "Canvas", "CanvasText"),
        ("button", "ButtonFace", "ButtonText"),
        ("highlight", "Highlight", "HighlightText"),
    ];
    const FIELD: (&str, &str, &str) = ("field", "Field", "FieldText");

    pub(crate) struct DisplayContrastWatch {
        window: Window,
        document: Document,
        canvas: HtmlCanvasElement,
        forced: Option<MediaQueryList>,
        more: Option<MediaQueryList>,
        listeners: Vec<(EventTarget, &'static str)>,
        on_change: Closure<dyn FnMut(Event)>,
        on_tick: Closure<dyn FnMut()>,
        timer: Option<i32>,
        probe: Option<ColorProbe>,
        last_warning: Option<String>,
    }

    impl DisplayContrastWatch {
        /// Browser callbacks signal the host. The event loop owns Runtime.
        pub(crate) fn new(
            canvas: HtmlCanvasElement,
            notify: impl Fn() + 'static,
        ) -> Result<Option<Self>, JsValue> {
            let Some(document) = canvas.owner_document() else {
                return Ok(None);
            };
            let Some(window) = document.default_view() else {
                return Ok(None);
            };
            let forced = window.match_media(FORCED)?;
            let more = window.match_media(MORE)?;
            if forced.is_none() && more.is_none() {
                return Ok(None);
            }
            let dark = window.match_media(DARK).ok().flatten();
            let notify = Rc::new(notify);
            let changes = Rc::clone(&notify);
            let mut watch = Self {
                window: window.clone(),
                document: document.clone(),
                canvas,
                forced: forced.clone(),
                more: more.clone(),
                listeners: Vec::new(),
                on_change: Closure::new(move |_: Event| changes()),
                on_tick: Closure::new(move || notify()),
                timer: None,
                probe: None,
                last_warning: None,
            };
            for query in [forced, more, dark].into_iter().flatten() {
                watch.listen(query.unchecked_into(), "change")?;
            }
            watch.listen(window.unchecked_into(), "focus")?;
            watch.listen(document.unchecked_into(), "visibilitychange")?;
            Ok(Some(watch))
        }

        fn listen(&mut self, target: EventTarget, event: &'static str) -> Result<(), JsValue> {
            target
                .add_event_listener_with_callback(event, self.on_change.as_ref().unchecked_ref())?;
            self.listeners.push((target, event));
            Ok(())
        }

        pub(crate) fn refresh<A: Application>(&mut self, runtime: &mut Runtime<A>) -> bool {
            let forced = self.forced.as_ref().is_some_and(MediaQueryList::matches);
            let more = self.more.as_ref().is_some_and(MediaQueryList::matches);
            // Focus/visibility/media changes handle the common case. A visible
            // forced palette can change without a media query changing value.
            self.set_timer(forced && !self.document.hidden());
            if !forced {
                self.probe = None;
            }
            let previous = runtime.display_contrast();
            let result = preference(forced, more, || self.read_colors()).and_then(|appearance| {
                runtime
                    .set_display_contrast(appearance)
                    .map_err(|error| format!("could not complete preference update: {error}"))
            });
            match result {
                Ok(changed) => {
                    self.last_warning = None;
                    changed
                }
                Err(error) => {
                    // Avoid repeating the same warning on each sample.
                    if self.last_warning.as_deref() != Some(error.as_str()) {
                        web_sys::console::warn_2(
                            &"Surface Web display contrast:".into(),
                            &error.as_str().into(),
                        );
                        self.last_warning = Some(error);
                    }
                    // A later reconciliation can fail after installing contrast.
                    runtime.display_contrast() != previous
                }
            }
        }

        fn set_timer(&mut self, enabled: bool) {
            match (enabled, self.timer) {
                (true, None) => {
                    self.timer = self
                        .window
                        .set_interval_with_callback_and_timeout_and_arguments_0(
                            self.on_tick.as_ref().unchecked_ref(),
                            RESAMPLE_MS,
                        )
                        .ok();
                }
                (false, Some(timer)) => {
                    self.window.clear_interval_with_handle(timer);
                    self.timer = None;
                }
                _ => {}
            }
        }

        fn read_colors(&mut self) -> Result<SystemContrastColors, String> {
            // Read the scheme before mounting the probe. Chrome can resolve
            // Highlight against the initial inherited scheme and retain it
            // when a mounted probe is restyled.
            let scheme = self
                .window
                .get_computed_style(&self.canvas)
                .ok()
                .flatten()
                .and_then(|style| style.get_property_value("color-scheme").ok())
                .ok_or("could not read canvas color-scheme")?;
            if self
                .probe
                .as_ref()
                .is_none_or(|probe| !probe.root.is_connected() || probe.scheme != scheme)
            {
                self.probe = None;
                self.probe = Some(
                    ColorProbe::new(&self.document, &self.canvas, scheme).map_err(|error| {
                        error
                            .as_string()
                            .unwrap_or_else(|| "could not mount system color probe".to_owned())
                    })?,
                );
            }
            self.probe
                .as_ref()
                .expect("mounted system color probe")
                .read(&self.window)
        }
    }

    impl Drop for DisplayContrastWatch {
        fn drop(&mut self) {
            self.set_timer(false);
            for (target, event) in self.listeners.drain(..) {
                let _ = target.remove_event_listener_with_callback(
                    event,
                    self.on_change.as_ref().unchecked_ref(),
                );
            }
        }
    }

    struct ColorProbe {
        root: HtmlElement,
        pairs: [HtmlElement; 3],
        field: Option<HtmlElement>,
        gray_text_supported: bool,
        scheme: String,
    }

    impl ColorProbe {
        fn new(
            document: &Document,
            canvas: &HtmlCanvasElement,
            scheme: String,
        ) -> Result<Self, JsValue> {
            let root: HtmlElement = document.create_element("span")?.dyn_into()?;
            root.set_attribute("aria-hidden", "true")?;
            root.set_attribute("data-surface-system-colors", "")?;
            let style = root.style();
            style.set_css_text(
                "position:fixed;visibility:hidden;pointer-events:none;width:0;height:0;overflow:hidden;forced-color-adjust:auto;",
            );
            style.set_property_with_priority("color-scheme", &scheme, "important")?;
            let gray_text_supported = style
                .set_property_with_priority("color", "GrayText", "important")
                .is_ok()
                && style
                    .get_property_value("color")
                    .is_ok_and(|value| !value.is_empty());
            let pair = |(name, background, foreground), marker| -> Result<HtmlElement, JsValue> {
                let element: HtmlElement = document.create_element("span")?.dyn_into()?;
                element.set_attribute(marker, name)?;
                let style = element.style();
                for (property, keyword) in [("background-color", background), ("color", foreground)]
                {
                    style.set_property_with_priority(property, keyword, "important")?;
                    if style.get_property_value(property)?.is_empty() {
                        return Err(JsValue::from_str(&format!(
                            "unsupported system color {keyword}"
                        )));
                    }
                }
                root.append_child(&element)?;
                Ok(element)
            };
            let [canvas_pair, button, highlight] = PAIRS;
            let required = "data-surface-system-pair";
            let pairs = [
                pair(canvas_pair, required)?,
                pair(button, required)?,
                pair(highlight, required)?,
            ];
            let field = pair(FIELD, "data-surface-optional-system-pair").ok();
            let parent = canvas
                .parent_element()
                .or_else(|| document.body().map(Into::into))
                .ok_or_else(|| JsValue::from_str("canvas has no document parent"))?;
            parent.append_child(&root)?;
            Ok(Self {
                root,
                pairs,
                field,
                gray_text_supported,
                scheme,
            })
        }

        fn read(&self, window: &Window) -> Result<SystemContrastColors, String> {
            let read_pair = |element: &HtmlElement, (name, background, foreground)| {
                let style = window
                    .get_computed_style(element)
                    .ok()
                    .flatten()
                    .ok_or_else(|| format!("missing computed {name} colors"))?;
                let read_color = |property, system_name| {
                    style
                        .get_property_value(property)
                        .ok()
                        .and_then(|value| css_color(&value))
                        .ok_or_else(|| format!("missing or unsupported system color {system_name}"))
                };
                Ok::<_, String>(SystemColorPair {
                    background: read_color("background-color", background)?,
                    foreground: read_color("color", foreground)?,
                })
            };
            let [canvas, button, highlight] = PAIRS;
            let [canvas_probe, button_probe, highlight_probe] = &self.pairs;
            let colors = SystemContrastColors::new(
                read_pair(canvas_probe, canvas)?,
                read_pair(button_probe, button)?,
                read_pair(highlight_probe, highlight)?,
            );
            let gray_text = self
                .gray_text_supported
                .then(|| window.get_computed_style(&self.root).ok().flatten())
                .flatten()
                .and_then(|style| style.get_property_value("color").ok())
                .and_then(|color| css_color(&color));
            let colors = gray_text.map_or(colors, |ink| colors.with_disabled_text(ink));
            let field_style = self
                .field
                .as_ref()
                .filter(|element| element.is_connected())
                .and_then(|element| window.get_computed_style(element).ok().flatten());
            let read_field = |property| field_style.as_ref()?.get_property_value(property).ok();
            Ok(with_field_css(
                colors,
                read_field("background-color").as_deref(),
                read_field("color").as_deref(),
            ))
        }
    }

    impl Drop for ColorProbe {
        fn drop(&mut self) {
            self.root.remove();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_serializations_preserve_srgb_and_alpha() {
        for (css, expected) in [
            ("rgb(0, 230, 255)", Color(0, 230, 255, 255)),
            ("rgba(0, 230, 255, 0.8)", Color(0, 230, 255, 204)),
            ("rgb(0 230 255 / 80%)", Color(0, 230, 255, 204)),
            ("color(srgb 1 0.5 0 / 0.1)", Color(255, 128, 0, 26)),
            ("rgb(100% 0% 50%)", Color(255, 0, 128, 255)),
        ] {
            assert_eq!(css_color(css), Some(expected), "{css}");
        }
    }

    #[test]
    fn missing_malformed_and_non_srgb_values_fail_closed() {
        for css in [
            "",
            "CanvasText",
            "rgb(0, 230)",
            "rgb(0, 230, 255, 0.5, 0.2)",
            "rgb(0 230 255 / NaN)",
            "rgb(256, 230, 255)",
            "rgb(0 230 255 / -1)",
            "color(display-p3 1 0.5 0)",
        ] {
            assert_eq!(css_color(css), None, "{css}");
        }
    }

    #[test]
    fn forced_wins_even_when_a_pair_is_unavailable() {
        let absent = || Err::<SystemContrastColors, _>("CanvasText");
        assert_eq!(preference(true, true, absent), Err("CanvasText"));
        assert_eq!(preference(true, false, absent), Err("CanvasText"));
        assert_eq!(
            preference(false, true, absent),
            Ok(DisplayContrast::Increased)
        );
        assert_eq!(
            preference(false, false, absent),
            Ok(DisplayContrast::Normal)
        );
    }

    #[test]
    fn same_forced_mode_resolves_new_pairs() {
        let palette = |alpha| {
            let pair = SystemColorPair {
                background: Color(0, 0, 0, 255),
                foreground: Color(0, 230, 255, alpha),
            };
            SystemContrastColors::new(pair, pair, pair)
        };
        let old = preference(true, true, || Ok::<_, ()>(palette(204))).unwrap();
        let changed = preference(true, true, || Ok::<_, ()>(palette(255))).unwrap();
        assert_eq!(old, DisplayContrast::Forced(palette(204)));
        assert_eq!(changed, DisplayContrast::Forced(palette(255)));
        assert_ne!(old, changed);
    }

    fn base_colors_with_gray_text() -> SystemContrastColors {
        let pair = |background, foreground| SystemColorPair {
            background,
            foreground,
        };
        SystemContrastColors::new(
            pair(Color::rgb(255, 255, 255), Color::rgb(0, 0, 0)),
            pair(Color::rgb(230, 230, 230), Color::rgb(32, 24, 40)),
            pair(Color::rgb(55, 0, 110), Color::rgb(255, 255, 255)),
        )
        .with_disabled_text(Color::rgb(96, 0, 0))
    }

    #[test]
    fn field_probe_requires_both_halves_without_losing_other_colors() {
        let base = base_colors_with_gray_text();
        assert_eq!(base.field, None);
        let valid = Some("rgb(12, 24, 36)");
        for (background, foreground) in [
            (None, None),
            (None, valid),
            (valid, None),
            (Some(""), valid),
            (valid, Some("")),
            (Some("Field"), valid),
            (valid, Some("FieldText")),
            (Some("rgb(12, 24)"), valid),
            (valid, Some("rgb(12, 24, 256)")),
        ] {
            let resolve = || Ok::<_, ()>(with_field_css(base, background, foreground));
            assert_eq!(
                preference(true, true, resolve),
                Ok(DisplayContrast::Forced(base)),
                "{background:?}, {foreground:?}"
            );
        }
    }

    #[test]
    fn field_probe_refresh_replaces_or_removes_the_pair_atomically() {
        let base = base_colors_with_gray_text();
        let background = Some("rgba(19, 45, 47, 0.8)");
        let initial = with_field_css(base, background, Some("rgb(249, 220, 126)"));
        let updated = with_field_css(base, background, Some("rgb(238 228 190 / 50%)"));
        let unavailable = with_field_css(base, background, None);
        assert_eq!(
            initial,
            base.with_field(SystemColorPair {
                background: Color(19, 45, 47, 204),
                foreground: Color(249, 220, 126, 255),
            })
        );
        assert_eq!(
            updated,
            base.with_field(SystemColorPair {
                background: Color(19, 45, 47, 204),
                foreground: Color(238, 228, 190, 128),
            })
        );
        assert_ne!(initial, updated);
        assert_eq!(unavailable, base);
    }
}
