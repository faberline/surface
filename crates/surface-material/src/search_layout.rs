//! Pure window geometry for Material 3 Search. All dimensions are logical dp.

use surface::Rect;

use crate::{adaptive::WindowWidthSizeClass, shape::LayoutDirection};

const DOCKED_MIN_WIDTH: f32 = 360.;
const DOCKED_MAX_WIDTH: f32 = 720.;
const DOCKED_MIN_HEIGHT: f32 = 240.;
const BAR_HEIGHT: f32 = 56.;
const DIVIDED_FULL_SCREEN_BAR_HEIGHT: f32 = 72.;
const CONTAINED_RESTING_MARGIN: f32 = 24.;
const CONTAINED_FOCUSED_MARGIN: f32 = 12.;
const CONTAINED_DOCKED_GAP: f32 = 2.;
const CONTAINED_DOCKED_RESULTS_INSET: f32 = 4.;
const DIVIDED_DOCKED_RESULTS_BOTTOM: f32 = 16.;
const DIVIDER_HEIGHT: f32 = 1.;
const CONTROL_SIDE: f32 = 48.;
const CONTROL_EDGE_INSET: f32 = 4.;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Appearance of the expanding Material search bar.
pub enum SearchStyle {
    /// Keep the rounded search pill above suggestions and results.
    Contained,
    /// Separate the search row and result surface with a divider.
    Divided,
}

impl SearchStyle {
    /// Place a resting face in its un-inset allocation. Pass the same
    /// allocation as the expanded overlay's anchor.
    pub fn resting_bar_bounds(self, anchor: Rect) -> Option<Rect> {
        if !valid(anchor) {
            return None;
        }
        let mut bar = anchor;
        bar.height = BAR_HEIGHT.min(anchor.height);
        if self == Self::Contained {
            bar = inset_bar(bar, CONTAINED_RESTING_MARGIN);
        }
        Some(bar)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SearchPresentation {
    Docked,
    FullScreen,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SearchLayout {
    pub(super) presentation: SearchPresentation,
    /// Outer bounds of the bar and results. Docked rendering adds a viewport scrim.
    pub(super) bounds: Rect,
    /// The pinned input row's painted bounds.
    pub(super) bar: Rect,
    /// The bar's inner strip for the editor and centered icon targets.
    pub(super) controls: Rect,
    pub(super) divider: Option<Rect>,
    /// Painted results background, including internal blank space.
    pub(super) results_surface: Rect,
    /// Clipped viewport for the scrolling results. The input is outside it.
    pub(super) results_viewport: Rect,
}

impl SearchLayout {
    /// Interpolate only between geometries already contained by the viewport.
    pub(super) fn interpolate_from(self, rest: Rect, progress: f32) -> Self {
        if progress == 1. {
            return self;
        }
        let empty = Rect::new(rest.x, bottom(rest), rest.width, 0.);
        let bar = mix_rect(rest, self.bar, progress);
        Self {
            presentation: self.presentation,
            bounds: mix_rect(rest, self.bounds, progress),
            bar,
            controls: mix_rect(inset_bar(rest, CONTROL_EDGE_INSET), self.controls, progress),
            divider: self
                .divider
                .filter(|_| progress > 0.)
                .map(|rect| mix_rect(empty, rect, progress)),
            results_surface: mix_rect(empty, self.results_surface, progress),
            results_viewport: mix_rect(empty, self.results_viewport, progress),
        }
    }
}

fn mix_rect(from: Rect, to: Rect, progress: f32) -> Rect {
    let mix = |a, b| a + (b - a) * progress;
    Rect::new(
        mix(from.x, to.x),
        mix(from.y, to.y),
        mix(from.width, to.width),
        mix(from.height, to.height),
    )
}

/// Plan the open layout from the current safe viewport and stable bar
/// allocation in window coordinates. The anchor's y is the resting bar top.
/// Docking keeps the logical start edge and grows only downward. A missing,
/// clipped, or unfit anchor picks Full-screen. An absent height preference
/// uses the available docked height. A preference describes the whole surface.
pub(super) fn plan_search_layout(
    viewport: Rect,
    anchor: Option<Rect>,
    style: SearchStyle,
    direction: LayoutDirection,
    preferred_docked_height: Option<f32>,
) -> Option<SearchLayout> {
    if !valid(viewport)
        || preferred_docked_height.is_some_and(|height| !height.is_finite() || height < 0.)
    {
        return None;
    }
    let width_class = WindowWidthSizeClass::from_dp(viewport.width)?;
    let docked = if width_class >= WindowWidthSizeClass::Medium {
        anchor.and_then(|anchor| {
            docked_bounds(viewport, anchor, style, direction, preferred_docked_height)
        })
    } else {
        None
    };
    let (bounds, presentation) = docked
        .map_or((viewport, SearchPresentation::FullScreen), |bounds| {
            (bounds, SearchPresentation::Docked)
        });
    Some(place_contents(bounds, presentation, style))
}

fn docked_bounds(
    viewport: Rect,
    anchor: Rect,
    style: SearchStyle,
    direction: LayoutDirection,
    preferred_height: Option<f32>,
) -> Option<Rect> {
    if !valid(anchor)
        || anchor.x < viewport.x
        || anchor.y < viewport.y
        || right(anchor) > right(viewport)
        || bottom(anchor) > bottom(viewport)
    {
        return None;
    }
    let focused = match style {
        SearchStyle::Contained => inset_bar(anchor, CONTAINED_FOCUSED_MARGIN),
        SearchStyle::Divided => anchor,
    };
    let (start, room) = match direction {
        LayoutDirection::LeftToRight => (focused.x, right(viewport) - focused.x),
        LayoutDirection::RightToLeft => (right(focused), right(focused) - viewport.x),
    };
    let max_height = (viewport.height / 3. * 2.).min(bottom(viewport) - anchor.y);
    if room < DOCKED_MIN_WIDTH || max_height < DOCKED_MIN_HEIGHT {
        return None;
    }

    let width = focused
        .width
        .clamp(DOCKED_MIN_WIDTH, DOCKED_MAX_WIDTH)
        .min(room);
    let height = preferred_height
        .unwrap_or(max_height)
        .clamp(DOCKED_MIN_HEIGHT, max_height);
    let x = match direction {
        LayoutDirection::LeftToRight => start,
        LayoutDirection::RightToLeft => start - width,
    };
    Some(Rect::new(x, anchor.y, width, height))
}

fn place_contents(
    bounds: Rect,
    presentation: SearchPresentation,
    style: SearchStyle,
) -> SearchLayout {
    let docked = presentation == SearchPresentation::Docked;
    let contained = style == SearchStyle::Contained;
    let bar_height = match (style, presentation) {
        (SearchStyle::Divided, SearchPresentation::FullScreen) => DIVIDED_FULL_SCREEN_BAR_HEIGHT,
        _ => BAR_HEIGHT,
    };
    let mut bar = bounds;
    if contained && !docked {
        // Keep the input usable first if the safe viewport becomes very small.
        bar = inset_bar(bar, CONTAINED_FOCUSED_MARGIN);
        bar.y += CONTAINED_FOCUSED_MARGIN.min((bounds.height - bar_height).max(0.));
    }
    bar.height = bar_height.min((bottom(bounds) - bar.y).max(0.));
    // A 24 dp centered icon in a 48 dp target starts 16 dp from the face edge.
    let controls = inset_bar(bar, CONTROL_EDGE_INSET);
    let remaining = section_from(bounds, bottom(bar));
    let divider = (!contained && remaining.height >= DIVIDER_HEIGHT)
        .then(|| Rect::new(remaining.x, remaining.y, remaining.width, DIVIDER_HEIGHT));
    let separation = match (style, presentation) {
        (SearchStyle::Contained, SearchPresentation::Docked) => CONTAINED_DOCKED_GAP,
        (SearchStyle::Divided, _) if divider.is_some() => DIVIDER_HEIGHT,
        _ => 0.,
    };
    let results_surface = section_from(bounds, remaining.y + separation);
    let results_viewport = match (style, presentation) {
        (SearchStyle::Contained, SearchPresentation::Docked) => {
            results_surface.inset(CONTAINED_DOCKED_RESULTS_INSET)
        }
        (SearchStyle::Divided, SearchPresentation::Docked) => {
            let mut viewport = results_surface;
            viewport.height -= DIVIDED_DOCKED_RESULTS_BOTTOM;
            viewport
        }
        _ => results_surface,
    };
    SearchLayout {
        presentation,
        bounds,
        bar,
        controls,
        divider,
        results_surface,
        results_viewport,
    }
}

fn inset_bar(bounds: Rect, inset: f32) -> Rect {
    let inset = inset.min(((bounds.width - CONTROL_SIDE) / 2.).max(0.));
    Rect::new(
        bounds.x + inset,
        bounds.y,
        bounds.width - 2. * inset,
        bounds.height,
    )
}

fn section_from(bounds: Rect, y: f32) -> Rect {
    let y = y.min(bottom(bounds));
    Rect::new(bounds.x, y, bounds.width, bottom(bounds) - y)
}

fn right(rect: Rect) -> f32 {
    rect.x + rect.width
}

fn bottom(rect: Rect) -> f32 {
    rect.y + rect.height
}

fn valid(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && right(rect).is_finite()
        && bottom(rect).is_finite()
        && right(rect) > rect.x
        && bottom(rect) > rect.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use LayoutDirection::{LeftToRight, RightToLeft};
    use SearchPresentation::{Docked, FullScreen};
    use SearchStyle::{Contained, Divided};

    #[test]
    fn switches_at_600_dp_for_both_styles() {
        let narrow = Rect::new(0., 0., 599., 720.);
        let medium = Rect::new(0., 0., 600., 720.);
        let anchor = Some(Rect::new(0., 64., 410., 56.));
        for style in [Contained, Divided] {
            let compact = plan_search_layout(narrow, anchor, style, LeftToRight, None).unwrap();
            assert_eq!(compact.presentation, FullScreen);
            assert_eq!(compact.bounds, narrow);

            let docked = plan_search_layout(medium, anchor, style, LeftToRight, None).unwrap();
            assert_eq!(docked.presentation, Docked);
            assert_eq!(docked.bounds.y, 64.);
            assert_eq!(docked.bounds.height, 480.);
        }
    }

    #[test]
    fn docked_bounds_keep_logical_start_and_respect_width_limits() {
        let viewport = Rect::new(40., 20., 960., 600.);
        let narrow = Some(Rect::new(404., 60., 210., 56.));
        let wide = Some(Rect::new(80., 60., 860., 56.));
        let plan = |anchor, style, direction| {
            plan_search_layout(viewport, anchor, style, direction, Some(240.)).unwrap()
        };
        assert_eq!(
            plan(narrow, Contained, LeftToRight).bounds,
            Rect::new(416., 60., 360., 240.)
        );
        assert_eq!(
            plan(narrow, Contained, RightToLeft).bounds,
            Rect::new(242., 60., 360., 240.)
        );
        assert_eq!(
            plan(wide, Divided, LeftToRight).bounds,
            Rect::new(80., 60., 720., 240.)
        );
        assert_eq!(
            plan(wide, Divided, RightToLeft).bounds,
            Rect::new(220., 60., 720., 240.)
        );
    }

    #[test]
    fn docking_never_flips_when_the_logical_start_cannot_hold_minimum_width() {
        let viewport = Rect::new(40., 20., 960., 600.);
        let near_right = Some(Rect::new(630., 60., 350., 56.));
        let near_left = Some(Rect::new(60., 60., 350., 56.));
        let plan = |anchor, direction| {
            plan_search_layout(viewport, anchor, Contained, direction, None).unwrap()
        };
        assert_eq!(plan(near_right, LeftToRight).presentation, FullScreen);
        assert_eq!(plan(near_left, RightToLeft).presentation, FullScreen);
        let exact = Some(Rect::new(628., 60., 350., 56.));
        assert_eq!(
            plan(exact, LeftToRight).bounds,
            Rect::new(640., 60., 360., 400.)
        );
        assert_eq!(plan(near_left, LeftToRight).presentation, Docked);
        assert_eq!(plan(near_right, RightToLeft).presentation, Docked);
    }

    #[test]
    fn height_clamps_to_minimum_two_thirds_and_actual_room_below() {
        let plan = |height, y, preferred| {
            plan_search_layout(
                Rect::new(0., 10., 780., height),
                Some(Rect::new(60., y, 440., 56.)),
                Divided,
                LeftToRight,
                preferred,
            )
            .unwrap()
        };
        assert_eq!(plan(359., 10., None).presentation, FullScreen);
        assert_eq!(plan(360., 10., Some(800.)).bounds.height, 240.);
        assert_eq!(plan(720., 10., Some(0.)).bounds.height, 240.);
        assert_eq!(plan(720., 20., Some(600.)).bounds.height, 480.);
        assert_eq!(plan(720., 380., Some(460.)).bounds.height, 350.);
        assert_eq!(plan(720., 490., None).bounds.height, 240.);
        assert_eq!(plan(720., 491., None).presentation, FullScreen);
    }

    #[test]
    fn contained_keeps_a_pinned_pill_and_its_separate_scroller() {
        let anchor = Rect::new(0., 40., 440., 56.);
        let viewport = Rect::new(0., 0., 620., 720.);
        let open =
            plan_search_layout(viewport, Some(anchor), Contained, LeftToRight, Some(300.)).unwrap();
        assert_eq!(
            Contained.resting_bar_bounds(anchor),
            Some(Rect::new(24., 40., 392., 56.))
        );
        assert_eq!(open.bar, Rect::new(12., 40., 416., 56.));
        assert_eq!(open.controls, Rect::new(16., 40., 408., 56.));
        assert_eq!(open.divider, None);
        assert_eq!(open.results_surface, Rect::new(12., 98., 416., 242.));
        assert_eq!(open.results_viewport, Rect::new(16., 102., 408., 234.));

        let compact = Rect::new(5., 20., 320., 400.);
        let open = plan_search_layout(compact, None, Contained, RightToLeft, None).unwrap();
        assert_eq!(open.bounds, compact);
        assert_eq!(open.bar, Rect::new(17., 32., 296., 56.));
        assert_eq!(open.divider, None);
        assert_eq!(open.results_viewport, Rect::new(5., 88., 320., 332.));
    }

    #[test]
    fn divided_separates_the_pinned_header_from_both_scrollers() {
        let anchor = Rect::new(80., 40., 400., 56.);
        let viewport = Rect::new(0., 0., 620., 720.);
        assert_eq!(Divided.resting_bar_bounds(anchor), Some(anchor));
        let docked =
            plan_search_layout(viewport, Some(anchor), Divided, RightToLeft, Some(300.)).unwrap();
        assert_eq!(docked.bar, anchor);
        assert_eq!(docked.controls, Rect::new(84., 40., 392., 56.));
        assert_eq!(docked.divider, Some(Rect::new(80., 96., 400., 1.)));
        assert_eq!(docked.results_viewport, Rect::new(80., 97., 400., 227.));

        let viewport = Rect::new(12., 19., 320., 400.);
        let full = plan_search_layout(viewport, None, Divided, LeftToRight, None).unwrap();
        assert_eq!(full.bar, Rect::new(12., 19., 320., 72.));
        assert_eq!(
            full.controls.x + (CONTROL_SIDE - 24.) / 2.,
            viewport.x + 16.
        );
        assert_eq!(full.divider, Some(Rect::new(12., 91., 320., 1.)));
        assert_eq!(full.results_viewport, Rect::new(12., 92., 320., 327.));
    }

    #[test]
    fn invalid_viewport_returns_none_and_missing_or_stale_anchor_falls_back() {
        let viewport = Rect::new(20., 20., 620., 720.);
        for invalid in [
            Rect::new(f32::NAN, 0., 600., 600.),
            Rect::new(0., f32::INFINITY, 600., 600.),
            Rect::new(0., 0., 0., 600.),
            Rect::new(0., 0., 600., -1.),
            Rect::new(0., 0., 600., f32::INFINITY),
            Rect::new(f32::MAX, 0., f32::MAX, 600.),
        ] {
            assert!(plan_search_layout(invalid, None, Contained, LeftToRight, None).is_none());
            assert!(Contained.resting_bar_bounds(invalid).is_none());
            let result =
                plan_search_layout(viewport, Some(invalid), Contained, LeftToRight, None).unwrap();
            assert_eq!(result.presentation, FullScreen);
        }
        for anchor in [
            None,
            Some(Rect::new(10., 40., 400., 56.)),
            Some(Rect::new(650., 40., 60., 56.)),
        ] {
            assert_eq!(
                plan_search_layout(viewport, anchor, Divided, RightToLeft, None)
                    .unwrap()
                    .presentation,
                FullScreen
            );
        }
        assert!(
            plan_search_layout(viewport, None, Contained, LeftToRight, Some(f32::NAN)).is_none()
        );
    }

    #[test]
    fn a_very_small_full_screen_viewport_allocates_input_before_results() {
        let viewport = Rect::new(10., 20., 60., 61.);
        let contained = plan_search_layout(viewport, None, Contained, LeftToRight, None).unwrap();
        assert_eq!(contained.bar, Rect::new(16., 25., 48., 56.));
        assert_eq!(contained.results_viewport.height, 0.);
        assert_eq!(contained.results_viewport.y, 81.);

        let divided = plan_search_layout(viewport, None, Divided, LeftToRight, None).unwrap();
        assert_eq!(divided.bar, viewport);
        assert_eq!(divided.divider, None);
        assert_eq!(divided.results_viewport.height, 0.);
    }
}
