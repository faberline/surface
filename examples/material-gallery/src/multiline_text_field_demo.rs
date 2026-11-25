use std::num::NonZeroU16;

use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChoiceState, MultilineTextFieldSize,
    TextFieldAdornments, TextFieldAffix, TextFieldAppearance, TextFieldCounterOptions,
    TextFieldEditability, TextFieldOptions, TextFieldTrailingAction, Theme,
};

use crate::{scroll_field_demo::control_row, text_field_counter_demo::clear_icon, Message};

pub(crate) const SECTION: usize = 0;
const TITLE: &str = "多行文字欄位";
const SHOW: &str = "multiline-text-field-demo-show";
const BACK: &str = "multiline-text-field-demo-back";
const APPEARANCE: &str = "multiline-text-field-demo-appearance";
const THEME: &str = "multiline-text-field-demo-theme";
const RTL: &str = "multiline-text-field-demo-rtl";
const LARGE_TEXT: &str = "multiline-text-field-demo-large-text";
const SCROLL: &str = "multiline-text-field-demo-scroll";
const GROW: &str = "multiline-text-field-demo-grow";
const FIXED: &str = "multiline-text-field-demo-fixed";
const SUBMIT: &str = "multiline-text-field-demo-submit";
const READ_ONLY: &str = "multiline-text-field-demo-read-only";
const DISABLED: &str = "multiline-text-field-demo-disabled";
const FIXED_ROWS: NonZeroU16 = NonZeroU16::new(3).unwrap();
const READ_ONLY_ROWS: NonZeroU16 = NonZeroU16::new(2).unwrap();
const GROW_LIMIT: usize = 12;
const GROW_INITIAL: &str = "第一行";
const FIXED_INITIAL: &str =
    "第一行😀\n第二行 a\u{301}\n第三行 ABC 中文文字\n第四行\n第五行\n第六行\n第七行\n第八行\n";

#[derive(Clone)]
pub enum MultilineTextFieldDemoEvent {
    Show,
    Back,
    ToggleAppearance,
    ToggleDirection,
    ToggleLargeText,
    EditGrow(String),
    EditFixed(String),
    EditLocked,
    ClearGrow,
    Submit,
}

#[derive(Clone, Copy)]
enum Sample {
    Grow,
    Fixed,
    ReadOnly,
    Disabled,
}

struct Value {
    value: String,
    edits: usize,
}

impl Value {
    fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
            edits: 0,
        }
    }

    fn edit(&mut self, value: String) {
        self.value = value;
        self.edits += 1;
    }
}

pub(crate) struct MultilineTextFieldDemo {
    visible: bool,
    width: f32,
    appearance: TextFieldAppearance,
    rtl: bool,
    large_text: bool,
    grow: Value,
    fixed: Value,
    actions: usize,
    submits: usize,
    last_submit: Option<(String, String)>,
    locked_edit_requests: usize,
}

impl Default for MultilineTextFieldDemo {
    fn default() -> Self {
        Self {
            visible: false,
            width: 960.,
            appearance: TextFieldAppearance::Outlined,
            rtl: false,
            large_text: false,
            grow: Value::new(GROW_INITIAL),
            fixed: Value::new(FIXED_INITIAL),
            actions: 0,
            submits: 0,
            last_submit: None,
            locked_edit_requests: 0,
        }
    }
}

impl MultilineTextFieldDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        if self.width == width {
            return false;
        }
        self.width = width;
        self.visible
    }

    pub(crate) fn update(&mut self, event: MultilineTextFieldDemoEvent) {
        use MultilineTextFieldDemoEvent::*;
        if !self.visible && !matches!(event, Show) {
            return;
        }
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleAppearance => {
                self.appearance = match self.appearance {
                    TextFieldAppearance::Outlined => TextFieldAppearance::Filled,
                    TextFieldAppearance::Filled => TextFieldAppearance::Outlined,
                };
            }
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            EditGrow(value) => self.grow.edit(value),
            EditFixed(value) => self.fixed.edit(value),
            EditLocked => self.locked_edit_requests += 1,
            ClearGrow => {
                self.grow.value.clear();
                self.actions += 1;
            }
            Submit => {
                self.submits += 1;
                self.last_submit = Some((self.grow.value.clone(), self.fixed.value.clone()));
            }
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                TITLE,
                ButtonKind::Outlined,
                false,
                Message::MultilineTextFieldDemo(MultilineTextFieldDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
            .suppress_repeated_activation()
    }

    fn field(&self, theme: &Theme, sample: Sample) -> View<Message> {
        use MultilineTextFieldDemoEvent::*;
        use MultilineTextFieldSize::{FixedRows, Grow};
        let demo = Message::MultilineTextFieldDemo;
        let (id, label, value, size, editability, help) = match sample {
            Sample::Grow => (
                GROW,
                "多行備註",
                self.grow.value.as_str(),
                Grow,
                TextFieldEditability::Editable,
                "請簡短記錄這次的變更。",
            ),
            Sample::Fixed => (
                FIXED,
                "詳細說明",
                self.fixed.value.as_str(),
                FixedRows(FIXED_ROWS),
                TextFieldEditability::Editable,
                "內容會在固定高度內捲動。",
            ),
            Sample::ReadOnly => (
                READ_ONLY,
                "唯讀內容",
                "已儲存的第一行\n已儲存的第二行\n",
                FixedRows(READ_ONLY_ROWS),
                TextFieldEditability::ReadOnly,
                "這段文字可以選取和複製。",
            ),
            Sample::Disabled => (
                DISABLED,
                "停用內容",
                "停用的第一行\n停用的第二行",
                Grow,
                TextFieldEditability::Disabled,
                "目前無法編輯這段文字。",
            ),
        };
        let mut options = TextFieldOptions::new(id, label, value);
        options.editability = editability;
        options.supporting_text = Some(help);
        options.text_scale = self.text_scale();
        options.direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let mut adornments = TextFieldAdornments::default();
        let describe = |count, limit| format!("字數，已輸入 {count} 個字，上限 {limit} 個字。");
        let counter = if matches!(sample, Sample::Grow) {
            options.placeholder = Some("輸入備註");
            adornments.prefix = Some(TextFieldAffix {
                visual: "#",
                spoken_name: "井字號",
            });
            adornments.trailing_action = Some(TextFieldTrailingAction {
                icon: clear_icon(),
                name: "清除備註",
                disabled: value.is_empty(),
                on_press: demo(ClearGrow),
            });
            Some(TextFieldCounterOptions {
                max_graphemes: GROW_LIMIT,
                over_limit_error: "備註已超過 12 個字。",
                describe: &describe,
            })
        } else {
            None
        };
        theme
            .multiline_text_field(
                self.appearance,
                options,
                size,
                adornments,
                counter,
                move |value| {
                    demo(match sample {
                        Sample::Grow => EditGrow(value),
                        Sample::Fixed => EditFixed(value),
                        Sample::ReadOnly | Sample::Disabled => EditLocked,
                    })
                },
            )
            .expect("valid Gallery multiline field")
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use MultilineTextFieldDemoEvent::*;
        let demo = Message::MultilineTextFieldDemo;
        let toggle = |id, label, selected, message| {
            theme
                .segmented_button(
                    id,
                    label,
                    ChoiceState {
                        selected,
                        disabled: false,
                    },
                    message,
                )
                .accessible_activation()
                .suppress_repeated_activation()
        };
        let controls = control_row(vec![
            theme
                .tonal_button(BACK, "返回", demo(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation()
                .suppress_repeated_activation(),
            toggle(
                APPEARANCE,
                "填色",
                self.appearance == TextFieldAppearance::Filled,
                demo(ToggleAppearance),
            ),
            toggle(THEME, "深色", dark, Message::Theme),
            toggle(RTL, "右起", self.rtl, demo(ToggleDirection)),
            toggle(LARGE_TEXT, "大字", self.large_text, demo(ToggleLargeText)),
        ]);
        let submit = theme
            .button_with(SUBMIT, "送出草稿", ButtonKind::Filled, false, demo(Submit))
            .accessible_activation()
            .accessibility_scroll_into_view()
            .suppress_repeated_activation();
        let feedback = View::new(
            Node::new(Role::Status, "")
                .id("multiline-text-field-demo-feedback")
                .text(if self.last_submit.is_some() {
                    "草稿已送出。"
                } else {
                    ""
                })
                .styled(
                    Style::default()
                        .with_text_style(typography::BODY_MEDIUM)
                        .foreground(theme.colors.on_surface),
                ),
        );
        let mut scroll_style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .align(Align::Start)
            .padding(8.)
            .gap(16.)
            .clip(true);
        scroll_style.max_width = Length::Px(520.);
        scroll_style.max_height = Length::Px(480.);
        let scroll = View::new(
            Node::new(Role::Group, "多行欄位範例")
                .id(SCROLL)
                .styled(scroll_style),
        )
        .accessibility_scroll_pages()
        .children([
            self.field(theme, Sample::Grow),
            self.field(theme, Sample::Fixed),
            submit,
            feedback,
            self.field(theme, Sample::ReadOnly),
            self.field(theme, Sample::Disabled),
        ]);
        View::new(
            Node::new(Role::Window, TITLE)
                .id("multiline-text-field-demo-route")
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.))
                        .align(Align::Start)
                        .padding(16.)
                        .gap(12.)
                        .background(theme.background)
                        .foreground(theme.on_background)
                        .clip(true),
                ),
        )
        .children([
            theme.heading("multiline-text-field-demo-title", TITLE),
            controls,
            scroll,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme, dark: bool) -> serde_json::Value {
        let color = |surface::Color(r, g, b, a)| [r, g, b, a];
        let colors = theme.colors;
        let on_surface_alpha = |alpha| {
            let surface::Color(r, g, b, _) = colors.on_surface;
            [r, g, b, alpha]
        };
        let last_submit = self
            .last_submit
            .as_ref()
            .map(|(grow, fixed)| serde_json::json!({ "grow": grow, "fixed": fixed }));
        serde_json::json!({
            "visible": self.visible,
            "width": self.width,
            "dark": dark,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "appearance": match self.appearance {
                TextFieldAppearance::Outlined => "outlined",
                TextFieldAppearance::Filled => "filled",
            },
            "grow": {
                "value": self.grow.value, "edits": self.grow.edits, "actions": self.actions,
                "limit": GROW_LIMIT,
            },
            "fixed": { "value": self.fixed.value, "edits": self.fixed.edits, "rows": FIXED_ROWS.get() },
            "submits": self.submits,
            "last_submit": last_submit,
            "locked_edit_requests": self.locked_edit_requests,
            "palette": {
                "background": color(theme.background),
                "surface": color(colors.surface),
                "surface_container_highest": color(colors.surface_container_highest),
                "outline": color(colors.outline),
                "on_surface": color(colors.on_surface),
                "on_surface_variant": color(colors.on_surface_variant),
                "on_surface_alpha_4": on_surface_alpha(10),
                "on_surface_alpha_8": on_surface_alpha(20),
                "on_surface_alpha_38": on_surface_alpha(97),
                "primary": color(colors.primary),
                "secondary": color(colors.secondary),
                "error": color(colors.error),
                "on_error_container": color(colors.on_error_container),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallery;
    use surface::{ComputedNode, TextMeasurer};
    use surface_app::{AccessibilityFocusResult, Event, Runtime};

    struct Metrics;

    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
            let cell = size / 2.;
            let columns = width.map(|width| (width / cell) as usize);
            let mut rows = 0;
            let mut widest = 0;
            for line in text.split('\n') {
                let len = line.chars().count();
                let columns = columns.unwrap_or(len).max(1);
                rows += len.div_ceil(columns).max(1);
                widest = widest.max(len.min(columns));
            }
            (widest as f32 * cell, rows as f32 * size * 1.5)
        }
    }

    fn focus(runtime: &mut Runtime<Gallery>, id: &str) {
        let editor = runtime.dom().by_id(id).unwrap();
        let target = runtime
            .accessibility_frame()
            .unwrap()
            .focus_target(editor)
            .unwrap();
        assert_eq!(
            runtime.accessibility_focus(&target).unwrap(),
            AccessibilityFocusResult::Moved
        );
    }

    fn geometry<'a>(runtime: &'a Runtime<Gallery>, id: &str) -> &'a ComputedNode {
        let node = runtime.dom().by_id(id).unwrap();
        &runtime.computed().unwrap().nodes[node.index()]
    }

    #[test]
    fn multiline_route_retains_two_editors_and_usable_nested_scroll_space() {
        use MultilineTextFieldDemoEvent::*;
        let message = Message::MultilineTextFieldDemo;
        for (width, scale) in [(960., 1.), (320., 2.)] {
            let mut runtime = Runtime::new(Gallery::default()).unwrap();
            runtime.apply(message(Show)).unwrap();
            if scale == 2. {
                if !runtime.app().dark {
                    runtime.apply(Message::Theme).unwrap();
                }
                runtime.apply(message(ToggleDirection)).unwrap();
                runtime.apply(message(ToggleLargeText)).unwrap();
            } else if runtime.app().dark {
                runtime.apply(Message::Theme).unwrap();
            }
            let mut fonts = Metrics;
            let bounds = |runtime: &Runtime<Gallery>, id| geometry(runtime, id).bounds;
            for appearance in [TextFieldAppearance::Outlined, TextFieldAppearance::Filled] {
                runtime.render_extended(width, 900., &mut fonts).unwrap();
                let demo = &runtime.app().multiline_text_field_demo;
                assert_eq!(demo.appearance, appearance);
                assert_eq!(demo.text_scale(), scale);
                assert_eq!(runtime.app().dark, scale == 2.);
                assert_eq!(demo.fixed.value, FIXED_INITIAL);
                let scroller = bounds(&runtime, SCROLL);
                let body = bounds(&runtime, "multiline-text-field-demo-fixed-value-box");
                assert_eq!(scroller.height, 480.);
                assert_eq!(body.height, 72. * scale);
                assert_eq!(body.intersect(scroller), body);
                let disabled = bounds(&runtime, DISABLED);
                assert!(disabled.y + disabled.height > scroller.y + scroller.height);
                let editor_id = runtime.dom().by_id(FIXED).unwrap();
                let frame = runtime.accessibility_frame().unwrap();
                assert!(
                    frame
                        .node(editor_id)
                        .unwrap()
                        .text_options
                        .unwrap()
                        .multiline
                );
                runtime.apply(message(ToggleAppearance)).unwrap();
            }
            runtime.render_extended(width, 900., &mut fonts).unwrap();
            focus(&mut runtime, GROW);
            runtime.render_extended(width, 900., &mut fonts).unwrap();
            let before = bounds(&runtime, GROW);
            runtime.dispatch(Event::Text("\n".into())).unwrap();
            runtime.render_extended(width, 900., &mut fonts).unwrap();
            assert_eq!(bounds(&runtime, GROW).height, before.height + 24. * scale);
            assert_eq!(runtime.app().multiline_text_field_demo.submits, 0);
            focus(&mut runtime, SUBMIT);
            runtime.dispatch(Event::Activate).unwrap();
            let demo = &runtime.app().multiline_text_field_demo;
            assert_eq!(demo.grow.edits, 1);
            assert_eq!(demo.fixed.edits, 0);
            assert_eq!(demo.submits, 1);
            assert_eq!(
                demo.last_submit,
                Some((demo.grow.value.clone(), FIXED_INITIAL.into()))
            );
        }
    }
}
