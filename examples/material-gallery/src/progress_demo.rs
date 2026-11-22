use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    shape, shape::LayoutDirection, typography, ButtonKind, ChoiceState, ProgressIndicatorOptions,
    ProgressShape, ProgressState, Theme,
};

use crate::Message;

pub(crate) const SECTION: usize = 1;
const SHOW: &str = "progress-demo-show";
const BACK: &str = "progress-demo-back";
const PAGE_PADDING: f32 = 12.;
const CARD_PADDING: f32 = 16.;
const CARD_GAP: f32 = 12.;

#[derive(Clone, Copy)]
pub enum ProgressEvent {
    Show,
    Back,
    Value(u8),
    Known(bool),
    Direction,
    Motion,
    TextScale,
}

pub(crate) struct ProgressDemo {
    visible: bool,
    known: bool,
    value: u8,
    rtl: bool,
    large_text: bool,
    width: f32,
    running: bool,
    reduced_motion: bool,
    epoch_ms: Option<f64>,
    elapsed_ms: f64,
}

impl Default for ProgressDemo {
    fn default() -> Self {
        Self {
            visible: false,
            known: true,
            value: 25,
            rtl: false,
            large_text: false,
            width: 960.,
            running: false,
            reduced_motion: false,
            epoch_ms: None,
            elapsed_ms: 0.,
        }
    }
}

impl ProgressDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.running = false;
        self.epoch_ms = None;
        self.elapsed_ms = 0.;
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn update(&mut self, event: ProgressEvent) {
        match event {
            ProgressEvent::Show => self.visible = true,
            ProgressEvent::Back => self.leave(),
            ProgressEvent::Value(value) => {
                self.value = value.min(100);
                self.known = true;
            }
            ProgressEvent::Known(known) => {
                self.known = known;
                self.epoch_ms = None;
                self.elapsed_ms = 0.;
            }
            ProgressEvent::Direction => self.rtl = !self.rtl,
            ProgressEvent::Motion => {
                self.running = !self.running;
                self.epoch_ms = None;
                if self.running {
                    self.elapsed_ms = 0.;
                }
            }
            ProgressEvent::TextScale => self.large_text = !self.large_text,
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        if !width.is_finite() || self.width == width {
            return false;
        }
        self.width = width.max(0.);
        true
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        let policy_changed = self.reduced_motion != frame.reduced_motion;
        self.reduced_motion = frame.reduced_motion;
        if !self.visible {
            return AnimationUpdate::default();
        }
        if self.reduced_motion {
            self.epoch_ms = None;
            self.elapsed_ms = 0.;
            return AnimationUpdate {
                rebuild: policy_changed,
                request_next: false,
            };
        }
        if !self.running {
            return AnimationUpdate {
                rebuild: policy_changed,
                request_next: false,
            };
        }
        let Some(now) = frame.now_ms.filter(|time| time.is_finite()) else {
            return AnimationUpdate {
                rebuild: policy_changed,
                request_next: true,
            };
        };
        let epoch = self.epoch_ms.get_or_insert(now);
        let elapsed = (now - *epoch).max(0.);
        let rebuild = elapsed != self.elapsed_ms || policy_changed;
        self.elapsed_ms = elapsed;
        AnimationUpdate {
            rebuild,
            request_next: true,
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "進度指示器",
                ButtonKind::Outlined,
                false,
                Message::Progress(ProgressEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let change = |id, label, selected, event| {
            theme
                .segmented_button(id, label, choice(selected), Message::Progress(event))
                .accessible_activation()
        };
        let options = theme.row([
            theme
                .tonal_button(BACK, "返回", Message::Progress(ProgressEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme.switch("theme", "深色", choice(dark), Message::Theme),
            change("progress-rtl", "右至左", self.rtl, ProgressEvent::Direction),
            change(
                "progress-scale",
                "大字",
                self.large_text,
                ProgressEvent::TextScale,
            ),
            change(
                "progress-motion",
                "動態",
                self.running,
                ProgressEvent::Motion,
            ),
        ]);
        let mode = [
            change(
                "progress-mode-known",
                "已知",
                self.known,
                ProgressEvent::Known(true),
            ),
            change(
                "progress-mode-unknown",
                "未知",
                !self.known,
                ProgressEvent::Known(false),
            ),
        ];
        let values = [
            ("progress-value-0", "0%", 0),
            ("progress-value-25", "25%", 25),
            ("progress-value-75", "75%", 75),
            ("progress-value-100", "100%", 100),
        ]
        .map(|(id, label, value)| {
            change(
                id,
                label,
                self.known && self.value == value,
                ProgressEvent::Value(value),
            )
        });
        let progress_controls = theme.row(mode.into_iter().chain(values));
        let samples = View::new(
            Node::new(Role::Group, "")
                .styled(Style::row().gap(CARD_GAP).align(Align::Start).wrap(true)),
        )
        .children([self.linear_card(theme), self.circular_card(theme)]);
        View::new(
            Node::new(Role::Window, "進度指示器")
                .id("surface-page")
                .styled(
                    Style::column()
                        .padding(PAGE_PADDING)
                        .gap(8.)
                        .background(theme.colors.surface)
                        .foreground(theme.colors.on_surface)
                        .clip(true),
                ),
        )
        .children([options, progress_controls, samples])
        .accessibility_scroll_pages()
    }

    fn card_width(&self) -> f32 {
        let available = (self.width - 2. * PAGE_PADDING).max(0.);
        if available >= 570. {
            ((available - CARD_GAP) / 2.).min(420.).floor()
        } else {
            available
        }
    }

    fn linear_width(&self) -> f32 {
        (self.card_width() - 2. * CARD_PADDING).clamp(0., 240.)
    }

    fn options(
        &self,
        id: &'static str,
        name: &'static str,
        shape: ProgressShape,
    ) -> ProgressIndicatorOptions<'static> {
        ProgressIndicatorOptions {
            id,
            name,
            state: if self.known {
                ProgressState::Determinate(f64::from(self.value) / 100.)
            } else {
                ProgressState::Indeterminate
            },
            shape,
            elapsed_ms: self.elapsed_ms,
            reduced_motion: self.reduced_motion || (!self.running && self.elapsed_ms == 0.),
        }
    }

    fn linear_card(&self, theme: &Theme) -> View<Message> {
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let width = self.linear_width();
        let flat = self.options("progress-linear-flat", "線形進度", ProgressShape::Flat);
        let wavy = self.options("progress-linear-wavy", "波形線形進度", ProgressShape::Wavy);
        self.card(
            theme,
            "progress-linear-card",
            "線形",
            [
                theme.text("progress-linear-flat-label", "平直"),
                theme.linear_progress_indicator(flat, width, direction),
                theme.text("progress-linear-wavy-label", "波形"),
                theme.linear_progress_indicator(wavy, width, direction),
            ],
        )
    }

    fn circular_card(&self, theme: &Theme) -> View<Message> {
        let flat = self.options("progress-circular-flat", "圓形進度", ProgressShape::Flat);
        let wavy = self.options(
            "progress-circular-wavy",
            "波形圓形進度",
            ProgressShape::Wavy,
        );
        let sample = |id, label, options| {
            View::new(
                Node::new(Role::Group, "").styled(Style::column().gap(8.).align(Align::Start)),
            )
            .children([
                theme.text(id, label),
                theme.circular_progress_indicator(options),
            ])
        };
        self.card(
            theme,
            "progress-circular-card",
            "圓形",
            [theme.row([
                sample("progress-circular-flat-label", "平直", flat),
                sample("progress-circular-wavy-label", "波形", wavy),
            ])],
        )
    }

    fn card(
        &self,
        theme: &Theme,
        id: &str,
        label: &str,
        children: impl IntoIterator<Item = View<Message>>,
    ) -> View<Message> {
        let heading = View::new(
            Node::new(Role::Heading, label).text(label).styled(
                Style::default()
                    .with_text_style(typography::TITLE_MEDIUM)
                    .foreground(theme.colors.on_surface),
            ),
        );
        View::new(
            Node::new(Role::Group, "").id(id).styled(
                Style::column()
                    .width(Length::Px(self.card_width()))
                    .padding(CARD_PADDING)
                    .gap(8.)
                    .align(Align::Start)
                    .background(theme.colors.surface_container_low)
                    .corners(shape::MEDIUM),
            ),
        )
        .children(Some(heading).into_iter().chain(children))
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        serde_json::json!({
            "known": self.known,
            "value": f64::from(self.value) / 100.,
            "rtl": self.rtl,
            "running": self.running,
            "elapsed_ms": self.elapsed_ms,
            "linear_width": self.linear_width(),
            "reduced_motion": self.reduced_motion,
            "text_scale": self.text_scale(),
            "active_color": theme.primary,
            "track_color": theme.colors.secondary_container,
        })
    }
}

fn choice(selected: bool) -> ChoiceState {
    ChoiceState {
        selected,
        disabled: false,
    }
}
