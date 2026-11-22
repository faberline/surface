//! Material 3 floating and anchored FAB menus.

use surface::{Align, Color, Corners, Insets, Length, Node, Rect, Role, Style};
use surface_app::{CursorStyle, Key, View};

use crate::{
    fab::{fab_action, fab_colors, fab_face, FabPose},
    fab_menu_layout::{
        fab_menu_edge_margin, place_fab_menu_floating, place_fab_menu_trigger, place_fab_menu_web,
        FAB_MENU_LIST_INSET,
    },
    shape::LayoutDirection,
    split_button::anchored_menu,
    tokens::blend,
    typography, Elevation, ExpressiveFabAppearance, ExpressiveFabElevation, ExpressiveFabOptions,
    ExpressiveFabSize, FabMenuMotion, SplitButtonMenuItem, Theme, VectorIcon,
};

const CLOSE_SIDE: f32 = 56.;
const CLOSE_ICON: f32 = 20.;
const PILL_SIDE: f32 = 56.;
const PILL_INSET: f32 = 24.;
const ITEM_ICON: f32 = 24.;
const ICON_GAP: f32 = 8.;
const ITEM_GAP: f32 = 4.;
const MIN_TARGET: f32 = 48.;
const FOCUS_WIDTH: f32 = 3.;
const WEB_WIDTH: f32 = 240.;
const WEB_ROW_HEIGHT: f32 = 48.;
const WEB_PADDING: f32 = 8.;

/// Select the floating button stack or an anchored Web menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FabMenuPresentation {
    #[default]
    NativeFloating,
    WebAnchored,
}

/// The three Material 3 FAB Menu color sets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FabMenuAppearance {
    #[default]
    Primary,
    Secondary,
    Tertiary,
}

impl FabMenuAppearance {
    fn tonal(self) -> ExpressiveFabAppearance {
        match self {
            Self::Primary => ExpressiveFabAppearance::TonalPrimary,
            Self::Secondary => ExpressiveFabAppearance::TonalSecondary,
            Self::Tertiary => ExpressiveFabAppearance::TonalTertiary,
        }
    }

    fn solid(self) -> ExpressiveFabAppearance {
        match self {
            Self::Primary => ExpressiveFabAppearance::Primary,
            Self::Secondary => ExpressiveFabAppearance::Secondary,
            Self::Tertiary => ExpressiveFabAppearance::Tertiary,
        }
    }
}

/// Presentation, labels and application-controlled availability.
#[derive(Clone, Copy, Debug)]
pub struct FabMenuOptions<'a> {
    /// Stable ID of the square anchor. The action uses `{id}-trigger`.
    pub id: &'a str,
    pub open_label: &'a str,
    pub close_label: &'a str,
    pub menu_label: &'a str,
    pub presentation: FabMenuPresentation,
    pub appearance: FabMenuAppearance,
    pub size: ExpressiveFabSize,
    pub direction: LayoutDirection,
    /// Visible window bounds in logical units. This can exclude safe areas.
    pub viewport: Rect,
    pub open: bool,
    pub disabled: bool,
    pub restore_focus_to: Option<&'a str>,
}

/// A named action. The caller chooses whether activating it closes the menu.
pub struct FabMenuItem<'a, M> {
    pub id: &'a str,
    pub label: &'a str,
    pub icon: Option<VectorIcon>,
    pub disabled: bool,
    pub on_press: M,
}

pub struct FabMenuContent<'a, M> {
    pub icon: VectorIcon,
    /// Material recommends two to six related actions.
    pub items: Vec<FabMenuItem<'a, M>>,
    pub on_toggle: M,
    pub on_dismiss: M,
}

impl Theme {
    /// Build a FAB with a stable square footprint and a controlled menu.
    ///
    /// Place the returned anchor in the parent. For the floating presentation,
    /// use a bottom trailing position. Keep one [FabMenuMotion] per mounted
    /// anchor and advance it in the accepted input frame hook. An initially
    /// open menu waits for the anchor's first measured position.
    ///
    /// [Material 3 FAB Menu](https://m3.material.io/components/fab-menu/specs)
    pub fn fab_menu<M: Clone + 'static>(
        &self,
        options: FabMenuOptions<'_>,
        content: FabMenuContent<'_, M>,
        motion: &FabMenuMotion,
    ) -> View<M> {
        let geometry = FabPose::from(options.size);
        let context = MenuContext {
            theme: self,
            options,
            motion,
            anchor: motion
                .anchor()
                .filter(|anchor| visible(*anchor, options.viewport)),
            geometry,
        };
        let children = match options.presentation {
            FabMenuPresentation::NativeFloating => context.floating(content),
            FabMenuPresentation::WebAnchored => context.web(content),
        };
        View::new(
            Node::new(Role::Group, "")
                .id(options.id)
                .styled(sized(geometry.side, geometry.side)),
        )
        .children(children)
    }
}

struct MenuContext<'a> {
    theme: &'a Theme,
    options: FabMenuOptions<'a>,
    motion: &'a FabMenuMotion,
    anchor: Option<Rect>,
    geometry: FabPose,
}

impl MenuContext<'_> {
    fn requested<M>(&self, content: &FabMenuContent<'_, M>) -> bool {
        self.options.open && !self.options.disabled && !content.items.is_empty()
    }

    fn trigger_options<'a>(&'a self, id: &'a str, label: &'a str) -> ExpressiveFabOptions<'a> {
        ExpressiveFabOptions {
            id,
            name: label,
            size: self.options.size,
            appearance: self.options.appearance.tonal(),
            elevation: ExpressiveFabElevation::Standard,
            visible: true,
            disabled: self.options.disabled,
            restore_focus_to: self.options.restore_focus_to,
        }
    }

    fn scope<M>(&self, bounds: Rect, children: impl IntoIterator<Item = View<M>>) -> View<M> {
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-scope", self.options.id))
                .styled(fixed(bounds)),
        )
        .children(children)
    }

    fn web<M: Clone + 'static>(&self, content: FabMenuContent<'_, M>) -> Vec<View<M>> {
        let options = self.options;
        let wanted = content.items.len() as f32 * WEB_ROW_HEIGHT + 2. * WEB_PADDING;
        let height = wanted.max(self.motion.menu_height().unwrap_or(0.));
        let fit = self
            .anchor
            .filter(|_| self.requested(&content))
            .and_then(|anchor| {
                place_fab_menu_web(
                    anchor,
                    options.viewport,
                    (WEB_WIDTH, height),
                    options.direction,
                )
                .map(|popup| (anchor, popup))
            });
        let id = format!("{}-trigger", options.id);
        let open = fit.is_some();
        let key = content.on_toggle.clone();
        let mut trigger = self
            .theme
            .expressive_fab(
                self.trigger_options(&id, options.open_label),
                crate::ExpressiveFabContent {
                    icon: content.icon,
                    on_press: content.on_toggle,
                },
                self.motion.trigger_motion(),
            )
            .menu_expanded(open);
        if !open && !options.disabled && !content.items.is_empty() {
            trigger = trigger.on_key(move |press| {
                (press.key == Key::ArrowDown && press.modifiers.is_empty() && !press.repeat)
                    .then(|| key.clone())
            });
        }
        let Some((anchor, popup)) = fit else {
            return vec![trigger];
        };

        let menu = anchored_menu(
            self.theme,
            options.id,
            options.menu_label,
            popup,
            options.direction,
            content.items.into_iter().map(|item| SplitButtonMenuItem {
                id: item.id,
                label: item.label,
                icon: item.icon,
                disabled: item.disabled,
                on_press: item.on_press,
            }),
        );
        // The menu comes first in authored order so scope entry selects its
        // first enabled item. The trigger stays a sibling of the menu.
        let scope = self
            .scope(anchor, [menu, trigger])
            .modal_overlay(content.on_dismiss)
            .dismiss_modal_on_tab();
        vec![self.focus_fallback(scope)]
    }

    fn floating<M: Clone + 'static>(&self, content: FabMenuContent<'_, M>) -> Vec<View<M>> {
        let options = self.options;
        let max_width = self.anchor.and_then(|anchor| {
            place_fab_menu_floating(
                anchor,
                options.viewport,
                (options.viewport.width, PILL_SIDE),
                options.direction,
            )
            .map(|list| list.width - 2. * FAB_MENU_LIST_INSET)
        });
        let safe_width = if options.viewport.width.is_finite() {
            (options.viewport.width - 2. * fab_menu_edge_margin(options.viewport.width))
                .max(MIN_TARGET)
        } else {
            WEB_WIDTH
        };
        let max_width = max_width.unwrap_or(safe_width).max(1.);
        let rows: Vec<_> = content
            .items
            .iter()
            .map(|item| NativePillLayout::new(max_width, item, self.motion.item_size(item.id)))
            .collect();
        let stack_width = rows.iter().map(|row| row.width).fold(0., f32::max);
        let stack_height = rows.iter().map(|row| row.height).sum::<f32>()
            + ITEM_GAP * rows.len().saturating_sub(1) as f32;
        let list = self.anchor.and_then(|anchor| {
            place_fab_menu_floating(
                anchor,
                options.viewport,
                (stack_width, stack_height),
                options.direction,
            )
        });
        let open = self.requested(&content) && list.is_some();
        let progress = if options.disabled {
            0.
        } else {
            self.motion.progress().clamp(0., 1.)
        };
        let animate_trigger = open || progress > 0.;
        let geometry = if animate_trigger {
            FabPose {
                side: lerp(self.geometry.side, CLOSE_SIDE, progress).round(),
                radius: lerp(self.geometry.radius, CLOSE_SIDE / 2., progress),
                icon: lerp(self.geometry.icon, CLOSE_ICON, progress),
            }
        } else {
            self.geometry
        };
        let bounds = self
            .anchor
            .and_then(|anchor| place_fab_menu_trigger(anchor, geometry.side, options.direction));
        let id = format!("{}-trigger", options.id);
        let label = if open {
            options.close_label
        } else {
            options.open_label
        };
        let fab = self.trigger_options(&id, label);
        let closed = fab_colors(self.theme, fab);
        let close = fab_colors(
            self.theme,
            ExpressiveFabOptions {
                appearance: options.appearance.solid(),
                ..fab
            },
        );
        let colors = if options.disabled {
            closed
        } else {
            (
                blend(closed.0, close.0, progress),
                blend(closed.1, close.1, progress),
            )
        };
        let icon = if progress > 0.5 {
            close_icon()
        } else {
            content.icon
        };
        let motion = self.motion.trigger_motion();
        let face = fab_face(self.theme, fab, geometry, &icon, motion, Some(colors));
        let trigger =
            fab_action(fab, geometry.side, face, content.on_toggle, true).disclosure_expanded(open);
        let probes = measure_labels(&content.items, &rows);
        let list = list
            .filter(|_| !options.disabled && (open || self.motion.items_present()))
            .map(|bounds| {
                let initial_offset =
                    (stack_height + 2. * FAB_MENU_LIST_INSET - bounds.height).max(0.);
                self.floating_list(bounds, initial_offset, content.items, rows, open, closed)
            });
        let root = match bounds.filter(|_| animate_trigger || list.is_some()) {
            Some(bounds) if open => {
                let scope = self
                    .scope(bounds, std::iter::once(trigger).chain(list))
                    .modal_overlay(content.on_dismiss);
                self.focus_fallback(scope)
            }
            Some(bounds) => self.scope(
                bounds,
                std::iter::once(trigger).chain(list.map(View::visual_overlay)),
            ),
            None => trigger,
        };
        vec![root, probes]
    }

    fn floating_list<M>(
        &self,
        bounds: Rect,
        initial_offset: f32,
        items: Vec<FabMenuItem<'_, M>>,
        rows: Vec<NativePillLayout>,
        open: bool,
        colors: (Color, Color),
    ) -> View<M> {
        let align = match self.options.direction {
            LayoutDirection::LeftToRight => Align::End,
            LayoutDirection::RightToLeft => Align::Start,
        };
        let mut style = fixed(bounds)
            .align(align)
            .gap(ITEM_GAP)
            .padding(FAB_MENU_LIST_INSET)
            .clip(true);
        // Keep the first animated pill next to the close button visible.
        // The runtime takes over when the user scrolls or focuses a pill.
        style.scroll_y = initial_offset;
        style.z_index = 1000;
        View::new(
            Node::new(Role::Group, self.options.menu_label)
                .id(format!("{}-list", self.options.id))
                .styled(style),
        )
        .children(
            items
                .into_iter()
                .zip(rows)
                .enumerate()
                .map(|(index, (item, layout))| {
                    self.floating_item(index, item, layout, open, colors)
                }),
        )
        .accessibility_scroll_pages()
    }

    fn floating_item<M>(
        &self,
        index: usize,
        item: FabMenuItem<'_, M>,
        layout: NativePillLayout,
        open: bool,
        colors: (Color, Color),
    ) -> View<M> {
        let input = self.motion.interaction(item.id);
        let animation = self.motion.item(index);
        let emerging = if open && input.focused {
            1.
        } else {
            animation.width
        };
        let opacity = if open && input.focused {
            1.
        } else {
            animation.opacity
        };
        let width = (layout.width * emerging.clamp(0., 1.)).round();
        let height = layout.height;
        let radius = width.min(height) / 2.;
        let in_scope = open && opacity > 0. && width >= MIN_TARGET.min(layout.width);
        let active = in_scope && !item.disabled;
        let (fill, ink) = if item.disabled {
            let on_surface = self.theme.colors.on_surface;
            (tint(on_surface, 0.12), tint(on_surface, 0.38))
        } else {
            colors
        };
        let tint_layer = if !active {
            0.
        } else if input.pressed || input.focused {
            0.10
        } else if input.hovered {
            0.08
        } else {
            0.
        };
        let level = if !active {
            Elevation::Level0
        } else if input.pressed {
            Elevation::Level3
        } else if input.hovered {
            Elevation::Level4
        } else if input.focused {
            Elevation::Level3
        } else {
            Elevation::Level0
        };
        let shadows = self.theme.elevation(level).map(|mut layer| {
            layer.color = tint(layer.color, opacity);
            layer
        });
        let corners = Corners::all(radius);
        let fill = if tint_layer > 0. {
            blend(fill, ink, tint_layer)
        } else {
            fill
        };
        let paint = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-face", item.id))
                .styled(
                    sized(width, height)
                        .corners(corners)
                        .background(tint(fill, opacity)),
                )
                .box_shadows(shadows),
        )
        .children(pill_paint(
            &item,
            layout,
            width,
            tint(ink, opacity),
            self.options.direction,
        ))
        .without_interaction(None);
        let outline = (active && input.focused).then(|| {
            let inset = FAB_MENU_LIST_INSET;
            View::new(
                Node::new(Role::Group, "")
                    .id(format!("{}-focus-ring", item.id))
                    .styled(
                        placed(Rect::new(
                            -inset,
                            -inset,
                            width + 2. * inset,
                            height + 2. * inset,
                        ))
                        .corners(Corners::all(radius + inset))
                        .border(FOCUS_WIDTH, self.theme.colors.secondary),
                    ),
            )
            .without_interaction(None)
        });
        let mut style = sized(width, height);
        style.min_height = height;
        let button = View::new(
            Node::new(Role::Button, item.label)
                .id(item.id)
                .styled(style)
                .disabled(item.disabled)
                .focus_color(Color::TRANSPARENT),
        )
        .children(std::iter::once(paint).chain(outline));
        if !in_scope {
            button.without_interaction(None)
        } else if item.disabled {
            button
        } else {
            button
                .on_press(item.on_press)
                .accessible_activation()
                .accessibility_scroll_into_view()
                .cursor(CursorStyle::Pointer)
        }
    }

    fn focus_fallback<M>(&self, scope: View<M>) -> View<M> {
        if let Some(target) = self.options.restore_focus_to {
            scope.modal_focus_fallback(target)
        } else {
            scope
        }
    }
}

#[derive(Clone, Copy)]
struct NativePillLayout {
    width: f32,
    height: f32,
    inset: f32,
    label_width: f32,
    label_height: f32,
    measurement_width: f32,
}

impl NativePillLayout {
    fn new<M>(limit: f32, item: &FabMenuItem<'_, M>, text: Option<(f32, f32)>) -> Self {
        let accessory = if item.icon.is_some() {
            ITEM_ICON + ICON_GAP
        } else {
            0.
        };
        let inset = ((limit - accessory - 24.) / 2.).clamp(1., PILL_INSET);
        let measurement_width = (limit - accessory - 2. * inset).max(1.).floor();
        let (natural, label_height) = text.unwrap_or_else(|| {
            (
                item.label.chars().count() as f32 * typography::TITLE_MEDIUM.font_size,
                typography::TITLE_MEDIUM.line_height.unwrap_or(24.),
            )
        });
        let label_width = natural.ceil().clamp(1., measurement_width);
        let width = (label_width + accessory + 2. * inset)
            .max(PILL_SIDE)
            .min(limit)
            .floor();
        let content = if item.icon.is_some() {
            label_height.max(ITEM_ICON)
        } else {
            label_height
        };
        let height = pill_height(width, inset, content);
        Self {
            width,
            height,
            inset,
            label_width,
            label_height,
            measurement_width,
        }
    }
}

fn pill_height(width: f32, inset: f32, content: f32) -> f32 {
    let minimum = PILL_SIDE.max(content + 16.);
    let edge = (inset - 1.).max(1.);
    let body = content / 2. + 1.;
    // In a horizontal capsule, requiring both outer text corners to be
    // inside the end circles gives h * edge >= edge² + body².
    let capsule = minimum.max(edge + body * body / edge);
    if capsule <= width {
        return capsule.ceil();
    }
    minimum
        .max(content + 2. * corner_clearance(width / 2., inset))
        .ceil()
}

fn corner_clearance(radius: f32, margin: f32) -> f32 {
    let dx = (radius - (margin - 1.).max(0.)).clamp(0., radius);
    (radius - (radius * radius - dx * dx).max(0.).sqrt()).ceil() + 1.
}

fn pill_paint<M>(
    item: &FabMenuItem<'_, M>,
    layout: NativePillLayout,
    width: f32,
    color: Color,
    direction: LayoutDirection,
) -> Vec<View<M>> {
    let pos = |start: f32, y: f32, child_width: f32, height: f32| {
        let x = match direction {
            LayoutDirection::LeftToRight => width - layout.width + start,
            LayoutDirection::RightToLeft => layout.width - start - child_width,
        };
        Rect::new(x, y, child_width, height)
    };
    let clip = |rect: Rect, draw: &dyn Fn(Rect) -> View<M>| {
        let visible = rect.intersect(Rect::new(0., 0., width, layout.height));
        if visible.width <= 0. || visible.height <= 0. {
            return None;
        }
        let margin = visible.x.min(width - visible.x - visible.width);
        let clear = corner_clearance(width.min(layout.height) / 2., margin);
        let visible = visible.intersect(Rect::new(
            0.,
            clear,
            width,
            (layout.height - 2. * clear).max(0.),
        ));
        if visible.width <= 0. || visible.height <= 0. {
            return None;
        }
        let content = draw(Rect::new(
            rect.x - visible.x,
            rect.y - visible.y,
            rect.width,
            rect.height,
        ));
        Some(
            View::new(Node::new(Role::Group, "").styled(placed(visible).clip(true)))
                .children([content]),
        )
    };
    let icon = item.icon.as_ref().and_then(|icon| {
        let contour = icon.contour_dp(ITEM_ICON, color)?;
        let rect = pos(
            layout.inset,
            (layout.height - ITEM_ICON) / 2.,
            ITEM_ICON,
            ITEM_ICON,
        );
        clip(rect, &|local| {
            View::new(
                Node::new(Role::Group, "")
                    .id(format!("{}-icon", item.id))
                    .styled(placed(local))
                    .fill_contour(contour.clone()),
            )
        })
    });
    let label_start = layout.inset
        + if item.icon.is_some() {
            ITEM_ICON + ICON_GAP
        } else {
            0.
        };
    let label = pos(
        label_start,
        (layout.height - layout.label_height) / 2.,
        layout.label_width,
        layout.label_height,
    );
    let label = clip(label, &|local| {
        View::new(
            Node::new(Role::Text, item.label)
                .id(format!("{}-label", item.id))
                .text(item.label)
                .styled(
                    placed(local)
                        .with_text_style(typography::TITLE_MEDIUM)
                        .foreground(color),
                ),
        )
    });
    icon.into_iter().chain(label).collect()
}

fn measure_labels<M>(items: &[FabMenuItem<'_, M>], rows: &[NativePillLayout]) -> View<M> {
    let style = sized(16384., 1.).align(Align::Start).fixed(Insets {
        left: Length::Px(-32768.),
        top: Length::Px(0.),
        ..Insets::default()
    });
    View::new(Node::new(Role::Group, "").styled(style))
        .children(items.iter().zip(rows).flat_map(|(item, row)| {
            [
                ("measure", Length::Auto),
                ("wrapped", Length::Px(row.measurement_width)),
            ]
            .map(|(suffix, width)| {
                View::new(
                    Node::new(Role::Text, item.label)
                        .id(format!("{}-{suffix}", item.id))
                        .text(item.label)
                        .styled(
                            Style::column()
                                .width(width)
                                .with_text_style(typography::TITLE_MEDIUM),
                        ),
                )
            })
        }))
        .without_interaction(None)
}

fn close_icon() -> VectorIcon {
    VectorIcon::new([
        [6., 4.5],
        [12., 10.5],
        [18., 4.5],
        [19.5, 6.],
        [13.5, 12.],
        [19.5, 18.],
        [18., 19.5],
        [12., 13.5],
        [6., 19.5],
        [4.5, 18.],
        [10.5, 12.],
        [4.5, 6.],
    ])
    .expect("the close icon stays inside its vector viewport")
}

fn visible(anchor: Rect, viewport: Rect) -> bool {
    let values = [
        anchor.x,
        anchor.y,
        anchor.width,
        anchor.height,
        viewport.x,
        viewport.y,
        viewport.width,
        viewport.height,
    ];
    if !values.into_iter().all(f32::is_finite) {
        return false;
    }
    let visible = anchor.intersect(viewport);
    visible.width > 0. && visible.height > 0.
}

fn lerp(start: f32, end: f32, progress: f32) -> f32 {
    start + (end - start) * progress
}

fn tint(Color(r, g, b, a): Color, opacity: f32) -> Color {
    Color(r, g, b, (f32::from(a) * opacity).round() as u8)
}

fn sized(width: f32, height: f32) -> Style {
    Style::column()
        .width(Length::Px(width))
        .height(Length::Px(height))
}

fn position(bounds: Rect) -> Insets {
    Insets {
        left: Length::Px(bounds.x),
        top: Length::Px(bounds.y),
        ..Insets::default()
    }
}

fn placed(bounds: Rect) -> Style {
    sized(bounds.width, bounds.height).absolute(position(bounds))
}

fn fixed(bounds: Rect) -> Style {
    sized(bounds.width, bounds.height).fixed(position(bounds))
}
