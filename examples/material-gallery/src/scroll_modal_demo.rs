use surface::{Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, Queue, View};
use surface_material::{
    scroll_field_modal_layout, scroll_field_pair_layout, shape::LayoutDirection, typography,
    ButtonKind, ChoiceState, ScrollChoiceEvent, ScrollChoiceState, ScrollFieldEvent,
    ScrollFieldModalLayout, ScrollFieldModalOptions, ScrollFieldMode, ScrollFieldPairOptions,
    ScrollFieldSize, ScrollFieldState, Theme,
};

use crate::{
    scroll_field_demo::{control_row, PADDING},
    scroll_pair_demo::{measurement_options, measurement_states},
    Message,
};

pub(crate) const SECTION: usize = 0;
const PAGE: &str = "scroll-modal-demo";
const SHOW: &str = "scroll-modal-demo-show";
const BACK: &str = "scroll-modal-demo-back";
const OPENER: &str = "scroll-modal-demo-opener";
const OUTSIDE: &str = "scroll-modal-demo-outside";
const PAIR: &str = "scroll-modal-demo-pair";
const QUANTITY: &str = "scroll-modal-demo-quantity";
const UNIT: &str = "scroll-modal-demo-unit";
const QUANTITY_WHEEL: &str = "scroll-modal-demo-quantity-adjustable";
const QUANTITY_EDITOR: &str = "scroll-modal-demo-quantity-editor";
#[cfg(any(test, target_arch = "wasm32"))]
const UNIT_EDITOR: &str = "scroll-modal-demo-unit-editor";
const BODY: &str = "scroll-modal-demo-body";
const STATUS: &str = "scroll-modal-demo-status";

#[derive(Clone)]
pub enum ScrollModalDemoEvent {
    Show,
    Back,
    Open,
    Cancel,
    Save,
    Outside,
    Direction,
    LargeText,
    Quantity(ScrollFieldEvent),
    Unit(ScrollChoiceEvent),
}

#[derive(Debug, PartialEq, Eq)]
struct Measurement {
    quantity: i32,
    unit_id: String,
    unit_label: String,
}

impl Measurement {
    fn saved_label(&self) -> String {
        format!("已儲存 {} {}", self.quantity, self.unit_label)
    }
}

struct WorkingPair {
    quantity: ScrollFieldState,
    unit: ScrollChoiceState,
    show_save_errors: bool,
}

impl WorkingPair {
    fn new(accepted: &Measurement) -> Self {
        let (mut quantity, mut unit) = measurement_states();
        quantity.sync_value(accepted.quantity);
        unit.sync_choice(&accepted.unit_id);
        Self {
            quantity,
            unit,
            show_save_errors: false,
        }
    }

    fn measurement(&self) -> Measurement {
        let unit = self.unit.choice();
        Measurement {
            quantity: self.quantity.value(),
            unit_id: unit.id.to_owned(),
            unit_label: unit.label.to_owned(),
        }
    }

    fn validate_save(&mut self) -> Option<Measurement> {
        // Run the existing validators in a fresh fixture before changing any
        // mounted editor. A failed Save must leave a valid peer's draft as-is.
        let mut candidate = Self::new(&self.measurement());
        if self.quantity.mode() == ScrollFieldMode::Input {
            candidate.quantity.apply(ScrollFieldEvent::Edit, true);
            candidate.quantity.apply(
                ScrollFieldEvent::SetText(self.quantity.raw().to_owned()),
                true,
            );
            candidate.quantity.apply(ScrollFieldEvent::Submit, true);
        }
        if self.unit.mode() == ScrollFieldMode::Input {
            candidate.unit.apply(ScrollChoiceEvent::Edit, true);
            // This fixture has unique labels. Equal text represents the same
            // choice whether the editor is still pristine or was edited.
            if self.unit.raw() != self.unit.choice().label {
                candidate
                    .unit
                    .apply(ScrollChoiceEvent::SetText(self.unit.raw().to_owned()), true);
            }
            candidate.unit.apply(ScrollChoiceEvent::Submit, true);
        }
        let invalid_quantity = candidate.quantity.error().is_some();
        let invalid_unit = candidate.unit.error().is_some();
        if !invalid_quantity && !invalid_unit {
            return Some(candidate.measurement());
        }

        // These submissions cannot succeed. Publish the original validators'
        // field errors and leave their original raw values and modes mounted.
        if invalid_quantity {
            self.quantity.apply(ScrollFieldEvent::Submit, true);
        }
        if invalid_unit {
            self.unit.apply(ScrollChoiceEvent::Submit, true);
        }
        self.show_save_errors = true;
        None
    }

    fn save_errors(&self) -> Option<&'static str> {
        if !self.show_save_errors {
            return None;
        }
        match (self.quantity.error().is_some(), self.unit.error().is_some()) {
            (true, true) => Some("請修正數量和單位"),
            (true, false) => Some("請修正數量"),
            (false, true) => Some("請修正單位"),
            (false, false) => None,
        }
    }

    #[cfg(any(test, target_arch = "wasm32"))]
    fn first_invalid_editor(&self) -> Option<&'static str> {
        if !self.show_save_errors {
            return None;
        }
        self.quantity
            .error()
            .map(|_| QUANTITY_EDITOR)
            .or_else(|| self.unit.error().map(|_| UNIT_EDITOR))
    }
}

pub(crate) struct ScrollModalDemo {
    visible: bool,
    viewport: Rect,
    rtl: bool,
    large_text: bool,
    accepted: Measurement,
    working: Option<WorkingPair>,
    saves: usize,
    outside_presses: usize,
    reduced_motion: bool,
}

impl Default for ScrollModalDemo {
    fn default() -> Self {
        let (quantity, unit) = measurement_states();
        let pair = WorkingPair {
            quantity,
            unit,
            show_save_errors: false,
        };
        Self {
            visible: false,
            viewport: Rect::new(0., 0., 960., 820.),
            rtl: false,
            large_text: false,
            accepted: pair.measurement(),
            working: None,
            saves: 0,
            outside_presses: 0,
            reduced_motion: false,
        }
    }
}

impl ScrollModalDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn options(&self, initial_focus: Option<&'static str>) -> ScrollFieldModalOptions<'static> {
        ScrollFieldModalOptions {
            id: PAGE,
            label: "編輯容量",
            body_label: "容量欄位",
            cancel_label: "取消",
            save_label: "儲存",
            initial_focus,
            restore_focus_to: Some(OPENER),
            viewport: self.viewport,
            direction: self.direction(),
            preferred_size: (696., 440.),
            minimum_body_height: ScrollFieldSize::Large.compact_editor_height(self.text_scale()),
            text_scale: self.text_scale(),
        }
    }

    fn pair_options(&self, layout: &ScrollFieldModalLayout) -> ScrollFieldPairOptions<'static> {
        measurement_options(
            [PAIR, QUANTITY, UNIT],
            (layout.body_width, layout.body_height),
            self.text_scale(),
            self.direction(),
            (false, false),
        )
    }

    fn reconcile_layout(&mut self) {
        let Some(layout) = scroll_field_modal_layout(self.options(None)) else {
            // The view hides the dialog until its full targets fit again.
            // Keep the draft through this resize.
            return;
        };
        let options = self.pair_options(&layout);
        if let Some(working) = &mut self.working {
            let plan = scroll_field_pair_layout(options, &working.quantity, &working.unit);
            working.quantity.invalidate_layout();
            working.unit.invalidate_layout();
            working.quantity.update_fit(plan.quantity_fits);
            working.unit.update_fit(plan.unit_fits);
        }
    }

    pub(crate) fn leave(&mut self) {
        self.working = None;
        self.visible = false;
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let viewport = Rect::new(0., 0., width, height);
        if self.viewport == viewport {
            return false;
        }
        self.viewport = viewport;
        if self.working.is_some() {
            self.reconcile_layout();
        }
        self.visible
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        self.reduced_motion = frame.reduced_motion;
        let Some(working) = &mut self.working else {
            return AnimationUpdate::default();
        };
        let quantity = working.quantity.advance(frame.now_ms, frame.reduced_motion);
        let unit = working.unit.advance(frame.now_ms, frame.reduced_motion);
        AnimationUpdate {
            rebuild: quantity.rebuild || unit.rebuild,
            request_next: quantity.request_next || unit.request_next,
        }
    }

    fn open(&self) -> bool {
        self.working.is_some() && scroll_field_modal_layout(self.options(None)).is_some()
    }

    fn save(&mut self) -> bool {
        if !self.open() {
            return false;
        }
        let Some(working) = &mut self.working else {
            return false;
        };
        let Some(accepted) = working.validate_save() else {
            return true;
        };
        self.accepted = accepted;
        self.saves += 1;
        self.working = None;
        false
    }

    /// Return whether a rejected Save needs its validation status revealed.
    pub(crate) fn update(&mut self, event: ScrollModalDemoEvent) -> bool {
        use ScrollModalDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            Open if self.visible
                && self.working.is_none()
                && scroll_field_modal_layout(self.options(None)).is_some() =>
            {
                self.working = Some(WorkingPair::new(&self.accepted));
                self.reconcile_layout();
            }
            Open => {}
            Cancel => self.working = None,
            Save => return self.save(),
            Outside if !self.open() => self.outside_presses += 1,
            Outside => {}
            Direction => {
                self.rtl = !self.rtl;
                self.reconcile_layout();
            }
            LargeText => {
                self.large_text = !self.large_text;
                self.reconcile_layout();
            }
            Quantity(event) => {
                if let Some(working) = &mut self.working {
                    working.quantity.apply(event, self.reduced_motion);
                }
            }
            Unit(event) => {
                if let Some(working) = &mut self.working {
                    working.unit.apply(event, self.reduced_motion);
                }
            }
        }
        false
    }

    pub(crate) fn reveal_validation(queue: &Queue<Message>) {
        queue.scroll_to_child_top(BODY, STATUS, 5.);
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "對話框捲動欄位",
                ButtonKind::Outlined,
                false,
                Message::ScrollModal(ScrollModalDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn modal(&self, theme: &Theme, layout: &ScrollFieldModalLayout) -> Option<View<Message>> {
        use ScrollModalDemoEvent::*;
        let working = self.working.as_ref()?;
        let options = self.pair_options(layout);
        let plan = scroll_field_pair_layout(options, &working.quantity, &working.unit);
        let focus = if plan.quantity_fits && working.quantity.mode() == ScrollFieldMode::Scroll {
            QUANTITY_WHEEL
        } else {
            QUANTITY_EDITOR
        };
        let pair = theme.scroll_field_pair(
            options,
            &working.quantity,
            &working.unit,
            |event| Message::ScrollModal(Quantity(event)),
            |event| Message::ScrollModal(Unit(event)),
        );
        let status = View::new(
            Node::new(Role::Status, "")
                .id(STATUS)
                .text(working.save_errors().unwrap_or_default())
                .shrink_to_fit_width()
                .styled(
                    Style::column()
                        .with_text_style(typography::BODY_MEDIUM)
                        .foreground(theme.colors.error),
                ),
        );
        let body = View::new(Node::new(Role::Group, "").styled(Style::column().gap(8.)))
            .children([status, pair]);
        theme.scroll_field_modal(
            self.options(Some(focus)),
            body,
            Message::ScrollModal(Cancel),
            Message::ScrollModal(Save),
        )
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use ScrollModalDemoEvent::*;
        let choice = |selected| ChoiceState {
            selected,
            disabled: false,
        };
        let toggle = |id, label, on, event| {
            theme
                .segmented_button(id, label, choice(on), Message::ScrollModal(event))
                .accessible_activation()
        };
        let profile = control_row(vec![
            theme
                .tonal_button(BACK, "返回", Message::ScrollModal(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme
                .switch(
                    "scroll-modal-demo-theme",
                    "深色",
                    choice(dark),
                    Message::Theme,
                )
                .accessible_activation(),
            toggle("scroll-modal-demo-rtl", "右起", self.rtl, Direction),
            toggle(
                "scroll-modal-demo-large-text",
                "大字",
                self.large_text,
                LargeText,
            ),
        ]);
        let saved = View::new(
            Node::new(Role::Text, "")
                .id("scroll-modal-demo-saved")
                .text(self.accepted.saved_label())
                .styled(
                    Style::column()
                        .with_text_style(typography::BODY_MEDIUM)
                        .foreground(theme.colors.on_surface_variant),
                ),
        );
        let layout = scroll_field_modal_layout(self.options(None));
        let opener = theme
            .button_with(
                OPENER,
                "開啟欄位",
                ButtonKind::Tonal,
                layout.is_none(),
                Message::ScrollModal(Open),
            )
            .disclosure_expanded(layout.is_some() && self.working.is_some())
            .accessible_activation();
        let outside = theme
            .button_with(
                OUTSIDE,
                &format!("外部 {} 次", self.outside_presses),
                ButtonKind::Outlined,
                false,
                Message::ScrollModal(Outside),
            )
            .accessible_activation();
        let controls = control_row(vec![opener, outside]);
        View::new(
            Node::new(Role::Window, "對話框捲動欄位").id(PAGE).styled(
                Style::column()
                    .height(Length::Percent(1.))
                    .padding(PADDING)
                    .gap(8.)
                    .background(theme.colors.surface),
            ),
        )
        .children(
            [profile, saved, controls]
                .into_iter()
                .chain(layout.as_ref().and_then(|layout| self.modal(theme, layout))),
        )
        .restore_focus_in_subtree_on_removal(SHOW)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        runtime: &surface_app::Runtime<crate::Gallery>,
    ) -> serde_json::Value {
        let mode = |mode| {
            if mode == ScrollFieldMode::Input {
                "input"
            } else {
                "scroll"
            }
        };
        serde_json::json!({
            "visible": self.visible,
            "open": self.open(),
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "accepted": {"quantity": self.accepted.quantity, "unit_id": self.accepted.unit_id},
            "working": self.working.as_ref().map(|w| serde_json::json!({
                "quantity": {"value": w.quantity.value(), "raw": w.quantity.raw(), "mode": mode(w.quantity.mode()), "error": w.quantity.error().map(|e| format!("{e:?}"))},
                "unit": {"id": w.unit.choice().id, "raw": w.unit.raw(), "mode": mode(w.unit.mode()), "error": w.unit.error().map(|e| format!("{e:?}"))},
                "first_invalid_editor": w.first_invalid_editor(),
                "status": w.save_errors(),
            })),
            "saves": self.saves,
            "outside_presses": self.outside_presses,
            "body_scroll_top": runtime.scroll_metrics(BODY).map(|m| m.offset.1),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ScrollModalDemo {
        let mut demo = ScrollModalDemo::default();
        demo.update(ScrollModalDemoEvent::Show);
        demo.update(ScrollModalDemoEvent::Open);
        assert!(demo.working.is_some());
        demo
    }

    fn type_pair(demo: &mut ScrollModalDemo, quantity: &str, unit: &str) {
        use ScrollModalDemoEvent::*;
        demo.update(Quantity(ScrollFieldEvent::Edit));
        demo.update(Quantity(ScrollFieldEvent::SetText(quantity.to_owned())));
        demo.update(Unit(ScrollChoiceEvent::Edit));
        demo.update(Unit(ScrollChoiceEvent::SetText(unit.to_owned())));
    }

    #[test]
    fn child_submits_update_only_working_values_and_cancel_discards_both() {
        use ScrollModalDemoEvent::*;
        let mut demo = fixture();
        type_pair(&mut demo, "1204", "L");
        demo.update(Quantity(ScrollFieldEvent::Submit));
        demo.update(Unit(ScrollChoiceEvent::Submit));
        let working = demo.working.as_ref().unwrap();
        assert_eq!(working.quantity.value(), 1204);
        assert_eq!(working.unit.choice().id, "liter");
        assert_eq!(demo.accepted.quantity, 1200);
        assert_eq!(demo.accepted.unit_id, "ml");
        assert_eq!(demo.saves, 0);

        demo.update(Cancel);
        demo.update(Open);
        assert_eq!(demo.working.as_ref().unwrap().measurement(), demo.accepted);
    }

    #[test]
    fn invalid_save_reports_both_then_preserves_the_valid_peers_edit() {
        use ScrollModalDemoEvent::*;
        let mut demo = fixture();
        type_pair(&mut demo, "1203", "無");
        assert!(demo.update(Save));
        let working = demo.working.as_ref().unwrap();
        assert!(working.quantity.error().is_some());
        assert!(working.unit.error().is_some());
        assert_eq!(working.first_invalid_editor(), Some(QUANTITY_EDITOR));
        assert_eq!(working.save_errors(), Some("請修正數量和單位"));

        demo.update(Quantity(ScrollFieldEvent::SetText("１２０６".to_owned())));
        assert!(demo.update(Save));
        let working = demo.working.as_ref().unwrap();
        assert_eq!(working.first_invalid_editor(), Some(UNIT_EDITOR));
        assert_eq!(working.quantity.mode(), ScrollFieldMode::Input);
        assert_eq!(working.quantity.raw(), "１２０６");
        assert_eq!(working.quantity.value(), 1200);
        assert_eq!(demo.accepted.quantity, 1200);
        assert_eq!(demo.accepted.unit_id, "ml");
        assert_eq!(demo.saves, 0);

        demo.update(Unit(ScrollChoiceEvent::SetText("fl oz".to_owned())));
        assert!(!demo.update(Save));
        assert!(demo.working.is_none());
        assert_eq!(demo.accepted.quantity, 1206);
        assert_eq!(demo.accepted.unit_id, "fluid-ounce");
        assert_eq!(demo.accepted.saved_label(), "已儲存 1206 液量盎司");
        assert_eq!(demo.saves, 1);
        demo.update(Open);
        assert_eq!(demo.working.as_ref().unwrap().measurement(), demo.accepted);
    }

    #[test]
    fn a_short_view_hides_the_modal_and_preserves_the_pair_until_it_fits() {
        use ScrollModalDemoEvent::*;
        let mut demo = fixture();
        demo.update(LargeText);
        type_pair(&mut demo, "1204", "L");
        assert!(demo.open());
        assert!(demo.viewport_changed(320., 217.));
        assert!(!demo.open());
        assert!(!demo.update(Save));
        assert_eq!(demo.accepted.quantity, 1200);
        assert_eq!(demo.saves, 0);

        assert!(demo.viewport_changed(320., 218.));
        assert!(demo.open());
        let working = demo.working.as_ref().unwrap();
        assert_eq!(working.quantity.raw(), "1204");
        assert_eq!(working.unit.raw(), "L");
        assert!(!demo.update(Save));
        assert_eq!(demo.accepted.quantity, 1204);
        assert_eq!(demo.accepted.unit_id, "liter");
        assert_eq!(demo.saves, 1);
    }

    #[test]
    fn closed_messages_cannot_save_and_outside_counter_only_moves_when_closed() {
        use ScrollModalDemoEvent::*;
        let mut demo = fixture();
        demo.update(Outside);
        assert_eq!(demo.outside_presses, 0);
        demo.update(Cancel);
        demo.update(Save);
        demo.update(Quantity(ScrollFieldEvent::SetValue(1800)));
        assert_eq!(demo.saves, 0);
        assert_eq!(demo.accepted.quantity, 1200);
        demo.update(Outside);
        assert_eq!(demo.outside_presses, 1);
    }
}
