//! Typed layout and paint values. Dimensions use logical pixels.
use crate::typography::{is_default_family, is_regular_weight, is_zero, regular_weight};
use crate::{Corners, FontFamily, TextStyle, Transition};
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
    /// Per-corner radii. When set they win over `radius`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corners: Option<Corners>,
    pub font_size: f32,
    pub border_width: f32,
    pub border_color: Color,
    pub state_layer: Color,
    /// How changes to `background`, `foreground`, `border_color`,
    /// `state_layer` and the corners show. Corners come from `corners`, or
    /// from `radius` when `corners` is unset. Layout never animates.
    ///
    /// Animate only nodes with a stable ID. A node without one is tracked by
    /// position, so inserted or reordered siblings can inherit another
    /// node's track. When no node in the tree has a transition, motion keeps
    /// no state. A transition added in the same update as a value change then
    /// does not animate that first change. Later changes animate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<Transition>,
    // Text style fields are omitted from JSON while they hold their defaults,
    // so styles that predate typography export byte-identically. A missing
    // weight must read back as regular, not as the numeric default of zero.
    #[serde(default = "regular_weight", skip_serializing_if = "is_regular_weight")]
    pub font_weight: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f32>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub letter_spacing: f32,
    #[serde(default, skip_serializing_if = "is_default_family")]
    pub font_family: FontFamily,
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
            corners: None,
            font_size: 16.,
            border_width: 0.,
            border_color: Color::TRANSPARENT,
            state_layer: Color::TRANSPARENT,
            transition: None,
            font_weight: TextStyle::WEIGHT_REGULAR,
            line_height: None,
            letter_spacing: 0.,
            font_family: FontFamily::Default,
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
    pub fn font_weight(mut self, value: u16) -> Self {
        self.font_weight = value;
        self
    }
    /// Absolute line height in logical pixels.
    pub fn line_height(mut self, value: f32) -> Self {
        self.line_height = Some(value);
        self
    }
    /// Carried in the style. The bundled renderer does not apply it yet.
    pub fn letter_spacing(mut self, value: f32) -> Self {
        self.letter_spacing = value;
        self
    }
    pub fn font_family(mut self, value: FontFamily) -> Self {
        self.font_family = value;
        self
    }
    /// Set the size, line height, weight, letter spacing and family together.
    pub fn with_text_style(mut self, value: TextStyle) -> Self {
        self.font_size = value.font_size;
        self.line_height = value.line_height;
        self.font_weight = value.weight;
        self.letter_spacing = value.letter_spacing;
        self.font_family = value.family;
        self
    }
    pub fn text_style(&self) -> TextStyle {
        TextStyle {
            font_size: self.font_size,
            line_height: self.line_height,
            weight: self.font_weight,
            letter_spacing: self.letter_spacing,
            family: self.font_family.clone(),
        }
    }
    /// True when both styles give a renderer the same text attributes.
    pub(crate) fn same_text_style(&self, other: &Self) -> bool {
        self.font_size == other.font_size
            && self.line_height == other.line_height
            && self.font_weight == other.font_weight
            && self.letter_spacing == other.letter_spacing
            && self.font_family == other.font_family
    }
    pub fn radius(mut self, value: f32) -> Self {
        self.radius = value;
        self
    }
    /// Per-corner radii. They win over the legacy `radius`.
    pub fn corners(mut self, corners: Corners) -> Self {
        self.corners = Some(corners);
        self
    }
    /// The corner radii that apply to this style, before clamping to the node size.
    pub fn corner_radii(&self) -> Corners {
        self.corners.unwrap_or(Corners::all(self.radius))
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
    pub fn transition(mut self, value: Transition) -> Self {
        self.transition = Some(value);
        self
    }
}
