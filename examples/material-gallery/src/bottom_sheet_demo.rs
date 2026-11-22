use surface::{Length, Node, Role, Style, TextScaling};
use surface_app::{AnimationFrame, AnimationUpdate, VerticalDrag, View};
use surface_material::{
    typography, BottomSheetContent, BottomSheetHeights, BottomSheetMotion, BottomSheetOptions,
    BottomSheetPresentation, BottomSheetState, ButtonKind, ChoiceState, Theme,
};

use crate::Message;

pub(crate) const SECTION: usize = 2;
const SHOW: &str = "bottom-sheet-demo-show";
const BACK: &str = "bottom-sheet-demo-back";
const OPEN: &str = "bottom-sheet-demo-open";
const PANEL: &str = "bottom-sheet-demo-panel";

#[derive(Clone, Copy)]
pub enum BottomSheetEvent {
    Show,
    Back,
    Open,
    Toggle,
    Dismiss,
    Presentation(BottomSheetPresentation),
    SkipPeek,
    TextScale,
    MainAction,
    PanelAction,
    Drag(VerticalDrag),
}

pub(crate) struct BottomSheetDemo {
    visible: bool,
    state: BottomSheetState,
    presentation: BottomSheetPresentation,
    skip_peek: bool,
    large_text: bool,
    viewport_height: f32,
    motion: BottomSheetMotion,
    reduced_motion: bool,
    main_actions: usize,
    panel_actions: usize,
    dismissals: usize,
}

impl Default for BottomSheetDemo {
    fn default() -> Self {
        let presentation = BottomSheetPresentation::Standard;
        let state = BottomSheetState::Closed;
        Self {
            visible: false,
            state,
            presentation,
            skip_peek: false,
            large_text: false,
            viewport_height: 800.,
            motion: BottomSheetMotion::new(state, sheet_heights(800., false, false), presentation),
            reduced_motion: false,
            main_actions: 0,
            panel_actions: 0,
            dismissals: 0,
        }
    }
}

impl BottomSheetDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.state = BottomSheetState::Closed;
        self.reset_motion();
    }

    pub(crate) fn viewport_changed(&mut self, height: f32) -> bool {
        if self.viewport_height == height {
            return false;
        }
        self.viewport_height = height;
        self.reset_motion();
        self.visible
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        self.reduced_motion = frame.reduced_motion;
        if !self.visible {
            return AnimationUpdate::default();
        }
        let (update, state) = self.motion.advance(frame.now_ms, frame.reduced_motion);
        if let Some(state) = state {
            self.set_state(state);
        }
        update
    }

    pub(crate) fn update(&mut self, event: BottomSheetEvent) {
        use BottomSheetState::{Closed, Expanded, Peek};
        match event {
            BottomSheetEvent::Show => self.visible = true,
            BottomSheetEvent::Back => self.leave(),
            BottomSheetEvent::Open if self.state == Closed => {
                self.set_state(if self.skip_peek { Expanded } else { Peek });
            }
            BottomSheetEvent::Open | BottomSheetEvent::Dismiss => self.set_state(Closed),
            BottomSheetEvent::Toggle => self.set_state(match self.state {
                Closed | Peek => Expanded,
                Expanded if !self.skip_peek => Peek,
                Expanded if self.presentation == BottomSheetPresentation::Modal => Closed,
                Expanded => Expanded,
            }),
            BottomSheetEvent::Presentation(presentation) => {
                self.presentation = presentation;
                self.reset_motion();
            }
            BottomSheetEvent::SkipPeek => {
                self.skip_peek = !self.skip_peek;
                if self.skip_peek && self.state == Peek {
                    self.state = Expanded;
                }
                self.reset_motion();
            }
            BottomSheetEvent::TextScale => {
                self.large_text = !self.large_text;
                self.reset_motion();
            }
            BottomSheetEvent::MainAction => self.main_actions += 1,
            BottomSheetEvent::PanelAction => self.panel_actions += 1,
            BottomSheetEvent::Drag(event) => self.motion.apply(event),
        }
    }

    fn set_state(&mut self, state: BottomSheetState) {
        if self.state != state {
            self.state = state;
            self.motion.set_state(state);
            if state == BottomSheetState::Closed {
                self.dismissals += 1;
            }
        }
    }

    fn reset_motion(&mut self) {
        self.motion = BottomSheetMotion::new(self.state, self.heights(), self.presentation);
    }

    fn heights(&self) -> BottomSheetHeights {
        sheet_heights(self.viewport_height, self.skip_peek, self.large_text)
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                "bottom-sheet-demo-show",
                "底部面板",
                ButtonKind::Outlined,
                false,
                Message::BottomSheet(BottomSheetEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use BottomSheetState::{Closed, Expanded, Peek};
        let handle_label = match (self.state, self.skip_peek, self.presentation) {
            (Expanded, false, _) => "收合底部面板",
            (Expanded, true, BottomSheetPresentation::Modal) => "收起底部面板",
            (Expanded, true, BottomSheetPresentation::Standard) => "底部面板已展開",
            (Closed | Peek, _, _) => "展開底部面板",
        };
        let sheet = theme.bottom_sheet_with_motion(
            BottomSheetOptions {
                id: PANEL,
                title: "工作區篩選",
                handle_label,
                close_label: "關閉底部面板",
                restore_focus_to: OPEN,
                presentation: self.presentation,
                heights: self.heights(),
            },
            self.state,
            &self.motion,
            BottomSheetContent {
                body: self.body(theme, dark),
                panel: self.panel(theme),
                on_toggle: Message::BottomSheet(BottomSheetEvent::Toggle),
                on_dismiss: Message::BottomSheet(BottomSheetEvent::Dismiss),
            },
            |event| Message::BottomSheet(BottomSheetEvent::Drag(event)),
        );
        View::new(
            Node::new(Role::Window, "底部面板")
                .id("bottom-sheet-demo-page")
                .styled(Style::column().background(theme.colors.surface).clip(true)),
        )
        .children([sheet])
    }

    fn body(&self, theme: &Theme, dark: bool) -> View<Message> {
        let toggle = |id, label, selected, event| {
            theme
                .segmented_button(id, label, choice(selected), Message::BottomSheet(event))
                .accessible_activation()
        };
        let tools = theme.row([
            theme
                .tonal_button(BACK, "返回", Message::BottomSheet(BottomSheetEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme.switch("theme", "深色", choice(dark), Message::Theme),
            toggle(
                "bottom-sheet-demo-scale",
                "大字",
                self.large_text,
                BottomSheetEvent::TextScale,
            ),
        ]);
        let presentations = theme.row(
            [
                (
                    "bottom-sheet-demo-standard",
                    "標準",
                    BottomSheetPresentation::Standard,
                ),
                (
                    "bottom-sheet-demo-modal",
                    "模態",
                    BottomSheetPresentation::Modal,
                ),
            ]
            .map(|(id, label, mode)| {
                toggle(
                    id,
                    label,
                    self.presentation == mode,
                    BottomSheetEvent::Presentation(mode),
                )
            }),
        );
        let open = self.state != BottomSheetState::Closed;
        let actions = theme.row([
            theme
                .tonal_button(
                    OPEN,
                    if open { "收起面板" } else { "開啟面板" },
                    Message::BottomSheet(BottomSheetEvent::Open),
                )
                .disclosure_expanded(open)
                .accessible_activation(),
            theme
                .tonal_button(
                    "bottom-sheet-demo-main-action",
                    "更新頁面",
                    Message::BottomSheet(BottomSheetEvent::MainAction),
                )
                .accessible_activation(),
        ]);
        View::new(
            Node::new(Role::Group, "主要內容")
                .id("bottom-sheet-demo-main")
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.))
                        .padding(12.)
                        .gap(8.)
                        .background(theme.colors.surface_container),
                ),
        )
        .children([
            tools,
            presentations,
            theme.row([theme.switch(
                "bottom-sheet-demo-skip-peek",
                "直接展開",
                choice(self.skip_peek),
                Message::BottomSheet(BottomSheetEvent::SkipPeek),
            )]),
            actions,
            theme.text(
                "bottom-sheet-demo-status",
                format!("頁面更新 {} 次", self.main_actions),
            ),
        ])
        .accessibility_scroll_pages()
    }

    fn panel(&self, theme: &Theme) -> View<Message> {
        let action = |id, label, disabled| {
            theme
                .button_with(
                    id,
                    label,
                    ButtonKind::Tonal,
                    disabled,
                    Message::BottomSheet(BottomSheetEvent::PanelAction),
                )
                .accessible_activation()
        };
        let paragraph = |id, label: &'static str| {
            View::new(
                Node::new(Role::Text, label)
                    .id(id)
                    .text(label)
                    .text_scaling(TextScaling::Expand)
                    .shrink_to_fit_width()
                    .styled(
                        Style::default()
                            .with_text_style(typography::BODY_MEDIUM)
                            .foreground(theme.colors.on_surface_variant),
                    ),
            )
        };
        View::new(Node::new(Role::Group, "").styled(Style::column().gap(16.))).children([
            action("bottom-sheet-demo-action", "套用篩選", false),
            paragraph("bottom-sheet-demo-description", "選擇要顯示的工作區項目。篩選結果會包含你有權查看的設計檔、測試結果、討論和發佈紀錄。"),
            action("bottom-sheet-demo-disabled", "清除篩選", true),
            paragraph("bottom-sheet-demo-projects", "工作區成員可以用專案類別和更新時間縮小範圍。含註解的項目仍會顯示在搜尋結果中。"),
            paragraph("bottom-sheet-demo-matches", "若結果太多，可先篩選最近更新的資料。若找不到項目，可清除條件並重新套用篩選。"),
            action("bottom-sheet-demo-last", "檢視所有結果", false),
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        let heights = self.heights();
        serde_json::json!({
            "state": match self.state {
                BottomSheetState::Closed => "closed",
                BottomSheetState::Peek => "peek",
                BottomSheetState::Expanded => "expanded",
            },
            "presentation": match self.presentation {
                BottomSheetPresentation::Standard => "standard",
                BottomSheetPresentation::Modal => "modal",
            },
            "visible_height": self.motion.visible_height(),
            "peek_height": heights.height(BottomSheetState::Peek),
            "expanded_height": heights.height(BottomSheetState::Expanded),
            "dragging": self.motion.is_dragging(),
            "settling": self.motion.is_settling(),
            "reduced_motion": self.reduced_motion,
            "skip_peek": self.skip_peek,
            "main_actions": self.main_actions,
            "panel_actions": self.panel_actions,
            "dismissals": self.dismissals,
            "text_scale": self.text_scale(),
        })
    }
}

fn sheet_heights(viewport_height: f32, skip_peek: bool, large_text: bool) -> BottomSheetHeights {
    let expanded = (viewport_height - 24.).clamp(48., 624.);
    let peek = if skip_peek {
        expanded
    } else if large_text {
        246.
    } else {
        206.
    };
    BottomSheetHeights::new(peek.min(expanded), expanded)
}

fn choice(selected: bool) -> ChoiceState {
    ChoiceState {
        selected,
        disabled: false,
    }
}
