use surface_app::{AnimationFrame, AnimationUpdate, View};
#[cfg(target_arch = "wasm32")]
use surface_material::ScrollFieldMode;
use surface_material::{
    scroll_field_pair_layout, shape::LayoutDirection, ButtonKind, ChoiceState, ScrollChoiceEvent,
    ScrollChoiceFieldOptions, ScrollChoiceItem, ScrollChoiceQueryError, ScrollChoiceState,
    ScrollFieldError, ScrollFieldEvent, ScrollFieldOptions, ScrollFieldPairOptions,
    ScrollFieldRange, ScrollFieldSize, ScrollFieldState, Theme,
};

use crate::{
    scroll_field_demo::{control_row, ScrollFieldScene, CONTENT_PADDING, PADDING},
    Message,
};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "scroll-pair-demo-show";
const BACK: &str = "scroll-pair-demo-back";
const SCROLL: &str = "scroll-pair-demo-scroll";
const QUANTITY: &str = "scroll-pair-demo-quantity";
const UNIT: &str = "scroll-pair-demo-unit";
const SCENE: ScrollFieldScene = ScrollFieldScene {
    page: "scroll-pair-demo",
    scroll: SCROLL,
    show: SHOW,
    saved: "scroll-pair-demo-saved",
    title: "數量與單位",
    scroller_label: "容量範例",
};

const UNITS: [ScrollChoiceItem<'_>; 3] = [
    ScrollChoiceItem {
        id: "ml",
        label: "毫升",
        detail: None,
        aliases: &["mL"],
    },
    ScrollChoiceItem {
        id: "liter",
        label: "公升",
        detail: None,
        aliases: &["L"],
    },
    ScrollChoiceItem {
        id: "fluid-ounce",
        label: "液量盎司",
        detail: None,
        aliases: &["fl oz"],
    },
];

fn quantity_error(error: ScrollFieldError) -> String {
    match error {
        ScrollFieldError::Empty => "請輸入數量",
        ScrollFieldError::InvalidNumber => "請輸入整數",
        ScrollFieldError::OutOfRange => "數量超出範圍",
        ScrollFieldError::OffStep => "請輸入偶數",
    }
    .to_owned()
}

fn unit_error(error: ScrollChoiceQueryError) -> String {
    match error {
        ScrollChoiceQueryError::Empty => "請輸入單位",
        ScrollChoiceQueryError::NoMatch => "找不到單位",
        ScrollChoiceQueryError::Ambiguous { .. } => "請輸入完整單位名稱",
    }
    .to_owned()
}

#[derive(Clone)]
pub enum ScrollPairDemoEvent {
    Show,
    Back,
    Direction,
    LargeText,
    DisableQuantity,
    DisableUnit,
    Quantity(ScrollFieldEvent),
    Unit(ScrollChoiceEvent),
}

pub(crate) struct ScrollPairDemo {
    visible: bool,
    viewport: (f32, f32),
    rtl: bool,
    large_text: bool,
    quantity_disabled: bool,
    unit_disabled: bool,
    quantity: ScrollFieldState,
    unit: ScrollChoiceState,
    quantity_commits: usize,
    unit_commits: usize,
    reduced_motion: bool,
}

impl Default for ScrollPairDemo {
    fn default() -> Self {
        Self {
            visible: false,
            viewport: (960., 820.),
            rtl: false,
            large_text: false,
            quantity_disabled: false,
            unit_disabled: false,
            quantity: ScrollFieldState::new(
                ScrollFieldRange::new(-1800, 1800, 2).expect("valid Gallery range"),
                1200,
            )
            .expect("valid Gallery quantity"),
            unit: ScrollChoiceState::with_match_key(UNITS, "ml", str::to_lowercase)
                .expect("valid Gallery unit catalog"),
            quantity_commits: 0,
            unit_commits: 0,
            reduced_motion: false,
        }
    }
}

impl ScrollPairDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.quantity.invalidate_layout();
        self.unit.invalidate_layout();
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    fn options(&self) -> ScrollFieldPairOptions<'_> {
        let available_width = (self.viewport.0 - 2. * (PADDING + 2. * CONTENT_PADDING)).max(0.);
        let available_height = (self.viewport.1 - 2. * PADDING).max(0.);
        let text_scale = self.text_scale();
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        ScrollFieldPairOptions {
            id: "scroll-pair-demo-pair",
            label: "容量",
            available_width,
            available_height,
            direction,
            quantity: ScrollFieldOptions {
                id: QUANTITY,
                label: "數量",
                size: ScrollFieldSize::Large,
                height: 200.,
                available_width,
                available_height,
                text_scale,
                direction,
                disabled: self.quantity_disabled,
                hint: Some("每次增加或減少 2"),
                error_text: &quantity_error,
                value_text: &|value| value.to_string(),
                use_input: "輸入數量",
                use_scroll: "捲動數量",
            },
            unit: ScrollChoiceFieldOptions {
                id: UNIT,
                label: "單位",
                size: ScrollFieldSize::Medium,
                height: 200.,
                available_width,
                available_height,
                text_scale,
                direction,
                disabled: self.unit_disabled,
                hint: Some("可輸入單位名稱"),
                use_input: "輸入單位",
                use_scroll: "捲動單位",
                error_text: &unit_error,
                choice_text: &|label, _| label.to_owned(),
                suggestion_text: &|label| format!("建議單位 {label}"),
            },
        }
    }

    fn update_fit(&mut self) {
        self.quantity.invalidate_layout();
        self.unit.invalidate_layout();
        let layout = scroll_field_pair_layout(self.options(), &self.quantity, &self.unit);
        self.quantity.update_fit(layout.quantity_fits);
        self.unit.update_fit(layout.unit_fits);
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        if self.viewport == (width, height) {
            return false;
        }
        self.viewport = (width, height);
        if self.visible {
            self.update_fit();
        }
        self.visible
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if !self.visible {
            return AnimationUpdate::default();
        }
        self.reduced_motion = frame.reduced_motion;
        let quantity = self.quantity.advance(frame.now_ms, frame.reduced_motion);
        let unit = self.unit.advance(frame.now_ms, frame.reduced_motion);
        AnimationUpdate {
            rebuild: quantity.rebuild || unit.rebuild,
            request_next: quantity.request_next || unit.request_next,
        }
    }

    pub(crate) fn update(&mut self, event: ScrollPairDemoEvent) {
        use ScrollPairDemoEvent::*;
        match event {
            Show => {
                self.visible = true;
                self.update_fit();
            }
            Back => self.leave(),
            Direction => {
                self.rtl = !self.rtl;
                self.update_fit();
            }
            LargeText => {
                self.large_text = !self.large_text;
                self.update_fit();
            }
            DisableQuantity => {
                self.quantity_disabled = !self.quantity_disabled;
                self.quantity.invalidate_layout();
            }
            DisableUnit => {
                self.unit_disabled = !self.unit_disabled;
                self.unit.invalidate_layout();
            }
            Quantity(event) if !self.quantity_disabled => {
                if self.quantity.apply(event, self.reduced_motion).is_some() {
                    self.quantity_commits += 1;
                }
            }
            Unit(event) if !self.unit_disabled => {
                if self.unit.apply(event, self.reduced_motion).is_some() {
                    self.unit_commits += 1;
                }
            }
            Quantity(_) | Unit(_) => {}
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                SCENE.title,
                ButtonKind::Outlined,
                false,
                Message::ScrollPair(ScrollPairDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use ScrollPairDemoEvent::*;
        let selected = |selected| ChoiceState {
            selected,
            disabled: false,
        };
        let toggle = |id, label, on, event| {
            theme
                .segmented_button(id, label, selected(on), Message::ScrollPair(event))
                .accessible_activation()
        };
        let profile = control_row(vec![
            theme
                .tonal_button(BACK, "返回", Message::ScrollPair(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme
                .switch(
                    "scroll-pair-demo-theme",
                    "深色",
                    selected(dark),
                    Message::Theme,
                )
                .accessible_activation(),
            toggle("scroll-pair-demo-rtl", "右起", self.rtl, Direction),
            toggle(
                "scroll-pair-demo-large-text",
                "大字",
                self.large_text,
                LargeText,
            ),
        ]);
        let settings = control_row(vec![
            toggle(
                "scroll-pair-demo-quantity-disabled",
                "停用數量",
                self.quantity_disabled,
                DisableQuantity,
            ),
            toggle(
                "scroll-pair-demo-unit-disabled",
                "停用單位",
                self.unit_disabled,
                DisableUnit,
            ),
        ]);
        let field = theme.scroll_field_pair(
            self.options(),
            &self.quantity,
            &self.unit,
            |event| Message::ScrollPair(Quantity(event)),
            |event| Message::ScrollPair(Unit(event)),
        );
        SCENE.view(
            theme,
            self.rtl,
            [profile, settings],
            field,
            format!(
                "已選 {} {}",
                self.quantity.value(),
                self.unit.choice().label
            ),
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        runtime: &surface_app::Runtime<crate::Gallery>,
    ) -> serde_json::Value {
        let layout = scroll_field_pair_layout(self.options(), &self.quantity, &self.unit);
        let mode = |mode| {
            if mode == ScrollFieldMode::Input {
                "input"
            } else {
                "scroll"
            }
        };
        serde_json::json!({
            "visible": self.visible, "rtl": self.rtl, "text_scale": self.text_scale(),
            "stacked": layout.stacked, "quantity_fits": layout.quantity_fits, "unit_fits": layout.unit_fits,
            "scroll_top": runtime.scroll_metrics(SCROLL).map(|metrics| metrics.offset.1),
            "quantity": {
                "value": self.quantity.value(), "raw": self.quantity.raw(), "mode": mode(self.quantity.mode()),
                "error": self.quantity.error().map(|error| format!("{error:?}")),
                "commits": self.quantity_commits, "disabled": self.quantity_disabled,
            },
            "unit": {
                "id": self.unit.choice().id, "ordinal": self.unit.index() + 1,
                "raw": self.unit.raw(), "mode": mode(self.unit.mode()),
                "error": self.unit.error().map(|error| format!("{error:?}")),
                "suggestion": self.unit.suggestion().map(|item| item.label),
                "commits": self.unit_commits, "disabled": self.unit_disabled,
            },
        })
    }
}
