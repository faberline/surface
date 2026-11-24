mod app_bar_demo;
mod badge_demo;
mod bottom_sheet_demo;
mod button_group_demo;
mod card_demo;
mod carousel_demo;
mod date_picker_demo;
mod date_range_picker_demo;
mod dialog_demo;
mod divider_demo;
mod docked_date_picker_demo;
mod extended_fab_demo;
mod fab_demo;
mod fab_menu_demo;
mod flexible_navigation_bar_demo;
mod list_actions_demo;
mod list_expand_demo;
mod list_media_demo;
mod list_reorder_demo;
mod lists_demo;
mod loading_indicator_demo;
mod menu_demo;
mod modal_wide_navigation_rail_demo;
mod navigation_drawer_demo;
mod progress_demo;
mod rail_icons;
mod scroll_choice_demo;
mod scroll_field_demo;
mod scroll_modal_demo;
mod scroll_pair_demo;
mod scroll_popup_demo;
mod search_overlay_demo;
mod side_sheet_demo;
mod snackbar_demo;
mod split_demo;
mod switch_demo;
mod time_dial_demo;
mod time_input_demo;
mod time_scroll_demo;
mod toolbar_demo;
mod tooltip_demo;
mod wide_navigation_rail_demo;

use app_bar_demo::{AppBarDemo, AppBarEvent};
use badge_demo::{BadgeDemo, BadgeEvent};
use bottom_sheet_demo::{BottomSheetDemo, BottomSheetEvent};
use button_group_demo::{ButtonGroupDemo, ButtonGroupEvent};
use card_demo::{CardDemo, CardEvent};
use carousel_demo::{CarouselDemo, CarouselEvent};
use date_picker_demo::{DatePickerDemo, DatePickerDemoEvent};
use date_range_picker_demo::{DateRangeDemo, DateRangeDemoEvent};
use dialog_demo::{DialogDemo, DialogDemoEvent};
use divider_demo::{DividerDemo, DividerDemoEvent};
use docked_date_picker_demo::{DockedDateDemo, DockedDateDemoEvent};
use extended_fab_demo::{ExtendedFabDemo, ExtendedFabEvent};
use fab_demo::{FabDemo, FabEvent};
use fab_menu_demo::{FabMenuDemo, FabMenuEvent};
use flexible_navigation_bar_demo::{FlexibleNavigationBarDemo, FlexibleNavigationBarDemoEvent};
use list_actions_demo::{ListActionsDemo, ListActionsEvent};
use list_expand_demo::{ListExpandDemo, ListExpandEvent};
use list_media_demo::{ListMediaDemo, ListMediaEvent};
use list_reorder_demo::{ListReorderDemo, ListReorderEvent};
use lists_demo::{ListsDemo, ListsEvent};
use loading_indicator_demo::{LoadingDemo, LoadingEvent};
use menu_demo::{MenuDemo, MenuDemoEvent};
use modal_wide_navigation_rail_demo::{ModalWideRailDemo, ModalWideRailEvent};
use navigation_drawer_demo::{DrawerDemo, DrawerDemoEvent};
use progress_demo::{ProgressDemo, ProgressEvent};
use scroll_choice_demo::{ScrollChoiceDemo, ScrollChoiceDemoEvent};
use scroll_field_demo::{ScrollFieldDemo, ScrollFieldDemoEvent};
use scroll_modal_demo::{ScrollModalDemo, ScrollModalDemoEvent};
use scroll_pair_demo::{ScrollPairDemo, ScrollPairDemoEvent};
use scroll_popup_demo::{ScrollPopupDemo, ScrollPopupDemoEvent};
use search_overlay_demo::{SearchOverlayDemo, SearchOverlayDemoEvent};
use side_sheet_demo::{SideSheetDemo, SideSheetEvent};
use snackbar_demo::{SnackbarDemo, SnackbarEvent};
use split_demo::{SplitDemo, SplitEvent};
use surface::{Color, Node, RangeValue, Role, Style};
use surface_app::{
    AnimationFrame, AnimationUpdate, Application, DisplayContrast, FrameInput, FramePress,
    InputAnimationUpdate, Queue, SystemColorPair, View,
};
use surface_material::{
    ButtonKind, CardKind, CheckValue, ChipKind, ChoiceState, DividerKind, FabColor, FabSize,
    IconButtonKind, NavigationRailItem, SegmentPosition, Theme,
};
use switch_demo::{SwitchDemo, SwitchDemoEvent};
use time_dial_demo::{TimeDialDemo, TimeDialDemoEvent};
use time_input_demo::{TimeInputDemo, TimeInputDemoEvent};
use time_scroll_demo::{TimeScrollDemo, TimeScrollDemoEvent};
use toolbar_demo::{ToolbarDemo, ToolbarEvent};
use tooltip_demo::{TooltipDemo, TooltipEvent};
use wide_navigation_rail_demo::{WideRailDemo, WideRailEvent};

pub struct Gallery {
    pub section: usize,
    pub dark: bool,
    display_contrast: DisplayContrast,
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
    divider_demo: DividerDemo,
    switch_demo: SwitchDemo,
    drawer_demo: DrawerDemo,
    progress_demo: ProgressDemo,
    loading_demo: LoadingDemo,
    side_sheet_demo: SideSheetDemo,
    bottom_sheet_demo: BottomSheetDemo,
    button_group_demo: ButtonGroupDemo,
    split_demo: SplitDemo,
    date_picker_demo: DatePickerDemo,
    date_range_picker_demo: DateRangeDemo,
    time_input_demo: TimeInputDemo,
    time_dial_demo: TimeDialDemo,
    time_scroll_demo: TimeScrollDemo,
    scroll_field_demo: ScrollFieldDemo,
    scroll_choice_demo: ScrollChoiceDemo,
    scroll_pair_demo: ScrollPairDemo,
    scroll_popup_demo: ScrollPopupDemo,
    scroll_modal_demo: ScrollModalDemo,
    search_overlay_demo: SearchOverlayDemo,
    dialog_demo: DialogDemo,
    menu_demo: MenuDemo,
    flexible_navigation_bar_demo: FlexibleNavigationBarDemo,
    lists_demo: ListsDemo,
    list_actions_demo: ListActionsDemo,
    list_expand_demo: ListExpandDemo,
    list_media_demo: ListMediaDemo,
    list_reorder_demo: ListReorderDemo,
    queue: Option<Queue<Message>>,
    docked_date_picker_demo: DockedDateDemo,
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
            display_contrast: DisplayContrast::Normal,
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
            divider_demo: DividerDemo::default(),
            switch_demo: SwitchDemo::default(),
            drawer_demo: DrawerDemo::default(),
            progress_demo: ProgressDemo::default(),
            loading_demo: LoadingDemo::default(),
            side_sheet_demo: SideSheetDemo::default(),
            bottom_sheet_demo: BottomSheetDemo::default(),
            button_group_demo: ButtonGroupDemo::default(),
            split_demo: SplitDemo::default(),
            date_picker_demo: DatePickerDemo::default(),
            date_range_picker_demo: DateRangeDemo::default(),
            time_input_demo: TimeInputDemo::default(),
            time_dial_demo: TimeDialDemo::default(),
            time_scroll_demo: TimeScrollDemo::default(),
            scroll_field_demo: ScrollFieldDemo::default(),
            scroll_choice_demo: ScrollChoiceDemo::default(),
            scroll_pair_demo: ScrollPairDemo::default(),
            scroll_popup_demo: ScrollPopupDemo::default(),
            scroll_modal_demo: ScrollModalDemo::default(),
            search_overlay_demo: SearchOverlayDemo::default(),
            dialog_demo: DialogDemo::default(),
            menu_demo: MenuDemo::default(),
            flexible_navigation_bar_demo: FlexibleNavigationBarDemo::default(),
            lists_demo: ListsDemo::default(),
            list_actions_demo: ListActionsDemo::default(),
            list_expand_demo: ListExpandDemo::default(),
            list_media_demo: ListMediaDemo::default(),
            list_reorder_demo: ListReorderDemo::default(),
            queue: None,
            docked_date_picker_demo: DockedDateDemo::default(),
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
impl Gallery {
    fn effective_theme(&self) -> Theme {
        match self.display_contrast {
            DisplayContrast::Forced(colors) => Theme::from_forced_colors(
                (colors.canvas.background, colors.canvas.foreground),
                (colors.button.background, colors.button.foreground),
                forced_dark(colors.canvas, self.dark),
            ),
            DisplayContrast::Increased => {
                // The Gallery uses the same authored seed as Theme::light/dark.
                static LIGHT: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
                static DARK: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
                let cache = if self.dark { &DARK } else { &LIGHT };
                *cache.get_or_init(|| {
                    Theme::from_seed_high_contrast(Color::rgb(103, 80, 164), self.dark)
                })
            }
            _ if self.dark => Theme::dark(),
            _ => Theme::light(),
        }
    }
}

fn forced_dark(canvas: SystemColorPair, authored_dark: bool) -> bool {
    if canvas.background.3 != 255 || canvas.foreground.3 != 255 {
        return authored_dark;
    }
    let luminance = |Color(r, g, b, _)| {
        let linear = |byte: u8| {
            let channel = f64::from(byte) / 255.;
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
    };
    let background = luminance(canvas.background);
    let foreground = luminance(canvas.foreground);
    if (background - foreground).abs() <= f64::EPSILON {
        authored_dark
    } else {
        background < foreground
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
    DividerDemo(DividerDemoEvent),
    Drawer(usize),
    DrawerDemo(DrawerDemoEvent),
    Progress(ProgressEvent),
    Loading(LoadingEvent),
    SideSheet(SideSheetEvent),
    BottomSheet(BottomSheetEvent),
    ButtonGroup(ButtonGroupEvent),
    Split(SplitEvent),
    DatePicker(DatePickerDemoEvent),
    DateRange(DateRangeDemoEvent),
    TimeInput(TimeInputDemoEvent),
    TimeDial(TimeDialDemoEvent),
    TimeScroll(TimeScrollDemoEvent),
    ScrollField(ScrollFieldDemoEvent),
    SwitchDemo(SwitchDemoEvent),
    ScrollChoice(ScrollChoiceDemoEvent),
    ScrollPair(ScrollPairDemoEvent),
    ScrollPopup(ScrollPopupDemoEvent),
    ScrollModal(ScrollModalDemoEvent),
    SearchOverlay(SearchOverlayDemoEvent),
    DialogDemo(DialogDemoEvent),
    MenuDemo(MenuDemoEvent),
    FlexibleNavigationBarDemo(FlexibleNavigationBarDemoEvent),
    Lists(ListsEvent),
    ListActions(ListActionsEvent),
    ListExpand(ListExpandEvent),
    ListMedia(ListMediaEvent),
    ListReorder(ListReorderEvent),
    DockedDate(DockedDateDemoEvent),
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

    fn display_contrast_changed(&mut self, contrast: DisplayContrast) -> bool {
        if self.display_contrast == contrast {
            return false;
        }
        self.display_contrast = contrast;
        true
    }

    fn text_scale(&self) -> f32 {
        if self.drawer_demo.visible() {
            return self.drawer_demo.text_scale();
        }
        if self.progress_demo.visible() {
            return self.progress_demo.text_scale();
        }
        if self.loading_demo.visible() {
            return self.loading_demo.text_scale();
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
        if self.date_picker_demo.visible() {
            return self.date_picker_demo.text_scale();
        }
        if self.docked_date_picker_demo.visible() {
            return self.docked_date_picker_demo.text_scale();
        }
        if self.date_range_picker_demo.visible() {
            return self.date_range_picker_demo.text_scale();
        }
        if self.time_input_demo.visible() {
            return self.time_input_demo.text_scale();
        }
        if self.time_dial_demo.visible() {
            return self.time_dial_demo.text_scale();
        }
        if self.time_scroll_demo.visible() {
            return self.time_scroll_demo.text_scale();
        }
        if self.scroll_field_demo.visible() {
            return self.scroll_field_demo.text_scale();
        }
        if self.scroll_choice_demo.visible() {
            return self.scroll_choice_demo.text_scale();
        }
        if self.scroll_pair_demo.visible() {
            return self.scroll_pair_demo.text_scale();
        }
        if self.scroll_popup_demo.visible() {
            return self.scroll_popup_demo.text_scale();
        }
        if self.scroll_modal_demo.visible() {
            return self.scroll_modal_demo.text_scale();
        }
        if self.search_overlay_demo.visible() {
            return self.search_overlay_demo.text_scale();
        }
        if self.dialog_demo.visible() {
            return self.dialog_demo.text_scale();
        }
        if self.menu_demo.visible() {
            return self.menu_demo.text_scale();
        }
        if self.flexible_navigation_bar_demo.visible() {
            return self.flexible_navigation_bar_demo.text_scale();
        }
        if self.lists_demo.visible() {
            return self.lists_demo.text_scale();
        }
        if self.list_actions_demo.visible() {
            return self.list_actions_demo.text_scale();
        }
        if self.list_expand_demo.visible() {
            return self.list_expand_demo.text_scale();
        }
        if self.list_media_demo.visible() {
            return self.list_media_demo.text_scale();
        }
        if self.list_reorder_demo.visible() {
            return self.list_reorder_demo.text_scale();
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
        if self.divider_demo.visible() {
            return self.divider_demo.text_scale();
        }
        if self.switch_demo.visible() {
            return self.switch_demo.text_scale();
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
        let loading = self.loading_demo.viewport_changed(width);
        let side_sheet = self.side_sheet_demo.viewport_changed(width, height);
        let bottom_sheet = self.bottom_sheet_demo.viewport_changed(height);
        let button_group = self.button_group_demo.viewport_changed(width);
        let split = self.split_demo.viewport_changed(width);
        let date_picker = self.date_picker_demo.viewport_changed(width, height);
        let docked_date = self.docked_date_picker_demo.viewport_changed(width, height);
        let date_range = self.date_range_picker_demo.viewport_changed(width, height);
        let time_input = self.time_input_demo.viewport_changed(width, height);
        let time_dial = self.time_dial_demo.viewport_changed(width, height);
        let time_scroll = self.time_scroll_demo.viewport_changed(width, height);
        let scroll_field = self.scroll_field_demo.viewport_changed(width, height);
        let scroll_choice = self.scroll_choice_demo.viewport_changed(width, height);
        let scroll_pair = self.scroll_pair_demo.viewport_changed(width, height);
        let scroll_popup = self.scroll_popup_demo.viewport_changed(width, height);
        let scroll_modal = self.scroll_modal_demo.viewport_changed(width, height);
        let search_overlay = self.search_overlay_demo.viewport_changed(width, height);
        let dialog = self.dialog_demo.viewport_changed(width, height);
        let flexible_navigation_bar = self.flexible_navigation_bar_demo.viewport_changed(width);
        let list_actions = self.list_actions_demo.viewport_changed(width);
        let list_media = self.list_media_demo.viewport_changed(width, height);
        let list_reorder = self.list_reorder_demo.viewport_changed(width);
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
            || (self.loading_demo.visible() && loading)
            || (self.side_sheet_demo.visible() && side_sheet)
            || (self.bottom_sheet_demo.visible() && bottom_sheet)
            || button_group
            || split
            || extended_fab
            || fab
            || fab_menu
            || date_picker
            || docked_date
            || date_range
            || time_input
            || time_dial
            || time_scroll
            || scroll_field
            || scroll_choice
            || scroll_pair
            || scroll_popup
            || scroll_modal
            || search_overlay
            || dialog
            || flexible_navigation_bar
            || list_actions
            || list_media
            || list_reorder
            || carousel
            || app_bar
            || toolbar
            || tooltip
    }
    fn observes_frame_input(&self) -> bool {
        self.switch_demo.visible()
            || self.tooltip_demo.visible()
            || self.scroll_popup_demo.visible()
            || self.button_group_demo.visible()
            || self.split_demo.visible()
            || self.extended_fab_demo.is_open()
            || self.fab_demo.is_open()
            || self.fab_menu_demo.in_scene()
            || self.date_picker_demo.observes_input()
            || self.docked_date_picker_demo.visible()
            || self.date_range_picker_demo.observes_input()
            || self.time_input_demo.visible()
            || self.time_dial_demo.visible()
            || self.time_scroll_demo.visible()
    }
    fn attach_queue(&mut self, queue: Queue<Message>) {
        self.queue = Some(queue);
    }
    fn cancel_transient(&mut self) -> bool {
        (self.tooltip_demo.visible() && self.tooltip_demo.dismiss())
            || (self.fab_demo.is_open() && self.fab_demo.dismiss_tooltip())
            || self.list_reorder_demo.cancel()
    }
    fn animation_frame_with_input(
        &mut self,
        frame: AnimationFrame<'_>,
        input: FrameInput<'_>,
    ) -> InputAnimationUpdate {
        let mut update = self.tooltip_demo.frame(&frame, input);
        let date = self
            .date_picker_demo
            .observe(&frame, input.focused, self.queue.as_ref());
        update.animation.rebuild |= date.rebuild;
        update.animation.request_next |= date.request_next;
        let docked = self.docked_date_picker_demo.observe(&frame, input);
        update.animation.rebuild |= docked.rebuild;
        update.animation.request_next |= docked.request_next;
        let scroll_popup = self.scroll_popup_demo.observe(&frame);
        update.animation.rebuild |= scroll_popup.rebuild;
        update.animation.request_next |= scroll_popup.request_next;
        let search_overlay = self.search_overlay_demo.observe(&frame);
        update.animation.rebuild |= search_overlay.rebuild;
        update.animation.request_next |= search_overlay.request_next;
        let range = self.date_range_picker_demo.observe(&frame, input);
        update.animation.rebuild |= range.rebuild;
        update.animation.request_next |= range.request_next;
        update.animation.rebuild |= self.time_input_demo.observe_focus(input.focused);
        update.animation.rebuild |= self.time_dial_demo.observe_focus(input.focused);
        update.animation.rebuild |= self.time_scroll_demo.observe_focus(input.focused);
        if let Some(queue) = &self.queue {
            self.date_range_picker_demo.flush_scroll_requests(queue);
        }
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
        let switch = self.switch_demo.frame(&frame, &input, press, hovered);
        let mut update = self.animation_frame_with_press(frame, input, pressed);
        update.animation.rebuild |= fab.animation.rebuild || menu.rebuild || switch.rebuild;
        update.animation.request_next |=
            fab.animation.request_next || menu.request_next || switch.request_next;
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
        let loading = self.loading_demo.frame(&frame);
        let side_sheet = self.side_sheet_demo.frame(&frame);
        let bottom_sheet = self.bottom_sheet_demo.frame(&frame);
        let carousel = self.carousel_demo.frame(&frame);
        let app_bar = self.app_bar_demo.frame(&frame);
        let toolbar = self.toolbar_demo.frame(&frame);
        let time_dial = self.time_dial_demo.frame(&frame);
        let time_scroll = self.time_scroll_demo.frame(&frame);
        let scroll_field = self.scroll_field_demo.frame(&frame);
        let scroll_choice = self.scroll_choice_demo.frame(&frame);
        let scroll_pair = self.scroll_pair_demo.frame(&frame);
        let scroll_popup = self.scroll_popup_demo.frame(&frame);
        let scroll_modal = self.scroll_modal_demo.frame(&frame);
        let dialog = self.dialog_demo.frame(&frame);
        let menu = self.menu_demo.frame(&frame);
        let list_expand = self.list_expand_demo.frame(&frame);
        let list_reorder = self.list_reorder_demo.frame(&frame, self.queue.as_ref());
        AnimationUpdate {
            rebuild: inline.rebuild
                || modal.rebuild
                || snackbar.rebuild
                || drawer.rebuild
                || progress.rebuild
                || loading.rebuild
                || side_sheet.rebuild
                || bottom_sheet.rebuild
                || carousel.rebuild
                || app_bar.rebuild
                || toolbar.rebuild
                || time_dial.rebuild
                || time_scroll.rebuild
                || scroll_field.rebuild
                || scroll_choice.rebuild
                || scroll_pair.rebuild
                || scroll_popup.rebuild
                || scroll_modal.rebuild
                || dialog.rebuild
                || menu.rebuild
                || list_expand.rebuild
                || list_reorder.rebuild,
            request_next: inline.request_next
                || modal.request_next
                || snackbar.request_next
                || drawer.request_next
                || progress.request_next
                || loading.request_next
                || side_sheet.request_next
                || bottom_sheet.request_next
                || carousel.request_next
                || time_dial.request_next
                || time_scroll.request_next
                || scroll_field.request_next
                || scroll_choice.request_next
                || scroll_pair.request_next
                || scroll_popup.request_next
                || scroll_modal.request_next
                || dialog.request_next
                || menu.request_next
                || list_expand.request_next
                || list_reorder.request_next,
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
                if i != loading_indicator_demo::SECTION {
                    self.loading_demo.leave();
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
                if i != date_picker_demo::SECTION {
                    self.date_picker_demo.leave();
                }
                if i != docked_date_picker_demo::SECTION {
                    self.docked_date_picker_demo.leave();
                }
                if i != date_range_picker_demo::SECTION {
                    self.date_range_picker_demo.leave();
                }
                if i != time_input_demo::SECTION {
                    self.time_input_demo.leave();
                }
                if i != time_dial_demo::SECTION {
                    self.time_dial_demo.leave();
                }
                if i != time_scroll_demo::SECTION {
                    self.time_scroll_demo.leave();
                }
                if i != scroll_field_demo::SECTION {
                    self.scroll_field_demo.leave();
                }
                if i != scroll_choice_demo::SECTION {
                    self.scroll_choice_demo.leave();
                }
                if i != scroll_pair_demo::SECTION {
                    self.scroll_pair_demo.leave();
                }
                if i != scroll_popup_demo::SECTION {
                    self.scroll_popup_demo.leave();
                }
                if i != scroll_modal_demo::SECTION {
                    self.scroll_modal_demo.leave();
                }
                if i != lists_demo::SECTION {
                    self.lists_demo.leave();
                    self.list_actions_demo.leave();
                    self.list_expand_demo.leave();
                    self.list_media_demo.leave();
                    self.list_reorder_demo.leave();
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
                if i != search_overlay_demo::SECTION {
                    self.search_overlay_demo.leave();
                }
                if i != dialog_demo::SECTION {
                    self.dialog_demo.leave();
                }
                if i != menu_demo::SECTION {
                    self.menu_demo.leave();
                }
                if i != flexible_navigation_bar_demo::SECTION {
                    self.flexible_navigation_bar_demo.leave();
                }
                if i != toolbar_demo::SECTION {
                    self.toolbar_demo.leave();
                }
                if i != divider_demo::SECTION {
                    self.divider_demo.leave();
                }
                if i != switch_demo::SECTION {
                    self.switch_demo.leave();
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
            Message::DividerDemo(event) => self.divider_demo.update(event),
            Message::SwitchDemo(event) => self.switch_demo.update(event),
            Message::Drawer(i) => self.drawer = i,
            Message::DrawerDemo(event) => self.drawer_demo.update(event),
            Message::Progress(event) => self.progress_demo.update(event),
            Message::Loading(event) => self.loading_demo.update(event),
            Message::SideSheet(event) => self.side_sheet_demo.update(event),
            Message::BottomSheet(event) => self.bottom_sheet_demo.update(event),
            Message::ButtonGroup(event) => self.button_group_demo.update(event),
            Message::Split(event) => self.split_demo.update(event),
            Message::DatePicker(event) => self.date_picker_demo.update(event),
            Message::DockedDate(event) => self.docked_date_picker_demo.update(event),
            Message::DateRange(event) => self.date_range_picker_demo.update(event),
            Message::TimeInput(event) => self.time_input_demo.update(event),
            Message::TimeDial(event) => self.time_dial_demo.update(event),
            Message::TimeScroll(event) => self.time_scroll_demo.update(event),
            Message::ScrollField(event) => self.scroll_field_demo.update(event),
            Message::ScrollChoice(event) => self.scroll_choice_demo.update(event),
            Message::ScrollPair(event) => self.scroll_pair_demo.update(event),
            Message::ScrollPopup(event) => self.scroll_popup_demo.update(event),
            Message::ScrollModal(event) => {
                let reveal = self.scroll_modal_demo.update(event);
                if let Some(queue) = self.queue.as_ref().filter(|_| reveal) {
                    ScrollModalDemo::reveal_validation(queue);
                }
            }
            Message::Lists(event) => self.lists_demo.update(event),
            Message::ListActions(event) => self.list_actions_demo.update(event),
            Message::ListExpand(event) => self.list_expand_demo.update(event),
            Message::ListMedia(event) => self.list_media_demo.update(event),
            Message::ListReorder(event) => {
                self.list_reorder_demo.update(event, self.queue.as_ref())
            }
            Message::ExtendedFab(event) => self.extended_fab_demo.update(event),
            Message::Fab(event) => self.fab_demo.update(event),
            Message::FabMenu(event) => self.fab_menu_demo.update(event),
            Message::Card(event) => self.card_demo.update(event),
            Message::Carousel(event) => self.carousel_demo.update(event),
            Message::AppBar(event) => self.app_bar_demo.update(event),
            Message::SearchOverlay(event) => self.search_overlay_demo.update(event),
            Message::DialogDemo(event) => self.dialog_demo.update(event),
            Message::MenuDemo(event) => self.menu_demo.update(event),
            Message::FlexibleNavigationBarDemo(event) => {
                self.flexible_navigation_bar_demo.update(event)
            }
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
        let t = self.effective_theme();
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
        if self.loading_demo.visible() {
            return self.loading_demo.view(&t, self.dark);
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
        if self.date_picker_demo.visible() {
            return self.date_picker_demo.view(&t);
        }
        if self.docked_date_picker_demo.visible() {
            return self.docked_date_picker_demo.view(&t);
        }
        if self.date_range_picker_demo.visible() {
            return self.date_range_picker_demo.view(&t);
        }
        if self.time_input_demo.visible() {
            return self.time_input_demo.view(&t);
        }
        if self.time_dial_demo.visible() {
            return self.time_dial_demo.view(&t);
        }
        if self.time_scroll_demo.visible() {
            return self.time_scroll_demo.view(&t);
        }
        if self.scroll_field_demo.visible() {
            return self.scroll_field_demo.view(&t, self.dark);
        }
        if self.scroll_choice_demo.visible() {
            return self.scroll_choice_demo.view(&t, self.dark);
        }
        if self.scroll_pair_demo.visible() {
            return self.scroll_pair_demo.view(&t, self.dark);
        }
        if self.scroll_popup_demo.visible() {
            return self.scroll_popup_demo.view(&t, self.dark);
        }
        if self.scroll_modal_demo.visible() {
            return self.scroll_modal_demo.view(&t, self.dark);
        }
        if self.lists_demo.visible() {
            return self.lists_demo.view(&t);
        }
        if self.list_actions_demo.visible() {
            return self.list_actions_demo.view(&t);
        }
        if self.list_expand_demo.visible() {
            return self.list_expand_demo.view(&t);
        }
        if self.list_reorder_demo.visible() {
            return self.list_reorder_demo.view(&t);
        }
        if self.list_media_demo.visible() {
            return self.list_media_demo.view(&t);
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
        if self.search_overlay_demo.visible() {
            return self.search_overlay_demo.view(&t, self.dark);
        }
        if self.dialog_demo.visible() {
            return self.dialog_demo.view(&t, self.dark);
        }
        if self.menu_demo.visible() {
            return self.menu_demo.view(&t, self.dark);
        }
        if self.flexible_navigation_bar_demo.visible() {
            return self.flexible_navigation_bar_demo.view(&t, self.dark);
        }
        if self.toolbar_demo.visible() {
            return self.toolbar_demo.view(&t, self.dark);
        }
        if self.tooltip_demo.visible() {
            return self.tooltip_demo.view(&t, self.dark);
        }
        if self.divider_demo.visible() {
            return self.divider_demo.view(&t, self.dark);
        }
        if self.switch_demo.visible() {
            return self.switch_demo.view(&t, self.dark);
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
                            ScrollFieldDemo::show_button(&t, false),
                            ScrollChoiceDemo::show_button(&t, false),
                            ScrollFieldDemo::show_button(&t, true),
                            ScrollChoiceDemo::show_button(&t, true),
                            ScrollPairDemo::show_button(&t),
                            ScrollPopupDemo::show_button(&t),
                            ScrollModalDemo::show_button(&t),
                            SwitchDemo::show_button(&t),
                        ],
                    ),
                );
            }
            1 => {
                children.push(t.row([
                    AppBarDemo::show_button(&t),
                    SearchOverlayDemo::show_button(&t),
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
                        LoadingDemo::show_button(&t),
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
                children.push(DatePickerDemo::show_button(&t));
                children.push(DockedDateDemo::show_button(&t));
                children.push(DateRangeDemo::show_button(&t));
                children.push(TimeInputDemo::show_button(&t));
                children.push(TimeDialDemo::show_button(&t));
                children.push(TimeScrollDemo::show_button(&t));
                children.push(TimeInputDemo::show_baseline_button(&t));
                children.push(TimeDialDemo::show_baseline_button(&t));
                children.push(ListsDemo::show_button(&t));
                children.push(ListActionsDemo::show_button(&t));
                children.push(ListExpandDemo::show_button(&t));
                children.push(ListReorderDemo::show_button(&t));
                children.push(ListMediaDemo::show_button(&t));
                children.push(t.row([
                    DialogDemo::show_button(&t),
                    DialogDemo::show_transform_button(&t),
                ]));
                children.push(MenuDemo::show_button(&t));
            }
            _ => {
                children.push(CardDemo::show_button(&t));
                children.push(self.badge_demo.card(&t));
                children.push(DividerDemo::show_button(&t));
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
                children.push(FlexibleNavigationBarDemo::show_button(&t));
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
    fn inspect_contrast(contrast: DisplayContrast, theme: &Theme) -> serde_json::Value {
        let pair = |colors: SystemColorPair| {
            serde_json::json!({
                "background": colors.background,
                "foreground": colors.foreground,
            })
        };
        let (mode, system_colors) = match contrast {
            DisplayContrast::Normal => ("normal", None),
            DisplayContrast::Increased => ("increased", None),
            DisplayContrast::Forced(colors) => (
                "forced",
                Some(serde_json::json!({
                    "canvas": pair(colors.canvas),
                    "button": pair(colors.button),
                    "highlight": pair(colors.highlight),
                })),
            ),
            _ => ("unsupported", None),
        };
        let colors = theme.colors;
        serde_json::json!({
            "mode": mode,
            "system_colors": system_colors,
            "theme": {
                "is_dark": theme.is_dark,
                "surface": colors.surface,
                "on_surface": colors.on_surface,
                "surface_container": colors.surface_container,
                "surface_container_lowest": colors.surface_container_lowest,
                "outline": colors.outline,
                "primary": colors.primary,
                "on_primary": colors.on_primary,
                "focus": colors.secondary,
                "error": colors.error,
                "tonal": theme.tonal,
                "on_tonal": theme.on_tonal,
            },
        })
    }

    #[wasm_bindgen]
    pub fn inspect() -> String {
        APP.with(|a|{
        let a=a.borrow();let runtime=a.as_ref().expect("started").borrow();let s=runtime.app();
        let theme = s.effective_theme();
        let time_input = s.time_input_demo.inspect();
        let time_dial = s.time_dial_demo.inspect();
        let baseline_time_colors = [&time_input, &time_dial].into_iter().any(|demo| demo["variant"] == "Baseline").then(|| {
            let colors = theme.colors;
            serde_json::json!({
                "surface_container_high": colors.surface_container_high,
                "surface_container_highest": colors.surface_container_highest,
                "primary_container": colors.primary_container,
                "primary": colors.primary,
                "tertiary_container": colors.tertiary_container,
                "outline": colors.outline,
            })
        });
        let mut state = serde_json::json!({"section":s.section,"dark":s.dark,"radio":s.radio,"enabled":s.enabled,"check":format!("{:?}",s.check),"segment":s.segment,
            "volume":s.volume,"clicks":s.clicks,"filter":s.filter,"input_chip":s.input_chip,"dialog":s.dialog,"menu":s.menu,"name":s.name,"draft":s.draft,"nav":s.nav,"focused":runtime.focused(),
            "wide_navigation_rail": s.wide_rail_demo.inspect(s.text_scale()),
            "modal_wide_navigation_rail": s.modal_wide_rail_demo.inspect(s.text_scale(), &runtime),
            "snackbar": s.snackbar_demo.inspect(s.text_scale()),
            "badges": s.badge_demo.inspect(s.text_scale(), &theme),
            "navigation_drawer": s.drawer_demo.inspect(s.text_scale(), &runtime),
            "progress": s.progress_demo.inspect(&theme),
            "side_sheet": s.side_sheet_demo.inspect(&theme),
            "bottom_sheet": s.bottom_sheet_demo.inspect(),
            "cards": s.card_demo.inspect(),
            "button_group": s.button_group_demo.inspect(),
            "split_button": s.split_demo.inspect(),
            "extended_fab": s.extended_fab_demo.inspect(),
            "expressive_fab": s.fab_demo.inspect(),
            "fab_menu": s.fab_menu_demo.inspect(),
            "date_picker": s.date_picker_demo.inspect(),
            "docked_date_picker": s.docked_date_picker_demo.inspect(),
            "date_range_picker": s.date_range_picker_demo.inspect(),
            "time_input": time_input,
            "time_dial": time_dial,
            "baseline_time_colors": baseline_time_colors,
            "time_scroll": s.time_scroll_demo.inspect(),
            "carousel": s.carousel_demo.inspect(&runtime),
            "app_bar": s.app_bar_demo.inspect(&theme),
            "toolbar": s.toolbar_demo.inspect(&theme),
            "tooltip": s.tooltip_demo.inspect(&theme)
        });
        state["display_contrast"] = inspect_contrast(runtime.display_contrast(), &theme);
        state["lists"] = s.lists_demo.inspect(&runtime, &theme);
        state["loading"] = s.loading_demo.inspect(&theme);
        state["scroll_field"] = s.scroll_field_demo.inspect(&runtime, &theme);
        state["scroll_choice"] = s.scroll_choice_demo.inspect(&runtime, &theme);
        state["scroll_pair"] = s.scroll_pair_demo.inspect(&runtime);
        state["scroll_popup"] = s.scroll_popup_demo.inspect(&runtime);
        state["scroll_modal"] = s.scroll_modal_demo.inspect(&runtime);
        state["search_overlay"] = s.search_overlay_demo.inspect();
        state["dialog_demo"] = s.dialog_demo.inspect();
        state["menu_demo"] = s.menu_demo.inspect();
        state["navigation_bar"] = s.flexible_navigation_bar_demo.inspect(&theme);
        state["switch_demo"] = s.switch_demo.inspect(&theme);
        state["list_actions"] = s.list_actions_demo.inspect(&runtime, &theme);
        state["list_expand"] = s.list_expand_demo.inspect(&theme);
        state["list_reorder"] = s.list_reorder_demo.inspect(&runtime, &theme);
        state["list_media"] = s.list_media_demo.inspect(&runtime, &theme);
        state.to_string()
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

#[cfg(test)]
mod display_contrast_tests {
    use super::*;
    use surface_app::SystemContrastColors;

    #[test]
    fn forced_palette_uses_system_pairs_and_preserves_the_authored_theme() {
        let mut gallery = Gallery::default();
        let normal = gallery.effective_theme();
        let canvas = SystemColorPair {
            background: Color::rgb(0, 0, 0),
            foreground: Color::rgb(255, 245, 240),
        };
        let button = SystemColorPair {
            background: Color::rgb(18, 34, 45),
            foreground: Color::rgb(255, 221, 171),
        };
        let highlight = SystemColorPair {
            background: Color(0, 230, 255, 204),
            foreground: Color::rgb(0, 0, 0),
        };
        let preference =
            DisplayContrast::Forced(SystemContrastColors::new(canvas, button, highlight));
        assert!(gallery.display_contrast_changed(preference));
        let forced = gallery.effective_theme();
        assert!(forced.is_dark);
        assert!(!gallery.dark);
        assert_eq!(forced.background, canvas.background);
        assert_eq!(forced.on_background, canvas.foreground);
        assert_eq!(forced.tonal, button.background);
        assert_eq!(forced.on_tonal, button.foreground);
        assert_eq!(forced.colors.secondary, canvas.foreground);
        assert!(!gallery.display_contrast_changed(preference));

        assert!(gallery.display_contrast_changed(DisplayContrast::Normal));
        let restored = gallery.effective_theme();
        assert_eq!(restored.background, normal.background);
        assert_eq!(restored.primary, normal.primary);
        assert_eq!(restored.outline, normal.outline);
        assert!(!restored.is_dark);
    }

    #[test]
    fn increased_uses_the_authored_dark_variant() {
        let mut gallery = Gallery {
            dark: true,
            ..Gallery::default()
        };
        assert!(gallery.display_contrast_changed(DisplayContrast::Increased));
        let increased = gallery.effective_theme();
        assert!(increased.is_dark);
        assert_ne!(increased.outline, Theme::dark().outline);
        assert!(gallery.display_contrast_changed(DisplayContrast::Normal));
        assert_eq!(gallery.effective_theme().outline, Theme::dark().outline);
    }

    #[test]
    fn ambiguous_forced_canvas_retains_the_authored_polarity() {
        let ink = Color::rgb(110, 122, 130);
        let equal = SystemColorPair {
            background: ink,
            foreground: ink,
        };
        let translucent = SystemColorPair {
            background: Color(255, 255, 255, 128),
            foreground: Color::rgb(0, 0, 0),
        };
        assert!(!forced_dark(equal, false));
        assert!(forced_dark(equal, true));
        assert!(forced_dark(translucent, true));
    }
}
