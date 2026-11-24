use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChoiceState, DividerKind, DividerLayout,
    DividerWeight, Theme,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 3;
const SHOW: &str = "divider-demo-show";
const BACK: &str = "divider-demo-back";

#[derive(Clone)]
pub enum DividerDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    ToggleWeight,
}

#[derive(Default)]
pub(crate) struct DividerDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    heavy: bool,
}

impl DividerDemo {
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

    pub(crate) fn update(&mut self, event: DividerDemoEvent) {
        use DividerDemoEvent::{Back, Show, ToggleDirection, ToggleLargeText, ToggleWeight};
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            ToggleWeight => self.heavy = !self.heavy,
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "分隔線",
                ButtonKind::Outlined,
                false,
                Message::DividerDemo(DividerDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use DividerDemoEvent::{Back, ToggleDirection, ToggleLargeText, ToggleWeight};
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let weight = if self.heavy {
            DividerWeight::Heavy
        } else {
            DividerWeight::Regular
        };
        let horizontal = |id, kind| {
            theme.decorative_divider_with_weight(
                id,
                DividerLayout::Horizontal { kind, direction },
                weight,
            )
        };
        let sample = |id: &str, label: &str, divider| {
            View::new(Node::new(Role::Group, "").styled(Style::column().gap(4.))).children([
                View::new(
                    Node::new(Role::Heading, label).id(id).text(label).styled(
                        Style::column()
                            .with_text_style(typography::TITLE_MEDIUM)
                            .foreground(theme.on_background),
                    ),
                ),
                divider,
            ])
        };
        let vertical = View::new(
            Node::new(Role::Group, "").styled(
                Style::row()
                    .align(Align::Center)
                    .gap(16.)
                    .height(Length::Px(64.))
                    .width(Length::Percent(1.)),
            ),
        )
        .children([
            theme.text("divider-demo-items", "項目"),
            theme.decorative_divider_with_weight(
                "divider-demo-vertical",
                DividerLayout::Vertical,
                weight,
            ),
            theme.text("divider-demo-notes", "備註"),
        ]);
        let toggle = |id, label, selected, event| {
            theme
                .segmented_button(
                    id,
                    label,
                    ChoiceState {
                        selected,
                        disabled: false,
                    },
                    Message::DividerDemo(event),
                )
                .accessible_activation()
        };
        theme.page([
            theme.heading("divider-demo-title", "分隔線"),
            control_row(vec![
                theme
                    .tonal_button(BACK, "返回", Message::DividerDemo(Back))
                    .restore_focus_on_removal(SHOW)
                    .accessible_activation(),
                theme
                    .switch(
                        "divider-demo-theme",
                        "深色",
                        ChoiceState {
                            selected: dark,
                            disabled: false,
                        },
                        Message::Theme,
                    )
                    .accessible_activation(),
                toggle("divider-demo-rtl", "右起", self.rtl, ToggleDirection),
                toggle(
                    "divider-demo-large-text",
                    "大字",
                    self.large_text,
                    ToggleLargeText,
                ),
                toggle("divider-demo-weight", "粗線", self.heavy, ToggleWeight),
            ]),
            sample(
                "divider-demo-full-label",
                "全寬",
                horizontal("divider-demo-full", DividerKind::Full),
            ),
            sample(
                "divider-demo-inset-label",
                "單側內縮",
                horizontal("divider-demo-inset", DividerKind::Inset),
            ),
            sample(
                "divider-demo-middle-label",
                "兩側內縮",
                horizontal("divider-demo-middle", DividerKind::MiddleInset),
            ),
            sample("divider-demo-vertical-label", "垂直", vertical),
        ])
    }
}
