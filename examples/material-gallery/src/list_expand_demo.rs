use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChipKind, ChoiceState, ExpandableListOptions,
    ExpandableListRow, ListExpansionMotion, ListItem, ListLines, ListStyle, Theme,
};

use crate::{rail_icons::Destination, Message};

const SHOW: &str = "list-expand-demo-show";
const HEADER: &str = "list-expand-header";
const STYLES: [(ListStyle, &str, &str); 3] = [
    (ListStyle::Baseline, "baseline", "基準"),
    (ListStyle::Standard, "standard", "標準"),
    (ListStyle::Segmented, "segmented", "分段"),
];

#[derive(Clone)]
pub enum ListExpandEvent {
    Show,
    Back,
    Direction,
    LargeText,
    Style(ListStyle),
    Toggle,
    Collapse,
    Reset,
    Clear,
    Activate {
        id: &'static str,
        label: &'static str,
    },
}

pub(crate) struct ListExpandDemo {
    visible: bool,
    style: ListStyle,
    rtl: bool,
    large_text: bool,
    expanded: bool,
    motion: ListExpansionMotion,
    activations: usize,
    toggles: usize,
    last: Option<(&'static str, &'static str)>,
}

impl Default for ListExpandDemo {
    fn default() -> Self {
        Self {
            visible: false,
            style: ListStyle::Segmented,
            rtl: false,
            large_text: false,
            expanded: false,
            motion: ListExpansionMotion::new(false),
            activations: 0,
            toggles: 0,
            last: None,
        }
    }
}

impl ListExpandDemo {
    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "可展開清單",
                ButtonKind::Outlined,
                false,
                Message::ListExpand(ListExpandEvent::Show),
            )
            .accessible_activation()
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        if self.visible {
            self.visible = false;
            self.reset();
        }
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if self.visible {
            self.motion.advance(HEADER, self.expanded, frame)
        } else {
            AnimationUpdate::default()
        }
    }

    pub(crate) fn update(&mut self, event: ListExpandEvent) {
        match event {
            ListExpandEvent::Show => self.visible = true,
            ListExpandEvent::Back => self.leave(),
            ListExpandEvent::Direction => self.rtl = !self.rtl,
            ListExpandEvent::LargeText => self.large_text = !self.large_text,
            ListExpandEvent::Style(style) => self.style = style,
            ListExpandEvent::Toggle => {
                self.expanded = !self.expanded;
                self.toggles += 1;
            }
            ListExpandEvent::Collapse => self.expanded = false,
            ListExpandEvent::Reset => self.reset(),
            ListExpandEvent::Clear => self.clear_actions(),
            ListExpandEvent::Activate { id, label } => {
                self.activations += 1;
                self.last = Some((id, label));
            }
        }
    }

    fn clear_actions(&mut self) {
        self.activations = 0;
        self.last = None;
    }

    fn reset(&mut self) {
        self.expanded = false;
        self.motion = ListExpansionMotion::new(false);
        self.toggles = 0;
        self.clear_actions();
    }

    fn list(&self, theme: &Theme) -> View<Message> {
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let mut header = item(
            HEADER,
            "旅行行程",
            Some("向下查看步道、地圖與其他旅行資料"),
            false,
        );
        header.on_activate = Some(Message::ListExpand(ListExpandEvent::Toggle));
        theme.expandable_list(
            ExpandableListOptions {
                id: "list-expand-items",
                name: "旅行清單",
                style: self.style,
                direction,
            },
            [
                ExpandableListRow::Leaf(item("list-expand-before", "出發前", None, false)),
                ExpandableListRow::Branch {
                    header,
                    children: vec![
                        item("list-expand-child-a", "步道地圖與無障礙路線", None, false),
                        item("list-expand-child-disabled", "暫停行程", None, true),
                        item("list-expand-child-b", "下載地圖", None, false),
                    ],
                    expanded: self.expanded,
                    motion: &self.motion,
                },
                ExpandableListRow::Leaf(item("list-expand-after", "回程", None, false)),
                ExpandableListRow::Leaf(item("list-expand-end", "行程記錄", None, false)),
            ],
        )
    }

    pub(crate) fn view(&self, theme: &Theme) -> View<Message> {
        let viewport = View::new(
            Node::new(Role::Group, "可展開清單捲動區")
                .id("list-expand-scroll")
                .styled(Style {
                    height: Length::Px(0.),
                    min_height: 120.,
                    max_height: Length::Px(236.),
                    grow: 1.,
                    clip: true,
                    ..Style::column()
                }),
        )
        .accessibility_scroll_pages()
        .children([self.list(theme)]);
        let viewport = if self.expanded {
            // The model closes the branch while the child still has focus.
            viewport.on_cancel_within(Message::ListExpand(ListExpandEvent::Collapse))
        } else {
            viewport
        };
        let status = self.last.map_or_else(
            || "尚未操作".to_owned(),
            |(_, label)| format!("最近操作：{label}"),
        );
        let footer = View::new(
            Node::new(Role::Group, "操作紀錄").styled(Style::row().align(Align::Center).gap(8.)),
        )
        .children([
            theme
                .button(
                    "list-expand-demo-after",
                    "清除",
                    Message::ListExpand(ListExpandEvent::Clear),
                )
                .accessible_activation(),
            View::new(
                Node::new(Role::Status, "")
                    .id("list-expand-demo-status")
                    .text(status)
                    .shrink_to_fit_width()
                    .styled(
                        Style::column()
                            .width(Length::Px(0.))
                            .grow(1.)
                            .with_text_style(typography::BODY_MEDIUM)
                            .foreground(theme.colors.on_surface),
                    ),
            ),
        ]);
        View::new(
            Node::new(Role::Window, "可展開清單")
                .id("list-expand-demo")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .padding(12.)
                        .gap(8.)
                        .clip(true)
                        .background(theme.colors.surface_container),
                ),
        )
        .restore_focus_in_subtree_on_removal(SHOW)
        .accessibility_scroll_pages()
        .children([self.controls(theme), viewport, footer])
    }

    fn controls(&self, theme: &Theme) -> View<Message> {
        let toggle = |id: &str, label: &str, selected, event| {
            theme
                .chip(
                    id,
                    label,
                    ChipKind::Filter,
                    ChoiceState {
                        selected,
                        disabled: false,
                    },
                    Message::ListExpand(event),
                    None,
                )
                .accessible_activation()
        };
        let display = controls("畫面設定").children([
            theme
                .tonal_button(
                    "list-expand-demo-back",
                    "返回",
                    Message::ListExpand(ListExpandEvent::Back),
                )
                .accessible_activation(),
            theme
                .button("list-expand-demo-theme", "配色", Message::Theme)
                .accessible_activation(),
            toggle(
                "list-expand-demo-rtl",
                "右起",
                self.rtl,
                ListExpandEvent::Direction,
            ),
            toggle(
                "list-expand-demo-large-text",
                "大字",
                self.large_text,
                ListExpandEvent::LargeText,
            ),
        ]);
        let styles = STYLES.map(|(style, suffix, name)| {
            toggle(
                &format!("list-expand-demo-{suffix}"),
                name,
                self.style == style,
                ListExpandEvent::Style(style),
            )
        });
        let reset = theme
            .button(
                "list-expand-demo-before",
                "重設",
                Message::ListExpand(ListExpandEvent::Reset),
            )
            .accessible_activation();
        let presets = controls("清單樣式").children(styles.into_iter().chain([reset]));
        View::new(Node::new(Role::Group, "").styled(Style::column().gap(4.)))
            .children([display, presets])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        let variant = STYLES
            .iter()
            .find_map(|(style, name, _)| (*style == self.style).then_some(*name));
        let colors = theme.colors;
        serde_json::json!({
            "visible": self.visible,
            "variant": variant,
            "expanded": self.expanded,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "activations": self.activations,
            "toggles": self.toggles,
            "last": self.last.map(|(id, _)| id),
            "colors": {
                "surface": colors.surface,
                "surface_container": colors.surface_container,
                "on_surface": colors.on_surface,
                "secondary": colors.secondary,
                "outline_variant": colors.outline_variant,
            },
        })
    }
}

fn item(
    id: &'static str,
    headline: &'static str,
    supporting: Option<&'static str>,
    disabled: bool,
) -> ListItem<'static, Message> {
    ListItem {
        id,
        headline,
        overline: None,
        supporting,
        leading: Some(Destination::Home.icon()),
        trailing: None,
        lines: if supporting.is_some() {
            ListLines::Two
        } else {
            ListLines::One
        },
        disabled,
        on_activate: Some(Message::ListExpand(ListExpandEvent::Activate {
            id,
            label: headline,
        })),
    }
}

fn controls(name: &str) -> View<Message> {
    View::new(Node::new(Role::Group, name).styled(Style::row().gap(8.).wrap(true)))
}
