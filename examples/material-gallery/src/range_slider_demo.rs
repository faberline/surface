use surface::{Align, Length, Node, Role, Style};
use surface_app::{FrameInput, View};
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChoiceState, RangeSliderChange,
    RangeSliderOptions, RangeSliderThumb, StandardSliderMode, StandardSliderSize, Theme,
};

use crate::{scroll_field_demo::control_row, Message};

const SHOW: &str = "range-slider-demo-show";
const BACK: &str = "range-slider-demo-back";
const THEME: &str = "range-slider-demo-theme";
const RTL: &str = "range-slider-demo-rtl";
const LARGE_TEXT: &str = "range-slider-demo-large-text";
const COLLAPSE: &str = "range-slider-demo-collapse";
const SCROLL: &str = "range-slider-demo-scroll";

const CONTINUOUS: StandardSliderMode = StandardSliderMode::Continuous { keyboard_step: 5. };
const DISCRETE: StandardSliderMode = StandardSliderMode::Discrete { intervals: 8 };

struct Sample {
    id: &'static str,
    name: &'static str,
    thumb_names: [&'static str; 2],
    bounds: [f64; 2],
    initially: [f64; 2],
    mode: StandardSliderMode,
    size: StandardSliderSize,
    disabled: bool,
}

const SAMPLES: [Sample; 4] = [
    Sample {
        id: "range-slider-demo-continuous",
        name: "連續區間",
        thumb_names: ["連續下限", "連續上限"],
        bounds: [0., 100.],
        initially: [25., 75.],
        mode: CONTINUOUS,
        size: StandardSliderSize::ExtraSmall,
        disabled: false,
    },
    Sample {
        id: "range-slider-demo-discrete",
        name: "階梯區間",
        thumb_names: ["階梯下限", "階梯上限"],
        bounds: [-10., 90.],
        initially: [15., 65.],
        mode: DISCRETE,
        size: StandardSliderSize::Small,
        disabled: false,
    },
    Sample {
        id: "range-slider-demo-equal",
        name: "相等端點",
        thumb_names: ["相等下限", "相等上限"],
        bounds: [0., 100.],
        initially: [50., 50.],
        mode: CONTINUOUS,
        size: StandardSliderSize::ExtraSmall,
        disabled: false,
    },
    Sample {
        id: "range-slider-demo-disabled",
        name: "停用區間",
        thumb_names: ["停用下限", "停用上限"],
        bounds: [0., 100.],
        initially: [25., 75.],
        mode: CONTINUOUS,
        size: StandardSliderSize::Medium,
        disabled: true,
    },
];

impl Sample {
    fn labels(&self, values: [f64; 2]) -> [String; 2] {
        values.map(|value| value_label(value, self.bounds[0] < 0.))
    }
}

#[derive(Clone)]
pub enum RangeSliderDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    Collapse,
    Change(usize, RangeSliderChange),
}

pub(crate) struct RangeSliderDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    viewport: (f32, f32),
    values: [[f64; 2]; SAMPLES.len()],
    requests: [[usize; 2]; SAMPLES.len()],
    engaged: Option<(usize, RangeSliderThumb)>,
    collapses: usize,
}

impl Default for RangeSliderDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            viewport: (960., 880.),
            values: SAMPLES.each_ref().map(|sample| sample.initially),
            requests: [[0; 2]; SAMPLES.len()],
            engaged: None,
            collapses: 0,
        }
    }
}

impl RangeSliderDemo {
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
        // A held thumb takes precedence over the most recent keyboard focus.
        let next = [pressed, input.focused]
            .into_iter()
            .flatten()
            .find_map(thumb_target);
        if self.engaged == next {
            return false;
        }
        self.engaged = next;
        true
    }

    pub(crate) fn update(&mut self, event: RangeSliderDemoEvent) {
        use RangeSliderDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            Collapse if self.visible => {
                let [lower, upper] = self.values[0];
                if lower != upper {
                    self.values[0] = [lower.midpoint(upper); 2];
                    self.collapses += 1;
                }
            }
            Change(index, change) if self.visible => self.change(index, change),
            Collapse | Change(..) => {}
        }
    }

    fn change(&mut self, index: usize, change: RangeSliderChange) {
        let Some(sample) = SAMPLES.get(index).filter(|sample| !sample.disabled) else {
            return;
        };
        let [lo, hi] = sample.bounds;
        let values = [change.lower, change.upper];
        let thumb = match change.thumb {
            RangeSliderThumb::Lower => 0,
            RangeSliderThumb::Upper => 1,
        };
        if change.lower <= change.upper
            && values.iter().all(|value| (lo..=hi).contains(value))
            && values[1 - thumb] == self.values[index][1 - thumb]
            && values[thumb] != self.values[index][thumb]
        {
            self.values[index] = values;
            self.requests[index][thumb] += 1;
        }
    }

    fn engaged_thumb(&self, index: usize) -> Option<RangeSliderThumb> {
        self.engaged
            .filter(|(sample, _)| *sample == index)
            .map(|(_, thumb)| thumb)
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "範圍滑桿",
                ButtonKind::Outlined,
                false,
                Message::RangeSliderDemo(RangeSliderDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn sample(&self, theme: &Theme, index: usize, sample: &Sample) -> View<Message> {
        let [lower, upper] = self.values[index];
        let [lower_label, upper_label] = sample.labels([lower, upper]);
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let slider = theme
            .range_slider(
                RangeSliderOptions {
                    id: sample.id,
                    name: sample.name,
                    lower_name: sample.thumb_names[0],
                    upper_name: sample.thumb_names[1],
                    min: sample.bounds[0],
                    max: sample.bounds[1],
                    lower,
                    upper,
                    mode: sample.mode,
                    size: sample.size,
                    direction,
                    disabled: sample.disabled,
                    engaged: self.engaged_thumb(index),
                    lower_label: Some(lower_label.as_str()),
                    upper_label: Some(upper_label.as_str()),
                },
                move |change| Message::RangeSliderDemo(RangeSliderDemoEvent::Change(index, change)),
            )
            .expect("valid Gallery range slider");
        // The label, padding and focus ring fit here without shifting the rail.
        let label_room = typography::LABEL_LARGE.line_height_px() * self.text_scale() + 24. + 9.;
        let clearance = View::new(
            Node::new(Role::Group, "").styled(Style::column().height(Length::Px(label_room))),
        );
        let align = if self.rtl { Align::End } else { Align::Start };
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-row", sample.id))
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .align(align)
                        .gap(4.),
                ),
        )
        .children([
            theme
                .text(&format!("{}-caption", sample.id), sample.name)
                .without_interaction(None),
            clearance,
            slider,
        ])
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use RangeSliderDemoEvent::*;
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
                .tonal_button(BACK, "返回", Message::RangeSliderDemo(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            toggle(THEME, "深色", dark, Message::Theme),
            toggle(
                RTL,
                "右起",
                self.rtl,
                Message::RangeSliderDemo(ToggleDirection),
            ),
            toggle(
                LARGE_TEXT,
                "大字",
                self.large_text,
                Message::RangeSliderDemo(ToggleLargeText),
            ),
            theme
                .tonal_button(COLLAPSE, "合併連續端點", Message::RangeSliderDemo(Collapse))
                .accessible_activation(),
        ]);
        let mut scroll_style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .gap(24.)
            .padding(6.)
            .clip(true);
        scroll_style.max_width = Length::Px(856.);
        let scroller = View::new(
            Node::new(Role::Group, "範圍滑桿範例")
                .id(SCROLL)
                .styled(scroll_style),
        )
        .accessibility_scroll_pages()
        .children(
            SAMPLES
                .iter()
                .enumerate()
                .map(|(index, sample)| self.sample(theme, index, sample)),
        );

        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("range-slider-demo-route")
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
            theme.heading("range-slider-demo-title", "範圍滑桿"),
            controls,
            scroller,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme, dark: bool) -> serde_json::Value {
        let color = |surface::Color(r, g, b, a)| [r, g, b, a];
        let colors = theme.colors;
        let samples: Vec<_> = SAMPLES
            .iter()
            .enumerate()
            .map(|(index, sample)| {
                let intervals = match sample.mode {
                    StandardSliderMode::Continuous { .. } => None,
                    StandardSliderMode::Discrete { intervals } => Some(intervals),
                };
                let engaged = self.engaged_thumb(index).map(|thumb| match thumb {
                    RangeSliderThumb::Lower => "lower",
                    RangeSliderThumb::Upper => "upper",
                });
                serde_json::json!({
                    "id": sample.id, "min": sample.bounds[0], "max": sample.bounds[1],
                    "intervals": intervals, "values": self.values[index],
                    "labels": sample.labels(self.values[index]), "engaged": engaged,
                    "requests": self.requests[index], "disabled": sample.disabled,
                })
            })
            .collect();
        serde_json::json!({
            "visible": self.visible, "rtl": self.rtl, "text_scale": self.text_scale(),
            "dark": dark, "width": self.viewport.0, "height": self.viewport.1,
            "collapses": self.collapses, "samples": samples,
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

fn thumb_target(id: &str) -> Option<(usize, RangeSliderThumb)> {
    let (pair, suffix) = id.rsplit_once('-')?;
    let index = SAMPLES
        .iter()
        .position(|sample| sample.id == pair && !sample.disabled)?;
    let thumb = match suffix {
        "lower" => RangeSliderThumb::Lower,
        "upper" => RangeSliderThumb::Upper,
        _ => return None,
    };
    Some((index, thumb))
}

fn value_label(value: f64, signed: bool) -> String {
    let rounded = (value * 10.).round() / 10.;
    if rounded == 0. {
        "0％".into()
    } else if rounded < 0. {
        format!("−{}％", -rounded)
    } else if signed {
        format!("+{rounded}％")
    } else {
        format!("{rounded}％")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallery;
    use surface::{Rect, TextMeasurer};
    use surface_app::{EditKey, Event, Runtime};

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
        draw(&mut runtime, WIDE);
        runtime
    }

    fn open(runtime: &mut Runtime<Gallery>, viewport: (f32, f32)) {
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

    fn key(runtime: &mut Runtime<Gallery>, key: EditKey) {
        assert!(runtime
            .dispatch(Event::Edit {
                key,
                extend: false,
                by_word: false
            })
            .unwrap());
    }

    #[test]
    fn old_slider_routes_keep_their_ids_and_range_has_its_own_named_focus_cycle() {
        let mut runtime = home();
        for (show, old_control, back) in [
            ("slider-demo-show", "slider-demo-xs", "slider-demo-back"),
            (
                "slider-forms-demo-show",
                "slider-forms-demo-centered-discrete",
                "slider-forms-demo-back",
            ),
        ] {
            assert!(runtime.activate(show).unwrap());
            draw(&mut runtime, WIDE);
            assert!(runtime.dom().by_id(old_control).is_some());
            assert!(runtime.dom().by_id(SAMPLES[0].id).is_none());
            assert!(runtime.activate(back).unwrap());
            draw(&mut runtime, WIDE);
            assert_eq!(runtime.focused(), Some(show));
        }

        open(&mut runtime, WIDE);
        let scroll = bounds(&runtime, SCROLL);
        let disabled_index = SAMPLES.len() - 1;
        for (index, sample) in SAMPLES.iter().enumerate() {
            let group = runtime.dom().by_id(sample.id).unwrap();
            assert_eq!(runtime.dom().get(group).unwrap().name(), sample.name);
            assert!(contains(scroll, bounds(&runtime, sample.id)));
            for (i, suffix) in ["-lower", "-upper"].into_iter().enumerate() {
                let key = format!("{}{suffix}", sample.id);
                let thumb = runtime
                    .dom()
                    .get(runtime.dom().by_id(&key).unwrap())
                    .unwrap();
                assert_eq!(thumb.name(), sample.thumb_names[i]);
                assert_eq!(thumb.role(), Role::Slider);
                assert_eq!(thumb.is_tab_stop(), index != disabled_index);
                assert_eq!(bounds(&runtime, &key).width, 48.);
            }
        }
        assert_eq!(bounds(&runtime, "range-slider-demo-equal-active").width, 0.);
        let route = [THEME, RTL, LARGE_TEXT, COLLAPSE]
            .into_iter()
            .map(str::to_owned)
            .chain(SAMPLES[..disabled_index].iter().flat_map(|sample| {
                ["lower", "upper"].map(|suffix| format!("{}-{suffix}", sample.id))
            }))
            .chain([BACK.to_owned()]);
        for expected in route {
            tab(&mut runtime, WIDE, false);
            assert_eq!(runtime.focused(), Some(expected.as_str()));
        }
        tab(&mut runtime, WIDE, true);
        assert_eq!(runtime.focused(), Some("range-slider-demo-equal-upper"));
        assert!(runtime.activate(BACK).unwrap());
        draw(&mut runtime, WIDE);
        assert_eq!(runtime.focused(), Some(SHOW));
        assert!(runtime.dom().by_id("range-slider-demo-route").is_none());
    }

    #[test]
    fn signed_steps_control_just_one_end_and_external_collapse_keeps_request_counts() {
        let mut runtime = home();
        open(&mut runtime, WIDE);
        assert!(runtime
            .set_value("range-slider-demo-discrete-lower", 28.)
            .unwrap());
        assert!(runtime
            .set_value("range-slider-demo-discrete-upper", -9.)
            .unwrap());
        assert!(!runtime
            .set_value("range-slider-demo-discrete-lower", 65.)
            .unwrap());
        assert!(runtime
            .set_value("range-slider-demo-discrete-upper", 88.)
            .unwrap());
        assert!(runtime
            .set_value("range-slider-demo-disabled-lower", 80.)
            .is_err());
        assert!(runtime.activate(COLLAPSE).unwrap());
        assert!(runtime.activate(COLLAPSE).unwrap());
        draw(&mut runtime, WIDE);
        let values = &runtime.app().range_slider_demo;
        assert_eq!(
            values.values,
            [[50., 50.], [27.5, 90.], [50., 50.], [25., 75.]]
        );
        assert_eq!(values.requests, [[0, 0], [1, 2], [0, 0], [0, 0]]);
        assert_eq!(values.collapses, 1);
        assert_eq!(
            bounds(&runtime, "range-slider-demo-continuous-active").width,
            0.
        );
        assert_eq!(value_label(-10., true), "−10％");
        assert_eq!(value_label(27.5, true), "+27.5％");
        assert_eq!(value_label(27.5, false), "27.5％");
    }

    #[test]
    fn compact_dark_rtl_focus_and_scroll_keep_the_rail_label_and_disabled_targets_inside() {
        let mut runtime = home();
        open(&mut runtime, COMPACT);
        for id in [THEME, RTL, LARGE_TEXT] {
            assert!(runtime.activate(id).unwrap());
            draw(&mut runtime, COMPACT);
        }
        assert!(runtime.app().dark && runtime.app().range_slider_demo.rtl);
        assert_eq!(runtime.app().range_slider_demo.text_scale(), 2.);
        let rail_key = "range-slider-demo-continuous-rail";
        let rail = bounds(&runtime, rail_key);
        for id in [COLLAPSE, "range-slider-demo-continuous-lower"] {
            tab(&mut runtime, COMPACT, false);
            assert_eq!(runtime.focused(), Some(id));
        }
        assert_eq!(bounds(&runtime, rail_key), rail);
        let lower = bounds(&runtime, "range-slider-demo-continuous-lower-handle");
        let upper = bounds(&runtime, "range-slider-demo-continuous-upper-handle");
        let ring = bounds(&runtime, "range-slider-demo-continuous-lower-focus");
        let label = bounds(&runtime, "range-slider-demo-continuous-lower-label");
        let caption = bounds(&runtime, "range-slider-demo-continuous-caption");
        assert_eq!((lower.width, upper.width), (2., 4.));
        assert!(lower.x > upper.x);
        assert_eq!(ring.width, 12.);
        assert!(label.height >= 64.);
        assert!(caption.y + caption.height <= label.y);
        assert!((label.y + label.height + 4. - ring.y).abs() < 0.001);
        assert!(runtime
            .dom()
            .by_id("range-slider-demo-continuous-upper-label")
            .is_none());

        let scroller = bounds(&runtime, SCROLL);
        for region in [lower, upper, ring, label] {
            assert!(
                contains(scroller, region),
                "{region:?} exceeds {scroller:?}"
            );
        }
        key(&mut runtime, EditKey::Left);
        draw(&mut runtime, COMPACT);
        assert_eq!(runtime.app().range_slider_demo.values[0], [30., 75.]);
        assert_eq!(runtime.app().range_slider_demo.requests[0], [1, 0]);

        let id = "range-slider-demo-disabled-lower";
        let initial = bounds(&runtime, id);
        runtime
            .dispatch(Event::Scroll {
                x: scroller.x + scroller.width / 2.,
                y: scroller.y + scroller.height / 2.,
                delta_x: 0.,
                delta_y: 1024.,
            })
            .unwrap();
        draw(&mut runtime, COMPACT);
        let revealed = bounds(&runtime, id);
        assert!(revealed.y < initial.y);
        assert!(contains(scroller, revealed));
        assert_eq!(runtime.app().range_slider_demo.requests[3], [0, 0]);
    }

    #[test]
    fn pointer_on_the_shared_rail_engages_only_the_nearer_real_target() {
        let mut runtime = home();
        open(&mut runtime, WIDE);
        let rail = bounds(&runtime, "range-slider-demo-continuous-rail");
        let (x, y) = (rail.x + rail.width * 0.875, rail.y + rail.height / 2.);
        runtime.dispatch(Event::PointerDown { x, y }).unwrap();
        draw(&mut runtime, WIDE);
        let model = &runtime.app().range_slider_demo;
        assert_eq!(model.engaged, Some((0, RangeSliderThumb::Upper)));
        assert_eq!(model.requests[0], [0, 1]);
        assert!((model.values[0][1] - 87.5).abs() < 0.001);
        assert_eq!(model.values[0][0], 25.);
        assert!(runtime
            .dom()
            .by_id("range-slider-demo-continuous-upper-label")
            .is_some());
        runtime.dispatch(Event::PointerUp { x, y }).unwrap();
        draw(&mut runtime, WIDE);
        assert_eq!(runtime.app().range_slider_demo.requests[0], [0, 1]);
    }
}
