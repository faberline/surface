//! Material 3 plain and rich tooltips.

use surface::{Align, Color, FillContour, Insets, Length, Node, Rect, Role, Style};
use surface_app::View;

use crate::{shape, shape::LayoutDirection, typography, Elevation, Theme};

const GAP: f32 = 4.;
const MIN_WIDTH: f32 = 40.;
const MIN_HEIGHT: f32 = 24.;
const CARET_WIDTH: f32 = 16.;
const CARET_HEIGHT: f32 = 8.;

/// The tooltip's content and visual treatment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TooltipKind {
    Plain,
    Rich,
}

impl TooltipKind {
    fn max_width(self) -> f32 {
        match self {
            Self::Plain => 200.,
            Self::Rich => 320.,
        }
    }
}

/// The preferred or actual vertical side of the anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipSide {
    #[default]
    Above,
    Below,
}

/// Tooltip surface bounds in logical viewport coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipPlacement {
    pub bounds: Rect,
    pub side: TooltipSide,
    /// The anchor's center relative to the surface's left edge.
    pub caret_x: f32,
}

/// Place a measured tooltip near the visible part of its anchor.
///
/// The tooltip is centered horizontally, with a 4 dp gap. It flips to the
/// opposite side when the preferred side is full. If neither side fits,
/// use the side with more room and constrain the result to the viewport.
/// An invalid, empty, or entirely offscreen anchor has no placement.
pub fn place_tooltip(
    anchor: Rect,
    popup_size: (f32, f32),
    viewport: Rect,
    preferred: TooltipSide,
) -> Option<TooltipPlacement> {
    let anchor = visible_anchor(anchor, viewport)?;
    let (width, height) = popup_size;
    if !positive(width) || !positive(height) {
        return None;
    }
    let (width, height) = (width.min(viewport.width), height.min(viewport.height));
    let center = anchor.x + anchor.width / 2.;
    let max_x = (viewport.x + viewport.width - width).max(viewport.x);
    let x = (center - width / 2.).clamp(viewport.x, max_x);
    let above_edge = anchor.y - GAP;
    let below_edge = anchor.y + anchor.height + GAP;
    let above_room = above_edge - viewport.y;
    let below_room = viewport.y + viewport.height - below_edge;
    let side = match preferred {
        TooltipSide::Above if above_room >= height => TooltipSide::Above,
        TooltipSide::Below if below_room >= height => TooltipSide::Below,
        _ if above_room > below_room => TooltipSide::Above,
        _ if below_room > above_room => TooltipSide::Below,
        _ => preferred,
    };
    let y = match side {
        TooltipSide::Above => above_edge - height,
        TooltipSide::Below => below_edge,
    };
    let max_y = (viewport.y + viewport.height - height).max(viewport.y);
    Some(TooltipPlacement {
        bounds: Rect::new(x, y.clamp(viewport.y, max_y), width, height),
        side,
        caret_x: (center - x).clamp(0., width),
    })
}

/// Content, anchor and placement for one tooltip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipOptions<'a> {
    /// ID of the surface in both the measurement and visible passes.
    pub id: &'a str,
    pub text: &'a str,
    /// Optional heading of a rich tooltip.
    pub title: Option<&'a str>,
    /// Anchor bounds in logical viewport coordinates.
    pub anchor: Rect,
    /// Bounds the tooltip can occupy, in the same coordinate space.
    pub viewport: Rect,
    pub preferred: TooltipSide,
    pub direction: LayoutDirection,
    pub caret: bool,
    /// Focus target if an interactive tooltip closes while it contains focus.
    pub restore_focus_to: &'a str,
    /// The surface's last measured width and height. With `None`, the view
    /// measures out of view and exposes no focus or accessibility actions.
    /// Read the `id` bounds from the next layout and rebuild with that size.
    /// Clear it when the content, viewport or text scale changes.
    pub measured_size: Option<(f32, f32)>,
}

impl Theme {
    /// Build a plain, passive tooltip without any focus targets.
    ///
    /// The caller controls how long it stays in the view tree.
    ///
    /// [Material 3 tooltips](https://m3.material.io/components/tooltips/overview)
    /// [Compose tooltip layout](https://android.googlesource.com/platform/frameworks/support/+/ffbddbd1295b99a57b395a5dcfb86af31f451492/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/Tooltip.kt)
    pub fn plain_tooltip<M>(&self, options: TooltipOptions<'_>) -> View<M> {
        self.tooltip_view(TooltipKind::Plain, options, None)
    }

    /// Build a rich tooltip. A tooltip with an action is a named nonmodal
    /// dialog. It puts the supplied action in the normal focus order and
    /// returns focus to the caller's target when the focused action leaves.
    pub fn rich_tooltip<M>(&self, options: TooltipOptions<'_>, action: Option<View<M>>) -> View<M> {
        self.tooltip_view(TooltipKind::Rich, options, action)
    }

    fn tooltip_view<M>(
        &self,
        kind: TooltipKind,
        options: TooltipOptions<'_>,
        action: Option<View<M>>,
    ) -> View<M> {
        let Some(anchor) = visible_anchor(options.anchor, options.viewport) else {
            return View::new(Node::new(Role::Group, ""));
        };
        let (background, foreground, corners, elevation) = match kind {
            TooltipKind::Plain => (
                self.colors.inverse_surface,
                self.colors.inverse_on_surface,
                shape::EXTRA_SMALL,
                Elevation::Level0,
            ),
            TooltipKind::Rich => (
                self.colors.surface_container,
                self.colors.on_surface_variant,
                shape::MEDIUM,
                Elevation::Level2,
            ),
        };
        let max_width = kind.max_width().min(options.viewport.width);
        let min_width = MIN_WIDTH.min(max_width);
        let min_height = MIN_HEIGHT.min(options.viewport.height);
        let placement = options.measured_size.and_then(|(width, height)| {
            if positive(width) && positive(height) {
                place_tooltip(
                    anchor,
                    (width.clamp(min_width, max_width), height.max(min_height)),
                    options.viewport,
                    options.preferred,
                )
            } else {
                None
            }
        });
        let (x, y) = placement.map_or(
            // The caller can pass a safe area smaller than the viewport.
            // Keep the measuring pass and its shadow above both.
            (options.viewport.x, -options.viewport.height - 32.),
            |placed| (placed.bounds.x, placed.bounds.y),
        );
        let mut style = Style::column()
            .fixed(Insets {
                left: Length::Px(x),
                top: Length::Px(y),
                ..Insets::default()
            })
            .corners(corners)
            .background(background);
        style.min_width = min_width;
        style.min_height = min_height;
        style.max_width = Length::Px(max_width);
        style.max_height = Length::Px(options.viewport.height);
        style.z_index = 1000;
        if let Some(placement) = placement {
            style.width = Length::Px(placement.bounds.width);
            style.height = Length::Px(placement.bounds.height);
        }
        let interactive = action.is_some();
        let title = if kind == TooltipKind::Rich {
            options.title
        } else {
            None
        };
        let name = match (interactive, title) {
            (false, Some(title)) => format!("{title}\n{}", options.text),
            (_, title) => title.unwrap_or(options.text).to_owned(),
        };
        let role = if interactive {
            Role::Dialog
        } else {
            Role::Tooltip
        };
        let content = self.tooltip_content(kind, options, foreground, title, action);
        let caret = placement
            .filter(|_| options.caret)
            .and_then(|placement| tooltip_caret(options, placement, anchor, kind, background));
        let mut surface = Node::new(role, name)
            .id(options.id)
            .styled(style)
            .box_shadows(self.elevation(elevation));
        if interactive {
            // Blank interactive panel areas shield covered controls. The
            // caller's actions make up the popup's place in the Tab order.
            surface = surface.tab_index(-1);
        }
        let tooltip = View::new(surface).children(std::iter::once(content).chain(caret));
        if placement.is_none() {
            tooltip.without_interaction(interactive.then_some(options.restore_focus_to))
        } else if interactive {
            tooltip.restore_focus_in_subtree_on_removal(options.restore_focus_to)
        } else {
            tooltip
        }
    }

    fn tooltip_content<M>(
        &self,
        kind: TooltipKind,
        options: TooltipOptions<'_>,
        foreground: Color,
        title: Option<&str>,
        action: Option<View<M>>,
    ) -> View<M> {
        let rich = kind == TooltipKind::Rich;
        let align = match options.direction {
            LayoutDirection::LeftToRight => Align::Start,
            LayoutDirection::RightToLeft => Align::End,
        };
        let label = |id, role, text, font| {
            let mut style = Style::default()
                .with_text_style(font)
                .foreground(foreground);
            style.max_width = Length::Percent(1.);
            View::new(Node::new(role, text).id(id).text(text).styled(style))
        };
        let space = |height| {
            View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(height))))
        };
        let heading = title.map(|text| {
            [
                label(
                    format!("{}-title", options.id),
                    Role::Heading,
                    text,
                    typography::TITLE_SMALL,
                ),
                space(4.),
            ]
        });
        let body = label(
            format!("{}-text", options.id),
            Role::Text,
            options.text,
            if rich {
                typography::BODY_MEDIUM
            } else {
                typography::BODY_SMALL
            },
        );
        let interactive = action.is_some();
        let tail = action.map(|action| [space(16.), action]);
        let mut lines = Style::column().align(align);
        lines.max_width = Length::Percent(1.);
        let labels = View::new(
            Node::new(Role::Group, "")
                .shrink_to_fit_width()
                .styled(lines),
        )
        .children(
            heading
                .into_iter()
                .flatten()
                .chain([body])
                .chain(tail.into_iter().flatten()),
        );
        let horizontal = if rich { 16. } else { 8. };
        let side = || {
            View::new(Node::new(Role::Group, "").styled(Style::row().width(Length::Px(horizontal))))
        };
        let row = View::new(Node::new(Role::Group, "").styled(Style::row())).children([
            side(),
            labels,
            side(),
        ]);
        // Approximate the first-baseline insets with the line boxes provided
        // by Surface's text measurer. Simple tooltips use symmetric padding.
        let (top, bottom) = match (title.is_some(), interactive) {
            (true, true) => (12., 8.),
            (true, false) => (12., 16.),
            (false, true) => (8., 8.),
            (false, false) => (4., 4.),
        };
        let mut style = Style::column().clip(true);
        style.max_height = Length::Percent(1.);
        let mut node = Node::new(Role::Group, "").styled(style);
        if interactive {
            node = node.id(format!("{}-content", options.id));
        }
        let view = View::new(node).children([space(top), row, space(bottom)]);
        if interactive {
            view.accessibility_scroll_pages()
        } else {
            view
        }
    }
}

fn positive(value: f32) -> bool {
    value.is_finite() && value > 0.
}

fn valid_rect(rect: Rect) -> bool {
    positive(rect.width)
        && positive(rect.height)
        && rect.x.is_finite()
        && rect.y.is_finite()
        && (rect.x + rect.width).is_finite()
        && (rect.y + rect.height).is_finite()
}

fn visible_anchor(anchor: Rect, viewport: Rect) -> Option<Rect> {
    if !valid_rect(anchor) || !valid_rect(viewport) {
        return None;
    }
    let visible = anchor.intersect(viewport);
    valid_rect(visible).then_some(visible)
}

fn tooltip_caret<M>(
    options: TooltipOptions<'_>,
    placement: TooltipPlacement,
    anchor: Rect,
    kind: TooltipKind,
    color: Color,
) -> Option<View<M>> {
    let corner = if kind == TooltipKind::Plain { 4. } else { 12. };
    let inset = CARET_WIDTH / 2. + corner;
    let bounds = placement.bounds;
    if bounds.width < inset * 2. {
        return None;
    }
    let left = placement.caret_x.clamp(inset, bounds.width - inset) - CARET_WIDTH / 2.;
    let (top, points) = match placement.side {
        TooltipSide::Above if bounds.y + bounds.height <= anchor.y => (
            bounds.height,
            [
                [0., 0.],
                [CARET_WIDTH, 0.],
                [CARET_WIDTH / 2., CARET_HEIGHT],
            ],
        ),
        TooltipSide::Below if bounds.y >= anchor.y + anchor.height => (
            -CARET_HEIGHT,
            [
                [0., CARET_HEIGHT],
                [CARET_WIDTH / 2., 0.],
                [CARET_WIDTH, CARET_HEIGHT],
            ],
        ),
        _ => return None,
    };
    let caret_bounds = Rect::new(bounds.x + left, bounds.y + top, CARET_WIDTH, CARET_HEIGHT);
    if caret_bounds.intersect(options.viewport) != caret_bounds {
        return None;
    }
    let caret = FillContour::new(points, color)?;
    Some(View::new(
        Node::new(Role::Group, "")
            .id(format!("{}-caret", options.id))
            .styled(
                Style::column()
                    .width(Length::Px(CARET_WIDTH))
                    .height(Length::Px(CARET_HEIGHT))
                    .absolute(Insets {
                        left: Length::Px(left),
                        top: Length::Px(top),
                        ..Insets::default()
                    }),
            )
            .fill_contour(caret),
    ))
}
