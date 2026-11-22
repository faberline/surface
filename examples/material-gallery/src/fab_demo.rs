use std::sync::OnceLock;

use surface::{Align, Corners, Insets, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, FrameInput, FramePress, InputAnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChipKind, ChoiceState, ExpressiveFabAppearance,
    ExpressiveFabContent, ExpressiveFabElevation, ExpressiveFabMotion, ExpressiveFabOptions,
    ExpressiveFabSize, Theme, TooltipInput, TooltipOptions, TooltipSide, TooltipState, VectorIcon,
};

use crate::{tooltip_demo::gap_contains, Message};

pub(crate) const SECTION: usize = 1;
const SHOW: &str = "fab-demo-show";
const BACK: &str = "fab-demo-back";
const ACTION: &str = "fab-demo-action";
const FACE: &str = "fab-demo-action-face";
const RING: &str = "fab-demo-action-focus-ring";
const SCROLL: &str = "fab-demo-scroll";
const VISIBILITY: &str = "fab-demo-toggle-visibility";
const TOOLTIP: &str = "fab-demo-tooltip";
const LABEL: &str = "新增專案筆記";
const EDGE: f32 = 16.;

const SIZES: [(ExpressiveFabSize, &str, &str); 3] = [
    (ExpressiveFabSize::Regular, "regular", "一般"),
    (ExpressiveFabSize::Medium, "medium", "中"),
    (ExpressiveFabSize::Large, "large", "大"),
];

const COLORS: [(ExpressiveFabAppearance, &str, &str); 6] = [
    (
        ExpressiveFabAppearance::TonalPrimary,
        "tonal-primary",
        "柔色主色",
    ),
    (
        ExpressiveFabAppearance::TonalSecondary,
        "tonal-secondary",
        "柔色輔色",
    ),
    (
        ExpressiveFabAppearance::TonalTertiary,
        "tonal-tertiary",
        "柔色第三色",
    ),
    (ExpressiveFabAppearance::Primary, "primary", "主色"),
    (ExpressiveFabAppearance::Secondary, "secondary", "輔色"),
    (ExpressiveFabAppearance::Tertiary, "tertiary", "第三色"),
];

#[derive(Clone, Copy)]
pub enum FabEvent {
    Show,
    Back,
    Size(ExpressiveFabSize),
    Appearance(ExpressiveFabAppearance),
    Direction,
    TextScale,
    Disable,
    LowerElevation,
    Visibility,
    HideOnPress,
    Activate,
}

pub(crate) struct FabDemo {
    open: bool,
    viewport: Rect,
    size: ExpressiveFabSize,
    appearance: ExpressiveFabAppearance,
    rtl: bool,
    large_text: bool,
    disabled: bool,
    lowered: bool,
    action_visible: bool,
    hide_on_press: bool,
    actions: usize,
    scroll_y: f32,
    motion: ExpressiveFabMotion,
    reduced_motion: bool,
    tooltip: TooltipState,
    anchor: Option<Rect>,
    tooltip_size: Option<(f32, f32)>,
}

impl Default for FabDemo {
    fn default() -> Self {
        Self {
            open: false,
            viewport: Rect::new(0., 0., 960., 800.),
            size: ExpressiveFabSize::Regular,
            appearance: ExpressiveFabAppearance::TonalPrimary,
            rtl: false,
            large_text: false,
            disabled: false,
            lowered: false,
            action_visible: true,
            hide_on_press: false,
            actions: 0,
            scroll_y: 0.,
            motion: ExpressiveFabMotion::new(),
            reduced_motion: false,
            tooltip: TooltipState::default(),
            anchor: None,
            tooltip_size: None,
        }
    }
}

impl FabDemo {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn leave(&mut self) {
        self.open = false;
        self.scroll_y = 0.;
        self.motion = ExpressiveFabMotion::new().with_visibility(self.action_visible);
        self.tooltip = TooltipState::default();
        self.anchor = None;
        self.tooltip_size = None;
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let next = Rect::new(0., 0., width, height);
        if next == self.viewport {
            return false;
        }
        self.viewport = next;
        self.tooltip_size = None;
        self.open
    }

    pub(crate) fn dismiss_tooltip(&mut self) -> bool {
        self.tooltip_size = None;
        self.tooltip.dismiss()
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    pub(crate) fn frame(
        &mut self,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        pressed: Option<&str>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> InputAnimationUpdate {
        if !self.open {
            return InputAnimationUpdate::default();
        }
        if let Some(body) = frame
            .dom
            .by_id(SCROLL)
            .and_then(|id| frame.dom.get(id).ok())
        {
            self.scroll_y = body.style().scroll_y;
        }
        self.reduced_motion = frame.reduced_motion;
        let motion = self
            .motion
            .advance(ACTION, frame, input, press, hovered, self.action_visible);
        let mut update = self.tooltip_frame(frame, input, pressed == Some(ACTION), hovered);
        update.animation.rebuild |= motion.rebuild;
        update.animation.request_next |= motion.request_next;
        update
    }

    fn tooltip_frame(
        &mut self,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        pressing: bool,
        hovered: Option<&str>,
    ) -> InputAnimationUpdate {
        if self.disabled || !self.action_visible {
            return InputAnimationUpdate::default();
        }
        let bounds = |id: &str| {
            let geometry = frame.layout?.nodes.get(frame.dom.by_id(id)?.index())?;
            let rect = geometry.bounds;
            (rect.width > 0. && rect.height > 0.).then_some(rect)
        };
        let active = self.tooltip.active().is_some();
        let mut update = InputAnimationUpdate::default();
        if frame.layout.is_some() {
            let next = bounds(RING).or_else(|| bounds(FACE));
            update.animation.rebuild |= active && next != self.anchor;
            self.anchor = next;
        }

        let face = bounds(FACE).or(self.anchor);
        let owns = |(x, y)| face.is_some_and(|rect| rect.contains(x, y));
        let pointer = input.pointer.filter(|_| !pressing);
        let hovered_anchor = hovered.filter(|id| !pressing && *id == ACTION);
        let touch_anchor = input.touch.and_then(|contact| {
            let dx = contact.position.0 - contact.origin.0;
            let dy = contact.position.1 - contact.origin.1;
            (dx * dx + dy * dy <= 64. && owns(contact.origin)).then_some(ACTION)
        });
        let popup = active.then(|| bounds(TOOLTIP)).flatten();
        let popup_active = self.tooltip_size.is_some()
            && pointer.is_some_and(|(x, y)| {
                popup.is_some_and(|popup| {
                    if popup.contains(x, y) {
                        return true;
                    }
                    self.anchor
                        .is_some_and(|anchor| gap_contains(anchor, popup, (x, y)))
                })
            });
        let tooltip = self.tooltip.update(TooltipInput {
            now_ms: frame.now_ms,
            hovered_anchor,
            focused_anchor: input.focused.filter(|id| !pressing && *id == ACTION),
            popup_active,
            touching: input.touch.is_some(),
            touch_anchor,
            touch_started_ms: input.touch.and_then(|contact| contact.started_ms),
            touch_cancelled: input.touch_cancelled,
        });
        if tooltip.changed {
            self.tooltip_size = None;
            update.animation.rebuild = true;
        } else if let Some(popup) = popup {
            let size = (popup.width, popup.height);
            update.animation.rebuild |= self.tooltip_size.replace(size) != Some(size);
        }
        update.animation.request_next = tooltip.request_next
            || (self.tooltip.active().is_some()
                && self.anchor.is_some()
                && self.tooltip_size.is_none());
        update.suppress_touch_activation = tooltip.suppress_touch_activation;
        update
    }

    pub(crate) fn update(&mut self, event: FabEvent) {
        match event {
            FabEvent::Show => self.open = true,
            FabEvent::Back => self.leave(),
            FabEvent::Size(size) => self.size = size,
            FabEvent::Appearance(appearance) => self.appearance = appearance,
            FabEvent::Direction => self.rtl = !self.rtl,
            FabEvent::TextScale => {
                self.large_text = !self.large_text;
                self.tooltip_size = None;
            }
            FabEvent::Disable => {
                self.disabled = !self.disabled;
                self.dismiss_tooltip();
            }
            FabEvent::LowerElevation => self.lowered = !self.lowered,
            FabEvent::Visibility => {
                self.action_visible = !self.action_visible;
                self.dismiss_tooltip();
            }
            FabEvent::HideOnPress => self.hide_on_press = !self.hide_on_press,
            FabEvent::Activate if self.action_visible && !self.disabled => {
                self.actions += 1;
                self.action_visible = !self.hide_on_press;
                self.dismiss_tooltip();
            }
            FabEvent::Activate => {}
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "圖示浮動按鈕",
                ButtonKind::Outlined,
                false,
                app(FabEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let header = self.row("顯示設定").children([
            theme
                .tonal_button(BACK, "返回", app(FabEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            chip(theme, "fab-demo-theme", "深色", dark, Message::Theme),
            chip(
                theme,
                "fab-demo-rtl",
                "右起",
                self.rtl,
                app(FabEvent::Direction),
            ),
            chip(
                theme,
                "fab-demo-large-text",
                "大字",
                self.large_text,
                app(FabEvent::TextScale),
            ),
        ]);
        let mut settings_style = Style::column().clip(true);
        settings_style.max_height = Length::Px((self.viewport.height * 0.38).max(64.));
        let settings = View::new(
            Node::new(Role::Group, "圖示浮動按鈕設定")
                .id("fab-demo-settings")
                .styled(settings_style),
        )
        .children([self.settings(theme)])
        .accessibility_scroll_pages();

        View::new(
            Node::new(Role::Window, "圖示浮動按鈕")
                .id("fab-demo-page")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .background(theme.colors.surface),
                ),
        )
        .children(
            [header, settings, self.body(theme), self.anchor(theme)]
                .into_iter()
                .chain(self.popup(theme)),
        )
    }

    fn row(&self, name: &str) -> View<Message> {
        let row = Node::new(Role::Group, name).styled(
            Style::row()
                .padding(8.)
                .gap(8.)
                .align(Align::Center)
                .wrap(true),
        );
        View::new(if self.rtl { row.reverse_row() } else { row })
    }

    fn settings(&self, theme: &Theme) -> View<Message> {
        let switches = [
            (
                VISIBILITY,
                "顯示按鈕",
                self.action_visible,
                FabEvent::Visibility,
            ),
            (
                "fab-demo-disable",
                "停用按鈕",
                self.disabled,
                FabEvent::Disable,
            ),
            (
                "fab-demo-lowered",
                "低浮層",
                self.lowered,
                FabEvent::LowerElevation,
            ),
            (
                "fab-demo-hide-on-press",
                "執行後隱藏",
                self.hide_on_press,
                FabEvent::HideOnPress,
            ),
        ];
        let sizes = SIZES.into_iter().map(|(size, slug, label)| {
            chip(
                theme,
                &format!("fab-demo-size-{slug}"),
                label,
                self.size == size,
                app(FabEvent::Size(size)),
            )
        });
        let colors = COLORS.into_iter().map(|(color, slug, label)| {
            chip(
                theme,
                &format!("fab-demo-color-{slug}"),
                label,
                self.appearance == color,
                app(FabEvent::Appearance(color)),
            )
        });
        View::new(Node::new(Role::Group, "").styled(Style::column())).children([
            self.row("操作").children(
                switches.into_iter().map(|(id, label, selected, event)| {
                    chip(theme, id, label, selected, app(event))
                }),
            ),
            self.row("尺寸").children(sizes),
            self.row("色彩").children(colors),
        ])
    }

    fn body(&self, theme: &Theme) -> View<Message> {
        let header = theme.heading("fab-demo-notes", "專案筆記");
        let status = (self.actions != 0).then(|| {
            let message = format!("已新增 {} 則筆記", self.actions);
            View::new(
                Node::new(Role::Status, message.clone())
                    .id("fab-demo-status")
                    .text(message)
                    .styled(Style::column().foreground(theme.colors.on_surface)),
            )
        });
        let notes = (1..=24).map(|index| {
            let title = ["設計紀錄", "測試結果", "發佈進度"][(index - 1) % 3];
            View::new(
                Node::new(Role::Group, "").styled(
                    Style::column()
                        .padding(12.)
                        .corners(Corners::all(12.))
                        .background(theme.colors.surface_container_low),
                ),
            )
            .children([theme.text(
                &format!("fab-demo-note-{index}"),
                format!("{title} {index}"),
            )])
        });
        let bottom =
            View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(112.))));
        let list =
            View::new(Node::new(Role::Group, "").styled(Style::column().padding(16.).gap(8.)))
                .children(
                    std::iter::once(header)
                        .chain(status)
                        .chain(notes)
                        .chain([bottom]),
                );
        View::new(
            Node::new(Role::Group, "工作區筆記")
                .id(SCROLL)
                .styled(Style::column().height(Length::Px(0.)).grow(1.).clip(true)),
        )
        .children([list])
        .accessibility_scroll_pages()
    }

    fn anchor(&self, theme: &Theme) -> View<Message> {
        let width = (self.viewport.width - 2. * EDGE).max(1.);
        let mut style = Style::row()
            .width(Length::Px(width))
            .justify(if self.rtl { Align::Start } else { Align::End })
            .fixed(Insets {
                left: Length::Px(EDGE),
                bottom: Length::Px(EDGE),
                ..Insets::default()
            });
        style.z_index = 10;
        View::new(
            Node::new(Role::Group, "")
                .id("fab-demo-anchor")
                .styled(style),
        )
        .children([theme.expressive_fab(
            ExpressiveFabOptions {
                id: ACTION,
                name: LABEL,
                size: self.size,
                appearance: self.appearance,
                elevation: if self.lowered {
                    ExpressiveFabElevation::Lowered
                } else {
                    ExpressiveFabElevation::Standard
                },
                visible: self.action_visible,
                disabled: self.disabled,
                restore_focus_to: Some(VISIBILITY),
            },
            ExpressiveFabContent {
                icon: add_icon(),
                on_press: app(FabEvent::Activate),
            },
            &self.motion,
        )])
    }

    fn popup(&self, theme: &Theme) -> Option<View<Message>> {
        self.tooltip.active()?;
        Some(theme.plain_tooltip(TooltipOptions {
            id: TOOLTIP,
            text: LABEL,
            title: None,
            anchor: self.anchor?,
            viewport: self.viewport,
            preferred: TooltipSide::Above,
            direction: self.direction(),
            caret: false,
            restore_focus_to: ACTION,
            measured_size: self.tooltip_size,
        }))
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        let ripple = self.motion.ripple().map(|ripple| {
            serde_json::json!({
                "center": ripple.center,
                "progress": ripple.progress,
                "opacity": ripple.opacity,
            })
        });
        serde_json::json!({
            "open": self.open,
            "size": SIZES.iter().find(|(size, ..)| self.size == *size).map(|(_, slug, _)| *slug),
            "palette": COLORS.iter().find(|(color, ..)| self.appearance == *color).map(|(_, slug, _)| *slug),
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "disabled": self.disabled,
            "lowered": self.lowered,
            "visible": self.action_visible,
            "hide_on_press": self.hide_on_press,
            "scroll_y": self.scroll_y,
            "actions": self.actions,
            "tooltip_shown": self.tooltip.active().is_some() && self.tooltip_size.is_some(),
            "visibility_scale": self.motion.visibility_scale(),
            "visibility_opacity": self.motion.visibility_opacity(),
            "reduced_motion": self.reduced_motion,
            "ripple": ripple,
        })
    }
}

fn app(event: FabEvent) -> Message {
    Message::Fab(event)
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

pub(crate) fn add_icon() -> VectorIcon {
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
