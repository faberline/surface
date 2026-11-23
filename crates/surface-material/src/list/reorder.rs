mod geometry;
mod state;
mod view;

pub use state::ListReorderState;

use surface::Rect;
use surface_app::PointDrag;

use super::{LayoutDirection, ListItem, ListStyle};

/// Placement, presentation and current order version for the list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReorderableListOptions<'a> {
    pub id: &'a str,
    pub name: &'a str,
    /// An ancestor with a stable ID and clipping enabled.
    pub scroller_id: &'a str,
    pub style: ListStyle,
    pub direction: LayoutDirection,
    pub available_width: f32,
    /// Change this whenever the app changes the order or the fixed rows.
    pub revision: u64,
}

/// One row and the accessible names of its move commands.
pub struct ReorderableListItem<'a, M> {
    pub row: ListItem<'a, M>,
    /// Fixed rows stay in place. Other rows cannot cross them.
    pub movable: bool,
    pub move_up: &'a str,
    pub move_down: &'a str,
}

/// One entry in the current caller-owned order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListReorderEntry<'a> {
    pub id: &'a str,
    pub movable: bool,
}

/// A requested move. Remove `item`, then insert it immediately before the
/// stable ID in `before`. `None` means the end of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListReorderRequest {
    pub item: String,
    pub before: Option<String>,
}

/// An input from a list view. Forward it to [`ListReorderState::update`].
#[derive(Clone)]
pub struct ListReorderInput(Input);

#[derive(Clone)]
enum Input {
    Step {
        revision: u64,
        item: String,
        direction: MoveDirection,
    },
    Pointer(state::Pointer),
}

#[derive(Clone, Copy)]
enum MoveDirection {
    Up,
    Down,
}

impl MoveDirection {
    fn neighbor(self, index: usize) -> Option<usize> {
        match self {
            Self::Up => index.checked_sub(1),
            Self::Down => index.checked_add(1),
        }
    }
}

fn can_step(order: &[ListReorderEntry<'_>], index: usize, direction: MoveDirection) -> bool {
    order.get(index).is_some_and(|entry| entry.movable)
        && direction
            .neighbor(index)
            .and_then(|index| order.get(index))
            .is_some_and(|entry| entry.movable)
}

impl ListReorderInput {
    fn step(revision: u64, item: &str, direction: MoveDirection) -> Self {
        Self(Input::Step {
            revision,
            item: item.to_owned(),
            direction,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ReorderPreview {
    item: String,
    lift: Rect,
    drop: Option<Rect>,
}

fn vertical_drag(event: PointDrag) -> bool {
    let (origin, position) = match event {
        PointDrag::Move { origin, position } | PointDrag::End { origin, position } => {
            (origin, position)
        }
        _ => return false,
    };
    let dx = (position.0 - origin.0).abs();
    let dy = (position.1 - origin.1).abs();
    dy >= 6. && dy >= dx
}
