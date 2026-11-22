use surface::{Insets, Length, Node, Role, Style, TextMeasurer};
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
