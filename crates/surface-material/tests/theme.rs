use surface::Color;
use surface_material::Theme;
fn luminance(c: Color) -> f64 {
    let channel = |v: u8| {
        let x = v as f64 / 255.;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(c.0) + 0.7152 * channel(c.1) + 0.0722 * channel(c.2)
}
#[test]
fn seeded_schemes_keep_readable_on_colors_in_both_modes() {
    for seed in [
        Color::rgb(103, 80, 164),
        Color::rgb(255, 0, 0),
        Color::rgb(0, 160, 60),
        Color::rgb(0, 80, 255),
    ] {
        for dark in [false, true] {
            let t = Theme::from_seed(seed, dark);
            for (a, b) in [
                (t.colors.primary, t.colors.on_primary),
                (
                    t.colors.secondary_container,
                    t.colors.on_secondary_container,
                ),
                (t.colors.surface, t.colors.on_surface),
                (t.colors.error, t.colors.on_error),
            ] {
                let (a, b) = (luminance(a), luminance(b));
                let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
                assert!(ratio >= 4.5, "{seed:?} dark={dark} ratio={ratio}");
            }
        }
    }
    assert_ne!(Theme::light().primary, Theme::dark().primary);
}

fn contrast(a: Color, b: Color) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn high_contrast_tonal_spot_preserves_mode_and_keeps_text_and_outlines_readable() {
    for seed in [
        Color::rgb(103, 80, 164),
        Color::rgb(210, 30, 19),
        Color::rgb(0, 160, 60),
        Color::rgb(0, 80, 255),
    ] {
        for dark in [false, true] {
            let normal = Theme::from_seed(seed, dark);
            let high = Theme::from_seed_high_contrast(seed, dark);
            assert_eq!(high.is_dark, dark);
            assert_eq!(high.primary, high.colors.primary);
            assert_eq!(high.on_background, high.colors.on_surface);
            assert_ne!(high.colors.outline, normal.colors.outline);
            for (ink, background, minimum) in [
                (high.colors.on_surface, high.colors.surface, 4.5),
                (high.colors.on_surface_variant, high.colors.surface, 4.5),
                (high.colors.on_primary, high.colors.primary, 4.5),
                (
                    high.colors.on_secondary_container,
                    high.colors.secondary_container,
                    4.5,
                ),
                (
                    high.colors.on_error_container,
                    high.colors.error_container,
                    4.5,
                ),
                (high.colors.outline, high.colors.surface, 3.),
                (high.colors.secondary, high.colors.surface, 3.),
            ] {
                assert!(
                    contrast(ink, background) >= minimum,
                    "{seed:?} dark={dark} contrast below {minimum}: {ink:?} / {background:?}",
                );
            }
        }
    }
}

#[test]
fn forced_pairs_cover_six_role_groups_and_set_legacy_aliases() {
    for (canvas, button, dark) in [
        (
            (Color::rgb(16, 19, 22), Color::rgb(232, 238, 255)),
            (Color::rgb(53, 35, 18), Color::rgb(255, 231, 139)),
            true,
        ),
        (
            (Color::rgb(249, 248, 235), Color::rgb(18, 48, 35)),
            (Color::rgb(217, 250, 255), Color::rgb(9, 43, 100)),
            false,
        ),
    ] {
        let theme = Theme::from_forced_colors(canvas, button, dark);
        let c = theme.colors;
        // Neutral, body and field backgrounds use the same paired ink.
        for surface in [
            c.background,
            c.surface_dim,
            c.surface,
            c.surface_bright,
            c.surface_container_lowest,
            c.surface_container_low,
            c.surface_container,
            c.surface_container_high,
            c.surface_container_highest,
            c.surface_variant,
        ] {
            assert_eq!((surface, c.on_surface), canvas);
        }
        assert_eq!((c.background, c.on_background), canvas);
        assert_eq!(c.on_surface_variant, canvas.1);
        // Filled accents invert both sides of Canvas, while secondary ink
        // remains available for text and the Scroll Field focus stroke.
        for (fill, ink) in [
            (c.primary, c.on_primary),
            (c.secondary, c.on_secondary),
            (c.tertiary, c.on_tertiary),
        ] {
            assert_eq!((fill, ink), (canvas.1, canvas.0));
        }
        // Tonal and fixed actions only use matching button pairs.
        for pair in [
            (c.primary_container, c.on_primary_container),
            (c.secondary_container, c.on_secondary_container),
            (c.tertiary_container, c.on_tertiary_container),
            (c.primary_fixed, c.on_primary_fixed),
            (c.secondary_fixed_dim, c.on_secondary_fixed_variant),
            (c.tertiary_fixed_dim, c.on_tertiary_fixed_variant),
        ] {
            assert_eq!(pair, button);
        }
        // Inverse surfaces such as the Snackbar use a reversed Canvas pair.
        assert_eq!(
            (c.inverse_surface, c.inverse_on_surface),
            (canvas.1, canvas.0)
        );
        assert_eq!(c.inverse_primary, canvas.0);
        // Edges, focus and scrims use nearby Canvas ink; no shadow is needed.
        assert_eq!(c.outline, canvas.1);
        assert_eq!(c.outline_variant, canvas.1);
        assert_eq!(c.secondary, canvas.1);
        assert_eq!(c.scrim, canvas.1);
        assert_eq!(c.shadow, Color::TRANSPARENT);
        // Worded errors remain readable even without a dedicated red.
        assert_eq!(c.error, canvas.1);
        assert_eq!((c.error_container, c.on_error_container), canvas);
        assert_eq!((c.error, c.on_error), (canvas.1, canvas.0));

        assert_eq!(theme.is_dark, dark);
        assert_eq!((theme.background, theme.on_background), canvas);
        assert_eq!((theme.tonal, theme.on_tonal), button);
        assert_eq!((theme.primary, theme.on_primary), (canvas.1, canvas.0));
        assert_eq!(theme.outline, canvas.1);
        assert_eq!(theme.input_container, canvas.0);
    }
}

#[test]
fn forced_colors_keep_custom_pairs_and_alpha() {
    let canvas = (Color::rgb(55, 58, 64), Color::rgb(75, 78, 79));
    let button = (Color(36, 45, 95, 182), Color(111, 224, 80, 215));
    let theme = Theme::from_forced_colors(canvas, button, true);
    assert_eq!((theme.background, theme.on_background), canvas);
    assert_eq!((theme.tonal, theme.on_tonal), button);
}
