use surface::Color;

/// A platform preference supplied by the host. Applications choose how to
/// apply it to their own paint roles.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum DisplayContrast {
    #[default]
    Normal,
    Increased,
    Forced(SystemContrastColors),
}

/// Two colors intended to be used together. The values retain their original
/// alpha. A foreground might lack contrast on another pair's background.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SystemColorPair {
    pub background: Color,
    pub foreground: Color,
}

/// Resolved system colors. The host owns color lookup and any policy for
/// a forced palette. Surface keeps the resolved RGBA values as supplied.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SystemContrastColors {
    /// General content background and its text.
    pub canvas: SystemColorPair,
    /// Button face and its text.
    pub button: SystemColorPair,
    /// Selected background and its text.
    pub highlight: SystemColorPair,
    /// System disabled text ink, when the host can resolve it.
    pub disabled_text: Option<Color>,
}

impl SystemContrastColors {
    pub const fn new(
        canvas: SystemColorPair,
        button: SystemColorPair,
        highlight: SystemColorPair,
    ) -> Self {
        Self {
            canvas,
            button,
            highlight,
            disabled_text: None,
        }
    }

    /// Attach the system's disabled text color without changing its alpha.
    pub const fn with_disabled_text(self, ink: Color) -> Self {
        Self {
            disabled_text: Some(ink),
            ..self
        }
    }
}
