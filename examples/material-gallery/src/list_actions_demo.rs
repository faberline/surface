use std::collections::BTreeSet;
use std::sync::OnceLock;

use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChipKind, ChoiceState, ListLines, ListStyle,
    ListSupplementaryAction, MultiActionListItem, MultiActionListOptions, Theme, VectorIcon,
};

use crate::{rail_icons, rail_icons::Destination, Message};

const SHOW: &str = "list-actions-demo-show";
const PAGE_PADDING: f32 = 12.;
const DEFAULT_PIN: &str = "list-action-a-pin";
const STYLES: [(ListStyle, &str, &str); 3] = [
    (ListStyle::Baseline, "baseline", "基準"),
    (ListStyle::Standard, "standard", "標準"),
    (ListStyle::Segmented, "segmented", "分段"),
];

#[derive(Clone, Copy)]
enum ActionIcon {
    Pin,
    Archive,
    More,
}

#[derive(Clone, Copy)]
struct Action {
    id: &'static str,
    label: &'static str,
    icon: ActionIcon,
    disabled: bool,
}

impl Action {
    const fn new(id: &'static str, label: &'static str, icon: ActionIcon) -> Self {
        Self {
            id,
            label,
            icon,
            disabled: false,
        }
    }

    const fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

const A_ACTIONS: &[Action] = &[
    Action::new(DEFAULT_PIN, "固定山", ActionIcon::Pin),
    Action::new("list-action-a-more", "更多山的選項", ActionIcon::More),
];
const B_ACTIONS: &[Action] = &[
    Action::new("list-action-b-pin", "固定海", ActionIcon::Pin),
    Action::new("list-action-b-more", "更多海的選項", ActionIcon::More).disabled(),
];
const C_ACTIONS: &[Action] = &[
    Action::new("list-action-c-pin", "固定步道", ActionIcon::Pin),
    Action::new("list-action-c-archive", "封存步道", ActionIcon::Archive),
    Action::new("list-action-c-more", "更多步道的選項", ActionIcon::More),
];
const F_ACTIONS: &[Action] = &[Action::new(
    "list-action-f-more",
    "更多森林的選項",
    ActionIcon::More,
)];

#[derive(Clone, Copy)]
struct Row {
    id: &'static str,
    label: &'static str,
    actions: &'static [Action],
}

impl Row {
    fn disabled(self) -> bool {
        self.id == "list-action-b"
    }
}

const ROWS: [Row; 6] = [
    Row {
        id: "list-action-a",
        label: "山",
        actions: A_ACTIONS,
    },
    Row {
        id: "list-action-b",
        label: "海",
        actions: B_ACTIONS,
    },
    Row {
        id: "list-action-c",
        label: "長文字會在窄畫面自動換行",
        actions: C_ACTIONS,
    },
    Row {
        id: "list-action-d",
        label: "步道",
        actions: &[],
    },
    Row {
        id: "list-action-e",
        label: "島",
        actions: &[],
    },
    Row {
        id: "list-action-f",
        label: "森林",
        actions: F_ACTIONS,
    },
];

#[derive(Clone)]
pub enum ListActionsEvent {
    Show,
    Back,
    Direction,
    LargeText,
    Style(ListStyle),
    Reorder,
    Remove,
    Reset,
    Clear,
    Activate {
        id: &'static str,
        label: &'static str,
        pin: bool,
    },
}

pub(crate) struct ListActionsDemo {
    visible: bool,
    style: ListStyle,
    rtl: bool,
    large_text: bool,
    width: f32,
    reversed: bool,
    hide_last_action: bool,
    pinned: BTreeSet<&'static str>,
    last: Option<(&'static str, &'static str)>,
    activations: usize,
}

impl Default for ListActionsDemo {
    fn default() -> Self {
        Self {
            visible: false,
            style: ListStyle::Segmented,
            rtl: false,
            large_text: false,
            width: 960.,
            reversed: false,
            hide_last_action: false,
            pinned: BTreeSet::from([DEFAULT_PIN]),
            last: None,
            activations: 0,
        }
    }
}

impl ListActionsDemo {
    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "清單多動作",
                ButtonKind::Outlined,
                false,
                Message::ListActions(ListActionsEvent::Show),
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

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        let changed = self.width != width;
        self.width = width;
        self.visible && changed
    }

    fn available_width(&self) -> f32 {
        (self.width - 2. * PAGE_PADDING).max(0.)
    }

    pub(crate) fn update(&mut self, event: ListActionsEvent) {
        match event {
            ListActionsEvent::Show => self.visible = true,
            ListActionsEvent::Back => self.leave(),
            ListActionsEvent::Direction => self.rtl = !self.rtl,
            ListActionsEvent::LargeText => self.large_text = !self.large_text,
            ListActionsEvent::Style(style) => self.style = style,
            ListActionsEvent::Reorder => self.reversed = !self.reversed,
            ListActionsEvent::Remove => self.hide_last_action = !self.hide_last_action,
            ListActionsEvent::Reset => {
                self.pinned = BTreeSet::from([DEFAULT_PIN]);
                self.reversed = false;
                self.hide_last_action = false;
                self.last = None;
                self.activations = 0;
            }
            ListActionsEvent::Clear => {
                self.last = None;
                self.activations = 0;
            }
            ListActionsEvent::Activate { id, label, pin } => {
                self.last = Some((id, label));
                self.activations += 1;
                if pin && !self.pinned.remove(id) {
                    self.pinned.insert(id);
                }
            }
        }
    }

    fn rows(&self) -> impl Iterator<Item = Row> + '_ {
        (0..ROWS.len()).map(|index| {
            let index = if self.reversed {
                ROWS.len() - 1 - index
            } else {
                index
            };
            let mut row = ROWS[index];
            if self.hide_last_action && row.id == "list-action-f" {
                row.actions = &[];
            }
            row
        })
    }

    fn action_checked(&self, action: Action) -> Option<bool> {
        matches!(action.icon, ActionIcon::Pin).then(|| self.pinned.contains(action.id))
    }

    fn item(&self, row: Row) -> MultiActionListItem<'static, Message> {
        let long = row.id == "list-action-c";
        let on_activate = |id, label, pin| {
            Some(Message::ListActions(ListActionsEvent::Activate {
                id,
                label,
                pin,
            }))
        };
        MultiActionListItem {
            id: row.id,
            headline: row.label,
            overline: long.then_some("路線"),
            supporting: long.then_some("放大文字後仍可完整閱讀"),
            leading: Some(Destination::Home.icon()),
            lines: if long {
                ListLines::Three
            } else {
                ListLines::One
            },
            disabled: row.disabled(),
            on_activate: on_activate(row.id, row.label, false),
            actions: row
                .actions
                .iter()
                .map(|&action| {
                    let checked = self.action_checked(action);
                    ListSupplementaryAction {
                        id: action.id,
                        label: action.label,
                        icon: match action.icon {
                            ActionIcon::Pin => pin_icon(),
                            ActionIcon::Archive => Destination::Archive.icon(),
                            ActionIcon::More => rail_icons::more(),
                        },
                        checked,
                        disabled: action.disabled,
                        on_activate: on_activate(action.id, action.label, checked.is_some()),
                    }
                })
                .collect(),
        }
    }

    pub(crate) fn view(&self, theme: &Theme) -> View<Message> {
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let list = theme
            .multi_action_list(
                MultiActionListOptions {
                    id: "list-actions",
                    name: "書籤動作",
                    style: self.style,
                    direction,
                    available_width: self.available_width(),
                },
                self.rows().map(|row| self.item(row)),
            )
            .unwrap_or_else(|| theme.text("list-actions-width", "視窗太窄"));
        let scroller = View::new(Node::new(Role::Group, "").id("list-actions-scroll").styled(
            Style {
                height: Length::Px(0.),
                min_height: 140.,
                max_height: Length::Px(320.),
                grow: 1.,
                clip: true,
                ..Style::column()
            },
        ))
        .accessibility_scroll_pages()
        .children([list]);
        let action = |id: &str, label: &str, event| {
            theme
                .button(id, label, Message::ListActions(event))
                .accessible_activation()
        };
        let management = controls("管理清單").children([
            action(
                "list-actions-demo-reorder",
                "倒序",
                ListActionsEvent::Reorder,
            ),
            action(
                "list-actions-demo-remove",
                if self.hide_last_action {
                    "還原動作"
                } else {
                    "移除動作"
                },
                ListActionsEvent::Remove,
            ),
            action("list-actions-demo-before", "重設", ListActionsEvent::Reset),
        ]);
        let status = self.last.map_or_else(
            || "尚未操作".to_owned(),
            |(_, label)| format!("最近操作：{label}"),
        );
        let after = View::new(
            Node::new(Role::Group, "操作記錄").styled(Style::row().align(Align::Center).gap(8.)),
        )
        .children([
            action("list-actions-demo-after", "清除", ListActionsEvent::Clear),
            View::new(
                Node::new(Role::Status, "")
                    .id("list-actions-demo-status")
                    .text(status)
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
            Node::new(Role::Window, "清單多動作")
                .id("list-actions-demo")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .padding(PAGE_PADDING)
                        .gap(8.)
                        .clip(true)
                        .background(theme.colors.surface_container),
                ),
        )
        .restore_focus_in_subtree_on_removal(SHOW)
        .accessibility_scroll_pages()
        .children([self.view_controls(theme), management, scroller, after])
    }

    fn view_controls(&self, theme: &Theme) -> View<Message> {
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
                    Message::ListActions(event),
                    None,
                )
                .accessible_activation()
        };
        let settings = controls("畫面設定").children([
            theme
                .tonal_button(
                    "list-actions-demo-back",
                    "返回",
                    Message::ListActions(ListActionsEvent::Back),
                )
                .accessible_activation(),
            theme
                .button("list-actions-demo-theme", "配色", Message::Theme)
                .accessible_activation(),
            toggle(
                "list-actions-demo-rtl",
                "右起",
                self.rtl,
                ListActionsEvent::Direction,
            ),
            toggle(
                "list-actions-demo-large-text",
                "大字",
                self.large_text,
                ListActionsEvent::LargeText,
            ),
        ]);
        let styles = controls("清單樣式").children(STYLES.map(|(style, suffix, name)| {
            toggle(
                &format!("list-actions-demo-{suffix}"),
                name,
                self.style == style,
                ListActionsEvent::Style(style),
            )
        }));
        View::new(Node::new(Role::Group, "").styled(Style::column().gap(4.)))
            .children([settings, styles])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        runtime: &surface_app::Runtime<crate::Gallery>,
        theme: &Theme,
    ) -> serde_json::Value {
        use serde_json::json;

        let inspect_action = |id, label, disabled, checked: Option<bool>| {
            let geometry = runtime
                .dom()
                .by_id(id)
                .and_then(|node| runtime.computed()?.nodes.get(node.index()));
            let rect = |r: surface::Rect| [r.x, r.y, r.width, r.height];
            json!({"id":id,"label":label,"disabled":disabled,"checked":checked,
                "bounds":geometry.map(|node| rect(node.bounds)),"clip":geometry.map(|node| rect(node.clip))})
        };
        let actions: Vec<_> = self
            .rows()
            .flat_map(|row| {
                std::iter::once(inspect_action(row.id, row.label, row.disabled(), None)).chain(
                    row.actions.iter().map(move |&action| {
                        inspect_action(
                            action.id,
                            action.label,
                            action.disabled,
                            self.action_checked(action),
                        )
                    }),
                )
            })
            .collect();
        let variant = STYLES
            .iter()
            .find(|(style, _, _)| *style == self.style)
            .map(|(_, suffix, _)| *suffix);
        let colors = theme.colors;
        json!({"visible":self.visible,"variant":variant,"rtl":self.rtl,"text_scale":self.text_scale(),
            "available_width":self.available_width(),"activations":self.activations,"last":self.last.map(|(id, _)| id),"actions":actions,
            "colors":{"surface":colors.surface,"surface_container":colors.surface_container,
            "primary":colors.primary,
            "secondary":colors.secondary,"secondary_container":colors.secondary_container,
            "on_surface":colors.on_surface,"on_surface_variant":colors.on_surface_variant,
            "on_secondary_container":colors.on_secondary_container}})
    }
}

fn controls(name: &str) -> View<Message> {
    View::new(Node::new(Role::Group, name).styled(Style::row().gap(8.).wrap(true)))
}

fn pin_icon() -> &'static VectorIcon {
    static PIN: OnceLock<VectorIcon> = OnceLock::new();
    PIN.get_or_init(|| {
        VectorIcon::new([
            [7., 3.],
            [17., 3.],
            [17., 5.],
            [16., 5.],
            [16., 11.],
            [19., 14.],
            [19., 16.],
            [13., 16.],
            [13., 21.],
            [11., 21.],
            [11., 16.],
            [5., 16.],
            [5., 14.],
            [8., 11.],
            [8., 5.],
            [7., 5.],
        ])
        .expect("the pin has a valid contour")
    })
}
