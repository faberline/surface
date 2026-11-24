//! Controlled Material Search overlay. The caller owns the query and results.

use surface::{Align, Color, Corners, Insets, Length, Node, Rect, Role, Style, TextScaling};
use surface_app::{CursorStyle, FocusNavigation, Key, TextAction, View};

use crate::{
    search_layout::{plan_search_layout, SearchLayout, SearchPresentation, SearchStyle},
    search_motion::{SearchOverlayMotion, SearchOverlayMotionTarget},
    shape::{self, LayoutDirection},
    typography, IconButtonKind, Theme,
};

const TOUCH_TARGET: f32 = 48.;
const CONTROL_GAP: f32 = 4.;
const EDITOR_TEXT_INSET: f32 = 4.;
const MIN_EDITOR_WIDTH: f32 = 24.;
const MIN_MOTION_BAR_WIDTH: f32 = MIN_EDITOR_WIDTH + 2. * (TOUCH_TARGET + CONTROL_GAP) + 8.;

/// Accessible labels, state and geometry for [Theme::search_overlay].
#[derive(Clone, Copy, Debug)]
pub struct SearchOverlayOptions<'a> {
    /// Unique, stable prefix for the editor and overlay controls.
    pub id: &'a str,
    pub open: bool,
    pub title: &'a str,
    pub input_label: &'a str,
    /// Hint shown while the query is empty. Use the same localized words
    /// for `input_label` when the hint is the field's only label.
    pub placeholder: Option<&'a str>,
    pub results_label: &'a str,
    pub back_label: &'a str,
    pub clear_label: &'a str,
    pub query: &'a str,
    pub style: SearchStyle,
    pub direction: LayoutDirection,
    /// Safe area in window coordinates. Recompute it after a resize.
    pub viewport: Rect,
    /// The caller's stable resting bar allocation before margins. Leave it
    /// absent until measured, or when the overlay always fills the viewport.
    pub anchor: Option<Rect>,
    /// The preferred outer height when docked. `None` uses the available room.
    pub preferred_docked_height: Option<f32>,
    /// A fallback when the original focus target vanishes while Search is open.
    pub restore_focus_to: Option<&'a str>,
    /// Localized result count or empty-state announcement. Place any visible
    /// empty-state content in the results slot.
    pub results_status: Option<&'a str>,
}

impl SearchOverlayOptions<'_> {
    /// Snapshot current geometry and state before advancing a retained motion.
    pub fn motion_target(&self) -> SearchOverlayMotionTarget {
        let destination = self.layout();
        let resting_bar = self
            .anchor
            .and_then(|anchor| self.style.resting_bar_bounds(anchor))
            .filter(|bar| {
                bar.width >= MIN_MOTION_BAR_WIDTH
                    && bar.height >= TOUCH_TARGET
                    && bar.x >= self.viewport.x
                    && bar.y >= self.viewport.y
                    && bar.x + bar.width <= self.viewport.x + self.viewport.width
                    && bar.y + bar.height <= self.viewport.y + self.viewport.height
                    && destination.is_some_and(|destination| {
                        destination.bar.width >= MIN_MOTION_BAR_WIDTH
                            && destination.bar.height >= TOUCH_TARGET
                    })
            });
        SearchOverlayMotionTarget {
            open: self.open,
            style: self.style,
            resting_bar,
            destination,
        }
    }

    fn layout(&self) -> Option<SearchLayout> {
        plan_search_layout(
            self.viewport,
            self.anchor,
            self.style,
            self.direction,
            self.preferred_docked_height,
        )
    }
}

/// Caller-authored result views and search actions.
pub struct SearchOverlayContent<M> {
    /// A results or suggestions view with stable IDs for interactive children.
    /// It may contain a roving group for result arrow navigation.
    pub results: View<M>,
    /// Optional plain ArrowDown target from the input. Supply the first
    /// enabled result from the current filtered list. The message may mount
    /// or reveal the target before focus moves. It should not activate it.
    pub first_result: Option<FocusNavigation<M>>,
    pub on_submit: M,
    pub on_clear: M,
    pub on_close: M,
}

impl Theme {
    /// Render the controlled open Search overlay as a sibling of page content.
    ///
    /// Docked and Full-screen share `{id}-dialog`, `{id}-input`, and
    /// `{id}-results`. Keep these IDs and the authored query across resizes
    /// to retain editing state, focus, and scroll where the new viewport fits.
    /// The modal initially focuses the editor and returns focus on dismissal.
    ///
    /// Plain Enter in the editor sends `on_submit` after IME composition.
    /// Clear sends `on_clear`. Back, unhandled Escape, and backdrop dismissal
    /// send `on_close`. Render the caller's new model on the next rebuild.
    pub fn search_overlay<M: Clone + 'static>(
        &self,
        options: SearchOverlayOptions<'_>,
        content: SearchOverlayContent<M>,
        on_query: impl Fn(String) -> M + 'static,
    ) -> Option<View<M>> {
        if !options.open {
            return None;
        }
        Some(self.search_overlay_at(options, content, on_query, options.layout()?, 1.))
    }

    /// Build Search at the current frame of a retained motion. Advance motion
    /// with [SearchOverlayOptions::motion_target] from the same caller state.
    /// The live editor and scope remain while `open` is true. Setting `open`
    /// false immediately leaves only an inert closing shell, if visible.
    pub fn search_overlay_with_motion<M: Clone + 'static>(
        &self,
        options: SearchOverlayOptions<'_>,
        content: SearchOverlayContent<M>,
        on_query: impl Fn(String) -> M + 'static,
        motion: &SearchOverlayMotion,
    ) -> Option<View<M>> {
        let target = options.motion_target();
        let destination = target.destination?;
        let progress = motion.progress();
        let Some(rest) = target.resting_bar else {
            return self.search_overlay(options, content, on_query);
        };
        let layout = destination.interpolate_from(rest, progress);
        if options.open {
            Some(self.search_overlay_at(options, content, on_query, layout, progress))
        } else {
            (progress > 0.).then(|| self.closing_search(options, layout, progress))
        }
    }

    fn search_overlay_at<M: Clone + 'static>(
        &self,
        options: SearchOverlayOptions<'_>,
        content: SearchOverlayContent<M>,
        on_query: impl Fn(String) -> M + 'static,
        layout: SearchLayout,
        progress: f32,
    ) -> View<M> {
        let SearchOverlayContent {
            results,
            first_result,
            on_submit,
            on_clear,
            on_close,
        } = content;
        let id = |suffix| format!("{}-{suffix}", options.id);
        let input_id = id("input");
        let panel = Node::new(Role::Dialog, options.title)
            .id(id("dialog"))
            .tab_index(-1)
            .focus_color(Color::TRANSPARENT)
            .styled(panel_style(self, layout, options.style, progress));

        let results_live = progress == 1. || layout.results_viewport.height >= TOUCH_TARGET;
        let first_result = first_result.filter(|_| results_live);

        let input = search_input(
            self,
            options,
            layout,
            &input_id,
            on_query,
            on_submit,
            first_result,
        );
        let bar = search_bar(
            self,
            options,
            layout,
            input,
            &input_id,
            on_clear,
            on_close.clone(),
        );
        let mut children = vec![bar];
        if let Some(divider) = layout.divider {
            children.push(View::new(
                Node::new(Role::Separator, "").id(id("divider")).styled(
                    placed(Style::column(), divider, layout.bounds)
                        .background(self.colors.outline_variant),
                ),
            ));
        }

        let results_view = View::new(
            Node::new(Role::Group, options.results_label)
                .id(id("results"))
                .styled(
                    placed(
                        Style::column(),
                        layout.results_viewport,
                        layout.results_surface,
                    )
                    .clip(true),
                ),
        )
        .children([results.restore_focus_in_subtree_on_removal(&input_id)])
        .accessibility_scroll_pages();
        let results_view = if results_live {
            results_view
        } else {
            results_view.without_interaction(Some(&input_id))
        };
        children.push(
            View::new(
                Node::new(Role::Group, "")
                    .id(id("results-surface"))
                    .styled(results_style(self, layout, options.style)),
            )
            .children([results_view]),
        );
        if let Some(status) = options.results_status {
            // Keep announcements projected if the results are scrolled away.
            children.push(View::new(
                Node::new(Role::Status, status)
                    .id(id("status"))
                    .styled(placed(
                        Style::column(),
                        Rect::new(layout.bounds.x, layout.bounds.y, 1., 1.),
                        layout.bounds,
                    )),
            ));
        }

        let mut panel = View::new(panel)
            .children(children)
            .modal_initial_focus(input_id);
        if let Some(target) = options.restore_focus_to {
            panel = panel.modal_focus_fallback(target);
        }
        panel.modal_with_backdrop(
            backdrop(self, id("layer"), layout.presentation, progress),
            on_close,
        )
    }

    fn closing_search<M>(
        &self,
        options: SearchOverlayOptions<'_>,
        layout: SearchLayout,
        progress: f32,
    ) -> View<M> {
        let id = |suffix| format!("{}-closing-{suffix}", options.id);
        let bar = bar_style(self, layout, options.style);
        let mut children = vec![View::new(
            Node::new(Role::Group, "").id(id("bar")).styled(bar),
        )];
        if let Some(divider) = layout.divider {
            children.push(View::new(
                Node::new(Role::Group, "").id(id("divider")).styled(
                    placed(Style::column(), divider, layout.bounds)
                        .background(self.colors.outline_variant),
                ),
            ));
        }
        children.push(View::new(
            Node::new(Role::Group, "")
                .id(id("results-surface"))
                .styled(results_style(self, layout, options.style)),
        ));
        let panel = View::new(
            Node::new(Role::Group, "")
                .id(id("shell"))
                .styled(panel_style(self, layout, options.style, progress)),
        )
        .children(children);
        View::new(backdrop(self, id("layer"), layout.presentation, progress))
            .children([panel])
            .visual_overlay()
    }
}

fn backdrop(theme: &Theme, id: String, presentation: SearchPresentation, progress: f32) -> Node {
    let mut style = Style::column().fixed(Insets::all(0.));
    if presentation == SearchPresentation::Docked {
        let Color(red, green, blue, _) = theme.colors.scrim;
        style = style.background(Color(red, green, blue, (82. * progress).round() as u8));
    }
    style.z_index = 1000;
    Node::new(Role::Group, "").id(id).styled(style)
}

fn panel_style(theme: &Theme, layout: SearchLayout, style: SearchStyle, progress: f32) -> Style {
    let docked = layout.presentation == SearchPresentation::Docked;
    let color = match (docked, style) {
        (true, SearchStyle::Divided) => theme.colors.surface_container_high,
        (true, SearchStyle::Contained) => Color::TRANSPARENT,
        (false, _) => theme.colors.surface_container_low,
    };
    let panel = placed(Style::column(), layout.bounds, Rect::default()).background(color);
    if docked && style == SearchStyle::Divided {
        panel.corners(shape::EXTRA_LARGE)
    } else if !docked && progress < 1. {
        panel.corners(Corners::all(28. * (1. - progress)))
    } else {
        panel
    }
}

fn results_style(theme: &Theme, layout: SearchLayout, style: SearchStyle) -> Style {
    let results = placed(Style::column(), layout.results_surface, layout.bounds);
    if layout.presentation == SearchPresentation::Docked && style == SearchStyle::Contained {
        results
            .background(theme.colors.surface_container_high)
            .corners(shape::SMALL)
    } else {
        results
    }
}

fn bar_style(theme: &Theme, layout: SearchLayout, style: SearchStyle) -> Style {
    let bar = placed(Style::column(), layout.bar, layout.bounds);
    if style == SearchStyle::Contained {
        bar.background(theme.colors.surface_container_high)
            .corners(shape::FULL)
    } else {
        bar
    }
}

fn search_input<M: Clone + 'static>(
    theme: &Theme,
    options: SearchOverlayOptions<'_>,
    layout: SearchLayout,
    input_id: &str,
    on_query: impl Fn(String) -> M + 'static,
    on_submit: M,
    first_result: Option<FocusNavigation<M>>,
) -> View<M> {
    let input = View::new(
        Node::new(Role::TextInput, options.input_label)
            .id(input_id)
            .text(options.query)
            .styled(
                Style::column()
                    .width(Length::Px(0.))
                    .height(Length::Px(TOUCH_TARGET.min(layout.controls.height)))
                    .grow(1.)
                    .corners(match options.style {
                        SearchStyle::Contained => shape::FULL,
                        SearchStyle::Divided => shape::EXTRA_SMALL,
                    })
                    .with_text_style(typography::BODY_LARGE)
                    .foreground(theme.colors.on_surface),
            ),
    )
    .on_input(on_query)
    .center_single_line_text()
    .inset_single_line_text_horizontally(EDITOR_TEXT_INSET)
    .on_key(move |press| {
        (!press.repeat && press.text_action() == Some(TextAction::Submit))
            .then(|| on_submit.clone())
    })
    .cursor(CursorStyle::Text);
    let input = if options.direction == LayoutDirection::RightToLeft {
        input.right_align_single_line_text()
    } else {
        input
    };
    let input = if let Some(hint) = options.placeholder {
        input.text_input_placeholder(hint, theme.colors.on_surface_variant)
    } else {
        input
    };
    if let Some(first) = first_result.filter(|_| layout.results_viewport.height > 0.) {
        input.on_text_input_arrow_navigation(move |press| {
            (press.key == Key::ArrowDown).then(|| first.clone())
        })
    } else {
        input
    }
}

fn search_bar<M>(
    theme: &Theme,
    options: SearchOverlayOptions<'_>,
    layout: SearchLayout,
    input: View<M>,
    input_id: &str,
    on_clear: M,
    on_close: M,
) -> View<M> {
    let rtl = options.direction == LayoutDirection::RightToLeft;
    let id = |suffix| format!("{}-{suffix}", options.id);
    let mut controls = Vec::new();
    let room_for_buttons = layout.controls.width - MIN_EDITOR_WIDTH;
    if layout.controls.height >= TOUCH_TARGET && room_for_buttons >= TOUCH_TARGET + CONTROL_GAP {
        controls.push(search_button(
            theme,
            &id("back"),
            options.back_label,
            if rtl { "→" } else { "←" },
            on_close,
        ));
    }
    controls.push(input);
    if !options.query.is_empty()
        && layout.controls.height >= TOUCH_TARGET
        && room_for_buttons >= 2. * (TOUCH_TARGET + CONTROL_GAP)
    {
        controls.push(
            search_button(theme, &id("clear"), options.clear_label, "×", on_clear)
                .restore_focus_on_removal(input_id),
        );
    }
    let controls_style = placed(
        Style::row().align(Align::Center).gap(CONTROL_GAP),
        layout.controls,
        layout.bar,
    );
    let controls_node = Node::new(Role::Group, "").styled(controls_style);
    let controls_node = if rtl {
        controls_node.reverse_row()
    } else {
        controls_node
    };
    let bar = bar_style(theme, layout, options.style);
    View::new(Node::new(Role::Group, "").id(id("bar")).styled(bar))
        .children([View::new(controls_node).children(controls)])
}

fn search_button<M>(theme: &Theme, id: &str, label: &str, icon: &str, message: M) -> View<M> {
    theme
        .icon_button_with_text_scaling(
            id,
            label,
            (icon, TextScaling::Fixed),
            IconButtonKind::Standard,
            None,
            false,
            message,
        )
        .accessible_activation()
        .suppress_repeated_activation()
        .protect_composition_on_press()
}

fn placed(style: Style, rect: Rect, parent: Rect) -> Style {
    style
        .width(Length::Px(rect.width))
        .height(Length::Px(rect.height))
        .absolute(Insets {
            left: Length::Px(rect.x - parent.x),
            top: Length::Px(rect.y - parent.y),
            ..Insets::default()
        })
}
