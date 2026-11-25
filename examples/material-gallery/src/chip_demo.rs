use surface::{Align, Length, Node, RasterImage, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChipContent, ChipGridLayout, ChipLeading, ChipOptions,
    ChipRemove, ChipStyle, ChipTrailing, ChoiceState, CommandMenuEntry, CommandMenuItem,
    CommandMenuMeasurement, CommandMenuOptions, Theme,
};

use crate::{rail_icons, scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "chip-demo-show";
const BACK: &str = "chip-demo-back";
const MENU: &str = "chip-demo-menu-panel";
const FILTER_MENU: &str = "chip-demo-filter-type";
const FILTER_MENU_WIDE: &str = "chip-demo-filter-type-menu";
const ACTIONS: [&str; 5] = [
    "chip-demo-assist",
    "chip-demo-assist-elevated",
    "chip-demo-suggest",
    "chip-demo-suggest-elevated",
    "chip-demo-disabled",
];
const ACTION_TEXT: [&str; 5] = ["排程", "查看行程", "今天", "建議下一步", "停用"];
const FILTERS: [&str; 3] = ["chip-demo-news", "chip-demo-images", FILTER_MENU];
const FILTER_TEXT: [&str; 3] = ["新聞", "圖片", "依檔案類型篩選"];
const CONTACTS: [&str; 3] = ["chip-demo-chen", "chip-demo-mei", "chip-demo-lin"];
const CONTACT_NAMES: [&str; 3] = ["小陳", "小美", "林小姐的工作用電子郵件"];
const REMOVE_NAMES: [&str; 3] = ["移除小陳", "移除小美", "移除林小姐"];

#[derive(Clone)]
pub enum ChipDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    ToggleSingle,
    Action(&'static str),
    Filter(usize),
    Edit(usize),
    Remove(usize),
    ToggleMenu,
    DismissMenu,
    ChooseType,
}

pub(crate) struct ChipDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    multi: bool,
    menu_open: bool,
    width: f32,
    selected: [bool; 3],
    removed: [bool; 3],
    actions: usize,
    edits: usize,
    removals: usize,
    last: Option<&'static str>,
    layouts: [ChipGridLayout; 3],
    menu: CommandMenuMeasurement,
    avatar: RasterImage,
}

impl Default for ChipDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            multi: true,
            menu_open: false,
            width: 720.,
            selected: [true, false, false],
            removed: [false; 3],
            actions: 0,
            edits: 0,
            removals: 0,
            last: None,
            layouts: std::array::from_fn(|_| ChipGridLayout::default()),
            menu: CommandMenuMeasurement::default(),
            avatar: avatar(),
        }
    }
}

fn avatar() -> RasterImage {
    let mut pixels = Vec::with_capacity(24 * 24 * 4);
    for y in 0_i32..24 {
        for x in 0_i32..24 {
            let head = (x - 12).pow(2) + (y - 8).pow(2) <= 12;
            let shoulders = y >= 13 && (x - 12).pow(2) + (y - 22).pow(2) <= 87;
            pixels.extend_from_slice(if head || shoulders {
                &[247, 211, 150, 255]
            } else {
                &[67, 89, 135, 255]
            });
        }
    }
    RasterImage::new(24, 24, pixels).expect("fixed Gallery avatar")
}

impl ChipDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.menu_open = false;
        self.layouts = std::array::from_fn(|_| ChipGridLayout::default());
        self.menu = CommandMenuMeasurement::default();
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        let inner = (width - 44.).clamp(48., 720.);
        let changed = self.width != inner;
        self.width = inner;
        changed && self.visible
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn compact(&self) -> bool {
        self.width < 400.
    }

    fn trigger(&self) -> &str {
        if self.compact() {
            FILTER_MENU
        } else {
            FILTER_MENU_WIDE
        }
    }

    fn contacts(&self) -> Vec<usize> {
        (0..CONTACTS.len())
            .filter(|&index| !self.removed[index])
            .collect()
    }

    fn filter(&mut self, index: usize) {
        if self.multi {
            self.selected[index] = !self.selected[index];
        } else {
            self.selected = std::array::from_fn(|candidate| candidate == index);
        }
    }

    pub(crate) fn update(&mut self, message: ChipDemoEvent) {
        use ChipDemoEvent::*;
        match message {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            ToggleSingle => {
                self.multi = !self.multi;
                if !self.multi {
                    let selected = self.selected.iter().position(|&value| value).unwrap_or(0);
                    self.filter(selected);
                }
            }
            Action(id) => {
                self.actions += 1;
                self.last = Some(id);
            }
            Filter(index) => self.filter(index),
            Edit(index) => {
                self.edits += 1;
                self.last = Some(CONTACTS[index]);
            }
            Remove(index) => {
                if !self.removed[index] {
                    self.removed[index] = true;
                    self.removals += 1;
                    self.last = Some(CONTACTS[index]);
                }
            }
            ToggleMenu => self.menu_open = !self.menu_open,
            DismissMenu => self.menu_open = false,
            ChooseType => {
                self.filter(2);
                self.menu_open = false;
            }
        }
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if !self.visible {
            return AnimationUpdate::default();
        }
        let trigger = if self.compact() {
            FILTER_MENU
        } else {
            FILTER_MENU_WIDE
        };
        let ids: Vec<_> = self
            .contacts()
            .iter()
            .map(|&index| CONTACTS[index])
            .collect();
        let scale = self.text_scale();
        let mut update = self.menu.advance(MENU, trigger, frame);
        for (layout, ids) in self
            .layouts
            .iter_mut()
            .zip([&ACTIONS[..], &FILTERS[..], &ids[..]])
        {
            let next = layout.advance(frame, self.width, scale, ids);
            update.rebuild |= next.rebuild;
            update.request_next |= next.request_next;
        }
        update
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "標籤範例",
                ButtonKind::Outlined,
                false,
                Message::ChipDemo(ChipDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn options(
        &self,
        id: &'static str,
        label: &'static str,
        disabled: bool,
    ) -> ChipOptions<'static> {
        ChipOptions {
            id,
            label,
            accessible_name: None,
            direction: self.direction(),
            text_scale: self.text_scale(),
            max_width: self.width,
            disabled,
        }
    }

    fn action(&self, theme: &Theme, index: usize) -> Option<View<Message>> {
        let message = Message::ChipDemo(ChipDemoEvent::Action(ACTIONS[index]));
        let style = if matches!(index, 1 | 3) {
            ChipStyle::Elevated
        } else {
            ChipStyle::Flat
        };
        let content = if index < 2 {
            ChipContent::Assist {
                style,
                leading: Some(if index == 0 {
                    ChipLeading::Icon(rail_icons::search())
                } else {
                    ChipLeading::Image(&self.avatar)
                }),
                message,
            }
        } else {
            ChipContent::Suggestion {
                style,
                leading: (index == 3).then(|| ChipLeading::Icon(rail_icons::search())),
                message,
            }
        };
        theme.chip_in_grid(
            self.options(ACTIONS[index], ACTION_TEXT[index], index == 4),
            content,
        )
    }

    fn filter_chip(&self, theme: &Theme, index: usize) -> Option<View<Message>> {
        use ChipDemoEvent::*;
        let compact_menu = index == 2 && self.compact();
        let trailing = if index != 2 {
            None
        } else if compact_menu {
            Some(ChipTrailing::MenuOnBody {
                expanded: self.menu_open,
            })
        } else {
            Some(ChipTrailing::Menu {
                accessible_name: "開啟檔案類型選單",
                expanded: self.menu_open,
                message: Message::ChipDemo(ToggleMenu),
            })
        };
        let filter = theme.chip_in_grid(
            self.options(FILTERS[index], FILTER_TEXT[index], false),
            ChipContent::Filter {
                style: if index == 1 {
                    ChipStyle::Elevated
                } else {
                    ChipStyle::Flat
                },
                leading: None,
                selected: self.selected[index],
                show_checkmark: true,
                message: Message::ChipDemo(if compact_menu {
                    ToggleMenu
                } else {
                    Filter(index)
                }),
                trailing,
            },
        )?;
        if index != 2 {
            return Some(filter);
        }
        Some(theme.command_menu(
            CommandMenuOptions {
                id: MENU,
                label: "檔案類型",
                trigger_id: self.trigger(),
                open: self.menu_open,
                direction: self.direction(),
                preferred_width: 180.,
                restore_focus_to: Some(self.trigger()),
            },
            filter,
            [CommandMenuEntry::Item(CommandMenuItem {
                id: "chip-demo-documents",
                label: "文件",
                leading_icon: None,
                trailing_icon: None,
                disabled: false,
                on_press: Message::ChipDemo(ChooseType),
            })],
            Message::ChipDemo(DismissMenu),
            &self.menu,
        ))
    }

    fn contact(&self, theme: &Theme, index: usize) -> Option<View<Message>> {
        use ChipDemoEvent::*;
        theme.chip_in_grid(
            self.options(CONTACTS[index], CONTACT_NAMES[index], false),
            ChipContent::Input {
                leading: (index != 1).then_some(ChipLeading::Avatar(&self.avatar)),
                selected: index == 0,
                body: (index != 1).then_some(Message::ChipDemo(Edit(index))),
                remove: ChipRemove {
                    accessible_name: REMOVE_NAMES[index],
                    message: Message::ChipDemo(Remove(index)),
                },
            },
        )
    }

    fn grid(
        &self,
        theme: &Theme,
        index: usize,
        name: &str,
        members: &[&str],
        mut child: impl FnMut(usize) -> Option<View<Message>>,
    ) -> View<Message> {
        let rows: Vec<Vec<_>> = self.layouts[index]
            .rows(self.width, self.text_scale(), members)
            .into_iter()
            .map(|row| row.filter_map(&mut child).collect())
            .collect();
        let id = [
            "chip-demo-actions",
            "chip-demo-filters",
            "chip-demo-contacts",
        ][index];
        theme.chip_grid(id, name, self.direction(), rows)
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use ChipDemoEvent::*;
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
        };
        let controls = control_row(vec![
            theme
                .tonal_button(BACK, "返回", Message::ChipDemo(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            toggle("chip-demo-theme", "深色", dark, Message::Theme),
            toggle(
                "chip-demo-rtl",
                "右起",
                self.rtl,
                Message::ChipDemo(ToggleDirection),
            ),
            toggle(
                "chip-demo-large",
                "大字",
                self.large_text,
                Message::ChipDemo(ToggleLargeText),
            ),
            toggle(
                "chip-demo-single",
                "單選",
                !self.multi,
                Message::ChipDemo(ToggleSingle),
            ),
        ]);
        let contacts = self.contacts();
        let ids: Vec<_> = contacts.iter().map(|&index| CONTACTS[index]).collect();
        let mut column = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .gap(12.)
            .padding(6.)
            .clip(true);
        column.max_width = Length::Px(732.);
        let preview = View::new(
            Node::new(Role::Group, "")
                .id("chip-demo-preview")
                .styled(column),
        )
        .accessibility_scroll_pages()
        .children([
            theme.heading("chip-demo-action-heading", "動作與建議"),
            self.grid(theme, 0, "動作與建議", &ACTIONS, |index| {
                self.action(theme, index)
            }),
            theme.heading("chip-demo-filter-heading", "搜尋篩選"),
            self.grid(theme, 1, "搜尋篩選", &FILTERS, |index| {
                self.filter_chip(theme, index)
            }),
            theme.heading("chip-demo-contact-heading", "收件人"),
            self.grid(theme, 2, "收件人", &ids, |index| {
                self.contact(theme, contacts[index])
            }),
        ]);
        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("chip-demo")
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
        .children([theme.heading("chip-demo-title", "標籤"), controls, preview])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "multi": self.multi,
            "menu_open": self.menu_open,
            "compact": self.compact(),
            "selected": self.selected,
            "removed": self.removed,
            "actions": self.actions,
            "edits": self.edits,
            "removals": self.removals,
            "last": self.last,
            "palette": {
                "surface": theme.colors.surface,
                "surface_container_low": theme.colors.surface_container_low,
                "outline_variant": theme.colors.outline_variant,
                "secondary_container": theme.colors.secondary_container,
                "secondary": theme.colors.secondary,
            },
        })
    }
}
