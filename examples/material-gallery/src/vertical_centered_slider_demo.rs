use surface::{Align, Length, Node, Role, Style};
use surface_app::{FrameInput, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChoiceState, StandardSliderMode, StandardSliderOptions,
    StandardSliderSize, Theme, VerticalStandardSliderPlacement,
};

use crate::{scroll_field_demo::control_row, Message};

const SHOW: &str = "vertical-centered-slider-demo-show";
const BACK: &str = "vertical-centered-slider-demo-back";
const THEME: &str = "vertical-centered-slider-demo-theme";
const RTL: &str = "vertical-centered-slider-demo-rtl";
const LARGE_TEXT: &str = "vertical-centered-slider-demo-large-text";
const SCROLL: &str = "vertical-centered-slider-demo-scroll";

const WIDE_AT: f32 = 720.;
const HEIGHT: f32 = 240.;
const TARGET_WIDTH: f32 = 48.;
const LEADING_ROOM: f32 = 8.;
const LABEL_ROOM: f32 = 120.;
const ROUTE_PADDING: f32 = 16.;
const SCROLL_PADDING: f32 = 6.;
const MIN: f64 = -100.;
const MAX: f64 = 100.;
const NEUTRAL: f64 = 0.;

struct Sample {
    id: &'static str,
    name: &'static str,
    initially: f64,
    mode: StandardSliderMode,
    disabled: bool,
}

const CONTINUOUS: StandardSliderMode = StandardSliderMode::Continuous { keyboard_step: 5. };
const DISCRETE: StandardSliderMode = StandardSliderMode::Discrete { intervals: 8 };

const SAMPLES: [Sample; 5] = [
    Sample {
        id: "vertical-centered-slider-demo-negative",
        name: "負值平衡",
        initially: -50.,
        mode: CONTINUOUS,
        disabled: false,
    },
    Sample {
        id: "vertical-centered-slider-demo-neutral",
        name: "中立平衡",
        initially: 0.,
        mode: CONTINUOUS,
        disabled: false,
    },
    Sample {
        id: "vertical-centered-slider-demo-positive",
        name: "正值平衡",
        initially: 50.,
        mode: CONTINUOUS,
        disabled: false,
    },
    Sample {
        id: "vertical-centered-slider-demo-discrete",
        name: "階梯平衡",
        initially: -25.,
        mode: DISCRETE,
        disabled: false,
    },
    Sample {
        id: "vertical-centered-slider-demo-disabled",
        name: "停用平衡",
        initially: 50.,
        mode: DISCRETE,
        disabled: true,
    },
];

#[derive(Clone)]
pub enum VerticalCenteredSliderEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    Change(usize, f64),
}

pub(crate) struct VerticalCenteredSliderDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    viewport: (f32, f32),
    values: [f64; SAMPLES.len()],
    requests: [usize; SAMPLES.len()],
    engaged: Option<usize>,
}

impl Default for VerticalCenteredSliderDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            viewport: (960., 1040.),
            values: SAMPLES.each_ref().map(|sample| sample.initially),
            requests: [0; SAMPLES.len()],
            engaged: None,
        }
    }
}

impl VerticalCenteredSliderDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.engaged = None;
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let next = (width, height);
        if self.viewport == next {
            return false;
        }
        self.viewport = next;
        self.visible
    }

    pub(crate) fn observe_frame(&mut self, input: FrameInput<'_>, pressed: Option<&str>) -> bool {
        if !self.visible {
            return false;
        }
        let next = [pressed, input.focused]
            .into_iter()
            .flatten()
            .find_map(|id| {
                SAMPLES
                    .iter()
                    .position(|sample| sample.id == id && !sample.disabled)
            });
        if self.engaged == next {
            return false;
        }
        self.engaged = next;
        true
    }

    pub(crate) fn update(&mut self, event: VerticalCenteredSliderEvent) {
        use VerticalCenteredSliderEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            Change(index, value)
                if self.visible
                    && SAMPLES.get(index).is_some_and(|sample| !sample.disabled)
                    && (MIN..=MAX).contains(&value)
                    && self.values[index] != value =>
            {
                self.values[index] = value;
                self.requests[index] += 1;
            }
            Change(..) => {}
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "垂直置中滑桿",
                ButtonKind::Outlined,
                false,
                Message::VerticalCenteredSlider(VerticalCenteredSliderEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn sample(&self, theme: &Theme, index: usize, wide: bool) -> View<Message> {
        let sample = &SAMPLES[index];
        let value = self.values[index];
        let label = signed_value(value);
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let available = self.viewport.0 - 2. * (ROUTE_PADDING + SCROLL_PADDING);
        let label_room = (available - TARGET_WIDTH - LEADING_ROOM).clamp(0., LABEL_ROOM);
        let slider = theme
            .vertical_centered_slider(
                StandardSliderOptions {
                    id: sample.id,
                    name: sample.name,
                    min: MIN,
                    max: MAX,
                    value,
                    mode: sample.mode,
                    size: StandardSliderSize::Small,
                    direction,
                    disabled: sample.disabled,
                    engaged: self.engaged == Some(index),
                    value_label: Some(&label),
                },
                VerticalStandardSliderPlacement {
                    height: HEIGHT,
                    label_room,
                },
                NEUTRAL,
                move |value| {
                    Message::VerticalCenteredSlider(VerticalCenteredSliderEvent::Change(
                        index, value,
                    ))
                },
            )
            .expect("valid Gallery vertical centered slider");
        let lane = Node::new(Role::Group, "").styled(Style::row());
        let lane = if self.rtl { lane.reverse_row() } else { lane };
        let space = |width| {
            View::new(Node::new(Role::Group, "").styled(Style::column().width(Length::Px(width))))
        };
        let column = Style::column().align(Align::Center).gap(8.);
        let column = if wide {
            column.width(Length::Px(0.)).grow(1.)
        } else {
            column.width(Length::Percent(1.))
        };
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-row", sample.id))
                .styled(column),
        )
        .children([
            theme
                .text(&format!("{}-caption", sample.id), sample.name)
                .without_interaction(None),
            View::new(lane).children([space(LEADING_ROOM), slider, space(label_room)]),
        ])
    }

    fn sample_row(&self, theme: &Theme, indices: std::ops::Range<usize>) -> View<Message> {
        let row = Node::new(Role::Group, "").styled(
            Style::row()
                .width(Length::Percent(1.))
                .align(Align::Start)
                .gap(24.),
        );
        let row = if self.rtl { row.reverse_row() } else { row };
        View::new(row).children(indices.map(|index| self.sample(theme, index, true)))
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use VerticalCenteredSliderEvent::*;
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
                .tonal_button(BACK, "返回", Message::VerticalCenteredSlider(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            toggle(THEME, "深色", dark, Message::Theme),
            toggle(
                RTL,
                "右起",
                self.rtl,
                Message::VerticalCenteredSlider(ToggleDirection),
            ),
            toggle(
                LARGE_TEXT,
                "大字",
                self.large_text,
                Message::VerticalCenteredSlider(ToggleLargeText),
            ),
        ]);

        let mut scroll_style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .gap(24.)
            .padding(SCROLL_PADDING)
            .clip(true);
        scroll_style.max_width = Length::Px(856.);
        let scroll = View::new(
            Node::new(Role::Group, "垂直置中滑桿範例")
                .id(SCROLL)
                .styled(scroll_style),
        )
        .accessibility_scroll_pages();
        let scroll = if self.viewport.0 >= WIDE_AT {
            scroll.children([
                self.sample_row(theme, 0..3),
                self.sample_row(theme, 3..SAMPLES.len()),
            ])
        } else {
            scroll.children((0..SAMPLES.len()).map(|index| self.sample(theme, index, false)))
        };
        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("vertical-centered-slider-demo-route")
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
            theme.heading("vertical-centered-slider-demo-title", "垂直置中滑桿"),
            controls,
            scroll,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme, dark: bool) -> serde_json::Value {
        let color = |surface::Color(r, g, b, a)| [r, g, b, a];
        let colors = theme.colors;
        let samples: Vec<_> = SAMPLES.iter().enumerate().map(|(index, sample)| {
            let intervals = match sample.mode {
                StandardSliderMode::Continuous { .. } => None,
                StandardSliderMode::Discrete { intervals } => Some(intervals),
            };
            serde_json::json!({
                "id": sample.id, "value": self.values[index], "label": signed_value(self.values[index]),
                "intervals": intervals, "requests": self.requests[index], "engaged": self.engaged == Some(index),
                "disabled": sample.disabled,
            })
        }).collect();
        serde_json::json!({
            "visible": self.visible, "rtl": self.rtl, "text_scale": self.text_scale(),
            "dark": dark, "width": self.viewport.0, "height": self.viewport.1,
            "min": MIN, "max": MAX, "neutral": NEUTRAL, "samples": samples,
            "palette": {
                "background": color(theme.background), "primary": color(colors.primary),
                "on_primary": color(colors.on_primary), "secondary_container": color(colors.secondary_container),
                "on_secondary_container": color(colors.on_secondary_container), "secondary": color(colors.secondary),
                "on_surface": color(colors.on_surface), "inverse_surface": color(colors.inverse_surface),
                "inverse_on_surface": color(colors.inverse_on_surface)
            }
        })
    }
}

fn signed_value(value: f64) -> String {
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

    const HOME: (f32, f32) = (960., 1900.);
    const WIDE: (f32, f32) = (960., 1040.);
    const COMPACT: (f32, f32) = (320., 880.);

    struct Fonts;

    impl TextMeasurer for Fonts {
        fn measure(&mut self, text: &str, size: f32, max_width: Option<f32>) -> (f32, f32) {
            let natural = text.chars().count() as f32 * size;
            let limit = max_width.unwrap_or(natural).max(1.);
            (natural.min(limit), size * (natural / limit).ceil().max(1.))
        }
    }

    fn draw(runtime: &mut Runtime<Gallery>, viewport: (f32, f32)) {
        runtime.render(viewport.0, viewport.1, &mut Fonts).unwrap();
    }

    fn home() -> Runtime<Gallery> {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        runtime
            .apply(Message::Section(crate::slider_demo::SECTION))
            .unwrap();
        draw(&mut runtime, HOME);
        runtime
    }

    fn open(runtime: &mut Runtime<Gallery>, viewport: (f32, f32)) {
        let entry = runtime.dom().by_id(SHOW).unwrap();
        let geometry = &runtime.computed().unwrap().nodes[entry.index()];
        assert!(contains(geometry.clip, geometry.bounds), "{geometry:?}");
        assert!(runtime.activate(SHOW).unwrap());
        draw(runtime, viewport);
        assert_eq!(runtime.focused(), Some(BACK));
    }

    fn bounds(runtime: &Runtime<Gallery>, key: &str) -> Rect {
        let id = runtime.dom().by_id(key).unwrap();
        runtime.computed().unwrap().nodes[id.index()].bounds
    }

    fn contains(outer: Rect, inner: Rect) -> bool {
        let clipped = outer.intersect(inner);
        (clipped.x - inner.x).abs() < 0.001
            && (clipped.y - inner.y).abs() < 0.001
            && (clipped.width - inner.width).abs() < 0.001
            && (clipped.height - inner.height).abs() < 0.001
    }

    fn tab(runtime: &mut Runtime<Gallery>, viewport: (f32, f32), reverse: bool) {
        runtime.dispatch(Event::FocusNext { reverse }).unwrap();
        draw(runtime, viewport);
    }

    fn key(runtime: &mut Runtime<Gallery>, value: EditKey) {
        assert!(runtime
            .dispatch(Event::Edit {
                key: value,
                extend: false,
                by_word: false
            })
            .unwrap());
    }

    #[test]
    fn wide_route_preserves_old_launchers_and_shows_three_distinct_signed_spans() {
        let mut runtime = home();
        for (show, sample, back) in [
            ("slider-demo-show", "slider-demo-xs", "slider-demo-back"),
            (
                "slider-forms-demo-show",
                "slider-forms-demo-vertical-discrete",
                "slider-forms-demo-back",
            ),
            (
                "range-slider-demo-show",
                "range-slider-demo-continuous-lower",
                "range-slider-demo-back",
            ),
        ] {
            assert!(runtime.activate(show).unwrap());
            draw(&mut runtime, WIDE);
            assert!(runtime.dom().by_id(sample).is_some());
            assert!(runtime.dom().by_id(SAMPLES[0].id).is_none());
            assert!(runtime.activate(back).unwrap());
            draw(&mut runtime, HOME);
            assert_eq!(runtime.focused(), Some(show));
        }
        open(&mut runtime, WIDE);
        let targets = SAMPLES.each_ref().map(|sample| bounds(&runtime, sample.id));
        assert_eq!(targets[0].y, targets[1].y);
        assert_eq!(targets[1].y, targets[2].y);
        assert!(targets[0].x < targets[1].x && targets[1].x < targets[2].x);
        assert!(targets[3].y > targets[0].y && targets[3].y == targets[4].y);
        let clip = bounds(&runtime, SCROLL);
        for (index, sample) in SAMPLES.iter().enumerate() {
            let id = runtime.dom().by_id(sample.id).unwrap();
            let node = runtime.dom().get(id).unwrap();
            assert_eq!(node.name(), sample.name);
            assert_eq!(node.role(), Role::Slider);
            assert!(node.is_vertical_slider());
            assert_eq!(
                node.slider_value_text(),
                Some(signed_value(sample.initially).as_str())
            );
            assert_eq!(node.is_tab_stop(), !sample.disabled);
            assert!(contains(clip, targets[index]));
            assert_eq!((targets[index].width, targets[index].height), (48., 240.));
        }
        let rails = SAMPLES
            .each_ref()
            .map(|sample| bounds(&runtime, &format!("{}-rail", sample.id)));
        let negative = bounds(&runtime, "vertical-centered-slider-demo-negative-active");
        let neutral = bounds(&runtime, "vertical-centered-slider-demo-neutral-active");
        let positive = bounds(&runtime, "vertical-centered-slider-demo-positive-active");
        let middle = rails[0].y + rails[0].height / 2.;
        assert_eq!(negative.y, middle + 2.);
        assert_eq!(neutral.height, 0.);
        assert_eq!(positive.y + positive.height, middle - 2.);
        assert_eq!(negative.height, positive.height);
        assert!(negative.height > 0.);
        for expected in [THEME, RTL, LARGE_TEXT]
            .into_iter()
            .chain(
                SAMPLES
                    .iter()
                    .filter(|sample| !sample.disabled)
                    .map(|sample| sample.id),
            )
            .chain([BACK])
        {
            tab(&mut runtime, WIDE, false);
            assert_eq!(runtime.focused(), Some(expected));
        }
        tab(&mut runtime, WIDE, true);
        assert_eq!(runtime.focused(), Some(SAMPLES[3].id));
        assert!(runtime.activate(BACK).unwrap());
        draw(&mut runtime, WIDE);
        assert_eq!(runtime.focused(), Some(SHOW));
        assert!(runtime.dom().by_id(SCROLL).is_none());
    }

    #[test]
    fn compact_dark_rtl_scroll_reveals_upright_large_labels_and_visible_focus_rings() {
        let mut runtime = home();
        open(&mut runtime, COMPACT);
        for control in [THEME, RTL, LARGE_TEXT] {
            assert!(runtime.activate(control).unwrap());
            draw(&mut runtime, COMPACT);
        }
        let model = &runtime.app().vertical_centered_slider_demo;
        assert!(runtime.app().dark && model.rtl);
        assert_eq!(model.text_scale(), 2.);
        assert_eq!(model.viewport, COMPACT);
        let scroll = bounds(&runtime, SCROLL);
        for sample in &SAMPLES[..4] {
            tab(&mut runtime, COMPACT, false);
            assert_eq!(runtime.focused(), Some(sample.id));
            let part = |suffix| bounds(&runtime, &format!("{}-{suffix}", sample.id));
            let target = bounds(&runtime, sample.id);
            let handle = part("handle");
            let focus = part("focus");
            let label = part("label");
            let text = part("label-text");
            assert_eq!(handle.height, 2.);
            assert_eq!(focus.height, 12.);
            assert_eq!(label.x + label.width + 4., focus.x);
            assert!(label.height >= 64.);
            assert!(contains(label, text));
            for region in [target, focus, label] {
                assert!(
                    contains(scroll, region),
                    "{}: {region:?} exceeds {scroll:?}",
                    sample.id
                );
            }
        }
        let id = SAMPLES[4].id;
        let initial = bounds(&runtime, id);
        runtime
            .dispatch(Event::Scroll {
                x: scroll.x + scroll.width / 2.,
                y: scroll.y + scroll.height / 2.,
                delta_x: 0.,
                delta_y: 1024.,
            })
            .unwrap();
        draw(&mut runtime, COMPACT);
        let disabled = bounds(&runtime, id);
        assert!(disabled.y < initial.y && contains(scroll, disabled));
        assert!(runtime.set_value(id, 75.).is_err());
        assert_eq!(runtime.app().vertical_centered_slider_demo.requests[4], 0);
    }

    #[test]
    fn signed_discrete_drag_and_keys_use_y_and_the_same_authored_stops() {
        let mut runtime = home();
        open(&mut runtime, WIDE);
        assert!(runtime.activate(RTL).unwrap());
        draw(&mut runtime, WIDE);
        let id = SAMPLES[3].id;
        let rail = bounds(&runtime, &format!("{id}-rail"));
        let x = rail.x + rail.width / 2.;
        let y = rail.y + rail.height * 0.87;
        for event in [
            Event::PointerDown { x, y },
            Event::PointerMove { x: x + 10., y },
            Event::PointerMove { x, y: rail.y },
            Event::PointerUp { x, y: rail.y },
        ] {
            runtime.dispatch_touch(event).unwrap();
            draw(&mut runtime, WIDE);
        }
        assert_eq!(runtime.focused(), Some(id));
        assert_eq!(runtime.app().vertical_centered_slider_demo.values[3], 100.);
        assert_eq!(runtime.app().vertical_centered_slider_demo.requests[3], 2);
        for (event, expected) in [
            (EditKey::Home, -100.),
            (EditKey::Right, -75.),
            (EditKey::Up, -50.),
        ] {
            key(&mut runtime, event);
            draw(&mut runtime, WIDE);
            assert_eq!(
                runtime.app().vertical_centered_slider_demo.values[3],
                expected
            );
        }
        assert!(runtime.set_value(id, 34.).unwrap());
        assert!(!runtime.set_value(id, 27.).unwrap());
        assert_eq!(
            runtime.app().vertical_centered_slider_demo.values,
            [-50., 0., 50., 25., 50.]
        );
        assert_eq!(
            runtime.app().vertical_centered_slider_demo.requests,
            [0, 0, 0, 6, 0]
        );
    }
}
