use surface::{Color, FillContour, Length, Node, Role, Style};
use surface_app::View;

const VIEWBOX: f32 = 24.;

#[derive(Clone, Copy)]
pub(crate) enum VectorIconSize {
    Dp24,
    Dp28,
    Dp36,
}

impl VectorIconSize {
    const ALL: [Self; 3] = [Self::Dp24, Self::Dp28, Self::Dp36];

    pub(crate) fn dp(self) -> f32 {
        match self {
            Self::Dp24 => 24.,
            Self::Dp28 => 28.,
            Self::Dp36 => 36.,
        }
    }
}

/// A reusable monochrome vector icon in a 24 × 24 coordinate space.
/// Its component supplies the display size and color.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorIcon {
    points: Vec<[f32; 2]>,
}

impl VectorIcon {
    /// Create an icon from one automatically closed contour.
    /// Each axis must be finite and within `0..=24`.
    /// Returns `None` for invalid or degenerate geometry or more than 256 points.
    pub fn new(points: impl IntoIterator<Item = [f32; 2]>) -> Option<Self> {
        Self::from_contours([points])
    }

    /// Create an icon from one or more automatically closed contours.
    /// The even-odd fill rule supports holes and disconnected parts.
    /// Each axis must be finite and within `0..=24`. Each contour must have at
    /// least three non-collinear points at every supported display size.
    /// The input and the resulting path, including internal connector points,
    /// are each limited to 256 points. Invalid input returns `None`.
    pub fn from_contours<I, C>(contours: I) -> Option<Self>
    where
        I: IntoIterator<Item = C>,
        C: IntoIterator<Item = [f32; 2]>,
    {
        let mut contours = contours.into_iter();
        let mut point_budget = FillContour::MAX_POINTS;
        let mut points = read_contour(contours.next()?, &mut point_budget)?;
        let anchor = points[0];
        for contour in contours {
            let contour = read_contour(contour, &mut point_budget)?;
            if points.len() + contour.len() + 2 > FillContour::MAX_POINTS {
                return None;
            }
            let start = contour[0];
            // Return to the anchor after each contour. Each connector is
            // traced in both directions, so it contributes no even-odd fill.
            points.push(anchor);
            points.extend(contour);
            points.push(start);
        }
        Some(Self { points })
    }

    pub(crate) fn view<M>(&self, size: VectorIconSize, color: Color) -> View<M> {
        self.view_dp(size.dp(), color)
    }

    fn view_dp<M>(&self, size: f32, color: Color) -> View<M> {
        let contour = self
            .contour_dp(size, color)
            .expect("validated icon contour");
        let length = Length::Px(size);
        let square = Style::row().width(length).height(length);
        View::new(Node::new(Role::Group, "").styled(square.clone().clip(true))).children([
            View::new(
                Node::new(Role::Group, "")
                    .styled(square)
                    .fill_contour(contour),
            ),
        ])
    }

    pub(crate) fn contour_dp(&self, size: f32, color: Color) -> Option<FillContour> {
        fill_at_size(&self.points, size, color)
    }
}

fn read_contour(
    points: impl IntoIterator<Item = [f32; 2]>,
    point_budget: &mut usize,
) -> Option<Vec<[f32; 2]>> {
    let mut contour = Vec::new();
    for point in points {
        *point_budget = point_budget.checked_sub(1)?;
        if !point.iter().all(|axis| (0.0..=VIEWBOX).contains(axis)) {
            return None;
        }
        if contour.last() != Some(&point) {
            contour.push(point);
        }
    }
    if contour.first() == contour.last() {
        contour.pop();
    }
    for size in VectorIconSize::ALL {
        fill_at_size(&contour, size.dp(), Color::TRANSPARENT)?;
    }
    Some(contour)
}

fn fill_at_size(points: &[[f32; 2]], size: f32, color: Color) -> Option<FillContour> {
    let factor = f64::from(size) / f64::from(VIEWBOX);
    FillContour::new(
        points
            .iter()
            .map(|point| point.map(|axis| (f64::from(axis) * factor) as f32)),
        color,
    )
}
