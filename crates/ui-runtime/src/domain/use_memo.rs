//! `use_memo`: a value recomputed only when its dependency hashes change.

use super::hook_slot::HookSlot;
use super::runtime::RUNTIME;

/// Deps array element for `use_memo` / `use_callback`. We hash deps
/// into `u64` at the call site so the slot doesn't need to carry
/// arbitrary types. The transpiler emits `hash_dep(x)` for each
/// dep the TSX source passes.
pub type MemoDepHash = u64;

/// `useMemo` — recompute `compute()` only when `deps` change.
pub fn use_memo<T: Clone + 'static, F: FnOnce() -> T>(compute: F, deps: Vec<MemoDepHash>) -> T {
    RUNTIME.with(|r| {
        let mut rt = r.borrow_mut();
        let id = rt
            .current
            .expect("hook called outside a render — rules-of-hooks violation");
        let fiber = rt.fiber_mut(id);
        let idx = fiber.hook_cursor;
        fiber.hook_cursor += 1;
        let needs_recompute = match fiber.hooks.get(idx) {
            None => true,
            Some(HookSlot::Memo { deps: prev, .. }) => prev != &deps,
            Some(_) => panic!("hook slot {idx} type mismatch: expected Memo, got another kind"),
        };
        if needs_recompute {
            let value = compute();
            let slot = HookSlot::Memo {
                value: Box::new(value),
                deps,
            };
            if fiber.hooks.len() <= idx {
                fiber.hooks.push(slot);
            } else {
                fiber.hooks[idx] = slot;
            }
        }
        let HookSlot::Memo { value, .. } = &fiber.hooks[idx] else {
            unreachable!("just populated the slot above");
        };
        value
            .downcast_ref::<T>()
            .expect("memo slot type mismatch — transpiler bug")
            .clone()
    })
}

/// Hash any `Hash + ?Sized` value into a `MemoDepHash`. The
/// transpiler emits `hash_dep(x)` per dep expression.
pub fn hash_dep<H: std::hash::Hash + ?Sized>(v: &H) -> MemoDepHash {
    use std::hash::{DefaultHasher, Hasher};
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}
