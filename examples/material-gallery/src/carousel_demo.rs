use surface::{Align, Color, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, CarouselInput, CarouselItem, CarouselKind, CarouselLayout,
    CarouselOptions, CarouselOrientation, CarouselState, ChipKind, ChoiceState, Theme,
};

use crate::Message;

pub(crate) const SECTION: usize = 1;
const SHOW: &str = "carousel-demo-show";
const BACK: &str = "carousel-demo-back";
const TRACK: &str = "carousel-track";
const SHOW_ALL: &str = "carousel-show-all";
const LIST_BACK: &str = "carousel-show-all-back";
const TITLES: [&str; 6] = ["山野", "海灣", "森林", "沙丘", "湖畔", "夜空"];
const ASPECT_RATIOS: [f32; 6] = [9. / 16., 16. / 9., 1., 4. / 3., 2. / 3., 3. / 2.];
const VARIANTS: [(CarouselKind, &str, &str, &str); 6] = [
    (
        CarouselKind::MultiBrowse,
        "multi-browse",
        "多項瀏覽",
        "多項",
    ),
    (CarouselKind::Uncontained, "uncontained", "自由捲動", "自由"),
    (
        CarouselKind::UncontainedMultiAspectRatio,
        "mixed",
        "混合比例",
        "混合",
    ),
    (CarouselKind::HeroStart, "hero-start", "起點主圖", "起點"),
    (CarouselKind::HeroCenter, "hero-center", "中央主圖", "中央"),
    (CarouselKind::FullScreen, "full-screen", "全幅", "全幅"),
];

#[derive(Clone)]
pub enum CarouselEvent {
    Show,
    Back,
    Kind(CarouselKind),
    Direction,
    TextScale,
    Input(CarouselInput),
    Step(i32),
    OpenList,
    CloseList,
    Activate(usize),
}

pub(crate) struct CarouselDemo {
    visible: bool,
    viewport: (f32, f32),
    kind: CarouselKind,
    state: CarouselState,
    rtl: bool,
    large_text: bool,
    reduced_motion: bool,
    settling: bool,
    show_all: bool,
    activations: usize,
    last_activated: Option<usize>,
}

impl Default for CarouselDemo {
    fn default() -> Self {
        Self {
            visible: false,
            viewport: (960., 800.),
            kind: CarouselKind::MultiBrowse,
            state: CarouselState::default(),
            rtl: false,
            large_text: false,
            reduced_motion: false,
            settling: false,
            show_all: false,
            activations: 0,
            last_activated: None,
        }
    }
}

impl CarouselDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.show_all = false;
        self.state = CarouselState::default();
        self.settling = false;
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        if self.viewport == (width, height) {
            return false;
        }
        if self.visible {
            let index = self.active();
            self.viewport = (width, height);
            self.send(CarouselInput::Focus(index));
        } else {
            self.viewport = (width, height);
        }
        self.visible
    }

    fn options(&self) -> CarouselOptions<'static> {
        let vertical = self.kind == CarouselKind::FullScreen;
        let height = if vertical {
            (self.viewport.1 * 0.57).clamp(180., 520.)
        } else {
            (self.viewport.0 * 0.43).clamp(176., 250.)
        };
        CarouselOptions {
            id: TRACK,
            name: "風景輪播",
            kind: self.kind,
            orientation: if vertical {
                CarouselOrientation::Vertical
            } else {
                CarouselOrientation::Horizontal
            },
            direction: if self.rtl {
                LayoutDirection::RightToLeft
            } else {
                LayoutDirection::LeftToRight
            },
            viewport: (self.viewport.0, height),
            preferred_item_extent: if self.viewport.0 < 520. { 190. } else { 258. },
            reduced_motion: self.reduced_motion,
        }
    }

    fn layout(&self) -> CarouselLayout {
        CarouselLayout::new(self.options(), &ASPECT_RATIOS)
    }

    fn active(&self) -> usize {
        self.state.active_index(&self.layout()).unwrap_or(0)
    }

    fn send(&mut self, input: CarouselInput) {
        let layout = self.layout();
        self.state.apply(input, &layout);
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if !self.visible || self.show_all {
            return AnimationUpdate::default();
        }
        let preference_changed = self.reduced_motion != frame.reduced_motion;
        self.reduced_motion = frame.reduced_motion;
        let layout = self.layout();
        let mut update = self
            .state
            .advance(frame.now_ms, self.reduced_motion, &layout);
        update.rebuild |= preference_changed;
        self.settling = update.request_next && !self.state.dragging();
        update
    }

    pub(crate) fn update(&mut self, event: CarouselEvent) {
        match event {
            CarouselEvent::Show => self.visible = true,
            CarouselEvent::Back => self.leave(),
            CarouselEvent::Kind(kind) => {
                if self.kind != kind {
                    self.kind = kind;
                    self.state = CarouselState::default();
                    self.settling = false;
                }
            }
            CarouselEvent::Direction => {
                let active = self.active();
                self.rtl = !self.rtl;
                self.send(CarouselInput::Focus(active));
            }
            CarouselEvent::TextScale => self.large_text = !self.large_text,
            CarouselEvent::Input(input) => self.send(input),
            CarouselEvent::Step(delta) => {
                let target = self
                    .state
                    .target_index(&self.layout())
                    .unwrap_or(0)
                    .saturating_add_signed(delta as isize)
                    .min(TITLES.len() - 1);
                self.send(CarouselInput::GoTo(target));
            }
            CarouselEvent::OpenList => {
                self.send(CarouselInput::Focus(self.active()));
                self.show_all = true;
                self.settling = false;
            }
            CarouselEvent::CloseList => self.show_all = false,
            CarouselEvent::Activate(index) => {
                self.activations += 1;
                self.last_activated = Some(index);
                self.send(CarouselInput::Focus(index));
                self.show_all = false;
            }
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "輪播",
                ButtonKind::Outlined,
                false,
                app(CarouselEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        if self.show_all {
            return self.list(theme);
        }
        let options = self.options();
        let items: Vec<_> = TITLES
            .iter()
            .enumerate()
            .map(|(index, title)| CarouselItem {
                id: card_id(index),
                label: item_name(index, title),
                aspect_ratio: ASPECT_RATIOS[index],
                content: self.scene(theme, index, title),
                on_activate: app(CarouselEvent::Activate(index)),
                disabled: false,
            })
            .collect();
        let layout = CarouselLayout::for_items(options, &items);
        let track = theme.carousel(options, &layout, &self.state, items, |input| {
            app(CarouselEvent::Input(input))
        });
        Self::page(theme, "carousel-demo-page", "風景輪播").children([
            self.header(theme, dark),
            View::new(
                Node::new(Role::Group, "")
                    .id("carousel-demo-scroll")
                    .styled(
                        Style::column()
                            .height(Length::Px(0.))
                            .grow(1.)
                            .gap(8.)
                            .clip(true),
                    ),
            )
            .accessibility_scroll_pages()
            .children([track, self.controls(theme), self.variants(theme)]),
        ])
    }

    fn page(theme: &Theme, id: &str, name: &str) -> View<Message> {
        View::new(
            Node::new(Role::Window, name).id(id).styled(
                Style::column()
                    .height(Length::Percent(1.))
                    .background(theme.colors.surface),
            ),
        )
    }

    fn row(&self, name: &str) -> View<Message> {
        let mut row = Node::new(Role::Group, name).styled(
            Style::row()
                .align(Align::Center)
                .padding(8.)
                .gap(8.)
                .wrap(true),
        );
        if self.rtl {
            row = row.reverse_row();
        }
        View::new(row)
    }

    fn header(&self, theme: &Theme, dark: bool) -> View<Message> {
        let toggle = |id, label, selected, message| {
            theme
                .chip(id, label, ChipKind::Filter, choice(selected), message, None)
                .accessible_activation()
        };
        self.row("輪播設定").children([
            theme
                .tonal_button(BACK, "返回", app(CarouselEvent::Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            toggle("carousel-theme", "深色", dark, Message::Theme),
            toggle(
                "carousel-direction",
                "RTL",
                self.rtl,
                app(CarouselEvent::Direction),
            ),
            toggle(
                "carousel-large-text",
                "大字",
                self.large_text,
                app(CarouselEvent::TextScale),
            ),
        ])
    }

    fn controls(&self, theme: &Theme) -> View<Message> {
        let active = self.active();
        let mut controls = vec![
            theme
                .button_with(
                    "carousel-prev",
                    "上一張",
                    ButtonKind::Outlined,
                    active == 0,
                    app(CarouselEvent::Step(-1)),
                )
                .restore_focus_on_removal("carousel-next")
                .accessible_activation(),
            theme
                .button_with(
                    "carousel-next",
                    "下一張",
                    ButtonKind::Outlined,
                    active + 1 == TITLES.len(),
                    app(CarouselEvent::Step(1)),
                )
                .restore_focus_on_removal("carousel-prev")
                .accessible_activation(),
        ];
        if self.kind != CarouselKind::FullScreen {
            controls.push(
                theme
                    .tonal_button(SHOW_ALL, "顯示全部", app(CarouselEvent::OpenList))
                    .restore_focus_on_removal(LIST_BACK)
                    .accessible_activation(),
            );
        }
        controls.push(theme.text(
            "carousel-position",
            format!("{} / {}", active + 1, TITLES.len()),
        ));
        if let Some(index) = self.last_activated {
            controls.push(View::new(
                Node::new(Role::Status, "")
                    .id("carousel-selection")
                    .text(format!("已選取{}", TITLES[index]))
                    .styled(Style::column().foreground(theme.colors.on_surface)),
            ));
        }
        self.row("輪播導覽").children(controls)
    }

    fn variants(&self, theme: &Theme) -> View<Message> {
        let compact = self.large_text && self.viewport.0 < 440.;
        self.row("輪播版型")
            .children(VARIANTS.iter().map(|&(kind, slug, label, short)| {
                theme
                    .chip(
                        &format!("carousel-{slug}"),
                        if compact { short } else { label },
                        ChipKind::Filter,
                        choice(self.kind == kind),
                        app(CarouselEvent::Kind(kind)),
                        None,
                    )
                    .accessible_activation()
            }))
    }

    fn list(&self, theme: &Theme) -> View<Message> {
        let back = theme
            .tonal_button(LIST_BACK, "返回輪播", app(CarouselEvent::CloseList))
            .accessible_activation();
        let mut items = Vec::with_capacity(TITLES.len());
        for (index, title) in TITLES.iter().enumerate() {
            let id = format!("carousel-list-{index}");
            let button = theme
                .button_with(
                    &id,
                    &item_name(index, title),
                    ButtonKind::Tonal,
                    false,
                    app(CarouselEvent::Activate(index)),
                )
                .accessible_activation();
            items.push(View::new(Node::new(Role::ListItem, "")).children([button]));
        }
        let list = View::new(
            Node::new(Role::List, "全部風景")
                .id("carousel-list")
                .styled(
                    Style::column()
                        .height(Length::Px(0.))
                        .grow(1.)
                        .padding(12.)
                        .gap(8.)
                        .clip(true),
                ),
        )
        .accessibility_scroll_pages()
        .children(items);
        Self::page(theme, "carousel-list-page", "全部風景")
            .children([self.row("全部風景").children([back]), list])
            .restore_focus_in_subtree_on_removal(SHOW_ALL)
    }

    fn scene(&self, theme: &Theme, index: usize, title: &str) -> View<Message> {
        let colors = &theme.colors;
        let (background, foreground, accent) = [
            (
                colors.primary_container,
                colors.on_primary_container,
                colors.primary,
            ),
            (
                Color::rgb(177, 228, 234),
                Color::rgb(12, 52, 63),
                Color::rgb(22, 111, 130),
            ),
            (
                Color::rgb(186, 226, 191),
                Color::rgb(17, 55, 33),
                Color::rgb(47, 119, 65),
            ),
            (
                Color::rgb(246, 211, 163),
                Color::rgb(77, 43, 22),
                Color::rgb(149, 82, 33),
            ),
            (
                colors.tertiary_container,
                colors.on_tertiary_container,
                colors.tertiary,
            ),
            (
                Color::rgb(57, 64, 108),
                Color::rgb(231, 230, 255),
                Color::rgb(174, 184, 255),
            ),
        ][index];
        let disc = View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Percent(0.42))
                    .height(Length::Percent(0.36))
                    .background(accent)
                    .radius(32.),
            ),
        );
        let caption = View::new(
            Node::new(Role::Text, "")
                .text(title)
                .styled(Style::column().font_size(18.).foreground(foreground)),
        );
        View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .width(Length::Percent(1.))
                    .height(Length::Percent(1.))
                    .align(Align::Center)
                    .justify(Align::Center)
                    .gap(8.)
                    .radius(if self.kind == CarouselKind::FullScreen {
                        0.
                    } else {
                        28.
                    })
                    .background(background),
            ),
        )
        .children([disc, caption])
        .without_interaction(None)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        runtime: &surface_app::Runtime<crate::Gallery>,
    ) -> serde_json::Value {
        use serde_json::json;

        let items: Vec<_> = TITLES
            .iter()
            .enumerate()
            .map(|(i, title)| {
                let id = card_id(i);
                let geometry = runtime
                    .dom()
                    .by_id(&id)
                    .and_then(|node| runtime.computed()?.nodes.get(node.index()));
                let bounds = geometry.map(|node| {
                    let rect = node.bounds;
                    [rect.x, rect.y, rect.width, rect.height]
                });
                let visible = geometry.is_some_and(|node| {
                    let visible = node.bounds.intersect(node.clip);
                    visible.width > 0. && visible.height > 0.
                });
                json!({
                    "id": id,
                    "label": *title,
                    "bounds": bounds,
                    "visible": visible,
                    "aspect_ratio": ASPECT_RATIOS[i],
                })
            })
            .collect();
        let variant = VARIANTS
            .iter()
            .find(|(kind, ..)| *kind == self.kind)
            .map(|(_, slug, ..)| *slug);
        json!({
            "open": self.visible,
            "variant": variant,
            "active": self.active(),
            "offset": self.state.offset(),
            "activations": self.activations,
            "last_activated": self.last_activated,
            "show_all": self.show_all,
            "items": items,
            "dragging": self.state.dragging(),
            "settling": self.settling,
            "reduced_motion": self.reduced_motion,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
        })
    }
}

fn app(event: CarouselEvent) -> Message {
    Message::Carousel(event)
}

fn choice(selected: bool) -> ChoiceState {
    ChoiceState {
        selected,
        disabled: false,
    }
}

fn card_id(index: usize) -> String {
    format!("carousel-card-{index}")
}

fn item_name(index: usize, title: &str) -> String {
    format!("{title}，第 {} 張，共 {} 張", index + 1, TITLES.len())
}
