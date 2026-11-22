use surface::{Color, MeasuredGroupLayout, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{CursorStyle, View};

use super::{
    unit_fraction, ItemAppearance, LayoutDirection, NavigationRailItem, Theme, VectorIconSize,
    WideNavigationRailMode, COLLAPSED_ICON_LABEL_GAP, COLLAPSED_INDICATOR_HEIGHT,
    COLLAPSED_INDICATOR_WIDTH, COLLAPSED_ITEM_MIN_HEIGHT, COLLAPSED_WIDTH, EXPANDED_ICON_LABEL_GAP,
    EXPANDED_INDICATOR_PADDING, EXPANDED_ITEM_INSET, EXPANDED_MAX_WIDTH,
};
use crate::typography;

const ICON: f32 = 24.;
const EXPANDED_MARGIN: f32 =
    2. * (EXPANDED_ITEM_INSET + EXPANDED_INDICATOR_PADDING) + ICON + EXPANDED_ICON_LABEL_GAP;

impl Theme {
    /// Animate one wide rail item with a stable button and one moving icon.
    /// Pass the rail's Standard Default Spatial position in `0..=1`. Use
    /// this builder with `animated_wide_navigation_rail` for an inline rail,
    /// or with a retained modal's item track. The standalone modal always
    /// shows expanded items while the panel slides.
    ///
    /// The label moves from below the icon to its logical end at zero
    /// opacity. It wraps at the endpoint's text width throughout that half
    /// of the transition. The rail clips any still-hidden leading content.
    pub fn animated_wide_navigation_rail_item_with_icon<M>(
        &self,
        progress: f32,
        direction: LayoutDirection,
        item: NavigationRailItem<'_, M>,
    ) -> View<M> {
        let progress = unit_fraction(progress);
        if progress == 0. || progress == 1. {
            let mode = if progress == 0. {
                WideNavigationRailMode::Collapsed
            } else {
                WideNavigationRailMode::Expanded
            };
            return self.wide_navigation_rail_item_with_icon(mode, direction, item);
        }

        let ItemAppearance {
            owner,
            indicator,
            indicator_style,
            ink,
            compact_ink,
        } = self.wide_rail_item_appearance(&item);
        let indicator = indicator
            .id(format!("{}-motion-indicator", item.id))
            .styled(indicator_style);
        let icon = Node::new(Role::Group, "")
            .id(format!("{}-motion-icon", item.id))
            .fill_contour(
                item.icon
                    .contour_dp(VectorIconSize::Dp24.dp(), ink)
                    .expect("validated icon contour"),
            );
        let top = progress < 0.5;
        let alpha = (2. * progress - 1.).powi(2);
        let text = |is_top, style, Color(r, g, b, a)| {
            let visible = is_top == top;
            Node::new(Role::Text, "")
                .id(format!(
                    "{}-motion-label{}",
                    item.id,
                    if visible { "" } else { "-measure" }
                ))
                .text(item.label)
                .styled(Style::default().with_text_style(style).foreground(Color(
                    r,
                    g,
                    b,
                    if visible {
                        (f32::from(a) * alpha).round() as u8
                    } else {
                        0
                    },
                )))
        };
        let owner = owner
            .styled(Style::column().clip(true))
            .measured_group(ItemLayout {
                progress,
                direction,
            });
        let decoration = [
            indicator,
            icon,
            text(true, typography::LABEL_MEDIUM, compact_ink),
            text(false, typography::LABEL_LARGE, ink),
        ];
        let view = View::new(owner)
            .children(decoration.map(|node| View::new(node).without_interaction(None)))
            .on_press(item.message)
            .accessible_activation();
        if item.state.disabled {
            view
        } else {
            view.cursor(CursorStyle::Pointer)
        }
    }
}

#[derive(Debug)]
struct ItemLayout {
    progress: f32,
    direction: LayoutDirection,
}

struct Metrics {
    compact: (f32, f32),
    expanded: (f32, f32),
}

impl Metrics {
    fn new(children: &[&Node], fonts: &mut dyn TextMeasurer) -> Self {
        let text = |fonts: &mut dyn TextMeasurer, child: &Node, width| {
            let (w, h) = fonts.measure_text(
                child.text_value().unwrap_or_default(),
                &child.style().text_style(),
                Some(width),
            );
            (w.ceil().min(width), h.ceil())
        };
        Self {
            compact: text(fonts, children[2], COLLAPSED_WIDTH),
            expanded: text(fonts, children[3], EXPANDED_MAX_WIDTH - EXPANDED_MARGIN),
        }
    }

    fn top_height(&self) -> f32 {
        (COLLAPSED_INDICATOR_HEIGHT + COLLAPSED_ICON_LABEL_GAP + self.compact.1)
            .max(COLLAPSED_ITEM_MIN_HEIGHT)
    }

    fn leading_height(&self) -> f32 {
        self.expanded.1.max(ICON) + 2. * EXPANDED_INDICATOR_PADDING
    }
}

impl ItemLayout {
    // Keep the upper label's vertical space until it fades out. The row and
    // icon then move vertically into the leading arrangement together.
    fn vertical_progress(&self) -> f32 {
        (2. * self.progress - 1.).max(0.)
    }

    fn mirrored(&self, rect: Rect, width: f32) -> Rect {
        match self.direction {
            LayoutDirection::LeftToRight => rect,
            LayoutDirection::RightToLeft => {
                Rect::new(width - rect.x - rect.width, rect.y, rect.width, rect.height)
            }
        }
    }
}

impl MeasuredGroupLayout for ItemLayout {
    fn measure(
        &self,
        children: &[&Node],
        available_width: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let metrics = Metrics::new(children, fonts);
        let natural = metrics.expanded.0 + EXPANDED_MARGIN;
        (
            natural.min(available_width.unwrap_or(natural)),
            lerp(
                metrics.top_height(),
                metrics.leading_height(),
                self.vertical_progress(),
            ),
        )
    }

    fn place(
        &self,
        children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let metrics = Metrics::new(children, fonts);
        let p = self.progress;
        let vertical = self.vertical_progress();
        let top_y = ((metrics.top_height()
            - COLLAPSED_INDICATOR_HEIGHT
            - COLLAPSED_ICON_LABEL_GAP
            - metrics.compact.1)
            * 0.5)
            .max(0.);
        let icon_x = lerp(
            (width - ICON) * 0.5,
            EXPANDED_ITEM_INSET + EXPANDED_INDICATOR_PADDING,
            p,
        )
        .max(0.);
        let icon_y = lerp(
            top_y + (COLLAPSED_INDICATOR_HEIGHT - ICON) * 0.5,
            (height - ICON) * 0.5,
            vertical,
        );
        let pill_w = lerp(
            COLLAPSED_INDICATOR_WIDTH.min(width),
            (metrics.expanded.0 + EXPANDED_MARGIN - 2. * EXPANDED_ITEM_INSET)
                .min((width - 2. * EXPANDED_ITEM_INSET).max(0.)),
            p,
        );
        let pill_x = lerp(
            (width - COLLAPSED_INDICATOR_WIDTH.min(width)) * 0.5,
            EXPANDED_ITEM_INSET.min(width * 0.5),
            p,
        );
        let pill_h = lerp(
            COLLAPSED_INDICATOR_HEIGHT,
            metrics.leading_height(),
            vertical,
        );
        let pill_y = lerp(top_y, (height - pill_h) * 0.5, vertical);
        let top_label = if p < 0.5 {
            Rect::new(
                (width - metrics.compact.0) * 0.5,
                top_y + COLLAPSED_INDICATOR_HEIGHT + COLLAPSED_ICON_LABEL_GAP,
                metrics.compact.0,
                metrics.compact.1,
            )
        } else {
            Rect::default()
        };
        let leading_label = if p >= 0.5 {
            Rect::new(
                icon_x + ICON + EXPANDED_ICON_LABEL_GAP,
                (height - metrics.expanded.1) * 0.5,
                metrics.expanded.0,
                metrics.expanded.1,
            )
        } else {
            Rect::default()
        };
        [
            Rect::new(pill_x, pill_y, pill_w, pill_h),
            Rect::new(icon_x, icon_y, ICON, ICON),
            top_label,
            leading_label,
        ]
        .map(|rect| self.mirrored(rect, width))
        .to_vec()
    }
}

fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}
