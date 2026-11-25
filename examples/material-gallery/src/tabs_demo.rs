use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, Queue, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChoiceState, TabActivation, TabBarInput, TabBarKind,
    TabBarLayout, TabBarMotion, TabBarOptions, TabItem, TabPanelEntry, Theme, VectorIcon,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "tabs-demo-show";
const BACK: &str = "tabs-demo-back";
const LISTS: [&str; 2] = ["tabs-demo-primary", "tabs-demo-secondary"];
const LOCKED: usize = 1;
const TOTAL: usize = 5;
const TAB_IDS: [[&str; TOTAL]; 2] = [
    [
        "tabs-demo-overview",
        "tabs-demo-lock",
        "tabs-demo-activity",
        "tabs-demo-people",
        "tabs-demo-history",
    ],
    [
        "tabs-demo-all",
        "tabs-demo-private",
        "tabs-demo-unread",
        "tabs-demo-mentions",
        "tabs-demo-archived",
    ],
];
const PANEL_IDS: [[&str; TOTAL]; 2] = [
    [
        "tabs-demo-overview-panel",
        "tabs-demo-lock-panel",
        "tabs-demo-activity-panel",
        "tabs-demo-people-panel",
        "tabs-demo-history-panel",
    ],
    [
        "tabs-demo-all-panel",
        "tabs-demo-private-panel",
        "tabs-demo-unread-panel",
        "tabs-demo-mentions-panel",
        "tabs-demo-archived-panel",
    ],
];
const LABELS: [[&str; TOTAL]; 2] = [
    ["總覽", "鎖定", "最新活動", "成員管理", "搜尋歷史記錄"],
    ["全部", "私密", "尚未閱讀", "提及我的訊息", "已封存的項目"],
];
const LIST_NAMES: [&str; 2] = ["工作區頁籤", "訊息頁籤"];
const BODY: [[&str; TOTAL]; 2] = [
    [
        "查看工作區的進度與成員。",
        "此項目目前停用。",
        "查看工作區的最新變更。",
        "管理誰可開啟這個工作區。",
        "重新開啟之前瀏覽的項目。",
    ],
    [
        "閱讀所有訊息。",
        "此項目目前停用。",
        "閱讀你還沒開啟的訊息。",
        "閱讀提及你的訊息。",
        "閱讀已封存的訊息。",
    ],
];

#[derive(Clone)]
pub enum TabsDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    ToggleScrollable,
    ToggleActivation,
    Select { list: usize, index: usize },
    Input { list: usize, input: TabBarInput },
    OpenMessage,
}

pub(crate) struct TabsDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    scrollable: bool,
    automatic: bool,
    width: f32,
    selected: [usize; 2],
    requests: [usize; 2],
    opened: usize,
    motions: [TabBarMotion; 2],
    icon: VectorIcon,
}

impl Default for TabsDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            scrollable: false,
            automatic: false,
            width: 720.,
            selected: [0, 2],
            requests: [0; 2],
            opened: 0,
            motions: std::array::from_fn(|_| TabBarMotion::default()),
            icon: VectorIcon::new([
                [12., 2.],
                [19., 9.],
                [15., 9.],
                [15., 21.],
                [9., 21.],
                [9., 9.],
                [5., 9.],
            ])
            .expect("tab activity icon"),
        }
    }
}

impl TabsDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.motions = std::array::from_fn(|_| TabBarMotion::default());
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        let inner = (width - 32.).clamp(48., 720.);
        let changed = self.width != inner;
        self.width = inner;
        changed && self.visible
    }

    fn count(&self) -> usize {
        if self.scrollable {
            TOTAL
        } else {
            3
        }
    }

    pub(crate) fn update(&mut self, event: TabsDemoEvent, queue: Option<&Queue<Message>>) {
        use TabsDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            ToggleActivation => self.automatic = !self.automatic,
            ToggleScrollable => {
                self.scrollable = !self.scrollable;
                let count = self.count();
                for selected in &mut self.selected {
                    if *selected >= count {
                        *selected = 0;
                    }
                }
            }
            Select { list, index }
                if list < LISTS.len() && index < self.count() && index != LOCKED =>
            {
                self.selected[list] = index;
                self.requests[list] += 1;
            }
            Select { .. } => {}
            Input { list, input } if list < LISTS.len() => {
                if let Some(queue) = queue {
                    let options = self.options(list);
                    self.motions[list].apply(options, input, queue);
                }
            }
            Input { .. } => {}
            OpenMessage => self.opened += 1,
        }
    }

    fn options(&self, list: usize) -> TabBarOptions<'static> {
        TabBarOptions {
            id: LISTS[list],
            name: LIST_NAMES[list],
            kind: if list == 0 {
                TabBarKind::Primary
            } else {
                TabBarKind::Secondary
            },
            layout: if self.scrollable || self.width < 144. {
                TabBarLayout::Scrollable
            } else {
                TabBarLayout::Fixed
            },
            activation: if self.automatic {
                TabActivation::Automatic
            } else {
                TabActivation::Manual
            },
            direction: if self.rtl {
                LayoutDirection::RightToLeft
            } else {
                LayoutDirection::LeftToRight
            },
            available_width: self.width,
            text_scale: self.text_scale(),
            selected: Some(TAB_IDS[list][self.selected[list]]),
        }
    }

    pub(crate) fn frame(
        &mut self,
        frame: &AnimationFrame<'_>,
        queue: Option<&Queue<Message>>,
    ) -> AnimationUpdate {
        let Some(queue) = queue else {
            return AnimationUpdate::default();
        };
        let mut result = AnimationUpdate::default();
        if self.visible {
            for list in 0..LISTS.len() {
                let options = self.options(list);
                let update = self.motions[list].advance(options, frame, queue);
                result.rebuild |= update.rebuild;
                result.request_next |= update.request_next;
            }
        }
        result
    }

    pub(crate) fn observe_interaction(
        &mut self,
        frame: &AnimationFrame<'_>,
        input: FrameInput<'_>,
        pressed: Option<&str>,
        hovered: Option<&str>,
    ) -> bool {
        let mut changed = false;
        if self.visible {
            for list in 0..LISTS.len() {
                let options = self.options(list);
                changed |=
                    self.motions[list].observe_interaction(options, frame, input, pressed, hovered);
            }
        }
        changed
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "頁籤範例",
                ButtonKind::Outlined,
                false,
                Message::TabsDemo(TabsDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use TabsDemoEvent::*;
        let event = Message::TabsDemo;
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
                .tonal_button(BACK, "返回", event(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            toggle("tabs-demo-theme", "深色", dark, Message::Theme),
            toggle("tabs-demo-rtl", "右起", self.rtl, event(ToggleDirection)),
            toggle(
                "tabs-demo-large",
                "大字",
                self.large_text,
                event(ToggleLargeText),
            ),
            toggle(
                "tabs-demo-scrollable",
                "捲動",
                self.scrollable,
                event(ToggleScrollable),
            ),
            toggle(
                "tabs-demo-automatic",
                "自動",
                self.automatic,
                event(ToggleActivation),
            ),
        ]);
        let mut column = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .gap(12.)
            .clip(true);
        column.max_width = Length::Px(720.);
        let content = View::new(
            Node::new(Role::Group, "工作區")
                .id("tabs-demo-preview")
                .styled(column),
        )
        .accessibility_scroll_pages()
        .children([
            self.tabs(theme, 0),
            self.panel(theme, 0),
            self.tabs(theme, 1),
            self.panel(theme, 1),
        ]);
        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("tabs-demo")
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
        .children([theme.heading("tabs-demo-title", "頁籤"), controls, content])
    }

    fn tabs(&self, theme: &Theme, list: usize) -> View<Message> {
        theme
            .tab_bar(
                self.options(list),
                (0..self.count()).map(|index| TabItem {
                    id: TAB_IDS[list][index],
                    label: LABELS[list][index],
                    name: None,
                    icon: (index == 2 || index == 3).then(|| self.icon.clone()),
                    disabled: index == LOCKED,
                    panel_id: PANEL_IDS[list][index],
                    on_select: Message::TabsDemo(TabsDemoEvent::Select { list, index }),
                }),
                &self.motions[list],
                move |input| Message::TabsDemo(TabsDemoEvent::Input { list, input }),
            )
            .expect("the Gallery uses unique tab and panel IDs")
    }

    fn panel(&self, theme: &Theme, list: usize) -> View<Message> {
        let index = self.selected[list];
        let panel_id = PANEL_IDS[list][index];
        let body_style = Style::column()
            .width(Length::Percent(1.))
            .padding(12.)
            .gap(8.)
            .align(Align::Start)
            .background(theme.colors.surface_container_low);
        let mut body = Vec::with_capacity(2);
        if list != 0 {
            body.push(
                theme
                    .tonal_button(
                        "tabs-demo-read-message",
                        "開啟訊息",
                        Message::TabsDemo(TabsDemoEvent::OpenMessage),
                    )
                    .accessible_activation(),
            );
        }
        body.push(theme.text(&format!("{panel_id}-text"), BODY[list][index]));
        theme.tab_panel(
            panel_id,
            LISTS[list],
            TAB_IDS[list][index],
            if list == 0 {
                TabPanelEntry::Panel
            } else {
                TabPanelEntry::Contents
            },
            View::new(Node::new(Role::Group, "").styled(body_style)).children(body),
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "scrollable": self.scrollable,
            "automatic": self.automatic,
            "selected": std::array::from_fn::<_, 2, _>(|list| TAB_IDS[list][self.selected[list]]),
            "panels": std::array::from_fn::<_, 2, _>(|list| PANEL_IDS[list][self.selected[list]]),
            "requests": self.requests,
            "opened": self.opened,
            "indicators": self.motions.each_ref().map(|motion| {
                motion.indicator().map(|rect| [rect.x, rect.y, rect.width, rect.height])
            }),
            "palette": {
                "surface": theme.colors.surface,
                "surface_variant": theme.colors.surface_variant,
                "on_surface": theme.colors.on_surface,
                "on_surface_variant": theme.colors.on_surface_variant,
                "primary": theme.colors.primary,
                "secondary": theme.colors.secondary,
            },
        })
    }
}
