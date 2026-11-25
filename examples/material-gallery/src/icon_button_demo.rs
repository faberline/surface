use std::sync::OnceLock;

use surface::{Align, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, FrameInput, InputAnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChoiceState, IconButtonKind, IconButtonOptions,
    IconButtonShape, IconButtonSize, IconButtonWidth, MaterialIcon, Theme, TooltipInput,
    TooltipOptions, TooltipSide, TooltipState, VectorIcon,
};

use crate::{rail_icons, scroll_field_demo::control_row, tooltip_demo::gap_contains, Message};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "icon-button-demo-show";
const BACK: &str = "icon-button-demo-back";
const COMMAND: &str = "icon-button-demo-command";
const TOGGLE: &str = "icon-button-demo-toggle";
const TOOLTIP: &str = "icon-button-demo-tooltip";
const SCROLL: &str = "icon-button-demo-scroll";

#[derive(Clone)]
pub enum IconButtonDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    Activate(&'static str),
}

pub(crate) struct IconButtonDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    checked: bool,
    requests: usize,
    last: Option<&'static str>,
    viewport: Rect,
    tooltip: TooltipState,
    anchor: Option<Rect>,
    tooltip_size: Option<(f32, f32)>,
    reduced_motion: bool,
}

impl Default for IconButtonDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            checked: false,
            requests: 0,
            last: None,
            viewport: Rect::new(0., 0., 960., 800.),
            tooltip: TooltipState::default(),
            anchor: None,
            tooltip_size: None,
            reduced_motion: false,
        }
    }
}

impl IconButtonDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
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
        let viewport = Rect::new(0., 0., width, height);
        if self.viewport == viewport {
            return false;
        }
        self.viewport = viewport;
        self.tooltip_size = None;
        self.visible
    }

    pub(crate) fn dismiss_tooltip(&mut self) -> bool {
        self.tooltip_size = None;
        self.tooltip.dismiss()
    }

    pub(crate) fn update(&mut self, event: IconButtonDemoEvent) {
        use IconButtonDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => {
                self.rtl = !self.rtl;
                self.tooltip_size = None;
            }
            ToggleLargeText => {
                self.large_text = !self.large_text;
                self.tooltip_size = None;
            }
            Activate(id) => {
                self.requests += 1;
                self.last = Some(id);
                if id == TOGGLE {
                    self.checked = !self.checked;
                }
            }
        }
    }

    pub(crate) fn frame(
        &mut self,
        frame: &AnimationFrame<'_>,
        input: FrameInput<'_>,
        pressed: Option<&str>,
        hovered: Option<&str>,
    ) -> InputAnimationUpdate {
        if !self.visible {
            return InputAnimationUpdate::default();
        }
        let geometry = |id| frame.layout?.nodes.get(frame.dom.by_id(id)?.index());
        self.reduced_motion = frame.reduced_motion;
        let mut update = InputAnimationUpdate::default();
        if frame.layout.is_some() {
            let anchor = geometry(COMMAND)
                .map(|target| target.bounds.intersect(target.clip))
                .filter(|rect| rect.width > 0. && rect.height > 0.);
            update.animation.rebuild = self.tooltip.active().is_some() && anchor != self.anchor;
            self.anchor = anchor;
        }

        let active = self.tooltip.active().is_some();
        let popup = active
            .then(|| geometry(TOOLTIP).map(|tooltip| tooltip.bounds))
            .flatten()
            .filter(|rect| rect.width > 0. && rect.height > 0.);
        let can_show = self.anchor.is_some() && pressed != Some(COMMAND);
        let popup_active = self.tooltip_size.is_some()
            && input.pointer.is_some_and(|point| {
                popup.is_some_and(|popup| {
                    popup.contains(point.0, point.1)
                        || self
                            .anchor
                            .is_some_and(|anchor| gap_contains(anchor, popup, point))
                })
            });
        let touch_anchor = input.touch.and_then(|touch| {
            let dx = touch.position.0 - touch.origin.0;
            let dy = touch.position.1 - touch.origin.1;
            let target = geometry(COMMAND)?.bounds;
            (self.anchor.is_some()
                && dx * dx + dy * dy <= 64.
                && target.contains(touch.origin.0, touch.origin.1))
            .then_some(COMMAND)
        });
        let tooltip = self.tooltip.update(TooltipInput {
            now_ms: frame.now_ms,
            hovered_anchor: hovered.filter(|id| can_show && *id == COMMAND),
            focused_anchor: input.focused.filter(|id| can_show && *id == COMMAND),
            popup_active,
            touching: input.touch.is_some(),
            touch_anchor,
            touch_started_ms: input.touch.and_then(|touch| touch.started_ms),
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

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "圖示按鈕範例",
                ButtonKind::Outlined,
                false,
                Message::IconButtonDemo(IconButtonDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn popup(&self, theme: &Theme) -> Option<View<Message>> {
        self.tooltip.active()?;
        Some(theme.plain_tooltip(TooltipOptions {
            id: TOOLTIP,
            text: "儲存目前對話",
            title: None,
            anchor: self.anchor?,
            viewport: self.viewport,
            preferred: TooltipSide::Above,
            direction: self.direction(),
            caret: false,
            restore_focus_to: COMMAND,
            measured_size: self.tooltip_size,
        }))
    }

    fn specimen_row(
        &self,
        id: &str,
        children: impl IntoIterator<Item = View<Message>>,
    ) -> View<Message> {
        let node = Node::new(Role::Group, "").id(id).styled(
            Style::row()
                .width(Length::Percent(1.))
                .align(Align::Center)
                .gap(16.)
                .wrap(true),
        );
        View::new(if self.rtl { node.reverse_row() } else { node }).children(children)
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use IconButtonDemoEvent::*;
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
                .tonal_button(BACK, "返回", Message::IconButtonDemo(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            toggle("icon-button-demo-theme", "深色", dark, Message::Theme),
            toggle(
                "icon-button-demo-rtl",
                "右起",
                self.rtl,
                Message::IconButtonDemo(ToggleDirection),
            ),
            toggle(
                "icon-button-demo-large-text",
                "大字",
                self.large_text,
                Message::IconButtonDemo(ToggleLargeText),
            ),
        ]);

        let assets = [
            MaterialIcon::Command(rail_icons::Destination::Archive.icon()),
            MaterialIcon::Toggle {
                checked: self.checked,
                off: star_outline(),
                on: star_fill(),
            },
            MaterialIcon::Command(lock()),
            MaterialIcon::Command(rail_icons::search()),
            MaterialIcon::Command(rail_icons::more()),
            MaterialIcon::Command(rail_icons::Destination::Inbox.icon()),
        ];
        let [command, toggle, disabled, medium, large, extra_large] =
            std::array::from_fn(|index| {
                let options = SAMPLES[index];
                theme
                    .icon_button_with_options(
                        options,
                        assets[index],
                        Message::IconButtonDemo(Activate(options.id)),
                    )
                    .expect("valid Gallery icon button")
            });
        let mut scroll_style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .gap(16.)
            .padding(6.)
            .clip(true);
        scroll_style.max_width = Length::Px(640.);
        let preview = View::new(Node::new(Role::Group, "").id(SCROLL).styled(scroll_style))
            .accessibility_scroll_pages()
            .children([
                self.specimen_row(
                    "icon-button-demo-compact",
                    [command, toggle, disabled, medium],
                ),
                self.specimen_row("icon-button-demo-sizes", [large, extra_large]),
            ]);
        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("icon-button-demo-route")
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
        .children(
            [
                theme.heading("icon-button-demo-title", "圖示按鈕"),
                controls,
                preview,
            ]
            .into_iter()
            .chain(self.popup(theme)),
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        let color = |surface::Color(r, g, b, a)| [r, g, b, a];
        let c = theme.colors;
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "checked": self.checked,
            "requests": self.requests,
            "last": self.last,
            "width": self.viewport.width,
            "height": self.viewport.height,
            "reduced_motion": self.reduced_motion,
            "tooltip_shown": self.tooltip.active().is_some() && self.tooltip_size.is_some() && self.anchor.is_some(),
            "palette": {
                "background": color(theme.background),
                "primary": color(c.primary),
                "on_primary": color(c.on_primary),
                "secondary": color(c.secondary),
                "on_secondary": color(c.on_secondary),
                "secondary_container": color(c.secondary_container),
                "on_secondary_container": color(c.on_secondary_container),
                "inverse_surface": color(c.inverse_surface),
                "inverse_on_surface": color(c.inverse_on_surface),
                "outline_variant": color(c.outline_variant),
                "on_surface": color(c.on_surface),
                "on_surface_variant": color(c.on_surface_variant),
                "surface_container": color(c.surface_container)
            }
        })
    }
}

const SAMPLES: [IconButtonOptions<'static>; 6] = [
    IconButtonOptions {
        id: COMMAND,
        name: "儲存",
        description: Some(TOOLTIP),
        kind: IconButtonKind::Filled,
        size: IconButtonSize::ExtraSmall,
        width: IconButtonWidth::Narrow,
        shape: IconButtonShape::Round,
        disabled: false,
    },
    IconButtonOptions {
        id: TOGGLE,
        name: "加星號",
        description: None,
        kind: IconButtonKind::Tonal,
        size: IconButtonSize::Small,
        width: IconButtonWidth::Wide,
        shape: IconButtonShape::Round,
        disabled: false,
    },
    IconButtonOptions {
        id: "icon-button-demo-disabled",
        name: "鎖定",
        description: None,
        kind: IconButtonKind::Standard,
        size: IconButtonSize::Medium,
        width: IconButtonWidth::Narrow,
        shape: IconButtonShape::Round,
        disabled: true,
    },
    IconButtonOptions {
        id: "icon-button-demo-medium",
        name: "搜尋",
        description: None,
        kind: IconButtonKind::Outlined,
        size: IconButtonSize::Medium,
        width: IconButtonWidth::Default,
        shape: IconButtonShape::Square,
        disabled: false,
    },
    IconButtonOptions {
        id: "icon-button-demo-large",
        name: "更多選項",
        description: None,
        kind: IconButtonKind::Standard,
        size: IconButtonSize::Large,
        width: IconButtonWidth::Narrow,
        shape: IconButtonShape::Round,
        disabled: false,
    },
    IconButtonOptions {
        id: "icon-button-demo-extra-large",
        name: "收件匣",
        description: None,
        kind: IconButtonKind::Outlined,
        size: IconButtonSize::ExtraLarge,
        width: IconButtonWidth::Wide,
        shape: IconButtonShape::Square,
        disabled: false,
    },
];

const STAR: [[f32; 2]; 10] = [
    [12., 2.],
    [15., 8.5],
    [22., 9.5],
    [17., 14.5],
    [18.5, 22.],
    [12., 18.5],
    [5.5, 22.],
    [7., 14.5],
    [2., 9.5],
    [9., 8.5],
];

fn star_fill() -> &'static VectorIcon {
    static ICON: OnceLock<VectorIcon> = OnceLock::new();
    ICON.get_or_init(|| VectorIcon::new(STAR).expect("valid star contour"))
}

fn star_outline() -> &'static VectorIcon {
    static ICON: OnceLock<VectorIcon> = OnceLock::new();
    ICON.get_or_init(|| {
        let cutout = STAR.map(|point| point.map(|axis| 12. + (axis - 12.) * 0.58));
        VectorIcon::from_contours([STAR, cutout]).expect("valid outlined star contours")
    })
}

fn lock() -> &'static VectorIcon {
    static ICON: OnceLock<VectorIcon> = OnceLock::new();
    ICON.get_or_init(|| {
        let outline = [
            [5., 10.],
            [7., 10.],
            [7., 5.],
            [10., 2.],
            [14., 2.],
            [17., 5.],
            [17., 10.],
            [19., 10.],
            [19., 21.],
            [5., 21.],
        ];
        let hole = [
            [9., 10.],
            [9., 6.],
            [11., 4.],
            [13., 4.],
            [15., 6.],
            [15., 10.],
        ];
        VectorIcon::from_contours(
            [outline.as_slice(), hole.as_slice()].map(|path| path.iter().copied()),
        )
        .expect("valid lock contours")
    })
}
