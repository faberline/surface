use std::sync::OnceLock;

use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChipKind, ChoiceState, SplitButtonAppearance,
    SplitButtonContent, SplitButtonLeading, SplitButtonMenu, SplitButtonMenuItem,
    SplitButtonMotion, SplitButtonOptions, SplitButtonSize, Theme, VectorIcon,
};

use crate::Message;

pub(crate) const SECTION: usize = 1;
const SHOW: &str = "split-demo-show";
const BACK: &str = "split-demo-back";
const TRACK: &str = "split-track";
const LABEL: &str = "匯出報告";
const COMMANDS: [(&str, &str); 3] = [
    ("split-menu-item-0", "PDF"),
    ("split-menu-item-1", "試算表"),
    ("split-menu-item-2", "純文字"),
];
const SIZES: [(SplitButtonSize, &str, &str); 5] = [
    (SplitButtonSize::ExtraSmall, "xs", "XS"),
    (SplitButtonSize::Small, "s", "S"),
    (SplitButtonSize::Medium, "m", "M"),
    (SplitButtonSize::Large, "l", "L"),
    (SplitButtonSize::ExtraLarge, "xl", "XL"),
];
const APPEARANCES: [(SplitButtonAppearance, &str, &str); 4] = [
    (SplitButtonAppearance::Elevated, "elevated", "浮起"),
    (SplitButtonAppearance::Filled, "filled", "填色"),
    (SplitButtonAppearance::Tonal, "tonal", "柔色"),
    (SplitButtonAppearance::Outlined, "outlined", "描邊"),
];
const LEADING: [(Leading, &str, &str); 3] = [
    (Leading::Text, "text", "文字"),
    (Leading::Icon, "icon", "圖示"),
    (Leading::Both, "both", "兩者"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Leading {
    Text,
    Icon,
    Both,
}

#[derive(Clone, Copy)]
pub enum SplitEvent {
    Show,
    Back,
    Size(SplitButtonSize),
    Appearance(SplitButtonAppearance),
    Leading(Leading),
    Direction,
    TextScale,
    DisableMain,
    DisableOptions,
    AnchorBottom,
    Primary,
    Toggle,
    Dismiss,
    Command(usize),
}

pub(crate) struct SplitDemo {
    open: bool,
    width: f32,
    size: SplitButtonSize,
    appearance: SplitButtonAppearance,
    leading: Leading,
    rtl: bool,
    large_text: bool,
    leading_disabled: bool,
    trailing_disabled: bool,
    bottom: bool,
    menu_open: bool,
    primary_count: usize,
    command_counts: [usize; COMMANDS.len()],
    last_action: Option<&'static str>,
    motion: SplitButtonMotion,
    reduced_motion: bool,
}

impl Default for SplitDemo {
    fn default() -> Self {
        Self {
            open: false,
            width: 960.,
            size: SplitButtonSize::Medium,
            appearance: SplitButtonAppearance::Filled,
            leading: Leading::Both,
            rtl: false,
            large_text: false,
            leading_disabled: false,
            trailing_disabled: false,
            bottom: false,
            menu_open: false,
            primary_count: 0,
            command_counts: [0; COMMANDS.len()],
            last_action: None,
            motion: SplitButtonMotion::new(false),
            reduced_motion: false,
        }
    }
}

impl SplitDemo {
    pub(crate) fn visible(&self) -> bool {
        self.open
    }

    pub(crate) fn leave(&mut self) {
        self.open = false;
        self.menu_open = false;
        self.motion = SplitButtonMotion::new(false);
    }

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        if self.width == width {
            return false;
        }
        self.width = width;
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
        input: &FrameInput<'_>,
        pressed: Option<&str>,
    ) -> AnimationUpdate {
        if !self.open {
            return AnimationUpdate::default();
        }
        self.reduced_motion = frame.reduced_motion;
        self.motion
            .advance(TRACK, frame, input, pressed, self.menu_open)
    }

    pub(crate) fn update(&mut self, event: SplitEvent) {
        match event {
            SplitEvent::Show => self.open = true,
            SplitEvent::Back => self.leave(),
            SplitEvent::Size(size) => self.size = size,
            SplitEvent::Appearance(appearance) => self.appearance = appearance,
            SplitEvent::Leading(leading) => self.leading = leading,
            SplitEvent::Direction => self.rtl = !self.rtl,
            SplitEvent::TextScale => self.large_text = !self.large_text,
            SplitEvent::DisableMain => self.leading_disabled = !self.leading_disabled,
            SplitEvent::DisableOptions => {
                self.trailing_disabled = !self.trailing_disabled;
                self.menu_open &= !self.trailing_disabled;
            }
            SplitEvent::AnchorBottom => self.bottom = !self.bottom,
            SplitEvent::Primary if !self.leading_disabled => {
                self.primary_count += 1;
                self.last_action = Some(LABEL);
            }
            SplitEvent::Primary => {}
            SplitEvent::Toggle if !self.trailing_disabled => self.menu_open = !self.menu_open,
            SplitEvent::Toggle => {}
            SplitEvent::Dismiss => self.menu_open = false,
            SplitEvent::Command(index) => {
                self.command_counts[index] += 1;
                self.last_action = Some(COMMANDS[index].1);
                self.menu_open = false;
            }
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "分割按鈕",
                ButtonKind::Outlined,
                false,
                app(SplitEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn split(&self, theme: &Theme) -> View<Message> {
        let leading = match self.leading {
            Leading::Text => SplitButtonLeading::Text(LABEL),
            Leading::Icon => SplitButtonLeading::Icon {
                label: LABEL,
                icon: export_icon(),
            },
            Leading::Both => SplitButtonLeading::TextAndIcon {
                label: LABEL,
                icon: export_icon(),
            },
        };
        let items = COMMANDS
            .into_iter()
            .enumerate()
            .map(|(index, (id, label))| SplitButtonMenuItem {
                id,
                label,
                icon: None,
                disabled: index == 1,
                on_press: app(SplitEvent::Command(index)),
            })
            .collect();
        theme.split_button(
            SplitButtonOptions {
                id: TRACK,
                size: self.size,
                appearance: self.appearance,
                direction: if self.rtl {
                    LayoutDirection::RightToLeft
                } else {
                    LayoutDirection::LeftToRight
                },
                available_width: (self.width - 32.).max(1.),
                open: self.menu_open,
                leading_disabled: self.leading_disabled,
                trailing_disabled: self.trailing_disabled,
                restore_focus_to: Some("split-disable-options"),
            },
            SplitButtonContent {
                leading,
                trailing_label: "匯出選項",
                on_primary: app(SplitEvent::Primary),
                on_toggle: app(SplitEvent::Toggle),
                menu: Some(SplitButtonMenu {
                    label: "匯出格式",
                    on_dismiss: app(SplitEvent::Dismiss),
                    items,
                }),
            },
            &self.motion,
        )
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let header = self.row("顯示設定").children([
            theme
                .tonal_button(BACK, "返回", app(SplitEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            chip(theme, "split-theme", "深色", dark, Message::Theme),
            chip(
                theme,
                "split-rtl",
                "右起",
                self.rtl,
                app(SplitEvent::Direction),
            ),
            chip(
                theme,
                "split-large-text",
                "大字",
                self.large_text,
                app(SplitEvent::TextScale),
            ),
        ]);
        let mut elements = vec![self.split(theme)];
        if let Some(action) = self.last_action {
            let status = format!("已執行 {action}");
            elements.push(View::new(
                Node::new(Role::Status, status.clone())
                    .id("split-status")
                    .text(status)
                    .styled(Style::column().foreground(theme.colors.on_surface)),
            ));
        }
        let scene = View::new(Node::new(Role::Group, "").id("split-demo-scene").styled(
            Style::column().padding(16.).gap(8.).align(if self.rtl {
                Align::End
            } else {
                Align::Start
            }),
        ))
        .children(elements);
        let settings = View::new(
            Node::new(Role::Group, "")
                .id("split-demo-scroll")
                .styled(Style::column().height(Length::Px(0.)).grow(1.).clip(true)),
        )
        .accessibility_scroll_pages()
        .children([self.settings(theme)]);
        let children = if self.bottom {
            [header, settings, scene]
        } else {
            [header, scene, settings]
        };
        View::new(
            Node::new(Role::Window, "分割按鈕")
                .id("split-demo-page")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .background(theme.colors.surface),
                ),
        )
        .children(children)
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

    fn settings(&self, theme: &Theme) -> View<Message> {
        let sizes = SIZES.into_iter().map(|(size, slug, label)| {
            chip(
                theme,
                &format!("split-size-{slug}"),
                label,
                size == self.size,
                app(SplitEvent::Size(size)),
            )
        });
        let appearances = APPEARANCES.into_iter().map(|(appearance, slug, label)| {
            chip(
                theme,
                &format!("split-appearance-{slug}"),
                label,
                appearance == self.appearance,
                app(SplitEvent::Appearance(appearance)),
            )
        });
        let leading = LEADING.into_iter().map(|(leading, slug, label)| {
            chip(
                theme,
                &format!("split-leading-{slug}"),
                label,
                leading == self.leading,
                app(SplitEvent::Leading(leading)),
            )
        });
        let available = self.row("操作設定").children([
            chip(
                theme,
                "split-disable-main",
                "停用主鍵",
                self.leading_disabled,
                app(SplitEvent::DisableMain),
            ),
            chip(
                theme,
                "split-disable-options",
                "停用選單",
                self.trailing_disabled,
                app(SplitEvent::DisableOptions),
            ),
            chip(
                theme,
                "split-anchor-bottom",
                "移到下方",
                self.bottom,
                app(SplitEvent::AnchorBottom),
            ),
        ]);
        View::new(Node::new(Role::Group, "分割按鈕設定").styled(Style::column())).children([
            available,
            self.row("尺寸").children(sizes),
            self.row("色彩").children(appearances),
            self.row("主鍵內容").children(leading),
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        let angle = self.motion.angle_degrees();
        serde_json::json!({
            "open": self.open,
            "menu_open": self.menu_open,
            "size": SIZES.iter().find(|(s, ..)| self.size == *s).map(|(_, slug, _)| *slug),
            "appearance": APPEARANCES.iter().find(|(a, ..)| self.appearance == *a).map(|(_, slug, _)| *slug),
            "leading": LEADING.iter().find(|(l, ..)| self.leading == *l).map(|(_, slug, _)| *slug),
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "leading_disabled": self.leading_disabled,
            "trailing_disabled": self.trailing_disabled,
            "primary_count": self.primary_count,
            "command_counts": self.command_counts,
            "angle": angle,
            "physical_angle": if self.rtl { -angle } else { angle },
            "reduced_motion": self.reduced_motion,
            "bottom": self.bottom,
        })
    }
}

fn app(event: SplitEvent) -> Message {
    Message::Split(event)
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

fn export_icon() -> VectorIcon {
    static ICON: OnceLock<VectorIcon> = OnceLock::new();
    ICON.get_or_init(|| {
        let arrow = [
            [11., 2.],
            [13., 2.],
            [13., 11.],
            [17., 7.],
            [19., 9.],
            [12., 16.],
            [5., 9.],
            [7., 7.],
            [11., 11.],
        ];
        let tray = [
            [4., 15.],
            [6., 15.],
            [6., 19.],
            [18., 19.],
            [18., 15.],
            [20., 15.],
            [20., 21.],
            [4., 21.],
        ];
        VectorIcon::from_contours(
            [arrow.as_slice(), tray.as_slice()].map(|path| path.iter().copied()),
        )
        .expect("valid export icon")
    })
    .clone()
}
