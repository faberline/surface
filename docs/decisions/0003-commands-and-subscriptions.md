# 0003: Tasks, timers and subscriptions without a runtime dependency

Status: Proposed (waiting for the owner's decision)
Date: 2026-11-14
Issue: #123 (egui-gap wave 1, design only)

This record proposes how an app asks Surface to run work and get a message
back. It is a design. No code changes with it. The owner decides before any
implementation issue is filed. Line numbers below are from `origin/main` at
`a273b50`.

## Context

### What exists

- `Application::update(&mut self, message)` returns nothing
  (`crates/surface-app/src/lib.rs:107`). The trait is at `:102`. Its message
  type is `Clone + 'static` and not `Send` (`:103`).
- `Application::wake(&mut self)` is the hook for work from other threads. It
  carries no message (`crates/surface-app/src/lib.rs:108-112`).
- `Queue<M>` is an `Rc` queue on the event loop thread. It is not `Send`
  (`crates/surface-app/src/lib.rs:199-201`). `Queue::post` appends a message
  that runs after the current `update`, in order, and never re-enters
  `update` (`:207-213`).
- `Runtime::apply` posts and flushes (`:291-294`). `Runtime::wake` calls
  `Application::wake`, then flushes (`:297-300`). `Runtime::flush` drains the
  queue, then rebuilds the view once (`:318-325`).
- `WakeHandle` is `Clone + Send + Sync`. Many calls coalesce into one pending
  event. A call made during `wake` schedules one more
  (`crates/surface-platform/src/wake.rs:12-41`).
- `run_with_wake` gives the app a `WakeHandle` before the loop starts
  (`crates/surface-platform/src/lib.rs:666-678`). The host runs one
  `Runtime::wake` per coalesced wake and redraws once (`:255-266`).
- Time is injected. `Runtime::set_time(now_ms)` takes `f64` milliseconds and
  never moves back (`crates/surface-app/src/motion.rs:384-389`).
  `needs_frame` asks the host for another frame (`:399-401`). No `Instant`
  is used in `surface-app`; its dependencies are `surface`, `thiserror`,
  `unicode-segmentation`, `serde`, `serde_json` and an optional
  `jet-bridge-protocol` (`crates/surface-app/Cargo.toml:11-17`).
- The host owns the clock. It holds an `Instant` on native and reads
  `performance.now()` on the web (`crates/surface-platform/src/motion.rs:7-31`).
  It calls `set_time` only before a render (`crates/surface-platform/src/lib.rs:353-355`).
- The host never sets a `ControlFlow`. `about_to_wait` only flushes a held
  web key and checks for an app close (`crates/surface-platform/src/lib.rs:504-509`).
  So the loop sleeps until an OS event or a wake.
- The README says async commands need a separate design (`README.md:179`).
  The roadmap lists them under "Component runtime" (`docs/roadmap.md:14`)
  and as item 6 of wave 1 (`docs/roadmap.md:44`).

### The gap

An app cannot say "run this, then send me a message". It cannot say "send
me this message in 500 ms", or "send me a tick every second while this
panel is open". Every app rebuilds this by hand from a thread, shared state,
`WakeHandle::wake` and `Application::wake`. Nothing cancels the work when
the state no longer needs it. Tests cannot drive it with a fake clock.

### How the first consumer works around it

The infera desktop app (infera PR 25, read only) shows the cost:

- **Repaint timer.** Its UI-agnostic core returns `Effect::RepaintAfter`.
  The Surface front end handles it with its own timer thread (`timer.rs`),
  which calls `WakeHandle::wake` when the time is up.
- **Streaming.** The core holds a wake callback (`AppState::set_wake`).
  A worker calls `WakeHandle::wake`; `Application::wake` sends `Msg::Wake`
  to the core, which drains the stream state.
- **SIGTERM.** A signal handler writes a pipe. Another thread reads the
  pipe and wakes the event loop. The app then takes the same path as the
  close button.

Each of these is a thread plus shared state plus a wake. None of them can be
cancelled from `update`, and none of them is visible to a test without real
time.

## Decision requested

Adopt the design below as the plan for async work in Surface. It adds three
things to `surface-app` and one host loop change to `surface-platform`:

1. A `Task<M>` value that `update` can return. A task finishes with zero or
   more messages.
2. One-shot timers as tasks, repeating timers as subscriptions, both on the
   injected clock.
3. Subscriptions that run while the app state asks for them.
4. A host that wakes for tasks and sleeps until the next timer deadline.

`surface-app` gets no new dependency. It polls futures itself with
`std::task`. The host gives it a way to wake the loop. Heavy or blocking
work runs on an executor or thread that the app chooses.

## Design

### 1. `Task<M>`

`Task` is a value. Building one does nothing. The runtime starts it after
the `update` that returned it.

```rust
/// Work that ends with messages for `update`. Inert until the runtime
/// receives it. Not `Send`: it lives on the event loop thread.
#[must_use]
pub struct Task<M> { /* private: Vec<Op<M>> */ }

/// The name of a task or subscription. Same key, same work.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(std::borrow::Cow<'static, str>);

impl<M: 'static> Task<M> {
    /// No work.
    pub fn none() -> Self;
    /// Deliver `message` after the current update, like `Queue::post`.
    pub fn done(message: M) -> Self;
    /// Deliver `message` once, `delay_ms` after the current clock time.
    pub fn after(delay_ms: f64, message: M) -> Self;
    /// Poll `future` on the event loop thread; deliver its output.
    /// The future may be `!Send`. It must not block.
    pub fn future(future: impl Future<Output = M> + 'static) -> Self;
    /// Run several tasks. Their messages keep the order of `tasks`
    /// only where rule O3 below says so.
    pub fn batch(tasks: impl IntoIterator<Item = Self>) -> Self;
    /// Name this task. Starting a task with a key that is still running
    /// cancels the old one first.
    pub fn keyed(self, key: impl Into<Key>) -> Self;
    /// Cancel the running task with this key. No-op if none.
    pub fn cancel(key: impl Into<Key>) -> Self;
    /// Change the message type, for child components.
    pub fn map<N: 'static>(self, f: impl Fn(M) -> N + 'static) -> Task<N>;
}
```

`Task::future` is the general case. A future that waits on a channel fed by
another thread covers blocking work without a runtime in `surface-app`:

```rust
// App code, native. The app picks the thread or executor.
let (tx, rx) = oneshot::channel(); // any runtime-free oneshot
std::thread::spawn(move || tx.send(load_file(path)));
Task::future(async move { Msg::Loaded(rx.await.ok()) }).keyed("load")
```

A runtime-free oneshot is a small type. Surface may ship one (open question
Q7).

### 2. Timers

- **One-shot:** `Task::after(delay_ms, message)`. The deadline is
  `now + delay_ms` on the injected clock. Give it a key to restart or cancel
  it, for example a debounce: `Task::after(300., Msg::Search).keyed("search")`.
- **Repeating:** `Subscriptions::every(key, period_ms, f)` (section 4). A
  repeating timer has an owner: the state that asks for it. When the state
  stops asking, the timer stops. There is no `Task::every`, so a repeating
  timer cannot leak.
- **Clock.** Timers use the time from `Runtime::set_time`. `surface-app`
  never reads a clock. Tests drive timers with `set_time` alone.
- **No burst.** A repeating timer that is late (sleep, a throttled
  background tab) fires once. Its next deadline is `now + period`, not a
  catch-up series.
- `delay_ms` that is negative or not finite counts as 0. `period_ms` must be
  finite and at least 1; a smaller value is raised to 1.

### 3. Cancellation

Rust drop is the cancel. A cancelled task's future is dropped on the event
loop thread. Its output is never delivered.

| Event | Effect |
| --- | --- |
| A task starts with a key that is running | The old task is dropped first. Latest wins. |
| `Task::cancel(key)` | The task with that key is dropped. Unknown key: no-op. |
| A pending `Task::after` is cancelled | It does not fire. |
| A subscription key leaves `subscriptions()` | That subscription is dropped (section 4). |
| A task ends | Its key is free again. |
| The window closes or `Runtime` drops | All tasks and subscriptions drop before the app state. |

Unkeyed tasks cannot be cancelled. They end on their own or when the
runtime drops. Work that may need a cancel gets a key.

A thread that a future waits on is not stopped by the drop. The drop closes
the channel, so the thread sees a closed sender and can stop. A cooperative
stop is the app's job, as it is today.

### 4. Subscriptions

A subscription is work that lives while the state asks for it. The runtime
asks the app for its subscriptions after each flush and compares keys.

```rust
pub struct Subscriptions<M> { /* private: Vec<(Key, Sub<M>)> */ }

impl<M: 'static> Subscriptions<M> {
    pub fn none() -> Self;
    pub fn batch(items: impl IntoIterator<Item = Self>) -> Self;
    /// A message every `period_ms`, built from the clock time.
    pub fn every(key: impl Into<Key>, period_ms: f64,
                 f: impl Fn(f64) -> M + 'static) -> Self;
    /// A producer future. It sends messages through `emit` until it ends
    /// or the subscription stops. Runs on the event loop thread.
    pub fn run<F: Future<Output = ()> + 'static>(
        key: impl Into<Key>, start: impl FnOnce(Emit<M>) -> F + 'static) -> Self;
    /// A source fed from any thread or OS callback. `start` runs once when
    /// the key appears; the returned guard drops when the key goes away.
    /// `T` crosses threads; `map` turns it into a message on the event loop.
    pub fn external<T: Send + 'static, G: 'static>(
        key: impl Into<Key>,
        start: impl FnOnce(Sender<T>) -> G + 'static,
        map: impl Fn(T) -> M + 'static) -> Self;
    pub fn map<N: 'static>(self, f: impl Fn(M) -> N + 'static) -> Subscriptions<N>;
}

/// Event-loop-side sender for `run`. Not `Send`.
pub struct Emit<M> { /* Rc queue + task waker */ }
/// Cross-thread sender for `external`. `Clone + Send + Sync`.
/// `send` returns `Err` once the subscription stopped.
pub struct Sender<T> { /* Arc<Mutex<VecDeque<T>>> + notifier */ }
```

Diff rules after each flush:

- A key that is new starts its subscription.
- A key that was there and still is keeps running. It is not restarted.
  Its `f` or `map` closure is replaced by the new one, so captured values
  stay current.
- A key that is gone is dropped. Its guard drops, its producer future drops,
  its `Sender` returns `Err` from then on.
- Two entries with one key in one call: the first wins. Debug builds panic.

### 5. Trait change, additive

`Application` gets two defaulted methods. `update` gets a default empty
body. Nothing else changes.

```rust
pub trait Application: 'static {
    type Message: Clone + 'static;
    fn title(&self) -> &str { "Surface" }
    /// Unchanged signature. Now has a default body, so an app that
    /// implements only `update_task` does not write an empty `update`.
    fn update(&mut self, message: Self::Message) {}
    /// New. The runtime always calls this one. The default runs `update`
    /// and starts no work, which is exactly today's behavior.
    fn update_task(&mut self, message: Self::Message) -> Task<Self::Message> {
        self.update(message);
        Task::none()
    }
    /// New. Called after each flush. The default asks for nothing.
    fn subscriptions(&self) -> Subscriptions<Self::Message> {
        Subscriptions::none()
    }
    fn wake(&mut self) {}
    fn close_requested(&mut self) -> CloseDecision { CloseDecision::Allow }
    fn view(&self) -> View<Self::Message>;
}
```

Why existing impls compile unchanged:

- An impl that defines `update` still overrides a trait method with the
  same signature. Adding a default body to a required method is not a
  breaking change.
- `update_task` and `subscriptions` are new defaulted methods. An impl that
  does not name them gets the defaults.
- The default `update_task` calls `update` and returns `Task::none()`. The
  runtime then behaves as today: no task, no subscription, no timer.
- `Runtime::flush` changes from `self.app.update(message)` (`lib.rs:322`)
  to `let task = self.app.update_task(message);` plus starting `task`.
  `Runtime::new`, `apply`, `wake`, `queue`, `request_close` keep their
  signatures. `Queue::post` keeps its meaning.
- Name clash risk: an app with its own inherent method named `update_task`
  or `subscriptions` still compiles; inherent methods win on method-call
  syntax. Only a fully qualified call would change.

The cost of the default `update` body: an app that implements neither
method compiles and ignores all messages. Open question Q2 covers this.

`View`, `Queue`, the jet-bridge path and the Core-compatible APIs named in
`docs/migration.md` do not change.

### 6. Runtime and host contract

`surface-app` gains these `Runtime` methods. They use only `std`.

```rust
impl<A: Application> Runtime<A> {
    /// The host's way to wake the loop from a task waker or a `Sender`.
    /// Without it (headless tests), work is polled on each flush only.
    pub fn set_notifier(&mut self, notify: Arc<dyn Fn() + Send + Sync>);
    /// Set the clock (as `set_time`), fire due timers, poll woken tasks
    /// and subscriptions, run their messages, then rebuild once.
    /// Does not call `Application::wake`.
    pub fn advance(&mut self, now_ms: f64) -> Result<(), AppError>;
    /// The earliest timer deadline on the injected clock, if any.
    pub fn next_deadline(&self) -> Option<f64>;
}
```

Task wakers are built with `std::task::Wake` on an `Arc` that holds a ready
flag per task and the notifier. A wake sets the flag and calls the
notifier. The runtime polls only flagged tasks. This is the whole
executor: a single-threaded poll loop on the event loop thread. It is
enough because blocking work never runs inside it.

Before the first `set_time`, there is no clock. A timer created then is
anchored at the first `set_time` value. The host calls `set_time` once at
start (see the host mappings), so this only matters in headless tests.

### 7. Ordering rules

One flush, as proposed (`Runtime::flush`, `wake`, `apply` and `advance` all
end in it):

1. **O1. Queue first.** Drain the `Queue` in order. Each message runs
   `update_task`.
2. **O2. Posts before tasks.** Messages that an `update` posted with
   `Queue::post` run before the `Task::done` messages that the same update
   returned. Both go to the back of the queue, in that order, after
   messages that were already queued. `Task::done(m)` is exactly
   `Queue::post(m)` made after `update` returns.
3. **O3. Batch order.** `Task::batch([done(a), done(b)])` delivers `a` then
   `b`. Futures and timers in a batch deliver in completion order.
4. **O4. Then work.** When the queue is empty: fire due timers in
   `(deadline, start order)` order, then poll flagged tasks in start order,
   then drain subscription senders in key order. Each output is posted to
   the queue. Go back to O1. Stop when the queue is empty and nothing is
   flagged or due.
5. **O5. New tasks poll once in the same flush.** A task started in this
   flush is flagged. So `Task::future(async { m })` delivers `m` in the
   same flush, after the queue drains. `Task::after(0., m)` also fires in
   the same flush.
6. **O6. Subscriptions diff once.** After the loop ends, call
   `subscriptions()` once and apply the diff. A new `every` gets its first
   deadline at `now + period`; it never fires in the flush that started it.
   A new `run` or `external` is polled at the next flush.
7. **O7. One rebuild.** The view rebuilds once at the end, as today.
8. **O8. `wake` first.** `Runtime::wake` calls `Application::wake` before
   the flush, so the app drains its own shared state before task results.
   A task waker does not call `Application::wake`.
9. **O9. Frames.** `render` never runs `update`, a task or a timer. Timers
   are not aligned to frames. A timer due during a transition fires on the
   next `advance`, which the host calls before its next draw.
10. **O10. Close.** After the host decides to exit, no task or timer
    message is delivered. `Queue::close` from a task's message works as
    from any other message.

An endless chain (`done` that returns `done`, or `after(0)` that returns
`after(0)`) never yields to the host. That is the same as a `Queue::post`
loop today. The runtime does not cap it. Open question Q9.

### 8. What the trace recorder sees (issue #125)

#125 adds `Input`, `Message`, `Update`, `Rebuild`, `Layout`, `Paint`,
`Present` and `Error` records. A `Message` record has a source; it is null
for a `Queue` post from outside a binding. Under this design:

- A message from a task, timer or subscription gets a source:
  `{"kind":"task"|"timer"|"subscription","key":<key or null>,"id":<start seq>}`.
- A later issue adds record kinds `Task` (`start`, `cancel`, `end`) and
  `Subscription` (`start`, `stop`). They carry the key and start sequence.
  #125 does not need to change for this design; the new kinds come after it.
- `t_ms` comes from the same injected clock, so timer firings are
  reproducible: replaying the `set_time` values replays every timer
  message in the same order.
- Future completions depend on the outside world. The trace records the
  delivered message, so a replay can inject it with `Runtime::apply` in the
  same position.
- One `advance` gives `Message`/`Update` records for each fired timer, then
  one `Rebuild`, all with the frame id of the next render.

### 9. Native host mapping (`surface-platform`, not wasm32)

- **Notifier.** `run_with_wake` passes the runtime a notifier that sends a
  payload-free `UserEvent::Work` through the existing `SendProxy`, behind
  its own coalescing flag like `WakeHandle`. On that event the host calls
  `runtime.advance(clock.now_ms())`, then requests a redraw. `UserEvent::Wake`
  keeps calling `Runtime::wake`.
- **Start time.** The host calls `set_time(clock.now_ms())` once before the
  first update, in `run_with_wake`.
- **Timers.** In `about_to_wait` the host reads `next_deadline()`. If there
  is one, it sets `ControlFlow::WaitUntil(clock.start + deadline)`;
  otherwise `ControlFlow::Wait`. On `new_events` with
  `StartCause::ResumeTimeReached` it calls `advance(clock.now_ms())`.
- **Clock.** The existing `motion::Clock` (`Instant` in `surface-platform`).
  `surface-app` still sees only `f64` milliseconds.
- **Blocking work.** Not in the poll loop. The app uses `std::thread` or its
  own runtime (tokio, smol) and returns `Task::future` over a channel.
  `surface-platform` may add `spawn_blocking(f) -> impl Future` built on
  `std::thread` (Q7).

### 10. wasm32 host mapping

- **Notifier.** The same `UserEvent::Work` through the proxy. The web
  `WakeHandle` already works this way (`crates/surface-platform/src/lib.rs:132-142`).
- **Futures.** Polled by `Runtime` on the event loop, so
  `wasm_bindgen_futures::JsFuture` (a JS promise) works with `Task::future`
  as is. No `spawn_local` is needed for tasks. `surface-app` still has no
  wasm dependency.
- **Timers.** winit 0.30 on the web maps `ControlFlow::WaitUntil` to a
  browser timeout. The clock is `performance.now()` (`motion.rs:28`).
  Browsers throttle timeouts in background tabs; the no-burst rule keeps
  repeating timers sane there.
- **Blocking work.** There is none on the web main thread. Web workers are
  out of scope.
- **Scheduling.** Decision 0001 D-B limits web work until D1 reports. The
  web mapping is a separate issue that waits for D-B. Native ships first.
  The `surface-app` part builds for wasm32 from day one, because it uses
  only `std::task` and `Rc`.

### 11. The infera workarounds under this design

| Today | Under this design |
| --- | --- |
| `Effect::RepaintAfter(d)` handled by a timer thread (`timer.rs`) and `WakeHandle::wake` | The Surface front end maps the effect in `update_task`: `Task::after(d_ms, Msg::Repaint).keyed("repaint")`. The timer thread goes away. Latest wins (Q4). |
| Streaming: worker → `AppState::set_wake` → `WakeHandle::wake` → `Application::wake` → `Msg::Wake` | Still valid; `wake` stays. Optionally `Subscriptions::external("stream", ...)` with the worker holding a `Sender<Chunk>`. The stream stops when the state no longer lists the key, for example after Stop. |
| SIGTERM: handler writes a pipe, a thread reads it and wakes the loop | The handler and pipe stay in the app; Surface installs no signal handlers. The reader thread becomes the `start` of `Subscriptions::external("sigterm", ...)`: it sends `()` on the `Sender`, `map` makes `Msg::Terminate`, and `update_task` takes the close path and calls `Queue::close`. The hand-written wake plumbing goes away. |

The infera core stays UI-agnostic. Only its Surface front end changes. That
change is in the infera repo and goes through its desk, not this record.

## Alternatives rejected

### A. A mandatory tokio dependency in `surface-app`

- Every app, including web and headless tests, pulls in tokio's tree.
- tokio's runtime and timers do not serve the wasm32 browser target in the
  same way; the web host would need a second model anyway.
- `tokio::time` reads `Instant`. That breaks the injected clock, so tests
  and trace replay lose determinism.
- A multi-thread runtime wants `Send` futures. Surface messages and views
  use `Rc` and are not `Send`.
- An app with its own tokio version or a different runtime (smol,
  async-std) would get two runtimes.
- Under this design a tokio app still works: it keeps its runtime and hands
  results back with `Task::future` over a channel.

### B. Change `update` to return a task

`fn update(&mut self, m) -> Task<M>` is cleaner, but it breaks every
`Application` impl, the showcase, infera and the APIs Core imports.
AGENTS.md requires those APIs to stay compatible until the migration in
`docs/migration.md` is done. Rejected for now; it could be a later major
change once the migration lands.

### C. A generic executor parameter (`Runtime<A, E: Executor>`)

Generics spread into `run_with_wake`, the bridge and devtools. Ordering and
cancellation would depend on the executor, so the rules in section 7 could
not be tested in `surface-app` alone. Rejected.

### D. A command handle passed into `update` (`update(&mut self, m, cx)`)

Changing the signature is breaking. Adding it as a new method works, but a
returned `Task` is a plain value: tests can call `update_task` and inspect
the result without a runtime. A context also invites holding it past the
call. Rejected in favor of a return value.

### E. Keep the status quo and document the recipe

`WakeHandle` plus `wake` already works. It has no cancellation, no fake
clock for tests, no trace source and costs each app a thread per timer.
infera shows the cost. Rejected.

### F. `Task::every` for repeating timers

A repeating task has no owner and outlives the state that wanted it unless
the app remembers to cancel it. A subscription stops on its own. Rejected.

## Open questions for the owner

Each has a recommended default. The design above uses the defaults.

| # | Question | Recommended default |
| --- | --- | --- |
| Q1 | Name the type `Task` or `Command`? | `Task`. "Command" is already the macOS key in `Modifiers` docs (`docs/architecture.md:53`) and in the jet-bridge "command set". |
| Q2 | Give `update` a default empty body? | Yes. Without it, apps that use `update_task` write an empty `update`. The risk (an app that implements neither) is visible at once: nothing reacts. |
| Q3 | Can unkeyed tasks be cancelled (handle returned)? | No. A key is the only cancel. A handle needs the runtime to exist when `update_task` returns, which makes tests harder. |
| Q4 | Same key while running: replace or ignore the new one? | Replace (latest wins). It fits debounce and restart. "Keep the running one" can be added later as `keyed_once`. |
| Q5 | Should a task waker also call `Application::wake`? | No. A separate `UserEvent::Work`. `wake` stays for the app's own shared state, and traces stay clean. |
| Q6 | Repeating timer after a long gap: one fire or catch-up? | One fire, next deadline `now + period`. |
| Q7 | Ship a runtime-free oneshot channel and a native `spawn_blocking`? | Yes, small: `surface_app::oneshot` (std only) and `surface_platform::spawn_blocking` (native only, `std::thread`). |
| Q8 | Depend on `futures-core` for a `Stream` subscription? | No for now. `run` with `Emit` covers streams. Add a `Stream` adapter behind a feature if a consumer asks. |
| Q9 | Cap the work loop in one flush (section 7)? | No cap, same as `Queue::post` today. Debug builds warn after 10 000 rounds. |
| Q10 | A frame subscription (`on_frame` with the frame time)? | Out of scope. Motion already has `needs_frame`. Revisit with layout animation. |
| Q11 | When does implementation start? | After O1 (#125) lands, so the trace sources go in with the tasks. Native first; the web mapping waits for D-B. |

## Implementation issues that would follow

Not filed. Filed only after the owner decides.

1. **surface-app: `Task` type and `update_task`.** `Task`, `Key`,
   `done`/`after`/`future`/`batch`/`keyed`/`cancel`/`map`, the default
   methods, the poll loop and timer table, `advance`, `next_deadline`,
   `set_notifier`; headless tests on `set_time` for every rule in section 7.
2. **surface-app: subscriptions.** `Subscriptions`, `every`, `run`,
   `external`, `Emit`, `Sender`, the key diff and drop rules; headless tests.
3. **surface-app: oneshot channel (Q7).** A `std`-only oneshot for
   `Task::future` over threads.
4. **surface-platform native: work wake and timers.** `UserEvent::Work`,
   start-time `set_time`, `ControlFlow::WaitUntil` from `next_deadline`,
   `ResumeTimeReached` → `advance`, `spawn_blocking`; a macOS smoke test
   with a real timer.
5. **devtools: task, timer and subscription trace records.** Message
   sources and the `Task`/`Subscription` record kinds on top of #125.
6. **docs and showcase.** Update `docs/architecture.md`, the README note
   (`README.md:179`) and `docs/roadmap.md`; a showcase example with a
   debounce, a clock tick and a cancellable load.
7. **surface-platform web: work wake and timers.** The wasm32 mapping in
   section 10. Waits for D-B.

The infera front end change (section 11) is a consumer change in the infera
repo. It is requested through the desks, not filed here.
