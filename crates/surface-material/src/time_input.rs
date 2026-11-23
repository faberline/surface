//! The inline and modal Material 3 time inputs share one controlled draft.

use std::rc::Rc;

use surface::{Align, Color, Insets, Length, Node, Rect, Role, Style, TextScaling, TextStyle};
use surface_app::{CursorStyle, Key, Roving, View};

use crate::{
    date_picker::{button, disabled_ink, text, TARGET},
    shape::{self, LayoutDirection},
    typography, ClockFormat, Theme, TimeField, TimeInputError, TimeInputEvent, TimeInputLocale,
    TimeInputSession, TimePeriod,
};

const FIELD_HEIGHT: f32 = 72.;
const SEPARATOR_WIDTH: f32 = 16.;
pub(crate) const PANEL_PADDING: f32 = 12.;
pub(crate) const PANEL_MARGIN: f32 = 8.;
const SHORT_MODAL_HEIGHT: f32 = 320.;

/// Show the editor in surrounding content or in a named, modal dialog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeInputPresentation {
    #[default]
    Inline,
    Modal,
}

/// Caller-localized copy for the fields and modal actions.
#[derive(Clone, Copy)]
pub struct TimeInputLabels<'a> {
    pub hour: &'a str,
    pub minute: &'a str,
    pub period: &'a str,
    pub hour_hint: &'a str,
    pub minute_hint: &'a str,
    pub confirm: &'a str,
    pub cancel: &'a str,
    pub error_text: &'a dyn Fn(TimeField, TimeInputError, ClockFormat) -> String,
}

/// Placement, labels and controlled availability of the editor.
#[derive(Clone, Copy)]
pub struct TimeInputOptions<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub labels: &'a TimeInputLabels<'a>,
    pub locale: TimeInputLocale<'a>,
    pub direction: LayoutDirection,
    pub presentation: TimeInputPresentation,
    pub viewport: Rect,
    pub disabled: bool,
    pub restore_focus_to: Option<&'a str>,
}

impl Theme {
    /// Render a controlled time input. Forward events to [TimeInputSession::apply].
    ///
    /// Supply the available dimensions in the viewport. The inline variant has
    /// no built-in submit action. The modal variant sends Confirm, even when a
    /// draft is invalid, so the session can display both applicable errors.
    /// Call [TimeInputSession::observe_input_focus] in the application's frame
    /// input observer with the currently focused field.
    ///
    /// The editors append `-hour-input` and `-minute-input` to the picker ID.
    pub fn time_input<M: Clone + 'static>(
        &self,
        options: TimeInputOptions<'_>,
        session: &TimeInputSession,
        on_event: impl Fn(TimeInputEvent) -> M + 'static,
    ) -> View<M> {
        TimeInput {
            theme: self,
            options,
            session,
            emit: Rc::new(on_event),
        }
        .view()
    }
}

pub(crate) struct TimeInput<'a, 'b, M> {
    pub(crate) theme: &'a Theme,
    pub(crate) options: TimeInputOptions<'b>,
    pub(crate) session: &'a TimeInputSession,
    pub(crate) emit: Rc<dyn Fn(TimeInputEvent) -> M>,
}

impl<M: Clone + 'static> TimeInput<'_, '_, M> {
    pub(crate) fn id(&self, part: &str) -> String {
        format!("{}-{part}", self.options.id)
    }

    pub(crate) fn row(&self, node: Node) -> Node {
        if self.options.direction == LayoutDirection::RightToLeft {
            node.reverse_row()
        } else {
            node
        }
    }

    fn short_modal(&self) -> bool {
        self.options.presentation == TimeInputPresentation::Modal
            && self.options.viewport.height < SHORT_MODAL_HEIGHT
    }

    fn view(&self) -> View<M> {
        let modal = self.options.presentation == TimeInputPresentation::Modal;
        let available = self.options.viewport.width.max(0.);
        let period = self.session.period();
        if !modal {
            return View::new(
                Node::new(Role::Group, self.options.title)
                    .id(self.options.id)
                    .styled(Style::column().width(Length::Percent(1.))),
            )
            .children([self.body(available, period, false)]);
        }

        let default_width = if period.is_some() { 336. } else { 264. };
        let width = (available - PANEL_MARGIN * 2.).max(0.).min(default_width);
        self.modal_panel(
            width,
            PANEL_PADDING,
            self.modal_body(width, None),
            self.actions(None),
            self.id(if self.options.disabled {
                "cancel"
            } else {
                "hour-input"
            }),
        )
    }

    pub(crate) fn modal_body(&self, width: f32, extra: Option<View<M>>) -> View<M> {
        // The Dialog keeps its accessible title at every height. In a short
        // modal, use the available body height for fields and their help.
        let inner_width = (width - PANEL_PADDING * 2.).max(0.);
        let title = (!self.short_modal()).then(|| self.title(inner_width));
        View::new(
            Node::new(Role::Group, "")
                .id(self.id("body"))
                .shrink_to_fit_height()
                .styled(Style::column().clip(true).gap(12.)),
        )
        .children(title.into_iter().chain(extra).chain([self.body(
            inner_width,
            self.session.period(),
            true,
        )]))
        .accessibility_scroll_pages()
    }

    pub(crate) fn title(&self, width: f32) -> View<M> {
        View::new(
            Node::new(Role::Heading, self.options.title)
                .id(self.id("title"))
                .text(self.options.title)
                .shrink_to_fit_width()
                .styled(
                    Style::column()
                        .width(Length::Px(width))
                        .with_text_style(typography::TITLE_LARGE)
                        .foreground(self.theme.colors.on_surface),
                ),
        )
    }

    pub(crate) fn modal_panel(
        &self,
        width: f32,
        padding: f32,
        body: View<M>,
        actions: View<M>,
        initial_focus: String,
    ) -> View<M> {
        let mut panel_style = Style::column()
            .width(Length::Px(width))
            .padding(padding)
            .gap(8.)
            .corners(shape::EXTRA_LARGE)
            .background(self.theme.colors.surface_container)
            .clip(true);
        // Let the actual wrapped action row keep its measured height. A
        // scrollable Input body can give up the remaining column space.
        panel_style.max_height =
            Length::Px((self.options.viewport.height - PANEL_MARGIN * 2.).max(0.));
        let panel = View::new(
            Node::new(Role::Dialog, self.options.title)
                .id(self.options.id)
                .tab_index(-1)
                .styled(panel_style),
        )
        .children([body, actions])
        .accessibility_scroll_pages()
        .modal_initial_focus(initial_focus);
        let panel = if let Some(fallback) = self.options.restore_focus_to {
            panel.modal_focus_fallback(fallback)
        } else {
            panel
        };
        let Color(r, g, b, _) = self.theme.colors.scrim;
        let mut layer = Style::column()
            .fixed(Insets::all(0.))
            .align(Align::Center)
            .justify(Align::Center)
            .background(Color(r, g, b, 82));
        layer.z_index = 1000;
        panel.modal_with_backdrop(
            Node::new(Role::Group, "")
                .id(self.id("layer"))
                .styled(layer),
            (self.emit)(TimeInputEvent::Cancel),
        )
    }

    fn body(&self, width: f32, period: Option<TimePeriod>, modal: bool) -> View<M> {
        let alongside = period.is_some() && width >= if modal { 312. } else { 392. };
        let period_width = if modal { 56. } else { 48. };
        let field_width =
            ((width - SEPARATOR_WIDTH - if alongside { period_width + 8. } else { 0. }) / 2.)
                .max(0.);
        let fields = self.fields(field_width, alongside);
        let period = period.map(|selected| self.period(selected, alongside, period_width, modal));
        let style = if alongside {
            Style::row()
                .width(Length::Percent(1.))
                .align(Align::Center)
                .gap(8.)
        } else {
            Style::column().width(Length::Percent(1.)).gap(8.)
        };
        let id = if modal { "form" } else { "body" };
        View::new(self.row(Node::new(Role::Group, "").id(self.id(id)).styled(style)))
            .children(std::iter::once(fields).chain(period))
    }

    fn fields(&self, width: f32, grows: bool) -> View<M> {
        let labels = self.options.labels;
        let text_ink = self.theme.colors.on_surface;
        let short = self.short_modal();
        let label = |part, caption| {
            let size = if short {
                typography::LABEL_MEDIUM
            } else {
                typography::LABEL_LARGE
            };
            text(self.id(part), caption, size, text_ink)
        };
        let digits = if short {
            typography::TITLE_LARGE
        } else if width >= 155. {
            typography::DISPLAY_MEDIUM
        } else if width >= 122. {
            typography::HEADLINE_MEDIUM
        } else {
            typography::TITLE_LARGE
        };
        let hour = self.field(
            TimeField::Hour,
            labels.hour,
            labels.hour_hint,
            digits.clone(),
        );
        let minute = self.field(TimeField::Minute, labels.minute, labels.minute_hint, digits);
        let separator_type = if short || self.options.viewport.width < 360. {
            typography::TITLE_LARGE_EMPHASIZED
        } else {
            typography::DISPLAY_LARGE_EMPHASIZED
        };
        let separator = View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Px(SEPARATOR_WIDTH))
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
                        .with_text_style(separator_type)
                        .foreground(text_ink),
                ),
        )])
        .without_interaction(None);
        let fields = Style::column().gap(if short { 2. } else { 4. });
        let fields = if grows {
            fields.width(Length::Px(0.)).grow(1.)
        } else {
            fields.width(Length::Percent(1.))
        };
        View::new(
            Node::new(Role::Group, "")
                .id(self.id("fields"))
                .styled(fields),
        )
        .children([
            self.columns(
                label("hour-label", labels.hour),
                self.spacer(),
                label("minute-label", labels.minute),
            ),
            self.columns(hour.0, separator, minute.0),
            self.columns(hour.1, self.spacer(), minute.1),
        ])
    }

    fn spacer(&self) -> View<M> {
        View::new(
            Node::new(Role::Group, "").styled(Style::column().width(Length::Px(SEPARATOR_WIDTH))),
        )
        .without_interaction(None)
    }

    fn columns(&self, hour: View<M>, spacer: View<M>, minute: View<M>) -> View<M> {
        let column = |content| {
            View::new(
                Node::new(Role::Group, "").styled(Style::column().width(Length::Px(0.)).grow(1.)),
            )
            .children([content])
        };
        View::new(
            self.row(Node::new(Role::Group, "").styled(Style::row().width(Length::Percent(1.)))),
        )
        .children([column(hour), spacer, column(minute)])
    }

    fn field(
        &self,
        field: TimeField,
        label: &str,
        hint: &str,
        digits: TextStyle,
    ) -> (View<M>, View<M>) {
        let part = match field {
            TimeField::Hour => "hour",
            TimeField::Minute => "minute",
        };
        let colors = self.theme.colors;
        let error = self.session.displayed_error(field);
        let (help_name, help, ink) = match error {
            Some(error) => (
                "error",
                (self.options.labels.error_text)(field, error, self.session.clock()),
                colors.error,
            ),
            None => ("hint", hint.to_owned(), colors.on_surface_variant),
        };
        let help_id = self.id(&format!("{part}-{help_name}"));
        let mut style = Style::column()
            .width(Length::Percent(1.))
            .padding(6.)
            .corners(shape::MEDIUM)
            .background(colors.surface_container_lowest)
            .foreground(if self.options.disabled {
                disabled_ink(self.theme)
            } else {
                colors.on_surface
            })
            .with_text_style(digits);
        style.min_height = FIELD_HEIGHT;
        if error.is_some() {
            style = style.border(1., colors.error);
        }
        let mut input = View::new(
            Node::new(Role::TextInput, label)
                .id(self.id(&format!("{part}-input")))
                .text(self.session.raw(field))
                .described_by_text(&help_id)
                .focus_color(if error.is_some() {
                    colors.error
                } else {
                    colors.secondary
                })
                .disabled(self.options.disabled)
                .styled(style),
        );
        if !self.options.disabled {
            let edit = self.emit.clone();
            let finish = self.emit.clone();
            input = input
                .on_input(move |raw| edit(TimeInputEvent::Edit(field, raw)))
                .on_key(move |key| {
                    (key.key == Key::Enter && key.modifiers.is_empty() && !key.repeat)
                        .then(|| finish(TimeInputEvent::FinishEditing(field)))
                })
                .cursor(CursorStyle::Text)
                .accessibility_scroll_into_view();
        }
        (input, text(help_id, &help, typography::BODY_SMALL, ink))
    }

    fn period(&self, selected: TimePeriod, column: bool, width: f32, modal: bool) -> View<M> {
        self.period_view(selected, column, width, modal, None)
    }

    pub(crate) fn dial_period(&self, selected: TimePeriod, column: bool, width: f32) -> View<M> {
        self.period_view(selected, column, width, true, (!column).then_some(width))
    }

    fn period_view(
        &self,
        selected: TimePeriod,
        column: bool,
        width: f32,
        modal: bool,
        row_width: Option<f32>,
    ) -> View<M> {
        let style = if column {
            Style::column().width(Length::Px(width)).gap(8.)
        } else {
            Style::row().gap(8.)
        };
        let make_period = |period, id| {
            let active = selected == period;
            let disabled = self.options.disabled;
            let colors = self.theme.colors;
            let ink = if disabled {
                disabled_ink(self.theme)
            } else if active {
                colors.on_primary_container
            } else {
                colors.on_surface_variant
            };
            let corners = match (active, modal) {
                (true, true) => shape::LARGE,
                (true, false) => shape::MEDIUM,
                (false, _) => shape::FULL,
            };
            let mut style = Style::row()
                .align(Align::Center)
                .justify(Align::Center)
                .padding(4.)
                .corners(corners)
                .background(if active && !disabled {
                    colors.primary_container
                } else {
                    colors.surface_container_lowest
                })
                .state_layer(if active {
                    colors.on_primary_container
                } else {
                    colors.on_surface
                });
            style.min_height = TARGET;
            style.min_width = TARGET;
            if column {
                style.width = Length::Percent(1.);
            } else if let Some(width) = row_width {
                style.width = Length::Px(width);
            }
            let label = self.options.locale.period_label(period);
            button(
                Node::new(Role::Radio, label)
                    .id(self.id(id))
                    .checked(active)
                    .focus_color(colors.secondary)
                    .state_layer_alphas(20, 26, 26)
                    .styled(style),
                (!disabled).then(|| (self.emit)(TimeInputEvent::SelectPeriod(period))),
            )
            .children([text(
                "",
                label,
                if active {
                    typography::TITLE_MEDIUM_EMPHASIZED
                } else {
                    typography::TITLE_MEDIUM
                },
                ink,
            )
            .without_interaction(None)])
            .protect_composition_on_press()
        };
        View::new(
            self.row(
                Node::new(Role::RadioGroup, self.options.labels.period)
                    .id(self.id("period"))
                    .styled(style),
            ),
        )
        .children([
            make_period(TimePeriod::Am, "am"),
            make_period(TimePeriod::Pm, "pm"),
        ])
        .roving(if column {
            Roving::vertical(true)
        } else {
            Roving::horizontal(true)
        })
    }

    pub(crate) fn actions(&self, leading: Option<View<M>>) -> View<M> {
        let make_action = |id, label, event, enabled| {
            let ink = if enabled {
                self.theme.colors.primary
            } else {
                disabled_ink(self.theme)
            };
            let mut style = Style::row()
                .width(Length::Px(0.))
                .grow(1.)
                .align(Align::Center)
                .justify(Align::Center)
                .padding(4.)
                .corners(shape::FULL)
                .state_layer(ink);
            style.min_height = TARGET;
            style.min_width = TARGET;
            button(
                Node::new(Role::Button, label)
                    .id(self.id(id))
                    .focus_color(self.theme.colors.secondary)
                    .styled(style),
                enabled.then(|| (self.emit)(event)),
            )
            .children([text("", label, typography::LABEL_LARGE, ink).without_interaction(None)])
        };
        View::new(
            self.row(
                Node::new(Role::Group, "")
                    .id(self.id("actions"))
                    .styled(Style::row().gap(8.)),
            ),
        )
        .children(
            leading.into_iter().chain([
                make_action(
                    "cancel",
                    self.options.labels.cancel,
                    TimeInputEvent::Cancel,
                    true,
                ),
                make_action(
                    "confirm",
                    self.options.labels.confirm,
                    TimeInputEvent::Confirm,
                    !self.options.disabled,
                )
                .protect_composition_on_press(),
            ]),
        )
    }
}
