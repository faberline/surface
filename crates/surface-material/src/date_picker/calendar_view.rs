use surface::{Align, Color, GridRole, Length, Node, Role, Style};
use surface_app::{EditKey, FocusNavigation, Roving, View};

use crate::{
    shape, typography, vector_icon::VectorIconSize, CivilDate, DatePickerEvent, DatePickerMode,
    DatePickerStep, StepDirection,
};

use super::{button, centered, text, Glyph, Picker, TARGET};

const WEEK_LENGTH: usize = 7;
const GRID_WIDTH: f32 = TARGET * WEEK_LENGTH as f32;
const DAY_DIAMETER: f32 = 40.;
const YEAR_PAGE: u16 = 72;
const YEAR_PILL_WIDTH: f32 = 72.;
const YEAR_PILL_HEIGHT: f32 = 36.;

#[derive(Clone, Copy)]
struct YearPage {
    first: u16,
    last: u16,
    min: u16,
    max: u16,
    columns: usize,
}

impl<M: Clone + 'static> Picker<'_, '_, M> {
    pub(super) fn active_day(&self) -> Option<CivilDate> {
        let allowed = |date: &CivilDate| self.options.constraints.allows(*date);
        self.session.active().filter(allowed).or_else(|| {
            self.session
                .month()
                .calendar_days(self.options.locale.first_weekday)
                .into_iter()
                .flatten()
                .find(allowed)
        })
    }

    pub(super) fn calendar(&self) -> View<M> {
        let month = self.session.month();
        let active = self.active_day();
        let title = (self.options.locale.month_title)(month);
        let days = month.calendar_days(self.options.locale.first_weekday);
        let mut grid_rows = Vec::with_capacity(7);
        grid_rows.push(self.weekdays());
        grid_rows.extend(
            days.chunks_exact(WEEK_LENGTH)
                .enumerate()
                .map(|(week, dates)| {
                    let cells = dates.iter().copied().enumerate().map(|(weekday, date)| {
                        if let Some(date) = date {
                            self.day(date, active == Some(date))
                        } else {
                            View::new(
                                Node::new(Role::Group, "")
                                    .id(self.id(&format!("blank-{}", week * WEEK_LENGTH + weekday)))
                                    .grid_role(GridRole::GridCell)
                                    .styled(centered(TARGET, TARGET)),
                            )
                        }
                    });
                    View::new(
                        self.row(
                            Node::new(Role::Group, "")
                                .id(self.id(&format!("week-{week}")))
                                .grid_role(GridRole::Row)
                                .styled(Style::row().width(Length::Px(GRID_WIDTH))),
                        ),
                    )
                    .children(cells)
                }),
        );
        let grid = View::new(
            Node::new(Role::Group, &title)
                .id(self.id("grid"))
                .grid_role(GridRole::Grid)
                .styled(Style::column().width(Length::Px(GRID_WIDTH))),
        )
        .children(grid_rows)
        .roving(Roving::horizontal(false));
        let wide = self.width >= GRID_WIDTH;
        let mut clip = Style::row().width(Length::Percent(1.)).justify(if wide {
            Align::Center
        } else {
            Align::Start
        });
        clip = clip.clip(!wide);
        if self.rtl() && !wide {
            clip.scroll_x = GRID_WIDTH - self.width;
        }
        let viewport = View::new(
            Node::new(Role::Group, "")
                .id(self.id("grid-scroll"))
                .styled(clip),
        )
        .children([grid])
        .accessibility_scroll_pages();
        View::new(Node::new(Role::Group, "").styled(Style::column()))
            .children([self.month_controls(&title, false), viewport])
    }

    fn weekdays(&self) -> View<M> {
        let locale = self.options.locale;
        let columns = (0..WEEK_LENGTH).map(|column| {
            let index = (locale.first_weekday.index() + column) % WEEK_LENGTH;
            let name = locale.weekday_full[index];
            let caption = text(
                "",
                locale.weekday_short[index],
                typography::BODY_LARGE,
                self.theme.colors.on_surface_variant,
            )
            .without_interaction(None);
            let mut style = Style::column()
                .width(Length::Px(TARGET))
                .align(Align::Center)
                .justify(Align::Center);
            style.min_height = 24.;
            View::new(
                Node::new(Role::Group, name)
                    .id(self.id(&format!("weekday-{column}")))
                    .grid_role(GridRole::ColumnHeader)
                    .styled(style),
            )
            .children([caption])
        });
        View::new(
            self.row(
                Node::new(Role::Group, "")
                    .id(self.id("weekdays"))
                    .grid_role(GridRole::Row)
                    .styled(Style::row().width(Length::Px(GRID_WIDTH))),
            ),
        )
        .children(columns)
    }

    fn day(&self, date: CivilDate, active: bool) -> View<M> {
        let enabled = self.options.constraints.allows(date);
        let selected = enabled && self.session.draft() == Some(date);
        let today = self.options.today == date;
        let id = self.id(&format!("day-{date}"));
        let mut node = Node::new(Role::Button, (self.options.locale.date_name)(date))
            .id(&id)
            .tab_index(if active { 0 } else { -1 })
            .grid_role(GridRole::GridCell)
            .current_date(today)
            .focus_color(self.theme.colors.secondary)
            .styled(centered(TARGET, TARGET).corners(shape::FULL));
        if enabled {
            node = node.selected(selected);
        }
        let mut style = centered(DAY_DIAMETER, DAY_DIAMETER);
        if today && !selected {
            style = style.border(1., self.theme.colors.primary);
        }
        let label = date.day().to_string();
        let paint = self.date_face(&id, &label, style, enabled, selected, today);
        let view = button(
            node,
            enabled.then(|| (self.emit)(DatePickerEvent::Select(date))),
        )
        .children([paint]);
        if enabled {
            view.on_focus_navigation_with_extend(self.day_navigation(date, id))
                .accessibility_scroll_into_view()
                .restore_focus_on_removal(self.id("month"))
        } else {
            view
        }
    }

    fn date_face(
        &self,
        id: &str,
        label: &str,
        style: Style,
        enabled: bool,
        selected: bool,
        today: bool,
    ) -> View<M> {
        let colors = self.theme.colors;
        let ink = if !enabled {
            self.disabled_ink()
        } else if selected {
            colors.on_primary
        } else if today {
            colors.primary
        } else {
            colors.on_surface
        };
        let fill = if selected {
            colors.primary
        } else {
            Color::TRANSPARENT
        };
        let layer = if enabled { ink } else { Color::TRANSPARENT };
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-face"))
                .state_layer_alphas(20, 26, 26)
                .styled(
                    style
                        .corners(shape::FULL)
                        .background(fill)
                        .state_layer(layer),
                ),
        )
        .children([text("", label, typography::BODY_LARGE, ink)])
        .without_interaction(None)
    }

    fn day_navigation(
        &self,
        date: CivilDate,
        current: String,
    ) -> impl Fn(EditKey, bool) -> Option<FocusNavigation<M>> + 'static {
        let steps = [
            DatePickerStep::Day(StepDirection::Backward),
            DatePickerStep::Day(StepDirection::Forward),
            DatePickerStep::Week(StepDirection::Backward),
            DatePickerStep::Week(StepDirection::Forward),
            DatePickerStep::WeekStart,
            DatePickerStep::WeekEnd,
            DatePickerStep::Month(StepDirection::Backward),
            DatePickerStep::Month(StepDirection::Forward),
            DatePickerStep::Year(StepDirection::Backward),
            DatePickerStep::Year(StepDirection::Forward),
        ];
        let targets = steps.map(|step| {
            self.session
                .navigation_target_from(date, step, self.options.constraints, self.options.locale)
                .map(|target| self.id(&format!("day-{target}")))
        });
        let emit = self.emit.clone();
        let rtl = self.rtl();
        move |key, extend| {
            let index = match (key, extend) {
                (EditKey::Left, _) => usize::from(rtl),
                (EditKey::Right, _) => usize::from(!rtl),
                (EditKey::Up, _) => 2,
                (EditKey::Down, _) => 3,
                (EditKey::Home, _) => 4,
                (EditKey::End, _) => 5,
                (EditKey::PageUp, false) => 6,
                (EditKey::PageDown, false) => 7,
                (EditKey::PageUp, true) => 8,
                (EditKey::PageDown, true) => 9,
                _ => return None,
            };
            Some(FocusNavigation {
                target: targets[index].clone().unwrap_or_else(|| current.clone()),
                message: emit(DatePickerEvent::NavigateFrom(date, steps[index])),
            })
        }
    }

    fn month_controls(&self, title: &str, show_year: bool) -> View<M> {
        let labels = self.options.labels;
        let month = self.session.month();

        let name = format!("{} {title}", labels.choose_year);
        let caption = text(
            "",
            title,
            typography::LABEL_LARGE,
            self.theme.colors.on_surface_variant,
        );
        let glyph = if show_year { Glyph::Up } else { Glyph::Down };
        let icon = glyph
            .vector()
            .view(VectorIconSize::Dp24, self.theme.colors.on_surface_variant);
        let mut style = Style::row()
            .width(Length::Percent(1.))
            .align(Align::Center)
            .justify(Align::Center)
            .corners(shape::FULL)
            .gap(2.)
            .state_layer(self.theme.colors.on_surface_variant);
        style.min_width = TARGET;
        style.min_height = TARGET;
        let mode = if show_year {
            DatePickerMode::Calendar
        } else {
            DatePickerMode::Year
        };
        let title_control = button(
            self.row(
                Node::new(Role::Button, name)
                    .id(self.id("month"))
                    .styled(style),
            ),
            Some((self.emit)(DatePickerEvent::SetMode(mode))),
        )
        .children([
            caption.without_interaction(None),
            icon.without_interaction(None),
        ])
        .disclosure_expanded(show_year);
        let status = View::new(
            Node::new(Role::Status, title)
                .id(self.id("month-status"))
                .styled(Style::column().width(Length::Px(0.)).grow(1.)),
        )
        .children([title_control]);
        let controls = if show_year {
            vec![status]
        } else {
            let min_month = self.options.constraints.bounds.min().year_month();
            let max_month = self.options.constraints.bounds.max().year_month();
            let (left, right) = self.arrows();
            let previous = (month > min_month)
                .then_some(DatePickerEvent::BrowseMonth(StepDirection::Backward));
            let next =
                (month < max_month).then_some(DatePickerEvent::BrowseMonth(StepDirection::Forward));
            vec![
                self.icon_button(self.id("prev"), labels.previous_month, left, previous),
                status,
                self.icon_button(self.id("next"), labels.next_month, right, next),
            ]
        };
        View::new(
            self.row(
                Node::new(Role::Group, "")
                    .id(self.id("month-controls"))
                    .styled(Style::row().align(Align::Center)),
            ),
        )
        .children(controls)
    }

    fn arrows(&self) -> (Glyph, Glyph) {
        if self.rtl() {
            (Glyph::Right, Glyph::Left)
        } else {
            (Glyph::Left, Glyph::Right)
        }
    }

    pub(super) fn years(&self) -> View<M> {
        let month = self.session.month();
        let title = (self.options.locale.month_title)(month);
        let current = month.year();
        let min = self.options.constraints.bounds.min().year();
        let max = self.options.constraints.bounds.max().year();
        let first = min + current.saturating_sub(min) / YEAR_PAGE * YEAR_PAGE;
        let last = max.min(first + YEAR_PAGE - 1);
        let viewport_width = self.width.max(TARGET);
        let columns = if viewport_width >= 240. {
            3
        } else if viewport_width >= 160. {
            2
        } else {
            1
        };
        let page = YearPage {
            first,
            last,
            min,
            max,
            columns,
        };
        let year_width = (viewport_width / columns as f32).clamp(TARGET, 96.);
        let years: Vec<_> = (first..=last).collect();
        let rows = years.chunks(columns).map(|row| {
            View::new(
                self.row(Node::new(Role::Group, "").styled(Style::row().justify(Align::Center))),
            )
            .children(
                row.iter()
                    .copied()
                    .map(|year| self.year(year, year_width, current == year, page)),
            )
        });
        let years = View::new(
            Node::new(Role::Group, self.options.labels.choose_year)
                .id(self.id("years"))
                .styled(Style::column()),
        )
        .children(rows)
        .roving(Roving::horizontal(false));
        View::new(Node::new(Role::Group, "").styled(Style::column())).children([
            self.month_controls(&title, true),
            self.year_navigation(page),
            years,
        ])
    }

    fn year_navigation(&self, page: YearPage) -> View<M> {
        let YearPage {
            first,
            last,
            min,
            max,
            ..
        } = page;
        let (left, right) = self.arrows();
        let previous = (first > min).then(|| {
            let next_first = first.saturating_sub(YEAR_PAGE).max(min);
            (next_first, first - 1)
        });
        let next = (last < max).then(|| (last + 1, (last + YEAR_PAGE).min(max)));
        let pager = |id: &str, icon, range: Option<(u16, u16)>| {
            let label = if let Some((start, end)) = range {
                format!("{} {start:04}–{end:04}", self.options.labels.choose_year)
            } else {
                self.options.labels.choose_year.to_owned()
            };
            self.icon_button(
                self.id(id),
                &label,
                icon,
                range.map(|(year, _)| DatePickerEvent::BrowseYears(year)),
            )
        };
        let label = format!("{first:04}–{last:04}");
        let caption = text(
            self.id("year-range"),
            &label,
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
                pager("year-prev", left, previous),
                range,
                pager("year-next", right, next),
            ])
    }

    fn year(&self, year: u16, width: f32, selected: bool, page: YearPage) -> View<M> {
        let label = format!("{year:04}");
        let id = self.id(&format!("year-{year:04}"));
        let today = self.options.today.year() == year;
        let target = Style::row()
            .width(Length::Px(width))
            .align(Align::Center)
            .justify(Align::Center);
        let mut target = target.corners(shape::FULL);
        target.min_height = TARGET;
        let mut visual = Style::row()
            .align(Align::Center)
            .justify(Align::Center)
            .padding(4.);
        visual.min_height = YEAR_PILL_HEIGHT;
        visual.min_width = YEAR_PILL_WIDTH.min(width);
        visual.max_width = Length::Px(width);
        if today && !selected {
            visual = visual.border(1., self.theme.colors.primary);
        }
        let face = self.date_face(&id, &label, visual, true, selected, today);
        button(
            Node::new(Role::Button, &label)
                .id(id)
                .selected(selected)
                .focus_color(self.theme.colors.secondary)
                .styled(target),
            Some((self.emit)(DatePickerEvent::ChooseYear(year))),
        )
        .children([face])
        .on_focus_navigation(self.year_focus(year, page))
        .accessibility_scroll_into_view()
        .restore_focus_on_removal(self.id("month"))
    }

    fn year_focus(
        &self,
        year: u16,
        page: YearPage,
    ) -> impl Fn(EditKey) -> Option<FocusNavigation<M>> + 'static {
        let emit = self.emit.clone();
        let id = self.options.id.to_owned();
        let horizontal = if self.rtl() { -1 } else { 1 };
        move |key| {
            let offset = match key {
                EditKey::Left => -horizontal,
                EditKey::Right => horizontal,
                EditKey::Up => -(page.columns as i32),
                EditKey::Down => page.columns as i32,
                EditKey::Home => i32::from(page.first) - i32::from(year),
                EditKey::End => i32::from(page.last) - i32::from(year),
                EditKey::PageUp => -i32::from(YEAR_PAGE),
                EditKey::PageDown => i32::from(YEAR_PAGE),
                _ => return None,
            };
            let wanted = i32::from(year) + offset;
            let target = wanted.clamp(i32::from(page.min), i32::from(page.max)) as u16;
            let event = if (page.first..=page.last).contains(&target) {
                DatePickerEvent::SetMode(DatePickerMode::Year)
            } else {
                DatePickerEvent::BrowseYears(target)
            };
            Some(FocusNavigation {
                target: format!("{id}-year-{target:04}"),
                message: emit(event),
            })
        }
    }
}
