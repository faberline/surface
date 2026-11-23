use surface::{Align, Color, Length, Node, RasterImage, Role, Style};
use surface_app::View;

use super::{
    action_item, list_icon, list_icon_size, logical_row, row_alignment, row_copy, row_trailing,
    spacer, ListInteraction, ListItem, ListLines, ListPresentation, ListStyle, RowPlacement,
    BASELINE_END_INSET, CHECKBOX_SIZE, EDGE_INSET, FOCUS_WIDTH, ROW_PADDING, SLOT_GAP,
};
use crate::{
    shape::{self, LayoutDirection},
    Theme,
};

const MIN_TARGET: f32 = 48.;
const COMPACT_WIDTH: f32 = 144.;
const COMPACT_INSET: f32 = 8.;
const MIN_COPY_WIDTH: f32 = 32.;
const MIN_INLINE_MEDIA: f32 = 16.;

/// The preferred dimensions of a leading image in a Material list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListMediaKind {
    /// Square 56 × 56 dp thumbnail.
    Thumbnail,
    /// 100 × 56 dp video poster.
    VideoSmall,
    /// 114 × 64 dp video poster.
    VideoLarge,
    /// App-selected 80 × 80 dp thumbnail for a wider row.
    WideThumbnail,
}

impl ListMediaKind {
    fn dimensions(self) -> (f32, f32) {
        match self {
            Self::Thumbnail => (56., 56.),
            Self::VideoSmall => (100., 56.),
            Self::VideoLarge => (114., 64.),
            Self::WideThumbnail => (80., 80.),
        }
    }

    fn is_video(self) -> bool {
        matches!(self, Self::VideoSmall | Self::VideoLarge)
    }
}

/// Decorative image content. The renderer fills the slot with a centered crop.
/// Include essential image details in the row's text.
#[derive(Clone, Copy)]
pub struct ListMedia<'a> {
    pub image: &'a RasterImage,
    pub kind: ListMediaKind,
}

/// A list row with optional leading artwork.
pub struct MediaListItem<'a, M> {
    /// The artwork replaces the row's leading icon when supplied.
    pub row: ListItem<'a, M>,
    /// Reserve the row's `-media` suffix for the decorative image.
    pub media: Option<ListMedia<'a>>,
}

/// The interaction shared by a list with leading media.
#[derive(Clone, Copy)]
pub enum MediaListMode<'a> {
    Passive,
    Action,
    /// Each row has one checkbox action. The caller owns the selection.
    MultiSelect {
        is_selected: &'a dyn Fn(&str) -> bool,
    },
}

/// Placement and interaction of a list with leading media.
#[derive(Clone, Copy)]
pub struct MediaListOptions<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub style: ListStyle,
    pub direction: LayoutDirection,
    /// Actual width allocated by the parent after its insets.
    pub available_width: f32,
    pub mode: MediaListMode<'a>,
}

impl Theme {
    /// Build a list with optional leading thumbnail art or video posters.
    ///
    /// The image keeps its preferred aspect ratio as the row gets narrower.
    /// Very narrow rows stack their content. Widths below 48 dp or nonfinite
    /// widths return `None`. Use a clipped parent when the list needs to scroll.
    /// Baseline video posters sit at the leading edge. Interactive rows inset
    /// them 3 dp to leave room for the focus outline.
    ///
    /// The image is decorative. A row's command or checkbox is its only
    /// control. In multi-select mode the caller updates its model in response
    /// to `on_activate` and returns the checked state through `is_selected`.
    pub fn media_list<'a, M>(
        &self,
        options: MediaListOptions<'_>,
        items: impl IntoIterator<Item = MediaListItem<'a, M>>,
    ) -> Option<View<M>> {
        let width = options.available_width;
        if !width.is_finite() || width < MIN_TARGET {
            return None;
        }
        let interaction = match options.mode {
            MediaListMode::Passive => ListInteraction::Passive,
            MediaListMode::Action => ListInteraction::Action,
            MediaListMode::MultiSelect { is_selected } => ListInteraction::MultiSelect(is_selected),
        };
        let presentation = ListPresentation {
            style: options.style,
            direction: options.direction,
            interaction,
        };
        let items: Vec<_> = items.into_iter().collect();
        let end = items.len().saturating_sub(1);
        let rows = items.into_iter().enumerate().map(|(index, item)| {
            let id = item.row.id;
            let first = index == 0;
            let last = index == end;
            let layout = (item.media.is_some() || width < COMPACT_WIDTH).then_some(MediaRow {
                image: item.media,
                width,
            });
            let row = self.list_row_control_with_media(
                presentation,
                item.row,
                RowPlacement::Regular { first, last },
                None,
                layout,
            );
            if interaction.row_role() == Role::Button {
                action_item(id, row)
            } else {
                row
            }
        });
        Some(self.list_container(
            options.id,
            options.name,
            presentation,
            Length::Px(width),
            rows,
        ))
    }
}

#[derive(Clone, Copy)]
pub(super) struct MediaRow<'a> {
    image: Option<ListMedia<'a>>,
    width: f32,
}

impl MediaRow<'_> {
    fn vertical_inset(self, lines: ListLines) -> f32 {
        if self.width < COMPACT_WIDTH {
            return COMPACT_INSET;
        }
        match self.image.map(|media| media.kind) {
            Some(ListMediaKind::VideoLarge | ListMediaKind::WideThumbnail) => 12.,
            Some(_) if lines == ListLines::Three => 12.,
            Some(_) => 8.,
            None => ROW_PADDING,
        }
    }

    pub(super) fn content<M>(
        self,
        options: ListPresentation<'_>,
        item: &ListItem<'_, M>,
        mark: Option<View<M>>,
        ink: Color,
        supporting_ink: Color,
    ) -> View<M> {
        let narrow = self.width < COMPACT_WIDTH;
        let baseline = options.style == ListStyle::Baseline;
        let passive = matches!(options.interaction, ListInteraction::Passive);
        let (start, end, gap) = if narrow {
            (COMPACT_INSET, COMPACT_INSET, COMPACT_INSET)
        } else if baseline {
            let start = if self.image.is_some_and(|media| media.kind.is_video()) {
                if passive {
                    0.
                } else {
                    FOCUS_WIDTH
                }
            } else {
                EDGE_INSET
            };
            (start, BASELINE_END_INSET, EDGE_INSET)
        } else {
            (EDGE_INSET, EDGE_INSET, SLOT_GAP)
        };
        let available = self.width - start - end;
        let copy = row_copy(item, ink, supporting_ink, passive);

        // Cap optional trailing text before allocating the leading slot. The
        // middle keeps more room than either side, even with a trailing mark.
        let trailing_label_width = available * 0.22;
        let trailing_parts = usize::from(item.trailing.is_some()) + usize::from(mark.is_some());
        let trailing_width = if item.trailing.is_some() {
            trailing_label_width
        } else {
            0.
        } + if mark.is_some() { CHECKBOX_SIZE } else { 0. };
        let trailing_inset = trailing_width + trailing_parts as f32 * gap;
        let icon_width = if self.image.is_none() && item.leading.is_some() {
            list_icon_size(options.style)
        } else {
            0.
        };
        let has_leading = self.image.is_some() || icon_width > 0.;
        let main = available - trailing_inset - if has_leading { gap } else { 0. };
        let copy_min = MIN_COPY_WIDTH.max(trailing_inset - gap + 1.);
        let media_cap = (main - copy_min).min((main - 1.) / 2.);
        let inline = if self.image.is_some() {
            media_cap >= MIN_INLINE_MEDIA
        } else {
            main >= copy_min + icon_width
        };
        let align = row_alignment(item.lines);
        let line = |style: Style| {
            View::new(logical_row(
                style.width(Length::Percent(1.)),
                options.direction,
            ))
        };

        let body = if inline {
            let leading = self.leading(options.style, item, media_cap, supporting_ink);
            let trailing = row_trailing(
                item,
                supporting_ink,
                passive,
                Length::Px(trailing_label_width),
            );
            View::new(logical_row(
                Style::row()
                    .width(Length::Px(0.))
                    .grow(1.)
                    .align(align)
                    .gap(gap),
                options.direction,
            ))
            .children(
                leading
                    .into_iter()
                    .chain([copy])
                    .chain(trailing)
                    .chain(mark),
            )
        } else {
            let leading = self
                .leading(options.style, item, (available - 1.) / 2., supporting_ink)
                .map(|view| line(Style::row()).children([view]));
            let trailing = row_trailing(item, supporting_ink, passive, Length::Px(available))
                .map(|view| line(Style::row().justify(Align::End)).children([view]));
            let mark = mark.map(|view| line(Style::row().justify(Align::End)).children([view]));
            View::new(
                Node::new(Role::Group, "")
                    .styled(Style::column().width(Length::Px(0.)).grow(1.).gap(gap)),
            )
            .children(
                leading
                    .into_iter()
                    .chain([line(Style::row()).children([copy])])
                    .chain(trailing)
                    .chain(mark),
            )
        };
        let inset = self.vertical_inset(item.lines);
        let block = || {
            View::new(Node::new(Role::Group, "").styled(Style::column().height(Length::Px(inset))))
                .without_interaction(None)
        };
        View::new(Node::new(Role::Group, "").styled(Style::column().width(Length::Percent(1.))))
            .children([
                block(),
                line(Style::row().align(align)).children([spacer(start), body, spacer(end)]),
                block(),
            ])
    }

    fn leading<M>(
        self,
        style: ListStyle,
        item: &ListItem<'_, M>,
        media_cap: f32,
        icon_ink: Color,
    ) -> Option<View<M>> {
        if let Some(media) = self.image {
            let (preferred_width, preferred_height) = media.kind.dimensions();
            let width = preferred_width.min(media_cap);
            let height = preferred_height * (width / preferred_width);
            let corners = if style == ListStyle::Baseline {
                shape::NONE
            } else {
                shape::SMALL
            };
            return Some(
                View::new(
                    Node::new(Role::Group, "")
                        .id(format!("{}-media", item.id))
                        .styled(
                            Style::column()
                                .width(Length::Px(width))
                                .height(Length::Px(height))
                                .corners(corners),
                        )
                        .raster_image(media.image.clone()),
                )
                .without_interaction(None),
            );
        }
        item.leading.map(|icon| {
            let size = list_icon_size(style);
            list_icon(
                format!("{}-leading", item.id),
                size,
                icon.contour_dp(size, icon_ink),
            )
        })
    }
}
