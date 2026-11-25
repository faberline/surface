use std::{collections::BTreeMap, sync::OnceLock};

use surface::{Align, Length, Node, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, typography, ButtonAppearance, ButtonIcon, ButtonKind, ButtonOptions,
    ButtonShape, ButtonSize, ChoiceState, Theme, VectorIcon,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "button-demo-show";
const BACK: &str = "button-demo-back";
const THEME: &str = "button-demo-theme";
const RTL: &str = "button-demo-rtl";
const LARGE_TEXT: &str = "button-demo-large-text";
const SCROLL: &str = "button-demo-scroll";
const TITLE: &str = "按鈕範例";
const OUTER_PADDING: f32 = 16.;
const SCROLL_PADDING: f32 = 6.;
const MAX_SCROLL_WIDTH: f32 = 856.;

#[derive(Clone, Copy)]
enum Art {
    Leading,
    Trailing,
}

#[derive(Clone, Copy)]
struct Sample {
    id: &'static str,
    name: &'static str,
    appearance: ButtonAppearance,
    size: ButtonSize,
    shape: ButtonShape,
    checked: Option<bool>,
    icon: Option<Art>,
    disabled: bool,
}

impl Sample {
    const fn command(id: &'static str, name: &'static str, appearance: ButtonAppearance) -> Self {
        Self {
            id,
            name,
            appearance,
            size: ButtonSize::Small,
            shape: ButtonShape::Round,
            checked: None,
            icon: None,
            disabled: false,
        }
    }

    const fn sized(id: &'static str, name: &'static str, size: ButtonSize) -> Self {
        Self {
            size,
            icon: Some(Art::Leading),
            ..Self::command(id, name, ButtonAppearance::Filled)
        }
    }

    const fn toggle(
        id: &'static str,
        name: &'static str,
        appearance: ButtonAppearance,
        shape: ButtonShape,
    ) -> Self {
        Self {
            checked: Some(false),
            shape,
            ..Self::command(id, name, appearance)
        }
    }
}

const SIZES: [Sample; 5] = [
    Sample::sized("button-demo-size-xs", "極小", ButtonSize::ExtraSmall),
    Sample::sized("button-demo-size-s", "小型", ButtonSize::Small),
    Sample::sized("button-demo-size-m", "中型", ButtonSize::Medium),
    Sample::sized("button-demo-size-l", "大型", ButtonSize::Large),
    Sample::sized("button-demo-size-xl", "特大", ButtonSize::ExtraLarge),
];
const COMMANDS: [Sample; 5] = [
    Sample::command(
        "button-demo-command-elevated",
        "建立",
        ButtonAppearance::Elevated,
    ),
    Sample::command(
        "button-demo-command-filled",
        "儲存",
        ButtonAppearance::Filled,
    ),
    Sample::command("button-demo-command-tonal", "分享", ButtonAppearance::Tonal),
    Sample::command(
        "button-demo-command-outlined",
        "詳細資料",
        ButtonAppearance::Outlined,
    ),
    Sample::command("button-demo-command-text", "略過", ButtonAppearance::Text),
];
const TOGGLES: [Sample; 4] = [
    Sample::toggle(
        "button-demo-toggle-elevated",
        "提醒",
        ButtonAppearance::Elevated,
        ButtonShape::Round,
    ),
    Sample::toggle(
        "button-demo-toggle-filled",
        "收藏",
        ButtonAppearance::Filled,
        ButtonShape::Round,
    ),
    Sample::toggle(
        "button-demo-toggle-tonal",
        "追蹤",
        ButtonAppearance::Tonal,
        ButtonShape::Square,
    ),
    Sample::toggle(
        "button-demo-toggle-outlined",
        "固定",
        ButtonAppearance::Outlined,
        ButtonShape::Square,
    ),
];
const DETAILS: [Sample; 5] = [
    Sample {
        size: ButtonSize::ExtraSmall,
        icon: Some(Art::Leading),
        ..Sample::command("button-demo-leading", "新增相片", ButtonAppearance::Filled)
    },
    Sample {
        icon: Some(Art::Trailing),
        ..Sample::command("button-demo-trailing", "加入清單", ButtonAppearance::Tonal)
    },
    Sample {
        icon: Some(Art::Leading),
        disabled: true,
        ..Sample::command(
            "button-demo-disabled",
            "停用操作",
            ButtonAppearance::Outlined,
        )
    },
    Sample {
        checked: Some(true),
        disabled: true,
        ..Sample::toggle(
            "button-demo-disabled-toggle",
            "停用切換",
            ButtonAppearance::Outlined,
            ButtonShape::Round,
        )
    },
    Sample {
        size: ButtonSize::Medium,
        icon: Some(Art::Leading),
        ..Sample::command(
            "button-demo-wrapped",
            "下載播放清單以便離線收聽",
            ButtonAppearance::Filled,
        )
    },
];
const GROUPS: [(&str, &str, &[Sample]); 4] = [
    ("button-demo-sizes", "尺寸", &SIZES),
    ("button-demo-commands", "操作", &COMMANDS),
    ("button-demo-toggles", "切換", &TOGGLES),
    ("button-demo-details", "圖示與長標籤", &DETAILS),
];

fn samples() -> impl Iterator<Item = &'static Sample> {
    GROUPS.iter().flat_map(|(_, _, row)| row.iter())
}

#[derive(Clone)]
pub enum ButtonDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    Activate(&'static str),
}

pub(crate) struct ButtonDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    viewport: (f32, f32),
    counts: BTreeMap<&'static str, usize>,
    checked: BTreeMap<&'static str, bool>,
    last: Option<&'static str>,
}

impl Default for ButtonDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            viewport: (960., 1160.),
            counts: samples().map(|sample| (sample.id, 0)).collect(),
            checked: samples()
                .filter_map(|sample| Some((sample.id, sample.checked?)))
                .collect(),
            last: None,
        }
    }
}

impl ButtonDemo {
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

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let viewport = (width, height);
        if viewport == self.viewport {
            return false;
        }
        self.viewport = viewport;
        self.visible
    }

    pub(crate) fn update(&mut self, event: ButtonDemoEvent) {
        use ButtonDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            Activate(id)
                if self.visible && samples().any(|sample| sample.id == id && !sample.disabled) =>
            {
                if let Some(count) = self.counts.get_mut(id) {
                    *count += 1;
                }
                if let Some(value) = self.checked.get_mut(id) {
                    *value = !*value;
                }
                self.last = Some(id);
            }
            Activate(..) => {}
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                TITLE,
                ButtonKind::Outlined,
                false,
                Message::ButtonDemo(ButtonDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
            .suppress_repeated_activation()
    }

    fn group(&self, theme: &Theme, id: &str, label: &str, row: &[Sample]) -> View<Message> {
        let style = Style::row()
            .width(Length::Percent(1.))
            .wrap(true)
            .gap(16.)
            .align(Align::Center);
        let actions = Node::new(Role::Group, "").styled(style);
        let actions = if self.rtl {
            actions.reverse_row()
        } else {
            actions
        };
        let available =
            (self.viewport.0 - 2. * OUTER_PADDING).min(MAX_SCROLL_WIDTH) - 2. * SCROLL_PADDING;
        View::new(
            Node::new(Role::Group, "")
                .id(id)
                .styled(Style::column().width(Length::Percent(1.)).gap(12.)),
        )
        .children([
            theme.heading(&format!("{id}-title"), label),
            View::new(actions).children(row.iter().map(|sample| {
                let mut options = ButtonOptions::new(sample.id, sample.name, available.max(48.));
                options.appearance = sample.appearance;
                options.size = sample.size;
                options.shape = sample.shape;
                options.direction = if self.rtl {
                    LayoutDirection::RightToLeft
                } else {
                    LayoutDirection::LeftToRight
                };
                options.disabled = sample.disabled;
                options.checked = self.checked.get(sample.id).copied();
                options.icon = sample.icon.map(|side| match side {
                    Art::Leading => ButtonIcon::Leading(artwork()),
                    Art::Trailing => ButtonIcon::Trailing(artwork()),
                });
                options.text_scale = self.text_scale();
                theme
                    .button_with_options(
                        options,
                        Message::ButtonDemo(ButtonDemoEvent::Activate(sample.id)),
                    )
                    .expect("valid Gallery common button")
            })),
        ])
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use ButtonDemoEvent::*;
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
                .tonal_button(BACK, "返回", Message::ButtonDemo(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation()
                .suppress_repeated_activation(),
            toggle(THEME, "深色", dark, Message::Theme),
            toggle(RTL, "右起", self.rtl, Message::ButtonDemo(ToggleDirection)),
            toggle(
                LARGE_TEXT,
                "大字",
                self.large_text,
                Message::ButtonDemo(ToggleLargeText),
            ),
        ]);
        let status = self
            .last
            .and_then(|id| samples().find(|sample| sample.id == id))
            .map_or_else(
                || "尚未操作".to_owned(),
                |sample| format!("最近操作：{}", sample.name),
            );
        let status = View::new(
            Node::new(Role::Status, "")
                .id("button-demo-status")
                .text(status)
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .with_text_style(typography::BODY_MEDIUM)
                        .foreground(theme.colors.on_surface),
                ),
        );
        let mut scroll_style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .gap(28.)
            .padding(SCROLL_PADDING)
            .clip(true);
        scroll_style.max_width = Length::Px(MAX_SCROLL_WIDTH);
        let scroll = View::new(
            Node::new(Role::Group, "按鈕範例捲動區")
                .id(SCROLL)
                .styled(scroll_style),
        )
        .accessibility_scroll_pages()
        .children(
            GROUPS
                .into_iter()
                .map(|(id, label, row)| self.group(theme, id, label, row)),
        );
        View::new(
            Node::new(Role::Window, TITLE)
                .id("button-demo-route")
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Percent(1.))
                        .align(Align::Start)
                        .padding(OUTER_PADDING)
                        .gap(12.)
                        .background(theme.background)
                        .foreground(theme.on_background)
                        .clip(true),
                ),
        )
        .children([
            theme.heading("button-demo-title", TITLE),
            controls,
            status,
            scroll,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme, dark: bool) -> serde_json::Value {
        let color = |surface::Color(r, g, b, a)| [r, g, b, a];
        let colors = theme.colors;
        let names: BTreeMap<_, _> = samples().map(|s| (s.id, s.name)).collect();
        serde_json::json!({
            "visible": self.visible, "rtl": self.rtl, "dark": dark, "text_scale": self.text_scale(),
            "width": self.viewport.0, "height": self.viewport.1, "names": names,
            "counts": self.counts, "last": self.last, "checked": self.checked,
            "palette": {
                "background": color(theme.background), "primary": color(colors.primary),
                "on_primary": color(colors.on_primary), "secondary_container": color(colors.secondary_container),
                "on_secondary_container": color(colors.on_secondary_container), "secondary": color(colors.secondary),
                "on_secondary": color(colors.on_secondary), "surface_container": color(colors.surface_container),
                "surface_container_low": color(colors.surface_container_low), "on_surface": color(colors.on_surface),
                "on_surface_variant": color(colors.on_surface_variant), "outline_variant": color(colors.outline_variant),
                "inverse_surface": color(colors.inverse_surface), "inverse_on_surface": color(colors.inverse_on_surface),
                "shadow": color(colors.shadow),
            },
        })
    }
}

fn artwork() -> &'static VectorIcon {
    const PLUS: [[f32; 2]; 12] = [
        [10., 3.],
        [14., 3.],
        [14., 10.],
        [21., 10.],
        [21., 14.],
        [14., 14.],
        [14., 21.],
        [10., 21.],
        [10., 14.],
        [3., 14.],
        [3., 10.],
        [10., 10.],
    ];
    static ICON: OnceLock<VectorIcon> = OnceLock::new();
    ICON.get_or_init(|| VectorIcon::from_contours([PLUS]).expect("valid Gallery plus icon"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallery;
    use surface::{
        Color, ExtendedDisplayList, ExtendedPaintOp, NodeId, PaintOp, Rect, TextMeasurer,
    };
    use surface_app::{Event, Key, KeyPress, Modifiers, Runtime};

    const HOME: (f32, f32) = (960., 2100.);
    const WIDE: (f32, f32) = (960., 1160.);
    const COMPACT: (f32, f32) = (320., 880.);

    struct Fonts;

    impl TextMeasurer for Fonts {
        fn measure(&mut self, text: &str, font: f32, width: Option<f32>) -> (f32, f32) {
            let natural = text.chars().count() as f32 * font;
            let limit = width.unwrap_or(natural).max(font);
            (
                natural.min(limit),
                (natural / limit).ceil().max(1.) * font * 1.5,
            )
        }
    }

    fn draw(runtime: &mut Runtime<Gallery>, viewport: (f32, f32)) -> ExtendedDisplayList {
        runtime
            .render_extended(viewport.0, viewport.1, &mut Fonts)
            .unwrap()
    }

    fn home() -> Runtime<Gallery> {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        runtime.set_reduced_motion(true);
        runtime.apply(Message::Section(SECTION)).unwrap();
        draw(&mut runtime, HOME);
        runtime
    }

    fn id(runtime: &Runtime<Gallery>, name: &str) -> NodeId {
        runtime.dom().by_id(name).unwrap()
    }

    fn bounds(runtime: &Runtime<Gallery>, name: &str) -> Rect {
        runtime.computed().unwrap().nodes[id(runtime, name).index()].bounds
    }

    fn part(runtime: &Runtime<Gallery>, sample: &Sample, suffix: &str) -> Rect {
        bounds(runtime, &format!("{}-{suffix}", sample.id))
    }

    fn contains(outer: Rect, inner: Rect) -> bool {
        let clipped = outer.intersect(inner);
        [
            clipped.x - inner.x,
            clipped.y - inner.y,
            clipped.width - inner.width,
            clipped.height - inner.height,
        ]
        .into_iter()
        .all(|delta| delta.abs() < 0.002)
    }

    fn open(runtime: &mut Runtime<Gallery>, viewport: (f32, f32)) -> ExtendedDisplayList {
        let entry = &runtime.computed().unwrap().nodes[id(runtime, SHOW).index()];
        assert!(contains(entry.clip, entry.bounds), "{entry:?}");
        assert!(runtime.activate(SHOW).unwrap());
        let paint = draw(runtime, viewport);
        assert_eq!(runtime.focused(), Some(BACK));
        paint
    }

    fn fill(paint: &ExtendedDisplayList, area: Rect, ink: Color) -> bool {
        paint.ops().iter().any(|op| matches!(op,
            ExtendedPaintOp::Legacy(PaintOp::Quad { rect, color, .. }) if *rect == area && *color == ink))
    }

    fn tab(runtime: &mut Runtime<Gallery>, viewport: (f32, f32), expected: &str) {
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        draw(runtime, viewport);
        assert_eq!(runtime.focused(), Some(expected));
    }

    fn key(runtime: &mut Runtime<Gallery>, key: Key, repeat: bool) {
        let focus = runtime
            .accessibility_frame()
            .unwrap()
            .focus_target(id(runtime, runtime.focused().unwrap()))
            .unwrap();
        let press = KeyPress::new(key, Modifiers::default()).with_repeat(repeat);
        assert!(runtime
            .accessibility_key(&focus, &press, Event::Activate)
            .unwrap());
        draw(runtime, WIDE);
    }

    #[test]
    fn wide_scene_exposes_five_sizes_and_style_palettes_with_stable_action_names() {
        let mut runtime = home();
        for old in ["icon-button-demo-show", "slider-inset-demo-show"] {
            assert!(runtime.dom().by_id(old).is_some());
        }
        let paint = open(&mut runtime, WIDE);
        let frame = runtime.accessibility_frame().unwrap();
        let scroll = bounds(&runtime, SCROLL);
        let sizes = SIZES.map(|sample| part(&runtime, &sample, "face"));
        assert_eq!(sizes.map(|rect| rect.height), [32., 40., 56., 96., 136.]);
        let icons = SIZES.map(|sample| part(&runtime, &sample, "icon"));
        assert_eq!(icons.map(|rect| rect.width), [20., 20., 24., 32., 40.]);
        assert_eq!(icons.map(|rect| rect.height), icons.map(|rect| rect.width));
        let gaps = SIZES.map(|sample| {
            let art = part(&runtime, &sample, "icon");
            part(&runtime, &sample, "label").x - art.x - art.width
        });
        assert_eq!(gaps, [4., 8., 8., 12., 16.]);
        for [left, right] in sizes.array_windows() {
            assert!(left.x + left.width < right.x);
            assert_eq!(left.y + left.height / 2., right.y + right.height / 2.);
        }
        let colors = Theme::light().colors;
        for (sample, color) in COMMANDS.iter().zip([
            colors.surface_container_low,
            colors.primary,
            colors.secondary_container,
            Color::TRANSPARENT,
            Color::TRANSPARENT,
        ]) {
            let face = part(&runtime, sample, "face");
            if color != Color::TRANSPARENT {
                assert!(fill(&paint, face, color), "{}", sample.id);
            } else {
                assert!(!paint.ops().iter().any(|op| matches!(op, ExtendedPaintOp::Legacy(PaintOp::Quad { rect, .. }) if *rect == face)));
            }
        }
        for sample in samples() {
            let action_id = id(&runtime, sample.id);
            let node = frame.node(action_id).unwrap().node;
            assert_eq!(node.role(), Role::Button);
            assert_eq!(node.name(), sample.name);
            assert_eq!(node.is_checked(), sample.checked);
            assert_eq!(node.is_disabled(), sample.disabled);
            assert_eq!(frame.children(action_id).count(), 0);
            let hit = bounds(&runtime, sample.id);
            assert!(contains(scroll, hit), "{}: {hit:?}", sample.id);
            assert!(hit.width >= 48. && hit.height >= 48.);
            for suffix in ["face", "label", "focus"] {
                let child = format!("{}-{suffix}", sample.id);
                assert!(frame.node(id(&runtime, &child)).is_none(), "{child}");
            }
        }
        assert_eq!(part(&runtime, &DETAILS[0], "icon").width, 20.);
        let leading = part(&runtime, &DETAILS[0], "icon");
        let leading_label = part(&runtime, &DETAILS[0], "label");
        assert_eq!(leading_label.x - (leading.x + leading.width), 4.);
        let trailing = part(&runtime, &DETAILS[1], "icon");
        let trailing_label = part(&runtime, &DETAILS[1], "label");
        assert_eq!(trailing.x - (trailing_label.x + trailing_label.width), 8.);
    }

    #[test]
    fn toggles_change_independently_and_disabled_cancel_and_repeat_send_no_extra_action() {
        let mut runtime = home();
        open(&mut runtime, WIDE);
        for expected in [THEME, RTL, LARGE_TEXT]
            .into_iter()
            .chain(SIZES.into_iter().chain(COMMANDS).map(|sample| sample.id))
        {
            tab(&mut runtime, WIDE, expected);
        }
        for (index, sample) in TOGGLES.iter().enumerate() {
            tab(&mut runtime, WIDE, sample.id);
            let before = bounds(&runtime, sample.id);
            for repeat in [false, true, true] {
                key(
                    &mut runtime,
                    if index % 2 == 0 {
                        Key::Space
                    } else {
                        Key::Enter
                    },
                    repeat,
                );
            }
            let app = &runtime.app().button_demo;
            assert_eq!(app.counts[sample.id], 1);
            assert!(app.checked[sample.id]);
            assert_eq!(app.last, Some(sample.id));
            assert_eq!(bounds(&runtime, sample.id), before);
            let focus = runtime.accessibility_frame().unwrap();
            let node = focus.node(id(&runtime, sample.id)).unwrap().node;
            assert_eq!(node.name(), sample.name);
            assert_eq!(node.is_checked(), Some(true));
            if let Some(next) = TOGGLES.get(index + 1) {
                assert!(!app.checked[next.id]);
            }
        }
        let small = bounds(&runtime, SIZES[0].id);
        let x = small.x + small.width / 2.;
        let y = small.y + 1.;
        assert!(y < part(&runtime, &SIZES[0], "face").y);
        for event in [
            Event::PointerDown { x, y },
            Event::PointerUp { x: 0., y: 0. },
        ] {
            runtime.dispatch(event).unwrap();
        }
        assert_eq!(runtime.app().button_demo.counts[SIZES[0].id], 0);
        draw(&mut runtime, WIDE);
        for event in [Event::PointerDown { x, y }, Event::PointerUp { x, y }] {
            runtime.dispatch(event).unwrap();
        }
        assert_eq!(runtime.app().button_demo.counts[SIZES[0].id], 1);
        for sample in DETAILS.iter().filter(|sample| sample.disabled) {
            assert!(runtime.activate(sample.id).is_err());
            assert_eq!(runtime.app().button_demo.counts[sample.id], 0);
        }
        assert!(runtime.app().button_demo.checked[DETAILS[3].id]);
        draw(&mut runtime, WIDE);
        assert!(runtime.activate(BACK).unwrap());
        draw(&mut runtime, HOME);
        assert_eq!(runtime.focused(), Some(SHOW));
    }

    #[test]
    fn compact_dark_rtl_reveals_icons_and_a_full_large_label_in_authored_tab_order() {
        let mut runtime = home();
        open(&mut runtime, COMPACT);
        for control in [THEME, RTL, LARGE_TEXT] {
            assert!(runtime.activate(control).unwrap());
            draw(&mut runtime, COMPACT);
        }
        let scroll = bounds(&runtime, SCROLL);
        let initial = bounds(&runtime, DETAILS[4].id);
        assert!(!contains(scroll, initial));
        for sample in samples().filter(|sample| !sample.disabled) {
            tab(&mut runtime, COMPACT, sample.id);
        }
        let app = &runtime.app().button_demo;
        assert!(runtime.app().dark && app.rtl);
        assert_eq!(app.text_scale(), 2.);
        assert!(bounds(&runtime, DETAILS[4].id).y < initial.y);
        for (sample, gap, trailing) in [
            (&DETAILS[0], 4., false),
            (&DETAILS[1], 8., true),
            (&DETAILS[4], 8., false),
        ] {
            let icon = part(&runtime, sample, "icon");
            let label = part(&runtime, sample, "label");
            let physical_gap = if trailing {
                label.x - (icon.x + icon.width)
            } else {
                icon.x - (label.x + label.width)
            };
            assert_eq!(physical_gap, gap, "{}", sample.id);
            assert!(contains(part(&runtime, sample, "face"), label));
        }
        let long = &DETAILS[4];
        let label = part(&runtime, long, "label");
        assert_eq!(
            label.height,
            2. * typography::TITLE_MEDIUM.line_height_px() * 2.
        );
        assert!(contains(scroll, label));
        let frame = runtime.accessibility_frame().unwrap();
        assert_eq!(
            frame.node(id(&runtime, long.id)).unwrap().node.name(),
            long.name
        );
        assert!(runtime
            .dispatch(Event::Scroll {
                x: scroll.x + scroll.width / 2.,
                y: scroll.y + scroll.height / 2.,
                delta_x: 0.,
                delta_y: 64.,
            })
            .unwrap());
        let paint = draw(&mut runtime, COMPACT);
        let ring = part(&runtime, long, "focus");
        assert!(contains(scroll, ring), "{ring:?} exceeds {scroll:?}");
        assert!(paint.ops().iter().any(|op| matches!(op,
            ExtendedPaintOp::Legacy(PaintOp::Stroke { rect, color, width, .. })
                if *rect == ring && *color == Theme::dark().colors.secondary && *width == 3.)));
        assert_eq!(runtime.focused(), Some(long.id));
    }
}
