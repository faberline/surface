//! The mouse cursor an app asks for (issue #121). Plain data: no platform type.
use crate::{Application, Runtime};

/// The cursor shape over a node. The host maps it to its own cursor type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorStyle {
    #[default]
    Default,
    Text,
    Pointer,
    ResizeHorizontal,
    ResizeVertical,
    NotAllowed,
    Grab,
}

impl<A: Application> Runtime<A> {
    /// The cursor at a point in logical window pixels. Reads state only.
    ///
    /// Candidates are enabled nodes in the active modal scope that have an
    /// explicit cursor (`View::cursor`) or are focusable. The last painted
    /// candidate that contains the point decides. A focusable node with no
    /// explicit cursor gives `Default`: there is no policy by role. No
    /// candidate, or no layout yet, gives `Default`.
    pub fn cursor_at(&self, x: f32, y: f32) -> CursorStyle {
        let Some(computed) = self.computed() else {
            return CursorStyle::Default;
        };
        computed
            .paint_order
            .iter()
            .rev()
            .copied()
            .find_map(|id| {
                let node = self.dom.get(id).ok()?;
                let explicit = self.bindings.get(&id).and_then(|b| b.cursor);
                let n = &computed.nodes[id.index()];
                let candidate = !node.is_disabled()
                    && (explicit.is_some() || node.is_focusable())
                    && self.in_scope(id)
                    && n.bounds.contains(x, y)
                    && n.clip.contains(x, y);
                candidate.then(|| explicit.unwrap_or_default())
            })
            .unwrap_or_default()
    }
}
