//! One positional hook entry in a fiber.

use std::cell::RefCell;
use std::rc::Rc;

use super::use_memo::MemoDepHash;

/// A single slot in the hook list. Type-erased box so different
/// state types coexist in one Vec. The transpiler emits a cast on
/// each access because it knows the type from the surrounding TSX.
///
/// Variant = hook kind. A rules-of-hooks violation (conditional
/// hook call that re-orders slots across renders) produces a
/// variant mismatch at access time and panics with a clear message.
pub(crate) enum HookSlot {
    /// `use_state`: the cell value.
    State(Box<dyn std::any::Any>),
    /// `use_memo` / `use_callback`: memoised value + last-seen deps.
    Memo {
        value: Box<dyn std::any::Any>,
        deps: Vec<MemoDepHash>,
    },
    /// `use_ref`: an `Rc<RefCell<T>>` behind a shared handle so
    /// clones in closures see mutations.
    Ref(Rc<RefCell<Box<dyn std::any::Any>>>),
    /// `use_context`: records which context the hook is watching so
    /// the runtime (future) can re-render on provider change. For
    /// now this is a no-op placeholder — the value is read from the
    /// active provider stack at render time, not stored.
    #[allow(dead_code)]
    Context,
    /// `use_effect_once`: a narrow effect slot for empty-deps effects.
    /// The effect is scheduled exactly once for the owning fiber.
    EffectOnce { ran: bool },
}
