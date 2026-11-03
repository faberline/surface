//! `use_effect_once`: an effect that runs once per fiber.

use super::hook_slot::HookSlot;
use super::runtime::RUNTIME;

/// Narrow `useEffect(..., [])` primitive for generated WASM code.
///
/// This intentionally handles only the empty-deps shape. The
/// transpiler uses it for side effects that start browser host
/// capabilities such as `fetch`; dependency-aware reruns will use a
/// richer effect slot later.
pub fn use_effect_once<F: FnOnce() + 'static>(effect: F) {
    let should_run = RUNTIME.with(|r| {
        let mut rt = r.borrow_mut();
        let id = rt
            .current
            .expect("hook called outside a render — rules-of-hooks violation");
        let fiber = rt.fiber_mut(id);
        let idx = fiber.hook_cursor;
        fiber.hook_cursor += 1;
        if fiber.hooks.len() <= idx {
            fiber.hooks.push(HookSlot::EffectOnce { ran: false });
        }
        let HookSlot::EffectOnce { ran } = &mut fiber.hooks[idx] else {
            panic!(
                "hook slot {idx} type mismatch: expected EffectOnce, got another kind — \
                 rules-of-hooks violation OR transpiler bug"
            );
        };
        if *ran {
            false
        } else {
            *ran = true;
            true
        }
    });

    if should_run {
        effect();
    }
}
