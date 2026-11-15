//! Paint attribution export `surface.paint.v1` (issue #128, O3).
//! Rows 3 and 4 of the acceptance table, plus the before-first-frame error.
#![cfg(feature = "devtools")]
use surface::{Color, Length, Node, PaintOp, Role, Style, TextMeasurer};
use surface_app::{Application, Event, ImeEvent, PaintExport, Runtime, View};

#[derive(Default)]
struct App {
    name: String,
    clicks: u32,
}
#[derive(Clone)]
enum Msg {
    Name(String),
    Click,
}
impl Application for App {
    type Message = Msg;
    fn update(&mut self, message: Msg) {
        match message {
            Msg::Name(value) => self.name = value,
            Msg::Click => self.clicks += 1,
        }
    }
    fn view(&self) -> View<Msg> {
        View::new(
            Node::new(Role::Window, "Paint")
                .id("root")
                .styled(Style::column().padding(4.).gap(4.).clip(true)),
        )
        .children([
            View::new(
                Node::new(Role::Button, "OK").id("ok").text("OK").styled(
                    Style::default()
                        .height(Length::Px(24.))
                        .background(Color::rgb(103, 80, 164))
                        .border(1., Color::rgb(0, 0, 0)),
                ),
            )
            .on_press(Msg::Click),
            // No stable id: the export owner id is null.
            View::new(Node::new(Role::Text, "Note").text("Hi")),
            View::new(
                Node::new(Role::TextInput, "Name")
                    .id("name")
                    .text(&self.name)
                    .styled(
                        Style::default()
                            .height(Length::Px(24.))
                            .background(Color::rgb(255, 255, 255)),
                    ),
            )
            .on_input(Msg::Name),
        ])
    }
}
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size * 0.5, size)
    }
}

/// The pinned export of the first frame at 120 x 100. Button `ok`: fill,
/// border, text. Unnamed text: a null id. Field `name`: fill, empty text.
const FIXTURE: &str = r#"{
  "schema": "surface.paint.v1",
  "viewport": {
    "x": 0.0,
    "y": 0.0,
    "width": 120.0,
    "height": 100.0
  },
  "ops": [
    {
      "owner": {
        "node": 1,
        "id": "ok"
      },
      "op": {
        "Quad": {
          "rect": {
            "x": 4.0,
            "y": 4.0,
            "width": 112.0,
            "height": 24.0
          },
          "clip": {
            "x": 0.0,
            "y": 0.0,
            "width": 120.0,
            "height": 100.0
          },
          "radius": 0.0,
          "color": [
            103,
            80,
            164,
            255
          ]
        }
      }
    },
    {
      "owner": {
        "node": 1,
        "id": "ok"
      },
      "op": {
        "Stroke": {
          "rect": {
            "x": 4.0,
            "y": 4.0,
            "width": 112.0,
            "height": 24.0
          },
          "clip": {
            "x": 0.0,
            "y": 0.0,
            "width": 120.0,
            "height": 100.0
          },
          "radius": 0.0,
          "width": 1.0,
          "color": [
            0,
            0,
            0,
            255
          ]
        }
      }
    },
    {
      "owner": {
        "node": 1,
        "id": "ok"
      },
      "op": {
        "Text": {
          "rect": {
            "x": 4.0,
            "y": 4.0,
            "width": 112.0,
            "height": 24.0
          },
          "clip": {
            "x": 4.0,
            "y": 4.0,
            "width": 112.0,
            "height": 24.0
          },
          "text": "OK",
          "font_size": 16.0,
          "color": [
            29,
            27,
            32,
            255
          ]
        }
      }
    },
    {
      "owner": {
        "node": 2,
        "id": null
      },
      "op": {
        "Text": {
          "rect": {
            "x": 4.0,
            "y": 32.0,
            "width": 112.0,
            "height": 16.0
          },
          "clip": {
            "x": 4.0,
            "y": 32.0,
            "width": 112.0,
            "height": 16.0
          },
          "text": "Hi",
          "font_size": 16.0,
          "color": [
            29,
            27,
            32,
            255
          ]
        }
      }
    },
    {
      "owner": {
        "node": 3,
        "id": "name"
      },
      "op": {
        "Quad": {
          "rect": {
            "x": 4.0,
            "y": 52.0,
            "width": 112.0,
            "height": 24.0
          },
          "clip": {
            "x": 0.0,
            "y": 0.0,
            "width": 120.0,
            "height": 100.0
          },
          "radius": 0.0,
          "color": [
            255,
            255,
            255,
            255
          ]
        }
      }
    },
    {
      "owner": {
        "node": 3,
        "id": "name"
      },
      "op": {
        "EditableText": {
          "rect": {
            "x": 4.0,
            "y": 52.0,
            "width": 112.0,
            "height": 24.0
          },
          "clip": {
            "x": 4.0,
            "y": 52.0,
            "width": 112.0,
            "height": 24.0
          },
          "text": "",
          "font_size": 16.0,
          "color": [
            29,
            27,
            32,
            255
          ],
          "scroll": [
            0.0,
            0.0
          ],
          "multiline": false
        }
      }
    }
  ]
}"#;

fn rendered() -> (Runtime<App>, surface::DisplayList) {
    let mut runtime = Runtime::new(App::default()).unwrap();
    let list = runtime.render(120., 100., &mut Metrics).unwrap();
    (runtime, list)
}

#[test]
fn export_before_the_first_frame_is_an_error() {
    let runtime = Runtime::new(App::default()).unwrap();
    assert!(runtime.paint_snapshot().is_none());
    let error = runtime.export_paint().unwrap_err();
    assert!(error.to_string().contains("render"), "{error}");
}

#[test]
fn export_after_a_state_change_is_an_error_until_the_next_render() {
    let (mut runtime, _) = rendered();
    assert!(runtime.activate("ok").unwrap());
    assert_eq!(runtime.app().clicks, 1);
    assert!(
        runtime.export_paint().is_err(),
        "old geometry is not current"
    );
    runtime.render(120., 100., &mut Metrics).unwrap();
    assert!(runtime.export_paint().is_ok());
}

// Row 3: the JSON text is pinned.
#[test]
fn export_json_is_pinned() {
    let (runtime, _) = rendered();
    assert_eq!(runtime.export_paint().unwrap(), FIXTURE);
}

// Row 3: the JSON round-trips through the typed form.
#[test]
fn export_round_trips() {
    let (runtime, _) = rendered();
    let json = runtime.export_paint().unwrap();
    let parsed: PaintExport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, runtime.paint_snapshot().unwrap());
    assert_eq!(parsed.schema, "surface.paint.v1");
    assert_eq!(serde_json::to_string_pretty(&parsed).unwrap(), json);
    assert_eq!(runtime.export_paint().unwrap(), json, "read-only");
}

/// The export ops are the list `render` returned, and each owner id is the
/// stable id that `DomExport` has for the same node index.
fn assert_matches_frame(runtime: &Runtime<App>, list: &surface::DisplayList) -> PaintExport {
    let paint = runtime.paint_snapshot().unwrap();
    assert_eq!(paint.viewport, list.viewport);
    let ops: Vec<PaintOp> = paint.ops.iter().map(|o| o.op.clone()).collect();
    assert_eq!(ops, list.ops);
    let dom = runtime.snapshot().dom;
    for o in &paint.ops {
        let node = &dom.nodes[o.owner.node as usize];
        assert_eq!(o.owner.id.as_deref(), node.stable_id(), "{o:?}");
        if let PaintOp::Text { text, .. }
        | PaintOp::EditableText { text, .. }
        | PaintOp::StyledText { text, .. } = &o.op
        {
            assert_eq!(node.text_value(), Some(text.as_str()));
        }
    }
    paint
}

// Row 4: owner ids match the DomExport stable ids.
#[test]
fn owner_ids_match_dom_export() {
    let (runtime, list) = rendered();
    let paint = assert_matches_frame(&runtime, &list);
    let ids: Vec<Option<&str>> = paint.ops.iter().map(|o| o.owner.id.as_deref()).collect();
    assert!(ids.contains(&Some("ok")));
    assert!(ids.contains(&Some("name")));
    assert!(ids.contains(&None), "the unnamed text has a null id");
}

// Row 4 with a focused field, composed text, and a caret.
#[test]
fn caret_and_text_belong_to_the_focused_field() {
    let (mut runtime, _) = rendered();
    runtime.activate("name").unwrap();
    runtime
        .dispatch(Event::Ime(ImeEvent::Commit("abc".into())))
        .unwrap();
    let list = runtime.render(120., 100., &mut Metrics).unwrap();
    let paint = assert_matches_frame(&runtime, &list);
    let field: Vec<&PaintOp> = paint
        .ops
        .iter()
        .filter(|o| o.owner.id.as_deref() == Some("name"))
        .map(|o| &o.op)
        .collect();
    assert!(field
        .iter()
        .any(|op| matches!(op, PaintOp::EditableText { text, .. } if text == "abc")));
    // fill, 4 focus quads, text, caret
    assert_eq!(field.len(), 7, "{field:#?}");
}
