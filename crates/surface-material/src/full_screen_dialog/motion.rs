//! Motion from a measured FAB to a full-screen dialog.

use std::{cell::Cell, rc::Rc};

use surface::{Color, FillContour, Rect};
use surface_app::{AnimationFrame, AnimationUpdate, ToggleMotion, ToggleSpring};

/// Measured visual bounds, corner radius and fill of the originating FAB.
/// The bounds use logical window coordinates, excluding shadow and focus ink.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FullScreenDialogAnchor {
    pub bounds: Rect,
    pub radius: f32,
    pub fill: Color,
}

impl FullScreenDialogAnchor {
    /// Read the last fully visible bounds of an authored FAB or its visual face.
    pub fn measured(
        frame: &AnimationFrame<'_>,
        id: &str,
        radius: f32,
        fill: Color,
    ) -> Option<Self> {
        let layout = frame.layout?;
        let node = layout.nodes.get(frame.dom.by_id(id)?.index())?;
        let anchor = Self {
            bounds: node.bounds,
            radius,
            fill,
        };
        (anchor.fits(layout.viewport) && contains(node.clip, node.bounds)).then_some(anchor)
    }

    pub(super) fn fits(self, viewport: Rect) -> bool {
        valid_rect(viewport)
            && valid_rect(self.bounds)
            && self.radius.is_finite()
            && self.radius >= 0.
            && contains(viewport, self.bounds)
    }
}

fn valid_rect(rect: Rect) -> bool {
    rect.width > 0.
        && rect.height > 0.
        && [rect.x, rect.y, rect.x + rect.width, rect.y + rect.height]
            .into_iter()
            .all(f32::is_finite)
}

fn contains(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width <= outer.x + outer.width
        && inner.y + inner.height <= outer.y + outer.height
}

/// Retained motion for one full-screen dialog launched by a FAB.
/// Keep the same stable IDs when switching the application-owned open flag.
pub struct FullScreenDialogMotion {
    track: ToggleMotion,
    progress: Rc<Cell<f32>>,
}

impl Default for FullScreenDialogMotion {
    fn default() -> Self {
        Self::new(false)
    }
}

impl FullScreenDialogMotion {
    /// Start at the requested resting state.
    pub fn new(open: bool) -> Self {
        Self {
            track: ToggleMotion::new(open),
            progress: Rc::new(Cell::new(if open { 1. } else { 0. })),
        }
    }

    /// Current spatial progress. Zero is the FAB and one is the viewport.
    pub fn progress(&self) -> f32 {
        self.progress.get()
    }

    /// Whether to build the live dialog or keep its outgoing paint.
    pub fn should_mount(&self, open: bool) -> bool {
        open || self.progress() > 0.
    }

    /// Advance from the last pose. An unavailable origin, missing clock or
    /// Reduce Motion settles immediately. A valid retiring view is removed
    /// only after it reaches the opener again.
    pub fn advance(
        &mut self,
        frame: &AnimationFrame<'_>,
        open: bool,
        anchor: Option<FullScreenDialogAnchor>,
        viewport: Rect,
    ) -> AnimationUpdate {
        let was_mounted = self.progress() > 0.;
        let origin_available = anchor.is_some_and(|anchor| anchor.fits(viewport));
        let snap = frame.reduced_motion || !origin_available || (!open && !was_mounted);
        let (progress, request_next) = self.track.sample(
            open,
            frame.now_ms,
            snap,
            ToggleSpring::StandardDefaultSpatial,
        );
        self.progress.set(progress);
        AnimationUpdate {
            rebuild: !open && was_mounted && progress == 0.,
            request_next,
        }
    }

    pub(super) fn pose_painter(
        &self,
        anchor: FullScreenDialogAnchor,
        viewport: Rect,
    ) -> impl Fn() -> Option<TransformPose> + 'static {
        let progress = self.progress.clone();
        move || {
            let progress = progress.get();
            (progress < 1.)
                .then(|| TransformPose::at(anchor, viewport, progress))
                .flatten()
        }
    }

    pub(super) fn shell_painter(
        &self,
        anchor: FullScreenDialogAnchor,
        viewport: Rect,
        surface: Color,
    ) -> impl Fn() -> FillContour + 'static {
        let progress = self.progress.clone();
        move || {
            let progress = progress.get();
            let pose = TransformPose::at(anchor, viewport, progress)
                .expect("shell geometry was validated when building the modal");
            let color = crate::tokens::blend(anchor.fill, surface, fade(progress, 0.15, 0.84));
            // At the first sample the actual launcher remains visible. The
            // growing shell covers its glyph before the incoming content fades in.
            let opacity = if progress == 1. {
                0.
            } else {
                pose.cover_opacity
            };
            let color = Color(color.0, color.1, color.2, (255. * opacity).round() as u8);
            let local = Rect::new(
                pose.clip.x - viewport.x,
                pose.clip.y - viewport.y,
                pose.clip.width,
                pose.clip.height,
            );
            rounded_shell(local, pose.radius, color)
        }
    }

    /// The rounded container's current logical bounds. An unavailable
    /// anchor uses the full viewport, which is the resting dialog layout.
    pub fn container_bounds(&self, anchor: Option<FullScreenDialogAnchor>, viewport: Rect) -> Rect {
        anchor
            .and_then(|anchor| TransformPose::at(anchor, viewport, self.progress()))
            .map_or(viewport, |pose| pose.clip)
    }
}

pub(super) struct TransformPose {
    pub(super) clip: Rect,
    pub(super) destination: Rect,
    pub(super) radius: f32,
    cover_opacity: f32,
    pub(super) content_opacity: f32,
}

impl TransformPose {
    fn at(anchor: FullScreenDialogAnchor, viewport: Rect, progress: f32) -> Option<Self> {
        if !anchor.fits(viewport) {
            return None;
        }
        let lerp = |from: f32, to: f32| from + (to - from) * progress;
        let start = anchor.bounds;
        let clip = if progress == 1. {
            viewport
        } else {
            Rect::new(
                lerp(start.x, viewport.x),
                lerp(start.y, viewport.y),
                lerp(start.width, viewport.width),
                lerp(start.height, viewport.height),
            )
        };
        // Scale the content to the animated width and keep its top edge
        // pinned. A separate shell fills the rounded aperture even when a
        // wide destination makes the scaled content shorter than the shell.
        let destination = Rect::new(
            clip.x,
            clip.y,
            clip.width,
            viewport.height * (clip.width / viewport.width),
        );
        let origin_radius = anchor.radius.min(start.width.min(start.height) / 2.);
        Some(Self {
            clip,
            destination,
            radius: lerp(origin_radius, 0.),
            cover_opacity: fade(progress, 0., 0.20),
            content_opacity: fade(progress, 0.20, 0.76),
        })
    }
}

fn rounded_shell(rect: Rect, radius: f32, color: Color) -> FillContour {
    use std::f32::consts::FRAC_PI_2;

    let radius = radius.min(rect.width.min(rect.height) / 2.);
    if radius == 0. || color.3 == 0 {
        return FillContour::new(
            [
                [rect.x, rect.y],
                [rect.x + rect.width, rect.y],
                [rect.x + rect.width, rect.y + rect.height],
                [rect.x, rect.y + rect.height],
            ],
            color,
        )
        .expect("a positive finite rectangle is a valid shell");
    }
    let centers = [
        [rect.x + rect.width - radius, rect.y + radius],
        [rect.x + rect.width - radius, rect.y + rect.height - radius],
        [rect.x + radius, rect.y + rect.height - radius],
        [rect.x + radius, rect.y + radius],
    ];
    let corners = centers
        .into_iter()
        .enumerate()
        .flat_map(|(corner, [x, y])| {
            (0..=8).map(move |step| {
                let angle = (corner as f32 - 1. + step as f32 / 8.) * FRAC_PI_2;
                let (sin, cos) = angle.sin_cos();
                [x + radius * cos, y + radius * sin]
            })
        });
    FillContour::new(corners, color).expect("a rounded rectangle is a valid shell")
}

fn fade(progress: f32, start: f32, end: f32) -> f32 {
    let t = ((progress - start) / (end - start)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use surface::{MicroDom, Node, Role};

    fn anchor(bounds: Rect) -> FullScreenDialogAnchor {
        FullScreenDialogAnchor {
            bounds,
            radius: 16.,
            fill: Color::rgb(110, 90, 140),
        }
    }

    #[test]
    fn shell_tracks_the_real_origin_and_content_pins_to_its_top_edge() {
        let viewport = Rect::new(3., 5., 320., 400.);
        let anchor = anchor(Rect::new(251., 333., 56., 56.));
        let start = TransformPose::at(anchor, viewport, 0.).unwrap();
        assert_eq!(start.clip, anchor.bounds);
        assert_eq!(start.radius, 16.);
        assert_eq!(start.cover_opacity, 0.);
        assert_eq!(start.content_opacity, 0.);
        let middle = TransformPose::at(anchor, viewport, 0.5).unwrap();
        assert_eq!(middle.clip, Rect::new(127., 169., 188., 228.));
        assert_eq!(middle.radius, 8.);
        assert_eq!((middle.destination.x, middle.destination.y), (127., 169.));
        assert_eq!(middle.destination.width, 188.);
        assert!((middle.destination.height - 235.).abs() < 0.001);
        assert_eq!(middle.cover_opacity, 1.);
        assert!((0. ..1.).contains(&middle.content_opacity));
        let rest = TransformPose::at(anchor, viewport, 1.).unwrap();
        assert_eq!(rest.destination, viewport);
        assert_eq!(rest.clip, viewport);
        assert_eq!(rest.radius, 0.);
        assert_eq!(rest.content_opacity, 1.);
    }

    #[test]
    fn landscape_keeps_content_proportions_under_the_aperture() {
        let viewport = Rect::new(0., 0., 560., 320.);
        let anchor = anchor(Rect::new(488., 248., 56., 56.));
        let pose = TransformPose::at(anchor, viewport, 0.).unwrap();
        assert_eq!(pose.destination, Rect::new(488., 248., 56., 32.));
        assert_eq!(pose.clip, anchor.bounds);
        assert_eq!(
            FullScreenDialogMotion::default()
                .container_bounds(Some(anchor), Rect::new(0., 0., 320., 280.),),
            Rect::new(0., 0., 320., 280.)
        );
    }

    #[test]
    fn reverse_preserves_the_current_pose_and_reduced_motion_clears_retirement() {
        let viewport = Rect::new(0., 0., 320., 400.);
        let origin = Some(anchor(Rect::new(248., 328., 56., 56.)));
        let dom = MicroDom::new(Node::new(Role::Window, ""));
        let frame = |now_ms, reduced_motion| AnimationFrame {
            now_ms,
            reduced_motion,
            dom: &dom,
            layout: None,
        };
        let mut motion = FullScreenDialogMotion::default();
        motion.advance(&frame(Some(0.), false), true, origin, viewport);
        assert!(
            motion
                .advance(&frame(Some(60.), false), true, origin, viewport)
                .request_next
        );
        let presented = motion.container_bounds(origin, viewport);
        assert!(presented.width > 56. && presented.width < 320.);
        motion.advance(&frame(Some(60.), false), false, origin, viewport);
        assert_eq!(motion.container_bounds(origin, viewport), presented);
        assert!(motion.should_mount(false));
        let stop = motion.advance(&frame(Some(70.), true), false, origin, viewport);
        assert!(stop.rebuild && !stop.request_next);
        assert!(!motion.should_mount(false));
        let fallback = motion.advance(&frame(Some(80.), false), true, None, viewport);
        assert!(!fallback.request_next);
        assert_eq!(motion.progress(), 1.);
    }
}
