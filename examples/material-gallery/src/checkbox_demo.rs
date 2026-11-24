use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, FrameInput, FramePress, View};
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, CheckValue, CheckboxMotion, CheckboxOptions,
    ChoiceState, Theme,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "checkbox-demo-show";
const BACK: &str = "checkbox-demo-back";
const ERROR_HINT: &str = "checkbox-demo-error-hint";
const IDS: [&str; 5] = [
    "checkbox-demo-all",
    "checkbox-demo-notes",
    "checkbox-demo-attachments",
    "checkbox-demo-locked",
    "checkbox-demo-consent",
];
const LABELS: [&str; IDS.len()] = [
    "同步全部",
    "筆記",
    "附件",
    "已鎖定的專案",
    "允許在行動網路上同步大型附件",
];
const ALL: usize = 0;
const NOTES: usize = 1;
const ATTACHMENTS: usize = 2;
const LOCKED: usize = 3;
const CONSENT: usize = 4;

#[derive(Clone)]
pub enum CheckboxDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    Set(usize, CheckValue),
}

pub(crate) struct CheckboxDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    values: [bool; 2],
    consent: bool,
    requests: usize,
    motions: [CheckboxMotion; IDS.len()],
}

impl Default for CheckboxDemo {
    fn default() -> Self {
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            values: [true, false],
            consent: false,
            requests: 0,
            motions: std::array::from_fn(|_| CheckboxMotion::new()),
        }
    }
}

impl CheckboxDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.motions = std::array::from_fn(|_| CheckboxMotion::new());
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

    fn values(&self) -> [CheckValue; IDS.len()] {
        let value = |checked| {
            if checked {
                CheckValue::Checked
            } else {
                CheckValue::Unchecked
            }
        };
        let [notes, attachments] = self.values;
        let all = if notes == attachments {
            value(notes)
        } else {
            CheckValue::Mixed
        };
        [
            all,
            value(notes),
            value(attachments),
            CheckValue::Mixed,
            value(self.consent),
        ]
    }

    pub(crate) fn update(&mut self, event: CheckboxDemoEvent) {
        use CheckboxDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            Set(index, value) => {
                self.requests += 1;
                let checked = match value {
                    CheckValue::Checked => true,
                    CheckValue::Unchecked => false,
                    CheckValue::Mixed => return,
                };
                match index {
                    ALL => self.values.fill(checked),
                    CONSENT => self.consent = checked,
                    NOTES | ATTACHMENTS => self.values[index - NOTES] = checked,
                    _ => {}
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
            let values = self.values();
            for (index, motion) in self.motions.iter_mut().enumerate() {
                let next = motion.advance(
                    options(index, values[index], direction, scale),
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
                "核取方塊範例",
                ButtonKind::Outlined,
                false,
                Message::CheckboxDemo(CheckboxDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use CheckboxDemoEvent::*;
        let event = Message::CheckboxDemo;
        let direction = self.direction();
        let scale = self.text_scale();
        let values = self.values();
        let rtl = direction == LayoutDirection::RightToLeft;
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
            setting("checkbox-demo-theme", "深色", dark, Message::Theme),
            setting("checkbox-demo-rtl", "右起", rtl, event(ToggleDirection)),
            setting(
                "checkbox-demo-large-text",
                "大字",
                self.large_text,
                event(ToggleLargeText),
            ),
        ]);
        let render = |index| {
            theme.checkbox_with_options(
                options(index, values[index], direction, scale),
                &self.motions[index],
                move |value| event(Set(index, value)),
            )
        };
        let documents = View::new(
            Node::new(Role::Group, "同步內容")
                .id("checkbox-demo-documents")
                .styled(Style::column().width(Length::Percent(1.)).gap(4.)),
        )
        .children((0..=LOCKED).map(render));
        let error_help = (!self.consent).then(|| {
            let text = "請勾選後再繼續。";
            View::new(
                Node::new(Role::Text, text)
                    .id(ERROR_HINT)
                    .text(text)
                    .text_alignment(if rtl {
                        surface::TextAlignment::Right
                    } else {
                        surface::TextAlignment::Left
                    })
                    .styled(
                        Style::default()
                            .with_text_style(typography::BODY_MEDIUM)
                            .foreground(theme.colors.error),
                    )
                    .shrink_to_fit_width(),
            )
        });

        let mut column = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(0.))
            .grow(1.)
            .padding(8.)
            .gap(8.)
            .clip(true);
        column.max_width = Length::Px(480.);
        let preview = View::new(
            Node::new(Role::Group, "同步設定")
                .id("checkbox-demo-scroll")
                .styled(column),
        )
        .accessibility_scroll_pages()
        .children([documents, render(CONSENT)].into_iter().chain(error_help));

        View::new(
            Node::new(Role::Window, "Material gallery")
                .id("checkbox-demo")
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
            theme.heading("checkbox-demo-title", "核取方塊"),
            controls,
            preview,
        ])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        let values = self.values().map(|value| match value {
            CheckValue::Unchecked => "unchecked",
            CheckValue::Checked => "checked",
            CheckValue::Mixed => "mixed",
        });
        serde_json::json!({
            "visible": self.visible,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "values": values,
            "requests": self.requests,
            "palette": {
                "surface": theme.colors.surface,
                "on_surface": theme.colors.on_surface,
                "on_surface_variant": theme.colors.on_surface_variant,
                "primary": theme.colors.primary,
                "on_primary": theme.colors.on_primary,
                "secondary": theme.colors.secondary,
                "error": theme.colors.error,
                "on_error": theme.colors.on_error,
            },
        })
    }
}

fn options(
    index: usize,
    value: CheckValue,
    direction: LayoutDirection,
    text_scale: f32,
) -> CheckboxOptions<'static> {
    let error = index == CONSENT && value != CheckValue::Checked;
    CheckboxOptions {
        id: IDS[index],
        label: LABELS[index],
        name: None,
        value,
        disabled: index == LOCKED,
        error,
        error_description: error.then_some(ERROR_HINT),
        direction,
        text_scale,
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

    fn node<'a>(runtime: &'a Runtime<Gallery>, id: &str) -> &'a Node {
        runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
    }

    #[test]
    fn checked_children_control_the_parent_without_changing_the_locked_row() {
        let mut runtime = route();
        assert_eq!(runtime.focused(), Some(BACK));
        assert!(node(&runtime, IDS[ALL]).is_mixed());
        for id in [
            "checkbox-demo-theme",
            "checkbox-demo-rtl",
            "checkbox-demo-large-text",
            IDS[ALL],
        ] {
            runtime
                .dispatch(Event::FocusNext { reverse: false })
                .unwrap();
            render(&mut runtime, 960., 800.);
            assert_eq!(runtime.focused(), Some(id));
            assert_eq!(runtime.app().checkbox_demo.requests, 0);
        }
        runtime.dispatch(Event::Activate).unwrap();
        render(&mut runtime, 960., 800.);
        assert_eq!(runtime.app().checkbox_demo.values, [true, true]);
        assert_eq!(runtime.app().checkbox_demo.requests, 1);
        assert!(!node(&runtime, IDS[ALL]).is_mixed());

        assert!(runtime.activate(IDS[NOTES]).unwrap());
        render(&mut runtime, 960., 800.);
        assert_eq!(runtime.app().checkbox_demo.values, [false, true]);
        assert!(node(&runtime, IDS[ALL]).is_mixed());
        assert!(node(&runtime, IDS[LOCKED]).is_mixed());
        assert!(node(&runtime, IDS[LOCKED]).is_disabled());
        assert!(matches!(runtime.activate(IDS[LOCKED]),
            Err(surface_app::AppError::MissingControl(id)) if id == IDS[LOCKED]));
        assert_eq!(runtime.app().checkbox_demo.requests, 2);

        assert!(runtime.activate(BACK).unwrap());
        render(&mut runtime, 960., 1000.);
        assert_eq!(runtime.focused(), Some(SHOW));
    }

    #[test]
    fn compact_rtl_error_has_visible_help_and_a_reachable_wrapped_label() {
        let mut runtime = route();
        for id in [
            "checkbox-demo-theme",
            "checkbox-demo-rtl",
            "checkbox-demo-large-text",
        ] {
            assert!(runtime.activate(id).unwrap());
        }
        render(&mut runtime, 320., 640.);
        let id = IDS[CONSENT];
        let hit = bounds(&runtime, id);
        let shape = bounds(&runtime, "checkbox-demo-consent-box");
        let label = bounds(&runtime, "checkbox-demo-consent-label");
        let help = bounds(&runtime, ERROR_HINT);
        assert_eq!(hit.intersect(label), label);
        assert_eq!(hit.intersect(shape), shape);
        assert_eq!((shape.width, shape.height), (18., 18.));
        assert!(hit.height >= 48. && label.height > 48.);
        assert!(label.x + label.width < shape.x);
        assert!(help.y >= hit.y + hit.height);
        assert!(node(&runtime, id).is_invalid());
        assert_eq!(
            node(&runtime, id).description_text_target(),
            Some(ERROR_HINT)
        );

        assert!(runtime.activate(id).unwrap());
        render(&mut runtime, 320., 640.);
        assert!(!node(&runtime, id).is_invalid());
        assert_eq!(node(&runtime, id).description_text_target(), None);
        assert!(runtime.dom().by_id(ERROR_HINT).is_none());
        assert_eq!(runtime.focused(), Some(id));
    }
}
