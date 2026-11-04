//! Typed layout and paint values. Dimensions use logical pixels.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);
impl Color {
    pub const TRANSPARENT: Self = Self(0, 0, 0, 0);
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(r, g, b, 255)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    Row,
    #[default]
    Column,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Align {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Position {
    #[default]
    Flow,
    Absolute,
    /// Viewport layer. Escapes ancestor clipping and scroll offsets.
    Fixed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Insets {
    pub left: Length,
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
}
impl Insets {
    pub fn all(value: f32) -> Self {
        Self {
            left: Length::Px(value),
            top: Length::Px(value),
            right: Length::Px(value),
            bottom: Length::Px(value),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    pub width: Length,
    pub height: Length,
    pub min_width: f32,
    pub min_height: f32,
    pub max_width: Length,
    pub max_height: Length,
    pub direction: Direction,
    pub align: Align,
    pub justify: Align,
    pub position: Position,
    pub inset: Insets,
    pub gap: f32,
    pub padding: f32,
    pub grow: f32,
    pub wrap: bool,
    pub clip: bool,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub z_index: i32,
    pub background: Color,
    pub foreground: Color,
    pub radius: f32,
    pub font_size: f32,
    pub border_width: f32,
    pub border_color: Color,
    pub state_layer: Color,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            width: Length::Auto,
            height: Length::Auto,
            min_width: 0.,
            min_height: 0.,
            max_width: Length::Auto,
            max_height: Length::Auto,
            direction: Direction::Column,
            align: Align::Stretch,
            justify: Align::Start,
            position: Position::Flow,
            inset: Insets::default(),
            gap: 0.,
            padding: 0.,
            grow: 0.,
            wrap: false,
            clip: false,
            scroll_x: 0.,
            scroll_y: 0.,
            z_index: 0,
            background: Color::TRANSPARENT,
            foreground: Color::rgb(29, 27, 32),
            radius: 0.,
            font_size: 16.,
            border_width: 0.,
            border_color: Color::TRANSPARENT,
            state_layer: Color::TRANSPARENT,
        }
    }
}
impl Style {
    pub fn row() -> Self {
        Self {
            direction: Direction::Row,
            ..Self::default()
        }
    }
    pub fn column() -> Self {
        Self::default()
    }
    pub fn width(mut self, value: Length) -> Self {
        self.width = value;
        self
    }
    pub fn height(mut self, value: Length) -> Self {
        self.height = value;
        self
    }
    pub fn gap(mut self, value: f32) -> Self {
        self.gap = value;
        self
    }
    pub fn padding(mut self, value: f32) -> Self {
        self.padding = value;
        self
    }
    pub fn background(mut self, value: Color) -> Self {
        self.background = value;
        self
    }
    pub fn foreground(mut self, value: Color) -> Self {
        self.foreground = value;
        self
    }
    pub fn font_size(mut self, value: f32) -> Self {
        self.font_size = value;
        self
    }
    pub fn radius(mut self, value: f32) -> Self {
        self.radius = value;
        self
    }
    pub fn align(mut self, value: Align) -> Self {
        self.align = value;
        self
    }
    pub fn justify(mut self, value: Align) -> Self {
        self.justify = value;
        self
    }
    pub fn absolute(mut self, inset: Insets) -> Self {
        self.position = Position::Absolute;
        self.inset = inset;
        self
    }
    pub fn fixed(mut self, inset: Insets) -> Self {
        self.position = Position::Fixed;
        self.inset = inset;
        self
    }
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }
    pub fn state_layer(mut self, color: Color) -> Self {
        self.state_layer = color;
        self
    }
    pub fn grow(mut self, value: f32) -> Self {
        self.grow = value;
        self
    }
    pub fn wrap(mut self, value: bool) -> Self {
        self.wrap = value;
        self
    }
    pub fn clip(mut self, value: bool) -> Self {
        self.clip = value;
        self
    }
}
