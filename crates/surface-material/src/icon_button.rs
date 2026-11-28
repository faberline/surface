use surface::{
    Align, Color, Corners, Insets, Length, MeasuredGroupLayout, Node, Rect, Role, Style,
    TextMeasurer,
};
use surface_app::{CursorStyle, SystemColorPair, View};

use crate::{IconButtonKind, Theme, VectorIcon};

const MIN_TARGET: f32 = 48.;
const FOCUS_STROKE: f32 = 3.;
const FOCUS_OUTSET: f32 = 2. + FOCUS_STROKE;
// Material 3 Expressive Fast Spatial.
const CORNER_STIFFNESS: f32 = 800.;
const CORNER_DAMPING_RATIO: f32 = 0.6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonSize {
    ExtraSmall,
    #[default]
    Small,
    Medium,
    Large,
    ExtraLarge,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonWidth {
    Narrow,
    #[default]
    Default,
    Wide,
}

/// The resting shape of a command or an off toggle. An on toggle inverts it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonShape {
    #[default]
    Round,
    Square,
}

/// Vector artwork for a command or a caller-controlled toggle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialIcon<'a> {
    Command(&'a VectorIcon),
    Toggle {
        checked: bool,
        off: &'a VectorIcon,
        on: &'a VectorIcon,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct IconButtonOptions<'a> {
    pub id: &'a str,
    /// A localized action name that stays the same for both toggle states.
    pub name: &'a str,
    /// The stable ID of a visible tooltip or passive text description.
    pub description: Option<&'a str>,
    pub kind: IconButtonKind,
    pub size: IconButtonSize,
    pub width: IconButtonWidth,
    pub shape: IconButtonShape,
    pub disabled: bool,
}

impl Theme {
    /// Build a standalone Material 3 icon button with one named action.
    ///
    /// Use a unique, stable ID and reserve its `-face`, `-icon` and `-focus`
    /// child IDs. Leave room around the button for its focus outline. The
    /// application updates a toggle's checked value when it handles the
    /// message. Returns `None` for an empty ID or name, or artwork that
    /// cannot be drawn at the token size.
    pub fn icon_button_with_options<M>(
        &self,
        options: IconButtonOptions<'_>,
        icon: MaterialIcon<'_>,
        message: M,
    ) -> Option<View<M>> {
        self.build_icon_button(options, IconButtonArtwork::Glyph(icon), message)
    }

    /// A 48 dp trailing action. The field paints the separate passive glyph.
    pub(crate) fn text_field_trailing_icon_button<M>(
        &self,
        id: &str,
        name: &str,
        disabled: bool,
        field_icon_ink: Color,
        pair: Option<SystemColorPair>,
        message: M,
    ) -> Option<View<M>> {
        self.build_icon_button(
            IconButtonOptions {
                id,
                name,
                description: None,
                kind: IconButtonKind::Standard,
                size: IconButtonSize::Small,
                width: IconButtonWidth::Narrow,
                shape: IconButtonShape::Round,
                disabled,
            },
            IconButtonArtwork::FieldAdornment {
                ink: field_icon_ink,
                pair,
            },
            message,
        )
    }

    fn build_icon_button<M>(
        &self,
        options: IconButtonOptions<'_>,
        artwork: IconButtonArtwork<'_>,
        message: M,
    ) -> Option<View<M>> {
        if options.id.trim().is_empty() || options.name.trim().is_empty() {
            return None;
        }
        let (asset, checked, field_ink, field_pair) = match artwork {
            IconButtonArtwork::Glyph(MaterialIcon::Command(asset)) => {
                (Some(asset), None, None, None)
            }
            IconButtonArtwork::Glyph(MaterialIcon::Toggle { checked, off, on }) => (
                Some(if checked { on } else { off }),
                Some(checked),
                None,
                None,
            ),
            IconButtonArtwork::FieldAdornment { ink, pair } => (None, None, Some(ink), pair),
        };
        let tokens = options.size.tokens();
        let [narrow, default, wide] = tokens.widths;
        let width = match options.width {
            IconButtonWidth::Narrow => narrow,
            IconButtonWidth::Default => default,
            IconButtonWidth::Wide => wide,
        };
        let height = tokens.height;
        let target_width = width.max(MIN_TARGET);
        let target_height = height.max(MIN_TARGET);
        let palette = Palette::resolve(self, options.kind, checked, options.disabled);
        let radius = if (options.shape == IconButtonShape::Round) != (checked == Some(true)) {
            width.min(height) / 2.
        } else {
            tokens.square_radius
        };

        let pressed_corners =
            |outset| (!options.disabled).then_some(Corners::all(tokens.pressed_radius + outset));

        let face_id = format!("{}-face", options.id);
        let mut face = View::new(
            Node::new(Role::Group, "")
                .id(&face_id)
                .styled(
                    centered_box(width, height)
                        .corners(Corners::all(radius))
                        .background(palette.fill)
                        .border(
                            palette.outline.map_or(0., |_| tokens.outline),
                            palette.outline.unwrap_or(Color::TRANSPARENT),
                        )
                        .state_layer(if options.disabled {
                            Color::TRANSPARENT
                        } else {
                            field_ink.unwrap_or(palette.icon)
                        }),
                )
                // Keep the pressed target with the passive face. The parent
                // owns input and will supply its pressed state here.
                .interaction_corners(None, pressed_corners(0.), None)
                .interaction_corners_from_parent()
                .animate_interaction_corners_with_spring(CORNER_STIFFNESS, CORNER_DAMPING_RATIO)
                .state_layer_alphas(20, 26, 26),
        )
        .without_interaction(None);
        if let Some(asset) = asset {
            let icon_view = View::new(
                Node::new(Role::Group, "")
                    .id(format!("{}-icon", options.id))
                    .styled(centered_box(tokens.icon, tokens.icon))
                    .fill_contour(asset.contour_dp(tokens.icon, palette.icon)?),
            );
            face = face.children([icon_view]);
        }
        let mut focus_style = centered_box(width + 2. * FOCUS_OUTSET, height + 2. * FOCUS_OUTSET)
            .corners(Corners::all(radius + FOCUS_OUTSET));
        if field_ink.is_none() {
            focus_style = focus_style.absolute(Insets {
                left: Length::Px((target_width - width) / 2. - FOCUS_OUTSET),
                top: Length::Px((target_height - height) / 2. - FOCUS_OUTSET),
                ..Insets::default()
            });
        }
        let backing = field_pair.map(|pair| {
            View::new(
                Node::new(Role::Group, "")
                    .id(format!("{}-system-face", options.id))
                    .styled(focus_style.clone().background(pair.background)),
            )
            .without_interaction(None)
        });
        let focus = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-focus", options.id))
                .styled(focus_style)
                // The focus outline uses the same target plus its outset.
                .interaction_corners(None, pressed_corners(FOCUS_OUTSET), None)
                .interaction_corners_from_parent()
                .animate_interaction_corners_with_spring(CORNER_STIFFNESS, CORNER_DAMPING_RATIO)
                .focus_outline_from_parent()
                .focus_color(field_pair.map_or(self.colors.secondary, |pair| pair.foreground))
                .focus_outline_width(FOCUS_STROKE),
        )
        .without_interaction(None);

        let mut action = Node::new(Role::Button, options.name)
            .id(options.id)
            .styled(centered_box(target_width, target_height))
            .disabled(options.disabled)
            .focus_color(Color::TRANSPARENT)
            .press_ripple_to(face_id);
        if let Some(value) = checked {
            action = action.checked(value);
        }
        if let Some(target) = options.description {
            action = action.described_by_text(target);
        }
        if field_ink.is_some() {
            action = action.measured_group(FieldActionLayout {
                width,
                height,
                has_backing: backing.is_some(),
            });
        }
        let mut view = View::new(action).children(backing.into_iter().chain([face, focus]));
        if options.disabled {
            return Some(view);
        }
        if let Some(target) = options.description {
            view = view.described_by(target);
        }
        Some(
            view.on_press(message)
                .accessible_activation()
                .accessibility_scroll_into_view()
                .focus_on_pointer_release()
                .protect_composition_on_press()
                .suppress_repeated_activation()
                .cursor(CursorStyle::Pointer),
        )
    }
}

enum IconButtonArtwork<'a> {
    Glyph(MaterialIcon<'a>),
    FieldAdornment {
        ink: Color,
        pair: Option<SystemColorPair>,
    },
}

#[derive(Debug)]
struct FieldActionLayout {
    width: f32,
    height: f32,
    has_backing: bool,
}

impl MeasuredGroupLayout for FieldActionLayout {
    fn measure(&self, _: &[&Node], _: Option<f32>, _: &mut dyn TextMeasurer) -> (f32, f32) {
        (MIN_TARGET, MIN_TARGET)
    }

    fn place(
        &self,
        _: &[&Node],
        (width, height): (f32, f32),
        _: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let x = (width - self.width) / 2.;
        let y = (height - self.height) / 2.;
        let face = Rect::new(x, y, self.width, self.height);
        let focus = Rect::new(
            x - FOCUS_OUTSET,
            y - FOCUS_OUTSET,
            self.width + 2. * FOCUS_OUTSET,
            self.height + 2. * FOCUS_OUTSET,
        );
        self.has_backing
            .then_some(focus)
            .into_iter()
            .chain([face, focus])
            .collect()
    }
}

fn centered_box(width: f32, height: f32) -> Style {
    let mut style = Style::row()
        .width(Length::Px(width))
        .height(Length::Px(height))
        .align(Align::Center)
        .justify(Align::Center);
    style.min_width = width;
    style.min_height = height;
    style
}

struct SizeTokens {
    height: f32,
    icon: f32,
    widths: [f32; 3],
    square_radius: f32,
    pressed_radius: f32,
    outline: f32,
}

impl IconButtonSize {
    fn tokens(self) -> SizeTokens {
        let (height, icon, widths, square_radius, pressed_radius, outline) = match self {
            Self::ExtraSmall => (32., 20., [28., 32., 40.], 12., 8., 1.),
            Self::Small => (40., 24., [32., 40., 52.], 12., 8., 1.),
            Self::Medium => (56., 24., [48., 56., 72.], 16., 12., 1.),
            Self::Large => (96., 32., [64., 96., 128.], 28., 16., 2.),
            Self::ExtraLarge => (136., 40., [104., 136., 184.], 28., 16., 3.),
        };
        SizeTokens {
            height,
            icon,
            widths,
            square_radius,
            pressed_radius,
            outline,
        }
    }
}

struct Palette {
    fill: Color,
    icon: Color,
    outline: Option<Color>,
}

impl Palette {
    fn resolve(theme: &Theme, kind: IconButtonKind, checked: Option<bool>, disabled: bool) -> Self {
        use IconButtonKind::{Filled, Outlined, Standard, Tonal};
        let colors = theme.colors;
        let transparent = Color::TRANSPARENT;
        let (fill, icon) = match (kind, checked) {
            (Filled, Some(false)) => (colors.surface_container, colors.on_surface_variant),
            (Filled, _) => (colors.primary, colors.on_primary),
            (Tonal, Some(true)) => (colors.secondary, colors.on_secondary),
            (Tonal, _) => (colors.secondary_container, colors.on_secondary_container),
            (Outlined, Some(true)) => (colors.inverse_surface, colors.inverse_on_surface),
            (Standard, Some(true)) => (transparent, colors.primary),
            (Outlined | Standard, _) => (transparent, colors.on_surface_variant),
        };
        let outline = (kind == Outlined && checked != Some(true)).then_some(colors.outline_variant);
        if disabled {
            let Color(r, g, b, _) = colors.on_surface;
            Self {
                fill: if fill == transparent {
                    transparent
                } else {
                    Color(r, g, b, 26)
                },
                icon: Color(r, g, b, 97),
                outline,
            }
        } else {
            Self {
                fill,
                icon,
                outline,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surface::{Rect, TextMeasurer};
    use surface_app::{Application, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime};

    const INK: Color = Color(115, 27, 38, 255);

    struct Field {
        disabled: bool,
        requests: usize,
    }

    impl Application for Field {
        type Message = ();

        fn update(&mut self, (): ()) {
            self.requests += 1;
        }

        fn view(&self) -> View<()> {
            let action = Theme::light()
                .text_field_trailing_icon_button(
                    "trailing",
                    "清除文字",
                    self.disabled,
                    INK,
                    None,
                    (),
                )
                .unwrap();
            View::new(Node::new(Role::Window, "Test").styled(Style::row().align(Align::Center)))
                .children([
                    View::new(
                        Node::new(Role::TextInput, "名稱")
                            .id("editor")
                            .styled(centered_box(80., 56.)),
                    )
                    .on_input(|_| ()),
                    action,
                ])
        }
    }

    struct Metrics;

    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
            (text.chars().count() as f32 * size, size)
        }
    }

    #[test]
    fn trailing_action_retains_the_icon_button_target_without_an_internal_glyph() {
        assert!(Theme::light()
            .text_field_trailing_icon_button("trailing", " ", false, INK, None, ())
            .is_none());
        for disabled in [false, true] {
            let mut rt = Runtime::new(Field {
                disabled,
                requests: 0,
            })
            .unwrap();
            rt.activate("editor").unwrap();
            rt.render(160., 72., &mut Metrics).unwrap();
            let bounds = |key| {
                let id = rt.dom().by_id(key).unwrap();
                rt.computed().unwrap().nodes[id.index()].bounds
            };
            let editor = bounds("editor");
            let target = bounds("trailing");
            let face = bounds("trailing-face");
            let focus = bounds("trailing-focus");
            let id = rt.dom().by_id("trailing").unwrap();
            let action = rt.dom().get(id).unwrap();
            assert_eq!(action.role(), Role::Button);
            assert_eq!(action.name(), "清除文字");
            assert_eq!(action.is_disabled(), disabled);
            assert_eq!(action.press_ripple_target_name(), Some("trailing-face"));
            assert!(rt.dom().by_id("trailing-icon").is_none());
            assert_eq!(editor.intersect(target).width, 0.);
            assert_eq!(target, Rect::new(80., 12., 48., 48.));
            assert_eq!(face, Rect::new(88., 16., 32., 40.));
            assert_eq!(focus, Rect::new(83., 11., 42., 50.));
            let face_node = rt
                .dom()
                .get(rt.dom().by_id("trailing-face").unwrap())
                .unwrap();
            assert_eq!(
                face_node.style().state_layer,
                if disabled { Color::TRANSPARENT } else { INK }
            );
            assert_eq!(
                rt.accessibility_frame()
                    .unwrap()
                    .activation_target(id)
                    .is_some(),
                !disabled
            );

            let pointer = [
                Event::PointerDown { x: 104., y: 36. },
                Event::PointerUp { x: 104., y: 36. },
            ];
            if !disabled {
                rt.dispatch(Event::Ime(ImeEvent::Preedit {
                    text: "ㄓ".into(),
                    selection: None,
                }))
                .unwrap();
                for event in pointer.clone() {
                    rt.dispatch(event).unwrap();
                }
                assert!(rt.input_snapshot().unwrap().composing);
                assert_eq!(rt.app().requests, 0);
                rt.dispatch(Event::Ime(ImeEvent::Cancel)).unwrap();
                rt.render(160., 72., &mut Metrics).unwrap();
            }
            for event in pointer {
                rt.dispatch(event).unwrap();
            }
            if disabled {
                assert_eq!(rt.app().requests, 0);
            } else {
                assert_eq!(rt.focused(), Some("trailing"));
                assert_eq!(rt.app().requests, 1);
                let space = KeyPress::new(Key::Space, Modifiers::default());
                assert!(rt.key(&space.clone().with_repeat(true)).unwrap());
                assert_eq!(rt.app().requests, 1);
                // An ordinary Space falls through to the platform's activation event.
                assert!(!rt.key(&space).unwrap());
                assert!(rt.dispatch(Event::Activate).unwrap());
                assert_eq!(rt.app().requests, 2);
            }
        }
    }
}
