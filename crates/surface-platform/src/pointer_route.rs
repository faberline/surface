//! Host decisions for mouse buttons (issue #120). Pure, so they run without
//! a window, like `key_route`. The host maps the winit button to `Button`,
//! asks `PointerRoute::route_mouse`, and only applies the step.
use surface_app::{Event, Modifiers};
use winit::event::MouseButton;

/// A mouse button, as Surface sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Button {
    Primary,
    Secondary,
    Other,
}

/// What the host does with a mouse button event.
#[derive(Debug)]
pub(crate) enum PointerStep {
    /// A left-button press or release: send the pointer event.
    Primary(Event),
    /// A secondary press: send `Event::SecondaryDown`.
    Secondary(Event),
    /// Send nothing.
    Ignore,
}

/// Map a winit button. This and the host are the only places that name winit.
pub(crate) fn from_winit_button(button: MouseButton) -> Button {
    match button {
        MouseButton::Left => Button::Primary,
        MouseButton::Right => Button::Secondary,
        _ => Button::Other,
    }
}

#[derive(Default)]
pub(crate) struct PointerRoute {
    /// The button whose press was turned into a secondary press and whose
    /// release must not reach the runtime as `PointerUp`.
    held: Option<Button>,
}

impl PointerRoute {
    pub(crate) fn route_mouse(
        &mut self,
        button: Button,
        pressed: bool,
        mods: Modifiers,
        (x, y): (f32, f32),
        macos: bool,
    ) -> PointerStep {
        let _ = (button, pressed, mods, x, y, macos, &mut self.held);
        PointerStep::Ignore
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: (f32, f32) = (3., 4.);
    fn none() -> Modifiers {
        Modifiers::default()
    }
    fn ctrl() -> Modifiers {
        Modifiers {
            control: true,
            ..Modifiers::default()
        }
    }
    fn secondary() -> PointerStep {
        PointerStep::Secondary(Event::SecondaryDown { x: 3., y: 4. })
    }
    /// `Event` has no `PartialEq`; steps compare by their `Debug` text.
    fn same(a: PointerStep, b: PointerStep) {
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }

    #[test]
    fn right_press_is_secondary_and_the_release_is_ignored() {
        let mut r = PointerRoute::default();
        same(
            r.route_mouse(Button::Secondary, true, none(), AT, true),
            secondary(),
        );
        same(
            r.route_mouse(Button::Secondary, false, none(), AT, true),
            PointerStep::Ignore,
        );
    }

    #[test]
    fn control_left_on_macos_is_secondary_even_if_control_is_released_first() {
        let mut r = PointerRoute::default();
        same(
            r.route_mouse(Button::Primary, true, ctrl(), AT, true),
            secondary(),
        );
        // Control is up when the button is released: still no PointerUp.
        same(
            r.route_mouse(Button::Primary, false, none(), AT, true),
            PointerStep::Ignore,
        );
        // The state is clear: the next plain click is primary again.
        same(
            r.route_mouse(Button::Primary, true, none(), AT, true),
            PointerStep::Primary(Event::PointerDown { x: 3., y: 4. }),
        );
    }

    #[test]
    fn control_left_off_macos_is_a_plain_primary_press() {
        let mut r = PointerRoute::default();
        same(
            r.route_mouse(Button::Primary, true, ctrl(), AT, false),
            PointerStep::Primary(Event::PointerDown { x: 3., y: 4. }),
        );
        same(
            r.route_mouse(Button::Primary, false, ctrl(), AT, false),
            PointerStep::Primary(Event::PointerUp { x: 3., y: 4. }),
        );
    }

    #[test]
    fn plain_left_is_primary_down_and_up() {
        let mut r = PointerRoute::default();
        same(
            r.route_mouse(Button::Primary, true, none(), AT, true),
            PointerStep::Primary(Event::PointerDown { x: 3., y: 4. }),
        );
        same(
            r.route_mouse(Button::Primary, false, none(), AT, true),
            PointerStep::Primary(Event::PointerUp { x: 3., y: 4. }),
        );
    }

    #[test]
    fn other_buttons_are_ignored() {
        let mut r = PointerRoute::default();
        for pressed in [true, false] {
            same(
                r.route_mouse(Button::Other, pressed, none(), AT, true),
                PointerStep::Ignore,
            );
        }
    }

    #[test]
    fn a_second_right_press_while_one_is_held_keeps_the_state_sound() {
        let mut r = PointerRoute::default();
        same(
            r.route_mouse(Button::Secondary, true, none(), AT, true),
            secondary(),
        );
        same(
            r.route_mouse(Button::Secondary, true, none(), AT, true),
            secondary(),
        );
        same(
            r.route_mouse(Button::Secondary, false, none(), AT, true),
            PointerStep::Ignore,
        );
        same(
            r.route_mouse(Button::Primary, true, none(), AT, true),
            PointerStep::Primary(Event::PointerDown { x: 3., y: 4. }),
        );
    }

    #[test]
    fn winit_buttons_map_to_surface_buttons() {
        assert_eq!(from_winit_button(MouseButton::Left), Button::Primary);
        assert_eq!(from_winit_button(MouseButton::Right), Button::Secondary);
        assert_eq!(from_winit_button(MouseButton::Middle), Button::Other);
    }
}
