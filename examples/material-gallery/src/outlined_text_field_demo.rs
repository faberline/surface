use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChoiceState, OutlinedTextFieldOptions,
    TextFieldEditability, Theme,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "outlined-text-field-demo-show";
const BACK: &str = "outlined-text-field-demo-back";
const THEME: &str = "outlined-text-field-demo-theme";
const RTL: &str = "outlined-text-field-demo-rtl";
const LARGE_TEXT: &str = "outlined-text-field-demo-large-text";
const ERROR: &str = "outlined-text-field-demo-error";
const EDITABLE: &str = "outlined-text-field-demo-editable";
const READ_ONLY: &str = "outlined-text-field-demo-read-only";
const DISABLED: &str = "outlined-text-field-demo-disabled";
const LONG_LABEL: &str = "outlined-text-field-demo-long-label";
const TITLE: &str = "外框文字欄位";

#[derive(Clone)]
pub enum OutlinedTextFieldDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    ToggleError,
    Edit(String),
    EditLong(String),
    EditLocked,
    Submit,
}

pub(crate) struct OutlinedTextFieldDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    show_error: bool,
    width: f32,
    value: String,
    edits: usize,
    submits: usize,
    long_value: String,
    long_edits: usize,
    locked_edit_requests: usize,
}

impl Default for OutlinedTextFieldDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            show_error: false,
            width: 960.,
            value: String::new(),
            edits: 0,
            submits: 0,
            long_value: "共用名稱".to_owned(),
            long_edits: 0,
            locked_edit_requests: 0,
        }
    }
}

impl OutlinedTextFieldDemo {
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

    pub(crate) fn update(&mut self, event: OutlinedTextFieldDemoEvent) {
        use OutlinedTextFieldDemoEvent::*;
        if !self.visible && !matches!(event, Show) {
            return;
        }
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            ToggleError => self.show_error = !self.show_error,
            Edit(value) => {
                self.value = value;
                self.edits += 1;
            }
            EditLong(value) => {
                self.long_value = value;
                self.long_edits += 1;
            }
            EditLocked => self.locked_edit_requests += 1,
            Submit => self.submits += 1,
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                TITLE,
                ButtonKind::Outlined,
                false,
                Message::OutlinedTextFieldDemo(OutlinedTextFieldDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
            .suppress_repeated_activation()
    }

    fn options<'a>(
        &self,
        id: &'a str,
        label: &'a str,
        value: &'a str,
    ) -> OutlinedTextFieldOptions<'a> {
        let mut options = OutlinedTextFieldOptions::new(id, label, value);
        options.text_scale = self.text_scale();
        options.direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        options
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use OutlinedTextFieldDemoEvent::*;
        let demo = Message::OutlinedTextFieldDemo;
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
            toggle(THEME, "深色", dark, Message::Theme),
            toggle(RTL, "右起", self.rtl, demo(ToggleDirection)),
            toggle(LARGE_TEXT, "大字", self.large_text, demo(ToggleLargeText)),
            toggle(ERROR, "錯誤", self.show_error, demo(ToggleError)),
        ]);

        let mut editable = self.options(EDITABLE, "專案名稱", &self.value);
        editable.placeholder = Some("輸入名稱");
        editable.supporting_text = Some("請輸入中文或英文名稱。");
        editable.error = self.show_error.then_some("請輸入有效的專案名稱。");
        let editable = theme
            .outlined_text_field_with_submit(
                editable,
                move |value| demo(Edit(value)),
                move || Some(demo(Submit)),
            )
            .expect("valid editable Gallery field");

        let mut read_only = self.options(READ_ONLY, "唯讀名稱", "既有專案");
        read_only.editability = TextFieldEditability::ReadOnly;
        let read_only = theme
            .outlined_text_field(read_only, move |_| demo(EditLocked))
            .expect("valid read-only Gallery field");

        let mut disabled = self.options(DISABLED, "停用名稱", "已封存專案");
        disabled.editability = TextFieldEditability::Disabled;
        let disabled = theme
            .outlined_text_field(disabled, move |_| demo(EditLocked))
            .expect("valid disabled Gallery field");

        let long_label = self.options(
            LONG_LABEL,
            "輸入團隊成員在共用專案清單中容易辨認的完整名稱",
            &self.long_value,
        );
        let long_label = theme
            .outlined_text_field(long_label, move |value| demo(EditLong(value)))
            .expect("valid long-label Gallery field");

        let status = View::new(
            Node::new(Role::Status, "")
                .id("outlined-text-field-demo-status")
                .text(format!("送出次數：{}", self.submits))
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .with_text_style(typography::BODY_MEDIUM)
                        .foreground(theme.colors.on_surface),
                ),
        );
        let mut scroll_style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .padding(8.)
            .gap(20.)
            .clip(true);
        scroll_style.max_width = Length::Px(520.);

        let scroll = View::new(
            Node::new(Role::Group, "文字欄位範例")
                .id("outlined-text-field-demo-scroll")
                .styled(scroll_style),
        )
        .accessibility_scroll_pages()
        .children([editable, read_only, disabled, long_label, status]);

        View::new(
            Node::new(Role::Window, TITLE)
                .id("outlined-text-field-demo-route")
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
            theme.heading("outlined-text-field-demo-title", TITLE),
            controls,
            scroll,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme, dark: bool) -> serde_json::Value {
        let color = |surface::Color(r, g, b, a)| [r, g, b, a];
        let colors = theme.colors;
        serde_json::json!({
            "visible": self.visible,
            "width": self.width,
            "dark": dark,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "show_error": self.show_error,
            "value": self.value,
            "edits": self.edits,
            "submits": self.submits,
            "long_value": self.long_value,
            "long_edits": self.long_edits,
            "locked_edit_requests": self.locked_edit_requests,
            "palette": {
                "background": color(theme.background),
                "surface": color(colors.surface),
                "outline": color(colors.outline),
                "on_surface": color(colors.on_surface),
                "on_surface_variant": color(colors.on_surface_variant),
                "primary": color(colors.primary),
                "error": color(colors.error),
                "on_error_container": color(colors.on_error_container),
            },
        })
    }
}
