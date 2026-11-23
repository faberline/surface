use surface::{Align, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, time_scroll_layout, ButtonKind, ClockFormat, Theme, TimeField,
    TimeInputEffect, TimeInputEvent, TimeInputOptions, TimeInputPresentation, TimeInputSession,
    TimeOfDay, TimeScrollEvent, TimeScrollLabels, TimeScrollMode, TimeScrollOptions,
    TimeScrollState,
};

use crate::{
    time_input_demo::{field, labels, locale},
    Message,
};

pub(crate) const SECTION: usize = 2;
const INLINE: &str = "time-scroll-inline";
const MODAL: &str = "time-scroll-modal";
const OPEN: &str = "time-scroll-demo-open";
const PAGE_PADDING: f32 = 12.;

#[derive(Clone)]
pub enum TimeScrollDemoEvent {
    Show,
    Back,
    Direction,
    Scale,
    Locale,
    Clock,
    Disabled,
    Open,
    Outside,
    Inline(TimeScrollEvent),
    Modal(TimeScrollEvent),
}

pub(crate) struct TimeScrollDemo {
    visible: bool,
    rtl: bool,
    large_text: bool,
    japanese: bool,
    disabled: bool,
    clock: ClockFormat,
    viewport: Rect,
    reduced_motion: bool,
    inline: Option<TimeScrollState>,
    modal: Option<TimeScrollState>,
    inline_saved: TimeOfDay,
    saved: TimeOfDay,
    inline_commits: usize,
    commits: usize,
    outside_actions: usize,
}

impl Default for TimeScrollDemo {
    fn default() -> Self {
        let initial = TimeOfDay::new(9, 7).expect("valid fixture time");
        Self {
            visible: false,
            rtl: false,
            large_text: false,
            japanese: false,
            disabled: false,
            clock: ClockFormat::TwelveHour,
            viewport: Rect::new(0., 0., 960., 760.),
            reduced_motion: false,
            inline: None,
            modal: None,
            inline_saved: initial,
            saved: initial,
            inline_commits: 0,
            commits: 0,
            outside_actions: 0,
        }
    }
}

impl TimeScrollDemo {
    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme.button_with(
            "show-time-scroll-demo",
            "捲動選時",
            ButtonKind::Outlined,
            false,
            Message::TimeScroll(TimeScrollDemoEvent::Show),
        )
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.inline = None;
        self.modal = None;
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    fn viewport_for(&self, presentation: TimeInputPresentation) -> Rect {
        let mut viewport = self.viewport;
        if presentation == TimeInputPresentation::Inline {
            viewport.width = (viewport.width - 2. * PAGE_PADDING).max(0.);
        }
        viewport
    }

    fn fits(&self, presentation: TimeInputPresentation) -> bool {
        let viewport = self.viewport_for(presentation);
        time_scroll_layout(
            viewport.width,
            viewport.height,
            self.text_scale(),
            self.clock,
            presentation,
        )
        .is_some()
    }

    fn start(
        &self,
        time: TimeOfDay,
        presentation: TimeInputPresentation,
        mode: TimeScrollMode,
    ) -> TimeScrollState {
        TimeScrollState::new(
            TimeInputSession::new(Some(time), self.clock, locale(self.japanese)),
            mode,
            self.fits(presentation),
        )
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let viewport = Rect::new(0., 0., width, height);
        if self.viewport == viewport {
            return false;
        }
        self.viewport = viewport;
        self.refresh_fit();
        self.visible
    }

    fn refresh_fit(&mut self) {
        let inline_fits = self.fits(TimeInputPresentation::Inline);
        let modal_fits = self.fits(TimeInputPresentation::Modal);
        for (draft, fits) in [
            (&mut self.inline, inline_fits),
            (&mut self.modal, modal_fits),
        ] {
            if let Some(draft) = draft {
                draft.apply(TimeScrollEvent::SetScrollFits(fits), self.reduced_motion);
            }
        }
    }

    fn change_settings(&mut self) {
        if let Some(inline) = &self.inline {
            let seed = inline
                .input()
                .validation()
                .candidate()
                .unwrap_or(self.inline_saved);
            self.inline = Some(self.start(seed, TimeInputPresentation::Inline, inline.mode()));
        }
    }

    pub(crate) fn observe_focus(&mut self, focused: Option<&str>) -> bool {
        let mut changed = false;
        for (draft, id) in [(&mut self.inline, INLINE), (&mut self.modal, MODAL)] {
            if let Some(draft) = draft {
                let wheel = focused
                    .and_then(|focus| focus.strip_prefix(id))
                    .and_then(|part| match part {
                        "-hour-wheel" => Some(TimeField::Hour),
                        "-minute-wheel" => Some(TimeField::Minute),
                        _ => None,
                    });
                changed |= draft.observe_input_focus(field(focused, id));
                changed |= draft.observe_scroll_focus(wheel);
            }
        }
        changed
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        self.reduced_motion = frame.reduced_motion;
        let mut update = AnimationUpdate::default();
        for state in self.inline.iter_mut().chain(self.modal.iter_mut()) {
            let next = state.advance(frame.now_ms, frame.reduced_motion);
            update.rebuild |= next.rebuild;
            update.request_next |= next.request_next;
        }
        update
    }

    pub(crate) fn update(&mut self, event: TimeScrollDemoEvent) {
        match event {
            TimeScrollDemoEvent::Show => {
                self.visible = true;
                self.inline = Some(self.start(
                    self.inline_saved,
                    TimeInputPresentation::Inline,
                    TimeScrollMode::Scroll,
                ));
            }
            TimeScrollDemoEvent::Back => self.leave(),
            TimeScrollDemoEvent::Direction => {
                self.rtl = !self.rtl;
                self.refresh_fit();
            }
            TimeScrollDemoEvent::Scale => {
                self.large_text = !self.large_text;
                self.refresh_fit();
            }
            TimeScrollDemoEvent::Locale => {
                self.japanese = !self.japanese;
                self.change_settings();
            }
            TimeScrollDemoEvent::Clock => {
                self.clock = match self.clock {
                    ClockFormat::TwelveHour => ClockFormat::TwentyFourHour,
                    ClockFormat::TwentyFourHour => ClockFormat::TwelveHour,
                };
                self.change_settings();
            }
            TimeScrollDemoEvent::Disabled => {
                self.disabled = !self.disabled;
                self.refresh_fit();
            }
            TimeScrollDemoEvent::Open => {
                self.modal = Some(self.start(
                    self.saved,
                    TimeInputPresentation::Modal,
                    TimeScrollMode::Scroll,
                ));
            }
            TimeScrollDemoEvent::Outside => self.outside_actions += 1,
            TimeScrollDemoEvent::Inline(event) => {
                if let Some(draft) = &mut self.inline {
                    if let TimeInputEffect::Commit(value) = draft.apply(event, self.reduced_motion)
                    {
                        self.inline_saved = value;
                        self.inline_commits += 1;
                    }
                }
            }
            TimeScrollDemoEvent::Modal(event) => {
                if let Some(draft) = &mut self.modal {
                    match draft.apply(event, self.reduced_motion) {
                        TimeInputEffect::Commit(value) => {
                            self.saved = value;
                            self.commits += 1;
                            self.modal = None;
                        }
                        TimeInputEffect::Dismiss => self.modal = None,
                        TimeInputEffect::None => {}
                    }
                }
            }
        }
    }

    fn picker(
        &self,
        theme: &Theme,
        presentation: TimeInputPresentation,
        state: &TimeScrollState,
    ) -> View<Message> {
        let modal = presentation == TimeInputPresentation::Modal;
        let labels = labels(self.japanese, state.input().clock());
        let (title, scroll_labels) = if self.japanese {
            (
                "時刻を選択",
                TimeScrollLabels {
                    use_input: "文字で入力",
                    use_scroll: "スクロールで選択",
                    unfit: "スクロールで選択するには広い画面が必要です",
                    unset: "未設定",
                },
            )
        } else {
            (
                "選擇時間",
                TimeScrollLabels {
                    use_input: "文字輸入",
                    use_scroll: "捲動選時",
                    unfit: "空間不足，請用文字輸入",
                    unset: "尚未設定",
                },
            )
        };
        theme.time_scroll(
            TimeScrollOptions {
                input: TimeInputOptions {
                    id: if modal { MODAL } else { INLINE },
                    title,
                    labels: &labels,
                    locale: locale(self.japanese),
                    direction: if self.rtl {
                        LayoutDirection::RightToLeft
                    } else {
                        LayoutDirection::LeftToRight
                    },
                    presentation,
                    viewport: self.viewport_for(presentation),
                    disabled: self.disabled,
                    restore_focus_to: modal.then_some(OPEN),
                },
                labels: &scroll_labels,
                text_scale: self.text_scale(),
            },
            state,
            move |event| {
                Message::TimeScroll(if modal {
                    TimeScrollDemoEvent::Modal(event)
                } else {
                    TimeScrollDemoEvent::Inline(event)
                })
            },
        )
    }

    pub(crate) fn view(&self, theme: &Theme) -> View<Message> {
        let action = |id, label, event| theme.button(id, label, Message::TimeScroll(event));
        let controls = View::new(
            Node::new(Role::Group, "").styled(Style::row().wrap(true).gap(8.)),
        )
        .children([
            action("time-scroll-demo-back", "返回", TimeScrollDemoEvent::Back),
            action(OPEN, "開啟對話框", TimeScrollDemoEvent::Open),
            theme.button("time-scroll-demo-theme", "切換配色", Message::Theme),
            action(
                "time-scroll-demo-direction",
                if self.rtl { "左起" } else { "右起" },
                TimeScrollDemoEvent::Direction,
            ),
            action(
                "time-scroll-demo-scale",
                if self.large_text {
                    "一般文字"
                } else {
                    "大字"
                },
                TimeScrollDemoEvent::Scale,
            ),
            action(
                "time-scroll-demo-locale",
                if self.japanese { "中文" } else { "日本語" },
                TimeScrollDemoEvent::Locale,
            ),
            action(
                "time-scroll-demo-clock",
                if self.clock == ClockFormat::TwelveHour {
                    "24 小時"
                } else {
                    "12 小時"
                },
                TimeScrollDemoEvent::Clock,
            ),
            action(
                "time-scroll-demo-disabled",
                if self.disabled { "啟用" } else { "停用" },
                TimeScrollDemoEvent::Disabled,
            ),
        ]);
        let inline = self
            .inline
            .as_ref()
            .map(|state| self.picker(theme, TimeInputPresentation::Inline, state));
        let save = theme.button_with(
            "time-scroll-demo-confirm-inline",
            "儲存",
            ButtonKind::Filled,
            self.disabled,
            Message::TimeScroll(TimeScrollDemoEvent::Inline(TimeScrollEvent::Input(
                TimeInputEvent::Confirm,
            ))),
        );
        let summary = theme.text(
            "time-scroll-demo-saved",
            format!("對話框已儲存 {}", self.saved),
        );
        let inline_summary = theme.text(
            "time-scroll-demo-inline-saved",
            format!("頁面已儲存 {}", self.inline_saved),
        );
        let outside = action(
            "time-scroll-demo-outside",
            "其他操作",
            TimeScrollDemoEvent::Outside,
        );
        let scroll = View::new(
            Node::new(Role::Group, "")
                .id("time-scroll-demo-page")
                .shrink_to_fit_height()
                .styled(
                    Style::column()
                        .grow(1.)
                        .clip(true)
                        .gap(12.)
                        .align(Align::Start),
                ),
        )
        .children(
            inline.into_iter().chain([
                theme.row([save, inline_summary]),
                outside,
                summary,
                View::new(
                    Node::new(Role::Group, "")
                        .styled(Style::column().height(Length::Px(self.viewport.height))),
                ),
            ]),
        )
        .accessibility_scroll_pages();
        let modal = self
            .modal
            .as_ref()
            .map(|state| self.picker(theme, TimeInputPresentation::Modal, state));
        View::new(
            Node::new(Role::Window, "捲動選時")
                .id("time-scroll-demo")
                .styled(
                    Style::column()
                        .padding(PAGE_PADDING)
                        .gap(8.)
                        .clip(true)
                        .background(theme.colors.surface)
                        .foreground(theme.colors.on_surface),
                ),
        )
        .children([controls, scroll].into_iter().chain(modal))
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        fn draft(state: &TimeScrollState) -> serde_json::Value {
            let input = state.input();
            let column = |field| {
                let column = state.column(field);
                serde_json::json!({
                    "accepted": column.selected, "preview": column.preview,
                    "offset": column.offset, "settling": column.settling,
                })
            };
            serde_json::json!({
                "mode": format!("{:?}", state.mode()),
                "fits": state.scroll_fits(),
                "active": format!("{:?}", state.active_field()),
                "candidate": input.validation().candidate().map(|value| value.to_string()),
                "raw_hour": input.raw(TimeField::Hour),
                "raw_minute": input.raw(TimeField::Minute),
                "period": input.period().map(|period| format!("{period:?}")),
                "hour_error": input.displayed_error(TimeField::Hour).map(|error| format!("{error:?}")),
                "minute_error": input.displayed_error(TimeField::Minute).map(|error| format!("{error:?}")),
                "hour": column(TimeField::Hour),
                "minute": column(TimeField::Minute),
            })
        }
        serde_json::json!({
            "visible": self.visible, "rtl": self.rtl, "disabled": self.disabled,
            "text_scale": self.text_scale(), "locale": if self.japanese { "ja" } else { "zh-Hant" },
            "clock": format!("{:?}", self.clock),
            "inline": self.inline.as_ref().map(draft), "modal": self.modal.as_ref().map(draft),
            "inline_saved": self.inline_saved.to_string(), "saved": self.saved.to_string(),
            "inline_commits": self.inline_commits, "commits": self.commits,
            "outside_actions": self.outside_actions,
        })
    }
}
