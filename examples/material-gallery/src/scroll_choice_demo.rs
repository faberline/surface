use surface_app::{AnimationFrame, AnimationUpdate, View};
#[cfg(target_arch = "wasm32")]
use surface_material::ScrollFieldMode;
use surface_material::{
    scroll_choice_field_layout, shape::LayoutDirection, ButtonKind, ChoiceState, ScrollChoiceEvent,
    ScrollChoiceFieldOptions, ScrollChoiceItem, ScrollChoiceQueryError, ScrollChoiceState,
    ScrollFieldSize, Theme,
};

use crate::{
    scroll_field_demo::{control_row, ScrollFieldScene, CONTENT_PADDING, PADDING},
    Message,
};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "scroll-choice-demo-show";
const SHOW_LOOP: &str = "scroll-loop-choice-show";
const BACK: &str = "scroll-choice-demo-back";
const SCROLL: &str = "scroll-choice-demo-scroll";
const FIELD: &str = "scroll-choice-demo-value";
const SCENE: ScrollFieldScene = ScrollFieldScene {
    page: "scroll-choice-demo",
    scroll: SCROLL,
    show: SHOW,
    saved: "scroll-choice-demo-saved",
    title: "文字捲動欄位",
    scroller_label: "地點範例",
    scroll_padding: 0.,
};

fn scene(looping: bool) -> ScrollFieldScene {
    if looping {
        ScrollFieldScene {
            show: SHOW_LOOP,
            title: "循環文字欄位",
            ..SCENE
        }
    } else {
        SCENE
    }
}

fn catalog(
    reverse: bool,
    relabeled: bool,
    looping: bool,
) -> impl Iterator<Item = ScrollChoiceItem<'static>> {
    let mut items = [
        ScrollChoiceItem {
            id: "banqiao",
            label: if relabeled { "板橋站" } else { "板橋" },
            detail: None,
            aliases: &["Banqiao"],
        },
        ScrollChoiceItem {
            id: "taipei-north",
            label: "台北",
            detail: Some("北門"),
            aliases: &["台北北門", "Taipei North Gate"],
        },
        ScrollChoiceItem {
            id: "taipei-station",
            label: "台北",
            detail: Some("車站"),
            aliases: &["台北車站", "Taipei Station"],
        },
        ScrollChoiceItem {
            id: "taoyuan",
            label: "桃園",
            detail: None,
            aliases: &["Taoyuan"],
        },
        ScrollChoiceItem {
            id: "hsinchu",
            label: "新竹",
            detail: None,
            aliases: &["Hsinchu"],
        },
    ];
    if reverse {
        items.reverse();
    }
    items
        .into_iter()
        .filter(move |item| !looping || matches!(item.id, "banqiao" | "taipei-station"))
}

fn error_text(error: ScrollChoiceQueryError) -> String {
    match error {
        ScrollChoiceQueryError::Empty => "請輸入地點",
        ScrollChoiceQueryError::NoMatch => "找不到地點",
        ScrollChoiceQueryError::Ambiguous { .. } => "有多個相符地點，請輸入完整名稱",
    }
    .to_owned()
}

fn choice_text(label: &str, detail: Option<&str>) -> String {
    detail.map_or_else(|| label.to_owned(), |detail| format!("{label} {detail}"))
}

fn suggestion_text(label: &str) -> String {
    format!("建議選項 {label}")
}

#[derive(Clone)]
pub enum ScrollChoiceDemoEvent {
    Show(bool),
    Back,
    Direction,
    LargeText,
    Compact,
    Disabled,
    Reorder,
    Relabel,
    Field(ScrollChoiceEvent),
}

pub(crate) struct ScrollChoiceDemo {
    visible: bool,
    viewport: (f32, f32),
    rtl: bool,
    large_text: bool,
    compact: bool,
    disabled: bool,
    reverse: bool,
    relabeled: bool,
    state: ScrollChoiceState,
    committed: String,
    commits: usize,
    reduced_motion: bool,
    requests_frame: bool,
}

impl Default for ScrollChoiceDemo {
    fn default() -> Self {
        Self::new(false)
    }
}

impl ScrollChoiceDemo {
    fn new(looping: bool) -> Self {
        let state = ScrollChoiceState::with_match_key(
            catalog(false, false, looping),
            "banqiao",
            str::to_lowercase,
        )
        .expect("valid Gallery location catalog");
        let state = if looping { state.with_looping() } else { state };
        Self {
            visible: false,
            viewport: (960., 760.),
            rtl: false,
            large_text: false,
            compact: false,
            disabled: false,
            reverse: false,
            relabeled: false,
            state,
            committed: "banqiao".to_owned(),
            commits: 0,
            reduced_motion: false,
            requests_frame: false,
        }
    }
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

    fn options(&self) -> ScrollChoiceFieldOptions<'_> {
        ScrollChoiceFieldOptions {
            id: FIELD,
            label: "地點",
            size: ScrollFieldSize::Medium,
            height: if self.compact { 120. } else { 200. },
            available_width: (self.viewport.0 - 2. * (PADDING + 2. * CONTENT_PADDING)).max(0.),
            available_height: (self.viewport.1 - 2. * PADDING).max(0.),
            text_scale: self.text_scale(),
            direction: if self.rtl {
                LayoutDirection::RightToLeft
            } else {
                LayoutDirection::LeftToRight
            },
            disabled: self.disabled,
            hint: Some("可輸入部分地名"),
            use_input: "輸入地點",
            use_scroll: "捲動選取",
            error_text: &error_text,
            choice_text: &choice_text,
            suggestion_text: &suggestion_text,
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        if (width, height) == self.viewport {
            return false;
        }
        self.viewport = (width, height);
        self.update_fit();
        self.visible
    }

    fn update_fit(&mut self) {
        self.state.invalidate_layout();
        let fits = scroll_choice_field_layout(self.options(), &self.state).is_some();
        self.state.update_fit(fits);
    }

    fn replace_catalog(&mut self) {
        let items = catalog(self.reverse, self.relabeled, self.state.looping());
        self.state
            .replace_items(items, &self.committed)
            .expect("reordered catalog carries every original location ID");
        self.update_fit();
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

    pub(crate) fn update(&mut self, event: ScrollChoiceDemoEvent) {
        use ScrollChoiceDemoEvent::*;
        match event {
            Show(looping) => {
                if looping != self.state.looping() {
                    *self = Self {
                        viewport: self.viewport,
                        ..Self::new(looping)
                    };
                }
                self.visible = true;
                self.update_fit();
            }
            Back => self.leave(),
            Direction => {
                self.rtl = !self.rtl;
                self.update_fit();
            }
            LargeText => {
                self.large_text = !self.large_text;
                self.update_fit();
            }
            Compact => {
                self.compact = !self.compact;
                self.update_fit();
            }
            Disabled => {
                self.disabled = !self.disabled;
                self.state.invalidate_layout();
            }
            Reorder => {
                self.reverse = !self.reverse;
                self.replace_catalog();
            }
            Relabel => {
                self.relabeled = !self.relabeled;
                self.replace_catalog();
            }
            Field(event) => {
                if !self.disabled {
                    if let Some(id) = self.state.apply(event, self.reduced_motion) {
                        self.committed = id;
                        self.commits += 1;
                    }
                }
            }
        }
    }

    pub(crate) fn show_button(theme: &Theme, looping: bool) -> View<Message> {
        let scene = scene(looping);
        theme
            .button_with(
                scene.show,
                scene.title,
                ButtonKind::Outlined,
                false,
                Message::ScrollChoice(ScrollChoiceDemoEvent::Show(looping)),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use ScrollChoiceDemoEvent::*;
        let scene = scene(self.state.looping());
        let choice = |selected| ChoiceState {
            selected,
            disabled: false,
        };
        let toggle = |id, title, selected, event| {
            theme
                .segmented_button(id, title, choice(selected), Message::ScrollChoice(event))
                .accessible_activation()
        };
        let profile = control_row(vec![
            theme
                .tonal_button(BACK, "返回", Message::ScrollChoice(Back))
                .restore_focus_on_removal(scene.show)
                .accessible_activation(),
            theme
                .switch(
                    "scroll-choice-demo-theme",
                    "深色",
                    choice(dark),
                    Message::Theme,
                )
                .accessible_activation(),
            toggle("scroll-choice-demo-rtl", "右起", self.rtl, Direction),
            toggle(
                "scroll-choice-demo-large-text",
                "大字",
                self.large_text,
                LargeText,
            ),
        ]);
        let settings = control_row(vec![
            toggle(
                "scroll-choice-demo-compact",
                "矮欄位",
                self.compact,
                Compact,
            ),
            toggle(
                "scroll-choice-demo-disabled",
                "停用",
                self.disabled,
                Disabled,
            ),
            toggle("scroll-choice-demo-reorder", "倒序", self.reverse, Reorder),
            toggle(
                "scroll-choice-demo-relabel",
                "改名",
                self.relabeled,
                Relabel,
            ),
        ]);
        let field = theme.scroll_choice_field(self.options(), &self.state, |event| {
            Message::ScrollChoice(Field(event))
        });
        let chosen = self.state.choice();
        scene.view(
            theme,
            self.rtl,
            [profile, settings],
            field,
            format!("已選 {}", choice_text(chosen.label, chosen.detail)),
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        runtime: &surface_app::Runtime<crate::Gallery>,
        theme: &Theme,
    ) -> serde_json::Value {
        let options = self.options();
        let layout = scroll_choice_field_layout(options, &self.state);
        let mode = if self.state.mode() == ScrollFieldMode::Scroll {
            "scroll"
        } else {
            "input"
        };
        let items: Vec<_> = (0..self.state.count())
            .filter_map(|index| self.state.get(index))
            .collect();
        let preview = self
            .state
            .get(self.state.preview_index())
            .map(|item| item.id);
        serde_json::json!({
            "visible": self.visible, "rtl": self.rtl, "text_scale": self.text_scale(),
            "looping": self.state.looping(),
            "height": options.height, "face_height": layout.map(|layout| layout.face_height),
            "row_height": layout.map(|layout| layout.row_height), "fits": layout.is_some(),
            "disabled": self.disabled, "committed": self.committed, "preview": preview,
            "ordinal": self.state.index() + 1, "commits": self.commits,
            "raw": self.state.raw(), "error": self.state.error().map(|error| format!("{error:?}")),
            "suggestion": self.state.suggestion().map(|item| choice_text(item.label, item.detail)),
            "mode": mode, "offset": self.state.offset(),
            "dragging": self.state.is_dragging(), "settling": self.state.settling(),
            "requests_frame": self.requests_frame, "reduced_motion": self.reduced_motion,
            "scroll_top": runtime.scroll_metrics(SCROLL).map(|metrics| metrics.offset.1),
            "order": items.iter().map(|item| item.id).collect::<Vec<_>>(),
            "labels": items.iter().map(|item| choice_text(item.label, item.detail)).collect::<Vec<_>>(),
            "palette": crate::scroll_field_demo::palette(theme),
        })
    }
}
