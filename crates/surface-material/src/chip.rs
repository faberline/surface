//! Material 3 chips with caller-owned choices and separate action targets.

use surface::{
    Align, Color, Corners, FillContour, GridRole, Insets, Length, MeasuredGroupLayout, Node,
    RasterImage, Rect, Role, Style, TextAlignment, TextMeasurer, TextScaling, TextStyle,
};
use surface_app::{CursorStyle, Key, View};

use crate::{
    motion::STATE_CHANGE,
    shape::{self, LayoutDirection},
    typography,
    vector_icon::chevron_contour,
    ChipKind, Elevation, Theme, VectorIcon,
};

const TOUCH: f32 = 48.;
const HEIGHT: f32 = 32.;
const VERTICAL_SLACK: f32 = (TOUCH - HEIGHT) / 2.;
const ICON: f32 = 18.;
const AVATAR: f32 = 24.;
const GAP: f32 = 8.;
const ICON_EDGE: f32 = 8.;
const BARE_EDGE: f32 = 16.;
const TAIL_PAINT: f32 = GAP + ICON + ICON_EDGE;
const MIN_SPLIT_PAINT: f32 = 88.;
const FOCUS_STROKE: f32 = 3.;
const FOCUS_OUTSET: f32 = 2. + FOCUS_STROKE;
const DISABLED_CONTENT: u8 = 97;
const DISABLED_CONTAINER: u8 = 31;
const HOVER: u8 = 20;
const ACTIVE: u8 = 26;

/// The appearance of an Assist, Filter, or Suggestion chip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChipStyle {
    #[default]
    Flat,
    Elevated,
}

/// Decorative artwork at the chip's logical start.
#[derive(Clone, Copy, Debug)]
pub enum ChipLeading<'a> {
    /// An 18 dp icon tinted with the chip's role color.
    Icon(&'a VectorIcon),
    /// An 18 dp image, cropped to its square slot without a tint.
    Image(&'a RasterImage),
    /// A 24 dp circular image for an Input chip.
    Avatar(&'a RasterImage),
}

/// Common layout and naming for a Material chip.
#[derive(Clone, Copy, Debug)]
pub struct ChipOptions<'a> {
    /// Stable ID of the body action, or the sole action in a remove-only chip.
    pub id: &'a str,
    pub label: &'a str,
    /// Omitted or blank uses the visible label. A remove-only Input uses its
    /// explicit removal name instead.
    pub accessible_name: Option<&'a str>,
    pub direction: LayoutDirection,
    pub text_scale: f32,
    /// Width available for the artwork and all action targets in logical dp.
    pub max_width: f32,
    pub disabled: bool,
}

/// The explicit localized name and message of an Input chip's remove action.
pub struct ChipRemove<'a, M> {
    pub accessible_name: &'a str,
    pub message: M,
}

/// An optional trailing action on a Filter chip.
pub enum ChipTrailing<'a, M> {
    Remove(ChipRemove<'a, M>),
    /// A separate, independently named menu trigger at the logical end.
    Menu {
        accessible_name: &'a str,
        expanded: bool,
        message: M,
    },
    /// The body message opens the menu. Use this when the chip should have
    /// one action, such as in a compact layout.
    MenuOnBody {
        expanded: bool,
    },
}

/// Variant-specific chip artwork, app-owned state, and messages.
pub enum ChipContent<'a, M> {
    Assist {
        style: ChipStyle,
        leading: Option<ChipLeading<'a>>,
        message: M,
    },
    Filter {
        style: ChipStyle,
        leading: Option<ChipLeading<'a>>,
        selected: bool,
        /// Replace the leading artwork with a check when selected.
        show_checkmark: bool,
        message: M,
        trailing: Option<ChipTrailing<'a, M>>,
    },
    Input {
        leading: Option<ChipLeading<'a>>,
        selected: bool,
        /// An optional select or edit action. Without one the entire chip
        /// removes the token, using one keyboard and accessibility target.
        body: Option<M>,
        remove: ChipRemove<'a, M>,
    },
    Suggestion {
        style: ChipStyle,
        leading: Option<ChipLeading<'a>>,
        message: M,
    },
}

#[derive(Clone, Copy)]
enum Glyph {
    Check,
    Remove,
    Menu,
}

#[derive(Clone, Copy)]
enum Leading<'a> {
    Art(ChipLeading<'a>),
    Check,
}

impl Leading<'_> {
    fn size(self) -> f32 {
        if matches!(self, Self::Art(ChipLeading::Avatar(_))) {
            AVATAR
        } else {
            ICON
        }
    }
}

struct Secondary<'a, M> {
    name: &'a str,
    expanded: Option<bool>,
    message: M,
}

impl<M> Secondary<'_, M> {
    fn glyph(&self) -> Glyph {
        if self.expanded.is_some() {
            Glyph::Menu
        } else {
            Glyph::Remove
        }
    }
}

struct Chip<'a, M> {
    kind: ChipKind,
    style: ChipStyle,
    selected: bool,
    leading: Option<Leading<'a>>,
    body_role: Role,
    body_name: &'a str,
    body_message: M,
    body_menu: Option<bool>,
    body_icon: Option<Glyph>,
    secondary: Option<Secondary<'a, M>>,
    remove_key: Option<M>,
}

impl<'a, M: Clone> Chip<'a, M> {
    fn new(options: ChipOptions<'a>, content: ChipContent<'a, M>) -> Self {
        let body_name = options
            .accessible_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(options.label);
        let plain = |kind, style, leading: Option<ChipLeading<'a>>, body_message| Self {
            kind,
            style,
            selected: false,
            leading: leading.map(Leading::Art),
            body_role: Role::Button,
            body_name,
            body_message,
            body_menu: None,
            body_icon: None,
            secondary: None,
            remove_key: None,
        };
        match content {
            ChipContent::Assist {
                style,
                leading,
                message,
            } => plain(ChipKind::Assist, style, leading, message),
            ChipContent::Suggestion {
                style,
                leading,
                message,
            } => plain(ChipKind::Suggestion, style, leading, message),
            ChipContent::Filter {
                style,
                leading,
                selected,
                show_checkmark,
                message,
                trailing,
            } => {
                let mut chip = plain(ChipKind::Filter, style, leading, message);
                chip.selected = selected;
                chip.body_role = Role::Checkbox;
                if selected && show_checkmark {
                    chip.leading = Some(Leading::Check);
                }
                match trailing {
                    Some(ChipTrailing::Remove(remove)) => {
                        chip.secondary = Some(Secondary {
                            name: remove.accessible_name,
                            expanded: None,
                            message: remove.message,
                        });
                    }
                    Some(ChipTrailing::Menu {
                        accessible_name,
                        expanded,
                        message,
                    }) => {
                        chip.secondary = Some(Secondary {
                            name: accessible_name,
                            expanded: Some(expanded),
                            message,
                        });
                    }
                    Some(ChipTrailing::MenuOnBody { expanded }) => {
                        chip.body_role = Role::Button;
                        chip.body_menu = Some(expanded);
                        chip.body_icon = Some(Glyph::Menu);
                    }
                    None => {}
                }
                chip
            }
            ChipContent::Input {
                leading,
                selected,
                body,
                remove,
            } => {
                let remove_key = Some(remove.message.clone());
                let mut chip = if let Some(body) = body {
                    let mut chip = plain(ChipKind::Input, ChipStyle::Flat, leading, body);
                    chip.secondary = Some(Secondary {
                        name: remove.accessible_name,
                        expanded: None,
                        message: remove.message,
                    });
                    chip
                } else {
                    let mut chip = plain(ChipKind::Input, ChipStyle::Flat, leading, remove.message);
                    chip.body_name = remove.accessible_name;
                    chip.body_icon = Some(Glyph::Remove);
                    chip
                };
                chip.selected = selected;
                chip.remove_key = remove_key;
                chip
            }
        }
    }

    fn valid(&self, options: ChipOptions<'_>) -> bool {
        !options.id.trim().is_empty()
            && !options.label.trim().is_empty()
            && !self.body_name.trim().is_empty()
            && self
                .secondary
                .as_ref()
                .is_none_or(|part| !part.name.trim().is_empty())
            && (matches!(self.kind, ChipKind::Input)
                || !matches!(self.leading, Some(Leading::Art(ChipLeading::Avatar(_)))))
    }
}

#[derive(Clone, Copy, Debug)]
struct Geometry {
    start: f32,
    end: f32,
    min_body: f32,
    max_body: f32,
    face_extension: f32,
}

impl Geometry {
    fn new<M>(chip: &Chip<'_, M>, typography: &TextStyle, max_width: f32) -> Option<Self> {
        let start = match chip.leading {
            Some(Leading::Art(ChipLeading::Avatar(_))) => 4.,
            Some(_) => ICON_EDGE,
            None if matches!(chip.kind, ChipKind::Input) => 12.,
            None => BARE_EDGE,
        };
        let split = chip.secondary.is_some();
        let end = if split {
            0.
        } else if chip.body_icon.is_some() {
            ICON_EDGE
        } else {
            BARE_EDGE
        };
        let leading_width = chip.leading.map_or(0., |leading| leading.size() + GAP);
        let trailing_width = chip.body_icon.map_or(0., |_| ICON + GAP);
        let label_space = typography.font_size;
        let min_body = (start + leading_width + label_space + trailing_width + end).max(if split {
            MIN_SPLIT_PAINT - TAIL_PAINT
        } else {
            TOUCH
        });
        let max_body = max_width - if split { TOUCH } else { 0. };
        (max_body.is_finite() && max_body >= min_body).then_some(Self {
            start,
            end,
            min_body,
            max_body,
            face_extension: if split { TAIL_PAINT } else { 0. },
        })
    }
}

struct Palette {
    background: Color,
    border: Color,
    focused_border: Color,
    label: Color,
    leading: Color,
    trailing: Color,
    layer: Color,
    pressed: Color,
    elevation: Elevation,
    hover_elevation: Elevation,
}

impl Palette {
    fn new<M>(theme: &Theme, chip: &Chip<'_, M>, disabled: bool) -> Self {
        let c = &theme.colors;
        let assist = matches!(chip.kind, ChipKind::Assist);
        let filter = matches!(chip.kind, ChipKind::Filter);
        let input = matches!(chip.kind, ChipKind::Input);
        let elevated = chip.style == ChipStyle::Elevated;
        let selected = chip.selected;
        let label = if selected {
            c.on_secondary_container
        } else if assist {
            c.on_surface
        } else {
            c.on_surface_variant
        };
        let leading = if selected && filter {
            c.on_secondary_container
        } else if input && !selected {
            c.on_surface_variant
        } else {
            c.primary
        };
        let filled = selected || elevated;
        let background = if selected {
            c.secondary_container
        } else if elevated {
            c.surface_container_low
        } else {
            Color::TRANSPARENT
        };
        let base = if elevated {
            Elevation::Level1
        } else {
            Elevation::Level0
        };
        let hover = if elevated {
            Elevation::Level2
        } else if filter && selected {
            Elevation::Level1
        } else {
            base
        };
        if disabled {
            let ink = alpha(c.on_surface, DISABLED_CONTENT);
            let surface = alpha(c.on_surface, DISABLED_CONTAINER);
            return Self {
                background: if filled { surface } else { Color::TRANSPARENT },
                border: if filled { Color::TRANSPARENT } else { surface },
                focused_border: Color::TRANSPARENT,
                label: ink,
                leading: ink,
                trailing: ink,
                layer: Color::TRANSPARENT,
                pressed: Color::TRANSPARENT,
                elevation: Elevation::Level0,
                hover_elevation: Elevation::Level0,
            };
        }
        Self {
            background,
            border: if filled {
                Color::TRANSPARENT
            } else {
                c.outline_variant
            },
            focused_border: if filled { Color::TRANSPARENT } else { label },
            label,
            leading,
            trailing: if selected {
                c.on_secondary_container
            } else {
                c.on_surface_variant
            },
            layer: label,
            pressed: if filter {
                if selected {
                    c.on_surface_variant
                } else {
                    c.on_secondary_container
                }
            } else {
                label
            },
            elevation: base,
            hover_elevation: hover,
        }
    }
}

impl Theme {
    /// Build a standard Material 3 chip. Returns `None` if a name is blank,
    /// the artwork is unsupported, or the requested width cannot fit its
    /// fixed artwork, label, and non-overlapping action targets.
    ///
    /// The outer ID is `{id}-chip`. The body or sole action uses `id`.
    /// A separate trailing action uses `{id}-remove` or `{id}-menu`.
    pub fn chip_with_options<M: Clone + 'static>(
        &self,
        options: ChipOptions<'_>,
        content: ChipContent<'_, M>,
    ) -> Option<View<M>> {
        self.build_chip(options, content, false)
    }

    /// Build the same chip with a passive grid cell around each action.
    /// Place it under a named grid's row. The row owner handles directional
    /// navigation. Cell IDs append `-cell` to the unchanged action IDs.
    pub fn chip_in_grid<M: Clone + 'static>(
        &self,
        options: ChipOptions<'_>,
        content: ChipContent<'_, M>,
    ) -> Option<View<M>> {
        self.build_chip(options, content, true)
    }

    fn build_chip<M: Clone + 'static>(
        &self,
        options: ChipOptions<'_>,
        content: ChipContent<'_, M>,
        grid: bool,
    ) -> Option<View<M>> {
        let chip = Chip::new(options, content);
        if !chip.valid(options) {
            return None;
        }
        let scale = options.text_scale;
        let font = typography::LABEL_LARGE.scaled(if scale > 0. && (20. * scale).is_finite() {
            scale
        } else {
            1.
        });
        let geometry = Geometry::new(&chip, &font, options.max_width)?;
        let palette = Palette::new(self, &chip, options.disabled);
        let face = self.chip_face(options, &geometry, &palette);
        let content = body_content(options, &chip, &geometry, font, &palette);
        let inset = |outset: f32| {
            logical_insets(
                options.direction,
                -outset,
                -geometry.face_extension - outset,
                VERTICAL_SLACK - outset,
            )
        };
        let focus = focus_ring(
            format!("{}-focus", options.id),
            inset(FOCUS_OUTSET),
            Corners::all(8. + FOCUS_OUTSET),
            self.colors.secondary,
            FOCUS_STROKE,
        );
        let border = (palette.focused_border.3 > 0).then(|| {
            focus_ring(
                format!("{}-focused-border", options.id),
                inset(0.),
                shape::SMALL,
                palette.focused_border,
                1.,
            )
        });
        let mut body_style = Style::column();
        body_style.min_width = geometry.min_body;
        body_style.max_width = Length::Px(geometry.max_body);
        body_style.min_height = TOUCH;
        let mut body = Node::new(chip.body_role, chip.body_name)
            .id(options.id)
            .press_ripple_to(format!("{}-face", options.id))
            .styled(body_style);
        if chip.body_role == Role::Checkbox {
            body = body.checked(chip.selected);
        } else if chip.body_menu.is_some()
            || matches!(chip.kind, ChipKind::Input) && chip.secondary.is_some()
        {
            body = body.selected(chip.selected);
        }
        let keyboard_removal = chip.remove_key.is_some();
        let second = chip
            .secondary
            .map(|part| self.chip_secondary(options, part, &palette, keyboard_removal, grid));
        let body = bind_action(
            body,
            options.disabled,
            chip.body_menu,
            chip.body_message,
            chip.remove_key,
        )
        .children(
            [
                face,
                fixed_space(0., VERTICAL_SLACK),
                content,
                fixed_space(0., VERTICAL_SLACK),
                focus,
            ]
            .into_iter()
            .chain(border),
        );
        let mut root_style = Style::row();
        root_style.min_width = geometry.min_body + if second.is_some() { TOUCH } else { 0. };
        root_style.max_width = Length::Px(options.max_width);
        Some(
            View::new(directed(
                Node::new(Role::Group, "")
                    .id(format!("{}-chip", options.id))
                    .styled(root_style),
                options.direction,
            ))
            .children(std::iter::once(grid_cell(options.id, body, grid)).chain(second)),
        )
    }

    fn chip_face<M>(
        &self,
        options: ChipOptions<'_>,
        geometry: &Geometry,
        palette: &Palette,
    ) -> View<M> {
        let style = Style::row()
            .absolute(logical_insets(
                options.direction,
                0.,
                -geometry.face_extension,
                VERTICAL_SLACK,
            ))
            .corners(shape::SMALL)
            .background(palette.background)
            .border(1., palette.border)
            .state_layer(palette.layer)
            .transition(STATE_CHANGE);
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-face", options.id))
                .styled(style)
                .box_shadows(self.elevation(palette.elevation))
                .interaction_box_shadows(Some(self.elevation(palette.hover_elevation)), None, None)
                .interaction_box_shadows_from_parent()
                .state_layer_alphas(HOVER, ACTIVE, ACTIVE)
                .press_state_layer_ink(palette.pressed),
        )
        .without_interaction(None)
    }

    fn chip_secondary<M: Clone + 'static>(
        &self,
        options: ChipOptions<'_>,
        part: Secondary<'_, M>,
        palette: &Palette,
        keyboard_removal: bool,
        grid: bool,
    ) -> View<M> {
        let glyph_kind = part.glyph();
        let suffix = if part.expanded.is_some() {
            "menu"
        } else {
            "remove"
        };
        let id = format!("{}-{suffix}", options.id);
        let style = |size: f32| {
            let start = GAP + (ICON - size) / 2.;
            let mut style = Style::column()
                .width(Length::Px(size))
                .height(Length::Px(size));
            let inset = if options.direction == LayoutDirection::LeftToRight {
                Insets {
                    left: Length::Px(start),
                    ..Default::default()
                }
            } else {
                Insets {
                    right: Length::Px(start),
                    ..Default::default()
                }
            };
            style = style.absolute(inset);
            style
        };
        let glyph = View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-icon"))
                .styled(style(ICON))
                .fill_contour(glyph(glyph_kind, palette.trailing)),
        )
        .without_interaction(None);
        let face = View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-face"))
                .styled(
                    style(24.)
                        .corners(shape::FULL)
                        .state_layer(palette.trailing),
                )
                .state_layer_alphas(HOVER, ACTIVE, ACTIVE)
                .press_state_layer_ink(palette.trailing),
        )
        .without_interaction(None);
        let ring = View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-focus"))
                .styled(style(ICON + 2. * FOCUS_OUTSET).corners(shape::FULL))
                .focus_outline_from_parent()
                .focus_color(self.colors.secondary)
                .focus_outline_width(FOCUS_STROKE),
        )
        .without_interaction(None);
        let mut hit = Style::row().align(Align::Center).width(Length::Px(TOUCH));
        hit.min_width = TOUCH;
        hit.min_height = TOUCH;
        let remove_key = keyboard_removal.then(|| part.message.clone());
        let action = bind_action(
            Node::new(Role::Button, part.name)
                .id(&id)
                .press_ripple_to(format!("{id}-face"))
                .styled(hit),
            options.disabled,
            part.expanded,
            part.message,
            remove_key,
        )
        .children([face, glyph, ring]);
        grid_cell(&id, action, grid)
    }
}

fn grid_cell<M>(id: &str, action: View<M>, grid: bool) -> View<M> {
    if grid {
        View::new(
            Node::new(Role::Group, "")
                .id(format!("{id}-cell"))
                .grid_role(GridRole::GridCell)
                .styled(Style::row()),
        )
        .children([action])
    } else {
        action
    }
}

fn body_content<M>(
    options: ChipOptions<'_>,
    chip: &Chip<'_, M>,
    geometry: &Geometry,
    font: TextStyle,
    palette: &Palette,
) -> View<M> {
    let label = View::new(
        Node::new(Role::Text, "")
            .id(format!("{}-label", options.id))
            .text(options.label)
            .text_scaling(TextScaling::Fixed)
            .text_alignment(if options.direction == LayoutDirection::LeftToRight {
                TextAlignment::Left
            } else {
                TextAlignment::Right
            })
            .styled(
                Style::column()
                    .with_text_style(font)
                    .foreground(palette.label),
            ),
    );
    let leading = chip
        .leading
        .map(|icon| leading_view(options, icon, palette.leading));
    let trailing = chip
        .body_icon
        .map(|icon| glyph_view(format!("{}-trailing", options.id), icon, palette.trailing));
    View::new(Node::new(Role::Group, "").measured_group(ContentsLayout {
        geometry: *geometry,
        direction: options.direction,
        leading_size: chip.leading.map(Leading::size),
        trailing: chip.body_icon.is_some(),
    }))
    .children(std::iter::once(label).chain(leading).chain(trailing))
    .without_interaction(None)
}

#[derive(Debug)]
struct ContentsLayout {
    geometry: Geometry,
    direction: LayoutDirection,
    leading_size: Option<f32>,
    trailing: bool,
}

impl ContentsLayout {
    fn label_start(&self) -> f32 {
        self.geometry.start + self.leading_size.map_or(0., |size| size + GAP)
    }

    fn overhead(&self) -> f32 {
        self.label_start() + self.geometry.end + if self.trailing { GAP + ICON } else { 0. }
    }

    fn label_height(&self, label: &Node, width: f32, fonts: &mut dyn TextMeasurer) -> f32 {
        fonts
            .measure_text(
                label.text_value().unwrap_or_default(),
                &label.style().text_style(),
                Some(width),
            )
            .1
    }
}

impl MeasuredGroupLayout for ContentsLayout {
    fn measure(
        &self,
        children: &[&Node],
        available_width: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let label = children[0];
        let width_cap = available_width
            .unwrap_or(self.geometry.max_body)
            .clamp(self.geometry.min_body, self.geometry.max_body);
        let text_width = fonts
            .measure_text(
                label.text_value().unwrap_or_default(),
                &label.style().text_style(),
                None,
            )
            .0;
        let overhead = self.overhead();
        let width = (text_width + overhead)
            .clamp(self.geometry.min_body, width_cap)
            .ceil()
            .min(width_cap);
        let height = self.label_height(label, width - overhead, fonts);
        (width, HEIGHT.max(height + 12.))
    }

    fn place(
        &self,
        children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let label_width = (width - self.overhead()).max(0.);
        let label_height = self.label_height(children[0], label_width, fonts);
        let rect = |start, w, h| {
            let x = if self.direction == LayoutDirection::LeftToRight {
                start
            } else {
                width - start - w
            };
            Rect::new(x, (height - h) / 2., w, h)
        };
        std::iter::once(rect(self.label_start(), label_width, label_height))
            .chain(
                self.leading_size
                    .map(|size| rect(self.geometry.start, size, size)),
            )
            .chain(
                self.trailing
                    .then(|| rect(width - self.geometry.end - ICON, ICON, ICON)),
            )
            .collect()
    }
}

fn bind_action<M: Clone + 'static>(
    mut node: Node,
    disabled: bool,
    menu: Option<bool>,
    message: M,
    remove: Option<M>,
) -> View<M> {
    if let Some(expanded) = menu {
        node = node.menu_expanded(expanded);
    }
    let view = View::new(node.disabled(disabled).focus_color(Color::TRANSPARENT));
    if disabled {
        return view;
    }
    let view = view
        .on_press(message)
        .accessible_activation()
        .accessibility_scroll_into_view()
        .focus_on_pointer_release()
        .protect_composition_on_press()
        .suppress_repeated_activation()
        .cursor(CursorStyle::Pointer);
    if let Some(remove) = remove {
        view.on_key(move |press| {
            (matches!(press.key, Key::Backspace | Key::Delete)
                && press.modifiers.is_empty()
                && !press.repeat)
                .then(|| remove.clone())
        })
    } else {
        view
    }
}

fn leading_view<M>(options: ChipOptions<'_>, leading: Leading<'_>, color: Color) -> View<M> {
    let size = leading.size();
    let id = format!("{}-leading", options.id);
    let style = fixed_style(size, size);
    let node = Node::new(Role::Group, "").id(id).styled(style.clone());
    let node = match leading {
        Leading::Check => node.fill_contour(glyph(Glyph::Check, color)),
        Leading::Art(ChipLeading::Icon(icon)) => {
            node.fill_contour(icon.contour_dp(size, color).expect("validated icon"))
        }
        Leading::Art(ChipLeading::Image(image) | ChipLeading::Avatar(image)) => {
            let style = if size == AVATAR {
                style.corners(shape::FULL)
            } else {
                style
            };
            node.styled(style).raster_image(if options.disabled {
                dimmed(image)
            } else {
                image.clone()
            })
        }
    };
    View::new(node)
}

fn glyph_view<M>(id: String, icon: Glyph, color: Color) -> View<M> {
    View::new(
        Node::new(Role::Group, "")
            .id(id)
            .styled(fixed_style(ICON, ICON))
            .fill_contour(glyph(icon, color)),
    )
}

fn glyph(icon: Glyph, color: Color) -> FillContour {
    let check = [
        [3.0, 9.0],
        [4.3, 7.7],
        [7.3, 10.7],
        [14.4, 3.7],
        [15.7, 5.0],
        [7.3, 13.4],
    ];
    let remove = [
        [5., 3.6],
        [9., 7.6],
        [13., 3.6],
        [14.4, 5.],
        [10.4, 9.],
        [14.4, 13.],
        [13., 14.4],
        [9., 10.4],
        [5., 14.4],
        [3.6, 13.],
        [7.6, 9.],
        [3.6, 5.],
    ];
    match icon {
        Glyph::Check => FillContour::new(check, color),
        Glyph::Remove => FillContour::new(remove, color),
        Glyph::Menu => chevron_contour(ICON, 0., color),
    }
    .expect("valid chip icon")
}

fn dimmed(image: &RasterImage) -> RasterImage {
    let mut pixels = image.rgba8().to_vec();
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[3] = ((u16::from(pixel[3]) * u16::from(DISABLED_CONTENT) + 127) / 255) as u8;
    }
    RasterImage::new(image.width(), image.height(), pixels).expect("same image size")
}

fn focus_ring<M>(id: String, inset: Insets, corners: Corners, color: Color, width: f32) -> View<M> {
    View::new(
        Node::new(Role::Group, "")
            .id(id)
            .styled(Style::column().absolute(inset).corners(corners))
            .focus_outline_from_parent()
            .focus_color(color)
            .focus_outline_width(width),
    )
    .without_interaction(None)
}

fn fixed_style(width: f32, height: f32) -> Style {
    let mut style = Style::column()
        .width(Length::Px(width))
        .height(Length::Px(height));
    style.min_width = width;
    style.min_height = height;
    style
}

fn fixed_space<M>(width: f32, height: f32) -> View<M> {
    View::new(Node::new(Role::Group, "").styled(fixed_style(width, height)))
        .without_interaction(None)
}

fn logical_insets(direction: LayoutDirection, start: f32, end: f32, y: f32) -> Insets {
    let (left, right) = if direction == LayoutDirection::LeftToRight {
        (start, end)
    } else {
        (end, start)
    };
    Insets {
        left: Length::Px(left),
        right: Length::Px(right),
        top: Length::Px(y),
        bottom: Length::Px(y),
    }
}

fn directed(node: Node, direction: LayoutDirection) -> Node {
    if direction == LayoutDirection::RightToLeft {
        node.reverse_row()
    } else {
        node
    }
}

fn alpha(Color(red, green, blue, original): Color, opacity: u8) -> Color {
    Color(
        red,
        green,
        blue,
        ((u16::from(original) * u16::from(opacity) + 127) / 255) as u8,
    )
}
