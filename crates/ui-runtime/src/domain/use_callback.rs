//! `use_callback`: a memoised callback.

use surface::Callback;

use super::use_memo::{use_memo, MemoDepHash};

/// `useCallback` — stable-identity callback that rebinds iff deps
/// change. Sugar over `use_memo`.
pub fn use_callback<P: Clone + 'static, F: Fn(P) + 'static>(
    f: F,
    deps: Vec<MemoDepHash>,
) -> Callback<P> {
    use_memo(move || Callback::new(f), deps)
}
