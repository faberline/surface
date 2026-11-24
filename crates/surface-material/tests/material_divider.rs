use surface::{Length, Node, PaintOp, Rect, Role, Style};
use surface_app::{Application, Event, Runtime, View};
use surface_material::{
    shape::LayoutDirection::{self, LeftToRight, RightToLeft},
    ButtonKind, DividerKind, DividerLayout, DividerWeight, Theme,
};
use surface_wgpu::TextSystem;

struct DividerApp {
    theme: Theme,
    direction: LayoutDirection,
    weight: Option<DividerWeight>,
    scale: f32,
    clicks: usize,
}

impl Application for DividerApp {
    type Message = ();

    fn update(&mut self, _: ()) {
        self.clicks += 1;
    }

    fn text_scale(&self) -> f32 {
        self.scale
    }

    fn view(&self) -> View<()> {
        let divider = |id, layout| match self.weight {
            Some(weight) => self
                .theme
                .decorative_divider_with_weight(id, layout, weight),
            None => self.theme.decorative_divider(id, layout),
        };
        let horizontal = |id, kind| {
            divider(
                id,
                DividerLayout::Horizontal {
                    kind,
                    direction: self.direction,
                },
            )
        };
        let button = |id| {
            self.theme
                .button_with(id, id, ButtonKind::Text, false, ())
                .accessible_activation()
        };
        View::new(Node::new(Role::Window, "Dividers").styled(Style::column().gap(4.))).children([
            button("first"),
            horizontal("full", DividerKind::Full),
            horizontal("inset", DividerKind::Inset),
            horizontal("middle", DividerKind::MiddleInset),
            View::new(Node::new(Role::Group, "").styled(Style::row().height(Length::Px(24.))))
                .children([
                    divider("vertical", DividerLayout::Vertical),
                    self.theme.vertical_divider("legacy-vertical"),
                ]),
            self.theme.divider("legacy"),
            self.theme
                .divider_with("legacy-inset", DividerKind::MiddleInset),
            button("last"),
        ])
    }
}

fn bounds(app: &Runtime<DividerApp>, id: &str) -> Rect {
    app.computed().unwrap().nodes[app.dom().by_id(id).unwrap().index()].bounds
}

#[test]
fn both_weights_keep_material_geometry_color_and_legacy_thickness() {
    let mut fonts = TextSystem::default();
    for (weight, thickness) in [
        (None, 1.),
        (Some(DividerWeight::Regular), 1.),
        (Some(DividerWeight::Heavy), 8.),
    ] {
        for (theme, direction, start, scale) in [
            (Theme::light(), LeftToRight, 16., 1.),
            (Theme::dark(), RightToLeft, 0., 2.),
        ] {
            let mut app = Runtime::new(DividerApp {
                theme,
                direction,
                weight,
                scale,
                clicks: 0,
            })
            .unwrap();
            let display = app.render(320., 240., &mut fonts).unwrap();
            for (id, expected) in [
                ("full", (0., 320., thickness)),
                ("inset", (start, 304., thickness)),
                ("middle", (16., 288., thickness)),
                ("vertical", (0., thickness, 24.)),
            ] {
                let rect = bounds(&app, id);
                assert_eq!(
                    (rect.x, rect.width, rect.height),
                    expected,
                    "{id} {weight:?}"
                );
                assert!(
                    display.ops.iter().any(|op| matches!(op,
                        PaintOp::Quad { rect: painted, clip, color, .. }
                            if *painted == rect && rect.intersect(*clip) == rect
                                && *color == theme.colors.outline_variant
                    )),
                    "{id} {weight:?}"
                );
            }
            let legacy = bounds(&app, "legacy");
            let legacy_inset = bounds(&app, "legacy-inset");
            let legacy_vertical = bounds(&app, "legacy-vertical");
            assert_eq!((legacy.width, legacy.height), (320., 1.));
            assert_eq!(
                (legacy_inset.x, legacy_inset.width, legacy_inset.height),
                (16., 288., 1.)
            );
            assert_eq!((legacy_vertical.width, legacy_vertical.height), (1., 24.));
        }
    }
}

#[test]
fn small_containers_clip_the_insets_and_keep_dividers_out_of_input_and_ax() {
    let mut fonts = TextSystem::default();
    for weight in [None, Some(DividerWeight::Heavy)] {
        let mut app = Runtime::new(DividerApp {
            theme: Theme::dark(),
            direction: RightToLeft,
            weight,
            scale: 2.,
            clicks: 0,
        })
        .unwrap();
        app.render(10., 240., &mut fonts).unwrap();
        for id in ["inset", "middle"] {
            let node = app.dom().by_id(id).unwrap();
            let layout = &app.computed().unwrap().nodes[node.index()];
            assert_eq!(layout.bounds.width, 0.);
            assert_eq!(layout.bounds.intersect(layout.clip).width, 0.);
        }
        app.render(320., 240., &mut fonts).unwrap();
        let frame = app.accessibility_frame().unwrap();
        for name in [
            "full",
            "inset",
            "inset-container",
            "middle",
            "middle-container",
            "vertical",
        ] {
            let id = app.dom().by_id(name).unwrap();
            assert!(frame.node(id).is_none(), "{name} {weight:?}");
            assert!(frame.focus_target(id).is_none());
            assert!(frame.activation_target(id).is_none());
        }
        for name in ["legacy", "legacy-inset", "legacy-vertical"] {
            assert!(
                frame.node(app.dom().by_id(name).unwrap()).is_some(),
                "{name}"
            );
        }
        app.dispatch(Event::FocusNext { reverse: false }).unwrap();
        assert_eq!(app.focused(), Some("first"));
        app.dispatch(Event::FocusNext { reverse: false }).unwrap();
        assert_eq!(app.focused(), Some("last"));

        let rect = bounds(&app, "full");
        let (x, y) = (rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        app.dispatch(Event::PointerDown { x, y }).unwrap();
        app.dispatch(Event::PointerUp { x, y }).unwrap();
        assert_eq!(app.app().clicks, 0);
    }
}
