use std::sync::OnceLock;

use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, FramePress, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChoiceState, SwitchIcons, SwitchMotion, SwitchOptions,
    Theme, VectorIcon,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "switch-demo-show";
const BACK: &str = "switch-demo-back";
const IDS: [&str; 5] = [
    "switch-demo-plain",
    "switch-demo-selected-icon",
    "switch-demo-both-icons",
    "switch-demo-disabled-off",
    "switch-demo-disabled-on",
];
const LABELS: [&str; IDS.len()] = [
    "通知",
    "相機",
    "在行動網路上同步專案資料與附件",
    "位置存取",
    "自動備份",
];
const FIRST_DISABLED: usize = 3;

#[derive(Clone)]
pub enum SwitchDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    Set(usize, bool),
}

pub(crate) struct SwitchDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    checked: [bool; IDS.len()],
    changes: usize,
    motions: [SwitchMotion; IDS.len()],
}

impl Default for SwitchDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            checked: [false, true, false, false, true],
            changes: 0,
            motions: std::array::from_fn(|_| SwitchMotion::new()),
        }
    }
}

impl SwitchDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.motions = std::array::from_fn(|_| SwitchMotion::new());
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

    pub(crate) fn update(&mut self, event: SwitchDemoEvent) {
        use SwitchDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            Set(index, checked) if index < FIRST_DISABLED && self.checked[index] != checked => {
                self.checked[index] = checked;
                self.changes += 1;
            }
            Set(_, _) => {}
        }
    }

    pub(crate) fn frame(
        &mut self,
        frame: &AnimationFrame<'_>,
        input: &FrameInput<'_>,
        press: Option<FramePress<'_>>,
        hovered: Option<&str>,
    ) -> AnimationUpdate {
        let mut update = AnimationUpdate::default();
        if self.visible {
            let direction = self.direction();
            let scale = self.text_scale();
            for (index, motion) in self.motions.iter_mut().enumerate() {
                let options = options(index, self.checked[index], direction, scale);
                let row = motion.advance(options, frame, input, press, hovered);
                update.rebuild |= row.rebuild;
                update.request_next |= row.request_next;
            }
        }
        update
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "開關",
                ButtonKind::Outlined,
                false,
                Message::SwitchDemo(SwitchDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use SwitchDemoEvent::*;
        let event = Message::SwitchDemo;
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
            setting("switch-demo-theme", "深色", dark, Message::Theme),
            setting("switch-demo-rtl", "右起", self.rtl, event(ToggleDirection)),
            setting(
                "switch-demo-large-text",
                "大字",
                self.large_text,
                event(ToggleLargeText),
            ),
        ]);
        let mut switch_column = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .padding(8.)
            .gap(8.)
            .clip(true);
        switch_column.max_width = Length::Px(480.);
        let switches = View::new(
            Node::new(Role::Group, "裝置設定")
                .id("switch-demo-scroll")
                .styled(switch_column),
        )
        .accessibility_scroll_pages()
        .children(self.motions.iter().enumerate().map(|(index, motion)| {
            theme.switch_with_options(
                options(index, self.checked[index], direction, scale),
                motion,
                move |checked| event(Set(index, checked)),
            )
        }));
        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("switch-demo")
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
            theme.heading("switch-demo-title", "開關"),
            controls,
            switches,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "checked": self.checked,
            "changes": self.changes,
            "palette": {
                "surface": theme.colors.surface,
                "primary": theme.colors.primary,
                "on_primary": theme.colors.on_primary,
                "outline": theme.colors.outline,
                "surface_container_highest": theme.colors.surface_container_highest,
                "secondary": theme.colors.secondary,
            },
        })
    }
}

fn options(
    index: usize,
    selected: bool,
    direction: LayoutDirection,
    text_scale: f32,
) -> SwitchOptions<'static> {
    let [off, on] = icons();
    let icons = match index {
        0 => SwitchIcons::None,
        1 => SwitchIcons::Selected(on),
        _ => SwitchIcons::Both {
            unselected: off,
            selected: on,
        },
    };
    SwitchOptions {
        id: IDS[index],
        label: LABELS[index],
        name: (index == 2).then_some("行動網路同步"),
        state: ChoiceState {
            selected,
            disabled: index >= FIRST_DISABLED,
        },
        icons,
        direction,
        text_scale,
    }
}

fn icons() -> &'static [VectorIcon; 2] {
    static ICONS: OnceLock<[VectorIcon; 2]> = OnceLock::new();
    ICONS.get_or_init(|| {
        [
            VectorIcon::new([[5., 10.], [19., 10.], [19., 14.], [5., 14.]])
                .expect("valid minus contour"),
            VectorIcon::new([
                [3., 12.],
                [6., 9.],
                [10., 13.],
                [18., 5.],
                [21., 8.],
                [10., 19.],
            ])
            .expect("valid check contour"),
        ]
    })
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
        render(&mut runtime, 960., 1000.);
        assert!(runtime.activate(SHOW).unwrap());
        render(&mut runtime, 960., 800.);
        runtime
    }

    fn bounds(runtime: &Runtime<Gallery>, id: &str) -> Rect {
        runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
    }

    #[test]
    fn route_restores_focus_and_only_enabled_switches_change() {
        let mut runtime = route();
        assert_eq!(runtime.focused(), Some(BACK));
        for target in [
            "switch-demo-theme",
            "switch-demo-rtl",
            "switch-demo-large-text",
            IDS[0],
            IDS[1],
            IDS[2],
        ] {
            runtime
                .dispatch(Event::FocusNext { reverse: false })
                .unwrap();
            render(&mut runtime, 960., 800.);
            assert_eq!(runtime.focused(), Some(target));
            assert_eq!(runtime.app().switch_demo.changes, 0);
        }
        runtime.dispatch(Event::Activate).unwrap();
        render(&mut runtime, 960., 800.);
        assert_eq!(runtime.focused(), Some(IDS[2]));
        assert_eq!(
            runtime.app().switch_demo.checked,
            [false, true, true, false, true]
        );
        assert_eq!(runtime.app().switch_demo.changes, 1);
        for disabled in &IDS[FIRST_DISABLED..] {
            assert!(matches!(
                runtime.activate(disabled),
                Err(surface_app::AppError::MissingControl(id)) if id == *disabled
            ));
        }
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        render(&mut runtime, 960., 800.);
        assert!(!runtime
            .focused()
            .is_some_and(|id| IDS[FIRST_DISABLED..].contains(&id)));
        assert!(runtime.activate(BACK).unwrap());
        render(&mut runtime, 960., 1000.);
        assert_eq!(runtime.focused(), Some(SHOW));
    }

    #[test]
    fn compact_rtl_keeps_the_managed_label_and_art_inside_the_hit_target() {
        let mut runtime = route();
        for setting in [
            "switch-demo-theme",
            "switch-demo-rtl",
            "switch-demo-large-text",
        ] {
            assert!(runtime.activate(setting).unwrap());
        }
        render(&mut runtime, 320., 480.);
        let screen = Rect::new(0., 0., 320., 480.);
        let hit = bounds(&runtime, IDS[2]);
        let track = bounds(&runtime, "switch-demo-both-icons-track");
        let handle = bounds(&runtime, "switch-demo-both-icons-handle");
        let label = bounds(&runtime, "switch-demo-both-icons-label");

        assert_eq!(hit.intersect(screen), hit);
        assert_eq!(hit.intersect(track), track);
        assert_eq!(hit.intersect(label), label);
        assert_eq!((track.width, track.height), (52., 32.));
        assert_eq!((handle.width, handle.height), (24., 24.));
        assert_eq!(handle.x + handle.width / 2. - track.x, 36.);
        assert!(track.x + track.width < label.x);
        assert!(label.height > hit.height.min(48.));
    }
}
