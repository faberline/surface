use surface::{Align, Length, Node, Role, Style, TextScaling};
use surface_app::{AnimationFrame, AnimationUpdate, HorizontalDrag, View};
use surface_material::{
    adaptive, shape::LayoutDirection, typography, ButtonKind, ChoiceState, SideSheetContent,
    SideSheetEdge, SideSheetMotion, SideSheetOptions, SideSheetPresentation, SideSheetStyle, Theme,
};

use crate::Message;

pub(crate) const SECTION: usize = 2;
const SHOW: &str = "side-sheet-demo-show";
const BACK: &str = "sheet-demo-back";
const OPEN: &str = "sheet-demo-open";
const PANEL: &str = "sheet-demo-panel";
const TITLE: &str = "工作區詳細資料";

#[derive(Clone, Copy)]
pub enum SideSheetEvent {
    Show,
    Back,
    Toggle,
    Dismiss,
    Presentation(SideSheetPresentation),
    Style(SideSheetStyle),
    Edge(SideSheetEdge),
    Direction,
    TextScale,
    MainAction,
    PanelAction,
    Drag(HorizontalDrag, f32),
}

pub(crate) struct SideSheetDemo {
    visible: bool,
    open: bool,
    presentation: SideSheetPresentation,
    style: SideSheetStyle,
    edge: SideSheetEdge,
    rtl: bool,
    large_text: bool,
    viewport: [f32; 2],
    motion: SideSheetMotion,
    reduced_motion: bool,
    main_actions: usize,
    panel_actions: usize,
    dismissals: usize,
}

impl Default for SideSheetDemo {
    fn default() -> Self {
        Self {
            visible: false,
            open: false,
            presentation: SideSheetPresentation::StandardCoplanar,
            style: SideSheetStyle::Docked,
            edge: SideSheetEdge::End,
            rtl: false,
            large_text: false,
            viewport: [960., 800.],
            motion: SideSheetMotion::new(false),
            reduced_motion: false,
            main_actions: 0,
            panel_actions: 0,
            dismissals: 0,
        }
    }
}

impl SideSheetDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.open = false;
        self.motion = SideSheetMotion::new(false);
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let viewport = [width, height];
        if self.viewport == viewport {
            return false;
        }
        self.viewport = viewport;
        if self.visible && self.motion.is_dragging() {
            self.motion = SideSheetMotion::new(self.open);
        }
        self.visible
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        let policy_changed = self.reduced_motion != frame.reduced_motion;
        self.reduced_motion = frame.reduced_motion;
        if !self.visible {
            return AnimationUpdate::default();
        }
        let (mut update, intent) = self.motion.advance(frame.now_ms, frame.reduced_motion);
        if let Some(open) = intent {
            self.set_open(open);
            update.rebuild = true;
        }
        update.rebuild |= policy_changed;
        update
    }

    pub(crate) fn update(&mut self, event: SideSheetEvent) {
        match event {
            SideSheetEvent::Show => self.visible = true,
            SideSheetEvent::Back => self.leave(),
            SideSheetEvent::Toggle => self.set_open(!self.open),
            SideSheetEvent::Dismiss => self.set_open(false),
            SideSheetEvent::Presentation(presentation) => {
                self.presentation = presentation;
                self.motion = SideSheetMotion::new(self.open);
            }
            SideSheetEvent::Style(style) => {
                self.style = style;
                self.motion = SideSheetMotion::new(self.open);
            }
            SideSheetEvent::Edge(edge) => {
                self.edge = edge;
                self.motion = SideSheetMotion::new(self.open);
            }
            SideSheetEvent::Direction => {
                self.rtl = !self.rtl;
                self.motion = SideSheetMotion::new(self.open);
            }
            SideSheetEvent::TextScale => self.large_text = !self.large_text,
            SideSheetEvent::MainAction => self.main_actions += 1,
            SideSheetEvent::PanelAction => self.panel_actions += 1,
            SideSheetEvent::Drag(drag, width) => {
                self.motion.apply(drag, width, self.edge, self.direction());
            }
        }
    }

    fn set_open(&mut self, open: bool) {
        if self.open != open {
            self.open = open;
            self.motion.set_open(open);
            if !open {
                self.dismissals += 1;
            }
        }
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn presentation(&self) -> SideSheetPresentation {
        if self.presentation == SideSheetPresentation::StandardCoplanar
            && adaptive::WindowWidthSizeClass::from_dp(self.viewport[0])
                == Some(adaptive::WindowWidthSizeClass::Compact)
        {
            SideSheetPresentation::Modal
        } else {
            self.presentation
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "側邊面板",
                ButtonKind::Outlined,
                false,
                Message::SideSheet(SideSheetEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let layer = theme.side_sheet_with_motion(
            SideSheetOptions {
                id: PANEL,
                title: TITLE,
                close_label: "關閉側邊面板",
                restore_focus_to: OPEN,
                presentation: self.presentation(),
                style: self.style,
                edge: self.edge,
                direction: self.direction(),
            },
            self.open,
            &self.motion,
            SideSheetContent {
                body: self.body(theme, dark),
                panel: self.panel(theme),
                on_dismiss: Message::SideSheet(SideSheetEvent::Dismiss),
            },
            |drag, width| Message::SideSheet(SideSheetEvent::Drag(drag, width)),
        );
        View::new(
            Node::new(Role::Window, "側邊面板")
                .id("surface-page")
                .styled(Style::column().background(theme.colors.surface).clip(true)),
        )
        .children([layer])
    }

    fn body(&self, theme: &Theme, dark: bool) -> View<Message> {
        let choice_button = |id, label, selected, event| {
            theme
                .segmented_button(id, label, choice(selected), Message::SideSheet(event))
                .accessible_activation()
        };
        let controls = theme.row([
            theme
                .tonal_button(BACK, "返回", Message::SideSheet(SideSheetEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme.switch("theme", "深色", choice(dark), Message::Theme),
            choice_button(
                "sheet-demo-rtl",
                "右至左",
                self.rtl,
                SideSheetEvent::Direction,
            ),
            choice_button(
                "sheet-demo-scale",
                "大字",
                self.large_text,
                SideSheetEvent::TextScale,
            ),
        ]);
        let modes = theme.row(
            [
                (
                    "sheet-demo-overlay",
                    "覆蓋",
                    SideSheetPresentation::StandardOverlay,
                ),
                (
                    "sheet-demo-coplanar",
                    "並排",
                    SideSheetPresentation::StandardCoplanar,
                ),
                ("sheet-demo-modal", "模態", SideSheetPresentation::Modal),
            ]
            .map(|(id, label, mode)| {
                choice_button(
                    id,
                    label,
                    self.presentation == mode,
                    SideSheetEvent::Presentation(mode),
                )
            }),
        );
        let details = theme.row([
            choice_button(
                "sheet-demo-docked",
                "貼邊",
                self.style == SideSheetStyle::Docked,
                SideSheetEvent::Style(SideSheetStyle::Docked),
            ),
            choice_button(
                "sheet-demo-detached",
                "留邊",
                self.style == SideSheetStyle::Detached,
                SideSheetEvent::Style(SideSheetStyle::Detached),
            ),
            choice_button(
                "sheet-demo-edge-start",
                "起側",
                self.edge == SideSheetEdge::Start,
                SideSheetEvent::Edge(SideSheetEdge::Start),
            ),
            choice_button(
                "sheet-demo-edge-end",
                "末側",
                self.edge == SideSheetEdge::End,
                SideSheetEvent::Edge(SideSheetEdge::End),
            ),
        ]);
        let opener = theme.row([theme
            .tonal_button(
                OPEN,
                if self.open {
                    "收起面板"
                } else {
                    "展開面板"
                },
                Message::SideSheet(SideSheetEvent::Toggle),
            )
            .disclosure_expanded(self.open)
            .accessible_activation()]);
        let panel_on_left = (self.edge == SideSheetEdge::Start) != self.rtl;
        let main_action = View::new(Node::new(Role::Group, "").styled(Style::row().justify(
            if panel_on_left {
                Align::End
            } else {
                Align::Start
            },
        )))
        .children([theme
            .tonal_button(
                "sheet-demo-main-action",
                "更新頁面",
                Message::SideSheet(SideSheetEvent::MainAction),
            )
            .accessible_activation()]);
        View::new(
            Node::new(Role::Group, "主要內容")
                .id("sheet-demo-main")
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.))
                        .padding(12.)
                        .gap(8.)
                        .clip(true)
                        .background(theme.colors.surface_container),
                ),
        )
        .children([
            controls,
            modes,
            details,
            opener,
            main_action,
            theme.text(
                "sheet-demo-main-status",
                format!("頁面更新 {} 次", self.main_actions),
            ),
        ])
        .accessibility_scroll_pages()
    }

    fn panel(&self, theme: &Theme) -> View<Message> {
        let notice = |id, label: &'static str| {
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
        let action = |id, label, disabled| {
            theme
                .button_with(
                    id,
                    label,
                    ButtonKind::Tonal,
                    disabled,
                    Message::SideSheet(SideSheetEvent::PanelAction),
                )
                .accessible_activation()
        };
        View::new(Node::new(Role::Group, "").styled(Style::column().gap(12.)))
            .children([
                action("sheet-demo-action", "儲存設定", false),
                notice("sheet-demo-long", "這個工作區包含設計檔、測試結果和發佈紀錄。每次更新設定後，團隊成員都能看到最新資料。"),
                action("sheet-demo-disabled", "解除封存", true),
                notice("sheet-demo-visibility", "目前只有工作區成員可以查看資料。請確認分享範圍，再調整團隊權限。"),
                notice("sheet-demo-history", "發佈紀錄包含原始碼版本、測試結果與畫面。你可以在驗證完成後更新紀錄。"),
                action("sheet-demo-last", "更新紀錄", false),
            ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        let presentation = |mode| match mode {
            SideSheetPresentation::StandardOverlay => "overlay",
            SideSheetPresentation::StandardCoplanar => "coplanar",
            SideSheetPresentation::Modal => "modal",
        };
        let effective = self.presentation();
        serde_json::json!({
            "open": self.open,
            "reveal": self.motion.reveal(),
            "settling": self.motion.is_settling(),
            "dragging": self.motion.is_dragging(),
            "presentation": presentation(self.presentation),
            "effective_presentation": presentation(effective),
            "style": if self.style == SideSheetStyle::Docked { "docked" } else { "detached" },
            "edge": if self.edge == SideSheetEdge::Start { "start" } else { "end" },
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "main_actions": self.main_actions,
            "panel_actions": self.panel_actions,
            "dismissals": self.dismissals,
            "reduced_motion": self.reduced_motion,
            "active_surface": if effective == SideSheetPresentation::Modal {
                theme.colors.surface_container_low
            } else { theme.colors.surface },
            "standard_surface": theme.colors.surface,
            "scrim_color": theme.colors.scrim,
        })
    }
}

fn choice(selected: bool) -> ChoiceState {
    ChoiceState {
        selected,
        disabled: false,
    }
}
