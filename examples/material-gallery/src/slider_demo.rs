use surface::{Align, Length, Node, Role, Style};
use surface_app::{FrameInput, View};
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChoiceState, StandardSliderMode,
    StandardSliderOptions, StandardSliderSize, Theme,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "slider-demo-show";
const BACK: &str = "slider-demo-back";
const SCROLL: &str = "slider-demo-scroll";
const THEME: &str = "slider-demo-theme";
const RTL: &str = "slider-demo-rtl";
const LARGE_TEXT: &str = "slider-demo-large-text";
const TWO_COLUMNS_AT: f32 = 720.;
const INITIAL_VALUES: [f64; 6] = [25., 50., 50., 50., 50., 50.];

struct Sample {
    id: &'static str,
    label: &'static str,
    size: StandardSliderSize,
    mode: StandardSliderMode,
    disabled: bool,
}

const SAMPLES: [Sample; 6] = [
    Sample {
        id: "slider-demo-xs",
        label: "特小連續",
        size: StandardSliderSize::ExtraSmall,
        mode: StandardSliderMode::Continuous { keyboard_step: 5. },
        disabled: false,
    },
    Sample {
        id: "slider-demo-s",
        label: "小型階梯",
        size: StandardSliderSize::Small,
        mode: StandardSliderMode::Discrete { intervals: 8 },
        disabled: false,
    },
    Sample {
        id: "slider-demo-m",
        label: "中型滑桿",
        size: StandardSliderSize::Medium,
        mode: StandardSliderMode::Continuous { keyboard_step: 10. },
        disabled: false,
    },
    Sample {
        id: "slider-demo-l",
        label: "大型滑桿",
        size: StandardSliderSize::Large,
        mode: StandardSliderMode::Discrete { intervals: 4 },
        disabled: false,
    },
    Sample {
        id: "slider-demo-xl",
        label: "特大滑桿",
        size: StandardSliderSize::ExtraLarge,
        mode: StandardSliderMode::Continuous { keyboard_step: 10. },
        disabled: false,
    },
    Sample {
        id: "slider-demo-disabled",
        label: "停用滑桿",
        size: StandardSliderSize::Small,
        mode: StandardSliderMode::Discrete { intervals: 8 },
        disabled: true,
    },
];

#[derive(Clone)]
pub enum SliderDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    Value(usize, f64),
}

pub(crate) struct SliderDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    viewport: (f32, f32),
    engaged: [bool; SAMPLES.len()],
    values: [f64; SAMPLES.len()],
    requests: [usize; SAMPLES.len()],
}

impl Default for SliderDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            viewport: (960., 800.),
            engaged: [false; SAMPLES.len()],
            values: INITIAL_VALUES,
            requests: [0; SAMPLES.len()],
        }
    }
}

impl SliderDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.engaged.fill(false);
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let viewport = (width, height);
        if self.viewport == viewport {
            return false;
        }
        self.viewport = viewport;
        self.visible
    }

    pub(crate) fn observe_frame(&mut self, input: FrameInput<'_>, pressed: Option<&str>) -> bool {
        if !self.visible {
            return false;
        }
        let engaged = SAMPLES
            .each_ref()
            .map(|sample| !sample.disabled && [input.focused, pressed].contains(&Some(sample.id)));
        if self.engaged == engaged {
            return false;
        }
        self.engaged = engaged;
        true
    }

    pub(crate) fn update(&mut self, event: SliderDemoEvent) {
        match event {
            SliderDemoEvent::Show => self.visible = true,
            SliderDemoEvent::Back => self.leave(),
            SliderDemoEvent::ToggleDirection => self.rtl = !self.rtl,
            SliderDemoEvent::ToggleLargeText => self.large_text = !self.large_text,
            SliderDemoEvent::Value(index, value) => {
                if self.visible
                    && SAMPLES.get(index).is_some_and(|sample| !sample.disabled)
                    && (0. ..=100.).contains(&value)
                {
                    self.values[index] = value;
                    self.requests[index] += 1;
                }
            }
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "標準滑桿",
                ButtonKind::Outlined,
                false,
                Message::SliderDemo(SliderDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn sample(&self, theme: &Theme, index: usize, wide: bool) -> View<Message> {
        let sample = &SAMPLES[index];
        let value = self.values[index];
        let display_value = (value * 10.).round() / 10.;
        let value_label = format!("{display_value}％");
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let slider = theme
            .standard_slider(
                StandardSliderOptions {
                    id: sample.id,
                    name: sample.label,
                    min: 0.,
                    max: 100.,
                    value,
                    mode: sample.mode,
                    size: sample.size,
                    direction,
                    disabled: sample.disabled,
                    engaged: self.engaged[index],
                    value_label: Some(&value_label),
                },
                move |value| Message::SliderDemo(SliderDemoEvent::Value(index, value)),
            )
            .expect("valid Gallery slider");
        let mut style = Style::column().align(if self.rtl { Align::End } else { Align::Start });
        if wide {
            style = style.width(Length::Px(0.)).grow(1.);
        } else {
            style = style.width(Length::Percent(1.));
        }
        // One Label Large line, its 24dp block padding, and clearance from
        // the handle's outer focus outline. Keep the reserved space stable
        // as focus and pressed feedback come and go.
        let reserve = typography::LABEL_LARGE.line_height_px() * self.text_scale() + 24. + 9.;
        let clearance = View::new(
            Node::new(Role::Group, "").styled(Style::column().height(Length::Px(reserve))),
        );
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-row", sample.id))
                .styled(style),
        )
        .children([
            theme
                .text(&format!("{}-caption", sample.id), sample.label)
                .without_interaction(None),
            clearance,
            slider,
        ])
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use SliderDemoEvent::*;
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
        };
        let controls = control_row(vec![
            theme
                .tonal_button(BACK, "返回", Message::SliderDemo(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            toggle(THEME, "深色", dark, Message::Theme),
            toggle(RTL, "右起", self.rtl, Message::SliderDemo(ToggleDirection)),
            toggle(
                LARGE_TEXT,
                "大字",
                self.large_text,
                Message::SliderDemo(ToggleLargeText),
            ),
        ]);
        let wide = self.viewport.0 >= TWO_COLUMNS_AT;
        let mut scroll_style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .gap(16.)
            .padding(6.)
            .clip(true);
        scroll_style.max_width = Length::Px(856.);
        let scroller = View::new(
            Node::new(Role::Group, "滑桿範例")
                .id(SCROLL)
                .styled(scroll_style),
        )
        .accessibility_scroll_pages();
        let scroller = if wide {
            scroller.children((0..SAMPLES.len()).step_by(2).map(|index| {
                let row = Node::new(Role::Group, "").styled(
                    Style::row()
                        .width(Length::Percent(1.))
                        .align(Align::Start)
                        .gap(24.),
                );
                let row = if self.rtl { row.reverse_row() } else { row };
                View::new(row).children([
                    self.sample(theme, index, wide),
                    self.sample(theme, index + 1, wide),
                ])
            }))
        } else {
            scroller.children((0..SAMPLES.len()).map(|index| self.sample(theme, index, wide)))
        };

        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("slider-demo-route")
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
            theme.heading("slider-demo-title", "標準滑桿"),
            controls,
            scroller,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        let color = |surface::Color(r, g, b, a)| [r, g, b, a];
        let colors = theme.colors;
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "width": self.viewport.0,
            "height": self.viewport.1,
            "engaged": self.engaged,
            "values": self.values,
            "requests": self.requests,
            "palette": {
                "background": color(theme.background),
                "primary": color(colors.primary),
                "on_primary": color(colors.on_primary),
                "secondary_container": color(colors.secondary_container),
                "on_secondary_container": color(colors.on_secondary_container),
                "secondary": color(colors.secondary),
                "on_surface": color(colors.on_surface),
                "inverse_surface": color(colors.inverse_surface),
                "inverse_on_surface": color(colors.inverse_on_surface)
            }
        })
    }
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
                size * (width / available).ceil().max(1.),
            )
        }
    }

    fn draw(runtime: &mut Runtime<Gallery>, width: f32, height: f32) {
        runtime.render(width, height, &mut Fonts).unwrap();
    }

    fn route() -> Runtime<Gallery> {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        runtime.apply(Message::Section(SECTION)).unwrap();
        draw(&mut runtime, 960., 800.);
        assert!(runtime.activate(SHOW).unwrap());
        draw(&mut runtime, 960., 800.);
        runtime
    }

    fn bounds(runtime: &Runtime<Gallery>, id: &str) -> Rect {
        runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
    }

    fn tab(runtime: &mut Runtime<Gallery>, reverse: bool) {
        runtime.dispatch(Event::FocusNext { reverse }).unwrap();
    }

    #[test]
    fn wide_presets_are_visible_and_keyboard_visits_one_stop_per_enabled_slider() {
        let mut runtime = route();
        let scroll = bounds(&runtime, SCROLL);
        let first = bounds(&runtime, SAMPLES[0].id);
        let second = bounds(&runtime, SAMPLES[1].id);
        assert_eq!(first.y, second.y);
        assert!(first.x + first.width < second.x);
        assert_eq!(first.width, second.width);
        for (sample, height) in SAMPLES.iter().zip([48., 48., 52., 68., 108., 48.]) {
            let slider = bounds(&runtime, sample.id);
            assert_eq!(slider.height, height);
            assert_eq!(scroll.intersect(slider), slider);
        }

        assert_eq!(runtime.focused(), Some(BACK));
        for id in [THEME, RTL, LARGE_TEXT]
            .into_iter()
            .chain(SAMPLES[..5].iter().map(|sample| sample.id))
            .chain([BACK])
        {
            tab(&mut runtime, false);
            draw(&mut runtime, 960., 800.);
            assert_eq!(runtime.focused(), Some(id));
        }
        tab(&mut runtime, true);
        draw(&mut runtime, 960., 800.);
        assert_eq!(runtime.focused(), Some(SAMPLES[4].id));
        assert_eq!(runtime.app().slider_demo.values, INITIAL_VALUES);
        assert_eq!(runtime.app().slider_demo.requests, [0; 6]);

        assert!(runtime.set_value(SAMPLES[1].id, 63.).unwrap());
        draw(&mut runtime, 960., 800.);
        assert_eq!(
            runtime.app().slider_demo.values,
            [25., 62.5, 50., 50., 50., 50.]
        );
        assert_eq!(runtime.app().slider_demo.requests, [0, 1, 0, 0, 0, 0]);
        let disabled = bounds(&runtime, SAMPLES[5].id);
        for event in [
            Event::PointerDown {
                x: disabled.x + 12.,
                y: disabled.y + 24.,
            },
            Event::PointerUp {
                x: disabled.x + 12.,
                y: disabled.y + 24.,
            },
        ] {
            runtime.dispatch(event).unwrap();
        }
        assert_eq!(runtime.app().slider_demo.requests, [0, 1, 0, 0, 0, 0]);
        assert!(!runtime.app().slider_demo.engaged[5]);
        runtime.activate(BACK).unwrap();
        draw(&mut runtime, 960., 800.);
        assert_eq!(runtime.focused(), Some(SHOW));
        assert!(!runtime.app().slider_demo.visible());
    }

    #[test]
    fn compact_dark_rtl_keeps_an_enlarged_value_clear_of_caption_and_focus() {
        let mut runtime = route();
        for id in [THEME, RTL, LARGE_TEXT] {
            runtime.activate(id).unwrap();
            draw(&mut runtime, 320., 400.);
        }
        for _ in 0..5 {
            if runtime.focused() == Some(SAMPLES[0].id) {
                break;
            }
            tab(&mut runtime, false);
            draw(&mut runtime, 320., 400.);
        }

        assert!(runtime.app().dark);
        assert!(runtime.app().slider_demo.rtl);
        assert_eq!(runtime.app().slider_demo.text_scale(), 2.);
        assert_eq!(runtime.focused(), Some(SAMPLES[0].id));
        assert_eq!(
            runtime.app().slider_demo.engaged,
            [true, false, false, false, false, false]
        );
        let id = SAMPLES[0].id;
        let slider = bounds(&runtime, id);
        let second = bounds(&runtime, SAMPLES[1].id);
        assert_eq!((slider.x, slider.width), (second.x, second.width));
        assert!(second.y > slider.y + slider.height);
        let rail = bounds(&runtime, &format!("{id}-rail"));
        let handle = bounds(&runtime, &format!("{id}-handle"));
        let focus = bounds(&runtime, &format!("{id}-focus"));
        assert_eq!(handle.width, 2.);
        assert!((handle.x + 1. - (rail.x + rail.width * 0.75)).abs() < 0.01);

        let caption = bounds(&runtime, &format!("{id}-caption"));
        let label = bounds(&runtime, &format!("{id}-label"));
        let text = bounds(&runtime, &format!("{id}-label-text"));
        assert!(label.y >= caption.y + caption.height - 0.01);
        assert!(label.y + label.height + 4. <= focus.y + 0.01);
        assert!(label.height >= 64.);
        assert_eq!(label.intersect(text), text);
        let scroll = bounds(&runtime, SCROLL);
        for part in [label, focus, slider] {
            assert_eq!(scroll.intersect(part), part);
        }

        tab(&mut runtime, false);
        draw(&mut runtime, 320., 400.);
        assert_eq!(runtime.focused(), Some(SAMPLES[1].id));
        let revealed = bounds(&runtime, SAMPLES[1].id);
        assert_eq!(scroll.intersect(revealed), revealed);
        assert_eq!(runtime.app().slider_demo.requests, [0; 6]);
    }
}
