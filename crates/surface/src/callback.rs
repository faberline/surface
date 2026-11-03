//! Event callbacks, typed by their payload.

use std::rc::Rc;

/// Event callback typed by payload.
#[derive(Clone)]
pub struct Callback<P: Clone>(Rc<dyn Fn(P)>);

impl<P: Clone> std::fmt::Debug for Callback<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<callback>")
    }
}

impl<P: Clone> Callback<P> {
    pub fn new<F: Fn(P) + 'static>(f: F) -> Self {
        Self(Rc::new(f))
    }

    pub fn call(&self, payload: P) {
        (self.0)(payload);
    }
}
