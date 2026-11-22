//! Material 3 status badges and top-trailing anchor placement.
//!
//! Sizes and colors use the pinned AndroidX BadgeTokens. The top-trailing
//! overlaps use the BadgedBox placement at the same revision.
//! <https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/Badge.kt>
use std::borrow::Cow;

use crate::{shape, shape::LayoutDirection, typography, Theme};
use surface::{
    Align, Insets, Length, MeasuredGroupLayout, Node, OverlayHorizontalEdge, Rect, Role, Style,
    TextMeasurer, TextScaling,
};
use surface_app::View;

pub use surface::OverlayBoundary as BadgeBoundary;

const DOT_SIZE: f32 = 6.;
const LABEL_SIZE: f32 = 16.;
const LABEL_SIDE_PADDING: f32 = 4.;

const LABEL_HORIZONTAL_OVERLAP: f32 = 12.;
const LABEL_VERTICAL_OVERLAP: f32 = 14.;
// Leave most of the 24 dp navigation artwork visible beneath the label.
const NAVIGATION_LABEL_VERTICAL_OVERLAP: f32 = 4.;

/// Content shown on a badged anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeContent<'a> {
    /// No visible badge.
    Hidden,
    /// A small dot without visible text.
    Dot,
    /// A short localized label. The first line is shown. A blank label shows a dot.
    Label(&'a str),
    /// A decimal count. Values above 999 show as `999+`. Use a label to localize the count.
    Count(u32),
}

/// Location of the anchor's artwork inside its measured bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BadgedAnchorOptions {
    /// The badge is placed at the logical trailing edge.
    pub direction: LayoutDirection,
    /// Distance in dp from the top of the anchor to the top of its artwork.
    pub top_inset: f32,
    /// Distance in dp from the trailing edge of the anchor to the edge of its artwork.
    pub end_inset: f32,
}

impl<'a> BadgeContent<'a> {
    fn label(self) -> Option<Cow<'a, str>> {
        match self {
            Self::Hidden | Self::Dot => None,
            Self::Label(label) => first_line(label).map(Cow::Borrowed),
            Self::Count(count) if count > 999 => Some(Cow::Borrowed("999+")),
            Self::Count(count) => Some(Cow::Owned(count.to_string())),
        }
    }
}

fn first_line(label: &str) -> Option<&str> {
    label
        .trim()
        .split(['\r', '\n', '\u{2028}', '\u{2029}'])
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
}

fn artwork_inset(inset: f32) -> f32 {
    if inset.is_finite() {
        inset.max(0.)
    } else {
        0.
    }
}

pub(crate) fn badge_headroom<M>(art_top_inset: f32) -> View<M> {
    // An empty label gives layout the same scaled typography as a real
    // badge without producing ink or a second accessible label.
    let label = Node::new(Role::Group, "")
        .text("")
        .styled(Style::default().with_text_style(typography::LABEL_SMALL));
    View::new(Node::new(Role::Group, "").measured_group(BadgeHeadroom { art_top_inset }))
        .children([View::new(label)])
}

#[derive(Debug)]
struct BadgeHeadroom {
    art_top_inset: f32,
}

impl MeasuredGroupLayout for BadgeHeadroom {
    fn measure(
        &self,
        children: &[&Node],
        _available_width: Option<f32>,
        _fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let line_height = children[0].style().text_style().line_height_px();
        let headroom = if line_height > LABEL_SIZE {
            (line_height - NAVIGATION_LABEL_VERTICAL_OVERLAP - self.art_top_inset).max(0.)
        } else {
            // The standard center-aligned item has room at its default size.
            0.
        };
        (0., headroom)
    }

    fn place(
        &self,
        _children: &[&Node],
        _size: (f32, f32),
        _fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        vec![Rect::default()]
    }
}

impl Theme {
    /// A non-interactive status badge with an accessible name.
    ///
    /// `None` or a blank `label` produces a 6 dp dot. A non-empty label
    /// produces a badge that is 16 dp high at the default text size.
    /// It grows to fit larger text. Provide a
    /// localized, descriptive `accessible_name`, such as "3 unread messages".
    /// If it is empty, the label is used, or "Notification" for an empty dot.
    /// This builds only the badge; it does not place it over another control.
    pub fn badge<M>(&self, id: &str, accessible_name: &str, label: Option<&str>) -> View<M> {
        let label = label.and_then(first_line);
        let name = if accessible_name.trim().is_empty() {
            label.unwrap_or("Notification")
        } else {
            accessible_name
        };
        let badge = self.badge_view(Node::new(Role::Text, name).id(id), label, None);
        // A parent column stretches auto-width children. Keep the badge itself intrinsic.
        View::new(Node::new(Role::Group, "").styled(Style::row().align(Align::Center)))
            .children([badge])
    }

    /// Show a decorative badge at the anchor's top trailing edge.
    ///
    /// Supply a flow anchor with fixed or intrinsic size and a stable ID.
    /// Use its existing accessible name to announce the badge's meaning
    /// in the user's language, for example "Inbox, 24 unread messages".
    /// The badge takes no focus and the anchor keeps its actions.
    ///
    /// The badge adds no layout space. Leave room for it in clipped parents.
    /// Set both insets to zero to use the anchor's bounds. Negative and
    /// non-finite insets use zero. Both insets are measured in logical dp.
    pub fn badged_anchor<M>(
        &self,
        anchor: View<M>,
        content: BadgeContent<'_>,
        options: BadgedAnchorOptions,
    ) -> View<M> {
        self.anchored_badge(anchor, content, options, None, LABEL_VERTICAL_OVERLAP)
    }

    /// Place a badge inside the visible clip at the top and logical end.
    ///
    /// Supply the stable ID of the anchor or an enclosing control as
    /// `owner_id`. Use `BadgeBoundary::Owner` to fit within that control
    /// when the badge fits. On an axis where the control is too small,
    /// the surrounding visible clip supplies the limit.
    ///
    /// The measured badge keeps its text size. It stops painting when
    /// the owner scrolls fully out of view.
    pub fn badged_anchor_with_clearance<M>(
        &self,
        anchor: View<M>,
        owner_id: &str,
        content: BadgeContent<'_>,
        options: BadgedAnchorOptions,
        boundary: BadgeBoundary,
    ) -> View<M> {
        self.anchored_badge(
            anchor,
            content,
            options,
            Some((owner_id, boundary)),
            LABEL_VERTICAL_OVERLAP,
        )
    }

    pub(crate) fn navigation_badged_anchor<M>(
        &self,
        anchor: View<M>,
        content: BadgeContent<'_>,
        options: BadgedAnchorOptions,
        clearance: Option<(&str, BadgeBoundary)>,
    ) -> View<M> {
        self.anchored_badge(
            anchor,
            content,
            options,
            clearance,
            NAVIGATION_LABEL_VERTICAL_OVERLAP,
        )
    }

    fn anchored_badge<M>(
        &self,
        anchor: View<M>,
        content: BadgeContent<'_>,
        options: BadgedAnchorOptions,
        clearance: Option<(&str, BadgeBoundary)>,
        label_vertical_overlap: f32,
    ) -> View<M> {
        let row = Node::new(Role::Group, "").styled(Style::row().align(Align::Start));
        let row = match options.direction {
            LayoutDirection::LeftToRight => row,
            LayoutDirection::RightToLeft => row
                .styled(Style::row().align(Align::Start).justify(Align::End))
                .reverse_row(),
        };
        if content == BadgeContent::Hidden {
            return View::new(row).children([anchor]);
        }

        let label = content.label();
        let (horizontal, vertical) = if label.is_some() {
            (LABEL_HORIZONTAL_OVERLAP, label_vertical_overlap)
        } else {
            (DOT_SIZE, DOT_SIZE)
        };
        let horizontal = Length::Px(-(artwork_inset(options.end_inset) + horizontal));
        // The bottom stays at the same distance from the artwork when text grows.
        let mut inset = Insets {
            bottom: Length::Px(-(artwork_inset(options.top_inset) + vertical)),
            ..Default::default()
        };
        let edge = match options.direction {
            LayoutDirection::LeftToRight => {
                inset.left = horizontal;
                OverlayHorizontalEdge::Right
            }
            LayoutDirection::RightToLeft => {
                inset.right = horizontal;
                OverlayHorizontalEdge::Left
            }
        };
        let face = Node::new(Role::Group, "");
        let face = if let Some((owner, boundary)) = clearance {
            face.fit_absolute_overlay(owner, edge, boundary)
        } else {
            face
        };

        // A zero-sized sibling follows the measured anchor's edge inside a
        // stretched row. Its stacking order keeps the badge above the anchor.
        let mut lane = Style::default()
            .width(Length::Px(0.))
            .height(Length::Px(0.));
        lane.z_index = 1;
        let lane = View::new(Node::new(Role::Group, "").styled(lane))
            .children([self.badge_view(face, label.as_deref(), Some(inset))])
            .without_interaction(None);
        View::new(row).children([anchor, lane])
    }

    fn badge_view<M>(&self, node: Node, label: Option<&str>, inset: Option<Insets>) -> View<M> {
        let size = if label.is_some() {
            LABEL_SIZE
        } else {
            DOT_SIZE
        };
        let mut style = Style::row()
            .align(Align::Center)
            .justify(Align::Center)
            .height(Length::Px(size))
            .corners(shape::FULL)
            .background(self.colors.error)
            .foreground(self.colors.on_error);
        style.min_width = size;
        if label.is_none() {
            style.width = Length::Px(size);
        }
        if let Some(inset) = inset {
            style = style.absolute(inset);
        }
        let node = node.styled(style);
        if let Some(label) = label {
            let space = || {
                View::new(
                    Node::new(Role::Group, "").styled(
                        Style::default()
                            .width(Length::Px(LABEL_SIDE_PADDING))
                            .height(Length::Px(0.)),
                    ),
                )
                .without_interaction(None)
            };
            let text = View::new(
                Node::new(Role::Group, "").text(label).styled(
                    Style::default()
                        .with_text_style(typography::LABEL_SMALL)
                        .foreground(self.colors.on_error),
                ),
            )
            .without_interaction(None);
            View::new(node.text_scaling(TextScaling::Expand)).children([space(), text, space()])
        } else {
            View::new(node)
        }
    }
}
