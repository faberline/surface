//! The controlled Material 3 single-date modal picker.

mod calendar_view;

use std::rc::Rc;

use surface::{Align, Color, Insets, Length, Node, Rect, Role, Style, TextStyle};
use surface_app::{CursorStyle, Key, View};

use crate::{
    adaptive::WindowWidthSizeClass,
    shape::{self, LayoutDirection},
    typography,
    vector_icon::VectorIconSize,
    CivilDate, DatePickerConstraints, DatePickerEvent, DatePickerLocale, DatePickerMode,
    DatePickerSession, Theme, VectorIcon,
};

const DIALOG_WIDTH: f32 = 360.;
const DIALOG_HEIGHT: f32 = 524.;
const INPUT_WIDTH: f32 = 328.;
const INPUT_HEIGHT: f32 = 512.;
const HEADER_HEIGHT: f32 = 120.;
const STANDARD_INSET: f32 = 12.;
const TARGET: f32 = 48.;

/// The caller's preferred presentation of the single-date modal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DatePickerLayout {
    /// Use the full-screen presentation below 600 dp.
    #[default]
    Adaptive,
    /// Center the dialog within the available viewport.
    Dialog,
    /// Use the whole viewport and keep Close and Save at the top.
    FullScreen,
}

/// Localized names for the picker and its controls.
#[derive(Clone, Copy, Debug)]
pub struct DatePickerLabels<'a> {
    pub title: &'a str,
    pub input_name: &'a str,
    pub previous_month: &'a str,
    pub next_month: &'a str,
    pub choose_year: &'a str,
    pub edit_mode: &'a str,
    pub calendar_mode: &'a str,
    pub confirm: &'a str,
    pub cancel: &'a str,
    pub save: &'a str,
    pub close: &'a str,
    pub today: &'a str,
    pub selected: &'a str,
}

/// The data policy, locale and placement of a single-date modal.
#[derive(Clone, Copy)]
pub struct DatePickerOptions<'a> {
    pub id: &'a str,
    pub today: CivilDate,
    pub constraints: DatePickerConstraints<'a>,
    pub locale: &'a DatePickerLocale<'a>,
    pub labels: &'a DatePickerLabels<'a>,
    pub layout: DatePickerLayout,
    pub viewport: Rect,
    pub restore_focus_to: Option<&'a str>,
}

impl Theme {
    /// Mount while the caller has an open session. Forward events to that session.
    ///
    /// The input uses Surface's normal single-line editing and IME handling.
    /// To normalize text on field exit, dispatch [DatePickerEvent::FinishEditing]
    /// when focus leaves a retained input while this picker is open.
    ///
    /// The dialog uses the supplied ID. The editor, grid, mode switch and
    /// confirm/cancel controls append -input, -grid, -mode, -confirm and
    /// -cancel. Day and year controls append -day-YYYY-MM-DD and -year-YYYY.
    pub fn date_picker<M: Clone + 'static>(
        &self,
        options: DatePickerOptions<'_>,
        session: &DatePickerSession,
        on_event: impl Fn(DatePickerEvent) -> M + 'static,
    ) -> View<M> {
        let compact = WindowWidthSizeClass::from_dp(options.viewport.width)
            == Some(WindowWidthSizeClass::Compact);
        let fullscreen = options.layout == DatePickerLayout::FullScreen
            || (options.layout == DatePickerLayout::Adaptive && compact);
        let input = session.mode() == DatePickerMode::Input;
        let width = options.viewport.width.max(0.);
        let height = options.viewport.height.max(0.);
        let size = if fullscreen {
            (width, height)
        } else {
            let (target_width, target_height) = if input {
                (INPUT_WIDTH, INPUT_HEIGHT)
            } else {
                (DIALOG_WIDTH, DIALOG_HEIGHT)
            };
            (
                target_width.min(available(width)),
                target_height.min(available(height)),
            )
        };
        Picker {
            theme: self,
            options,
            session,
            emit: Rc::new(on_event),
            width: size.0,
            height: size.1,
            fullscreen,
        }
        .view()
    }
}

fn available(extent: f32) -> f32 {
    // Preserve a usable panel even when an explicitly forced dialog is narrow.
    extent - 2. * STANDARD_INSET.min((extent - TARGET).max(0.) / 2.)
}

struct Picker<'a, 'b, M> {
    theme: &'a Theme,
    options: DatePickerOptions<'b>,
    session: &'a DatePickerSession,
    emit: Rc<dyn Fn(DatePickerEvent) -> M>,
    width: f32,
    height: f32,
    fullscreen: bool,
}

impl<M: Clone + 'static> Picker<'_, '_, M> {
    fn id(&self, part: &str) -> String {
        format!("{}-{part}", self.options.id)
    }

    fn rtl(&self) -> bool {
        self.options.locale.direction == LayoutDirection::RightToLeft
    }

    fn row(&self, node: Node) -> Node {
        if self.rtl() {
            node.reverse_row()
        } else {
            node
        }
    }

    fn view(&self) -> View<M> {
        let short = self.height < 360.;
        let headline_scrolls = self.fullscreen || short;
        let (body, initial_focus) = match self.session.mode() {
            DatePickerMode::Calendar => (
                self.calendar(),
                self.active_day()
                    .map(|date| self.id(&format!("day-{date}")))
                    .unwrap_or_else(|| self.id("month")),
            ),
            DatePickerMode::Year => (
                self.years(),
                self.id(&format!("year-{:04}", self.session.month().year())),
            ),
            DatePickerMode::Input => (self.input(), self.id("input")),
        };
        let mut body_contents = Vec::with_capacity(2);
        if headline_scrolls {
            body_contents.push(self.header(false));
        }
        body_contents.push(body);
        let body = View::new(
            Node::new(Role::Group, "").id(self.id("body")).styled(
                Style::column()
                    .width(Length::Percent(1.))
                    .height(Length::Px(0.))
                    .grow(1.)
                    .clip(true),
            ),
        )
        .children(body_contents)
        .accessibility_scroll_pages();
        let mut content = Vec::with_capacity(3);
        if self.fullscreen {
            content.push(self.actions(true));
            content.push(body);
        } else {
            if !short {
                content.push(self.header(true));
            }
            content.push(body);
            content.push(self.actions(short));
        }

        let radius = if self.fullscreen { 0. } else { 28. };
        let panel = View::new(
            Node::new(Role::Dialog, self.options.labels.title)
                .id(self.options.id)
                .tab_index(-1)
                .styled(
                    Style::column()
                        .width(Length::Px(self.width))
                        .height(Length::Px(self.height))
                        .corners(surface::Corners::all(radius))
                        .background(self.theme.colors.surface_container_high)
                        .clip(true),
                ),
        )
        .children(content)
        .modal_initial_focus(initial_focus);
        let panel = if let Some(fallback) = self.options.restore_focus_to {
            panel.modal_focus_fallback(fallback)
        } else {
            panel
        };
        let scrim = if self.fullscreen {
            Color::TRANSPARENT
        } else {
            let Color(r, g, b, _) = self.theme.colors.scrim;
            Color(r, g, b, 82)
        };
        let mut backdrop_style = Style::column()
            .fixed(Insets::all(0.))
            .align(Align::Center)
            .justify(Align::Center)
            .background(scrim);
        backdrop_style.z_index = 1000;
        panel.modal_with_backdrop(
            Node::new(Role::Group, "")
                .id(self.id("layer"))
                .styled(backdrop_style),
            (self.emit)(DatePickerEvent::Cancel),
        )
    }

    fn header(&self, include_switch: bool) -> View<M> {
        let colors = self.theme.colors;
        let label = self.options.labels;
        let title = self
            .session
            .draft()
            .map(|date| (self.options.locale.date_headline)(date))
            .unwrap_or_else(|| label.input_name.to_owned());
        let heading = Node::new(Role::Heading, &title)
            .id(self.id("headline"))
            .text(title)
            .styled(
                Style::column()
                    .width(Length::Px(0.))
                    .grow(1.)
                    .with_text_style(typography::HEADLINE_LARGE)
                    .foreground(colors.on_surface),
            );
        let mut row = vec![View::new(heading)];
        if include_switch {
            row.push(self.mode_button());
        }
        let headline = View::new(
            self.row(Node::new(Role::Group, "").styled(Style::row().gap(4.).align(Align::Center))),
        )
        .children(row);
        let mut style = Style::column().padding(STANDARD_INSET).gap(4.);
        if !self.fullscreen && include_switch {
            style.min_height = HEADER_HEIGHT;
            style = style.justify(Align::Center);
        }
        View::new(
            Node::new(Role::Group, "")
                .id(self.id("header"))
                .styled(style),
        )
        .children([
            text(
                self.id("title"),
                label.title,
                typography::LABEL_LARGE,
                colors.on_surface_variant,
            ),
            headline,
        ])
    }

    fn mode_button(&self) -> View<M> {
        let (label, next, icon) = if self.session.mode() == DatePickerMode::Input {
            (
                self.options.labels.calendar_mode,
                DatePickerMode::Calendar,
                Glyph::Calendar,
            )
        } else {
            (
                self.options.labels.edit_mode,
                DatePickerMode::Input,
                Glyph::Edit,
            )
        };
        self.icon_button(
            self.id("mode"),
            label,
            icon,
            Some(DatePickerEvent::SetMode(next)),
        )
        .protect_composition_on_press()
    }

    fn actions(&self, include_switch: bool) -> View<M> {
        let labels = self.options.labels;
        let cancel = if self.fullscreen {
            self.icon_button(
                self.id("cancel"),
                labels.close,
                Glyph::Close,
                Some(DatePickerEvent::Cancel),
            )
        } else {
            self.action_button(
                self.id("cancel"),
                labels.cancel,
                Some(DatePickerEvent::Cancel),
            )
        };
        let confirm = self
            .session
            .draft()
            .filter(|date| self.options.constraints.allows(*date))
            .map(|_| DatePickerEvent::Confirm);
        let confirm = self
            .action_button(
                self.id("confirm"),
                if self.fullscreen {
                    labels.save
                } else {
                    labels.confirm
                },
                confirm,
            )
            .protect_composition_on_press();
        let padding = if self.width >= 288. { 8. } else { 0. };
        let style = Style::row()
            .padding(padding)
            .align(Align::Center)
            .wrap(true);
        let mut children = vec![cancel, confirm];
        if include_switch {
            let spacer = View::new(Node::new(Role::Group, "").styled(Style::column().grow(1.)));
            if self.fullscreen {
                // The controls keep their logical order under RTL mirroring.
                children.insert(1, spacer);
                children.insert(2, self.mode_button());
            } else {
                children.insert(0, spacer);
                children.insert(0, self.mode_button());
            }
        }
        let style = if include_switch {
            style
        } else {
            style.justify(Align::End)
        };
        View::new(
            self.row(
                Node::new(Role::Group, "")
                    .id(self.id("footer"))
                    .styled(style),
            ),
        )
        .children(children)
    }

    fn action_button(&self, id: String, label: &str, event: Option<DatePickerEvent>) -> View<M> {
        let ink = if event.is_some() {
            self.theme.colors.primary
        } else {
            self.disabled_ink()
        };
        let mut control = Style::row()
            .padding(10.)
            .align(Align::Center)
            .justify(Align::Center)
            .corners(shape::FULL)
            .state_layer(ink);
        control.min_width = TARGET;
        control.min_height = TARGET;
        control.max_width = Length::Px((self.width - 16.).max(0.));
        let label_view = text("", label, typography::LABEL_LARGE, ink).without_interaction(None);
        button(
            Node::new(Role::Button, label)
                .id(id)
                .focus_color(self.theme.colors.primary)
                .styled(control),
            event.map(|event| (self.emit)(event)),
        )
        .children([label_view])
    }

    fn icon_button(
        &self,
        id: String,
        label: &str,
        icon: Glyph,
        event: Option<DatePickerEvent>,
    ) -> View<M> {
        let ink = if event.is_some() {
            self.theme.colors.on_surface_variant
        } else {
            self.disabled_ink()
        };
        let paint = View::new(
            Node::new(Role::Group, "")
                .styled(centered(40., 40.).corners(shape::FULL).state_layer(ink)),
        )
        .children([icon.vector().view(VectorIconSize::Dp24, ink)])
        .without_interaction(None);
        button(
            Node::new(Role::Button, label)
                .id(id)
                .focus_color(self.theme.colors.primary)
                .styled(centered(TARGET, TARGET).corners(shape::FULL)),
            event.map(|event| (self.emit)(event)),
        )
        .children([paint])
    }

    fn disabled_ink(&self) -> Color {
        let Color(r, g, b, _) = self.theme.colors.on_surface;
        Color(r, g, b, 97)
    }

    fn input(&self) -> View<M> {
        let id = self.id("input");
        let locale = self.options.locale;
        let error = self.session.error();
        let (hint_id, hint, ink) = if let Some(error) = error {
            (
                self.id("error"),
                format!("{}\n{}", (locale.error_text)(error), locale.format_hint),
                self.theme.colors.error,
            )
        } else {
            (
                self.id("hint"),
                locale.format_hint.to_owned(),
                self.theme.colors.on_surface_variant,
            )
        };
        let label = self.options.labels.input_name;
        let mut style = Style::column()
            .width(Length::Percent(1.))
            .padding(12.)
            .corners(shape::EXTRA_SMALL)
            .border(
                1.,
                if error.is_some() {
                    ink
                } else {
                    self.theme.colors.outline
                },
            )
            .with_text_style(typography::BODY_LARGE)
            .foreground(self.theme.colors.on_surface);
        style.min_height = 56.;
        let edit = self.emit.clone();
        let confirm = (self.emit)(DatePickerEvent::Confirm);
        let input = View::new(
            Node::new(Role::TextInput, label)
                .id(id)
                .text(self.session.raw_input())
                .described_by_text(&hint_id)
                .styled(style),
        )
        .on_input(move |raw| edit(DatePickerEvent::Edit(raw)))
        .on_key(move |key| {
            (key.key == Key::Enter && key.modifiers.is_empty() && !key.repeat)
                .then(|| confirm.clone())
        })
        .cursor(CursorStyle::Text)
        .restore_focus_on_removal(self.id("mode"))
        .accessibility_scroll_into_view();
        View::new(Node::new(Role::Group, "").styled(Style::column().padding(12.).gap(6.))).children(
            [
                text(
                    "",
                    label,
                    typography::LABEL_LARGE,
                    self.theme.colors.on_surface,
                ),
                input,
                text(hint_id, &hint, typography::BODY_SMALL, ink),
            ],
        )
    }
}

fn button<M>(node: Node, message: Option<M>) -> View<M> {
    let view = View::new(node.disabled(message.is_none()));
    if let Some(message) = message {
        view.on_press(message)
            .accessible_activation()
            .cursor(CursorStyle::Pointer)
    } else {
        view
    }
}

fn text<M>(id: impl Into<String>, label: &str, style: TextStyle, color: Color) -> View<M> {
    let id = id.into();
    let node = Node::new(Role::Text, label)
        .text(label)
        .shrink_to_fit_width()
        .styled(Style::column().with_text_style(style).foreground(color));
    View::new(if id.is_empty() { node } else { node.id(id) })
}

fn centered(width: f32, height: f32) -> Style {
    Style::row()
        .width(Length::Px(width))
        .height(Length::Px(height))
        .align(Align::Center)
        .justify(Align::Center)
}

#[derive(Clone, Copy)]
enum Glyph {
    Left,
    Right,
    Down,
    Up,
    Edit,
    Calendar,
    Close,
}

impl Glyph {
    fn vector(self) -> VectorIcon {
        let arrow = [
            [14.8, 5.],
            [8., 12.],
            [14.8, 19.],
            [16.2, 17.6],
            [10.8, 12.],
            [16.2, 6.4],
        ];
        let rectangle =
            |x: f32, y: f32, w: f32, h: f32| [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        match self {
            Self::Left => VectorIcon::new(arrow),
            Self::Right => VectorIcon::new(arrow.map(|[x, y]| [24. - x, y])),
            Self::Down => VectorIcon::new(arrow.map(|[x, y]| [y, 24. - x])),
            Self::Up => VectorIcon::new(arrow.map(|[x, y]| [y, x])),
            Self::Edit => VectorIcon::from_contours([
                vec![[4., 16.], [13.7, 6.3], [17.7, 10.3], [8., 20.], [4., 20.]],
                vec![[15., 5.], [17.5, 2.5], [21.5, 6.5], [19., 9.]],
            ]),
            Self::Calendar => VectorIcon::from_contours([
                rectangle(3., 5., 18., 16.).to_vec(),
                rectangle(5., 10., 14., 9.).to_vec(),
                rectangle(7., 3., 2., 2.).to_vec(),
                rectangle(15., 3., 2., 2.).to_vec(),
                rectangle(11., 12., 3., 3.).to_vec(),
            ]),
            Self::Close => VectorIcon::new([
                [6.4, 5.],
                [12., 10.6],
                [17.6, 5.],
                [19., 6.4],
                [13.4, 12.],
                [19., 17.6],
                [17.6, 19.],
                [12., 13.4],
                [6.4, 19.],
                [5., 17.6],
                [10.6, 12.],
                [5., 6.4],
            ]),
        }
        .expect("date picker has valid fixed vector icons")
    }
}
