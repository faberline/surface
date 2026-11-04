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
