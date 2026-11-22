use std::sync::OnceLock;

use surface::{Align, Corners, Insets, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChipKind, ChoiceState, ExtendedFabAppearance,
    ExtendedFabContent, ExtendedFabElevation, ExtendedFabMotion, ExtendedFabOptions,
    ExtendedFabScroll, ExtendedFabSize, Theme, VectorIcon,
};

use crate::Message;

pub(crate) const SECTION: usize = 1;
const SHOW: &str = "extended-fab-demo-show";
const BACK: &str = "extended-fab-back";
const ACTION: &str = "extended-fab-action";
const SCROLL: &str = "extended-fab-scroll";
const VISIBILITY: &str = "extended-fab-toggle-visibility";
const LABEL: &str = "新增專案筆記";
const EDGE: f32 = 16.;

const SIZES: [(ExtendedFabSize, &str, &str); 3] = [
    (ExtendedFabSize::Small, "small", "小"),
    (ExtendedFabSize::Medium, "medium", "中"),
    (ExtendedFabSize::Large, "large", "大"),
];

const COLORS: [(ExtendedFabAppearance, &str, &str); 6] = [
    (
        ExtendedFabAppearance::TonalPrimary,
        "tonal-primary",
        "柔色主色",
    ),
    (
        ExtendedFabAppearance::TonalSecondary,
        "tonal-secondary",
        "柔色輔色",
    ),
    (
        ExtendedFabAppearance::TonalTertiary,
        "tonal-tertiary",
        "柔色第三色",
    ),
    (ExtendedFabAppearance::Primary, "primary", "主色"),
    (ExtendedFabAppearance::Secondary, "secondary", "輔色"),
    (ExtendedFabAppearance::Tertiary, "tertiary", "第三色"),
];

#[derive(Clone, Copy)]
pub enum ExtendedFabEvent {
    Show,
    Back,
    Size(ExtendedFabSize),
    Appearance(ExtendedFabAppearance),
    Direction,
    TextScale,
    TextOnly,
    Expand,
    Visibility,
    Disable,
    AutoScroll,
    Center,
    HideOnPress,
    Lowered,
    Scroll(ExtendedFabScroll),
    Activate,
}

pub(crate) struct ExtendedFabDemo {
    open: bool,
    viewport: [f32; 2],
    size: ExtendedFabSize,
    appearance: ExtendedFabAppearance,
    rtl: bool,
    large_text: bool,
    text_only: bool,
    visible: bool,
    disabled: bool,
    auto_scroll: bool,
    center: bool,
    hide_on_press: bool,
    lowered: bool,
    scroll: ExtendedFabScroll,
    scroll_y: f32,
    actions: usize,
    motion: ExtendedFabMotion,
    reduced_motion: bool,
}

impl Default for ExtendedFabDemo {
    fn default() -> Self {
        Self {
            open: false,
            viewport: [960., 800.],
            size: ExtendedFabSize::Small,
            appearance: ExtendedFabAppearance::TonalPrimary,
            rtl: false,
            large_text: false,
            text_only: false,
            visible: true,
            disabled: false,
            auto_scroll: true,
            center: false,
            hide_on_press: false,
            lowered: false,
            scroll: ExtendedFabScroll::new(true),
            scroll_y: 0.,
            actions: 0,
            motion: ExtendedFabMotion::new(true, true),
            reduced_motion: false,
        }
    }
}

impl ExtendedFabDemo {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn leave(&mut self) {
        self.open = false;
        self.scroll = self.scroll.with_expanded(true);
        self.scroll_y = 0.;
        self.motion = ExtendedFabMotion::new(true, self.visible);
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
        changed && self.open
    }

    fn expanded(&self) -> bool {
        self.text_only || self.scroll.expanded()
    }

    pub(crate) fn frame(
        &mut self,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        pressed: Option<&str>,
    ) -> AnimationUpdate {
        if !self.open {
            return AnimationUpdate::default();
        }
        if let Some(scroll) = frame
            .dom
            .by_id(SCROLL)
            .and_then(|id| frame.dom.get(id).ok())
        {
            self.scroll_y = scroll.style().scroll_y;
        }
        self.reduced_motion = frame.reduced_motion;
        let expanded = self.expanded();
        self.motion
            .advance(ACTION, frame, input, pressed, expanded, self.visible)
    }

    pub(crate) fn update(&mut self, event: ExtendedFabEvent) {
        match event {
            ExtendedFabEvent::Show => self.open = true,
            ExtendedFabEvent::Back => self.leave(),
            ExtendedFabEvent::Size(size) => self.size = size,
            ExtendedFabEvent::Appearance(appearance) => self.appearance = appearance,
            ExtendedFabEvent::Direction => self.rtl = !self.rtl,
            ExtendedFabEvent::TextScale => self.large_text = !self.large_text,
            ExtendedFabEvent::TextOnly => self.text_only = !self.text_only,
            ExtendedFabEvent::Expand => {
                self.scroll = self.scroll.with_expanded(!self.scroll.expanded())
            }
            ExtendedFabEvent::Visibility => self.visible = !self.visible,
            ExtendedFabEvent::Disable => self.disabled = !self.disabled,
            ExtendedFabEvent::AutoScroll => {
                self.auto_scroll = !self.auto_scroll;
                self.scroll = self.scroll.with_expanded(self.scroll.expanded());
            }
            ExtendedFabEvent::Center => self.center = !self.center,
            ExtendedFabEvent::HideOnPress => self.hide_on_press = !self.hide_on_press,
            ExtendedFabEvent::Lowered => self.lowered = !self.lowered,
            ExtendedFabEvent::Scroll(scroll) => self.scroll = scroll,
            ExtendedFabEvent::Activate if self.visible && !self.disabled => {
                self.actions += 1;
                self.visible = !self.hide_on_press;
            }
            ExtendedFabEvent::Activate => {}
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "延伸浮動按鈕",
                ButtonKind::Outlined,
                false,
                app(ExtendedFabEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let header = self.row("顯示設定").children([
            theme
                .tonal_button(BACK, "返回", app(ExtendedFabEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            chip(theme, "extended-fab-theme", "深色", dark, Message::Theme),
            chip(
                theme,
                "extended-fab-rtl",
                "右起",
                self.rtl,
                app(ExtendedFabEvent::Direction),
            ),
            chip(
                theme,
                "extended-fab-large-text",
                "大字",
                self.large_text,
                app(ExtendedFabEvent::TextScale),
            ),
        ]);
        let mut settings_style = Style::column().clip(true);
        settings_style.max_height = Length::Px((self.viewport[1] * 0.38).max(64.));
        let settings = View::new(
            Node::new(Role::Group, "浮動按鈕設定")
                .id("extended-fab-settings")
                .styled(settings_style),
        )
        .children([self.settings(theme)])
        .accessibility_scroll_pages();
        View::new(
            Node::new(Role::Window, "延伸浮動按鈕")
                .id("extended-fab-page")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .background(theme.colors.surface),
                ),
        )
        .children([header, settings, self.body(theme), self.anchor(theme)])
    }

    fn row(&self, name: &str) -> View<Message> {
        let node = Node::new(Role::Group, name).styled(
            Style::row()
                .padding(8.)
                .gap(8.)
                .align(Align::Center)
                .wrap(true),
        );
        View::new(if self.rtl { node.reverse_row() } else { node })
    }

    fn settings(&self, theme: &Theme) -> View<Message> {
        let toggles = [
            (
                VISIBILITY,
                "顯示按鈕",
                self.visible,
                ExtendedFabEvent::Visibility,
            ),
            (
                "extended-fab-toggle-expand",
                "展開",
                self.expanded(),
                ExtendedFabEvent::Expand,
            ),
            (
                "extended-fab-text-only",
                "純文字",
                self.text_only,
                ExtendedFabEvent::TextOnly,
            ),
            (
                "extended-fab-auto-scroll",
                "隨捲動收合",
                self.auto_scroll,
                ExtendedFabEvent::AutoScroll,
            ),
            (
                "extended-fab-disable",
                "停用按鈕",
                self.disabled,
                ExtendedFabEvent::Disable,
            ),
            (
                "extended-fab-center",
                "底部置中",
                self.center,
                ExtendedFabEvent::Center,
            ),
            (
                "extended-fab-hide-on-press",
                "執行後隱藏",
                self.hide_on_press,
                ExtendedFabEvent::HideOnPress,
            ),
            (
                "extended-fab-lowered",
                "低浮層",
                self.lowered,
                ExtendedFabEvent::Lowered,
            ),
        ];
        let toggles = toggles
            .into_iter()
            .map(|(id, label, selected, event)| chip(theme, id, label, selected, app(event)));
        let sizes = SIZES.into_iter().map(|(size, slug, label)| {
            chip(
                theme,
                &format!("extended-fab-size-{slug}"),
                label,
                self.size == size,
                app(ExtendedFabEvent::Size(size)),
            )
        });
        let colors = COLORS.into_iter().map(|(color, slug, label)| {
            chip(
                theme,
                &format!("extended-fab-color-{slug}"),
                label,
                self.appearance == color,
                app(ExtendedFabEvent::Appearance(color)),
            )
        });
        View::new(Node::new(Role::Group, "").styled(Style::column())).children([
            self.row("操作").children(toggles),
            self.row("尺寸").children(sizes),
            self.row("色彩").children(colors),
        ])
    }

    fn body(&self, theme: &Theme) -> View<Message> {
        let header = theme.heading("extended-fab-notes", "專案筆記");
        let status = (self.actions != 0).then(|| {
            let message = format!("已新增 {} 則筆記", self.actions);
            View::new(
                Node::new(Role::Status, message.clone())
                    .id("extended-fab-status")
                    .text(message)
                    .styled(Style::column().foreground(theme.colors.on_surface)),
            )
        });
        let notes = (0..24).map(|index| {
            let name = ["設計紀錄", "測試結果", "發佈進度"][index % 3];
            View::new(
                Node::new(Role::Group, "").styled(
                    Style::column()
                        .padding(12.)
                        .corners(Corners::all(12.))
                        .background(theme.colors.surface_container_low),
                ),
            )
            .children([theme.text(&format!("extended-fab-note-{index}"), name)])
        });
        let tail =
            View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(104.))));
        let items =
            View::new(Node::new(Role::Group, "").styled(Style::column().padding(16.).gap(8.)))
                .children(
                    std::iter::once(header)
                        .chain(status)
                        .chain(notes)
                        .chain([tail]),
                );
        let scroll = self.scroll;
        let auto = self.auto_scroll;
        let text_only = self.text_only;
        View::new(
            Node::new(Role::Group, "工作區筆記")
                .id(SCROLL)
                .styled(Style::column().height(Length::Px(0.)).grow(1.).clip(true)),
        )
        .children([items])
        .accessibility_scroll_pages()
        .on_vertical_scroll_with_focus(move |gesture, _| {
            if auto {
                scroll.apply(gesture, text_only, |next| {
                    app(ExtendedFabEvent::Scroll(next))
                })
            } else {
                None
            }
        })
    }

    fn anchor(&self, theme: &Theme) -> View<Message> {
        let width = (self.viewport[0] - 2. * EDGE).max(1.);
        let placement = if self.center {
            Align::Center
        } else if self.rtl && self.viewport[0] < 840. {
            Align::Start
        } else {
            Align::End
        };
        let mut style = Style::row()
            .width(Length::Px(width))
            .justify(placement)
            .fixed(Insets {
                left: Length::Px(EDGE),
                bottom: Length::Px(EDGE),
                ..Insets::default()
            });
        style.z_index = 10;
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        View::new(
            Node::new(Role::Group, "")
                .id("extended-fab-anchor")
                .styled(style),
        )
        .children([theme.extended_fab(
            ExtendedFabOptions {
                id: ACTION,
                label: LABEL,
                name: None,
                size: self.size,
                appearance: self.appearance,
                elevation: if self.lowered {
                    ExtendedFabElevation::Lowered
                } else {
                    ExtendedFabElevation::Standard
                },
                direction,
                available_width: width,
                visible: self.visible,
                disabled: self.disabled,
                restore_focus_to: Some(VISIBILITY),
            },
            ExtendedFabContent {
                icon: (!self.text_only).then(add_icon),
                on_press: app(ExtendedFabEvent::Activate),
            },
            &self.motion,
        )])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        serde_json::json!({
            "open": self.open,
            "size": SIZES.iter().find(|(size, ..)| self.size == *size).map(|(_, slug, _)| *slug),
            "palette": COLORS.iter().find(|(color, ..)| self.appearance == *color).map(|(_, slug, _)| *slug),
            "text_only": self.text_only,
            "expanded": self.expanded(),
            "requested_expanded": self.scroll.expanded(),
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "disabled": self.disabled,
            "scroll_y": self.scroll_y,
            "auto_scroll": self.auto_scroll,
            "center": self.center,
            "hide_on_press": self.hide_on_press,
            "lowered": self.lowered,
            "actions": self.actions,
            "expansion": self.motion.expansion(),
            "label_opacity": self.motion.label_opacity(),
            "visibility_scale": self.motion.visibility_scale(),
            "visibility_opacity": self.motion.visibility_opacity(),
            "reduced_motion": self.reduced_motion,
        })
    }
}

fn app(event: ExtendedFabEvent) -> Message {
    Message::ExtendedFab(event)
}

fn chip(theme: &Theme, id: &str, label: &str, selected: bool, message: Message) -> View<Message> {
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
}

fn add_icon() -> VectorIcon {
    static ICON: OnceLock<VectorIcon> = OnceLock::new();
    ICON.get_or_init(|| {
        VectorIcon::new([
            [11., 5.],
            [13., 5.],
            [13., 11.],
            [19., 11.],
            [19., 13.],
            [13., 13.],
            [13., 19.],
            [11., 19.],
            [11., 13.],
            [5., 13.],
            [5., 11.],
            [11., 11.],
        ])
        .expect("valid plus icon")
    })
    .clone()
}
