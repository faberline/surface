//! A small Material 3-inspired component set. Values remain renderer-independent.
use surface::{Align, Color, Length, Node, Role, Style};
use surface_app::{CursorStyle, View};
pub mod adaptive;
mod app_bar;
mod app_bar_scroll;
mod app_bar_search;
mod badge;
mod baseline_segmented_button;
mod basic_dialog;
mod bottom_sheet;
mod bottom_sheet_motion;
mod button;
mod button_group;
mod button_group_layout;
mod button_group_motion;
mod card;
mod carousel;
mod carousel_layout;
mod carousel_state;
mod checkbox;
mod chip;
mod chip_group;
mod command_menu;
mod components;
mod date_picker;
mod date_picker_state;
mod date_range_picker;
mod date_range_picker_state;
mod divider;
mod docked_date_picker;
mod docked_date_picker_state;
mod elevation;
mod extended_fab;
mod extended_fab_motion;
mod fab;
mod fab_menu;
mod fab_menu_layout;
mod fab_menu_motion;
mod fab_motion;
mod flexible_navigation_bar;
mod full_screen_dialog;
mod icon_button;
mod list;
mod loading_indicator;
mod menu_layout;
pub mod motion;
mod navigation_drawer;
mod outlined_text_field;
mod progress_indicator;
mod radio;
mod scroll_choice_catalog;
mod scroll_choice_field_state;
mod scroll_field;
mod scroll_field_state;
mod search_layout;
mod search_motion;
mod search_overlay;
pub mod shape;
mod side_sheet;
mod snackbar;
mod snackbar_state;
mod split_button;
mod split_button_layout;
mod split_button_motion;
mod standard_navigation_drawer;
mod standard_slider;
mod switch;
mod tab_bar;
mod time_dial;
mod time_dial_state;
mod time_input;
mod time_input_state;
mod time_scroll;
mod time_scroll_state;
mod tokens;
mod toolbar;
mod toolbar_scroll;
mod tooltip;
mod tooltip_state;
pub mod typography;
mod vector_icon;
mod wide_navigation_rail;
pub use app_bar::{AppBarContent, AppBarKind, AppBarOptions};
pub use app_bar_scroll::{AppBarScroll, AppBarScrollBehavior, AppBarScrollUpdate};
pub use app_bar_search::{AppBarSearchContent, AppBarSearchOptions};
pub use badge::{BadgeBoundary, BadgeContent, BadgedAnchorOptions};
pub use baseline_segmented_button::{
    BaselineSegmentedButtonOptions, BaselineSegmentedItem, BaselineSegmentedSelection,
};
pub use basic_dialog::{BasicDialogContent, BasicDialogMotion, BasicDialogOptions};
pub use bottom_sheet::{
    BottomSheetContent, BottomSheetHeights, BottomSheetOptions, BottomSheetPresentation,
    BottomSheetState,
};
pub use bottom_sheet_motion::BottomSheetMotion;
pub use button::{ButtonAppearance, ButtonIcon, ButtonOptions, ButtonShape, ButtonSize};
pub use button_group::{
    ButtonGroupAppearance, ButtonGroupItem, ButtonGroupKind, ButtonGroupOptions,
    ButtonGroupOverflow, ButtonGroupSelection, ButtonGroupShape, ButtonGroupSize, ButtonGroupWidth,
};
pub use button_group_motion::ButtonGroupMotion;
pub use card::{CardAction, CardContent, CardInteraction, MaterialCardOptions};
pub use carousel::{CarouselItem, CarouselKind, CarouselOptions, CarouselOrientation};
pub use carousel_layout::{CarouselLayout, CarouselSlot};
pub use carousel_state::{CarouselInput, CarouselState};
pub use checkbox::{CheckboxMotion, CheckboxOptions, CheckboxPose};
pub use chip::{ChipContent, ChipLeading, ChipOptions, ChipRemove, ChipStyle, ChipTrailing};
pub use chip_group::ChipGridLayout;
pub use command_menu::{
    CommandMenuEntry, CommandMenuItem, CommandMenuMeasurement, CommandMenuOptions,
};
pub use components::{
    BadgedNavigationBarItem, ButtonKind, CardKind, CheckValue, ChipKind, ChoiceState, DividerKind,
    FabColor, FabSize, IconButtonKind, LoadingKind, NavigationBarIcon, NavigationRailItem,
    SegmentPosition,
};
pub use date_picker::{
    DatePickerLabels, DatePickerLayout, DatePickerOptions, DatePickerSwipe, DatePickerSwipeInput,
};
pub use date_picker_state::{
    CivilDate, DateBounds, DateInputError, DateInputFormat, DateInputOrder, DateInputSeparator,
    DatePickerConstraints, DatePickerEffect, DatePickerEvent, DatePickerLocale, DatePickerMode,
    DatePickerSession, DatePickerStep, StepDirection, Weekday, YearMonth,
};
pub use date_range_picker::{
    DateRangePickerInput, DateRangePickerLabels, DateRangePickerOptions, DateRangePickerSession,
};
pub use date_range_picker_state::{
    CivilDateRange, DateRangeConstraints, DateRangeEffect, DateRangeEndpoint, DateRangeEvent,
    DateRangeMode, DateRangePairError, DateRangePickerState, DateRangePosition,
    DateRangeValidation,
};
pub use divider::{DividerLayout, DividerWeight};
pub use docked_date_picker::{DockedDatePickerLabels, DockedDatePickerOptions};
pub use docked_date_picker_state::{
    place_docked_date_picker, DockedDatePickerEvent, DockedDatePickerMode, DockedDatePickerState,
};
pub use elevation::Elevation;
pub use extended_fab::{
    ExtendedFabAppearance, ExtendedFabContent, ExtendedFabElevation, ExtendedFabOptions,
    ExtendedFabSize,
};
pub use extended_fab_motion::{ExtendedFabMotion, ExtendedFabScroll};
pub use fab::{
    ExpressiveFabAppearance, ExpressiveFabContent, ExpressiveFabElevation, ExpressiveFabOptions,
    ExpressiveFabSize,
};
pub use fab_menu::{
    FabMenuAppearance, FabMenuContent, FabMenuItem, FabMenuOptions, FabMenuPresentation,
};
pub use fab_menu_layout::{
    fab_menu_edge_margin, place_fab_menu_floating, place_fab_menu_trigger, place_fab_menu_web,
    FAB_MENU_LIST_INSET,
};
pub use fab_menu_motion::{FabMenuItemMotion, FabMenuMotion, FabMenuMotionTarget};
pub use fab_motion::{ExpressiveFabMotion, ExpressiveFabRipple};
pub use flexible_navigation_bar::FlexibleNavigationBarOptions;
pub use full_screen_dialog::{
    full_screen_dialog_layout, FullScreenDialogAnchor, FullScreenDialogLayout,
    FullScreenDialogMotion, FullScreenDialogOptions, FullScreenDialogTransition,
};
pub use icon_button::{
    IconButtonOptions, IconButtonShape, IconButtonSize, IconButtonWidth, MaterialIcon,
};
pub use list::{
    ExpandableListOptions, ExpandableListRow, ListExpansionMotion, ListItem, ListLines, ListMedia,
    ListMediaKind, ListMode, ListOptions, ListReorderEntry, ListReorderInput, ListReorderRequest,
    ListReorderState, ListStyle, ListSupplementaryAction, MediaListItem, MediaListMode,
    MediaListOptions, MultiActionListItem, MultiActionListOptions, MultiSelectListOptions,
    ReorderableListItem, ReorderableListOptions,
};
pub use loading_indicator::{
    LoadingIndicatorMotion, LoadingIndicatorOptions, LoadingIndicatorPose,
};
pub use menu_layout::{place_menu, MenuAnchor, MenuLayout, MenuPlacement};
pub use navigation_drawer::{
    ModalNavigationDrawerOptions, NavigationDrawerItem, NavigationDrawerSwipe,
};
pub use outlined_text_field::OutlinedTextFieldOptions as TextFieldOptions;
pub use outlined_text_field::{OutlinedTextFieldOptions, TextFieldEditability};
pub use progress_indicator::{ProgressIndicatorOptions, ProgressShape, ProgressState};
pub use radio::{RadioGroupLayout, RadioGroupOptions, RadioMotion, RadioOptions, RadioPose};
pub use scroll_choice_catalog::{
    ScrollChoice, ScrollChoiceCatalog, ScrollChoiceCatalogError, ScrollChoiceItem,
    ScrollChoiceMatch, ScrollChoiceMatchKind, ScrollChoiceQueryError,
};
pub use scroll_choice_field_state::{ScrollChoiceEvent, ScrollChoiceState};
pub use scroll_field::{
    scroll_choice_field_layout, scroll_field_layout, scroll_field_modal_layout,
    scroll_field_pair_layout, scroll_field_popup_layout, ScrollChoiceFieldOptions,
    ScrollFieldLayout, ScrollFieldModalLayout, ScrollFieldModalOptions, ScrollFieldOptions,
    ScrollFieldPairLayout, ScrollFieldPairOptions, ScrollFieldPopupLayout, ScrollFieldPopupOptions,
    ScrollFieldSize,
};
pub use scroll_field_state::{
    ScrollFieldDrag, ScrollFieldError, ScrollFieldEvent, ScrollFieldMode, ScrollFieldRange,
    ScrollFieldState, ScrollFieldStep,
};
pub use search_layout::SearchStyle;
pub use search_motion::{SearchOverlayMotion, SearchOverlayMotionTarget};
pub use search_overlay::{SearchOverlayContent, SearchOverlayOptions};
pub use side_sheet::{
    SideSheetContent, SideSheetEdge, SideSheetMotion, SideSheetOptions, SideSheetPresentation,
    SideSheetStyle,
};
pub use snackbar::SnackbarOptions;
pub use snackbar_state::{SnackbarDuration, SnackbarId, SnackbarQueue};
pub use split_button::{
    SplitButtonAppearance, SplitButtonContent, SplitButtonLeading, SplitButtonMenu,
    SplitButtonMenuItem, SplitButtonOptions, SplitButtonSize,
};
pub use split_button_layout::{place_split_button_popup, SplitButtonAnchor};
pub use split_button_motion::SplitButtonMotion;
pub use standard_navigation_drawer::{
    StandardNavigationDrawerMode, StandardNavigationDrawerOptions,
};
pub use standard_slider::VerticalStandardSliderPlacement;
pub use standard_slider::{
    RangeSliderChange, RangeSliderOptions, RangeSliderThumb, SliderInsetIcon, StandardSliderMode,
    StandardSliderOptions, StandardSliderSize,
};
pub use switch::{SwitchIcons, SwitchMotion, SwitchOptions, SwitchPose};
pub use tab_bar::{
    TabActivation, TabBarInput, TabBarKind, TabBarLayout, TabBarMotion, TabBarOptions, TabItem,
    TabPanelEntry,
};
pub use time_dial::{
    time_dial_baseline_layout, time_dial_layout, DialHandMotion, TimeDialLabels, TimeDialLayout,
    TimeDialOptions,
};
pub use time_dial_state::{DialEvent, DialMode, DialPoint, DialPreview, DialState};
pub use time_input::{TimeInputLabels, TimeInputOptions, TimeInputPresentation};
pub use time_input_state::{
    ClockFormat, PeriodPosition, TimeField, TimeInputEffect, TimeInputError, TimeInputEvent,
    TimeInputLocale, TimeInputSession, TimeInputValidation, TimeOfDay, TimePeriod,
};
pub use time_scroll::{time_scroll_layout, TimeScrollLabels, TimeScrollLayout, TimeScrollOptions};
pub use time_scroll_state::{
    TimeScrollColumn, TimeScrollDrag, TimeScrollEvent, TimeScrollMode, TimeScrollState,
    TimeScrollStep,
};
pub use tokens::ColorScheme;
pub use toolbar::{
    ToolbarButtonOptions, ToolbarContent, ToolbarKind, ToolbarOptions, ToolbarPalette,
    ToolbarVisibility,
};
pub use toolbar_scroll::{ToolbarScroll, ToolbarScrollBehavior};
pub use tooltip::{place_tooltip, TooltipKind, TooltipOptions, TooltipPlacement, TooltipSide};
pub use tooltip_state::{TooltipInput, TooltipState, TooltipUpdate};
pub use vector_icon::VectorIcon;
pub use wide_navigation_rail::{
    ModalWideNavigationRailCollapse, ModalWideNavigationRailOptions,
    ModalWideNavigationRailSwipeOptions, WideNavigationRailMode, WideNavigationRailMotion,
    WideNavigationRailMotionFrame, WideNavigationRailMotionKind, WideNavigationRailSwipe,
};

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub colors: ColorScheme,
    pub is_dark: bool,
    pub primary: Color,
    pub on_primary: Color,
    pub background: Color,
    pub on_background: Color,
    pub container: Color,
    pub on_container: Color,
    pub outline: Color,
    pub disabled: Color,
    pub tonal: Color,
    pub on_tonal: Color,
    pub input_container: Color,
}
impl Theme {
    pub fn light() -> Self {
        static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
        *THEME.get_or_init(|| Self::from_seed(Color::rgb(103, 80, 164), false))
    }
    pub fn dark() -> Self {
        static THEME: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
        *THEME.get_or_init(|| Self::from_seed(Color::rgb(103, 80, 164), true))
    }
    pub fn from_seed(seed: Color, dark: bool) -> Self {
        Self::from_color_scheme(ColorScheme::from_seed(seed, dark), dark)
    }

    /// Generate a Tonal Spot theme at the highest Material contrast level.
    ///
    /// Keep the authored seed and light or dark appearance. Use a system
    /// palette instead when the host provides explicit forced colors.
    pub fn from_seed_high_contrast(seed: Color, dark: bool) -> Self {
        Self::from_color_scheme(ColorScheme::high_contrast(seed, dark), dark)
    }

    /// Use resolved system color pairs for a Canvas-backed theme.
    ///
    /// Each pair is `(background, foreground)`. Pass the host's effective
    /// light or dark appearance. Canvas and Button pairs stay separate.
    /// Components still need an explicit edge when their surfaces coincide.
    /// Focus, error and disabled states keep their own semantics. This theme
    /// does not change platform text selection paint.
    pub fn from_forced_colors(canvas: (Color, Color), button: (Color, Color), dark: bool) -> Self {
        Self::from_color_scheme(ColorScheme::forced_colors(canvas, button), dark)
    }

    fn from_color_scheme(colors: ColorScheme, dark: bool) -> Self {
        Self {
            colors,
            is_dark: dark,
            primary: colors.primary,
            on_primary: colors.on_primary,
            background: colors.surface,
            on_background: colors.on_surface,
            container: colors.surface_container,
            on_container: colors.on_surface,
            outline: colors.outline,
            disabled: tokens::blend(colors.surface, colors.on_surface, 0.12),
            tonal: colors.secondary_container,
            on_tonal: colors.on_secondary_container,
            input_container: colors.surface_container_highest,
        }
    }
    pub fn page<M>(&self, children: impl IntoIterator<Item = View<M>>) -> View<M> {
        View::new(
            Node::new(Role::Window, "Application")
                .id("surface-page")
                .styled(
                    Style::column()
                        .padding(28.)
                        .gap(24.)
                        .clip(true)
                        .background(self.background)
                        .foreground(self.on_background),
                ),
        )
        .children(children)
    }
    pub fn heading<M>(&self, id: &str, text: impl Into<String>) -> View<M> {
        let text = text.into();
        View::new(
            Node::new(Role::Heading, text.clone())
                .id(id)
                .text(text)
                .styled(
                    Style::default()
                        .with_text_style(typography::HEADLINE_LARGE)
                        .foreground(self.on_background),
                ),
        )
    }
    pub fn text<M>(&self, id: &str, text: impl Into<String>) -> View<M> {
        let text = text.into();
        View::new(
            Node::new(Role::Text, text.clone())
                .id(id)
                .text(text)
                .styled(
                    Style::default()
                        .with_text_style(typography::BODY_LARGE)
                        .foreground(self.on_background),
                ),
        )
    }
    pub fn card<M>(&self, id: &str, children: impl IntoIterator<Item = View<M>>) -> View<M> {
        View::new(
            Node::new(Role::Group, "Card").id(id).styled(
                Style::column()
                    .padding(24.)
                    .gap(16.)
                    .background(self.container)
                    .foreground(self.on_container)
                    .corners(shape::MEDIUM),
            ),
        )
        .children(children)
    }
    pub fn row<M>(&self, children: impl IntoIterator<Item = View<M>>) -> View<M> {
        View::new(
            Node::new(Role::Group, "Row")
                .styled(Style::row().gap(12.).align(Align::Center).wrap(true)),
        )
        .children(children)
    }
    pub fn button<M>(&self, id: &str, label: &str, message: M) -> View<M> {
        self.button_with(id, label, ButtonKind::Filled, false, message)
    }
    pub fn tonal_button<M>(&self, id: &str, label: &str, message: M) -> View<M> {
        self.button_with(id, label, ButtonKind::Tonal, false, message)
    }
    pub fn disabled_button<M>(&self, id: &str, label: &str, message: M) -> View<M> {
        self.button_with(id, label, ButtonKind::Filled, true, message)
    }
    pub fn checkbox<M>(&self, id: &str, label: &str, checked: bool, message: M) -> View<M> {
        self.checkbox_with(
            id,
            label,
            if checked {
                CheckValue::Checked
            } else {
                CheckValue::Unchecked
            },
            false,
            message,
        )
    }
    /// Controlled text field with runtime caret, selection and IME support.
    pub fn text_field<M>(
        &self,
        id: &str,
        label: &str,
        value: &str,
        on_input: impl Fn(String) -> M + 'static,
    ) -> View<M> {
        View::new(
            Node::new(Role::TextInput, label).id(id).text(value).styled(
                Style::default()
                    .with_text_style(typography::BODY_LARGE)
                    .padding(16.)
                    .height(Length::Px(56.))
                    .corners(shape::EXTRA_SMALL_TOP)
                    .background(self.input_container)
                    .foreground(self.on_container),
            ),
        )
        .on_input(on_input)
        .cursor(CursorStyle::Text)
    }
}
impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}
