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
        if pressed {
            let secondary =
                button == Button::Secondary || (button == Button::Primary && macos && mods.control);
            if secondary {
                // Keep the first held button: its release is the one to drop.
                self.held.get_or_insert(button);
                return PointerStep::Secondary(Event::SecondaryDown { x, y });
            }
            return match button {
                Button::Primary => PointerStep::Primary(Event::PointerDown { x, y }),
                _ => PointerStep::Ignore,
            };
        }
        match button {
            Button::Secondary => PointerStep::Ignore,
            Button::Primary if self.held == Some(Button::Primary) => {
                self.held = None;
                PointerStep::Ignore
            }
            Button::Primary => PointerStep::Primary(Event::PointerUp { x, y }),
            Button::Other => PointerStep::Ignore,
        }
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

    fn up_down(
        r: &mut PointerRoute,
        button: Button,
        mods: Modifiers,
        pressed: bool,
    ) -> PointerStep {
        r.route_mouse(button, pressed, mods, AT, true)
    }

    #[test]
    fn right_click_then_control_left_click_swallows_the_left_release() {
        let mut r = PointerRoute::default();
        same(
            up_down(&mut r, Button::Secondary, none(), true),
            secondary(),
        );
        same(
            up_down(&mut r, Button::Secondary, none(), false),
            PointerStep::Ignore,
        );
        same(up_down(&mut r, Button::Primary, ctrl(), true), secondary());
        same(
            up_down(&mut r, Button::Primary, ctrl(), false),
            PointerStep::Ignore,
        );
    }

    #[test]
    fn control_left_then_right_with_either_release_order_sends_no_pointer_up() {
        for left_first in [true, false] {
            let mut r = PointerRoute::default();
            same(up_down(&mut r, Button::Primary, ctrl(), true), secondary());
            same(
                up_down(&mut r, Button::Secondary, none(), true),
                secondary(),
            );
            let order = if left_first {
                [Button::Primary, Button::Secondary]
            } else {
                [Button::Secondary, Button::Primary]
            };
            for button in order {
                same(up_down(&mut r, button, none(), false), PointerStep::Ignore);
            }
        }
    }

    #[test]
    fn right_then_control_left_with_either_release_order_sends_no_pointer_up() {
        for left_first in [true, false] {
            let mut r = PointerRoute::default();
            same(
                up_down(&mut r, Button::Secondary, none(), true),
                secondary(),
            );
            same(up_down(&mut r, Button::Primary, ctrl(), true), secondary());
            let order = if left_first {
                [Button::Primary, Button::Secondary]
            } else {
                [Button::Secondary, Button::Primary]
            };
            for button in order {
                same(up_down(&mut r, button, none(), false), PointerStep::Ignore);
            }
            // The next plain click is primary again.
            same(
                up_down(&mut r, Button::Primary, none(), true),
                PointerStep::Primary(Event::PointerDown { x: 3., y: 4. }),
            );
        }
    }

    #[test]
    fn winit_buttons_map_to_surface_buttons() {
        assert_eq!(from_winit_button(MouseButton::Left), Button::Primary);
        assert_eq!(from_winit_button(MouseButton::Right), Button::Secondary);
        assert_eq!(from_winit_button(MouseButton::Middle), Button::Other);
    }
}
