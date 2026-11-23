//! Material 3 Expressive Scroll with a shared direct-entry draft.

use std::rc::Rc;

use surface::{AdjustableValue, Align, Insets, Length, Node, Rect, Role, Style};
use surface_app::{EditKey, FocusNavigation, PointDrag, VerticalScrollResponse, View};

use crate::{
    date_picker::{button, disabled_ink, text, TARGET},
    shape::{self, LayoutDirection},
    time_input::{keyboard_icon, TimeInput, PANEL_MARGIN, PANEL_PADDING},
    typography,
    vector_icon::VectorIconSize,
    ClockFormat, Theme, TimeField, TimeInputEvent, TimeInputOptions, TimeInputPresentation,
    TimeScrollColumn, TimeScrollDrag, TimeScrollEvent, TimeScrollMode, TimeScrollState,
    TimeScrollStep, VectorIcon,
};

const PREFERRED_FIELD: f32 = 100.;
const MAX_FIELD: f32 = 132.;
const SEPARATOR: f32 = 16.;
const PERIOD_WIDTH: f32 = 56.;
const GAP: f32 = 8.;

/// Localized labels for the two modes and an unset scroll value.
#[derive(Clone, Copy)]
pub struct TimeScrollLabels<'a> {
    pub use_input: &'a str,
    pub use_scroll: &'a str,
    pub unfit: &'a str,
    pub unset: &'a str,
}

/// Options shared by the scroll wheels and their direct-entry alternative.
#[derive(Clone, Copy)]
pub struct TimeScrollOptions<'a> {
    pub input: TimeInputOptions<'a>,
    pub labels: &'a TimeScrollLabels<'a>,
    /// The application's effective text scale.
    pub text_scale: f32,
}

/// Size of each scroll wheel, in logical pixels, and placement of its controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeScrollLayout {
    pub field_width: f32,
    pub row_height: f32,
    pub content_width: f32,
    pub period_below: bool,
    pub inline_toggle_below: bool,
    period_side_width: f32,
}

fn number_type(scale: f32) -> surface::TextStyle {
    if scale > 1.5 {
        typography::HEADLINE_SMALL
    } else {
        typography::HEADLINE_MEDIUM
    }
}

/// Plan an inline or modal Scroll layout. Give inline callers the space assigned
/// to the picker. Send SetScrollFits when that space or the text scale changes.
pub fn time_scroll_layout(
    width: f32,
    height: f32,
    text_scale: f32,
    clock: ClockFormat,
    presentation: TimeInputPresentation,
) -> Option<TimeScrollLayout> {
    if !width.is_finite() || !height.is_finite() {
        return None;
    }
    let scale = if text_scale.is_finite() && text_scale > 1. {
        text_scale
    } else {
        1.
    };
    let modal = presentation == TimeInputPresentation::Modal;
    let period = clock == ClockFormat::TwelveHour;
    let typography = number_type(scale);
    let field_min = (typography.font_size * scale * 2. + 16.).max(80.);
    if field_min > MAX_FIELD {
        return None;
    }
    let margin = if modal {
        (PANEL_MARGIN + PANEL_PADDING) * 2.
    } else {
        0.
    };
    let available = (width - margin).min(if modal { 320. } else { 420. });
    let fields_min = 2. * field_min + SEPARATOR;
    if available < fields_min {
        return None;
    }
    let period_side_width = PERIOD_WIDTH.max(2. * typography::TITLE_MEDIUM.font_size * scale + 8.);
    let period_below = period && available < fields_min + period_side_width + GAP;
    let period_side = if period && !period_below {
        period_side_width + GAP
    } else {
        0.
    };
    let inline_toggle_below =
        !modal && (period_below || available < fields_min + period_side + TARGET + GAP);
    let toggle_side = if !modal && !inline_toggle_below {
        TARGET + GAP
    } else {
        0.
    };
    let row_height = TARGET.max(typography.line_height_px() * scale + 8.);
    let field_width =
        ((available - SEPARATOR - period_side - toggle_side) / 2.).min(if width >= 480. {
            MAX_FIELD
        } else {
            PREFERRED_FIELD.max(field_min)
        });
    let mut below = 0.;
    if period_below || inline_toggle_below {
        let period_height = if period_below { 40. * scale + 8. } else { 0. };
        below += TARGET.max(period_height) + GAP;
    }
    if modal {
        below += TARGET.max(40. * scale + 8.) + GAP;
    }
    (3. * row_height + margin + below <= height).then_some(TimeScrollLayout {
        field_width,
        row_height,
        content_width: field_width * 2. + SEPARATOR + period_side + toggle_side,
        period_below,
        inline_toggle_below,
        period_side_width,
    })
}

impl Theme {
    /// Render the Scroll picker or its direct-entry alternative.
    ///
    /// Apply events to the retained state and save only on Commit. Pass focus
    /// observations and animation time from the host to that state.
    pub fn time_scroll<M: Clone + 'static>(
        &self,
        options: TimeScrollOptions<'_>,
        state: &TimeScrollState,
        on_event: impl Fn(TimeScrollEvent) -> M + 'static,
    ) -> View<M> {
        let emit: Rc<dyn Fn(TimeScrollEvent) -> M> = Rc::new(on_event);
        let input_emit = emit.clone();
        ScrollPicker {
            input: TimeInput {
                theme: self,
                options: options.input,
                session: state.input(),
                emit: Rc::new(move |event| input_emit(TimeScrollEvent::Input(event))),
                editor_fallback_suffix: Some(("hour-wheel", "minute-wheel")),
                style: crate::time_input::TimePickerStyle::Expressive,
            },
            labels: options.labels,
            scale: options.text_scale,
            state,
            emit,
        }
        .view()
    }
}

struct ScrollPicker<'a, 'b, M> {
    input: TimeInput<'a, 'b, M>,
    labels: &'b TimeScrollLabels<'b>,
    scale: f32,
    state: &'a TimeScrollState,
    emit: Rc<dyn Fn(TimeScrollEvent) -> M>,
}

impl<M: Clone + 'static> ScrollPicker<'_, '_, M> {
    fn view(&self) -> View<M> {
        let options = self.input.options;
        let modal = options.presentation == TimeInputPresentation::Modal;
        let fit = time_scroll_layout(
            options.viewport.width,
            options.viewport.height,
            self.scale,
            self.state.input().clock(),
            options.presentation,
        );
        let active = self.state.active_field();
        let mode = self.state.mode();
        let layout = fit.filter(|_| mode == TimeScrollMode::Scroll && self.state.scroll_fits());
        let show_scroll = layout.is_some();
        let toggle = self.toggle(
            if show_scroll {
                TimeScrollMode::Input
            } else {
                TimeScrollMode::Scroll
            },
            fit.is_some() && self.state.scroll_fits(),
        );
        let mut modal_toggle = Some(toggle);
        let width = layout.map_or_else(
            || {
                let preferred = if self.input.session.period().is_some() {
                    336.
                } else {
                    264.
                };
                (options.viewport.width - 2. * PANEL_MARGIN)
                    .max(0.)
                    .min(preferred)
            },
            |layout| {
                let footer_button = TARGET.max(56. * self.scale + 8.);
                let footer_width = TARGET + 2. * GAP + 2. * footer_button;
                let inner_width = layout.content_width.max(footer_width);
                (inner_width + 2. * PANEL_PADDING)
                    .min((options.viewport.width - 2. * PANEL_MARGIN).max(0.))
            },
        );
        let fallback_reason = fit.is_none().then(|| {
            text(
                self.input.id("unfit"),
                self.labels.unfit,
                typography::LABEL_MEDIUM,
                self.input.theme.colors.on_surface_variant,
            )
        });
        let body = if let Some(layout) = layout {
            let inline_toggle = (!modal).then(|| modal_toggle.take()).flatten();
            self.scroll_body(layout, inline_toggle)
        } else if modal {
            self.input.modal_body(width, fallback_reason)
        } else {
            View::new(Node::new(Role::Group, "").styled(Style::column().gap(GAP))).children(
                fallback_reason
                    .into_iter()
                    .chain([self.input.body(
                        options.viewport.width.max(0.),
                        self.input.session.period(),
                        false,
                    )])
                    .chain(modal_toggle.take()),
            )
        };
        if modal {
            let initial = if options.disabled {
                self.input.id("cancel")
            } else if show_scroll {
                self.input.id(wheel_part(active))
            } else {
                self.input.id(input_part(active))
            };
            self.input.modal_panel(
                width,
                PANEL_PADDING,
                body,
                self.input.actions(modal_toggle),
                initial,
            )
        } else {
            View::new(
                Node::new(Role::Group, options.title)
                    .id(options.id)
                    .styled(Style::column().width(Length::Percent(1.))),
            )
            .children([body])
        }
    }

    fn toggle(&self, target: TimeScrollMode, fits: bool) -> View<M> {
        let (part, label, icon, focus) = match target {
            TimeScrollMode::Input => (
                "use-input",
                self.labels.use_input,
                keyboard_icon(),
                input_part(self.state.active_field()),
            ),
            TimeScrollMode::Scroll => (
                "use-scroll",
                self.labels.use_scroll,
                swipe_icon(),
                wheel_part(self.state.active_field()),
            ),
        };
        let enabled = !self.input.options.disabled && (fits || target == TimeScrollMode::Input);
        let colors = self.input.theme.colors;
        let ink = if enabled {
            colors.on_surface_variant
        } else {
            disabled_ink(self.input.theme)
        };
        button(
            Node::new(Role::Button, label)
                .id(self.input.id(part))
                .focus_color(colors.secondary)
                .state_layer_alphas(20, 26, 26)
                .styled(
                    centered(TARGET, TARGET)
                        .corners(shape::FULL)
                        .state_layer(ink),
                ),
            enabled.then(|| (self.emit)(TimeScrollEvent::Show(target))),
        )
        .children([icon
            .view(VectorIconSize::Dp24, ink)
            .without_interaction(None)])
        .protect_composition_on_press()
        .focus_on_pointer_release()
        .restore_focus_on_removal(self.input.id(focus))
    }

    fn scroll_body(&self, layout: TimeScrollLayout, mut toggle: Option<View<M>>) -> View<M> {
        let colors = self.input.theme.colors;
        let colon_size = typography::HEADLINE_MEDIUM_EMPHASIZED;
        let colon = View::new(
            Node::new(Role::Text, "")
                .text(":")
                .text_scaling(surface::TextScaling::Fixed)
                .styled(
                    Style::column()
                        .with_text_style(colon_size)
                        .width(Length::Px(SEPARATOR))
                        .height(Length::Px(
                            typography::HEADLINE_MEDIUM_EMPHASIZED.line_height_px(),
                        ))
                        .foreground(colors.on_surface),
                ),
        )
        .without_interaction(None);
        let period_width = if layout.period_below {
            let toggle_width = if toggle.is_some() { TARGET + GAP } else { 0. };
            ((layout.content_width - toggle_width - GAP) / 2.).min(100.)
        } else {
            layout.period_side_width
        };
        let mut period = self.input.session.period().map(|period| {
            self.input
                .picker_period(period, !layout.period_below, period_width)
        });
        let hour = self.wheel(TimeField::Hour, layout);
        let minute = self.wheel(TimeField::Minute, layout);
        let fields = View::new(
            self.input
                .row(Node::new(Role::Group, "").styled(Style::row().align(Align::Center))),
        )
        .children([hour, colon, minute]);
        let modal = self.input.options.presentation == TimeInputPresentation::Modal;
        let row = |children: Vec<_>| {
            View::new(
                self.input.row(
                    Node::new(Role::Group, "").styled(
                        Style::row()
                            .gap(GAP)
                            .align(Align::Center)
                            .justify(if modal { Align::Center } else { Align::Start }),
                    ),
                ),
            )
            .children(children)
        };
        let mut main = vec![fields];
        if !layout.period_below {
            main.extend(period.take());
        }
        if !layout.inline_toggle_below {
            main.extend(toggle.take());
        }
        let footer = period.into_iter().chain(toggle).collect::<Vec<_>>();
        let mut node = Node::new(Role::Group, "")
            .id(self.input.id("body"))
            .styled(Style::column().clip(modal).gap(GAP));
        if modal {
            node = node.shrink_to_fit_height();
        }
        let body = View::new(node)
            .children(std::iter::once(row(main)).chain((!footer.is_empty()).then(|| row(footer))));
        if modal {
            body.accessibility_scroll_pages()
        } else {
            body
        }
    }

    fn wheel(&self, field: TimeField, layout: TimeScrollLayout) -> View<M> {
        let colors = self.input.theme.colors;
        let column = self.state.column(field);
        let disabled = self.input.options.disabled;
        let labels = self.input.options.labels;
        let name = match field {
            TimeField::Hour => labels.hour,
            TimeField::Minute => labels.minute,
        };
        let error = self.input.session.displayed_error(field);
        let value_text = column.selected.map_or_else(
            || {
                error.map_or_else(
                    || self.labels.unset.to_owned(),
                    |error| (labels.error_text)(field, error, self.input.session.clock()),
                )
            },
            |value| self.input.session.digit_text(field, value),
        );
        let id = self.input.id(wheel_part(field));
        let node = Node::new(Role::Group, name)
            .id(&id)
            .adjustable(AdjustableValue {
                min: f64::from(column.minimum),
                max: f64::from(column.maximum),
                step: 1.,
                value: column.selected.map(f64::from),
                value_text: Some(value_text),
            })
            .disabled(disabled)
            .focus_color(colors.secondary)
            .state_layer_alphas(20, 26, 26)
            .styled(
                Style::column()
                    .width(Length::Px(layout.field_width))
                    .height(Length::Px(layout.row_height * 3.))
                    .clip(true)
                    .corners(shape::LARGE)
                    .background(colors.surface_container_lowest)
                    .state_layer(colors.on_surface),
            );
        let wheel = View::new(node)
            .children([self.wheel_paint(field, column, layout)])
            .restore_focus_on_removal(self.input.id(input_part(field)));
        if disabled {
            return wheel;
        }
        let neighbor = self.input.id(wheel_part(other_field(field)));
        let rtl = self.input.options.direction == LayoutDirection::RightToLeft;
        let row_height = layout.row_height;
        let drag_session = self.state.drag_session(field);
        let drag = self.emit.clone();
        let wheel_scroll = self.emit.clone();
        let keys = self.emit.clone();
        let value = self.emit.clone();
        wheel
            .on_press((self.emit)(TimeScrollEvent::Input(TimeInputEvent::Confirm)))
            .on_vertical_scroll(move |scroll| {
                Some(VerticalScrollResponse {
                    consumed: scroll.delta,
                    message: wheel_scroll(TimeScrollEvent::Wheel {
                        field,
                        delta_y: scroll.delta,
                        row_height,
                        now_ms: None,
                    }),
                })
            })
            .on_point_drag_with_bounds(drag_session.clone(), move |point, bounds| {
                Some(drag(TimeScrollEvent::Drag {
                    field,
                    session: drag_session.clone(),
                    sample: wheel_drag(point, bounds, row_height, column),
                    row_height,
                }))
            })
            .on_focus_navigation_with_extend(move |key, extend| {
                if extend {
                    return None;
                }
                let across = if (field == TimeField::Hour) ^ rtl {
                    EditKey::Right
                } else {
                    EditKey::Left
                };
                if key == across {
                    return Some(FocusNavigation {
                        target: neighbor.clone(),
                        message: keys(TimeScrollEvent::Focus(other_field(field))),
                    });
                }
                let step = match key {
                    EditKey::Up => TimeScrollStep::Previous,
                    EditKey::Down => TimeScrollStep::Next,
                    EditKey::Home => TimeScrollStep::First,
                    EditKey::End => TimeScrollStep::Last,
                    _ => return None,
                };
                Some(FocusNavigation {
                    target: id.clone(),
                    message: keys(TimeScrollEvent::Adjust { field, step }),
                })
            })
            .on_value(move |accepted| {
                value(TimeScrollEvent::Pick {
                    field,
                    value: accepted as u8,
                })
            })
    }

    fn wheel_paint(
        &self,
        field: TimeField,
        column: TimeScrollColumn,
        layout: TimeScrollLayout,
    ) -> View<M> {
        let colors = self.input.theme.colors;
        let ink = if self.input.options.disabled {
            disabled_ink(self.input.theme)
        } else {
            colors.on_surface
        };
        let h = layout.row_height;
        let w = layout.field_width;
        let active = column.preview.or(column.selected);
        let center = column.offset.round() as i16;
        let digits = number_type(self.scale);
        let rows = (-2..=2).filter_map(|relative| {
            let index = center + relative;
            let value = u8::try_from(i16::from(column.minimum) + index).ok()?;
            if !(column.minimum..=column.maximum).contains(&value) {
                return None;
            }
            let top = (1. + f32::from(index) - column.offset) * h;
            if top + h <= 0. || top >= 3. * h {
                return None;
            }
            let label = if active.is_none() && relative == 0 {
                "–".to_owned()
            } else {
                self.input.session.digit_text(field, value)
            };
            let size = if active == Some(value) {
                digits.clone()
            } else {
                typography::TITLE_LARGE
            };
            Some(
                View::new(Node::new(Role::Group, "").styled(at(centered(w, h), 0., top)))
                    .children([text("", &label, size, ink)]),
            )
        });
        let separator_ink = if self.input.session.displayed_error(field).is_some() {
            colors.error
        } else {
            colors.outline
        };
        let rules = [h, h * 2.].map(|y| {
            View::new(
                Node::new(Role::Group, "").styled(at(
                    Style::column()
                        .width(Length::Px(w - 16.))
                        .height(Length::Px(1.))
                        .background(separator_ink),
                    8.,
                    y,
                )),
            )
        });
        View::new(Node::new(Role::Group, "").styled(centered(w, h * 3.)))
            .children(rows.chain(rules))
            .without_interaction(None)
    }
}

fn centered(w: f32, h: f32) -> Style {
    Style::column()
        .width(Length::Px(w))
        .height(Length::Px(h))
        .align(Align::Center)
        .justify(Align::Center)
}

fn at(style: Style, x: f32, y: f32) -> Style {
    style.absolute(Insets {
        left: Length::Px(x),
        top: Length::Px(y),
        ..Insets::default()
    })
}

fn wheel_part(field: TimeField) -> &'static str {
    match field {
        TimeField::Hour => "hour-wheel",
        TimeField::Minute => "minute-wheel",
    }
}

fn input_part(field: TimeField) -> &'static str {
    match field {
        TimeField::Hour => "hour-input",
        TimeField::Minute => "minute-input",
    }
}

fn other_field(field: TimeField) -> TimeField {
    match field {
        TimeField::Hour => TimeField::Minute,
        TimeField::Minute => TimeField::Hour,
    }
}

fn wheel_drag(
    point: PointDrag,
    bounds: Rect,
    row: f32,
    column: TimeScrollColumn,
) -> TimeScrollDrag {
    match point {
        PointDrag::Tap {
            position: (_, y), ..
        } => {
            let value =
                (f32::from(column.minimum) + column.offset + (y - bounds.y) / row - 1.5).round();
            let inside = (f32::from(column.minimum)..=f32::from(column.maximum)).contains(&value);
            if inside && (column.selected.is_some() || (y - bounds.y - 1.5 * row).abs() >= row / 2.)
            {
                TimeScrollDrag::Tap { value: value as u8 }
            } else {
                TimeScrollDrag::Cancel
            }
        }
        PointDrag::Move { origin, position } | PointDrag::End { origin, position } => {
            let delta_y = position.1 - origin.1;
            if delta_y.abs() < (position.0 - origin.0).abs() {
                return TimeScrollDrag::Cancel;
            }
            match point {
                PointDrag::Move { .. } => TimeScrollDrag::Move { delta_y },
                _ => TimeScrollDrag::End { delta_y },
            }
        }
        _ => TimeScrollDrag::Cancel,
    }
}

fn swipe_icon() -> VectorIcon {
    VectorIcon::from_contours([
        [
            [2., 8.],
            [7., 3.],
            [12., 8.],
            [10.5, 9.5],
            [8., 7.],
            [8., 19.],
            [6., 19.],
            [6., 7.],
            [3.5, 9.5],
        ],
        [
            [12., 16.],
            [17., 21.],
            [22., 16.],
            [20.5, 14.5],
            [18., 17.],
            [18., 5.],
            [16., 5.],
            [16., 17.],
            [13.5, 14.5],
        ],
    ])
    .expect("valid time picker swipe icon")
}
