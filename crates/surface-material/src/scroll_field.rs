//! Material 3 Expressive Scroll Fields.

mod choice;
pub use choice::{scroll_choice_field_layout, ScrollChoiceFieldOptions};

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

#[derive(Clone, Copy)]
struct FieldOptions<'a> {
    id: &'a str,
    label: &'a str,
    size: ScrollFieldSize,
    height: f32,
    available_width: f32,
    available_height: f32,
    text_scale: f32,
    direction: LayoutDirection,
    disabled: bool,
    hint: Option<&'a str>,
    use_input: &'a str,
    use_scroll: &'a str,
}

impl<'a> From<ScrollFieldOptions<'a>> for FieldOptions<'a> {
    fn from(options: ScrollFieldOptions<'a>) -> Self {
        Self {
            id: options.id,
            label: options.label,
            size: options.size,
            height: options.height,
            available_width: options.available_width,
            available_height: options.available_height,
            text_scale: options.text_scale,
            direction: options.direction,
            disabled: options.disabled,
            hint: options.hint,
            use_input: options.use_input,
            use_scroll: options.use_scroll,
        }
    }
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
    let width = numeric_width(options.size, state, scale(options.text_scale));
    field_layout(options.into(), width, false)
}

fn field_layout(
    options: FieldOptions<'_>,
    width: f32,
    text_choices: bool,
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
    // Measure the gap between visible labels. Ideographs and some other
    // choices can occupy more of the line than digits.
    let ink_fraction = if text_choices { 0.46 } else { 0.36 };
    let ink_separation = ink_fraction * (CENTER.font_size + NEIGHBOR.font_size) * scale;
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

fn numeric_width(size: ScrollFieldSize, state: &ScrollFieldState, scale: f32) -> f32 {
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
    width_for_ems(size, ems, scale)
}

fn width_for_ems(size: ScrollFieldSize, ems: f32, scale: f32) -> f32 {
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
            options: options.into(),
            wheel: state,
            copy: FieldCopy::Numeric {
                error_text: options.error_text,
                value_text: options.value_text,
            },
            preferred_width: numeric_width(options.size, state, scale(options.text_scale)),
            emit: Rc::new(on_event),
        }
        .view()
    }
}

struct ScrollField<'a, 'b, M> {
    theme: &'a Theme,
    options: FieldOptions<'b>,
    wheel: &'a ScrollFieldState,
    copy: FieldCopy<'a, 'b>,
    preferred_width: f32,
    emit: Rc<dyn Fn(ScrollFieldEvent) -> M>,
}

enum FieldCopy<'a, 'b> {
    Numeric {
        error_text: &'b dyn Fn(ScrollFieldError) -> String,
        value_text: &'b dyn Fn(i32) -> String,
    },
    Choice(choice::ChoiceCopy<'a, 'b>),
}

impl<M: Clone + 'static> ScrollField<'_, '_, M> {
    fn id(&self, part: &str) -> String {
        format!("{}-{part}", self.options.id)
    }

    fn view(&self) -> View<M> {
        let fit = field_layout(
            self.options,
            self.preferred_width,
            matches!(&self.copy, FieldCopy::Choice(_)),
        );
        let mode = match &self.copy {
            FieldCopy::Numeric { .. } => self.wheel.mode(),
            FieldCopy::Choice(copy) => copy.state.mode(),
        };
        let editing = mode == ScrollFieldMode::Input || fit.is_none();
        let status = self.status(editing);
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

    fn status(&self, editing: bool) -> Option<(String, String, Color)> {
        if let Some(message) = self.error_text() {
            return Some((self.id("error"), message, self.theme.colors.error));
        }
        if editing && !self.options.disabled {
            if let FieldCopy::Choice(copy) = &self.copy {
                if let Some(message) = copy.suggestion() {
                    return Some((
                        self.id("suggestion"),
                        message,
                        self.theme.colors.on_surface_variant,
                    ));
                }
            }
        }
        let hint = self.options.hint?;
        let ink = if self.options.disabled {
            disabled_ink(self.theme)
        } else {
            self.theme.colors.on_surface_variant
        };
        Some((self.id("hint"), hint.to_owned(), ink))
    }

    fn error_text(&self) -> Option<String> {
        match &self.copy {
            FieldCopy::Numeric { error_text, .. } => self.wheel.error().map(error_text),
            FieldCopy::Choice(copy) => copy.state.error().map(copy.error_text),
        }
    }

    fn adjustable(&self, layout: ScrollFieldLayout, description: Option<&str>) -> View<M> {
        let id = self.id("adjustable");
        let editor = self.id("editor");
        let range = self.wheel.range();
        let value = self.wheel.value();
        let (ordinal_offset, value_text) = match &self.copy {
            FieldCopy::Numeric { value_text, .. } => (0., value_text(value)),
            FieldCopy::Choice(copy) => (1., copy.qualified(copy.state.choice())),
        };
        let node = self.describe(
            Node::new(Role::Group, self.options.label)
                .id(&id)
                .adjustable(AdjustableValue {
                    min: f64::from(range.min()) + ordinal_offset,
                    max: f64::from(range.max()) + ordinal_offset,
                    step: f64::from(range.step()),
                    value: Some(f64::from(value) + ordinal_offset),
                    value_text: Some(value_text),
                })
                .disabled(self.options.disabled)
                .focus_color(Color::TRANSPARENT)
                .styled(fixed(layout.slot_width(), layout.target_height)),
            description,
        );
        let node = if matches!(&self.copy, FieldCopy::Choice(_)) {
            node.web_adjustable_value_description()
        } else {
            node
        };
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

        let session = self.wheel.drag_session();
        let looping = self.wheel.looping();
        let drag = self.emit.clone();
        let steps = self.emit.clone();
        let keys = self.emit.clone();
        let set = self.emit.clone();
        let field = field
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
            .on_value(move |value| set(ScrollFieldEvent::SetValue((value - ordinal_offset) as i32)))
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
                        let (step, target) = if y < bounds.y + TARGET {
                            (ScrollFieldStep::Increase, value.checked_add(range.step()))
                        } else if y >= bounds.y + bounds.height - TARGET {
                            (ScrollFieldStep::Decrease, value.checked_sub(range.step()))
                        } else {
                            return Some(drag(ScrollFieldEvent::Edit));
                        };
                        if looping {
                            return Some(drag(ScrollFieldEvent::Step(step)));
                        }
                        ScrollFieldDrag::Tap {
                            value: target.filter(|value| range.contains(*value))?,
                        }
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
            .cursor(CursorStyle::Pointer);
        if looping {
            let step = self.emit.clone();
            field.on_accessible_step(move |increment| {
                let direction = if increment {
                    ScrollFieldStep::Increase
                } else {
                    ScrollFieldStep::Decrease
                };
                step(ScrollFieldEvent::Step(direction))
            })
        } else {
            field
        }
    }

    fn scroll_face(&self, layout: ScrollFieldLayout) -> View<M> {
        let face = at(
            fixed(layout.width, layout.face_height),
            FOCUS_OUTSET,
            OUTSIDE_TARGET,
        );
        let offset = self.wheel.offset();
        let preview = offset.round() as i64;
        let colors = self.theme.colors;
        let positions = (-2..=2).filter_map(|relative| {
            let index = preview + i64::from(relative);
            let value = self.wheel.value_at(index)?;
            let label = match &self.copy {
                FieldCopy::Numeric { .. } => self.wheel.format_value(value),
                FieldCopy::Choice(copy) => copy.qualified(copy.state.get(value as usize)?),
            };
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
                .text(label)
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
                .measured_group(ValueLayout { centers })
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
        let width = self.preferred_width.min(available - 2. * outset);
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
        let error = match &self.copy {
            FieldCopy::Numeric { .. } => self.wheel.error().is_some(),
            FieldCopy::Choice(copy) => copy.state.error().is_some(),
        };
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
        let raw = match &self.copy {
            FieldCopy::Numeric { .. } if self.wheel.mode() == ScrollFieldMode::Input => {
                self.wheel.raw().to_owned()
            }
            FieldCopy::Numeric { .. } => self.wheel.format_value(self.wheel.value()),
            FieldCopy::Choice(copy) => copy.state.raw().to_owned(),
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
struct ValueLayout {
    centers: Vec<f32>,
}

impl MeasuredGroupLayout for ValueLayout {
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
