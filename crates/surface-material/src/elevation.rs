//! Material 3 shadow levels using the Material Web key and ambient layers.

use surface::{BoxShadow, Color};

use crate::Theme;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Elevation {
    #[default]
    Level0,
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
}

impl Theme {
    /// The ambient and key shadow layers in logical pixels. Apply them
    /// to the visual surface with `surface::Node::box_shadows`.
    pub fn elevation(&self, level: Elevation) -> [BoxShadow; 2] {
        let (key, ambient) = match level {
            Elevation::Level0 => return [BoxShadow::NONE; 2],
            Elevation::Level1 => ((1., 2., 0.), (1., 3., 1.)),
            Elevation::Level2 => ((1., 2., 0.), (2., 6., 2.)),
            Elevation::Level3 => ((1., 3., 0.), (4., 8., 3.)),
            Elevation::Level4 => ((2., 3., 0.), (6., 10., 4.)),
            Elevation::Level5 => ((4., 4., 0.), (8., 12., 6.)),
        };
        let Color(red, green, blue, alpha) = self.colors.shadow;
        let key_color = Color(red, green, blue, ((alpha as u16 * 30 + 50) / 100) as u8);
        let ambient_color = Color(red, green, blue, ((alpha as u16 * 15 + 50) / 100) as u8);
        [
            BoxShadow::new(0., ambient.0, ambient.1, ambient.2, ambient_color),
            BoxShadow::new(0., key.0, key.1, key.2, key_color),
        ]
    }
}
