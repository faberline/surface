use std::sync::OnceLock;

use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    adaptive::WindowWidthSizeClass, shape::LayoutDirection, typography, ButtonKind, ChoiceState,
    FlexibleNavigationBarOptions, NavigationRailItem, Theme, VectorIcon,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 3;
const SHOW: &str = "navigation-bar-demo-show";
const SHOW_MEDIUM: &str = "navigation-bar-medium-demo-show";
const BACK: &str = "navigation-bar-demo-back";
const PREVIEW: &str = "navigation-bar-demo-preview";
const SCROLL: &str = "navigation-bar-demo-scroll";
const BAR: &str = "navigation-bar-demo-bar";
const PAGE: &str = "navigation-bar-demo-page";
const PREVIEW_WIDTH: f32 = 480.;
const MEDIUM_PREVIEW_WIDTH: f32 = 839.;
const DISABLED: usize = 1;
const IDS: [&str; 5] = [
    "navigation-bar-demo-home",
    "navigation-bar-demo-explore",
    "navigation-bar-demo-saved",
    "navigation-bar-demo-updates",
    "navigation-bar-demo-settings",
];
const LABELS: [&str; 5] = ["首頁", "探索路線", "收藏", "最新消息", "設定"];
const NAMES: [&str; 5] = ["首頁", "探索路線", "收藏的地點", "最新消息", "設定"];
const CONTENT: [[&str; 3]; 5] = [
    ["今日行程", "河畔散步", "閱讀清單"],
    ["城市散步", "山徑", "河岸路線"],
    ["河岸步道", "城市圖書館", "公園地圖"],
    ["步道已開放", "新展覽", "週末活動"],
    ["通知", "語言", "隱私"],
];

#[derive(Clone)]
pub enum FlexibleNavigationBarDemoEvent {
    Show,
    ShowMedium,
    Back,
    ToggleDirection,
    ToggleLargeText,
    SetCount(usize),
    Choose(usize),
}

pub(crate) struct FlexibleNavigationBarDemo {
    visible: bool,
    medium_preview: bool,
    viewport_width: f32,
    rtl: bool,
    large_text: bool,
    count: usize,
    selected: usize,
    activations: usize,
}

impl Default for FlexibleNavigationBarDemo {
    fn default() -> Self {
        Self {
            visible: false,
            medium_preview: false,
            viewport_width: PREVIEW_WIDTH,
            rtl: false,
            large_text: false,
            count: IDS.len(),
            selected: 0,
            activations: 0,
        }
    }
}

impl FlexibleNavigationBarDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
    }

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        let previous = self.preview_width();
        self.viewport_width = width;
        self.visible && previous != self.preview_width()
    }

    fn preview_width(&self) -> f32 {
        self.viewport_width.min(if self.medium_preview {
            MEDIUM_PREVIEW_WIDTH
        } else {
            PREVIEW_WIDTH
        })
    }

    fn horizontal(&self) -> bool {
        self.medium_preview
            && WindowWidthSizeClass::from_dp(self.preview_width())
                == Some(WindowWidthSizeClass::Medium)
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn update(&mut self, event: FlexibleNavigationBarDemoEvent) {
        use FlexibleNavigationBarDemoEvent::*;
        match event {
            Show | ShowMedium => {
                self.visible = true;
                self.medium_preview = matches!(event, ShowMedium);
            }
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            SetCount(count @ 3..=5) => {
                self.count = count;
                if self.selected >= count {
                    self.selected = 0;
                }
            }
            Choose(index) if index < self.count && index != DISABLED => {
                self.selected = index;
                self.activations += 1;
            }
            SetCount(_) | Choose(_) => {}
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        Self::launcher(
            theme,
            SHOW,
            "彈性導覽列",
            FlexibleNavigationBarDemoEvent::Show,
        )
    }

    pub(crate) fn show_medium_button(theme: &Theme) -> View<Message> {
        Self::launcher(
            theme,
            SHOW_MEDIUM,
            "橫向導覽列",
            FlexibleNavigationBarDemoEvent::ShowMedium,
        )
    }

    fn launcher(
        theme: &Theme,
        id: &str,
        label: &str,
        event: FlexibleNavigationBarDemoEvent,
    ) -> View<Message> {
        theme
            .button_with(
                id,
                label,
                ButtonKind::Outlined,
                false,
                Message::FlexibleNavigationBarDemo(event),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use FlexibleNavigationBarDemoEvent::*;
        let event = Message::FlexibleNavigationBarDemo;
        let selected_label = LABELS[self.selected];
        let preview_width = self.preview_width();
        let launcher = if self.medium_preview {
            SHOW_MEDIUM
        } else {
            SHOW
        };
        let header = Node::new(Role::Group, "")
            .id("navigation-bar-demo-header")
            .styled(
                Style::row()
                    .width(Length::Percent(1.))
                    .align(Align::Center)
                    .padding(16.)
                    .gap(12.),
            );
        let header = if self.rtl {
            header.reverse_row()
        } else {
            header
        };
        let header = View::new(header).children([
            theme
                .tonal_button(BACK, "返回", event(Back))
                .restore_focus_on_removal(launcher)
                .accessible_activation(),
            View::new(
                Node::new(Role::Heading, selected_label)
                    .id("navigation-bar-demo-page-title")
                    .text(selected_label)
                    .shrink_to_fit_width()
                    .styled(
                        Style::default()
                            .with_text_style(typography::TITLE_MEDIUM)
                            .foreground(theme.on_background),
                    ),
            ),
        ]);
        let toggle = |id, label, selected, message| {
            theme
                .segmented_button(id, label, choice(selected), message)
                .accessible_activation()
        };
        let controls = control_row(vec![
            toggle("navigation-bar-demo-theme", "深色", dark, Message::Theme),
            toggle(
                "navigation-bar-demo-rtl",
                "右起",
                self.rtl,
                event(ToggleDirection),
            ),
            toggle(
                "navigation-bar-demo-large-text",
                "大字",
                self.large_text,
                event(ToggleLargeText),
            ),
        ]);
        let counts = control_row(
            [
                ("navigation-bar-demo-count-3", "3 個", 3),
                ("navigation-bar-demo-count-4", "4 個", 4),
                ("navigation-bar-demo-count-5", "5 個", 5),
            ]
            .into_iter()
            .map(|(id, label, count)| {
                toggle(id, label, count == self.count, event(SetCount(count)))
            })
            .collect(),
        );
        let page =
            View::new(
                Node::new(Role::Group, "")
                    .id(PAGE)
                    .styled(Style::column().gap(12.)),
            )
            .children(CONTENT[self.selected].into_iter().enumerate().map(
                |(index, label)| theme.text(&format!("navigation-bar-demo-entry-{index}"), label),
            ));
        let scroll = View::new(
            Node::new(Role::Group, "頁面內容").id(SCROLL).styled(
                Style::column()
                    .width(Length::Percent(1.))
                    .height(Length::Px(0.))
                    .grow(1.)
                    .padding(16.)
                    .gap(16.)
                    .clip(true),
            ),
        )
        .accessibility_scroll_pages()
        .children([controls, counts, page]);
        let options = FlexibleNavigationBarOptions {
            id: BAR,
            name: "主要導覽",
            available_width: preview_width,
            direction: if self.rtl {
                LayoutDirection::RightToLeft
            } else {
                LayoutDirection::LeftToRight
            },
        };
        let destinations = (0..self.count).map(|index| NavigationRailItem {
            id: IDS[index],
            label: LABELS[index],
            accessible_name: NAMES[index],
            icon: icon(index, self.selected == index),
            state: ChoiceState {
                selected: self.selected == index,
                disabled: index == DISABLED,
            },
            message: event(Choose(index)),
        });
        let bar = if self.horizontal() {
            theme.horizontal_flexible_navigation_bar(options, destinations)
        } else {
            theme.flexible_navigation_bar(options, destinations)
        };
        let preview = View::new(
            Node::new(Role::Group, "").id(PREVIEW).styled(
                Style::column()
                    .width(Length::Px(preview_width))
                    .height(Length::Percent(1.))
                    .background(theme.background),
            ),
        )
        .children([header, scroll].into_iter().chain(bar));
        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("navigation-bar-demo")
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.))
                        .align(Align::Center)
                        .background(theme.colors.surface_container_low)
                        .foreground(theme.on_background)
                        .clip(true),
                ),
        )
        .children([preview])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "preview_width": self.preview_width(),
            "horizontal": self.horizontal(),
            "count": self.count,
            "destinations": &IDS[..self.count],
            "disabled": IDS[DISABLED],
            "selected": IDS[self.selected],
            "page": LABELS[self.selected],
            "activations": self.activations,
            "palette": {
                "surface_container": theme.colors.surface_container,
                "secondary_container": theme.colors.secondary_container,
                "on_secondary_container": theme.colors.on_secondary_container,
                "secondary": theme.colors.secondary,
                "on_surface_variant": theme.colors.on_surface_variant,
            },
        })
    }
}

fn choice(selected: bool) -> ChoiceState {
    ChoiceState {
        selected,
        disabled: false,
    }
}

pub(crate) fn icon(index: usize, selected: bool) -> &'static VectorIcon {
    static ICONS: OnceLock<[[VectorIcon; 2]; 5]> = OnceLock::new();
    let icons = ICONS.get_or_init(|| {
        [
            icon_pair([[4., 10.], [12., 2.], [20., 10.], [20., 21.], [4., 21.]]),
            icon_pair([[12., 2.], [22., 12.], [12., 22.], [2., 12.]]),
            icon_pair([[6., 3.], [18., 3.], [18., 21.], [12., 17.], [6., 21.]]),
            icon_pair([[8., 5.], [12., 2.], [16., 5.], [20., 19.], [4., 19.]]),
            icon_pair([
                [8., 3.],
                [16., 3.],
                [21., 8.],
                [21., 16.],
                [16., 21.],
                [8., 21.],
                [3., 16.],
                [3., 8.],
            ]),
        ]
    });
    &icons[index][usize::from(selected)]
}

fn icon_pair<const N: usize>(outer: [[f32; 2]; N]) -> [VectorIcon; 2] {
    let inner = outer.map(|point| point.map(|axis| 12. + (axis - 12.) * 0.72));
    [
        VectorIcon::from_contours([outer, inner]).expect("valid outlined destination icon"),
        VectorIcon::new(outer).expect("valid filled destination icon"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallery;
    use surface::{Rect, TextMeasurer};
    use surface_app::{Event, Runtime};

    struct Fonts;

    impl TextMeasurer for Fonts {
        fn measure(&mut self, text: &str, size: f32, max_width: Option<f32>) -> (f32, f32) {
            let width = text.chars().count() as f32 * size;
            let available = max_width.unwrap_or(width).max(1.);
            (
                width.min(available),
                size * 1.33 * (width / available).ceil().max(1.),
            )
        }
    }

    fn render(runtime: &mut Runtime<Gallery>, width: f32, height: f32) {
        runtime.render(width, height, &mut Fonts).unwrap();
    }

    fn route() -> Runtime<Gallery> {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        render(&mut runtime, 960., 800.);
        runtime.apply(Message::Section(SECTION)).unwrap();
        render(&mut runtime, 960., 800.);
        runtime.activate(SHOW).unwrap();
        render(&mut runtime, 960., 800.);
        runtime
    }

    fn bounds(runtime: &Runtime<Gallery>, id: &str) -> Rect {
        runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
    }

    #[test]
    fn tab_visits_enabled_pages_without_changing_selection() {
        let mut runtime = route();
        assert_eq!(runtime.focused(), Some(BACK));
        assert_eq!(bounds(&runtime, BAR), Rect::new(240., 736., 480., 64.));
        for target in [
            "navigation-bar-demo-theme",
            "navigation-bar-demo-rtl",
            "navigation-bar-demo-large-text",
            "navigation-bar-demo-count-3",
            "navigation-bar-demo-count-4",
            "navigation-bar-demo-count-5",
            IDS[0],
            IDS[2],
            IDS[3],
            IDS[4],
        ] {
            runtime
                .dispatch(Event::FocusNext { reverse: false })
                .unwrap();
            render(&mut runtime, 960., 800.);
            assert_eq!(runtime.focused(), Some(target));
            assert_eq!(runtime.app().flexible_navigation_bar_demo.selected, 0);
        }
        runtime.dispatch(Event::Activate).unwrap();
        render(&mut runtime, 960., 800.);
        assert_eq!(runtime.app().flexible_navigation_bar_demo.selected, 4);
        assert_eq!(runtime.app().flexible_navigation_bar_demo.activations, 1);
        assert_eq!(runtime.focused(), Some(IDS[4]));
        runtime.activate("navigation-bar-demo-count-4").unwrap();
        render(&mut runtime, 960., 800.);
        assert!(runtime.dom().by_id(IDS[4]).is_none());
        assert_eq!(runtime.app().flexible_navigation_bar_demo.selected, 0);
        runtime.activate("navigation-bar-demo-count-3").unwrap();
        render(&mut runtime, 960., 800.);
        assert!(runtime.dom().by_id(IDS[3]).is_none());
        runtime.activate(BACK).unwrap();
        render(&mut runtime, 960., 800.);
        assert_eq!(runtime.focused(), Some(SHOW));
    }

    #[test]
    fn large_compact_rtl_bar_keeps_its_place_while_the_page_scrolls() {
        let mut runtime = route();
        runtime.activate("navigation-bar-demo-theme").unwrap();
        runtime.activate("navigation-bar-demo-rtl").unwrap();
        runtime.activate("navigation-bar-demo-large-text").unwrap();
        render(&mut runtime, 320., 400.);

        let bar = bounds(&runtime, BAR);
        let header = bounds(&runtime, "navigation-bar-demo-header");
        for id in [BACK, "navigation-bar-demo-page-title"] {
            let control = bounds(&runtime, id);
            assert_eq!(header.intersect(control), control);
        }
        assert_eq!(bar.width, 320.);
        assert_eq!(bar.y + bar.height, 400.);
        assert!(bar.height > 80.);
        for (index, id) in IDS.iter().enumerate() {
            let item = bounds(&runtime, id);
            assert_eq!(item.x, (IDS.len() - index - 1) as f32 * 64.);
            assert_eq!(item.width, 64.);
            assert_eq!(bar.intersect(item), item);
            let label = bounds(&runtime, &format!("{id}-label"));
            assert_eq!(item.intersect(label), label);
        }

        let page_before = bounds(&runtime, PAGE);
        let scroll = bounds(&runtime, SCROLL);
        assert_eq!(scroll.y + scroll.height, bar.y);
        runtime
            .dispatch(Event::Scroll {
                x: scroll.width / 2.,
                y: scroll.height / 2.,
                delta_x: 0.,
                delta_y: 500.,
            })
            .unwrap();
        render(&mut runtime, 320., 400.);
        assert!(bounds(&runtime, PAGE).y < page_before.y);
        assert_eq!(bounds(&runtime, "navigation-bar-demo-header"), header);
        assert_eq!(bounds(&runtime, BAR), bar);
    }
}
