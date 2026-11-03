//! A summary of one fiber's hook slots.

use crate::domain::fiber_id::FiberId;
use crate::domain::hook_slot::HookSlot;
use crate::domain::runtime::RUNTIME;

/// One hook slot's value rendered for debug. `value_json` is `None`
/// when the runtime can't cheaply read it (non-primitive `State`,
/// `Memo` / `Ref` body, or `Context` placeholder).
pub struct DebugHookSummary {
    pub kind: &'static str,
    pub type_name: Option<&'static str>,
    pub value_json: Option<serde_json::Value>,
}

/// Read every hook slot for `fiber_id` and render a debug summary.
/// Returns an empty Vec if the fiber doesn't exist (rather than
/// panicking — `jet browser hooks <bogus-id>` should be a gentle
/// error, not a crash).
pub fn debug_snapshot_hooks(fiber_id: FiberId) -> Vec<DebugHookSummary> {
    RUNTIME.with(|r| {
        let rt = r.borrow();
        let Some(fiber) = rt.fibers.iter().find(|f| f.id == fiber_id) else {
            return Vec::new();
        };
        fiber
            .hooks
            .iter()
            .map(|slot| match slot {
                HookSlot::State(any) => {
                    let (type_name, value_json) = summarize_any(any.as_ref());
                    DebugHookSummary {
                        kind: "State",
                        type_name,
                        value_json,
                    }
                }
                HookSlot::Memo { value, .. } => {
                    let (type_name, value_json) = summarize_any(value.as_ref());
                    DebugHookSummary {
                        kind: "Memo",
                        type_name,
                        value_json,
                    }
                }
                HookSlot::Ref(_) => DebugHookSummary {
                    kind: "Ref",
                    type_name: None,
                    value_json: None,
                },
                HookSlot::Context => DebugHookSummary {
                    kind: "Context",
                    type_name: None,
                    value_json: None,
                },
                HookSlot::EffectOnce { ran } => DebugHookSummary {
                    kind: "EffectOnce",
                    type_name: Some("bool"),
                    value_json: Some(serde_json::json!(*ran)),
                },
            })
            .collect()
    })
}

/// Try a small chain of primitive downcasts. Returns (type_name,
/// value_json) — `type_name` is `None` only when Any's concrete type
/// can't even be named (shouldn't happen for boxed Anys).
fn summarize_any(v: &dyn std::any::Any) -> (Option<&'static str>, Option<serde_json::Value>) {
    use serde_json::json;
    if let Some(x) = v.downcast_ref::<i64>() {
        return (Some("i64"), Some(json!(*x)));
    }
    if let Some(x) = v.downcast_ref::<i32>() {
        return (Some("i32"), Some(json!(*x)));
    }
    if let Some(x) = v.downcast_ref::<u64>() {
        return (Some("u64"), Some(json!(*x)));
    }
    if let Some(x) = v.downcast_ref::<u32>() {
        return (Some("u32"), Some(json!(*x)));
    }
    if let Some(x) = v.downcast_ref::<f64>() {
        return (Some("f64"), Some(json!(*x)));
    }
    if let Some(x) = v.downcast_ref::<f32>() {
        return (Some("f32"), Some(json!(*x)));
    }
    if let Some(x) = v.downcast_ref::<bool>() {
        return (Some("bool"), Some(json!(*x)));
    }
    if let Some(x) = v.downcast_ref::<String>() {
        return (Some("String"), Some(json!(x.clone())));
    }
    if let Some(x) = v.downcast_ref::<&'static str>() {
        return (Some("&'static str"), Some(json!(*x)));
    }
    if v.downcast_ref::<()>().is_some() {
        return (Some("()"), Some(json!(null)));
    }
    // Unknown concrete type — return just the type_id form so the UI
    // can at least tell the user something. std::any::type_name::<T>()
    // requires knowing T at compile time, which we don't here; best
    // we can do without type reflection is "unknown".
    (Some("<unknown primitive chain missed>"), None)
}
