use crate::{HostError, UserEvent};
use jet_bridge_protocol::{Request, Response, VERSION};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use winit::event_loop::EventLoopProxy;

pub(crate) struct Bridge {
    object: js_sys::Object,
    _request: Closure<dyn FnMut(JsValue) -> js_sys::Promise>,
}
impl Bridge {
    pub(crate) fn install(
        proxy: EventLoopProxy<UserEvent>,
        _application: &str,
    ) -> Result<Option<Self>, HostError> {
        let window = web_sys::window().ok_or_else(|| HostError("missing browser window".into()))?;
        let key = JsValue::from_str("__jet_bridge");
        let previous = js_sys::Reflect::get(&window, &key).map_err(js_error)?;
        if !previous.is_undefined() {
            return Err(HostError("window.__jet_bridge is already in use".into()));
        }
        let callback = Closure::wrap(Box::new(move |value: JsValue| {
            let request = js_sys::JSON::stringify(&value)
                .ok()
                .and_then(|json| json.as_string())
                .and_then(|json| serde_json::from_str::<Request>(&json).ok());
            js_sys::Promise::new(&mut |resolve, _reject| {
                let reply = super::Reply::Web(resolve.clone());
                match request.clone() {
                    Some(request) => {
                        if let Err(error) = proxy.send_event(UserEvent::Bridge { request, reply }) {
                            if let UserEvent::Bridge { request, reply } = error.0 {
                                reply.send(Response::error(
                                    request.id,
                                    "host_closed",
                                    "the application event loop closed",
                                ));
                            }
                        }
                    }
                    None => reply.send(Response::error(
                        0,
                        "invalid_request",
                        "invalid application bridge request",
                    )),
                }
            })
        }) as Box<dyn FnMut(JsValue) -> js_sys::Promise>);
        let object = js_sys::Object::new();
        js_sys::Reflect::set(&object, &"version".into(), &VERSION.into()).map_err(js_error)?;
        js_sys::Reflect::set(
            &object,
            &"request".into(),
            callback.as_ref().unchecked_ref(),
        )
        .map_err(js_error)?;
        js_sys::Object::freeze(&object);
        js_sys::Reflect::set(&window, &key, &object).map_err(js_error)?;
        Ok(Some(Self {
            object,
            _request: callback,
        }))
    }
}
fn js_error(error: JsValue) -> HostError {
    HostError(format!("WASM application bridge: {error:?}"))
}
impl Drop for Bridge {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            let key = JsValue::from_str("__jet_bridge");
            if js_sys::Reflect::get(&window, &key)
                .ok()
                .is_some_and(|value| value == JsValue::from(self.object.clone()))
            {
                let _ = js_sys::Reflect::delete_property(
                    window.unchecked_ref::<js_sys::Object>(),
                    &key,
                );
            }
        }
    }
}
