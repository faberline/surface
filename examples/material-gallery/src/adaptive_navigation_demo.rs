use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, typography, AdaptiveNavigationScaffoldOptions, ButtonKind, ChoiceState,
    NavigationRailItem, TextFieldOptions, Theme,
};

use crate::{flexible_navigation_bar_demo::icon, scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 3;
const SHOW: &str = "adaptive-navigation-demo-show";
const BACK: &str = "adaptive-navigation-demo-back";
const SCAFFOLD: &str = "adaptive-navigation-demo-scaffold";
const NAVIGATION: &str = "adaptive-navigation-demo-bar";
const PAGE: &str = "adaptive-navigation-demo-page";
const EDITOR: &str = "adaptive-navigation-demo-draft";
const IDS: [&str; 5] = [
    "adaptive-navigation-demo-home",
    "adaptive-navigation-demo-explore",
    "adaptive-navigation-demo-saved",
    "adaptive-navigation-demo-updates",
    "adaptive-navigation-demo-settings",
];
const LABELS: [&str; 5] = ["首頁", "探索路線", "收藏", "最新消息", "設定"];
const CONTENT: [[&str; 3]; 5] = [
    ["今日行程", "河畔散步", "閱讀清單"],
    ["城市散步", "山徑", "河岸路線"],
    ["河岸步道", "城市圖書館", "公園地圖"],
    ["步道已開放", "新展覽", "週末活動"],
    ["通知", "語言", "隱私"],
];
const DISABLED: usize = 1;

#[derive(Clone)]
pub enum AdaptiveNavigationDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    Choose(usize),
    Edit(String),
}

pub(crate) struct AdaptiveNavigationDemo {
    visible: bool,
    viewport: (f32, f32),
    rtl: bool,
    large_text: bool,
    selected: usize,
    activations: usize,
    draft: String,
}

impl Default for AdaptiveNavigationDemo {
    fn default() -> Self {
        Self {
            visible: false,
            viewport: (960., 640.),
            rtl: false,
            large_text: false,
            selected: 0,
            activations: 0,
            draft: String::new(),
        }
    }
}

impl AdaptiveNavigationDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let previous = std::mem::replace(&mut self.viewport, (width, height));
        self.visible && previous != self.viewport
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn update(&mut self, event: AdaptiveNavigationDemoEvent) {
        use AdaptiveNavigationDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            Choose(index) if index < IDS.len() && index != DISABLED => {
                self.selected = index;
                self.activations += 1;
            }
            Choose(_) => {}
            Edit(value) => self.draft = value,
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "自適應導覽",
                ButtonKind::Outlined,
                false,
                Message::AdaptiveNavigationDemo(AdaptiveNavigationDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use AdaptiveNavigationDemoEvent::*;
        let event = Message::AdaptiveNavigationDemo;
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let controls = control_row(vec![
            theme
                .tonal_button(BACK, "返回", event(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme
                .segmented_button(
                    "adaptive-navigation-demo-theme",
                    "深色",
                    choice(dark),
                    Message::Theme,
                )
                .accessible_activation(),
            theme
                .segmented_button(
                    "adaptive-navigation-demo-rtl",
                    "右起",
                    choice(self.rtl),
                    event(ToggleDirection),
                )
                .accessible_activation(),
            theme
                .segmented_button(
                    "adaptive-navigation-demo-large",
                    "大字",
                    choice(self.large_text),
                    event(ToggleLargeText),
                )
                .accessible_activation(),
        ]);
        let title = LABELS[self.selected];
        let heading = View::new(
            Node::new(Role::Heading, title)
                .id("adaptive-navigation-demo-title")
                .text(title)
                .styled(
                    Style::row()
                        .with_text_style(typography::TITLE_LARGE)
                        .foreground(theme.on_background),
                ),
        );
        let mut field = TextFieldOptions::new(EDITOR, "草稿", &self.draft);
        field.direction = direction;
        field.text_scale = self.text_scale();
        let editor = theme.outlined_text_field(field, move |text| event(Edit(text)));
        let content = View::new(Node::new(Role::Group, "").styled(Style::column().gap(8.)))
            .children(
                CONTENT[self.selected]
                    .into_iter()
                    .enumerate()
                    .map(|(index, title)| {
                        theme.text(&format!("adaptive-navigation-demo-line-{index}"), title)
                    }),
            );
        let page = View::new(
            Node::new(Role::Group, "頁面內容").id(PAGE).styled(
                Style::column()
                    .width(Length::Percent(1.))
                    .height(Length::Percent(1.))
                    .align(Align::Stretch)
                    .padding(16.)
                    .gap(12.)
                    .clip(true),
            ),
        )
        .accessibility_scroll_pages()
        .children(
            [controls, heading]
                .into_iter()
                .chain(editor)
                .chain([content]),
        );
        let options = AdaptiveNavigationScaffoldOptions {
            id: SCAFFOLD,
            navigation_id: NAVIGATION,
            name: "主要導覽",
            available_width: self.viewport.0,
            available_height: self.viewport.1,
            direction,
        };
        let items = IDS
            .into_iter()
            .enumerate()
            .map(|(index, id)| NavigationRailItem {
                id,
                label: LABELS[index],
                accessible_name: LABELS[index],
                icon: icon(index, self.selected == index),
                state: ChoiceState {
                    selected: self.selected == index,
                    disabled: index == DISABLED,
                },
                message: event(Choose(index)),
            });
        let scaffold = theme.adaptive_navigation_scaffold(options, page, items);
        View::new(
            Node::new(Role::Window, "自適應導覽")
                .id("adaptive-navigation-demo")
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.))
                        .background(theme.background)
                        .foreground(theme.on_background)
                        .clip(true),
                ),
        )
        .children(scaffold)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        serde_json::json!({
            "visible": self.visible,
            "viewport": self.viewport,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "selected": IDS[self.selected],
            "activations": self.activations,
            "draft": self.draft,
            "palette": {
                "bar": theme.colors.surface_container,
                "rail": theme.colors.surface,
                "selected": theme.colors.secondary_container,
            },
        })
    }
}

fn choice(selected: bool) -> ChoiceState {
    ChoiceState {
        selected,
        disabled: false,
    }
}
