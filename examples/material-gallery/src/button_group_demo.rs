use std::sync::OnceLock;

use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, ButtonGroupAppearance, ButtonGroupItem, ButtonGroupKind,
    ButtonGroupMotion, ButtonGroupOptions, ButtonGroupOverflow, ButtonGroupSelection,
    ButtonGroupShape, ButtonGroupSize, ButtonGroupWidth, ButtonKind, ChipKind, ChoiceState, Theme,
    VectorIcon,
};

use crate::Message;

pub(crate) const SECTION: usize = 1;
const SHOW: &str = "button-group-demo-show";
const BACK: &str = "button-group-demo-back";
const TRACK: &str = "button-group-track";
const ITEMS: [(&str, &str); 5] = [
    ("button-group-item-0", "裁切"),
    ("button-group-item-1", "標示"),
    ("button-group-item-2", "鎖定"),
    ("button-group-item-3", "對齊"),
    ("button-group-item-4", "匯出"),
];
const DISABLED: usize = 2;
const COMMAND: usize = 4;
const SIZES: [(ButtonGroupSize, &str, &str); 5] = [
    (ButtonGroupSize::ExtraSmall, "xs", "XS"),
    (ButtonGroupSize::Small, "s", "S"),
    (ButtonGroupSize::Medium, "m", "M"),
    (ButtonGroupSize::Large, "l", "L"),
    (ButtonGroupSize::ExtraLarge, "xl", "XL"),
];
const APPEARANCES: [(ButtonGroupAppearance, &str, &str); 4] = [
    (ButtonGroupAppearance::Elevated, "elevated", "浮起"),
    (ButtonGroupAppearance::Filled, "filled", "填色"),
    (ButtonGroupAppearance::Tonal, "tonal", "柔色"),
    (ButtonGroupAppearance::Outlined, "outlined", "描邊"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Commands,
    Single,
    Multi,
}

#[derive(Clone, Copy)]
pub enum ButtonGroupEvent {
    Show,
    Back,
    Kind(ButtonGroupKind),
    Size(ButtonGroupSize),
    Shape(ButtonGroupShape),
    Appearance(ButtonGroupAppearance),
    Direction,
    TextScale,
    Selection(SelectionMode),
    Required,
    Activate(usize),
    ToggleOverflow,
    DismissOverflow,
}

pub(crate) struct ButtonGroupDemo {
    open: bool,
    width: f32,
    kind: ButtonGroupKind,
    size: ButtonGroupSize,
    shape: ButtonGroupShape,
    appearance: ButtonGroupAppearance,
    rtl: bool,
    large_text: bool,
    selection: SelectionMode,
    required: bool,
    selected: [bool; ITEMS.len()],
    activations: [usize; ITEMS.len()],
    last_command: Option<usize>,
    overflow_open: bool,
    motion: ButtonGroupMotion,
    pressed: Option<usize>,
    reduced_motion: bool,
}

impl Default for ButtonGroupDemo {
    fn default() -> Self {
        Self {
            open: false,
            width: 960.,
            kind: ButtonGroupKind::Standard,
            size: ButtonGroupSize::Small,
            shape: ButtonGroupShape::Round,
            appearance: ButtonGroupAppearance::Tonal,
            rtl: false,
            large_text: false,
            selection: SelectionMode::Single,
            required: false,
            selected: [false, true, false, false, false],
            activations: [0; ITEMS.len()],
            last_command: None,
            overflow_open: false,
            motion: ButtonGroupMotion::default(),
            pressed: None,
            reduced_motion: false,
        }
    }
}

impl ButtonGroupDemo {
    pub(crate) fn visible(&self) -> bool {
        self.open
    }

    pub(crate) fn leave(&mut self) {
        self.open = false;
        self.overflow_open = false;
        self.motion = ButtonGroupMotion::default();
        self.pressed = None;
    }

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        if self.width == width {
            return false;
        }
        self.width = width;
        self.overflow_open = false;
        self.open
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn frame(
        &mut self,
        frame: &AnimationFrame<'_>,
        pressed: Option<&str>,
    ) -> AnimationUpdate {
        if !self.open {
            return AnimationUpdate::default();
        }
        self.pressed = ITEMS.iter().position(|(id, _)| Some(*id) == pressed);
        self.reduced_motion = frame.reduced_motion;
        self.motion.advance(TRACK, frame, pressed)
    }

    fn is_toggle(&self, index: usize) -> bool {
        self.selection != SelectionMode::Commands
            && (self.kind == ButtonGroupKind::Connected || index != COMMAND)
    }

    fn selected_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.selected
            .iter()
            .enumerate()
            .filter_map(|(index, &selected)| (selected && self.is_toggle(index)).then_some(index))
    }

    fn set_mode(&mut self, mode: SelectionMode) {
        self.selection = mode;
        if mode == SelectionMode::Single {
            let first = self.selected_indices().next();
            self.selected = std::array::from_fn(|index| Some(index) == first);
        }
        self.reconcile_selection();
    }

    fn reconcile_selection(&mut self) {
        if self.selection == SelectionMode::Commands {
            return;
        }
        if self.kind == ButtonGroupKind::Standard {
            self.selected[COMMAND] = false;
        }
        if self.required && self.selected_indices().next().is_none() {
            self.selected[0] = true;
        }
    }

    fn activate(&mut self, index: usize) {
        self.overflow_open = false;
        self.activations[index] += 1;
        if self.pressed != Some(index) {
            self.motion.pulse(ITEMS[index].0);
        }
        if !self.is_toggle(index) {
            self.last_command = Some(index);
            return;
        }
        self.last_command = None;
        let was_selected = self.selected[index];
        if was_selected && self.required && self.selected_indices().count() == 1 {
            return;
        }
        if self.selection == SelectionMode::Single {
            self.selected.fill(false);
        }
        self.selected[index] = !was_selected;
    }

    pub(crate) fn update(&mut self, event: ButtonGroupEvent) {
        match event {
            ButtonGroupEvent::Show => self.open = true,
            ButtonGroupEvent::Back => self.leave(),
            ButtonGroupEvent::Kind(kind) => {
                self.kind = kind;
                self.overflow_open = false;
                if kind == ButtonGroupKind::Connected && self.selection == SelectionMode::Commands {
                    self.set_mode(SelectionMode::Single);
                } else {
                    self.reconcile_selection();
                }
            }
            ButtonGroupEvent::Size(size) => {
                self.size = size;
                self.overflow_open = false;
            }
            ButtonGroupEvent::Shape(shape) => self.shape = shape,
            ButtonGroupEvent::Appearance(appearance) => self.appearance = appearance,
            ButtonGroupEvent::Direction => {
                self.rtl = !self.rtl;
                self.overflow_open = false;
            }
            ButtonGroupEvent::TextScale => {
                self.large_text = !self.large_text;
                self.overflow_open = false;
            }
            ButtonGroupEvent::Selection(mode) => {
                if mode == SelectionMode::Commands {
                    self.kind = ButtonGroupKind::Standard;
                }
                self.overflow_open = false;
                self.set_mode(mode);
            }
            ButtonGroupEvent::Required if self.selection != SelectionMode::Commands => {
                self.required = !self.required;
                self.reconcile_selection();
            }
            ButtonGroupEvent::Required => {}
            ButtonGroupEvent::Activate(index) => self.activate(index),
            ButtonGroupEvent::ToggleOverflow => self.overflow_open = !self.overflow_open,
            ButtonGroupEvent::DismissOverflow => self.overflow_open = false,
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "按鈕群組",
                ButtonKind::Outlined,
                false,
                app(ButtonGroupEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn group(&self, theme: &Theme) -> View<Message> {
        let selected: Vec<_> = self
            .selected_indices()
            .map(|index| ITEMS[index].0)
            .collect();
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let selection = match self.selection {
            SelectionMode::Commands => ButtonGroupSelection::Commands,
            SelectionMode::Single => ButtonGroupSelection::Single {
                selected: selected.first().copied(),
                required: self.required,
            },
            SelectionMode::Multi => ButtonGroupSelection::Multiple {
                selected: &selected,
                required: self.required,
            },
        };
        let items = ITEMS.iter().enumerate().map(|(index, &(id, label))| {
            let icon_only = index == 3;
            let width = if self.kind == ButtonGroupKind::Connected {
                ButtonGroupWidth::Flex(1.)
            } else if icon_only {
                ButtonGroupWidth::Narrow
            } else {
                ButtonGroupWidth::Intrinsic
            };
            ButtonGroupItem {
                id,
                label,
                icon: icon(index),
                icon_only,
                appearance: None,
                width,
                toggle: self.is_toggle(index),
                disabled: index == DISABLED,
                on_press: app(ButtonGroupEvent::Activate(index)),
            }
        });
        let overflow = (self.kind == ButtonGroupKind::Standard).then_some(ButtonGroupOverflow {
            label: "更多工具",
            open: self.overflow_open,
            on_toggle: app(ButtonGroupEvent::ToggleOverflow),
            on_dismiss: app(ButtonGroupEvent::DismissOverflow),
        });
        theme.button_group(
            ButtonGroupOptions {
                id: TRACK,
                name: "圖片工具",
                kind: self.kind,
                size: self.size,
                shape: self.shape,
                direction,
                selection,
                available_width: (self.width - 32.).max(1.),
                appearance: self.appearance,
            },
            items,
            &self.motion,
            overflow,
        )
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let header = self.row("顯示設定").children([
            theme
                .tonal_button(BACK, "返回", app(ButtonGroupEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            chip(theme, "button-group-theme", "深色", dark, Message::Theme),
            chip(
                theme,
                "button-group-direction",
                "右起",
                self.rtl,
                app(ButtonGroupEvent::Direction),
            ),
            chip(
                theme,
                "button-group-large-text",
                "大字",
                self.large_text,
                app(ButtonGroupEvent::TextScale),
            ),
        ]);
        let mut scene = Style::column().padding(16.).gap(12.).align(if self.rtl {
            Align::End
        } else {
            Align::Start
        });
        scene.min_height = 128.;
        let scene = View::new(
            Node::new(Role::Group, "")
                .id("button-group-demo-scene")
                .styled(scene),
        )
        .children([self.group(theme), self.status(theme)]);
        let scroll = View::new(
            Node::new(Role::Group, "")
                .id("button-group-demo-scroll")
                .styled(Style::column().height(Length::Px(0.)).grow(1.).clip(true)),
        )
        .accessibility_scroll_pages()
        .children([scene, self.settings(theme)]);
        View::new(
            Node::new(Role::Window, "按鈕群組")
                .id("button-group-demo-page")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .background(theme.colors.surface),
                ),
        )
        .children([header, scroll])
    }

    fn row(&self, name: &str) -> View<Message> {
        let mut row = Node::new(Role::Group, name).styled(
            Style::row()
                .padding(8.)
                .gap(8.)
                .align(Align::Center)
                .wrap(true),
        );
        if self.rtl {
            row = row.reverse_row();
        }
        View::new(row)
    }

    fn status(&self, theme: &Theme) -> View<Message> {
        let status = if let Some(index) = self.last_command {
            format!("已執行 {}", ITEMS[index].1)
        } else if self.selection == SelectionMode::Commands {
            "圖片工具".to_owned()
        } else {
            let selection = self
                .selected_indices()
                .map(|i| ITEMS[i].1)
                .collect::<Vec<_>>();
            if selection.is_empty() {
                "尚未選取".to_owned()
            } else {
                format!("已選 {}", selection.join("、"))
            }
        };
        View::new(
            Node::new(Role::Status, status.clone())
                .id("button-group-status")
                .text(status)
                .styled(Style::column().foreground(theme.colors.on_surface)),
        )
    }

    fn settings(&self, theme: &Theme) -> View<Message> {
        let options = [
            (
                "button-group-standard",
                "標準",
                self.kind == ButtonGroupKind::Standard,
                ButtonGroupEvent::Kind(ButtonGroupKind::Standard),
            ),
            (
                "button-group-connected",
                "連接",
                self.kind == ButtonGroupKind::Connected,
                ButtonGroupEvent::Kind(ButtonGroupKind::Connected),
            ),
            (
                "button-group-round",
                "圓形",
                self.shape == ButtonGroupShape::Round,
                ButtonGroupEvent::Shape(ButtonGroupShape::Round),
            ),
            (
                "button-group-square",
                "方形",
                self.shape == ButtonGroupShape::Square,
                ButtonGroupEvent::Shape(ButtonGroupShape::Square),
            ),
        ];
        let selection = [
            (
                "button-group-single",
                "單選",
                self.selection == SelectionMode::Single,
                ButtonGroupEvent::Selection(SelectionMode::Single),
            ),
            (
                "button-group-multi",
                "多選",
                self.selection == SelectionMode::Multi,
                ButtonGroupEvent::Selection(SelectionMode::Multi),
            ),
            (
                "button-group-commands",
                "指令",
                self.selection == SelectionMode::Commands,
                ButtonGroupEvent::Selection(SelectionMode::Commands),
            ),
            (
                "button-group-required",
                "必選",
                self.required && self.selection != SelectionMode::Commands,
                ButtonGroupEvent::Required,
            ),
        ];
        let choice = |(id, label, selected, event)| chip(theme, id, label, selected, app(event));
        let sizes = SIZES.into_iter().map(|(size, slug, label)| {
            chip(
                theme,
                &format!("button-group-size-{slug}"),
                label,
                size == self.size,
                app(ButtonGroupEvent::Size(size)),
            )
        });
        let appearances = APPEARANCES.into_iter().map(|(appearance, slug, label)| {
            chip(
                theme,
                &format!("button-group-appearance-{slug}"),
                label,
                appearance == self.appearance,
                app(ButtonGroupEvent::Appearance(appearance)),
            )
        });
        View::new(Node::new(Role::Group, "群組設定").styled(Style::column())).children([
            self.row("版型與形狀")
                .children(options.into_iter().map(choice)),
            self.row("尺寸").children(sizes),
            self.row("色彩").children(appearances),
            self.row("選取方式").children(
                selection
                    .into_iter()
                    .filter(|(id, ..)| {
                        self.selection != SelectionMode::Commands || *id != "button-group-required"
                    })
                    .map(choice),
            ),
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        serde_json::json!({
            "open": self.open,
            "kind": if self.kind == ButtonGroupKind::Standard { "standard" } else { "connected" },
            "size": SIZES.iter().find(|(size, ..)| self.size == *size).map(|(_, slug, _)| *slug),
            "shape": if self.shape == ButtonGroupShape::Round { "round" } else { "square" },
            "appearance": APPEARANCES.iter().find(|(a, ..)| self.appearance == *a).map(|(_, slug, _)| *slug),
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "selected": self.selected_indices().collect::<Vec<_>>(),
            "selection": match self.selection {
                SelectionMode::Commands => "commands",
                SelectionMode::Single => "single",
                SelectionMode::Multi => "multi",
            },
            "required": self.required && self.selection != SelectionMode::Commands,
            "pressed": self.pressed,
            "reduced_motion": self.reduced_motion,
            "activations": self.activations,
            "overflow_open": self.overflow_open,
        })
    }
}

fn app(event: ButtonGroupEvent) -> Message {
    Message::ButtonGroup(event)
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

fn icon(index: usize) -> Option<VectorIcon> {
    static CROP: OnceLock<VectorIcon> = OnceLock::new();
    static ALIGN: OnceLock<VectorIcon> = OnceLock::new();
    match index {
        0 => Some(
            CROP.get_or_init(|| {
                VectorIcon::from_contours([
                    [[5., 2.], [7., 2.], [7., 17.], [5., 17.]],
                    [[5., 17.], [22., 17.], [22., 19.], [5., 19.]],
                    [[9., 5.], [19., 5.], [19., 7.], [9., 7.]],
                    [[17., 7.], [19., 7.], [19., 15.], [17., 15.]],
                    [[17., 19.], [19., 19.], [19., 22.], [17., 22.]],
                ])
                .expect("valid crop icon")
            })
            .clone(),
        ),
        3 => Some(
            ALIGN
                .get_or_init(|| {
                    VectorIcon::from_contours([
                        [[3., 5.], [21., 5.], [21., 7.], [3., 7.]],
                        [[3., 11.], [15., 11.], [15., 13.], [3., 13.]],
                        [[3., 17.], [21., 17.], [21., 19.], [3., 19.]],
                    ])
                    .expect("valid alignment icon")
                })
                .clone(),
        ),
        _ => None,
    }
}
