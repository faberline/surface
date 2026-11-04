//! Material color roles, generated from a seed with the HCT tonal-spot scheme.
use material_colors::{color::Argb, theme::ThemeBuilder};
use surface::Color;

#[derive(Clone, Copy, Debug)]
pub struct ColorScheme {
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub inverse_primary: Color,
    pub primary_fixed: Color,
    pub primary_fixed_dim: Color,
    pub on_primary_fixed: Color,
    pub on_primary_fixed_variant: Color,
    pub secondary: Color,
    pub on_secondary: Color,
    pub secondary_container: Color,
    pub on_secondary_container: Color,
    pub secondary_fixed: Color,
    pub secondary_fixed_dim: Color,
    pub on_secondary_fixed: Color,
    pub on_secondary_fixed_variant: Color,
    pub tertiary: Color,
    pub on_tertiary: Color,
    pub tertiary_container: Color,
    pub on_tertiary_container: Color,
    pub tertiary_fixed: Color,
    pub tertiary_fixed_dim: Color,
    pub on_tertiary_fixed: Color,
    pub on_tertiary_fixed_variant: Color,
    pub error: Color,
    pub on_error: Color,
    pub error_container: Color,
    pub on_error_container: Color,
    pub surface_dim: Color,
    pub surface: Color,
    pub surface_tint: Color,
    pub surface_bright: Color,
    pub surface_container_lowest: Color,
    pub surface_container_low: Color,
    pub surface_container: Color,
    pub surface_container_high: Color,
    pub surface_container_highest: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub outline_variant: Color,
    pub inverse_surface: Color,
    pub inverse_on_surface: Color,
    pub surface_variant: Color,
    pub background: Color,
    pub on_background: Color,
    pub shadow: Color,
    pub scrim: Color,
}
impl ColorScheme {
    pub fn from_seed(seed: Color, dark: bool) -> Self {
        let theme = ThemeBuilder::with_source(Argb {
            alpha: 255,
            red: seed.0,
            green: seed.1,
            blue: seed.2,
        })
        .build();
        let scheme = if dark {
            theme.schemes.dark
        } else {
            theme.schemes.light
        };
        let color = |c: Argb| Color(c.red, c.green, c.blue, c.alpha);
        Self {
            primary: color(scheme.primary),
            on_primary: color(scheme.on_primary),
            primary_container: color(scheme.primary_container),
            on_primary_container: color(scheme.on_primary_container),
            inverse_primary: color(scheme.inverse_primary),
            primary_fixed: color(scheme.primary_fixed),
            primary_fixed_dim: color(scheme.primary_fixed_dim),
            on_primary_fixed: color(scheme.on_primary_fixed),
            on_primary_fixed_variant: color(scheme.on_primary_fixed_variant),
            secondary: color(scheme.secondary),
            on_secondary: color(scheme.on_secondary),
            secondary_container: color(scheme.secondary_container),
            on_secondary_container: color(scheme.on_secondary_container),
            secondary_fixed: color(scheme.secondary_fixed),
            secondary_fixed_dim: color(scheme.secondary_fixed_dim),
            on_secondary_fixed: color(scheme.on_secondary_fixed),
            on_secondary_fixed_variant: color(scheme.on_secondary_fixed_variant),
            tertiary: color(scheme.tertiary),
            on_tertiary: color(scheme.on_tertiary),
            tertiary_container: color(scheme.tertiary_container),
            on_tertiary_container: color(scheme.on_tertiary_container),
            tertiary_fixed: color(scheme.tertiary_fixed),
            tertiary_fixed_dim: color(scheme.tertiary_fixed_dim),
            on_tertiary_fixed: color(scheme.on_tertiary_fixed),
            on_tertiary_fixed_variant: color(scheme.on_tertiary_fixed_variant),
            error: color(scheme.error),
            on_error: color(scheme.on_error),
            error_container: color(scheme.error_container),
            on_error_container: color(scheme.on_error_container),
            surface_dim: color(scheme.surface_dim),
            surface: color(scheme.surface),
            surface_tint: color(scheme.surface_tint),
            surface_bright: color(scheme.surface_bright),
            surface_container_lowest: color(scheme.surface_container_lowest),
            surface_container_low: color(scheme.surface_container_low),
            surface_container: color(scheme.surface_container),
            surface_container_high: color(scheme.surface_container_high),
            surface_container_highest: color(scheme.surface_container_highest),
            on_surface: color(scheme.on_surface),
            on_surface_variant: color(scheme.on_surface_variant),
            outline: color(scheme.outline),
            outline_variant: color(scheme.outline_variant),
            inverse_surface: color(scheme.inverse_surface),
            inverse_on_surface: color(scheme.inverse_on_surface),
            surface_variant: color(scheme.surface_variant),
            background: color(scheme.background),
            on_background: color(scheme.on_background),
            shadow: color(scheme.shadow),
            scrim: color(scheme.scrim),
        }
    }
}

pub(crate) fn blend(base: Color, overlay: Color, alpha: f32) -> Color {
    let c = |a: u8, b: u8| (a as f32 * (1. - alpha) + b as f32 * alpha).round() as u8;
    Color::rgb(
        c(base.0, overlay.0),
        c(base.1, overlay.1),
        c(base.2, overlay.2),
    )
}
