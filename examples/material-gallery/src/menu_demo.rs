use surface::{Align, Length, Node, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, View};
use surface_material::{
    shape::LayoutDirection, ButtonKind, ChoiceState, CommandMenuEntry, CommandMenuItem,
    CommandMenuMeasurement, CommandMenuOptions, Theme,
};

use crate::{
    rail_icons::{self, Destination},
    scroll_field_demo::control_row,
    Message,
};

pub(crate) const SECTION: usize = 2;
const SHOW: &str = "menu-demo-show";
const BACK: &str = "menu-demo-back";
const OPENER: &str = "menu-demo-opener";
const MENU: &str = "menu-demo-panel";
const BOTTOM_BODY: &str = "menu-demo-preview-scroll";
const FOOTER_CLEARANCE: f32 = 8.;

#[derive(Clone)]
pub enum MenuDemoEvent {
    Show,
    Back,
    ToggleDirection,
    ToggleLargeText,
    ToggleBottomAnchor,
    ToggleOpen,
    Dismiss,
    Choose(&'static str),
    OpenList,
}

#[derive(Default)]
pub(crate) struct MenuDemo {
    measurement: CommandMenuMeasurement,
    visible: bool,
    open: bool,
    rtl: bool,
    large_text: bool,
    bottom_anchor: bool,
    last: Option<&'static str>,
    outside_actions: usize,
}

impl MenuDemo {
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn leave(&mut self) {
        self.visible = false;
        self.open = false;
        self.measurement = CommandMenuMeasurement::default();
    }

    pub(crate) fn text_scale(&self) -> f32 {
        if self.large_text {
            2.
        } else {
            1.
        }
    }

    pub(crate) fn update(&mut self, event: MenuDemoEvent) {
        use MenuDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            ToggleDirection => self.rtl = !self.rtl,
            ToggleLargeText => self.large_text = !self.large_text,
            ToggleBottomAnchor => self.bottom_anchor = !self.bottom_anchor,
            ToggleOpen => self.open = !self.open,
            Dismiss => self.open = false,
            Choose(label) => {
                self.last = Some(label);
                self.open = false;
            }
            OpenList => self.outside_actions += 1,
        }
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if !self.visible {
            return AnimationUpdate::default();
        }
        let mut update = self.measurement.advance(MENU, OPENER, frame);
        if self.open && frame.layout.is_some() && self.measurement.anchor().is_none() {
            self.open = false;
            update.rebuild = true;
        }
        update
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "命令選單",
                ButtonKind::Outlined,
                false,
                Message::MenuDemo(MenuDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use MenuDemoEvent::*;
        let direction = if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        };
        let trigger = theme
            .button_with(
                OPENER,
                "檔案動作",
                ButtonKind::Tonal,
                false,
                Message::MenuDemo(ToggleOpen),
            )
            .accessible_activation();
        let item = |id, label, leading_icon, trailing_icon, disabled| {
            CommandMenuEntry::Item(CommandMenuItem {
                id,
                label,
                leading_icon,
                trailing_icon,
                disabled,
                on_press: Message::MenuDemo(Choose(label)),
            })
        };
        let entries = [
            item(
                "menu-demo-inbox",
                "移到收件匣",
                Some(Destination::Inbox.icon().clone()),
                None,
                false,
            ),
            item(
                "menu-demo-archive",
                "封存",
                Some(Destination::Archive.icon().clone()),
                None,
                true,
            ),
            item(
                "menu-demo-find",
                "在清單中尋找",
                None,
                Some(rail_icons::search().clone()),
                false,
            ),
            CommandMenuEntry::Separator {
                id: "menu-demo-divider",
            },
            item(
                "menu-demo-rename",
                "重新命名目前選取的檔案",
                None,
                None,
                false,
            ),
            item("menu-demo-copy", "建立副本", None, None, false),
            item("menu-demo-export", "匯出檔案", None, None, false),
            item("menu-demo-details", "顯示詳細資料", None, None, false),
        ];
        let menu = theme.command_menu(
            CommandMenuOptions {
                id: MENU,
                label: "檔案動作",
                trigger_id: OPENER,
                open: self.open,
                direction,
                preferred_width: 240.,
                restore_focus_to: Some(OPENER),
            },
            trigger,
            entries,
            Message::MenuDemo(Dismiss),
            &self.measurement,
        );
        let toggle = |id, label, selected, event| {
            theme
                .segmented_button(
                    id,
                    label,
                    ChoiceState {
                        selected,
                        disabled: false,
                    },
                    Message::MenuDemo(event),
                )
                .accessible_activation()
        };
        let controls = [
            theme.heading("menu-demo-title", "命令選單"),
            control_row(vec![
                theme
                    .tonal_button(BACK, "返回", Message::MenuDemo(Back))
                    .restore_focus_on_removal(SHOW)
                    .accessible_activation(),
                theme
                    .switch(
                        "menu-demo-theme",
                        "深色",
                        ChoiceState {
                            selected: dark,
                            disabled: false,
                        },
                        Message::Theme,
                    )
                    .accessible_activation(),
                toggle("menu-demo-rtl", "右起", self.rtl, ToggleDirection),
                toggle(
                    "menu-demo-large-text",
                    "大字",
                    self.large_text,
                    ToggleLargeText,
                ),
                toggle(
                    "menu-demo-bottom",
                    "底部入口",
                    self.bottom_anchor,
                    ToggleBottomAnchor,
                ),
            ]),
        ];
        let list = theme
            .button_with(
                "menu-demo-list",
                "查看清單",
                ButtonKind::Text,
                false,
                Message::MenuDemo(OpenList),
            )
            .accessible_activation();
        let status = self
            .last
            .map(|label| theme.text("menu-demo-last", format!("已執行「{label}」")));
        if !self.bottom_anchor {
            return theme.page(controls.into_iter().chain([menu, list]).chain(status));
        }

        // Only the preview body scrolls. A separate footer lets the button
        // keep its natural height and keeps the popup out of the scroll clip.
        let body = View::new(
            Node::new(Role::Group, "命令選單設定")
                .id(BOTTOM_BODY)
                .styled(
                    Style::column()
                        .height(Length::Px(0.))
                        .grow(1.)
                        .gap(24.)
                        .clip(true),
                ),
        )
        .accessibility_scroll_pages()
        .children(controls.into_iter().chain([list]).chain(status));
        let alignment = match direction {
            LayoutDirection::LeftToRight => Align::Start,
            LayoutDirection::RightToLeft => Align::End,
        };
        let footer = View::new(Node::new(Role::Group, "").styled(Style::column().align(alignment)))
            .children([menu]);
        let preview = View::new(
            Node::new(Role::Group, "").styled(
                Style::column()
                    .height(Length::Px(0.))
                    .grow(1.)
                    .gap(FOOTER_CLEARANCE),
            ),
        )
        .children([body, footer]);
        theme.page([preview])
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(&self) -> serde_json::Value {
        serde_json::json!({
            "visible": self.visible,
            "open": self.open,
            "rtl": self.rtl,
            "text_scale": self.text_scale(),
            "bottom_anchor": self.bottom_anchor,
            "last": self.last,
            "outside_actions": self.outside_actions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BACK, MENU, OPENER, SECTION, SHOW};
    use crate::{Gallery, Message};
    use surface::{Rect, TextMeasurer};
    use surface_app::{EditKey, Event, Runtime};

    const BODY: &str = "menu-demo-preview-scroll";
    const BOTTOM: &str = "menu-demo-bottom";
    const STATUS: &str = "menu-demo-last";
    const OUTSIDE: &str = "menu-demo-list";
    const FIRST: &str = "menu-demo-inbox";
    const LAST: &str = "menu-demo-details";

    struct Fonts;

    impl TextMeasurer for Fonts {
        fn measure(&mut self, text: &str, size: f32, width: Option<f32>) -> (f32, f32) {
            let natural = text.chars().count() as f32 * size;
            let width = width.unwrap_or(natural).min(natural);
            (width, (natural / width.max(1.)).ceil().max(1.) * size * 1.5)
        }
    }

    fn draw(runtime: &mut Runtime<Gallery>, (width, height): (f32, f32)) {
        for _ in 0..3 {
            runtime.render(width, height, &mut Fonts).unwrap();
        }
    }

    fn route(viewport: (f32, f32)) -> Runtime<Gallery> {
        let mut runtime = Runtime::new(Gallery::default()).unwrap();
        draw(&mut runtime, viewport);
        runtime.apply(Message::Section(SECTION)).unwrap();
        runtime.activate(SHOW).unwrap();
        draw(&mut runtime, viewport);
        runtime
    }

    fn rect(runtime: &Runtime<Gallery>, id: &str) -> Rect {
        let id = runtime
            .dom()
            .by_id(id)
            .unwrap_or_else(|| panic!("missing {id}"));
        runtime.computed().unwrap().nodes[id.index()].bounds
    }

    fn height_edge(rect: Rect) -> f32 {
        rect.y + rect.height
    }

    #[test]
    fn bottom_menu_scrolls_status_clear_of_its_stationary_trigger() {
        for viewport in [(320., 480.), (320., 240.)] {
            let mut runtime = route(viewport);
            for setting in [
                "menu-demo-theme",
                "menu-demo-rtl",
                "menu-demo-large-text",
                BOTTOM,
            ] {
                runtime.activate(setting).unwrap();
            }
            draw(&mut runtime, viewport);
            let trigger = rect(&runtime, OPENER);
            assert_eq!(height_edge(trigger), viewport.1 - 28.);
            assert_eq!(trigger.x + trigger.width, viewport.0 - 28.);
            assert!(trigger.height >= 48.);

            runtime.activate(OPENER).unwrap();
            draw(&mut runtime, viewport);
            assert_eq!(runtime.focused(), Some(FIRST));
            assert_eq!(height_edge(rect(&runtime, MENU)) + 4., trigger.y);
            runtime
                .dispatch(Event::Edit {
                    key: EditKey::End,
                    extend: false,
                    by_word: false,
                })
                .unwrap();
            draw(&mut runtime, viewport);
            assert_eq!(runtime.focused(), Some(LAST));
            runtime.dispatch(Event::Activate).unwrap();
            draw(&mut runtime, viewport);
            assert_eq!(runtime.focused(), Some(OPENER));
            assert!(runtime.dom().by_id(MENU).is_none());
            let original = rect(&runtime, STATUS);

            let body = runtime.dom().by_id(BODY).map(|_| rect(&runtime, BODY));
            let scroll_region = body.unwrap_or(Rect::new(28., 28., 264., trigger.y - 28.));
            runtime
                .dispatch(Event::Scroll {
                    x: scroll_region.x + scroll_region.width / 2.,
                    y: scroll_region.y + scroll_region.height / 2.,
                    delta_x: 0.,
                    delta_y: 800.,
                })
                .unwrap();
            draw(&mut runtime, viewport);
            let revealed = rect(&runtime, STATUS);
            assert!(
                height_edge(revealed) + 8. <= trigger.y,
                "{viewport:?} status overlaps trigger: {revealed:?} vs {trigger:?}"
            );
            let body = body.expect("bottom preview needs its own scroll region");
            assert_eq!(height_edge(body) + 8., trigger.y);
            assert_eq!(body.intersect(revealed), revealed);
            assert_eq!(rect(&runtime, OPENER), trigger);
            if height_edge(original) > height_edge(body) {
                assert!(revealed.y < original.y);
            }
            let x = trigger.x + trigger.width / 2.;
            let y = trigger.y + trigger.height / 2.;
            runtime.dispatch(Event::PointerDown { x, y }).unwrap();
            runtime.dispatch(Event::PointerUp { x, y }).unwrap();
            draw(&mut runtime, viewport);
            assert_eq!(runtime.focused(), Some(FIRST));
            assert_eq!(height_edge(rect(&runtime, MENU)) + 4., trigger.y);
        }
    }

    #[test]
    fn ordinary_top_trigger_keeps_its_tab_order_and_handback() {
        let viewport = (960., 800.);
        let mut runtime = route(viewport);
        assert_eq!(runtime.focused(), Some(BACK));
        assert!(runtime.dom().by_id(BODY).is_none());
        for target in [
            "menu-demo-theme",
            "menu-demo-rtl",
            "menu-demo-large-text",
            BOTTOM,
            OPENER,
        ] {
            runtime
                .dispatch(Event::FocusNext { reverse: false })
                .unwrap();
            assert_eq!(runtime.focused(), Some(target));
        }
        let trigger = rect(&runtime, OPENER);
        for (reverse, expected) in [(false, OUTSIDE), (true, BOTTOM)] {
            runtime.activate(OPENER).unwrap();
            draw(&mut runtime, viewport);
            assert_eq!(runtime.focused(), Some(FIRST));
            assert_eq!(rect(&runtime, MENU).y, height_edge(trigger) + 4.);
            runtime.dispatch(Event::FocusNext { reverse }).unwrap();
            draw(&mut runtime, viewport);
            assert_eq!(runtime.focused(), Some(expected));
            assert!(runtime.dom().by_id(MENU).is_none());
        }
    }
}
