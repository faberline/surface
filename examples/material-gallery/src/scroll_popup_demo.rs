use surface::{Align, Length, Node, Rect, Role, Style};
use surface_app::{AnimationFrame, AnimationUpdate, FocusNavigation, Key, View};
use surface_material::{
    scroll_field_pair_layout, scroll_field_popup_layout, shape::LayoutDirection, typography,
    ButtonKind, ChoiceState, ScrollChoiceEvent, ScrollChoiceState, ScrollFieldEvent,
    ScrollFieldMode, ScrollFieldPairOptions, ScrollFieldPopupLayout, ScrollFieldPopupOptions,
    ScrollFieldState, Theme,
};

use crate::{
    scroll_field_demo::{control_row, PADDING},
    scroll_pair_demo::{measurement_options, measurement_states},
    Message,
};

pub(crate) const SECTION: usize = 0;
const SHOW: &str = "scroll-popup-demo-show";
const BACK: &str = "scroll-popup-demo-back";
const OPENER: &str = "scroll-popup-demo-opener";
const DIALOG: &str = "scroll-popup-demo-dialog";
const BODY: &str = "scroll-popup-demo-body";
const CLOSE: &str = "scroll-popup-demo-close";
const PAIR: &str = "scroll-popup-demo-pair";
const QUANTITY: &str = "scroll-popup-demo-quantity";
const UNIT: &str = "scroll-popup-demo-unit";
const QUANTITY_WHEEL: &str = "scroll-popup-demo-quantity-adjustable";
const QUANTITY_EDITOR: &str = "scroll-popup-demo-quantity-editor";

#[derive(Clone)]
pub enum ScrollPopupDemoEvent {
    Show,
    Back,
    Toggle,
    Close,
    Direction,
    LargeText,
    Bottom,
    HideOpener,
    Outside,
    Quantity(ScrollFieldEvent),
    Unit(ScrollChoiceEvent),
}

pub(crate) struct ScrollPopupDemo {
    visible: bool,
    viewport: Rect,
    anchor: Option<Rect>,
    open: bool,
    rtl: bool,
    large_text: bool,
    near_bottom: bool,
    hide_opener: bool,
    quantity: ScrollFieldState,
    unit: ScrollChoiceState,
    quantity_commits: usize,
    unit_commits: usize,
    outside_presses: usize,
    reduced_motion: bool,
}

impl Default for ScrollPopupDemo {
    fn default() -> Self {
        let (quantity, unit) = measurement_states();
        Self {
            visible: false,
            viewport: Rect::new(0., 0., 960., 820.),
            anchor: None,
            open: false,
            rtl: false,
            large_text: false,
            near_bottom: false,
            hide_opener: false,
            quantity,
            unit,
            quantity_commits: 0,
            unit_commits: 0,
            outside_presses: 0,
            reduced_motion: false,
        }
    }
}

impl ScrollPopupDemo {
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

    fn direction(&self) -> LayoutDirection {
        if self.rtl {
            LayoutDirection::RightToLeft
        } else {
            LayoutDirection::LeftToRight
        }
    }

    fn options(&self) -> Option<ScrollFieldPopupOptions<'_>> {
        Some(ScrollFieldPopupOptions {
            id: DIALOG,
            label: "編輯容量",
            body_id: BODY,
            close_id: CLOSE,
            close_label: "關閉",
            restore_focus_to: OPENER,
            anchor: self.anchor?,
            viewport: self.viewport,
            direction: self.direction(),
            preferred_size: (696., 440.),
        })
    }

    fn layout(&self) -> Option<ScrollFieldPopupLayout> {
        scroll_field_popup_layout(self.options()?)
    }

    fn pair_options(&self, layout: ScrollFieldPopupLayout) -> ScrollFieldPairOptions<'_> {
        measurement_options(
            [PAIR, QUANTITY, UNIT],
            (layout.body_width, layout.body_height),
            self.text_scale(),
            self.direction(),
            (false, false),
        )
    }

    fn update_fit(&mut self, layout: ScrollFieldPopupLayout) {
        let pair = scroll_field_pair_layout(self.pair_options(layout), &self.quantity, &self.unit);
        self.quantity.invalidate_layout();
        self.unit.invalidate_layout();
        self.quantity.update_fit(pair.quantity_fits);
        self.unit.update_fit(pair.unit_fits);
    }

    fn reconcile_layout(&mut self) {
        if self.open {
            match self.layout() {
                Some(layout) => self.update_fit(layout),
                None => self.close(),
            }
        }
    }

    fn close(&mut self) {
        self.open = false;
        self.quantity.update_fit(true);
        self.unit.update_fit(true);
        self.quantity.apply(ScrollFieldEvent::Cancel, true);
        self.unit.apply(ScrollChoiceEvent::Cancel, true);
        self.quantity.invalidate_layout();
        self.unit.invalidate_layout();
    }

    pub(crate) fn leave(&mut self) {
        if self.open {
            self.close();
        }
        self.visible = false;
        self.anchor = None;
    }

    pub(crate) fn viewport_changed(&mut self, width: f32, height: f32) -> bool {
        let viewport = Rect::new(0., 0., width, height);
        if self.viewport == viewport {
            return false;
        }
        // Preserve right/bottom anchoring for the transition frame. The next
        // layout replaces these coordinates with the measured visible opener.
        if let Some(anchor) = &mut self.anchor {
            if self.rtl {
                anchor.x += width - self.viewport.width;
            }
            if self.near_bottom {
                anchor.y += height - self.viewport.height;
            }
        }
        self.viewport = viewport;
        self.reconcile_layout();
        self.visible
    }

    pub(crate) fn observe(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if !self.visible {
            return AnimationUpdate::default();
        }
        let Some(layout) = frame.layout else {
            return AnimationUpdate {
                rebuild: false,
                request_next: true,
            };
        };
        let anchor = frame
            .dom
            .by_id(OPENER)
            .and_then(|id| layout.nodes.get(id.index()))
            .map(|node| node.bounds.intersect(node.clip).intersect(self.viewport))
            .filter(|bounds| bounds.width > 0. && bounds.height > 0.);
        if self.anchor == anchor {
            return AnimationUpdate::default();
        }
        self.anchor = anchor;
        self.reconcile_layout();
        AnimationUpdate {
            rebuild: true,
            request_next: false,
        }
    }

    pub(crate) fn frame(&mut self, frame: &AnimationFrame<'_>) -> AnimationUpdate {
        if !self.open {
            return AnimationUpdate::default();
        }
        self.reduced_motion = frame.reduced_motion;
        let quantity = self.quantity.advance(frame.now_ms, frame.reduced_motion);
        let unit = self.unit.advance(frame.now_ms, frame.reduced_motion);
        AnimationUpdate {
            rebuild: quantity.rebuild || unit.rebuild,
            request_next: quantity.request_next || unit.request_next,
        }
    }

    pub(crate) fn update(&mut self, event: ScrollPopupDemoEvent) {
        use ScrollPopupDemoEvent::*;
        match event {
            Show => self.visible = true,
            Back => self.leave(),
            Close => self.close(),
            Toggle if self.open => self.close(),
            Toggle => {
                if let Some(layout) = self.layout() {
                    self.open = true;
                    self.update_fit(layout);
                }
            }
            Direction => {
                self.rtl = !self.rtl;
                self.reconcile_layout();
            }
            LargeText => {
                self.large_text = !self.large_text;
                self.reconcile_layout();
            }
            Bottom => self.near_bottom = !self.near_bottom,
            HideOpener => {
                self.hide_opener = !self.hide_opener;
                if self.hide_opener {
                    self.anchor = None;
                    self.close();
                }
            }
            Outside => self.outside_presses += 1,
            Quantity(event) if self.open => {
                self.quantity_commits +=
                    usize::from(self.quantity.apply(event, self.reduced_motion).is_some());
            }
            Unit(event) if self.open => {
                self.unit_commits +=
                    usize::from(self.unit.apply(event, self.reduced_motion).is_some());
            }
            Quantity(_) | Unit(_) => {}
        }
    }

    pub(crate) fn show_button(theme: &Theme) -> View<Message> {
        theme
            .button_with(
                SHOW,
                "浮層捲動欄位",
                ButtonKind::Outlined,
                false,
                Message::ScrollPopup(ScrollPopupDemoEvent::Show),
            )
            .restore_focus_on_removal(BACK)
            .accessible_activation()
    }

    fn opener(&self, theme: &Theme, layout: Option<ScrollFieldPopupLayout>) -> View<Message> {
        use ScrollPopupDemoEvent::*;
        let target = layout.map(|layout| {
            let plan =
                scroll_field_pair_layout(self.pair_options(layout), &self.quantity, &self.unit);
            if plan.quantity_fits && self.quantity.mode() == ScrollFieldMode::Scroll {
                QUANTITY_WHEEL
            } else {
                QUANTITY_EDITOR
            }
        });
        let closed = !self.open;
        theme
            .button_with(
                OPENER,
                "開啟欄位",
                ButtonKind::Tonal,
                layout.is_none(),
                Message::ScrollPopup(Toggle),
            )
            .disclosure_expanded(self.open)
            .accessible_activation()
            .protect_composition_on_press()
            .on_key_focus_navigation(move |press| {
                if closed
                    && press.modifiers.is_empty()
                    && !press.repeat
                    && matches!(press.key, Key::Enter | Key::Space)
                {
                    target.map(|id| FocusNavigation {
                        target: id.to_owned(),
                        message: Message::ScrollPopup(Toggle),
                    })
                } else {
                    None
                }
            })
    }

    pub(crate) fn view(&self, theme: &Theme, dark: bool) -> View<Message> {
        use ScrollPopupDemoEvent::*;
        let choice = |selected| ChoiceState {
            selected,
            disabled: false,
        };
        let toggle = |id, label, on, event| {
            theme
                .segmented_button(id, label, choice(on), Message::ScrollPopup(event))
                .accessible_activation()
        };
        let profile = control_row(vec![
            theme
                .tonal_button(BACK, "返回", Message::ScrollPopup(Back))
                .restore_focus_on_removal(SHOW)
                .accessible_activation(),
            theme
                .switch(
                    "scroll-popup-demo-theme",
                    "深色",
                    choice(dark),
                    Message::Theme,
                )
                .accessible_activation(),
            toggle("scroll-popup-demo-rtl", "右起", self.rtl, Direction),
            toggle(
                "scroll-popup-demo-large-text",
                "大字",
                self.large_text,
                LargeText,
            ),
        ]);
        let settings = control_row(vec![
            toggle(
                "scroll-popup-demo-bottom",
                "靠近底部",
                self.near_bottom,
                Bottom,
            ),
            toggle(
                "scroll-popup-demo-hide-opener",
                "隱藏開啟鍵",
                self.hide_opener,
                HideOpener,
            ),
        ]);
        let saved = View::new(
            Node::new(Role::Text, "")
                .id("scroll-popup-demo-saved")
                .text(format!(
                    "已選 {} {}",
                    self.quantity.value(),
                    self.unit.choice().label
                ))
                .styled(
                    Style::column()
                        .with_text_style(typography::BODY_MEDIUM)
                        .foreground(theme.colors.on_surface_variant),
                ),
        );
        let layout = self.layout();
        let popup =
            self.options()
                .zip(layout)
                .filter(|_| self.open)
                .and_then(|(options, layout)| {
                    let pair = theme.scroll_field_pair(
                        self.pair_options(layout),
                        &self.quantity,
                        &self.unit,
                        |event| Message::ScrollPopup(Quantity(event)),
                        |event| Message::ScrollPopup(Unit(event)),
                    );
                    theme.scroll_field_popup(options, pair, Message::ScrollPopup(Close))
                });
        let outside = theme
            .button_with(
                "scroll-popup-demo-outside",
                &format!("外部 {} 次", self.outside_presses),
                ButtonKind::Outlined,
                false,
                Message::ScrollPopup(Outside),
            )
            .accessible_activation();
        let mut row = Node::new(Role::Group, "").styled(
            Style::row()
                .width(Length::Percent(1.))
                .align(Align::Center)
                .wrap(true)
                .gap(8.),
        );
        if self.rtl {
            row = row.reverse_row();
        }
        let actions = View::new(row).children(
            (!self.hide_opener)
                .then(|| self.opener(theme, layout))
                .into_iter()
                .chain(popup)
                .chain([outside]),
        );
        let space = View::new(
            Node::new(Role::Group, "").styled(Style::column().height(Length::Px(0.)).grow(1.)),
        )
        .without_interaction(None);
        let children = if self.near_bottom {
            [profile, settings, saved, space, actions]
        } else {
            [profile, settings, saved, actions, space]
        };
        View::new(
            Node::new(Role::Window, "浮層捲動欄位")
                .id("scroll-popup-demo")
                .styled(
                    Style::column()
                        .height(Length::Percent(1.))
                        .padding(PADDING)
                        .gap(8.)
                        .background(theme.colors.surface),
                ),
        )
        .children(children)
        .restore_focus_in_subtree_on_removal(SHOW)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn inspect(
        &self,
        runtime: &surface_app::Runtime<crate::Gallery>,
    ) -> serde_json::Value {
        let layout = self.layout().filter(|_| self.open);
        let rect = |r: Rect| [r.x, r.y, r.width, r.height];
        let mode = |mode| {
            if mode == ScrollFieldMode::Input {
                "input"
            } else {
                "scroll"
            }
        };
        serde_json::json!({
            "visible": self.visible, "open": self.open, "rtl": self.rtl, "text_scale": self.text_scale(),
            "near_bottom": self.near_bottom, "hide_opener": self.hide_opener,
            "above": layout.map(|p| p.above), "bounds": layout.map(|p| rect(p.bounds)),
            "anchor": self.anchor.map(rect), "outside_presses": self.outside_presses,
            "body_scroll_top": runtime.scroll_metrics(BODY).map(|m| m.offset.1),
            "quantity": {"value": self.quantity.value(), "raw": self.quantity.raw(),
                "mode": mode(self.quantity.mode()), "error": self.quantity.error().map(|e| format!("{e:?}")), "commits": self.quantity_commits},
            "unit": {"id": self.unit.choice().id, "ordinal": self.unit.index()+1,
                "raw": self.unit.raw(), "mode": mode(self.unit.mode()),
                "error": self.unit.error().map(|e| format!("{e:?}")), "commits": self.unit_commits},
        })
    }
}
