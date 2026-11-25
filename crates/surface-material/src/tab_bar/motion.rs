use std::{cell::Cell, rc::Rc};

use surface::{scroll_extent, Color, FillContour, NodeId, Rect, Role};
use surface_app::{
    AnimationFrame, AnimationUpdate, FrameInput, HorizontalDrag, HorizontalDragSession, Queue,
    ScrollTarget, SpatialMotion,
};

use super::{member, LayoutDirection, TabBarKind, TabBarLayout, TabBarOptions};

/// Movement of the strip itself. It never selects a tab.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum TabBarInput {
    Drag(HorizontalDrag),
}

#[derive(PartialEq)]
struct GeometryKey {
    id: String,
    kind: TabBarKind,
    layout: TabBarLayout,
    direction: LayoutDirection,
    available_width: f32,
    text_scale: f32,
}

impl GeometryKey {
    fn matches(&self, options: TabBarOptions<'_>) -> bool {
        self.id == options.id
            && self.kind == options.kind
            && self.layout == options.layout
            && self.direction == options.direction
            && self.available_width == options.available_width
            && self.text_scale == options.text_scale
    }
}

#[derive(Default)]
struct Interacting {
    hover: Option<String>,
    focus: Option<String>,
    press: Option<String>,
}

impl Interacting {
    fn has(&self, tab: &str) -> bool {
        self.hover.as_deref() == Some(tab)
            || self.focus.as_deref() == Some(tab)
            || self.press.as_deref() == Some(tab)
    }
}

struct ScrollGeometry {
    displayed: f32,
    max: f32,
}

struct IndicatorTracks {
    target: Rect,
    left: SpatialMotion,
    width: SpatialMotion,
}

/// Retain the strip's measured indicator and captured drag across rebuilds.
///
/// The indicator uses two Material Standard Fast Spatial coordinates.
/// Accepted selection and content remain caller-owned and change immediately.
/// Paint-only motion does not rebuild the accessibility tree.
#[derive(Default)]
pub struct TabBarMotion {
    geometry: Option<GeometryKey>,
    measured: Vec<(String, Rect)>,
    indicator: Rc<Cell<Option<Rect>>>,
    tracks: Option<IndicatorTracks>,
    drag_session: HorizontalDragSession,
    scroll: Option<ScrollGeometry>,
    drag_anchor: Option<f32>,
    interacting: Interacting,
}

impl TabBarMotion {
    /// Current physical indicator rectangle in the strip's unscrolled space.
    pub fn indicator(&self) -> Option<Rect> {
        self.indicator.get()
    }

    /// Pan the real scroll container. A canceled drag restores its down-time
    /// offset. Mouse or touch taps and movements on the other axis stay with
    /// the runtime's normal capture and activation handling.
    pub fn apply<M>(&mut self, options: TabBarOptions<'_>, input: TabBarInput, queue: &Queue<M>) {
        if options.layout != TabBarLayout::Scrollable
            || !self
                .geometry
                .as_ref()
                .is_some_and(|key| key.matches(options))
        {
            self.drag_anchor = None;
            return;
        }
        let Some(geometry) = &self.scroll else {
            return;
        };
        let goal = match input {
            TabBarInput::Drag(HorizontalDrag::Move { delta_x }) if delta_x.is_finite() => {
                *self.drag_anchor.get_or_insert(geometry.displayed) - delta_x
            }
            TabBarInput::Drag(HorizontalDrag::End { delta_x, .. }) if delta_x.is_finite() => {
                let Some(anchor) = self.drag_anchor.take() else {
                    return;
                };
                anchor - delta_x
            }
            TabBarInput::Drag(HorizontalDrag::Cancel) => {
                let Some(anchor) = self.drag_anchor.take() else {
                    return;
                };
                anchor
            }
            _ => return,
        };
        set_scroll(queue, options.id, goal.clamp(0., geometry.max));
    }

    /// Observe the accepted input state from an application frame. Return
    /// whether inactive label or icon ink changed and needs one view rebuild.
    /// Use this with `Application::animation_frame_with_press_detail`.
    pub fn observe_interaction(
        &mut self,
        options: TabBarOptions<'_>,
        frame: &AnimationFrame<'_>,
        input: FrameInput<'_>,
        pressed: Option<&str>,
        hovered: Option<&str>,
    ) -> bool {
        let strip = frame.dom.by_id(&member(options.id, "strip"));
        let active_key = |key: Option<&str>| {
            let key = key?;
            let node = frame.dom.get(frame.dom.by_id(key)?).ok()?;
            (node.role() == Role::Tab
                && !node.is_disabled()
                && node.parent() == strip
                && strip.is_some())
            .then(|| key.to_owned())
        };
        let next = Interacting {
            hover: active_key(hovered),
            focus: active_key(input.focused),
            press: active_key(pressed),
        };
        let changed = [&self.interacting, &next].into_iter().any(|state| {
            [&state.hover, &state.focus, &state.press]
                .into_iter()
                .flatten()
                .filter(|id| Some(id.as_str()) != options.selected)
                .any(|id| self.interacting.has(id) != next.has(id))
        });
        self.interacting = next;
        changed
    }

    /// Advance with the current presentation options before each draw.
    /// Horizontal scrolling uses the layout's physical coordinates so
    /// viewport scrolling never retargets the indicator.
    pub fn advance<M>(
        &mut self,
        options: TabBarOptions<'_>,
        frame: &AnimationFrame<'_>,
        queue: &Queue<M>,
    ) -> AnimationUpdate {
        let Some(strip) = frame.dom.by_id(&member(options.id, "strip")) else {
            self.geometry = None;
            self.measured.clear();
            self.tracks = None;
            self.indicator.set(None);
            self.scroll = None;
            self.drag_anchor = None;
            return AnimationUpdate::default();
        };
        self.reconcile_geometry(options, queue);
        let has_layout = frame.layout.is_some();
        if has_layout {
            self.measure(options, frame, strip);
        }
        if options.selected.is_none() {
            self.indicator.set(None);
            self.tracks = None;
            return AnimationUpdate {
                rebuild: false,
                request_next: !has_layout && options.layout == TabBarLayout::Scrollable,
            };
        }
        let target = options.selected.and_then(|selected| {
            self.measured
                .iter()
                .find_map(|(id, rectangle)| (id == selected).then_some(*rectangle))
        });
        let now = frame.now_ms.filter(|_| !frame.reduced_motion);
        let request_next = self.present(target, now) || !has_layout;
        AnimationUpdate {
            rebuild: false,
            request_next,
        }
    }

    fn reconcile_geometry<M>(&mut self, options: TabBarOptions<'_>, queue: &Queue<M>) {
        let prior = self.geometry.as_ref();
        if prior.is_some_and(|key| key.matches(options)) {
            return;
        }
        let direction_changed = prior.is_some_and(|prior| prior.direction != options.direction);
        let was_scrollable = prior.is_some_and(|prior| {
            prior.id == options.id && prior.layout == TabBarLayout::Scrollable
        });
        if options.layout == TabBarLayout::Scrollable
            && (!was_scrollable || direction_changed)
            && self.interacting.focus.is_none()
        {
            let start = match options.direction {
                LayoutDirection::LeftToRight => 0.,
                LayoutDirection::RightToLeft => f32::MAX,
            };
            // Runtime clamps the logical start against newly measured content.
            set_scroll(queue, options.id, start);
        }
        self.geometry = Some(GeometryKey {
            id: options.id.to_owned(),
            kind: options.kind,
            layout: options.layout,
            direction: options.direction,
            available_width: options.available_width,
            text_scale: options.text_scale,
        });
        self.measured.clear();
        self.scroll = None;
        self.drag_anchor = None;
    }

    fn measure(&mut self, options: TabBarOptions<'_>, frame: &AnimationFrame<'_>, strip: NodeId) {
        let Some(layout) = frame.layout else { return };
        let strip_bounds = layout.nodes[strip.index()].bounds;
        let height = options.kind.indicator_height();
        self.measured.clear();
        for tab in frame.dom.children(strip) {
            let Ok(node) = frame.dom.get(tab) else {
                continue;
            };
            let Some(key) = node.stable_id().filter(|_| node.role() == Role::Tab) else {
                continue;
            };
            let tab_bounds = layout.nodes[tab.index()].bounds;
            let (center, width) = match options.kind {
                TabBarKind::Primary => {
                    let Some(content) = frame.dom.by_id(&member(key, "content")) else {
                        continue;
                    };
                    let content = layout.nodes[content.index()].bounds;
                    (
                        tab_bounds.x + tab_bounds.width / 2.,
                        content.width.max(24.).min(tab_bounds.width),
                    )
                }
                TabBarKind::Secondary => (tab_bounds.x + tab_bounds.width / 2., tab_bounds.width),
            };
            if width > 0. && strip_bounds.height >= height {
                self.measured.push((
                    key.to_owned(),
                    Rect::new(
                        center - strip_bounds.x - width / 2.,
                        strip_bounds.height - height,
                        width,
                        height,
                    ),
                ));
            }
        }
        self.scroll = frame
            .dom
            .by_id(&member(options.id, "scroll"))
            .and_then(|scroller| {
                let width = layout.nodes[scroller.index()].bounds.width;
                let (content, _) = scroll_extent(frame.dom, layout, scroller)?;
                let displayed = frame.dom.get(scroller).ok()?.style().scroll_x;
                Some(ScrollGeometry {
                    displayed,
                    max: (content - width).max(0.),
                })
            });
    }

    fn present(&mut self, target: Option<Rect>, now: Option<f64>) -> bool {
        let Some(target) = target else {
            if !self.measured.is_empty() {
                self.indicator.set(None);
                self.tracks = None;
            }
            return false;
        };
        let Some(now) = now.filter(|time| time.is_finite()) else {
            self.indicator.set(Some(target));
            self.tracks = None;
            return false;
        };
        let initial = self.indicator.get().unwrap_or(target);
        let tracks = self.tracks.get_or_insert_with(|| IndicatorTracks {
            target: initial,
            left: SpatialMotion::new(f64::from(initial.x)),
            width: SpatialMotion::new(f64::from(initial.width)),
        });
        if (tracks.target.x - target.x).abs() > 0.01 {
            tracks.left.aim(f64::from(target.x), now);
        }
        if (tracks.target.width - target.width).abs() > 0.01 {
            tracks.width.aim(f64::from(target.width), now);
        }
        tracks.target = target;
        let (left, moving_left) = tracks.left.advance(now);
        let (width, moving_width) = tracks.width.advance(now);
        let moving = moving_left || moving_width;
        let shown = if moving {
            Rect::new(
                left as f32,
                target.y,
                (width as f32).max(target.height * 2.),
                target.height,
            )
        } else {
            target
        };
        self.indicator.set(Some(shown));
        moving
    }

    pub(super) fn is_interacting(&self, id: &str) -> bool {
        self.interacting.has(id)
    }

    pub(super) fn drag_session(&self) -> HorizontalDragSession {
        self.drag_session.clone()
    }

    pub(super) fn painter(
        &self,
        kind: TabBarKind,
        color: Color,
    ) -> impl Fn() -> FillContour + 'static {
        let indicator = self.indicator.clone();
        move || {
            indicator
                .get()
                .and_then(|pose| contour(pose, kind, color))
                .unwrap_or_else(|| {
                    FillContour::new([[0., 0.], [1., 0.], [0., 1.]], Color::TRANSPARENT)
                        .expect("nondegenerate invisible contour")
                })
        }
    }
}

fn set_scroll<M>(queue: &Queue<M>, id: &str, x: f32) {
    queue.scroll_to(&member(id, "scroll"), ScrollTarget::Offset { x, y: 0. });
}

fn contour(rect: Rect, kind: TabBarKind, color: Color) -> Option<FillContour> {
    let x = rect.x;
    let right = rect.x + rect.width;
    let y = rect.height;
    let mut points = Vec::with_capacity(12);
    if kind == TabBarKind::Primary {
        // The radius equals the full indicator height. The quad renderer
        // clamps this special top-only corner to half the strip height.
        let radius = y.min(rect.width / 2.);
        for angle in 0..=4 {
            let angle = f32::from(angle as u16) * std::f32::consts::FRAC_PI_8;
            points.push([x + radius * (1. - angle.cos()), y - radius * angle.sin()]);
        }
        for angle in (0..=4).rev() {
            let angle = f32::from(angle as u16) * std::f32::consts::FRAC_PI_8;
            points.push([
                right - radius * (1. - angle.cos()),
                y - radius * angle.sin(),
            ]);
        }
    } else {
        points.extend([[x, 0.], [right, 0.]]);
    }
    points.extend([[right, y], [x, y]]);
    FillContour::new(points, color)
}
