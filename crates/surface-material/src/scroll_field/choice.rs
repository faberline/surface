//! String choices use the same finite wheel as the numeric field.

use std::rc::Rc;

use crate::{
    shape::LayoutDirection, ScrollChoice, ScrollChoiceEvent, ScrollChoiceQueryError,
    ScrollChoiceState, Theme,
};
use surface_app::View;

use super::{
    field_layout, scale, width_for_ems, FieldCopy, FieldOptions, ScrollField, ScrollFieldEvent,
    ScrollFieldLayout, ScrollFieldSize,
};

/// Allocation and localized copy for one inline text-choice Scroll Field.
#[derive(Clone, Copy)]
pub struct ScrollChoiceFieldOptions<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub size: ScrollFieldSize,
    /// Prefer 120 dp for a compact face or 200 dp for a regular face.
    pub height: f32,
    /// Space assigned to the field, including the mode button and copy.
    pub available_width: f32,
    pub available_height: f32,
    pub text_scale: f32,
    pub direction: LayoutDirection,
    pub disabled: bool,
    pub hint: Option<&'a str>,
    pub use_input: &'a str,
    pub use_scroll: &'a str,
    pub error_text: &'a dyn Fn(ScrollChoiceQueryError) -> String,
    /// Format a label with its optional qualifier. The result is shown on
    /// the wheel and used as its human-readable accessible value.
    pub choice_text: &'a dyn Fn(&str, Option<&str>) -> String,
    /// Describe a unique matching choice below the editor.
    pub suggestion_text: &'a dyn Fn(&str) -> String,
}

impl<'a> From<ScrollChoiceFieldOptions<'a>> for FieldOptions<'a> {
    fn from(options: ScrollChoiceFieldOptions<'a>) -> Self {
        Self {
            id: options.id,
            label: options.label,
            size: options.size,
            height: options.height,
            available_width: options.available_width,
            available_height: options.available_height,
            text_scale: options.text_scale,
            direction: options.direction,
            disabled: options.disabled,
            hint: options.hint,
            use_input: options.use_input,
            use_scroll: options.use_scroll,
        }
    }
}

/// Plan the text wheel. Call `state.update_fit` with the result after changes
/// to the space, catalog, formatted labels, or application text scale.
///
/// Width accounts for the full labels, including qualifiers. With less room,
/// the field uses its existing editor and preserves focus and active text entry.
pub fn scroll_choice_field_layout(
    options: ScrollChoiceFieldOptions<'_>,
    state: &ScrollChoiceState,
) -> Option<ScrollFieldLayout> {
    field_layout(
        options.into(),
        ChoiceCopy::new(options, state).preferred_width(options),
        true,
    )
}

impl Theme {
    /// Build a finite Scroll Field backed by caller-owned text choices.
    ///
    /// Apply messages with `state.apply` and use the returned ID as the
    /// accepted choice. Report fit through [scroll_choice_field_layout] and
    /// call `state.advance` with the application's frame clock while snapping.
    /// The direct editor shows a unique prefix match without changing raw text.
    /// Enter accepts and Escape cancels. Duplicate labels should have visible
    /// qualifiers and distinct input phrases in the catalog.
    pub fn scroll_choice_field<M: Clone + 'static>(
        &self,
        options: ScrollChoiceFieldOptions<'_>,
        state: &ScrollChoiceState,
        on_event: impl Fn(ScrollChoiceEvent) -> M + 'static,
    ) -> View<M> {
        let copy = ChoiceCopy::new(options, state);
        let preferred_width = copy.preferred_width(options);
        // The value route must refer to the published catalog. It cannot
        // reinterpret a stale ordinal after a reorder.
        let ids: Vec<_> = (0..state.count())
            .filter_map(|index| state.get(index).map(|choice| choice.id.to_owned()))
            .collect();
        let emit = move |event| {
            on_event(match event {
                ScrollFieldEvent::Edit => ScrollChoiceEvent::Edit,
                ScrollFieldEvent::SetText(text) => ScrollChoiceEvent::SetText(text),
                ScrollFieldEvent::Submit => ScrollChoiceEvent::Submit,
                ScrollFieldEvent::Cancel => ScrollChoiceEvent::Cancel,
                ScrollFieldEvent::SetValue(index) => {
                    let id = ids.get(index as usize).cloned().unwrap_or_default();
                    ScrollChoiceEvent::SetChoice(id)
                }
                ScrollFieldEvent::Step(step) => ScrollChoiceEvent::Step(step),
                ScrollFieldEvent::Drag {
                    session,
                    sample,
                    row_height,
                } => ScrollChoiceEvent::Drag {
                    session,
                    sample,
                    row_height,
                },
            })
        };
        ScrollField {
            theme: self,
            options: options.into(),
            wheel: state.wheel(),
            copy: FieldCopy::Choice(copy),
            preferred_width,
            emit: Rc::new(emit),
        }
        .view()
    }
}

type ChoiceText<'a> = &'a dyn Fn(&str, Option<&str>) -> String;

pub(super) struct ChoiceCopy<'state, 'copy> {
    pub(super) state: &'state ScrollChoiceState,
    pub(super) error_text: &'copy dyn Fn(ScrollChoiceQueryError) -> String,
    choice_text: ChoiceText<'copy>,
    suggestion_text: &'copy dyn Fn(&str) -> String,
}

impl<'state, 'copy> ChoiceCopy<'state, 'copy> {
    fn new(options: ScrollChoiceFieldOptions<'copy>, state: &'state ScrollChoiceState) -> Self {
        Self {
            state,
            error_text: options.error_text,
            choice_text: options.choice_text,
            suggestion_text: options.suggestion_text,
        }
    }

    pub(super) fn qualified(&self, choice: ScrollChoice<'_>) -> String {
        let formatted = (self.choice_text)(choice.label, choice.detail);
        let label = if formatted.trim().is_empty() {
            choice.label
        } else {
            &formatted
        };
        label.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    pub(super) fn suggestion(&self) -> Option<String> {
        let choice = self.state.suggestion()?;
        Some((self.suggestion_text)(&self.qualified(choice)))
    }

    fn preferred_width(&self, options: ScrollChoiceFieldOptions<'_>) -> f32 {
        // Reserve room before the layout pass. The common paint group measures
        // each visible label with the current text renderer.
        let longest = (0..self.state.count())
            .filter_map(|index| self.state.get(index))
            .map(|choice| self.qualified(choice).chars().map(estimated_advance).sum())
            .fold(0., f32::max);
        width_for_ems(options.size, longest, scale(options.text_scale))
    }
}

fn estimated_advance(glyph: char) -> f32 {
    match glyph {
        ' ' => 0.4,
        'i' | 'l' | 'I' | '.' | ',' | '\'' | ':' | ';' | '!' => 0.46,
        'm' | 'w' | 'M' | 'W' | '@' | '%' => 1.,
        _ if glyph.is_ascii() => 0.78,
        _ => 1.08,
    }
}
