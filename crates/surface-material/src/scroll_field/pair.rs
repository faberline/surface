//! One inline group for a quantity and its independently selected unit.

use surface::{Align, Length, Node, Role, Style};
use surface_app::View;

use crate::{shape::LayoutDirection, ScrollChoiceEvent, ScrollChoiceState, Theme};

use super::{
    choice::ChoiceCopy, compact_field_width, field_layout, numeric_width, scale,
    ScrollChoiceFieldOptions, ScrollFieldEvent, ScrollFieldLayout, ScrollFieldOptions,
    ScrollFieldState,
};

const GAP: f32 = 16.;

/// Allocation and labels for a quantity/unit pair.
///
/// Use distinct stable IDs for the group and both fields. The pair supplies
/// each field's available width, available height and direction. Each field
/// keeps its own label, preferred size, text scale and copy.
#[derive(Clone, Copy)]
pub struct ScrollFieldPairOptions<'a> {
    pub id: &'a str,
    /// Accessible name for the group containing the two named fields.
    pub label: &'a str,
    pub available_width: f32,
    pub available_height: f32,
    pub direction: LayoutDirection,
    pub quantity: ScrollFieldOptions<'a>,
    pub unit: ScrollChoiceFieldOptions<'a>,
}

/// The pair's arrangement and each wheel's fit in that arrangement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollFieldPairLayout {
    pub stacked: bool,
    pub quantity_fits: bool,
    pub unit_fits: bool,
}

/// Check both wheels against the pair's allocation.
///
/// Pass `quantity_fits` and `unit_fits` separately to each state's
/// `update_fit` when space, type scale or the choice catalog changes.
/// Retire old pointer geometry with `invalidate_layout` on resize.
pub fn scroll_field_pair_layout(
    options: ScrollFieldPairOptions<'_>,
    quantity: &ScrollFieldState,
    unit: &ScrollChoiceState,
) -> ScrollFieldPairLayout {
    plan(options, quantity, unit).layout
}

impl Theme {
    /// Arrange a numeric field and a text-choice field for one measurement.
    ///
    /// Each field keeps its own events, editor, and accepted value. Apply the
    /// events to the respective state, and use its normal frame hook. A unit
    /// selection leaves the numeric value under the caller's control.
    /// When both fields cannot fit side by side, the pair stacks them. Give
    /// the pair a scrolling parent if the full stack may exceed the viewport.
    pub fn scroll_field_pair<M: Clone + 'static>(
        &self,
        options: ScrollFieldPairOptions<'_>,
        quantity: &ScrollFieldState,
        unit: &ScrollChoiceState,
        on_quantity: impl Fn(ScrollFieldEvent) -> M + 'static,
        on_unit: impl Fn(ScrollChoiceEvent) -> M + 'static,
    ) -> View<M> {
        let plan = plan(options, quantity, unit);
        let rtl = options.direction == LayoutDirection::RightToLeft;
        let style = if plan.layout.stacked {
            Style::column().align(if rtl { Align::End } else { Align::Start })
        } else {
            Style::row().align(Align::Start)
        }
        .gap(GAP)
        .width(Length::Px(plan.width));
        let node = Node::new(Role::Group, options.label)
            .id(options.id)
            .styled(style);
        let node = if rtl && !plan.layout.stacked {
            node.reverse_row()
        } else {
            node
        };
        View::new(node).children([
            self.scroll_field(plan.quantity, quantity, on_quantity),
            self.scroll_choice_field(plan.unit, unit, on_unit),
        ])
    }
}

struct PairPlan<'a> {
    layout: ScrollFieldPairLayout,
    width: f32,
    quantity: ScrollFieldOptions<'a>,
    unit: ScrollChoiceFieldOptions<'a>,
}

fn plan<'a>(
    options: ScrollFieldPairOptions<'a>,
    quantity: &ScrollFieldState,
    unit: &ScrollChoiceState,
) -> PairPlan<'a> {
    let available_width = if options.available_width.is_finite() {
        options.available_width.max(0.)
    } else {
        0.
    };
    let mut quantity_options = ScrollFieldOptions {
        available_width,
        available_height: options.available_height,
        direction: options.direction,
        ..options.quantity
    };
    let mut unit_options = ScrollChoiceFieldOptions {
        available_width,
        available_height: options.available_height,
        direction: options.direction,
        ..options.unit
    };
    let quantity_width = numeric_width(
        quantity_options.size,
        quantity,
        scale(quantity_options.text_scale),
    );
    let unit_width = ChoiceCopy::new(unit_options, unit).preferred_width(unit_options);
    let quantity_wheel = field_layout(quantity_options.into(), quantity_width, false);
    let unit_wheel = field_layout(unit_options.into(), unit_width, true);
    let quantity_width = allocated_width(quantity_wheel, available_width, quantity_width);
    let unit_width = allocated_width(unit_wheel, available_width, unit_width);
    let row_width = quantity_width + GAP + unit_width;
    let stacked = row_width > available_width;
    if !stacked {
        quantity_options.available_width = quantity_width;
        unit_options.available_width = unit_width;
    }
    PairPlan {
        layout: ScrollFieldPairLayout {
            stacked,
            quantity_fits: quantity_wheel.is_some(),
            unit_fits: unit_wheel.is_some(),
        },
        width: if stacked { available_width } else { row_width },
        quantity: quantity_options,
        unit: unit_options,
    }
}

fn allocated_width(
    layout: Option<ScrollFieldLayout>,
    available_width: f32,
    preferred_width: f32,
) -> f32 {
    layout.map_or_else(
        || {
            let (width, outset) = compact_field_width(available_width, preferred_width);
            width + 2. * outset
        },
        ScrollFieldLayout::width_with_mode,
    )
}
