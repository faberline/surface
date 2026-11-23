use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, ChipKind, ChoiceState, ListItem, ListLines, ListMode, ListOptions,
    ListStyle, Theme,
};

use crate::{rail_icons::Destination, Message};

pub(crate) const SECTION: usize = 2;
const SHOW: &str = "lists-demo-show";
const LIST: &str = "lists-items";
const SCROLL: &str = "lists-scroll";
const ROWS: [(&str, &str); 6] = [
    ("lists-a", "山"),
    ("lists-b", "海"),
    ("lists-c", "河"),
    ("lists-d", "長文字會在窄畫面自動換行"),
    ("lists-e", "島"),
    ("lists-f", "森林"),
];
const STYLES: [(ListStyle, &str, &str); 3] = [
    (ListStyle::Baseline, "baseline", "基準"),
    (ListStyle::Standard, "standard", "標準"),
    (ListStyle::Segmented, "segmented", "分段"),
];
const MODES: [(ListsMode, &str, &str); 3] = [
    (ListsMode::Passive, "passive", "唯讀"),
    (ListsMode::Action, "action", "操作"),
    (ListsMode::Select, "select", "單選"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ListsMode {
    Passive,
    Action,
    Select,
}

#[derive(Clone)]
pub enum ListsEvent {
    Show,
    Back,
    Direction,
    LargeText,
    Style(ListStyle),
    Mode(ListsMode),
    Reorder,
    ToggleLast,
    Reset,
    Clear,
    Activate(&'static str),
}

pub(crate) struct ListsDemo {
    visible: bool,
    style: ListStyle,
    mode: ListsMode,
    rtl: bool,
    large_text: bool,
    reversed: bool,
    hide_last: bool,
    selected: Option<&'static str>,
    last: Option<&'static str>,
    activations: usize,
}

impl Default for ListsDemo {
    fn default() -> Self {
        Self {
            visible: false,
            style: ListStyle::Segmented,
            mode: ListsMode::Select,
            rtl: false,
            large_text: false,
            reversed: false,
            hide_last: false,
            selected: Some("lists-c"),
            last: None,
            activations: 0,
        }
    }
}

impl ListsDemo {
    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "清單",
                surface_material::ButtonKind::Outlined,
                false,
                Message::Lists(ListsEvent::Show),
            )
            .accessible_activation()
    }

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

    pub(crate) fn update(&mut self, event: ListsEvent) {
        match event {
            ListsEvent::Show => self.visible = true,
            ListsEvent::Back => self.leave(),
            ListsEvent::Direction => self.rtl = !self.rtl,
            ListsEvent::LargeText => self.large_text = !self.large_text,
            ListsEvent::Style(style) => self.style = style,
            ListsEvent::Mode(mode) => self.mode = mode,
            ListsEvent::Reorder => self.reversed = !self.reversed,
            ListsEvent::ToggleLast => {
                self.hide_last = !self.hide_last;
                if self.hide_last && self.selected == Some("lists-f") {
                    self.selected = None;
                }
            }
            ListsEvent::Reset => {
                self.selected = Some("lists-c");
                self.reversed = false;
                self.hide_last = false;
                self.last = None;
                self.activations = 0;
            }
            ListsEvent::Clear => {
                self.selected = None;
                self.last = None;
            }
            ListsEvent::Activate(id) => {
                self.last = Some(id);
                self.activations += 1;
                if self.mode == ListsMode::Select {
                    self.selected = Some(id);
                }
            }
        }
    }

    pub(crate) fn view(&self, theme: &Theme) -> View<Message> {
        let mode = match self.mode {
            ListsMode::Passive => ListMode::Passive,
            ListsMode::Action => ListMode::Action,
            ListsMode::Select => ListMode::SingleSelect {
                selected_id: self.selected,
            },
        };
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let options = ListOptions {
            id: LIST,
            name: "書籤",
            style: self.style,
            mode,
            direction,
        };
        let list = theme.list(options, self.rows().map(Self::item));
        let body = View::new(Node::new(Role::Group, "").id(SCROLL).styled(Style {
            height: Length::Px(0.),
            min_height: 140.,
            max_height: Length::Px(320.),
            grow: 1.,
            clip: true,
            ..Style::column()
        }))
        .accessibility_scroll_pages()
        .children([list]);
        let action = |id, label, event| {
            theme
                .button(id, label, Message::Lists(event))
                .accessible_activation()
        };
        let status = match self.mode {
            ListsMode::Passive => "書籤預覽".to_owned(),
            ListsMode::Action => format!("最近操作：{}", row_name(self.last)),
            ListsMode::Select => format!("已選取：{}", row_name(self.selected)),
        };
        let after = View::new(
            Node::new(Role::Group, "清單操作").styled(Style::row().align(Align::Center).gap(8.)),
        )
        .children([
            action("lists-demo-after", "清除", ListsEvent::Clear),
            View::new(
                Node::new(Role::Status, "")
                    .id("lists-demo-status")
                    .text(status)
                    .styled(
                        Style::column()
                            .width(Length::Px(0.))
                            .grow(1.)
                            .with_text_style(surface_material::typography::BODY_MEDIUM)
                            .foreground(theme.colors.on_surface),
                    ),
            ),
        ]);
        View::new(
            Node::new(Role::Window, "清單").id("lists-demo").styled(
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
        .children([
            self.controls(theme),
            control_row("清單管理").children([
                action("lists-demo-reorder", "倒序", ListsEvent::Reorder),
                action(
                    "lists-demo-remove",
                    if self.hide_last {
                        "還原列"
                    } else {
                        "移除列"
                    },
                    ListsEvent::ToggleLast,
                ),
                action("lists-demo-before", "重設", ListsEvent::Reset),
            ]),
            body,
            after,
        ])
    }

    fn rows(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        (0..ROWS.len())
            .map(|index| {
                ROWS[if self.reversed {
                    ROWS.len() - 1 - index
                } else {
                    index
                }]
            })
            .filter(|(id, _)| !self.hide_last || *id != "lists-f")
    }

    fn item((id, headline): (&'static str, &'static str)) -> ListItem<'static, Message> {
        let (lines, overline, supporting) = match id {
            "lists-c" => (ListLines::Two, None, Some("河岸的步道")),
            "lists-d" => (
                ListLines::Three,
                Some("步道路線"),
                Some("放大文字後仍可完整閱讀"),
            ),
            _ => (ListLines::One, None, None),
        };
        ListItem {
            id,
            headline,
            overline,
            supporting,
            leading: Some(Destination::Archive.icon()),
            trailing: (id == "lists-a").then_some("置頂"),
            lines,
            disabled: id == "lists-b",
            on_activate: Some(Message::Lists(ListsEvent::Activate(id))),
        }
    }

    fn controls(&self, theme: &Theme) -> View<Message> {
        let toggle = |id: &str, label: &str, selected, message| {
            theme
                .chip(
                    id,
                    label,
                    ChipKind::Filter,
                    ChoiceState {
                        selected,
                        disabled: false,
                    },
                    message,
                    None,
                )
                .accessible_activation()
        };
        let preferences = control_row("畫面設定").children([
            theme
                .tonal_button("lists-demo-back", "返回", Message::Lists(ListsEvent::Back))
                .accessible_activation(),
            theme
                .button("lists-demo-theme", "配色", Message::Theme)
                .accessible_activation(),
            toggle(
                "lists-demo-rtl",
                "右起",
                self.rtl,
                Message::Lists(ListsEvent::Direction),
            ),
            toggle(
                "lists-demo-large-text",
                "大字",
                self.large_text,
                Message::Lists(ListsEvent::LargeText),
            ),
        ]);
        let styles = control_row("清單樣式").children(STYLES.map(|(style, suffix, label)| {
            toggle(
                &format!("lists-demo-{suffix}"),
                label,
                self.style == style,
                Message::Lists(ListsEvent::Style(style)),
            )
        }));
        let modes = control_row("清單行為").children(MODES.map(|(mode, suffix, label)| {
            toggle(
                &format!("lists-demo-{suffix}"),
                label,
                self.mode == mode,
                Message::Lists(ListsEvent::Mode(mode)),
            )
        }));
        View::new(Node::new(Role::Group, "").styled(Style::column().gap(4.))).children([
            preferences,
            styles,
            modes,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        runtime: &surface_app::Runtime<crate::Gallery>,
        theme: &Theme,
    ) -> serde_json::Value {
        use serde_json::json;

        let items: Vec<_> = self
            .rows()
            .map(|(id, label)| {
                let geometry = runtime
                    .dom()
                    .by_id(id)
                    .and_then(|node| runtime.computed()?.nodes.get(node.index()));
                json!({
                    "id": id,
                    "label": label,
                    "bounds": geometry.map(|node| box_array(node.bounds)),
                    "clip": geometry.map(|node| box_array(node.clip)),
                })
            })
            .collect();
        let variant = STYLES
            .iter()
            .find(|(style, _, _)| *style == self.style)
            .map(|(_, suffix, _)| *suffix);
        let mode = MODES
            .iter()
            .find(|(mode, _, _)| *mode == self.mode)
            .map(|(_, suffix, _)| *suffix);
        json!({
            "visible": self.visible,
            "variant": variant,
            "mode": mode,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "selected": if self.mode == ListsMode::Select { self.selected } else { None },
            "activations": self.activations,
            "last": self.last,
            "items": items,
            "colors": {
                "surface": theme.colors.surface,
                "surface_container": theme.colors.surface_container,
                "secondary": theme.colors.secondary,
                "secondary_container": theme.colors.secondary_container,
                "on_surface": theme.colors.on_surface,
                "on_surface_variant": theme.colors.on_surface_variant,
                "on_secondary_container": theme.colors.on_secondary_container,
            },
        })
    }
}

fn control_row(name: &str) -> View<Message> {
    View::new(Node::new(Role::Group, name).styled(Style::row().gap(8.).wrap(true)))
}

fn row_name(id: Option<&str>) -> &str {
    ROWS.iter()
        .find(|(key, _)| Some(*key) == id)
        .map_or("無", |(_, name)| *name)
}

#[cfg(target_arch = "wasm32")]
fn box_array(rect: surface::Rect) -> [f32; 4] {
    [rect.x, rect.y, rect.width, rect.height]
}
