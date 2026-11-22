use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    adaptive::WindowWidthSizeClass, shape::LayoutDirection, ChoiceState, IconButtonKind,
    NavigationRailItem, Theme, WideNavigationRailMotion, WideNavigationRailMotionKind,
};

use crate::{rail_icons::Destination, Message};

pub(crate) const SECTION: usize = 1;

const NAVIGATION: &str = "wide-rail";
const BOTTOM: &str = "wide-rail-bottom";
const TOGGLE: &str = "wide-rail-toggle";
const LAYOUT: &str = "wide-rail-layout";
const CONTENT: &str = "wide-rail-content";
const PAGE: &str = "wide-rail-page";
const DIRECTION: &str = "wide-rail-direction";
const SCALE: &str = "wide-rail-scale";
const NAME: &str = "工作區導覽";

struct Page {
    id: &'static str,
    label: &'static str,
    rail_label: Option<&'static str>,
    name: &'static str,
    destination: Destination,
}

const HOME: Page = Page {
    id: "wide-rail-home",
    label: "首頁",
    rail_label: None,
    name: "瀏覽首頁",
    destination: Destination::Home,
};
const ARCHIVE: Page = Page {
    id: "wide-rail-archive",
    label: "封存",
    rail_label: None,
    name: "封存資料",
    destination: Destination::Archive,
};
const INBOX: Page = Page {
    id: "wide-rail-inbox",
    label: "收件",
    rail_label: Some("跨部門協作與專案通知收件匣"),
    name: "收件匣",
    destination: Destination::Inbox,
};
const PAGES: [Page; 3] = [HOME, ARCHIVE, INBOX];

impl Page {
    fn state(&self, selected: Destination) -> ChoiceState {
        ChoiceState {
            selected: self.destination == selected,
            disabled: self.destination == Destination::Archive,
        }
    }
}

#[derive(Clone, Copy)]
pub enum WideRailEvent {
    Toggle,
    Direction,
    Select(Destination),
}

pub(crate) struct WideRailDemo {
    expanded: bool,
    compact: bool,
    rtl: bool,
    selected: Destination,
    actions: usize,
    motion: WideNavigationRailMotion,
}

impl Default for WideRailDemo {
    fn default() -> Self {
        Self {
            expanded: false,
            compact: false,
            rtl: false,
            selected: Destination::default(),
            actions: 0,
            motion: WideNavigationRailMotion::new(false, WideNavigationRailMotionKind::Inline),
        }
    }
}

impl WideRailDemo {
    pub(crate) fn update(&mut self, event: WideRailEvent) {
        match event {
            WideRailEvent::Toggle if !self.compact => self.expanded = !self.expanded,
            WideRailEvent::Direction => self.rtl = !self.rtl,
            WideRailEvent::Select(page) if page != Destination::Archive => {
                self.selected = page;
                self.actions = self.actions.saturating_add(1);
            }
            _ => {}
        }
    }

    pub(crate) fn viewport_changed(&mut self, width: f32) -> bool {
        let compact = WindowWidthSizeClass::from_dp(width) == Some(WindowWidthSizeClass::Compact);
        let changed = self.compact != compact;
        self.compact = compact;
        if changed {
            self.motion.snap(self.expanded);
        }
        changed
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        let now = frame
            .now_ms
            .filter(|_| frame.dom.by_id(NAVIGATION).is_some());
        self.motion
            .advance(self.expanded, now, frame.reduced_motion)
    }

    pub(crate) fn card(&self, theme: &Theme, enlarged: bool) -> View<Message> {
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let direction_name = if self.rtl {
            "從左到右"
        } else {
            "從右到左"
        };
        let controls = theme.row([
            theme
                .icon_button(
                    DIRECTION,
                    direction_name,
                    if self.rtl { "→" } else { "←" },
                    IconButtonKind::Tonal,
                    Some(self.rtl),
                    false,
                    Message::WideRail(WideRailEvent::Direction),
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
        ]);
        let selected_page = self.page();
        let mut page_style = Style::column()
            .padding(12.)
            .background(theme.colors.surface_container_low);
        if !self.compact {
            page_style = page_style.width(Length::Px(0.)).grow(1.);
        }
        let content = View::new(Node::new(Role::Group, "").id(CONTENT).styled(page_style))
            .children([theme.text(PAGE, selected_page.name)]);
        let layout = if self.compact {
            let bottom = theme.navigation_bar_with_direction(
                BOTTOM,
                NAME,
                direction,
                PAGES.iter().map(|page| {
                    let state = page.state(self.selected);
                    theme
                        .navigation_bar_item_with_icon(NavigationRailItem {
                            id: page.id,
                            label: page.label,
                            accessible_name: page.name,
                            icon: page.destination.icon(),
                            state,
                            message: Message::WideRail(WideRailEvent::Select(page.destination)),
                        })
                        .current_page(state.selected)
                        .accessible_activation()
                }),
            );
            View::new(
                Node::new(Role::Group, "")
                    .id(LAYOUT)
                    .styled(Style::column()),
            )
            .children([content, bottom])
        } else {
            let motion = self.motion.frame();
            let expanded_layout = motion.rail > 0.;
            let toggle = theme
                .icon_button(
                    TOGGLE,
                    if self.expanded {
                        "收合導覽列"
                    } else {
                        "展開導覽列"
                    },
                    if self.rtl == self.expanded {
                        "»"
                    } else {
                        "«"
                    },
                    IconButtonKind::Standard,
                    None,
                    false,
                    Message::WideRail(WideRailEvent::Toggle),
                )
                .disclosure_expanded(self.expanded)
                .restore_focus_on_removal(selected_page.id)
                .accessible_activation();
            let header_alignment = match (expanded_layout, self.rtl) {
                (true, false) => Align::Start,
                (true, true) => Align::End,
                (false, _) => Align::Center,
            };
            let header = View::new(
                Node::new(Role::Group, "").id("wide-rail-header").styled(
                    Style::row()
                        .justify(header_alignment)
                        .width(Length::Percent(1.)),
                ),
            )
            .children([toggle]);
            let rail = theme.animated_wide_navigation_rail(
                NAVIGATION,
                NAME,
                motion,
                Some(header),
                PAGES.iter().map(|page| {
                    theme.animated_wide_navigation_rail_item_with_icon(
                        motion.rail,
                        direction,
                        NavigationRailItem {
                            id: page.id,
                            label: page.rail_label.unwrap_or(page.label),
                            accessible_name: page.name,
                            icon: page.destination.icon(),
                            state: page.state(self.selected),
                            message: Message::WideRail(WideRailEvent::Select(page.destination)),
                        },
                    )
                }),
            );
            let row = Node::new(Role::Group, "")
                .id(LAYOUT)
                .styled(Style::row().align(Align::Stretch).gap(8.));
            let row = if self.rtl { row.reverse_row() } else { row };
            View::new(row).children([rail, content])
        };
        theme.card(
            "wide-rail-card",
            [
                theme.text("wide-rail-heading", "寬導覽列"),
                controls,
                layout,
            ],
        )
    }

    fn page(&self) -> &Page {
        PAGES
            .iter()
            .find(|page| page.destination == self.selected)
            .unwrap_or(&HOME)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self, text_scale: f32) -> serde_json::Value {
        let mode = match (self.compact, self.expanded) {
            (true, _) => "bottom",
            (false, true) => "expanded",
            (false, false) => "collapsed",
        };
        serde_json::json!({
            "mode": mode,
            "expanded": self.expanded,
            "compact": self.compact,
            "rtl": self.rtl,
            "selected": self.page().id,
            "actions": self.actions,
            "settling": self.motion.is_running(),
            "rail": self.motion.frame().rail,
            "text_scale": text_scale,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallery;
    use surface::{Rect, TextMeasurer};
    use surface_app::{AppError, Application, EditKey, Event, Runtime};

    struct Metrics;

    impl TextMeasurer for Metrics {
        fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
            (text.chars().count() as f32 * size, size * 1.5)
        }
    }

    fn bounds(runtime: &Runtime<Gallery>, id: &str) -> Rect {
        runtime.computed().unwrap().nodes[runtime.dom().by_id(id).unwrap().index()].bounds
    }

    fn node<'a>(runtime: &'a Runtime<Gallery>, id: &str) -> &'a Node {
        runtime.dom().get(runtime.dom().by_id(id).unwrap()).unwrap()
    }

    fn arrow(runtime: &mut Runtime<Gallery>, key: EditKey) {
        assert!(
            runtime
                .dispatch(Event::Edit {
                    key,
                    extend: false,
                    by_word: false,
                })
                .unwrap(),
            "{key:?} was not handled with focus on {:?}",
            runtime.focused()
        );
    }

    #[test]
    fn gallery_wide_rail_animates_reflow_and_settles_for_reduced_motion_and_compact_layout() {
        let app = Gallery {
            section: SECTION,
            ..Default::default()
        };
        let mut runtime = Runtime::new(app).unwrap();
        runtime.set_time(0.);
        runtime.render(960., 800., &mut Metrics).unwrap();
        let collapsed = bounds(&runtime, NAVIGATION);
        let content = bounds(&runtime, CONTENT);

        runtime.activate(TOGGLE).unwrap();
        runtime.render(960., 800., &mut Metrics).unwrap();
        assert_eq!(bounds(&runtime, NAVIGATION).width, collapsed.width);
        assert_eq!(node(&runtime, TOGGLE).expanded_state(), Some(true));

        runtime.set_time(40.);
        runtime.render(960., 800., &mut Metrics).unwrap();
        let growing = bounds(&runtime, NAVIGATION);
        assert!(growing.width > collapsed.width);
        assert_eq!(
            growing.width - collapsed.width,
            content.width - bounds(&runtime, CONTENT).width
        );
        assert_eq!(runtime.focused(), Some(TOGGLE));

        runtime.set_reduced_motion(true);
        runtime.render(960., 800., &mut Metrics).unwrap();
        let expanded = bounds(&runtime, NAVIGATION);
        assert!(expanded.width > growing.width);
        assert!(!runtime.app().wide_rail_demo.motion.is_running());

        runtime.set_reduced_motion(false);
        runtime.activate(TOGGLE).unwrap();
        runtime.render(960., 800., &mut Metrics).unwrap();
        assert!(runtime.app().wide_rail_demo.motion.is_running());
        runtime.render(599., 760., &mut Metrics).unwrap();
        assert!(runtime.dom().by_id(BOTTOM).is_some());
        assert_eq!(runtime.focused(), Some(HOME.id));
        assert!(!runtime.app().wide_rail_demo.motion.is_running());
        runtime.render(600., 760., &mut Metrics).unwrap();
        assert_eq!(bounds(&runtime, NAVIGATION).width, collapsed.width);
        assert_eq!(runtime.focused(), Some(HOME.id));
    }

    #[test]
    fn gallery_wide_rail_reflows_and_retains_destination_focus_and_actions() {
        let app = Gallery {
            section: SECTION,
            ..Default::default()
        };
        let mut runtime = Runtime::new(app).unwrap();
        runtime.render(960., 3000., &mut Metrics).unwrap();
        let compact_rail = bounds(&runtime, NAVIGATION);
        let initial_content = bounds(&runtime, CONTENT);
        assert!(initial_content.x > compact_rail.x + compact_rail.width);
        assert_eq!(node(&runtime, TOGGLE).expanded_state(), Some(false));
        assert!(runtime.dom().by_id(BOTTOM).is_none());

        assert!(runtime.activate(TOGGLE).unwrap());
        runtime.render(960., 3000., &mut Metrics).unwrap();
        assert_eq!(runtime.focused(), Some(TOGGLE));
        assert_eq!(node(&runtime, TOGGLE).expanded_state(), Some(true));
        assert!(bounds(&runtime, NAVIGATION).width > compact_rail.width);
        assert!(bounds(&runtime, CONTENT).width < initial_content.width);
        assert!(runtime.activate(DIRECTION).unwrap());
        assert!(runtime.activate(SCALE).unwrap());
        runtime.render(960., 3000., &mut Metrics).unwrap();
        let rail = bounds(&runtime, NAVIGATION);
        let layout = bounds(&runtime, LAYOUT);
        let content = bounds(&runtime, CONTENT);
        assert_eq!(rail.x + rail.width, layout.x + layout.width);
        assert!(content.x + content.width < rail.x);
        assert_eq!(runtime.app().text_scale(), 2.);

        assert!(matches!(
            runtime.activate(ARCHIVE.id),
            Err(AppError::MissingControl(id)) if id == ARCHIVE.id
        ));
        assert!(runtime.activate(HOME.id).unwrap());
        runtime.render(960., 3000., &mut Metrics).unwrap();
        arrow(&mut runtime, EditKey::Down);
        assert_eq!(runtime.focused(), Some(INBOX.id));
        assert_eq!(runtime.app().wide_rail_demo.actions, 1);
        assert_eq!(runtime.app().wide_rail_demo.selected, Destination::Home);

        runtime.render(599., 3000., &mut Metrics).unwrap();
        assert_eq!(runtime.focused(), Some(INBOX.id));
        assert!(runtime.dom().by_id(NAVIGATION).is_none());
        assert!(runtime.dom().by_id(TOGGLE).is_none());
        runtime.render(320., 3000., &mut Metrics).unwrap();
        let bottom = bounds(&runtime, BOTTOM);
        let content = bounds(&runtime, CONTENT);
        assert!(bottom.y >= content.y + content.height);
        assert!(bounds(&runtime, HOME.id).x > bounds(&runtime, INBOX.id).x);
        assert!(PAGES
            .iter()
            .all(|page| bounds(&runtime, page.id).width >= 48.));
        assert!(runtime.dispatch(Event::Activate).unwrap());
        assert_eq!(runtime.app().wide_rail_demo.actions, 2);
        assert_eq!(runtime.app().wide_rail_demo.selected, Destination::Inbox);
        assert_eq!(node(&runtime, PAGE).text_value(), Some(INBOX.name));
        assert_eq!(node(&runtime, INBOX.id).is_selected(), Some(true));
        assert_eq!(node(&runtime, INBOX.id).is_current_page(), Some(true));
        runtime.render(320., 3000., &mut Metrics).unwrap();
        arrow(&mut runtime, EditKey::Right);
        assert_eq!(runtime.focused(), Some(HOME.id));
        arrow(&mut runtime, EditKey::Left);
        assert_eq!(runtime.focused(), Some(INBOX.id));

        runtime.render(600., 3000., &mut Metrics).unwrap();
        assert_eq!(runtime.focused(), Some(INBOX.id));
        assert_eq!(bounds(&runtime, NAVIGATION).width, rail.width);
        assert!(runtime.dom().by_id(BOTTOM).is_none());
        assert_eq!(node(&runtime, INBOX.id).is_current_page(), Some(true));
        assert!(runtime.activate(TOGGLE).unwrap());
        assert_eq!(runtime.focused(), Some(TOGGLE));
        runtime.render(599., 3000., &mut Metrics).unwrap();
        assert_eq!(runtime.focused(), Some(INBOX.id));
        assert_eq!(runtime.app().wide_rail_demo.actions, 2);
    }
}
