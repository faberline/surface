//! Allocation overhead of the trace hooks (issue #125, O1). Its own test
//! binary, because it installs a counting global allocator. The source
//! builds with and without the `devtools` feature, and both builds must
//! count exactly `CYCLE_ALLOCS`.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
use surface::{Length, Node, Role, Style, TextMeasurer};
use surface_app::{Application, Event, Runtime, View};

/// Allocations in 100 click+render cycles, measured once on `origin/main`
/// (`a273b50`) without the feature.
const CYCLE_ALLOCS: u64 = 4001;
const CYCLES: usize = 100;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<u64> = const { Cell::new(0) };
}

/// Counts `alloc` and `realloc` calls on the test thread while counting is on.
struct Counting;
fn bump() {
    let _ = COUNTING.try_with(|on| {
        if on.get() {
            let _ = COUNT.try_with(|n| n.set(n.get() + 1));
        }
    });
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        bump();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        bump();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        bump();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Allocations made by `f` on this thread.
fn count(f: impl FnOnce()) -> u64 {
    COUNT.with(|n| n.set(0));
    COUNTING.with(|on| on.set(true));
    f();
    COUNTING.with(|on| on.set(false));
    COUNT.with(Cell::get)
}

#[derive(Clone)]
enum Msg {
    Bump,
}
struct App {
    n: u32,
}
impl Application for App {
    type Message = Msg;
    fn update(&mut self, Msg::Bump: Msg) {
        self.n += 1;
    }
    fn view(&self) -> View<Msg> {
        View::new(Node::new(Role::Window, "Alloc").styled(Style::column())).children([View::new(
            Node::new(Role::Button, "OK")
                .id("ok")
                .styled(Style::default().height(Length::Px(40.))),
        )
        .on_press(Msg::Bump)])
    }
}
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&mut self, text: &str, size: f32, _: Option<f32>) -> (f32, f32) {
        (text.chars().count() as f32 * size, size * 1.5)
    }
}

fn runtime() -> Runtime<App> {
    let mut runtime = Runtime::new(App { n: 0 }).unwrap();
    runtime.render(320., 240., &mut Metrics).unwrap();
    runtime
}

fn cycles(runtime: &mut Runtime<App>) {
    for _ in 0..CYCLES {
        runtime
            .dispatch(Event::PointerDown { x: 10., y: 10. })
            .unwrap();
        runtime
            .dispatch(Event::PointerUp { x: 10., y: 10. })
            .unwrap();
        runtime.render(320., 240., &mut Metrics).unwrap();
    }
}

// Rows 5 and 6: no recorder attached, with and without the feature.
#[test]
fn hooks_add_no_allocations() {
    let mut runtime = runtime();
    let n = count(|| cycles(&mut runtime));
    assert_eq!(runtime.app().n, CYCLES as u32);
    assert_eq!(n, CYCLE_ALLOCS, "allocations in {CYCLES} cycles");
}

/// An attached recorder whose `describe` returns `None` allocates nothing
/// per cycle once its ring exists.
#[cfg(feature = "devtools")]
#[test]
fn attached_recorder_adds_no_allocations_per_cycle() {
    use surface_app::trace::Recorder;
    let mut runtime = runtime();
    let mut now = 0.;
    let recorder = Recorder::new(Box::new(move || {
        now += 1.;
        now
    }));
    let attach = count(|| runtime.attach_trace(recorder));
    assert_eq!(attach, 0, "attach moves the ring in");
    let n = count(|| cycles(&mut runtime));
    assert_eq!(runtime.app().n, CYCLES as u32);
    let trace = runtime.trace().unwrap();
    assert_eq!(trace.len(), CYCLES * 7, "the records were written");
    assert_eq!(n, CYCLE_ALLOCS, "allocations in {CYCLES} cycles");
}
