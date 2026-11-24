use surface::{Align, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, FocusNavigation, View};
use surface_material::{
    shape::{self, LayoutDirection},
    ButtonKind, ChoiceState, ListItem, ListLines, ListMode, ListOptions, ListStyle,
    SearchOverlayContent, SearchOverlayMotion, SearchOverlayOptions, SearchStyle, Theme,
};

use crate::{scroll_field_demo::control_row, Message};

pub(crate) const SECTION: usize = 1;
const ID: &str = "search-overlay-demo";
const SHOW: &str = "search-overlay-demo-show";
const BACK: &str = "search-overlay-demo-route-back";
const OPENER: &str = "search-overlay-demo-opener";
const ANCHOR: &str = "search-overlay-demo-anchor";

struct Document {
    id: &'static str,
    title: &'static str,
    details: &'static str,
    disabled: bool,
}

const DOCUMENTS: &[Document] = &[
    Document {
        id: "search-doc-design",
        title: "設計紀錄",
        details: "元件與版面規格",
        disabled: false,
    },
    Document {
        id: "search-doc-archive",
        title: "設計封存",
        details: "封存文件暫停開啟",
        disabled: true,
    },
    Document {
        id: "search-doc-review",
        title: "設計審查",
        details: "本週核對重點",
        disabled: false,
    },
    Document {
        id: "search-doc-tests",
        title: "測試結果",
        details: "鍵盤與瀏覽器",
        disabled: false,
    },
    Document {
        id: "search-doc-accessibility",
        title: "可及性檢查",
        details: "焦點與閱讀順序",
        disabled: false,
    },
    Document {
        id: "search-doc-release",
        title: "發佈進度",
        details: "當前工作與變更",
        disabled: false,
    },
    Document {
        id: "search-doc-palette",
        title: "配色驗證",
        details: "深色與提高對比",
        disabled: false,
    },
    Document {
        id: "search-doc-localization",
        title: "語系設計",
        details: "字型與右起版面",
        disabled: false,
    },
    Document {
        id: "search-doc-gesture",
        title: "觸控驗收",
        details: "按壓與捲動",
        disabled: false,
    },
    Document {
        id: "search-doc-guide",
        title: "開發指南",
        details: "元件接線範例",
        disabled: false,
    },
    Document {
        id: "search-doc-notes",
        title: "開發筆記",
        details: "整合紀錄",
        disabled: false,
    },
    Document {
        id: "search-doc-handoff",
        title: "交接清單",
        details: "後續驗證項目",
        disabled: false,
    },
];

#[derive(Clone)]
pub enum SearchOverlayDemoEvent {
    Show,
    Back,
    Open,
    Close,
    ToggleStyle,
    Direction,
    LargeText,
    Query(String),
    Clear,
    Submit,
    FocusResult,
    Select(&'static str),
}

pub(crate) struct SearchOverlayDemo {
    visible: bool,
    open: bool,
    style: SearchStyle,
    rtl: bool,
    large_text: bool,
    viewport: Rect,
    anchor: Option<Rect>,
    motion: SearchOverlayMotion,
    query: String,
    selected: Option<&'static str>,
    submissions: usize,
}

impl Default for SearchOverlayDemo {
    fn default() -> Self {
        Self {
            visible: false,
            open: false,
            style: SearchStyle::Contained,
            rtl: false,
            large_text: false,
            viewport: Rect::default(),
            anchor: None,
            motion: SearchOverlayMotion::new(false),
            query: String::new(),
            selected: None,
            submissions: 0,
        }
    }
}

impl SearchOverlayDemo {
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

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.open = false;
        self.anchor = None;
        self.motion = SearchOverlayMotion::new(false);
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let viewport = Rect::new(0., 0., width, height);
        if self.viewport == viewport {
            return false;
        }
        self.viewport = viewport;
        self.anchor = None;
        self.visible
    }

    pub(crate) fn observe(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if !self.visible {
            return AnimationUpdate::default();
        }
        let Some(layout) = frame
            .layout
            .filter(|layout| layout.viewport == self.viewport)
        else {
            return AnimationUpdate {
                rebuild: false,
                request_next: true,
            };
        };
        let anchor = frame
            .dom
            .by_id(ANCHOR)
            .and_then(|id| layout.nodes.get(id.index()))
            .filter(|node| node.bounds.intersect(node.clip) == node.bounds)
            .map(|node| node.bounds);
        let changed = self.anchor != anchor;
        self.anchor = anchor;
        let target = self.overlay_options(None).motion_target();
        let mut update = self.motion.advance(frame, target);
        update.rebuild |= changed && (self.open || self.motion.progress() > 0.);
        update
    }

    pub(crate) fn update(&mut self, event: SearchOverlayDemoEvent) {
        use SearchOverlayDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            Open => self.open = true,
            Close => self.open = false,
            ToggleStyle => {
                self.style = if self.style == SearchStyle::Contained {
                    SearchStyle::Divided
                } else {
                    SearchStyle::Contained
                }
            }
            Direction => self.rtl = !self.rtl,
            LargeText => self.large_text = !self.large_text,
            Query(query) => self.query = query,
            Clear => self.query.clear(),
            Submit => self.submissions += 1,
            FocusResult => {}
            Select(id) => {
                self.selected = Some(id);
                self.open = false;
            }
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "搜尋浮層",
                ButtonKind::Outlined,
                false,
                Message::SearchOverlay(SearchOverlayDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn opener(&self, theme: &Theme) -> View<Message> {
        let width = (self.viewport.width - 56.).clamp(1., 620.);
        let allocation = Rect::new(0., 0., width, 56.);
        let bounds = self
            .style
            .resting_bar_bounds(allocation)
            .unwrap_or(allocation);
        let shape = if self.style == SearchStyle::Contained {
            shape::FULL
        } else {
            shape::LARGE
        };
        let label = if self.query.is_empty() {
            "搜尋文件"
        } else {
            &self.query
        };
        let opener = Node::new(Role::Button, "搜尋文件").id(OPENER).styled(
            Style::row()
                .width(Length::Px(bounds.width))
                .height(Length::Px(bounds.height))
                .corners(shape)
                .align(Align::Center)
                .justify(Align::Start)
                .padding(16.)
                .clip(true)
                .background(theme.colors.surface_container_high),
        );
        let opener = if self.rtl {
            opener.reverse_row()
        } else {
            opener
        };
        let opener = View::new(opener)
            .children([theme.text("search-overlay-demo-opener-label", label)])
            .on_press(Message::SearchOverlay(SearchOverlayDemoEvent::Open))
            .disclosure_expanded(self.open)
            .suppress_repeated_activation()
            .accessible_activation();
        View::new(
            Node::new(Role::Group, "").id(ANCHOR).styled(
                Style::column()
                    .width(Length::Px(width))
                    .height(Length::Px(56.))
                    .align(Align::Center),
            ),
        )
        .children([opener])
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use SearchOverlayDemoEvent::*;
        let toggle = |id, label, selected, event| {
            theme
                .segmented_button(
                    id,
                    label,
                    ChoiceState {
                        selected,
                        disabled: false,
                    },
                    Message::SearchOverlay(event),
                )
                .accessible_activation()
        };
        let profile = control_row(vec![
            theme
                .tonal_button(BACK, "返回", Message::SearchOverlay(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme
                .switch(
                    "search-overlay-demo-theme",
                    "深色",
                    ChoiceState {
                        selected: dark,
                        disabled: false,
                    },
                    Message::Theme,
                )
                .accessible_activation(),
            toggle(
                "search-overlay-demo-style",
                "分隔",
                self.style == SearchStyle::Divided,
                ToggleStyle,
            ),
            toggle("search-overlay-demo-rtl", "右起", self.rtl, Direction),
            toggle(
                "search-overlay-demo-large-text",
                "大字",
                self.large_text,
                LargeText,
            ),
        ]);
        let selected = self
            .selected
            .and_then(|id| DOCUMENTS.iter().find(|entry| entry.id == id));
        let mut page = vec![
            theme.heading("search-overlay-demo-title", "搜尋工作區"),
            profile,
            self.opener(theme),
        ];
        if let Some(entry) = selected {
            page.push(theme.text(
                "search-overlay-demo-selected",
                format!("已開啟 {}", entry.title),
            ));
        }
        if let Some(overlay) = self.overlay(theme) {
            page.push(overlay);
        }
        theme.page(page)
    }

    fn matches(&self) -> impl Iterator<Item = &'static Document> + '_ {
        let query = self.query.trim().to_lowercase();
        DOCUMENTS.iter().filter(move |entry| {
            entry.title.to_lowercase().contains(&query)
                || entry.details.to_lowercase().contains(&query)
        })
    }

    fn overlay_options<'a>(&'a self, status: Option<&'a str>) -> SearchOverlayOptions<'a> {
        SearchOverlayOptions {
            id: ID,
            open: self.open,
            title: "搜尋工作區",
            input_label: "搜尋文件",
            placeholder: Some("搜尋文件"),
            results_label: "搜尋結果",
            back_label: "關閉搜尋",
            clear_label: "清除搜尋",
            query: &self.query,
            style: self.style,
            direction: self.direction(),
            viewport: self.viewport,
            anchor: self.anchor,
            preferred_docked_height: Some(440.),
            restore_focus_to: Some(OPENER),
            results_status: status,
        }
    }

    fn overlay(&self, theme: &Theme) -> Option<View<Message>> {
        if !self.open && self.motion.progress() == 0. {
            return None;
        }
        use SearchOverlayDemoEvent::*;
        let entries: Vec<_> = self.matches().collect();
        let status = if entries.is_empty() {
            "找不到相符文件".to_owned()
        } else if self.query.trim().is_empty() {
            format!("近期文件，共 {} 筆", entries.len())
        } else {
            format!("找到 {} 筆文件", entries.len())
        };
        let results = if entries.is_empty() {
            View::new(Node::new(Role::Group, "").styled(Style::column().padding(16.)))
                .children([theme.text("search-overlay-demo-empty", "找不到相符文件")])
        } else {
            theme.list(
                ListOptions {
                    id: "search-overlay-demo-documents",
                    name: "文件",
                    style: if self.style == SearchStyle::Contained {
                        ListStyle::Standard
                    } else {
                        ListStyle::Baseline
                    },
                    mode: ListMode::Action,
                    direction: self.direction(),
                },
                entries.iter().map(|entry| ListItem {
                    id: entry.id,
                    headline: entry.title,
                    supporting: Some(entry.details),
                    overline: None,
                    leading: None,
                    trailing: None,
                    lines: ListLines::Two,
                    disabled: entry.disabled,
                    on_activate: Some(Message::SearchOverlay(Select(entry.id))),
                }),
            )
        };

        theme.search_overlay_with_motion(
            self.overlay_options(Some(&status)),
            SearchOverlayContent {
                results,
                first_result: entries.iter().find(|entry| !entry.disabled).map(|entry| {
                    FocusNavigation {
                        target: entry.id.to_owned(),
                        message: Message::SearchOverlay(FocusResult),
                    }
                }),
                on_submit: Message::SearchOverlay(Submit),
                on_clear: Message::SearchOverlay(Clear),
                on_close: Message::SearchOverlay(Close),
            },
            |query| Message::SearchOverlay(Query(query)),
            &self.motion,
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        let ids: Vec<_> = self.matches().map(|entry| entry.id).collect();
        serde_json::json!({"open":self.open,"style":format!("{:?}",self.style),"rtl":self.rtl,
            "text_scale":self.text_scale(),"query":self.query,"selected":self.selected,
            "submissions":self.submissions,"result_ids":ids,
            "motion":{"progress":self.motion.progress(),"moving":self.motion.is_animating()}})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallery;
    use surface::{Color, TextMeasurer};
    use surface_app::{
        Application, DisplayContrast, EditKey, Event, ImeEvent, Key, KeyPress, Modifiers, Runtime,
        SystemColorPair, SystemContrastColors,
    };

    const INPUT: &str = "search-overlay-demo-input";
    const DIALOG: &str = "search-overlay-demo-dialog";
    const BAR: &str = "search-overlay-demo-bar";
    const STATUS: &str = "search-overlay-demo-status";
    const RESULTS: &str = "search-overlay-demo-results";
    const LAYER: &str = "search-overlay-demo-layer";
    const CLOSING_BAR: &str = "search-overlay-demo-closing-bar";
    const CLOSING_SHELL: &str = "search-overlay-demo-closing-shell";

    struct Metrics;
    impl TextMeasurer for Metrics {
        fn measure(&mut self, value: &str, size: f32, _: Option<f32>) -> (f32, f32) {
            (value.chars().count() as f32 * size * 0.8, size * 1.2)
        }
    }

    fn render(runtime: &mut Runtime<Gallery>, width: f32, height: f32) {
        for _ in 0..3 {
            runtime.render(width, height, &mut Metrics).unwrap();
        }
    }

    fn route(runtime: &mut Runtime<Gallery>, width: f32, height: f32) {
        render(runtime, width, height);
        runtime.apply(Message::Section(SECTION)).unwrap();
        runtime.activate(SHOW).unwrap();
        render(runtime, width, height);
    }

    fn bounds(runtime: &Runtime<Gallery>, id: &str) -> Rect {
        runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
    }

    fn frame(runtime: &mut Runtime<Gallery>, width: f32, height: f32, time: f64) {
        runtime.set_time(time);
        render(runtime, width, height);
    }

    fn background(runtime: &Runtime<Gallery>, id: &str) -> Color {
        runtime
            .dom()
            .get(runtime.dom().by_id(id).unwrap())
            .unwrap()
            .style()
            .background
    }

    #[test]
    fn search_motion_interpolates_both_styles_and_both_presentations() {
        for (width, height) in [(960., 800.), (320., 400.)] {
            for divided in [false, true] {
                let mut runtime = Runtime::new(Gallery::default()).unwrap();
                route(&mut runtime, width, height);
                if divided {
                    runtime.activate("search-overlay-demo-style").unwrap();
                    render(&mut runtime, width, height);
                }
                let rest = bounds(&runtime, OPENER);
                runtime.activate(OPENER).unwrap();
                frame(&mut runtime, width, height, 0.);
                assert_eq!(bounds(&runtime, BAR), rest);
                assert_eq!(bounds(&runtime, RESULTS).height, 0.);
                assert_eq!(runtime.focused(), Some(INPUT));
                assert!(runtime.app().search_overlay_demo.motion.is_animating());
                assert!(!runtime
                    .key(&KeyPress::new(Key::ArrowDown, Modifiers::default()))
                    .unwrap());

                frame(&mut runtime, width, height, 60.);
                let middle = bounds(&runtime, BAR);
                let results = bounds(&runtime, RESULTS);
                let progress = runtime.app().search_overlay_demo.motion.progress();
                assert!(progress > 0. && progress < 1.);
                if divided && width > 600. {
                    assert_eq!(middle, rest);
                } else {
                    assert_ne!(middle, rest);
                }
                assert!(results.y >= middle.y + middle.height - 0.01);
                assert!(results.height > 0.);
                assert!(runtime.dom().by_id("search-overlay-demo-divider").is_some() == divided);
                assert_eq!(bounds(&runtime, "search-overlay-demo-back").width, 48.);
                assert_eq!(runtime.focused(), Some(INPUT));
                if width > 600. {
                    assert_eq!(middle.y, rest.y);
                    let Color(_, _, _, alpha) = background(&runtime, LAYER);
                    assert!((1..82).contains(&alpha));
                }

                frame(&mut runtime, width, height, 1000.);
                assert!(!runtime.app().search_overlay_demo.motion.is_animating());
                assert_eq!(runtime.app().search_overlay_demo.motion.progress(), 1.);
                if divided && width > 600. {
                    assert_eq!(bounds(&runtime, BAR), middle);
                } else {
                    assert_ne!(bounds(&runtime, BAR), middle);
                }
                assert!(bounds(&runtime, RESULTS).height > results.height);
                if width < 600. {
                    assert_eq!(bounds(&runtime, DIALOG), Rect::new(0., 0., width, height));
                }
            }
        }
    }

    #[test]
    fn search_motion_retains_settings_through_frames_and_reverses_without_stealing_input() {
        let (width, height) = (960., 800.);
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        route(&mut runtime, width, height);
        let rest = bounds(&runtime, OPENER);
        runtime.activate(OPENER).unwrap();
        frame(&mut runtime, width, height, 0.);
        runtime
            .dispatch(Event::InputValue {
                value: "設計".into(),
                anchor: 6,
                cursor: 3,
            })
            .unwrap();
        runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
        runtime
            .dispatch(Event::Ime(ImeEvent::Preedit {
                text: "ㄐ".into(),
                selection: Some((0, 3)),
            }))
            .unwrap();
        let initial = runtime.input_snapshot().unwrap();

        frame(&mut runtime, width, height, 60.);
        let editing = runtime.input_snapshot().unwrap();
        assert_eq!(editing.id, initial.id);
        assert_eq!(editing.selection, initial.selection);
        assert_eq!(editing.value, "設計");
        assert!(editing.composing);
        assert_eq!(
            runtime.editor(INPUT).unwrap().composition().unwrap().text,
            "ㄐ"
        );
        assert!(!runtime
            .key(&KeyPress::new(Key::Enter, Modifiers::default()))
            .unwrap());
        assert_eq!(runtime.app().search_overlay_demo.submissions, 0);

        let halfway = bounds(&runtime, BAR);
        runtime.dispatch(Event::Cancel).unwrap();
        assert_eq!(runtime.focused(), Some(INPUT));
        assert!(!runtime.input_snapshot().unwrap().composing);
        runtime.dispatch(Event::Cancel).unwrap();
        assert_eq!(runtime.focused(), Some(OPENER));
        assert!(runtime.dom().by_id(INPUT).is_none());
        frame(&mut runtime, width, height, 60.);
        assert_eq!(bounds(&runtime, CLOSING_BAR), halfway);
        let shell = runtime.dom().by_id(CLOSING_SHELL).unwrap();
        assert!(runtime.accessibility_frame().unwrap().node(shell).is_none());
        assert!(runtime.activate(INPUT).is_err());
        assert!(runtime.app().search_overlay_demo.motion.is_animating());

        let x = rest.x + rest.width / 2.;
        let y = rest.y + rest.height / 2.;
        runtime.dispatch(Event::PointerDown { x, y }).unwrap();
        runtime.dispatch(Event::PointerUp { x, y }).unwrap();
        frame(&mut runtime, width, height, 60.);
        assert_eq!(runtime.focused(), Some(INPUT));
        assert_eq!(bounds(&runtime, BAR), halfway);
        assert!(runtime.dom().by_id(CLOSING_SHELL).is_none());
        assert_eq!(runtime.app().search_overlay_demo.query, "設計");
        frame(&mut runtime, width, height, 1000.);
        assert_eq!(runtime.app().search_overlay_demo.motion.progress(), 1.);
    }

    #[test]
    fn search_motion_keeps_live_palette_and_editor_on_resize_and_respects_reduced_motion() {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        route(&mut runtime, 960., 800.);
        runtime.activate(OPENER).unwrap();
        frame(&mut runtime, 960., 800., 0.);
        runtime
            .dispatch(Event::InputValue {
                value: "設計".into(),
                anchor: 3,
                cursor: 0,
            })
            .unwrap();
        runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
        runtime
            .dispatch(Event::Ime(ImeEvent::Preedit {
                text: "ㄕ".into(),
                selection: None,
            }))
            .unwrap();
        frame(&mut runtime, 960., 800., 40.);
        let before = runtime.input_snapshot().unwrap();
        let progress = runtime.app().search_overlay_demo.motion.progress();
        let button = SystemColorPair {
            background: Color::rgb(10, 25, 40),
            foreground: Color::rgb(255, 255, 255),
        };
        let canvas = SystemColorPair {
            background: Color::rgb(0, 0, 0),
            foreground: Color::rgb(255, 255, 255),
        };
        runtime
            .set_display_contrast(DisplayContrast::Forced(SystemContrastColors::new(
                canvas, button, canvas,
            )))
            .unwrap();
        frame(&mut runtime, 960., 800., 40.);
        assert_eq!(
            background(&runtime, BAR),
            runtime
                .app()
                .effective_theme()
                .colors
                .surface_container_high
        );
        assert_eq!(
            runtime.app().search_overlay_demo.motion.progress(),
            progress
        );

        frame(&mut runtime, 320., 400., 56.);
        assert_eq!(runtime.focused(), Some(INPUT));
        let safe = Rect::new(0., 0., 320., 400.);
        let resized = bounds(&runtime, DIALOG);
        assert_eq!(safe.intersect(resized), resized);
        let after = runtime.input_snapshot().unwrap();
        assert_eq!(after.id, before.id);
        assert_eq!(after.selection, before.selection);
        assert_eq!(after.value, before.value);
        assert_eq!(after.composing, before.composing);
        assert_eq!(
            runtime.editor(INPUT).unwrap().composition().unwrap().text,
            "ㄕ"
        );
        frame(&mut runtime, 320., 400., 1000.);
        assert_eq!(bounds(&runtime, DIALOG), safe);
        assert!(runtime.input_snapshot().unwrap().composing);

        runtime.set_reduced_motion(true);
        runtime.dispatch(Event::Cancel).unwrap();
        runtime.dispatch(Event::Cancel).unwrap();
        frame(&mut runtime, 320., 400., 1016.);
        assert!(!runtime.app().search_overlay_demo.motion.is_animating());
        assert_eq!(runtime.app().search_overlay_demo.motion.progress(), 0.);
        assert_eq!(runtime.focused(), Some(OPENER));
        assert!(runtime.dom().by_id(CLOSING_SHELL).is_none());
        runtime.activate(OPENER).unwrap();
        frame(&mut runtime, 320., 400., 1032.);
        assert_eq!(runtime.app().search_overlay_demo.motion.progress(), 1.);
        assert!(!runtime.app().search_overlay_demo.motion.is_animating());
        assert_eq!(runtime.focused(), Some(INPUT));
    }

    #[test]
    fn gallery_route_has_all_style_layout_combinations() {
        for compact in [false, true] {
            let (width, height) = if compact { (320., 400.) } else { (960., 800.) };
            for divided in [false, true] {
                let mut runtime = Runtime::new(Gallery::default()).unwrap();
                route(&mut runtime, width, height);
                if compact {
                    for id in [
                        "search-overlay-demo-theme",
                        "search-overlay-demo-rtl",
                        "search-overlay-demo-large-text",
                    ] {
                        runtime.activate(id).unwrap();
                    }
                }
                if divided {
                    runtime.activate("search-overlay-demo-style").unwrap();
                }
                render(&mut runtime, width, height);
                let anchor = bounds(&runtime, ANCHOR);
                let opener = bounds(&runtime, OPENER);
                assert_eq!(opener.height, 56.);
                assert_eq!(anchor.intersect(opener), opener);
                let label = bounds(&runtime, "search-overlay-demo-opener-label");
                if compact {
                    assert_eq!(opener.x + opener.width - label.x - label.width, 16.);
                } else {
                    assert_eq!(label.x - opener.x, 16.);
                }
                runtime.activate(OPENER).unwrap();
                render(&mut runtime, width, height);

                assert_eq!(runtime.focused(), Some(INPUT));
                assert_eq!(runtime.app().text_scale(), if compact { 2. } else { 1. });
                assert_eq!(runtime.app().dark, compact);
                let dialog = bounds(&runtime, DIALOG);
                let bar = bounds(&runtime, BAR);
                assert_eq!(bar.height, if compact && divided { 72. } else { 56. });
                assert_eq!(
                    runtime.dom().by_id("search-overlay-demo-divider").is_some(),
                    divided
                );
                if compact {
                    assert_eq!(dialog, Rect::new(0., 0., width, height));
                    assert_eq!(bounds(&runtime, "search-overlay-demo-back").width, 48.);
                    assert!(
                        bounds(&runtime, "search-overlay-demo-back").x > bounds(&runtime, INPUT).x
                    );
                } else {
                    assert_eq!(dialog.y, anchor.y);
                    assert!((360. ..=720.).contains(&dialog.width));
                    assert_eq!(opener.x - bar.x, if divided { 0. } else { 12. });
                }
                assert!(runtime.dom().by_id("search-doc-archive").is_some());
                runtime.dispatch(Event::Cancel).unwrap();
                assert_eq!(runtime.focused(), Some(OPENER));
            }
        }
    }

    #[test]
    fn gallery_query_history_submit_and_results_are_controlled() {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        route(&mut runtime, 960., 800.);
        runtime.activate(OPENER).unwrap();
        render(&mut runtime, 960., 800.);
        let status = |runtime: &Runtime<Gallery>| {
            runtime
                .dom()
                .get(runtime.dom().by_id(STATUS).unwrap())
                .unwrap()
                .name()
                .to_owned()
        };
        assert_eq!(status(&runtime), "近期文件，共 12 筆");

        let enter = KeyPress::new(Key::Enter, Modifiers::default());
        let down = KeyPress::new(Key::ArrowDown, Modifiers::default());
        runtime.dispatch(Event::Ime(ImeEvent::Begin)).unwrap();
        runtime
            .dispatch(Event::Ime(ImeEvent::Preedit {
                text: "ㄕ".into(),
                selection: None,
            }))
            .unwrap();
        assert!(!runtime.key(&enter).unwrap());
        assert!(!runtime.key(&down).unwrap());
        assert_eq!(runtime.focused(), Some(INPUT));
        assert_eq!(runtime.app().search_overlay_demo.submissions, 0);
        runtime
            .dispatch(Event::Ime(ImeEvent::Commit("設計".into())))
            .unwrap();
        assert_eq!(runtime.app().search_overlay_demo.query, "設計");
        assert!(runtime.key(&enter).unwrap());
        assert_eq!(runtime.app().search_overlay_demo.submissions, 1);
        render(&mut runtime, 960., 800.);
        assert_eq!(status(&runtime), "找到 4 筆文件");

        assert!(runtime.key(&down).unwrap());
        assert_eq!(runtime.focused(), Some("search-doc-design"));
        assert_eq!(runtime.app().search_overlay_demo.selected, None);
        for expected in ["search-overlay-demo-clear", INPUT] {
            runtime
                .dispatch(Event::FocusNext { reverse: true })
                .unwrap();
            assert_eq!(runtime.focused(), Some(expected));
        }

        for expected in ["search-overlay-demo-clear", "search-doc-design"] {
            runtime
                .dispatch(Event::FocusNext { reverse: false })
                .unwrap();
            assert_eq!(runtime.focused(), Some(expected));
        }
        runtime
            .dispatch(Event::Edit {
                key: EditKey::Down,
                extend: false,
                by_word: false,
            })
            .unwrap();
        assert_eq!(runtime.focused(), Some("search-doc-review"));
        runtime.dispatch(Event::Activate).unwrap();
        assert_eq!(
            runtime.app().search_overlay_demo.selected,
            Some("search-doc-review")
        );
        assert_eq!(runtime.focused(), Some(OPENER));

        runtime.activate(OPENER).unwrap();
        runtime
            .dispatch(Event::InputValue {
                value: "封存".into(),
                anchor: 6,
                cursor: 6,
            })
            .unwrap();
        assert_eq!(status(&runtime), "找到 1 筆文件");
        assert!(!runtime.key(&down).unwrap());
        assert_eq!(runtime.focused(), Some(INPUT));
        runtime
            .dispatch(Event::InputValue {
                value: "無匹配".into(),
                anchor: 9,
                cursor: 9,
            })
            .unwrap();
        assert!(runtime.dom().by_id("search-overlay-demo-empty").is_some());
        assert_eq!(status(&runtime), "找不到相符文件");
        assert!(!runtime.key(&down).unwrap());
        assert_eq!(runtime.focused(), Some(INPUT));
    }
}
