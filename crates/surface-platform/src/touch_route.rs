//! Maps one physical touch contact to Surface's logical pointer events.
use surface_app::Event;
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceId, ElementState, Touch, TouchPhase};

pub(crate) enum TouchStep {
    Pointer(Event),
    Cancel,
    Ignore,
}

#[derive(Default)]
pub(crate) struct TouchRoute {
    contact: Option<(DeviceId, u64)>,
    mouse_held: bool,
}

impl TouchRoute {
    pub(crate) fn active(&self) -> bool {
        self.contact.is_some()
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Keep a mouse press from taking over a touch and an unmatched release
    /// from ending a later contact.
    pub(crate) fn mouse_primary(&mut self, state: ElementState) -> bool {
        match state {
            ElementState::Pressed if !self.active() => {
                self.mouse_held = true;
                true
            }
            ElementState::Pressed => false,
            ElementState::Released => std::mem::take(&mut self.mouse_held),
        }
    }

    pub(crate) fn route(&mut self, touch: Touch, scale: f64) -> TouchStep {
        let id = (touch.device_id, touch.id);
        match touch.phase {
            TouchPhase::Started if !self.active() && !self.mouse_held => {
                let Some((x, y)) = logical_point(touch.location, scale) else {
                    return TouchStep::Ignore;
                };
                self.contact = Some(id);
                TouchStep::Pointer(Event::PointerDown { x, y })
            }
            TouchPhase::Moved if self.contact == Some(id) => {
                let Some((x, y)) = logical_point(touch.location, scale) else {
                    return TouchStep::Ignore;
                };
                TouchStep::Pointer(Event::PointerMove { x, y })
            }
            TouchPhase::Ended if self.contact == Some(id) => {
                self.contact = None;
                let Some((x, y)) = logical_point(touch.location, scale) else {
                    return TouchStep::Cancel;
                };
                TouchStep::Pointer(Event::PointerUp { x, y })
            }
            TouchPhase::Cancelled if self.contact == Some(id) => {
                self.contact = None;
                TouchStep::Cancel
            }
            _ => TouchStep::Ignore,
        }
    }
}

fn logical_point(location: PhysicalPosition<f64>, scale: f64) -> Option<(f32, f32)> {
    if !scale.is_finite() || scale <= 0. {
        return None;
    }
    let x = (location.x / scale) as f32;
    let y = (location.y / scale) as f32;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use TouchPhase::{Cancelled, Ended, Moved, Started};

    fn contact(id: u64, phase: TouchPhase, x: f64, y: f64) -> Touch {
        Touch {
            device_id: DeviceId::dummy(),
            phase,
            location: PhysicalPosition::new(x, y),
            force: None,
            id,
        }
    }

    #[test]
    fn tracks_the_same_contact_in_logical_pixels_even_outside_the_window() {
        let mut route = TouchRoute::default();
        assert!(matches!(
            route.route(contact(1, Started, 220., 82.), 2.),
            TouchStep::Pointer(Event::PointerDown { x: 110., y: 41. })
        ));
        assert!(matches!(
            route.route(contact(1, Moved, 225., 90.), 2.),
            TouchStep::Pointer(Event::PointerMove { x: 112.5, y: 45. })
        ));
        assert!(matches!(
            route.route(contact(1, Ended, -3., 104.), 2.),
            TouchStep::Pointer(Event::PointerUp { x: -1.5, y: 52. })
        ));
        assert!(!route.active());
    }

    #[test]
    fn ignores_secondary_contacts_and_never_promotes_one_still_held() {
        let mut route = TouchRoute::default();
        route.route(contact(1, Started, 5., 10.), 1.);
        for phase in [Started, Moved, Ended, Cancelled] {
            assert!(matches!(
                route.route(contact(2, phase, 20., 30.), 1.),
                TouchStep::Ignore
            ));
        }
        assert!(route.active());
        assert!(matches!(
            route.route(contact(1, Cancelled, 5., 10.), 1.),
            TouchStep::Cancel
        ));
        assert!(matches!(
            route.route(contact(2, Moved, 20., 30.), 1.),
            TouchStep::Ignore
        ));
        assert!(!route.active());
        assert!(matches!(
            route.route(contact(3, Started, 20., 30.), 1.),
            TouchStep::Pointer(Event::PointerDown { x: 20., y: 30. })
        ));
    }

    #[test]
    fn touch_and_mouse_cannot_take_over_each_others_press() {
        let mut route = TouchRoute::default();
        assert!(route.mouse_primary(ElementState::Pressed));
        assert!(matches!(
            route.route(contact(1, Started, 1., 1.), 1.),
            TouchStep::Ignore
        ));
        assert!(route.mouse_primary(ElementState::Released));
        route.route(contact(2, Started, 1., 1.), 1.);
        assert!(!route.mouse_primary(ElementState::Pressed));
        assert!(matches!(
            route.route(contact(2, Ended, 2., 2.), 1.),
            TouchStep::Pointer(Event::PointerUp { x: 2., y: 2. })
        ));
        assert!(!route.mouse_primary(ElementState::Released));
    }

    #[test]
    fn invalid_terminal_coordinates_cancel_while_blur_releases_ownership() {
        let mut route = TouchRoute::default();
        assert!(matches!(
            route.route(contact(1, Started, 1., 1.), 0.),
            TouchStep::Ignore
        ));
        route.route(contact(1, Started, 1., 1.), 1.);
        assert!(matches!(
            route.route(contact(1, Ended, f64::NAN, 2.), 1.),
            TouchStep::Cancel
        ));
        route.route(contact(2, Started, 1., 1.), 1.);
        route.reset();
        assert!(matches!(
            route.route(contact(2, Ended, 2., 2.), 1.),
            TouchStep::Ignore
        ));
        assert!(route.mouse_primary(ElementState::Pressed));
    }
}
