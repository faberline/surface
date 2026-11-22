use surface::{Align, Insets, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChoiceState, FabColor, FabSize, Theme,
    ToolbarButtonOptions, ToolbarContent, ToolbarKind, ToolbarOptions, ToolbarPalette,
    ToolbarScroll, ToolbarScrollBehavior, ToolbarVisibility,
};

use crate::Message;

pub(crate) const SECTION: usize = 1;
const SHOW: &str = "toolbar-demo-show";
const BACK: &str = "toolbar-demo-back";
const BAR: &str = "toolbar-demo-bar";
const SCROLL: &str = "toolbar-demo-scroll";
const BODY_ACTION: &str = "toolbar-demo-body-action";
const OVERFLOW: &str = "toolbar-demo-overflow";
const MENU: &str = "toolbar-demo-overflow-menu";
const MENU_ACTION: &str = "toolbar-demo-overflow-action";
const FAB: &str = "toolbar-demo-action-fab";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToolbarEdge {
    Top,
    Bottom,
    Start,
    End,
}

#[derive(Clone, Copy)]
pub enum ToolbarEvent {
    Show,
    Back,
    Kind(ToolbarKind),
    Palette(ToolbarPalette),
    Edge(ToolbarEdge),
    Behavior(ToolbarScrollBehavior),
    Direction,
    TextScale,
    PairFab,
    KeepVisible,
    Preview,
    Scroll(ToolbarScroll),
    Toggle,
    Action,
    Body,
    Overflow(bool),
    MenuAction,
}

pub(crate) struct ToolbarDemo {
    visible: bool,
    kind: ToolbarKind,
    palette: ToolbarPalette,
    edge: ToolbarEdge,
    behavior: ToolbarScrollBehavior,
    scroll: ToolbarScroll,
    offset: f32,
    viewport: [f32; 2],
    rtl: bool,
    large_text: bool,
    fab: bool,
    keep_visible: bool,
    selected: bool,
    overflow_open: bool,
    overflow_bounds: Option<Rect>,
    actions: usize,
    body_actions: usize,
}

impl Default for ToolbarDemo {
    fn default() -> Self {
        Self {
            visible: false,
            kind: ToolbarKind::Docked,
            palette: ToolbarPalette::Standard,
            edge: ToolbarEdge::Bottom,
            behavior: ToolbarScrollBehavior::Pinned,
            scroll: ToolbarScroll::default(),
            offset: 0.,
            viewport: [960., 800.],
            rtl: false,
            large_text: false,
            fab: false,
            keep_visible: false,
            selected: false,
            overflow_open: false,
            overflow_bounds: None,
            actions: 0,
            body_actions: 0,
        }
    }
}

impl ToolbarDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.overflow_open = false;
        self.scroll = ToolbarScroll::default();
        self.offset = 0.;
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let next = [width, height];
        let changed = next != self.viewport;
        self.viewport = next;
        changed && self.visible
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if !self.visible {
            return AnimationUpdate::default();
        }
        if let Some(body) = frame
            .dom
            .by_id(SCROLL)
            .and_then(|id| frame.dom.get(id).ok())
        {
            self.offset = body.style().scroll_y;
        }
        let next = frame.dom.by_id(OVERFLOW).and_then(|id| {
            frame
                .layout?
                .nodes
                .get(id.index())
                .map(|geometry| geometry.bounds)
        });
        let rebuild = next.is_some_and(|bounds| {
            let changed = self.overflow_bounds.replace(bounds) != Some(bounds);
            changed && self.overflow_open
        });
        AnimationUpdate {
            rebuild,
            request_next: false,
        }
    }

    pub(crate) fn update(&mut self, event: ToolbarEvent) {
        match event {
            ToolbarEvent::Show => self.visible = true,
            ToolbarEvent::Back => self.leave(),
            ToolbarEvent::Kind(kind) => {
                self.kind = kind;
                self.edge = match (kind, self.edge) {
                    (ToolbarKind::FloatingVertical, ToolbarEdge::Start) => ToolbarEdge::Start,
                    (ToolbarKind::FloatingVertical, _) => ToolbarEdge::End,
                    (_, ToolbarEdge::Top) => ToolbarEdge::Top,
                    _ => ToolbarEdge::Bottom,
                };
                if kind == ToolbarKind::Docked
                    && self.behavior == ToolbarScrollBehavior::CollapseOnScroll
                {
                    self.behavior = ToolbarScrollBehavior::Pinned;
                }
                self.scroll = ToolbarScroll::default();
            }
            ToolbarEvent::Palette(palette) => self.palette = palette,
            ToolbarEvent::Edge(edge) => self.edge = edge,
            ToolbarEvent::Behavior(behavior) => {
                self.behavior = behavior;
                self.scroll = ToolbarScroll::default();
            }
            ToolbarEvent::Direction => self.rtl = !self.rtl,
            ToolbarEvent::TextScale => self.large_text = !self.large_text,
            ToolbarEvent::PairFab => self.fab = !self.fab,
            ToolbarEvent::KeepVisible => {
                self.keep_visible = !self.keep_visible;
                if self.keep_visible {
                    self.scroll = ToolbarScroll::default();
                }
            }
            ToolbarEvent::Preview => {
                self.scroll = self.scroll.with_visibility(self.next_visibility());
            }
            ToolbarEvent::Scroll(scroll) => self.scroll = scroll,
            ToolbarEvent::Toggle => {
                self.selected = !self.selected;
                self.actions += 1;
            }
            ToolbarEvent::Action => self.actions += 1,
            ToolbarEvent::Body => self.body_actions += 1,
            ToolbarEvent::Overflow(open) => self.overflow_open = open,
            ToolbarEvent::MenuAction => {
                self.actions += 1;
                self.overflow_open = false;
            }
        }
    }

    fn next_visibility(&self) -> ToolbarVisibility {
        match self.scroll.visibility() {
            ToolbarVisibility::Expanded if self.kind != ToolbarKind::Docked => {
                ToolbarVisibility::Collapsed
            }
            ToolbarVisibility::Expanded | ToolbarVisibility::Collapsed => ToolbarVisibility::Hidden,
            ToolbarVisibility::Hidden => ToolbarVisibility::Expanded,
        }
    }

    fn has_fab(&self) -> bool {
        self.kind != ToolbarKind::Docked && self.fab
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "工具列",
                ButtonKind::Outlined,
                false,
                app(ToolbarEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let mut children = Vec::new();
        let settings = self.settings(theme, dark);
        let controls = if self.viewport[0] < 560. {
            Some(settings)
        } else {
            children.push(settings);
            None
        };
        let body = self.body(theme, controls);
        let bar = self.bar(theme);
        let workspace = match (self.kind, self.edge) {
            (ToolbarKind::Docked, ToolbarEdge::Top) => vec![bar, body],
            (ToolbarKind::Docked, _) => vec![body, bar],
            _ => vec![body, self.floating(bar)],
        };
        children.push(
            View::new(
                Node::new(Role::Group, "")
                    .id("toolbar-demo-workspace")
                    .styled(Style::column().height(Length::Px(0.)).grow(1.)),
            )
            .children(workspace),
        );
        if self.overflow_open {
            children.push(self.menu(theme));
        }
        View::new(
            Node::new(Role::Window, "工具列")
                .id("toolbar-demo-page")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .background(theme.colors.surface),
                ),
        )
        .children(children)
    }

    fn floating(&self, bar: View<Message>) -> View<Message> {
        let (align, justify) = match self.edge {
            ToolbarEdge::Top => (Align::Center, Align::Start),
            ToolbarEdge::Bottom => (Align::Center, Align::End),
            ToolbarEdge::Start | ToolbarEdge::End if self.floating_on_left() => {
                (Align::Start, Align::Center)
            }
            ToolbarEdge::Start | ToolbarEdge::End => (Align::End, Align::Center),
        };
        let mut style = Style::column()
            .absolute(Insets::all(16.))
            .align(align)
            .justify(justify);
        style.z_index = 10;
        View::new(
            Node::new(Role::Group, "")
                .id("toolbar-demo-floating-layer")
                .styled(style),
        )
        .children([bar])
    }

    fn floating_on_left(&self) -> bool {
        (self.edge == ToolbarEdge::Start) != self.rtl
    }

    fn body_style(&self) -> Style {
        let style = Style::column().clip(true);
        if self.viewport[0] >= 560.
            || self.kind == ToolbarKind::Docked
            || self.scroll.visibility() == ToolbarVisibility::Hidden
        {
            return style.height(Length::Px(0.)).grow(1.);
        }
        let mut insets = Insets::all(0.);
        // Leave room for the 64 dp bar, the outer margin and the scroll
        // content gap. An adjacent FAB can wrap below the compact row.
        let gap = Length::Px(
            if self.kind == ToolbarKind::FloatingHorizontal
                && self.has_fab()
                && self.scroll.visibility() == ToolbarVisibility::Expanded
            {
                152.
            } else {
                88.
            },
        );
        match self.edge {
            ToolbarEdge::Top => insets.top = gap,
            ToolbarEdge::Bottom => insets.bottom = gap,
            ToolbarEdge::Start | ToolbarEdge::End if self.floating_on_left() => insets.left = gap,
            ToolbarEdge::Start | ToolbarEdge::End => insets.right = gap,
        }
        style.absolute(insets)
    }

    fn bar(&self, theme: &Theme) -> View<Message> {
        let button = |id, label, icon, selected, disabled, event| {
            theme.toolbar_button(
                ToolbarButtonOptions {
                    id,
                    label,
                    icon,
                    palette: self.palette,
                    selected,
                    disabled,
                },
                app(event),
            )
        };
        let leading = button(
            "toolbar-demo-lead",
            "返回",
            if self.rtl { "→" } else { "←" },
            None,
            false,
            ToolbarEvent::Action,
        );
        let core = button(
            "toolbar-demo-core",
            "釘選",
            "★",
            Some(self.selected),
            false,
            ToolbarEvent::Toggle,
        );
        let inactive = button(
            "toolbar-demo-disabled",
            "尚未開放",
            "×",
            None,
            true,
            ToolbarEvent::Action,
        );
        let extra = button(
            "toolbar-demo-extra",
            "分享",
            "↑",
            None,
            false,
            ToolbarEvent::Action,
        );
        let menu = button(
            OVERFLOW,
            "更多動作",
            "…",
            None,
            false,
            ToolbarEvent::Overflow(true),
        )
        .disclosure_expanded(self.overflow_open);
        let fab = self.has_fab().then(|| {
            theme
                .fab(
                    FAB,
                    "新增",
                    "+",
                    FabSize::Regular,
                    FabColor::Primary,
                    false,
                    app(ToolbarEvent::Action),
                )
                .accessible_activation()
        });
        theme.toolbar(
            ToolbarOptions {
                id: BAR,
                name: "編輯工具列",
                kind: self.kind,
                palette: self.palette,
                direction: if self.rtl {
                    LayoutDirection::RightToLeft
                } else {
                    LayoutDirection::LeftToRight
                },
                visibility: self.scroll.visibility(),
                restore_focus_to: BODY_ACTION,
            },
            ToolbarContent {
                leading: vec![leading],
                core: vec![core, inactive],
                trailing: vec![extra, menu],
                fab,
            },
        )
    }

    fn settings(&self, theme: &Theme, dark: bool) -> View<Message> {
        use ToolbarEdge::{Bottom, End, Start, Top};
        use ToolbarKind::{Docked, FloatingHorizontal, FloatingVertical};
        use ToolbarPalette::{Standard, Vibrant};
        use ToolbarScrollBehavior::{CollapseOnScroll, HideOnScroll, Pinned};
        let choose = |id, label, selected, event| {
            theme
                .segmented_button(id, label, choice(selected), app(event))
                .accessible_activation()
        };
        let display = theme.row([
            theme
                .tonal_button(BACK, "返回", app(ToolbarEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme.switch("theme", "深色", choice(dark), Message::Theme),
            choose(
                "toolbar-demo-rtl",
                "右至左",
                self.rtl,
                ToolbarEvent::Direction,
            ),
            choose(
                "toolbar-demo-scale",
                "大字",
                self.large_text,
                ToolbarEvent::TextScale,
            ),
        ]);
        let kinds = [
            (Docked, "toolbar-demo-docked", "停駐"),
            (FloatingHorizontal, "toolbar-demo-horizontal", "橫向"),
            (FloatingVertical, "toolbar-demo-vertical", "直向"),
        ];
        let colors = [
            (Standard, "toolbar-demo-standard", "標準"),
            (Vibrant, "toolbar-demo-vibrant", "鮮明"),
        ];
        let edges = if self.kind == FloatingVertical {
            [
                (Start, "toolbar-demo-start", "起側"),
                (End, "toolbar-demo-end", "末側"),
            ]
        } else {
            [
                (Top, "toolbar-demo-top", "頂端"),
                (Bottom, "toolbar-demo-bottom", "底端"),
            ]
        };
        let mut modes = vec![
            (Pinned, "toolbar-demo-pinned", "固定"),
            (HideOnScroll, "toolbar-demo-hide-on-scroll", "捲動隱藏"),
        ];
        if self.kind != Docked {
            modes.push((
                CollapseOnScroll,
                "toolbar-demo-collapse-on-scroll",
                "捲動收合",
            ));
        }
        let mut extras = vec![
            theme.switch(
                "toolbar-demo-keep-visible",
                "保持顯示",
                choice(self.keep_visible),
                app(ToolbarEvent::KeepVisible),
            ),
            theme
                .tonal_button(
                    "toolbar-demo-preview",
                    match self.next_visibility() {
                        ToolbarVisibility::Expanded => "顯示",
                        ToolbarVisibility::Collapsed => "收合",
                        ToolbarVisibility::Hidden => "隱藏",
                    },
                    app(ToolbarEvent::Preview),
                )
                .accessible_activation(),
        ];
        if self.kind != Docked {
            extras.push(theme.switch(
                "toolbar-demo-fab",
                "新增按鈕",
                choice(self.fab),
                app(ToolbarEvent::PairFab),
            ));
        }
        View::new(Node::new(Role::Group, "工具列設定").styled(Style::column().padding(6.).gap(4.)))
            .children([
                display,
                theme.row(kinds.map(|(k, id, label)| {
                    choose(id, label, self.kind == k, ToolbarEvent::Kind(k))
                })),
                theme.row(colors.map(|(c, id, label)| {
                    choose(id, label, self.palette == c, ToolbarEvent::Palette(c))
                })),
                theme.row(edges.map(|(edge, id, label)| {
                    choose(id, label, self.edge == edge, ToolbarEvent::Edge(edge))
                })),
                theme.row(modes.into_iter().map(|(mode, id, label)| {
                    choose(
                        id,
                        label,
                        self.behavior == mode,
                        ToolbarEvent::Behavior(mode),
                    )
                })),
                theme.row(extras),
            ])
    }

    fn body(&self, theme: &Theme, controls: Option<View<Message>>) -> View<Message> {
        let status = theme.text(
            "toolbar-demo-status",
            format!(
                "工具列動作 {} 次，開啟項目 {} 次",
                self.actions, self.body_actions
            ),
        );
        let open = theme.row([theme
            .tonal_button(BODY_ACTION, "開啟項目", app(ToolbarEvent::Body))
            .accessible_activation()]);
        let rows = (0..24).map(|row| {
            let label = ["設計紀錄", "測試結果", "發佈進度"][row % 3];
            View::new(
                Node::new(Role::Group, "")
                    .id(format!("toolbar-demo-row-{row}"))
                    .styled(
                        Style::column()
                            .padding(12.)
                            .background(theme.colors.surface_container_low),
                    ),
            )
            .children([theme.heading(&format!("toolbar-demo-row-title-{row}"), label)])
        });
        let motion = self.scroll;
        let behavior = self.behavior;
        let keep = self.keep_visible;
        let items =
            View::new(Node::new(Role::Group, "").styled(Style::column().padding(8.).gap(6.)))
                .children(controls.into_iter().chain([status, open]).chain(rows));
        View::new(
            Node::new(Role::Group, "工作區內容")
                .id(SCROLL)
                .styled(self.body_style()),
        )
        .children([items])
        .accessibility_scroll_pages()
        .on_vertical_scroll_with_focus(move |gesture, focus| {
            motion.apply(
                gesture,
                behavior,
                keep || focus.is_some_and(toolbar_focus),
                |scroll| app(ToolbarEvent::Scroll(scroll)),
            )
        })
    }

    fn menu(&self, theme: &Theme) -> View<Message> {
        let bounds = self.overflow_bounds.unwrap_or(Rect {
            x: 12.,
            y: 12.,
            width: 48.,
            height: 48.,
        });
        let [width, height] = self.viewport;
        let right = bounds.x + bounds.width;
        let x = (if self.rtl { bounds.x } else { right - 240. }).clamp(0., (width - 240.).max(0.));
        let below = bounds.y + bounds.height + 8.;
        let y = if below + 112. <= height {
            below
        } else {
            (bounds.y - 120.).max(0.)
        };
        theme.menu_at(
            MENU,
            "其他動作",
            (x, y),
            [
                theme
                    .menu_item(
                        MENU_ACTION,
                        "重新整理",
                        false,
                        app(ToolbarEvent::MenuAction),
                    )
                    .accessible_activation(),
                theme.menu_item(
                    "toolbar-demo-overflow-disabled",
                    "尚未開放",
                    true,
                    app(ToolbarEvent::MenuAction),
                ),
            ],
            app(ToolbarEvent::Overflow(false)),
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        let (palette, container, selected_container) = match self.palette {
            ToolbarPalette::Standard => (
                "standard",
                theme.colors.surface_container,
                theme.colors.secondary_container,
            ),
            ToolbarPalette::Vibrant => (
                "vibrant",
                theme.colors.primary_container,
                theme.colors.surface_container,
            ),
        };
        serde_json::json!({
            "kind": match self.kind {
                ToolbarKind::Docked => "docked",
                ToolbarKind::FloatingHorizontal => "horizontal",
                ToolbarKind::FloatingVertical => "vertical",
            },
            "palette": palette, "container": container, "selected_container": selected_container,
            "visibility": match self.scroll.visibility() {
                ToolbarVisibility::Expanded => "expanded",
                ToolbarVisibility::Collapsed => "collapsed",
                ToolbarVisibility::Hidden => "hidden",
            },
            "edge": match self.edge {
                ToolbarEdge::Top => "top", ToolbarEdge::Bottom => "bottom",
                ToolbarEdge::Start => "start", ToolbarEdge::End => "end",
            },
            "behavior": match self.behavior {
                ToolbarScrollBehavior::Pinned => "pinned",
                ToolbarScrollBehavior::HideOnScroll => "hide-on-scroll",
                ToolbarScrollBehavior::CollapseOnScroll => "collapse-on-scroll",
            },
            "rtl": self.rtl, "fab": self.has_fab(), "selected": self.selected,
            "scrolled_offset": self.offset, "keep_visible": self.keep_visible,
            "overflow_open": self.overflow_open, "actions": self.actions, "body_actions": self.body_actions,
            "text_scale": self.text_scale(),
        })
    }
}

fn toolbar_focus(id: &str) -> bool {
    matches!(
        id,
        "toolbar-demo-lead" | "toolbar-demo-core" | "toolbar-demo-extra" | OVERFLOW | FAB
    )
}

fn choice(selected: bool) -> ChoiceState {
    ChoiceState {
        selected,
        disabled: false,
    }
}

fn app(event: ToolbarEvent) -> Message {
    Message::Toolbar(event)
}
