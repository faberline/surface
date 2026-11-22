use std::sync::OnceLock;

use surface::{Align, Corners, Insets, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, FramePress, View};
use surface_material::{
    fab_menu_edge_margin, shape::LayoutDirection, ButtonKind, ChipKind, ChoiceState,
    ExpressiveFabSize, FabMenuAppearance, FabMenuContent, FabMenuItem, FabMenuMotion,
    FabMenuMotionTarget, FabMenuOptions, FabMenuPresentation, Theme, VectorIcon,
};

use crate::{fab_demo::add_icon, Message};

pub(crate) const SECTION: usize = 1;
const ID: &str = "fab-menu-demo";
const SHOW: &str = "fab-menu-demo-show";
const BACK: &str = "fab-menu-demo-back";
const OUTSIDE: &str = "fab-menu-demo-outside";
const ITEMS: [(&str, &str); 6] = [
    ("fab-menu-demo-item-0", "筆記"),
    ("fab-menu-demo-item-1", "任務"),
    ("fab-menu-demo-item-2", "會議紀錄"),
    ("fab-menu-demo-item-3", "匯入專案參考資料"),
    ("fab-menu-demo-item-4", "提醒"),
    ("fab-menu-demo-item-5", "工作區捷徑"),
];
const ITEM_IDS: [&str; ITEMS.len()] = [
    ITEMS[0].0, ITEMS[1].0, ITEMS[2].0, ITEMS[3].0, ITEMS[4].0, ITEMS[5].0,
];
const SIZES: [(ExpressiveFabSize, &str, &str); 3] = [
    (ExpressiveFabSize::Regular, "regular", "一般"),
    (ExpressiveFabSize::Medium, "medium", "中"),
    (ExpressiveFabSize::Large, "large", "大"),
];
const PALETTES: [(FabMenuAppearance, &str, &str); 3] = [
    (FabMenuAppearance::Primary, "primary", "主色"),
    (FabMenuAppearance::Secondary, "secondary", "輔色"),
    (FabMenuAppearance::Tertiary, "tertiary", "第三色"),
];

#[derive(Clone, Copy)]
pub enum FabMenuEvent {
    Show,
    Back,
    Presentation(FabMenuPresentation),
    Size(ExpressiveFabSize),
    Palette(FabMenuAppearance),
    Count(usize),
    Direction,
    TextScale,
    Top,
    Disable,
    AllDisabled,
    Toggle,
    Dismiss,
    Choose(usize),
    Outside,
}

pub(crate) struct FabMenuDemo {
    in_scene: bool,
    open: bool,
    viewport: Rect,
    presentation: FabMenuPresentation,
    size: ExpressiveFabSize,
    palette: FabMenuAppearance,
    rtl: bool,
    large_text: bool,
    top: bool,
    count: usize,
    disabled: bool,
    all_disabled: bool,
    toggles: usize,
    dismissals: usize,
    actions: [usize; ITEMS.len()],
    last: Option<usize>,
    outside_actions: usize,
    motion: FabMenuMotion,
    reduced_motion: bool,
}

impl Default for FabMenuDemo {
    fn default() -> Self {
        Self {
            in_scene: false,
            open: false,
            viewport: Rect::new(0., 0., 960., 800.),
            presentation: if cfg!(target_arch = "wasm32") {
                FabMenuPresentation::WebAnchored
            } else {
                FabMenuPresentation::NativeFloating
            },
            size: ExpressiveFabSize::Regular,
            palette: FabMenuAppearance::Primary,
            rtl: false,
            large_text: false,
            top: false,
            count: 2,
            disabled: false,
            all_disabled: false,
            toggles: 0,
            dismissals: 0,
            actions: [0; ITEMS.len()],
            last: None,
            outside_actions: 0,
            motion: FabMenuMotion::new(false),
            reduced_motion: false,
        }
    }
}

impl FabMenuDemo {
    pub(crate) fn in_scene(&self) -> bool {
        self.in_scene
    }

    pub(crate) fn leave(&mut self) {
        self.in_scene = false;
        self.open = false;
        self.motion = FabMenuMotion::new(false);
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
        let changed = next != self.viewport;
        self.viewport = next;
        self.in_scene && changed
    }

    pub(crate) fn frame(
        &mut self,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> AnimationUpdate {
        if !self.in_scene {
            return AnimationUpdate::default();
        }
        self.reduced_motion = frame.reduced_motion;
        self.motion.advance(
            frame,
            input,
            press,
            hovered,
            FabMenuMotionTarget {
                id: ID,
                open: self.open,
                item_ids: &ITEM_IDS[..self.count],
            },
        )
    }

    fn choice_disabled(&self, index: usize) -> bool {
        self.all_disabled || (self.count == 6 && index == 2)
    }

    pub(crate) fn update(&mut self, event: FabMenuEvent) {
        match event {
            FabMenuEvent::Show => self.in_scene = true,
            FabMenuEvent::Back => self.leave(),
            FabMenuEvent::Presentation(presentation) => {
                self.presentation = presentation;
                self.open = false;
                self.motion = FabMenuMotion::new(false);
            }
            FabMenuEvent::Size(size) => self.size = size,
            FabMenuEvent::Palette(palette) => self.palette = palette,
            FabMenuEvent::Count(count) => self.count = count.clamp(2, ITEMS.len()),
            FabMenuEvent::Direction => self.rtl = !self.rtl,
            FabMenuEvent::TextScale => self.large_text = !self.large_text,
            FabMenuEvent::Top => self.top = !self.top,
            FabMenuEvent::Disable => {
                self.disabled = !self.disabled;
                if self.disabled {
                    self.open = false;
                }
            }
            FabMenuEvent::AllDisabled => self.all_disabled = !self.all_disabled,
            FabMenuEvent::Toggle if !self.disabled => {
                self.toggles += 1;
                self.open = !self.open;
            }
            FabMenuEvent::Dismiss if self.open => {
                self.dismissals += 1;
                self.open = false;
            }
            FabMenuEvent::Choose(index)
                if self.open && index < self.count && !self.choice_disabled(index) =>
            {
                self.actions[index] += 1;
                self.last = Some(index);
                self.open = false;
            }
            FabMenuEvent::Outside => self.outside_actions += 1,
            FabMenuEvent::Toggle | FabMenuEvent::Dismiss | FabMenuEvent::Choose(_) => {}
        }
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "浮動動作選單",
                ButtonKind::Outlined,
                false,
                app(FabMenuEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let header = self.row("顯示設定").children([
            theme
                .tonal_button(BACK, "返回", app(FabMenuEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            chip(theme, "fab-menu-demo-theme", "深色", dark, Message::Theme),
            chip(
                theme,
                "fab-menu-demo-rtl",
                "右起",
                self.rtl,
                app(FabMenuEvent::Direction),
            ),
            chip(
                theme,
                "fab-menu-demo-large-text",
                "大字",
                self.large_text,
                app(FabMenuEvent::TextScale),
            ),
        ]);
        let mut clip = Style::column().clip(true);
        clip.max_height = Length::Px((self.viewport.height * 0.38).max(64.));
        let settings = View::new(
            Node::new(Role::Group, "動作選單設定")
                .id("fab-menu-demo-settings")
                .styled(clip),
        )
        .children([self.settings(theme)])
        .accessibility_scroll_pages();
        View::new(
            Node::new(Role::Window, "浮動動作選單")
                .id("fab-menu-demo-page")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .background(theme.colors.surface),
                ),
        )
        .children([
            header,
            settings,
            self.body(theme),
            self.menu_anchor(theme),
            self.outside(theme),
        ])
    }

    fn row(&self, title: &str) -> View<Message> {
        let node = Node::new(Role::Group, title).styled(
            Style::row()
                .padding(8.)
                .gap(8.)
                .align(Align::Center)
                .wrap(true),
        );
        View::new(if self.rtl { node.reverse_row() } else { node })
    }

    fn settings(&self, theme: &Theme) -> View<Message> {
        use FabMenuPresentation::{NativeFloating, WebAnchored};
        let presentations = self.row("呈現方式").children([
            chip(
                theme,
                "fab-menu-demo-floating",
                "浮動動作",
                self.presentation == NativeFloating,
                app(FabMenuEvent::Presentation(NativeFloating)),
            ),
            chip(
                theme,
                "fab-menu-demo-web",
                "錨定選單",
                self.presentation == WebAnchored,
                app(FabMenuEvent::Presentation(WebAnchored)),
            ),
        ]);
        let mut options = vec![
            chip(
                theme,
                "fab-menu-demo-count-two",
                "兩項",
                self.count == 2,
                app(FabMenuEvent::Count(2)),
            ),
            chip(
                theme,
                "fab-menu-demo-count-six",
                "六項",
                self.count == 6,
                app(FabMenuEvent::Count(6)),
            ),
            chip(
                theme,
                "fab-menu-demo-disable-trigger",
                "停用主鈕",
                self.disabled,
                app(FabMenuEvent::Disable),
            ),
            chip(
                theme,
                "fab-menu-demo-all-disabled",
                "停用所有選項",
                self.all_disabled,
                app(FabMenuEvent::AllDisabled),
            ),
        ];
        if self.presentation == WebAnchored {
            options.push(chip(
                theme,
                "fab-menu-demo-top",
                "上方錨點",
                self.top,
                app(FabMenuEvent::Top),
            ));
        }
        let sizes = SIZES.into_iter().map(|(size, slug, label)| {
            chip(
                theme,
                &format!("fab-menu-demo-size-{slug}"),
                label,
                size == self.size,
                app(FabMenuEvent::Size(size)),
            )
        });
        let palettes = PALETTES.into_iter().map(|(palette, slug, label)| {
            chip(
                theme,
                &format!("fab-menu-demo-palette-{slug}"),
                label,
                palette == self.palette,
                app(FabMenuEvent::Palette(palette)),
            )
        });
        View::new(Node::new(Role::Group, "").styled(Style::column())).children([
            presentations,
            self.row("選項").children(options),
            self.row("尺寸").children(sizes),
            self.row("色彩").children(palettes),
        ])
    }

    fn body(&self, theme: &Theme) -> View<Message> {
        let title = theme.heading("fab-menu-demo-workspace", "專案工作區");
        let status = self.last.map(|index| {
            let label = format!("已新增 {}", ITEMS[index].1);
            View::new(
                Node::new(Role::Status, label.clone())
                    .id("fab-menu-demo-status")
                    .text(label)
                    .styled(Style::column().foreground(theme.colors.on_surface)),
            )
        });
        let notes = (1..=16).map(|index| {
            let label = ["設計紀錄", "測試結果", "發佈進度"][(index - 1) % 3];
            View::new(
                Node::new(Role::Group, "").styled(
                    Style::column()
                        .padding(12.)
                        .corners(Corners::all(12.))
                        .background(theme.colors.surface_container_low),
                ),
            )
            .children([theme.text(
                &format!("fab-menu-demo-note-{index}"),
                format!("{label} {index}"),
            )])
        });
        let tail =
            View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(104.))));
        View::new(
            Node::new(Role::Group, "工作區")
                .id("fab-menu-demo-scroll")
                .styled(Style::column().height(Length::Px(0.)).grow(1.).clip(true)),
        )
        .children([View::new(
            Node::new(Role::Group, "").styled(Style::column().padding(16.).gap(8.)),
        )
        .children(
            std::iter::once(title)
                .chain(status)
                .chain(notes)
                .chain([tail]),
        )])
        .accessibility_scroll_pages()
    }

    fn dock(&self, is_trailing: bool, at_top: bool, z: i32) -> Style {
        let margin = fab_menu_edge_margin(self.viewport.width);
        let edge = Length::Px(margin);
        let mut style = Style::row()
            .width(Length::Px((self.viewport.width - 2. * margin).max(1.)))
            .justify(if self.rtl == is_trailing {
                Align::Start
            } else {
                Align::End
            })
            .fixed(Insets {
                left: edge,
                top: if at_top { edge } else { Length::Auto },
                bottom: if at_top { Length::Auto } else { edge },
                ..Insets::default()
            });
        style.z_index = z;
        style
    }

    fn menu_anchor(&self, theme: &Theme) -> View<Message> {
        let items = ITEMS[..self.count]
            .iter()
            .enumerate()
            .map(|(index, &(id, label))| FabMenuItem {
                id,
                label,
                icon: Some(item_icons()[index].clone()),
                disabled: self.choice_disabled(index),
                on_press: app(FabMenuEvent::Choose(index)),
            })
            .collect();
        let top = self.top && self.presentation == FabMenuPresentation::WebAnchored;
        let trigger = theme.fab_menu(
            FabMenuOptions {
                id: ID,
                open_label: "新增項目",
                close_label: "關閉動作選單",
                menu_label: "動作選單",
                presentation: self.presentation,
                appearance: self.palette,
                size: self.size,
                direction: self.direction(),
                viewport: self.viewport,
                open: self.open,
                disabled: self.disabled,
                restore_focus_to: Some(OUTSIDE),
            },
            FabMenuContent {
                icon: add_icon(),
                items,
                on_toggle: app(FabMenuEvent::Toggle),
                on_dismiss: app(FabMenuEvent::Dismiss),
            },
            &self.motion,
        );
        View::new(
            Node::new(Role::Group, "")
                .id("fab-menu-demo-dock")
                .styled(self.dock(true, top, 20)),
        )
        .children([trigger])
    }

    fn outside(&self, theme: &Theme) -> View<Message> {
        View::new(Node::new(Role::Group, "").styled(self.dock(false, false, 10))).children([theme
            .tonal_button(OUTSIDE, "工作區動作", app(FabMenuEvent::Outside))
            .accessible_activation()])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        let item_motion: Vec<_> = (0..self.count)
            .map(|index| {
                let item = self.motion.item(index);
                serde_json::json!({"width": item.width, "opacity": item.opacity})
            })
            .collect();
        serde_json::json!({
            "is_open": self.in_scene,
            "open": self.open,
            "presentation": match self.presentation {
                FabMenuPresentation::NativeFloating => "floating",
                FabMenuPresentation::WebAnchored => "web",
            },
            "size": SIZES.iter().find(|(size, ..)| self.size == *size).map(|(_, slug, _)| *slug),
            "palette": PALETTES.iter().find(|(palette, ..)| self.palette == *palette).map(|(_, slug, _)| *slug),
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "top": self.top,
            "count": self.count,
            "disabled": self.disabled,
            "all_disabled": self.all_disabled,
            "toggles": self.toggles,
            "dismissals": self.dismissals,
            "actions": self.actions,
            "outside_actions": self.outside_actions,
            "progress": self.motion.progress(),
            "visible_items": self.motion.visible_items(),
            "items_present": self.motion.items_present(),
            "item_motion": item_motion,
            "anchor": self.motion.anchor(),
            "reduced_motion": self.reduced_motion,
        })
    }
}

fn app(event: FabMenuEvent) -> Message {
    Message::FabMenu(event)
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

fn item_icons() -> &'static [VectorIcon; ITEMS.len()] {
    static ICONS: OnceLock<[VectorIcon; ITEMS.len()]> = OnceLock::new();
    ICONS.get_or_init(|| {
        let contour = |points: &[[f32; 2]]| {
            VectorIcon::new(points.iter().copied()).expect("valid action icon")
        };
        let note = [[5., 3.], [14., 3.], [19., 8.], [19., 21.], [5., 21.]];
        let tick = [
            [3., 12.],
            [5., 10.],
            [10., 15.],
            [19., 6.],
            [21., 8.],
            [10., 19.],
        ];
        let calendar = [
            [4., 5.],
            [8., 5.],
            [8., 3.],
            [10., 3.],
            [10., 5.],
            [14., 5.],
            [14., 3.],
            [16., 3.],
            [16., 5.],
            [20., 5.],
            [20., 21.],
            [4., 21.],
        ];
        let import = [
            [11., 3.],
            [13., 3.],
            [13., 10.],
            [17., 7.],
            [19., 9.],
            [12., 16.],
            [5., 9.],
            [7., 7.],
            [11., 10.],
        ];
        let bell = [
            [10., 3.],
            [14., 3.],
            [14., 5.],
            [17., 7.],
            [18., 13.],
            [21., 17.],
            [15., 17.],
            [14., 20.],
            [10., 20.],
            [9., 17.],
            [3., 17.],
            [6., 13.],
            [7., 7.],
            [10., 5.],
        ];
        let shortcut = [
            [6., 6.],
            [19., 6.],
            [19., 19.],
            [16., 19.],
            [16., 11.],
            [7., 20.],
            [5., 18.],
            [14., 9.],
            [6., 9.],
        ];
        [
            contour(&note[..]),
            contour(&tick[..]),
            contour(&calendar[..]),
            contour(&import[..]),
            contour(&bell[..]),
            contour(&shortcut[..]),
        ]
    })
}
