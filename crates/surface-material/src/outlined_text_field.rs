//! Single-line Material 3 fields with a stable Runtime editor.

use std::f32::consts::FRAC_PI_2;

use surface::{
    Color, FillContour, Length, MeasuredGroupLayout, Node, Rect, Role, Style, TextAlignment,
    TextMeasurer, TextScaling, TextStyle,
};
use surface_app::{CursorStyle, Key, TextInputPaintState, TextOptions, View};

use crate::{
    shape::{self, LayoutDirection},
    text_field_adornments::{physical, AffixText, FieldAdornments, ACTION_SIZE, ICON_SIZE, SIDE},
    typography, TextFieldAdornments, TextFieldAppearance, TextFieldOptions, Theme,
};

const FACE_MIN: f32 = 56.;
const NOTCH_PAD: f32 = 4.;
const DESCRIPTION_GAP: f32 = 4.;
const CORNER: f32 = 4.;
const FILLED_VERTICAL_PADDING: f32 = 8.;

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
        self.text_field_with_adornments(
            TextFieldAppearance::Outlined,
            options,
            TextFieldAdornments::default(),
            on_input,
            on_submit,
        )
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
        self.text_field_with_adornments(
            TextFieldAppearance::Filled,
            options,
            TextFieldAdornments::default(),
            on_input,
            on_submit,
        )
    }

    /// Add passive icons or affixes and an independent named trailing action.
    /// Affixes and the action stay separate from the committed editor value.
    /// In narrow space the affixes and action can move below the field.
    pub fn text_field_with_adornments<M>(
        &self,
        appearance: TextFieldAppearance,
        options: TextFieldOptions<'_>,
        adornments: TextFieldAdornments<'_, M>,
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
        let filled = appearance == TextFieldAppearance::Filled;
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
        let action_id = format!("{id}-trailing-action");
        let value_box_id = format!("{id}-value-box");
        let affix_text = |affix: crate::TextFieldAffix<'_>| {
            Some(AffixText {
                visual: visible_text(Some(affix.visual))?.to_owned(),
                spoken_name: visible_text(Some(affix.spoken_name))?.to_owned(),
            })
        };
        let field_adornments = FieldAdornments {
            leading: adornments.leading_icon.is_some(),
            trailing: adornments.trailing_action.is_some(),
            prefix: adornments.prefix.and_then(affix_text),
            suffix: adornments.suffix.and_then(affix_text),
        };
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
        input = input.described_by_texts(
            [
                field_adornments.prefix.as_ref().map(|_| "prefix"),
                field_adornments.suffix.as_ref().map(|_| "suffix"),
            ]
            .into_iter()
            .flatten()
            .map(|part| format!("{id}-{part}-description")),
        );
        let mut editor = View::new(input)
            .text_options(TextOptions {
                read_only: options.editability == TextFieldEditability::ReadOnly,
                ..TextOptions::default()
            })
            .inset_single_line_text_horizontally(SIDE)
            .accessibility_scroll_into_view();
        if field_adornments.has_slots() {
            editor = editor.single_line_text_horizontal_bounds_from(&value_box_id);
        }
        if field_adornments.trailing {
            editor = editor.text_input_hover_with_action(&action_id);
        }
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
            .paint_text_input_contour_with_bounds(
                id,
                (!filled).then_some(floating_id.as_str()),
                move |state, face, floating| {
                    let width = face.width;
                    let height = face.height;
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
                            floats(state) && rect.width > 0. && rect.y + rect.height > face.y
                        })
                        .map(|rect| {
                            let left = rect.x - face.x;
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
        let mut slots = Vec::new();
        if field_adornments.has_slots() {
            slots.push(
                View::new(Node::new(Role::Group, "").id(value_box_id)).without_interaction(None),
            );
        }
        if let Some(icon) = adornments.leading_icon {
            let ink = if disabled {
                tokens.disabled_text
            } else {
                tokens.neutral_label
            };
            let leading = icon.contour_dp(ICON_SIZE, ink)?;
            slots.push(
                View::new(
                    Node::new(Role::Group, "")
                        .id(format!("{id}-leading-icon"))
                        .styled(Style::default().clip(true)),
                )
                .paint_text_input_contour_with_bounds(
                    id,
                    None,
                    move |_, bounds, _| {
                        (bounds.width > 0. && bounds.height > 0.).then(|| leading.clone())
                    },
                ),
            );
        }
        if let Some(action) = adornments.trailing_action {
            let disabled = disabled || action.disabled;
            let ink = if disabled {
                tokens.disabled_text
            } else if tokens.error {
                tokens.error_ink
            } else {
                tokens.neutral_label
            };
            let icon = action.icon.contour_dp(ICON_SIZE, ink)?;
            let icon_hover = if !disabled && tokens.error {
                action.icon.contour_dp(ICON_SIZE, tokens.error_hover)?
            } else {
                icon.clone()
            };
            slots.push(
                self.text_field_trailing_icon_button(
                    &action_id,
                    action.name,
                    disabled,
                    ink,
                    action.on_press,
                )?
                .restore_focus_on_removal(id)
                .accessibility_scroll_into_view(),
            );
            slots.push(
                View::new(
                    Node::new(Role::Group, "")
                        .id(format!("{id}-trailing-icon"))
                        .styled(Style::default().clip(true)),
                )
                .paint_text_input_contour(id, None, move |state, _| {
                    Some(if state.is_hovered && !state.is_focused {
                        icon_hover.clone()
                    } else {
                        icon.clone()
                    })
                }),
            );
        }
        let affix_ink = if disabled {
            tokens.disabled_text
        } else {
            tokens.neutral_label
        };
        for (part, affix) in [
            ("prefix", field_adornments.prefix.as_ref()),
            ("suffix", field_adornments.suffix.as_ref()),
        ] {
            let Some(affix) = affix else { continue };
            let text = |kind: &str, value: &str, font: TextStyle, ink| {
                View::new(
                    Node::new(Role::Text, "")
                        .id(format!("{id}-{part}-{kind}"))
                        .text(value)
                        .text_alignment(alignment)
                        .text_scaling(TextScaling::Fixed)
                        .styled(Style::default().with_text_style(font).foreground(ink)),
                )
            };
            slots.push(
                text("visual", &affix.visual, body.clone(), affix_ink)
                    .paint_when_text_input(id, floats),
            );
            slots.push(
                text("wrapped", &affix.spoken_name, caption.clone(), affix_ink)
                    .without_interaction(None),
            );
            slots.push(text(
                "description",
                &affix.spoken_name,
                caption.clone(),
                Color::TRANSPARENT,
            ));
        }
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
            adornments: field_adornments,
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
                    .chain([validation_status])
                    .chain(slots),
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
    appearance: TextFieldAppearance,
    label: String,
    description: Option<String>,
    body: TextStyle,
    caption: TextStyle,
    line_height: f32,
    caption_height: f32,
    rtl: bool,
    disabled_value: bool,
    adornments: FieldAdornments,
}

impl FieldLayout {
    fn rects(&self, width: f32, fonts: &mut dyn TextMeasurer) -> Vec<Rect> {
        let content_width = (width - 2. * SIDE).max(1.);
        let filled = self.appearance == TextFieldAppearance::Filled;
        let slots = self
            .adornments
            .measure(width, fonts, &self.body, self.line_height);
        let body_width = slots.body_end - slots.body_start;
        let floating_start = if filled { slots.body_start } else { SIDE };
        let floating_width = if filled { body_width } else { content_width };
        let (inline_width, inline_height) = fonts.measure_text(&self.label, &self.body, None);
        let (float_width, float_height) = fonts.measure_text(&self.label, &self.caption, None);
        let label_fits = inline_width <= body_width
            && float_width <= floating_width
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
        let physical = |rect| physical(rect, width, self.rtl);
        let center_line_y = top + (face_height - self.line_height) / 2.;
        let line_y = if filled {
            top + face_padding + label_slot
        } else {
            center_line_y
        };
        let editor = if slots.trailing_inline {
            physical(Rect::new(0., top, width - ACTION_SIZE, face_height))
        } else {
            face
        };
        let value = physical(slots.value(line_y, self.line_height));
        let content = physical(Rect::new(
            slots.body_start,
            center_line_y,
            body_width,
            self.line_height,
        ));
        let inline = if label_fits { content } else { empty };
        let floating = if label_fits {
            let y = if filled { top + face_padding } else { 0. };
            physical(Rect::new(
                floating_start,
                y,
                float_width,
                self.caption_height,
            ))
        } else {
            empty
        };
        let above = Rect::new(SIDE, 0., content_width, above_height);
        let mut rects = Vec::new();
        if filled {
            rects.extend([face, face]);
        }
        rects.extend([editor, face, inline, floating, above]);
        if self.disabled_value {
            rects.push(value);
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
        if self.adornments.has_slots() {
            rects.push(value);
        }
        let icon_inset = (ACTION_SIZE - ICON_SIZE) / 2.;
        if self.adornments.leading {
            let icon = Rect::new(
                icon_inset,
                top + (face_height - ICON_SIZE) / 2.,
                ICON_SIZE,
                ICON_SIZE,
            );
            rects.push(if slots.leading { physical(icon) } else { empty });
        }
        let mut below = rects
            .iter()
            .map(|rect| rect.y + rect.height)
            .fold(0., f32::max)
            + DESCRIPTION_GAP;
        if self.adornments.trailing {
            let y = if slots.trailing_inline {
                top + (face_height - ACTION_SIZE) / 2.
            } else {
                let y = below;
                below += ACTION_SIZE + DESCRIPTION_GAP;
                y
            };
            let x = width - ACTION_SIZE;
            rects.push(physical(Rect::new(x, y, ACTION_SIZE, ACTION_SIZE)));
            rects.push(physical(Rect::new(
                x + icon_inset,
                y + icon_inset,
                ICON_SIZE,
                ICON_SIZE,
            )));
        }
        for (affix, left, width) in [
            (
                self.adornments.prefix.as_ref(),
                slots.body_start,
                slots.prefix_width,
            ),
            (
                self.adornments.suffix.as_ref(),
                slots.body_end - slots.suffix_width,
                slots.suffix_width,
            ),
        ] {
            let Some(affix) = affix else { continue };
            if slots.affixes_inline {
                rects.push(physical(Rect::new(left, line_y, width, self.line_height)));
                rects.push(empty);
            } else {
                let height = fonts
                    .measure_text(&affix.spoken_name, &self.caption, Some(content_width))
                    .1
                    .max(self.caption_height)
                    .ceil();
                rects.push(empty);
                rects.push(Rect::new(SIDE, below, content_width, height));
                below += height + DESCRIPTION_GAP;
            }
            rects.push(physical(Rect::new(SIDE, top + face_height / 2., 1., 1.)));
        }
        rects
    }
}

impl MeasuredGroupLayout for FieldLayout {
    fn places_measured_groups(&self) -> bool {
        true
    }

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
