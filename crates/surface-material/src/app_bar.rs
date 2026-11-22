//! Material 3 app bars for titles, navigation, actions and entry to Search.

use surface::{Align, Length, Node, Role, Style, TextScaling, TextStyle};
use surface_app::{CursorStyle, View};

use crate::{
    motion::STATE_CHANGE, shape, shape::LayoutDirection, typography, vector_icon::VectorIconSize,
    Elevation, IconButtonKind, Theme, VectorIcon,
};

const SMALL_HEIGHT: f32 = 64.;
const CONTROL_SIZE: f32 = 48.;
const EDGE_SPACE: f32 = 4.;

/// The kind of app bar at the top of a page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppBarKind {
    #[default]
    Small,
    CenterAligned,
    MediumFlexible,
    LargeFlexible,
    Search,
}

impl AppBarKind {
    /// The difference between the nominal expanded and collapsed heights.
    /// A bar may grow beyond the nominal heights when its content needs room.
    pub fn collapse_distance(self, has_subtitle: bool) -> f32 {
        match (self, has_subtitle) {
            (Self::MediumFlexible, false) => 112. - SMALL_HEIGHT,
            (Self::MediumFlexible, true) => 136. - SMALL_HEIGHT,
            (Self::LargeFlexible, false) => 120. - SMALL_HEIGHT,
            (Self::LargeFlexible, true) => 152. - SMALL_HEIGHT,
            _ => 0.,
        }
    }

    fn flexible(self) -> bool {
        matches!(self, Self::MediumFlexible | Self::LargeFlexible)
    }
}

/// Localized names, layout and caller-controlled scroll appearance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppBarOptions<'a> {
    pub id: &'a str,
    /// Page title, or the group name for the Search kind.
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub kind: AppBarKind,
    pub direction: LayoutDirection,
    /// 0 is expanded and 1 is collapsed. Only flexible bars use this value.
    pub collapse: f32,
    /// The main content is scrolled under this bar.
    pub scrolled: bool,
}

impl AppBarOptions<'_> {
    pub fn collapse_distance(self) -> f32 {
        self.kind.collapse_distance(self.subtitle.is_some())
    }
}

/// Leading control, trailing controls and optional Search entry.
///
/// Supply actions in logical order. Use compact controls in the aligned
/// top row, and place additional actions in an overflow menu as needed.
pub struct AppBarContent<M> {
    pub leading: Option<View<M>>,
    pub actions: Vec<View<M>>,
    /// The Search kind displays this view in place of the visual title.
    pub search: Option<View<M>>,
}

impl Theme {
    /// A named 48 dp tap target with a fixed 24 dp icon for app bar slots.
    pub fn app_bar_icon_button<M>(
        &self,
        id: &str,
        label: &str,
        icon: &str,
        disabled: bool,
        on_press: M,
    ) -> View<M> {
        self.icon_button_with_text_scaling(
            id,
            label,
            (icon, TextScaling::Fixed),
            IconButtonKind::Standard,
            None,
            disabled,
            on_press,
        )
        .accessible_activation()
    }

    /// Create a Material 3 app bar.
    ///
    /// Place it above the page's scroll container. Keep the same `id` and
    /// control IDs as the bar changes state. Flexible bars move a single
    /// heading from below the actions to their row as they collapse.
    /// The caller sets the collapse fraction and the scrolled state.
    ///
    /// Reserves `{id}-title`, `{id}-subtitle`, `{id}-leading`,
    /// `{id}-actions` and `{id}-controls`.
    ///
    /// [Material 3 app bars](https://m3.material.io/components/app-bars/overview)
    pub fn app_bar<M>(&self, options: AppBarOptions<'_>, content: AppBarContent<M>) -> View<M> {
        let AppBarContent {
            leading,
            actions,
            search,
        } = content;
        let collapse = if options.kind.flexible() && options.collapse.is_finite() {
            options.collapse.clamp(0., 1.)
        } else {
            0.
        };
        let lower_heading = options.kind.flexible() && collapse < 0.5;
        let centered = options.kind == AppBarKind::CenterAligned;
        let has_leading = leading.is_some();
        let has_actions = !actions.is_empty();
        let outer_width = centered.then(|| {
            let slots = actions.len().max(usize::from(has_leading));
            (slots as f32 * CONTROL_SIZE).max(EDGE_SPACE * 2.)
        });
        let leading_style = outer_lane(outer_width, has_leading);
        let actions_style = outer_lane(outer_width, has_actions).justify(Align::End);
        let leading = View::new(logical_row(
            Node::new(Role::Group, "")
                .id(format!("{}-leading", options.id))
                .styled(leading_style),
            options.direction,
        ))
        .children(leading);
        let actions = View::new(logical_row(
            Node::new(Role::Group, "")
                .id(format!("{}-actions", options.id))
                .styled(actions_style),
            options.direction,
        ))
        .children(actions);
        let has_search = options.kind == AppBarKind::Search && search.is_some();
        let middle = if lower_heading {
            None
        } else if has_search {
            search
        } else {
            Some(self.app_bar_heading(options, centered, false, collapse))
        };
        let middle = View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Px(0.))
                    .grow(1.)
                    .align(if centered {
                        Align::Center
                    } else {
                        Align::Stretch
                    }),
            ),
        )
        .children(middle);
        let mut row = Style::row()
            .width(Length::Percent(1.))
            .padding(EDGE_SPACE)
            .gap(EDGE_SPACE)
            .align(Align::Center);
        row.min_height = if lower_heading {
            SMALL_HEIGHT - 16. * collapse
        } else {
            SMALL_HEIGHT
        };
        let controls = View::new(logical_row(
            Node::new(Role::Group, "")
                .id(format!("{}-controls", options.id))
                .styled(row),
            options.direction,
        ))
        .children([leading, middle, actions]);
        let mut style = Style::column()
            .width(Length::Percent(1.))
            .background(if options.scrolled {
                self.colors.surface_container
            } else {
                self.colors.surface
            })
            .transition(STATE_CHANGE);
        let expanded_space = options.collapse_distance() * (1. - collapse);
        style.min_height = SMALL_HEIGHT + expanded_space;
        let elevation = if options.scrolled {
            Elevation::Level2
        } else {
            Elevation::Level0
        };
        let heading = lower_heading.then(|| {
            // The title row and bottom space shrink before the heading moves
            // into the controls. Their measured minimums still fit large text.
            let mut label_row = Style::row().gap(16.);
            label_row.min_height = (expanded_space + 16. * collapse).max(0.);
            let title_flow = View::new(
                Node::new(Role::Group, "").styled(
                    Style::column()
                        .width(Length::Px(0.))
                        .grow(1.)
                        .justify(Align::End),
                ),
            )
            .children([
                self.app_bar_heading(options, false, true, collapse),
                View::new(
                    Node::new(Role::Group, "")
                        .styled(Style::column().height(Length::Px(12. * (1. - 2. * collapse)))),
                ),
            ]);
            View::new(Node::new(Role::Group, "").styled(label_row)).children([
                View::new(Node::new(Role::Group, "")),
                title_flow,
                View::new(Node::new(Role::Group, "")),
            ])
        });
        View::new(
            Node::new(Role::Group, if has_search { options.title } else { "" })
                .id(options.id)
                .styled(style)
                .box_shadows(self.elevation(elevation)),
        )
        .children(std::iter::once(controls).chain(heading))
    }

    /// Create the named Search trigger displayed in an app bar. It opens
    /// the caller's Search View on a click, Enter, Space or platform action.
    /// Its text is a label. It has no text editor or query state.
    pub fn app_bar_search_entry<M>(&self, id: &str, label: &str, on_press: M) -> View<M> {
        self.app_bar_search_entry_in_direction(id, label, LayoutDirection::LeftToRight, on_press)
    }

    /// Create a Search trigger with its icon at the logical start.
    pub fn app_bar_search_entry_in_direction<M>(
        &self,
        id: &str,
        label: &str,
        direction: LayoutDirection,
        on_press: M,
    ) -> View<M> {
        let text = View::new(
            Node::new(Role::Text, "")
                .text(label)
                .shrink_to_fit_width()
                .styled(
                    Style::default()
                        .with_text_style(typography::BODY_LARGE)
                        .foreground(self.colors.on_surface_variant),
                ),
        );
        let mut style = Style::row()
            .width(Length::Percent(1.))
            .padding(12.)
            .gap(12.)
            .align(Align::Center)
            .corners(shape::FULL)
            .foreground(self.colors.secondary)
            .background(self.colors.surface_container_high)
            .state_layer(self.colors.on_surface_variant)
            .transition(STATE_CHANGE);
        style.min_height = 56.;
        View::new(logical_row(
            Node::new(Role::Button, label)
                .id(id)
                .styled(style)
                .box_shadows(self.elevation(Elevation::Level3)),
            direction,
        ))
        .children([
            search_icon().view(VectorIconSize::Dp24, self.colors.on_surface),
            text,
        ])
        .on_press(on_press)
        .accessible_activation()
        .protect_composition_on_press()
        .cursor(CursorStyle::Pointer)
    }

    fn app_bar_heading<M>(
        &self,
        options: AppBarOptions<'_>,
        centered: bool,
        lower: bool,
        collapse: f32,
    ) -> View<M> {
        let (title_font, subtitle_font) = match options.kind {
            AppBarKind::MediumFlexible if lower => {
                (typography::HEADLINE_MEDIUM, typography::LABEL_LARGE)
            }
            AppBarKind::LargeFlexible if lower => {
                (typography::DISPLAY_SMALL, typography::TITLE_MEDIUM)
            }
            _ => (typography::TITLE_LARGE, typography::LABEL_MEDIUM),
        };
        let label_style = |font, target, color| {
            let mut style = Style::default()
                .with_text_style(interpolate_font(font, target, collapse))
                .foreground(color);
            style.max_width = Length::Percent(1.);
            style
        };
        let title = View::new(
            Node::new(Role::Heading, options.title)
                .id(format!("{}-title", options.id))
                .text(options.title)
                .styled(label_style(
                    title_font,
                    typography::TITLE_LARGE,
                    self.colors.on_surface,
                )),
        );
        let subtitle = options.subtitle.map(|subtitle| {
            View::new(
                Node::new(Role::Text, subtitle)
                    .id(format!("{}-subtitle", options.id))
                    .text(subtitle)
                    .styled(label_style(
                        subtitle_font,
                        typography::LABEL_MEDIUM,
                        self.colors.on_surface_variant,
                    )),
            )
        });
        let align = if centered {
            Align::Center
        } else if options.direction == LayoutDirection::RightToLeft {
            Align::End
        } else {
            Align::Start
        };
        let mut style = Style::column().align(align);
        style.max_width = Length::Percent(1.);
        View::new(Node::new(Role::Group, "").styled(style))
            .children(std::iter::once(title).chain(subtitle))
    }
}

fn outer_lane(centered_width: Option<f32>, has_controls: bool) -> Style {
    let mut style = Style::row().wrap(true).align(Align::Center);
    if let Some(width) = centered_width {
        style.width = Length::Px(width);
        style.max_width = Length::Percent(0.375);
    } else if has_controls {
        style.max_width = Length::Percent(0.5);
    } else {
        style.width = Length::Px(EDGE_SPACE * 2.);
    }
    style
}

fn logical_row(node: Node, direction: LayoutDirection) -> Node {
    match direction {
        LayoutDirection::LeftToRight => node,
        LayoutDirection::RightToLeft => node.reverse_row(),
    }
}

fn interpolate_font(mut style: TextStyle, target: TextStyle, progress: f32) -> TextStyle {
    style.font_size += (target.font_size - style.font_size) * progress;
    if let (Some(from), Some(to)) = (style.line_height, target.line_height) {
        style.line_height = Some(from + (to - from) * progress);
    }
    style
}

fn search_icon() -> &'static VectorIcon {
    static ICON: std::sync::OnceLock<VectorIcon> = std::sync::OnceLock::new();
    ICON.get_or_init(|| {
        let outer = [
            [3., 7.],
            [7., 3.],
            [12., 3.],
            [16., 7.],
            [16., 12.],
            [21., 17.],
            [19., 19.],
            [14., 14.],
            [12., 16.],
            [7., 16.],
            [3., 12.],
        ];
        let hole = [
            [5., 8.],
            [8., 5.],
            [11., 5.],
            [14., 8.],
            [14., 11.],
            [11., 14.],
            [8., 14.],
            [5., 11.],
        ];
        VectorIcon::from_contours(
            [outer.as_slice(), hole.as_slice()].map(|path| path.iter().copied()),
        )
        .expect("valid Search icon")
    })
}
