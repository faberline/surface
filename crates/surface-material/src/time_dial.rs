//! Material 3 Dial variants and their Input alternative in one modal.

mod baseline;
mod hand_motion;
mod polar;

pub use baseline::time_dial_baseline_layout;
pub use hand_motion::DialHandMotion;

use std::rc::Rc;

use surface::{
    Align, Color, FillContour, Insets, Length, Node, Rect, Role, Style, TextScaling, TextStyle,
};
use surface_app::{EditKey, FocusNavigation, Key, KeyPress, PointDrag, Roving, View};

use crate::{
    date_picker::{button, disabled_ink, text, TARGET},
    shape,
    time_input::{
        keyboard_icon, TimeInput, TimePickerStyle, BASELINE_PANEL_PADDING, PANEL_MARGIN,
        PANEL_PADDING,
    },
    typography,
    vector_icon::VectorIconSize,
    ClockFormat, DialEvent, DialMode, DialPoint, DialState, Theme, TimeField, TimeInputOptions,
    TimeInputPresentation, VectorIcon,
};

const FACE: f32 = 256.;
const FACE_HANDLE: f32 = 48.;
const VERTICAL_FIELD: f32 = 132.;
const SEPARATOR: f32 = 16.;
const HORIZONTAL_WIDTH: f32 = VERTICAL_FIELD * 2. + SEPARATOR;
const PERIOD_WIDTH: f32 = 56.;
const PERIOD_GAP: f32 = 12.;
const HORIZONTAL_PADDING: f32 = 24.;
const HORIZONTAL_GAP: f32 = 52.;
const BODY_GAP: f32 = 12.;
const FOOTER_GAP: f32 = 8.;

/// Placement of the full 256 dp Dial face in the modal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeDialLayout {
    Vertical,
    Horizontal,
}

/// The localized names of both mode switches and the visible fallback message.
#[derive(Clone, Copy)]
pub struct TimeDialLabels<'a> {
    pub use_input: &'a str,
    pub use_dial: &'a str,
    pub unfit: &'a str,
    /// Name a dial choice for assistive technology, including the field, the
    /// localized digits and the number of choices (12, 24 or 60).
    pub value_name: &'a dyn Fn(TimeField, &str, u8) -> String,
}

/// Configure the shared Input and Dial dialog. Dial is always modal.
#[derive(Clone, Copy)]
pub struct TimeDialOptions<'a> {
    pub input: TimeInputOptions<'a>,
    pub labels: &'a TimeDialLabels<'a>,
    /// Must match the host application's effective text scale.
    pub text_scale: f32,
}

/// Determine which arrangement can display a complete dial with its controls.
/// Use the same inputs here and in [Theme::time_dial], and send the result
/// to [DialEvent::SetDialFits] when the viewport or text size changes.
pub fn time_dial_layout(
    width: f32,
    height: f32,
    text_scale: f32,
    clock: ClockFormat,
) -> Option<TimeDialLayout> {
    if !width.is_finite() || !height.is_finite() {
        return None;
    }
    let scale = effective_scale(text_scale).max(1.);
    let has_period = clock == ClockFormat::TwelveHour;
    let vertical_width = vertical_controls_width(has_period) + 2. * PANEL_PADDING;
    let vertical_height = FACE
        + 2. * BODY_GAP
        + FOOTER_GAP
        + 2. * PANEL_PADDING
        + control_height(scale)
        + footer_height(scale);
    let horizontal_width = HORIZONTAL_WIDTH + HORIZONTAL_GAP + FACE + 2. * HORIZONTAL_PADDING;
    let horizontal_controls = control_height(scale)
        + if has_period {
            period_height(scale) + BODY_GAP
        } else {
            0.
        };
    let horizontal_height =
        FACE.max(horizontal_controls) + 2. * HORIZONTAL_PADDING + FOOTER_GAP + footer_height(scale);
    let fits = |w, h| w + 2. * PANEL_MARGIN <= width && h + 2. * PANEL_MARGIN <= height;
    let horizontal = fits(horizontal_width, horizontal_height);
    if horizontal && width > height {
        Some(TimeDialLayout::Horizontal)
    } else if fits(vertical_width, vertical_height) {
        Some(TimeDialLayout::Vertical)
    } else {
        horizontal.then_some(TimeDialLayout::Horizontal)
    }
}

fn effective_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0. {
        scale
    } else {
        1.
    }
}

fn vertical_controls_width(period: bool) -> f32 {
    HORIZONTAL_WIDTH
        + if period {
            PERIOD_WIDTH + PERIOD_GAP
        } else {
            0.
        }
}

fn control_height(scale: f32) -> f32 {
    28. * scale + field_height(scale) + 20. * scale + 2. * BODY_GAP
}

fn field_height(scale: f32) -> f32 {
    88_f32.max(52. * scale + 12.)
}

fn period_height(scale: f32) -> f32 {
    TARGET.max(24. * scale + 8.)
}

fn footer_height(scale: f32) -> f32 {
    TARGET.max(20. * scale + 8.)
}

impl Theme {
    /// Render a shared Input and Dial dialog from the retained draft.
    ///
    /// Pass events to [DialState::apply] and save only when it emits a commit.
    /// Pass the active Input field to [DialState::observe_input_focus] when
    /// the host observes accepted frame input.
    pub fn time_dial<M: Clone + 'static>(
        &self,
        options: TimeDialOptions<'_>,
        state: &DialState,
        on_event: impl Fn(DialEvent) -> M + 'static,
    ) -> View<M> {
        self.build_time_dial(options, state, None, TimePickerStyle::Expressive, on_event)
    }

    /// Render the dial with a retained selector animation. Pass dial events
    /// through [DialHandMotion::apply] and advance it from the frame callback.
    pub fn time_dial_with_motion<M: Clone + 'static>(
        &self,
        options: TimeDialOptions<'_>,
        state: &DialState,
        motion: &DialHandMotion,
        on_event: impl Fn(DialEvent) -> M + 'static,
    ) -> View<M> {
        self.build_time_dial(
            options,
            state,
            Some(motion),
            TimePickerStyle::Expressive,
            on_event,
        )
    }

    /// Render the Baseline Dial and Baseline Input alternative in one modal.
    ///
    /// Use [time_dial_baseline_layout] for fit and forward editor focus to
    /// [DialState::observe_baseline_input_focus]. The draft handles both modes.
    pub fn time_dial_baseline<M: Clone + 'static>(
        &self,
        options: TimeDialOptions<'_>,
        state: &DialState,
        on_event: impl Fn(DialEvent) -> M + 'static,
    ) -> View<M> {
        self.build_time_dial(options, state, None, TimePickerStyle::Baseline, on_event)
    }

    fn build_time_dial<M: Clone + 'static>(
        &self,
        mut options: TimeDialOptions<'_>,
        state: &DialState,
        motion: Option<&DialHandMotion>,
        style: TimePickerStyle,
        on_event: impl Fn(DialEvent) -> M + 'static,
    ) -> View<M> {
        options.input.presentation = TimeInputPresentation::Modal;
        let emit: Rc<dyn Fn(DialEvent) -> M> = Rc::new(on_event);
        let emit_input = emit.clone();
        Dial {
            input: TimeInput {
                theme: self,
                options: options.input,
                session: state.input(),
                emit: Rc::new(move |event| emit_input(DialEvent::Input(event))),
                editor_fallback_suffix: None,
                style,
            },
            labels: options.labels,
            scale: effective_scale(options.text_scale),
            state,
            motion,
            emit,
        }
        .view()
    }
}

struct Dial<'a, 'b, M> {
    input: TimeInput<'a, 'b, M>,
    labels: &'b TimeDialLabels<'b>,
    scale: f32,
    state: &'a DialState,
    motion: Option<&'a DialHandMotion>,
    emit: Rc<dyn Fn(DialEvent) -> M>,
}

#[derive(Clone, Copy)]
struct HandPaint {
    center: (f32, f32),
    resting_value: Option<u8>,
}

impl<M: Clone + 'static> Dial<'_, '_, M> {
    fn is_baseline(&self) -> bool {
        matches!(self.input.style, TimePickerStyle::Baseline)
    }

    fn view(&self) -> View<M> {
        let options = self.input.options;
        let fit_layout = if self.is_baseline() {
            time_dial_baseline_layout
        } else {
            time_dial_layout
        };
        let fit = fit_layout(
            options.viewport.width,
            options.viewport.height,
            self.scale,
            self.state.input().clock(),
        );
        let layout = fit.filter(|_| self.state.mode() == DialMode::Dial && self.state.dial_fits());
        let (width, padding, body, initial, mode) = if let Some(layout) = layout {
            let (width, padding) = if self.is_baseline() {
                let metrics =
                    baseline::DialMetrics::new(layout, self.scale, self.state.input().clock());
                (metrics.panel_width, BASELINE_PANEL_PADDING)
            } else {
                match layout {
                    TimeDialLayout::Vertical => (
                        vertical_controls_width(self.state.input().period().is_some())
                            + 2. * PANEL_PADDING,
                        PANEL_PADDING,
                    ),
                    TimeDialLayout::Horizontal => (
                        HORIZONTAL_WIDTH + HORIZONTAL_GAP + FACE + 2. * HORIZONTAL_PADDING,
                        HORIZONTAL_PADDING,
                    ),
                }
            };
            (
                width,
                padding,
                self.dial_body(layout)
                    .restore_focus_in_subtree_on_removal(self.input.id("hour-input")),
                "hour-selector",
                DialMode::Input,
            )
        } else {
            let width = (options.viewport.width - 2. * PANEL_MARGIN).max(0.).min(
                if self.state.input().period().is_some() {
                    336.
                } else {
                    264.
                },
            );
            let reason = fit.is_none().then(|| {
                text(
                    self.input.id("unfit"),
                    self.labels.unfit,
                    typography::LABEL_MEDIUM,
                    self.input.theme.colors.on_surface_variant,
                )
            });
            let (width, padding, body) = if self.is_baseline() {
                let width = self.input.baseline_modal_width(self.scale);
                let padding = self.input.baseline_modal_padding(self.scale);
                (
                    width,
                    padding,
                    self.input
                        .baseline_modal_body(width, padding, self.scale, reason),
                )
            } else {
                (width, PANEL_PADDING, self.input.modal_body(width, reason))
            };
            (
                width,
                padding,
                body.restore_focus_in_subtree_on_removal(self.input.id("hour-selector")),
                "hour-input",
                DialMode::Dial,
            )
        };
        let toggle = self.mode_button(mode, fit.is_some() && self.state.dial_fits());
        let mut actions = if self.is_baseline() && layout.is_some() {
            self.input
                .actions_with_height(Some(toggle), footer_height(self.scale.max(1.)))
        } else {
            self.input.actions(Some(toggle))
        };
        if layout.is_some() {
            actions = actions.restore_focus_in_subtree_on_removal(self.input.id("hour-input"));
        }
        self.input.modal_panel(
            width,
            padding,
            body,
            actions,
            self.input
                .id(if options.disabled { "cancel" } else { initial }),
        )
    }

    fn mode_button(&self, mode: DialMode, dial_fits: bool) -> View<M> {
        let (id, name, fallback) = match mode {
            DialMode::Input => ("use-input", self.labels.use_input, "hour-input"),
            DialMode::Dial => (
                "use-dial",
                self.labels.use_dial,
                if self.state.field() == TimeField::Hour {
                    "hour-selector"
                } else {
                    "minute-selector"
                },
            ),
        };
        let colors = self.input.theme.colors;
        let enabled = !self.input.options.disabled && (dial_fits || mode == DialMode::Input);
        let ink = if enabled {
            colors.on_surface_variant
        } else {
            disabled_ink(self.input.theme)
        };
        button(
            Node::new(Role::Button, name)
                .id(self.input.id(id))
                .focus_color(colors.secondary)
                .state_layer_alphas(20, 26, 26)
                .styled(square(TARGET).corners(shape::FULL).state_layer(ink)),
            enabled.then(|| (self.emit)(DialEvent::Show(mode))),
        )
        .children([mode_icon(mode)
            .view(VectorIconSize::Dp24, ink)
            .without_interaction(None)])
        .protect_composition_on_press()
        .focus_on_pointer_release()
        .restore_focus_on_removal(self.input.id(fallback))
    }

    fn dial_body(&self, layout: TimeDialLayout) -> View<M> {
        let controls = self.controls(layout);
        let face = self.face();
        let style = match layout {
            TimeDialLayout::Vertical => Style::column().gap(BODY_GAP),
            TimeDialLayout::Horizontal => Style::row().gap(HORIZONTAL_GAP),
        }
        .align(Align::Center);
        View::new(
            self.input.row(
                Node::new(Role::Group, "")
                    .id(self.input.id("body"))
                    .styled(style),
            ),
        )
        .children([controls, face])
    }

    fn controls(&self, layout: TimeDialLayout) -> View<M> {
        if self.is_baseline() {
            return self.baseline_controls(layout);
        }
        let vertical = layout == TimeDialLayout::Vertical;
        let period = self.state.input().period();
        let width = if vertical {
            vertical_controls_width(period.is_some())
        } else {
            HORIZONTAL_WIDTH
        };
        let title = self.input.title(width);
        let fields = self.fields(None);
        let help = View::new(Node::new(Role::Group, "").styled(Style::column().gap(2.))).children(
            [TimeField::Hour, TimeField::Minute]
                .into_iter()
                .filter_map(|field| self.error(field)),
        );
        let controls = if vertical {
            let row = View::new(
                self.input.row(
                    Node::new(Role::Group, "")
                        .styled(Style::row().gap(PERIOD_GAP).align(Align::Center)),
                ),
            )
            .children(
                std::iter::once(fields)
                    .chain(period.map(|value| self.input.picker_period(value, true, PERIOD_WIDTH))),
            );
            vec![title, row, help]
        } else {
            let period = period.map(|value| self.input.picker_period(value, false, 100.));
            [title, fields, help].into_iter().chain(period).collect()
        };
        let mut style = Style::column().width(Length::Px(width)).gap(BODY_GAP);
        let scale = self.scale.max(1.);
        style.max_height = Length::Px(if vertical {
            control_height(scale)
        } else {
            // The controls can grow beside the face. Reserve enough room
            // for the footer before the controls need to scroll.
            self.input.options.viewport.height
                - 2. * (PANEL_MARGIN + HORIZONTAL_PADDING)
                - FOOTER_GAP
                - footer_height(scale)
        });
        style.clip = true;
        View::new(
            Node::new(Role::Group, "")
                .id(self.input.id("dial-controls"))
                .styled(style),
        )
        .children(controls)
        .accessibility_scroll_pages()
    }

    fn baseline_controls(&self, layout: TimeDialLayout) -> View<M> {
        let vertical = layout == TimeDialLayout::Vertical;
        let metrics = baseline::DialMetrics::new(layout, self.scale, self.state.input().clock());
        let fields = self.fields(Some(&metrics));
        let period = self.state.input().period().map(|value| {
            self.input
                .baseline_period(value, metrics.period, self.scale)
        });
        let mut controls = vec![self.input.title(metrics.controls_width)];
        if vertical {
            let row = View::new(
                self.input.row(
                    Node::new(Role::Group, "")
                        .styled(Style::row().gap(PERIOD_GAP).align(Align::Center)),
                ),
            )
            .children(std::iter::once(fields).chain(period));
            controls.push(row);
        } else {
            controls.push(fields);
            controls.extend(period);
        }
        controls.extend(
            [TimeField::Hour, TimeField::Minute]
                .into_iter()
                .filter_map(|field| self.error(field)),
        );
        let mut style = Style::column()
            .width(Length::Px(metrics.controls_width))
            .gap(BODY_GAP)
            .clip(true);
        let available_height =
            baseline::body_height_available(self.input.options.viewport.height, self.scale);
        let face_row = if vertical { FACE + BODY_GAP } else { 0. };
        style.max_height = Length::Px((available_height - face_row).max(0.));
        View::new(
            Node::new(Role::Group, "")
                .id(self.input.id("dial-controls"))
                .styled(style),
        )
        .children(controls)
        .accessibility_scroll_pages()
    }

    fn fields(&self, baseline: Option<&baseline::DialMetrics>) -> View<M> {
        let mut separator_type = typography::DISPLAY_LARGE_EMPHASIZED;
        if baseline.is_some() {
            separator_type = typography::DISPLAY_LARGE;
            separator_type.font_size *= self.scale;
            separator_type.line_height =
                separator_type.line_height.map(|height| height * self.scale);
        }
        let line_height = separator_type.line_height_px();
        let separator = View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Px(
                        baseline.map_or(SEPARATOR, |metrics| metrics.separator_width),
                    ))
                    .align(Align::Center)
                    .justify(Align::Center),
            ),
        )
        .children([View::new(
            Node::new(Role::Text, "")
                .id(self.input.id("separator"))
                .text(":")
                .text_scaling(TextScaling::Fixed)
                .styled(
                    Style::column()
                        .with_text_style(separator_type)
                        .height(Length::Px(line_height))
                        .foreground(self.input.theme.colors.on_surface),
                ),
        )])
        .without_interaction(None);
        View::new(
            self.input.row(
                Node::new(Role::RadioGroup, "")
                    .id(self.input.id("dial-fields"))
                    .styled(Style::row().align(Align::Center)),
            ),
        )
        .children([
            self.selector(TimeField::Hour, baseline),
            separator,
            self.selector(TimeField::Minute, baseline),
        ])
    }

    fn caption(&self, field: TimeField) -> &str {
        match field {
            TimeField::Hour => self.input.options.labels.hour,
            TimeField::Minute => self.input.options.labels.minute,
        }
    }

    fn selector(&self, field: TimeField, baseline: Option<&baseline::DialMetrics>) -> View<M> {
        let session = self.state.input();
        let validation = session.validation().field(field);
        let preview = self
            .state
            .preview()
            .filter(|preview| preview.field == field);
        let shown = preview.map(|p| p.value).or(validation.ok());
        let digits = shown
            .map(|v| session.digit_text(field, v))
            .unwrap_or_else(|| "–".into());
        let selected = field == self.state.field();
        let colors = self.input.theme.colors;
        let disabled = self.input.options.disabled;
        let ink = if disabled {
            disabled_ink(self.input.theme)
        } else if selected {
            colors.on_primary_container
        } else {
            colors.on_surface
        };
        let label = match validation {
            Ok(value) => format!(
                "{} {}",
                self.caption(field),
                session.digit_text(field, value)
            ),
            Err(error) => (self.input.options.labels.error_text)(field, error, session.clock()),
        };
        let corners = match (baseline.is_some(), selected) {
            (true, _) => shape::SMALL,
            (false, true) => shape::LARGE,
            (false, false) => shape::MEDIUM,
        };
        let surface = if baseline.is_some() {
            colors.surface_container_highest
        } else {
            colors.surface_container_lowest
        };
        let width = baseline.map_or(VERTICAL_FIELD, |metrics| metrics.field_width);
        let mut style = Style::column()
            .width(Length::Px(width))
            .align(Align::Center)
            .justify(Align::Center)
            .corners(corners)
            .background(if selected && !disabled {
                colors.primary_container
            } else {
                surface
            })
            .state_layer(ink);
        if let Some(metrics) = baseline {
            style.height = Length::Px(metrics.field_height);
        } else {
            style.min_height = field_height(self.scale);
        }
        let mut node = Node::new(Role::Radio, label)
            .id(self.selector_id(field))
            .checked(selected)
            .focus_color(colors.secondary)
            .state_layer_alphas(20, 26, 26)
            .styled(style);
        if session.displayed_error(field).is_some() {
            node = node.described_by_text(self.input.id(&format!("{}-error", field_part(field))));
        }
        let id = self.input.id(&format!("{}-dial-value", field_part(field)));
        let number = if baseline.is_some() {
            let number_style = baseline::number_style(&digits, self.scale, width);
            View::new(
                Node::new(Role::Text, "")
                    .id(id)
                    .text(digits)
                    .text_scaling(TextScaling::Fixed)
                    .styled(
                        Style::column()
                            .with_text_style(number_style)
                            .foreground(ink),
                    ),
            )
        } else {
            let number_style = if self.scale > 1.5 {
                typography::HEADLINE_LARGE
            } else {
                typography::DISPLAY_MEDIUM
            };
            text(id, &digits, number_style, ink)
        };
        let caption = baseline.is_none().then(|| {
            text("", self.caption(field), typography::LABEL_MEDIUM, ink).without_interaction(None)
        });
        let view = button(
            node,
            (!disabled).then(|| (self.emit)(DialEvent::SetField(field))),
        )
        .children(
            caption
                .into_iter()
                .chain([number.without_interaction(None)]),
        );
        if disabled {
            return view;
        }
        let emit = self.emit.clone();
        let clock = session.clock();
        let start = value_id(self.input.options.id, field, default_value(field, clock));
        let view = view.on_key_focus_navigation(move |press| {
            plain_enter(press).then(|| FocusNavigation {
                target: start.clone(),
                message: emit(DialEvent::SetField(field)),
            })
        });
        if selected {
            let emit = self.emit.clone();
            let base = self.input.options.id.to_string();
            let value = validation.unwrap_or(default_value(field, clock));
            view.on_focus_navigation_with_extend(move |key, extend| {
                if extend || !matches!(key, EditKey::Down | EditKey::Right) {
                    return None;
                }
                Some(FocusNavigation {
                    target: value_id(&base, field, value),
                    message: emit(DialEvent::SetField(field)),
                })
            })
        } else {
            view
        }
    }

    fn error(&self, field: TimeField) -> Option<View<M>> {
        let error = self.state.input().displayed_error(field)?;
        let message =
            (self.input.options.labels.error_text)(field, error, self.state.input().clock());
        Some(text(
            self.input.id(&format!("{}-error", field_part(field))),
            &message,
            typography::BODY_SMALL,
            self.input.theme.colors.error,
        ))
    }

    fn selector_id(&self, field: TimeField) -> String {
        self.input.id(&format!("{}-selector", field_part(field)))
    }

    fn face(&self) -> View<M> {
        let state = self.state;
        let field = state.field();
        let clock = state.input().clock();
        let shown = state.input().validation().field(field).ok();
        let preview = state.preview().map(|preview| preview.value);
        let hand = preview.or(shown).map(|value| {
            let target = polar::center(field, clock, value);
            let center = self
                .motion
                .and_then(DialHandMotion::center)
                .unwrap_or(target);
            let settled = self.motion.is_none_or(DialHandMotion::is_settled);
            HandPaint {
                center,
                resting_value: (settled && center == target).then_some(value),
            }
        });
        let visual = hand.into_iter().flat_map(|hand| self.hand(hand.center));
        let values = match (field, clock) {
            (TimeField::Hour, ClockFormat::TwelveHour) => {
                std::iter::once(12).chain(1..12).collect()
            }
            (TimeField::Hour, ClockFormat::TwentyFourHour) => (0..24).collect(),
            (TimeField::Minute, _) => (0..60).step_by(5).collect::<Vec<_>>(),
        };
        let extras = [shown, preview]
            .into_iter()
            .flatten()
            .filter(|value| field == TimeField::Minute && value % 5 != 0);
        let mut values = values.into_iter().chain(extras).collect::<Vec<_>>();
        if field == TimeField::Minute {
            values.sort_unstable();
            values.dedup();
        }
        let colors = self.input.theme.colors;
        let background = if self.is_baseline() {
            colors.surface_container_highest
        } else {
            colors.surface_container_lowest
        };
        let face = View::new(
            Node::new(Role::RadioGroup, self.caption(field))
                .id(self.input.id("dial-face"))
                .styled(square(FACE).corners(shape::FULL).background(background)),
        )
        .children(visual.chain(values.into_iter().map(|value| {
            // Preview at a non-label minute has paint, but it is not a choice
            // and does not change the checked value exposed to accessibility.
            if field == TimeField::Minute && value % 5 != 0 && shown != Some(value) {
                self.preview_label(value, clock)
            } else {
                self.choice(field, value, shown, hand, clock)
            }
        })))
        .roving(Roving::horizontal(false));
        if self.input.options.disabled {
            return face;
        }
        let emit = self.emit.clone();
        face.on_point_drag_with_bounds(state.point_session.clone(), move |point, bounds| {
            resolve_point(point, bounds, field, clock)
                .map(|phase| emit(DialEvent::Point { field, phase }))
        })
    }

    fn hand(&self, (x, y): (f32, f32)) -> [View<M>; 3] {
        let mid = FACE / 2.;
        let length = (x - mid).hypot(y - mid);
        let normal = [(y - mid) / length, (mid - x) / length];
        let color = if self.input.options.disabled {
            disabled_ink(self.input.theme)
        } else {
            self.input.theme.colors.primary
        };
        let path = FillContour::new(
            [
                [mid + normal[0], mid + normal[1]],
                [x + normal[0], y + normal[1]],
                [x - normal[0], y - normal[1]],
                [mid - normal[0], mid - normal[1]],
            ],
            color,
        )
        .expect("valid dial hand");
        let track = View::new(
            Node::new(Role::Group, "")
                .id(self.input.id("dial-track"))
                .styled(absolute(0., 0., FACE))
                .fill_contour(path),
        )
        .without_interaction(None);
        let dot = |name, x, y, size| {
            View::new(
                Node::new(Role::Group, "").id(self.input.id(name)).styled(
                    absolute(x - size / 2., y - size / 2., size)
                        .corners(shape::FULL)
                        .background(color),
                ),
            )
            .without_interaction(None)
        };
        [
            track,
            dot("dial-center", mid, mid, 8.),
            dot("dial-handle", x, y, FACE_HANDLE),
        ]
    }

    fn choice(
        &self,
        field: TimeField,
        value: u8,
        selected: Option<u8>,
        hand: Option<HandPaint>,
        clock: ClockFormat,
    ) -> View<M> {
        let digits = self.state.input().digit_text(field, value);
        let (x, y) = polar::center(field, clock, value);
        let colors = self.input.theme.colors;
        let base = self.input.options.id;
        let chosen = selected == Some(value);
        let disabled = self.input.options.disabled;
        let size = self.face_type(field, clock);
        let (lit, obscured) = match hand {
            Some(hand) if hand.resting_value == Some(value) => (true, false),
            Some(hand) => match label_coverage(&digits, &size, (x, y), hand.center) {
                LabelCoverage::Outside => (false, false),
                LabelCoverage::Inside if hand.resting_value.is_none() => (true, false),
                _ => (false, true),
            },
            None => (false, false),
        };
        let ink = if lit && !disabled {
            colors.on_primary
        } else if disabled {
            colors.on_surface_variant
        } else {
            colors.on_surface
        };
        let name = (self.labels.value_name)(field, &digits, choice_count(field, clock));
        let exact_minute = field == TimeField::Minute && !value.is_multiple_of(5);
        let show_label = !obscured
            && (!exact_minute || hand.is_some_and(|paint| paint.resting_value == Some(value)));
        let node = Node::new(Role::Radio, name)
            .id(value_id(base, field, value))
            .checked(chosen)
            .focus_color(colors.secondary)
            .state_layer_alphas(20, 26, 26)
            .styled(
                absolute(x - TARGET / 2., y - TARGET / 2., TARGET)
                    .corners(shape::FULL)
                    .state_layer(ink),
            );
        let view = button(
            node,
            (!disabled).then(|| (self.emit)(DialEvent::Choose { field, value })),
        )
        // Keep the radio present during motion. An exact minute label fits
        // among the five-minute marks only when its handle covers that spot.
        .children(show_label.then(|| self.face_label(&digits, ink, size)))
        .restore_focus_on_removal(self.input.id(if field == TimeField::Hour {
            "minute-selector"
        } else {
            "hour-selector"
        }));
        if disabled {
            return view;
        }
        let base = base.to_owned();
        let emit = self.emit.clone();
        let rid = value_id(&base, field, value);
        let repeat = self.emit.clone();
        view.on_key_focus_navigation(move |press| {
            // Holding Enter after entering the face must not pick the mark.
            (press.repeat && plain_enter(press)).then(|| FocusNavigation {
                target: rid.clone(),
                message: repeat(DialEvent::SetField(field)),
            })
        })
        .on_focus_navigation_with_extend(move |key, extend| {
            if extend {
                return None;
            }
            let value = step(key, value, field, clock)?;
            Some(FocusNavigation {
                target: value_id(&base, field, value),
                message: emit(DialEvent::Adjust { field, value }),
            })
        })
    }

    fn preview_label(&self, value: u8, clock: ClockFormat) -> View<M> {
        let field = TimeField::Minute;
        let digits = self.state.input().digit_text(field, value);
        let (x, y) = polar::center(field, clock, value);
        View::new(Node::new(Role::Group, "").styled(absolute(
            x - TARGET / 2.,
            y - TARGET / 2.,
            TARGET,
        )))
        .children([self.face_label(
            &digits,
            self.input.theme.colors.on_primary,
            self.face_type(field, clock),
        )])
        .without_interaction(None)
    }

    fn face_type(&self, field: TimeField, clock: ClockFormat) -> TextStyle {
        // Keep adjacent rings clear even with two full-width digits. The
        // field summary and the Input alternative use the full text scale.
        let max_scale = if (field, clock) == (TimeField::Hour, ClockFormat::TwentyFourHour) {
            1.
        } else {
            1.5
        };
        let mut size = typography::BODY_LARGE;
        let scale = self.scale.min(max_scale);
        size.font_size *= scale;
        size.line_height = size.line_height.map(|height| height * scale);
        size
    }

    fn face_label(&self, digits: &str, ink: Color, size: TextStyle) -> View<M> {
        View::new(
            Node::new(Role::Text, "")
                .text(digits)
                .text_scaling(TextScaling::Fixed)
                .styled(Style::column().with_text_style(size).foreground(ink)),
        )
        .without_interaction(None)
    }
}

enum LabelCoverage {
    Outside,
    Partial,
    Inside,
}

fn label_coverage(
    digits: &str,
    size: &TextStyle,
    (x, y): (f32, f32),
    (handle_x, handle_y): (f32, f32),
) -> LabelCoverage {
    let width_in_em = digits
        .chars()
        .map(|digit| if digit.is_ascii() { 0.65 } else { 1. })
        .sum::<f32>();
    let half_width = size.font_size * width_in_em / 2.;
    let half_height = size.font_size / 2.;
    let dx = (x - handle_x).abs();
    let dy = (y - handle_y).abs();
    let radius = FACE_HANDLE / 2.;
    if (dx + half_width).hypot(dy + half_height) <= radius {
        LabelCoverage::Inside
    } else if (dx - half_width).max(0.).hypot((dy - half_height).max(0.)) < radius {
        LabelCoverage::Partial
    } else {
        LabelCoverage::Outside
    }
}

fn choice_count(field: TimeField, clock: ClockFormat) -> u8 {
    match (field, clock) {
        (TimeField::Hour, ClockFormat::TwelveHour) => 12,
        (TimeField::Hour, ClockFormat::TwentyFourHour) => 24,
        (TimeField::Minute, _) => 60,
    }
}

fn field_part(field: TimeField) -> &'static str {
    match field {
        TimeField::Hour => "hour",
        TimeField::Minute => "minute",
    }
}

fn plain_enter(press: &KeyPress) -> bool {
    press.key == Key::Enter && press.modifiers.is_empty()
}

fn value_id(base: &str, field: TimeField, value: u8) -> String {
    format!("{base}-dial-{}-{value}", field_part(field))
}

fn default_value(field: TimeField, clock: ClockFormat) -> u8 {
    match (field, clock) {
        (TimeField::Hour, ClockFormat::TwelveHour) => 12,
        _ => 0,
    }
}

fn step(key: EditKey, value: u8, field: TimeField, clock: ClockFormat) -> Option<u8> {
    let count = choice_count(field, clock);
    let zero = default_value(field, clock);
    let delta = match key {
        EditKey::Down | EditKey::Right => 1,
        EditKey::Up | EditKey::Left => count - 1,
        EditKey::Home => return Some(zero),
        EditKey::End => return Some(count - 1),
        _ => return None,
    };
    let value = (value + delta) % count;
    Some(if value == 0 { zero } else { value })
}

fn resolve_point(
    point: PointDrag,
    bounds: Rect,
    field: TimeField,
    clock: ClockFormat,
) -> Option<DialPoint> {
    let resolve = |origin: (f32, f32), position: (f32, f32)| {
        let local = |(x, y)| (f64::from(x - bounds.x), f64::from(y - bounds.y));
        polar::captured_hit(field, clock, local(origin), local(position))
    };
    Some(match point {
        PointDrag::Move { origin, position } => DialPoint::Move(resolve(origin, position)),
        PointDrag::Tap { origin, position } => DialPoint::Tap(resolve(origin, position)),
        PointDrag::End { origin, position } => DialPoint::End(resolve(origin, position)),
        PointDrag::Cancel => DialPoint::Cancel,
        _ => return None,
    })
}

fn square(size: f32) -> Style {
    Style::column()
        .width(Length::Px(size))
        .height(Length::Px(size))
        .align(Align::Center)
        .justify(Align::Center)
}

fn absolute(x: f32, y: f32, size: f32) -> Style {
    square(size).absolute(Insets {
        left: Length::Px(x),
        top: Length::Px(y),
        ..Default::default()
    })
}

fn mode_icon(mode: DialMode) -> VectorIcon {
    if mode == DialMode::Input {
        return keyboard_icon();
    }
    let ring = |r: f32| {
        (0..32)
            .map(|index| {
                let angle = (index as f32 / 32.) * std::f32::consts::TAU;
                [12. + r * angle.cos(), 12. + r * angle.sin()]
            })
            .collect::<Vec<_>>()
    };
    VectorIcon::from_contours([
        ring(10.),
        ring(8.),
        vec![
            [11., 6.],
            [13., 6.],
            [13., 11.5],
            [17., 14.],
            [16., 16.],
            [11., 13.],
        ],
    ])
    .expect("valid time picker dial icon")
}
