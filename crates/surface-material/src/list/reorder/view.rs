use std::{rc::Rc, sync::OnceLock};

use surface::{Color, Insets, Length, Node, Rect, Role, Style};
use surface_app::{CursorStyle, View};

use super::{
    can_step, ListReorderEntry, ListReorderInput, ListReorderState, MoveDirection, ReorderPreview,
    ReorderableListItem, ReorderableListOptions,
};
use crate::{
    date_picker::disabled_ink,
    list::{
        list_gap, list_icon, list_icon_size, logical_row,
        multi_action::{list_action_copy, list_action_target, ListActionRow, TARGET},
    },
    shape,
    vector_icon::chevron_contour,
    Elevation, ListItem, ListStyle, MultiActionListOptions, Theme, VectorIcon,
};

impl Theme {
    /// Build a list with a drag grip and independent Move Up and Move Down buttons.
    ///
    /// Put the list inside the named clipped vertical scroller in the options.
    /// Supply its allocated width. The builder returns `None` if the width is
    /// nonfinite or less than 48 dp, or if any row ID is empty or repeated. Keep
    /// row IDs stable as the caller applies the requested moves. A fixed row
    /// divides the list into separate movable runs. The primary row action is
    /// independent of the move actions.
    ///
    /// Use the row's ID for the primary action. The row shell uses `-item`.
    /// Move buttons use `-up` and `-down`, and the pointer grip uses `-grip`.
    /// The list uses its own `-lift` and `-drop` IDs for passive drag paint.
    pub fn reorderable_list<'a, M: 'static>(
        &self,
        options: ReorderableListOptions<'_>,
        items: impl IntoIterator<Item = ReorderableListItem<'a, M>>,
        state: &ListReorderState,
        on_input: impl Fn(ListReorderInput) -> M + 'static,
    ) -> Option<View<M>> {
        if !options.available_width.is_finite() || options.available_width < TARGET {
            return None;
        }
        let items: Vec<_> = items.into_iter().collect();
        let order: Vec<_> = items
            .iter()
            .map(|item| ListReorderEntry {
                id: item.row.id,
                movable: item.movable,
            })
            .collect();
        if !super::state::unique_order(&order) {
            return None;
        }
        let measured_ids: Rc<[String]> = [options.id.to_owned(), options.scroller_id.to_owned()]
            .into_iter()
            .chain(order.iter().map(|item| format!("{}-item", item.id)))
            .collect();
        let on_input: Rc<dyn Fn(ListReorderInput) -> M> = Rc::new(on_input);
        let box_options = MultiActionListOptions {
            id: options.id,
            name: options.name,
            style: options.style,
            direction: options.direction,
            available_width: options.available_width,
        };
        let overlays = state
            .preview()
            .and_then(|preview| {
                let source = items
                    .iter()
                    .find(|item| item.movable && item.row.id == preview.item)?;
                Some(self.list_reorder_paint(box_options, &source.row, preview))
            })
            .unwrap_or_default();
        let size = list_icon_size(options.style);
        let last = items.len().saturating_sub(1);
        let rows = items.into_iter().enumerate().map(|(index, item)| {
            let id = item.row.id;
            let up = can_step(&order, index, MoveDirection::Up);
            let down = can_step(&order, index, MoveDirection::Down);
            let step = |direction, enabled: bool| {
                enabled.then(|| on_input(ListReorderInput::step(options.revision, id, direction)))
            };
            let grip = self.reorder_grip(id, size, up || down);
            let grip = if up || down {
                state.bind_grip(grip, options, id, measured_ids.clone(), on_input.clone())
            } else {
                grip
            };
            let up = self.reorder_move_button(
                id,
                item.move_up,
                size,
                MoveDirection::Up,
                step(MoveDirection::Up, up),
            );
            let down = self.reorder_move_button(
                id,
                item.move_down,
                size,
                MoveDirection::Down,
                step(MoveDirection::Down, down),
            );
            self.list_action_row(
                box_options,
                item.row,
                vec![up, down, grip],
                index == 0,
                index == last,
            )
        });
        Some(
            View::new(
                Node::new(Role::List, options.name).id(options.id).styled(
                    Style::column()
                        .width(Length::Px(options.available_width))
                        .gap(list_gap(options.style)),
                ),
            )
            .children(rows.chain(overlays))
            .roving_all_arrows(),
        )
    }

    fn reorder_move_button<M>(
        &self,
        row_id: &str,
        label: &str,
        size: f32,
        direction: MoveDirection,
        on_press: Option<M>,
    ) -> View<M> {
        let (suffix, opposite, angle) = match direction {
            MoveDirection::Up => ("up", "down", 180.),
            MoveDirection::Down => ("down", "up", 0.),
        };
        let id = format!("{row_id}-{suffix}");
        let ink = if on_press.is_some() {
            self.colors.on_surface_variant
        } else {
            disabled_ink(self)
        };
        let icon = list_icon(
            format!("{id}-icon"),
            size,
            chevron_contour(size, angle, ink),
        );
        self.list_action_button(
            Node::new(Role::Button, label).id(id),
            list_action_target(),
            ink,
            on_press,
        )
        .children([icon])
        .restore_focus_on_removal(format!("{row_id}-{opposite}"))
        .restore_focus_in_subtree_on_removal(row_id)
    }

    fn reorder_grip<M>(&self, row_id: &str, size: f32, enabled: bool) -> View<M> {
        let id = format!("{row_id}-grip");
        let ink = if enabled {
            self.colors.on_surface_variant
        } else {
            disabled_ink(self)
        };
        let icon = list_icon(
            format!("{id}-icon"),
            size,
            grip_icon().contour_dp(size, ink),
        );
        let grip = View::new(
            Node::new(Role::Group, "")
                .id(id)
                .disabled(!enabled)
                .styled(list_action_target()),
        )
        .children([icon]);
        if enabled {
            grip.cursor(CursorStyle::Grab)
        } else {
            grip
        }
    }

    fn list_reorder_paint<M>(
        &self,
        options: MultiActionListOptions<'_>,
        source: &ListItem<'_, M>,
        preview: &ReorderPreview,
    ) -> Vec<View<M>> {
        let id = format!("{}-lift", options.id);
        let baseline = options.style == ListStyle::Baseline;
        let colors = self.colors;
        let (fill, ink, icon_ink, opacity, corners, elevation) = if baseline {
            (
                colors.surface,
                colors.on_surface,
                colors.on_surface_variant,
                41,
                shape::NONE,
                Elevation::Level4,
            )
        } else {
            (
                colors.tertiary_container,
                colors.on_tertiary_container,
                colors.on_tertiary_container,
                26,
                shape::LARGE,
                Elevation::Level2,
            )
        };
        let copy = ListItem {
            id: &id,
            headline: source.headline,
            overline: source.overline,
            supporting: source.supporting,
            leading: source.leading,
            trailing: source.trailing,
            lines: source.lines,
            disabled: false,
            on_activate: None,
        };
        let placement = ListActionRow::new(options.available_width, options.style, &copy, 3);
        let primary = View::new(
            Node::new(Role::Group, "").styled(placement.primary_style(options.style, copy.lines)),
        )
        .children([list_action_copy(options, &copy, placement, ink, icon_ink)]);
        let size = list_icon_size(options.style);
        let extras = [
            ("up", chevron_contour(size, 180., icon_ink)),
            ("down", chevron_contour(size, 0., icon_ink)),
            ("grip", grip_icon().contour_dp(size, icon_ink)),
        ]
        .into_iter()
        .map(|(kind, contour)| {
            View::new(Node::new(Role::Group, "").styled(list_action_target()))
                .children([list_icon(format!("{id}-{kind}-icon"), size, contour)])
        })
        .collect();
        let row = View::new(logical_row(placement.style(), options.direction))
            .children(placement.children(primary, extras));
        let local = Rect::new(0., 0., preview.lift.width, preview.lift.height);
        let Color(r, g, b, _) = ink;
        let tint = View::new(
            Node::new(Role::Group, "").styled(placed(
                Style::column()
                    .corners(corners)
                    .background(Color(r, g, b, opacity)),
                local,
            )),
        );
        let lift = View::new(
            Node::new(Role::Group, "")
                .id(&id)
                .styled(placed(
                    Style::column().corners(corners).background(fill),
                    preview.lift,
                ))
                .box_shadows(self.elevation(elevation)),
        )
        .children([tint, row])
        .visual_overlay();
        let drop = preview.drop.map(|rect| {
            View::new(
                Node::new(Role::Group, "")
                    .id(format!("{}-drop", options.id))
                    .styled(placed(
                        Style::column()
                            .corners(shape::EXTRA_SMALL)
                            .background(colors.surface_container_low),
                        rect,
                    )),
            )
            .visual_overlay()
        });
        // Keep the destination marker clear of the lifted row's shadow.
        [lift].into_iter().chain(drop).collect()
    }
}

fn placed(style: Style, rect: Rect) -> Style {
    style
        .width(Length::Px(rect.width))
        .height(Length::Px(rect.height))
        .absolute(Insets {
            left: Length::Px(rect.x),
            top: Length::Px(rect.y),
            right: Length::Auto,
            bottom: Length::Auto,
        })
}

fn grip_icon() -> &'static VectorIcon {
    static GRIP: OnceLock<VectorIcon> = OnceLock::new();
    GRIP.get_or_init(|| {
        const DOT: [[f32; 2]; 8] = [
            [-1., -2.],
            [1., -2.],
            [2., -1.],
            [2., 1.],
            [1., 2.],
            [-1., 2.],
            [-2., 1.],
            [-2., -1.],
        ];
        VectorIcon::from_contours([6., 12., 18.].into_iter().flat_map(|y| {
            [9., 15.]
                .into_iter()
                .map(move |x| DOT.map(|[dx, dy]| [x + dx, y + dy]))
        }))
        .expect("valid list grip icon")
    })
}
