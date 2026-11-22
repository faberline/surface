//! Material 3 bottom sheets with caller-controlled snap positions.

use surface::{Align, Color, Insets, Length, Node, Role, Style, TextScaling};
use surface_app::{CursorStyle, View};

use crate::{shape, typography, Elevation, IconButtonKind, Theme};

const MAX_WIDTH: f32 = 640.;
const CONTROL_SIZE: f32 = 48.;
const SCRIM_ALPHA: f32 = 82.;

/// How a bottom sheet relates to the main content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BottomSheetPresentation {
    /// Keep uncovered main content interactive.
    Standard,
    /// Open a dialog and block the main content until dismissal.
    Modal,
}

/// The caller's resting state for a bottom sheet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BottomSheetState {
    #[default]
    Closed,
    Peek,
    Expanded,
}

/// Resting heights in logical pixels. The layout clamps them to its height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BottomSheetHeights {
    peek: f32,
    expanded: f32,
}

impl BottomSheetHeights {
    /// Leave space for the controls at either anchor. A nonfinite height
    /// uses the minimum. Equal anchors allow opening directly to Expanded.
    pub fn new(peek: f32, expanded: f32) -> Self {
        let normalize = |height: f32| {
            if height.is_finite() {
                height.max(CONTROL_SIZE)
            } else {
                CONTROL_SIZE
            }
        };
        let peek = normalize(peek);
        Self {
            peek,
            expanded: normalize(expanded).max(peek),
        }
    }

    pub fn peek(self) -> f32 {
        self.peek
    }

    pub fn expanded(self) -> f32 {
        self.expanded
    }

    pub fn height(self, state: BottomSheetState) -> f32 {
        match state {
            BottomSheetState::Closed => 0.,
            BottomSheetState::Peek => self.peek,
            BottomSheetState::Expanded => self.expanded,
        }
    }
}

/// Placement, snap positions and localized names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BottomSheetOptions<'a> {
    /// Stable ID of the supplementary group or dialog.
    pub id: &'a str,
    /// Visible heading and accessible name of the panel.
    pub title: &'a str,
    /// Accessible name of the handle. Describe its action for the current state.
    pub handle_label: &'a str,
    /// Accessible name of the Close button.
    pub close_label: &'a str,
    /// Focus target when a standard sheet containing focus closes.
    /// A modal sheet restores the focus that preceded the dialog.
    pub restore_focus_to: &'a str,
    pub presentation: BottomSheetPresentation,
    pub heights: BottomSheetHeights,
}

/// Main view, supplementary view and control messages.
pub struct BottomSheetContent<M> {
    pub body: View<M>,
    pub panel: View<M>,
    pub on_toggle: M,
    pub on_dismiss: M,
}

impl Theme {
    /// Lay out a caller-controlled Material 3 bottom sheet.
    ///
    /// A standard sheet anchors to the supplied layout. Its Close button
    /// or Escape from inside it sends `on_dismiss`. An active modal handles
    /// Escape first. A modal sheet anchors to the viewport, traps focus,
    /// and also dismisses on Escape or an outside click.
    ///
    /// The handle sends `on_toggle`. The caller may switch between Peek and
    /// Expanded. The handle and Close button stay above the scrollable
    /// heading and content at both heights. An expanded standard sheet with
    /// equal heights shows a disabled handle and an enabled Close button.
    ///
    /// The layout reserves `{id}-layout`, `{id}-body`, `{id}-layer`,
    /// `{id}-header`, `{id}-handle`, `{id}-title`, `{id}-close` and
    /// `{id}-content`.
    ///
    /// [Material 3 bottom sheets](https://m3.material.io/components/bottom-sheets/overview)
    pub fn bottom_sheet_layout<M: Clone>(
        &self,
        options: BottomSheetOptions<'_>,
        state: BottomSheetState,
        content: BottomSheetContent<M>,
    ) -> View<M> {
        self.bottom_sheet_at_height(
            options,
            state,
            options.heights.height(state),
            content,
            |handle| handle,
        )
    }

    pub(super) fn bottom_sheet_at_height<M: Clone>(
        &self,
        options: BottomSheetOptions<'_>,
        state: BottomSheetState,
        visible_height: f32,
        content: BottomSheetContent<M>,
        bind_handle: impl FnOnce(View<M>) -> View<M>,
    ) -> View<M> {
        let BottomSheetContent {
            body,
            panel,
            on_toggle,
            on_dismiss,
        } = content;
        let id = options.id;
        let full_size = || {
            Style::column()
                .width(Length::Percent(1.))
                .height(Length::Percent(1.))
                .clip(true)
        };
        let layout = Node::new(Role::Group, "")
            .id(format!("{id}-layout"))
            .styled(full_size());
        let mut children = vec![View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-body"))
                .styled(full_size()),
        )
        .children([body])];
        let open = state != BottomSheetState::Closed;
        let height = if visible_height.is_finite() {
            visible_height.clamp(0., options.heights.expanded())
        } else {
            options.heights.height(state)
        };
        if open || height > 0. {
            // Keep the capture and modal scope alive when a drag reaches
            // zero before the caller receives its release event.
            let panel_height = if open { height.max(1.) } else { height };
            let header = self.bottom_sheet_header(
                options,
                state,
                on_toggle,
                on_dismiss.clone(),
                bind_handle,
            );
            let sheet = self.bottom_sheet_panel(options, panel_height, header, panel);
            let layer = self.bottom_sheet_layer(options, height);
            let rendered = match options.presentation {
                BottomSheetPresentation::Modal if open => {
                    sheet.modal_with_backdrop(layer, on_dismiss)
                }
                BottomSheetPresentation::Modal => {
                    View::new(layer).children([sheet]).visual_overlay()
                }
                BottomSheetPresentation::Standard => {
                    let sheet = if open {
                        sheet
                            .restore_focus_in_subtree_on_removal(options.restore_focus_to)
                            .on_cancel_within(on_dismiss)
                    } else {
                        sheet.without_interaction(Some(options.restore_focus_to))
                    };
                    View::new(layer).children([sheet])
                }
            };
            children.push(rendered);
        }
        View::new(layout).children(children)
    }

    fn bottom_sheet_header<M>(
        &self,
        options: BottomSheetOptions<'_>,
        state: BottomSheetState,
        on_toggle: M,
        on_dismiss: M,
        bind_handle: impl FnOnce(View<M>) -> View<M>,
    ) -> View<M> {
        let id = options.id;
        let disabled = options.presentation == BottomSheetPresentation::Standard
            && state == BottomSheetState::Expanded
            && options.heights.peek() == options.heights.expanded();
        let handle = self.bottom_sheet_handle(options, state, disabled, on_toggle);
        let handle = if state != BottomSheetState::Closed && !disabled {
            bind_handle(handle)
        } else {
            handle
        };
        let close = self
            .icon_button_with_text_scaling(
                &format!("{id}-close"),
                options.close_label,
                ("×", TextScaling::Fixed),
                IconButtonKind::Standard,
                None,
                false,
                on_dismiss,
            )
            .accessible_activation()
            .protect_composition_on_press();
        let spacer = View::new(
            Node::new(Role::Group, "")
                .shrink_to_fit_width()
                .styled(Style::default().width(Length::Px(CONTROL_SIZE))),
        );
        let mut center = Style::column()
            .width(Length::Px(0.))
            .grow(1.)
            .align(Align::Center);
        center.min_width = CONTROL_SIZE;
        let center = View::new(Node::new(Role::Group, "").styled(center)).children([handle]);
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-header"))
                .styled(
                    Style::row()
                        .width(Length::Percent(1.))
                        .height(Length::Px(CONTROL_SIZE)),
                ),
        )
        .children([spacer, center, close])
    }

    fn bottom_sheet_panel<M>(
        &self,
        options: BottomSheetOptions<'_>,
        height: f32,
        header: View<M>,
        panel: View<M>,
    ) -> View<M> {
        let id = options.id;
        let heading = View::new(
            Node::new(Role::Heading, options.title)
                .id(format!("{id}-title"))
                .text(options.title)
                .styled(
                    Style::default()
                        .with_text_style(typography::TITLE_LARGE)
                        .foreground(self.colors.on_surface),
                ),
        );
        let content = View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-content"))
                .styled(Style::column().height(Length::Px(0.)).grow(1.).clip(true)),
        )
        .children([View::new(
            Node::new(Role::Group, "").styled(Style::column().padding(16.).gap(16.)),
        )
        .children([heading, panel])])
        .accessibility_scroll_pages();
        let mut style = Style::column()
            .width(Length::Percent(1.))
            .height(Length::Px(height))
            .corners(shape::EXTRA_LARGE_TOP)
            .background(self.colors.surface_container_low)
            .clip(true);
        style.max_width = Length::Px(MAX_WIDTH);
        style.max_height = Length::Percent(1.);
        let role = match options.presentation {
            BottomSheetPresentation::Standard => Role::Group,
            BottomSheetPresentation::Modal => Role::Dialog,
        };
        View::new(
            Node::new(role, options.title)
                .id(id)
                .tab_index(-1)
                .styled(style)
                .box_shadows(self.elevation(Elevation::Level1)),
        )
        .children([header, content])
    }

    fn bottom_sheet_handle<M>(
        &self,
        options: BottomSheetOptions<'_>,
        state: BottomSheetState,
        disabled: bool,
        on_toggle: M,
    ) -> View<M> {
        let grip = View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Px(32.))
                    .height(Length::Px(4.))
                    .corners(shape::FULL)
                    .background(self.colors.on_surface_variant),
            ),
        );
        let handle = View::new(
            Node::new(Role::Button, options.handle_label)
                .id(format!("{}-handle", options.id))
                .disabled(disabled)
                .styled(
                    Style::column()
                        .width(Length::Px(CONTROL_SIZE))
                        .height(Length::Px(CONTROL_SIZE))
                        .align(Align::Center)
                        .justify(Align::Center)
                        .corners(shape::FULL)
                        .foreground(self.colors.primary)
                        .state_layer(self.colors.on_surface_variant),
                ),
        )
        .children([grip])
        .disclosure_expanded(state == BottomSheetState::Expanded);
        if disabled {
            handle
        } else {
            handle
                .on_press(on_toggle)
                .accessible_activation()
                .protect_composition_on_press()
                .cursor(CursorStyle::Pointer)
        }
    }

    fn bottom_sheet_layer(&self, options: BottomSheetOptions<'_>, height: f32) -> Node {
        let mut style = Style::column().align(Align::Center).justify(Align::End);
        style.z_index = 1000;
        style = match options.presentation {
            BottomSheetPresentation::Standard => style.absolute(Insets::all(0.)),
            BottomSheetPresentation::Modal => {
                let Color(red, green, blue, _) = self.colors.scrim;
                let alpha = SCRIM_ALPHA * (height / options.heights.peek()).min(1.);
                style.fixed(Insets::all(0.)).background(Color(
                    red,
                    green,
                    blue,
                    alpha.round() as u8,
                ))
            }
        };
        Node::new(Role::Group, "")
            .id(format!("{}-layer", options.id))
            .styled(style)
    }
}
