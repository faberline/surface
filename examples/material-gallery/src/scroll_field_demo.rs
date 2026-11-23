use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
#[cfg(target_arch = "wasm32")]
use surface_material::ScrollFieldMode;
use surface_material::{
    scroll_field_layout, shape::LayoutDirection, typography, ButtonKind, ChoiceState,
    ScrollFieldError, ScrollFieldEvent, ScrollFieldOptions, ScrollFieldRange, ScrollFieldSize,
    ScrollFieldState, Theme,
};

use crate::Message;

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "scroll-field-demo-show";
const BACK: &str = "scroll-field-demo-back";
const SCROLL: &str = "scroll-field-demo-scroll";
const FIELD: &str = "scroll-field-demo-value";
const PADDING: f32 = 12.;
const CONTENT_PADDING: f32 = 8.;
const SIZES: [(ScrollFieldSize, &str, &str); 3] = [
    (ScrollFieldSize::Small, "scroll-field-demo-small", "小"),
    (ScrollFieldSize::Medium, "scroll-field-demo-medium", "中"),
    (ScrollFieldSize::Large, "scroll-field-demo-large", "大"),
];

#[derive(Clone)]
pub enum ScrollFieldDemoEvent {
    Show,
    Back,
    Direction,
    LargeText,
    Size(ScrollFieldSize),
    Compact,
    Disabled,
    WideRange,
    Field(ScrollFieldEvent),
}

pub(crate) struct ScrollFieldDemo {
    visible: bool,
    width: f32,
    height: f32,
    rtl: bool,
    large_text: bool,
    size: ScrollFieldSize,
    compact: bool,
    disabled: bool,
    wide_range: bool,
    state: ScrollFieldState,
    committed: i32,
    commits: usize,
    reduced_motion: bool,
    requests_frame: bool,
}

impl Default for ScrollFieldDemo {
    fn default() -> Self {
        Self {
            visible: false,
            width: 960.,
            height: 760.,
            rtl: false,
            large_text: false,
            size: ScrollFieldSize::Medium,
            compact: false,
            disabled: false,
            wide_range: false,
            state: field_state(false),
            committed: 14,
            commits: 0,
            reduced_motion: false,
            requests_frame: false,
        }
    }
}

fn field_state(wide: bool) -> ScrollFieldState {
    let (min, max, initial) = if wide {
        (-1800, 1800, 1200)
    } else {
        (0, 30, 14)
    };
    let range = ScrollFieldRange::new(min, max, 2).expect("valid demo range");
    ScrollFieldState::new(range, initial).expect("initial value in demo range")
}

fn error_text(error: ScrollFieldError) -> String {
    match error {
        ScrollFieldError::Empty => "請輸入數量",
        ScrollFieldError::InvalidNumber => "請輸入整數",
        ScrollFieldError::OutOfRange => "數量超出範圍",
        ScrollFieldError::OffStep => "請輸入偶數",
    }
    .to_owned()
}

fn value_text(value: i32) -> String {
    format!("{value} 件")
}

impl ScrollFieldDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.state.invalidate_layout();
        self.requests_frame = false;
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    fn field_height(&self) -> f32 {
        if self.compact {
            120.
        } else {
            200.
        }
    }

    fn options(&self) -> ScrollFieldOptions<'_> {
        ScrollFieldOptions {
            id: FIELD,
            label: "數量",
            size: self.size,
            height: self.field_height(),
            available_width: (self.width - 2. * (PADDING + 2. * CONTENT_PADDING)).max(0.),
            available_height: (self.height - 2. * PADDING).max(0.),
            text_scale: self.text_scale(),
            direction: if self.rtl {
                LayoutDirection::RightToLeft
            } else {
                LayoutDirection::LeftToRight
            },
            disabled: self.disabled,
            hint: Some("每次增加或減少 2 件"),
            error_text: &error_text,
            value_text: &value_text,
            use_input: "輸入數值",
            use_scroll: "捲動選取",
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        if (width, height) == (self.width, self.height) {
            return false;
        }
        self.width = width;
        self.height = height;
        self.update_fit();
        self.visible
    }

    fn update_fit(&mut self) {
        self.state.invalidate_layout();
        let fits = scroll_field_layout(self.options(), &self.state).is_some();
        self.state.update_fit(fits);
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if !self.visible {
            return AnimationUpdate::default();
        }
        self.reduced_motion = frame.reduced_motion;
        let update = self.state.advance(frame.now_ms, frame.reduced_motion);
        self.requests_frame = update.request_next;
        update
    }

    pub(crate) fn update(&mut self, event: ScrollFieldDemoEvent) {
        match event {
            ScrollFieldDemoEvent::Show => {
                self.visible = true;
                self.update_fit();
            }
            ScrollFieldDemoEvent::Back => self.leave(),
            ScrollFieldDemoEvent::Direction => {
                self.rtl = !self.rtl;
                self.update_fit();
            }
            ScrollFieldDemoEvent::LargeText => {
                self.large_text = !self.large_text;
                self.update_fit();
            }
            ScrollFieldDemoEvent::Size(size) => {
                self.size = size;
                self.update_fit();
            }
            ScrollFieldDemoEvent::Compact => {
                self.compact = !self.compact;
                self.update_fit();
            }
            ScrollFieldDemoEvent::Disabled => {
                self.disabled = !self.disabled;
                self.state.invalidate_layout();
            }
            ScrollFieldDemoEvent::WideRange => {
                self.wide_range = !self.wide_range;
                self.state = field_state(self.wide_range);
                self.update_fit();
                self.committed = self.state.value();
                self.commits = 0;
            }
            ScrollFieldDemoEvent::Field(event) => {
                if !self.disabled {
                    if let Some(value) = self.state.apply(event, self.reduced_motion) {
                        self.committed = value;
                        self.commits += 1;
                    }
                }
            }
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "捲動欄位",
                ButtonKind::Outlined,
                false,
                Message::ScrollField(ScrollFieldDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let choice = |selected| ChoiceState {
            selected,
            disabled: false,
        };
        let toggle = |id, label, selected, event| {
            theme
                .segmented_button(id, label, choice(selected), Message::ScrollField(event))
                .accessible_activation()
        };
        let row = |children: Vec<View<Message>>| {
            View::new(
                Node::new(Role::Group, "").styled(
                    Style::row()
                        .width(Length::Percent(1.))
                        .gap(8.)
                        .wrap(true)
                        .align(Align::Center),
                ),
            )
            .children(children)
        };
        let profile = row(vec![
            theme
                .tonal_button(
                    BACK,
                    "返回",
                    Message::ScrollField(ScrollFieldDemoEvent::Back),
                )
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme
                .switch(
                    "scroll-field-demo-theme",
                    "深色",
                    choice(dark),
                    Message::Theme,
                )
                .accessible_activation(),
            toggle(
                "scroll-field-demo-rtl",
                "右起",
                self.rtl,
                ScrollFieldDemoEvent::Direction,
            ),
            toggle(
                "scroll-field-demo-large-text",
                "大字",
                self.large_text,
                ScrollFieldDemoEvent::LargeText,
            ),
        ]);
        let settings = row(SIZES
            .into_iter()
            .map(|(size, id, label)| {
                toggle(
                    id,
                    label,
                    self.size == size,
                    ScrollFieldDemoEvent::Size(size),
                )
            })
            .chain([
                toggle(
                    "scroll-field-demo-compact",
                    "矮欄位",
                    self.compact,
                    ScrollFieldDemoEvent::Compact,
                ),
                toggle(
                    "scroll-field-demo-disabled",
                    "停用",
                    self.disabled,
                    ScrollFieldDemoEvent::Disabled,
                ),
                toggle(
                    "scroll-field-demo-wide-range",
                    "四位數",
                    self.wide_range,
                    ScrollFieldDemoEvent::WideRange,
                ),
            ])
            .collect());
        let field = theme.scroll_field(self.options(), &self.state, |event| {
            Message::ScrollField(ScrollFieldDemoEvent::Field(event))
        });
        let field_position = Node::new(Role::Group, "").styled(
            Style::row()
                .width(Length::Percent(1.))
                .padding(CONTENT_PADDING),
        );
        let field_position = if self.rtl {
            field_position.reverse_row()
        } else {
            field_position
        };
        let saved = View::new(
            Node::new(Role::Text, "")
                .id("scroll-field-demo-saved")
                .text(format!("已選 {}", value_text(self.committed)))
                .styled(
                    Style::row()
                        .with_text_style(typography::BODY_MEDIUM)
                        .foreground(theme.colors.on_surface_variant),
                ),
        );
        let contents = View::new(Node::new(Role::Group, "").styled(Style {
            min_height: 900.,
            ..Style::column().padding(CONTENT_PADDING)
        }))
        .children([
            View::new(field_position).children([field]),
            View::new(
                Node::new(Role::Group, "").styled(Style::column().height(Length::Px(0.)).grow(1.)),
            )
            .without_interaction(None),
            saved,
        ]);
        let scroller = View::new(Node::new(Role::Group, "數值範例").id(SCROLL).styled(Style {
            min_height: 120.,
            ..Style::column().height(Length::Px(0.)).grow(1.).clip(true)
        }))
        .accessibility_scroll_pages()
        .children([contents]);
        View::new(
            Node::new(Role::Window, "捲動欄位")
                .id("scroll-field-demo")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .padding(PADDING)
                        .gap(8.)
                        .clip(true)
                        .background(theme.colors.surface),
                ),
        )
        .restore_focus_in_subtree_on_removal(SHOW)
        .accessibility_scroll_pages()
        .children([profile, settings, scroller])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        runtime: &surface_app::Runtime<crate::Gallery>,
        theme: &Theme,
    ) -> serde_json::Value {
        let layout = scroll_field_layout(self.options(), &self.state);
        let fits = layout.is_some();
        let mode = if fits && self.state.mode() == ScrollFieldMode::Scroll {
            "scroll"
        } else {
            "input"
        };
        let size = match self.size {
            ScrollFieldSize::Small => "small",
            ScrollFieldSize::Medium => "medium",
            ScrollFieldSize::Large => "large",
        };
        serde_json::json!({
            "visible": self.visible, "rtl": self.rtl, "text_scale": self.text_scale(),
            "size": size, "height": self.field_height(), "disabled": self.disabled,
            "face_height": layout.map(|layout| layout.face_height),
            "row_height": layout.map(|layout| layout.row_height),
            "committed": self.committed, "preview": self.state.preview(),
            "raw": self.state.raw(), "error": self.state.error().map(|error| format!("{error:?}")),
            "mode": mode, "offset": self.state.offset(), "fits": fits,
            "dragging": self.state.is_dragging(), "settling": self.state.settling(),
            "requests_frame": self.requests_frame, "reduced_motion": self.reduced_motion,
            "scroll_top": runtime.scroll_metrics(SCROLL).map(|metrics| metrics.offset.1),
            "commits": self.commits,
            "palette": {
                "surface": theme.colors.surface, "face": theme.colors.surface_container_lowest,
                "center": theme.colors.on_surface, "neighbor": theme.colors.outline,
                "focus": theme.colors.secondary, "editing": theme.colors.primary,
                "error": theme.colors.error,
            },
        })
    }
}
