//! Exactly the same state and view are used on desktop and the web.
use surface::{Length, Node, Role, Style};
use surface_app::{Application, View};
use surface_material::Theme;

#[derive(Default)]
pub struct Showcase {
    pub count: u32,
    pub dark: bool,
    pub name: String,
    pub scroll_name: String,
}
#[derive(Clone)]
pub enum Message {
    Increment,
    Reset,
    ToggleTheme,
    Name(String),
    ScrollName(String),
    Noop,
}
impl Application for Showcase {
    type Message = Message;
    fn title(&self) -> &str {
        "Surface · Material showcase"
    }
    fn update(&mut self, message: Message) {
        match message {
            Message::Increment => self.count = self.count.saturating_add(1),
            Message::Reset => self.count = 0,
            Message::ToggleTheme => self.dark = !self.dark,
            Message::Name(name) => self.name = name,
            Message::ScrollName(name) => self.scroll_name = name,
            Message::Noop => {}
        }
    }
    fn view(&self) -> View<Message> {
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        theme.page([
            theme.heading("title", "Surface"),
            theme.text("subtitle", "One Rust app. Desktop GPU and WebGPU."),
            theme.card(
                "counter-card",
                [
                    theme.heading("counter", format!("{}", self.count)),
                    theme.text(
                        "counter-label",
                        "State lives in Rust. The view is a function of that state.",
                    ),
                    theme.row([
                        theme.button("increment", "Add one", Message::Increment),
                        theme.tonal_button("reset", "Reset", Message::Reset),
                        theme.disabled_button("disabled", "Disabled", Message::Increment),
                    ]),
                ],
            ),
            theme.card(
                "profile-card",
                [
                    theme.text("name-label", "Your name"),
                    theme.text_field("name", "Your name", &self.name, Message::Name),
                    theme.text(
                        "greeting",
                        if self.name.is_empty() {
                            "Hello, Rust.".to_owned()
                        } else {
                            format!("Hello, {}.", self.name)
                        },
                    ),
                    theme.checkbox("dark-mode", "Dark theme", self.dark, Message::ToggleTheme),
                ],
            ),
            View::new(
                Node::new(Role::Group, "Scroll fixture")
                    .id("scroll-fixture")
                    .styled(Style::column().height(Length::Px(160.)).clip(true)),
            )
            .children([
                theme.tonal_button("scroll-first", "Scroll first", Message::Noop),
                View::new(
                    Node::new(Role::Group, "Scroll spacer")
                        .styled(Style::default().height(Length::Px(320.))),
                ),
                theme.text_field(
                    "scroll-name",
                    "Scroll name",
                    &self.scroll_name,
                    Message::ScrollName,
                ),
            ]),
            theme.text(
                "footer",
                "Use Tab to move focus. Press Enter or Space to activate a control.",
            ),
        ])
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    use surface_app::Runtime;
    use wasm_bindgen::prelude::*;
    thread_local! {static APP:RefCell<Option<Rc<RefCell<Runtime<Showcase>>>>>=const{RefCell::new(None)};}
    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        console_error_panic_hook::set_once();
        let runtime = Rc::new(RefCell::new(
            Runtime::new(Showcase::default()).map_err(|e| JsValue::from_str(&e.to_string()))?,
        ));
        APP.with(|slot| *slot.borrow_mut() = Some(runtime.clone()));
        surface_platform::run_runtime(runtime, Default::default())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    /// Example diagnostics for browser smoke tests; the framework needs no JS UI.
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
        APP.with(|app| {
            let app = app.borrow();
            let runtime = app.as_ref().expect("app started").borrow();
            let state = runtime.app();
            format!(
                "count={};dark={};name={};nodes={};focused={}",
                state.count,
                state.dark,
                state.name,
                runtime.dom().len(),
                runtime.focused().unwrap_or("")
            )
        })
    }
    #[wasm_bindgen]
    pub fn control_bounds(id: &str) -> Vec<f32> {
        APP.with(|app| {
            let app = app.borrow();
            let runtime = app.as_ref().expect("app started").borrow();
            let Some(node) = runtime.dom().by_id(id) else {
                return vec![];
            };
            let Some(computed) = runtime.computed() else {
                return vec![];
            };
            let r = computed.nodes[node.index()].bounds;
            vec![r.x, r.y, r.width, r.height]
        })
    }
    #[wasm_bindgen]
    pub fn editing_state() -> Vec<String> {
        APP.with(|app| {
            let app = app.borrow();
            let runtime = app.as_ref().expect("app started").borrow();
            runtime
                .editor("name")
                .map(|e| {
                    vec![
                        e.value().to_owned(),
                        e.display_value(),
                        e.selection().anchor.to_string(),
                        e.selection().cursor.to_string(),
                        e.is_composing().to_string(),
                    ]
                })
                .unwrap_or_default()
        })
    }
    #[wasm_bindgen]
    pub fn input_caret() -> Vec<f32> {
        APP.with(|app| {
            let app = app.borrow();
            let runtime = app.as_ref().expect("app started").borrow();
            runtime
                .input_snapshot()
                .and_then(|s| s.caret)
                .map(|r| vec![r.x, r.y, r.width, r.height])
                .unwrap_or_default()
        })
    }
}
