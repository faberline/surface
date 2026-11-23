use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, Queue, ScrollTarget, View};
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChipKind, ChoiceState, ListItem, ListLines,
    ListReorderEntry, ListReorderInput, ListReorderRequest, ListReorderState, ListStyle,
    ReorderableListItem, ReorderableListOptions, Theme,
};

use crate::{rail_icons::Destination, Message};

const SHOW: &str = "list-reorder-demo-show";
const LIST: &str = "list-reorder-items";
const SCROLL: &str = "list-reorder-scroll";
const PAGE_PADDING: f32 = 12.;
const INITIAL_STATUS: &str = "尚未調整順序";
const STYLES: [(ListStyle, &str, &str); 3] = [
    (ListStyle::Baseline, "baseline", "基準"),
    (ListStyle::Standard, "standard", "標準"),
    (ListStyle::Segmented, "segmented", "分段"),
];

#[derive(Clone, Copy)]
struct Row {
    id: &'static str,
    headline: &'static str,
    overline: Option<&'static str>,
    supporting: Option<&'static str>,
    movable: bool,
}

impl Row {
    const fn new(id: &'static str, headline: &'static str) -> Self {
        Self {
            id,
            headline,
            overline: None,
            supporting: None,
            movable: true,
        }
    }

    const fn with_supporting(mut self, text: &'static str) -> Self {
        self.supporting = Some(text);
        self
    }

    const fn with_overline(mut self, text: &'static str) -> Self {
        self.overline = Some(text);
        self
    }

    const fn fixed(mut self) -> Self {
        self.movable = false;
        self
    }

    fn item(self) -> ListItem<'static, Message> {
        let lines = match (self.overline, self.supporting) {
            (None, None) => ListLines::One,
            (Some(_), Some(_)) => ListLines::Three,
            _ => ListLines::Two,
        };
        ListItem {
            id: self.id,
            headline: self.headline,
            overline: self.overline,
            supporting: self.supporting,
            leading: Some(Destination::Home.icon()),
            trailing: None,
            lines,
            disabled: !self.movable,
            on_activate: self
                .movable
                .then_some(Message::ListReorder(ListReorderEvent::Activate(self.id))),
        }
    }
}

const ROWS: [Row; 12] = [
    Row::new("list-reorder-a", "筆記"),
    Row::new("list-reorder-b", "山徑").with_supporting("出發前核對步道路線"),
    Row::new("list-reorder-c", "地圖")
        .with_overline("旅程")
        .with_supporting("下載離線地圖供沿途使用"),
    Row::new("list-reorder-d", "車站"),
    Row::new("list-reorder-e", "行程").with_supporting("集合地點與出發時間"),
    Row::new("list-reorder-f", "相簿"),
    Row::new("list-reorder-g", "文件").with_supporting("攜帶住宿證明和其他旅行資料"),
    Row::new("list-reorder-h", "旅館"),
    Row::new("list-reorder-i", "清單")
        .with_overline("出發前")
        .with_supporting("整理行李並檢查隨身用品"),
    Row::new("list-reorder-fixed", "固定分隔").fixed(),
    Row::new("list-reorder-j", "票券"),
    Row::new("list-reorder-k", "聯絡人"),
];

#[derive(Clone)]
pub enum ListReorderEvent {
    Show,
    Back,
    Direction,
    LargeText,
    Style(ListStyle),
    Reset,
    External,
    Activate(&'static str),
    Input(ListReorderInput),
}

pub(crate) struct ListReorderDemo {
    visible: bool,
    view_width: f32,
    style: ListStyle,
    rtl: bool,
    large_text: bool,
    rows: [Row; ROWS.len()],
    state: ListReorderState,
    revision: u64,
    moves: usize,
    external_changes: usize,
    activations: usize,
    last_move: Option<ListReorderRequest>,
    last_activation: Option<&'static str>,
    status: String,
}

impl Default for ListReorderDemo {
    fn default() -> Self {
        Self {
            visible: false,
            view_width: 960.,
            style: ListStyle::Segmented,
            rtl: false,
            large_text: false,
            rows: ROWS,
            state: ListReorderState::new(),
            revision: 0,
            moves: 0,
            external_changes: 0,
            activations: 0,
            last_move: None,
            last_activation: None,
            status: INITIAL_STATUS.to_owned(),
        }
    }
}

impl ListReorderDemo {
    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "可排序清單",
                ButtonKind::Outlined,
                false,
                Message::ListReorder(ListReorderEvent::Show),
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

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        let changed = self.view_width != width;
        self.view_width = width;
        self.visible && changed
    }

    pub(crate) fn cancel(&mut self) -> bool {
        self.visible && self.state.cancel()
    }

    pub(crate) fn frame(
        &mut self,
        frame: &AnimationFrame<'_>,
        queue: Option<&Queue<Message>>,
    ) -> AnimationUpdate {
        if !self.visible {
            return AnimationUpdate::default();
        }
        let options = self.options();
        let order = self.order();
        self.state.advance(options, &order, frame, queue)
    }

    pub(crate) fn update(&mut self, event: ListReorderEvent, queue: Option<&Queue<Message>>) {
        if !self.visible && !matches!(event, ListReorderEvent::Show) {
            return;
        }
        match event {
            ListReorderEvent::Show => {
                self.visible = true;
                Self::scroll_to_top(queue);
            }
            ListReorderEvent::Back => self.leave(),
            ListReorderEvent::Direction => {
                self.state.cancel();
                self.rtl = !self.rtl;
            }
            ListReorderEvent::LargeText => {
                self.state.cancel();
                self.large_text = !self.large_text;
            }
            ListReorderEvent::Style(style) => {
                self.state.cancel();
                self.style = style;
            }
            ListReorderEvent::Reset => {
                self.reset();
                Self::scroll_to_top(queue);
            }
            ListReorderEvent::External => {
                // Swap the two rows after the barrier as if new order arrived.
                // Keep the active source mounted so revision alone retires it.
                let len = self.rows.len();
                self.rows.swap(len - 2, len - 1);
                self.revision += 1;
                self.external_changes += 1;
                self.status = "已更新清單順序".to_owned();
            }
            ListReorderEvent::Activate(id) => {
                if let Some(row) = self.rows.iter().find(|row| row.id == id && row.movable) {
                    self.activations += 1;
                    self.last_activation = Some(id);
                    self.status = format!("已開啟{}", row.headline);
                }
            }
            ListReorderEvent::Input(input) => {
                let order = self.order();
                if let Some(request) = self.state.update(input, self.revision, &order) {
                    self.accept_move(request);
                }
            }
        }
    }

    fn scroll_to_top(queue: Option<&Queue<Message>>) {
        if let Some(queue) = queue {
            queue.scroll_to(SCROLL, ScrollTarget::Top);
        }
    }

    fn reset(&mut self) {
        self.state.cancel();
        self.rows = ROWS;
        self.revision += 1;
        self.moves = 0;
        self.external_changes = 0;
        self.activations = 0;
        self.last_move = None;
        self.last_activation = None;
        self.status = INITIAL_STATUS.to_owned();
    }

    fn order(&self) -> [ListReorderEntry<'static>; ROWS.len()] {
        self.rows.map(|row| ListReorderEntry {
            id: row.id,
            movable: row.movable,
        })
    }

    fn accept_move(&mut self, request: ListReorderRequest) {
        let Some(from) = self.rows.iter().position(|row| row.id == request.item) else {
            return;
        };
        let before = match request.before.as_deref() {
            Some(id) => {
                let Some(index) = self.rows.iter().position(|row| row.id == id) else {
                    return;
                };
                index
            }
            None => self.rows.len(),
        };
        let to = if before > from { before - 1 } else { before };
        let crossed = &mut self.rows[from.min(to)..=from.max(to)];
        if from == to || crossed.iter().any(|row| !row.movable) {
            return;
        }
        if from < to {
            crossed.rotate_left(1);
        } else {
            crossed.rotate_right(1);
        }
        self.revision += 1;
        self.moves += 1;
        self.last_move = Some(request);
        self.status = format!("已將{}移到第 {} 項", self.rows[to].headline, to + 1);
    }

    fn options(&self) -> ReorderableListOptions<'static> {
        ReorderableListOptions {
            id: LIST,
            name: "旅程資料順序",
            scroller_id: SCROLL,
            style: self.style,
            direction: if self.rtl {
                LayoutDirection::RightToLeft
            } else {
                LayoutDirection::LeftToRight
            },
            available_width: (self.view_width - 2. * PAGE_PADDING).max(0.),
            revision: self.revision,
        }
    }

    pub(crate) fn view(&self, theme: &Theme) -> View<Message> {
        let labels = self.rows.map(|row| {
            (
                row,
                format!("上移{}", row.headline),
                format!("下移{}", row.headline),
            )
        });
        let items = labels
            .iter()
            .map(|(row, move_up, move_down)| ReorderableListItem {
                row: row.item(),
                movable: row.movable,
                move_up,
                move_down,
            });
        let list = theme.reorderable_list(self.options(), items, &self.state, |input| {
            Message::ListReorder(ListReorderEvent::Input(input))
        });
        let scroller = View::new(
            Node::new(Role::Group, "可排序清單捲動區")
                .id(SCROLL)
                .styled(Style {
                    height: Length::Px(0.),
                    min_height: 112.,
                    max_height: Length::Px(272.),
                    grow: 1.,
                    clip: true,
                    background: theme.colors.surface,
                    ..Style::column()
                }),
        )
        .accessibility_scroll_pages()
        .children(list);
        let status = View::new(
            Node::new(Role::Status, "")
                .id("list-reorder-demo-status")
                .text(self.status.as_str())
                .shrink_to_fit_width()
                .styled(
                    Style::column()
                        .with_text_style(typography::BODY_MEDIUM)
                        .foreground(theme.colors.on_surface),
                ),
        );
        View::new(
            Node::new(Role::Window, "可排序清單")
                .id("list-reorder-demo")
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
        .children([self.controls(theme), status, scroller])
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
                    Message::ListReorder(event),
                    None,
                )
                .accessible_activation()
        };
        let action = |id: &str, label: &str, event| {
            theme
                .button(id, label, Message::ListReorder(event))
                .accessible_activation()
                .suppress_repeated_activation()
        };
        let display = controls("畫面設定").children([
            theme
                .tonal_button(
                    "list-reorder-demo-back",
                    "返回",
                    Message::ListReorder(ListReorderEvent::Back),
                )
                .accessible_activation(),
            theme
                .button("list-reorder-demo-theme", "配色", Message::Theme)
                .accessible_activation(),
            toggle(
                "list-reorder-demo-rtl",
                "右起",
                self.rtl,
                ListReorderEvent::Direction,
            ),
            toggle(
                "list-reorder-demo-large-text",
                "大字",
                self.large_text,
                ListReorderEvent::LargeText,
            ),
        ]);
        let styles = STYLES.map(|(style, suffix, name)| {
            toggle(
                &format!("list-reorder-demo-{suffix}"),
                name,
                self.style == style,
                ListReorderEvent::Style(style),
            )
        });
        let order_actions = [
            action("list-reorder-demo-reset", "重設", ListReorderEvent::Reset),
            action(
                "list-reorder-demo-external",
                "更新",
                ListReorderEvent::External,
            ),
        ];
        let presets = controls("清單樣式").children(styles.into_iter().chain(order_actions));
        View::new(Node::new(Role::Group, "").styled(Style::column().gap(4.)))
            .children([display, presets])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        runtime: &surface_app::Runtime<crate::Gallery>,
        theme: &Theme,
    ) -> serde_json::Value {
        let variant = STYLES
            .iter()
            .find_map(|(style, name, _)| (*style == self.style).then_some(*name));
        let colors = theme.colors;
        serde_json::json!({
            "visible": self.visible,
            "variant": variant,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "order": self.rows.map(|row| row.id),
            "revision": self.revision,
            "moves": self.moves,
            "external_changes": self.external_changes,
            "activations": self.activations,
            "last_activation": self.last_activation,
            "last_move": self.last_move.as_ref().map(|request| serde_json::json!({
                "item": request.item,
                "before": request.before,
            })),
            "scroll_y": runtime.scroll_metrics(SCROLL).map(|metrics| metrics.offset.1),
            "colors": {
                "tertiary_container": colors.tertiary_container,
                "on_tertiary_container": colors.on_tertiary_container,
                "surface_container_low": colors.surface_container_low,
                "shadow": colors.shadow,
                "surface": colors.surface,
                "surface_container": colors.surface_container,
                "on_surface": colors.on_surface,
                "on_surface_variant": colors.on_surface_variant,
                "secondary": colors.secondary,
            },
        })
    }
}

fn controls(name: &str) -> View<Message> {
    View::new(
        Node::new(Role::Group, name).styled(Style::row().gap(8.).wrap(true).align(Align::Center)),
    )
}
