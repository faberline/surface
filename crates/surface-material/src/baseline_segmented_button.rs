//! The original outlined Material 3 segmented button.

use surface::{
    Align, Color, Corners, FillContour, Length, MeasuredGroupLayout, Node, Rect, Role, Style,
    TextAlignment, TextMeasurer, TextScaling, TextStyle,
};
use surface_app::{CursorStyle, Roving, View};

use crate::{shape::LayoutDirection, typography, Theme, VectorIcon};

const FACE_HEIGHT: f32 = 40.;
const TARGET: f32 = 48.;
const BORDER: f32 = 1.;
const INSET: f32 = 12.;
const GAP: f32 = 8.;
const ICON: f32 = 18.;
const BLOCK_PADDING: f32 = 10.;
const HIT_SLACK: f32 = TARGET - FACE_HEIGHT;
const FOCUS_WIDTH: f32 = 3.;
const FOCUS_OUTSET: f32 = 2. + FOCUS_WIDTH;
const CHECK: [[f32; 2]; 6] = [
    [2.5, 9.1],
    [4., 7.6],
    [7., 10.6],
    [14.2, 3.4],
    [15.7, 4.9],
    [7., 13.6],
];

/// The current selection supplied by the application.
#[derive(Clone, Copy, Debug)]
pub enum BaselineSegmentedSelection<'a> {
    /// Exactly one stable item ID. Activating the checked radio has no effect.
    Single(&'a str),
    /// Zero or more stable item IDs. An enabled item may always be toggled.
    Multiple(&'a [&'a str]),
}

/// One named, horizontal Baseline segmented button group.
#[derive(Clone, Copy, Debug)]
pub struct BaselineSegmentedButtonOptions<'a> {
    pub id: &'a str,
    pub name: &'a str,
    /// The available viewport width, including the exterior focus padding.
    pub available_width: f32,
    pub direction: LayoutDirection,
    /// Scales labels. The icon, border and minimum hit target stay fixed.
    pub text_scale: f32,
    pub selection: BaselineSegmentedSelection<'a>,
}

/// A segment's visual content, accessibility name and activation message.
pub struct BaselineSegmentedItem<'a, M> {
    pub id: &'a str,
    pub label: Option<&'a str>,
    /// Defaults to the label. Required for an icon-only choice.
    pub name: Option<&'a str>,
    pub icon: Option<&'a VectorIcon>,
    pub disabled: bool,
    pub on_activate: M,
}

impl Theme {
    /// Build 2–5 equal-width outline segments with a caller-owned selection.
    ///
    /// The group enters at the first enabled item. Left and Right move focus
    /// in the visual direction. Space and Enter activate. This follows
    /// Material's keyboard design, including for single-select. Standalone
    /// WAI-ARIA radios enter at the checked item and select on arrow keys.
    ///
    /// The default face is 40 dp high and each hit target is at least
    /// 48 × 48 dp. Long labels can wrap within their segment. A viewport
    /// too narrow for all minimum targets scrolls the single row with wheel,
    /// keyboard focus, or accessibility reveal. Provide
    /// the actual available width and unique IDs, including generated
    /// descendant IDs. Unusable geometry, content or selection returns None.
    ///
    /// Use [`Theme::button_group`] with Connected for M3 Expressive.
    pub fn baseline_segmented_button<'a, M>(
        &self,
        options: BaselineSegmentedButtonOptions<'a>,
        items: impl IntoIterator<Item = BaselineSegmentedItem<'a, M>>,
    ) -> Option<View<M>> {
        let items: Vec<_> = items.into_iter().collect();
        let font = typography::LABEL_LARGE.scaled(options.text_scale);
        if text(options.id).is_none()
            || text(options.name).is_none()
            || !(2..=5).contains(&items.len())
            || !options.available_width.is_finite()
            || options.available_width < TARGET + 2. * FOCUS_OUTSET
            || !options.text_scale.is_finite()
            || options.text_scale <= 0.
            || !font.line_height.is_some_and(f32::is_finite)
            || !valid_ids(options, &items)
        {
            return None;
        }

        let single = matches!(options.selection, BaselineSegmentedSelection::Single(_));
        let rtl = options.direction == LayoutDirection::RightToLeft;
        let count = items.len();
        let mut contents = Vec::with_capacity(count);
        let mut controls = Vec::with_capacity(count);
        let mut outlines = Vec::with_capacity(count);
        let mut seams = Vec::with_capacity(count - 1);
        let mut previous_disabled = false;
        for (index, item) in items.into_iter().enumerate() {
            let label = item.label.and_then(text);
            let name = item.name.and_then(text).or(label)?;
            if label.is_none() && item.icon.is_none() {
                return None;
            }
            let checked = match options.selection {
                BaselineSegmentedSelection::Single(id) => item.id == id,
                BaselineSegmentedSelection::Multiple(ids) => ids.contains(&item.id),
            };
            let physical = if rtl { count - 1 - index } else { index };
            let edge = match physical {
                0 => Edge::Left,
                x if x == count - 1 => Edge::Right,
                _ => Edge::Middle,
            };
            let layout = ContentLayout {
                label: label.map(str::to_owned),
                font: font.clone(),
                rtl,
                icon: item.icon.is_some(),
                checked,
            };
            let disabled = item.disabled;
            outlines.push(outline(self, edge, disabled));
            if index > 0 {
                seams.push(passive(
                    Node::new(Role::Group, "")
                        .id(format!("{}-seam-{}", options.id, index - 1))
                        .styled(
                            Style::column()
                                .background(outline_ink(self, previous_disabled && disabled)),
                        ),
                ));
            }
            previous_disabled = disabled;
            controls.push(self.baseline_segment(item, name, layout.clone(), edge, single)?);
            contents.push(layout);
        }

        let minimum_slot = if contents.iter().any(|item| item.label.is_some()) {
            TARGET.max(2. * INSET + font.font_size.ceil())
        } else {
            TARGET
        };
        let geometry = TrackLayout {
            contents,
            minimum_slot,
            rtl,
            available: options.available_width,
        };
        let min_width = geometry.min_width();
        let scroll = min_width > options.available_width;
        let mut viewport = Style::column()
            .width(Length::Px(options.available_width))
            .align(if rtl && !scroll {
                Align::End
            } else {
                Align::Start
            })
            .clip(true);
        if rtl && scroll {
            viewport.scroll_x = min_width - options.available_width;
        }
        let outline = passive(
            Node::new(Role::Group, "")
                .id(format!("{}-outline", options.id))
                .measured_group(OutlineLayout { count, rtl }),
        )
        .children(outlines);
        let mut strip = Style::row();
        strip.min_width = min_width;
        let mut track = Node::new(Role::Group, "")
            .id(format!("{}-track", options.id))
            .styled(strip)
            .measured_group(geometry);
        if rtl {
            track = track.reverse_row();
        }
        let root = View::new(
            Node::new(
                if single {
                    Role::RadioGroup
                } else {
                    Role::Group
                },
                options.name,
            )
            .id(options.id)
            .styled(viewport),
        )
        .children([View::new(track)
            .children(controls.into_iter().chain([outline]).chain(seams))
            .roving_first(Roving::horizontal(false))]);
        if scroll {
            Some(
                root.on_horizontal_scroll(|_, _| None)
                    .accessibility_scroll_pages(),
            )
        } else {
            Some(root)
        }
    }

    fn baseline_segment<M>(
        &self,
        item: BaselineSegmentedItem<'_, M>,
        name: &str,
        layout: ContentLayout,
        edge: Edge,
        single: bool,
    ) -> Option<View<M>> {
        let checked = layout.checked;
        let disabled = item.disabled;
        let ink = if disabled {
            alpha(self.colors.on_surface, 97)
        } else if checked {
            self.colors.on_secondary_container
        } else {
            self.colors.on_surface
        };
        let face_id = format!("{}-face", item.id);
        let mut children = Vec::new();
        if let Some(label) = &layout.label {
            children.push(passive(
                Node::new(Role::Text, "")
                    .id(format!("{}-label", item.id))
                    .text(label)
                    .text_scaling(TextScaling::Fixed)
                    .text_alignment(TextAlignment::Center)
                    .styled(
                        Style::column()
                            .with_text_style(layout.font.clone())
                            .foreground(ink),
                    ),
            ));
        }
        if checked || item.icon.is_some() {
            let contour = if checked && layout.label.is_some() {
                FillContour::new(CHECK, ink)?
            } else if let Some(icon) = item.icon {
                icon.contour_dp(ICON, ink)?
            } else {
                return None;
            };
            children.push(passive(
                Node::new(Role::Group, "")
                    .id(format!("{}-icon", item.id))
                    .fill_contour(contour),
            ));
        }
        if checked && layout.label.is_none() {
            children.push(passive(
                Node::new(Role::Group, "")
                    .id(format!("{}-check", item.id))
                    .fill_contour(FillContour::new(CHECK, ink)?),
            ));
        }
        let face = passive(
            Node::new(Role::Group, "")
                .id(&face_id)
                .styled(
                    Style::column()
                        .corners(edge.corners())
                        .background(if checked && !disabled {
                            self.colors.secondary_container
                        } else {
                            Color::TRANSPARENT
                        })
                        .state_layer(if disabled { Color::TRANSPARENT } else { ink }),
                )
                .state_layer_alphas(20, 26, 26)
                .measured_group(layout),
        )
        .children(children);
        let focus = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-focus", item.id))
                .styled(Style::column().corners(edge.corners()))
                .focus_color(self.colors.secondary)
                .focus_outline_width(FOCUS_WIDTH)
                .focus_outline_from_parent(),
        )
        .visual_overlay();
        let action = View::new(
            Node::new(if single { Role::Radio } else { Role::Checkbox }, name)
                .id(item.id)
                .disabled(disabled)
                .checked(checked)
                .focus_color(Color::TRANSPARENT)
                .press_ripple_to(face_id)
                .measured_group(ActionLayout),
        )
        .children([face, focus]);
        if disabled {
            return Some(action);
        }

        let action = action
            .accessibility_scroll_into_view()
            .cursor(CursorStyle::Pointer);
        if single && checked {
            Some(action)
        } else {
            Some(
                action
                    .on_press(item.on_activate)
                    .accessible_activation()
                    .focus_on_pointer_release()
                    .protect_composition_on_press()
                    .suppress_repeated_activation(),
            )
        }
    }
}

fn text(s: &str) -> Option<&str> {
    (!s.trim().is_empty()).then_some(s)
}

fn valid_ids<M>(
    options: BaselineSegmentedButtonOptions<'_>,
    items: &[BaselineSegmentedItem<'_, M>],
) -> bool {
    let mut ids = std::collections::BTreeSet::new();
    for key in [
        options.id.to_owned(),
        format!("{}-track", options.id),
        format!("{}-outline", options.id),
    ] {
        if !ids.insert(key) {
            return false;
        }
    }
    for index in 0..items.len() - 1 {
        ids.insert(format!("{}-seam-{index}", options.id));
    }
    for item in items {
        if text(item.id).is_none() || !ids.insert(item.id.to_owned()) {
            return false;
        }
        for suffix in ["face", "focus", "label", "icon", "check"] {
            if !ids.insert(format!("{}-{suffix}", item.id)) {
                return false;
            }
        }
    }
    let selected: &[&str] = match &options.selection {
        BaselineSegmentedSelection::Single(id) => std::slice::from_ref(id),
        BaselineSegmentedSelection::Multiple(ids) => ids,
    };
    selected.iter().enumerate().all(|(index, id)| {
        items.iter().any(|item| item.id == *id) && !selected[..index].contains(id)
    })
}

fn alpha(Color(r, g, b, _): Color, a: u8) -> Color {
    Color(r, g, b, a)
}

fn outline_ink(theme: &Theme, disabled: bool) -> Color {
    if disabled {
        alpha(theme.colors.on_surface, 31)
    } else {
        theme.colors.outline
    }
}

fn passive<M>(node: Node) -> View<M> {
    View::new(node).without_interaction(None)
}

#[derive(Clone, Copy, Debug)]
enum Edge {
    Left,
    Middle,
    Right,
}
impl Edge {
    fn corners(self) -> Corners {
        match self {
            Self::Left => Corners::start(Corners::FULL_RADIUS),
            Self::Middle => Corners::ZERO,
            Self::Right => Corners::end(Corners::FULL_RADIUS),
        }
    }
}

fn outline<M>(theme: &Theme, edge: Edge, disabled: bool) -> View<M> {
    let color = outline_ink(theme, disabled);
    let line = || passive(Node::new(Role::Group, "").styled(Style::column().background(color)));
    let cap = (!matches!(edge, Edge::Middle)).then(|| {
        passive(
            Node::new(Role::Group, "")
                .styled(Style::column().clip(true))
                .measured_group(ClipCap(edge)),
        )
        .children([passive(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .corners(Corners::full())
                    .border(BORDER, color),
            ),
        )])
    });
    passive(Node::new(Role::Group, "").measured_group(OutlineCell(edge)))
        .children([line(), line()].into_iter().chain(cap))
}

#[derive(Clone, Debug)]
struct ContentLayout {
    label: Option<String>,
    font: TextStyle,
    rtl: bool,
    icon: bool,
    checked: bool,
}

impl ContentLayout {
    fn natural(&self, fonts: &mut dyn TextMeasurer) -> f32 {
        let content = self.label.as_ref().map_or(2. * ICON, |label| {
            fonts.measure_text(label, &self.font, None).0 + ICON
        });
        (content + GAP + 2. * INSET).ceil()
    }

    fn layout(&self, width: f32, fonts: &mut dyn TextMeasurer, reserve: bool) -> (f32, Vec<Rect>) {
        let usable = width - 2. * INSET;
        let first = self.label.as_ref().map_or((ICON, ICON), |label| {
            fonts.measure_text(label, &self.font, None)
        });
        let second = if self.label.is_some() {
            reserve || self.checked || self.icon
        } else {
            reserve || self.checked
        };
        let minimum = first.0.min(self.font.font_size.max(ICON));
        let stacked = second && usable < minimum + GAP + ICON;
        let label_width = if self.label.is_some() {
            first
                .0
                .min(if second && !stacked {
                    usable - GAP - ICON
                } else {
                    usable
                })
                .max(1.)
        } else {
            ICON
        };
        let label_height = self.label.as_ref().map_or(ICON, |label| {
            fonts
                .measure_text(label, &self.font, Some(label_width))
                .1
                .ceil()
        });
        if stacked {
            return (
                ICON + GAP + label_height,
                vec![
                    Rect::new(
                        (width - label_width) / 2.,
                        ICON + GAP,
                        label_width,
                        label_height,
                    ),
                    Rect::new((width - ICON) / 2., 0., ICON, ICON),
                ],
            );
        }
        let height = label_height.max(if second { ICON } else { 0. });
        let second_width = if second { ICON + GAP } else { 0. };
        let x = (width - label_width - second_width) / 2.;
        let first_x = if self.rtl { x } else { x + second_width };
        let second_x = if self.rtl { x + label_width + GAP } else { x };
        let mut contents = vec![Rect::new(
            first_x,
            (height - label_height) / 2.,
            label_width,
            label_height,
        )];
        if second {
            contents.push(Rect::new(second_x, (height - ICON) / 2., ICON, ICON));
        }
        (height, contents)
    }
}

impl MeasuredGroupLayout for ContentLayout {
    fn measure(
        &self,
        _children: &[&Node],
        width: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let width = width.unwrap_or_else(|| self.natural(fonts)).max(TARGET);
        (
            width,
            self.layout(width, fonts, true).0 + 2. * BLOCK_PADDING,
        )
    }

    fn place(
        &self,
        _children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let (content_height, mut contents) = self.layout(width, fonts, false);
        for rect in &mut contents {
            rect.y += (height - content_height) / 2.;
        }
        contents
    }
}

#[derive(Debug)]
struct TrackLayout {
    contents: Vec<ContentLayout>,
    minimum_slot: f32,
    rtl: bool,
    available: f32,
}
impl TrackLayout {
    fn min_width(&self) -> f32 {
        self.contents.len() as f32 * self.minimum_slot + 2. * FOCUS_OUTSET
    }

    fn slot_width(&self, width: f32) -> f32 {
        (width - 2. * FOCUS_OUTSET) / self.contents.len() as f32
    }

    fn face_height(&self, slot: f32, fonts: &mut dyn TextMeasurer) -> f32 {
        self.contents
            .iter()
            .map(|layout| layout.layout(slot, fonts, true).0 + 2. * BLOCK_PADDING)
            .fold(FACE_HEIGHT, f32::max)
    }
}
impl MeasuredGroupLayout for TrackLayout {
    fn places_measured_groups(&self) -> bool {
        true
    }

    fn measure(
        &self,
        _children: &[&Node],
        available: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        let intrinsic = self
            .contents
            .iter()
            .map(|item| item.natural(fonts))
            .fold(TARGET, f32::max);
        let width = (intrinsic * self.contents.len() as f32 + 2. * FOCUS_OUTSET)
            .min(available.unwrap_or(self.available).min(self.available))
            .max(self.min_width());
        (
            width,
            self.face_height(self.slot_width(width), fonts) + 2. * FOCUS_OUTSET,
        )
    }

    fn place(
        &self,
        _children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let count = self.contents.len();
        let slot = self.slot_width(width);
        let face = self.face_height(slot, fonts);
        let action = face + HIT_SLACK;
        let y = (height - action) / 2.;
        let actions = (0..count).map(|i| {
            let x = logical_index(count, i, self.rtl) as f32 * slot + FOCUS_OUTSET;
            Rect::new(x, y, slot, action)
        });
        let outline = Rect::new(
            FOCUS_OUTSET,
            (height - face) / 2.,
            width - 2. * FOCUS_OUTSET,
            face,
        );
        let seams = (1..count).map(|i| {
            let edge = if self.rtl { count - i } else { i };
            Rect::new(
                outline.x + edge as f32 * slot - BORDER / 2.,
                outline.y + BORDER,
                BORDER,
                face - 2. * BORDER,
            )
        });
        actions.chain([outline]).chain(seams).collect()
    }
}

fn logical_index(count: usize, index: usize, rtl: bool) -> usize {
    if rtl {
        count - 1 - index
    } else {
        index
    }
}

#[derive(Debug)]
struct ActionLayout;
impl MeasuredGroupLayout for ActionLayout {
    fn places_measured_groups(&self) -> bool {
        true
    }
    fn measure(
        &self,
        _children: &[&Node],
        _width: Option<f32>,
        _fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        (TARGET, TARGET)
    }
    fn place(
        &self,
        _children: &[&Node],
        (width, height): (f32, f32),
        _fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let face = height - HIT_SLACK;
        let y = (height - face) / 2.;
        vec![
            Rect::new(0., y, width, face),
            Rect::new(
                -FOCUS_OUTSET,
                y - FOCUS_OUTSET,
                width + 2. * FOCUS_OUTSET,
                face + 2. * FOCUS_OUTSET,
            ),
        ]
    }
}

#[derive(Debug)]
struct OutlineLayout {
    count: usize,
    rtl: bool,
}
impl MeasuredGroupLayout for OutlineLayout {
    fn places_measured_groups(&self) -> bool {
        true
    }
    fn measure(&self, _: &[&Node], _: Option<f32>, _: &mut dyn TextMeasurer) -> (f32, f32) {
        (0., 0.)
    }
    fn place(
        &self,
        _: &[&Node],
        (width, height): (f32, f32),
        _: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let slot = width / self.count as f32;
        (0..self.count)
            .map(|i| {
                Rect::new(
                    logical_index(self.count, i, self.rtl) as f32 * slot,
                    0.,
                    slot,
                    height,
                )
            })
            .collect()
    }
}

#[derive(Debug)]
struct OutlineCell(Edge);
impl MeasuredGroupLayout for OutlineCell {
    fn places_measured_groups(&self) -> bool {
        true
    }
    fn measure(&self, _: &[&Node], _: Option<f32>, _: &mut dyn TextMeasurer) -> (f32, f32) {
        (0., 0.)
    }
    fn place(
        &self,
        _: &[&Node],
        (width, height): (f32, f32),
        _: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let cap = width.min(height) / 2.;
        let (x, straight) = match self.0 {
            Edge::Left => (cap, width - cap),
            Edge::Middle => (0., width),
            Edge::Right => (0., width - cap),
        };
        let lines = [
            Rect::new(x, 0., straight, BORDER),
            Rect::new(x, height - BORDER, straight, BORDER),
        ];
        let rounded = match self.0 {
            Edge::Left => Some(Rect::new(0., 0., cap, height)),
            Edge::Middle => None,
            Edge::Right => Some(Rect::new(width - cap, 0., cap, height)),
        };
        lines.into_iter().chain(rounded).collect()
    }
}

#[derive(Debug)]
struct ClipCap(Edge);
impl MeasuredGroupLayout for ClipCap {
    fn measure(&self, _: &[&Node], _: Option<f32>, _: &mut dyn TextMeasurer) -> (f32, f32) {
        (0., 0.)
    }
    fn place(
        &self,
        _: &[&Node],
        (width, height): (f32, f32),
        _: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        // Only one end of this stroke belongs to the shared perimeter. Clip
        // the other end before painting so the straight edges meet once.
        let x = if matches!(self.0, Edge::Left) {
            0.
        } else {
            -width
        };
        vec![Rect::new(x, 0., width * 2., height)]
    }
}
