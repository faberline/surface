use surface::{Align, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, InputAnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChoiceState, Theme, TooltipInput, TooltipOptions,
    TooltipSide, TooltipState,
};

use crate::Message;

pub(crate) const SECTION: usize = 2;
const SHOW: &str = "tooltip-demo-show";
const BACK: &str = "tooltip-demo-back";
const PLAIN: &str = "tooltip-demo-plain";
const RICH: &str = "tooltip-demo-rich";
const PLAIN_POPUP: &str = "tooltip-demo-plain-popup";
const RICH_POPUP: &str = "tooltip-demo-rich-popup";
const ACTION: &str = "tooltip-demo-action";
const PLAIN_TEXT: &str = "複製目前所選的文字。";
const RICH_TITLE: &str = "分享設定";
const RICH_TEXT: &str = "你可以在分享之前查看誰能存取這個工作區。";
const ANCHORS: [&str; 2] = [PLAIN, RICH];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TooltipAlignment {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy)]
pub enum TooltipEvent {
    Show,
    Back,
    Top(bool),
    Alignment(TooltipAlignment),
    Direction,
    TextScale,
    Caret,
    Copy,
    OpenRich,
    Action,
    Body,
}

pub(crate) struct TooltipDemo {
    visible: bool,
    top: bool,
    alignment: TooltipAlignment,
    rtl: bool,
    large_text: bool,
    caret: bool,
    viewport: [f32; 2],
    anchors: [Option<Rect>; 2],
    popup: Option<Rect>,
    measured_size: Option<(f32, f32)>,
    state: TooltipState,
    copies: usize,
    actions: usize,
    body_actions: usize,
}

impl Default for TooltipDemo {
    fn default() -> Self {
        Self {
            visible: false,
            top: false,
            alignment: TooltipAlignment::Center,
            rtl: false,
            large_text: false,
            caret: true,
            viewport: [960., 800.],
            anchors: [None; 2],
            popup: None,
            measured_size: None,
            state: TooltipState::default(),
            copies: 0,
            actions: 0,
            body_actions: 0,
        }
    }
}

impl TooltipDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.state = TooltipState::default();
        self.invalidate_layout();
    }

    fn invalidate_layout(&mut self) {
        self.anchors = [None; 2];
        self.popup = None;
        self.measured_size = None;
    }

    pub(crate) fn dismiss(&mut self) -> bool {
        let dismissed = self.state.dismiss();
        self.popup = None;
        self.measured_size = None;
        dismissed
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
        if self.viewport == next {
            return false;
        }
        self.viewport = next;
        self.invalidate_layout();
        self.visible
    }

    fn viewport_rect(&self) -> Rect {
        Rect {
            x: 0.,
            y: 0.,
            width: self.viewport[0],
            height: self.viewport[1],
        }
    }

    pub(crate) fn frame(
        &mut self,
        frame: &AnimationFrame<'_>,
        input: FrameInput<'_>,
    ) -> InputAnimationUpdate {
        if !self.visible {
            return InputAnimationUpdate::default();
        }
        let rect = |id: &str| {
            let id = frame.dom.by_id(id)?;
            let layout = frame.layout?;
            let geometry = layout.nodes.get(id.index())?;
            let bounds = geometry.bounds;
            (bounds.width > 0. && bounds.height > 0.).then_some(bounds)
        };
        let mut rebuild = false;
        if frame.layout.is_some() {
            let anchors = ANCHORS.map(rect);
            rebuild |= anchors != self.anchors && self.state.active().is_some();
            self.anchors = anchors;
            self.popup = self
                .state
                .active()
                .and_then(|anchor| rect(popup_id(anchor)));
        }
        let hover = input.pointer.and_then(|point| self.anchor_at(point));
        let focus = input.focused.filter(|key| ANCHORS.contains(key));
        let touch_anchor = input.touch.and_then(|contact| {
            let dx = contact.position.0 - contact.origin.0;
            let dy = contact.position.1 - contact.origin.1;
            if dx * dx + dy * dy <= 64. {
                self.anchor_at(contact.origin)
            } else {
                None
            }
        });
        let active_rect = self.state.active().and_then(|key| self.anchor_rect(key));
        let popup_active = self.measured_size.is_some()
            && self.popup.is_some_and(|popup| {
                input.focused == Some(ACTION)
                    || input.pointer.is_some_and(|point| {
                        popup.contains(point.0, point.1)
                            || active_rect.is_some_and(|anchor| gap_contains(anchor, popup, point))
                    })
            });
        let update = self.state.update(TooltipInput {
            now_ms: frame.now_ms,
            hovered_anchor: hover,
            focused_anchor: focus,
            popup_active,
            touching: input.touch.is_some(),
            touch_anchor,
            touch_started_ms: input.touch.and_then(|contact| contact.started_ms),
            touch_cancelled: input.touch_cancelled,
        });
        if update.changed {
            self.popup = None;
            self.measured_size = None;
            rebuild = true;
        } else if self.state.active().is_some() {
            if let Some(bounds) = self.popup {
                let size = (bounds.width, bounds.height);
                if self.measured_size != Some(size) {
                    self.measured_size = Some(size);
                    rebuild = true;
                }
            }
        }
        InputAnimationUpdate {
            animation: AnimationUpdate {
                rebuild,
                request_next: update.request_next
                    || self.anchors.iter().all(Option::is_none)
                    || (self.state.active().is_some() && self.measured_size.is_none()),
            },
            suppress_touch_activation: update.suppress_touch_activation,
        }
    }

    fn anchor_rect(&self, key: &str) -> Option<Rect> {
        let index = ANCHORS.iter().position(|anchor| *anchor == key)?;
        self.anchors[index]
    }

    fn anchor_at(&self, point: (f32, f32)) -> Option<&'static str> {
        ANCHORS
            .into_iter()
            .zip(self.anchors)
            .find_map(|(name, rect)| {
                rect.filter(|bounds| bounds.contains(point.0, point.1))
                    .map(|_| name)
            })
    }

    pub(crate) fn update(&mut self, event: TooltipEvent) {
        match event {
            TooltipEvent::Show => self.visible = true,
            TooltipEvent::Back => self.leave(),
            TooltipEvent::Top(top) => {
                self.top = top;
                self.invalidate_layout();
            }
            TooltipEvent::Alignment(alignment) => {
                self.alignment = alignment;
                self.invalidate_layout();
            }
            TooltipEvent::Direction => {
                self.rtl = !self.rtl;
                self.invalidate_layout();
            }
            TooltipEvent::TextScale => {
                self.large_text = !self.large_text;
                self.invalidate_layout();
            }
            TooltipEvent::Caret => {
                self.caret = !self.caret;
                self.measured_size = None;
            }
            TooltipEvent::Copy => {
                self.copies += 1;
            }
            TooltipEvent::OpenRich => {
                self.state.activate(RICH);
                self.measured_size = None;
            }
            TooltipEvent::Action => {
                self.actions += 1;
                self.dismiss();
            }
            TooltipEvent::Body => {
                self.body_actions += 1;
                self.dismiss();
            }
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "提示泡泡",
                ButtonKind::Outlined,
                false,
                app(TooltipEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let mut anchors = self.triggers(theme);
        let popup = self.popup(theme);
        anchors.extend(popup);
        let mut children = Vec::new();
        let settings = self.settings(theme, dark);
        let spacer = View::new(Node::new(Role::Group, "").styled(Style::column().grow(1.)));
        if self.top {
            children.extend(anchors);
            children.push(settings);
            children.push(spacer);
        } else {
            children.push(settings);
            children.push(spacer);
            children.extend(anchors);
        }
        View::new(
            Node::new(Role::Window, "提示泡泡")
                .id("tooltip-demo-page")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .padding(8.)
                        .gap(8.)
                        .background(theme.colors.surface),
                ),
        )
        .children(children)
    }

    fn triggers(&self, theme: &Theme) -> Vec<View<Message>> {
        let side = match self.alignment {
            TooltipAlignment::Center => Align::Center,
            TooltipAlignment::Start => Align::Start,
            TooltipAlignment::End => Align::End,
        };
        let mut row = Node::new(Role::Group, "")
            .id("tooltip-demo-anchor-row")
            .styled(Style::row().align(Align::Center).justify(side).gap(8.));
        if self.rtl {
            row = row.reverse_row();
        }
        vec![View::new(row).children([
            theme
                .tonal_button(PLAIN, "複製", app(TooltipEvent::Copy))
                .described_by(PLAIN_POPUP)
                .accessible_activation(),
            theme
                .tonal_button(RICH, "分享", app(TooltipEvent::OpenRich))
                .disclosure_expanded(
                    self.state.active() == Some(RICH) && self.measured_size.is_some(),
                )
                .accessible_activation(),
        ])]
    }

    fn settings(&self, theme: &Theme, dark: bool) -> View<Message> {
        use TooltipAlignment::{Center, End, Start};
        let control = |id, label, selected, event| {
            theme
                .segmented_button(id, label, choice(selected), app(event))
                .accessible_activation()
        };
        View::new(Node::new(Role::Group, "提示設定").styled(Style::column().gap(6.))).children([
            theme.row([
                theme
                    .tonal_button(BACK, "返回", app(TooltipEvent::Back))
                    .restore_focus_on_removal(SHOW)
                    .accessible_activation(),
                theme.switch("theme", "深色", choice(dark), Message::Theme),
                control(
                    "tooltip-demo-rtl",
                    "右至左",
                    self.rtl,
                    TooltipEvent::Direction,
                ),
                control(
                    "tooltip-demo-scale",
                    "大字",
                    self.large_text,
                    TooltipEvent::TextScale,
                ),
            ]),
            theme.row([
                control(
                    "tooltip-demo-top",
                    "頂端",
                    self.top,
                    TooltipEvent::Top(true),
                ),
                control(
                    "tooltip-demo-bottom",
                    "底端",
                    !self.top,
                    TooltipEvent::Top(false),
                ),
                control(
                    "tooltip-demo-start",
                    "起側",
                    self.alignment == Start,
                    TooltipEvent::Alignment(Start),
                ),
                control(
                    "tooltip-demo-center",
                    "置中",
                    self.alignment == Center,
                    TooltipEvent::Alignment(Center),
                ),
                control(
                    "tooltip-demo-end",
                    "末側",
                    self.alignment == End,
                    TooltipEvent::Alignment(End),
                ),
            ]),
            theme.row([
                theme.switch(
                    "tooltip-demo-caret",
                    "指向標記",
                    choice(self.caret),
                    app(TooltipEvent::Caret),
                ),
                theme
                    .tonal_button(
                        "tooltip-demo-body-action",
                        "查看項目",
                        app(TooltipEvent::Body),
                    )
                    .accessible_activation(),
            ]),
            theme.text(
                "tooltip-demo-status",
                format!(
                    "複製 {} 次，選項 {} 次，查看 {} 次",
                    self.copies, self.actions, self.body_actions
                ),
            ),
        ])
    }

    fn popup(&self, theme: &Theme) -> Option<View<Message>> {
        let anchor = self.state.active()?;
        let rich = anchor == RICH;
        let options = TooltipOptions {
            id: popup_id(anchor),
            text: if rich { RICH_TEXT } else { PLAIN_TEXT },
            title: rich.then_some(RICH_TITLE),
            anchor: self.anchor_rect(anchor)?,
            viewport: self.viewport_rect(),
            preferred: TooltipSide::Above,
            direction: if self.rtl {
                LayoutDirection::RightToLeft
            } else {
                LayoutDirection::LeftToRight
            },
            caret: self.caret,
            restore_focus_to: anchor,
            measured_size: self.measured_size,
        };
        Some(if rich {
            theme.rich_tooltip(
                options,
                Some(
                    theme
                        .tonal_button(ACTION, "查看選項", app(TooltipEvent::Action))
                        .accessible_activation(),
                ),
            )
        } else {
            theme.plain_tooltip(options)
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        let placement = self.state.active().and_then(|anchor| {
            surface_material::place_tooltip(
                self.anchor_rect(anchor)?,
                self.measured_size?,
                self.viewport_rect(),
                TooltipSide::Above,
            )
        });
        serde_json::json!({
            "active": self.state.active(),
            "shown": self.measured_size.is_some() && self.state.active().is_some(),
            "side": placement.map(|value| match value.side {
                TooltipSide::Above => "above", TooltipSide::Below => "below",
            }),
            "top": self.top, "rtl": self.rtl, "text_scale": self.text_scale(), "caret": self.caret,
            "copies": self.copies, "actions": self.actions, "body_actions": self.body_actions,
            "plain_surface": theme.colors.inverse_surface, "rich_surface": theme.colors.surface_container,
        })
    }
}

fn gap_contains(anchor: Rect, popup: Rect, (x, y): (f32, f32)) -> bool {
    let left = anchor.x.max(popup.x);
    let right = (anchor.x + anchor.width).min(popup.x + popup.width);
    let above = popup.y + popup.height <= anchor.y && y >= popup.y + popup.height && y <= anchor.y;
    let below =
        anchor.y + anchor.height <= popup.y && y >= anchor.y + anchor.height && y <= popup.y;
    x >= left && x <= right && (above || below)
}

fn popup_id(anchor: &str) -> &'static str {
    match anchor {
        RICH => RICH_POPUP,
        _ => PLAIN_POPUP,
    }
}

fn choice(selected: bool) -> ChoiceState {
    ChoiceState {
        selected,
        disabled: false,
    }
}

fn app(event: TooltipEvent) -> Message {
    Message::Tooltip(event)
}
