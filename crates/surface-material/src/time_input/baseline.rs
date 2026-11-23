//! Baseline modal Input and the shared Baseline AM/PM visual group.

use std::rc::Rc;

use surface::{Align, Color, Corners, Insets, Length, Node, Role, Style, TextScaling};
use surface_app::{Roving, View};

use super::{
    bounded_caption, TimeInput, TimeInputOptions, TimeInputPresentation, TimePickerStyle,
    PANEL_MARGIN,
};
use crate::{
    date_picker::{button, disabled_ink, TARGET},
    shape, typography, Theme, TimeField, TimeInputEvent, TimeInputLocale, TimeInputSession,
    TimePeriod,
};

pub(crate) const BASELINE_PANEL_PADDING: f32 = 24.;
const FIELD_WIDTH: f32 = 96.;
const FIELD_HEIGHT: f32 = 72.;
const SEPARATOR: f32 = 16.;
const GROUP_GAP: f32 = 12.;
const FOOTER_GAP: f32 = 8.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BaselinePeriodLayout {
    Input,
    DialVertical,
    DialHorizontal,
}

#[derive(Clone, Copy)]
struct PeriodGeometry {
    width: f32,
    height: f32,
    paint_width: f32,
    paint_height: f32,
}

impl PeriodGeometry {
    fn within_width(self, width: f32) -> Self {
        Self {
            width: self.width.min(width),
            paint_width: self.paint_width.min(width),
            ..self
        }
    }
}

impl BaselinePeriodLayout {
    pub(crate) fn size(self, text_scale: f32) -> (f32, f32) {
        let geometry = self.geometry(text_scale, 2.);
        (geometry.width, geometry.height)
    }

    fn localized_geometry(self, text_scale: f32, locale: TimeInputLocale<'_>) -> PeriodGeometry {
        let label_em = [TimePeriod::Am, TimePeriod::Pm]
            .into_iter()
            .map(|period| text_em(locale.period_label(period)))
            .fold(2_f32, f32::max);
        self.geometry(text_scale, label_em)
    }

    fn geometry(self, text_scale: f32, label_em: f32) -> PeriodGeometry {
        let scale = scale(text_scale);
        let caption_width = typography::TITLE_MEDIUM.font_size * scale * label_em + 8.;
        let caption_height = 24. * scale + 8.;
        match self {
            Self::Input | Self::DialVertical => {
                let base = if self == Self::Input { 72_f32 } else { 80_f32 };
                let paint_width = 52_f32.max(caption_width);
                let paint_height = base.max(caption_height * 2.);
                PeriodGeometry {
                    width: paint_width.max(TARGET),
                    height: paint_height.max(TARGET * 2.),
                    paint_width,
                    paint_height,
                }
            }
            Self::DialHorizontal => {
                let paint_width = 216_f32.max(caption_width * 2.);
                let paint_height = 38_f32.max(caption_height);
                PeriodGeometry {
                    width: paint_width.max(TARGET * 2.),
                    height: paint_height.max(TARGET),
                    paint_width,
                    paint_height,
                }
            }
        }
    }
}

impl Theme {
    /// Render the Baseline modal time input with its own visual tokens.
    ///
    /// Give the builder the scale returned by the host application's
    /// [`Application::text_scale`](surface_app::Application::text_scale).
    /// Forward events to [TimeInputSession::apply], and observe accepted
    /// editor focus using [TimeInputSession::observe_baseline_input_focus].
    pub fn time_input_baseline<M: Clone + 'static>(
        &self,
        mut options: TimeInputOptions<'_>,
        text_scale: f32,
        session: &TimeInputSession,
        on_event: impl Fn(TimeInputEvent) -> M + 'static,
    ) -> View<M> {
        options.presentation = TimeInputPresentation::Modal;
        let input = TimeInput {
            theme: self,
            options,
            session,
            emit: Rc::new(on_event),
            editor_fallback_suffix: None,
            style: TimePickerStyle::Baseline,
        };
        let width = input.baseline_modal_width(text_scale);
        let padding = input.baseline_modal_padding(text_scale);
        input.modal_panel(
            width,
            padding,
            input.baseline_modal_body(width, padding, text_scale, None),
            input.actions(None),
            input.id(if options.disabled {
                "cancel"
            } else {
                "hour-input"
            }),
        )
    }
}

impl<M: Clone + 'static> TimeInput<'_, '_, M> {
    pub(crate) fn baseline_modal_width(&self, text_scale: f32) -> f32 {
        let (field_width, _) = self.baseline_field_size(text_scale);
        let period_width = self.session.period().map_or(0., |_| {
            GROUP_GAP
                + BaselinePeriodLayout::Input
                    .localized_geometry(text_scale, self.options.locale)
                    .width
        });
        let preferred = field_width * 2.
            + SEPARATOR
            + period_width
            + 2. * self.baseline_modal_padding(text_scale);
        preferred.min((self.options.viewport.width - 2. * PANEL_MARGIN).max(0.))
    }

    pub(crate) fn baseline_modal_padding(&self, text_scale: f32) -> f32 {
        if !self.short_modal() {
            return BASELINE_PANEL_PADDING;
        }
        let (_, editor_height) = self.baseline_field_size(text_scale);
        // Allow the actual footer to take its measured space. This budget
        // reduces the padding to leave one complete editor when labels wrap.
        let footer = (40. * scale(text_scale) + 8.).max(TARGET);
        let available = self.options.viewport.height - 2. * PANEL_MARGIN - FOOTER_GAP;
        ((available - editor_height - footer) / 2.).clamp(0., BASELINE_PANEL_PADDING)
    }

    pub(crate) fn baseline_modal_body(
        &self,
        width: f32,
        padding: f32,
        text_scale: f32,
        extra: Option<View<M>>,
    ) -> View<M> {
        let inner_width = (width - padding * 2.).max(0.);
        let title = (!self.short_modal()).then(|| self.title(inner_width));
        View::new(
            Node::new(Role::Group, "")
                .id(self.id("body"))
                .shrink_to_fit_height()
                .styled(
                    Style::column()
                        .width(Length::Px(inner_width))
                        .clip(true)
                        .gap(GROUP_GAP),
                ),
        )
        .children(
            title
                .into_iter()
                .chain(extra)
                .chain([self.baseline_form(inner_width, text_scale)]),
        )
        .accessibility_scroll_pages()
    }

    fn baseline_field_size(&self, text_scale: f32) -> (f32, f32) {
        let scale = scale(text_scale);
        let widest = (0..10)
            .map(|digit| text_em(&self.session.digit_text(TimeField::Minute, digit * 11)))
            .fold(0_f32, f32::max);
        let digits = typography::DISPLAY_MEDIUM.font_size * scale * widest;
        let line = 52. * scale;
        (FIELD_WIDTH.max(digits + 12.), FIELD_HEIGHT.max(line + 12.))
    }

    fn baseline_form(&self, width: f32, text_scale: f32) -> View<M> {
        let (field_width, field_height) = self.baseline_field_size(text_scale);
        let pair_width = field_width * 2. + SEPARATOR;
        let stacked = pair_width > width;
        let fields = self.baseline_fields(field_width.min(width), field_height, stacked);
        let layout = BaselinePeriodLayout::Input;
        let geometry = layout
            .localized_geometry(text_scale, self.options.locale)
            .within_width(width);
        let beside = self.session.period().is_some()
            && !stacked
            && pair_width + GROUP_GAP + geometry.width <= width;
        let period = self
            .session
            .period()
            .map(|value| self.baseline_period_view(value, layout, geometry));
        let style = if beside {
            Style::row().align(Align::Start)
        } else {
            Style::column().align(Align::Start)
        }
        .width(Length::Percent(1.))
        .gap(GROUP_GAP);
        View::new(self.row(Node::new(Role::Group, "").id(self.id("form")).styled(style)))
            .children(std::iter::once(fields).chain(period))
    }

    fn baseline_fields(&self, width: f32, height: f32, stacked: bool) -> View<M> {
        let labels = self.options.labels;
        let editor =
            |field, label, hint| self.field(field, label, hint, typography::DISPLAY_MEDIUM, height);
        let (hour, hour_help) = editor(TimeField::Hour, labels.hour, labels.hour_hint);
        let (minute, minute_help) = editor(TimeField::Minute, labels.minute, labels.minute_hint);
        let children = if stacked {
            let column = |input, help| {
                View::new(
                    Node::new(Role::Group, "")
                        .styled(Style::column().width(Length::Percent(1.)).gap(4.)),
                )
                .children([input, help])
            };
            [column(hour, hour_help), column(minute, minute_help)]
        } else {
            let separator = View::new(
                Node::new(Role::Group, "").styled(
                    Style::column()
                        .width(Length::Px(SEPARATOR))
                        .align(Align::Center)
                        .justify(Align::Center),
                ),
            )
            .children([View::new(
                Node::new(Role::Text, "")
                    .id(self.id("separator"))
                    .text(":")
                    .text_scaling(TextScaling::Fixed)
                    .styled(
                        Style::column()
                            .with_text_style(typography::DISPLAY_LARGE)
                            .foreground(self.theme.colors.on_surface),
                    ),
            )])
            .without_interaction(None);
            [
                self.columns(hour, separator, minute),
                self.columns(hour_help, self.spacer(), minute_help),
            ]
        };
        let total = if stacked {
            width
        } else {
            width * 2. + SEPARATOR
        };
        View::new(
            Node::new(Role::Group, "").id(self.id("fields")).styled(
                Style::column()
                    .width(Length::Px(total))
                    .gap(if stacked { GROUP_GAP } else { 4. }),
            ),
        )
        .children(children)
    }

    pub(crate) fn baseline_period(
        &self,
        selected: TimePeriod,
        layout: BaselinePeriodLayout,
        text_scale: f32,
    ) -> View<M> {
        self.baseline_period_view(selected, layout, layout.geometry(text_scale, 2.))
    }

    fn baseline_period_view(
        &self,
        selected: TimePeriod,
        layout: BaselinePeriodLayout,
        period: PeriodGeometry,
    ) -> View<M> {
        let vertical = layout != BaselinePeriodLayout::DialHorizontal;
        let style = if vertical {
            Style::column()
        } else {
            Style::row()
        };
        let left = (period.width - period.paint_width) / 2.;
        let top = (period.height - period.paint_height) / 2.;
        let colors = self.theme.colors;
        let outline_ink = if self.options.disabled {
            let Color(r, g, b, _) = colors.on_surface;
            Color(r, g, b, 31)
        } else {
            colors.outline
        };
        let outline = View::new(
            Node::new(Role::Group, "")
                .id(self.id("period-paint"))
                .styled(
                    absolute_size(left, top, period.paint_width, period.paint_height)
                        .corners(shape::SMALL)
                        .border(1., outline_ink),
                ),
        )
        .without_interaction(None);
        let divider = if vertical {
            absolute_size(left, period.height / 2. - 0.5, period.paint_width, 1.)
        } else {
            absolute_size(period.width / 2. - 0.5, top, 1., period.paint_height)
        };
        let divider = View::new(Node::new(Role::Group, "").styled(divider.background(outline_ink)))
            .without_interaction(None);
        View::new(
            self.row(
                Node::new(Role::RadioGroup, self.options.labels.period)
                    .id(self.id("period"))
                    .styled(
                        style
                            .width(Length::Px(period.width))
                            .height(Length::Px(period.height)),
                    ),
            ),
        )
        .children([
            self.baseline_period_button(TimePeriod::Am, selected, period, vertical),
            self.baseline_period_button(TimePeriod::Pm, selected, period, vertical),
            outline,
            divider,
        ])
        .roving(if vertical {
            Roving::vertical(true)
        } else {
            Roving::horizontal(true)
        })
    }

    fn baseline_period_button(
        &self,
        period: TimePeriod,
        selected: TimePeriod,
        geometry: PeriodGeometry,
        vertical: bool,
    ) -> View<M> {
        let colors = self.theme.colors;
        let disabled = self.options.disabled;
        let active = selected == period;
        let am = period == TimePeriod::Am;
        let id = if am { "am" } else { "pm" };
        let label = self.options.locale.period_label(period);
        let (width, height, paint_width, paint_height) = if vertical {
            (
                geometry.width,
                geometry.height / 2.,
                geometry.paint_width,
                geometry.paint_height / 2.,
            )
        } else {
            (
                geometry.width / 2.,
                geometry.height,
                geometry.paint_width / 2.,
                geometry.paint_height,
            )
        };
        let justify = match (vertical, am) {
            (true, true) => Align::End,
            (true, false) => Align::Start,
            (false, _) => Align::Center,
        };
        let radius = shape::SMALL.top_left;
        let corners = match (vertical, am) {
            (true, true) => Corners::top(radius),
            (true, false) => Corners::bottom(radius),
            (false, true) => shape::start(radius, self.options.direction),
            (false, false) => shape::end(radius, self.options.direction),
        };
        let ink = if disabled {
            disabled_ink(self.theme)
        } else if active {
            colors.on_tertiary_container
        } else {
            colors.on_surface_variant
        };
        let background = if active && !disabled {
            colors.tertiary_container
        } else {
            Color::TRANSPARENT
        };
        let paint = View::new(
            Node::new(Role::Group, "")
                .id(self.id(&format!("{id}-paint")))
                .state_layer_alphas(20, 26, 26)
                .styled(
                    Style::column()
                        .width(Length::Px(paint_width))
                        .height(Length::Px(paint_height))
                        .align(Align::Center)
                        .justify(Align::Center)
                        .corners(corners)
                        .state_layer(ink)
                        .background(background),
                ),
        )
        .children([bounded_caption(
            label,
            typography::TITLE_MEDIUM,
            ink,
            Length::Px((paint_width - 8.).max(0.)),
            (paint_height - 8.).max(0.),
        )])
        .without_interaction(None);
        button(
            Node::new(Role::Radio, label)
                .id(self.id(id))
                .checked(active)
                .focus_color(colors.secondary)
                .styled(
                    Style::column()
                        .width(Length::Px(width))
                        .height(Length::Px(height))
                        .align(Align::Center)
                        .justify(justify)
                        .corners(shape::SMALL),
                ),
            (!disabled).then(|| (self.emit)(TimeInputEvent::SelectPeriod(period))),
        )
        .children([paint])
        .protect_composition_on_press()
        .accessibility_scroll_into_view()
    }
}

fn text_em(text: &str) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { 0.65 } else { 1. })
        .sum()
}

fn scale(text_scale: f32) -> f32 {
    if text_scale.is_finite() {
        text_scale.max(1.)
    } else {
        1.
    }
}

fn absolute_size(left: f32, top: f32, width: f32, height: f32) -> Style {
    Style::column()
        .width(Length::Px(width))
        .height(Length::Px(height))
        .absolute(Insets {
            left: Length::Px(left),
            top: Length::Px(top),
            ..Default::default()
        })
}
