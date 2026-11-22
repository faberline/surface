use surface::{Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, AppBarContent, AppBarKind, AppBarOptions, AppBarScroll,
    AppBarScrollBehavior, AppBarScrollUpdate, AppBarSearchContent, AppBarSearchOptions, ButtonKind,
    ChoiceState, Theme,
};

use crate::Message;

pub(crate) const SECTION: usize = 1;
const SHOW: &str = "app-bar-demo-show";
const BACK: &str = "app-bar-demo-back";
const BAR: &str = "app-bar-demo-bar";
const SCROLL: &str = "app-bar-demo-scroll";
const SEARCH: &str = "app-bar-demo-search";
const ENTRY: &str = "app-bar-demo-search-entry";
const CONTEXT: &str = "app-bar-demo-context";
const TITLE: &str = "工作區總覽";
const SUBTITLE: &str = "專案與最近更新";
const RESULTS: [(&str, &str); 3] = [
    ("app-bar-demo-search-result", "設計紀錄"),
    ("app-bar-demo-search-result-tests", "測試結果"),
    ("app-bar-demo-search-result-releases", "發佈進度"),
];

#[derive(Clone)]
pub enum AppBarEvent {
    Show,
    Back,
    Kind(AppBarKind),
    Behavior(AppBarScrollBehavior),
    Subtitle,
    Direction,
    TextScale,
    Preview,
    Scroll(AppBarScrollUpdate),
    OpenSearch,
    CloseSearch,
    Query(String),
    Select(&'static str),
    SelectContext(bool),
    Leading,
    Action,
}

pub(crate) struct AppBarDemo {
    visible: bool,
    kind: AppBarKind,
    behavior: AppBarScrollBehavior,
    scroll: AppBarScroll,
    body_offset: f32,
    width: f32,
    rtl: bool,
    large_text: bool,
    subtitle: bool,
    search_open: bool,
    query: String,
    selected: Option<&'static str>,
    contextual: bool,
    actions: usize,
    search_selections: usize,
}

impl Default for AppBarDemo {
    fn default() -> Self {
        Self {
            visible: false,
            kind: AppBarKind::Small,
            behavior: AppBarScrollBehavior::Pinned,
            scroll: AppBarScroll::default(),
            body_offset: 0.,
            width: 960.,
            rtl: false,
            large_text: false,
            subtitle: true,
            search_open: false,
            query: String::new(),
            selected: None,
            contextual: false,
            actions: 0,
            search_selections: 0,
        }
    }
}

impl AppBarDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.search_open = false;
        self.contextual = false;
        self.body_offset = 0.;
        self.scroll = AppBarScroll::default();
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
        changed && self.visible
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        let Some(body) = frame
            .dom
            .by_id(SCROLL)
            .and_then(|id| frame.dom.get(id).ok())
        else {
            return AnimationUpdate::default();
        };
        let offset = body.style().scroll_y;
        let rebuild = self.scroll.scrolled() != self.scroll.with_content_offset(offset).scrolled();
        self.body_offset = offset;
        self.scroll = self.scroll.with_content_offset(offset);
        AnimationUpdate {
            rebuild,
            request_next: false,
        }
    }

    pub(crate) fn update(&mut self, event: AppBarEvent) {
        match event {
            AppBarEvent::Show => self.visible = true,
            AppBarEvent::Back => self.leave(),
            AppBarEvent::Kind(kind) => {
                self.kind = kind;
                if self.options().collapse_distance() == 0. {
                    self.behavior = AppBarScrollBehavior::Pinned;
                }
                self.scroll = AppBarScroll::new(0., self.body_offset);
            }
            AppBarEvent::Behavior(behavior) => {
                self.behavior = behavior;
                self.scroll = AppBarScroll::new(0., self.body_offset);
            }
            AppBarEvent::Subtitle => self.subtitle = !self.subtitle,
            AppBarEvent::Direction => self.rtl = !self.rtl,
            AppBarEvent::TextScale => self.large_text = !self.large_text,
            AppBarEvent::Preview => {
                let collapse = if self.scroll.collapse() > 0. { 0. } else { 1. };
                self.scroll = AppBarScroll::new(collapse, self.body_offset);
            }
            AppBarEvent::Scroll(update) => {
                self.scroll = update.bar;
                self.body_offset = update.content_offset;
            }
            AppBarEvent::OpenSearch => self.search_open = true,
            AppBarEvent::CloseSearch => self.search_open = false,
            AppBarEvent::Query(query) => self.query = query,
            AppBarEvent::Select(label) => {
                self.selected = Some(label);
                self.search_open = false;
                self.search_selections += 1;
            }
            AppBarEvent::SelectContext(selected) => self.contextual = selected,
            AppBarEvent::Leading => self.selected = None,
            AppBarEvent::Action => self.actions += 1,
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "頂部應用列",
                ButtonKind::Outlined,
                false,
                app(AppBarEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let options = self.options();
        let mut children = vec![theme.app_bar(options, self.bar_content(theme))];
        let controls = self.controls(theme, dark);
        let scroll_controls = if self.width < 560. {
            Some(controls)
        } else {
            children.push(controls);
            None
        };
        children.push(self.scroll_view(theme, options, scroll_controls));
        if self.search_open {
            children.push(self.search_view(theme));
        }
        View::new(
            Node::new(Role::Window, "頂部應用列")
                .id("app-bar-demo-page")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .background(theme.colors.surface),
                ),
        )
        .children(children)
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn options(&self) -> AppBarOptions<'static> {
        AppBarOptions {
            id: BAR,
            title: if self.contextual {
                "已選取 1 項"
            } else {
                TITLE
            },
            subtitle: (self.subtitle
                && !self.contextual
                && matches!(
                    self.kind,
                    AppBarKind::MediumFlexible | AppBarKind::LargeFlexible
                ))
            .then_some(SUBTITLE),
            kind: if self.contextual {
                AppBarKind::Small
            } else {
                self.kind
            },
            direction: self.direction(),
            collapse: if self.contextual {
                0.
            } else {
                self.scroll.collapse()
            },
            scrolled: self.scroll.scrolled(),
        }
    }

    fn bar_content(&self, theme: &Theme) -> AppBarContent<Message> {
        let icon = |id, label, glyph, disabled, event| {
            theme.app_bar_icon_button(id, label, glyph, disabled, app(event))
        };
        if self.contextual {
            return AppBarContent {
                leading: Some(
                    icon(
                        "app-bar-demo-context-close",
                        "取消選取",
                        "×",
                        false,
                        AppBarEvent::SelectContext(false),
                    )
                    .restore_focus_on_removal(CONTEXT),
                ),
                actions: vec![icon(
                    "app-bar-demo-context-action",
                    "套用選取",
                    "✓",
                    false,
                    AppBarEvent::SelectContext(false),
                )
                .restore_focus_on_removal(CONTEXT)],
                search: None,
            };
        }
        let back = if self.rtl { "→" } else { "←" };
        AppBarContent {
            leading: Some(icon(
                "app-bar-demo-leading",
                "返回專案",
                back,
                false,
                AppBarEvent::Leading,
            )),
            actions: vec![
                icon(
                    "app-bar-demo-action",
                    "重新整理",
                    "↻",
                    false,
                    AppBarEvent::Action,
                ),
                icon(
                    "app-bar-demo-disabled",
                    "尚未開放",
                    "…",
                    true,
                    AppBarEvent::Action,
                ),
            ],
            search: (self.kind == AppBarKind::Search).then(|| {
                theme
                    .app_bar_search_entry_in_direction(
                        ENTRY,
                        "搜尋工作區",
                        self.direction(),
                        app(AppBarEvent::OpenSearch),
                    )
                    .disclosure_expanded(self.search_open)
            }),
        }
    }

    fn controls(&self, theme: &Theme, dark: bool) -> View<Message> {
        use AppBarKind::{CenterAligned, LargeFlexible, MediumFlexible, Search, Small};
        use AppBarScrollBehavior::{EnterAlways, ExitUntilCollapsed, Pinned};
        let control = |id, label, selected, event| {
            theme
                .segmented_button(id, label, choice(selected), app(event))
                .accessible_activation()
        };
        let variant =
            |id, label, kind| control(id, label, self.kind == kind, AppBarEvent::Kind(kind));
        let behavior = |id, label, mode| {
            control(
                id,
                label,
                self.behavior == mode,
                AppBarEvent::Behavior(mode),
            )
        };
        let mut variants = vec![
            variant("app-bar-demo-small", "小型", Small),
            variant("app-bar-demo-center", "置中", CenterAligned),
            variant("app-bar-demo-medium", "中型", MediumFlexible),
            variant("app-bar-demo-large", "大型", LargeFlexible),
            variant("app-bar-demo-search", "搜尋", Search),
        ];
        let more_variants = (self.width < 560.).then(|| theme.row(variants.split_off(3)));
        let tools = theme.row([
            theme
                .tonal_button(BACK, "返回", app(AppBarEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme.switch("theme", "深色", choice(dark), Message::Theme),
            control(
                "app-bar-demo-rtl",
                "右至左",
                self.rtl,
                AppBarEvent::Direction,
            ),
            control(
                "app-bar-demo-scale",
                "大字",
                self.large_text,
                AppBarEvent::TextScale,
            ),
        ]);
        let flexible = self.options().collapse_distance() > 0.;
        let mut behaviors = vec![behavior("app-bar-demo-pinned", "固定", Pinned)];
        if flexible {
            behaviors.extend([
                behavior("app-bar-demo-enter-always", "反向展開", EnterAlways),
                behavior(
                    "app-bar-demo-exit-until-collapsed",
                    "頂端展開",
                    ExitUntilCollapsed,
                ),
            ]);
        }
        let mut children = vec![tools, theme.row(variants)];
        children.extend(more_variants);
        children.push(theme.row(behaviors));
        if flexible {
            children.push(
                theme.row([
                    theme.switch(
                        "app-bar-demo-subtitle",
                        "副標題",
                        choice(self.subtitle),
                        app(AppBarEvent::Subtitle),
                    ),
                    theme
                        .tonal_button(
                            "app-bar-demo-preview",
                            if self.scroll.collapse() > 0. {
                                "展開"
                            } else {
                                "收合"
                            },
                            app(AppBarEvent::Preview),
                        )
                        .accessible_activation(),
                ]),
            );
        }
        View::new(Node::new(Role::Group, "應用列設定").styled(Style::column().padding(6.).gap(4.)))
            .children(children)
    }

    fn scroll_view(
        &self,
        theme: &Theme,
        options: AppBarOptions<'_>,
        controls: Option<View<Message>>,
    ) -> View<Message> {
        let contents = (0..24).map(|row| {
            let title = ["設計紀錄", "測試結果", "發佈進度"][row % 3];
            let mut items = vec![theme.heading(&format!("app-bar-demo-row-title-{row}"), title)];
            if row == 0 {
                items.push(
                    theme.row([theme
                        .tonal_button(
                            CONTEXT,
                            "選取設計紀錄",
                            app(AppBarEvent::SelectContext(true)),
                        )
                        .accessible_activation()]),
                );
            }
            View::new(
                Node::new(Role::Group, "")
                    .id(format!("app-bar-demo-row-{row}"))
                    .styled(
                        Style::column()
                            .padding(12.)
                            .gap(4.)
                            .background(theme.colors.surface_container_low),
                    ),
            )
            .children(items)
        });
        let status = match self.selected {
            Some(label) => format!("目前項目 {label}"),
            None => format!("重新整理 {} 次", self.actions),
        };
        let motion = self.scroll;
        let behavior = self.behavior;
        let distance = options.collapse_distance();
        View::new(
            Node::new(Role::Group, "工作區內容")
                .id(SCROLL)
                .styled(Style::column().height(Length::Px(0.)).grow(1.).clip(true)),
        )
        .children([View::new(
            Node::new(Role::Group, "").styled(Style::column().padding(8.).gap(6.)),
        )
        .children(
            controls
                .into_iter()
                .chain(std::iter::once(theme.text("app-bar-demo-status", status)))
                .chain(contents),
        )])
        .accessibility_scroll_pages()
        .on_vertical_scroll(move |event| {
            motion.apply(event, behavior, distance, |update| {
                app(AppBarEvent::Scroll(update))
            })
        })
    }

    fn search_view(&self, theme: &Theme) -> View<Message> {
        let matches: Vec<_> = RESULTS
            .into_iter()
            .filter(|(_, label)| label.contains(self.query.trim()))
            .map(|(id, label)| {
                theme
                    .tonal_button(id, label, app(AppBarEvent::Select(label)))
                    .accessible_activation()
            })
            .collect();
        let results = if matches.is_empty() {
            vec![theme.text("app-bar-demo-search-empty", "找不到相符的項目")]
        } else {
            matches
        };
        theme.app_bar_search_view(
            AppBarSearchOptions {
                id: SEARCH,
                title: "搜尋工作區",
                input_label: "搜尋內容",
                close_label: "關閉搜尋",
                clear_label: "清除搜尋",
                query: &self.query,
                direction: self.direction(),
            },
            AppBarSearchContent {
                results: View::new(
                    Node::new(Role::Group, "").styled(Style::column().padding(12.).gap(8.)),
                )
                .children(results),
                on_close: app(AppBarEvent::CloseSearch),
                on_clear: app(AppBarEvent::Query(String::new())),
            },
            |query| app(AppBarEvent::Query(query)),
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        let kind = match self.kind {
            AppBarKind::Small => "small",
            AppBarKind::CenterAligned => "center",
            AppBarKind::MediumFlexible => "medium",
            AppBarKind::LargeFlexible => "large",
            AppBarKind::Search => "search",
        };
        let behavior = match self.behavior {
            AppBarScrollBehavior::Pinned => "pinned",
            AppBarScrollBehavior::EnterAlways => "enter-always",
            AppBarScrollBehavior::ExitUntilCollapsed => "exit-until-collapsed",
        };
        serde_json::json!({
            "kind": kind, "behavior": behavior, "collapse": self.scroll.collapse(),
            "scrolled": self.scroll.scrolled(), "body_offset": self.body_offset,
            "rtl": self.rtl, "text_scale": self.text_scale(), "subtitle": self.subtitle,
            "search_open": self.search_open, "query": self.query, "actions": self.actions,
            "search_selections": self.search_selections, "selected": self.selected,
            "contextual": self.contextual, "surface": theme.colors.surface,
            "scrolled_surface": theme.colors.surface_container,
        })
    }
}

fn choice(selected: bool) -> ChoiceState {
    ChoiceState {
        selected,
        disabled: false,
    }
}

fn app(event: AppBarEvent) -> Message {
    Message::AppBar(event)
}
