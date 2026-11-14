//! Trace recorder (issue #125, O1). Headless: a fixed-metric measurer and an
//! injected clock that steps by 1 ms per call, so every time is exact.
#![cfg(feature = "devtools")]
use std::{cell::RefCell, rc::Rc};
use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::trace::{Data, Handler, RecordKind, Recorder};
use surface_app::{Application, Event, Queue, Runtime, View};

#[derive(Clone, Debug)]
enum Msg {
    Bump,
    Echo,
    Name(String),
}

#[derive(Default)]
struct App {
    n: i32,
    name: String,
    /// When set, `Bump` posts `Echo` through the runtime's queue.
    echo: Rc<RefCell<Option<Queue<Msg>>>>,
    describe: bool,
}
impl Application for App {
    type Message = Msg;
    fn update(&mut self, message: Msg) {
        match message {
            Msg::Bump => {
                self.n += 1;
                if let Some(queue) = &*self.echo.borrow() {
                    queue.post(Msg::Echo);
                }
            }
            Msg::Echo => {}
            Msg::Name(name) => self.name = name,
        }
    }
    fn describe(&self, message: &Msg) -> Option<String> {
        self.describe.then(|| format!("{message:?}"))
    }
    fn view(&self) -> View<Msg> {
        View::new(Node::new(Role::Window, "Trace").styled(Style::column())).children([
            View::new(
                Node::new(Role::Button, "OK")
                    .id("ok")
                    .styled(Style::default().height(Length::Px(40.))),
            )
            .on_press(Msg::Bump),
            View::new(
                Node::new(Role::TextInput, "Name")
                    .id("name")
                    .text(&self.name)
                    .styled(Style::default().height(Length::Px(40.))),
            )
            .on_input(Msg::Name),
        ])
    }
}

struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size, size * 1.5)
    }
}

/// A clock that returns 1, 2, 3, ... ms.
fn step_clock() -> Box<dyn FnMut() -> f64> {
    let mut now = 0.;
    Box::new(move || {
        now += 1.;
        now
    })
}

/// A runtime after its first frame, so pointer hits have geometry.
fn runtime(app: App) -> Runtime<App> {
    let mut runtime = Runtime::new(app).unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    runtime
}

/// Press and release on the "ok" button, then render one frame.
fn click(runtime: &mut Runtime<App>) {
    runtime
        .dispatch(Event::PointerDown { x: 10., y: 10. })
        .unwrap();
    runtime
        .dispatch(Event::PointerUp { x: 10., y: 10. })
        .unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
}

fn kinds(recorder: &Recorder) -> Vec<RecordKind> {
    recorder.records_after(None).map(|r| r.kind()).collect()
}

// Row 1.
#[test]
fn click_yields_ordered_records_in_one_frame() {
    let mut runtime = runtime(App::default());
    runtime.attach_trace(Recorder::new(step_clock()));
    click(&mut runtime);
    assert_eq!(runtime.app().n, 1);
    let trace = runtime.trace().unwrap();
    use RecordKind::*;
    assert_eq!(
        kinds(trace),
        [Input, Input, Message, Update, Rebuild, Layout, Paint],
        "a click is press, release, message, update, rebuild, layout, paint"
    );
    let records: Vec<_> = trace.records_after(None).collect();
    assert!(records.iter().all(|r| r.frame() == 0), "one frame id");
    assert!(
        records.windows(2).all(|w| w[0].seq() < w[1].seq()),
        "seq rises"
    );
    assert_eq!(records[0].seq(), 0);
    assert_eq!(trace.frame(), 1, "the frame id moves on after Paint");
    // The next click belongs to the next frame.
    click(&mut runtime);
    let trace = runtime.trace().unwrap();
    let second: Vec<_> = trace.records_after(Some(6)).collect();
    assert_eq!(second.len(), 7);
    assert_eq!(second[0].seq(), 7);
    assert!(second.iter().all(|r| r.frame() == 1));
    // Exact times from the step clock: the update span is one tick.
    let update = second.iter().find(|r| r.kind() == Update).unwrap();
    assert_eq!(update.dur_ms(), 1.);
}

// Row 2.
#[test]
fn no_recorder_yields_no_records() {
    let mut runtime = runtime(App::default());
    assert!(runtime.trace().is_none());
    click(&mut runtime);
    assert_eq!(runtime.app().n, 1, "the app runs as before");
    runtime.attach_trace(Recorder::new(step_clock()));
    click(&mut runtime);
    let recorder = runtime.detach_trace().unwrap();
    assert_eq!(recorder.len(), 7);
    assert!(runtime.trace().is_none());
    click(&mut runtime);
    assert_eq!(runtime.app().n, 3);
    assert_eq!(recorder.len(), 7, "nothing is written after detach");
    runtime.attach_trace(recorder);
    assert_eq!(runtime.trace().unwrap().len(), 7);
}

// Row 3.
#[test]
fn message_records_carry_ordinal_describe_and_source() {
    let app = App {
        describe: true,
        ..App::default()
    };
    let echo = app.echo.clone();
    let mut runtime = runtime(app);
    *echo.borrow_mut() = Some(runtime.queue());
    runtime.attach_trace(Recorder::new(step_clock()));
    // From outside a binding.
    runtime.apply(Msg::Echo).unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    // A press on "ok". Bump posts Echo through the queue inside update.
    click(&mut runtime);
    // Typed text through the on_input binding of "name".
    runtime.activate("name").unwrap();
    runtime.dispatch(Event::Text("a".into())).unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    assert_eq!(runtime.app().name, "a");

    let trace = runtime.trace().unwrap();
    let messages: Vec<_> = trace
        .records_after(None)
        .filter(|r| r.kind() == RecordKind::Message)
        .collect();
    assert_eq!(messages.len(), 4);
    let ordinal = |i: usize| match messages[i].data() {
        Data::Message { ordinal } => *ordinal,
        other => panic!("not a message: {other:?}"),
    };

    assert_eq!(messages[0].text(), Some("Echo"));
    assert_eq!(messages[0].source(), None, "apply is not a binding");
    assert_eq!((messages[0].frame(), ordinal(0)), (0, 0));

    assert_eq!(messages[1].text(), Some("Bump"));
    assert_eq!(messages[1].source(), Some(("ok", Handler::Press)));
    assert_eq!((messages[1].frame(), ordinal(1)), (1, 0));

    assert_eq!(messages[2].text(), Some("Echo"));
    assert_eq!(messages[2].source(), None, "posted through Queue");
    assert_eq!((messages[2].frame(), ordinal(2)), (1, 1));

    assert_eq!(messages[3].text(), Some("Name(\"a\")"));
    assert_eq!(messages[3].source(), Some(("name", Handler::Input)));
    assert_eq!((messages[3].frame(), ordinal(3)), (2, 0));
}

#[test]
fn describe_defaults_to_none() {
    struct Plain;
    impl Application for Plain {
        type Message = ();
        fn update(&mut self, _: ()) {}
        fn view(&self) -> View<()> {
            View::new(Node::new(Role::Window, "Plain"))
        }
    }
    assert_eq!(Plain.describe(&()), None);
    let mut runtime = runtime(App::default());
    runtime.attach_trace(Recorder::new(step_clock()));
    click(&mut runtime);
    let trace = runtime.trace().unwrap();
    let message = trace
        .records_after(None)
        .find(|r| r.kind() == RecordKind::Message)
        .unwrap();
    assert_eq!(message.text(), None);
    assert_eq!(message.source(), Some(("ok", Handler::Press)));
}

// Row 4.
#[test]
fn ring_wraps_at_capacity_and_keeps_order() {
    let mut runtime = runtime(App::default());
    runtime.attach_trace(Recorder::with_capacity(10, step_clock()));
    for _ in 0..3 {
        click(&mut runtime);
    }
    let trace = runtime.trace().unwrap();
    assert_eq!(trace.capacity(), 10);
    assert_eq!(trace.len(), 10, "21 records written, the oldest 11 dropped");
    let seqs: Vec<u64> = trace.records_after(None).map(|r| r.seq()).collect();
    assert_eq!(seqs, (11..21).collect::<Vec<_>>());
    use RecordKind::*;
    assert_eq!(
        kinds(trace),
        [Rebuild, Layout, Paint, Input, Input, Message, Update, Rebuild, Layout, Paint]
    );
    let after: Vec<u64> = trace.records_after(Some(17)).map(|r| r.seq()).collect();
    assert_eq!(after, [18, 19, 20]);
    assert_eq!(trace.records_after(Some(20)).count(), 0);
    assert_eq!(
        trace.records_after(Some(3)).count(),
        10,
        "a cursor older than the ring returns what is left"
    );
}

#[test]
fn errors_are_recorded() {
    let mut runtime = runtime(App::default());
    runtime.attach_trace(Recorder::new(step_clock()));
    assert!(runtime.activate("missing").is_err());
    let trace = runtime.trace().unwrap();
    use RecordKind::*;
    assert_eq!(kinds(trace), [Input, Error]);
    let input = trace.records_after(None).next().unwrap();
    assert_eq!(input.id(), Some("missing"));
    let error = trace.records_after(Some(0)).next().unwrap();
    assert_eq!(error.text(), Some("no enabled control with ID missing"));
}

/// The exact JSON-lines text of one click with `describe` on.
const CLICK_JSONL: &str = r#"{"seq":0,"frame":0,"kind":"Input","t_ms":1.0,"dur_ms":1.0,"data":{"event":"PointerDown","id":null,"x":10.0,"y":10.0}}
{"seq":1,"frame":0,"kind":"Input","t_ms":3.0,"dur_ms":6.0,"data":{"event":"PointerUp","id":null,"x":10.0,"y":10.0}}
{"seq":2,"frame":0,"kind":"Message","t_ms":4.0,"dur_ms":0.0,"data":{"ordinal":0,"text":"Bump","source":{"id":"ok","handler":"press"}}}
{"seq":3,"frame":0,"kind":"Update","t_ms":5.0,"dur_ms":1.0,"data":{}}
{"seq":4,"frame":0,"kind":"Rebuild","t_ms":7.0,"dur_ms":1.0,"data":{"nodes":3}}
{"seq":5,"frame":0,"kind":"Layout","t_ms":10.0,"dur_ms":1.0,"data":{"nodes":3}}
{"seq":6,"frame":0,"kind":"Paint","t_ms":12.0,"dur_ms":1.0,"data":{"ops":5}}
"#;

#[test]
fn jsonl_export_is_pinned() {
    let mut runtime = runtime(App {
        describe: true,
        ..App::default()
    });
    runtime.attach_trace(Recorder::new(step_clock()));
    click(&mut runtime);
    assert_eq!(runtime.trace().unwrap().export_jsonl(), CLICK_JSONL);
}

mod sources {
    use super::{step_clock, Metrics};
    use surface::{Length, Node, RangeValue, Role, Style};
    use surface_app::trace::{Handler, RecordKind, Recorder};
    use surface_app::{Application, EditKey, Event, Key, KeyPress, Modifiers, Runtime, View};

    #[derive(Clone, Debug)]
    enum Msg {
        Volume(f64),
        Text(String),
        Submit,
        Open,
        Close,
    }
    #[derive(Default)]
    struct Form {
        volume: f64,
        text: String,
        dialog: bool,
    }
    impl Application for Form {
        type Message = Msg;
        fn update(&mut self, message: Msg) {
            match message {
                Msg::Volume(v) => self.volume = v,
                Msg::Text(t) => self.text = t,
                Msg::Submit => {}
                Msg::Open => self.dialog = true,
                Msg::Close => self.dialog = false,
            }
        }
        fn view(&self) -> View<Msg> {
            let mut children = vec![
                View::new(
                    Node::new(Role::Slider, "Volume")
                        .id("vol")
                        .range(RangeValue {
                            min: 0.,
                            max: 20.,
                            step: 2.,
                            value: self.volume,
                        })
                        .styled(Style::default().height(Length::Px(40.))),
                )
                .on_value(Msg::Volume),
                View::new(
                    Node::new(Role::TextInput, "Field")
                        .id("field")
                        .text(&self.text)
                        .styled(Style::default().height(Length::Px(40.))),
                )
                .on_input(Msg::Text)
                .on_key(|press| (press.key == Key::Enter).then_some(Msg::Submit)),
            ];
            if self.dialog {
                children.push(
                    View::new(Node::new(Role::Dialog, "Dialog").id("dialog").tab_index(-1))
                        .modal(Msg::Close),
                );
            }
            View::new(Node::new(Role::Window, "Form").styled(Style::column())).children(children)
        }
    }

    #[test]
    fn key_value_and_dismiss_bindings_are_sources() {
        let mut runtime = Runtime::new(Form::default()).unwrap();
        runtime.render(320., 240., &mut Metrics).unwrap();
        runtime.attach_trace(Recorder::new(step_clock()));

        runtime.activate("field").unwrap();
        let enter = KeyPress::new(Key::Enter, Modifiers::default());
        assert!(runtime.key(&enter).unwrap());

        runtime
            .dispatch(Event::SetValue {
                id: "vol".into(),
                value: 4.,
            })
            .unwrap();
        runtime.activate("vol").unwrap();
        runtime
            .dispatch(Event::Edit {
                key: EditKey::Right,
                extend: false,
                by_word: false,
            })
            .unwrap();
        assert_eq!(runtime.app().volume, 6.);

        runtime.apply(Msg::Open).unwrap();
        runtime.render(320., 240., &mut Metrics).unwrap();
        runtime.dispatch(Event::Cancel).unwrap();
        assert!(!runtime.app().dialog);

        let trace = runtime.trace().unwrap();
        let sources: Vec<_> = trace
            .records_after(None)
            .filter(|r| r.kind() == RecordKind::Message)
            .map(|r| r.source())
            .collect();
        assert_eq!(
            sources,
            [
                Some(("field", Handler::Key)),
                Some(("vol", Handler::Value)),
                Some(("vol", Handler::Value)),
                None,
                Some(("dialog", Handler::Dismiss)),
            ]
        );
        let inputs: Vec<_> = trace
            .records_after(None)
            .filter(|r| r.kind() == RecordKind::Input)
            .map(|r| match r.data() {
                surface_app::trace::Data::Input { event, .. } => (*event, r.id()),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            inputs,
            [
                ("ActivateId", Some("field")),
                ("Key", None),
                ("SetValue", Some("vol")),
                ("ActivateId", Some("vol")),
                ("Edit", None),
                ("Cancel", None),
            ]
        );
    }
}
