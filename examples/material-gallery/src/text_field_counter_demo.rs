use std::sync::OnceLock;

use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChoiceState, TextFieldAdornments,
    TextFieldAffix, TextFieldAppearance, TextFieldCounterOptions, TextFieldEditability,
    TextFieldOptions, TextFieldTrailingAction, Theme, VectorIcon,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const TITLE: &str = "文字欄位字數";
const SHOW: &str = "text-field-counter-demo-show";
const BACK: &str = "text-field-counter-demo-back";
const APPEARANCE: &str = "text-field-counter-demo-appearance";
const THEME: &str = "text-field-counter-demo-theme";
const RTL: &str = "text-field-counter-demo-rtl";
const LARGE_TEXT: &str = "text-field-counter-demo-large-text";
const NARROW: &str = "text-field-counter-demo-narrow";
const ERROR: &str = "text-field-counter-demo-error";
const EDITABLE: &str = "text-field-counter-demo-editable";
const READ_ONLY: &str = "text-field-counter-demo-read-only";
const DISABLED: &str = "text-field-counter-demo-disabled";
const NARROW_WIDTH: f32 = 112.;
const LIMIT: usize = 5;

#[derive(Clone)]
pub enum TextFieldCounterDemoEvent {
    Show,
    Back,
    ToggleAppearance,
    ToggleDirection,
    ToggleLargeText,
    ToggleNarrow,
    ToggleError,
    Edit(String),
    EditLocked,
    Clear,
    Submit,
}

#[derive(Clone, Copy)]
enum Feedback {
    Cleared,
    Submitted,
}

impl Feedback {
    fn text(self) -> &'static str {
        match self {
            Self::Cleared => "已清除標籤。",
            Self::Submitted => "已送出標籤。",
        }
    }
}

#[derive(Clone, Copy)]
enum Sample {
    Editable,
    ReadOnly,
    Disabled,
}

pub(crate) struct TextFieldCounterDemo {
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
    locked_edit_requests: usize,
    feedback: Option<Feedback>,
}

impl Default for TextFieldCounterDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            narrow: false,
            show_error: false,
            width: 960.,
            appearance: TextFieldAppearance::Outlined,
            value: "中a\u{301}👍🏽".to_owned(),
            edits: 0,
            actions: 0,
            submits: 0,
            last_submit: None,
            locked_edit_requests: 0,
            feedback: None,
        }
    }
}

impl TextFieldCounterDemo {
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

    pub(crate) fn update(&mut self, event: TextFieldCounterDemoEvent) {
        use TextFieldCounterDemoEvent::*;
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
            Clear => {
                self.value.clear();
                self.actions += 1;
                self.feedback = Some(Feedback::Cleared);
            }
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
                Message::TextFieldCounterDemo(TextFieldCounterDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
            .suppress_repeated_activation()
    }

    fn field(&self, theme: &Theme, sample: Sample) -> View<Message> {
        use TextFieldCounterDemoEvent::*;
        let demo = Message::TextFieldCounterDemo;
        let (id, label, value, editability) = match sample {
            Sample::Editable => (
                EDITABLE,
                "標籤名稱",
                self.value.as_str(),
                TextFieldEditability::Editable,
            ),
            Sample::ReadOnly => (
                READ_ONLY,
                "唯讀標籤",
                "🇹🇼a\u{301}",
                TextFieldEditability::ReadOnly,
            ),
            Sample::Disabled => (
                DISABLED,
                "停用標籤",
                "停用標籤內容",
                TextFieldEditability::Disabled,
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
        options.supporting_text = match sample {
            Sample::Editable => {
                Some("請輸入 5 個字以內的標籤名稱。英文字母、中文或表情符號都可以使用。")
            }
            Sample::ReadOnly => Some("已儲存的標籤。"),
            Sample::Disabled => None,
        };
        let mut adornments = TextFieldAdornments::default();
        if editable {
            options.placeholder = Some("輸入標籤名稱");
            options.error = self.show_error.then_some("這個標籤已存在。");
            adornments.prefix = Some(TextFieldAffix {
                visual: "#",
                spoken_name: "井字號",
            });
            adornments.trailing_action = Some(TextFieldTrailingAction {
                icon: clear_icon(),
                name: "清除標籤",
                disabled: self.value.is_empty(),
                on_press: demo(Clear),
            });
        }
        let field = theme
            .text_field_with_counter(
                self.appearance,
                options,
                adornments,
                TextFieldCounterOptions {
                    max_graphemes: LIMIT,
                    over_limit_error: "請輸入最多 5 個字。",
                    describe: &|count, limit| {
                        format!("字數，已輸入 {count} 個字，上限 {limit} 個字。")
                    },
                },
                move |value| demo(if editable { Edit(value) } else { EditLocked }),
                move || editable.then(|| demo(Submit)),
            )
            .expect("valid Gallery field with character count");
        if editable && self.narrow {
            let mut style = Style::column().width(Length::Px(NARROW_WIDTH));
            style.max_width = Length::Percent(1.);
            let wrapper = Node::new(Role::Group, "")
                .id("text-field-counter-demo-narrow-specimen")
                .styled(style);
            View::new(wrapper).children([field])
        } else {
            field
        }
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use TextFieldCounterDemoEvent::*;
        let demo = Message::TextFieldCounterDemo;
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
                .id("text-field-counter-demo-feedback")
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
            Node::new(Role::Group, "附字數的欄位範例")
                .id("text-field-counter-demo-scroll")
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
                .id("text-field-counter-demo-route")
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
            theme.heading("text-field-counter-demo-title", TITLE),
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
            "limit": LIMIT,
            "edits": self.edits,
            "actions": self.actions,
            "submits": self.submits,
            "last_submit": self.last_submit,
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

pub(crate) fn clear_icon() -> &'static VectorIcon {
    static CLEAR: OnceLock<VectorIcon> = OnceLock::new();
    CLEAR.get_or_init(|| {
        VectorIcon::new([
            [6., 4.],
            [12., 10.],
            [18., 4.],
            [20., 6.],
            [14., 12.],
            [20., 18.],
            [18., 20.],
            [12., 14.],
            [6., 20.],
            [4., 18.],
            [10., 12.],
            [4., 6.],
        ])
        .expect("valid clear glyph")
    })
}
