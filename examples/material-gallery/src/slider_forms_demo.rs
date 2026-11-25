use surface::{Align, Length, Node, Role, Style};
use surface_app::{FrameInput, View};
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChoiceState, StandardSliderMode,
    StandardSliderOptions, StandardSliderSize, Theme, VerticalStandardSliderPlacement,
};

use crate::{scroll_field_demo::control_row, Message};

const SHOW: &str = "slider-forms-demo-show";
const BACK: &str = "slider-forms-demo-back";
const THEME: &str = "slider-forms-demo-theme";
const RTL: &str = "slider-forms-demo-rtl";
const LARGE_TEXT: &str = "slider-forms-demo-large-text";
const SCROLL: &str = "slider-forms-demo-scroll";
const CENTERED: &str = "slider-forms-demo-centered";
const VERTICAL: &str = "slider-forms-demo-vertical";
const TWO_COLUMNS_AT: f32 = 720.;
const CENTER_RANGE: (f64, f64) = (-100., 100.);
const VERTICAL_RANGE: (f64, f64) = (0., 100.);
const VERTICAL_HEIGHT: f32 = 240.;
const VERTICAL_WIDTH: f32 = 48.;
const LEADING_CLEARANCE: f32 = 8.;
const LABEL_ROOM: f32 = 120.;
const ROUTE_PADDING: f32 = 16.;
const SCROLL_PADDING: f32 = 6.;
const NEUTRAL: f64 = 0.;
const CONTINUOUS: StandardSliderMode = StandardSliderMode::Continuous { keyboard_step: 5. };
const DISCRETE: StandardSliderMode = StandardSliderMode::Discrete { intervals: 8 };

struct Sample {
    id: &'static str,
    name: &'static str,
    mode: StandardSliderMode,
    disabled: bool,
}

const CENTERED_SAMPLES: [Sample; 4] = [
    Sample {
        id: "slider-forms-demo-centered-negative",
        name: "負值",
        mode: CONTINUOUS,
        disabled: false,
    },
    Sample {
        id: "slider-forms-demo-centered-neutral",
        name: "中立值",
        mode: CONTINUOUS,
        disabled: false,
    },
    Sample {
        id: "slider-forms-demo-centered-positive",
        name: "正值",
        mode: CONTINUOUS,
        disabled: false,
    },
    Sample {
        id: "slider-forms-demo-centered-discrete",
        name: "離散值",
        mode: DISCRETE,
        disabled: false,
    },
];

const VERTICAL_SAMPLES: [Sample; 3] = [
    Sample {
        id: "slider-forms-demo-vertical-continuous",
        name: "垂直連續",
        mode: CONTINUOUS,
        disabled: false,
    },
    Sample {
        id: "slider-forms-demo-vertical-discrete",
        name: "垂直階梯",
        mode: DISCRETE,
        disabled: false,
    },
    Sample {
        id: "slider-forms-demo-vertical-disabled",
        name: "垂直停用",
        mode: DISCRETE,
        disabled: true,
    },
];

struct Values<const N: usize> {
    values: [f64; N],
    requests: [usize; N],
    engaged: [bool; N],
}

impl<const N: usize> Values<N> {
    fn new(values: [f64; N]) -> Self {
        Self {
            values,
            requests: [0; N],
            engaged: [false; N],
        }
    }

    fn observe(&mut self, samples: &[Sample; N], focus: Option<&str>, press: Option<&str>) -> bool {
        let engaged = samples
            .each_ref()
            .map(|sample| !sample.disabled && [focus, press].contains(&Some(sample.id)));
        if self.engaged == engaged {
            return false;
        }
        self.engaged = engaged;
        true
    }

    fn update(&mut self, samples: &[Sample; N], range: (f64, f64), index: usize, value: f64) {
        if samples.get(index).is_some_and(|sample| !sample.disabled)
            && (range.0..=range.1).contains(&value)
        {
            self.values[index] = value;
            self.requests[index] += 1;
        }
    }
}

#[derive(Clone)]
pub enum SliderFormsEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    CenteredValue(usize, f64),
    VerticalValue(usize, f64),
}

pub(crate) struct SliderFormsDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    viewport: (f32, f32),
    centered: Values<{ CENTERED_SAMPLES.len() }>,
    vertical: Values<{ VERTICAL_SAMPLES.len() }>,
}

impl Default for SliderFormsDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            viewport: (960., 800.),
            centered: Values::new([-50., 0., 50., -25.]),
            vertical: Values::new([25., 50., 50.]),
        }
    }
}

impl SliderFormsDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.centered.engaged.fill(false);
        self.vertical.engaged.fill(false);
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
        let centered = self
            .centered
            .observe(&CENTERED_SAMPLES, input.focused, pressed);
        let vertical = self
            .vertical
            .observe(&VERTICAL_SAMPLES, input.focused, pressed);
        centered || vertical
    }

    pub(crate) fn update(&mut self, event: SliderFormsEvent) {
        match event {
            SliderFormsEvent::Show => self.visible = true,
            SliderFormsEvent::Back => self.leave(),
            SliderFormsEvent::ToggleDirection => self.rtl = !self.rtl,
            SliderFormsEvent::ToggleLargeText => self.large_text = !self.large_text,
            SliderFormsEvent::CenteredValue(index, value) if self.visible => {
                self.centered
                    .update(&CENTERED_SAMPLES, CENTER_RANGE, index, value);
            }
            SliderFormsEvent::VerticalValue(index, value) if self.visible => {
                self.vertical
                    .update(&VERTICAL_SAMPLES, VERTICAL_RANGE, index, value);
            }
            SliderFormsEvent::CenteredValue(..) | SliderFormsEvent::VerticalValue(..) => {}
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "置中與垂直滑桿",
                ButtonKind::Outlined,
                false,
                Message::SliderForms(SliderFormsEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn sample_column(wide: bool, align: Align) -> Style {
        let style = Style::column().align(align);
        if wide {
            style.width(Length::Px(0.)).grow(1.)
        } else {
            style.width(Length::Percent(1.))
        }
    }

    fn row(&self) -> View<Message> {
        let row = Node::new(Role::Group, "").styled(
            Style::row()
                .width(Length::Percent(1.))
                .align(Align::Start)
                .gap(24.),
        );
        View::new(if self.rtl { row.reverse_row() } else { row })
    }

    fn section(id: &str) -> View<Message> {
        View::new(
            Node::new(Role::Group, "")
                .id(id)
                .styled(Style::column().width(Length::Percent(1.)).gap(12.)),
        )
    }

    fn centered_sample(&self, theme: &Theme, index: usize, wide: bool) -> View<Message> {
        let sample = &CENTERED_SAMPLES[index];
        let value = self.centered.values[index];
        let value_label = signed_percent(value);
        let slider = theme
            .centered_slider(
                StandardSliderOptions {
                    id: sample.id,
                    name: sample.name,
                    min: CENTER_RANGE.0,
                    max: CENTER_RANGE.1,
                    value,
                    mode: sample.mode,
                    size: StandardSliderSize::Small,
                    direction: self.direction(),
                    disabled: sample.disabled,
                    engaged: self.centered.engaged[index],
                    value_label: Some(&value_label),
                },
                NEUTRAL,
                move |value| Message::SliderForms(SliderFormsEvent::CenteredValue(index, value)),
            )
            .expect("valid Centered Gallery slider");
        // Reserve the scaled label line, its vertical padding, and the gap
        // above the focus ring. The rail does not move when the label appears.
        let reserve = typography::LABEL_LARGE.line_height_px() * self.text_scale() + 24. + 9.;
        let clearance = View::new(
            Node::new(Role::Group, "").styled(Style::column().height(Length::Px(reserve))),
        );
        let align = if self.rtl { Align::End } else { Align::Start };
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-row", sample.id))
                .styled(Self::sample_column(wide, align)),
        )
        .children([
            theme
                .text(&format!("{}-caption", sample.id), &value_label)
                .without_interaction(None),
            clearance,
            slider,
        ])
    }

    fn centered_section(&self, theme: &Theme, wide: bool) -> View<Message> {
        let mut children = vec![theme.heading("slider-forms-demo-centered-title", "置中滑桿")];
        if wide {
            children.extend((0..CENTERED_SAMPLES.len()).step_by(2).map(|index| {
                self.row().children([
                    self.centered_sample(theme, index, wide),
                    self.centered_sample(theme, index + 1, wide),
                ])
            }));
        } else {
            children.extend(
                (0..CENTERED_SAMPLES.len()).map(|index| self.centered_sample(theme, index, wide)),
            );
        }
        Self::section(CENTERED).children(children)
    }

    fn vertical_sample(&self, theme: &Theme, index: usize, wide: bool) -> View<Message> {
        let sample = &VERTICAL_SAMPLES[index];
        let display = (self.vertical.values[index] * 10.).round() / 10.;
        let value_label = format!("{display}％");
        let available = self.viewport.0 - 2. * (ROUTE_PADDING + SCROLL_PADDING);
        let label_room = (available - VERTICAL_WIDTH - LEADING_CLEARANCE).clamp(0., LABEL_ROOM);
        let slider = theme
            .vertical_standard_slider(
                StandardSliderOptions {
                    id: sample.id,
                    name: sample.name,
                    min: VERTICAL_RANGE.0,
                    max: VERTICAL_RANGE.1,
                    value: self.vertical.values[index],
                    mode: sample.mode,
                    size: StandardSliderSize::Small,
                    direction: self.direction(),
                    disabled: sample.disabled,
                    engaged: self.vertical.engaged[index],
                    value_label: Some(&value_label),
                },
                VerticalStandardSliderPlacement {
                    height: VERTICAL_HEIGHT,
                    label_room,
                },
                move |value| Message::SliderForms(SliderFormsEvent::VerticalValue(index, value)),
            )
            .expect("valid Vertical Gallery slider");
        let space = |width| {
            View::new(Node::new(Role::Group, "").styled(Style::column().width(Length::Px(width))))
        };
        let lane = Node::new(Role::Group, "").styled(Style::row());
        let lane = if self.rtl { lane.reverse_row() } else { lane };
        let style = Self::sample_column(wide, Align::Center).gap(8.);
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-row", sample.id))
                .styled(style),
        )
        .children([
            theme
                .text(&format!("{}-caption", sample.id), sample.name)
                .without_interaction(None),
            View::new(lane).children([space(LEADING_CLEARANCE), slider, space(label_room)]),
        ])
    }

    fn vertical_section(&self, theme: &Theme, wide: bool) -> View<Message> {
        let mut children = vec![theme.heading("slider-forms-demo-vertical-title", "垂直滑桿")];
        let samples =
            (0..VERTICAL_SAMPLES.len()).map(|index| self.vertical_sample(theme, index, wide));
        if wide {
            children.push(self.row().children(samples));
        } else {
            children.extend(samples);
        }
        Self::section(VERTICAL).children(children)
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use SliderFormsEvent::*;
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
                .tonal_button(BACK, "返回", Message::SliderForms(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            toggle(THEME, "深色", dark, Message::Theme),
            toggle(RTL, "右起", self.rtl, Message::SliderForms(ToggleDirection)),
            toggle(
                LARGE_TEXT,
                "大字",
                self.large_text,
                Message::SliderForms(ToggleLargeText),
            ),
        ]);
        let wide = self.viewport.0 >= TWO_COLUMNS_AT;
        let mut scroll_style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .gap(16.)
            .padding(SCROLL_PADDING)
            .clip(true);
        scroll_style.max_width = Length::Px(856.);
        let scroller = View::new(
            Node::new(Role::Group, "滑桿範例")
                .id(SCROLL)
                .styled(scroll_style),
        )
        .accessibility_scroll_pages()
        .children([
            self.centered_section(theme, wide),
            self.vertical_section(theme, wide),
        ]);

        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("slider-forms-demo-route")
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.))
                        .align(Align::Start)
                        .padding(ROUTE_PADDING)
                        .gap(12.)
                        .background(theme.background)
                        .foreground(theme.on_background)
                        .clip(true),
                ),
        )
        .children([
            theme.heading("slider-forms-demo-title", "置中與垂直滑桿"),
            controls,
            scroller,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme, dark: bool) -> serde_json::Value {
        let color = |surface::Color(r, g, b, a)| [r, g, b, a];
        let colors = theme.colors;
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "dark": dark,
            "width": self.viewport.0,
            "height": self.viewport.1,
            "centered": {
                "min": CENTER_RANGE.0,
                "max": CENTER_RANGE.1,
                "neutral": NEUTRAL,
                "values": self.centered.values,
                "requests": self.centered.requests,
                "engaged": self.centered.engaged
            },
            "vertical": {
                "min": VERTICAL_RANGE.0,
                "max": VERTICAL_RANGE.1,
                "values": self.vertical.values,
                "requests": self.vertical.requests,
                "engaged": self.vertical.engaged
            },
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

fn signed_percent(value: f64) -> String {
    let rounded = (value * 10.).round() / 10.;
    if rounded == 0. {
        "0％".into()
    } else if rounded < 0. {
        format!("−{}％", -rounded)
    } else {
        format!("+{rounded}％")
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
                size * (width / available).ceil().max(1.),
            )
        }
    }

    fn draw(runtime: &mut Runtime<Gallery>, viewport: (f32, f32)) {
        runtime.render(viewport.0, viewport.1, &mut Fonts).unwrap();
    }

    fn route(viewport: (f32, f32)) -> Runtime<Gallery> {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        runtime
            .apply(Message::Section(crate::slider_demo::SECTION))
            .unwrap();
        draw(&mut runtime, viewport);
        assert!(runtime.activate("slider-demo-show").unwrap());
        draw(&mut runtime, viewport);
        assert!(runtime.dom().by_id("slider-demo-xs").is_some());
        assert!(runtime.dom().by_id(CENTERED).is_none());
        assert!(runtime.activate("slider-demo-back").unwrap());
        draw(&mut runtime, viewport);
        assert!(runtime.activate(SHOW).unwrap());
        draw(&mut runtime, viewport);
        runtime
    }

    fn bounds(runtime: &Runtime<Gallery>, id: &str) -> Rect {
        runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
    }

    fn contains(outer: Rect, inner: Rect) -> bool {
        let clipped = outer.intersect(inner);
        (clipped.x - inner.x).abs() < 0.001
            && (clipped.y - inner.y).abs() < 0.001
            && (clipped.width - inner.width).abs() < 0.001
            && (clipped.height - inner.height).abs() < 0.001
    }

    fn tab(runtime: &mut Runtime<Gallery>, viewport: (f32, f32)) {
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        draw(runtime, viewport);
    }

    fn key(runtime: &mut Runtime<Gallery>, key: EditKey) -> bool {
        runtime
            .dispatch(Event::Edit {
                key,
                extend: false,
                by_word: false,
            })
            .unwrap()
    }

    #[test]
    fn wide_route_mounts_both_forms_and_restores_the_opener() {
        let viewport = (960., 1040.);
        let mut runtime = route(viewport);
        let scroll = bounds(&runtime, SCROLL);
        let [negative, neutral, positive, discrete] = CENTERED_SAMPLES
            .each_ref()
            .map(|sample| bounds(&runtime, sample.id));
        assert_eq!(negative.y, neutral.y);
        assert_eq!(positive.y, discrete.y);
        assert!(negative.x + negative.width < neutral.x);
        assert!(positive.y > negative.y + negative.height);
        for (index, sample) in CENTERED_SAMPLES.iter().enumerate() {
            let rail = bounds(&runtime, &format!("{}-rail", sample.id));
            let track = bounds(&runtime, &format!("{}-inactive-min", sample.id));
            let handle = bounds(&runtime, &format!("{}-handle", sample.id));
            let fraction = [0.25, 0.5, 0.75, 0.375][index];
            assert!((handle.x + 2. - (rail.x + rail.width * fraction)).abs() < 0.001);
            assert_eq!(handle.width, 4.);
            assert_eq!(track.height, 24.);
            assert!(contains(scroll, rail));
            assert!(runtime
                .dom()
                .by_id(&format!("{}-label", sample.id))
                .is_none());
        }
        let [continuous, discrete, disabled] = VERTICAL_SAMPLES
            .each_ref()
            .map(|sample| bounds(&runtime, sample.id));
        assert!(continuous.x + continuous.width < discrete.x);
        assert!(discrete.x + discrete.width < disabled.x);
        assert_eq!(continuous.y, discrete.y);
        assert_eq!(continuous.y, disabled.y);
        for slider in [continuous, discrete, disabled] {
            assert_eq!((slider.width, slider.height), (48., 240.));
            assert!(contains(scroll, slider));
        }
        for id in [THEME, RTL, LARGE_TEXT]
            .into_iter()
            .chain(CENTERED_SAMPLES.iter().map(|sample| sample.id))
            .chain(VERTICAL_SAMPLES[..2].iter().map(|sample| sample.id))
            .chain([BACK])
        {
            tab(&mut runtime, viewport);
            assert_eq!(runtime.focused(), Some(id));
        }
        assert!(runtime.activate(BACK).unwrap());
        draw(&mut runtime, viewport);
        assert_eq!(runtime.focused(), Some(SHOW));
        assert!(runtime.dom().by_id("slider-demo-show").is_some());
        assert!(runtime.dom().by_id(VERTICAL).is_none());
    }

    #[test]
    fn compact_dark_rtl_focus_preserves_labels_and_adjusts_only_the_owners() {
        let viewport = (320., 880.);
        let mut runtime = route(viewport);
        for id in [THEME, RTL, LARGE_TEXT] {
            assert!(runtime.activate(id).unwrap());
            draw(&mut runtime, viewport);
        }
        assert!(runtime.app().dark && runtime.app().slider_forms_demo.rtl);
        assert_eq!(runtime.app().slider_forms_demo.text_scale(), 2.);
        tab(&mut runtime, viewport);
        let id = CENTERED_SAMPLES[0].id;
        assert_eq!(runtime.focused(), Some(id));
        let rail = bounds(&runtime, &format!("{id}-rail"));
        let handle = bounds(&runtime, &format!("{id}-handle"));
        let focus = bounds(&runtime, &format!("{id}-focus"));
        let caption = bounds(&runtime, &format!("{id}-caption"));
        let label = bounds(&runtime, &format!("{id}-label"));
        assert_eq!(handle.width, 2.);
        assert!((handle.x + 1. - (rail.x + rail.width * 0.75)).abs() < 0.001);
        assert!(label.y >= caption.y + caption.height - 0.001);
        assert!(label.y + label.height + 4. <= focus.y + 0.001);
        assert!(contains(bounds(&runtime, SCROLL), label));
        assert!(key(&mut runtime, EditKey::Left));
        draw(&mut runtime, viewport);
        assert_eq!(
            runtime.app().slider_forms_demo.centered.values,
            [-45., 0., 50., -25.]
        );
        assert_eq!(
            runtime.app().slider_forms_demo.centered.requests,
            [1, 0, 0, 0]
        );

        for _ in 0..4 {
            tab(&mut runtime, viewport);
        }
        let id = VERTICAL_SAMPLES[0].id;
        assert_eq!(runtime.focused(), Some(id));
        let root = bounds(&runtime, id);
        let focus = bounds(&runtime, &format!("{id}-focus"));
        let label = bounds(&runtime, &format!("{id}-label"));
        let handle = bounds(&runtime, &format!("{id}-handle"));
        let scroll = bounds(&runtime, SCROLL);
        assert!(label.x + label.width + 4. <= focus.x + 0.001);
        assert_eq!(handle.height, 2.);
        for part in [root, focus, label] {
            assert!(contains(scroll, part));
        }
        assert!(key(&mut runtime, EditKey::Up));
        draw(&mut runtime, viewport);
        assert!(key(&mut runtime, EditKey::End));
        draw(&mut runtime, viewport);
        assert_eq!(
            runtime.app().slider_forms_demo.vertical.values,
            [100., 50., 50.]
        );
        assert_eq!(runtime.app().slider_forms_demo.vertical.requests, [2, 0, 0]);
        assert_eq!(
            runtime.app().slider_forms_demo.vertical.engaged,
            [true, false, false]
        );
        assert!(runtime.set_value(VERTICAL_SAMPLES[2].id, 75.).is_err());
        assert_eq!(runtime.app().slider_forms_demo.vertical.requests, [2, 0, 0]);
    }
}
