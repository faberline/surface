//! A controlled Material 3 range calendar and range editor.
mod scroll;

use std::rc::Rc;

use surface::{Align, Color, Insets, Length, Node, Rect, Role, Style};
use surface_app::{
    AnimationFrame, AnimationUpdate, FrameInput, HorizontalDrag, HorizontalDragSession, Queue,
    VerticalDrag, VerticalDragSession, VerticalScroll, VerticalScrollResponse, View,
};

use self::scroll::{month_id, CalendarScroll};
use crate::{
    adaptive::WindowWidthSizeClass,
    date_picker::{
        button,
        calendar_view::{range_month, RangeMonth, GRID_WIDTH},
        date_editor, date_icon_button, disabled_ink, text, DateEditorOptions, Glyph, TARGET,
    },
    shape::{self, LayoutDirection},
    typography, CivilDate, CivilDateRange, DatePickerLocale, DateRangeConstraints, DateRangeEffect,
    DateRangeEndpoint, DateRangeEvent, DateRangeMode, DateRangePairError, DateRangePickerState,
    DateRangePosition, DateRangeValidation, Theme, YearMonth,
};

/// Localized names and descriptions for the range picker.
#[derive(Clone, Copy)]
pub struct DateRangePickerLabels<'a> {
    pub title: &'a str,
    pub start: &'a str,
    pub end: &'a str,
    pub edit_mode: &'a str,
    pub calendar_mode: &'a str,
    pub save: &'a str,
    pub close: &'a str,
    pub previous_month: &'a str,
    pub next_month: &'a str,
    pub calendar: &'a str,
    pub summary: &'a dyn Fn(Option<CivilDate>, Option<CivilDate>) -> String,
    pub day_name: &'a dyn Fn(CivilDate, DateRangePosition) -> String,
    pub pair_error: &'a dyn Fn(DateRangePairError) -> String,
}

/// Data policy, locale and placement for the range picker.
#[derive(Clone, Copy)]
pub struct DateRangePickerOptions<'a> {
    pub id: &'a str,
    pub today: CivilDate,
    pub constraints: DateRangeConstraints<'a>,
    pub locale: &'a DatePickerLocale<'a>,
    pub labels: &'a DateRangePickerLabels<'a>,
    pub viewport: Rect,
    pub restore_focus_to: Option<&'a str>,
}

/// Input from the range dialog. Forward to [DateRangePickerSession::apply].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DateRangePickerInput {
    Date(DateRangeEvent),
    Wheel(VerticalScroll),
    VerticalDrag(VerticalDrag),
    HorizontalDrag(HorizontalDrag),
}

/// Own this session while the dialog is open. Store a range on Commit
/// and drop the session on Commit or Dismiss.
pub struct DateRangePickerSession {
    state: DateRangePickerState,
    scroll: CalendarScroll,
    vertical_drag: VerticalDragSession,
    horizontal_drag: HorizontalDragSession,
}

impl DateRangePickerSession {
    pub fn new(
        committed: Option<CivilDateRange>,
        today: CivilDate,
        constraints: DateRangeConstraints<'_>,
        locale: &DatePickerLocale<'_>,
        mode: DateRangeMode,
    ) -> Self {
        let state = DateRangePickerState::new(committed, today, constraints, locale, mode);
        let mut scroll = CalendarScroll::new(state.month(), constraints.dates.bounds);
        if mode == DateRangeMode::Input {
            scroll.reset_input_scroll();
        }
        Self {
            state,
            scroll,
            vertical_drag: VerticalDragSession::new(),
            horizontal_drag: HorizontalDragSession::new(),
        }
    }

    pub fn state(&self) -> &DateRangePickerState {
        &self.state
    }

    pub fn mounted_months(&self) -> &[YearMonth] {
        self.scroll.months()
    }

    /// Apply a caller change in date availability or bounds.
    pub fn reconcile_constraints(&mut self, constraints: DateRangeConstraints<'_>) {
        self.state.reconcile_constraints(constraints);
        self.scroll
            .reset(self.state.month(), constraints.dates.bounds);
    }

    pub fn apply(
        &mut self,
        input: DateRangePickerInput,
        constraints: DateRangeConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> DateRangeEffect {
        match input {
            DateRangePickerInput::Date(event) => {
                let before = self.state.mode();
                let browse = matches!(event, DateRangeEvent::BrowseTo(_));
                let navigate = matches!(event, DateRangeEvent::NavigateFrom(..));
                let effect = self.state.apply(event, constraints, locale);
                let month = self.state.month();
                let bounds = constraints.dates.bounds;
                match (before, self.state.mode()) {
                    (DateRangeMode::Input, DateRangeMode::Calendar) => {
                        self.scroll.reset(month, bounds);
                    }
                    (DateRangeMode::Calendar, DateRangeMode::Input) => {
                        self.scroll.reset_input_scroll();
                    }
                    (_, DateRangeMode::Calendar) if browse => {
                        if !self.mounted_months().contains(&month) {
                            self.scroll.reset(month, bounds);
                        }
                        self.scroll.browse_to(month);
                    }
                    (_, DateRangeMode::Calendar) if navigate => {
                        self.scroll.mount_for_focus(month, bounds);
                    }
                    _ => {}
                }
                return effect;
            }
            DateRangePickerInput::Wheel(event) => self.scroll.wheel(event),
            DateRangePickerInput::VerticalDrag(event) => match event {
                VerticalDrag::Move { delta_y } => self.scroll.drag(delta_y, false, false),
                VerticalDrag::End { delta_y, .. } => self.scroll.drag(delta_y, false, true),
                VerticalDrag::Cancel => self.scroll.cancel_drag(),
                _ => {}
            },
            DateRangePickerInput::HorizontalDrag(event) => match event {
                HorizontalDrag::Move { delta_x } => self.scroll.drag(delta_x, true, false),
                HorizontalDrag::End { delta_x, .. } => self.scroll.drag(delta_x, true, true),
                HorizontalDrag::Cancel => self.scroll.cancel_drag(),
                _ => {}
            },
        }
        DateRangeEffect::None
    }

    /// Call in the application's input frame hook to measure calendar months
    /// and finish editing on focus exit. Then call [Self::flush_scroll_requests].
    pub fn observe_frame(
        &mut self,
        id: &str,
        frame: &AnimationFrame<'_>,
        input: FrameInput<'_>,
        constraints: DateRangeConstraints<'_>,
        locale: &DatePickerLocale<'_>,
    ) -> AnimationUpdate {
        if self.state.mode() == DateRangeMode::Input {
            let endpoint = [DateRangeEndpoint::Start, DateRangeEndpoint::End]
                .into_iter()
                .find(|endpoint| {
                    input.focused
                        == Some(format!("{id}-{}-input", endpoint_name(*endpoint)).as_str())
                });
            return AnimationUpdate {
                rebuild: self
                    .state
                    .observe_input_focus(endpoint, constraints, locale),
                request_next: self.scroll.observe_editor(id, frame),
            };
        }
        let (mut update, month) = self.scroll.observe(
            id,
            frame,
            constraints.dates.bounds,
            locale.direction == LayoutDirection::RightToLeft,
            input.focused,
        );
        if let Some(month) = month.filter(|month| *month != self.state.month()) {
            self.state
                .apply(DateRangeEvent::SetVisibleMonth(month), constraints, locale);
            update.rebuild = true;
            update.request_next = true;
        }
        update
    }

    /// Queue the measured scroll change for this frame before it is drawn.
    pub fn flush_scroll_requests<M>(&mut self, id: &str, queue: &Queue<M>) {
        self.scroll.flush_requests(id, queue);
    }
}

impl Theme {
    /// Mount a full-screen range calendar or an adaptive range input dialog.
    /// Close, Save and the mode switch stay above the clipped content.
    pub fn date_range_picker<M: Clone + 'static>(
        &self,
        options: DateRangePickerOptions<'_>,
        session: &DateRangePickerSession,
        on_input: impl Fn(DateRangePickerInput) -> M + 'static,
    ) -> View<M> {
        let window = (
            options.viewport.width.max(0.),
            options.viewport.height.max(0.),
        );
        let compact =
            WindowWidthSizeClass::from_dp(window.0) == Some(WindowWidthSizeClass::Compact);
        let fullscreen = session.state.mode() == DateRangeMode::Calendar || compact;
        let (width, height) = if fullscreen {
            window
        } else {
            (328_f32.min(window.0), 512_f32.min(window.1))
        };
        RangePicker {
            theme: self,
            options,
            session,
            emit: Rc::new(on_input),
            width,
            height,
            fullscreen,
        }
        .view()
    }
}

struct RangePicker<'a, 'b, M> {
    theme: &'a Theme,
    options: DateRangePickerOptions<'b>,
    session: &'a DateRangePickerSession,
    emit: Rc<dyn Fn(DateRangePickerInput) -> M>,
    width: f32,
    height: f32,
    fullscreen: bool,
}

impl<M: Clone + 'static> RangePicker<'_, '_, M> {
    fn id(&self, part: &str) -> String {
        format!("{}-{part}", self.options.id)
    }

    fn state(&self) -> &DateRangePickerState {
        self.session.state()
    }

    fn short(&self) -> bool {
        self.height < 480.
    }

    fn emit(&self, event: DateRangeEvent) -> M {
        (self.emit)(DateRangePickerInput::Date(event))
    }

    fn rtl(&self) -> bool {
        self.options.locale.direction == LayoutDirection::RightToLeft
    }

    fn row(&self, node: Node) -> Node {
        if self.rtl() {
            node.reverse_row()
        } else {
            node
        }
    }

    fn view(&self) -> View<M> {
        let validation = self
            .state()
            .validation(self.options.constraints, self.options.locale);
        let summary = (self.options.labels.summary)(
            validation.endpoint(DateRangeEndpoint::Start).ok(),
            validation.endpoint(DateRangeEndpoint::End).ok(),
        );
        let calendar = self.state().mode() == DateRangeMode::Calendar;
        let (body, focus) = if calendar {
            let active = self.active_day();
            (
                self.calendar(validation, active, &summary),
                active.map_or_else(|| self.id("calendar"), |day| self.id(&format!("day-{day}"))),
            )
        } else {
            (self.editors(validation, &summary), self.id("start-input"))
        };
        let (corners, color, scrim) = if self.fullscreen {
            (
                surface::Corners::all(0.),
                self.theme.colors.surface,
                Color::TRANSPARENT,
            )
        } else {
            let Color(r, g, b, _) = self.theme.colors.scrim;
            (
                shape::EXTRA_LARGE,
                self.theme.colors.surface_container_high,
                Color(r, g, b, 82),
            )
        };
        let panel = View::new(
            Node::new(Role::Dialog, self.options.labels.title)
                .id(self.options.id)
                .tab_index(-1)
                .styled(
                    Style::column()
                        .width(Length::Px(self.width))
                        .height(Length::Px(self.height))
                        .corners(corners)
                        .background(color)
                        .clip(true),
                ),
        )
        .children([self.header(calendar, validation, &summary), body])
        .modal_initial_focus(focus);
        let panel = if let Some(fallback) = self.options.restore_focus_to {
            panel.modal_focus_fallback(fallback)
        } else {
            panel
        };
        let mut style = Style::column()
            .fixed(Insets::all(0.))
            .align(Align::Center)
            .justify(Align::Center)
            .background(scrim);
        style.z_index = 1000;
        panel.modal_with_backdrop(
            Node::new(Role::Group, "")
                .id(self.id("layer"))
                .styled(style),
            self.emit(DateRangeEvent::Cancel),
        )
    }

    fn header(&self, calendar: bool, validation: DateRangeValidation, summary: &str) -> View<M> {
        let labels = self.options.labels;
        let mut style = Style::column().padding(8.);
        if !self.short() {
            style.min_height = if calendar { 128. } else { 120. };
        }
        let cancel = date_icon_button(
            self.theme,
            self.id("cancel"),
            labels.close,
            Glyph::Close,
            Some(self.emit(DateRangeEvent::Cancel)),
        );
        let (id, name, mode, glyph, focus) = if calendar {
            (
                "mode",
                labels.edit_mode,
                DateRangeMode::Input,
                Glyph::Edit,
                "start-input",
            )
        } else {
            (
                "calendar-mode",
                labels.calendar_mode,
                DateRangeMode::Calendar,
                Glyph::Calendar,
                "calendar",
            )
        };
        let mode = date_icon_button(
            self.theme,
            self.id(id),
            name,
            glyph,
            Some(self.emit(DateRangeEvent::SetMode(mode))),
        )
        .protect_composition_on_press()
        .restore_focus_on_removal(self.id(focus));
        let save = self
            .action_button(
                "confirm",
                labels.save,
                validation.candidate().map(|_| DateRangeEvent::Confirm),
            )
            .protect_composition_on_press();
        let actions = View::new(
            self.row(
                Node::new(Role::Group, "")
                    .id(self.id("actions"))
                    .styled(Style::row().align(Align::Center).wrap(true)),
            ),
        )
        .children([
            cancel,
            View::new(Node::new(Role::Group, "").styled(Style::column().grow(1.))),
            mode,
            save,
        ]);
        let mut children = vec![actions];
        if self.short() {
            // The short layout scrolls the visual summary. The live status
            // stays in the header so range edits can still be announced.
            children.push(View::new(
                Node::new(Role::Status, summary)
                    .id(self.id("summary"))
                    .styled(Style::column().width(Length::Px(1.)).height(Length::Px(1.))),
            ));
        } else {
            children.push(self.summary(calendar, validation, summary));
        }
        if calendar {
            children.push(self.month_controls());
        }
        View::new(
            Node::new(Role::Group, "")
                .id(self.id("header"))
                .styled(style),
        )
        .children(children)
    }

    fn summary(&self, calendar: bool, validation: DateRangeValidation, summary: &str) -> View<M> {
        let labels = self.options.labels;
        let (role, key) = if self.short() {
            (Role::Text, "summary-text")
        } else {
            (Role::Status, "summary")
        };
        let caption = View::new(
            Node::new(role, summary)
                .id(self.id(key))
                .text(summary)
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .with_text_style(typography::TITLE_LARGE)
                        .foreground(self.theme.colors.on_surface),
                ),
        );
        let caption = if self.short() {
            caption.without_interaction(None)
        } else {
            caption
        };
        let error = self
            .state()
            .displayed_pair_error(validation)
            .filter(|_| calendar)
            .map(|error| {
                text(
                    self.id("range-error"),
                    &(labels.pair_error)(error),
                    typography::BODY_SMALL,
                    self.theme.colors.error,
                )
            });
        let mut style = Style::column().width(Length::Percent(1.));
        if self.short() && calendar {
            // Keep the summary within the viewport on both horizontal edges.
            style = style
                .width(Length::Px(self.width.min(GRID_WIDTH)))
                .padding(8.);
        }
        View::new(Node::new(Role::Group, "").styled(style))
            .children(std::iter::once(caption).chain(error))
    }

    fn action_button(&self, id: &str, label: &str, event: Option<DateRangeEvent>) -> View<M> {
        let ink = if event.is_some() {
            self.theme.colors.primary
        } else {
            disabled_ink(self.theme)
        };
        let mut style = Style::row()
            .align(Align::Center)
            .justify(Align::Center)
            .state_layer(ink)
            .corners(shape::FULL)
            .padding(6.);
        style.min_height = TARGET;
        style.min_width = TARGET;
        style.max_width = Length::Px(self.width.max(TARGET));
        button(
            Node::new(Role::Button, label)
                .id(self.id(id))
                .focus_color(self.theme.colors.primary)
                .styled(style),
            event.map(|event| self.emit(event)),
        )
        .children([text("", label, typography::LABEL_LARGE, ink).without_interaction(None)])
    }

    fn month_controls(&self) -> View<M> {
        let labels = self.options.labels;
        let month = self.state().month();
        let bounds = self.options.constraints.dates.bounds;
        let move_button = |id, name, glyph, delta| {
            let event = month.step(delta).filter(|m| bounds.contains_month(*m));
            date_icon_button(
                self.theme,
                self.id(id),
                name,
                glyph,
                event.map(|m| self.emit(DateRangeEvent::BrowseTo(m))),
            )
        };
        let current = self.action_button(
            "calendar",
            labels.calendar,
            Some(DateRangeEvent::BrowseTo(month)),
        );
        let (prev, next) = if self.rtl() {
            (Glyph::Right, Glyph::Left)
        } else {
            (Glyph::Left, Glyph::Right)
        };
        View::new(
            self.row(
                Node::new(Role::Group, "")
                    .id(self.id("month-controls"))
                    .styled(
                        Style::row()
                            .align(Align::Center)
                            .justify(Align::Center)
                            .wrap(true),
                    ),
            ),
        )
        .children([
            move_button("previous-month", labels.previous_month, prev, -1),
            current,
            move_button("next-month", labels.next_month, next, 1),
        ])
    }

    fn active_day(&self) -> Option<CivilDate> {
        let constraints = self.options.constraints.dates;
        self.state()
            .active()
            .filter(|day| {
                constraints.allows(*day)
                    && self.session.mounted_months().contains(&day.year_month())
            })
            .or_else(|| {
                self.state()
                    .month()
                    .calendar_days(self.options.locale.first_weekday)
                    .into_iter()
                    .flatten()
                    .find(|day| constraints.allows(*day))
            })
    }

    fn calendar(
        &self,
        validation: DateRangeValidation,
        active: Option<CivilDate>,
        summary: &str,
    ) -> View<M> {
        let locale = self.options.locale;
        let emit = self.emit.clone();
        let emit: Rc<dyn Fn(DateRangeEvent) -> M> =
            Rc::new(move |event| emit(DateRangePickerInput::Date(event)));
        let fallback = self.id("calendar");
        let mut children = Vec::with_capacity(self.session.mounted_months().len() + 1);
        if self.short() {
            children.push(self.summary(true, validation, summary));
        }
        for month in self.session.mounted_months().iter().copied() {
            let key = month_id(self.options.id, month);
            let heading = (locale.month_title)(month);
            let leading = View::new(
                Node::new(Role::Group, "")
                    .styled(Style::column().width(Length::Px((GRID_WIDTH - self.width).max(0.)))),
            );
            let caption = text(
                "",
                &heading,
                typography::TITLE_SMALL,
                self.theme.colors.on_surface_variant,
            )
            .without_interaction(None);
            let title = View::new(
                self.row(
                    Node::new(Role::Heading, heading)
                        .id(format!("{key}-heading"))
                        .styled(Style::row().padding(8.)),
                ),
            )
            .children([leading, caption]);
            let grid = range_month(
                RangeMonth {
                    theme: self.theme,
                    id: self.options.id,
                    today: self.options.today,
                    constraints: self.options.constraints,
                    locale,
                    state: self.state(),
                    validation,
                    month,
                    active,
                    day_name: self.options.labels.day_name,
                    focus_fallback: &fallback,
                },
                emit.clone(),
            );
            children.push(
                View::new(
                    Node::new(Role::Group, "")
                        .id(key)
                        .styled(Style::column().width(Length::Px(GRID_WIDTH))),
                )
                .children([title, grid]),
            );
        }
        let wide = self.width >= GRID_WIDTH;
        let mut style = body_style();
        if wide {
            style.align = Align::Center;
        } else if self.rtl() {
            style.scroll_x = GRID_WIDTH - self.width;
        }
        let vertical = self.emit.clone();
        let wheel = self.emit.clone();
        let feed = View::new(Node::new(Role::Group, "").id(self.id("body")).styled(style))
            .children(children)
            .accessibility_scroll_pages()
            .on_vertical_drag(self.session.vertical_drag.clone(), move |event| {
                vertical(DateRangePickerInput::VerticalDrag(event))
            })
            .on_vertical_scroll(move |event| {
                Some(VerticalScrollResponse {
                    consumed: 0.,
                    message: wheel(DateRangePickerInput::Wheel(event)),
                })
            });
        if wide {
            feed
        } else {
            let horizontal = self.emit.clone();
            feed.on_horizontal_drag(self.session.horizontal_drag.clone(), move |event| {
                horizontal(DateRangePickerInput::HorizontalDrag(event))
            })
        }
    }

    fn editors(&self, validation: DateRangeValidation, summary: &str) -> View<M> {
        let labels = self.options.labels;
        let fields = [
            (DateRangeEndpoint::Start, labels.start),
            (DateRangeEndpoint::End, labels.end),
        ];
        let children = fields.into_iter().map(|(endpoint, label)| {
            let key = self.id(endpoint_name(endpoint));
            let locale = self.options.locale;
            let error = self
                .state()
                .displayed_field_error(endpoint, validation)
                .map(locale.error_text)
                .or_else(|| {
                    if endpoint == DateRangeEndpoint::End {
                        self.state()
                            .displayed_pair_error(validation)
                            .map(labels.pair_error)
                    } else {
                        None
                    }
                });
            let emit = self.emit.clone();
            let editor = date_editor(
                self.theme,
                DateEditorOptions {
                    id: &key,
                    label,
                    locale,
                    raw: self.state().raw_input(endpoint),
                    error,
                    style: Style::column().width(Length::Percent(1.)),
                },
                move |raw| {
                    emit(DateRangePickerInput::Date(DateRangeEvent::Edit(
                        endpoint, raw,
                    )))
                },
                self.emit(DateRangeEvent::FinishEditing(endpoint)),
            );
            View::new(Node::new(Role::Group, "").styled(Style::column().gap(4.))).children([
                editor.label,
                editor.input.restore_focus_on_removal(self.id("calendar")),
                editor.helper,
            ])
        });
        let emit = self.emit.clone();
        View::new(
            Node::new(Role::Group, "")
                .id(self.id("body"))
                .styled(body_style().padding(12.).gap(12.)),
        )
        .children(
            children.chain(
                self.short()
                    .then(|| self.summary(false, validation, summary)),
            ),
        )
        .accessibility_scroll_pages()
        .on_vertical_drag(self.session.vertical_drag.clone(), move |event| {
            emit(DateRangePickerInput::VerticalDrag(event))
        })
    }
}

fn endpoint_name(endpoint: DateRangeEndpoint) -> &'static str {
    match endpoint {
        DateRangeEndpoint::Start => "start",
        DateRangeEndpoint::End => "end",
    }
}

fn body_style() -> Style {
    Style::column()
        .width(Length::Percent(1.))
        .height(Length::Px(0.))
        .grow(1.)
        .clip(true)
}
