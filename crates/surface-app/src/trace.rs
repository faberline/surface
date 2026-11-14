//! Trace recorder, schema `surface.trace.v1`. Feature `devtools`.
//!
//! A bounded ring of what `Runtime` did for each frame: input, message,
//! update, rebuild, layout and paint. Plain Rust data with a JSON-lines
//! export. The host passes the clock; this crate never reads the time.
//! See `docs/trace.md`.
use crate::{AppError, Application, Event, Runtime};
use serde::Serialize;
use surface::Role;

/// The schema name of the JSON-lines export.
pub const SCHEMA: &str = "surface.trace.v1";
/// Records kept by `Recorder::new`.
pub const DEFAULT_CAPACITY: usize = 4096;
/// Bytes reserved per record for a stable id. A longer id grows that
/// record's buffer once; the buffer is then reused.
const ID_CAPACITY: usize = 64;

/// The kind of a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordKind {
    Input,
    Message,
    Update,
    Rebuild,
    Layout,
    Paint,
    Present,
    Error,
}
impl RecordKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "Input",
            Self::Message => "Message",
            Self::Update => "Update",
            Self::Rebuild => "Rebuild",
            Self::Layout => "Layout",
            Self::Paint => "Paint",
            Self::Present => "Present",
            Self::Error => "Error",
        }
    }
}

/// The binding handler that produced a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handler {
    /// `View::on_press`.
    Press,
    /// `View::on_input`.
    Input,
    /// `View::on_value`.
    Value,
    /// `View::on_key`.
    Key,
    /// `View::modal`.
    Dismiss,
}
impl Handler {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Press => "press",
            Self::Input => "input",
            Self::Value => "value",
            Self::Key => "key",
            Self::Dismiss => "dismiss",
        }
    }
}

/// The fixed-size part of a record. Strings live on `Record`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Data {
    /// `event` is the `Event` variant name, `"Key"` for `Runtime::key` or
    /// `"ActivateId"` for `Runtime::activate`. Text content is not kept.
    Input {
        event: &'static str,
        x: Option<f32>,
        y: Option<f32>,
    },
    /// `ordinal` counts messages in the frame from 0.
    Message {
        ordinal: u32,
    },
    Update,
    /// Nodes in the rebuilt `MicroDom`.
    Rebuild {
        nodes: u32,
    },
    /// Nodes in the computed layout.
    Layout {
        nodes: u32,
    },
    /// Ops in the display list.
    Paint {
        ops: u32,
    },
    /// Written by the host after it presents a frame.
    Present,
    Error,
}
impl Data {
    pub const fn kind(&self) -> RecordKind {
        match self {
            Self::Input { .. } => RecordKind::Input,
            Self::Message { .. } => RecordKind::Message,
            Self::Update => RecordKind::Update,
            Self::Rebuild { .. } => RecordKind::Rebuild,
            Self::Layout { .. } => RecordKind::Layout,
            Self::Paint { .. } => RecordKind::Paint,
            Self::Present => RecordKind::Present,
            Self::Error => RecordKind::Error,
        }
    }
}

/// One trace record. `t_ms` is the start time and `dur_ms` the duration,
/// both from the recorder's clock.
#[derive(Debug)]
pub struct Record {
    seq: u64,
    frame: u64,
    t_ms: f64,
    dur_ms: f64,
    data: Data,
    /// Reused buffer: the input target or the message source id.
    id: String,
    has_id: bool,
    handler: Option<Handler>,
    /// `describe` text or the error message.
    text: Option<String>,
}
impl Record {
    fn empty() -> Self {
        Self {
            seq: 0,
            frame: 0,
            t_ms: 0.,
            dur_ms: 0.,
            data: Data::Update,
            id: String::with_capacity(ID_CAPACITY),
            has_id: false,
            handler: None,
            text: None,
        }
    }
    pub fn seq(&self) -> u64 {
        self.seq
    }
    pub fn frame(&self) -> u64 {
        self.frame
    }
    pub fn kind(&self) -> RecordKind {
        self.data.kind()
    }
    pub fn t_ms(&self) -> f64 {
        self.t_ms
    }
    pub fn dur_ms(&self) -> f64 {
        self.dur_ms
    }
    pub fn data(&self) -> &Data {
        &self.data
    }
    /// The stable id an input names, or a message's source id.
    pub fn id(&self) -> Option<&str> {
        self.has_id.then_some(self.id.as_str())
    }
    /// A message's source binding: stable id and handler kind. `None` for a
    /// message posted through `Queue` or `Runtime::apply`.
    pub fn source(&self) -> Option<(&str, Handler)> {
        Some((self.id()?, self.handler?))
    }
    /// The `Application::describe` text of a message, or an error message.
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }
    /// One JSON object, with a fixed field order, and no newline.
    pub fn write_json(&self, out: &mut String) {
        out.push_str("{\"seq\":");
        json(out, &self.seq);
        out.push_str(",\"frame\":");
        json(out, &self.frame);
        out.push_str(",\"kind\":");
        json(out, self.kind().as_str());
        out.push_str(",\"t_ms\":");
        json(out, &self.t_ms);
        out.push_str(",\"dur_ms\":");
        json(out, &self.dur_ms);
        out.push_str(",\"data\":{");
        match self.data {
            Data::Input { event, x, y } => {
                out.push_str("\"event\":");
                json(out, event);
                out.push_str(",\"id\":");
                json(out, &self.id());
                out.push_str(",\"x\":");
                json(out, &x);
                out.push_str(",\"y\":");
                json(out, &y);
            }
            Data::Message { ordinal } => {
                out.push_str("\"ordinal\":");
                json(out, &ordinal);
                out.push_str(",\"text\":");
                json(out, &self.text());
                out.push_str(",\"source\":");
                match self.source() {
                    Some((id, handler)) => {
                        out.push_str("{\"id\":");
                        json(out, id);
                        out.push_str(",\"handler\":");
                        json(out, handler.as_str());
                        out.push('}');
                    }
                    None => out.push_str("null"),
                }
            }
            Data::Rebuild { nodes } | Data::Layout { nodes } => {
                out.push_str("\"nodes\":");
                json(out, &nodes);
            }
            Data::Paint { ops } => {
                out.push_str("\"ops\":");
                json(out, &ops);
            }
            Data::Error => {
                out.push_str("\"message\":");
                json(out, &self.text());
            }
            Data::Update | Data::Present => {}
        }
        out.push_str("}}");
    }
}

/// Serde's JSON text for a value: escaped strings and `null` for `None`.
/// Times are finite here; `Recorder::push` and `set_dur` guard them.
fn json(out: &mut String, value: &(impl Serialize + ?Sized)) {
    out.push_str(&serde_json::to_string(value).expect("trace values serialize"));
}

/// A fixed-capacity ring of records. The ring is allocated when the
/// recorder is created; writing a record does not allocate, except for the
/// `describe` text and error messages.
pub struct Recorder {
    clock: Box<dyn FnMut() -> f64>,
    ring: Vec<Record>,
    start: usize,
    len: usize,
    next_seq: u64,
    frame: u64,
    ordinal: u32,
    /// Source of the next message, set by the input hook.
    pending: String,
    pending_handler: Option<Handler>,
    /// Nested input calls, for example `dispatch` calling `activate`.
    depth: u32,
    /// The last written `t_ms`, used in place of a non-finite time.
    last_t_ms: f64,
}
impl std::fmt::Debug for Recorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Recorder")
            .field("capacity", &self.ring.len())
            .field("len", &self.len)
            .field("next_seq", &self.next_seq)
            .field("frame", &self.frame)
            .finish_non_exhaustive()
    }
}
impl Recorder {
    /// A recorder with `DEFAULT_CAPACITY` records. `clock` returns the
    /// time in milliseconds; the host supplies it.
    pub fn new(clock: Box<dyn FnMut() -> f64>) -> Self {
        Self::with_capacity(DEFAULT_CAPACITY, clock)
    }
    /// A recorder that keeps the last `capacity` records (at least one).
    pub fn with_capacity(capacity: usize, clock: Box<dyn FnMut() -> f64>) -> Self {
        Self {
            clock,
            ring: (0..capacity.max(1)).map(|_| Record::empty()).collect(),
            start: 0,
            len: 0,
            next_seq: 0,
            frame: 0,
            ordinal: 0,
            pending: String::with_capacity(ID_CAPACITY),
            pending_handler: None,
            depth: 0,
            last_t_ms: 0.,
        }
    }
    pub fn capacity(&self) -> usize {
        self.ring.len()
    }
    /// Records held now. At most `capacity`.
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// The frame id that the next `Paint` closes. Starts at 0.
    pub fn frame(&self) -> u64 {
        self.frame
    }
    /// Records with `seq` greater than `after`, oldest first. `None` reads
    /// everything held. A cursor older than the ring returns what is left.
    pub fn records_after(&self, after: Option<u64>) -> impl Iterator<Item = &Record> {
        let capacity = self.ring.len();
        (0..self.len)
            .map(move |i| &self.ring[(self.start + i) % capacity])
            .filter(move |r| after.is_none_or(|after| r.seq > after))
    }
    /// All held records as JSON lines, one record per line.
    pub fn export_jsonl(&self) -> String {
        let mut out = String::new();
        self.write_jsonl(None, &mut out);
        out
    }
    /// Append the records after `after` as JSON lines.
    pub fn write_jsonl(&self, after: Option<u64>, out: &mut String) {
        for record in self.records_after(after) {
            record.write_json(out);
            out.push('\n');
        }
    }
    /// The host writes this after it presents the last painted frame.
    pub fn record_present(&mut self, t_ms: f64, dur_ms: f64) {
        let frame = self.frame.saturating_sub(1);
        let seq = self.push(t_ms, Data::Present, None, None);
        self.slot(seq).expect("just written").frame = frame;
        self.set_dur(seq, dur_ms);
    }

    fn now(&mut self) -> f64 {
        (self.clock)()
    }
    fn push(&mut self, t_ms: f64, data: Data, id: Option<&str>, handler: Option<Handler>) -> u64 {
        let capacity = self.ring.len();
        let index = if self.len < capacity {
            self.len += 1;
            (self.start + self.len - 1) % capacity
        } else {
            let index = self.start;
            self.start = (self.start + 1) % capacity;
            index
        };
        // The export never holds `null` times: a non-finite start time
        // repeats the previous record's start time (0 for the first).
        let t_ms = if t_ms.is_finite() {
            t_ms
        } else {
            self.last_t_ms
        };
        self.last_t_ms = t_ms;
        let seq = self.next_seq;
        self.next_seq += 1;
        let slot = &mut self.ring[index];
        slot.seq = seq;
        slot.frame = self.frame;
        slot.t_ms = t_ms;
        slot.dur_ms = 0.;
        slot.data = data;
        slot.id.clear();
        slot.has_id = id.is_some();
        slot.id.push_str(id.unwrap_or_default());
        slot.handler = handler;
        slot.text = None;
        seq
    }
    fn slot(&mut self, seq: u64) -> Option<&mut Record> {
        let first = self.next_seq - self.len as u64;
        if seq < first || seq >= self.next_seq {
            return None;
        }
        let capacity = self.ring.len();
        let index = (self.start + (seq - first) as usize) % capacity;
        Some(&mut self.ring[index])
    }
    fn set_dur(&mut self, seq: u64, dur_ms: f64) {
        if let Some(slot) = self.slot(seq) {
            // A non-finite or negative duration (a clock that jumps or runs
            // backwards) becomes 0.
            slot.dur_ms = if dur_ms.is_finite() {
                dur_ms.max(0.)
            } else {
                0.
            };
        }
    }
    /// Note the binding that the next message comes from.
    pub(crate) fn set_source(&mut self, id: &str, handler: Handler) {
        self.pending.clear();
        self.pending.push_str(id);
        self.pending_handler = Some(handler);
    }
    fn error(&mut self, error: &AppError) {
        let t = self.now();
        let seq = self.push(t, Data::Error, None, None);
        self.slot(seq).expect("just written").text = Some(error.to_string());
    }
}

/// An input span: its record, if one was written at depth 0.
pub(crate) struct InputSpan(Option<(u64, f64)>);

impl<A: Application> Runtime<A> {
    /// Start recording into `recorder`. A recorder already attached is
    /// dropped.
    pub fn attach_trace(&mut self, recorder: Recorder) {
        self.trace = Some(recorder);
    }
    /// Stop recording and return the recorder with its records.
    pub fn detach_trace(&mut self) -> Option<Recorder> {
        self.trace.take()
    }
    pub fn trace(&self) -> Option<&Recorder> {
        self.trace.as_ref()
    }
    pub fn trace_mut(&mut self) -> Option<&mut Recorder> {
        self.trace.as_mut()
    }

    pub(crate) fn trace_now(&mut self) -> Option<f64> {
        self.trace.as_mut().map(Recorder::now)
    }
    /// Input hook for `dispatch`. Also notes which binding the event will
    /// reach, so the first message it produces carries that source.
    pub(crate) fn trace_event(&mut self, event: &Event) -> Option<InputSpan> {
        self.trace.as_ref()?;
        let (name, x, y, id) = match event {
            Event::PointerDown { x, y } => ("PointerDown", Some(*x), Some(*y), None),
            Event::PointerUp { x, y } => ("PointerUp", Some(*x), Some(*y), None),
            Event::PointerMove { x, y } => ("PointerMove", Some(*x), Some(*y), None),
            Event::Scroll { x, y, .. } => ("Scroll", Some(*x), Some(*y), None),
            Event::FocusNext { .. } => ("FocusNext", None, None, None),
            Event::Activate => ("Activate", None, None, None),
            Event::Text(_) => ("Text", None, None, None),
            Event::Backspace => ("Backspace", None, None, None),
            Event::PointerLeave => ("PointerLeave", None, None, None),
            Event::Edit { .. } => ("Edit", None, None, None),
            Event::Ime(_) => ("Ime", None, None, None),
            Event::ImeFor { id, .. } => ("ImeFor", None, None, Some(id.as_str())),
            Event::InputValue { .. } => ("InputValue", None, None, None),
            Event::Select { .. } => ("Select", None, None, None),
            Event::Blur => ("Blur", None, None, None),
            Event::Cancel => ("Cancel", None, None, None),
            Event::SetValue { id, .. } => ("SetValue", None, None, Some(id.as_str())),
        };
        let span = self.trace_input(name, id, x, y);
        self.trace_event_source(event);
        span
    }
    fn trace_event_source(&mut self, event: &Event) {
        // Moved out and back so the lookups below can borrow `self`.
        let Some(mut trace) = self.trace.take() else {
            return;
        };
        let focused = self.focused.as_deref();
        let focused_role = self
            .focused_node()
            .and_then(|id| self.dom.get(id).ok())
            .map(|n| n.role());
        let source = match event {
            Event::PointerDown { x, y } => self
                .hit(*x, *y)
                .and_then(|id| self.dom.get(id).ok())
                .filter(|n| n.role() == Role::Slider)
                .and_then(|n| n.stable_id())
                .map(|id| (id, Handler::Value)),
            // A range drag follows the focused slider.
            Event::PointerMove { .. } | Event::PointerUp { .. } if self.range_drag.is_some() => {
                focused.map(|id| (id, Handler::Value))
            }
            Event::PointerUp { .. } if self.modal_outside => self.trace_scope(),
            Event::Cancel => self.trace_scope(),
            Event::Edit { .. } if focused_role.is_some_and(|r| r != Role::TextInput) => {
                focused.map(|id| (id, Handler::Value))
            }
            Event::Text(_)
            | Event::Backspace
            | Event::Edit { .. }
            | Event::Ime(_)
            | Event::InputValue { .. }
            | Event::Select { .. }
            | Event::Blur => focused.map(|id| (id, Handler::Input)),
            Event::ImeFor { id, .. } => Some((id.as_str(), Handler::Input)),
            Event::SetValue { id, .. } => Some((id.as_str(), Handler::Value)),
            _ => None,
        };
        if let Some((id, handler)) = source {
            trace.set_source(id, handler);
        }
        self.trace = Some(trace);
    }
    /// The open modal panel: the in-scope node with a dismiss binding.
    /// Panels open in document order, so the last one is the active scope.
    fn trace_scope(&self) -> Option<(&str, Handler)> {
        self.bindings
            .iter()
            .filter(|(id, b)| b.dismiss.is_some() && self.in_scope(**id))
            .filter_map(|(id, _)| self.dom.get(*id).ok()?.stable_id())
            .next_back()
            .map(|id| (id, Handler::Dismiss))
    }
    /// Input hook for `Runtime::key` and `Runtime::activate`. Nested calls
    /// write no record.
    pub(crate) fn trace_input(
        &mut self,
        event: &'static str,
        id: Option<&str>,
        x: Option<f32>,
        y: Option<f32>,
    ) -> Option<InputSpan> {
        let trace = self.trace.as_mut()?;
        trace.depth += 1;
        if trace.depth > 1 {
            return Some(InputSpan(None));
        }
        let t = trace.now();
        let seq = trace.push(t, Data::Input { event, x, y }, id, None);
        Some(InputSpan(Some((seq, t))))
    }
    /// Note the binding that the next message comes from.
    pub(crate) fn trace_source(&mut self, id: &str, handler: Handler) {
        if let Some(trace) = self.trace.as_mut() {
            trace.set_source(id, handler);
        }
    }
    pub(crate) fn trace_input_end<T>(
        &mut self,
        span: Option<InputSpan>,
        result: &Result<T, AppError>,
    ) {
        let (Some(span), Some(trace)) = (span, self.trace.as_mut()) else {
            return;
        };
        trace.depth = trace.depth.saturating_sub(1);
        if trace.depth > 0 {
            return;
        }
        trace.pending_handler = None;
        if let Some((seq, t)) = span.0 {
            let end = trace.now();
            trace.set_dur(seq, end - t);
        }
        if let Err(error) = result {
            trace.error(error);
        }
    }
    /// Message hook in `flush`. Returns the update start time.
    pub(crate) fn trace_message(&mut self, message: &A::Message) -> Option<f64> {
        let trace = self.trace.as_mut()?;
        let text = self.app.describe(message);
        let t = trace.now();
        let ordinal = trace.ordinal;
        trace.ordinal += 1;
        // The first message after an input hook takes its source.
        let handler = trace.pending_handler.take();
        // Moving the buffer out and back does not allocate.
        let pending = std::mem::take(&mut trace.pending);
        let id = handler.map(|_| pending.as_str());
        let seq = trace.push(t, Data::Message { ordinal }, id, handler);
        trace.pending = pending;
        trace.slot(seq).expect("just written").text = text;
        Some(trace.now())
    }
    /// Update hook in `flush`, after `Application::update` returns.
    pub(crate) fn trace_update(&mut self, start: Option<f64>) {
        self.trace_span(start, Data::Update);
    }
    /// Write a record from `start` to now.
    pub(crate) fn trace_span(&mut self, start: Option<f64>, data: Data) {
        let (Some(t), Some(trace)) = (start, self.trace.as_mut()) else {
            return;
        };
        let end = trace.now();
        let seq = trace.push(t, data, None, None);
        trace.set_dur(seq, end - t);
    }
    /// Paint hook in `render`. Closes the frame.
    pub(crate) fn trace_paint(&mut self, start: Option<f64>, ops: usize) {
        self.trace_span(start, Data::Paint { ops: ops as u32 });
        if let Some(trace) = self.trace.as_mut() {
            trace.frame += 1;
            trace.ordinal = 0;
        }
    }
    /// Error hook for `flush` and `render` outside an input call.
    pub(crate) fn trace_error<T>(&mut self, result: &Result<T, AppError>) {
        if let (Err(error), Some(trace)) = (result, self.trace.as_mut()) {
            if trace.depth == 0 {
                trace.error(error);
            }
        }
    }
}
