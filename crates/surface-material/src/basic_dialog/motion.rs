use std::{cell::Cell, rc::Rc};

use surface::{Color, FillContour};
use surface_app::{AnimationFrame, AnimationUpdate, ToggleMotion, ToggleSpring};

/// Retained paint motion for one caller-controlled Basic dialog.
///
/// Rebuild the view when the caller changes `open`. Call `advance` on
/// every animation frame. The live focus scope follows `open` immediately.
/// The extended display list can retain the exiting paint until it settles.
#[derive(Default)]
pub struct BasicDialogMotion {
    spring: ToggleMotion,
    opacity: Rc<Cell<f32>>,
}

impl BasicDialogMotion {
    /// Start at either resting state. A closed track is the default.
    pub fn new(open: bool) -> Self {
        Self {
            spring: ToggleMotion::new(open),
            opacity: Rc::new(Cell::new(if open { 1. } else { 0. })),
        }
    }

    /// Current presented opacity in the range zero through one.
    pub fn opacity(&self) -> f32 {
        self.opacity.get()
    }

    /// Whether to build live content or retain it solely for outgoing paint.
    pub fn should_mount(&self, open: bool) -> bool {
        open || self.opacity() > 0.
    }

    /// Advance without rebuilding for paint-only changes. Rebuild when a
    /// retained exit ends so the caller can remove its inert content.
    pub fn advance(&mut self, frame: &AnimationFrame<'_>, open: bool) -> AnimationUpdate {
        let was_painted = self.opacity() > 0.;
        // Do not show a late exit for an opening closed before it painted.
        let snap = frame.reduced_motion || (!open && !was_painted);
        let (opacity, request_next) = self.spring.sample(
            open,
            frame.now_ms,
            snap,
            ToggleSpring::StandardDefaultEffects,
        );
        self.opacity.set(opacity);
        AnimationUpdate {
            rebuild: !open && was_painted && opacity == 0.,
            request_next,
        }
    }

    pub(super) fn panel_painter(&self) -> impl Fn() -> f32 + 'static {
        let opacity = self.opacity.clone();
        move || opacity.get()
    }

    pub(super) fn scrim_painter(
        &self,
        (width, height): (f32, f32),
        Color(red, green, blue, alpha): Color,
    ) -> impl Fn() -> FillContour + 'static {
        let opacity = self.opacity.clone();
        move || {
            let alpha = (opacity.get() * f32::from(alpha)).round() as u8;
            FillContour::new(
                [[0., 0.], [width, 0.], [width, height], [0., height]],
                Color(red, green, blue, alpha),
            )
            .expect("a positive finite viewport is a valid scrim")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surface::{MicroDom, Node, Role};

    #[test]
    fn unopened_visual_state_never_retires_and_unclocked_frames_snap() {
        let dom = MicroDom::new(Node::new(Role::Window, ""));
        let frame = |now_ms, reduced_motion| AnimationFrame {
            now_ms,
            reduced_motion,
            dom: &dom,
            layout: None,
        };
        let mut motion = BasicDialogMotion::default();
        assert!(!motion.should_mount(false));
        assert!(motion.should_mount(true));
        let first = motion.advance(&frame(Some(10.), false), true);
        assert!(!first.rebuild && first.request_next);
        assert_eq!(motion.opacity(), 0.);
        assert_eq!(
            motion.advance(&frame(Some(30.), false), false),
            AnimationUpdate::default()
        );
        assert!(!motion.should_mount(false));

        let open = motion.advance(&frame(None, false), true);
        assert_eq!(open, AnimationUpdate::default());
        assert_eq!(motion.opacity(), 1.);
        let close = motion.advance(&frame(Some(f64::NAN), false), false);
        assert!(close.rebuild && !close.request_next);
        assert!(!motion.should_mount(false));

        motion.advance(&frame(Some(40.), true), true);
        assert_eq!(motion.opacity(), 1.);
        let open_without_entry = BasicDialogMotion::new(true);
        assert_eq!(open_without_entry.opacity(), 1.);
        assert!(open_without_entry.should_mount(false));
    }
}
