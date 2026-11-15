//! Map the platform-neutral cursor choice to a winit icon (issue #121).
use surface_app::CursorStyle;
use winit::window::CursorIcon;

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
        for (style, icon) in ALL.into_iter().zip(expected) {
            assert_eq!(icon(style), icon, "{style:?}");
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
