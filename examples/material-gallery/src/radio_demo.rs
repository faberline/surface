use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, FramePress, View};
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChoiceState, RadioGroupLayout,
    RadioGroupOptions, RadioMotion, RadioOptions, Theme,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "radio-demo-show";
const BACK: &str = "radio-demo-back";
const CLEAR: &str = "radio-demo-clear";
const GROUP: &str = "radio-demo-frequency";
const IDS: [&str; 4] = [
    "radio-demo-daily",
    "radio-demo-locked",
    "radio-demo-weekly",
    "radio-demo-monthly",
];
const LABELS: [&str; IDS.len()] = [
    "每天",
    "即時通知",
    "使用行動網路時只在每週一發送通知",
    "每月",
];
const LOCKED: usize = 1;
const WEEKLY: usize = 2;

#[derive(Clone)]
pub enum RadioDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    ToggleLayout,
    Select(usize),
    Clear,
}

pub(crate) struct RadioDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    horizontal: bool,
    selected: Option<usize>,
    requests: usize,
    motions: [RadioMotion; IDS.len()],
}

impl Default for RadioDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            horizontal: false,
            selected: Some(WEEKLY),
            requests: 0,
            motions: std::array::from_fn(|_| RadioMotion::new()),
        }
    }
}

impl RadioDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.motions = std::array::from_fn(|_| RadioMotion::new());
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn layout(&self) -> RadioGroupLayout {
        if self.horizontal {
            RadioGroupLayout::Horizontal
        } else {
            RadioGroupLayout::Vertical
        }
    }

    pub(crate) fn update(&mut self, event: RadioDemoEvent) {
        use RadioDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            ToggleLayout => self.horizontal = !self.horizontal,
            Clear => self.selected = None,
            Select(index) => {
                self.requests += 1;
                if index < IDS.len() && index != LOCKED {
                    self.selected = Some(index);
                }
            }
        }
    }

    pub(crate) fn frame(
        &mut self,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> AnimationUpdate {
        let mut result = AnimationUpdate::default();
        if self.visible {
            let direction = self.direction();
            let scale = self.text_scale();
            for (index, motion) in self.motions.iter_mut().enumerate() {
                let next = motion.advance(
                    options(index, self.selected, direction, scale),
                    frame,
                    input,
                    press,
                    hovered,
                );
                result.rebuild |= next.rebuild;
                result.request_next |= next.request_next;
            }
        }
        result
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "單選按鈕範例",
                ButtonKind::Outlined,
                false,
                Message::RadioDemo(RadioDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use RadioDemoEvent::*;
        let event = Message::RadioDemo;
        let direction = self.direction();
        let scale = self.text_scale();
        let setting = |id, label, selected, message| {
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
        };
        let controls = control_row(vec![
            theme
                .tonal_button(BACK, "返回", event(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            setting("radio-demo-theme", "深色", dark, Message::Theme),
            setting("radio-demo-rtl", "右起", self.rtl, event(ToggleDirection)),
            setting(
                "radio-demo-large-text",
                "大字",
                self.large_text,
                event(ToggleLargeText),
            ),
            setting(
                "radio-demo-layout",
                "橫向",
                self.horizontal,
                event(ToggleLayout),
            ),
        ]);
        let group = theme.radio_group_with_options(
            RadioGroupOptions {
                id: GROUP,
                name: "通知頻率",
                direction,
                layout: self.layout(),
            },
            self.motions.iter().enumerate().map(|(index, motion)| {
                theme.radio_with_options(
                    options(index, self.selected, direction, scale),
                    motion,
                    event(Select(index)),
                )
            }),
        );
        let mut column = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .padding(8.)
            .gap(8.)
            .clip(true);
        column.max_width = Length::Px(560.);
        let preview = View::new(
            Node::new(Role::Group, "通知設定")
                .id("radio-demo-scroll")
                .styled(column),
        )
        .accessibility_scroll_pages()
        .children([
            View::new(
                Node::new(Role::Heading, "通知頻率")
                    .id("radio-demo-group-title")
                    .text("通知頻率")
                    .styled(
                        Style::default()
                            .with_text_style(typography::TITLE_MEDIUM)
                            .foreground(theme.on_background),
                    ),
            ),
            group,
            theme
                .button_with(CLEAR, "清除選擇", ButtonKind::Outlined, false, event(Clear))
                .accessible_activation(),
        ]);
        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("radio-demo")
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.))
                        .align(Align::Start)
                        .padding(16.)
                        .gap(12.)
                        .background(theme.background)
                        .foreground(theme.on_background)
                        .clip(true),
                ),
        )
        .children([
            theme.heading("radio-demo-title", "單選按鈕"),
            controls,
            preview,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "horizontal": self.horizontal,
            "selected": self.selected.map(|index| IDS[index]),
            "requests": self.requests,
            "palette": {
                "surface": theme.colors.surface,
                "on_surface": theme.colors.on_surface,
                "on_surface_variant": theme.colors.on_surface_variant,
                "primary": theme.colors.primary,
                "secondary": theme.colors.secondary,
            },
        })
    }
}

fn options(
    index: usize,
    selected: Option<usize>,
    direction: LayoutDirection,
    text_scale: f32,
) -> RadioOptions<'static> {
    RadioOptions {
        id: IDS[index],
        label: LABELS[index],
        name: (index == WEEKLY).then_some("每週一通知"),
        state: ChoiceState {
            selected: selected == Some(index),
            disabled: index == LOCKED,
        },
        direction,
        text_scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallery;
    use surface::{Rect, TextMeasurer};
    use surface_app::{EditKey, Event, Runtime};

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
        render(&mut runtime, 960., 1100.);
        assert!(runtime.activate(SHOW).unwrap());
        render(&mut runtime, 960., 800.);
        runtime
    }

    fn bounds(runtime: &Runtime<Gallery>, id: &str) -> Rect {
        runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
    }

    fn tab(runtime: &mut Runtime<Gallery>, reverse: bool, id: &str) {
        runtime.dispatch(Event::FocusNext { reverse }).unwrap();
        render(runtime, 960., 800.);
        assert_eq!(runtime.focused(), Some(id));
    }

    #[test]
    fn default_selection_is_the_tab_entry_and_arrows_choose_one_option() {
        let mut runtime = route();
        assert_eq!(runtime.focused(), Some(BACK));
        for id in [
            "radio-demo-theme",
            "radio-demo-rtl",
            "radio-demo-large-text",
            "radio-demo-layout",
            IDS[WEEKLY],
        ] {
            tab(&mut runtime, false, id);
        }
        assert_eq!(runtime.app().radio_demo.requests, 0);

        for (step, (key, index)) in [
            (EditKey::Right, 3),
            (EditKey::Down, 0),
            (EditKey::Right, WEEKLY),
        ]
        .into_iter()
        .enumerate()
        {
            runtime
                .dispatch(Event::Edit {
                    key,
                    extend: false,
                    by_word: false,
                })
                .unwrap();
            render(&mut runtime, 960., 800.);
            assert_eq!(runtime.focused(), Some(IDS[index]));
            assert_eq!(runtime.app().radio_demo.selected, Some(index));
            assert_eq!(runtime.app().radio_demo.requests, step + 1);
        }
        runtime.dispatch(Event::Activate).unwrap();
        render(&mut runtime, 960., 800.);
        assert_eq!(runtime.app().radio_demo.requests, 3);

        tab(&mut runtime, false, CLEAR);
        runtime.dispatch(Event::Activate).unwrap();
        render(&mut runtime, 960., 800.);
        assert_eq!(runtime.app().radio_demo.selected, None);
        tab(&mut runtime, true, IDS[3]);
        tab(&mut runtime, true, "radio-demo-layout");
        tab(&mut runtime, false, IDS[0]);
        assert_eq!(runtime.app().radio_demo.requests, 3);
        assert_eq!(runtime.app().radio_demo.selected, None);

        assert!(runtime.activate(BACK).unwrap());
        render(&mut runtime, 960., 1100.);
        assert_eq!(runtime.focused(), Some(SHOW));
    }

    #[test]
    fn compact_rtl_wraps_the_label_and_keeps_radio_art_at_logical_start() {
        let mut runtime = route();
        for id in [
            "radio-demo-theme",
            "radio-demo-rtl",
            "radio-demo-large-text",
        ] {
            assert!(runtime.activate(id).unwrap());
        }
        render(&mut runtime, 320., 720.);
        let hit = bounds(&runtime, IDS[WEEKLY]);
        let ring = bounds(&runtime, "radio-demo-weekly-ring");
        let label = bounds(&runtime, "radio-demo-weekly-label");
        let focus_ring = bounds(&runtime, "radio-demo-weekly-focus-ring");
        assert_eq!(hit.intersect(ring), ring);
        assert_eq!(hit.intersect(label), label);
        assert_eq!(hit.intersect(focus_ring), focus_ring);
        assert_eq!((ring.width, ring.height), (20., 20.));
        assert_eq!((focus_ring.width, focus_ring.height), (30., 30.));
        assert!(ring.x >= label.x + label.width);
        assert!(label.height > 48.);
        assert_eq!(runtime.app().radio_demo.selected, Some(WEEKLY));
        assert_eq!(runtime.app().radio_demo.requests, 0);
    }
}
