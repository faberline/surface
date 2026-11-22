//! Material 3 side sheets for supplementary content.

mod motion;

pub use motion::SideSheetMotion;

use surface::{Align, Color, Insets, Length, Node, Role, Style, TextScaling};
use surface_app::{HorizontalDrag, View};

use crate::{shape, shape::LayoutDirection, typography, Elevation, IconButtonKind, Theme};

const SHEET_WIDTH: f32 = 256.;
const DETACHED_MARGIN: f32 = 16.;
const COPLANAR_WIDTH_LIMIT: f32 = 0.6;
const SCRIM_ALPHA: f32 = 82.;

/// How a side sheet relates to the main content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideSheetPresentation {
    /// Layer over the main content. Uncovered main content stays interactive.
    StandardOverlay,
    /// Reserve space beside the main content. Both regions stay interactive.
    StandardCoplanar,
    /// Open a dialog and block the main content until it closes.
    Modal,
}

/// Whether the side sheet touches the viewport or layout edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideSheetStyle {
    Docked,
    Detached,
}

/// The sheet's logical edge. The direction determines its physical side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SideSheetEdge {
    Start,
    #[default]
    End,
}

/// Placement and localized names for a side sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SideSheetOptions<'a> {
    /// Stable ID of the supplementary group or dialog.
    pub id: &'a str,
    /// Visible heading and accessible name of the panel.
    pub title: &'a str,
    /// Accessible name of the Close button.
    pub close_label: &'a str,
    /// Focus target when a standard sheet containing focus closes.
    /// A modal sheet restores the focus that preceded the dialog.
    pub restore_focus_to: &'a str,
    pub presentation: SideSheetPresentation,
    pub style: SideSheetStyle,
    pub edge: SideSheetEdge,
    pub direction: LayoutDirection,
}

/// The caller's main view, supplementary view and dismissal message.
pub struct SideSheetContent<M> {
    pub body: View<M>,
    pub panel: View<M>,
    pub on_dismiss: M,
}

impl Theme {
    /// Lay out a caller-controlled Material 3 side sheet.
    ///
    /// Standard sheets occupy the supplied layout. A modal sheet occupies
    /// the viewport and opens a focus scope. Both provide a Close button and
    /// a scrollable content area. Escape closes a standard sheet when focus
    /// is inside it. If a modal is active, it handles Escape first.
    /// Escape or an outside click closes a modal sheet.
    ///
    /// Use a relatively wide layout for a coplanar sheet. The panel uses at
    /// most 60% of that layout so the main content has room at narrow widths.
    /// The caller can switch presentations at its own adaptive breakpoint.
    ///
    /// The layout reserves `{id}-layout`, `{id}-body`, `{id}-layer`,
    /// `{id}-header`, `{id}-title`, `{id}-close` and `{id}-content`.
    ///
    /// [Material 3 side sheets](https://m3.material.io/components/side-sheets/overview)
    /// [MDC side sheet tokens](https://github.com/material-components/material-components-android/blob/66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad/lib/java/com/google/android/material/sidesheet/res/values/tokens.xml)
    pub fn side_sheet<M: Clone>(
        &self,
        options: SideSheetOptions<'_>,
        open: bool,
        content: SideSheetContent<M>,
    ) -> View<M> {
        let reveal = if open { 1. } else { 0. };
        self.side_sheet_layout(options, open, reveal, content, |panel| panel)
    }

    /// Use the caller's animation state and enable horizontal dismissal.
    ///
    /// Keep the same motion state across rebuilds. Pass drag messages back to
    /// that state with the reported travel distance. The distance uses the
    /// measured panel width and includes any detached margin on the edge.
    /// The caller still owns the logical open state.
    pub fn side_sheet_with_motion<M: Clone>(
        &self,
        options: SideSheetOptions<'_>,
        open: bool,
        motion: &SideSheetMotion,
        content: SideSheetContent<M>,
        on_drag: impl Fn(HorizontalDrag, f32) -> M + 'static,
    ) -> View<M> {
        let margin = options.margin();
        self.side_sheet_layout(options, open, motion.reveal(), content, |panel| {
            if open && !motion.is_settling() {
                panel.on_horizontal_drag_with_bounds(motion.session(), move |event, bounds| {
                    on_drag(event, bounds.width + margin)
                })
            } else {
                panel
            }
        })
    }

    fn side_sheet_layout<M: Clone>(
        &self,
        options: SideSheetOptions<'_>,
        open: bool,
        reveal: f32,
        content: SideSheetContent<M>,
        bind_drag: impl FnOnce(View<M>) -> View<M>,
    ) -> View<M> {
        let SideSheetContent {
            body,
            panel,
            on_dismiss,
        } = content;
        let id = options.id;
        let mut layout = Node::new(Role::Group, "")
            .id(format!("{id}-layout"))
            .styled(
                Style::row()
                    .width(Length::Percent(1.))
                    .height(Length::Percent(1.))
                    .clip(true),
            );
        let body = View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-body"))
                .styled(Style::column().width(Length::Px(0.)).grow(1.).clip(true)),
        )
        .children([body]);
        let mut children = vec![body];
        if options.presentation == SideSheetPresentation::StandardCoplanar && reveal > 0. {
            // Keep the body in the same tree position as space opens or closes.
            if options.on_left() {
                layout = layout.reverse_row();
            }
            let mut reserve = Style::column().width(Length::Px(options.layer_width() * reveal));
            reserve.max_width = Length::Percent(COPLANAR_WIDTH_LIMIT * reveal);
            children.push(
                View::new(Node::new(Role::Group, "").styled(reserve)).without_interaction(None),
            );
        }
        if open || reveal > 0. {
            let sheet =
                bind_drag(self.side_sheet_panel(options, reveal, panel, on_dismiss.clone()));
            let layer = self.side_sheet_layer(options, reveal);
            let rendered = if options.presentation == SideSheetPresentation::Modal {
                if open {
                    sheet.modal_with_backdrop(layer, on_dismiss)
                } else {
                    View::new(layer).children([sheet]).visual_overlay()
                }
            } else {
                let sheet = if open {
                    sheet
                        .restore_focus_in_subtree_on_removal(options.restore_focus_to)
                        .on_cancel_within(on_dismiss)
                } else {
                    sheet.without_interaction(Some(options.restore_focus_to))
                };
                View::new(layer).children([sheet])
            };
            children.push(rendered);
        }
        View::new(layout).children(children)
    }

    fn side_sheet_panel<M>(
        &self,
        options: SideSheetOptions<'_>,
        reveal: f32,
        panel: View<M>,
        dismiss: M,
    ) -> View<M> {
        let id = options.id;
        let title = View::new(
            Node::new(Role::Heading, options.title)
                .id(format!("{id}-title"))
                .text(options.title)
                .shrink_to_fit_width()
                .styled(
                    Style::default()
                        .with_text_style(typography::TITLE_LARGE)
                        .grow(1.)
                        .foreground(self.colors.on_surface),
                ),
        );
        let close = self
            .icon_button_with_text_scaling(
                &format!("{id}-close"),
                options.close_label,
                ("×", TextScaling::Fixed),
                IconButtonKind::Standard,
                None,
                false,
                dismiss,
            )
            .accessible_activation()
            .protect_composition_on_press();
        let mut header = Node::new(Role::Group, "")
            .id(format!("{id}-header"))
            .text_scaling(TextScaling::Expand)
            .styled(Style::row().padding(12.).gap(12.).align(Align::Center));
        if options.direction == LayoutDirection::RightToLeft {
            header = header.reverse_row();
        }
        let header = View::new(header).children([title, close]);
        let content = View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-content"))
                .styled(
                    Style::column()
                        .height(Length::Px(0.))
                        .grow(1.)
                        .padding(16.)
                        .clip(true),
                ),
        )
        .children([panel])
        .accessibility_scroll_pages();
        let modal = options.presentation == SideSheetPresentation::Modal;
        let (role, color, elevation) = if modal {
            (
                Role::Dialog,
                self.colors.surface_container_low,
                Elevation::Level1,
            )
        } else {
            (Role::Group, self.colors.surface, Elevation::Level0)
        };
        let corners = match options.style {
            SideSheetStyle::Detached => shape::LARGE,
            SideSheetStyle::Docked if modal && options.on_left() => shape::LARGE_END,
            SideSheetStyle::Docked if modal => shape::LARGE_START,
            SideSheetStyle::Docked => shape::NONE,
        };
        let mut style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Percent(1.))
            .corners(corners)
            .background(color)
            .clip(true);
        style.max_width = Length::Px(SHEET_WIDTH);
        // A detached panel travels its own width plus the outer margin.
        let hidden = 1. - reveal;
        let side = if options.on_left() {
            style.inset.left = Length::Px(-options.margin() * hidden);
            -1.
        } else {
            style.inset.right = Length::Px(-options.margin() * hidden);
            1.
        };
        let mut node = Node::new(role, options.title)
            .id(id)
            .styled(style)
            .box_shadows(self.elevation(elevation))
            .slide_horizontally(side * hidden);
        if options.presentation == SideSheetPresentation::StandardOverlay {
            // The painted panel owns pointer hits in its blank areas. It
            // stays out of the Tab order and has no activation binding.
            node = node.tab_index(-1);
        }
        View::new(node).children([header, content])
    }

    fn side_sheet_layer(&self, options: SideSheetOptions<'_>, reveal: f32) -> Node {
        let mut layer = Style::row()
            .padding(options.margin())
            .justify(if options.on_left() {
                Align::Start
            } else {
                Align::End
            });
        layer.z_index = 1000;
        if options.presentation == SideSheetPresentation::Modal {
            let Color(red, green, blue, _) = self.colors.scrim;
            layer = layer.fixed(Insets::all(0.)).background(Color(
                red,
                green,
                blue,
                (SCRIM_ALPHA * reveal).round() as u8,
            ));
        } else {
            let mut insets = Insets {
                top: Length::Px(0.),
                bottom: Length::Px(0.),
                ..Insets::default()
            };
            if options.on_left() {
                insets.left = Length::Px(0.);
            } else {
                insets.right = Length::Px(0.);
            }
            layer = layer
                .absolute(insets)
                .width(Length::Px(options.layer_width()));
            layer.max_width = Length::Percent(
                if options.presentation == SideSheetPresentation::StandardCoplanar {
                    COPLANAR_WIDTH_LIMIT
                } else {
                    1.
                },
            );
        }
        Node::new(Role::Group, "")
            .id(format!("{}-layer", options.id))
            .styled(layer)
    }
}

impl SideSheetOptions<'_> {
    fn on_left(self) -> bool {
        matches!(
            (self.edge, self.direction),
            (SideSheetEdge::Start, LayoutDirection::LeftToRight)
                | (SideSheetEdge::End, LayoutDirection::RightToLeft)
        )
    }

    fn margin(self) -> f32 {
        match self.style {
            SideSheetStyle::Docked => 0.,
            SideSheetStyle::Detached => DETACHED_MARGIN,
        }
    }

    fn layer_width(self) -> f32 {
        SHEET_WIDTH + 2. * self.margin()
    }
}
