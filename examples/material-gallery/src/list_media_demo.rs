use std::{collections::BTreeSet, sync::OnceLock};

use surface::{Align, Length, Node, RasterImage, Role, Style};
use surface_app::View;
use surface_material::{
    shape::LayoutDirection, typography, ButtonKind, ChipKind, ChoiceState, ListItem, ListLines,
    ListMedia, ListMediaKind, ListStyle, MediaListItem, MediaListMode, MediaListOptions, Theme,
};

use crate::Message;

const SHOW: &str = "list-media-demo-show";
const LIST: &str = "list-media-items";
const SCROLL: &str = "list-media-scroll";
const DETAIL: &str = "list-media-detail";
const PAGE_PADDING: f32 = 12.;
const WIDE: f32 = 840.;
const STYLES: [(ListStyle, &str, &str); 3] = [
    (ListStyle::Baseline, "baseline", "基準"),
    (ListStyle::Standard, "standard", "標準"),
    (ListStyle::Segmented, "segmented", "分段"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Passive,
    Action,
    Select,
}

const MODES: [(Mode, &str, &str); 3] = [
    (Mode::Passive, "passive", "唯讀"),
    (Mode::Action, "action", "查看"),
    (Mode::Select, "select", "多選"),
];

#[derive(Clone, Copy)]
enum Asset {
    Earth,
    Poster,
    Probe,
}

impl Asset {
    fn image(self) -> &'static RasterImage {
        static EARTH: OnceLock<RasterImage> = OnceLock::new();
        static POSTER: OnceLock<RasterImage> = OnceLock::new();
        static PROBE: OnceLock<RasterImage> = OnceLock::new();
        match self {
            Self::Earth => EARTH.get_or_init(|| {
                let pixels = include_bytes!("../assets/list-media/blue-marble-160x160.rgba");
                RasterImage::new(160, 160, pixels.as_slice()).expect("embedded Blue Marble RGBA")
            }),
            Self::Poster => POSTER.get_or_init(|| {
                let pixels = include_bytes!("../assets/list-media/new-zealand-poster-216x144.rgba");
                RasterImage::new(216, 144, pixels.as_slice())
                    .expect("embedded New Zealand poster RGBA")
            }),
            Self::Probe => PROBE.get_or_init(crop_probe),
        }
    }

    fn kind(self, wide: bool) -> ListMediaKind {
        match (self, wide) {
            (Self::Earth, true) => ListMediaKind::WideThumbnail,
            (Self::Poster, true) => ListMediaKind::VideoLarge,
            (Self::Poster, false) => ListMediaKind::VideoSmall,
            _ => ListMediaKind::Thumbnail,
        }
    }
}

fn crop_probe() -> RasterImage {
    // A square frame shows the middle 48 source pixels, not the outside bands.
    let mut rgba = Vec::with_capacity(96 * 48 * 4);
    for y in 0..48 {
        for x in 0..96 {
            let color = match (x, y) {
                (0..24, _) => [231, 32, 176, 255],
                (72..96, _) => [255, 167, 16, 255],
                (24..48, 0..24) => [19, 190, 82, 255],
                (48..72, 0..24) => [27, 91, 238, 255],
                (24..48, _) => [239, 23, 64, 128],
                _ => [250, 205, 41, 255],
            };
            rgba.extend(color);
        }
    }
    RasterImage::new(96, 48, rgba).expect("96 by 48 crop probe RGBA")
}

struct Entry {
    id: &'static str,
    label: &'static str,
    overline: Option<&'static str>,
    caption: Option<&'static str>,
    detail: &'static str,
    asset: Option<Asset>,
    disabled: bool,
}

const ENTRIES: [Entry; 5] = [
    Entry {
        id: "list-media-earth",
        label: "藍色彈珠",
        overline: None,
        caption: Some("阿波羅十七號在前往月球途中拍攝地球"),
        detail: "1972 年 12 月 7 日由阿波羅十七號乘員拍攝。資料來源：NASA（AS17-148-22727）。",
        asset: Some(Asset::Earth),
        disabled: false,
    },
    Entry {
        id: "list-media-poster",
        label: "紐西蘭上空",
        overline: Some("縮時影像"),
        caption: Some("國際太空站飛越紐西蘭南北兩島"),
        detail:
            "遠征四十二號組員拍攝的縮時影像封面。資料來源：NASA 約翰遜太空中心（ISS042-E-170757）。",
        asset: Some(Asset::Poster),
        disabled: false,
    },
    Entry {
        id: "list-media-probe",
        label: "色彩測試圖",
        overline: None,
        caption: None,
        detail: "中間四個色塊可檢查裁切、方向與透明度。",
        asset: Some(Asset::Probe),
        disabled: false,
    },
    Entry {
        id: "list-media-text",
        label: "觀測筆記",
        overline: None,
        caption: Some("飛行照片與縮時影像的索引"),
        detail: "這是沒有縮圖的文字筆記。",
        asset: None,
        disabled: false,
    },
    Entry {
        id: "list-media-disabled",
        label: "封存影像",
        overline: None,
        caption: None,
        detail: "",
        asset: Some(Asset::Earth),
        disabled: true,
    },
];

#[derive(Clone)]
pub enum ListMediaEvent {
    Show,
    Back,
    Direction,
    LargeText,
    Style(ListStyle),
    Mode(Mode),
    Press(usize),
    CloseDetail,
    Clear,
}

pub(crate) struct ListMediaDemo {
    visible: bool,
    width: f32,
    height: f32,
    style: ListStyle,
    mode: Mode,
    rtl: bool,
    large_text: bool,
    detail: Option<usize>,
    activations: usize,
    selection_changes: usize,
    selected: BTreeSet<&'static str>,
}

impl Default for ListMediaDemo {
    fn default() -> Self {
        Self {
            visible: false,
            width: 960.,
            height: 820.,
            style: ListStyle::Segmented,
            mode: Mode::Passive,
            rtl: false,
            large_text: false,
            detail: None,
            activations: 0,
            selection_changes: 0,
            selected: BTreeSet::new(),
        }
    }
}

impl ListMediaDemo {
    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "影像清單",
                ButtonKind::Outlined,
                false,
                Message::ListMedia(ListMediaEvent::Show),
            )
            .accessible_activation()
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        if self.visible {
            self.visible = false;
            self.detail = None;
        }
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let changed = self.width != width || self.height != height;
        self.width = width;
        self.height = height;
        self.visible && changed
    }

    pub(crate) fn update(&mut self, event: ListMediaEvent) {
        match event {
            ListMediaEvent::Show => self.visible = true,
            ListMediaEvent::Back => self.leave(),
            ListMediaEvent::Direction => self.rtl = !self.rtl,
            ListMediaEvent::LargeText => self.large_text = !self.large_text,
            ListMediaEvent::Style(style) => self.style = style,
            ListMediaEvent::Mode(mode) => self.mode = mode,
            ListMediaEvent::Press(index) => self.press(index),
            ListMediaEvent::CloseDetail => self.detail = None,
            ListMediaEvent::Clear => self.selected.clear(),
        }
    }

    fn press(&mut self, index: usize) {
        let Some(entry) = ENTRIES.get(index).filter(|entry| !entry.disabled) else {
            return;
        };
        match self.mode {
            Mode::Passive => {}
            Mode::Action if self.detail.is_none() => {
                self.activations += 1;
                self.detail = Some(index);
            }
            Mode::Action => {}
            Mode::Select => {
                if !self.selected.remove(entry.id) {
                    self.selected.insert(entry.id);
                }
                self.selection_changes += 1;
            }
        }
    }

    fn list(&self, theme: &Theme) -> Option<View<Message>> {
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let selected = |id: &str| self.selected.contains(id);
        let mode = match self.mode {
            Mode::Passive => MediaListMode::Passive,
            Mode::Action => MediaListMode::Action,
            Mode::Select => MediaListMode::MultiSelect {
                is_selected: &selected,
            },
        };
        let options = MediaListOptions {
            id: LIST,
            name: "太空影像",
            style: self.style,
            direction,
            available_width: (self.width - 2. * PAGE_PADDING).max(0.),
            mode,
        };
        theme.media_list(
            options,
            ENTRIES.iter().enumerate().map(|(index, entry)| {
                let row = ListItem {
                    id: entry.id,
                    headline: entry.label,
                    overline: entry.overline,
                    supporting: entry.caption,
                    leading: None,
                    trailing: None,
                    lines: match (entry.overline, entry.caption) {
                        (None, None) => ListLines::One,
                        (Some(_), Some(_)) => ListLines::Three,
                        _ => ListLines::Two,
                    },
                    disabled: entry.disabled,
                    on_activate: (!entry.disabled)
                        .then_some(Message::ListMedia(ListMediaEvent::Press(index))),
                };
                let media = entry.asset.map(|asset| ListMedia {
                    image: asset.image(),
                    kind: asset.kind(self.width >= WIDE),
                });
                MediaListItem { row, media }
            }),
        )
    }

    fn detail(&self, theme: &Theme, index: usize) -> View<Message> {
        let entry = &ENTRIES[index];
        let hero = entry.asset.map(|asset| {
            View::new(
                Node::new(Role::Group, "")
                    .id("list-media-detail-image")
                    .styled(
                        Style::column()
                            .width(Length::Percent(1.))
                            .height(Length::Px((self.height * 0.24).clamp(72., 208.)))
                            .radius(12.),
                    )
                    .raster_image(asset.image().clone()),
            )
            .without_interaction(None)
        });
        let info = View::new(
            Node::new(Role::Text, entry.detail)
                .id("list-media-detail-text")
                .text(entry.detail)
                .shrink_to_fit_width()
                .styled(
                    Style::column()
                        .width(Length::Percent(1.))
                        .with_text_style(typography::BODY_MEDIUM)
                        .foreground(theme.colors.on_surface_variant),
                ),
        );
        theme.dialog(
            DETAIL,
            entry.label,
            hero.into_iter().chain([info]),
            [theme
                .button(
                    "list-media-detail-back",
                    "返回清單",
                    Message::ListMedia(ListMediaEvent::CloseDetail),
                )
                .accessible_activation()
                .suppress_repeated_activation()],
            Message::ListMedia(ListMediaEvent::CloseDetail),
        )
    }

    pub(crate) fn view(&self, theme: &Theme) -> View<Message> {
        let scroller = View::new(Node::new(Role::Group, "影像清單捲動區").id(SCROLL).styled(
            Style {
                height: Length::Px(0.),
                min_height: 120.,
                max_height: Length::Px(318.),
                grow: 1.,
                clip: true,
                ..Style::column()
            },
        ))
        .accessibility_scroll_pages()
        .children(self.list(theme));
        let mut children = vec![self.controls(theme), scroller];
        if self.mode == Mode::Select {
            let label = format!("已選 {} 項", self.selected.len());
            let status = View::new(
                Node::new(Role::Status, "")
                    .id("list-media-demo-status")
                    .text(label)
                    .styled(
                        Style::default()
                            .with_text_style(typography::BODY_MEDIUM)
                            .foreground(theme.colors.on_surface),
                    ),
            );
            children.push(
                View::new(
                    Node::new(Role::Group, "")
                        .styled(Style::row().wrap(true).gap(8.).align(Align::Center)),
                )
                .children([
                    status,
                    theme
                        .button(
                            "list-media-demo-clear",
                            "清除選擇",
                            Message::ListMedia(ListMediaEvent::Clear),
                        )
                        .accessible_activation(),
                ]),
            );
        }
        children.extend(self.detail.map(|index| self.detail(theme, index)));
        View::new(
            Node::new(Role::Window, "影像清單")
                .id("list-media-demo")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .padding(PAGE_PADDING)
                        .gap(8.)
                        .clip(true)
                        .background(theme.colors.surface_container),
                ),
        )
        .restore_focus_in_subtree_on_removal(SHOW)
        .accessibility_scroll_pages()
        .children(children)
    }

    fn controls(&self, theme: &Theme) -> View<Message> {
        let toggle = |id: &str, label: &str, selected, event| {
            theme
                .chip(
                    &format!("list-media-demo-{id}"),
                    label,
                    ChipKind::Filter,
                    ChoiceState {
                        selected,
                        disabled: false,
                    },
                    Message::ListMedia(event),
                    None,
                )
                .accessible_activation()
        };
        let display = controls("畫面設定").children([
            theme
                .tonal_button(
                    "list-media-demo-back",
                    "返回",
                    Message::ListMedia(ListMediaEvent::Back),
                )
                .accessible_activation(),
            theme
                .button("list-media-demo-theme", "配色", Message::Theme)
                .accessible_activation(),
            toggle("rtl", "右起", self.rtl, ListMediaEvent::Direction),
            toggle(
                "large-text",
                "大字",
                self.large_text,
                ListMediaEvent::LargeText,
            ),
        ]);
        let styles = controls("清單樣式").children(STYLES.map(|(style, id, label)| {
            toggle(id, label, self.style == style, ListMediaEvent::Style(style))
        }));
        let modes = controls("列操作").children(MODES.map(|(mode, id, label)| {
            toggle(id, label, self.mode == mode, ListMediaEvent::Mode(mode))
        }));
        View::new(Node::new(Role::Group, "").styled(Style::column().gap(4.)))
            .children([display, styles, modes])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        runtime: &surface_app::Runtime<crate::Gallery>,
        theme: &Theme,
    ) -> serde_json::Value {
        let variant = STYLES
            .iter()
            .find(|(style, _, _)| *style == self.style)
            .map(|(_, id, _)| id);
        let mode = MODES
            .iter()
            .find(|(mode, _, _)| *mode == self.mode)
            .map(|(_, id, _)| id);
        let selected: Vec<_> = ENTRIES
            .iter()
            .filter(|entry| self.selected.contains(entry.id))
            .map(|entry| entry.id)
            .collect();
        let colors = theme.colors;
        serde_json::json!({
            "visible":self.visible, "variant":variant, "mode":mode, "wide":self.width >= WIDE,
            "rtl":self.rtl, "text_scale":self.text_scale(),
            "details":self.detail.map(|index| ENTRIES[index].id),
            "activations":self.activations, "selection_changes":self.selection_changes,
            "selected":selected,
            "scroll_y":runtime.scroll_metrics(SCROLL).map(|metrics| metrics.offset.1),
            "colors":{"surface":colors.surface,"surface_container":colors.surface_container,
                "secondary_container":colors.secondary_container, "on_surface_variant":colors.on_surface_variant},
        })
    }
}

fn controls(name: &str) -> View<Message> {
    View::new(Node::new(Role::Group, name).styled(Style::row().gap(8.).wrap(true)))
}
