//! Material 3 Expressive numeric Scroll Fields.

use std::rc::Rc;

use surface::{
    AdjustableValue, Align, Color, Corners, Insets, Length, MeasuredGroupLayout, Node, Rect, Role,
    Style, TextMeasurer, TextStyle,
};
use surface_app::{CursorStyle, EditKey, FocusNavigation, Key, PointDrag, View};

use crate::{
    date_picker::{button, disabled_ink, text},
    scroll_field_state::{
        ScrollFieldDrag, ScrollFieldError, ScrollFieldEvent, ScrollFieldMode, ScrollFieldState,
        ScrollFieldStep,
    },
    shape::{self, LayoutDirection},
    time_input::keyboard_icon,
    typography,
    vector_icon::VectorIconSize,
    Theme, VectorIcon,
};

const MIN_FACE_HEIGHT: f32 = 120.;
const DEFAULT_FACE_HEIGHT: f32 = 200.;
const ITEM_GAP: f32 = 16.;
const TARGET: f32 = 48.;
const OUTSIDE_TARGET: f32 = 14.;
const FOCUS_WIDTH: f32 = 3.;
const FOCUS_OUTSET: f32 = 2. + FOCUS_WIDTH;
const MODE_GAP: f32 = 8.;
const COPY_GAP: f32 = 4.;
const CENTER: TextStyle = typography::DISPLAY_LARGE_EMPHASIZED;
const NEIGHBOR: TextStyle = typography::DISPLAY_MEDIUM;

/// Recommended floor for the field's painted width.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollFieldSize {
    #[default]
    Small,
    Medium,
    Large,
}

impl ScrollFieldSize {
    const fn min_width(self) -> f32 {
        match self {
            Self::Small => 100.,
            Self::Medium => 120.,
            Self::Large => 186.,
        }
    }

    const fn padding(self) -> f32 {
        match self {
            Self::Small | Self::Medium => 16.,
            Self::Large => 24.,
        }
    }
}

/// Allocation, labels and controlled availability for one inline field.
#[derive(Clone, Copy)]
pub struct ScrollFieldOptions<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub size: ScrollFieldSize,
    /// Preferred painted height. Use 120 for compact or 200 for regular.
    pub height: f32,
    /// Space assigned to the field, including its label and mode button.
    pub available_width: f32,
    pub available_height: f32,
    /// The application's effective text scale.
    pub text_scale: f32,
    pub direction: LayoutDirection,
    pub disabled: bool,
    pub hint: Option<&'a str>,
    pub use_input: &'a str,
    pub use_scroll: &'a str,
    pub error_text: &'a dyn Fn(ScrollFieldError) -> String,
    /// Accessible value text, which can include a localized unit.
    pub value_text: &'a dyn Fn(i32) -> String,
}

/// Geometry in logical pixels for a field that can show Scroll mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollFieldLayout {
    pub width: f32,
    pub face_height: f32,
    pub target_height: f32,
    pub row_height: f32,
    mode_below: bool,
}

impl ScrollFieldLayout {
    fn slot_width(self) -> f32 {
        self.width + 2. * FOCUS_OUTSET
    }

    fn width_with_mode(self) -> f32 {
        if self.mode_below {
            self.slot_width()
        } else {
            self.slot_width() + MODE_GAP + TARGET
        }
    }
}

/// Plan Scroll mode. If it does not fit, render the direct editor instead.
///
/// Pass the result to [ScrollFieldState::update_fit] when the allocation,
/// number format, or text scale changes. The direct editor keeps its draft
/// and remains usable even when the wheel or its complete targets cannot fit.
pub fn scroll_field_layout(
    options: ScrollFieldOptions<'_>,
    state: &ScrollFieldState,
) -> Option<ScrollFieldLayout> {
    if !options.available_width.is_finite() || !options.available_height.is_finite() {
        return None;
    }
    let scale = scale(options.text_scale);
    let wanted_height = if options.height.is_finite() {
        options.height.max(MIN_FACE_HEIGHT)
    } else {
        DEFAULT_FACE_HEIGHT
    };
    let width = preferred_width(options.size, state, scale);
    let slot_width = width + 2. * FOCUS_OUTSET;
    let mode_below = slot_width + MODE_GAP + TARGET > options.available_width;
    let label_height = copy_height(options.label, typography::LABEL_LARGE, scale);
    let hint_height = options
        .hint
        .map_or(0., |hint| copy_height(hint, typography::BODY_SMALL, scale));
    let mode_height = if mode_below { MODE_GAP + TARGET } else { 0. };
    let room_for_face =
        options.available_height - label_height - mode_height - hint_height - 2. * OUTSIDE_TARGET;
    let minimum_height = MIN_FACE_HEIGHT + 104. * (scale - 1.);
    if slot_width > options.available_width || room_for_face < minimum_height {
        return None;
    }
    let face_height = wanted_height.clamp(minimum_height, room_for_face);
    // Numeric ink is about 0.72 em tall. Measure the item gap between
    // the visible numbers. Line boxes include unused ascent and descent.
    let ink_separation = 0.36 * (CENTER.font_size + NEIGHBOR.font_size) * scale;
    let row_height = (ink_separation + ITEM_GAP).max(face_height / 2. - 8. * scale);
    Some(ScrollFieldLayout {
        width,
        face_height,
        target_height: face_height + 2. * OUTSIDE_TARGET,
        row_height,
        mode_below,
    })
}

fn copy_height(label: &str, style: TextStyle, scale: f32) -> f32 {
    if label.is_empty() {
        0.
    } else {
        style.line_height_px() * scale + COPY_GAP
    }
}

fn scale(scale: f32) -> f32 {
    if scale.is_finite() {
        scale.max(1.)
    } else {
        1.
    }
}

fn preferred_width(size: ScrollFieldSize, state: &ScrollFieldState, scale: f32) -> f32 {
    // Plan a stable width before the first frame using the longest possible
    // value, with room for full-width digits. The paint group uses the font's
    // actual advances to center each label and avoid text wrapping.
    let range = state.range();
    let ems = [range.min(), range.max()]
        .map(|bound| {
            state
                .format_value(bound)
                .chars()
                .map(|digit| if digit.is_ascii() { 0.595 } else { 1. })
                .sum::<f32>()
        })
        .into_iter()
        .fold(0., f32::max);
    (ems * CENTER.font_size * scale + 2. * size.padding())
        .ceil()
        .max(size.min_width())
}

impl Theme {
    /// Build one caller-owned finite numeric Scroll Field.
    ///
    /// Apply each event to the retained state and forward a changed committed
    /// value to the rest of the app. When space or text scale changes, pass
    /// [scroll_field_layout]'s fit to the state before rendering. Advance the
    /// state with the application's frame clock during a snap.
    ///
    /// The greater value is above the center. Up, Increment, and dragging the
    /// greater value down all increase the number. Wheel events continue to
    /// the surrounding page. The mode button or Enter/Space opens the editor.
    pub fn scroll_field<M: Clone + 'static>(
        &self,
        options: ScrollFieldOptions<'_>,
        state: &ScrollFieldState,
        on_event: impl Fn(ScrollFieldEvent) -> M + 'static,
    ) -> View<M> {
        ScrollField {
            theme: self,
            options,
            state,
            emit: Rc::new(on_event),
        }
        .view()
    }
}

struct ScrollField<'a, 'b, M> {
    theme: &'a Theme,
    options: ScrollFieldOptions<'b>,
    state: &'a ScrollFieldState,
    emit: Rc<dyn Fn(ScrollFieldEvent) -> M>,
}

impl<M: Clone + 'static> ScrollField<'_, '_, M> {
    fn id(&self, part: &str) -> String {
        format!("{}-{part}", self.options.id)
    }

    fn view(&self) -> View<M> {
        let fit = scroll_field_layout(self.options, self.state);
        let editing = self.state.mode() == ScrollFieldMode::Input || fit.is_none();
        let status = self.status();
        let status_id = status.as_ref().map(|(id, _, _)| id.as_str());
        let control = match fit.filter(|_| !editing) {
            Some(layout) => self.adjustable(layout, status_id),
            None => self.editor(fit, status_id),
        };
        let body = if let Some(layout) = fit {
            let axis = if layout.mode_below {
                Style::column()
            } else {
                Style::row()
            };
            let row = Node::new(Role::Group, "").styled(axis.align(Align::Center).gap(MODE_GAP));
            let row =
                if !layout.mode_below && self.options.direction == LayoutDirection::RightToLeft {
                    row.reverse_row()
                } else {
                    row
                };
            View::new(row).children([control, self.mode_button(editing)])
        } else {
            control
        };
        let width = fit.map_or_else(
            || {
                let (width, outset) = self.compact_width();
                width + 2. * outset
            },
            ScrollFieldLayout::width_with_mode,
        );
        let label = (!self.options.label.is_empty()).then(|| {
            text(
                self.id("label"),
                self.options.label,
                typography::LABEL_LARGE,
                if self.options.disabled {
                    disabled_ink(self.theme)
                } else {
                    self.theme.colors.on_surface
                },
            )
            .without_interaction(None)
        });
        View::new(
            Node::new(Role::Group, "")
                .id(self.options.id)
                .styled(Style::column().width(Length::Px(width)).gap(COPY_GAP)),
        )
        .children(label.into_iter().chain([body]).chain(
            status.map(|(id, message, ink)| text(id, &message, typography::BODY_SMALL, ink)),
        ))
    }

    fn status(&self) -> Option<(String, String, Color)> {
        if let Some(error) = self.state.error() {
            let message = (self.options.error_text)(error);
            return Some((self.id("error"), message, self.theme.colors.error));
        }
        let hint = self.options.hint?;
        let ink = if self.options.disabled {
            disabled_ink(self.theme)
        } else {
            self.theme.colors.on_surface_variant
        };
        Some((self.id("hint"), hint.to_owned(), ink))
    }

    fn adjustable(&self, layout: ScrollFieldLayout, description: Option<&str>) -> View<M> {
        let id = self.id("adjustable");
        let editor = self.id("editor");
        let range = self.state.range();
        let node = self.describe(
            Node::new(Role::Group, self.options.label)
                .id(&id)
                .adjustable(AdjustableValue {
                    min: f64::from(range.min()),
                    max: f64::from(range.max()),
                    step: f64::from(range.step()),
                    value: Some(f64::from(self.state.value())),
                    value_text: Some((self.options.value_text)(self.state.value())),
                })
                .disabled(self.options.disabled)
                .focus_color(Color::TRANSPARENT)
                .styled(fixed(layout.slot_width(), layout.target_height)),
            description,
        );
        let target = |suffix, y| {
            self.passive(
                self.id(suffix),
                at(fixed(layout.width, TARGET), FOCUS_OUTSET, y),
            )
        };
        let field = View::new(node)
            .children([
                self.scroll_face(layout),
                self.focus_ring(
                    layout.width,
                    layout.face_height,
                    FOCUS_OUTSET,
                    OUTSIDE_TARGET,
                ),
                target("above-target", 0.),
                target("below-target", layout.target_height - TARGET),
            ])
            .restore_focus_on_removal(&editor);
        if self.options.disabled {
            return field;
        }

        let session = self.state.drag_session();
        let value = self.state.value();
        let drag = self.emit.clone();
        let steps = self.emit.clone();
        let keys = self.emit.clone();
        let set = self.emit.clone();
        field
            .on_press((self.emit)(ScrollFieldEvent::Edit))
            .accessible_activation()
            .suppress_repeated_activation()
            .protect_composition_on_press()
            .focus_on_pointer_release()
            .accessibility_scroll_into_view()
            .on_key_focus_navigation(move |press| {
                (press.modifiers.is_empty() && matches!(press.key, Key::Enter | Key::Space)).then(
                    || FocusNavigation {
                        target: editor.clone(),
                        message: keys(ScrollFieldEvent::Edit),
                    },
                )
            })
            .on_focus_navigation_with_extend(move |key, extend| {
                if extend {
                    return None;
                }
                let step = match key {
                    EditKey::Up => ScrollFieldStep::Increase,
                    EditKey::Down => ScrollFieldStep::Decrease,
                    EditKey::Home => ScrollFieldStep::First,
                    EditKey::End => ScrollFieldStep::Last,
                    _ => return None,
                };
                Some(FocusNavigation {
                    target: id.clone(),
                    message: steps(ScrollFieldEvent::Step(step)),
                })
            })
            .on_value(move |value| set(ScrollFieldEvent::SetValue(value as i32)))
            .on_point_drag_with_bounds(session.clone(), move |point, bounds| {
                let sample = match point {
                    PointDrag::Tap {
                        position: (x, y), ..
                    } => {
                        if x < bounds.x + FOCUS_OUTSET
                            || x >= bounds.x + bounds.width - FOCUS_OUTSET
                        {
                            return None;
                        }
                        let target = if y < bounds.y + TARGET {
                            value.checked_add(range.step())?
                        } else if y >= bounds.y + bounds.height - TARGET {
                            value.checked_sub(range.step())?
                        } else {
                            return Some(drag(ScrollFieldEvent::Edit));
                        };
                        if !range.contains(target) {
                            return None;
                        }
                        ScrollFieldDrag::Tap { value: target }
                    }
                    PointDrag::Move { origin, position } | PointDrag::End { origin, position } => {
                        let delta_y = position.1 - origin.1;
                        if delta_y.abs() < (position.0 - origin.0).abs() {
                            ScrollFieldDrag::Cancel
                        } else if matches!(point, PointDrag::Move { .. }) {
                            ScrollFieldDrag::Move { delta_y }
                        } else {
                            ScrollFieldDrag::End { delta_y }
                        }
                    }
                    PointDrag::Cancel => ScrollFieldDrag::Cancel,
                    _ => return None,
                };
                Some(drag(ScrollFieldEvent::Drag {
                    session: session.clone(),
                    sample,
                    row_height: layout.row_height,
                }))
            })
            .cursor(CursorStyle::Pointer)
    }

    fn scroll_face(&self, layout: ScrollFieldLayout) -> View<M> {
        let face = at(
            fixed(layout.width, layout.face_height),
            FOCUS_OUTSET,
            OUTSIDE_TARGET,
        );
        let range = self.state.range();
        let minimum = i64::from(range.min());
        let step = i64::from(range.step());
        let preview = (i64::from(self.state.preview()) - minimum) / step;
        let offset = self.state.offset();
        let colors = self.theme.colors;
        let positions = (-2..=2).filter_map(|relative| {
            let index = preview + i64::from(relative);
            let value = range.value_at(index)?;
            let (suffix, role, ink) = match relative {
                0 => ("center", CENTER, colors.on_surface),
                1 => ("above", NEIGHBOR, colors.outline),
                -1 => ("below", NEIGHBOR, colors.outline),
                _ => ("", NEIGHBOR, colors.outline),
            };
            let ink = if self.options.disabled {
                alpha(ink, 97)
            } else {
                ink
            };
            let node = Node::new(Role::Text, "")
                .text(self.state.format_value(value))
                .styled(Style::column().foreground(ink).with_text_style(role));
            let node = if suffix.is_empty() {
                node
            } else {
                node.id(self.id(suffix))
            };
            Some((
                ((offset - index as f64) * f64::from(layout.row_height)) as f32,
                node,
            ))
        });
        let (centers, labels): (Vec<_>, Vec<_>) = positions.unzip();
        let paint = View::new(
            Node::new(Role::Group, "")
                .measured_group(DigitLayout { centers })
                .styled(fixed(layout.width, layout.face_height).clip(true)),
        )
        .children(labels.into_iter().map(View::new))
        .without_interaction(None);
        // The anonymous clip crops the glyphs. The named interaction owner
        // and markers remain outside that clip so a page wheel can reach its
        // surrounding scroller.
        self.passive(self.id("face"), self.face_style(face))
            .children([paint])
    }

    fn compact_width(&self) -> (f32, f32) {
        let available = if self.options.available_width.is_finite() {
            self.options.available_width.max(0.)
        } else {
            0.
        };
        let outset = ((available - TARGET) / 2.).clamp(0., FOCUS_OUTSET);
        let width = preferred_width(
            self.options.size,
            self.state,
            scale(self.options.text_scale),
        )
        .min(available - 2. * outset);
        (width, outset)
    }

    fn editor(&self, fit: Option<ScrollFieldLayout>, description: Option<&str>) -> View<M> {
        let id = self.id("editor");
        let font = if fit.is_some() {
            CENTER
        } else {
            typography::TITLE_LARGE
        };
        let (width, outset) = fit.map_or_else(
            || self.compact_width(),
            |layout| (layout.width, FOCUS_OUTSET),
        );
        let padding = self
            .options
            .size
            .padding()
            .min((width - TARGET).max(0.) / 2.);
        let line = font.line_height_px() * scale(self.options.text_scale);
        let editor_height = TARGET.max(line + 2. * padding);
        let face_height = fit.map_or(editor_height, |layout| {
            layout.face_height.max(editor_height)
        });
        let slot_height = fit.map_or(face_height + 2. * outset, |layout| layout.target_height);
        let top = (slot_height - face_height) / 2.;
        let colors = self.theme.colors;
        let error = self.state.error().is_some();
        let ink = if self.options.disabled {
            disabled_ink(self.theme)
        } else if error {
            colors.error
        } else {
            colors.primary
        };
        let face = self.passive(
            self.id("face"),
            self.face_style(at(fixed(width, face_height), outset, top))
                .border(2., ink),
        );
        let raw = if self.state.mode() == ScrollFieldMode::Input {
            self.state.raw().to_owned()
        } else {
            self.state.format_value(self.state.value())
        };
        let node = self.describe(
            Node::new(Role::TextInput, self.options.label)
                .id(&id)
                .text(raw)
                .focus_color(Color::TRANSPARENT)
                .disabled(self.options.disabled)
                .invalid(error)
                .styled(
                    at(
                        fixed(width, editor_height),
                        outset,
                        (slot_height - editor_height) / 2.,
                    )
                    .padding(padding)
                    .foreground(ink)
                    .with_text_style(font),
                ),
            description,
        );
        let mut editor = View::new(node).children([self.focus_ring(
            width,
            face_height,
            0.,
            -(face_height - editor_height) / 2.,
        )]);
        if !self.options.disabled {
            let modify = self.emit.clone();
            let actions = self.emit.clone();
            editor = editor
                .on_input(move |text| modify(ScrollFieldEvent::SetText(text)))
                .on_key(move |key| {
                    if !key.modifiers.is_empty() || key.repeat {
                        return None;
                    }
                    match key.key {
                        Key::Enter => Some(actions(ScrollFieldEvent::Submit)),
                        Key::Escape => Some(actions(ScrollFieldEvent::Cancel)),
                        _ => None,
                    }
                })
                .on_cancel_within((self.emit)(ScrollFieldEvent::Cancel))
                .accessibility_scroll_into_view()
                .cursor(CursorStyle::Text)
                .restore_focus_on_removal(self.id("adjustable"));
        }
        View::new(Node::new(Role::Group, "").styled(fixed(width + 2. * outset, slot_height)))
            .children([face, editor])
    }

    fn mode_button(&self, editing: bool) -> View<M> {
        let (part, label, action, fallback, icon) = if editing {
            (
                "use-scroll",
                self.options.use_scroll,
                ScrollFieldEvent::Cancel,
                "adjustable",
                scroll_icon(),
            )
        } else {
            (
                "use-input",
                self.options.use_input,
                ScrollFieldEvent::Edit,
                "editor",
                keyboard_icon(),
            )
        };
        let colors = self.theme.colors;
        let ink = if self.options.disabled {
            disabled_ink(self.theme)
        } else {
            colors.on_surface_variant
        };
        button(
            Node::new(Role::Button, label)
                .id(self.id(part))
                .focus_color(colors.secondary)
                .styled(
                    fixed(TARGET, TARGET)
                        .align(Align::Center)
                        .justify(Align::Center)
                        .corners(shape::FULL)
                        .state_layer(ink),
                ),
            (!self.options.disabled).then(|| (self.emit)(action)),
        )
        .children([icon
            .view(VectorIconSize::Dp24, ink)
            .without_interaction(None)])
        .restore_focus_on_removal(self.id(fallback))
        .protect_composition_on_press()
        .suppress_repeated_activation()
    }

    fn face_style(&self, style: Style) -> Style {
        let colors = self.theme.colors;
        style
            .corners(shape::LARGE)
            .background(if self.options.disabled {
                alpha(colors.surface_container_lowest, 97)
            } else {
                colors.surface_container_lowest
            })
            .state_layer(if self.options.disabled {
                Color::TRANSPARENT
            } else {
                colors.on_surface
            })
    }

    fn passive(&self, id: String, style: Style) -> View<M> {
        View::new(
            Node::new(Role::Group, "")
                .id(id)
                .styled(style)
                .state_layer_alphas(20, 26, 26),
        )
        .without_interaction(None)
    }

    fn focus_ring(&self, width: f32, height: f32, x: f32, y: f32) -> View<M> {
        View::new(
            Node::new(Role::Group, "")
                .id(self.id("focus"))
                .styled(
                    at(
                        fixed(width + 2. * FOCUS_OUTSET, height + 2. * FOCUS_OUTSET),
                        x - FOCUS_OUTSET,
                        y - FOCUS_OUTSET,
                    )
                    .corners(Corners::all(shape::LARGE.top_left + FOCUS_OUTSET)),
                )
                .focus_color(if self.options.disabled {
                    Color::TRANSPARENT
                } else {
                    self.theme.colors.secondary
                })
                .focus_outline_from_parent()
                .focus_outline_width(FOCUS_WIDTH),
        )
        .without_interaction(None)
    }

    fn describe(&self, node: Node, description: Option<&str>) -> Node {
        match description {
            Some(id) => node.described_by_text(id),
            None => node,
        }
    }
}

/// Keep the line widths from the actual font. The face clips only the
/// neighboring ink when those lines move partly outside the panel.
#[derive(Debug)]
struct DigitLayout {
    centers: Vec<f32>,
}

impl MeasuredGroupLayout for DigitLayout {
    fn measure(
        &self,
        _children: &[&Node],
        _available_width: Option<f32>,
        _fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        (0., 0.)
    }

    fn place(
        &self,
        children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        children
            .iter()
            .zip(&self.centers)
            .map(|(node, center)| {
                let style = node.style().text_style();
                let (text_width, _) =
                    fonts.measure_text(node.text_value().unwrap_or_default(), &style, None);
                let text_width = text_width.ceil() + 1.;
                let text_height = style.line_height_px();
                Rect::new(
                    (width - text_width) / 2.,
                    (height - text_height) / 2. + center,
                    text_width,
                    text_height,
                )
            })
            .collect()
    }
}

fn fixed(width: f32, height: f32) -> Style {
    Style::column()
        .width(Length::Px(width))
        .height(Length::Px(height))
}

fn at(style: Style, x: f32, y: f32) -> Style {
    style.absolute(Insets {
        left: Length::Px(x),
        top: Length::Px(y),
        ..Default::default()
    })
}

fn alpha(Color(r, g, b, _): Color, opacity: u8) -> Color {
    Color(r, g, b, opacity)
}

fn scroll_icon() -> VectorIcon {
    VectorIcon::from_contours([
        [
            [7., 8.],
            [12., 3.],
            [17., 8.],
            [15.5, 9.5],
            [12., 6.],
            [8.5, 9.5],
        ],
        [
            [7., 16.],
            [8.5, 14.5],
            [12., 18.],
            [15.5, 14.5],
            [17., 16.],
            [12., 21.],
        ],
    ])
    .expect("valid scroll mode icon")
}
