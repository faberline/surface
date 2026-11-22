use surface::{Align, Insets, Length, Node, Role, Style};
use surface_app::{AnimationFrame, VerticalDrag, VerticalDragSession, View};
use surface_material::{
    shape::{self, LayoutDirection},
    ButtonKind, CardAction, CardContent, CardInteraction, CardKind, ChipKind, ChoiceState,
    MaterialCardOptions, Theme,
};

use crate::Message;

pub(crate) const SECTION: usize = 3;
const SHOW: &str = "card-demo-show";
const BACK: &str = "card-demo-back";
const SCROLL: &str = "card-demo-scroll";
const PRIMARY: &str = "card-primary";
const SECONDARY: &str = "card-secondary";
const DISABLED: &str = "card-disabled";
const MOVE: &str = "card-move";
const TOGGLE: &str = "card-toggle-mode";
const DISABLE: &str = "card-disable";
const MOVE_THRESHOLD: f32 = 64.;
const PREVIEW_LIMIT: f32 = 104.;
const KINDS: [(CardKind, &str, &str); 3] = [
    (CardKind::Elevated, "elevated", "浮起"),
    (CardKind::Filled, "filled", "填色"),
    (CardKind::Outlined, "outlined", "描邊"),
];

#[derive(Clone)]
pub enum CardEvent {
    Show,
    Back,
    Kind(CardKind),
    Direction,
    TextScale,
    ToggleMode,
    Disable,
    Activate,
    NestedAction,
    Move,
    Drag(VerticalDrag),
}

pub(crate) struct CardDemo {
    open: bool,
    kind: CardKind,
    rtl: bool,
    large_text: bool,
    toggle_mode: bool,
    disabled: bool,
    selected: bool,
    activations: usize,
    nested_activations: usize,
    moves: usize,
    position: usize,
    drag_offset: Option<f32>,
    height: Option<f32>,
    session: VerticalDragSession,
}

impl Default for CardDemo {
    fn default() -> Self {
        Self {
            open: false,
            kind: CardKind::Elevated,
            rtl: false,
            large_text: false,
            toggle_mode: false,
            disabled: false,
            selected: false,
            activations: 0,
            nested_activations: 0,
            moves: 0,
            position: 0,
            drag_offset: None,
            height: None,
            session: VerticalDragSession::new(),
        }
    }
}

impl CardDemo {
    pub(crate) fn visible(&self) -> bool {
        self.open
    }

    pub(crate) fn leave(&mut self) {
        self.open = false;
        self.drag_offset = None;
        self.height = None;
        self.session = VerticalDragSession::new();
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) {
        if self.open && self.drag_offset.is_none() {
            self.height = frame.dom.by_id(SECONDARY).and_then(|id| {
                let height = frame.layout?.nodes.get(id.index())?.bounds.height;
                (height > 0.).then_some(height)
            });
        }
    }

    fn move_sign(&self) -> f32 {
        if self.position == 0 {
            1.
        } else {
            -1.
        }
    }

    fn move_card(&mut self) {
        self.position = 1 - self.position;
        self.moves += 1;
    }

    pub(crate) fn update(&mut self, event: CardEvent) {
        match event {
            CardEvent::Show => self.open = true,
            CardEvent::Back => self.leave(),
            CardEvent::Kind(kind) => self.kind = kind,
            CardEvent::Direction => self.rtl = !self.rtl,
            CardEvent::TextScale => self.large_text = !self.large_text,
            CardEvent::ToggleMode => self.toggle_mode = !self.toggle_mode,
            CardEvent::Disable => self.disabled = !self.disabled,
            CardEvent::Activate if !self.disabled => {
                self.activations += 1;
                if self.toggle_mode {
                    self.selected = !self.selected;
                }
            }
            CardEvent::Activate => {}
            CardEvent::NestedAction => self.nested_activations += 1,
            CardEvent::Move => self.move_card(),
            CardEvent::Drag(VerticalDrag::Move { delta_y }) => {
                let sign = self.move_sign();
                self.drag_offset = Some((delta_y * sign).clamp(0., PREVIEW_LIMIT) * sign);
            }
            CardEvent::Drag(VerticalDrag::End { delta_y, .. }) => {
                self.drag_offset = None;
                if delta_y * self.move_sign() >= MOVE_THRESHOLD {
                    self.move_card();
                }
            }
            CardEvent::Drag(VerticalDrag::Cancel) => self.drag_offset = None,
            CardEvent::Drag(_) => {}
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "卡片",
                ButtonKind::Outlined,
                false,
                app(CardEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn options<'a>(&self, id: &'a str, name: &'a str) -> MaterialCardOptions<'a> {
        MaterialCardOptions {
            id,
            name,
            kind: self.kind,
            disabled: false,
            direction: if self.rtl {
                LayoutDirection::RightToLeft
            } else {
                LayoutDirection::LeftToRight
            },
            interaction: CardInteraction::Rest,
            restore_focus_to: Some(DISABLE),
        }
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let primary = self.primary(theme);
        let secondary = self.secondary(theme);
        let mut cards = if self.position == 0 {
            vec![secondary, primary]
        } else {
            vec![primary, secondary]
        };
        cards.push(self.disabled_card(theme));
        let mut column = Style::column()
            .width(Length::Percent(1.))
            .padding(16.)
            .gap(16.);
        column.max_width = Length::Px(576.);
        let panel = View::new(
            Node::new(Role::Group, "行程卡片")
                .id("card-demo-list")
                .styled(column),
        )
        .children(cards);
        let scroll = View::new(
            Node::new(Role::Group, "").id(SCROLL).styled(
                Style::column()
                    .height(Length::Px(0.))
                    .grow(1.)
                    .align(Align::Center)
                    .clip(true),
            ),
        )
        .accessibility_scroll_pages()
        .children([panel]);
        View::new(
            Node::new(Role::Window, "卡片").id("card-demo-page").styled(
                Style::column()
                    .height(Length::Percent(1.))
                    .background(theme.colors.surface),
            ),
        )
        .children([self.settings(theme, dark), scroll])
    }

    fn settings(&self, theme: &Theme, dark: bool) -> View<Message> {
        let back = theme
            .tonal_button(BACK, "返回", app(CardEvent::Back))
            .restore_focus_on_removal(SHOW)
            .accessible_activation();
        let display = [
            back,
            chip(theme, "card-theme", "深色", dark, Message::Theme),
            chip(
                theme,
                "card-direction",
                "右起",
                self.rtl,
                app(CardEvent::Direction),
            ),
            chip(
                theme,
                "card-large-text",
                "大字",
                self.large_text,
                app(CardEvent::TextScale),
            ),
        ];
        let cards = KINDS.iter().map(|&(kind, slug, label)| {
            chip(
                theme,
                &format!("card-{slug}"),
                label,
                kind == self.kind,
                app(CardEvent::Kind(kind)),
            )
        });
        let options = [
            chip(
                theme,
                TOGGLE,
                "選取",
                self.toggle_mode,
                app(CardEvent::ToggleMode),
            ),
            chip(
                theme,
                DISABLE,
                "停用",
                self.disabled,
                app(CardEvent::Disable),
            ),
        ];
        View::new(Node::new(Role::Group, "卡片設定").styled(Style::column().padding(8.).gap(6.)))
            .children([
                self.row("顯示設定").children(display),
                self.row("卡片樣式").children(cards.chain(options)),
            ])
    }

    fn row(&self, name: &str) -> View<Message> {
        let mut row = Node::new(Role::Group, name)
            .styled(Style::row().gap(8.).align(Align::Center).wrap(true));
        if self.rtl {
            row = row.reverse_row();
        }
        View::new(row)
    }

    fn primary(&self, theme: &Theme) -> View<Message> {
        let action = if self.toggle_mode {
            CardAction::Toggle {
                checked: self.selected,
                on_toggle: app(CardEvent::Activate),
            }
        } else {
            CardAction::Press(app(CardEvent::Activate))
        };
        let supporting = if self.toggle_mode && self.selected {
            "已選取這段行程。"
        } else if self.activations == 0 {
            "查看兩天的健行安排。"
        } else {
            "行程已開啟。"
        };
        let content = CardContent {
            title: Some("山徑週末"),
            subtitle: None,
            supporting: Some(supporting),
            media: Some(landscape(theme, self.rtl)),
            body: None,
        };
        theme.material_action_card(
            MaterialCardOptions {
                disabled: self.disabled,
                ..self.options(PRIMARY, "山徑週末")
            },
            content,
            action,
        )
    }

    fn secondary(&self, theme: &Theme) -> View<Message> {
        let card = theme.material_card(
            MaterialCardOptions {
                interaction: if self.drag_offset.is_some() {
                    CardInteraction::Dragged
                } else {
                    CardInteraction::Rest
                },
                ..self.options(SECONDARY, "行程草稿")
            },
            CardContent {
                title: Some("行程草稿"),
                subtitle: Some("離線可用"),
                supporting: Some(if self.nested_activations == 0 {
                    "可拖曳「移動」調整順序。"
                } else {
                    "已開啟行程草稿。"
                }),
                media: None,
                body: None,
            },
            [
                theme
                    .tonal_button("card-nested-action", "開啟", app(CardEvent::NestedAction))
                    .accessible_activation(),
                theme
                    .button_with(
                        MOVE,
                        "移動",
                        ButtonKind::Outlined,
                        false,
                        app(CardEvent::Move),
                    )
                    .accessible_activation()
                    .on_vertical_drag(self.session.clone(), |drag| app(CardEvent::Drag(drag))),
            ],
        );
        let mut slot = Style::column().width(Length::Percent(1.));
        let content = if let Some((offset, height)) = self.drag_offset.zip(self.height) {
            slot = slot.height(Length::Px(height));
            slot.z_index = 3;
            let position = Style::column().width(Length::Percent(1.)).absolute(Insets {
                left: Length::Px(0.),
                top: Length::Px(offset),
                ..Insets::default()
            });
            View::new(Node::new(Role::Group, "").styled(position)).children([card])
        } else {
            View::new(Node::new(Role::Group, "")).children([card])
        };
        View::new(Node::new(Role::Group, "").id("card-drag-slot").styled(slot)).children([content])
    }

    fn disabled_card(&self, theme: &Theme) -> View<Message> {
        theme.material_action_card(
            MaterialCardOptions {
                disabled: true,
                ..self.options(DISABLED, "封存行程")
            },
            CardContent {
                title: Some("封存行程"),
                subtitle: None,
                supporting: Some("封存的行程暫時無法開啟。"),
                media: None,
                body: None,
            },
            CardAction::Press(app(CardEvent::Activate)),
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        let variant = KINDS
            .iter()
            .find(|(kind, ..)| self.kind == *kind)
            .map(|(_, slug, _)| *slug);
        serde_json::json!({
            "open": self.open,
            "variant": variant,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "activations": self.activations,
            "selected": self.selected,
            "nested_activations": self.nested_activations,
            "moves": self.moves,
            "position": self.position,
            "dragging": self.drag_offset.is_some(),
            "drag_offset": self.drag_offset.unwrap_or(0.),
            "toggle_mode": self.toggle_mode,
            "disabled": self.disabled,
        })
    }
}

fn landscape(theme: &Theme, rtl: bool) -> View<Message> {
    let shape_size = |width, height| {
        Style::column()
            .width(Length::Px(width))
            .height(Length::Px(height))
    };
    let sun = View::new(
        Node::new(Role::Group, "").styled(
            shape_size(20., 20.)
                .corners(shape::FULL)
                .background(theme.colors.tertiary),
        ),
    );
    let ridge = View::new(
        Node::new(Role::Group, "").styled(
            shape_size(88., 30.)
                .corners(shape::LARGE_TOP)
                .background(theme.colors.primary),
        ),
    );
    let mut sky = Node::new(Role::Group, "").styled(
        Style::row()
            .height(Length::Px(52.))
            .align(Align::End)
            .justify(Align::Center)
            .gap(12.)
            .corners(shape::SMALL)
            .background(theme.colors.primary_container),
    );
    if rtl {
        sky = sky.reverse_row();
    }
    View::new(sky).children([sun, ridge])
}

fn app(event: CardEvent) -> Message {
    Message::Card(event)
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
