mod app_bar_demo;
mod badge_demo;
mod bottom_sheet_demo;
mod button_group_demo;
mod card_demo;
mod carousel_demo;
mod extended_fab_demo;
mod fab_demo;
mod fab_menu_demo;
mod modal_wide_navigation_rail_demo;
mod navigation_drawer_demo;
mod progress_demo;
mod rail_icons;
mod side_sheet_demo;
mod snackbar_demo;
mod split_demo;
mod toolbar_demo;
mod tooltip_demo;
mod wide_navigation_rail_demo;

use app_bar_demo::{AppBarDemo, AppBarEvent};
use badge_demo::{BadgeDemo, BadgeEvent};
use bottom_sheet_demo::{BottomSheetDemo, BottomSheetEvent};
use button_group_demo::{ButtonGroupDemo, ButtonGroupEvent};
use card_demo::{CardDemo, CardEvent};
use carousel_demo::{CarouselDemo, CarouselEvent};
use extended_fab_demo::{ExtendedFabDemo, ExtendedFabEvent};
use fab_demo::{FabDemo, FabEvent};
use fab_menu_demo::{FabMenuDemo, FabMenuEvent};
use modal_wide_navigation_rail_demo::{ModalWideRailDemo, ModalWideRailEvent};
use navigation_drawer_demo::{DrawerDemo, DrawerDemoEvent};
use progress_demo::{ProgressDemo, ProgressEvent};
use side_sheet_demo::{SideSheetDemo, SideSheetEvent};
use snackbar_demo::{SnackbarDemo, SnackbarEvent};
use split_demo::{SplitDemo, SplitEvent};
use surface::{Color, Node, RangeValue, Role, Style};
use surface_app::{
    AnimationFrame, AnimationUpdate, Application, FrameInput, FramePress, InputAnimationUpdate,
    View,
};
use surface_material::{
    ButtonKind, CardKind, CheckValue, ChipKind, ChoiceState, DividerKind, FabColor, FabSize,
    IconButtonKind, LoadingKind, NavigationRailItem, SegmentPosition, Theme,
};
use toolbar_demo::{ToolbarDemo, ToolbarEvent};
use tooltip_demo::{TooltipDemo, TooltipEvent};
use wide_navigation_rail_demo::{WideRailDemo, WideRailEvent};

pub struct Gallery {
    pub section: usize,
    pub dark: bool,
    pub radio: usize,
    pub enabled: bool,
    pub check: CheckValue,
    pub segment: usize,
    pub volume: f64,
    pub clicks: usize,
    pub filter: bool,
    pub input_chip: bool,
    pub dialog: bool,
    pub menu: bool,
    pub sheet: bool,
    pub name: String,
    pub draft: String,
    pub rail: usize,
    pub drawer: usize,
    pub nav: usize,
    rail_large_text: bool,
    wide_rail_demo: WideRailDemo,
    modal_wide_rail_demo: ModalWideRailDemo,
    snackbar_demo: SnackbarDemo,
    badge_demo: BadgeDemo,
    drawer_demo: DrawerDemo,
    progress_demo: ProgressDemo,
    side_sheet_demo: SideSheetDemo,
    bottom_sheet_demo: BottomSheetDemo,
    button_group_demo: ButtonGroupDemo,
    split_demo: SplitDemo,
    extended_fab_demo: ExtendedFabDemo,
    fab_demo: FabDemo,
    fab_menu_demo: FabMenuDemo,
    card_demo: CardDemo,
    carousel_demo: CarouselDemo,
    app_bar_demo: AppBarDemo,
    toolbar_demo: ToolbarDemo,
    tooltip_demo: TooltipDemo,
}
impl Default for Gallery {
    fn default() -> Self {
        Self {
            section: 0,
            dark: false,
            radio: 0,
            enabled: false,
            check: CheckValue::Mixed,
            segment: 0,
            volume: 40.,
            clicks: 0,
            filter: false,
            input_chip: true,
            dialog: false,
            menu: false,
            sheet: false,
            name: "Surface".into(),
            draft: String::new(),
            rail: 0,
            drawer: 0,
            nav: 0,
            rail_large_text: false,
            wide_rail_demo: WideRailDemo::default(),
            modal_wide_rail_demo: ModalWideRailDemo::default(),
            snackbar_demo: SnackbarDemo::default(),
            badge_demo: BadgeDemo::default(),
            drawer_demo: DrawerDemo::default(),
            progress_demo: ProgressDemo::default(),
            side_sheet_demo: SideSheetDemo::default(),
            bottom_sheet_demo: BottomSheetDemo::default(),
            button_group_demo: ButtonGroupDemo::default(),
            split_demo: SplitDemo::default(),
            extended_fab_demo: ExtendedFabDemo::default(),
            fab_demo: FabDemo::default(),
            fab_menu_demo: FabMenuDemo::default(),
            card_demo: CardDemo::default(),
            carousel_demo: CarouselDemo::default(),
            app_bar_demo: AppBarDemo::default(),
            toolbar_demo: ToolbarDemo::default(),
            tooltip_demo: TooltipDemo::default(),
        }
    }
}
#[derive(Clone)]
pub enum Message {
    Section(usize),
    Rail(usize),
    RailLargeText,
    WideRail(WideRailEvent),
    ModalWideRail(ModalWideRailEvent),
    Snackbar(SnackbarEvent),
    Badge(BadgeEvent),
    Drawer(usize),
    DrawerDemo(DrawerDemoEvent),
    Progress(ProgressEvent),
    SideSheet(SideSheetEvent),
    BottomSheet(BottomSheetEvent),
    ButtonGroup(ButtonGroupEvent),
    Split(SplitEvent),
    ExtendedFab(ExtendedFabEvent),
    Fab(FabEvent),
    FabMenu(FabMenuEvent),
    Card(CardEvent),
    Carousel(CarouselEvent),
    AppBar(AppBarEvent),
    Toolbar(ToolbarEvent),
    Tooltip(TooltipEvent),
    Nav(usize),
    Theme,
    Radio(usize),
    Switch,
    Check,
    Segment(usize),
    Volume(f64),
    Click,
    Filter,
    RemoveChip,
    Dialog(bool),
    Menu(bool),
    Sheet(bool),
    Draft(String),
    Save,
}
impl Application for Gallery {
    type Message = Message;
    fn title(&self) -> &str {
        "Surface · Material gallery"
    }
    fn text_scale(&self) -> f32 {
        if self.drawer_demo.visible() {
            return self.drawer_demo.text_scale();
        }
        if self.progress_demo.visible() {
            return self.progress_demo.text_scale();
        }
        if self.side_sheet_demo.visible() {
            return self.side_sheet_demo.text_scale();
        }
        if self.bottom_sheet_demo.visible() {
            return self.bottom_sheet_demo.text_scale();
        }
        if self.button_group_demo.visible() {
            return self.button_group_demo.text_scale();
        }
        if self.split_demo.visible() {
            return self.split_demo.text_scale();
        }
        if self.extended_fab_demo.is_open() {
            return self.extended_fab_demo.text_scale();
        }
        if self.fab_demo.is_open() {
            return self.fab_demo.text_scale();
        }
        if self.fab_menu_demo.in_scene() {
            return self.fab_menu_demo.text_scale();
        }
        if self.card_demo.visible() {
            return self.card_demo.text_scale();
        }
        if self.carousel_demo.visible() {
            return self.carousel_demo.text_scale();
        }
        if self.app_bar_demo.visible() {
            return self.app_bar_demo.text_scale();
        }
        if self.toolbar_demo.visible() {
            return self.toolbar_demo.text_scale();
        }
        if self.tooltip_demo.visible() {
            return self.tooltip_demo.text_scale();
        }
        if self.rail_large_text
            || self.snackbar_demo.large_text()
            || (self.section == badge_demo::SECTION && self.badge_demo.large_text())
        {
            2.
        } else {
            1.
        }
    }
    fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let inline = self.wide_rail_demo.viewport_changed(width);
        let modal = self.modal_wide_rail_demo.viewport_changed(width, height);
        let drawer = self.drawer_demo.viewport_changed(width, height);
        let progress = self.progress_demo.viewport_changed(width);
        let side_sheet = self.side_sheet_demo.viewport_changed(width, height);
        let bottom_sheet = self.bottom_sheet_demo.viewport_changed(height);
        let button_group = self.button_group_demo.viewport_changed(width);
        let split = self.split_demo.viewport_changed(width);
        let extended_fab = self.extended_fab_demo.viewport_changed(width, height);
        let fab = self.fab_demo.viewport_changed(width, height);
        let fab_menu = self.fab_menu_demo.viewport_changed(width, height);
        let carousel = self.carousel_demo.viewport_changed(width, height);
        let app_bar = self.app_bar_demo.viewport_changed(width);
        let toolbar = self.toolbar_demo.viewport_changed(width, height);
        let tooltip = self.tooltip_demo.viewport_changed(width, height);
        (self.section == wide_navigation_rail_demo::SECTION && (inline || modal))
            || (self.drawer_demo.visible() && drawer)
            || (self.progress_demo.visible() && progress)
            || (self.side_sheet_demo.visible() && side_sheet)
            || (self.bottom_sheet_demo.visible() && bottom_sheet)
            || button_group
            || split
            || extended_fab
            || fab
            || fab_menu
            || carousel
            || app_bar
            || toolbar
            || tooltip
    }
    fn observes_frame_input(&self) -> bool {
        self.tooltip_demo.visible()
            || self.button_group_demo.visible()
            || self.split_demo.visible()
            || self.extended_fab_demo.is_open()
            || self.fab_demo.is_open()
            || self.fab_menu_demo.in_scene()
    }
    fn cancel_transient(&mut self) -> bool {
        (self.tooltip_demo.visible() && self.tooltip_demo.dismiss())
            || (self.fab_demo.is_open() && self.fab_demo.dismiss_tooltip())
    }
    fn animation_frame_with_input(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
    ) -> InputAnimationUpdate {
        let mut update = self.tooltip_demo.frame(&frame, input);
        let other = self.animation_frame(frame);
        update.animation.rebuild |= other.rebuild;
        update.animation.request_next |= other.request_next;
        update
    }
    fn animation_frame_with_press(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
        pressed: Option<&str>,
    ) -> InputAnimationUpdate {
        let button_group = self.button_group_demo.frame(&frame, pressed);
        let split = self.split_demo.frame(&frame, &input, pressed);
        let extended_fab = self.extended_fab_demo.frame(&frame, &input, pressed);
        let mut update = self.animation_frame_with_input(frame, input);
        update.animation.rebuild |= button_group.rebuild || split.rebuild || extended_fab.rebuild;
        update.animation.request_next |=
            button_group.request_next || split.request_next || extended_fab.request_next;
        update
    }
    fn animation_frame_with_press_detail(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
        pressed: Option<&str>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> InputAnimationUpdate {
        let fab = self.fab_demo.frame(&frame, &input, pressed, press, hovered);
        let menu = self.fab_menu_demo.frame(&frame, &input, press, hovered);
        let mut update = self.animation_frame_with_press(frame, input, pressed);
        update.animation.rebuild |= fab.animation.rebuild || menu.rebuild;
        update.animation.request_next |= fab.animation.request_next || menu.request_next;
        update.suppress_touch_activation |= fab.suppress_touch_activation;
        update
    }
    fn animation_frame(&mut self, frame: AnimationFrame<'_>) -> AnimationUpdate {
        self.card_demo.frame(&frame);
        let inline = self.wide_rail_demo.frame(&frame);
        let modal = self.modal_wide_rail_demo.frame(&frame);
        let snackbar = self.snackbar_demo.frame(&frame);
        let drawer = self.drawer_demo.frame(&frame);
        let progress = self.progress_demo.frame(&frame);
        let side_sheet = self.side_sheet_demo.frame(&frame);
        let bottom_sheet = self.bottom_sheet_demo.frame(&frame);
        let carousel = self.carousel_demo.frame(&frame);
        let app_bar = self.app_bar_demo.frame(&frame);
        let toolbar = self.toolbar_demo.frame(&frame);
        AnimationUpdate {
            rebuild: inline.rebuild
                || modal.rebuild
                || snackbar.rebuild
                || drawer.rebuild
                || progress.rebuild
                || side_sheet.rebuild
                || bottom_sheet.rebuild
                || carousel.rebuild
                || app_bar.rebuild
                || toolbar.rebuild,
            request_next: inline.request_next
                || modal.request_next
                || snackbar.request_next
                || drawer.request_next
                || progress.request_next
                || side_sheet.request_next
                || bottom_sheet.request_next
                || carousel.request_next,
        }
    }
    fn update(&mut self, m: Message) {
        match m {
            Message::Section(i) => {
                self.section = i;
                if i != modal_wide_navigation_rail_demo::SECTION {
                    self.modal_wide_rail_demo.close();
                }
                if i != navigation_drawer_demo::SECTION {
                    self.drawer_demo.leave();
                }
                if i != progress_demo::SECTION {
                    self.progress_demo.leave();
                }
                if i != side_sheet_demo::SECTION {
                    self.side_sheet_demo.leave();
                }
                if i != bottom_sheet_demo::SECTION {
                    self.bottom_sheet_demo.leave();
                }
                if i != button_group_demo::SECTION {
                    self.button_group_demo.leave();
                }
                if i != split_demo::SECTION {
                    self.split_demo.leave();
                }
                if i != extended_fab_demo::SECTION {
                    self.extended_fab_demo.leave();
                }
                if i != fab_demo::SECTION {
                    self.fab_demo.leave();
                }
                if i != fab_menu_demo::SECTION {
                    self.fab_menu_demo.leave();
                }
                if i != card_demo::SECTION {
                    self.card_demo.leave();
                }
                if i != carousel_demo::SECTION {
                    self.carousel_demo.leave();
                }
                if i != app_bar_demo::SECTION {
                    self.app_bar_demo.leave();
                }
                if i != toolbar_demo::SECTION {
                    self.toolbar_demo.leave();
                }
                if i != tooltip_demo::SECTION {
                    self.tooltip_demo.leave();
                }
            }
            Message::Rail(i) => self.rail = i,
            Message::RailLargeText => self.rail_large_text = !self.rail_large_text,
            Message::WideRail(event) => self.wide_rail_demo.update(event),
            Message::ModalWideRail(event) => self.modal_wide_rail_demo.update(event),
            Message::Snackbar(event) => self.snackbar_demo.update(event),
            Message::Badge(event) => self.badge_demo.update(event),
            Message::Drawer(i) => self.drawer = i,
            Message::DrawerDemo(event) => self.drawer_demo.update(event),
            Message::Progress(event) => self.progress_demo.update(event),
            Message::SideSheet(event) => self.side_sheet_demo.update(event),
            Message::BottomSheet(event) => self.bottom_sheet_demo.update(event),
            Message::ButtonGroup(event) => self.button_group_demo.update(event),
            Message::Split(event) => self.split_demo.update(event),
            Message::ExtendedFab(event) => self.extended_fab_demo.update(event),
            Message::Fab(event) => self.fab_demo.update(event),
            Message::FabMenu(event) => self.fab_menu_demo.update(event),
            Message::Card(event) => self.card_demo.update(event),
            Message::Carousel(event) => self.carousel_demo.update(event),
            Message::AppBar(event) => self.app_bar_demo.update(event),
            Message::Toolbar(event) => self.toolbar_demo.update(event),
            Message::Tooltip(event) => self.tooltip_demo.update(event),
            Message::Nav(i) => self.nav = i,
            Message::Theme => self.dark = !self.dark,
            Message::Radio(i) => self.radio = i,
            Message::Switch => self.enabled = !self.enabled,
            Message::Check => {
                self.check = if self.check == CheckValue::Checked {
                    CheckValue::Unchecked
                } else {
                    CheckValue::Checked
                }
            }
            Message::Segment(i) => self.segment = i,
            Message::Volume(v) => self.volume = v,
            Message::Click => self.clicks += 1,
            Message::Filter => self.filter = !self.filter,
            Message::RemoveChip => self.input_chip = false,
            Message::Dialog(v) => {
                self.dialog = v;
                self.menu = false;
                if v {
                    self.draft = self.name.clone();
                }
            }
            Message::Menu(v) => self.menu = v,
            Message::Sheet(v) => self.sheet = v,
            Message::Draft(v) => self.draft = v,
            Message::Save => {
                self.name = self.draft.clone();
                self.dialog = false;
            }
        }
    }
    fn view(&self) -> View<Message> {
        let t = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let choice = |selected| ChoiceState {
            selected,
            disabled: false,
        };
        if self.drawer_demo.visible() {
            return self.drawer_demo.view(&t, self.dark);
        }
        if self.progress_demo.visible() {
            return self.progress_demo.view(&t, self.dark);
        }
        if self.side_sheet_demo.visible() {
            return self.side_sheet_demo.view(&t, self.dark);
        }
        if self.bottom_sheet_demo.visible() {
            return self.bottom_sheet_demo.view(&t, self.dark);
        }
        if self.button_group_demo.visible() {
            return self.button_group_demo.view(&t, self.dark);
        }
        if self.split_demo.visible() {
            return self.split_demo.view(&t, self.dark);
        }
        if self.extended_fab_demo.is_open() {
            return self.extended_fab_demo.view(&t, self.dark);
        }
        if self.fab_demo.is_open() {
            return self.fab_demo.view(&t, self.dark);
        }
        if self.fab_menu_demo.in_scene() {
            return self.fab_menu_demo.view(&t, self.dark);
        }
        if self.card_demo.visible() {
            return self.card_demo.view(&t, self.dark);
        }
        if self.carousel_demo.visible() {
            return self.carousel_demo.view(&t, self.dark);
        }
        if self.app_bar_demo.visible() {
            return self.app_bar_demo.view(&t, self.dark);
        }
        if self.toolbar_demo.visible() {
            return self.toolbar_demo.view(&t, self.dark);
        }
        if self.tooltip_demo.visible() {
            return self.tooltip_demo.view(&t, self.dark);
        }
        let mut children = vec![
            t.heading("gallery-title", "Material gallery"),
            t.row([
                t.text("gallery-summary", "Rust state · Desktop GPU · WebGPU"),
                t.switch("theme", "深色模式", choice(self.dark), Message::Theme),
            ]),
            t.tabs(
                "sections",
                "元件分類",
                ["選取", "動作", "浮層", "色彩"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, name)| {
                        t.tab(
                            &format!("section-{i}"),
                            name,
                            choice(self.section == i),
                            Message::Section(i),
                        )
                    }),
            ),
        ];
        match self.section {
            0 => {
                children.push(
                    t.card(
                        "selection-card",
                        [
                            t.text("selection-heading", "選取元件"),
                            t.row([
                                t.checkbox_with(
                                    "check",
                                    "核取方塊",
                                    self.check,
                                    false,
                                    Message::Check,
                                ),
                                t.switch(
                                    "enabled",
                                    "啟用通知",
                                    choice(self.enabled),
                                    Message::Switch,
                                ),
                            ]),
                            t.radio_group(
                                "radios",
                                "配送方式",
                                ["標準", "快捷", "停用"].into_iter().enumerate().map(
                                    |(i, name)| {
                                        t.radio(
                                            &format!("radio-{i}"),
                                            name,
                                            ChoiceState {
                                                selected: self.radio == i,
                                                disabled: i == 2,
                                            },
                                            Message::Radio(i),
                                        )
                                    },
                                ),
                            ),
                            t.segmented_buttons(
                                "segments",
                                "版面",
                                ["清單", "網格", "停用"].into_iter().enumerate().map(
                                    |(i, name)| {
                                        t.segmented_button(
                                            &format!("segment-{i}"),
                                            name,
                                            ChoiceState {
                                                selected: self.segment == i,
                                                disabled: i == 2,
                                            },
                                            Message::Segment(i),
                                        )
                                    },
                                ),
                            ),
                            t.text("volume-label", format!("音量 {}", self.volume)),
                            t.slider(
                                "volume",
                                "音量",
                                RangeValue {
                                    min: 0.,
                                    max: 100.,
                                    step: 5.,
                                    value: self.volume,
                                },
                                false,
                                Message::Volume,
                            ),
                        ],
                    ),
                );
            }
            1 => {
                children.push(t.row([
                    AppBarDemo::show_button(&t),
                    ToolbarDemo::show_button(&t),
                    CarouselDemo::show_button(&t),
                    ButtonGroupDemo::show_button(&t),
                    SplitDemo::show_button(&t),
                    ExtendedFabDemo::show_button(&t),
                    FabDemo::show_button(&t),
                    FabMenuDemo::show_button(&t),
                ]));
                let nav_items = [
                    ("nav-home", "Home", "H"),
                    ("nav-search", "Search", "S"),
                    ("nav-more", "More", "M"),
                ];
                let nav_bar = t.navigation_bar(
                    "nav-bar",
                    "Destinations",
                    nav_items
                        .into_iter()
                        .enumerate()
                        .map(|(i, (id, name, icon))| {
                            t.navigation_bar_item(
                                id,
                                name,
                                icon,
                                choice(self.nav == i),
                                Message::Nav(i),
                            )
                        }),
                );
                children.push(t.card(
                    "action-card",
                    [
                        t.text("click-count", format!("已執行 {} 次", self.clicks)),
                        t.row([
                            t.button_with(
                                "filled",
                                "Filled",
                                ButtonKind::Filled,
                                false,
                                Message::Click,
                            ),
                            t.button_with(
                                "tonal",
                                "Tonal",
                                ButtonKind::Tonal,
                                false,
                                Message::Click,
                            ),
                            t.button_with(
                                "outlined",
                                "Outlined",
                                ButtonKind::Outlined,
                                false,
                                Message::Click,
                            ),
                            t.button_with(
                                "text-button",
                                "Text",
                                ButtonKind::Text,
                                false,
                                Message::Click,
                            ),
                            t.button_with(
                                "disabled-action",
                                "Disabled",
                                ButtonKind::Filled,
                                true,
                                Message::Click,
                            ),
                            ProgressDemo::show_button(&t),
                        ]),
                        t.row([
                            t.chip(
                                "assist",
                                "協助",
                                ChipKind::Assist,
                                choice(false),
                                Message::Click,
                                None,
                            ),
                            t.chip(
                                "suggestion",
                                "建議",
                                ChipKind::Suggestion,
                                choice(false),
                                Message::Click,
                                None,
                            ),
                            t.chip(
                                "filter",
                                "已選取",
                                ChipKind::Filter,
                                choice(self.filter),
                                Message::Filter,
                                None,
                            ),
                        ]),
                        t.segmented_buttons(
                            "segments-multi",
                            "樣式",
                            [
                                t.segment(
                                    "seg-a",
                                    "日",
                                    SegmentPosition::First,
                                    choice(true),
                                    Message::Click,
                                ),
                                t.segment(
                                    "seg-b",
                                    "週",
                                    SegmentPosition::Middle,
                                    choice(false),
                                    Message::Click,
                                ),
                                t.segment(
                                    "seg-c",
                                    "月",
                                    SegmentPosition::Last,
                                    choice(true),
                                    Message::Click,
                                ),
                            ],
                        ),
                        t.row([
                            t.fab(
                                "fab-small",
                                "Add small",
                                "+",
                                FabSize::Small,
                                FabColor::Primary,
                                false,
                                Message::Click,
                            ),
                            t.fab(
                                "fab",
                                "Add",
                                "+",
                                FabSize::Regular,
                                FabColor::Primary,
                                false,
                                Message::Click,
                            ),
                            t.fab(
                                "fab-large",
                                "Add large",
                                "+",
                                FabSize::Large,
                                FabColor::Tertiary,
                                false,
                                Message::Click,
                            ),
                            t.fab(
                                "fab-surface",
                                "Add surface",
                                "+",
                                FabSize::Regular,
                                FabColor::Surface,
                                false,
                                Message::Click,
                            ),
                            t.fab(
                                "fab-secondary",
                                "Add secondary",
                                "+",
                                FabSize::Regular,
                                FabColor::Secondary,
                                false,
                                Message::Click,
                            ),
                            t.fab(
                                "fab-disabled",
                                "Add disabled",
                                "+",
                                FabSize::Regular,
                                FabColor::Primary,
                                true,
                                Message::Click,
                            ),
                        ]),
                        nav_bar,
                        t.row([
                            t.linear_progress(
                                "lp-determinate",
                                "Determinate progress",
                                Some(0.4),
                                0.,
                            ),
                            t.linear_progress("lp-indeterminate", "Loading", None, 0.5),
                        ]),
                        t.row([
                            t.icon_button(
                                "icon-standard",
                                "Favorite",
                                "★",
                                IconButtonKind::Standard,
                                None,
                                false,
                                Message::Click,
                            ),
                            t.icon_button(
                                "icon-filled",
                                "Favorite filled",
                                "★",
                                IconButtonKind::Filled,
                                None,
                                false,
                                Message::Click,
                            ),
                            t.icon_button(
                                "icon-tonal",
                                "Favorite tonal",
                                "★",
                                IconButtonKind::Tonal,
                                None,
                                false,
                                Message::Click,
                            ),
                            t.icon_button(
                                "icon-outlined",
                                "Favorite outlined",
                                "★",
                                IconButtonKind::Outlined,
                                None,
                                false,
                                Message::Click,
                            ),
                            t.icon_button(
                                "icon-toggle",
                                "Favorite toggle",
                                "★",
                                IconButtonKind::Filled,
                                Some(false),
                                false,
                                Message::Click,
                            ),
                            t.icon_button(
                                "icon-disabled",
                                "Favorite disabled",
                                "★",
                                IconButtonKind::Filled,
                                None,
                                true,
                                Message::Click,
                            ),
                        ]),
                        t.row([
                            t.loading_indicator(
                                "loading-default",
                                "Loading",
                                LoadingKind::Default,
                                0.,
                            ),
                            t.loading_indicator(
                                "loading-contained",
                                "Loading contained",
                                LoadingKind::Contained,
                                0.,
                            ),
                        ]),
                    ],
                ));
                if self.input_chip {
                    children.push(t.chip(
                        "input-chip",
                        "計畫 A",
                        ChipKind::Input,
                        choice(false),
                        Message::Click,
                        Some(Message::RemoveChip),
                    ));
                }
                children.push(t.outlined_card(
                    "outlined-card",
                    [t.text("outlined-description", "Outlined card · 共用的色彩與形狀")],
                ));
                children.push(t.divider_with("inset-divider", DividerKind::Inset));
                children.push(t.divider_with("middle-inset-divider", DividerKind::MiddleInset));
                children.push(t.action_card(
                    "elevated-action-card",
                    "Elevated action card",
                    CardKind::Elevated,
                    false,
                    Message::Click,
                    [t.text("elevated-description", "Elevated card · 可點擊")],
                ));
                children.push(t.card_with(
                    "filled-card",
                    "Filled card",
                    CardKind::Filled,
                    [t.text("filled-description", "Filled card")],
                ));
                children.push(
                    t.card(
                        "navigation-rail-card",
                        [t.navigation_rail(
                            "nav-rail",
                            "Destinations",
                            [
                                ("rail-home", "Home", rail_icons::Destination::Home.icon()),
                                ("rail-search", "Search", rail_icons::search()),
                                ("rail-more", "More", rail_icons::more()),
                            ]
                            .into_iter()
                            .enumerate()
                            .map(|(i, (id, name, icon))| {
                                t.navigation_rail_item_with_icon(NavigationRailItem {
                                    id,
                                    label: name,
                                    accessible_name: name,
                                    icon,
                                    state: choice(self.rail == i),
                                    message: Message::Rail(i),
                                })
                            }),
                        )],
                    ),
                );
                children.push(
                    t.card(
                        "navigation-drawer-card",
                        [t.navigation_drawer(
                            "nav-drawer",
                            "Destinations",
                            [
                                ("drawer-inbox", "Inbox", "I"),
                                ("drawer-sent", "Sent", "S"),
                                ("drawer-trash", "Trash", "T"),
                            ]
                            .into_iter()
                            .enumerate()
                            .map(|(i, (id, name, icon))| {
                                t.navigation_drawer_item(
                                    id,
                                    name,
                                    icon,
                                    choice(self.drawer == i),
                                    Message::Drawer(i),
                                )
                            }),
                        )],
                    ),
                );
                children.push(self.wide_rail_demo.card(&t, self.rail_large_text));
                children.push(self.modal_wide_rail_demo.card(&t, self.rail_large_text));
            }
            2 => {
                children.push(t.card(
                    "overlay-card",
                    [
                        t.text("saved-name", format!("名稱：{}", self.name)),
                        t.row([
                            t.button_with(
                                "open-dialog",
                                "重新命名",
                                ButtonKind::Filled,
                                false,
                                Message::Dialog(true),
                            ),
                            t.button_with(
                                "open-menu",
                                "開啟選單",
                                ButtonKind::Outlined,
                                false,
                                Message::Menu(true),
                            ),
                            t.button_with(
                                "open-sheet",
                                "開啟底部面板",
                                ButtonKind::Outlined,
                                false,
                                Message::Sheet(true),
                            ),
                            self.drawer_demo.show_button(&t),
                            SideSheetDemo::show_button(&t),
                        ]),
                        BottomSheetDemo::show_button(&t),
                        TooltipDemo::show_button(&t),
                        t.text("overlay-help", "使用 Tab 切換焦點。按 Escape 關閉。"),
                    ],
                ));
                children.push(self.snackbar_demo.card(&t));
            }
            _ => {
                children.push(CardDemo::show_button(&t));
                children.push(self.badge_demo.card(&t));
                let swatch = |id: &str, text: &str, background: Color, foreground: Color| {
                    View::new(
                        Node::new(Role::Group, text).id(id).styled(
                            Style::default()
                                .padding(24.)
                                .radius(16.)
                                .background(background),
                        ),
                    )
                    .children([View::new(
                        Node::new(Role::Text, text)
                            .text(text)
                            .styled(Style::default().foreground(foreground)),
                    )])
                };
                children.extend([
                    swatch("primary-swatch", "Primary", t.primary, t.on_primary),
                    swatch(
                        "secondary-swatch",
                        "Secondary container",
                        t.colors.secondary_container,
                        t.colors.on_secondary_container,
                    ),
                    swatch(
                        "tertiary-swatch",
                        "Tertiary container",
                        t.colors.tertiary_container,
                        t.colors.on_tertiary_container,
                    ),
                    swatch("error-swatch", "Error", t.colors.error, t.colors.on_error),
                ]);
            }
        }
        children.extend(self.snackbar_demo.layer(&t));
        if self.dialog {
            children.push(t.dialog_with(
                "rename-dialog",
                Some("✎"),
                "重新命名",
                "輸入新名稱。取消會保留原名稱。",
                [t.text_field("dialog-name", "名稱", &self.draft, Message::Draft)],
                [
                    t.button_with(
                        "dialog-cancel",
                        "取消",
                        ButtonKind::Text,
                        false,
                        Message::Dialog(false),
                    ),
                    t.button_with(
                        "dialog-save",
                        "儲存",
                        ButtonKind::Filled,
                        false,
                        Message::Save,
                    ),
                ],
                Message::Dialog(false),
            ));
        }
        if self.menu {
            children.push(t.menu_at(
                "options-menu",
                "選單",
                (80., 240.),
                [
                    t.menu_item("menu-rename", "重新命名", false, Message::Dialog(true)),
                    t.menu_item("menu-disabled", "停用項目", true, Message::Click),
                    t.menu_item("menu-close", "關閉選單", false, Message::Menu(false)),
                ],
                Message::Menu(false),
            ));
        }
        if self.sheet {
            children.push(t.modal_bottom_sheet(
                "gallery-sheet",
                "底部面板",
                [
                    t.text("sheet-help", "這是底部面板。"),
                    t.button_with(
                        "sheet-close",
                        "關閉",
                        ButtonKind::Text,
                        false,
                        Message::Sheet(false),
                    ),
                ],
                Message::Sheet(false),
            ));
        }
        if self.section == modal_wide_navigation_rail_demo::SECTION {
            children.extend(self.modal_wide_rail_demo.layer(&t));
        }
        t.page(children)
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    use surface_app::Runtime;
    use wasm_bindgen::prelude::*;
    thread_local! {static APP:RefCell<Option<Rc<RefCell<Runtime<Gallery>>>>>=const{RefCell::new(None)};}
    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        console_error_panic_hook::set_once();
        let runtime = Rc::new(RefCell::new(
            Runtime::new(Gallery::default()).map_err(|e| JsValue::from_str(&e.to_string()))?,
        ));
        APP.with(|a| *a.borrow_mut() = Some(runtime.clone()));
        surface_platform::run_runtime(runtime, Default::default())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    #[wasm_bindgen]
    pub fn export_dom() -> Result<String, JsValue> {
        APP.with(|app| {
            app.borrow()
                .as_ref()
                .expect("app started")
                .borrow()
                .export_dom()
                .map_err(|error| JsValue::from_str(&error.to_string()))
        })
    }
    #[wasm_bindgen]
    pub fn inspect() -> String {
        APP.with(|a|{
        let a=a.borrow();let runtime=a.as_ref().expect("started").borrow();let s=runtime.app();
        serde_json::json!({"section":s.section,"dark":s.dark,"radio":s.radio,"enabled":s.enabled,"check":format!("{:?}",s.check),"segment":s.segment,
            "volume":s.volume,"clicks":s.clicks,"filter":s.filter,"input_chip":s.input_chip,"dialog":s.dialog,"menu":s.menu,"name":s.name,"draft":s.draft,"nav":s.nav,"focused":runtime.focused(),
            "wide_navigation_rail": s.wide_rail_demo.inspect(s.text_scale()),
            "modal_wide_navigation_rail": s.modal_wide_rail_demo.inspect(s.text_scale(), &runtime),
            "snackbar": s.snackbar_demo.inspect(s.text_scale()),
            "badges": s.badge_demo.inspect(s.text_scale(), s.dark),
            "navigation_drawer": s.drawer_demo.inspect(s.text_scale(), &runtime),
            "progress": s.progress_demo.inspect(&if s.dark { Theme::dark() } else { Theme::light() }),
            "side_sheet": s.side_sheet_demo.inspect(&if s.dark { Theme::dark() } else { Theme::light() }),
            "bottom_sheet": s.bottom_sheet_demo.inspect(),
            "cards": s.card_demo.inspect(),
            "button_group": s.button_group_demo.inspect(),
            "split_button": s.split_demo.inspect(),
            "extended_fab": s.extended_fab_demo.inspect(),
            "expressive_fab": s.fab_demo.inspect(),
            "fab_menu": s.fab_menu_demo.inspect(),
            "carousel": s.carousel_demo.inspect(&runtime),
            "app_bar": s.app_bar_demo.inspect(&if s.dark { Theme::dark() } else { Theme::light() }),
            "toolbar": s.toolbar_demo.inspect(&if s.dark { Theme::dark() } else { Theme::light() }),
            "tooltip": s.tooltip_demo.inspect(&if s.dark { Theme::dark() } else { Theme::light() })
        }).to_string()
    })
    }
    #[wasm_bindgen]
    pub fn control_bounds(id: &str) -> Vec<f32> {
        APP.with(|a| {
            let a = a.borrow();
            let r = a.as_ref().expect("started").borrow();
            let Some(id) = r.dom().by_id(id) else {
                return vec![];
            };
            let Some(c) = r.computed() else { return vec![] };
            let b = c.nodes[id.index()].bounds;
            vec![b.x, b.y, b.width, b.height]
        })
    }
}
