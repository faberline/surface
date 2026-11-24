use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, BadgeBoundary, BadgeContent, BadgedAnchorOptions,
    BadgedNavigationBarItem, ChoiceState, IconButtonKind, NavigationBarIcon, Theme,
};

use crate::{rail_icons::Destination, Message};

pub(crate) const SECTION: usize = 3;
const INBOX: &str = "badge-inbox";
const SCROLL_OWNER: &str = "badge-scroll-owner";

#[derive(Clone, Copy, Default)]
pub enum BadgeMode {
    Hidden,
    #[default]
    Dot,
    Count,
    Label,
}

impl BadgeMode {
    #[cfg(target_arch = "wasm32")]
    fn key(self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Dot => "dot",
            Self::Count => "count",
            Self::Label => "label",
        }
    }

    fn content(self, count: u32) -> BadgeContent<'static> {
        match self {
            Self::Hidden => BadgeContent::Hidden,
            Self::Dot => BadgeContent::Dot,
            Self::Count => BadgeContent::Count(count),
            Self::Label => BadgeContent::Label("新"),
        }
    }
}

#[derive(Clone, Copy)]
pub enum BadgeEvent {
    Show(BadgeMode),
    Direction,
    Scale,
    PressInbox,
    PressScrollOwner,
    SelectDestination(usize),
}

pub(crate) struct BadgeDemo {
    mode: BadgeMode,
    count: u32,
    presses: u32,
    scroll_presses: u32,
    selected: bool,
    nav: usize,
    rtl: bool,
    large_text: bool,
}

impl Default for BadgeDemo {
    fn default() -> Self {
        Self {
            mode: BadgeMode::Dot,
            count: 999,
            presses: 0,
            scroll_presses: 0,
            selected: false,
            nav: 0,
            rtl: false,
            large_text: false,
        }
    }
}

impl BadgeDemo {
    pub(crate) fn update(&mut self, event: BadgeEvent) {
        match event {
            BadgeEvent::Show(mode) => self.mode = mode,
            BadgeEvent::Direction => self.rtl = !self.rtl,
            BadgeEvent::Scale => self.large_text = !self.large_text,
            BadgeEvent::PressInbox => {
                self.count = self.count.saturating_add(1);
                self.presses = self.presses.saturating_add(1);
                self.selected = !self.selected;
            }
            BadgeEvent::PressScrollOwner => {
                self.scroll_presses = self.scroll_presses.saturating_add(1);
            }
            BadgeEvent::SelectDestination(index) => self.nav = index,
        }
    }

    pub(crate) fn large_text(&self) -> bool {
        self.large_text
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn options(&self) -> BadgedAnchorOptions {
        BadgedAnchorOptions {
            direction: self.direction(),
            ..Default::default()
        }
    }

    fn inbox(&self, theme: &Theme) -> View<Message> {
        let name = match self.mode {
            BadgeMode::Hidden => "收件匣".to_owned(),
            BadgeMode::Dot | BadgeMode::Label => "收件匣，有新訊息".to_owned(),
            BadgeMode::Count => format!("收件匣，{} 則未讀", self.count),
        };
        let button = theme
            .icon_button(
                INBOX,
                &name,
                "信",
                IconButtonKind::Tonal,
                Some(self.selected),
                false,
                Message::Badge(BadgeEvent::PressInbox),
            )
            .accessible_activation();
        View::new(
            Node::new(Role::Group, "")
                .id("badge-anchor-surface")
                .styled(
                    Style::row()
                        .height(Length::Px(84.))
                        .align(Align::Center)
                        .justify(Align::Center),
                ),
        )
        .children([theme.badged_anchor(
            button,
            self.mode.content(self.count),
            self.options(),
        )])
    }

    fn navigation(&self, theme: &Theme) -> View<Message> {
        let inbox = format!("收件匣，{} 則未讀", self.count);
        let items = [
            (
                "badge-nav-0",
                "首頁",
                "首頁",
                Destination::Home,
                BadgeContent::Hidden,
            ),
            (
                "badge-nav-1",
                "收件匣",
                inbox.as_str(),
                Destination::Inbox,
                BadgeContent::Count(self.count),
            ),
            (
                "badge-nav-2",
                "封存",
                "封存，有新訊息",
                Destination::Archive,
                BadgeContent::Dot,
            ),
        ];
        theme.navigation_bar_with_direction(
            "badge-navigation",
            "徽章導覽",
            self.direction(),
            items.into_iter().enumerate().map(
                |(index, (id, label, accessible_name, icon, badge))| {
                    theme.navigation_bar_item_with_badge_clearance(
                        BadgedNavigationBarItem {
                            id,
                            label,
                            accessible_name,
                            icon: NavigationBarIcon::Vector(icon.icon()),
                            badge,
                            state: ChoiceState {
                                selected: self.nav == index,
                                disabled: index == 2,
                            },
                            direction: self.direction(),
                            message: Message::Badge(BadgeEvent::SelectDestination(index)),
                        },
                        BadgeBoundary::Owner,
                    )
                },
            ),
        )
    }

    fn scroll_fixture(&self, theme: &Theme) -> View<Message> {
        let owner = theme
            .icon_button(
                SCROLL_OWNER,
                "收件匣，1200 則未讀",
                "信",
                IconButtonKind::Tonal,
                None,
                false,
                Message::Badge(BadgeEvent::PressScrollOwner),
            )
            .accessible_activation()
            .accessibility_scroll_into_view();
        let owner = theme.badged_anchor_with_clearance(
            owner,
            SCROLL_OWNER,
            BadgeContent::Count(1200),
            self.options(),
            BadgeBoundary::Clip,
        );
        let mut edge = Style::row().height(Length::Px(48.)).justify(if self.rtl {
            Align::Start
        } else {
            Align::End
        });
        edge.min_height = 48.;
        let edge = View::new(Node::new(Role::Group, "").styled(edge)).children([owner]);
        let mut content = Style::column().padding(16.).gap(24.);
        content.min_height = 220.;
        let content = View::new(Node::new(Role::Group, "").styled(content)).children([
            theme.text("badge-scroll-recent", "近期訊息"),
            theme.text("badge-scroll-older", "較早的訊息"),
        ]);
        let mut viewport = Style::column()
            .height(Length::Px(112.))
            .clip(true)
            .background(theme.colors.surface_container_high);
        viewport.min_height = 112.;
        View::new(
            Node::new(Role::Group, "訊息清單")
                .id("badge-scroll")
                .styled(viewport),
        )
        .children([edge, content])
        .accessibility_scroll_pages()
    }

    pub(crate) fn card(&self, theme: &Theme) -> View<Message> {
        let change = |id, label, event| {
            theme
                .tonal_button(id, label, Message::Badge(event))
                .accessible_activation()
        };
        theme.card(
            "badge-card",
            [
                theme.text("badge-heading", "徽章"),
                theme.row([
                    theme.badge("badge-standalone-dot", "有新訊息", None),
                    theme.badge("badge-standalone-label", "有新內容", Some("新")),
                ]),
                theme.row([
                    change("badge-show-dot", "圓點", BadgeEvent::Show(BadgeMode::Dot)),
                    change(
                        "badge-show-count",
                        "數量",
                        BadgeEvent::Show(BadgeMode::Count),
                    ),
                    change(
                        "badge-show-label",
                        "短標籤",
                        BadgeEvent::Show(BadgeMode::Label),
                    ),
                    change(
                        "badge-show-hidden",
                        "隱藏",
                        BadgeEvent::Show(BadgeMode::Hidden),
                    ),
                ]),
                theme.row([
                    change(
                        "badge-direction",
                        if self.rtl {
                            "由左至右"
                        } else {
                            "由右至左"
                        },
                        BadgeEvent::Direction,
                    ),
                    change(
                        "badge-scale",
                        if self.large_text {
                            "一般文字"
                        } else {
                            "放大文字"
                        },
                        BadgeEvent::Scale,
                    ),
                ]),
                self.inbox(theme),
                self.navigation(theme),
                self.scroll_fixture(theme),
            ],
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, text_scale: f32, theme: &Theme) -> serde_json::Value {
        serde_json::json!({
            "kind": self.mode.key(),
            "count": self.count,
            "presses": self.presses,
            "scroll_presses": self.scroll_presses,
            "selected": self.selected,
            "nav": self.nav,
            "rtl": self.rtl,
            "text_scale": text_scale,
            "error": theme.colors.error,
            "on_error": theme.colors.on_error,
        })
    }
}
