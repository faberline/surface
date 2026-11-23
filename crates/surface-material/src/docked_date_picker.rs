//! The anchored Material 3 single-date picker.

use std::rc::Rc;

use surface::{Align, Color, Insets, Length, Node, Rect, Role, Style};
use surface_app::{Roving, View};

use crate::{
    date_picker::{
        button,
        calendar_view::{CalendarGrid, YEAR_PAGE},
        date_editor, date_icon_button, disabled_ink, text, DateEditorOptions, Glyph, TARGET,
    },
    docked_date_picker_state::DOCKED_HEADER_HEIGHT,
    place_docked_date_picker,
    shape::{self, LayoutDirection},
    typography, CivilDate, DatePickerConstraints, DatePickerEvent, DatePickerLocale,
    DockedDatePickerEvent, DockedDatePickerMode, DockedDatePickerState, StepDirection, Theme,
    VectorIcon, YearMonth,
};

/// Localized names for the docked field, calendar button, and calendar controls.
#[derive(Clone, Copy)]
pub struct DockedDatePickerLabels<'a> {
    pub input_name: &'a str,
    pub calendar: &'a str,
    pub previous_month: &'a str,
    pub next_month: &'a str,
    pub choose_month: &'a str,
    pub choose_year: &'a str,
    /// The display name of a month without its year.
    pub month_name: &'a dyn Fn(YearMonth) -> String,
}

/// The app supplies the most recent visible field bounds.
#[derive(Clone, Copy)]
pub struct DockedDatePickerOptions<'a> {
    pub id: &'a str,
    pub today: CivilDate,
    pub viewport: Rect,
    /// Measure the field group with stable ID ending in -field.
    pub anchor: Option<Rect>,
    pub constraints: DatePickerConstraints<'a>,
    pub locale: &'a DatePickerLocale<'a>,
    pub labels: &'a DockedDatePickerLabels<'a>,
    pub restore_focus_to: Option<&'a str>,
}

impl Theme {
    /// Draw a docked field and an independently focusable calendar button.
    ///
    /// The caller keeps the state and dispatches its events. Use
    /// [DockedDatePickerState::observe_input_focus] to submit on ordinary
    /// field exit. Recompute the anchor after layout, and dismiss the popup
    /// if the field leaves the viewport or the popup no longer fits.
    ///
    /// The supplied ID remains the bounded field owner in both states.
    /// It owns the modal scope while the positioned popup is visible.
    /// The field, input, calendar button and popup append -field, -input,
    /// -calendar-trigger and -popup. Month/year list items append -month-MM
    /// and -year-YYYY. Day IDs use -day-YYYY-MM-DD.
    pub fn docked_date_picker<M: Clone + 'static>(
        &self,
        options: DockedDatePickerOptions<'_>,
        state: &DockedDatePickerState,
        on_event: impl Fn(DockedDatePickerEvent) -> M + 'static,
    ) -> View<M> {
        Docked {
            theme: self,
            options,
            state,
            emit: Rc::new(on_event),
        }
        .view()
    }
}

struct Docked<'a, 'b, M> {
    theme: &'a Theme,
    options: DockedDatePickerOptions<'b>,
    state: &'a DockedDatePickerState,
    emit: Rc<dyn Fn(DockedDatePickerEvent) -> M>,
}

impl<M: Clone + 'static> Docked<'_, '_, M> {
    fn id(&self, suffix: &str) -> String {
        format!("{}-{suffix}", self.options.id)
    }

    fn row(&self, node: Node) -> Node {
        if self.options.locale.direction == LayoutDirection::RightToLeft {
            node.reverse_row()
        } else {
            node
        }
    }

    fn view(&self) -> View<M> {
        let popup = self
            .state
            .mode()
            .zip(self.options.anchor.and_then(|anchor| {
                place_docked_date_picker(
                    anchor,
                    self.options.viewport,
                    self.options.locale.direction,
                )
            }));
        let field = self.field(popup.is_some());
        let root = |role, name| {
            Node::new(role, name)
                .id(self.options.id)
                .styled(Style::column().width(Length::Percent(1.)))
        };
        let Some((mode, bounds)) = popup else {
            return View::new(root(Role::Group, "")).children([field]);
        };
        let (popup, initial_focus) = self.popup(mode, bounds);
        View::new(root(Role::Dialog, self.options.labels.calendar))
            .children([field, popup])
            .modal_overlay((self.emit)(DockedDatePickerEvent::Dismiss))
            .modal_initial_focus(initial_focus)
            .modal_focus_fallback(
                self.options
                    .restore_focus_to
                    .map(str::to_owned)
                    .unwrap_or_else(|| self.id("calendar-trigger")),
            )
    }

    fn field(&self, expanded: bool) -> View<M> {
        let edit = self.emit.clone();
        let editor = date_editor(
            self.theme,
            DateEditorOptions {
                id: self.options.id,
                label: self.options.labels.input_name,
                locale: self.options.locale,
                raw: self.state.raw_input(),
                error: self.state.error(),
                style: Style::column().width(Length::Px(0.)).grow(1.),
            },
            move |value| edit(DockedDatePickerEvent::Edit(value)),
            (self.emit)(DockedDatePickerEvent::FinishEditing),
        );
        let trigger = self
            .icon(
                "calendar-trigger",
                self.options.labels.calendar,
                Glyph::Calendar,
                Some(DockedDatePickerEvent::ToggleCalendar),
            )
            .disclosure_expanded(expanded)
            .focus_on_pointer_release()
            .protect_composition_on_press();
        let trigger = if let Some(fallback) = self.options.restore_focus_to {
            trigger.restore_focus_on_removal(fallback)
        } else {
            trigger
        };
        let row = View::new(
            self.row(
                Node::new(Role::Group, "")
                    .id(self.id("input-row"))
                    .styled(Style::row().gap(4.).align(Align::Center)),
            ),
        )
        .children([editor.input, trigger]);
        View::new(
            Node::new(Role::Group, "")
                .id(self.id("field"))
                .styled(Style::column().gap(4.)),
        )
        .children([editor.label, row, editor.helper])
    }

    fn popup(&self, mode: DockedDatePickerMode, bounds: Rect) -> (View<M>, String) {
        let month = self.state.calendar().month();
        let (header, content, initial_focus) = match mode {
            DockedDatePickerMode::Calendar => {
                let emit = self.emit.clone();
                let grid = CalendarGrid {
                    theme: self.theme,
                    id: self.options.id,
                    today: self.options.today,
                    constraints: self.options.constraints,
                    locale: self.options.locale,
                    session: self.state.calendar(),
                    emit: Rc::new(move |event| emit(DockedDatePickerEvent::Calendar(event))),
                    width: bounds.width,
                    adjacent_dates: true,
                };
                let focus = grid
                    .active_day()
                    .map_or_else(|| self.id("month"), |date| self.id(&format!("day-{date}")));
                let title = (self.options.locale.month_title)(month);
                (
                    self.calendar_header(month, &title),
                    grid.view(&title),
                    focus,
                )
            }
            DockedDatePickerMode::Month => (
                self.list_header("month", self.options.labels.choose_month),
                self.month_list(month),
                self.month_id(month.month()),
            ),
            DockedDatePickerMode::Year => (
                self.list_header("year", self.options.labels.choose_year),
                self.year_list(month),
                self.year_id(month.year()),
            ),
        };
        let body = View::new(
            Node::new(Role::Group, "")
                .id(self.id("body"))
                .styled(Style::column().height(Length::Px(0.)).grow(1.).clip(true)),
        )
        .children([content])
        .accessibility_scroll_pages();
        let mut style = Style::column()
            .width(Length::Px(bounds.width))
            .height(Length::Px(bounds.height))
            .background(self.theme.colors.surface_container_high)
            .corners(shape::LARGE)
            .clip(true)
            .fixed(Insets {
                left: Length::Px(bounds.x),
                top: Length::Px(bounds.y),
                ..Insets::default()
            });
        style.z_index = 1000;
        let pane = View::new(
            Node::new(Role::Group, "")
                .id(self.id("popup"))
                .styled(style),
        )
        .children([header, body]);
        (pane, initial_focus)
    }

    fn arrows(&self) -> (Glyph, Glyph) {
        if self.options.locale.direction == LayoutDirection::RightToLeft {
            (Glyph::Right, Glyph::Left)
        } else {
            (Glyph::Left, Glyph::Right)
        }
    }

    fn icon(
        &self,
        suffix: &str,
        label: &str,
        icon: Glyph,
        event: Option<DockedDatePickerEvent>,
    ) -> View<M> {
        date_icon_button(
            self.theme,
            self.id(suffix),
            label,
            icon,
            event.map(|event| (self.emit)(event)),
        )
    }

    fn month_id(&self, month: u8) -> String {
        self.id(&format!("month-{month:02}"))
    }

    fn year_id(&self, year: u16) -> String {
        self.id(&format!("year-{year:04}"))
    }

    fn calendar_header(&self, month: YearMonth, title: &str) -> View<M> {
        let labels = self.options.labels;
        let (left, right) = self.arrows();
        let min_month = self.options.constraints.bounds.min().year_month();
        let max_month = self.options.constraints.bounds.max().year_month();
        let move_month =
            |direction| DockedDatePickerEvent::Calendar(DatePickerEvent::BrowseMonth(direction));
        let previous = self
            .icon(
                "prev",
                labels.previous_month,
                left,
                (month > min_month).then(|| move_month(StepDirection::Backward)),
            )
            .restore_focus_on_removal(self.id("month"));
        let next = self
            .icon(
                "next",
                labels.next_month,
                right,
                (month < max_month).then(|| move_month(StepDirection::Forward)),
            )
            .restore_focus_on_removal(self.id("month"));
        let month_name = (labels.month_name)(month);
        let month_menu = self
            .selector(
                "month",
                labels.choose_month,
                &month_name,
                DockedDatePickerMode::Month,
            )
            .restore_focus_on_removal(self.month_id(month.month()));
        let year_menu = self
            .selector(
                "year",
                labels.choose_year,
                &format!("{:04}", month.year()),
                DockedDatePickerMode::Year,
            )
            .restore_focus_on_removal(self.year_id(month.year()));
        let status = View::new(
            self.row(
                Node::new(Role::Status, title)
                    .id(self.id("month-status"))
                    .styled(
                        Style::row()
                            .width(Length::Px(0.))
                            .grow(1.)
                            .align(Align::Center),
                    ),
            ),
        )
        .children([month_menu, year_menu]);
        self.header([previous, status, next])
    }

    fn header(&self, children: impl IntoIterator<Item = View<M>>) -> View<M> {
        let mut style = Style::row().align(Align::Center);
        style.min_height = DOCKED_HEADER_HEIGHT;
        View::new(
            self.row(
                Node::new(Role::Group, "")
                    .id(self.id("header"))
                    .styled(style),
            ),
        )
        .children(children)
    }

    fn list_header(&self, return_to: &str, label: &str) -> View<M> {
        let back = self
            .icon(
                "list-back",
                self.options.labels.calendar,
                self.arrows().0,
                Some(DockedDatePickerEvent::SetMode(
                    DockedDatePickerMode::Calendar,
                )),
            )
            .restore_focus_on_removal(self.id(return_to));
        let label = text(
            self.id("list-title"),
            label,
            typography::LABEL_LARGE,
            self.theme.colors.on_surface_variant,
        );
        self.header([back, label])
    }

    fn selector(
        &self,
        suffix: &str,
        label: &str,
        value: &str,
        mode: DockedDatePickerMode,
    ) -> View<M> {
        let name = format!("{label} {value}");
        let ink = self.theme.colors.on_surface_variant;
        let caption = text("", value, typography::LABEL_LARGE, ink);
        let arrow = self.glyph(Glyph::Down.vector(), 18., ink);
        let mut visual = Style::row()
            .width(Length::Percent(1.))
            .padding(2.)
            .gap(2.)
            .align(Align::Center)
            .justify(Align::Center)
            .corners(shape::FULL)
            .state_layer(ink);
        visual.min_height = 40.;
        let paint = View::new(self.row(Node::new(Role::Group, "").styled(visual)))
            .children([caption, arrow])
            .without_interaction(None);
        let mut target = Style::row()
            .width(Length::Px(0.))
            .grow(1.)
            .align(Align::Center)
            .justify(Align::Center)
            .corners(shape::FULL);
        target.min_width = TARGET;
        target.min_height = TARGET;
        button(
            Node::new(Role::Button, name)
                .id(self.id(suffix))
                .focus_color(self.theme.colors.primary)
                .styled(target),
            Some((self.emit)(DockedDatePickerEvent::SetMode(mode))),
        )
        .children([paint])
        .disclosure_expanded(false)
    }

    fn glyph(&self, icon: VectorIcon, size: f32, color: Color) -> View<M> {
        let fill = icon
            .contour_dp(size, color)
            .expect("validated date picker icon");
        View::new(
            Node::new(Role::Group, "")
                .styled(
                    Style::row()
                        .width(Length::Px(size))
                        .height(Length::Px(size)),
                )
                .fill_contour(fill),
        )
        .without_interaction(None)
    }

    fn month_list(&self, current: YearMonth) -> View<M> {
        let min_month = self.options.constraints.bounds.min().year_month();
        let max_month = self.options.constraints.bounds.max().year_month();
        let choices = (1..=12).map(|number| {
            let month =
                YearMonth::new(current.year(), number).expect("valid month in current year");
            let label = (self.options.labels.month_name)(month);
            self.option(
                self.month_id(number),
                &label,
                month == current,
                (min_month..=max_month).contains(&month).then_some(month),
                "month",
            )
        });
        self.list("month-list", self.options.labels.choose_month, choices)
    }

    fn year_list(&self, current: YearMonth) -> View<M> {
        let min_month = self.options.constraints.bounds.min().year_month();
        let max_month = self.options.constraints.bounds.max().year_month();
        let min = min_month.year();
        let max = max_month.year();
        let first = min + current.year().saturating_sub(min) / YEAR_PAGE * YEAR_PAGE;
        let last = max.min(first + YEAR_PAGE - 1);
        let choices = (first..=last).map(|year| {
            let month = YearMonth::new(year, current.month())
                .expect("valid year and current month")
                .clamp(min_month, max_month);
            self.option(
                self.year_id(year),
                &format!("{year:04}"),
                year == current.year(),
                Some(month),
                "year",
            )
        });
        View::new(Node::new(Role::Group, "").styled(Style::column())).children([
            self.year_pager(first, last, min, max),
            self.list("year-list", self.options.labels.choose_year, choices),
        ])
    }

    fn year_pager(&self, first: u16, last: u16, min: u16, max: u16) -> View<M> {
        let previous = (first > min).then(|| (first.saturating_sub(YEAR_PAGE).max(min), first - 1));
        let next = (last < max).then(|| (last + 1, (last + YEAR_PAGE).min(max)));
        let (left, right) = self.arrows();
        let page = |id, icon, range: Option<(u16, u16)>| {
            let label = match range {
                Some((start, end)) => {
                    format!("{} {start:04}–{end:04}", self.options.labels.choose_year)
                }
                None => self.options.labels.choose_year.to_owned(),
            };
            self.icon(
                id,
                &label,
                icon,
                range.map(|(year, _)| {
                    DockedDatePickerEvent::Calendar(DatePickerEvent::BrowseYears(year))
                }),
            )
            .restore_focus_on_removal(
                self.year_id(range.map_or(self.state.calendar().month().year(), |(year, _)| year)),
            )
        };
        let caption = text(
            self.id("year-range"),
            &format!("{first:04}–{last:04}"),
            typography::BODY_MEDIUM,
            self.theme.colors.on_surface_variant,
        );
        let range = View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Px(0.))
                    .grow(1.)
                    .align(Align::Center),
            ),
        )
        .children([caption]);
        View::new(self.row(Node::new(Role::Group, "").styled(Style::row().align(Align::Center))))
            .children([
                page("year-prev", left, previous),
                range,
                page("year-next", right, next),
            ])
    }

    fn list(
        &self,
        suffix: &str,
        label: &str,
        choices: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        View::new(
            Node::new(Role::Group, label)
                .id(self.id(suffix))
                .styled(Style::column().padding(4.)),
        )
        .children(choices)
        .roving(Roving::vertical(false))
    }

    fn option(
        &self,
        id: String,
        label: &str,
        selected: bool,
        target: Option<YearMonth>,
        return_to: &str,
    ) -> View<M> {
        let color = if target.is_none() {
            disabled_ink(self.theme)
        } else {
            self.theme.colors.on_surface
        };
        let mut style = Style::row()
            .padding(8.)
            .gap(8.)
            .align(Align::Center)
            .corners(shape::FULL)
            .state_layer(color);
        style.min_height = TARGET;
        let caption = text("", label, typography::BODY_LARGE, color).without_interaction(None);
        let marker = if selected {
            let check = VectorIcon::new([
                [4., 12.],
                [9., 17.],
                [20., 6.],
                [22., 8.],
                [9., 21.],
                [2., 14.],
            ])
            .expect("date picker check icon");
            self.glyph(check, 24., self.theme.colors.primary)
        } else {
            View::new(Node::new(Role::Group, "").styled(Style::row().width(Length::Px(24.))))
        };
        let mut node = Node::new(Role::Button, label)
            .id(id)
            .focus_color(self.theme.colors.primary)
            .styled(style);
        if target.is_some() {
            node = node.selected(selected);
        }
        button(
            self.row(node),
            target.map(|month| (self.emit)(DockedDatePickerEvent::BrowseTo(month))),
        )
        .children([marker, caption])
        .accessibility_scroll_into_view()
        .restore_focus_on_removal(self.id(return_to))
    }
}
