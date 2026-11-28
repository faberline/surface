use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChoiceState, OutlinedTextFieldOptions, Theme,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "text-field-placeholder-demo-show";
const BACK: &str = "text-field-placeholder-demo-back";
const TITLE: &str = "提示文字淡入淡出";
const EDITOR: &str = "text-field-placeholder-demo-editor";
const SHORT_LABEL: &str = "名稱";
const LONG_LABEL: &str = "輸入團隊成員在共用專案清單中容易辨認的完整名稱";
const HINT: &str = "請輸入完整名稱";
const SEED: &str = "範例";

#[derive(Clone)]
pub enum TextFieldPlaceholderDemoEvent {
    Show,
    Back,
    ToggleFilled,
    ToggleRtl,
    ToggleLargeText,
    ToggleAbove,
    ToggleHint,
    SetValue,
    ClearValue,
    Edit(String),
}

#[derive(Default)]
pub(crate) struct TextFieldPlaceholderDemo {
    visible: bool,
    filled: bool,
    rtl: bool,
    large_text: bool,
    above: bool,
    without_hint: bool,
    value: String,
    edits: usize,
    authored_writes: usize,
}

impl TextFieldPlaceholderDemo {
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

    pub(crate) fn leave(&mut self) {
        self.visible = false;
    }

    pub(crate) fn update(&mut self, event: TextFieldPlaceholderDemoEvent) {
        use TextFieldPlaceholderDemoEvent::*;
        if !self.visible && !matches!(event, Show) {
            return;
        }
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleFilled => self.filled = !self.filled,
            ToggleRtl => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            ToggleAbove => self.above = !self.above,
            ToggleHint => self.without_hint = !self.without_hint,
            SetValue => {
                self.value = SEED.to_owned();
                self.authored_writes += 1;
            }
            ClearValue => {
                self.value.clear();
                self.authored_writes += 1;
            }
            Edit(value) => {
                self.value = value;
                self.edits += 1;
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
                Message::TextFieldPlaceholderDemo(TextFieldPlaceholderDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
            .suppress_repeated_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use TextFieldPlaceholderDemoEvent::*;

        let wrap = Message::TextFieldPlaceholderDemo;
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
        let command = |id, label, event| {
            theme
                .tonal_button(id, label, wrap(event))
                .accessible_activation()
                .suppress_repeated_activation()
        };
        let view = control_row(vec![
            command(BACK, "返回", Back).restore_focus_on_removal(SHOW),
            toggle(
                "text-field-placeholder-demo-theme",
                "深色",
                dark,
                Message::Theme,
            ),
            toggle(
                "text-field-placeholder-demo-filled",
                "填色",
                self.filled,
                wrap(ToggleFilled),
            ),
            toggle(
                "text-field-placeholder-demo-rtl",
                "右起",
                self.rtl,
                wrap(ToggleRtl),
            ),
            toggle(
                "text-field-placeholder-demo-large",
                "大字",
                self.large_text,
                wrap(ToggleLargeText),
            ),
            toggle(
                "text-field-placeholder-demo-above",
                "長標籤",
                self.above,
                wrap(ToggleAbove),
            ),
            toggle(
                "text-field-placeholder-demo-no-hint",
                "隱去提示",
                self.without_hint,
                wrap(ToggleHint),
            ),
        ]);
        let value = control_row(vec![
            command("text-field-placeholder-demo-set", "填入範例", SetValue),
            command("text-field-placeholder-demo-clear", "清空範例", ClearValue),
        ]);

        let label = if self.above { LONG_LABEL } else { SHORT_LABEL };
        let mut options = OutlinedTextFieldOptions::new(EDITOR, label, &self.value);
        options.placeholder = (!self.without_hint).then_some(HINT);
        options.direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        options.text_scale = self.text_scale();
        let field = if self.filled {
            theme.filled_text_field(options, move |value| wrap(Edit(value)))
        } else {
            theme.outlined_text_field(options, move |value| wrap(Edit(value)))
        }
        .expect("valid placeholder demo");
        let mut body = Style::column().width(Length::Percent(1.));
        body.max_width = Length::Px(480.);
        let field = View::new(Node::new(Role::Group, "").styled(body)).children([field]);

        View::new(
            Node::new(Role::Window, TITLE)
                .id("text-field-placeholder-demo-route")
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
            theme.heading("text-field-placeholder-demo-title", TITLE),
            view,
            value,
            field,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme, dark: bool) -> serde_json::Value {
        let color = |surface::Color(r, g, b, a)| [r, g, b, a];
        let palette = theme.colors;
        let paper = if self.filled {
            palette.surface_container_highest
        } else {
            theme.background
        };
        serde_json::json!({
            "visible": self.visible,
            "dark": dark,
            "filled": self.filled,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "above": self.above,
            "without_hint": self.without_hint,
            "value": self.value,
            "edits": self.edits,
            "authored_writes": self.authored_writes,
            "palette": {
                "paper": color(paper),
                "hint": color(palette.on_surface_variant),
                "text": color(palette.on_surface),
                "primary": color(palette.primary),
            },
        })
    }
}
