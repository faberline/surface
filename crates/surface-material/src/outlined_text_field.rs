//! Single-line Material 3 fields with a stable Runtime editor.

use std::f32::consts::FRAC_PI_2;

use surface::{
    Color, FillContour, Length, MeasuredGroupLayout, Node, Rect, Role, Style, TextAlignment,
    TextMeasurer, TextScaling, TextStyle,
};
use surface_app::{CursorStyle, Key, TextInputPaintState, TextOptions, View};

use crate::{
    shape::{self, LayoutDirection},
    typography, TextFieldOptions, Theme,
};

const FACE_MIN: f32 = 56.;
const SIDE: f32 = 16.;
const NOTCH_PAD: f32 = 4.;
const DESCRIPTION_GAP: f32 = 4.;
const CORNER: f32 = 4.;
const FILLED_VERTICAL_PADDING: f32 = 8.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldAppearance {
    Outlined,
    Filled,
}

/// The actions the field allows. Read-only fields can still select and copy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextFieldEditability {
    #[default]
    Editable,
    ReadOnly,
    Disabled,
}

/// Caller-owned data and presentation for a single-line Material text field.
#[derive(Clone, Copy, Debug)]
pub struct OutlinedTextFieldOptions<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub value: &'a str,
    pub placeholder: Option<&'a str>,
    pub supporting_text: Option<&'a str>,
    /// Replaces the supporting text and marks an enabled field invalid.
    pub error: Option<&'a str>,
    pub editability: TextFieldEditability,
    pub direction: LayoutDirection,
    pub text_scale: f32,
}

impl<'a> OutlinedTextFieldOptions<'a> {
    pub fn new(id: &'a str, label: &'a str, value: &'a str) -> Self {
        Self {
            id,
            label,
            value,
            placeholder: None,
            supporting_text: None,
            error: None,
            editability: TextFieldEditability::Editable,
            direction: LayoutDirection::LeftToRight,
            text_scale: 1.,
        }
    }
}

impl Theme {
    /// Build an outlined field which uses the width supplied by its parent.
    ///
    /// Keep the ID stable across changes to value, validation, theme and scale.
    /// The label floats when the field has focus or content. An optional hint
    /// shows only in the focused empty field outside IME composition. A long
    /// label that cannot fit inside the field stays above the outline.
    pub fn outlined_text_field<M>(
        &self,
        options: OutlinedTextFieldOptions<'_>,
        on_input: impl Fn(String) -> M + 'static,
    ) -> Option<View<M>> {
        self.outlined_text_field_with_submit(options, on_input, || None)
    }

    /// Also handle a plain, first Enter while the editable field has focus.
    /// The Runtime gives an active IME composition its keys before submission.
    pub fn outlined_text_field_with_submit<M>(
        &self,
        options: OutlinedTextFieldOptions<'_>,
        on_input: impl Fn(String) -> M + 'static,
        on_submit: impl Fn() -> Option<M> + 'static,
    ) -> Option<View<M>> {
        self.material_text_field(FieldAppearance::Outlined, options, on_input, on_submit)
    }

    /// Build a filled field with a bottom indicator and an inside label.
    /// Keep the ID stable to retain the editor through appearance or layout changes.
    pub fn filled_text_field<M>(
        &self,
        options: TextFieldOptions<'_>,
        on_input: impl Fn(String) -> M + 'static,
    ) -> Option<View<M>> {
        self.filled_text_field_with_submit(options, on_input, || None)
    }

    /// Also handle a plain, first Enter while the editable field has focus.
    /// The Runtime gives an active IME composition its keys before submission.
    pub fn filled_text_field_with_submit<M>(
        &self,
        options: TextFieldOptions<'_>,
        on_input: impl Fn(String) -> M + 'static,
        on_submit: impl Fn() -> Option<M> + 'static,
    ) -> Option<View<M>> {
        self.material_text_field(FieldAppearance::Filled, options, on_input, on_submit)
    }

    fn material_text_field<M>(
        &self,
        appearance: FieldAppearance,
        options: TextFieldOptions<'_>,
        on_input: impl Fn(String) -> M + 'static,
        on_submit: impl Fn() -> Option<M> + 'static,
    ) -> Option<View<M>> {
        let label = visible_text(Some(options.label))?;
        if options.id.trim().is_empty()
            || !options.text_scale.is_finite()
            || options.text_scale <= 0.
        {
            return None;
        }
        let body = typography::BODY_LARGE.scaled(options.text_scale);
        let caption = typography::BODY_SMALL.scaled(options.text_scale);
        let line_height = body.line_height.filter(|height| height.is_finite())?;
        let caption_height = caption.line_height.filter(|height| height.is_finite())?;
        let disabled = options.editability == TextFieldEditability::Disabled;
        let error = visible_text(options.error);
        let description = error.or_else(|| visible_text(options.supporting_text));
        let rgba = self.colors;
        let filled = appearance == FieldAppearance::Filled;
        let tokens = FieldColors {
            disabled,
            error: error.is_some(),
            boundary: if filled {
                rgba.on_surface_variant
            } else {
                rgba.outline
            },
            neutral_label: rgba.on_surface_variant,
            boundary_hover: rgba.on_surface,
            label_hover: if filled {
                rgba.on_surface_variant
            } else {
                rgba.on_surface
            },
            primary: rgba.primary,
            error_ink: rgba.error,
            error_hover: rgba.on_error_container,
            disabled_boundary: translucent(rgba.on_surface, if filled { 97 } else { 31 }),
            disabled_text: translucent(rgba.on_surface, 97),
        };
        let rtl = options.direction == LayoutDirection::RightToLeft;
        let alignment = if rtl {
            TextAlignment::Right
        } else {
            TextAlignment::Left
        };
        let id = options.id;
        let description_id = format!("{id}-description");
        let floating_id = format!("{id}-floating-label");
        let disabled_value = disabled && !options.value.is_empty();
        let mut input = Node::new(Role::TextInput, label)
            .id(id)
            .text(options.value)
            .disabled(disabled)
            .invalid(!disabled && error.is_some())
            .text_scaling(TextScaling::Fixed)
            .text_input_caret_color(if error.is_some() {
                rgba.error
            } else {
                rgba.primary
            })
            .focus_color(Color::TRANSPARENT)
            .styled(
                Style::default()
                    .with_text_style(body.clone())
                    .foreground(if disabled {
                        Color::TRANSPARENT
                    } else {
                        rgba.on_surface
                    }),
            );
        if description.is_some() {
            input = input.described_by_text(&description_id);
        }
        let mut editor = View::new(input)
            .text_options(TextOptions {
                read_only: options.editability == TextFieldEditability::ReadOnly,
                ..TextOptions::default()
            })
            .inset_single_line_text_horizontally(SIDE)
            .accessibility_scroll_into_view();
        editor = if filled {
            editor.inset_single_line_text_from_top(FILLED_VERTICAL_PADDING + caption_height)
        } else {
            editor.center_single_line_text()
        };
        if !disabled {
            editor = editor.on_input(on_input).cursor(CursorStyle::Text);
        }
        if options.editability == TextFieldEditability::Editable {
            editor = editor.on_key(move |press| {
                if press.key == Key::Enter && press.modifiers.is_empty() && !press.repeat {
                    on_submit()
                } else {
                    None
                }
            });
        }
        if let Some(hint) = visible_text(options.placeholder) {
            editor = editor.focused_text_input_placeholder(hint, rgba.on_surface_variant);
        }
        if rtl {
            editor = editor.right_align_single_line_text();
        }

        let boundary_part = if filled { "indicator" } else { "outline" };
        let boundary = View::new(Node::new(Role::Group, "").id(format!("{id}-{boundary_part}")))
            .without_interaction(None)
            .paint_text_input_contour(
                id,
                (!filled).then_some(floating_id.as_str()),
                move |state, floating| {
                    let width = state.bounds.width;
                    let height = state.bounds.height;
                    let thickness = if state.is_focused && !disabled {
                        3.
                    } else {
                        1.
                    };
                    let ink = tokens.boundary(state);
                    if filled {
                        return FillContour::new(
                            [
                                [0., height - thickness],
                                [width, height - thickness],
                                [width, height],
                                [0., height],
                            ],
                            ink,
                        );
                    }
                    let gap = floating
                        .filter(|rect| {
                            floats(state)
                                && rect.width > 0.
                                && rect.y + rect.height > state.bounds.y
                        })
                        .map(|rect| {
                            let left = rect.x - state.bounds.x;
                            (left - NOTCH_PAD, left + rect.width + NOTCH_PAD)
                        });
                    outline_contour(width, height, thickness, gap, ink)
                },
            );
        let label_view = |id, font, shown: fn(TextInputPaintState) -> bool| {
            View::new(
                Node::new(Role::Text, "")
                    .id(id)
                    .text(label)
                    .text_alignment(alignment)
                    .text_scaling(TextScaling::Fixed)
                    .styled(Style::default().with_text_style(font)),
            )
            .without_interaction(None)
            .paint_text_input_foreground(options.id, move |state| {
                shown(state).then(|| tokens.label(state))
            })
        };
        let inline = label_view(format!("{id}-inline-label"), body.clone(), |state| {
            !floats(state)
        });
        let floating = label_view(floating_id, caption.clone(), floats);
        let fallback = label_view(format!("{id}-above-label"), caption.clone(), |_| true);
        // Disabled peers have an authored value but do not mount an editor.
        // Draw their value in the same inset as an enabled single line.
        let disabled_ink = disabled_value.then(|| {
            View::new(
                Node::new(Role::Text, "")
                    .id(format!("{id}-disabled-value"))
                    .text(options.value)
                    .text_alignment(alignment)
                    .text_scaling(TextScaling::Fixed)
                    .styled(
                        Style::default()
                            .with_text_style(body.clone())
                            .foreground(tokens.disabled_text)
                            .clip(true),
                    ),
            )
            .without_interaction(None)
        });
        let description_view = description.map(|description| {
            View::new(
                Node::new(Role::Text, "")
                    .id(description_id)
                    .text(description)
                    .text_alignment(alignment)
                    .text_scaling(TextScaling::Fixed)
                    .styled(
                        Style::default()
                            .with_text_style(caption.clone())
                            .foreground(if disabled {
                                tokens.disabled_text
                            } else if error.is_some() {
                                tokens.error_ink
                            } else {
                                tokens.neutral_label
                            }),
                    ),
            )
        });
        let validation_status = View::new(
            Node::new(Role::Status, "")
                .id(format!("{id}-validation-status"))
                .text(error.filter(|_| !disabled).unwrap_or_default())
                .assertive_live_status()
                .styled(Style::default().foreground(Color::TRANSPARENT)),
        );
        let mut shell = Style::column().width(Length::Percent(1.));
        shell.min_width = FACE_MIN;
        let layout = FieldLayout {
            appearance,
            label: label.to_owned(),
            description: description.map(str::to_owned),
            body,
            caption,
            line_height,
            caption_height,
            rtl,
            disabled_value,
        };
        let filled_backgrounds = filled.then(|| {
            let container = if disabled {
                translucent(rgba.on_surface, 10)
            } else {
                rgba.surface_container_highest
            };
            let layer = |part, color| {
                View::new(
                    Node::new(Role::Group, "")
                        .id(format!("{id}-{part}"))
                        .styled(
                            Style::default()
                                .corners(shape::EXTRA_SMALL_TOP)
                                .background(color),
                        ),
                )
                .without_interaction(None)
            };
            [
                layer("container", container),
                layer("hover-layer", translucent(rgba.on_surface, 20))
                    .paint_when_text_input(id, |state| state.is_hovered && !state.is_focused),
            ]
        });
        Some(
            View::new(
                Node::new(Role::Group, "")
                    .id(format!("{id}-field"))
                    .styled(shell)
                    .measured_group(layout),
            )
            .children(
                filled_backgrounds
                    .into_iter()
                    .flatten()
                    .chain([editor, boundary, inline, floating, fallback])
                    .chain(disabled_ink)
                    .chain(description_view)
                    .chain([validation_status]),
            ),
        )
    }
}

fn visible_text(text: Option<&str>) -> Option<&str> {
    text.filter(|value| !value.trim().is_empty())
}

fn floats(state: TextInputPaintState) -> bool {
    state.is_focused || state.is_composing || !state.is_empty
}

fn translucent(Color(r, g, b, _): Color, alpha: u8) -> Color {
    Color(r, g, b, alpha)
}

#[derive(Clone, Copy)]
struct FieldColors {
    disabled: bool,
    error: bool,
    boundary: Color,
    neutral_label: Color,
    boundary_hover: Color,
    label_hover: Color,
    primary: Color,
    error_ink: Color,
    error_hover: Color,
    disabled_boundary: Color,
    disabled_text: Color,
}

impl FieldColors {
    fn interaction_ink(self, state: TextInputPaintState, normal: Color, hover: Color) -> Color {
        match (self.error, state.is_focused, state.is_hovered) {
            (true, false, true) => self.error_hover,
            (true, _, _) => self.error_ink,
            (false, true, _) => self.primary,
            (false, false, true) => hover,
            (false, false, false) => normal,
        }
    }

    fn boundary(self, state: TextInputPaintState) -> Color {
        if self.disabled {
            self.disabled_boundary
        } else {
            self.interaction_ink(state, self.boundary, self.boundary_hover)
        }
    }

    fn label(self, state: TextInputPaintState) -> Color {
        if self.disabled {
            self.disabled_text
        } else {
            self.interaction_ink(state, self.neutral_label, self.label_hover)
        }
    }
}

#[derive(Debug)]
struct FieldLayout {
    appearance: FieldAppearance,
    label: String,
    description: Option<String>,
    body: TextStyle,
    caption: TextStyle,
    line_height: f32,
    caption_height: f32,
    rtl: bool,
    disabled_value: bool,
}

impl FieldLayout {
    fn rects(&self, width: f32, fonts: &mut dyn TextMeasurer) -> Vec<Rect> {
        let content_width = (width - 2. * SIDE).max(1.);
        let (inline_width, inline_height) = fonts.measure_text(&self.label, &self.body, None);
        let (float_width, float_height) = fonts.measure_text(&self.label, &self.caption, None);
        let label_fits = inline_width <= content_width
            && float_width <= content_width
            && inline_height <= self.line_height + 0.5
            && float_height <= self.caption_height + 0.5;
        let above_height = if label_fits {
            0.
        } else {
            fonts
                .measure_text(&self.label, &self.caption, Some(content_width))
                .1
                .max(self.caption_height)
                .ceil()
        };
        let filled = self.appearance == FieldAppearance::Filled;
        let (face_padding, label_slot) = if filled {
            (FILLED_VERTICAL_PADDING, self.caption_height)
        } else {
            (SIDE, 0.)
        };
        let top = if label_fits {
            if filled {
                0.
            } else {
                self.caption_height / 2.
            }
        } else {
            above_height + DESCRIPTION_GAP
        };
        let face_height = FACE_MIN.max((self.line_height + label_slot + 2. * face_padding).ceil());
        let face = Rect::new(0., top, width, face_height);
        let empty = Rect::new(0., 0., 0., 0.);
        let content = Rect::new(
            SIDE,
            top + (face_height - self.line_height) / 2.,
            content_width,
            self.line_height,
        );
        let inline = if label_fits { content } else { empty };
        let floating = if label_fits {
            let x = if self.rtl {
                width - SIDE - float_width
            } else {
                SIDE
            };
            let y = if filled { top + face_padding } else { 0. };
            Rect::new(x, y, float_width, self.caption_height)
        } else {
            empty
        };
        let above = Rect::new(SIDE, 0., content_width, above_height);
        let mut rects = Vec::new();
        if filled {
            rects.extend([face, face]);
        }
        rects.extend([face, face, inline, floating, above]);
        if self.disabled_value {
            let y = if filled {
                top + face_padding + label_slot
            } else {
                content.y
            };
            rects.push(Rect::new(content.x, y, content.width, content.height));
        }
        if let Some(description) = &self.description {
            let height = fonts
                .measure_text(description, &self.caption, Some(content_width))
                .1
                .max(self.caption_height)
                .ceil();
            rects.push(Rect::new(
                SIDE,
                top + face_height + DESCRIPTION_GAP,
                content_width,
                height,
            ));
        }
        // The live region needs visible geometry even when it is empty.
        rects.push(Rect::new(1., 0., 1., 1.));
        rects
    }
}

impl MeasuredGroupLayout for FieldLayout {
    fn measure(
        &self,
        _children: &[&Node],
        available: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let width = available.unwrap_or(280.).max(FACE_MIN);
        let height = self
            .rects(width, fonts)
            .iter()
            .map(|rect| rect.y + rect.height)
            .fold(0., f32::max);
        (width, height)
    }

    fn place(
        &self,
        _children: &[&Node],
        (width, _height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        self.rects(width, fonts)
    }
}

fn outline_contour(
    width: f32,
    height: f32,
    thickness: f32,
    gap: Option<(f32, f32)>,
    color: Color,
) -> Option<FillContour> {
    if width < 2. * CORNER || height < 2. * CORNER {
        return None;
    }
    // Trace the outer edge clockwise and the inner edge back. For a closed
    // outline the two edges share one seam, which cancels under even-odd fill.
    let (left, right) = gap
        .filter(|&(left, right)| left >= CORNER && right <= width - CORNER && left < right)
        .unwrap_or((width / 2., width / 2.));
    let edge = |inset| {
        let radius = CORNER - inset;
        let mut points = vec![[right, inset]];
        for (x, y, start) in [
            (width - CORNER, CORNER, -FRAC_PI_2),
            (width - CORNER, height - CORNER, 0.),
            (CORNER, height - CORNER, FRAC_PI_2),
            (CORNER, CORNER, 2. * FRAC_PI_2),
        ] {
            for step in 0..=4 {
                let angle = start + step as f32 * FRAC_PI_2 / 4.;
                points.push([x + radius * angle.cos(), y + radius * angle.sin()]);
            }
        }
        points.push([left, inset]);
        points
    };
    FillContour::new(
        edge(0.)
            .into_iter()
            .chain(edge(thickness).into_iter().rev()),
        color,
    )
}
