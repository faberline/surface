//! Retained selection and editable text for a finite Scroll Field catalog.

use surface_app::{AnimationUpdate, PointDragSession};

use crate::{
    scroll_choice_catalog::{
        ScrollChoice, ScrollChoiceCatalog, ScrollChoiceCatalogError, ScrollChoiceItem,
        ScrollChoiceMatchKind, ScrollChoiceQueryError,
    },
    scroll_field_state::{
        ScrollFieldDrag, ScrollFieldEvent, ScrollFieldMode, ScrollFieldRange, ScrollFieldState,
        ScrollFieldStep,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub enum ScrollChoiceEvent {
    Edit,
    SetText(String),
    Submit,
    Cancel,
    SetChoice(String),
    Step(ScrollFieldStep),
    Drag {
        session: PointDragSession,
        /// A tap value is the zero-based choice index in this capture's catalog.
        sample: ScrollFieldDrag,
        row_height: f32,
    },
}

/// Caller-owned retained state for a finite ordered list of text choices.
pub struct ScrollChoiceState {
    catalog: ScrollChoiceCatalog,
    wheel: ScrollFieldState,
    scroll_fits: bool,
    raw: String,
    edited: bool,
    error: Option<ScrollChoiceQueryError>,
}

impl ScrollChoiceState {
    /// Validate the choice list and initial ID. The default key compares
    /// trimmed, case-sensitive Unicode text.
    pub fn new<'a>(
        items: impl IntoIterator<Item = ScrollChoiceItem<'a>>,
        committed_id: &str,
    ) -> Result<Self, ScrollChoiceCatalogError> {
        ScrollChoiceCatalog::new(items, committed_id)
            .map(|catalog| Self::from_catalog(catalog, committed_id))
    }

    /// Supply one deterministic match-key function for labels, aliases and
    /// input. This allows matching rules specific to the application's locale.
    pub fn with_match_key<'a>(
        items: impl IntoIterator<Item = ScrollChoiceItem<'a>>,
        committed_id: &str,
        match_key: impl Fn(&str) -> String + 'static,
    ) -> Result<Self, ScrollChoiceCatalogError> {
        ScrollChoiceCatalog::with_match_key(items, committed_id, match_key)
            .map(|catalog| Self::from_catalog(catalog, committed_id))
    }

    fn from_catalog(catalog: ScrollChoiceCatalog, committed_id: &str) -> Self {
        let wheel = ordinal_wheel(&catalog, committed_id);
        let raw = catalog
            .at(wheel.value() as usize)
            .expect("validated choice")
            .label
            .to_owned();
        Self {
            catalog,
            wheel,
            scroll_fits: true,
            raw,
            edited: false,
            error: None,
        }
    }

    pub fn count(&self) -> usize {
        self.catalog.items().len()
    }

    /// Get a choice at a zero-based index in the current authored order.
    pub fn get(&self, index: usize) -> Option<ScrollChoice<'_>> {
        self.catalog.at(index)
    }

    /// The accepted choice, independently of a held preview or text draft.
    pub fn choice(&self) -> ScrollChoice<'_> {
        self.get(self.index()).expect("validated choice")
    }

    /// Zero-based index of the accepted choice in the current catalog.
    pub fn index(&self) -> usize {
        self.wheel.value() as usize
    }

    /// Index of the choice displayed closest to the center.
    pub fn preview_index(&self) -> usize {
        self.wheel.preview() as usize
    }

    /// Fractional row position from the first choice.
    pub fn offset(&self) -> f64 {
        self.wheel.offset()
    }

    pub fn mode(&self) -> ScrollFieldMode {
        self.wheel.mode()
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn error(&self) -> Option<ScrollChoiceQueryError> {
        self.error
    }

    /// A visible completion candidate. The user still needs to submit
    /// the draft. Exact, empty and ambiguous input have no completion.
    pub fn suggestion(&self) -> Option<ScrollChoice<'_>> {
        if self.mode() != ScrollFieldMode::Input || !self.edited {
            return None;
        }
        let candidate = self.catalog.match_query(&self.raw).ok()?;
        (candidate.kind == ScrollChoiceMatchKind::Prefix).then_some(candidate.choice)
    }

    pub fn drag_session(&self) -> PointDragSession {
        self.wheel.drag_session()
    }

    pub fn is_dragging(&self) -> bool {
        self.wheel.is_dragging()
    }

    pub fn settling(&self) -> bool {
        self.wheel.settling()
    }

    pub(crate) fn wheel(&self) -> &ScrollFieldState {
        &self.wheel
    }

    /// Preserve a mounted editor across resize. Keep its stable view ID so
    /// focus and active composition remain with the editor.
    pub fn update_fit(&mut self, scroll_fits: bool) -> bool {
        self.scroll_fits = scroll_fits;
        self.wheel.update_fit(scroll_fits)
    }

    pub fn invalidate_layout(&mut self) -> bool {
        self.wheel.invalidate_layout()
    }

    /// Apply a current choice ID from the caller without emitting a commit.
    /// A mounted editor keeps its current text until submit or cancel. This
    /// also preserves pending IME input that has not changed the draft yet.
    pub fn sync_choice(&mut self, id: &str) -> bool {
        let Some(index) = self.catalog.index_of(id) else {
            return false;
        };
        let changed = self.wheel.sync_value(index as i32);
        if changed && self.mode() != ScrollFieldMode::Input {
            self.reset_draft();
        }
        changed
    }

    /// Atomically replace the catalog and its accepted ID without emitting a
    /// commit and retire the old capture. A mounted editor keeps its text
    /// and any pending IME input. A typed draft matches the new catalog.
    pub fn replace_items<'a>(
        &mut self,
        items: impl IntoIterator<Item = ScrollChoiceItem<'a>>,
        committed_id: &str,
    ) -> Result<(), ScrollChoiceCatalogError> {
        let editing = self.mode() == ScrollFieldMode::Input;
        self.catalog.replace_items(items, committed_id)?;
        self.wheel = ordinal_wheel(&self.catalog, committed_id);
        if editing {
            self.wheel.apply(ScrollFieldEvent::Edit, true);
        }
        self.wheel.update_fit(self.scroll_fits);
        if editing {
            self.refresh_error();
        } else {
            self.reset_draft();
        }
        Ok(())
    }

    /// Apply a field event. Only a changed, accepted choice emits its stable
    /// ID. Input suggestions, movement, cancellation and rebuilds emit none.
    pub fn apply(&mut self, event: ScrollChoiceEvent, reduced_motion: bool) -> Option<String> {
        match (self.mode(), event) {
            (ScrollFieldMode::Scroll, ScrollChoiceEvent::Edit) => {
                self.wheel.apply(ScrollFieldEvent::Edit, reduced_motion);
                self.reset_draft();
                None
            }
            (ScrollFieldMode::Input, ScrollChoiceEvent::SetText(raw)) => {
                self.raw = raw;
                self.edited = true;
                self.refresh_error();
                None
            }
            (ScrollFieldMode::Input, ScrollChoiceEvent::Submit) => self.submit(reduced_motion),
            (ScrollFieldMode::Input, ScrollChoiceEvent::Cancel) => {
                self.wheel.apply(ScrollFieldEvent::Cancel, reduced_motion);
                self.reset_draft();
                None
            }
            (ScrollFieldMode::Scroll, ScrollChoiceEvent::SetChoice(id)) => {
                let index = self.catalog.index_of(&id)?;
                self.apply_wheel(ScrollFieldEvent::SetValue(index as i32), reduced_motion)
            }
            (ScrollFieldMode::Scroll, ScrollChoiceEvent::Step(step)) => {
                self.apply_wheel(ScrollFieldEvent::Step(step), reduced_motion)
            }
            (
                ScrollFieldMode::Scroll,
                ScrollChoiceEvent::Drag {
                    session,
                    sample,
                    row_height,
                },
            ) => self.apply_wheel(
                ScrollFieldEvent::Drag {
                    session,
                    sample,
                    row_height,
                },
                reduced_motion,
            ),
            (ScrollFieldMode::Scroll, ScrollChoiceEvent::Cancel) => {
                self.apply_wheel(ScrollFieldEvent::Cancel, reduced_motion)
            }
            _ => None,
        }
    }

    pub fn advance(&mut self, now_ms: Option<f64>, reduced_motion: bool) -> AnimationUpdate {
        self.wheel.advance(now_ms, reduced_motion)
    }

    fn apply_wheel(&mut self, event: ScrollFieldEvent, reduced_motion: bool) -> Option<String> {
        self.wheel.apply(event, reduced_motion)?;
        self.reset_draft();
        Some(self.choice().id.to_owned())
    }

    fn submit(&mut self, reduced_motion: bool) -> Option<String> {
        // A duplicate label is ambiguous only when submitted as typed input.
        // The clean editor already has an unambiguous accepted identity.
        let index = if self.edited {
            match self.catalog.match_query(&self.raw) {
                Ok(matched) => self
                    .catalog
                    .index_of(matched.choice.id)
                    .expect("matched current choice"),
                Err(error) => {
                    self.error = Some(error);
                    return None;
                }
            }
        } else {
            self.index()
        };
        let changed = self.wheel.sync_value(index as i32);
        self.wheel.apply(ScrollFieldEvent::Cancel, reduced_motion);
        self.reset_draft();
        changed.then(|| self.choice().id.to_owned())
    }

    fn reset_draft(&mut self) {
        self.raw = self.choice().label.to_owned();
        self.edited = false;
        self.error = None;
    }

    fn refresh_error(&mut self) {
        if self.error.is_some() {
            self.error = self.catalog.match_query(&self.raw).err();
        }
    }
}

fn ordinal_wheel(catalog: &ScrollChoiceCatalog, committed_id: &str) -> ScrollFieldState {
    let last = catalog.items().len() - 1;
    let index = catalog
        .index_of(committed_id)
        .expect("validated committed ID");
    let range = ScrollFieldRange::new(0, last as i32, 1).expect("validated catalog length");
    ScrollFieldState::new(range, index as i32).expect("validated committed index")
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHOICES: [ScrollChoiceItem<'_>; 4] = [
        item("north", "北部", &["North"]),
        item("middle", "中部", &["Middle"]),
        item("south", "南部", &["South"]),
        item("outlying", "離島", &["Outlying"]),
    ];

    const fn item<'a>(id: &'a str, label: &'a str, aliases: &'a [&'a str]) -> ScrollChoiceItem<'a> {
        ScrollChoiceItem {
            id,
            label,
            detail: None,
            aliases,
        }
    }

    fn state() -> ScrollChoiceState {
        ScrollChoiceState::new(CHOICES, "middle").unwrap()
    }

    fn input(state: &mut ScrollChoiceState, text: &str) {
        assert_eq!(
            state.apply(ScrollChoiceEvent::SetText(text.to_owned()), false),
            None
        );
    }

    fn point(session: &PointDragSession, sample: ScrollFieldDrag) -> ScrollChoiceEvent {
        ScrollChoiceEvent::Drag {
            session: session.clone(),
            sample,
            row_height: 48.,
        }
    }

    #[test]
    fn unique_prefix_suggests_without_changing_raw_text_or_accepted_id() {
        let mut state = state();
        assert_eq!(state.apply(ScrollChoiceEvent::Edit, false), None);
        assert_eq!(state.raw(), "中部");
        assert_eq!(state.mode(), ScrollFieldMode::Input);
        input(&mut state, "南");
        assert_eq!(state.suggestion().unwrap().id, "south");
        assert_eq!(state.raw(), "南");
        assert_eq!(state.choice().id, "middle");
        assert_eq!(
            state.apply(ScrollChoiceEvent::Submit, false).as_deref(),
            Some("south")
        );
        assert_eq!(state.choice().id, "south");
        assert_eq!(state.index(), 2);
        assert_eq!(state.raw(), "南部");
        assert_eq!(state.mode(), ScrollFieldMode::Scroll);
        assert_eq!(state.apply(ScrollChoiceEvent::Submit, false), None);
    }

    #[test]
    fn untouched_duplicate_keeps_its_id_and_typed_duplicate_requires_a_qualifier() {
        let east = ScrollChoiceItem {
            detail: Some("東"),
            ..item("east", "河畔", &["河畔 東"])
        };
        let west = ScrollChoiceItem {
            detail: Some("西"),
            ..item("west", "河畔", &["河畔 西"])
        };
        let mut state = ScrollChoiceState::new([east, west], "west").unwrap();
        state.apply(ScrollChoiceEvent::Edit, false);
        assert_eq!(state.apply(ScrollChoiceEvent::Submit, false), None);
        assert_eq!(state.mode(), ScrollFieldMode::Scroll);
        assert_eq!(state.choice().id, "west");

        state.apply(ScrollChoiceEvent::Edit, false);
        input(&mut state, "河畔");
        assert_eq!(state.apply(ScrollChoiceEvent::Submit, false), None);
        assert_eq!(state.mode(), ScrollFieldMode::Input);
        assert_eq!(state.choice().id, "west");
        assert_eq!(
            state.error(),
            Some(ScrollChoiceQueryError::Ambiguous {
                kind: ScrollChoiceMatchKind::Exact,
                count: 2,
            })
        );
        input(&mut state, "河畔 東");
        assert_eq!(state.suggestion(), None);
        assert_eq!(state.error(), None);
        assert_eq!(
            state.apply(ScrollChoiceEvent::Submit, false).as_deref(),
            Some("east")
        );
        assert_eq!(state.choice().detail, Some("東"));
    }

    #[test]
    fn no_match_and_ambiguous_prefix_keep_the_draft_until_cancel() {
        let mut state = ScrollChoiceState::with_match_key(
            [item("north", "North", &[]), item("norway", "Norway", &[])],
            "north",
            str::to_lowercase,
        )
        .unwrap();
        state.apply(ScrollChoiceEvent::Edit, false);
        for (raw, error) in [
            (" ", ScrollChoiceQueryError::Empty),
            ("x", ScrollChoiceQueryError::NoMatch),
            (
                "no",
                ScrollChoiceQueryError::Ambiguous {
                    kind: ScrollChoiceMatchKind::Prefix,
                    count: 2,
                },
            ),
        ] {
            input(&mut state, raw);
            assert_eq!(state.apply(ScrollChoiceEvent::Submit, false), None);
            assert_eq!(state.error(), Some(error));
            assert_eq!(state.suggestion(), None);
            assert_eq!(state.raw(), raw);
            assert_eq!(state.choice().id, "north");
        }
        assert_eq!(state.apply(ScrollChoiceEvent::Cancel, false), None);
        assert_eq!(state.mode(), ScrollFieldMode::Scroll);
        assert_eq!(state.error(), None);
        assert_eq!(state.raw(), "North");
    }

    #[test]
    fn resize_keeps_a_mounted_editor_until_explicit_submission_or_cancel() {
        let mut state = state();
        assert!(state.update_fit(false));
        assert_eq!(state.mode(), ScrollFieldMode::Input);
        assert!(state.update_fit(true));
        assert_eq!(
            state.mode(),
            ScrollFieldMode::Input,
            "untouched editor stays mounted for preedit"
        );
        input(&mut state, "離");
        assert!(state.update_fit(false));
        assert_eq!(state.raw(), "離");
        assert_eq!(state.suggestion().unwrap().id, "outlying");
        assert_eq!(
            state.apply(ScrollChoiceEvent::Submit, false).as_deref(),
            Some("outlying")
        );
        assert_eq!(
            state.mode(),
            ScrollFieldMode::Input,
            "compact layout still needs input"
        );
        assert_eq!(state.raw(), "離島");
        assert!(state.update_fit(true));
        assert_eq!(state.mode(), ScrollFieldMode::Input);
        assert_eq!(state.apply(ScrollChoiceEvent::Cancel, false), None);
        assert_eq!(state.mode(), ScrollFieldMode::Scroll);
    }

    #[test]
    fn valid_replacement_keeps_the_committed_id_and_retires_old_capture() {
        let mut state = state();
        let old_capture = state.drag_session();
        state.apply(
            point(&old_capture, ScrollFieldDrag::Move { delta_y: 30. }),
            false,
        );
        assert!(state.is_dragging());
        assert_eq!(state.preview_index(), 2);

        let invalid = state.replace_items([CHOICES[0], CHOICES[0]], "north");
        assert!(invalid.is_err());
        assert_eq!(state.count(), 4);
        assert_eq!(state.choice().id, "middle");
        assert_eq!(state.drag_session(), old_capture);
        assert!(state.is_dragging());

        state
            .replace_items(
                [CHOICES[3], item("middle", "中央", &[]), CHOICES[0]],
                "middle",
            )
            .unwrap();
        assert_eq!(state.choice().id, "middle");
        assert_eq!(state.choice().label, "中央");
        assert_eq!(state.get(0).unwrap().id, "outlying");
        assert_eq!(state.raw(), "中央");
        assert!(!state.is_dragging());
        let stale = point(&old_capture, ScrollFieldDrag::End { delta_y: 90. });
        assert_eq!(state.apply(stale, true), None);
        assert_eq!(state.choice().id, "middle");
        assert_eq!(
            state
                .apply(ScrollChoiceEvent::Step(ScrollFieldStep::Increase), true)
                .as_deref(),
            Some("north")
        );
    }

    #[test]
    fn replacement_rematches_a_typed_draft_against_its_new_catalog() {
        let mut state = state();
        state.apply(ScrollChoiceEvent::Edit, false);
        input(&mut state, "東");
        assert_eq!(state.apply(ScrollChoiceEvent::Submit, false), None);
        assert_eq!(state.error(), Some(ScrollChoiceQueryError::NoMatch));
        let east = item("east", "東部", &[]);
        state
            .replace_items([east, CHOICES[1], CHOICES[0]], "middle")
            .unwrap();
        assert_eq!(state.mode(), ScrollFieldMode::Input);
        assert_eq!(state.error(), None);
        assert_eq!(state.raw(), "東");
        assert_eq!(state.suggestion().unwrap().id, "east");
        assert!(state.sync_choice("north"));
        assert!(!state.sync_choice("north"));
        assert!(!state.sync_choice("unknown"));
        assert_eq!(state.raw(), "東");
        assert_eq!(state.choice().id, "north");
        assert_eq!(
            state.apply(ScrollChoiceEvent::Submit, false).as_deref(),
            Some("east")
        );
    }

    #[test]
    fn mounted_editor_keeps_its_text_when_catalog_and_accepted_choice_change() {
        let mut state = state();
        state.apply(ScrollChoiceEvent::Edit, false);
        let middle = item("middle", "中央", &[]);
        state
            .replace_items([CHOICES[0], middle, CHOICES[2]], "middle")
            .unwrap();
        assert_eq!(state.mode(), ScrollFieldMode::Input);
        assert_eq!(state.raw(), "中部");
        assert_eq!(state.choice().id, "middle");
        assert_eq!(state.choice().label, "中央");
        assert_eq!(state.error(), None);

        assert!(state.sync_choice("north"));
        assert_eq!(state.mode(), ScrollFieldMode::Input);
        assert_eq!(state.raw(), "中部");
        assert_eq!(state.choice().id, "north");
        assert_eq!(state.apply(ScrollChoiceEvent::Submit, false), None);
        assert_eq!(state.mode(), ScrollFieldMode::Scroll);
        assert_eq!(state.choice().id, "north");
        assert_eq!(state.raw(), "北部");

        let north = item("north", "北區", &[]);
        state
            .replace_items([north, middle, CHOICES[2]], "north")
            .unwrap();
        assert_eq!(
            state.raw(),
            "北區",
            "an unmounted editor tracks the current label"
        );
    }

    #[test]
    fn ordinal_wheel_previews_releases_and_discards_stale_terminal_events() {
        let mut state = state();
        let session = state.drag_session();
        assert_eq!(
            state.apply(
                point(&session, ScrollFieldDrag::Move { delta_y: 35. }),
                false
            ),
            None
        );
        assert_eq!(state.choice().id, "middle");
        assert_eq!(state.preview_index(), 2);
        assert!((state.offset() - (1. + 35. / 48.)).abs() < 0.001);
        let release = point(&session, ScrollFieldDrag::End { delta_y: 89. });
        assert_eq!(
            state.apply(release.clone(), false).as_deref(),
            Some("outlying")
        );
        assert_eq!(state.apply(release, false), None);
        assert!(state.settling());
        let settled = state.advance(None, false);
        assert!(settled.rebuild && !settled.request_next);
        assert_eq!(state.preview_index(), 3);

        let reverse = point(
            &state.drag_session(),
            ScrollFieldDrag::End { delta_y: -50. },
        );
        assert_eq!(state.apply(reverse, true).as_deref(), Some("south"));
        assert_eq!(state.index(), 2);
        assert_eq!(state.offset(), 2.);
        assert!(!state.settling());
        let attempted = point(
            &state.drag_session(),
            ScrollFieldDrag::Move { delta_y: -48. },
        );
        assert_eq!(state.apply(attempted, false), None);
        assert_eq!(state.apply(ScrollChoiceEvent::Cancel, true), None);
        assert_eq!(state.choice().id, "south");
        assert_eq!(state.preview_index(), 2);
    }
}
