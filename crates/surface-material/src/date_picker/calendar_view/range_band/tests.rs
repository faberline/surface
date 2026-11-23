use super::{range_band, WeekRangeBand, WEEK_LENGTH};
use crate::{
    shape::LayoutDirection::{LeftToRight, RightToLeft},
    CivilDate, CivilDateRange, Weekday, YearMonth,
};

fn date(year: u16, month: u8, day: u8) -> CivilDate {
    CivilDate::new(year, month, day).unwrap()
}

fn feb(day: u8) -> CivilDate {
    date(2024, 2, day)
}

fn range(start: CivilDate, end: CivilDate) -> Option<CivilDateRange> {
    Some(CivilDateRange::new(start, end).unwrap())
}

fn week(
    month: YearMonth,
    first_weekday: Weekday,
    index: usize,
) -> [Option<CivilDate>; WEEK_LENGTH] {
    let days = month.calendar_days(first_weekday);
    days[index * WEEK_LENGTH..][..WEEK_LENGTH]
        .try_into()
        .unwrap()
}

fn expected(x: f32, width: f32) -> Option<WeekRangeBand> {
    Some(WeekRangeBand {
        x,
        y: 4.,
        width,
        height: 40.,
    })
}

fn check(
    month: YearMonth,
    week: &[Option<CivilDate>; WEEK_LENGTH],
    range: Option<CivilDateRange>,
    x: f32,
    width: f32,
) {
    assert_eq!(
        range_band(month, week, range, LeftToRight),
        expected(x, width)
    );
    assert_eq!(
        range_band(month, week, range, RightToLeft),
        expected(336. - x - width, width)
    );
}

#[test]
fn within_one_week_joins_endpoint_centers() {
    let month = feb(1).year_month();
    let week = week(month, Weekday::Monday, 1);
    check(month, &week, range(feb(6), feb(9)), 72., 144.);
}

#[test]
fn across_week_break_draws_half_cells_for_endpoints_at_the_break() {
    let month = feb(1).year_month();
    let range = range(feb(11), feb(12));
    check(month, &week(month, Weekday::Monday, 1), range, 312., 24.);
    check(month, &week(month, Weekday::Monday, 2), range, 0., 24.);
}

#[test]
fn middle_week_is_continuous_even_without_either_endpoint() {
    let month = feb(1).year_month();
    let range = range(feb(8), feb(23));
    check(month, &week(month, Weekday::Monday, 2), range, 0., 336.);
}

#[test]
fn cross_month_band_does_not_fill_adjacent_calendar_cells() {
    let month = feb(1).year_month();
    let range = range(date(2024, 1, 28), date(2024, 3, 5));
    let visible_first_week = std::array::from_fn(|i| date(2024, 1, 29).checked_add_days(i as i32));
    let visible_last_week = std::array::from_fn(|i| feb(26).checked_add_days(i as i32));

    check(month, &week(month, Weekday::Monday, 0), range, 144., 192.);
    check(month, &visible_first_week, range, 144., 192.);
    check(month, &visible_last_week, range, 0., 192.);
}

#[test]
fn cross_month_endpoint_centers_stay_in_their_own_month() {
    let january = date(2024, 1, 1).year_month();
    let february = feb(1).year_month();
    let range = range(date(2024, 1, 31), feb(1));

    check(
        january,
        &week(january, Weekday::Monday, 4),
        range,
        120.,
        24.,
    );
    check(
        february,
        &week(february, Weekday::Monday, 0),
        range,
        144.,
        24.,
    );
}

#[test]
fn sunday_first_is_already_reflected_in_logical_columns() {
    let month = feb(1).year_month();
    let range = range(feb(4), feb(5));
    check(month, &week(month, Weekday::Sunday, 1), range, 24., 48.);
}

#[test]
fn a_single_in_month_interior_cell_has_a_full_band() {
    let month = date(2024, 9, 1).year_month();
    let range = range(date(2024, 8, 25), date(2024, 9, 2));
    check(month, &week(month, Weekday::Monday, 0), range, 288., 48.);
}

#[test]
fn same_day_and_provisional_start_have_no_band() {
    let month = feb(1).year_month();
    let week = week(month, Weekday::Monday, 1);
    for direction in [LeftToRight, RightToLeft] {
        assert_eq!(
            range_band(month, &week, range(feb(6), feb(6)), direction),
            None
        );
        // A provisional Start has no validated CivilDateRange to pass.
        assert_eq!(range_band(month, &week, None, direction), None);
    }
}

#[test]
fn empty_and_disjoint_weeks_have_no_band() {
    let month = feb(1).year_month();
    let range = range(feb(15), feb(16));
    for direction in [LeftToRight, RightToLeft] {
        assert_eq!(
            range_band(month, &week(month, Weekday::Monday, 5), range, direction),
            None
        );
        assert_eq!(
            range_band(month, &week(month, Weekday::Monday, 0), range, direction),
            None
        );
    }
}
