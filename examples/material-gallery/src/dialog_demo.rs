use surface::{Align, Color, Insets, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, CursorStyle, ModalDismissal, View};
use surface_material::{
    adaptive::WindowWidthSizeClass, full_screen_dialog_layout, shape, shape::LayoutDirection,
    typography, BasicDialogContent, BasicDialogMotion, BasicDialogOptions, ButtonKind, ChoiceState,
    FabColor, FabSize, FullScreenDialogAnchor, FullScreenDialogMotion, FullScreenDialogOptions,
    FullScreenDialogTransition, Theme,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 2;
const SHOW: &str = "dialog-demo-show";
const BACK: &str = "dialog-demo-back";
const BASIC: &str = "dialog-demo-basic";
const ALERT: &str = "dialog-demo-alert";
const EDIT: &str = "dialog-demo-edit";
const DISCARD: &str = "dialog-demo-discard";
const BASIC_OPEN: &str = "dialog-demo-basic-open";
const ALERT_OPEN: &str = "dialog-demo-alert-open";
const EDIT_OPEN: &str = "dialog-demo-edit-open";
const SHOW_TRANSFORM: &str = "dialog-demo-show-transform";
const FAB_OPEN: &str = "dialog-demo-fab-open";
const FAB_PREVIEW_SCROLL: &str = "dialog-demo-fab-preview-scroll";
const FAB_SIZE: f32 = 56.;
const FAB_INSET: f32 = 24.;
const EDITOR: &str = "dialog-demo-editor";
const SAVED: &str = "dialog-demo-saved";
const EDIT_TITLE: &str = "編輯行程";
const INITIAL: &str = "週末行程";

#[derive(Clone)]
pub enum DialogDemoEvent {
    Show,
    ShowTransform,
    Back,
    ToggleDirection,
    ToggleLargeText,
    OpenBasic,
    CloseBasic,
    OpenAlert,
    Acknowledge,
    OpenEdit,
    Input(String),
    RequestClose,
    KeepEditing,
    Discard,
    Save,
}

enum Active {
    Basic,
    Alert,
    Edit { draft: String, ask_discard: bool },
}

pub(crate) struct DialogDemo {
    visible: bool,
    viewport: Rect,
    rtl: bool,
    large_text: bool,
    active: Option<Active>,
    basic_motion: BasicDialogMotion,
    alert_motion: BasicDialogMotion,
    discard_motion: BasicDialogMotion,
    transform_preview: bool,
    full_motion: FullScreenDialogMotion,
    fab_anchor: Option<FullScreenDialogAnchor>,
    retiring_editor: Option<String>,
    saved: String,
    saves: usize,
    discards: usize,
    alerts: usize,
}

impl Default for DialogDemo {
    fn default() -> Self {
        Self {
            visible: false,
            viewport: Rect::default(),
            rtl: false,
            large_text: false,
            active: None,
            basic_motion: BasicDialogMotion::default(),
            alert_motion: BasicDialogMotion::default(),
            discard_motion: BasicDialogMotion::default(),
            transform_preview: false,
            full_motion: FullScreenDialogMotion::default(),
            fab_anchor: None,
            retiring_editor: None,
            saved: INITIAL.into(),
            saves: 0,
            discards: 0,
            alerts: 0,
        }
    }
}

impl DialogDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let next = Rect::new(0., 0., width, height);
        let changed = self.viewport != next;
        self.viewport = next;
        if changed {
            self.fab_anchor = None;
        }
        self.visible && changed
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.active = None;
        self.basic_motion = BasicDialogMotion::default();
        self.alert_motion = BasicDialogMotion::default();
        self.discard_motion = BasicDialogMotion::default();
        self.transform_preview = false;
        self.fab_anchor = None;
        self.reset_full_motion();
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if !self.visible {
            return AnimationUpdate::default();
        }
        let basic = self
            .basic_motion
            .advance(frame, matches!(self.active, Some(Active::Basic)));
        let alert = self
            .alert_motion
            .advance(frame, matches!(self.active, Some(Active::Alert)));
        let discard = self.discard_motion.advance(
            frame,
            matches!(
                self.active,
                Some(Active::Edit {
                    ask_discard: true,
                    ..
                })
            ),
        );
        let full = self.advance_full_motion(frame);
        AnimationUpdate {
            rebuild: basic.rebuild || alert.rebuild || discard.rebuild || full.rebuild,
            request_next: basic.request_next
                || alert.request_next
                || discard.request_next
                || full.request_next,
        }
    }

    fn advance_full_motion(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if !self.transform_preview {
            return AnimationUpdate::default();
        }
        let previous_anchor = self.fab_anchor;
        let current_layout = frame
            .layout
            .is_some_and(|layout| layout.viewport == self.viewport);
        if current_layout {
            self.fab_anchor = FullScreenDialogAnchor::measured(
                frame,
                FAB_OPEN,
                shape::LARGE.top_left,
                Color::TRANSPARENT,
            )
            .filter(|anchor| anchor.bounds.width == FAB_SIZE && anchor.bounds.height == FAB_SIZE);
        }
        let is_full = self.is_full_screen();
        let editor_open = matches!(self.active, Some(Active::Edit { .. }));
        let anchor = if is_full { self.fab_anchor } else { None };
        let mut update = self
            .full_motion
            .advance(frame, editor_open, anchor, self.viewport);
        // Animation sees the previous layout. After a rebuild, schedule the
        // measurement even when the motion is idle or reduced.
        update.request_next |= is_full && !current_layout;
        if self.retiring_editor.is_some() && (!is_full || !self.full_motion.should_mount(false)) {
            self.retiring_editor = None;
            update.rebuild = true;
        }
        if is_full
            && (editor_open || self.retiring_editor.is_some())
            && previous_anchor != self.fab_anchor
        {
            update.rebuild = true;
        }
        update
    }

    fn finish_editor(&mut self) {
        if let Some(Active::Edit { draft, .. }) = self.active.take() {
            if self.transform_preview
                && self.is_full_screen()
                && self.full_motion.should_mount(false)
            {
                self.retiring_editor = Some(draft);
            }
        }
    }

    fn reset_full_motion(&mut self) {
        self.full_motion = FullScreenDialogMotion::default();
        self.retiring_editor = None;
    }

    pub(crate) fn update(&mut self, event: DialogDemoEvent) {
        use DialogDemoEvent::*;
        match event {
            Show | ShowTransform => {
                self.visible = true;
                self.transform_preview = matches!(event, ShowTransform);
            }
            Back => self.leave(),
            ToggleDirection => {
                self.rtl = !self.rtl;
                self.fab_anchor = None;
            }
            ToggleLargeText => self.large_text = !self.large_text,
            OpenBasic => {
                self.reset_full_motion();
                self.alert_motion = BasicDialogMotion::default();
                self.discard_motion = BasicDialogMotion::default();
                self.active = Some(Active::Basic);
            }
            CloseBasic if matches!(self.active, Some(Active::Basic)) => self.active = None,
            OpenAlert => {
                self.reset_full_motion();
                self.basic_motion = BasicDialogMotion::default();
                self.discard_motion = BasicDialogMotion::default();
                self.active = Some(Active::Alert);
            }
            Acknowledge if matches!(self.active, Some(Active::Alert)) => {
                self.alerts += 1;
                self.active = None;
            }
            OpenEdit => {
                self.retiring_editor = None;
                self.basic_motion = BasicDialogMotion::default();
                self.alert_motion = BasicDialogMotion::default();
                self.discard_motion = BasicDialogMotion::default();
                self.active = Some(Active::Edit {
                    draft: self.saved.clone(),
                    ask_discard: false,
                });
            }
            Input(value) => {
                if let Some(Active::Edit {
                    draft,
                    ask_discard: false,
                }) = &mut self.active
                {
                    *draft = value;
                }
            }
            RequestClose => {
                if let Some(Active::Edit { draft, ask_discard }) = &mut self.active {
                    if *draft == self.saved {
                        self.finish_editor();
                    } else {
                        *ask_discard = true;
                    }
                }
            }
            KeepEditing => {
                if let Some(Active::Edit { ask_discard, .. }) = &mut self.active {
                    *ask_discard = false;
                }
            }
            Discard
                if matches!(
                    self.active,
                    Some(Active::Edit {
                        ask_discard: true,
                        ..
                    })
                ) =>
            {
                self.discards += 1;
                self.finish_editor();
            }
            Save => {
                if let Some(Active::Edit {
                    draft,
                    ask_discard: false,
                }) = &mut self.active
                {
                    self.saved.clone_from(draft);
                    self.saves += 1;
                    self.finish_editor();
                }
            }
            CloseBasic | Acknowledge | Discard => {}
        }
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn full_options(&self) -> FullScreenDialogOptions<'static> {
        FullScreenDialogOptions {
            id: EDIT,
            title: EDIT_TITLE,
            body_label: "行程內容",
            close_label: "關閉",
            action_label: "儲存",
            action_disabled: false,
            initial_focus: Some(EDITOR),
            restore_focus_to: Some(self.editor_opener()),
            viewport: self.viewport,
            direction: self.direction(),
        }
    }

    fn is_full_screen(&self) -> bool {
        WindowWidthSizeClass::from_dp(self.viewport.width) == Some(WindowWidthSizeClass::Compact)
            && full_screen_dialog_layout(self.full_options()).is_some()
    }

    fn editor_opener(&self) -> &'static str {
        if self.transform_preview {
            FAB_OPEN
        } else {
            EDIT_OPEN
        }
    }

    fn demo_opener(&self) -> &'static str {
        if self.transform_preview {
            SHOW_TRANSFORM
        } else {
            SHOW
        }
    }

    pub(crate) fn show_transform_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW_TRANSFORM,
                "全螢幕轉場",
                ButtonKind::Outlined,
                false,
                Message::DialogDemo(DialogDemoEvent::ShowTransform),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn fab_launcher(&self, theme: &Theme) -> View<Message> {
        let mut inset = Insets {
            bottom: Length::Px(FAB_INSET),
            ..Insets::default()
        };
        if self.rtl {
            inset.left = Length::Px(FAB_INSET);
        } else {
            inset.right = Length::Px(FAB_INSET);
        }
        View::new(
            Node::new(Role::Group, "")
                .id("dialog-demo-fab-anchor")
                .styled(
                    Style::row()
                        .fixed(inset)
                        .width(Length::Px(FAB_SIZE))
                        .height(Length::Px(FAB_SIZE)),
                ),
        )
        .children([theme
            .fab(
                FAB_OPEN,
                "新增行程",
                "+",
                FabSize::Regular,
                FabColor::Primary,
                false,
                Message::DialogDemo(DialogDemoEvent::OpenEdit),
            )
            .suppress_repeated_activation()
            .accessible_activation()])
    }

    fn basic_options(&self, id: &'static str, label: &'static str) -> BasicDialogOptions<'static> {
        BasicDialogOptions {
            id,
            label,
            heading: Some(label),
            supporting: None,
            icon: None,
            window_size: (self.viewport.width, self.viewport.height),
            preferred_width: 400.,
            direction: self.direction(),
            dismissal: ModalDismissal::EscapeOrOutside,
            alert: false,
            initial_focus: None,
            restore_focus_to: None,
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "對話框",
                ButtonKind::Outlined,
                false,
                Message::DialogDemo(DialogDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use DialogDemoEvent::*;
        let toggle = |id, label, selected, event| {
            theme
                .segmented_button(
                    id,
                    label,
                    ChoiceState {
                        selected,
                        disabled: false,
                    },
                    Message::DialogDemo(event),
                )
                .accessible_activation()
        };
        let preview = |id, label, event| {
            theme
                .button_with(
                    id,
                    label,
                    ButtonKind::Tonal,
                    false,
                    Message::DialogDemo(event),
                )
                .accessible_activation()
        };
        let title = theme.heading("dialog-demo-title", "對話框");
        let controls = control_row(vec![
            theme
                .tonal_button(BACK, "返回", Message::DialogDemo(Back))
                .restore_focus_on_removal(self.demo_opener())
                .accessible_activation(),
            theme
                .switch(
                    "dialog-demo-theme",
                    "深色",
                    ChoiceState {
                        selected: dark,
                        disabled: false,
                    },
                    Message::Theme,
                )
                .accessible_activation(),
            toggle("dialog-demo-rtl", "右起", self.rtl, ToggleDirection),
            toggle(
                "dialog-demo-large-text",
                "大字",
                self.large_text,
                ToggleLargeText,
            ),
        ]);
        let saved = theme.text(SAVED, format!("已儲存：{}", self.saved));
        let mut page = if self.transform_preview {
            // Keep the scroll clip above the fixed FAB, including when text wraps.
            let page_top_inset = 28.;
            let fab_clearance = 8.;
            let scroll_height =
                (self.viewport.height - FAB_INSET - FAB_SIZE - page_top_inset - fab_clearance)
                    .max(0.);
            let content = View::new(
                Node::new(Role::Group, "對話框設定")
                    .id(FAB_PREVIEW_SCROLL)
                    .styled(
                        Style::column()
                            .width(Length::Percent(1.))
                            .height(Length::Px(scroll_height))
                            .gap(8.)
                            .clip(true),
                    ),
            )
            .accessibility_scroll_pages()
            .children([title, controls, saved]);
            vec![content, self.fab_launcher(theme)]
        } else {
            vec![
                title,
                controls,
                control_row(vec![
                    preview(BASIC_OPEN, "基本對話框", OpenBasic),
                    preview(ALERT_OPEN, "重要提示", OpenAlert),
                    preview(EDIT_OPEN, EDIT_TITLE, OpenEdit),
                ]),
                saved,
            ]
        };
        page.extend(self.basic(theme, matches!(self.active, Some(Active::Basic))));
        page.extend(self.alert(theme, matches!(self.active, Some(Active::Alert))));
        let mut ask_discard = false;
        if let Some(Active::Edit {
            draft,
            ask_discard: ask,
        }) = &self.active
        {
            page.extend(self.editor(theme, draft, true));
            ask_discard = *ask;
        } else if let Some(draft) = &self.retiring_editor {
            page.extend(self.editor(theme, draft, false));
        }
        page.extend(self.confirmation(theme, ask_discard));
        theme.page(page)
    }

    fn basic(&self, theme: &Theme, open: bool) -> Option<View<Message>> {
        use DialogDemoEvent::CloseBasic;
        if !self.basic_motion.should_mount(open) {
            return None;
        }
        let mut options = self.basic_options(BASIC, "行程通知");
        options.restore_focus_to = Some(BASIC_OPEN);
        options.initial_focus = Some("dialog-demo-basic-confirm");
        let extra = [
            ("reminders", "提醒時間", "行程開始前一天及一小時發送提醒。"),
            ("timezone", "時區", "依照行程所在地的時間顯示。"),
            ("invitees", "受邀者", "只有你和受邀者能查看行程。"),
            ("updates", "變更通知", "時間或地點變更時會通知受邀者。"),
            ("repeat", "重複行程", "每次調整前可選擇是否套用到整個系列。"),
            ("offline", "離線草稿", "尚未同步的編輯保留在此裝置。"),
        ]
        .map(|(key, title, text)| {
            View::new(
                Node::new(Role::Group, "")
                    .styled(Style::column().width(Length::Percent(1.)).gap(4.)),
            )
            .children([
                self.body_text(
                    theme,
                    &format!("{BASIC}-{key}-title"),
                    title,
                    typography::TITLE_MEDIUM,
                ),
                self.body_text(
                    theme,
                    &format!("{BASIC}-{key}"),
                    text,
                    typography::BODY_MEDIUM,
                ),
            ])
        })
        .into_iter()
        .collect();
        theme.basic_dialog_with_motion(
            options,
            BasicDialogContent {
                extra,
                confirm: ("我已閱讀", Message::DialogDemo(CloseBasic)),
                cancel: Some(("稍後再看", Message::DialogDemo(CloseBasic))),
                on_dismiss: Message::DialogDemo(CloseBasic),
            },
            open,
            &self.basic_motion,
        )
    }

    fn alert(&self, theme: &Theme, open: bool) -> Option<View<Message>> {
        use DialogDemoEvent::Acknowledge;
        let mut options = self.basic_options(ALERT, "連線中斷");
        options.alert = true;
        options.dismissal = ModalDismissal::ActionOnly;
        options.supporting = Some("目前無法同步行程。請確認網路連線後再試。");
        options.restore_focus_to = Some(ALERT_OPEN);
        theme.basic_dialog_with_motion(
            options,
            BasicDialogContent {
                extra: Vec::new(),
                confirm: ("知道了", Message::DialogDemo(Acknowledge)),
                cancel: None,
                on_dismiss: Message::DialogDemo(Acknowledge),
            },
            open,
            &self.alert_motion,
        )
    }

    fn editor(&self, theme: &Theme, value: &str, open: bool) -> Option<View<Message>> {
        use DialogDemoEvent::{Input, RequestClose, Save};
        let editor = View::new(
            Node::new(Role::TextInput, "行程名稱")
                .id(EDITOR)
                .text(value)
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .height(Length::Px(if self.large_text { 72. } else { 56. }))
                        .padding(8.)
                        .corners(shape::EXTRA_SMALL)
                        .background(theme.colors.surface_container_highest)
                        .foreground(theme.colors.on_surface)
                        .with_text_style(typography::BODY_LARGE),
                ),
        )
        .on_input(|value| Message::DialogDemo(Input(value)))
        .cursor(CursorStyle::Text)
        .center_single_line_text()
        .accessibility_scroll_into_view();
        let editor = if self.rtl {
            editor.right_align_single_line_text()
        } else {
            editor
        };
        let body = View::new(
            Node::new(Role::Group, "行程內容")
                .id("dialog-demo-edit-form")
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .gap(16.)
                        .align(if self.rtl { Align::End } else { Align::Start }),
                ),
        )
        .children([
            self.body_text(
                theme,
                "dialog-demo-editor-label",
                "行程名稱",
                typography::LABEL_LARGE,
            ),
            editor,
            self.body_text(
                theme,
                "dialog-demo-edit-when",
                "時間　週六上午十點",
                typography::BODY_LARGE,
            ),
            self.body_text(
                theme,
                "dialog-demo-edit-where",
                "地點　市立圖書館",
                typography::BODY_LARGE,
            ),
            self.body_text(
                theme,
                "dialog-demo-edit-who",
                "分享　只有自己",
                typography::BODY_LARGE,
            ),
            self.body_text(
                theme,
                "dialog-demo-edit-notice",
                "提醒　開始前一小時",
                typography::BODY_LARGE,
            ),
        ]);
        if self.is_full_screen() {
            if self.transform_preview {
                let anchor = self.fab_anchor.map(|anchor| FullScreenDialogAnchor {
                    fill: theme.colors.primary_container,
                    ..anchor
                });
                return theme.full_screen_dialog_with_motion(
                    self.full_options(),
                    body,
                    Message::DialogDemo(RequestClose),
                    Message::DialogDemo(Save),
                    FullScreenDialogTransition {
                        open,
                        anchor,
                        motion: &self.full_motion,
                    },
                );
            }
            return theme.full_screen_dialog(
                self.full_options(),
                body,
                Message::DialogDemo(RequestClose),
                Message::DialogDemo(Save),
            );
        }
        if !open {
            return None;
        }
        let mut options = self.basic_options(EDIT, EDIT_TITLE);
        options.dismissal = ModalDismissal::Escape;
        options.initial_focus = Some(EDITOR);
        options.restore_focus_to = Some(self.editor_opener());
        theme.basic_dialog(
            options,
            BasicDialogContent {
                extra: vec![body],
                confirm: ("儲存", Message::DialogDemo(Save)),
                cancel: Some(("關閉", Message::DialogDemo(RequestClose))),
                on_dismiss: Message::DialogDemo(RequestClose),
            },
        )
    }

    fn confirmation(&self, theme: &Theme, open: bool) -> Option<View<Message>> {
        use DialogDemoEvent::{Discard, KeepEditing};
        let mut options = self.basic_options(DISCARD, "捨棄變更？");
        options.supporting = Some("未儲存的行程變更會遺失。");
        options.dismissal = ModalDismissal::Escape;
        options.initial_focus = Some("dialog-demo-discard-cancel");
        options.restore_focus_to = Some(EDITOR);
        theme.basic_dialog_with_motion(
            options,
            BasicDialogContent {
                extra: Vec::new(),
                confirm: ("捨棄變更", Message::DialogDemo(Discard)),
                cancel: Some(("繼續編輯", Message::DialogDemo(KeepEditing))),
                on_dismiss: Message::DialogDemo(KeepEditing),
            },
            open,
            &self.discard_motion,
        )
    }

    fn body_text(
        &self,
        theme: &Theme,
        id: &str,
        caption: &str,
        style: surface::TextStyle,
    ) -> View<Message> {
        View::new(
            Node::new(Role::Text, caption)
                .id(id)
                .text(caption)
                .shrink_to_fit_width()
                .styled(
                    Style::column()
                        .with_text_style(style)
                        .foreground(theme.colors.on_surface_variant),
                ),
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        let (open, presentation, draft) = match &self.active {
            None => (None, None, self.saved.as_str()),
            Some(Active::Basic) => (Some("basic"), Some("basic"), self.saved.as_str()),
            Some(Active::Alert) => (Some("alert"), Some("basic"), self.saved.as_str()),
            Some(Active::Edit { draft, ask_discard }) => (
                Some(if *ask_discard { "discard" } else { "edit" }),
                Some(if self.is_full_screen() {
                    "fullscreen"
                } else {
                    "basic"
                }),
                draft.as_str(),
            ),
        };
        serde_json::json!({
            "visible": self.visible, "open": open, "presentation": presentation, "draft": draft,
            "saved": self.saved, "saves": self.saves, "discards": self.discards, "alerts": self.alerts,
            "rtl": self.rtl, "text_scale": self.text_scale(),
            "transform": {
                "preview": self.transform_preview,
                "anchor": self.fab_anchor.map(|anchor| anchor.bounds),
                "progress": self.full_motion.progress(),
                "container": self.full_motion.container_bounds(self.fab_anchor, self.viewport),
                "retiring": self.retiring_editor.is_some(),
            },
            "opacity": {
                "basic": self.basic_motion.opacity(),
                "alert": self.alert_motion.opacity(),
                "discard": self.discard_motion.opacity(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallery;
    use surface::{ExtendedDisplayList, ExtendedPaintOp, TextMeasurer};
    use surface_app::{Event, ImeEvent, Key, KeyPress, Modifiers, Runtime};

    struct Fonts;

    impl TextMeasurer for Fonts {
        fn measure(&mut self, value: &str, size: f32, width: Option<f32>) -> (f32, f32) {
            let natural = value
                .chars()
                .map(|character| size * if character.is_ascii() { 0.58 } else { 1. })
                .sum::<f32>();
            let width = width.unwrap_or(natural).min(natural);
            (width, (natural / width.max(1.)).ceil().max(1.) * size * 1.5)
        }
    }

    fn draw(runtime: &mut Runtime<Gallery>, (width, height): (f32, f32)) {
        for _ in 0..2 {
            runtime.render(width, height, &mut Fonts).unwrap();
        }
    }

    fn route(viewport: (f32, f32), compact_profile: bool) -> Runtime<Gallery> {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        draw(&mut runtime, viewport);
        runtime.apply(Message::Section(SECTION)).unwrap();
        runtime.activate(SHOW).unwrap();
        if compact_profile {
            for choice in [
                "dialog-demo-theme",
                "dialog-demo-rtl",
                "dialog-demo-large-text",
            ] {
                runtime.activate(choice).unwrap();
            }
        }
        draw(&mut runtime, viewport);
        runtime
    }

    fn rect(runtime: &Runtime<Gallery>, id: &str) -> Rect {
        runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
    }

    fn active_panel(runtime: &Runtime<Gallery>, id: &str) {
        assert_eq!(
            runtime.accessibility_frame().unwrap().modal_root(),
            runtime.dom().by_id(id)
        );
    }

    fn click(runtime: &mut Runtime<Gallery>, x: f32, y: f32) {
        runtime.dispatch(Event::PointerDown { x, y }).unwrap();
        runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    }

    fn set_input(runtime: &mut Runtime<Gallery>, value: &str, anchor: usize, cursor: usize) {
        runtime
            .dispatch(Event::InputValue {
                value: value.into(),
                anchor,
                cursor,
            })
            .unwrap();
    }

    fn timed_frame(
        runtime: &mut Runtime<Gallery>,
        (width, height): (f32, f32),
        now: f64,
    ) -> ExtendedDisplayList {
        runtime.set_time(now);
        runtime.render_extended(width, height, &mut Fonts).unwrap()
    }

    fn transform(frame: &ExtendedDisplayList) -> Option<surface::PaintTransform> {
        frame.ops().iter().find_map(|op| match op {
            ExtendedPaintOp::BeginTransformGroup { transform } => Some(*transform),
            _ => None,
        })
    }

    fn transform_route(viewport: (f32, f32), rtl: bool) -> Runtime<Gallery> {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        draw(&mut runtime, viewport);
        runtime.apply(Message::Section(SECTION)).unwrap();
        runtime.activate(SHOW_TRANSFORM).unwrap();
        if rtl {
            runtime.activate("dialog-demo-rtl").unwrap();
        }
        draw(&mut runtime, viewport);
        runtime
    }

    #[test]
    fn resting_fab_remeasures_after_resize_without_an_input_event() {
        let wide = (960., 1800.);
        let compact = (320., 400.);
        for reduced in [false, true] {
            let mut runtime = transform_route(wide, true);
            runtime.set_reduced_motion(reduced);
            assert_eq!(
                runtime.app().dialog_demo.fab_anchor.unwrap().bounds,
                Rect::new(24., 1720., 56., 56.)
            );

            timed_frame(&mut runtime, compact, 100.);
            let fab = rect(&runtime, FAB_OPEN);
            assert_eq!(fab, Rect::new(24., 320., 56., 56.));
            assert!(runtime.needs_frame(), "the new layout needs a measurement");
            assert!(runtime.app().dialog_demo.fab_anchor.is_none());

            timed_frame(&mut runtime, compact, 116.);
            assert_eq!(runtime.app().dialog_demo.fab_anchor.unwrap().bounds, fab);
            assert!(!runtime.needs_frame(), "the measured preview can stay idle");

            runtime.activate("dialog-demo-rtl").unwrap();
            timed_frame(&mut runtime, compact, 132.);
            assert!(runtime.needs_frame());
            assert!(runtime.app().dialog_demo.fab_anchor.is_none());
            timed_frame(&mut runtime, compact, 148.);
            assert_eq!(
                runtime.app().dialog_demo.fab_anchor.unwrap().bounds,
                Rect::new(240., 320., 56., 56.)
            );
            timed_frame(&mut runtime, compact, 800.);
            assert!(!runtime.needs_frame());
        }
    }

    #[test]
    fn measured_fab_starts_modal_and_reverses_transformed_exit() {
        let viewport = (320., 400.);
        for rtl in [false, true] {
            let mut runtime = transform_route(viewport, rtl);
            let fab = rect(&runtime, FAB_OPEN);
            for launcher in [BASIC_OPEN, ALERT_OPEN, EDIT_OPEN] {
                assert!(runtime.dom().by_id(launcher).is_none());
            }
            assert_eq!(fab, Rect::new(if rtl { 24. } else { 240. }, 320., 56., 56.));
            click(&mut runtime, fab.x + 28., fab.y + 28.);

            let first = timed_frame(&mut runtime, viewport, 100.);
            let start = transform(&first).expect("entry from actual FAB");
            assert_eq!(start.clip, fab);
            assert_eq!(start.radius, shape::LARGE.top_left);
            assert_eq!(start.opacity, 0.);
            active_panel(&runtime, EDIT);
            assert_eq!(runtime.focused(), Some(EDITOR));
            assert!(runtime.activate(FAB_OPEN).is_err());

            let halfway = timed_frame(&mut runtime, viewport, 160.);
            let open = transform(&halfway).expect("intermediate transform");
            assert!(open.clip.width > fab.width && open.clip.width < viewport.0);
            assert!(open.clip.height > fab.height && open.clip.height < viewport.1);
            assert!(open.radius > 0. && open.radius < start.radius);
            assert!(open.opacity > 0. && open.opacity < 1.);
            assert_eq!(open.destination.x, open.clip.x);
            assert_eq!(open.destination.y, open.clip.y);
            assert_eq!(open.destination.width, open.clip.width);
            assert_eq!(
                rect(&runtime, EDIT),
                Rect::new(0., 0., viewport.0, viewport.1)
            );

            let full = timed_frame(&mut runtime, viewport, 650.);
            assert!(transform(&full).is_none());
            assert!(!runtime.needs_frame());
            runtime.dispatch(Event::Cancel).unwrap();
            assert_eq!(runtime.focused(), Some(FAB_OPEN));
            timed_frame(&mut runtime, viewport, 700.);
            let out = timed_frame(&mut runtime, viewport, 760.);
            let exit = transform(&out).expect("inert full-screen exit");
            assert!(exit.clip.width > fab.width && exit.clip.width < viewport.0);
            let accessibility = runtime.accessibility_frame().unwrap();
            assert_eq!(accessibility.modal_root(), None);
            assert!(accessibility
                .node(runtime.dom().by_id(EDIT).unwrap())
                .is_none());
            assert!(runtime.activate("dialog-demo-edit-action").is_err());

            click(&mut runtime, fab.x + 28., fab.y + 28.);
            let reversal = timed_frame(&mut runtime, viewport, 760.);
            assert_eq!(transform(&reversal), Some(exit));
            active_panel(&runtime, EDIT);
            assert_eq!(runtime.focused(), Some(EDITOR));
            let settled = timed_frame(&mut runtime, viewport, 1340.);
            assert!(transform(&settled).is_none());
        }
    }

    #[test]
    fn short_fab_preview_scrolls_text_above_the_fixed_button() {
        let viewport = (320., 240.);
        let mut runtime = transform_route(viewport, true);
        runtime.activate("dialog-demo-large-text").unwrap();
        draw(&mut runtime, viewport);

        let fab = rect(&runtime, FAB_OPEN);
        let scroller = rect(&runtime, FAB_PREVIEW_SCROLL);
        let saved = rect(&runtime, SAVED);
        assert_eq!(scroller.y + scroller.height + 8., fab.y);
        assert!(saved.y + saved.height > scroller.y + scroller.height);
        runtime
            .dispatch(Event::Scroll {
                x: scroller.x + scroller.width / 2.,
                y: scroller.y + scroller.height / 2.,
                delta_x: 0.,
                delta_y: 800.,
            })
            .unwrap();
        draw(&mut runtime, viewport);
        let revealed = rect(&runtime, SAVED);
        assert_eq!(scroller.intersect(revealed), revealed);
        assert_eq!(rect(&runtime, FAB_OPEN), fab);
        click(
            &mut runtime,
            fab.x + fab.width / 2.,
            fab.y + fab.height / 2.,
        );
        draw(&mut runtime, viewport);
        active_panel(&runtime, EDIT);
    }

    #[test]
    fn resizing_a_transform_keeps_editor_value_selection_and_composition() {
        let mut runtime = transform_route((320., 400.), true);
        runtime.activate(FAB_OPEN).unwrap();
        timed_frame(&mut runtime, (320., 400.), 100.);
        timed_frame(&mut runtime, (320., 400.), 160.);
        set_input(&mut runtime, "秋日散步", 12, 9);
        runtime
            .dispatch(Event::Ime(surface_app::ImeEvent::Preedit {
                text: "ㄅ".into(),
                selection: None,
            }))
            .unwrap();
        let before = runtime.input_snapshot().unwrap();
        for viewport in [(360., 420.), (600., 550.), (320., 400.)] {
            timed_frame(&mut runtime, viewport, 180.);
            let after = runtime.input_snapshot().unwrap();
            assert_eq!(after.id, EDITOR);
            assert_eq!(after.value, before.value);
            assert_eq!(after.selection, before.selection);
            assert_eq!(after.composing, before.composing);
            assert!(after.composing);
            active_panel(&runtime, EDIT);
        }
    }

    fn opacity_group(frame: &ExtendedDisplayList) -> Option<f32> {
        frame.ops().iter().find_map(|op| match op {
            ExtendedPaintOp::BeginOpacityGroup { opacity } => Some(*opacity),
            _ => None,
        })
    }

    fn scrim_alpha(frame: &ExtendedDisplayList) -> Option<u8> {
        frame.ops().iter().find_map(|op| match op {
            ExtendedPaintOp::FillPath(path) if path.rect() == frame.viewport() => {
                Some(path.color().3)
            }
            _ => None,
        })
    }

    #[test]
    fn basic_fade_keeps_live_focus_and_semantics_and_reverses_its_inert_exit() {
        let viewport = (960., 800.);
        let mut runtime = route(viewport, false);
        runtime.activate(BASIC_OPEN).unwrap();
        let start = timed_frame(&mut runtime, viewport, 100.);
        assert_eq!(opacity_group(&start), Some(0.));
        assert_eq!(scrim_alpha(&start), None);
        active_panel(&runtime, BASIC);
        assert_eq!(runtime.focused(), Some("dialog-demo-basic-confirm"));
        assert!(runtime.needs_frame());
        let geometry = rect(&runtime, BASIC);
        let revision = runtime.accessibility_frame().unwrap().generation();
        let halfway = timed_frame(&mut runtime, viewport, 140.);
        let expected = 1. - 2.6_f32 * (-1.6_f32).exp();
        let open_alpha = opacity_group(&halfway).unwrap();
        assert!((open_alpha - expected).abs() < 0.001);
        assert_eq!(
            scrim_alpha(&halfway),
            Some((82. * open_alpha).round() as u8)
        );
        assert_eq!(
            runtime.accessibility_frame().unwrap().generation(),
            revision
        );
        assert_eq!(rect(&runtime, BASIC), geometry);
        let full = timed_frame(&mut runtime, viewport, 600.);
        assert_eq!(opacity_group(&full), None);
        assert_eq!(scrim_alpha(&full), Some(82));
        assert!(!runtime.needs_frame());

        click(&mut runtime, 2., 2.);
        assert_eq!(runtime.focused(), Some(BASIC_OPEN));
        timed_frame(&mut runtime, viewport, 660.);
        let closing = timed_frame(&mut runtime, viewport, 700.);
        let exit_alpha = opacity_group(&closing).unwrap();
        assert!((exit_alpha + expected - 1.).abs() < 0.001);
        assert_eq!(
            scrim_alpha(&closing),
            Some((82. * exit_alpha).round() as u8)
        );
        let retained = runtime.dom().by_id(BASIC).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        assert!(frame.node(retained).is_none());
        assert_eq!(frame.modal_root(), None);
        assert!(runtime.activate("dialog-demo-basic-confirm").is_err());
        assert_eq!(rect(&runtime, BASIC), geometry);

        let opener = rect(&runtime, BASIC_OPEN);
        click(
            &mut runtime,
            opener.x + opener.width / 2.,
            opener.y + opener.height / 2.,
        );
        let reversed = timed_frame(&mut runtime, viewport, 700.);
        assert_eq!(opacity_group(&reversed), Some(exit_alpha));
        active_panel(&runtime, BASIC);
        assert_eq!(runtime.focused(), Some("dialog-demo-basic-confirm"));
        timed_frame(&mut runtime, viewport, 800.);
        assert!(runtime.app().dialog_demo.basic_motion.opacity() > exit_alpha);
        runtime.dispatch(Event::Cancel).unwrap();
        timed_frame(&mut runtime, viewport, 830.);
        let done = timed_frame(&mut runtime, viewport, 1350.);
        assert!(!runtime.needs_frame());
        assert!(runtime.dom().by_id(BASIC).is_none());
        assert_eq!(opacity_group(&done), None);
        assert_eq!(scrim_alpha(&done), None);
        assert_eq!(runtime.focused(), Some(BASIC_OPEN));
    }

    #[test]
    fn retiring_nested_discard_restores_focus_and_keeps_the_draft() {
        let viewport = (320., 400.);
        let mut runtime = route(viewport, true);
        runtime.activate(EDIT_OPEN).unwrap();
        timed_frame(&mut runtime, viewport, 20.);
        set_input(&mut runtime, "秋日散步", 12, 12);
        runtime.dispatch(Event::Cancel).unwrap();
        assert_eq!(
            opacity_group(&timed_frame(&mut runtime, viewport, 50.)),
            Some(0.)
        );
        let partly_open = timed_frame(&mut runtime, viewport, 100.);
        let opacity = opacity_group(&partly_open).unwrap();
        assert!((0. ..1.).contains(&opacity));
        active_panel(&runtime, DISCARD);
        assert_eq!(runtime.focused(), Some("dialog-demo-discard-cancel"));
        assert!(runtime.activate("dialog-demo-edit-action").is_err());

        runtime.activate("dialog-demo-discard-cancel").unwrap();
        let first_exit = timed_frame(&mut runtime, viewport, 100.);
        assert_eq!(opacity_group(&first_exit), Some(opacity));
        active_panel(&runtime, EDIT);
        assert_eq!(runtime.focused(), Some(EDITOR));
        assert_eq!(runtime.editor(EDITOR).unwrap().value(), "秋日散步");
        assert!(runtime.activate("dialog-demo-discard-confirm").is_err());
        let exiting = timed_frame(&mut runtime, viewport, 160.);
        let exit_opacity = opacity_group(&exiting).unwrap();
        assert!((0. ..opacity).contains(&exit_opacity));

        runtime.dispatch(Event::Cancel).unwrap();
        let reversed = timed_frame(&mut runtime, viewport, 160.);
        assert_eq!(opacity_group(&reversed), Some(exit_opacity));
        active_panel(&runtime, DISCARD);
        runtime.set_reduced_motion(true);
        let restored = timed_frame(&mut runtime, viewport, 180.);
        assert_eq!(opacity_group(&restored), None);
        assert_eq!(scrim_alpha(&restored), Some(82));
        assert!(!runtime.needs_frame());
        runtime.activate("dialog-demo-discard-cancel").unwrap();
        timed_frame(&mut runtime, viewport, 181.);
        assert!(runtime.dom().by_id(DISCARD).is_none());
        assert_eq!(runtime.focused(), Some(EDITOR));
        assert_eq!(runtime.editor(EDITOR).unwrap().value(), "秋日散步");
        assert_eq!(runtime.app().dialog_demo.discards, 0);
        assert_eq!(runtime.app().dialog_demo.saves, 0);
    }

    #[test]
    fn basic_scrolls_under_pinned_actions_and_alert_requires_its_button() {
        let viewport = (320., 400.);
        let mut runtime = route(viewport, true);
        runtime.activate(BASIC_OPEN).unwrap();
        draw(&mut runtime, viewport);
        active_panel(&runtime, BASIC);
        let body = rect(&runtime, "dialog-demo-basic-body");
        let cancel = rect(&runtime, "dialog-demo-basic-cancel");
        let confirm = rect(&runtime, "dialog-demo-basic-confirm");
        assert!(body.height > 0.);
        assert_eq!(runtime.focused(), Some("dialog-demo-basic-confirm"));
        assert!(confirm.height >= 48. && confirm.y + confirm.height <= cancel.y);
        let panel = rect(&runtime, BASIC);
        for target in [confirm, cancel] {
            assert_eq!(panel.intersect(target), target);
        }
        let trailing = rect(&runtime, "dialog-demo-basic-offline").y;
        runtime
            .dispatch(Event::Scroll {
                x: body.x + body.width / 2.,
                y: body.y + body.height / 2.,
                delta_x: 0.,
                delta_y: 260.,
            })
            .unwrap();
        draw(&mut runtime, viewport);
        assert!(rect(&runtime, "dialog-demo-basic-offline").y < trailing);
        assert_eq!(rect(&runtime, "dialog-demo-basic-cancel"), cancel);
        assert_eq!(rect(&runtime, "dialog-demo-basic-confirm"), confirm);
        runtime.dispatch(Event::Cancel).unwrap();
        assert_eq!(runtime.focused(), Some(BASIC_OPEN));

        runtime.activate(ALERT_OPEN).unwrap();
        draw(&mut runtime, viewport);
        active_panel(&runtime, ALERT);
        assert!(runtime
            .dom()
            .get(runtime.dom().by_id(ALERT).unwrap())
            .unwrap()
            .is_alert_dialog());
        runtime.dispatch(Event::Cancel).unwrap();
        draw(&mut runtime, viewport);
        click(&mut runtime, 2., 2.);
        draw(&mut runtime, viewport);
        active_panel(&runtime, ALERT);
        assert_eq!(runtime.app().dialog_demo.alerts, 0);
        runtime.activate("dialog-demo-alert-confirm").unwrap();
        assert_eq!(runtime.app().dialog_demo.alerts, 1);
        assert_eq!(runtime.focused(), Some(ALERT_OPEN));
    }

    #[test]
    fn edit_breakpoint_preserves_the_active_editor_selection_and_preedit() {
        let compact = (599., 500.);
        let wide = (600., 500.);
        let mut runtime = route(compact, false);
        runtime.activate(EDIT_OPEN).unwrap();
        draw(&mut runtime, compact);
        active_panel(&runtime, EDIT);
        assert!(runtime.dom().by_id("dialog-demo-edit-header").is_some());
        assert_eq!(runtime.focused(), Some(EDITOR));
        set_input(&mut runtime, "秋日散步", 12, 3);
        runtime
            .dispatch(Event::Ime(ImeEvent::Preedit {
                text: "ㄅ".into(),
                selection: None,
            }))
            .unwrap();
        draw(&mut runtime, compact);
        let before = runtime.input_snapshot().unwrap();
        assert!(before.composing);
        for (viewport, full) in [(wide, false), ((320., 400.), true)] {
            draw(&mut runtime, viewport);
            active_panel(&runtime, EDIT);
            assert!(runtime.dom().by_id("dialog-demo-edit-body").is_some());
            assert_eq!(
                runtime.dom().by_id("dialog-demo-edit-header").is_some(),
                full
            );
            assert_eq!(
                runtime.dom().by_id("dialog-demo-edit-actions").is_some(),
                !full
            );
            let now = runtime.input_snapshot().unwrap();
            assert_eq!(now.id, EDITOR);
            assert_eq!(now.value, before.value);
            assert_eq!(now.selection, before.selection);
            assert!(now.composing);
            assert_eq!(
                runtime.editor(EDITOR).unwrap().composition().unwrap().text,
                "ㄅ"
            );
        }
        assert!(!runtime
            .key(&KeyPress::new(Key::Enter, Modifiers::default()))
            .unwrap());
        runtime.dispatch(Event::Cancel).unwrap();
        draw(&mut runtime, (320., 400.));
        active_panel(&runtime, EDIT);
        assert!(!runtime.input_snapshot().unwrap().composing);
        runtime.dispatch(Event::Cancel).unwrap();
        draw(&mut runtime, (320., 400.));
        active_panel(&runtime, DISCARD);
        assert_eq!(runtime.focused(), Some("dialog-demo-discard-cancel"));
        assert_eq!(runtime.app().dialog_demo.saved, INITIAL);
        assert_eq!(runtime.app().dialog_demo.saves, 0);
    }

    #[test]
    fn caller_retains_drafts_and_commits_or_discards_once() {
        let viewport = (320., 400.);
        let mut runtime = route(viewport, true);
        runtime.activate(EDIT_OPEN).unwrap();
        draw(&mut runtime, viewport);
        set_input(&mut runtime, "秋日散步", 12, 12);
        draw(&mut runtime, viewport);
        let close = rect(&runtime, "dialog-demo-edit-close");
        click(&mut runtime, close.x + 24., close.y + 24.);
        draw(&mut runtime, viewport);
        active_panel(&runtime, DISCARD);
        assert!(runtime.activate("dialog-demo-edit-action").is_err());
        runtime.activate("dialog-demo-discard-cancel").unwrap();
        draw(&mut runtime, viewport);
        active_panel(&runtime, EDIT);
        assert_eq!(runtime.editor(EDITOR).unwrap().value(), "秋日散步");
        runtime.activate("dialog-demo-edit-action").unwrap();
        assert_eq!(runtime.app().dialog_demo.saves, 1);
        assert_eq!(runtime.app().dialog_demo.saved, "秋日散步");
        assert_eq!(runtime.focused(), Some(EDIT_OPEN));
        runtime
            .apply(Message::DialogDemo(DialogDemoEvent::Save))
            .unwrap();
        assert_eq!(runtime.app().dialog_demo.saves, 1);

        runtime.activate(EDIT_OPEN).unwrap();
        draw(&mut runtime, viewport);
        set_input(&mut runtime, "待捨棄", 9, 9);
        runtime.dispatch(Event::Cancel).unwrap();
        draw(&mut runtime, viewport);
        active_panel(&runtime, DISCARD);
        runtime.activate("dialog-demo-discard-confirm").unwrap();
        assert_eq!(runtime.app().dialog_demo.discards, 1);
        assert_eq!(runtime.app().dialog_demo.saved, "秋日散步");
        assert_eq!(runtime.focused(), Some(EDIT_OPEN));
        runtime.activate(BACK).unwrap();
        draw(&mut runtime, viewport);
        assert_eq!(runtime.focused(), Some(SHOW));
        assert!(runtime.dom().by_id("open-dialog").is_some());
    }
}
