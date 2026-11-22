use surface::{Align, Length, Node, Role, Style, TextScaling};
use surface_app::{AnimationFrame, AnimationUpdate, HorizontalDrag, View};
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChoiceState, ModalNavigationDrawerOptions,
    NavigationDrawerItem, NavigationDrawerSwipe, StandardNavigationDrawerMode,
    StandardNavigationDrawerOptions, Theme,
};

use crate::{rail_icons::Destination, Message};

pub(crate) const SECTION: usize = 2;
const MODAL: &str = "drawer-demo";
const STANDARD: &str = "drawer-standard";
const STANDARD_TOGGLE: &str = "drawer-demo-standard-toggle";

struct Page {
    key: &'static str,
    label: &'static str,
    icon: Destination,
    disabled: bool,
}

const PAGES: [Page; 6] = [
    Page {
        key: "home",
        label: "首頁",
        icon: Destination::Home,
        disabled: false,
    },
    Page {
        key: "archive",
        label: "封存",
        icon: Destination::Archive,
        disabled: true,
    },
    Page {
        key: "inbox",
        label: "收件匣",
        icon: Destination::Inbox,
        disabled: false,
    },
    Page {
        key: "reports",
        label: "跨部門協作報告與專案進度總覽",
        icon: Destination::Archive,
        disabled: false,
    },
    Page {
        key: "members",
        label: "團隊成員",
        icon: Destination::Inbox,
        disabled: false,
    },
    Page {
        key: "settings",
        label: "設定",
        icon: Destination::Home,
        disabled: false,
    },
];

#[derive(Clone, Copy)]
pub enum DrawerDemoEvent {
    Show,
    Back,
    Open,
    Direction,
    TextScale,
    Select(usize),
    Dismiss,
    Refresh,
    Swipe(HorizontalDrag),
    StandardPermanent,
    StandardDismissible,
    StandardToggle,
    StandardSelect(usize),
    RefreshShared,
}

#[derive(Default)]
pub(crate) struct DrawerDemo {
    visible: bool,
    open: bool,
    swipe: NavigationDrawerSwipe,
    rtl: bool,
    large_text: bool,
    selected: usize,
    actions: usize,
    dismissals: usize,
    background_actions: usize,
    standard_dismissible: bool,
    standard_open: bool,
    standard_selected: usize,
    standard_actions: usize,
    shared_clicks: usize,
}

impl DrawerDemo {
    pub(crate) fn update(&mut self, event: DrawerDemoEvent) {
        match event {
            DrawerDemoEvent::Show => self.visible = true,
            DrawerDemoEvent::Back => self.leave(),
            DrawerDemoEvent::Open => {
                self.swipe.reset();
                self.open = true;
            }
            DrawerDemoEvent::Direction => {
                self.swipe.reset();
                self.rtl = !self.rtl;
            }
            DrawerDemoEvent::TextScale => self.large_text = !self.large_text,
            DrawerDemoEvent::Select(index) if self.open && Self::page_is_enabled(index) => {
                self.selected = index;
                self.actions += 1;
                self.close();
            }
            DrawerDemoEvent::Dismiss if self.open => {
                self.dismissals += 1;
                self.close();
            }
            DrawerDemoEvent::Refresh => self.background_actions += 1,
            DrawerDemoEvent::Swipe(drag) => self.swipe.apply(drag, self.open, self.direction()),
            DrawerDemoEvent::StandardPermanent => self.standard_dismissible = false,
            DrawerDemoEvent::StandardDismissible => {
                if !self.standard_dismissible {
                    self.standard_open = true;
                }
                self.standard_dismissible = true;
            }
            DrawerDemoEvent::StandardToggle if self.standard_dismissible => {
                self.standard_open = !self.standard_open;
            }
            DrawerDemoEvent::StandardSelect(index)
                if self.standard_visible() && Self::page_is_enabled(index) =>
            {
                self.standard_selected = index;
                self.standard_actions += 1;
            }
            DrawerDemoEvent::RefreshShared => self.shared_clicks += 1,
            _ => {}
        }
    }

    fn page_is_enabled(index: usize) -> bool {
        PAGES.get(index).is_some_and(|page| !page.disabled)
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.close();
    }

    fn close(&mut self) {
        self.swipe.reset();
        self.open = false;
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        self.swipe.set_viewport(width, height)
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        let (update, open) = self.swipe.advance(frame.now_ms, frame.reduced_motion);
        if let Some(open) = open {
            if self.open && !open {
                self.dismissals += 1;
            }
            self.open = open;
        }
        update
    }

    pub(crate) fn show_button(&self, theme: &Theme) -> View<Message> {
        theme
            .button_with(
                "drawer-demo-show",
                "導覽抽屜",
                ButtonKind::Outlined,
                false,
                Message::DrawerDemo(DrawerDemoEvent::Show),
            )
            .restore_focus_on_removal("drawer-demo-back")
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let controls = theme.row([
            theme
                .tonal_button(
                    "drawer-demo-back",
                    "返回",
                    Message::DrawerDemo(DrawerDemoEvent::Back),
                )
                .restore_focus_on_removal("drawer-demo-show")
                .accessible_activation(),
            theme.switch("theme", "深色", Self::choice(dark), Message::Theme),
            theme
                .segmented_button(
                    "drawer-demo-rtl",
                    "右至左",
                    Self::choice(self.rtl),
                    Message::DrawerDemo(DrawerDemoEvent::Direction),
                )
                .accessible_activation(),
            theme
                .segmented_button(
                    "drawer-demo-scale",
                    "大字",
                    Self::choice(self.large_text),
                    Message::DrawerDemo(DrawerDemoEvent::TextScale),
                )
                .accessible_activation(),
        ]);
        let modal_controls = View::new(
            Node::new(Role::Group, "模態抽屜").styled(
                Style::row()
                    .gap(8.)
                    .wrap(true)
                    .align(Align::Center)
                    .justify(Align::Center),
            ),
        )
        .children([
            theme
                .tonal_button(
                    "drawer-demo-open",
                    "模態抽屜",
                    Message::DrawerDemo(DrawerDemoEvent::Open),
                )
                .disclosure_expanded(self.open)
                .accessible_activation(),
            theme
                .button_with(
                    "drawer-demo-background",
                    "重新整理",
                    ButtonKind::Text,
                    false,
                    Message::DrawerDemo(DrawerDemoEvent::Refresh),
                )
                .accessible_activation(),
            theme.text("drawer-demo-page", PAGES[self.selected].label),
        ]);

        let mode_controls = theme.row([
            theme
                .segmented_button(
                    "drawer-demo-permanent",
                    "常駐",
                    Self::choice(!self.standard_dismissible),
                    Message::DrawerDemo(DrawerDemoEvent::StandardPermanent),
                )
                .accessible_activation(),
            theme
                .segmented_button(
                    "drawer-demo-dismissible",
                    "可收合",
                    Self::choice(self.standard_dismissible),
                    Message::DrawerDemo(DrawerDemoEvent::StandardDismissible),
                )
                .accessible_activation(),
            theme
                .button_with(
                    STANDARD_TOGGLE,
                    "抽屜",
                    ButtonKind::Text,
                    !self.standard_dismissible,
                    Message::DrawerDemo(DrawerDemoEvent::StandardToggle),
                )
                .disclosure_expanded(self.standard_visible())
                .accessible_activation(),
        ]);

        let mut controls_style = Style::column().gap(8.).clip(true);
        controls_style.max_height = Length::Percent(0.55);
        let preview_controls = View::new(
            Node::new(Role::Group, "抽屜控制")
                .id("drawer-demo-controls")
                .styled(controls_style),
        )
        .children([controls, modal_controls, mode_controls])
        .accessibility_scroll_pages();
        View::new(
            Node::new(Role::Window, "導覽抽屜")
                .id("surface-page")
                .styled(
                    Style::column()
                        .padding(12.)
                        .gap(8.)
                        .clip(true)
                        .background(theme.colors.surface_container)
                        .foreground(theme.colors.on_surface),
                ),
        )
        .children(
            [
                preview_controls,
                View::new(
                    Node::new(Role::Group, "")
                        .styled(Style::column().height(Length::Px(0.)).grow(1.).clip(true)),
                )
                .children([self.standard_layout(theme)]),
            ]
            .into_iter()
            .chain(self.modal_layers(theme)),
        )
    }

    fn choice(selected: bool) -> ChoiceState {
        ChoiceState {
            selected,
            disabled: false,
        }
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn standard_visible(&self) -> bool {
        !self.standard_dismissible || self.standard_open
    }

    fn standard_layout(&self, theme: &Theme) -> View<Message> {
        let direction = self.direction();
        let mode = if self.standard_dismissible {
            StandardNavigationDrawerMode::Dismissible {
                open: self.standard_open,
                restore_focus_to: STANDARD_TOGGLE,
            }
        } else {
            StandardNavigationDrawerMode::Permanent
        };
        let active_label = PAGES[self.standard_selected].label;
        let content =
            View::new(Node::new(Role::Group, "內容").styled(Style::column().padding(8.).gap(8.)))
                .children([
                    theme.row([theme
                        .tonal_button(
                            "drawer-demo-shared",
                            "更新",
                            Message::DrawerDemo(DrawerDemoEvent::RefreshShared),
                        )
                        .accessible_activation()]),
                    View::new(
                        Node::new(Role::Text, active_label)
                            .id("drawer-demo-shared-page")
                            .text(active_label)
                            .text_scaling(TextScaling::Expand)
                            .shrink_to_fit_width()
                            .styled(
                                Style::default()
                                    .with_text_style(typography::BODY_LARGE)
                                    .foreground(theme.colors.on_surface),
                            ),
                    ),
                ]);
        theme.standard_navigation_drawer_layout(
            StandardNavigationDrawerOptions {
                id: STANDARD,
                navigation_name: "工作區",
                headline: Some("工作區"),
                direction,
            },
            mode,
            self.destinations(
                theme,
                STANDARD,
                self.standard_selected,
                DrawerDemoEvent::StandardSelect,
            ),
            content,
        )
    }

    fn modal_layers<'a>(&'a self, theme: &'a Theme) -> impl Iterator<Item = View<Message>> + 'a {
        theme.swipeable_modal_navigation_drawer(
            ModalNavigationDrawerOptions {
                id: MODAL,
                name: "導覽抽屜",
                navigation_name: "工作區",
                close_name: "關閉抽屜",
                direction: self.direction(),
            },
            self.destinations(theme, MODAL, self.selected, DrawerDemoEvent::Select),
            Message::DrawerDemo(DrawerDemoEvent::Dismiss),
            &self.swipe,
            self.open,
            |drag| Message::DrawerDemo(DrawerDemoEvent::Swipe(drag)),
        )
    }

    fn destinations<'a>(
        &'a self,
        theme: &'a Theme,
        id: &'a str,
        selected: usize,
        select: fn(usize) -> DrawerDemoEvent,
    ) -> impl Iterator<Item = View<Message>> + 'a {
        PAGES.iter().enumerate().flat_map(move |(index, page)| {
            let section = (page.key == "reports").then(|| {
                [
                    theme.divider(&format!("{id}-reports-divider")),
                    theme.navigation_drawer_section_heading("團隊", self.direction()),
                ]
            });
            let destination = theme.navigation_drawer_item_with_icon(NavigationDrawerItem {
                id: &format!("{id}-{}", page.key),
                label: page.label,
                accessible_name: page.label,
                icon: page.icon.icon(),
                direction: self.direction(),
                state: ChoiceState {
                    selected: selected == index,
                    disabled: page.disabled,
                },
                message: Message::DrawerDemo(select(index)),
            });
            section.into_iter().flatten().chain([destination])
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        text_scale: f32,
        runtime: &surface_app::Runtime<crate::Gallery>,
    ) -> serde_json::Value {
        serde_json::json!({
            "open": self.open,
            "progress": self.swipe.progress(self.open),
            "settling": self.swipe.is_settling(),
            "rtl": self.rtl,
            "selected": PAGES[self.selected].key,
            "actions": self.actions,
            "dismissals": self.dismissals,
            "background_actions": self.background_actions,
            "text_scale": text_scale,
            "scroll": runtime.scroll_metrics("drawer-demo-navigation").map(|metrics| metrics.offset.1),
            "standard_mode": if self.standard_dismissible { "dismissible" } else { "permanent" },
            "standard_open": self.standard_visible(),
            "standard_selected": PAGES[self.standard_selected].key,
            "standard_actions": self.standard_actions,
            "shared_clicks": self.shared_clicks,
        })
    }
}
