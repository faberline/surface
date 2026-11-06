//! Compile-only contract for the `surface` and `ui-runtime` API that Jet and
//! Core consume.
//!
//! Jet's `jet-wasm` re-exports the `surface` types and `ui_runtime::*`.
//! Core's compatibility packages re-export both crate roots. Each item below
//! is one they name, pinned to the signature they rely on. This file stops
//! compiling when an item is renamed or removed, a signature changes, a
//! hook's generic parameter order changes, an `Element` or `SurfaceNodeKind`
//! variant is added, or a public snapshot or debug field changes. Change it
//! only together with a consumer migration (see `docs/migration.md`).
//!
//! The list was checked against Jet `a584413`: `wasm/src/lib.rs`,
//! `wasm/src/react/{mod,dom_app,webgpu_app}.rs`, `wasm/src/debug/mod.rs`,
//! `wasm/tests` and the code that `src/tsx_to_rust/emit.rs` generates.
//! `Props` and `Component` are pinned in their post-41c352e form: private
//! fields, read through builders and getters.
//!
//! Hooks need a render in progress, so nothing here runs. The test only
//! takes each contract function's address.

use std::any::Any;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::rc::Rc;

use surface::{
    Callback, Component, ComponentFn, Element, Props, SurfaceNode, SurfaceNodeKind, SurfaceProps,
    SurfaceRect, SurfaceSnapshot,
};
#[cfg(feature = "debug")]
use ui_runtime::{debug_snapshot_fibers, debug_snapshot_hooks, DebugFiberMeta, DebugHookSummary};
use ui_runtime::{
    hash_dep, mount, set_update_scheduler, use_callback, use_effect_once, use_memo, use_reducer,
    use_ref, use_state, DispatchHandle, FiberId, MemoDepHash, MountHandle, RefHandle, StateSetter,
};

/// The scheduler slot Jet's WebGPU loop sets and clears.
type UpdateScheduler = Option<Rc<dyn Fn()>>;

fn cloned<T: Clone>() {}
fn debugged<T: Clone + Debug>() {}
fn wire<T: Clone + Debug + PartialEq>() {}
fn id_value<T: Copy + Debug + Default + Eq + Hash>() {}

/// Jet's generated code passes `bool` and arbitrary expressions here.
fn from_any_display<N: Display>(n: N) -> Element {
    Element::from_number(n)
}

/// `Element`: the five variants that Jet's adapters match exhaustively, the
/// constructors generated code calls, and the queries Jet's tests use.
fn element_contract(element: &Element) {
    match element {
        Element::Intrinsic {
            tag,
            props,
            children,
        } => {
            let _: &&'static str = tag;
            let _: &Props = props;
            let _: &Vec<Element> = children;
        }
        Element::Text(text) => {
            let _: &String = text;
        }
        Element::Component(component) => {
            let _: &Component = component;
        }
        Element::Empty => {}
        Element::Fragment(children) => {
            let _: &Vec<Element> = children;
        }
    }
    let _: fn(&'static str, Props, Vec<Element>) -> Element = Element::intrinsic;
    let _: fn(String) -> Element = Element::Text;
    let _: fn(Component) -> Element = Element::Component;
    let _: fn(Vec<Element>) -> Element = Element::Fragment;
    let _: fn(String) -> Element = Element::text;
    let _: fn(&'static str) -> Element = Element::text;
    let _: fn(i64) -> Element = Element::from_number::<i64>;
    let _: fn(f64) -> Element = Element::from_number::<f64>;
    let _: fn(bool) -> Element = Element::from_number::<bool>;
    let _: Element = from_any_display(true);
    let _: Element = from_any_display(1.5f32 * 2.0);
    let _: fn(&Element) -> String = Element::text_content;
    let _: fn(&Element, &str) -> Option<Callback<()>> = Element::find_on_click;
    let _: fn(&Element) -> SurfaceSnapshot = Element::surface_snapshot;
    cloned::<Element>();
}

/// `Props`: every `with_*` builder and every getter.
fn props_contract(props: &Props) {
    let _: fn() -> Props = Props::default;
    let _: fn(Props, String) -> Props = Props::with_id;
    let _: fn(Props, String) -> Props = Props::with_class_name;
    let _: fn(Props, String) -> Props = Props::with_style;
    let _: fn(Props, String) -> Props = Props::with_value;
    let _: fn(Props, String) -> Props = Props::with_input_type;
    let _: fn(Props, String) -> Props = Props::with_placeholder;
    let _: fn(Props, bool) -> Props = Props::with_checked;
    let _: fn(Props, String) -> Props = Props::with_aria_label;
    let _: fn(Props, String) -> Props = Props::with_html_for;
    let _: fn(Props, bool) -> Props = Props::with_disabled;
    let _: fn(Props, Callback<()>) -> Props = Props::with_on_click;
    let _: fn(Props, Callback<String>) -> Props = Props::with_on_change;
    let _: fn(Props, Callback<bool>) -> Props = Props::with_on_checked_change;

    let _: fn(&Props) -> Option<&str> = Props::id;
    let _: fn(&Props) -> Option<&str> = Props::class_name;
    let _: fn(&Props) -> Option<&str> = Props::style;
    let _: fn(&Props) -> Option<&str> = Props::value;
    let _: fn(&Props) -> Option<&str> = Props::input_type;
    let _: fn(&Props) -> Option<&str> = Props::placeholder;
    let _: fn(&Props) -> Option<bool> = Props::checked;
    let _: fn(&Props) -> Option<&str> = Props::aria_label;
    let _: fn(&Props) -> Option<&str> = Props::html_for;
    let _: fn(&Props) -> bool = Props::disabled;
    let _: fn(&Props) -> Option<&Callback<()>> = Props::on_click;
    let _: fn(&Props) -> Option<&Callback<String>> = Props::on_change;
    let _: fn(&Props) -> Option<&Callback<bool>> = Props::on_checked_change;
    debugged::<Props>();

    // The shape Jet's generated code and DOM adapter use.
    let _: Element = Element::intrinsic(
        "input",
        Props::default()
            .with_id("draft")
            .with_class_name("field")
            .with_input_type("checkbox".to_string())
            .with_on_click(Callback::new(|()| {}))
            .with_on_change(Callback::new(|_value: String| {}))
            .with_on_checked_change(Callback::new(|_checked: bool| {})),
        vec![Element::text("label")],
    );
    let _: Option<Callback<()>> = props.on_click().cloned();
    let _: Option<Callback<String>> = props.on_change().cloned();
    let _: Option<Callback<bool>> = props.on_checked_change().cloned();
}

/// `Callback<P>` for the three payloads `Props` carries.
fn callback_contract(on_click: &Callback<()>) {
    let _: Callback<String> = Callback::new(|_value: String| {});
    let _: fn(&Callback<()>, ()) = Callback::<()>::call;
    let _: fn(&Callback<String>, String) = Callback::<String>::call;
    let _: fn(&Callback<bool>, bool) = Callback::<bool>::call;
    let _: Callback<()> = on_click.clone();
    debugged::<Callback<()>>();
}

fn render_counter(props: &Rc<dyn Any>) -> Element {
    let _: Option<&i64> = props.downcast_ref::<i64>();
    Element::Empty
}

/// `Component` and `ComponentFn`, as Jet's component factories build them.
fn component_contract() {
    let render: ComponentFn = render_counter;
    let _: fn(&Rc<dyn Any>) -> Element = render;
    let _: fn(&'static str, ComponentFn, Rc<dyn Any>) -> Component = Component::new;
    let _: fn(&Component) -> &'static str = Component::name;
    let _: fn(&Component) -> Element = Component::render;
    let _: Component = Component::new("Counter", render_counter, Rc::new(0i64));
    cloned::<Component>();
}

/// The snapshot wire types: every public field and every node kind.
fn snapshot_contract(snapshot: SurfaceSnapshot) {
    let _: fn(&Element) -> SurfaceSnapshot = SurfaceSnapshot::from_element;
    let _: for<'a> fn(&'a SurfaceSnapshot, &str) -> Option<&'a SurfaceNode> =
        SurfaceSnapshot::find_by_semantic_id;
    let _: SurfaceProps = SurfaceProps::from(&Props::default());
    let _: u32 = SurfaceSnapshot::SCHEMA_VERSION;
    let SurfaceSnapshot {
        schema_version,
        nodes,
    } = snapshot;
    let _: u32 = schema_version;
    for node in nodes {
        let SurfaceNode {
            node_id,
            semantic_id,
            parent_id,
            kind,
            tag,
            component,
            role,
            name,
            text,
            props,
            bounds,
        } = node;
        let _: [String; 2] = [node_id, semantic_id];
        let _: [Option<String>; 6] = [parent_id, tag, component, role, name, text];
        match kind {
            SurfaceNodeKind::Element | SurfaceNodeKind::Text | SurfaceNodeKind::Component => {}
        }
        let SurfaceProps {
            id,
            class_name,
            style,
            value,
            input_type,
            placeholder,
            checked,
            aria_label,
            html_for,
            disabled,
            has_on_click,
            has_on_change,
            has_on_checked_change,
        } = props;
        let _: [Option<String>; 8] = [
            id,
            class_name,
            style,
            value,
            input_type,
            placeholder,
            aria_label,
            html_for,
        ];
        let _: Option<bool> = checked;
        let _: [bool; 4] = [disabled, has_on_click, has_on_change, has_on_checked_change];
        let _: Option<SurfaceRect> = bounds;
        if let Some(SurfaceRect { x, y, w, h }) = bounds {
            let _: [f32; 4] = [x, y, w, h];
        }
    }
    wire::<SurfaceSnapshot>();
    wire::<SurfaceNode>();
    wire::<SurfaceProps>();
    wire::<SurfaceRect>();
    wire::<SurfaceNodeKind>();
}

/// Mounting, flushing and the renderer's update scheduler.
fn mount_contract(component: Component) {
    let _: fn(Component) -> MountHandle = mount;
    let _: fn(&MountHandle) -> bool = MountHandle::flush;
    let _: fn(&MountHandle) -> Element = MountHandle::snapshot;
    let _: fn(UpdateScheduler) = set_update_scheduler;
    let handle: Rc<MountHandle> = Rc::new(mount(component));
    // Core's standalone compatibility test reads this public field.
    let _: FiberId = handle.fiber_id;
    let _: bool = handle.flush();
    let _: Element = handle.snapshot();
    set_update_scheduler(Some(Rc::new(|| {})));
    set_update_scheduler(None);
}

/// Every `use_*` hook with its handle type, as a component body calls them.
fn hooks_contract() -> Element {
    let _: fn(i64) -> (i64, StateSetter<i64>) = use_state::<i64>;
    let _: fn(&StateSetter<i64>, i64) = StateSetter::<i64>::set;
    let _: fn(&String) -> MemoDepHash = hash_dep::<String>;
    let _: fn(MemoDepHash) -> u64 = |hash| hash;
    // Jet names the generics in this order, e.g. `use_reducer::<i64, Action, _>`.
    type Reducer = fn(&i64, u8) -> i64;
    type Deps = Vec<MemoDepHash>;
    let _: fn(Reducer, i64) -> (i64, DispatchHandle<i64, u8>) = use_reducer::<i64, u8, Reducer>;
    let _: fn(fn() -> i64, Deps) -> i64 = use_memo::<i64, fn() -> i64>;
    let _: fn(fn(()), Deps) -> Callback<()> = use_callback::<(), fn(())>;

    let (count, set_count) = use_state::<i64>(0);
    let _: StateSetter<i64> = set_count.clone();
    let doubled: i64 = use_memo(move || count * 2, vec![hash_dep(&count)]);
    // Generated effects move values out, so the closure is only `FnOnce`.
    let owned = String::new();
    use_effect_once(move || drop(owned));
    let on_click: Callback<()> = use_callback(
        move |()| set_count.set(doubled),
        vec![hash_dep(&count), hash_dep("label")],
    );
    let (total, dispatch): (i64, DispatchHandle<i64, i64>) =
        use_reducer(|state: &i64, delta: i64| state + delta, 0);
    let _: DispatchHandle<i64, i64> = dispatch.clone();
    dispatch.dispatch(total);
    let cell: RefHandle<i64> = use_ref(0i64);
    let _: RefHandle<i64> = cell.clone();
    let _: i64 = cell.current();
    cell.set(1);
    let _: i64 = cell.with_mut(|value| *value + 1);
    Element::intrinsic("button", Props::default().with_on_click(on_click), vec![])
}

/// `FiberId`, which debug clients send back as a raw number.
fn fiber_id_contract() {
    let _: fn(u64) -> FiberId = FiberId::new;
    let _: fn(FiberId) -> u64 = FiberId::get;
    id_value::<FiberId>();
}

/// The `debug` views behind Jet's `JetDebug` handle.
#[cfg(feature = "debug")]
fn debug_contract(handle: &MountHandle, raw_fiber_id: u32) {
    let _: fn() -> Vec<DebugFiberMeta> = debug_snapshot_fibers;
    let _: fn(FiberId) -> Vec<DebugHookSummary> = debug_snapshot_hooks;
    let _: fn(&MountHandle) = MountHandle::mark_root_dirty;
    for fiber in debug_snapshot_fibers() {
        let _: u64 = fiber.id.get();
        let _: usize = fiber.hook_count;
        let _: bool = fiber.dirty;
    }
    for hook in debug_snapshot_hooks(FiberId::new(u64::from(raw_fiber_id))) {
        let _: &'static str = hook.kind;
        let _: Option<&'static str> = hook.type_name;
        let _: Option<serde_json::Value> = hook.value_json.clone();
    }
    handle.mark_root_dirty();
}

#[test]
fn consumer_contract_compiles() {
    let _: fn(&Element) = element_contract;
    let _: fn(&Props) = props_contract;
    let _: fn(&Callback<()>) = callback_contract;
    let _: fn() = component_contract;
    let _: fn(SurfaceSnapshot) = snapshot_contract;
    let _: fn(Component) = mount_contract;
    let _: fn() -> Element = hooks_contract;
    let _: fn() = fiber_id_contract;
    #[cfg(feature = "debug")]
    let _: fn(&MountHandle, u32) = debug_contract;
}
