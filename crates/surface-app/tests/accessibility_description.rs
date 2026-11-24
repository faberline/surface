use surface::{GridRole, Insets, Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Runtime, View};

#[derive(Clone, Copy)]
enum Tooltip {
    Visible,
    Clipped,
    Suppressed,
    Disabled,
    IncorrectRole,
    Absent,
}

#[derive(Clone, Copy)]
struct App {
    tooltip: Tooltip,
    source_disabled: bool,
    modal: bool,
}

impl Application for App {
    type Message = Self;

    fn update(&mut self, state: Self) {
        *self = state;
    }

    fn view(&self) -> View<Self> {
        let trigger = View::new(
            Node::new(Role::Button, "Open")
                .id("trigger")
                .text("Open")
                .disabled(self.source_disabled),
        )
        .described_by("tip");
        let tooltip = (!matches!(self.tooltip, Tooltip::Absent)).then(|| {
            let role = if matches!(self.tooltip, Tooltip::IncorrectRole) {
                Role::Text
            } else {
                Role::Tooltip
            };
            let x = if matches!(self.tooltip, Tooltip::Clipped) {
                -200.
            } else {
                10.
            };
            let view = View::new(
                Node::new(role, "Opens the folder")
                    .id("tip")
                    .text("Opens the folder")
                    .disabled(matches!(self.tooltip, Tooltip::Disabled))
                    .styled(Style::default().width(Length::Px(120.)).fixed(Insets {
                        left: Length::Px(x),
                        top: Length::Px(40.),
                        ..Default::default()
                    })),
            );
            if matches!(self.tooltip, Tooltip::Suppressed) {
                view.without_interaction(None)
            } else {
                view
            }
        });
        let modal = self.modal.then(|| {
            View::new(
                Node::new(Role::Dialog, "Settings")
                    .id("settings")
                    .tab_index(-1)
                    .styled(Style::default().height(Length::Px(60.))),
            )
            .modal(*self)
        });
        View::new(Node::new(Role::Window, "App").styled(Style::column()))
            .children(std::iter::once(trigger).chain(tooltip).chain(modal))
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size / 2., size)
    }
}

#[test]
fn description_requires_a_current_visible_tooltip_in_scope() {
    let shown = App {
        tooltip: Tooltip::Visible,
        source_disabled: false,
        modal: false,
    };
    let mut runtime = Runtime::new(shown).unwrap();
    let trigger = runtime.dom().by_id("trigger").unwrap();
    let tip = runtime.dom().by_id("tip").unwrap();
    assert!(runtime.accessibility_frame().is_none());
    runtime.render(320., 240., &mut Metrics).unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.description_target(trigger), Some(tip));
    assert!(frame.focus_target(tip).is_none());
    assert!(frame.activation_target(tip).is_none());

    let cases = [
        App {
            tooltip: Tooltip::Absent,
            ..shown
        },
        App {
            tooltip: Tooltip::Clipped,
            ..shown
        },
        App {
            tooltip: Tooltip::Suppressed,
            ..shown
        },
        App {
            tooltip: Tooltip::Disabled,
            ..shown
        },
        App {
            tooltip: Tooltip::IncorrectRole,
            ..shown
        },
        App {
            source_disabled: true,
            ..shown
        },
        App {
            modal: true,
            ..shown
        },
    ];
    for case in cases {
        runtime.apply(case).unwrap();
        assert!(runtime.accessibility_frame().is_none());
        runtime.render(320., 240., &mut Metrics).unwrap();
        let trigger = runtime.dom().by_id("trigger").unwrap();
        assert!(runtime
            .accessibility_frame()
            .unwrap()
            .description_target(trigger)
            .is_none());
    }
}

#[derive(Clone, Copy)]
enum FormatHint {
    Visible,
    Missing,
    Blank,
    Focusable,
    Disabled,
    Suppressed,
    Clipped,
    OutsideModal,
    ClippedValidation,
    ClippedValidationSuppressed,
    ClippedValidationOutsideModal,
    ClippedValidationWithChild,
}

impl Application for FormatHint {
    type Message = Self;

    fn update(&mut self, message: Self) {
        *self = message;
    }

    fn view(&self) -> View<Self> {
        let clipped_validation = matches!(
            self,
            Self::ClippedValidation
                | Self::ClippedValidationSuppressed
                | Self::ClippedValidationOutsideModal
                | Self::ClippedValidationWithChild
        );
        let field = View::new(
            Node::new(Role::TextInput, "Event date")
                .id("date")
                .text("2026-11-23")
                .invalid(clipped_validation)
                .described_by_text("format"),
        )
        .on_input(|_| Self::Visible);
        let field = if matches!(
            self,
            Self::OutsideModal | Self::ClippedValidationOutsideModal
        ) {
            View::new(Node::new(Role::Dialog, "Edit date").id("dialog"))
                .modal(Self::Visible)
                .children([field])
        } else {
            field
        };
        let format = match self {
            Self::Blank => " ",
            _ => "YYYY-MM-DD",
        };
        let mut hint = Node::new(Role::Text, "Date format")
            .id("format")
            .text(format)
            .styled(Style::default().height(Length::Px(20.)))
            .disabled(matches!(self, Self::Disabled));
        if matches!(self, Self::Focusable) {
            hint = hint.tab_index(0);
        }
        if matches!(self, Self::Clipped) || clipped_validation {
            hint = hint.styled(Style::default().width(Length::Px(100.)).fixed(Insets {
                left: Length::Px(-200.),
                ..Default::default()
            }));
        }
        let mut hint = View::new(hint);
        if matches!(self, Self::Suppressed | Self::ClippedValidationSuppressed) {
            hint = hint.without_interaction(None);
        }
        if matches!(self, Self::ClippedValidationWithChild) {
            hint = hint.children([View::new(
                Node::new(Role::Text, "Visible note")
                    .id("visible-note")
                    .text("Visible note")
                    .styled(
                        Style::default()
                            .width(Length::Px(100.))
                            .height(Length::Px(20.))
                            .fixed(Insets {
                                left: Length::Px(40.),
                                top: Length::Px(100.),
                                ..Default::default()
                            }),
                    ),
            )]);
        }
        let date = View::new(
            Node::new(Role::Button, "Monday, November 23, 2026")
                .id("today")
                .text("23")
                .grid_role(GridRole::GridCell)
                .current_date(true)
                .selected(false),
        )
        .on_press(Self::Visible)
        .accessible_activation();
        View::new(Node::new(Role::Window, "Dates").styled(Style::column())).children(
            [field, date]
                .into_iter()
                .chain((!matches!(self, Self::Missing)).then_some(hint)),
        )
    }
}

#[test]
fn date_field_describes_itself_with_current_static_text() {
    use FormatHint::*;

    let mut runtime = Runtime::new(Visible).unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    let field = runtime.dom().by_id("date").unwrap();
    let hint = runtime.dom().by_id("format").unwrap();
    let date = runtime.dom().by_id("today").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.description_target(field), Some(hint));
    assert!(!frame.accepts_focus(hint));
    assert!(frame.activation_target(hint).is_none());
    assert!(frame.activation_target(date).is_some());

    for scene in [
        Missing,
        Blank,
        Focusable,
        Disabled,
        Suppressed,
        Clipped,
        OutsideModal,
    ] {
        runtime.apply(scene).unwrap();
        runtime.render(320., 240., &mut Metrics).unwrap();
        let field = runtime.dom().by_id("date").unwrap();
        assert!(runtime
            .accessibility_frame()
            .unwrap()
            .description_target(field)
            .is_none());
    }
}

#[test]
fn only_a_mounted_clipped_validation_error_in_scope_can_describe_an_invalid_editor() {
    use FormatHint::*;

    let mut runtime = Runtime::new(ClippedValidation).unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    let field = runtime.dom().by_id("date").unwrap();
    let hint = runtime.dom().by_id("format").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    assert_eq!(frame.description_target(field), None);
    assert_eq!(
        frame.clipped_validation_text(field),
        Some((hint, "YYYY-MM-DD"))
    );

    for scene in [
        Clipped,
        Missing,
        ClippedValidationSuppressed,
        ClippedValidationOutsideModal,
    ] {
        runtime.apply(scene).unwrap();
        runtime.render(320., 240., &mut Metrics).unwrap();
        let field = runtime.dom().by_id("date").unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        assert_eq!(frame.clipped_validation_text(field), None);
    }
}

#[test]
fn a_clipped_text_parent_with_a_visible_child_is_not_a_validation_target() {
    let mut runtime = Runtime::new(FormatHint::ClippedValidationWithChild).unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    let editor = runtime.dom().by_id("date").unwrap();
    let text = runtime.dom().by_id("format").unwrap();
    let child = runtime.dom().by_id("visible-note").unwrap();
    let frame = runtime.accessibility_frame().unwrap();
    let visible = |id| {
        let node = frame.node(id).unwrap();
        let geometry = node.layout.bounds.intersect(node.layout.clip);
        geometry.width > 0. && geometry.height > 0.
    };
    assert!(!visible(text));
    assert!(visible(child));
    assert_eq!(frame.children(text).collect::<Vec<_>>(), [child]);
    assert_eq!(frame.description_target(editor), None);
    assert_eq!(frame.clipped_validation_text(editor), None);
}

#[cfg(feature = "devtools")]
#[test]
fn devtools_exposes_grid_state_and_text_reference_through_the_existing_methods() {
    use serde_json::json;
    use surface_devtools_protocol::{Request, VERSION};

    let mut runtime = Runtime::new(FormatHint::Visible).unwrap();
    let mut call = |method: &str, params| {
        runtime
            .execute_devtools(Request {
                v: VERSION,
                id: 1,
                method: method.into(),
                params,
            })
            .result
            .unwrap()
    };
    let snapshot = call("DOM.snapshot", json!({}));
    assert_eq!(snapshot["version"], 1);
    for id in ["today", "date", "format"] {
        let query = call("DOM.query", json!({"id": id}));
        let node = snapshot["dom"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["stable_id"] == id)
            .unwrap();
        assert_eq!(query["matches"][0]["node"], *node);
        match id {
            "today" => {
                assert_eq!(node["role"], "Button");
                assert_eq!(node["grid_role"], "GridCell");
                assert_eq!(node["current_date"], true);
                assert_eq!(node["selected"], false);
            }
            "date" => assert_eq!(node["described_by_text"], "format"),
            "format" => assert_eq!(node["text"], "YYYY-MM-DD"),
            _ => unreachable!(),
        }
    }
}
