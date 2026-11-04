//! Opt-in transports. Application state stays on the window event loop.
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use native::Bridge;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub(super) use web::Bridge;

pub(super) enum Reply {
    #[cfg(not(target_arch = "wasm32"))]
    Native(std::sync::mpsc::SyncSender<jet_bridge_protocol::Response>),
    #[cfg(target_arch = "wasm32")]
    Web(js_sys::Function),
}
impl Reply {
    pub(super) fn send(self, response: jet_bridge_protocol::Response) {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Native(sender) => {
                let _ = sender.send(response);
            }
            #[cfg(target_arch = "wasm32")]
            Self::Web(resolve) => {
                if let Ok(json) = serde_json::to_string(&response) {
                    if let Ok(value) = js_sys::JSON::parse(&json) {
                        let _ = resolve.call1(&wasm_bindgen::JsValue::NULL, &value);
                    }
                }
            }
        }
    }
}
