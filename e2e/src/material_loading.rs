//! Expressive loading indicator through the real headless Chrome WebGPU host.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{
    material_list_corners::{distance, reduced_motion},
    material_time_input::{
        activate, ax_tree, bounds, contains, focus_projected, focused, projected, resize, Raster,
    },
    state, until,
};
use crate::browser::{Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

type Rect = [f64; 4];
type Rgb = [u8; 3];

const IDS: [&str; 2] = ["loading-default", "loading-contained"];
const NAMES: [&str; 2] = ["載入資料", "讀取快取"];
const SHOW: &str = "loading-demo-show";
const BACK: &str = "loading-demo-back";
const START: &str = "loading-demo-run";
const STOP: &str = "loading-demo-stop";
const GRID: usize = 48;
const POSES: [&str; 7] = [
    "SoftBurst",
    "Cookie9Sided",
    "Pentagon",
    "Pill",
    "Sunny",
    "Cookie4Sided",
    "Oval",
];

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(d => d && ({condition}))(s.loading)")).await?;
    tab.settle().await
}

async fn loading(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["loading"].take())
}

async fn choose(tab: &Tab, control: &str) -> Result<()> {
    activate(tab, &format!("loading-demo-{control}")).await
}

async fn open(tab: &Tab) -> Result<()> {
    activate(tab, SHOW).await?;
    ready(tab, "d.visible && d.running && d.size === 48").await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.viewport(960, 2044).await?;
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 2000).await?;
    activate(tab, "section-1").await?;
    until(tab, "s.section === 1").await?;
    reduced_motion(tab, false).await?;
    open(tab).await?;
    resize(tab, 960, 820).await
}

async fn freeze(tab: &Tab) -> Result<()> {
    // Change elapsed time only. The browser still presents real frames and
    // sends keyboard input through Surface's normal host.
    tab.eval(
        r#"(() => {
        const zero = Math.ceil(performance.now());
        window.loadingTestMs = 0;
        Object.defineProperty(performance, 'now', {
            configurable: true, value: () => zero + window.loadingTestMs,
        });
    })()"#,
    )
    .await?;
    tab.settle().await
}

async fn advance(tab: &Tab, ms: u32) -> Result<()> {
    tab.eval(&format!("window.loadingTestMs = {ms}")).await?;
    tab.settle().await
}

async fn at(tab: &Tab, ms: u32) -> Result<Pose> {
    advance(tab, ms).await?;
    ready(tab, &format!("d.elapsed_ms === {ms} && d.requests_frame")).await?;
    pose(&loading(tab).await?)
}

#[derive(Clone, Copy)]
struct Pose {
    index: u64,
    morph: f64,
    angle: f64,
}

fn pose(data: &Value) -> Result<Pose> {
    Ok(Pose {
        index: data["shape_index"]
            .as_u64()
            .context("loading shape index")?,
        morph: data["morph"].as_f64().context("loading morph fraction")?,
        angle: data["rotation_degrees"]
            .as_f64()
            .context("loading rotation in degrees")?,
    })
}

impl Pose {
    fn value(self) -> Value {
        json!({"index":self.index,"morph":self.morph,"rotation_degrees":self.angle})
    }
}

fn spin_delta(before: Pose, after: Pose) -> f64 {
    (after.angle - before.angle + 180.).rem_euclid(360.) - 180.
}

async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Raster> {
    tab.settle().await?;
    let path = env.artifact(&format!("material-loading-{name}.png"));
    tab.screenshot(&path).await?;
    Raster::read(tab, path).await
}

struct Palette {
    background: Rgb,
    primary: Rgb,
    container: Rgb,
    contained: Rgb,
}

async fn palette(tab: &Tab) -> Result<Palette> {
    let data = loading(tab).await?;
    let read = |key: &str| -> Result<Rgb> {
        let [r, g, b, a]: [u8; 4] = serde_json::from_value(data["palette"][key].clone())
            .with_context(|| format!("loading palette {key}"))?;
        check_eq!(a, 255, "the palette tokens are opaque");
        Ok([r, g, b])
    };
    Ok(Palette {
        background: read("background")?,
        primary: read("primary")?,
        container: read("primary_container")?,
        contained: read("on_primary_container")?,
    })
}

fn contrast(a: Rgb, b: Rgb) -> f64 {
    let luminance = |rgb: Rgb| {
        rgb.into_iter()
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(v, w)| {
                let c = f64::from(v) / 255.;
                let linear = if c <= 0.04045 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                };
                w * linear
            })
            .sum::<f64>()
    };
    let [a, b] = [luminance(a), luminance(b)];
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

struct Frame {
    outer: [Rect; 2],
    active: [Rect; 2],
}

async fn frame(tab: &Tab, size: f64) -> Result<Frame> {
    let [_, _, width, height] = tab.canvas_box().await?;
    let view = [0., 0., width, height];
    let outer = [bounds(tab, IDS[0]).await?, bounds(tab, IDS[1]).await?];
    let active = [
        bounds(tab, "loading-default-shape").await?,
        bounds(tab, "loading-contained-shape").await?,
    ];
    let expected = size * 38. / 48.;
    for n in 0..2 {
        let body = outer[n];
        let shape = active[n];
        check_eq!(
            body[2..],
            [size, size],
            "loading gets its exact square allocation"
        );
        check!(
            contains(view, body)
                && contains(body, shape)
                && (shape[2] - expected).abs() < 0.2
                && (shape[3] - expected).abs() < 0.2
                && (2. * (shape[0] - body[0]) + shape[2] - size).abs() < 0.4
                && (2. * (shape[1] - body[1]) + shape[3] - size).abs() < 0.4,
            "the 38:48 active box is centered in its fully visible fixed frame: {body:?}/{shape:?}"
        );
    }
    check!(
        outer[0][0] + size <= outer[1][0] + 0.25
            || outer[1][0] + size <= outer[0][0] + 0.25
            || outer[0][1] + size <= outer[1][1] + 0.25
            || outer[1][1] + size <= outer[0][1] + 0.25,
        "the two indicators do not overlap"
    );
    Ok(Frame { outer, active })
}

impl Frame {
    fn value(&self) -> Value {
        json!({"outer":self.outer,"active":self.active})
    }
}

struct Mask {
    values: Vec<bool>,
    coverage: usize,
    ink_box: [usize; 4],
}

impl Mask {
    fn difference(&self, other: &Self) -> usize {
        self.values
            .iter()
            .zip(&other.values)
            .filter(|(a, b)| a != b)
            .count()
    }
    fn value(&self) -> Value {
        json!({"coverage":self.coverage,"ink_box":self.ink_box})
    }
}

fn mask(image: &Raster, [x, y, w, h]: Rect, ink: Rgb, angle: f64) -> Result<Mask> {
    let (sin, cos) = angle.to_radians().sin_cos();
    let side = GRID as f64;
    let relative = |i: usize| (i as f64 + 0.5) / side - 0.5;
    let center = [x + w / 2., y + h / 2.];
    let mut values = Vec::with_capacity(GRID * GRID);
    let mut coverage = 0;
    let mut ink_box = [GRID, GRID, 0, 0];
    for j in 0..GRID {
        for i in 0..GRID {
            let u = relative(i);
            let v = relative(j);
            let nx = u * cos - v * sin;
            let ny = u * sin + v * cos;
            let hit = nx.abs() < 0.5
                && ny.abs() < 0.5
                && distance(image.sample(center[0] + nx * w, center[1] + ny * h)?, ink) <= 8;
            if hit {
                coverage += 1;
                ink_box[0] = ink_box[0].min(i);
                ink_box[1] = ink_box[1].min(j);
                ink_box[2] = ink_box[2].max(i + 1);
                ink_box[3] = ink_box[3].max(j + 1);
            }
            values.push(hit);
        }
    }
    check!(
        coverage >= 240 && ink_box[0]>=3 && ink_box[1]>=3
            && ink_box[2]<=GRID-3 && ink_box[3]<=GRID-3,
        "real indicator ink is substantial and does not touch its outer allocation: {coverage} cells {ink_box:?}"
    );
    Ok(Mask {
        values,
        coverage,
        ink_box,
    })
}

fn colors(image: &Raster, frame: &Frame, palette: &Palette) -> Result<Value> {
    let [x, y, w, _] = frame.outer[0];
    let [cx, cy, cw, _] = frame.outer[1];
    let blank = image.sample(x + w / 2., y + w / 24.)?;
    let circle = image.sample(cx + cw / 2., cy + cw / 24.)?;
    let corner = image.sample(cx + cw / 24., cy + cw / 24.)?;
    check!(
        distance(blank, palette.background) <= 4 && distance(corner, palette.background) <= 4,
        "Default is transparent and Contained leaves the circle's outside corner clear"
    );
    check_eq!(
        circle,
        palette.container,
        "the Contained cardinal edge shows Primary Container"
    );
    let inks = [
        mask(image, frame.outer[0], palette.primary, 0.)?,
        mask(image, frame.outer[1], palette.contained, 0.)?,
    ];
    let ratio = [
        contrast(palette.primary, blank),
        contrast(palette.contained, circle),
    ];
    check!(
        ratio.into_iter().all(|v| v >= 3.),
        "both active outlines have 3:1 against real background pixels: {ratio:?}"
    );
    Ok(
        json!({"blank":blank,"circle":circle,"corner":corner,"shape_colors":[palette.primary,palette.contained],
        "contrast":ratio,"masks":inks.map(|m|m.value())}),
    )
}

async fn semantics(tab: &Tab) -> Result<Value> {
    let active = tab
        .eval("document.activeElement?.dataset?.surfaceId ?? null")
        .await?;
    let nodes = ax_tree(tab).await?;
    let mut evidence = Vec::new();
    for (id, label) in IDS.into_iter().zip(NAMES) {
        let view = projected(id);
        tab.wait_for(&format!("{view}?.getAttribute('role') === 'progressbar'"))
            .await?;
        let attrs=tab.eval(&format!(r#"(() => {{
            const node={view};
            return {{
                label:node.getAttribute('aria-label'), now:node.getAttribute('aria-valuenow'),
                min:node.getAttribute('aria-valuemin'), max:node.getAttribute('aria-valuemax'),
                action:node.getAttribute('data-surface-action'), tab:node.getAttribute('tabindex'),
                live:node.getAttribute('aria-live'), submenu: node.querySelector('[tabindex], [data-surface-action]')!==null,
            }};
        }})()"#)).await?;
        let exposed: Vec<_> = nodes
            .iter()
            .filter(|n| {
                n["ignored"] == false
                    && n["role"]["value"] == "progressbar"
                    && n["name"]["value"] == label
            })
            .collect();
        check_eq!(
            exposed.len(),
            1,
            "one task-specific progressbar in actual Chrome AX"
        );
        let node = exposed[0];
        let can_focus = node["properties"].as_array().is_some_and(|properties| {
            properties
                .iter()
                .any(|p| p["name"] == "focusable" && p["value"]["value"] == true)
        });
        check!(
            attrs["label"]==label && attrs["now"].is_null() && attrs["min"].is_null()
                && attrs["max"].is_null() && attrs["tab"].is_null() && attrs["action"].is_null()
                && (attrs["live"].is_null() || attrs["live"]=="off")
                && attrs["submenu"]==false && node["value"]["value"].is_null() && !can_focus,
            "indeterminate progress is named, read-only, not live-region spam, and not Tab-focusable: {attrs} / {node}"
        );
        evidence.push(json!({"id":id,"name":label,"ax_id":node["nodeId"],"backend_id":node["backendDOMNodeId"],"attributes":attrs}));
    }
    check_eq!(
        tab.eval("document.activeElement?.dataset?.surfaceId ?? null")
            .await?,
        active,
        "reading the AX tree leaves browser focus as is"
    );
    Ok(json!(evidence))
}

const TRACK_AX: &str = r#"
    (() => {
        const root=document.getElementById('surface-accessibility');
        const ids=['loading-default','loading-contained'];
        const nodes=ids.map(id=>root.querySelector('[data-surface-id="'+id+'"]'));
        const liveRegions=[...document.querySelectorAll('[aria-live="polite"],[aria-live="assertive"]')];
        const announcements=liveRegions.map(region=>region.textContent);
        const changes={indicator:0,live:0,focus:0};
        const observer=new MutationObserver(records => {
            for(const record of records) {
                if(nodes.some(node=>node===record.target || node.contains(record.target))) changes.indicator++;
                if(liveRegions.some(region=>region===record.target || region.contains(record.target))) changes.live++;
            }
        });
        observer.observe(document.body,{attributes:true,attributeFilter:['aria-label','aria-live','aria-valuenow','role'],
            subtree:true,childList:true,characterData:true});
        const active=document.activeElement;
        document.addEventListener('focusin',()=>{ changes.focus++; },{capture:true});
        window.loadingAxTrace=()=>({
            sameNodes:nodes.every((node,i)=>node===root.querySelector('[data-surface-id="'+ids[i]+'"]')),
            sameFocus:document.activeElement===active,
            sameAnnouncements:liveRegions.every((region,i)=>region.textContent===announcements[i]),
            ...changes
        });
    })()
"#;

async fn visual_tokens(env: &Env, tab: &Tab) -> Result<Value> {
    step("M201","two real 48 dp variants paint their tokens; the named indeterminate progress stays outside Tab");
    ready(tab, "d.elapsed_ms > 50 && d.requests_frame").await?;
    let start = loading(tab).await?["elapsed_ms"]
        .as_f64()
        .context("live elapsed time")?;
    ready(tab, &format!("d.elapsed_ms > {}", start + 70.)).await?;
    let later = loading(tab).await?["elapsed_ms"]
        .as_f64()
        .context("later live elapsed time")?;
    tab.cdp("Accessibility.enable", json!({})).await?;
    focus_projected(tab, BACK).await?;
    tab.press("Tab").await?;
    until(
        tab,
        "s.focused?.startsWith('loading-demo-') && s.focused !== 'loading-demo-back'",
    )
    .await?;
    let tab_focus = state(tab).await?["focused"].clone();
    focused(tab, tab_focus.as_str().context("loading Tab target")?).await?;
    let semantics = semantics(tab).await?;
    let frame = frame(tab, 48.).await?;
    let palette = palette(tab).await?;
    let image = capture(env, tab, "tokens-light").await?;
    let pixels = colors(&image, &frame, &palette)?;
    Ok(
        json!({"live_elapsed_ms":[start,later],"frames":frame.value(),"colors":pixels,
        "tab_focus":tab_focus,"semantics":semantics}),
    )
}

async fn sequence(env: &Env, tab: &Tab) -> Result<Value> {
    step("M202","seven measured GPU silhouettes morph along the spring and wrap without focus or announcement churn");
    activate(tab, STOP).await?;
    ready(tab, "d.visible && !d.running && !d.requests_frame").await?;
    freeze(tab).await?;
    activate(tab, START).await?;
    ready(tab, "d.running && d.elapsed_ms === 0 && d.requests_frame").await?;
    focused(tab, START).await?;
    let initial_ax = semantics(tab).await?;
    tab.eval(TRACK_AX).await?;
    let initial = frame(tab, 48.).await?;
    let palette = palette(tab).await?;
    let mut variants = Vec::new();
    let mut poses = Vec::new();
    let mut overshoot = None;
    let mut midpoint = None;
    let mut just_before_step = None;
    for (index, name) in POSES.into_iter().enumerate() {
        let ms = u32::try_from(index)? * 650;
        let angles = at(tab, ms).await?;
        check!(
            angles.index == index as u64 && angles.morph.abs() < 0.0001,
            "shape {name} starts at t={ms}: {}",
            angles.value()
        );
        let expected_rotation = (90. + 90. * index as f64 + f64::from(ms) * 360. / 4666.) % 360.;
        check!(
            (angles.angle - expected_rotation).abs() < 0.05,
            "the calibrated phase has the expected physical rotation"
        );
        let current = frame(tab, 48.).await?;
        check!(
            initial.outer == current.outer && initial.active == current.active,
            "shape updates leave both layout rectangles fixed"
        );
        let filename = if index == 0 {
            "sequence-light".into()
        } else {
            format!("pose-{index}-{name}")
        };
        let image = capture(env, tab, &filename).await?;
        let body = mask(&image, current.outer[0], palette.primary, angles.angle)?;
        let other = mask(&image, current.outer[1], palette.contained, angles.angle)?;
        check!(
            body.difference(&other) <= 30,
            "the two variants paint the same outline in different colors"
        );
        let earlier: Vec<_> = variants.iter().map(|prev| body.difference(prev)).collect();
        check!(
            earlier.iter().all(|distance| *distance >= 18),
            "{name} has a distinct measured footprint after undoing rotation: {earlier:?}"
        );
        poses.push(json!({"name":name,"time_ms":ms,"pose":angles.value(),"default":body.value(),"contained":other.value(),
            "distance_to_earlier":earlier}));
        variants.push(body);
        if index == 0 {
            let middle = at(tab, 80).await?;
            check!(
                middle.index == 0
                    && (middle.morph - 0.3880).abs() < 0.02
                    && spin_delta(angles, middle) > 35.,
                "actual spring and global spin are already moving at 80ms"
            );
            let picture = capture(env, tab, "morph-midpoint").await?;
            let body = mask(&picture, initial.outer[0], palette.primary, middle.angle)?;
            check!(
                body.difference(&variants[0]) >= 18,
                "the mid-frame is a real intermediate silhouette"
            );
            midpoint = Some((middle, body));
            let overshot = at(tab, 278).await?;
            check!(
                overshot.index == 0 && overshot.morph > 1.06 && overshot.morph < 1.11,
                "the spring moves past the destination before settling"
            );
            let overshoot_frame = capture(env, tab, "spring-overshoot").await?;
            let overshoot_mask = mask(
                &overshoot_frame,
                initial.outer[0],
                palette.primary,
                overshot.angle,
            )?;
            overshoot =
                Some(json!({"time_ms":278,"pose":overshot.value(),"ink":overshoot_mask.value()}));
            let before = at(tab, 649).await?;
            check!(
                before.index == 0 && (before.morph - 1.).abs() < 0.0001,
                "the spring has settled before the next step"
            );
            just_before_step = Some(before);
        }
        if index == 1 {
            check!(
                (0. ..1.).contains(&spin_delta(
                    just_before_step.context("649ms boundary")?,
                    angles
                )),
                "physical spin is continuous across t=650"
            );
        }
    }
    let (intermediate, middle) = midpoint.context("first 80ms mask")?;
    let distances = [
        middle.difference(&variants[0]),
        middle.difference(&variants[1]),
    ];
    check!(
        distances.into_iter().all(|n| n >= 18),
        "80ms footprint differs from both adjacent endpoint silhouettes"
    );
    let mut wraps = Vec::new();
    for ms in [4550, 4666] {
        let before = at(tab, ms - 1).await?;
        let reference = capture(env, tab, &format!("continuity-before-{ms}")).await?;
        let from = mask(&reference, initial.outer[0], palette.primary, 0.)?;
        let after = at(tab, ms).await?;
        let target = capture(env, tab, &format!("continuity-after-{ms}")).await?;
        let to = mask(&target, initial.outer[0], palette.primary, 0.)?;
        let angle = spin_delta(before, after);
        let changed = from.difference(&to);
        check!(angle>0. && angle<2. && changed<=45,
            "the physical outline remains continuous across {ms} ms (angle {angle}, {changed} mask cells)");
        if ms == 4550 {
            check!(
                before.index == 6 && after.index == 0 && before.morph == 1. && after.morph == 0.,
                "the final outline hands off to active pair zero"
            );
        } else {
            check!(
                before.index == 0 && after.index == 0 && after.morph > before.morph,
                "global rotation wraps independently during a real morph"
            );
        }
        wraps.push(json!({"time_ms":ms,"before":before.value(),"after":after.value(),"spin_delta":angle,"diff_pixels":changed}));
    }
    let after_ax = semantics(tab).await?;
    check_eq!(
        after_ax,
        initial_ax,
        "animation keeps actual Chrome AX node identities and attributes"
    );
    let trace = tab.eval("window.loadingAxTrace()").await?;
    check_eq!(
        trace,
        json!({"sameNodes":true,"sameFocus":true,"sameAnnouncements":true,"indicator":0,"live":0,"focus":0}),
        "live motion changes no named indicator semantics, no focus and no live text"
    );
    Ok(
        json!({"poses":poses,"midpoint":{"time_ms":80,"pose":intermediate.value(),"distance_from_endpoints":distances},
        "overshoot":overshoot,"wraps":wraps,"frame":initial.value(),"ax_unchanged":trace}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M203",
        "24 and 48 dp fit 320 dp dark RTL at 200% text and the 240 dp frame stays bounded",
    );
    activate(tab, STOP).await?;
    ready(tab, "!d.running").await?;
    freeze(tab).await?;
    activate(tab, START).await?;
    ready(tab, "d.elapsed_ms === 0 && d.running").await?;
    choose(tab, "theme").await?;
    resize(tab, 320, 920).await?;
    ready(tab, "d.visible && !d.rtl && d.text_scale === 1").await?;
    let large_text_before = frame(tab, 48.).await?;
    let left_pose = at(tab, 325).await?;
    let lcd = capture(env, tab, "compact-dark-ltr").await?;
    let palette = palette(tab).await?;
    let ltr_mask = mask(&lcd, large_text_before.outer[0], palette.primary, 0.)?;
    choose(tab, "rtl").await?;
    choose(tab, "large-text").await?;
    ready(tab, "d.rtl && d.text_scale === 2 && d.elapsed_ms === 325").await?;
    let right_pose = pose(&loading(tab).await?)?;
    check!(
        (spin_delta(left_pose, right_pose)).abs() < 0.001 && left_pose.morph == right_pose.morph,
        "RTL and text scale keep physical rotation and spring phase"
    );
    let rtl = frame(tab, 48.).await?;
    let dark = capture(env, tab, "compact-dark-rtl").await?;
    let rtl_mask = mask(&dark, rtl.outer[0], palette.primary, 0.)?;
    check!(
        ltr_mask.difference(&rtl_mask) <= 30,
        "the actual RTL outline keeps its physical orientation"
    );
    let normal = colors(&dark, &rtl, &palette)?;
    let settled = at(tab, 520).await?;
    let next = at(tab, 551).await?;
    check!(
        (spin_delta(settled, next) - 31. * 360. / 4666.).abs() < 0.05,
        "rotation remains clockwise in RTL beyond a settled turn"
    );

    choose(tab, "small").await?;
    ready(tab, "d.size === 24 && d.active_size === 19").await?;
    let small = frame(tab, 24.).await?;
    let small_image = capture(env, tab, "compact-small").await?;
    let small_paint = colors(&small_image, &small, &palette)?;

    resize(tab, 960, 920).await?;
    choose(tab, "large").await?;
    ready(tab, "d.size === 240 && d.active_size === 190").await?;
    let large = frame(tab, 240.).await?;
    let large_image = capture(env, tab, "large").await?;
    let large_paint = colors(&large_image, &large, &palette)?;
    let dark_colors = json!({"regular":normal,"small":small_paint,"large":large_paint});
    Ok(
        json!({"compact_viewport":[320,920],"text_scale":2,"rtl":true,"sizes":[24,48,240],
        "frames":[small.value(),rtl.value(),large.value()],"colors":dark_colors,
        "ltr_to_rtl_difference":ltr_mask.difference(&rtl_mask),"clockwise_delta":spin_delta(settled,next)}),
    )
}

async fn idle_browser(tab: &Tab) -> Result<u64> {
    tab.eval(
        r#"(async () => {
        const request=window.requestAnimationFrame;
        const frame=()=>new Promise(resolve=>request.call(window,resolve));
        let requested=0;
        window.requestAnimationFrame=(callback)=>{
            requested++;
            return request.call(window,callback);
        };
        try { for(let i=0;i<4;i++) await frame(); }
        finally { window.requestAnimationFrame=request; }
        return requested;
    })()"#,
    )
    .await?
    .as_u64()
    .context("browser animation frame requests")
}

async fn no_progress(tab: &Tab) -> Result<()> {
    for id in IDS {
        let target = projected(id);
        tab.wait_for(&format!("{target} === null")).await?;
    }
    let tree = ax_tree(tab).await?;
    check!(
        !tree.iter().any(|node| node["ignored"] == false
            && node["role"]["value"] == "progressbar"
            && NAMES.iter().any(|name| node["name"]["value"] == *name)),
        "stopped work removes both actual task-specific AX progress bars"
    );
    Ok(())
}

async fn slots(tab: &Tab) -> Result<[Rect; 2]> {
    Ok([
        bounds(tab, "loading-default-slot").await?,
        bounds(tab, "loading-contained-slot").await?,
    ])
}

async fn lifecycle(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M204",
        "Reduce Motion and Stop leave no animation frames; restart and re-entry reset the pose",
    );
    activate(tab, STOP).await?;
    ready(tab, "!d.running && !d.requests_frame").await?;
    freeze(tab).await?;
    activate(tab, START).await?;
    ready(tab, "d.running && d.elapsed_ms === 0 && d.requests_frame").await?;
    focused(tab, START).await?;
    let viewport = frame(tab, 48.).await?;
    let slots_before = slots(tab).await?;
    let palette = palette(tab).await?;
    let static_image = capture(env, tab, "first-frame").await?;
    let first = pose(&loading(tab).await?)?;
    check!(
        first.index == 0 && first.morph == 0. && (first.angle - 90.).abs() < 0.001,
        "start paints the authored first shape at 90 degrees"
    );
    let first_mask = mask(&static_image, viewport.outer[0], palette.primary, 0.)?;
    at(tab, 812).await?;
    let live = capture(env, tab, "before-reduced").await?;
    check!(
        first_mask.difference(&mask(&live, viewport.outer[0], palette.primary, 0.)?) >= 18,
        "the preference is switched during a visibly different moving pose"
    );

    reduced_motion(tab, true).await?;
    ready(tab,"d.running && d.reduced_motion && !d.requests_frame && d.shape_index === 0 && d.morph === 0").await?;
    let reduced = capture(env, tab, "reduced").await?;
    let static_mask = mask(&reduced, viewport.outer[0], palette.primary, 0.)?;
    check_eq!(
        first_mask.difference(&static_mask),
        0,
        "Reduce Motion selects the fixed first shape"
    );
    let reduced_ax = semantics(tab).await?;
    focused(tab, START).await?;
    advance(tab, 1720).await?;
    let later = capture(env, tab, "reduced-later").await?;
    let still = static_mask.difference(&mask(&later, viewport.outer[0], palette.primary, 0.)?);
    check_eq!(
        still,
        0,
        "advancing time never repaints a different reduced-motion mask"
    );
    check_eq!(
        semantics(tab).await?,
        reduced_ax,
        "reduced frames keep one stable named AX node each"
    );
    let reduced_requests = idle_browser(tab).await?;
    check_eq!(
        reduced_requests,
        0,
        "after settling the reduced scene schedules no more browser frames"
    );

    activate(tab, STOP).await?;
    ready(tab, "d.visible && !d.running && !d.requests_frame").await?;
    focused(tab, STOP).await?;
    no_progress(tab).await?;
    check_eq!(
        slots(tab).await?,
        slots_before,
        "ending work keeps both reserved squares at the same location"
    );
    let stopped_requests = idle_browser(tab).await?;
    check_eq!(
        stopped_requests,
        0,
        "stopped work schedules no more browser frames"
    );
    reduced_motion(tab, false).await?;
    activate(tab, START).await?;
    ready(
        tab,
        "d.running && !d.reduced_motion && d.elapsed_ms === 0 && d.requests_frame",
    )
    .await?;
    let restarted = capture(env, tab, "restart").await?;
    check_eq!(
        mask(&restarted, viewport.outer[0], palette.primary, 0.)?.difference(&first_mask),
        0,
        "Run starts with the same first pose after a previous stopped task"
    );
    focused(tab, START).await?;
    advance(tab, 2100).await?;
    ready(tab, "d.elapsed_ms === 380").await?;
    choose(tab, "reset").await?;
    ready(tab, "d.elapsed_ms === 0 && d.running && d.requests_frame").await?;
    let reset = capture(env, tab, "reset").await?;
    check_eq!(
        mask(&reset, viewport.outer[0], palette.primary, 0.)?.difference(&first_mask),
        0,
        "Reset restores the first painted pose while work stays active"
    );
    activate(tab, BACK).await?;
    ready(tab, "!d.visible && !d.requests_frame").await?;
    focused(tab, SHOW).await?;
    no_progress(tab).await?;
    let outside_requests = idle_browser(tab).await?;
    check_eq!(outside_requests, 0, "exiting stops the scene frame loop");
    resize(tab, 960, 2000).await?;
    open(tab).await?;
    ready(
        tab,
        "d.elapsed_ms === 0 && d.shape_index === 0 && d.requests_frame",
    )
    .await?;
    let reopened = frame(tab, 48.).await?;
    let reentry = capture(env, tab, "reentered").await?;
    check_eq!(
        mask(&reentry, reopened.outer[0], palette.primary, 0.)?.difference(&first_mask),
        0,
        "re-entry renders the first visible shape, without elapsed time from the old scene"
    );
    Ok(
        json!({"same_reduced_mask":still==0,"browser_rAF_requests":[reduced_requests,stopped_requests,outside_requests],
        "reserved_slots":slots_before,"reduced_names":reduced_ax,"focused_on_stop":STOP,"focus_restored_to":SHOW,
        "restarted_at_ms":0,"reset_to_ms":0,"reentered_at_ms":0}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let tokens = visual_tokens(env, tab).await?;
    let sequence = sequence(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    reload(tab, url).await?;
    let lifecycle = lifecycle(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "Loading Gallery page errors: {errors:?}");
    env.write_json("browser-material-loading.json",&json!({
        "browser":chrome.version,"tokens":tokens,"sequence":sequence,"compact":compact,"lifecycle":lifecycle,"errors":errors
    }))
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 2044, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _=env.write_json("material-loading-failure.json",&json!({
            "error":format!("{error:#}"),"state":state(&tab).await.ok(),
            "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
            "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()
        }));
        let _ = tab
            .screenshot(&env.artifact("material-loading-failure.png"))
            .await;
    }
    result.context("Material 3 Expressive loading indicators")
}
