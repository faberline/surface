//! Map the platform-neutral cursor choice to a winit icon (issue #121).
//! macOS and other native hosts only. The web host sets no cursor (decision
//! 0001, D-B).
use super::Host;
use surface_app::{Application, CursorStyle};
use winit::window::CursorIcon;

/// The winit icon for a cursor style. Pure, so it is tested without a window.
pub(crate) fn icon(style: CursorStyle) -> CursorIcon {
    match style {
        CursorStyle::Default => CursorIcon::Default,
        CursorStyle::Text => CursorIcon::Text,
        CursorStyle::Pointer => CursorIcon::Pointer,
        CursorStyle::ResizeHorizontal => CursorIcon::EwResize,
        CursorStyle::ResizeVertical => CursorIcon::NsResize,
        CursorStyle::NotAllowed => CursorIcon::NotAllowed,
        CursorStyle::Grab => CursorIcon::Grab,
    }
}

impl<A: Application> Host<A> {
    /// Show the cursor the runtime picks at the last pointer position. It
    /// calls the window only when the style changed since the last call.
    pub(crate) fn sync_cursor(&mut self) {
        let style = self
            .runtime
            .borrow()
            .cursor_at(self.pointer.0, self.pointer.1);
        self.apply_cursor(style);
    }
    /// The pointer left the window: the next move or enter sets it again.
    pub(crate) fn apply_cursor(&mut self, style: CursorStyle) {
        if self.cursor == style {
            return;
        }
        if let Some(window) = &self.window {
            window.set_cursor(icon(style));
            self.cursor = style;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [CursorStyle; 7] = [
        CursorStyle::Default,
        CursorStyle::Text,
        CursorStyle::Pointer,
        CursorStyle::ResizeHorizontal,
        CursorStyle::ResizeVertical,
        CursorStyle::NotAllowed,
        CursorStyle::Grab,
    ];

    #[test]
    fn each_style_maps_to_its_icon() {
        let expected = [
            CursorIcon::Default,
            CursorIcon::Text,
            CursorIcon::Pointer,
            CursorIcon::EwResize,
            CursorIcon::NsResize,
            CursorIcon::NotAllowed,
            CursorIcon::Grab,
        ];
        for (style, want) in ALL.into_iter().zip(expected) {
            assert_eq!(icon(style), want, "{style:?}");
        }
    }

    #[test]
    fn no_two_styles_share_an_icon() {
        for (i, a) in ALL.iter().enumerate() {
            for b in &ALL[i + 1..] {
                assert_ne!(icon(*a), icon(*b), "{a:?} and {b:?}");
            }
        }
    }
}
