use std::{collections::BTreeSet, sync::OnceLock};

use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, typography, BaselineSegmentedButtonOptions, BaselineSegmentedItem,
    BaselineSegmentedSelection, ButtonKind, ChoiceState, Theme, VectorIcon,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "segmented-demo-show";
const BACK: &str = "segmented-demo-back";
const THEME: &str = "segmented-demo-theme";
const RTL: &str = "segmented-demo-rtl";
const LARGE_TEXT: &str = "segmented-demo-large-text";
const AFTER: &str = "segmented-demo-after";
const TITLE: &str = "分段按鈕";
const PADDING: f32 = 16.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Comparison,
    View,
    Filters,
}

#[derive(Clone, Copy)]
struct Sample {
    id: &'static str,
    name: &'static str,
    label: bool,
    icon: Option<SampleIcon>,
    disabled: bool,
}

impl Sample {
    const fn text(id: &'static str, name: &'static str) -> Self {
        Self {
            id,
            name,
            label: true,
            icon: None,
            disabled: false,
        }
    }
}

const COMPARISON: [Sample; 2] = [
    Sample::text("segment-left-pane", "左側"),
    Sample::text("segment-right-pane", "右側"),
];
const VIEWS: [Sample; 3] = [
    Sample {
        icon: Some(SampleIcon::List),
        ..Sample::text("segment-list-view", "清單檢視")
    },
    Sample {
        disabled: true,
        ..Sample::text("segment-table-view", "表格檢視")
    },
    Sample {
        label: false,
        icon: Some(SampleIcon::Grid),
        ..Sample::text("segment-grid-view", "格狀檢視")
    },
];
const FILTERS: [Sample; 5] = [
    Sample::text("segment-new", "最新"),
    Sample::text("segment-saved", "已儲存"),
    Sample {
        disabled: true,
        ..Sample::text("segment-local", "離線")
    },
    Sample::text("segment-shared", "已分享"),
    Sample::text("segment-archived", "封存"),
];

impl Group {
    fn samples(self) -> &'static [Sample] {
        match self {
            Self::Comparison => &COMPARISON,
            Self::View => &VIEWS,
            Self::Filters => &FILTERS,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Comparison => "segments-comparison",
            Self::View => "segments-view",
            Self::Filters => "segments-filters",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Comparison => "對照位置",
            Self::View => "檢視方式",
            Self::Filters => "篩選條件",
        }
    }
}

#[derive(Clone)]
pub enum SegmentedDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    Choose(Group, &'static str),
    After,
}

pub(crate) struct SegmentedDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    width: f32,
    comparison: &'static str,
    view: &'static str,
    filters: BTreeSet<&'static str>,
    activations: usize,
    after: usize,
}

impl Default for SegmentedDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            width: 960.,
            comparison: COMPARISON[1].id,
            view: VIEWS[2].id,
            filters: BTreeSet::new(),
            activations: 0,
            after: 0,
        }
    }
}

impl SegmentedDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        if self.width == width {
            return false;
        }
        self.width = width;
        self.visible
    }

    pub(crate) fn update(&mut self, event: SegmentedDemoEvent) {
        use SegmentedDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            Choose(group, id)
                if self.visible && group.samples().iter().any(|s| s.id == id && !s.disabled) =>
            {
                match group {
                    Group::Comparison => self.comparison = id,
                    Group::View => self.view = id,
                    Group::Filters => {
                        if !self.filters.remove(id) {
                            self.filters.insert(id);
                        }
                    }
                }
                self.activations += 1;
            }
            Choose(..) => {}
            After => self.after += 1,
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                TITLE,
                ButtonKind::Outlined,
                false,
                Message::SegmentedDemo(SegmentedDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
            .suppress_repeated_activation()
    }

    fn row(&self, theme: &Theme, group: Group) -> View<Message> {
        let selected: Vec<_> = self.filters.iter().copied().collect();
        let selection = match group {
            Group::Comparison => BaselineSegmentedSelection::Single(self.comparison),
            Group::View => BaselineSegmentedSelection::Single(self.view),
            Group::Filters => BaselineSegmentedSelection::Multiple(&selected),
        };
        let control = theme
            .baseline_segmented_button(
                BaselineSegmentedButtonOptions {
                    id: group.id(),
                    name: group.title(),
                    available_width: (self.width - 2. * PADDING).clamp(58., 600.),
                    direction: if self.rtl {
                        LayoutDirection::RightToLeft
                    } else {
                        LayoutDirection::LeftToRight
                    },
                    text_scale: self.text_scale(),
                    selection,
                },
                group.samples().iter().map(|s| BaselineSegmentedItem {
                    id: s.id,
                    label: s.label.then_some(s.name),
                    name: Some(s.name),
                    icon: s.icon.map(SampleIcon::art),
                    disabled: s.disabled,
                    on_activate: Message::SegmentedDemo(SegmentedDemoEvent::Choose(group, s.id)),
                }),
            )
            .expect("valid Gallery segmented row");

        View::new(Node::new(Role::Group, "").styled(Style::column().gap(8.))).children([
            theme.heading(&format!("{}-title", group.id()), group.title()),
            control,
        ])
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use SegmentedDemoEvent::*;
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
                .suppress_repeated_activation()
        };
        let controls = control_row(vec![
            theme
                .tonal_button(BACK, "返回", Message::SegmentedDemo(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation()
                .suppress_repeated_activation(),
            toggle(THEME, "深色", dark, Message::Theme),
            toggle(
                RTL,
                "右起",
                self.rtl,
                Message::SegmentedDemo(ToggleDirection),
            ),
            toggle(
                LARGE_TEXT,
                "大字",
                self.large_text,
                Message::SegmentedDemo(ToggleLargeText),
            ),
        ]);
        let after = theme
            .button_with(
                AFTER,
                "完成",
                ButtonKind::Text,
                false,
                Message::SegmentedDemo(After),
            )
            .accessible_activation()
            .suppress_repeated_activation();
        let status = if self.filters.is_empty() {
            "未選取篩選條件".to_owned()
        } else {
            format!("已選取 {} 項篩選條件", self.filters.len())
        };
        let status = View::new(
            Node::new(Role::Status, "")
                .id("segmented-demo-status")
                .text(status)
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .with_text_style(typography::BODY_MEDIUM)
                        .foreground(theme.colors.on_surface),
                ),
        );

        View::new(
            Node::new(Role::Window, TITLE)
                .id("segmented-demo-route")
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.))
                        .align(Align::Start)
                        .padding(PADDING)
                        .gap(12.)
                        .background(theme.background)
                        .foreground(theme.on_background)
                        .clip(true),
                ),
        )
        .accessibility_scroll_pages()
        .children([
            theme.heading("segmented-demo-title", TITLE),
            controls,
            self.row(theme, Group::Comparison),
            self.row(theme, Group::View),
            self.row(theme, Group::Filters),
            status,
            after,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme, dark: bool) -> serde_json::Value {
        let color = |surface::Color(r, g, b, a)| [r, g, b, a];
        let c = theme.colors;
        serde_json::json!({
            "visible": self.visible, "width": self.width, "rtl": self.rtl, "dark": dark,
            "text_scale": self.text_scale(), "comparison": self.comparison, "view": self.view,
            "filters": self.filters, "activations": self.activations, "after": self.after,
            "palette": {
                "background": color(theme.background), "ink": color(c.on_surface),
                "selection": color(c.secondary_container), "selection_ink": color(c.on_secondary_container),
                "outline": color(c.outline), "focus": color(c.secondary),
            },
        })
    }
}

#[derive(Clone, Copy)]
enum SampleIcon {
    List,
    Grid,
}

impl SampleIcon {
    fn art(self) -> &'static VectorIcon {
        static LIST: OnceLock<VectorIcon> = OnceLock::new();
        static GRID: OnceLock<VectorIcon> = OnceLock::new();
        match self {
            Self::List => LIST.get_or_init(|| {
                VectorIcon::from_contours([
                    [[2., 3.], [22., 3.], [22., 9.], [2., 9.]],
                    [[2., 15.], [22., 15.], [22., 21.], [2., 21.]],
                ])
                .expect("valid list icon")
            }),
            Self::Grid => GRID.get_or_init(|| {
                VectorIcon::from_contours([
                    [[3., 3.], [10., 3.], [10., 10.], [3., 10.]],
                    [[14., 3.], [21., 3.], [21., 10.], [14., 10.]],
                    [[3., 14.], [10., 14.], [10., 21.], [3., 21.]],
                    [[14., 14.], [21., 14.], [21., 21.], [14., 21.]],
                ])
                .expect("valid grid icon")
            }),
        }
    }
}
