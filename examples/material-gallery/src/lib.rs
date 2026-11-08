use surface::{Color, Node, RangeValue, Role, Style};
use surface_app::{Application, View};
use surface_material::{
    ButtonKind, CheckValue, ChipKind, ChoiceState, FabColor, FabSize, IconButtonKind, LoadingKind,
    SegmentPosition, Theme,
};

pub struct Gallery {
    pub section: usize,
    pub dark: bool,
    pub radio: usize,
    pub enabled: bool,
    pub check: CheckValue,
    pub segment: usize,
    pub volume: f64,
    pub clicks: usize,
    pub filter: bool,
    pub input_chip: bool,
    pub dialog: bool,
    pub menu: bool,
    pub name: String,
    pub draft: String,
}
impl Default for Gallery {
    fn default() -> Self {
        Self {
            section: 0,
            dark: false,
            radio: 0,
            enabled: false,
            check: CheckValue::Mixed,
            segment: 0,
            volume: 40.,
            clicks: 0,
            filter: false,
            input_chip: true,
            dialog: false,
            menu: false,
            name: "Surface".into(),
            draft: String::new(),
        }
    }
}
#[derive(Clone)]
pub enum Message {
    Section(usize),
    Theme,
    Radio(usize),
    Switch,
    Check,
    Segment(usize),
    Volume(f64),
    Click,
    Filter,
    RemoveChip,
    Dialog(bool),
    Menu(bool),
    Draft(String),
    Save,
}
impl Application for Gallery {
    type Message = Message;
    fn title(&self) -> &str {
        "Surface · Material gallery"
    }
    fn update(&mut self, m: Message) {
        match m {
            Message::Section(i) => self.section = i,
            Message::Theme => self.dark = !self.dark,
            Message::Radio(i) => self.radio = i,
            Message::Switch => self.enabled = !self.enabled,
            Message::Check => {
                self.check = if self.check == CheckValue::Checked {
                    CheckValue::Unchecked
                } else {
                    CheckValue::Checked
                }
            }
            Message::Segment(i) => self.segment = i,
            Message::Volume(v) => self.volume = v,
            Message::Click => self.clicks += 1,
            Message::Filter => self.filter = !self.filter,
            Message::RemoveChip => self.input_chip = false,
            Message::Dialog(v) => {
                self.dialog = v;
                self.menu = false;
                if v {
                    self.draft = self.name.clone();
                }
            }
            Message::Menu(v) => self.menu = v,
            Message::Draft(v) => self.draft = v,
            Message::Save => {
                self.name = self.draft.clone();
                self.dialog = false;
            }
        }
    }
    fn view(&self) -> View<Message> {
        let t = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let choice = |selected| ChoiceState {
            selected,
            disabled: false,
        };
        let mut children = vec![
            t.heading("gallery-title", "Material gallery"),
            t.row([
                t.text("gallery-summary", "Rust state · Desktop GPU · WebGPU"),
                t.switch("theme", "深色模式", choice(self.dark), Message::Theme),
            ]),
            t.tabs(
                "sections",
                "元件分類",
                ["選取", "動作", "浮層", "色彩"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, name)| {
                        t.tab(
                            &format!("section-{i}"),
                            name,
                            choice(self.section == i),
                            Message::Section(i),
                        )
                    }),
            ),
        ];
        match self.section {
            0 => {
                children.push(
                    t.card(
                        "selection-card",
                        [
                            t.text("selection-heading", "選取元件"),
                            t.row([
                                t.checkbox_with(
                                    "check",
                                    "核取方塊",
                                    self.check,
                                    false,
                                    Message::Check,
                                ),
                                t.switch(
                                    "enabled",
                                    "啟用通知",
                                    choice(self.enabled),
                                    Message::Switch,
                                ),
                            ]),
                            t.radio_group(
                                "radios",
                                "配送方式",
                                ["標準", "快捷", "停用"].into_iter().enumerate().map(
                                    |(i, name)| {
                                        t.radio(
                                            &format!("radio-{i}"),
                                            name,
                                            ChoiceState {
                                                selected: self.radio == i,
                                                disabled: i == 2,
                                            },
                                            Message::Radio(i),
                                        )
                                    },
                                ),
                            ),
                            t.segmented_buttons(
                                "segments",
                                "版面",
                                ["清單", "網格", "停用"].into_iter().enumerate().map(
                                    |(i, name)| {
                                        t.segmented_button(
                                            &format!("segment-{i}"),
                                            name,
                                            ChoiceState {
                                                selected: self.segment == i,
                                                disabled: i == 2,
                                            },
                                            Message::Segment(i),
                                        )
                                    },
                                ),
                            ),
                            t.text("volume-label", format!("音量 {}", self.volume)),
                            t.slider(
                                "volume",
                                "音量",
                                RangeValue {
                                    min: 0.,
                                    max: 100.,
                                    step: 5.,
                                    value: self.volume,
                                },
                                false,
                                Message::Volume,
                            ),
                        ],
                    ),
                );
            }
            1 => {
                children.push(t.card(
                    "action-card",
                    [
                        t.text("click-count", format!("已執行 {} 次", self.clicks)),
                        t.row([
                            t.button_with(
                                "filled",
                                "Filled",
                                ButtonKind::Filled,
                                false,
                                Message::Click,
                            ),
                            t.button_with(
                                "tonal",
                                "Tonal",
                                ButtonKind::Tonal,
                                false,
                                Message::Click,
                            ),
                            t.button_with(
                                "outlined",
                                "Outlined",
                                ButtonKind::Outlined,
                                false,
                                Message::Click,
                            ),
                            t.button_with(
                                "text-button",
                                "Text",
                                ButtonKind::Text,
                                false,
                                Message::Click,
                            ),
                            t.button_with(
                                "disabled-action",
                                "Disabled",
                                ButtonKind::Filled,
                                true,
                                Message::Click,
                            ),
                        ]),
                        t.row([
                            t.chip(
                                "assist",
                                "協助",
                                ChipKind::Assist,
                                choice(false),
                                Message::Click,
                                None,
                            ),
                            t.chip(
                                "suggestion",
                                "建議",
                                ChipKind::Suggestion,
                                choice(false),
                                Message::Click,
                                None,
                            ),
                            t.chip(
                                "filter",
                                "已選取",
                                ChipKind::Filter,
                                choice(self.filter),
                                Message::Filter,
                                None,
                            ),
                        ]),
                        t.segmented_buttons(
                            "segments-multi",
                            "樣式",
                            [
                                t.segment(
                                    "seg-a",
                                    "日",
                                    SegmentPosition::First,
                                    choice(true),
                                    Message::Click,
                                ),
                                t.segment(
                                    "seg-b",
                                    "週",
                                    SegmentPosition::Middle,
                                    choice(false),
                                    Message::Click,
                                ),
                                t.segment(
                                    "seg-c",
                                    "月",
                                    SegmentPosition::Last,
                                    choice(true),
                                    Message::Click,
                                ),
                            ],
                        ),
                        t.row([
                            t.fab(
                                "fab-small",
                                "Add small",
                                "+",
                                FabSize::Small,
                                FabColor::Primary,
                                false,
                                Message::Click,
                            ),
                            t.fab(
                                "fab",
                                "Add",
                                "+",
                                FabSize::Regular,
                                FabColor::Primary,
                                false,
                                Message::Click,
                            ),
                            t.fab(
                                "fab-large",
                                "Add large",
                                "+",
                                FabSize::Large,
                                FabColor::Tertiary,
                                false,
                                Message::Click,
                            ),
                            t.fab(
                                "fab-surface",
                                "Add surface",
                                "+",
                                FabSize::Regular,
                                FabColor::Surface,
                                false,
                                Message::Click,
                            ),
                            t.fab(
                                "fab-secondary",
                                "Add secondary",
                                "+",
                                FabSize::Regular,
                                FabColor::Secondary,
                                false,
                                Message::Click,
                            ),
                            t.fab(
                                "fab-disabled",
                                "Add disabled",
                                "+",
                                FabSize::Regular,
                                FabColor::Primary,
                                true,
                                Message::Click,
                            ),
                        ]),
                        t.row([
                            t.linear_progress(
                                "lp-determinate",
                                "Determinate progress",
                                Some(0.4),
                                0.,
                            ),
                            t.linear_progress("lp-indeterminate", "Loading", None, 0.5),
                        ]),
                        t.row([
                            t.icon_button(
                                "icon-standard",
                                "Favorite",
                                "★",
                                IconButtonKind::Standard,
                                None,
                                false,
                                Message::Click,
                            ),
                            t.icon_button(
                                "icon-filled",
                                "Favorite filled",
                                "★",
                                IconButtonKind::Filled,
                                None,
                                false,
                                Message::Click,
                            ),
                            t.icon_button(
                                "icon-tonal",
                                "Favorite tonal",
                                "★",
                                IconButtonKind::Tonal,
                                None,
                                false,
                                Message::Click,
                            ),
                            t.icon_button(
                                "icon-outlined",
                                "Favorite outlined",
                                "★",
                                IconButtonKind::Outlined,
                                None,
                                false,
                                Message::Click,
                            ),
                            t.icon_button(
                                "icon-toggle",
                                "Favorite toggle",
                                "★",
                                IconButtonKind::Filled,
                                Some(false),
                                false,
                                Message::Click,
                            ),
                            t.icon_button(
                                "icon-disabled",
                                "Favorite disabled",
                                "★",
                                IconButtonKind::Filled,
                                None,
                                true,
                                Message::Click,
                            ),
                        ]),
                        t.row([
                            t.loading_indicator(
                                "loading-default",
                                "Loading",
                                LoadingKind::Default,
                                0.,
                            ),
                            t.loading_indicator(
                                "loading-contained",
                                "Loading contained",
                                LoadingKind::Contained,
                                0.,
                            ),
                        ]),
                    ],
                ));
                if self.input_chip {
                    children.push(t.chip(
                        "input-chip",
                        "計畫 A",
                        ChipKind::Input,
                        choice(false),
                        Message::Click,
                        Some(Message::RemoveChip),
                    ));
                }
                children.push(t.outlined_card(
                    "outlined-card",
                    [t.text("outlined-description", "Outlined card · 共用的色彩與形狀")],
                ));
            }
            2 => {
                children.push(t.card(
                    "overlay-card",
                    [
                        t.text("saved-name", format!("名稱：{}", self.name)),
                        t.row([
                            t.button_with(
                                "open-dialog",
                                "重新命名",
                                ButtonKind::Filled,
                                false,
                                Message::Dialog(true),
                            ),
                            t.button_with(
                                "open-menu",
                                "開啟選單",
                                ButtonKind::Outlined,
                                false,
                                Message::Menu(true),
                            ),
                        ]),
                        t.text("overlay-help", "使用 Tab 切換焦點。按 Escape 關閉。"),
                    ],
                ));
            }
            _ => {
                let swatch = |id: &str, text: &str, background: Color, foreground: Color| {
                    View::new(
                        Node::new(Role::Group, text).id(id).styled(
                            Style::default()
                                .padding(24.)
                                .radius(16.)
                                .background(background),
                        ),
                    )
                    .children([View::new(
                        Node::new(Role::Text, text)
                            .text(text)
                            .styled(Style::default().foreground(foreground)),
                    )])
                };
                children.extend([
                    swatch("primary-swatch", "Primary", t.primary, t.on_primary),
                    swatch(
                        "secondary-swatch",
                        "Secondary container",
                        t.colors.secondary_container,
                        t.colors.on_secondary_container,
                    ),
                    swatch(
                        "tertiary-swatch",
                        "Tertiary container",
                        t.colors.tertiary_container,
                        t.colors.on_tertiary_container,
                    ),
                    swatch("error-swatch", "Error", t.colors.error, t.colors.on_error),
                ]);
            }
        }
        if self.dialog {
            children.push(t.dialog(
                "rename-dialog",
                "重新命名",
                [
                    t.text("dialog-help", "輸入新名稱。取消會保留原名稱。"),
                    t.text_field("dialog-name", "名稱", &self.draft, Message::Draft),
                ],
                [
                    t.button_with(
                        "dialog-cancel",
                        "取消",
                        ButtonKind::Text,
                        false,
                        Message::Dialog(false),
                    ),
                    t.button_with(
                        "dialog-save",
                        "儲存",
                        ButtonKind::Filled,
                        false,
                        Message::Save,
                    ),
                ],
                Message::Dialog(false),
            ));
        }
        if self.menu {
            children.push(t.menu_at(
                "options-menu",
                "選單",
                (80., 240.),
                [
                    t.menu_item("menu-rename", "重新命名", false, Message::Dialog(true)),
                    t.menu_item("menu-disabled", "停用項目", true, Message::Click),
                    t.menu_item("menu-close", "關閉選單", false, Message::Menu(false)),
                ],
                Message::Menu(false),
            ));
        }
        t.page(children)
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    use surface_app::Runtime;
    use wasm_bindgen::prelude::*;
    thread_local! {static APP:RefCell<Option<Rc<RefCell<Runtime<Gallery>>>>>=const{RefCell::new(None)};}
    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        console_error_panic_hook::set_once();
        let runtime = Rc::new(RefCell::new(
            Runtime::new(Gallery::default()).map_err(|e| JsValue::from_str(&e.to_string()))?,
        ));
        APP.with(|a| *a.borrow_mut() = Some(runtime.clone()));
        surface_platform::run_runtime(runtime, Default::default())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    #[wasm_bindgen]
    pub fn export_dom() -> Result<String, JsValue> {
        APP.with(|app| {
            app.borrow()
                .as_ref()
                .expect("app started")
                .borrow()
                .export_dom()
                .map_err(|error| JsValue::from_str(&error.to_string()))
        })
    }
    #[wasm_bindgen]
    pub fn inspect() -> String {
        APP.with(|a|{
        let a=a.borrow();let runtime=a.as_ref().expect("started").borrow();let s=runtime.app();
        serde_json::json!({"section":s.section,"dark":s.dark,"radio":s.radio,"enabled":s.enabled,"check":format!("{:?}",s.check),"segment":s.segment,
            "volume":s.volume,"clicks":s.clicks,"filter":s.filter,"input_chip":s.input_chip,"dialog":s.dialog,"menu":s.menu,"name":s.name,"draft":s.draft,"focused":runtime.focused()}).to_string()
    })
    }
    #[wasm_bindgen]
    pub fn control_bounds(id: &str) -> Vec<f32> {
        APP.with(|a| {
            let a = a.borrow();
            let r = a.as_ref().expect("started").borrow();
            let Some(id) = r.dom().by_id(id) else {
                return vec![];
            };
            let Some(c) = r.computed() else { return vec![] };
            let b = c.nodes[id.index()].bounds;
            vec![b.x, b.y, b.width, b.height]
        })
    }
}
