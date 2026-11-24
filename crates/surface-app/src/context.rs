//! Secondary press and the context menu pattern (issue #120). The host
//! sends `Event::SecondaryDown`; the runtime walks from the hit node to the
//! nearest ancestor with an `on_context_menu` handler and posts its message.
//! The app opens the menu itself (for example `Theme::menu_at` at the point).
use crate::{controls::ModalDismissInput, AppError, Application, Runtime};
use std::rc::Rc;

/// A point in logical pixels, in window coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerPoint {
    pub x: f32,
    pub y: f32,
}

/// A context menu handler. It gets the point and returns the message.
pub(crate) type ContextHandler<M> = Rc<dyn Fn(PointerPoint) -> M>;

impl<A: Application> Runtime<A> {
    /// Returns `Ok(true)` when a message was posted or an open modal was
    /// dismissed. It never sets `pressed`, `hovered`, `focused`, the text
    /// cursor or a drag, so a secondary press cannot activate a control.
    pub(crate) fn secondary_down(&mut self, x: f32, y: f32) -> Result<bool, AppError> {
        let mut node = self.hit(x, y);
        self.cancel_composition();
        if self.outside_scope(x, y, node) {
            return self.dismiss_scope(ModalDismissInput::Outside);
        }
        while let Some(id) = node.filter(|id| self.in_scope(*id)) {
            if let Some(handler) = self.bindings.get(&id).and_then(|b| b.context.clone()) {
                return self.message(Some(handler(PointerPoint { x, y })));
            }
            node = self.dom.get(id).ok().and_then(|n| n.parent());
        }
        Ok(false)
    }
}
