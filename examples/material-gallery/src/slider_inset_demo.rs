use std::sync::OnceLock;

use surface::{Align, Length, Node, Role, Style};
use surface_app::{FrameInput, View};
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChoiceState, SliderInsetIcon,
    StandardSliderMode, StandardSliderOptions, StandardSliderSize, Theme, VectorIcon,
    VerticalStandardSliderPlacement,
};

use crate::{scroll_field_demo::control_row, Message};

const SHOW: &str = "slider-inset-demo-show";
const BACK: &str = "slider-inset-demo-back";
const THEME: &str = "slider-inset-demo-theme";
const RTL: &str = "slider-inset-demo-rtl";
const LARGE_TEXT: &str = "slider-inset-demo-large-text";
const SCROLL: &str = "slider-inset-demo-scroll";
const HORIZONTAL_COUNT: usize = 4;
const ROUTE_PADDING: f32 = 16.;
const SCROLL_PADDING: f32 = 6.;
const MAX_SCROLL_WIDTH: f32 = 856.;
const COLUMN_GAP: f32 = 24.;
const LEADING_ROOM: f32 = 8.;
const LABEL_ROOM: f32 = 120.;
const VERTICAL_HEIGHT: f32 = 240.;
const SHORT_WIDTH: f32 = 88.;

#[derive(Clone, Copy)]
enum Axis {
    Horizontal,
    Short,
    Vertical,
}

struct Sample {
    id: &'static str,
    name: &'static str,
    size: StandardSliderSize,
    axis: Axis,
    initially: f64,
    disabled: bool,
}

const SAMPLES: [Sample; 8] = [
    Sample {
        id: "slider-inset-demo-horizontal-medium",
        name: "中型水平音量",
        size: StandardSliderSize::Medium,
        axis: Axis::Horizontal,
        initially: 65.,
        disabled: false,
    },
    Sample {
        id: "slider-inset-demo-horizontal-large",
        name: "大型水平音量",
        size: StandardSliderSize::Large,
        axis: Axis::Horizontal,
        initially: 0.,
        disabled: false,
    },
    Sample {
        id: "slider-inset-demo-horizontal-xl",
        name: "特大水平音量",
        size: StandardSliderSize::ExtraLarge,
        axis: Axis::Horizontal,
        initially: 65.,
        disabled: false,
    },
    Sample {
        id: "slider-inset-demo-short",
        name: "短軌道音量",
        size: StandardSliderSize::Medium,
        axis: Axis::Short,
        initially: 50.,
        disabled: false,
    },
    Sample {
        id: "slider-inset-demo-vertical-medium",
        name: "中型垂直音量",
        size: StandardSliderSize::Medium,
        axis: Axis::Vertical,
        initially: 65.,
        disabled: false,
    },
    Sample {
        id: "slider-inset-demo-vertical-large",
        name: "大型垂直音量",
        size: StandardSliderSize::Large,
        axis: Axis::Vertical,
        initially: 0.,
        disabled: false,
    },
    Sample {
        id: "slider-inset-demo-vertical-xl",
        name: "特大垂直音量",
        size: StandardSliderSize::ExtraLarge,
        axis: Axis::Vertical,
        initially: 65.,
        disabled: false,
    },
    Sample {
        id: "slider-inset-demo-disabled",
        name: "停用垂直音量",
        size: StandardSliderSize::Large,
        axis: Axis::Vertical,
        initially: 65.,
        disabled: true,
    },
];

impl Sample {
    fn target_width(&self) -> f32 {
        match self.size {
            StandardSliderSize::Medium => 52.,
            StandardSliderSize::Large => 68.,
            StandardSliderSize::ExtraLarge => 108.,
            _ => unreachable!("valid inset slider size"),
        }
    }
}

#[derive(Clone)]
pub enum SliderInsetDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    Change(usize, f64),
}

pub(crate) struct SliderInsetDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    viewport: (f32, f32),
    values: [f64; SAMPLES.len()],
    requests: [usize; SAMPLES.len()],
    engaged: Option<usize>,
}

impl Default for SliderInsetDemo {
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

impl SliderInsetDemo {
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

    pub(crate) fn update(&mut self, event: SliderInsetDemoEvent) {
        use SliderInsetDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            Change(index, value)
                if self.visible
                    && SAMPLES.get(index).is_some_and(|sample| !sample.disabled)
                    && (0.0..=100.0).contains(&value)
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
                "滑桿內嵌圖示",
                ButtonKind::Outlined,
                false,
                Message::SliderInsetDemo(SliderInsetDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn column_width(&self, columns: usize) -> f32 {
        let route = (self.viewport.0 - 2. * ROUTE_PADDING).min(MAX_SCROLL_WIDTH);
        (route - 2. * SCROLL_PADDING - (columns - 1) as f32 * COLUMN_GAP) / columns as f32
    }

    fn sample(&self, theme: &Theme, index: usize, columns: usize) -> View<Message> {
        let sample = &SAMPLES[index];
        let label = value_label(self.values[index]);
        let options = StandardSliderOptions {
            id: sample.id,
            name: sample.name,
            min: 0.,
            max: 100.,
            value: self.values[index],
            mode: StandardSliderMode::Continuous { keyboard_step: 5. },
            size: sample.size,
            direction: if self.rtl {
                LayoutDirection::RightToLeft
            } else {
                LayoutDirection::LeftToRight
            },
            disabled: sample.disabled,
            engaged: self.engaged == Some(index),
            value_label: Some(&label),
        };
        let changed =
            move |value| Message::SliderInsetDemo(SliderInsetDemoEvent::Change(index, value));
        let icon = SliderInsetIcon {
            icon: speaker(false),
            at_minimum: Some(speaker(true)),
        };
        let contents = if matches!(sample.axis, Axis::Vertical) {
            let label_room = (self.column_width(columns) - sample.target_width() - LEADING_ROOM)
                .clamp(0., LABEL_ROOM);
            let slider = theme
                .vertical_standard_slider_with_inset_icon(
                    options,
                    VerticalStandardSliderPlacement {
                        height: VERTICAL_HEIGHT,
                        label_room,
                    },
                    icon,
                    changed,
                )
                .expect("valid Gallery vertical inset slider");
            let lane = Node::new(Role::Group, "").styled(Style::row());
            let lane = if self.rtl { lane.reverse_row() } else { lane };
            let space = |width| {
                View::new(
                    Node::new(Role::Group, "").styled(Style::column().width(Length::Px(width))),
                )
            };
            View::new(lane).children([space(LEADING_ROOM), slider, space(label_room)])
        } else {
            let slider = theme
                .standard_slider_with_inset_icon(options, icon, changed)
                .expect("valid Gallery horizontal inset slider");
            let width = if matches!(sample.axis, Axis::Short) {
                Length::Px(SHORT_WIDTH)
            } else {
                Length::Percent(1.)
            };
            let room = typography::LABEL_LARGE.line_height_px() * self.text_scale() + 24. + 9.;
            View::new(Node::new(Role::Group, "").styled(Style::column().width(width))).children([
                View::new(
                    Node::new(Role::Group, "").styled(Style::column().height(Length::Px(room))),
                ),
                slider,
            ])
        };
        let column = Style::column().align(Align::Center).gap(8.);
        let column = if columns > 1 {
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
            contents,
        ])
    }

    fn sample_row(&self, theme: &Theme, start: usize, columns: usize) -> View<Message> {
        let row = Node::new(Role::Group, "").styled(
            Style::row()
                .width(Length::Percent(1.))
                .align(Align::Start)
                .gap(COLUMN_GAP),
        );
        let row = if self.rtl { row.reverse_row() } else { row };
        View::new(row)
            .children((start..start + columns).map(|index| self.sample(theme, index, columns)))
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use SliderInsetDemoEvent::*;
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
                .tonal_button(BACK, "返回", Message::SliderInsetDemo(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            toggle(THEME, "深色", dark, Message::Theme),
            toggle(
                RTL,
                "右起",
                self.rtl,
                Message::SliderInsetDemo(ToggleDirection),
            ),
            toggle(
                LARGE_TEXT,
                "大字",
                self.large_text,
                Message::SliderInsetDemo(ToggleLargeText),
            ),
        ]);
        let horizontal_columns = if self.viewport.0 >= 720. { 2 } else { 1 };
        let vertical_columns = if self.viewport.0 >= 900. {
            4
        } else {
            horizontal_columns
        };
        let mut samples = vec![theme.heading("slider-inset-demo-horizontal-title", "水平滑桿")];
        samples.extend(
            (0..HORIZONTAL_COUNT)
                .step_by(horizontal_columns)
                .map(|start| self.sample_row(theme, start, horizontal_columns)),
        );
        samples.push(theme.heading("slider-inset-demo-vertical-title", "垂直滑桿"));
        samples.extend(
            (HORIZONTAL_COUNT..SAMPLES.len())
                .step_by(vertical_columns)
                .map(|start| self.sample_row(theme, start, vertical_columns)),
        );
        let mut scroll_style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .gap(24.)
            .padding(SCROLL_PADDING)
            .clip(true);
        scroll_style.max_width = Length::Px(MAX_SCROLL_WIDTH);
        let scroll = View::new(
            Node::new(Role::Group, "內嵌圖示範例")
                .id(SCROLL)
                .styled(scroll_style),
        )
        .accessibility_scroll_pages()
        .children(samples);
        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("slider-inset-demo-route")
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
            theme.heading("slider-inset-demo-title", "滑桿內嵌圖示"),
            controls,
            scroll,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme, dark: bool) -> serde_json::Value {
        let color = |surface::Color(r, g, b, a)| [r, g, b, a];
        let colors = theme.colors;
        let samples: Vec<_> = SAMPLES.iter().enumerate().map(|(index, sample)| {
            let size = match sample.size {
                StandardSliderSize::Medium => "medium",
                StandardSliderSize::Large => "large",
                StandardSliderSize::ExtraLarge => "extra_large",
                _ => unreachable!("valid inset slider size"),
            };
            serde_json::json!({
                "id": sample.id, "value": self.values[index], "label": value_label(self.values[index]),
                "size": size, "vertical": matches!(sample.axis, Axis::Vertical),
                "minimum_glyph": self.values[index] == 0., "requests": self.requests[index],
                "engaged": self.engaged == Some(index), "disabled": sample.disabled,
            })
        }).collect();
        serde_json::json!({
            "visible": self.visible, "rtl": self.rtl, "text_scale": self.text_scale(),
            "dark": dark, "width": self.viewport.0, "height": self.viewport.1, "samples": samples,
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

fn value_label(value: f64) -> String {
    let rounded = (value * 10.).round() / 10.;
    format!("{rounded}％")
}

fn speaker(muted: bool) -> &'static VectorIcon {
    const BODY: [[f32; 2]; 6] = [
        [3., 9.],
        [7., 9.],
        [13., 4.],
        [13., 20.],
        [7., 15.],
        [3., 15.],
    ];
    const SOUND: [[f32; 2]; 4] = [[16., 7.], [21., 11.], [21., 13.], [16., 17.]];
    const MUTE: [[f32; 2]; 12] = [
        [15., 9.],
        [17., 7.],
        [19., 9.],
        [21., 7.],
        [23., 9.],
        [21., 11.],
        [23., 13.],
        [21., 15.],
        [19., 13.],
        [17., 15.],
        [15., 13.],
        [17., 11.],
    ];
    static SOUND_ICON: OnceLock<VectorIcon> = OnceLock::new();
    static MUTE_ICON: OnceLock<VectorIcon> = OnceLock::new();
    let (cache, mark) = if muted {
        (&MUTE_ICON, MUTE.as_slice())
    } else {
        (&SOUND_ICON, SOUND.as_slice())
    };
    cache.get_or_init(|| {
        VectorIcon::from_contours([BODY.as_slice().iter().copied(), mark.iter().copied()])
            .expect("valid Gallery speaker glyph")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallery;
    use surface::{Color, ExtendedDisplayList, ExtendedPaintOp, PathFill, Rect, TextMeasurer};
    use surface_app::{EditKey, Event, Runtime};

    const HOME: (f32, f32) = (960., 1900.);
    const WIDE: (f32, f32) = (960., 1040.);
    const COMPACT: (f32, f32) = (320., 880.);
    const ACTIVE: &str = "inset-icon-active";
    const INACTIVE: &str = "inset-icon-inactive";

    struct Fonts;

    impl TextMeasurer for Fonts {
        fn measure(&mut self, text: &str, size: f32, max_width: Option<f32>) -> (f32, f32) {
            let natural = text.chars().count() as f32 * size;
            let limit = max_width.unwrap_or(natural).max(1.);
            (natural.min(limit), size * (natural / limit).ceil().max(1.))
        }
    }

    fn draw(runtime: &mut Runtime<Gallery>, viewport: (f32, f32)) -> ExtendedDisplayList {
        runtime
            .render_extended(viewport.0, viewport.1, &mut Fonts)
            .unwrap()
    }

    fn home() -> Runtime<Gallery> {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        runtime
            .apply(Message::Section(crate::slider_demo::SECTION))
            .unwrap();
        draw(&mut runtime, HOME);
        runtime
    }

    fn open(runtime: &mut Runtime<Gallery>, viewport: (f32, f32)) -> ExtendedDisplayList {
        let entry = runtime.dom().by_id(SHOW).unwrap();
        let geometry = &runtime.computed().unwrap().nodes[entry.index()];
        assert!(contains(geometry.clip, geometry.bounds), "{geometry:?}");
        assert!(runtime.activate(SHOW).unwrap());
        let paint = draw(runtime, viewport);
        assert_eq!(runtime.focused(), Some(BACK));
        paint
    }

    fn bounds(runtime: &Runtime<Gallery>, key: &str) -> Rect {
        let id = runtime.dom().by_id(key).unwrap();
        runtime.computed().unwrap().nodes[id.index()].bounds
    }

    fn part(runtime: &Runtime<Gallery>, sample: &Sample, suffix: &str) -> Rect {
        bounds(runtime, &format!("{}-{suffix}", sample.id))
    }

    fn contains(outer: Rect, inner: Rect) -> bool {
        let clipped = outer.intersect(inner);
        (clipped.x - inner.x).abs() < 0.001
            && (clipped.y - inner.y).abs() < 0.001
            && (clipped.width - inner.width).abs() < 0.001
            && (clipped.height - inner.height).abs() < 0.001
    }

    fn has_area(rect: Rect) -> bool {
        rect.width > 0. && rect.height > 0.
    }

    fn glyph(list: &ExtendedDisplayList, rect: Rect) -> &PathFill {
        let mut paths = list.ops().iter().filter_map(|op| match op {
            ExtendedPaintOp::FillPath(path) if path.clip() == rect => Some(path),
            _ => None,
        });
        let path = paths.next().expect("visible artwork is painted");
        assert!(paths.next().is_none());
        assert!(has_area(path.rect()) && contains(rect, path.rect()));
        path
    }

    fn tab(runtime: &mut Runtime<Gallery>, viewport: (f32, f32)) -> ExtendedDisplayList {
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        draw(runtime, viewport)
    }

    fn key(runtime: &mut Runtime<Gallery>, input: EditKey) {
        assert!(runtime
            .dispatch(Event::Edit {
                key: input,
                extend: false,
                by_word: false
            })
            .unwrap());
    }

    #[test]
    fn both_orientations_show_m_l_xl_and_keep_short_track_stop_and_names() {
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
        let list = open(&mut runtime, WIDE);
        let scroll = bounds(&runtime, SCROLL);
        let frame = runtime.accessibility_frame().unwrap();
        let colors = Theme::light().colors;
        for sample in &SAMPLES {
            let id = runtime.dom().by_id(sample.id).unwrap();
            let node = frame.node(id).unwrap().node;
            assert_eq!(node.name(), sample.name);
            assert_eq!(
                node.is_vertical_slider(),
                matches!(sample.axis, Axis::Vertical)
            );
            assert_eq!(node.is_disabled(), sample.disabled);
            assert_eq!(
                node.slider_value_text(),
                Some(value_label(sample.initially).as_str())
            );
            assert_eq!(frame.children(id).count(), 0);
            let target = bounds(&runtime, sample.id);
            assert!(contains(scroll, target));
            let active = part(&runtime, sample, ACTIVE);
            let inactive = part(&runtime, sample, INACTIVE);
            for suffix in [
                ACTIVE,
                INACTIVE,
                "inset-icon-active-glyph",
                "inset-icon-inactive-glyph",
            ] {
                let key = format!("{}-{suffix}", sample.id);
                assert!(frame.node(runtime.dom().by_id(&key).unwrap()).is_none());
            }
            let stop = part(&runtime, sample, "stop");
            assert_eq!((stop.width, stop.height), (4., 4.));
            if matches!(sample.axis, Axis::Short) {
                assert_eq!(target.width, SHORT_WIDTH);
                assert_eq!(part(&runtime, sample, "rail").width, SHORT_WIDTH - 24.);
                assert!(!has_area(active) && !has_area(inactive));
                assert!(contains(part(&runtime, sample, "inactive"), stop));
                continue;
            }
            let (rect, track, other, ink) = if sample.initially == 0. {
                (inactive, "inactive", active, colors.on_secondary_container)
            } else {
                (active, "active", inactive, colors.on_primary)
            };
            assert!(!has_area(other));
            let side = if sample.size == StandardSliderSize::ExtraLarge {
                32.
            } else {
                24.
            };
            assert_eq!((rect.width, rect.height), (side, side));
            assert!(contains(part(&runtime, sample, track), rect));
            assert!(!has_area(stop.intersect(rect)));
            let painted = glyph(&list, rect);
            if sample.disabled {
                let Color(r, g, b, _) = colors.inverse_on_surface;
                assert_eq!(painted.color(), Color(r, g, b, 97));
                assert!(frame.adjustment_target(id).is_none());
            } else {
                assert_eq!(painted.color(), ink);
                assert!(frame.adjustment_target(id).is_some());
            }
        }
        assert!(runtime.activate(BACK).unwrap());
        draw(&mut runtime, HOME);
        assert_eq!(runtime.focused(), Some(SHOW));
    }

    #[test]
    fn keyboard_relocates_the_regular_glyph_then_changes_only_the_minimum_contour() {
        let mut runtime = home();
        open(&mut runtime, WIDE);
        let sample = &SAMPLES[0];
        for expected in [THEME, RTL, LARGE_TEXT, sample.id] {
            tab(&mut runtime, WIDE);
            assert_eq!(runtime.focused(), Some(expected));
        }
        let hit = bounds(&runtime, sample.id);
        let rail = part(&runtime, sample, "rail");
        let stop = part(&runtime, sample, "stop");
        let positive = draw(&mut runtime, WIDE);
        let active = part(&runtime, sample, ACTIVE);
        let original_points = glyph(&positive, active).points().len();
        assert!(runtime.set_value(sample.id, 5.).unwrap());
        let relocated = draw(&mut runtime, WIDE);
        let inactive = part(&runtime, sample, INACTIVE);
        assert!(!has_area(part(&runtime, sample, ACTIVE)));
        assert_eq!(glyph(&relocated, inactive).points().len(), original_points);
        assert_eq!(stop.x - inactive.x - inactive.width, 6.);
        key(&mut runtime, EditKey::Home);
        let muted = draw(&mut runtime, WIDE);
        assert_eq!(runtime.app().slider_inset_demo.values[0], 0.);
        assert_eq!(runtime.app().slider_inset_demo.requests[0], 2);
        assert_eq!(part(&runtime, sample, INACTIVE), inactive);
        assert_ne!(glyph(&muted, inactive).points().len(), original_points);
        assert_eq!(bounds(&runtime, sample.id), hit);
        assert_eq!(part(&runtime, sample, "rail"), rail);
        assert_eq!(part(&runtime, sample, "stop"), stop);
        key(&mut runtime, EditKey::End);
        let restored = draw(&mut runtime, WIDE);
        assert_eq!(runtime.app().slider_inset_demo.values[0], 100.);
        assert_eq!(runtime.app().slider_inset_demo.requests[0], 3);
        assert_eq!(
            glyph(&restored, part(&runtime, sample, ACTIVE))
                .points()
                .len(),
            original_points
        );
        assert_eq!(runtime.focused(), Some(sample.id));
    }

    #[test]
    fn compact_dark_rtl_scroll_reveals_the_active_vertical_glyph_and_large_focus_label() {
        let mut runtime = home();
        open(&mut runtime, COMPACT);
        for control in [THEME, RTL, LARGE_TEXT] {
            assert!(runtime.activate(control).unwrap());
            draw(&mut runtime, COMPACT);
        }
        let model = &runtime.app().slider_inset_demo;
        assert!(runtime.app().dark && model.rtl);
        assert_eq!(model.text_scale(), 2.);
        let scroll = bounds(&runtime, SCROLL);
        let h_medium = &SAMPLES[0];
        let paint = tab(&mut runtime, COMPACT);
        assert_eq!(runtime.focused(), Some(h_medium.id));
        let icon = part(&runtime, h_medium, ACTIVE);
        let rail = part(&runtime, h_medium, "rail");
        let handle = part(&runtime, h_medium, "handle");
        assert!(icon.x > handle.x);
        assert_eq!(rail.x + rail.width - icon.x - icon.width, 6.);
        assert_eq!(glyph(&paint, icon).color(), Theme::dark().colors.on_primary);
        for suffix in ["label", "focus"] {
            assert!(contains(scroll, part(&runtime, h_medium, suffix)));
        }

        let v_medium = &SAMPLES[4];
        let original = bounds(&runtime, v_medium.id);
        for sample in &SAMPLES[1..=4] {
            tab(&mut runtime, COMPACT);
            assert_eq!(runtime.focused(), Some(sample.id));
        }
        let visible = draw(&mut runtime, COMPACT);
        let target = bounds(&runtime, v_medium.id);
        let focus = part(&runtime, v_medium, "focus");
        let label = part(&runtime, v_medium, "label");
        let icon = part(&runtime, v_medium, ACTIVE);
        let handle = part(&runtime, v_medium, "handle");
        assert!(target.y < original.y);
        assert!(icon.y > handle.y);
        assert_eq!(label.x + label.width + 4., focus.x);
        assert!(label.height >= 64.);
        assert!(contains(label, part(&runtime, v_medium, "label-text")));
        for region in [target, focus, label, icon] {
            assert!(contains(scroll, region), "{region:?} exceeds {scroll:?}");
        }
        assert_eq!(
            glyph(&visible, icon).color(),
            Theme::dark().colors.on_primary
        );

        let disabled = &SAMPLES[7];
        let original = bounds(&runtime, disabled.id);
        runtime
            .dispatch(Event::Scroll {
                x: scroll.x + scroll.width / 2.,
                y: scroll.y + scroll.height / 2.,
                delta_x: 0.,
                delta_y: 4096.,
            })
            .unwrap();
        draw(&mut runtime, COMPACT);
        let target = bounds(&runtime, disabled.id);
        assert!(target.y < original.y && contains(scroll, target));
        assert!(runtime.set_value(disabled.id, 0.).is_err());
        assert_eq!(runtime.app().slider_inset_demo.requests[7], 0);
    }
}
