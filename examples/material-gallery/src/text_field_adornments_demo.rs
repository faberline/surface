use std::sync::OnceLock;

use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChoiceState, TextFieldAdornments,
    TextFieldAffix, TextFieldAppearance, TextFieldEditability, TextFieldOptions,
    TextFieldTrailingAction, Theme, VectorIcon,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const TITLE: &str = "文字欄位配件";
const SHOW: &str = "text-field-adornments-demo-show";
const BACK: &str = "text-field-adornments-demo-back";
const APPEARANCE: &str = "text-field-adornments-demo-appearance";
const THEME: &str = "text-field-adornments-demo-theme";
const RTL: &str = "text-field-adornments-demo-rtl";
const LARGE_TEXT: &str = "text-field-adornments-demo-large-text";
const NARROW: &str = "text-field-adornments-demo-narrow";
const ERROR: &str = "text-field-adornments-demo-error";
const EDITABLE: &str = "text-field-adornments-demo-editable";
const READ_ONLY: &str = "text-field-adornments-demo-read-only";
const DISABLED: &str = "text-field-adornments-demo-disabled";
const NARROW_WIDTH: f32 = 112.;
const SUGGESTED_VALUE: &str = "24";

#[derive(Clone)]
pub enum TextFieldAdornmentsDemoEvent {
    Show,
    Back,
    ToggleAppearance,
    ToggleDirection,
    ToggleLargeText,
    ToggleNarrow,
    ToggleError,
    Edit(String),
    EditLocked,
    ApplySuggestion,
    ReviewRate,
    DisabledAction,
    Submit,
}

#[derive(Clone, Copy)]
enum Feedback {
    Applied,
    Reviewed,
    Submitted,
}

impl Feedback {
    fn text(self) -> &'static str {
        match self {
            Self::Applied => "已套用 24 歐元的每月費用。",
            Self::Reviewed => "目前費率為每月 18 歐元。",
            Self::Submitted => "已送出欄位內的金額。",
        }
    }
}

#[derive(Clone, Copy)]
enum Sample {
    Editable,
    ReadOnly,
    Disabled,
}

pub(crate) struct TextFieldAdornmentsDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    narrow: bool,
    show_error: bool,
    width: f32,
    appearance: TextFieldAppearance,
    value: String,
    edits: usize,
    actions: usize,
    submits: usize,
    last_submit: Option<String>,
    read_only_actions: usize,
    disabled_action_requests: usize,
    locked_edit_requests: usize,
    feedback: Option<Feedback>,
}

impl Default for TextFieldAdornmentsDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            narrow: false,
            show_error: false,
            width: 960.,
            appearance: TextFieldAppearance::Outlined,
            value: String::new(),
            edits: 0,
            actions: 0,
            submits: 0,
            last_submit: None,
            read_only_actions: 0,
            disabled_action_requests: 0,
            locked_edit_requests: 0,
            feedback: None,
        }
    }
}

impl TextFieldAdornmentsDemo {
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

    pub(crate) fn update(&mut self, event: TextFieldAdornmentsDemoEvent) {
        use TextFieldAdornmentsDemoEvent::*;
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
            ToggleNarrow => self.narrow = !self.narrow,
            ToggleError => self.show_error = !self.show_error,
            Edit(value) => {
                self.value = value;
                self.edits += 1;
            }
            EditLocked => self.locked_edit_requests += 1,
            ApplySuggestion => {
                self.value = SUGGESTED_VALUE.to_owned();
                self.actions += 1;
                self.feedback = Some(Feedback::Applied);
            }
            ReviewRate => {
                self.read_only_actions += 1;
                self.feedback = Some(Feedback::Reviewed);
            }
            DisabledAction => self.disabled_action_requests += 1,
            Submit => {
                self.submits += 1;
                self.last_submit = Some(self.value.clone());
                self.feedback = Some(Feedback::Submitted);
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
                Message::TextFieldAdornmentsDemo(TextFieldAdornmentsDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
            .suppress_repeated_activation()
    }

    fn field(&self, theme: &Theme, sample: Sample) -> View<Message> {
        use TextFieldAdornmentsDemoEvent::*;
        let demo = Message::TextFieldAdornmentsDemo;
        let (id, label, value, editability, action_name, icon, action) = match sample {
            Sample::Editable => (
                EDITABLE,
                "方案費用",
                self.value.as_str(),
                TextFieldEditability::Editable,
                "套用建議金額",
                suggestion_icon(),
                ApplySuggestion,
            ),
            Sample::ReadOnly => (
                READ_ONLY,
                "目前方案費用",
                "18",
                TextFieldEditability::ReadOnly,
                "查看現有費率",
                details_icon(),
                ReviewRate,
            ),
            Sample::Disabled => (
                DISABLED,
                "停用方案費用",
                "30",
                TextFieldEditability::Disabled,
                "套用停用方案金額",
                suggestion_icon(),
                DisabledAction,
            ),
        };
        let mut options = TextFieldOptions::new(id, label, value);
        options.editability = editability;
        options.text_scale = self.text_scale();
        options.direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let editable = matches!(sample, Sample::Editable);
        if editable {
            options.placeholder = Some("輸入金額");
            options.supporting_text = Some("請輸入不含單位的金額。");
            options.error = self.show_error.then_some("請輸入大於零的金額。");
        }
        let adornments = TextFieldAdornments {
            leading_icon: Some(card_icon()),
            trailing_action: Some(TextFieldTrailingAction {
                icon,
                name: action_name,
                disabled: false,
                on_press: demo(action),
            }),
            prefix: Some(TextFieldAffix {
                visual: "€",
                spoken_name: "歐元",
            }),
            suffix: Some(TextFieldAffix {
                visual: "/月",
                spoken_name: "每月",
            }),
        };
        let field = theme
            .text_field_with_adornments(
                self.appearance,
                options,
                adornments,
                move |value| demo(if editable { Edit(value) } else { EditLocked }),
                move || editable.then(|| demo(Submit)),
            )
            .expect("valid adorned Gallery field");
        if editable && self.narrow {
            let mut style = Style::column().width(Length::Px(NARROW_WIDTH));
            style.max_width = Length::Percent(1.);
            let wrapper = Node::new(Role::Group, "")
                .id("text-field-adornments-demo-narrow-specimen")
                .styled(style);
            View::new(wrapper).children([field])
        } else {
            field
        }
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use TextFieldAdornmentsDemoEvent::*;
        let demo = Message::TextFieldAdornmentsDemo;
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
            toggle(NARROW, "窄欄", self.narrow, demo(ToggleNarrow)),
            toggle(ERROR, "錯誤", self.show_error, demo(ToggleError)),
        ]);
        let mut feedback_style = Style::column()
            .width(Length::Percent(1.))
            .with_text_style(typography::BODY_MEDIUM)
            .foreground(theme.colors.on_surface);
        feedback_style.min_height = typography::BODY_MEDIUM.line_height_px();
        let feedback = View::new(
            Node::new(Role::Status, "")
                .id("text-field-adornments-demo-feedback")
                .text(self.feedback.map_or("", Feedback::text))
                .styled(feedback_style),
        );
        let mut scroll_style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .align(Align::Start)
            .padding(8.)
            .gap(20.)
            .clip(true);
        scroll_style.max_width = Length::Px(520.);
        let scroll = View::new(
            Node::new(Role::Group, "附配件的欄位範例")
                .id("text-field-adornments-demo-scroll")
                .styled(scroll_style),
        )
        .accessibility_scroll_pages()
        .children([
            self.field(theme, Sample::Editable),
            self.field(theme, Sample::ReadOnly),
            self.field(theme, Sample::Disabled),
            feedback,
        ]);
        View::new(
            Node::new(Role::Window, TITLE)
                .id("text-field-adornments-demo-route")
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
            theme.heading("text-field-adornments-demo-title", TITLE),
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
        serde_json::json!({
            "visible": self.visible,
            "width": self.width,
            "dark": dark,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "narrow": self.narrow,
            "fixture_width": self.narrow.then_some(NARROW_WIDTH),
            "appearance": match self.appearance {
                TextFieldAppearance::Outlined => "outlined",
                TextFieldAppearance::Filled => "filled",
            },
            "show_error": self.show_error,
            "value": self.value,
            "edits": self.edits,
            "actions": self.actions,
            "submits": self.submits,
            "last_submit": self.last_submit,
            "read_only_actions": self.read_only_actions,
            "disabled_action_requests": self.disabled_action_requests,
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

fn card_icon() -> &'static VectorIcon {
    static CARD: OnceLock<VectorIcon> = OnceLock::new();
    CARD.get_or_init(|| {
        VectorIcon::from_contours([
            [[2., 5.], [22., 5.], [22., 19.], [2., 19.]],
            [[4., 7.], [20., 7.], [20., 17.], [4., 17.]],
            [[4., 9.], [20., 9.], [20., 12.], [4., 12.]],
        ])
        .expect("valid card glyph")
    })
}

fn suggestion_icon() -> &'static VectorIcon {
    static CHECK: OnceLock<VectorIcon> = OnceLock::new();
    CHECK.get_or_init(|| {
        VectorIcon::new([
            [2., 12.],
            [4., 10.],
            [9., 15.],
            [20., 4.],
            [22., 6.],
            [9., 19.],
        ])
        .expect("valid check glyph")
    })
}

fn details_icon() -> &'static VectorIcon {
    static INFO: OnceLock<VectorIcon> = OnceLock::new();
    INFO.get_or_init(|| {
        VectorIcon::from_contours([
            [[4., 2.], [20., 2.], [20., 22.], [4., 22.]],
            [[6., 4.], [18., 4.], [18., 20.], [6., 20.]],
            [[8., 8.], [16., 8.], [16., 10.], [8., 10.]],
            [[8., 13.], [16., 13.], [16., 15.], [8., 15.]],
        ])
        .expect("valid details glyph")
    })
}
