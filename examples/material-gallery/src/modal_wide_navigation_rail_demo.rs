use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, HorizontalDrag, View};
use surface_material::{
    shape::LayoutDirection, ChoiceState, IconButtonKind, ModalWideNavigationRailCollapse,
    ModalWideNavigationRailOptions, ModalWideNavigationRailSwipeOptions, NavigationRailItem, Theme,
    WideNavigationRailMode, WideNavigationRailMotion, WideNavigationRailMotionKind,
    WideNavigationRailSwipe,
};

use crate::{rail_icons::Destination, Message};

pub(crate) const SECTION: usize = 1;
const CARD: &str = "modal-wide-rail-card";
const DIALOG: &str = "modal-wide-rail";
const INLINE: &str = "modal-wide-rail-inline";
const OPEN: &str = "modal-wide-rail-open";
const RETAIN: &str = "modal-wide-rail-retain";
const DIRECTION: &str = "modal-wide-rail-direction";
const SCALE: &str = "modal-wide-rail-scale";
const CONTENT: &str = "modal-wide-rail-content";
const PAGE: &str = "modal-wide-rail-page";
const NAVIGATION_NAME: &str = "工作區目的地";
const OPEN_NAME: &str = "開啟導覽列";

struct Page {
    key: &'static str,
    label: &'static str,
    long_label: Option<&'static str>,
    icon: Destination,
    disabled: bool,
}

const PAGES: [Page; 5] = [
    Page {
        key: "home",
        label: "首頁",
        long_label: None,
        icon: Destination::Home,
        disabled: false,
    },
    Page {
        key: "archive",
        label: "封存",
        long_label: None,
        icon: Destination::Archive,
        disabled: true,
    },
    Page {
        key: "inbox",
        label: "收件匣",
        long_label: Some("跨部門協作與專案通知收件匣"),
        icon: Destination::Inbox,
        disabled: false,
    },
    Page {
        key: "reports",
        label: "報告",
        long_label: None,
        icon: Destination::Archive,
        disabled: false,
    },
    Page {
        key: "settings",
        label: "設定",
        long_label: None,
        icon: Destination::Home,
        disabled: false,
    },
];

#[derive(Clone, Copy)]
pub enum ModalWideRailEvent {
    Open,
    Dismiss,
    Retain,
    Direction,
    Select(usize),
    Swipe(HorizontalDrag, f32),
}

pub(crate) struct ModalWideRailDemo {
    open: bool,
    retain_inline: bool,
    rtl: bool,
    selected: usize,
    actions: usize,
    dismissals: usize,
    motion: WideNavigationRailMotion,
    swipe: WideNavigationRailSwipe,
}

impl Default for ModalWideRailDemo {
    fn default() -> Self {
        Self {
            open: false,
            retain_inline: false,
            rtl: false,
            selected: 0,
            actions: 0,
            dismissals: 0,
            motion: WideNavigationRailMotion::new(false, WideNavigationRailMotionKind::Modal),
            swipe: WideNavigationRailSwipe::default(),
        }
    }
}

impl ModalWideRailDemo {
    pub(crate) fn update(&mut self, event: ModalWideRailEvent) {
        match event {
            ModalWideRailEvent::Open => {
                self.swipe.reset();
                self.open = true;
            }
            ModalWideRailEvent::Dismiss if self.open => {
                self.swipe.reset();
                self.dismissals = self.dismissals.saturating_add(1);
                self.open = false;
            }
            ModalWideRailEvent::Retain => {
                self.swipe.reset();
                self.retain_inline = !self.retain_inline;
            }
            ModalWideRailEvent::Direction => {
                self.swipe.reset();
                self.rtl = !self.rtl;
            }
            ModalWideRailEvent::Select(index)
                if PAGES.get(index).is_some_and(|page| !page.disabled) =>
            {
                self.swipe.reset();
                self.selected = index;
                self.actions = self.actions.saturating_add(1);
                self.open = false;
            }
            ModalWideRailEvent::Swipe(drag, panel_width) if self.open && !self.retain_inline => {
                self.swipe.apply(drag, panel_width, self.direction());
            }
            _ => {}
        }
    }

    pub(crate) fn close(&mut self) {
        self.swipe.reset();
        self.open = false;
        self.motion.snap(false);
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let changed = self.swipe.set_viewport(width, height);
        changed && !self.retain_inline && (self.open || self.motion.is_running())
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        let now = frame.now_ms.filter(|_| frame.dom.by_id(CARD).is_some());
        let (swipe, dismiss) = self.swipe.advance(now, frame.reduced_motion);
        if self.open && dismiss {
            self.dismissals = self.dismissals.saturating_add(1);
            self.close();
        }
        let motion = self.motion.advance(self.open, now, frame.reduced_motion);
        AnimationUpdate {
            rebuild: motion.rebuild || swipe.rebuild || dismiss,
            request_next: motion.request_next || swipe.request_next,
        }
    }

    pub(crate) fn card(&self, theme: &Theme, enlarged: bool) -> View<Message> {
        let controls = theme.row([
            theme
                .icon_button(
                    DIRECTION,
                    if self.rtl {
                        "從左到右"
                    } else {
                        "從右到左"
                    },
                    if self.rtl { "→" } else { "←" },
                    IconButtonKind::Tonal,
                    Some(self.rtl),
                    false,
                    Message::ModalWideRail(ModalWideRailEvent::Direction),
                )
                .accessible_activation(),
            theme
                .icon_button(
                    SCALE,
                    "放大文字",
                    "字",
                    IconButtonKind::Tonal,
                    Some(enlarged),
                    false,
                    Message::RailLargeText,
                )
                .accessible_activation(),
            theme
                .switch(
                    RETAIN,
                    "保留窄列",
                    ChoiceState {
                        selected: self.retain_inline,
                        disabled: false,
                    },
                    Message::ModalWideRail(ModalWideRailEvent::Retain),
                )
                .accessible_activation(),
        ]);
        let event = Message::ModalWideRail(ModalWideRailEvent::Open);
        let mut page_style = Style::column()
            .gap(8.)
            .padding(12.)
            .background(theme.colors.surface_container_low);
        let mut content =
            vec![theme.text(PAGE, format!("目前頁面：{}", PAGES[self.selected].label))];
        let mut row = Node::new(Role::Group, "").styled(Style::row().align(Align::Stretch).gap(8.));
        if self.rtl {
            row = row.reverse_row();
        }
        let inline = if self.retain_inline {
            page_style = page_style.width(Length::Px(0.)).grow(1.);
            let open = theme
                .icon_button(
                    OPEN,
                    OPEN_NAME,
                    if self.rtl { "«" } else { "»" },
                    IconButtonKind::Standard,
                    None,
                    false,
                    event,
                )
                .disclosure_expanded(self.open)
                .accessible_activation();
            let header =
                View::new(Node::new(Role::Group, "").styled(Style::row().justify(Align::Center)))
                    .children([open]);
            Some(theme.wide_navigation_rail(
                INLINE,
                NAVIGATION_NAME,
                WideNavigationRailMode::Collapsed,
                Some(header),
                self.items(theme, WideNavigationRailMode::Collapsed),
            ))
        } else {
            page_style = page_style.width(Length::Percent(1.)).align(if self.rtl {
                Align::End
            } else {
                Align::Start
            });
            content.push(
                theme
                    .tonal_button(OPEN, OPEN_NAME, event)
                    .disclosure_expanded(self.open)
                    .accessible_activation(),
            );
            None
        };
        let content =
            View::new(Node::new(Role::Group, "").id(CONTENT).styled(page_style)).children(content);
        let preview = View::new(row).children(inline.into_iter().chain([content]));
        theme.card(
            CARD,
            [
                theme.text("modal-wide-rail-heading", "模態寬導覽列"),
                controls,
                preview,
            ],
        )
    }

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn items<'a>(
        &'a self,
        theme: &'a Theme,
        mode: WideNavigationRailMode,
    ) -> impl Iterator<Item = View<Message>> + 'a {
        PAGES.iter().enumerate().map(move |(index, page)| {
            let (prefix, label) = match mode {
                WideNavigationRailMode::Collapsed => (INLINE, page.label),
                WideNavigationRailMode::Expanded => (DIALOG, page.long_label.unwrap_or(page.label)),
            };
            let id = format!("{prefix}-{}", page.key);
            let item = NavigationRailItem {
                id: &id,
                label,
                accessible_name: label,
                icon: page.icon.icon(),
                state: ChoiceState {
                    selected: index == self.selected,
                    disabled: page.disabled,
                },
                message: Message::ModalWideRail(ModalWideRailEvent::Select(index)),
            };
            let direction = self.direction();
            if self.retain_inline && mode == WideNavigationRailMode::Expanded {
                theme.animated_wide_navigation_rail_item_with_icon(
                    self.motion.frame().slide,
                    direction,
                    item,
                )
            } else {
                theme.wide_navigation_rail_item_with_icon(mode, direction, item)
            }
        })
    }

    pub(crate) fn layer(&self, theme: &Theme) -> Option<View<Message>> {
        let options = ModalWideNavigationRailOptions {
            id: DIALOG,
            name: "模態導覽列",
            navigation_name: NAVIGATION_NAME,
            close_name: "關閉導覽列",
            direction: self.direction(),
        };
        let items = self.items(theme, WideNavigationRailMode::Expanded);
        let dismiss = Message::ModalWideRail(ModalWideRailEvent::Dismiss);
        let frame = self.motion.frame();
        if self.retain_inline {
            theme.animated_modal_wide_navigation_rail(
                options,
                items,
                dismiss,
                self.open,
                ModalWideNavigationRailCollapse::Retain,
                frame,
            )
        } else {
            theme.swipeable_modal_wide_navigation_rail(
                options,
                items,
                dismiss,
                ModalWideNavigationRailSwipeOptions {
                    open: self.open,
                    frame,
                    swipe: &self.swipe,
                },
                |drag, width| Message::ModalWideRail(ModalWideRailEvent::Swipe(drag, width)),
            )
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        text_scale: f32,
        runtime: &surface_app::Runtime<crate::Gallery>,
    ) -> serde_json::Value {
        let frame = self.motion.frame();
        serde_json::json!({
            "open": self.open,
            "retain_inline": self.retain_inline,
            "rtl": self.rtl,
            "selected": PAGES[self.selected].key,
            "actions": self.actions,
            "dismissals": self.dismissals,
            "settling": self.motion.is_running(),
            "rail": frame.rail,
            "slide": frame.slide,
            "scrim": frame.scrim,
            "swipe": self.swipe.progress(),
            "swipe_settling": self.swipe.is_settling(),
            "text_scale": text_scale,
            "scroll": runtime.scroll_metrics("modal-wide-rail-navigation").map(|metrics| metrics.offset.1),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallery;
    use surface::{Rect, TextMeasurer};
    use surface_app::{Application, EditKey, Event, Runtime, ScrollTarget};

    struct Metrics;

    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
            let intrinsic = size * text.chars().count() as f32;
            let line_width = intrinsic.min(width.unwrap_or(intrinsic).max(size));
            let lines = (intrinsic / line_width.max(size)).ceil().max(1.);
            (line_width, size * 1.5 * lines)
        }
    }

    fn bounds(runtime: &Runtime<Gallery>, id: &str) -> Rect {
        runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
    }

    fn node<'a>(runtime: &'a Runtime<Gallery>, id: &str) -> &'a Node {
        runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
    }

    fn click_opener(runtime: &mut Runtime<Gallery>, width: f32, height: f32) {
        let before = bounds(runtime, OPEN);
        let offset = runtime.scroll_metrics("surface-page").unwrap().offset;
        assert!(runtime.scroll_to(
            "surface-page",
            ScrollTarget::Offset {
                x: offset.0,
                y: offset.1 + before.y - 64.
            },
        ));
        runtime.render(width, height, &mut Metrics).unwrap();
        let opener = bounds(runtime, OPEN);
        assert_eq!(opener.intersect(Rect::new(0., 0., width, height)), opener);
        let x = opener.x + opener.width / 2.;
        let y = opener.y + opener.height / 2.;
        runtime.dispatch(Event::PointerDown { x, y }).unwrap();
        runtime.dispatch(Event::PointerUp { x, y }).unwrap();
        assert!(runtime.app().modal_wide_rail_demo.open);
        assert_eq!(runtime.focused(), Some("modal-wide-rail-close"));
    }

    #[test]
    fn gallery_modal_rail_animates_the_scrim_without_delaying_focus_restoration() {
        for retained in [false, true] {
            let mut runtime = Runtime::new(Gallery::default()).unwrap();
            runtime.set_time(0.);
            runtime.render(960., 640., &mut Metrics).unwrap();
            runtime.activate("section-1").unwrap();
            if retained {
                runtime.activate(RETAIN).unwrap();
            }
            runtime.render(960., 640., &mut Metrics).unwrap();
            click_opener(&mut runtime, 960., 640.);
            runtime.render(960., 640., &mut Metrics).unwrap();
            assert_eq!(
                node(&runtime, "modal-wide-rail-layer").style().background.3,
                0
            );

            runtime.set_time(40.);
            runtime.render(960., 640., &mut Metrics).unwrap();
            let alpha = node(&runtime, "modal-wide-rail-layer").style().background.3;
            assert!((1..82).contains(&alpha));
            let content = bounds(&runtime, CONTENT);

            runtime.dispatch(Event::Cancel).unwrap();
            assert_eq!(runtime.focused(), Some(OPEN));
            assert!(runtime.dom().by_id(DIALOG).is_none());
            runtime.render(960., 640., &mut Metrics).unwrap();
            assert!(runtime
                .accessibility_frame()
                .unwrap()
                .modal_root()
                .is_none());
            assert_eq!(bounds(&runtime, CONTENT), content);
            assert!(runtime.app().modal_wide_rail_demo.motion.is_running());

            runtime.activate(OPEN).unwrap();
            runtime.render(960., 640., &mut Metrics).unwrap();
            assert_eq!(
                node(&runtime, "modal-wide-rail-layer").style().background.3,
                alpha
            );
            runtime.set_reduced_motion(true);
            runtime.render(960., 640., &mut Metrics).unwrap();
            assert_eq!(
                node(&runtime, "modal-wide-rail-layer").style().background.3,
                82
            );
            assert!(!runtime.app().modal_wide_rail_demo.motion.is_running());
            assert_eq!(runtime.focused(), Some("modal-wide-rail-close"));
            runtime.dispatch(Event::Cancel).unwrap();
            runtime.render(960., 640., &mut Metrics).unwrap();
            assert!(!runtime.app().modal_wide_rail_demo.motion.is_running());
            assert_eq!(runtime.focused(), Some(OPEN));
        }
    }

    #[test]
    fn gallery_modal_rail_keeps_its_two_collapsed_layouts_across_resize() {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        runtime.render(960., 640., &mut Metrics).unwrap();
        runtime.activate("section-1").unwrap();
        runtime.activate(DIRECTION).unwrap();
        runtime.activate(SCALE).unwrap();
        assert_eq!(runtime.app().text_scale(), 2.);
        runtime.render(960., 640., &mut Metrics).unwrap();
        assert!(runtime.dom().by_id(INLINE).is_none());
        assert!(runtime.dom().by_id(DIALOG).is_none());
        assert_eq!(node(&runtime, OPEN).expanded_state(), Some(false));
        click_opener(&mut runtime, 960., 640.);
        runtime.render(960., 640., &mut Metrics).unwrap();
        let panel = bounds(&runtime, DIALOG);
        assert_eq!(panel.x + panel.width, 960.);
        let scope = runtime.accessibility_frame().unwrap().modal_root();
        assert_eq!(scope, runtime.dom().by_id(DIALOG));
        assert_eq!(
            node(&runtime, "modal-wide-rail-home").is_current_page(),
            Some(true)
        );
        assert!(runtime.activate(RETAIN).is_err());

        runtime.render(320., 240., &mut Metrics).unwrap();
        let panel = bounds(&runtime, DIALOG);
        assert_eq!(panel.x + panel.width, 320.);
        assert_eq!(panel.height, 240.);
        assert_eq!(runtime.focused(), Some("modal-wide-rail-close"));
        runtime
            .dispatch(Event::FocusNext { reverse: false })
            .unwrap();
        runtime
            .dispatch(Event::Edit {
                key: EditKey::Down,
                extend: false,
                by_word: false,
            })
            .unwrap();
        runtime.render(320., 240., &mut Metrics).unwrap();
        assert_eq!(runtime.focused(), Some("modal-wide-rail-inbox"));
        assert!(
            runtime
                .scroll_metrics("modal-wide-rail-navigation")
                .unwrap()
                .offset
                .1
                > 0.
        );
        runtime.dispatch(Event::Activate).unwrap();
        assert_eq!(runtime.focused(), Some(OPEN));
        assert_eq!(runtime.app().modal_wide_rail_demo.actions, 1);
        assert_eq!(node(&runtime, PAGE).text_value(), Some("目前頁面：收件匣"));
        assert!(runtime.dom().by_id(DIALOG).is_none());

        runtime.activate(RETAIN).unwrap();
        runtime.render(320., 240., &mut Metrics).unwrap();
        let inline = bounds(&runtime, INLINE);
        let content = bounds(&runtime, CONTENT);
        assert_eq!(inline.width, 96.);
        assert!(content.x + content.width < inline.x);
        assert_eq!(
            node(&runtime, "modal-wide-rail-inline-inbox").is_current_page(),
            Some(true)
        );
        click_opener(&mut runtime, 320., 240.);
        runtime.render(320., 240., &mut Metrics).unwrap();
        assert_eq!(bounds(&runtime, INLINE).width, inline.width);
        assert_eq!(bounds(&runtime, CONTENT).width, content.width);
        assert_eq!(
            node(&runtime, "modal-wide-rail-inbox").is_current_page(),
            Some(true)
        );
        let frame = runtime.accessibility_frame().unwrap();
        assert!(frame.node(runtime.dom().by_id(INLINE).unwrap()).is_none());
        runtime.dispatch(Event::Cancel).unwrap();
        assert_eq!(runtime.focused(), Some(OPEN));
        assert_eq!(runtime.app().modal_wide_rail_demo.dismissals, 1);
        assert!(runtime.dom().by_id(DIALOG).is_none());
    }
}
