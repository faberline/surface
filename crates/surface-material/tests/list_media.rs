use std::collections::BTreeSet;

use surface::{
    ExtendedDisplayList, ExtendedPaintOp, Length, Node, NodeId, RasterImage, Rect, Role, Style,
    TextMeasurer,
};
use surface_app::{Application, EditKey, Event, Runtime, View};
use surface_material::{
    shape::{self, LayoutDirection},
    ListItem, ListLines, ListMedia, ListMediaKind, ListStyle, MediaListItem, MediaListMode,
    MediaListOptions, Theme, VectorIcon,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Passive,
    Action,
    MultiSelect,
}

#[derive(Clone)]
enum Message {
    Activate(&'static str),
    Outside,
}

struct App {
    image: RasterImage,
    style: ListStyle,
    mode: Mode,
    direction: LayoutDirection,
    dark: bool,
    scale: f32,
    width: f32,
    scroll_height: f32,
    rows: Vec<&'static str>,
    checked: BTreeSet<&'static str>,
    actions: Vec<&'static str>,
}

impl App {
    fn new(style: ListStyle, mode: Mode) -> Self {
        let image = RasterImage::new(8, 4, vec![120; 8 * 4 * 4]).unwrap();
        Self {
            image,
            style,
            mode,
            direction: LayoutDirection::LeftToRight,
            dark: false,
            scale: 1.,
            width: 544.,
            scroll_height: 760.,
            rows: vec!["a", "b", "c", "small", "large", "wide", "plain", "missing"],
            checked: BTreeSet::from(["b", "c"]),
            actions: Vec::new(),
        }
    }
}

impl Application for App {
    type Message = Message;

    fn update(&mut self, message: Message) {
        if let Message::Activate(id) = message {
            if self.mode == Mode::MultiSelect && !self.checked.remove(id) {
                self.checked.insert(id);
            }
            self.actions.push(id);
        }
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn view(&self) -> View<Message> {
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let is_selected = |id: &str| self.checked.contains(id);
        let mode = match self.mode {
            Mode::Passive => MediaListMode::Passive,
            Mode::Action => MediaListMode::Action,
            Mode::MultiSelect => MediaListMode::MultiSelect {
                is_selected: &is_selected,
            },
        };
        let icon = VectorIcon::new([[4., 3.], [20., 3.], [12., 21.]]).unwrap();
        let rows = self.rows.iter().map(|&id| {
            let (headline, overline, supporting, lines, kind) = match id {
                "a" => (
                    "山徑",
                    None,
                    None,
                    ListLines::One,
                    Some(ListMediaKind::Thumbnail),
                ),
                "b" => (
                    "行程",
                    None,
                    Some("旅程資料"),
                    ListLines::Two,
                    Some(ListMediaKind::Thumbnail),
                ),
                "c" => (
                    "地圖",
                    Some("收藏"),
                    Some("下載離線地圖供出發後查看"),
                    ListLines::Three,
                    Some(ListMediaKind::Thumbnail),
                ),
                "small" => (
                    "短片",
                    None,
                    None,
                    ListLines::One,
                    Some(ListMediaKind::VideoSmall),
                ),
                "large" => (
                    "觀景影片",
                    Some("下午"),
                    Some("保留完整說明讓旅客準備出發"),
                    ListLines::Three,
                    Some(ListMediaKind::VideoLarge),
                ),
                "wide" => (
                    "風景",
                    None,
                    None,
                    ListLines::One,
                    Some(ListMediaKind::WideThumbnail),
                ),
                "plain" => ("純文字", None, None, ListLines::One, None),
                _ => (
                    "暫停",
                    None,
                    None,
                    ListLines::One,
                    Some(ListMediaKind::Thumbnail),
                ),
            };
            MediaListItem {
                row: ListItem {
                    id,
                    headline,
                    overline,
                    supporting,
                    lines,
                    leading: Some(&icon),
                    trailing: (id == "large").then_some("離線"),
                    disabled: id == "b",
                    on_activate: (id != "missing").then_some(Message::Activate(id)),
                },
                media: kind.map(|kind| ListMedia {
                    image: &self.image,
                    kind,
                }),
            }
        });
        let list = theme.media_list(
            MediaListOptions {
                id: "list",
                name: "旅程",
                style: self.style,
                mode,
                direction: self.direction,
                available_width: self.width,
            },
            rows,
        );
        let scroller = View::new(
            Node::new(Role::Group, "旅程捲動區").id("scroll").styled(
                Style::column()
                    .height(Length::Px(self.scroll_height))
                    .clip(true),
            ),
        )
        .children(list);
        let outside = View::new(
            Node::new(Role::Button, "返回")
                .id("outside")
                .styled(Style::column().height(Length::Px(48.))),
        )
        .on_press(Message::Outside);
        View::new(Node::new(Role::Window, "").styled(Style::column())).children([scroller, outside])
    }
}

struct Metrics;

impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
        let natural = text.chars().count() as f32 * size * 0.8;
        let width = width.map_or(natural, |width| width.min(natural));
        let lines = (natural / width.max(1.)).ceil().max(1.);
        (width, size * 1.35 * lines)
    }
}

fn runtime(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    render(&mut runtime);
    runtime
}

fn render(runtime: &mut Runtime<App>) -> ExtendedDisplayList {
    let width = runtime.app().width;
    let height = runtime.app().scroll_height + 48.;
    runtime
        .render_extended(width, height, &mut Metrics)
        .unwrap()
}

fn id(runtime: &Runtime<App>, key: &str) -> NodeId {
    runtime.dom().by_id(key).unwrap()
}

fn node<'a>(runtime: &'a Runtime<App>, key: &str) -> &'a Node {
    runtime.dom().get(id(runtime, key)).unwrap()
}

fn bounds(runtime: &Runtime<App>, key: &str) -> Rect {
    runtime.computed().unwrap().nodes[id(runtime, key).index()].bounds
}

fn tap(runtime: &mut Runtime<App>, key: &str) {
    let rect = bounds(runtime, key);
    let x = rect.x + rect.width / 2.;
    let y = rect.y + rect.height / 2.;
    runtime.dispatch(Event::PointerDown { x, y }).unwrap();
    runtime.dispatch(Event::PointerUp { x, y }).unwrap();
    render(runtime);
}

fn focus_next(runtime: &mut Runtime<App>, wanted: &str) {
    runtime
        .dispatch(Event::FocusNext { reverse: false })
        .unwrap();
    render(runtime);
    assert_eq!(runtime.focused(), Some(wanted));
}

#[test]
fn media_profiles_use_the_pinned_geometry_and_real_center_crop_in_every_style() {
    for style in [
        ListStyle::Baseline,
        ListStyle::Standard,
        ListStyle::Segmented,
    ] {
        let mut runtime = runtime(App::new(style, Mode::Passive));
        let paint = render(&mut runtime);
        assert_eq!(node(&runtime, "list").role(), Role::List);
        let corner = if style == ListStyle::Baseline {
            shape::NONE
        } else {
            shape::SMALL
        };
        for (row_id, width, height) in [
            ("a", 56., 56.),
            ("b", 56., 56.),
            ("c", 56., 56.),
            ("small", 100., 56.),
            ("large", 114., 64.),
            ("wide", 80., 80.),
        ] {
            let key = format!("{row_id}-media");
            let row = bounds(&runtime, row_id);
            let media = bounds(&runtime, &key);
            let headline = bounds(&runtime, &format!("{row_id}-headline"));
            let start = if style == ListStyle::Baseline && matches!(row_id, "small" | "large") {
                0.
            } else {
                16.
            };
            assert_eq!(
                (media.x - row.x, media.width, media.height),
                (start, width, height)
            );
            assert_eq!(
                headline.x - media.x - media.width,
                if style == ListStyle::Baseline {
                    16.
                } else {
                    12.
                }
            );
            assert!(headline.width > width);
            assert_eq!(media.intersect(row), media);
            assert_eq!(node(&runtime, &key).style().corner_radii(), corner);
            assert_eq!(node(&runtime, &key).role(), Role::Group);
            assert!(!node(&runtime, &key).is_focusable());
            assert!(runtime.dom().by_id(&format!("{row_id}-leading")).is_none());
            let image = paint
                .ops()
                .iter()
                .find_map(|op| match op {
                    ExtendedPaintOp::Raster(image) if image.rect() == media => Some(image),
                    _ => None,
                })
                .unwrap();
            assert!(image.image().shares_pixels(&runtime.app().image));
            assert_eq!(image.corners(), corner);
            if width == height {
                assert_eq!(image.source_uv(), [0.25, 0., 0.5, 1.]);
            }
            let frame = runtime.accessibility_frame().unwrap();
            assert!(frame.node(id(&runtime, &key)).is_none());
        }
        assert!(runtime.dom().by_id("plain-media").is_none());
        assert!(runtime.dom().by_id("plain-leading").is_some());
        let first = bounds(&runtime, "a");
        let second = bounds(&runtime, "b");
        let gap = if style == ListStyle::Segmented {
            2.
        } else {
            0.
        };
        assert_eq!(second.y - first.y - first.height, gap);
        if style == ListStyle::Baseline {
            for (row_id, want_height, want_top) in [
                ("a", 72., 8.),
                ("b", 72., 8.),
                ("c", 88., 12.),
                ("large", 88., 12.),
            ] {
                let row = bounds(&runtime, row_id);
                let media = bounds(&runtime, &format!("{row_id}-media"));
                assert_eq!((row.height, media.y - row.y), (want_height, want_top));
            }
        }
    }
}

#[test]
fn baseline_poster_is_flush_when_passive_and_reserves_focus_pixels_when_interactive() {
    for (mode, want_inset) in [
        (Mode::Passive, 0.),
        (Mode::Action, 3.),
        (Mode::MultiSelect, 3.),
    ] {
        for direction in [LayoutDirection::LeftToRight, LayoutDirection::RightToLeft] {
            let mut app = App::new(ListStyle::Baseline, mode);
            app.direction = direction;
            let runtime = runtime(app);
            for key in ["small", "large"] {
                let row = bounds(&runtime, key);
                let image = bounds(&runtime, &format!("{key}-media"));
                let copy = bounds(&runtime, &format!("{key}-headline"));
                let (start, gap) = match direction {
                    LayoutDirection::LeftToRight => {
                        (image.x - row.x, copy.x - image.x - image.width)
                    }
                    LayoutDirection::RightToLeft => (
                        row.x + row.width - image.x - image.width,
                        image.x - copy.x - copy.width,
                    ),
                };
                assert_eq!(start, want_inset);
                assert_eq!(gap, 16.);
            }
        }
    }
}

#[test]
fn action_rows_keep_a_single_press_target_and_pass_wheel_over_the_thumbnail() {
    let mut app = App::new(ListStyle::Segmented, Mode::Action);
    app.scroll_height = 96.;
    let mut runtime = runtime(app);
    let thumb = bounds(&runtime, "a-media");
    let first = node(&runtime, "a");
    assert_eq!(first.role(), Role::Button);
    assert_eq!(first.name(), "山徑");
    assert_eq!(
        runtime.dom().get(first.parent().unwrap()).unwrap().role(),
        Role::ListItem
    );
    let paint = render(&mut runtime);
    let drawn_media = paint
        .ops()
        .iter()
        .filter_map(|op| match op {
            ExtendedPaintOp::Raster(image) => Some(image),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(drawn_media.len(), 2);
    let scroll = bounds(&runtime, "scroll");
    assert_eq!(drawn_media[1].clip(), scroll);
    assert!(drawn_media[1].rect().height > drawn_media[1].rect().intersect(scroll).height);

    tap(&mut runtime, "a-media");
    assert_eq!(runtime.app().actions, ["a"]);
    runtime
        .dispatch(Event::Scroll {
            x: thumb.x + thumb.width / 2.,
            y: thumb.y + thumb.height / 2.,
            delta_x: 0.,
            delta_y: 70.,
        })
        .unwrap();
    render(&mut runtime);
    assert!(runtime.scroll_metrics("scroll").unwrap().offset.1 > 0.);
    assert_eq!(runtime.app().actions, ["a"]);
}

#[test]
fn checkbox_rows_stay_controlled_and_the_media_and_marks_are_passive() {
    let mut runtime = runtime(App::new(ListStyle::Standard, Mode::MultiSelect));
    assert!(node(&runtime, "list").uses_web_multiselect_listbox_semantics());
    for (key, selected) in [("a", false), ("b", true), ("c", true), ("missing", false)] {
        let row = node(&runtime, key);
        assert_eq!(row.role(), Role::Checkbox);
        assert_eq!(row.is_checked(), Some(selected));
        assert!(row.uses_web_multiselect_listbox_semantics());
        assert_eq!(
            (
                bounds(&runtime, &format!("{key}-mark")).width,
                bounds(&runtime, &format!("{key}-mark")).height
            ),
            (18., 18.)
        );
    }
    let frame = runtime.accessibility_frame().unwrap();
    for key in ["a-media", "a-mark", "b", "missing"] {
        assert!(frame.focus_target(id(&runtime, key)).is_none());
        assert!(frame.activation_target(id(&runtime, key)).is_none());
    }
    assert_eq!(
        frame.description_target(id(&runtime, "c")),
        Some(id(&runtime, "c-supporting"))
    );

    focus_next(&mut runtime, "c");
    runtime
        .dispatch(Event::Edit {
            key: EditKey::Up,
            extend: false,
            by_word: false,
        })
        .unwrap();
    render(&mut runtime);
    assert_eq!(runtime.focused(), Some("a"));
    assert!(runtime.app().actions.is_empty());
    tap(&mut runtime, "a-mark");
    assert_eq!(runtime.app().actions, ["a"]);
    assert_eq!(node(&runtime, "a").is_checked(), Some(true));
    assert_eq!(node(&runtime, "c").is_checked(), Some(true));
    focus_next(&mut runtime, "outside");
    tap(&mut runtime, "b-media");
    tap(&mut runtime, "missing-media");
    assert_eq!(runtime.app().actions, ["a"]);
    assert_eq!(node(&runtime, "b").is_checked(), Some(true));
}

#[test]
fn compact_rtl_caps_artwork_for_scaled_copy_without_reordering_targets() {
    let cases = [true, false].map(|dark| {
        let mut app = App::new(ListStyle::Segmented, Mode::MultiSelect);
        app.rows = vec!["large"];
        app.direction = LayoutDirection::RightToLeft;
        app.width = 320.;
        app.dark = dark;
        app.scale = 2.;
        let runtime = runtime(app);
        let row = bounds(&runtime, "large");
        let artwork = bounds(&runtime, "large-media");
        let middle = bounds(&runtime, "large-supporting");
        let metadata = bounds(&runtime, "large-trailing");
        let mark = bounds(&runtime, "large-mark");
        assert_eq!(row.width, 320.);
        assert_eq!(row.x + row.width - artwork.x - artwork.width, 16.);
        assert!(mark.x + mark.width < metadata.x);
        assert!(metadata.x + metadata.width < middle.x);
        assert!(middle.x + middle.width < artwork.x);
        assert!(middle.width > artwork.width && middle.width > metadata.width + mark.width);
        assert!(middle.height > 40.);
        assert!(artwork.width < 114.);
        assert!((artwork.width * 64. / 114. - artwork.height).abs() <= 1.);
        for slot in [artwork, middle, metadata, mark] {
            assert_eq!(slot.intersect(row), slot);
        }
        [row, artwork, middle, metadata, mark]
    });
    assert_eq!(cases[0], cases[1]);
}

#[test]
fn smallest_width_stacks_and_rejects_unusable_allocations() {
    let theme = Theme::light();
    for available_width in [0., 47., f32::INFINITY, f32::NAN] {
        assert!(theme
            .media_list::<()>(
                MediaListOptions {
                    id: "empty",
                    name: "",
                    style: ListStyle::Baseline,
                    mode: MediaListMode::Passive,
                    direction: LayoutDirection::LeftToRight,
                    available_width,
                },
                [],
            )
            .is_none());
    }
    let mut app = App::new(ListStyle::Baseline, Mode::MultiSelect);
    app.rows = vec!["large"];
    app.width = 48.;
    let runtime = runtime(app);
    let row = bounds(&runtime, "large");
    let artwork = bounds(&runtime, "large-media");
    let text = bounds(&runtime, "large-headline");
    let mark = bounds(&runtime, "large-mark");
    assert_eq!(row.width, 48.);
    assert!(row.height >= 48.);
    for slot in [artwork, text, mark] {
        assert_eq!(slot.intersect(row), slot);
    }
    assert!(text.width > artwork.width);
    assert!(artwork.y + artwork.height < text.y);
    assert!(text.y + text.height < mark.y);
}
