//! Material 3 card surfaces and structured content.

use surface::{Align, Color, FillContour, Insets, Length, Node, Role, Style, TextStyle};
use surface_app::{CursorStyle, View};

use crate::{
    motion::STATE_CHANGE, shape, shape::LayoutDirection, tokens::blend, typography, CardKind,
    Elevation, Theme,
};

const CONTENT_PADDING: f32 = 16.;
const CONTENT_GAP: f32 = 8.;
const MIN_TOUCH_SIZE: f32 = 48.;
const SELECTION_SIZE: f32 = 24.;
const DISABLED_INK: f32 = 0.38;

/// An application-controlled card state.
///
/// An actionable card with `Rest` also follows the runtime's hover, press
/// and focus state. Passive cards use their children's input feedback and
/// can display `Dragged` while the caller handles a drag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardInteraction {
    #[default]
    Rest,
    Hovered,
    Focused,
    Pressed,
    Dragged,
}

/// Shared appearance and accessibility options for a Material card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialCardOptions<'a> {
    pub id: &'a str,
    /// Localized accessible name. For a whole-card action, include the
    /// information users need before taking that action.
    pub name: &'a str,
    pub kind: CardKind,
    pub direction: LayoutDirection,
    /// For a passive card, keep the structured text readable and disable the
    /// supplied media, body and action subtrees.
    pub disabled: bool,
    pub interaction: CardInteraction,
    /// Enabled stable target when focus leaves a disabled or removed card.
    pub restore_focus_to: Option<&'a str>,
}

/// Optional content blocks. The card sizes to its contents.
pub struct CardContent<'a, M> {
    pub title: Option<&'a str>,
    pub subtitle: Option<&'a str>,
    pub supporting: Option<&'a str>,
    /// Caller-authored illustration or media, with its own layout size.
    pub media: Option<View<M>>,
    /// Additional caller-authored content. Passive cards can include
    /// interactive views here, such as inline links or selection controls.
    pub body: Option<View<M>>,
}

/// The one action of a whole-card button.
pub enum CardAction<M> {
    Press(M),
    Toggle { checked: bool, on_toggle: M },
}

impl Theme {
    /// Build a card with individually focusable actions. The named card
    /// itself stays out of the Tab order. Extra actions wrap when needed.
    ///
    /// Attach a captured drag to a dedicated action or handle if the card
    /// can move. Give that operation an equivalent button or menu action.
    ///
    /// [Material 3 cards](https://m3.material.io/components/cards/guidelines)
    /// [Pinned MDC card tokens](https://github.com/material-components/material-components-android/blob/66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad/lib/java/com/google/android/material/card/res/values/tokens.xml)
    pub fn material_card<M>(
        &self,
        options: MaterialCardOptions<'_>,
        content: CardContent<'_, M>,
        actions: impl IntoIterator<Item = View<M>>,
    ) -> View<M> {
        self.material_card_view(options, content, actions.into_iter().collect(), None)
    }

    /// Build one actionable surface. The authored content remains visible
    /// and the card's localized name describes it. The card publishes one
    /// button or toggle to the keyboard and platform accessibility.
    pub fn material_action_card<M>(
        &self,
        options: MaterialCardOptions<'_>,
        content: CardContent<'_, M>,
        action: CardAction<M>,
    ) -> View<M> {
        self.material_card_view(options, content, Vec::new(), Some(action))
    }

    fn material_card_view<M>(
        &self,
        options: MaterialCardOptions<'_>,
        content: CardContent<'_, M>,
        actions: Vec<View<M>>,
        action: Option<CardAction<M>>,
    ) -> View<M> {
        let id = options.id;
        let actionable = action.is_some();
        let selected = match &action {
            Some(CardAction::Toggle { checked, .. }) => Some(*checked),
            _ => None,
        };
        let state = if !options.disabled
            && (actionable || options.interaction == CardInteraction::Dragged)
        {
            options.interaction
        } else {
            CardInteraction::Rest
        };
        let palette = CardPalette::new(self, options);
        let mut style = Style::column()
            .width(Length::Percent(1.))
            .padding(CONTENT_PADDING)
            .gap(CONTENT_GAP)
            .corners(shape::MEDIUM)
            .background(palette.container);
        style.min_height = if actionable { MIN_TOUCH_SIZE } else { 0. };
        let role = if actionable {
            Role::Button
        } else {
            Role::Group
        };
        let mut node = Node::new(role, options.name)
            .id(id)
            .styled(style)
            .disabled(options.disabled)
            .focus_color(Color::TRANSPARENT)
            .box_shadows(self.elevation(card_elevation(options.kind, state)));
        if actionable && !options.disabled && state == CardInteraction::Rest {
            node = node.interaction_box_shadows(
                Some(self.elevation(card_elevation(options.kind, CardInteraction::Hovered))),
                None,
                None,
            );
        }
        if let Some(value) = selected {
            node = node.checked(value);
        }
        let CardContent {
            title,
            subtitle,
            supporting,
            media,
            body,
        } = content;
        let hide_decorative_semantics = |view: View<M>| {
            if actionable || options.disabled {
                view.without_interaction(options.restore_focus_to)
            } else {
                view
            }
        };
        let media = media.map(|media| {
            let clip = View::new(Node::new(Role::Group, "").styled(Style::column().clip(true)))
                .children([media]);
            hide_decorative_semantics(
                View::new(Node::new(Role::Group, "").id(format!("{id}-media"))).children([clip]),
            )
        });
        let heading = card_heading(options, palette, title, subtitle, selected).map(|view| {
            if actionable {
                view.without_interaction(None)
            } else {
                view
            }
        });
        let supporting = supporting.map(|text| {
            let view = card_text(
                id,
                "supporting",
                Role::Text,
                text,
                typography::BODY_MEDIUM,
                palette.secondary,
            );
            let view = View::new(
                Node::new(Role::Group, "")
                    .styled(Style::column().align(content_alignment(options.direction))),
            )
            .children([view]);
            if actionable {
                view.without_interaction(None)
            } else {
                view
            }
        });
        let body = body.map(|body| {
            hide_decorative_semantics(
                View::new(Node::new(Role::Group, "").id(format!("{id}-body"))).children([body]),
            )
        });
        let actions = (!actions.is_empty()).then(|| {
            let view = View::new(directed_row(
                Node::new(Role::Group, "")
                    .id(format!("{id}-actions"))
                    .styled(
                        Style::row()
                            .wrap(true)
                            .gap(CONTENT_GAP)
                            .align(Align::Center),
                    ),
                options.direction,
            ))
            .children(actions);
            if options.disabled {
                view.without_interaction(options.restore_focus_to)
            } else {
                view
            }
        });
        let state_layer = card_state_layer(options, palette, state, actionable);
        let card = View::new(node).children(
            media
                .into_iter()
                .chain(heading)
                .chain(supporting)
                .chain(body)
                .chain(actions)
                .chain([state_layer]),
        );
        let card = if let Some(target) = options.restore_focus_to {
            card.restore_focus_in_subtree_on_removal(target)
        } else {
            card
        };
        match action {
            Some(
                CardAction::Press(message)
                | CardAction::Toggle {
                    on_toggle: message, ..
                },
            ) if !options.disabled => card
                .on_press(message)
                .accessible_activation()
                .protect_composition_on_press()
                .cursor(CursorStyle::Pointer),
            _ => card,
        }
    }
}

#[derive(Clone, Copy)]
struct CardPalette {
    container: Color,
    primary: Color,
    secondary: Color,
    outline: Color,
    focus: Color,
    layer: Color,
    checked: Color,
    check_ink: Color,
}

impl CardPalette {
    fn new(theme: &Theme, options: MaterialCardOptions<'_>) -> Self {
        let colors = theme.colors;
        let container = match (options.kind, options.disabled) {
            (CardKind::Filled, true) => blend(
                colors.surface_container_highest,
                colors.surface_variant,
                0.38,
            ),
            (CardKind::Elevated, true) | (CardKind::Outlined, _) => colors.surface,
            (CardKind::Elevated, false) => colors.surface_container_low,
            (CardKind::Filled, false) => colors.surface_container_highest,
        };
        let outline = match (options.kind, options.disabled) {
            (CardKind::Outlined, true) => opacity(colors.outline, 0.12),
            (CardKind::Outlined, false) => colors.outline_variant,
            _ => Color::TRANSPARENT,
        };
        let focus = if options.kind == CardKind::Outlined {
            colors.on_surface
        } else {
            colors.secondary
        };
        let ink = |color| {
            if options.disabled {
                opacity(color, DISABLED_INK)
            } else {
                color
            }
        };
        Self {
            container,
            primary: ink(colors.on_surface),
            secondary: ink(colors.on_surface_variant),
            checked: ink(if options.disabled {
                colors.on_surface
            } else {
                colors.primary
            }),
            check_ink: if options.disabled {
                container
            } else {
                colors.on_primary
            },
            outline,
            focus,
            layer: colors.on_surface_variant,
        }
    }
}

fn card_text<M>(
    id: &str,
    suffix: &str,
    role: Role,
    text: &str,
    font: TextStyle,
    color: Color,
) -> View<M> {
    let mut style = Style::default().with_text_style(font).foreground(color);
    style.max_width = Length::Percent(1.);
    View::new(
        Node::new(role, text)
            .id(format!("{id}-{suffix}"))
            .text(text)
            .styled(style),
    )
}

fn card_heading<M>(
    options: MaterialCardOptions<'_>,
    palette: CardPalette,
    title: Option<&str>,
    subtitle: Option<&str>,
    selected: Option<bool>,
) -> Option<View<M>> {
    if title.is_none() && subtitle.is_none() && selected.is_none() {
        return None;
    }
    let id = options.id;
    let title = title.map(|text| {
        card_text(
            id,
            "title",
            Role::Heading,
            text,
            typography::TITLE_LARGE,
            palette.primary,
        )
    });
    let subtitle = subtitle.map(|text| {
        card_text(
            id,
            "subtitle",
            Role::Text,
            text,
            typography::BODY_MEDIUM,
            palette.secondary,
        )
    });
    let text = View::new(
        Node::new(Role::Group, "").styled(
            Style::column()
                .width(Length::Px(0.))
                .grow(1.)
                .gap(4.)
                .align(content_alignment(options.direction)),
        ),
    )
    .children(title.into_iter().chain(subtitle));
    let marker = selected.map(|value| selection_marker(id, palette, value));
    Some(
        View::new(directed_row(
            Node::new(Role::Group, "").styled(Style::row().gap(CONTENT_GAP).align(Align::Start)),
            options.direction,
        ))
        .children(std::iter::once(text).chain(marker)),
    )
}

fn selection_marker<M>(id: &str, palette: CardPalette, checked: bool) -> View<M> {
    let size = Length::Px(SELECTION_SIZE);
    let mut node = Node::new(Role::Group, "")
        .id(format!("{id}-selection"))
        .styled(
            Style::column()
                .width(size)
                .height(size)
                .corners(shape::FULL)
                .background(if checked {
                    palette.checked
                } else {
                    Color::TRANSPARENT
                }),
        );
    if checked {
        let points = [
            [5., 11.],
            [7., 9.],
            [10., 12.],
            [17., 5.],
            [19., 7.],
            [10., 16.],
        ];
        node = node
            .fill_contour(FillContour::new(points, palette.check_ink).expect("valid check mark"));
    }
    View::new(node)
}

fn card_state_layer<M>(
    options: MaterialCardOptions<'_>,
    palette: CardPalette,
    state: CardInteraction,
    actionable: bool,
) -> View<M> {
    let alpha = match state {
        CardInteraction::Rest => 0.,
        CardInteraction::Hovered => 0.08,
        CardInteraction::Focused | CardInteraction::Pressed => 0.12,
        CardInteraction::Dragged => 0.16,
    };
    let mut style = Style::column()
        .absolute(Insets::all(0.))
        .corners(shape::MEDIUM)
        .background(opacity(palette.layer, alpha))
        .transition(STATE_CHANGE);
    if state == CardInteraction::Focused {
        style = style.border(2., palette.focus);
    } else if palette.outline.3 > 0 {
        style = style.border(1., palette.outline);
    }
    let mut node = Node::new(Role::Group, "").id(format!("{}-state", options.id));
    if actionable && !options.disabled {
        if state == CardInteraction::Rest {
            style = style.state_layer(palette.layer);
        }
        node = node.focus_color(palette.focus).focus_outline_from_parent();
    }
    // The final layer covers authored media and carries its owner's feedback.
    View::new(node.styled(style)).without_interaction(None)
}

fn directed_row(node: Node, direction: LayoutDirection) -> Node {
    if direction == LayoutDirection::RightToLeft {
        node.reverse_row()
    } else {
        node
    }
}

fn content_alignment(direction: LayoutDirection) -> Align {
    match direction {
        LayoutDirection::LeftToRight => Align::Start,
        LayoutDirection::RightToLeft => Align::End,
    }
}

fn opacity(Color(r, g, b, a): Color, alpha: f32) -> Color {
    if alpha == 0. {
        Color::TRANSPARENT
    } else {
        Color(r, g, b, (f32::from(a) * alpha).round() as u8)
    }
}

fn card_elevation(kind: CardKind, state: CardInteraction) -> Elevation {
    match (kind, state) {
        (CardKind::Elevated, CardInteraction::Dragged) => Elevation::Level4,
        (_, CardInteraction::Dragged) => Elevation::Level3,
        (CardKind::Elevated, CardInteraction::Hovered) => Elevation::Level2,
        (CardKind::Elevated, _) | (_, CardInteraction::Hovered) => Elevation::Level1,
        _ => Elevation::Level0,
    }
}
