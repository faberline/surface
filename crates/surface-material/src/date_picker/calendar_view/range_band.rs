use crate::{shape::LayoutDirection, CivilDate, CivilDateRange, YearMonth};

use super::{DAY_DIAMETER, GRID_WIDTH, TARGET, WEEK_LENGTH};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WeekRangeBand {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

/// Returns at most one physical rectangle relative to a 48 dp week row.
/// Pass the validated range only. Date controls paint the endpoint faces.
pub(super) fn range_band(
    month: YearMonth,
    week: &[Option<CivilDate>; WEEK_LENGTH],
    range: Option<CivilDateRange>,
    direction: LayoutDirection,
) -> Option<WeekRangeBand> {
    let range = range?;
    let start = range.start();
    let end = range.end();
    if start == end {
        return None;
    }

    let mut covered = week.iter().enumerate().filter_map(|(column, date)| {
        let date = (*date)?;
        (date.year_month() == month && range.contains(date)).then_some((column, date))
    });
    let (first_column, first_date) = covered.next()?;
    let (last_column, last_date) = covered.next_back().unwrap_or((first_column, first_date));

    let leading = first_column as f32 * TARGET + if first_date == start { TARGET / 2. } else { 0. };
    let trailing = last_column as f32 * TARGET
        + if last_date == end {
            TARGET / 2.
        } else {
            TARGET
        };
    let x = match direction {
        LayoutDirection::LeftToRight => leading,
        LayoutDirection::RightToLeft => GRID_WIDTH - trailing,
    };
    Some(WeekRangeBand {
        x,
        y: (TARGET - DAY_DIAMETER) / 2.,
        width: trailing - leading,
        height: DAY_DIAMETER,
    })
}

#[cfg(test)]
mod tests;
