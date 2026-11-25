#[path = "../src/web_accessibility/descriptions.rs"]
mod descriptions;

use descriptions::DescriptionRelations;
use surface::{Insets, Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Runtime, View};

#[derive(Clone, Copy)]
struct App {
    clipped: bool,
    show_prefix: bool,
    show_suffix: bool,
}

impl Application for App {
    type Message = Self;

    fn update(&mut self, message: Self) {
        *self = message;
    }

    fn view(&self) -> View<Self> {
        let editor = View::new(
            Node::new(Role::TextInput, "Amount")
                .id("amount")
                .text("12")
                .described_by_texts(["prefix", "suffix", "prefix"])
                .described_by_text("error")
                .invalid(true),
        )
        .on_input(|_| App {
            clipped: false,
            show_prefix: true,
            show_suffix: true,
        });
        let prefix = self
            .show_prefix
            .then(|| View::new(Node::new(Role::Text, "").id("prefix").text("歐元")));
        let suffix = self
            .show_suffix
            .then(|| View::new(Node::new(Role::Text, "").id("suffix").text("公斤")));
        let style = if self.clipped {
            Style::default().width(Length::Px(100.)).fixed(Insets {
                left: Length::Px(-200.),
                ..Default::default()
            })
        } else {
            Style::default()
        };
        let error = View::new(
            Node::new(Role::Text, "")
                .id("error")
                .text("Use an integer.")
                .styled(style),
        );
        View::new(Node::new(Role::Window, "Application").styled(Style::column()))
            .children([editor, error].into_iter().chain(suffix).chain(prefix))
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size / 2., size)
    }
}

#[test]
fn web_relations_emit_ordered_idrefs_and_keep_the_clipped_error_anchor() {
    let authored = App {
        clipped: false,
        show_prefix: true,
        show_suffix: true,
    };
    let mut runtime = Runtime::new(authored).unwrap();
    for (state, expected) in [
        (authored, &["prefix", "suffix", "error"][..]),
        (
            App {
                clipped: true,
                ..authored
            },
            &["prefix", "suffix", "error"],
        ),
        (
            App {
                show_prefix: false,
                ..authored
            },
            &["suffix", "error"],
        ),
        (
            App {
                show_prefix: false,
                show_suffix: false,
                ..authored
            },
            &["error"],
        ),
    ] {
        runtime.apply(state).unwrap();
        runtime.render(320., 240., &mut Metrics).unwrap();
        let frame = runtime.accessibility_frame().unwrap();
        let visible = runtime
            .dom()
            .iter()
            .filter_map(|(id, _)| {
                let node = frame.node(id)?;
                let rect = node.layout.bounds.intersect(node.layout.clip);
                (rect.width > 0. && rect.height > 0.).then_some(id)
            })
            .collect();
        let relations = DescriptionRelations::collect(&frame, &visible);
        let editor = runtime.dom().by_id("amount").unwrap();
        let error = runtime.dom().by_id("error").unwrap();
        let ids = expected
            .iter()
            .map(|key| {
                let node = runtime.dom().by_id(key).unwrap();
                assert!(relations.text_targets.contains(&node));
                format!("surface-accessibility-text-{}", node.index())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            relations.described_by(editor).as_deref(),
            Some(ids.join(" ").as_str())
        );
        assert_eq!(
            relations.clipped_text.get(&error).map(String::as_str),
            state.clipped.then_some("Use an integer.")
        );
    }
}
