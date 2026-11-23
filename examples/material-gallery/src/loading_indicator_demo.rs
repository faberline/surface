use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    typography, ButtonKind, ChoiceState, LoadingIndicatorMotion, LoadingIndicatorOptions,
    LoadingKind, Theme,
};

use crate::Message;

pub(crate) const SECTION: usize = 1;
const SHOW: &str = "loading-demo-show";
const BACK: &str = "loading-demo-back";
const PAGE_PADDING: f32 = 12.;
const PANEL_PADDING: f32 = 16.;
const GAP: f32 = 12.;
const SIZES: [(&str, &str, f32); 3] = [
    ("loading-demo-small", "24 dp", 24.),
    ("loading-demo-regular", "48 dp", 48.),
    ("loading-demo-large", "240 dp", 240.),
];

#[derive(Clone, Copy)]
pub enum LoadingEvent {
    Show,
    Back,
    RightToLeft,
    LargeText,
    Size(f32),
    Run,
    Stop,
    Reset,
}

pub(crate) struct LoadingDemo {
    motion: LoadingIndicatorMotion,
    visible: bool,
    running: bool,
    reduced_motion: bool,
    requests_frame: bool,
    rtl: bool,
    large_text: bool,
    size: f32,
    width: f32,
}

impl Default for LoadingDemo {
    fn default() -> Self {
        Self {
            motion: LoadingIndicatorMotion::default(),
            visible: false,
            running: false,
            reduced_motion: false,
            requests_frame: false,
            rtl: false,
            large_text: false,
            size: 48.,
            width: 960.,
        }
    }
}

impl LoadingDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.stop();
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        if !width.is_finite() || self.width == width {
            return false;
        }
        self.width = width;
        true
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        self.reduced_motion = frame.reduced_motion;
        let update = self.motion.advance(frame, self.visible && self.running);
        self.requests_frame = update.request_next;
        update
    }

    fn stop(&mut self) {
        self.running = false;
        self.motion = LoadingIndicatorMotion::default();
        self.requests_frame = false;
    }

    pub(crate) fn update(&mut self, event: LoadingEvent) {
        match event {
            LoadingEvent::Show => {
                self.visible = true;
                self.running = true;
            }
            LoadingEvent::Back => self.leave(),
            LoadingEvent::RightToLeft => self.rtl = !self.rtl,
            LoadingEvent::LargeText => self.large_text = !self.large_text,
            LoadingEvent::Size(size) => self.size = size,
            LoadingEvent::Run => self.running = true,
            LoadingEvent::Stop => self.stop(),
            LoadingEvent::Reset => self.motion = LoadingIndicatorMotion::default(),
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "載入指示器",
                ButtonKind::Outlined,
                false,
                Message::Loading(LoadingEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn available_width(&self) -> f32 {
        (self.width - 2. * PAGE_PADDING).max(0.)
    }

    fn indicator_size(&self) -> f32 {
        self.size
            .min((self.available_width() - 2. * PANEL_PADDING).max(24.))
    }

    fn panel_width(&self) -> f32 {
        let available = self.available_width();
        let half = (available - GAP) / 2.;
        if half >= self.indicator_size() + 2. * PANEL_PADDING {
            half
        } else {
            available
        }
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        let choice = |selected| ChoiceState {
            selected,
            disabled: false,
        };
        let toggle = |id, label, selected, event| {
            theme
                .segmented_button(id, label, choice(selected), Message::Loading(event))
                .accessible_activation()
        };
        let button = |id, label, event| {
            theme
                .button_with(
                    id,
                    label,
                    ButtonKind::Outlined,
                    false,
                    Message::Loading(event),
                )
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
        let options = row(vec![
            theme
                .tonal_button(BACK, "返回", Message::Loading(LoadingEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme
                .switch("loading-demo-theme", "深色", choice(dark), Message::Theme)
                .accessible_activation(),
            toggle(
                "loading-demo-rtl",
                "右起",
                self.rtl,
                LoadingEvent::RightToLeft,
            ),
            toggle(
                "loading-demo-large-text",
                "大字",
                self.large_text,
                LoadingEvent::LargeText,
            ),
        ]);
        let sizes = row(SIZES
            .into_iter()
            .map(|(id, label, size)| toggle(id, label, self.size == size, LoadingEvent::Size(size)))
            .collect());
        let controls = row(vec![
            button("loading-demo-run", "開始", LoadingEvent::Run),
            button("loading-demo-stop", "停止", LoadingEvent::Stop),
            button("loading-demo-reset", "重設", LoadingEvent::Reset),
        ]);
        let scene = Node::new(Role::Group, "")
            .id("loading-demo-variants")
            .styled(
                Style::row()
                    .width(Length::Percent(1.))
                    .gap(GAP)
                    .wrap(true)
                    .align(Align::Start),
            );
        let scene = if self.rtl { scene.reverse_row() } else { scene };
        let variants = View::new(scene).children([
            self.panel(
                theme,
                "loading-default",
                "一般",
                "載入資料",
                LoadingKind::Default,
            ),
            self.panel(
                theme,
                "loading-contained",
                "有底色",
                "讀取快取",
                LoadingKind::Contained,
            ),
        ]);
        View::new(
            Node::new(Role::Window, "載入指示器")
                .id("loading-demo")
                .styled(
                    Style::column()
                        .padding(PAGE_PADDING)
                        .gap(GAP)
                        .clip(true)
                        .background(theme.colors.surface),
                ),
        )
        .children([options, sizes, controls, variants])
        .accessibility_scroll_pages()
    }

    fn panel(
        &self,
        theme: &Theme,
        id: &str,
        title: &str,
        name: &str,
        kind: LoadingKind,
    ) -> View<Message> {
        let size = self.indicator_size();
        let mut slot = View::new(
            Node::new(Role::Group, "").id(format!("{id}-slot")).styled(
                Style::column()
                    .width(Length::Px(size))
                    .height(Length::Px(size)),
            ),
        );
        if self.running {
            slot = slot.children([theme.loading_indicator_with_motion(
                LoadingIndicatorOptions {
                    id,
                    name,
                    kind,
                    size,
                },
                &self.motion,
            )]);
        }
        let title = View::new(
            Node::new(Role::Text, title).text(title).styled(
                Style::row()
                    .with_text_style(typography::TITLE_SMALL)
                    .foreground(theme.colors.on_surface),
            ),
        );
        View::new(
            Node::new(Role::Group, "").id(format!("{id}-panel")).styled(
                Style::column()
                    .width(Length::Px(self.panel_width()))
                    .padding(PANEL_PADDING)
                    .align(Align::Center)
                    .gap(GAP)
                    .radius(16.)
                    .background(theme.colors.surface_container),
            ),
        )
        .children([title, slot])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, theme: &Theme) -> serde_json::Value {
        let pose = self.motion.pose();
        let size = self.indicator_size();
        serde_json::json!({
            "visible": self.visible,
            "running": self.running,
            "reduced_motion": self.reduced_motion,
            "requests_frame": self.requests_frame,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "size": size,
            "requested_size": self.size,
            "active_size": size * 38. / 48.,
            "elapsed_ms": self.motion.elapsed_ms(),
            "shape_index": pose.shape_index,
            "morph": pose.morph,
            "rotation_degrees": pose.rotation_degrees,
            "palette": {
                "background": theme.colors.surface_container,
                "primary": theme.colors.primary,
                "primary_container": theme.colors.primary_container,
                "on_primary_container": theme.colors.on_primary_container
            }
        })
    }
}
