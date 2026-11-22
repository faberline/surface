//! Flat and wavy Material progress indicators in the WebGPU Gallery.

use super::{click, point, state, until, Point};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::ime::find_node;
use crate::{check, check_eq, Env};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::{fs::File, io::BufReader};

const INDICATORS: [(&str, &str); 4] = [
    ("progress-linear-flat", "線形進度"),
    ("progress-linear-wavy", "波形線形進度"),
    ("progress-circular-flat", "圓形進度"),
    ("progress-circular-wavy", "波形圓形進度"),
];

type Rgb = [u8; 3];

async fn ready(tab: &Tab, condition: &str) -> Result<()> {
    until(tab, &format!("(p => p && ({condition}))(s.progress)")).await?;
    tab.settle().await
}

async fn progress(tab: &Tab) -> Result<Value> {
    Ok(state(tab).await?["progress"].clone())
}

async fn set_value(tab: &Tab, percent: u8) -> Result<()> {
    click(tab, &format!("progress-value-{percent}")).await?;
    ready(tab, &format!("p.known && p.value === {percent} / 100")).await
}

fn ax_property<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node["properties"]
        .as_array()?
        .iter()
        .find(|item| item["name"] == name)
        .map(|item| &item["value"]["value"])
}

async fn semantics(tab: &Tab, value: Option<f64>) -> Result<Value> {
    let selectors =
        INDICATORS.map(|(id, _)| format!("#surface-accessibility [data-surface-id='{id}']"));
    let selectors = serde_json::to_string(&selectors)?;
    let value_js = serde_json::to_string(&value)?;
    tab.wait_for(&format!(
        "(() => {{
            const expected = {value_js};
            return {selectors}.every(selector => {{
                const node = document.querySelector(selector);
                const now = node?.getAttribute('aria-valuenow');
                return node?.getAttribute('role') === 'progressbar' &&
                    (expected === null ? now === null : now !== null && Number(now) === expected);
            }});
        }})()"
    ))
    .await?;
    let projected = tab
        .eval(&format!(
            "{selectors}.map(selector => {{
            const nodes = document.querySelectorAll(selector);
            const node = nodes[0];
            return {{
                count: nodes.length, label: node.getAttribute('aria-label'),
                now: node.getAttribute('aria-valuenow'), min: node.getAttribute('aria-valuemin'),
                max: node.getAttribute('aria-valuemax'), tab: node.getAttribute('tabindex'),
                action: node.getAttribute('data-surface-action'),
                controls: node.querySelectorAll('[tabindex], [data-surface-action]').length
            }};
        }})"
        ))
        .await?;
    let tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    let ax = tree["nodes"].as_array().context("Chrome AX nodes")?;
    let dom = tab.eval_json("window.gallery.export_dom()").await?;
    let mut result = Vec::new();
    for (index, (id, name)) in INDICATORS.into_iter().enumerate() {
        let attributes = &projected[index];
        let matching: Vec<_> = ax
            .iter()
            .filter(|node| {
                node["ignored"] == false
                    && node["role"]["value"] == "progressbar"
                    && node["name"]["value"] == name
            })
            .collect();
        check_eq!(
            matching.len(),
            1,
            format!("one Chrome progressbar named {name}")
        );
        let exposed = matching[0];
        let node = find_node(&dom, id)?;
        let range = &node["range"];
        check!(
            node["role"] == "Progress"
                && node["name"] == name
                && node["tab_index"].is_null()
                && attributes["count"] == 1
                && attributes["label"] == name
                && attributes["tab"].is_null()
                && attributes["action"].is_null()
                && attributes["controls"] == 0
                && ax_property(exposed, "focusable") != Some(&json!(true)),
            "one named, read-only progressbar with no actions for {id}: {attributes}"
        );
        let chrome_value = exposed["value"]["value"].as_f64();
        check_eq!(
            chrome_value,
            value,
            format!("Chrome progress value for {id}")
        );
        if let Some(value) = value {
            check!(
                range["min"].as_f64() == Some(0.)
                    && range["max"].as_f64() == Some(1.)
                    && range["value"].as_f64() == Some(value)
                    && attributes["min"] == "0"
                    && attributes["max"] == "1"
                    && ax_property(exposed, "valuemin").and_then(Value::as_f64) == Some(0.)
                    && ax_property(exposed, "valuemax").and_then(Value::as_f64) == Some(1.),
                "progress range 0..1 for {id}: {attributes} / {exposed}"
            );
        } else {
            // Chrome can supply implicit bounds. The absent value is what
            // distinguishes unknown progress in Chrome's accessibility tree.
            check!(
                range.is_null()
                    && exposed["value"].is_null()
                    && attributes["now"].is_null()
                    && attributes["min"].is_null()
                    && attributes["max"].is_null(),
                "changing {id} to unknown removes the old range: {attributes} / {exposed}"
            );
        }
        result.push(json!({"id":id,"name":name,"authored_range":range,
            "chrome_value":chrome_value,"projected":attributes}));
    }
    Ok(json!(result))
}

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
    fn from_point(point: &Point) -> Self {
        Self {
            x: point.canvas[0] + point.b[0],
            y: point.canvas[1] + point.b[1],
            width: point.b[2],
            height: point.b[3],
        }
    }
}

async fn visible_box(tab: &Tab, id: &str) -> Result<Rect> {
    let p = point(tab, id).await?;
    let [x, y, width, height] = p.b[..].try_into().context("progress bounds")?;
    check!(
        width > 0.
            && height > 0.
            && x >= 0.
            && y >= 0.
            && x + width <= p.canvas[2] + 0.5
            && y + height <= p.canvas[3] + 0.5,
        "{id} fits the visible canvas: {:?} in {:?}",
        p.b,
        p.canvas
    );
    Ok(Rect::from_point(&p))
}

async fn boxes(tab: &Tab) -> Result<[Rect; 4]> {
    Ok([
        visible_box(tab, INDICATORS[0].0).await?,
        visible_box(tab, INDICATORS[1].0).await?,
        visible_box(tab, INDICATORS[2].0).await?,
        visible_box(tab, INDICATORS[3].0).await?,
    ])
}

struct Png {
    bytes: Vec<u8>,
    info: png::OutputInfo,
    channels: usize,
    scale: f64,
}

impl Png {
    async fn capture(env: &Env, tab: &Tab, name: &str) -> Result<Self> {
        tab.settle().await?;
        let path = env.artifact(&format!("material-progress-{name}.png"));
        tab.screenshot(&path).await?;
        let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let mut bytes = vec![0; reader.output_buffer_size().context("PNG output size")?];
        let info = reader.next_frame(&mut bytes)?;
        let channels = match info.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => bail!("unexpected Chrome PNG type {other:?}"),
        };
        let width = tab
            .eval("window.innerWidth")
            .await?
            .as_f64()
            .context("Chrome width")?;
        let scale = f64::from(info.width) / width;
        Ok(Self {
            bytes,
            info,
            channels,
            scale,
        })
    }

    fn pixel(&self, x: usize, y: usize) -> Result<Rgb> {
        if x >= self.info.width as usize || y >= self.info.height as usize {
            bail!("pixel ({x}, {y}) is outside Chrome's PNG");
        }
        let offset = y * self.info.line_size + x * self.channels;
        Ok(self.bytes[offset..offset + 3].try_into()?)
    }

    fn sample(&self, area: Rect, x: f64, y: f64) -> Result<Rgb> {
        self.pixel(
            ((area.x + x) * self.scale).floor() as usize,
            ((area.y + y) * self.scale).floor() as usize,
        )
    }

    fn changed(&self, after: &Self, area: Rect) -> Result<usize> {
        check_eq!(
            self.scale,
            after.scale,
            "PNG uses one pixel scale across frames"
        );
        let left = (area.x * self.scale).ceil() as usize;
        let top = (area.y * self.scale).ceil() as usize;
        let right = ((area.x + area.width) * self.scale).floor() as usize;
        let bottom = ((area.y + area.height) * self.scale).floor() as usize;
        let mut pixels = 0;
        for y in top..bottom {
            for x in left..right {
                if distance(self.pixel(x, y)?, after.pixel(x, y)?) > 3 {
                    pixels += 1;
                }
            }
        }
        Ok(pixels)
    }
}

fn distance(lhs: Rgb, rhs: Rgb) -> u16 {
    lhs.into_iter()
        .zip(rhs)
        .map(|(a, b)| u16::from(a.abs_diff(b)))
        .sum()
}

fn palette(progress: &Value, key: &str) -> Result<Rgb> {
    let [red, green, blue, _]: [u8; 4] = serde_json::from_value(progress[key].clone())?;
    Ok([red, green, blue])
}

fn painted(observed: Rgb, expected: Rgb, what: &str) -> Result<()> {
    check!(
        distance(observed, expected) <= 12,
        "{what}: expected {expected:?}, observed {observed:?}"
    );
    Ok(())
}

fn flat_linear(png: &Png, area: Rect, active: Rgb, track: Rgb, rtl: bool) -> Result<Value> {
    check_eq!(area.height, 4., "flat linear height");
    let x = |logical: f64| if rtl { area.width - logical } else { logical };
    let center = |logical: f64| png.sample(area, x(logical), area.height / 2.);
    let base = png.sample(area, x(area.width * 0.30), -2.)?;
    let fill = center(area.width * 0.125)?;
    let gap = center(area.width * 0.25 + 2.)?;
    let rest = center(area.width * 0.65)?;
    let stop = center(area.width - 2.)?;
    let corner = png.sample(area, x(0.25), 0.25)?;
    painted(fill, active, "quarter linear active ink at logical start")?;
    painted(gap, base, "four-dp linear gap exposes background")?;
    painted(rest, track, "linear track ink")?;
    painted(stop, active, "primary stop at logical end")?;
    check!(
        distance(corner, base) < distance(corner, active),
        "outer corner of the active bar stays round: {corner:?}"
    );
    Ok(json!({"active":fill,"gap":gap,"track":rest,"stop":stop,"round_cap_corner":corner}))
}

fn flat_ring(png: &Png, area: Rect, active: Rgb, track: Rgb) -> Result<Value> {
    check_eq!(
        [area.width, area.height],
        [40., 40.],
        "flat circular diameter"
    );
    let fill = png.sample(area, 32.7, 7.3)?;
    let rest = png.sample(area, 2., 20.)?;
    let hole = png.sample(area, 20., 20.)?;
    let gap = png.sample(area, 37.6, 23.6)?;
    painted(fill, active, "clockwise circular quarter ink")?;
    painted(rest, track, "circular remainder track ink")?;
    painted(
        gap,
        hole,
        "circular gap exposes the same background as the hole",
    )?;
    check!(
        distance(hole, active) > 20 && distance(hole, track) > 20,
        "empty circular center: {hole:?}"
    );
    Ok(json!({"active":fill,"track":rest,"hole":hole,"gap":gap}))
}

fn spread(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - values.iter().copied().fold(f64::INFINITY, f64::min)
}

fn wavy_ink(png: &Png, linear: Rect, ring: Rect, active: Rgb, rtl: bool) -> Result<Value> {
    check_eq!(linear.height, 10., "wavy linear height");
    check_eq!(
        [ring.width, ring.height],
        [48., 48.],
        "wavy circular diameter"
    );
    let mut columns = Vec::new();
    for logical_x in (5..38).step_by(2) {
        let logical = f64::from(logical_x);
        let x = if rtl { linear.width - logical } else { logical };
        let mut ink = Vec::new();
        for half_y in 0..20 {
            let y = f64::from(half_y) / 2.;
            if distance(png.sample(linear, x, y)?, active) <= 12 {
                ink.push(y);
            }
        }
        if !ink.is_empty() {
            columns.push(ink.iter().sum::<f64>() / ink.len() as f64);
        }
    }
    let mut radii = Vec::new();
    for degree in (15..75).step_by(4) {
        let angle = f64::from(degree).to_radians();
        let mut ink = Vec::new();
        for half_r in 33..49 {
            let radius = f64::from(half_r) / 2.;
            let pixel = png.sample(ring, 24. + radius * angle.sin(), 24. - radius * angle.cos())?;
            if distance(pixel, active) <= 12 {
                ink.push(radius);
            }
        }
        if !ink.is_empty() {
            radii.push(ink.iter().sum::<f64>() / ink.len() as f64);
        }
    }
    let linear_spread = spread(&columns);
    let radial_spread = spread(&radii);
    check!(
        columns.len() >= 10 && linear_spread > 2.,
        "linear active wave bends across columns: {columns:?}"
    );
    check!(
        radii.len() >= 10 && radial_spread > 1.,
        "circular active wave bends across radii: {radii:?}"
    );
    Ok(
        json!({"linear_centers":columns,"linear_spread_dp":linear_spread,
        "circular_radii":radii,"circular_spread_dp":radial_spread}),
    )
}

async fn known(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M41",
        "known progress exposes a read-only range and paints a gap, stop, arc, and waves",
    );
    click(tab, "section-1").await?;
    click(tab, "progress-demo-show").await?;
    ready(tab, "p.known && p.value === 0.25 && !p.running && !p.rtl").await?;
    let quarter = semantics(tab, Some(0.25)).await?;
    let areas = boxes(tab).await?;
    check_eq!(areas[0].width, 240., "flat default width");
    check_eq!(areas[1].width, 240., "wavy default width");
    let reported = progress(tab).await?;
    let active = palette(&reported, "active_color")?;
    let track = palette(&reported, "track_color")?;
    let image = Png::capture(env, tab, "light-known-25").await?;
    let linear = flat_linear(&image, areas[0], active, track, false)?;
    let ring = flat_ring(&image, areas[2], active, track)?;
    let waves = wavy_ink(&image, areas[1], areas[3], active, false)?;

    set_value(tab, 0).await?;
    let zero = semantics(tab, Some(0.)).await?;
    let empty = Png::capture(env, tab, "light-known-0").await?;
    painted(empty.sample(areas[0], 120., 2.)?, track, "0% linear track")?;
    painted(
        empty.sample(areas[2], 32.7, 7.3)?,
        track,
        "0% circular track",
    )?;

    set_value(tab, 75).await?;
    let three_quarters = semantics(tab, Some(0.75)).await?;
    set_value(tab, 100).await?;
    let full = semantics(tab, Some(1.)).await?;
    let complete = Png::capture(env, tab, "light-known-100").await?;
    painted(
        complete.sample(areas[0], 216., 2.)?,
        active,
        "100% linear active",
    )?;
    painted(
        complete.sample(areas[2], 2., 20.)?,
        active,
        "100% circular active",
    )?;

    set_value(tab, 25).await?;
    click(tab, INDICATORS[0].0).await?;
    ready(tab, "p.value === 0.25 && p.known").await?;
    let mut reached = BTreeSet::new();
    for _ in 0..12 {
        tab.press("Tab").await?;
        tab.settle().await?;
        if let Some(focused) = state(tab).await?["focused"].as_str() {
            check!(
                INDICATORS.iter().all(|(id, _)| *id != focused),
                "progress node stole Tab focus: {focused}"
            );
            reached.insert(focused.to_owned());
        }
    }
    check!(
        reached.len() >= 5,
        "Tab visits the interactive preview controls: {reached:?}"
    );
    Ok(
        json!({"quarter_ax":quarter,"zero_ax":zero,"three_quarters_ax":three_quarters,
        "complete_ax":full,"linear_pixels":linear,"circular_pixels":ring,"wave_pixels":waves,
        "tab_controls":reached}),
    )
}

async fn unknown(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M42",
        "switching to unknown progress removes stale ranges on all four indicators",
    );
    click(tab, "progress-mode-unknown").await?;
    ready(tab, "!p.known && !p.running").await?;
    let unknown = semantics(tab, None).await?;
    let areas = boxes(tab).await?;
    let still = Png::capture(env, tab, "light-unknown-static").await?;
    let reported = progress(tab).await?;
    let active = palette(&reported, "active_color")?;
    let mut counts = Vec::new();
    for (area, (id, _)) in areas.into_iter().zip(INDICATORS) {
        let mut ink = 0;
        for y in 0..(area.height as u32) {
            for x in 0..(area.width as u32) {
                if distance(
                    still.sample(area, f64::from(x) + 0.5, f64::from(y) + 0.5)?,
                    active,
                ) <= 12
                {
                    ink += 1;
                }
            }
        }
        check!(
            ink >= 10,
            "unknown {id} has a visible still frame: {ink} active pixels"
        );
        counts.push(ink);
    }
    Ok(json!({"ax":unknown,"css_pixel_ink_counts":counts}))
}

async fn media(tab: &Tab, preference: &str) -> Result<()> {
    tab.cdp(
        "Emulation.setEmulatedMedia",
        json!({"features":[
            {"name":"prefers-reduced-motion","value":preference}
        ]}),
    )
    .await?;
    tab.wait_for(&format!(
        "matchMedia({}).matches",
        js(format!("(prefers-reduced-motion: {preference})"))
    ))
    .await?;
    tab.settle().await
}

async fn pin_clock(tab: &Tab) -> Result<()> {
    // Keep UI input, requestAnimationFrame and WebGPU real. Two fixed clock
    // points avoid a slow screenshot landing on the same animation cycle.
    tab.eval(
        "(() => {
            const start = performance.now();
            window.progressTestElapsed = 0;
            Object.defineProperty(performance, 'now', {
                configurable:true, value:() => start + window.progressTestElapsed,
            });
        })()",
    )
    .await?;
    Ok(())
}

async fn advance(tab: &Tab, ms: u32) -> Result<()> {
    tab.eval(&format!("window.progressTestElapsed = {ms}"))
        .await?;
    tab.settle().await
}

fn movement(before: &Png, after: &Png, areas: [Rect; 4]) -> Result<[usize; 4]> {
    Ok([
        before.changed(after, areas[0])?,
        before.changed(after, areas[1])?,
        before.changed(after, areas[2])?,
        before.changed(after, areas[3])?,
    ])
}

async fn motion(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M43",
        "browser frames animate all unknown indicators and Reduce Motion freezes them",
    );
    media(tab, "no-preference").await?;
    click(tab, "progress-motion").await?;
    ready(tab, "p.running && !p.reduced_motion && p.elapsed_ms > 80").await?;
    let start = progress(tab).await?["elapsed_ms"]
        .as_f64()
        .context("animation clock")?;
    ready(tab, &format!("p.elapsed_ms > {}", start + 100.)).await?;
    let end = progress(tab).await?["elapsed_ms"]
        .as_f64()
        .context("advancing animation clock")?;
    click(tab, "progress-motion").await?;
    ready(tab, "!p.running").await?;

    pin_clock(tab).await?;
    click(tab, "progress-motion").await?;
    ready(tab, "p.running && p.elapsed_ms === 0").await?;
    let areas = boxes(tab).await?;
    advance(tab, 320).await?;
    ready(tab, "Math.abs(p.elapsed_ms - 320) < 0.01").await?;
    let first = Png::capture(env, tab, "unknown-live-320ms").await?;
    advance(tab, 840).await?;
    ready(tab, "Math.abs(p.elapsed_ms - 840) < 0.01").await?;
    let second = Png::capture(env, tab, "unknown-live-840ms").await?;
    let live_pixels = movement(&first, &second, areas)?;
    check!(
        live_pixels.iter().all(|changed| *changed > 12),
        "each unknown indicator repaints across live frames: {live_pixels:?}"
    );

    click(tab, "progress-mode-known").await?;
    ready(tab, "p.known && p.value === 0.25 && p.elapsed_ms === 0").await?;
    advance(tab, 1000).await?;
    ready(tab, "Math.abs(p.elapsed_ms - 160) < 0.01").await?;
    let known_first = Png::capture(env, tab, "known-live-160ms").await?;
    advance(tab, 1300).await?;
    ready(tab, "Math.abs(p.elapsed_ms - 460) < 0.01").await?;
    let known_second = Png::capture(env, tab, "known-live-460ms").await?;
    let known_pixels = movement(&known_first, &known_second, areas)?;
    check!(
        known_pixels[0] == 0
            && known_pixels[2] == 0
            && known_pixels[1] > 12
            && known_pixels[3] > 12,
        "known flat ink is stable and both expressive waves move: {known_pixels:?}"
    );

    media(tab, "reduce").await?;
    ready(tab, "p.running && p.reduced_motion").await?;
    let known_reduced_a = Png::capture(env, tab, "known-reduced-motion").await?;
    advance(tab, 1820).await?;
    let known_reduced_b = Png::capture(env, tab, "known-reduced-motion-later").await?;
    let known_reduced = movement(&known_reduced_a, &known_reduced_b, areas)?;
    check_eq!(
        known_reduced,
        [0; 4],
        "known indicators stay still with Reduce Motion"
    );

    click(tab, "progress-mode-unknown").await?;
    ready(tab, "!p.known && p.running && p.reduced_motion").await?;
    let reduced_a = Png::capture(env, tab, "unknown-reduced-motion").await?;
    advance(tab, 2340).await?;
    let reduced_b = Png::capture(env, tab, "unknown-reduced-motion-later").await?;
    let reduced_pixels = movement(&reduced_a, &reduced_b, areas)?;
    check_eq!(
        reduced_pixels,
        [0; 4],
        "unknown indicators stay still with Reduce Motion"
    );

    media(tab, "no-preference").await?;
    ready(tab, "!p.reduced_motion").await?;
    click(tab, "progress-motion").await?;
    ready(tab, "!p.running").await?;
    tab.eval("delete performance.now").await?;

    Ok(
        json!({"live_clock_interval_ms":[start,end],"unknown_live_changed_pixels":live_pixels,
        "known_live_changed_pixels":known_pixels,"unknown_reduced_changed_pixels":reduced_pixels,
        "known_reduced_changed_pixels":known_reduced}),
    )
}

async fn viewport(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{
            const viewport = JSON.parse(window.gallery.export_dom()).layout.viewport;
            return viewport.width === {width} && viewport.height === {height};
        }})()"
    ))
    .await?;
    tab.settle().await
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M44",
        "dark RTL at 320 dp with 200% text adapts further at 260 dp",
    );
    // Capture final theme colors and control states after the motion exercise.
    media(tab, "reduce").await?;
    ready(tab, "p.reduced_motion").await?;
    click(tab, "progress-mode-known").await?;
    click(tab, "theme").await?;
    click(tab, "progress-rtl").await?;
    click(tab, "progress-scale").await?;
    until(tab, "s.dark").await?;
    ready(
        tab,
        "p.known && p.value === 0.25 && p.rtl && p.text_scale === 2",
    )
    .await?;
    viewport(tab, 320, 680).await?;
    let areas = boxes(tab).await?;
    let report = progress(tab).await?;
    let active = palette(&report, "active_color")?;
    let track = palette(&report, "track_color")?;
    let image = Png::capture(env, tab, "dark-rtl-320-200").await?;
    let linear = flat_linear(&image, areas[0], active, track, true)?;
    let ring = flat_ring(&image, areas[2], active, track)?;
    let wave = wavy_ink(&image, areas[1], areas[3], active, true)?;
    let roles = semantics(tab, Some(0.25)).await?;
    viewport(tab, 260, 860).await?;
    let narrow_flat = visible_box(tab, INDICATORS[0].0).await?;
    let narrow_wave = visible_box(tab, INDICATORS[1].0).await?;
    check!(
        narrow_flat.width < areas[0].width && narrow_wave.width < areas[1].width,
        "both lines adapt to the 260 dp window"
    );
    let image = Png::capture(env, tab, "dark-rtl-260-200-linear").await?;
    let narrow_linear = flat_linear(&image, narrow_flat, active, track, true)?;
    viewport(tab, 320, 480).await?;
    let id = INDICATORS[3].0;
    let below = point(tab, id).await?;
    check!(
        below.b[1] + below.b[3] > 480.,
        "short viewport needs scroll"
    );
    tab.mouse_move(below.canvas[0] + 6., below.canvas[1] + 150.)
        .await?;
    let scale = tab
        .eval("devicePixelRatio")
        .await?
        .as_f64()
        .context("pixel scale")?;
    tab.wheel(0., below.b[1] * scale).await?;
    tab.wait_for(&format!(
        "(() => {{
            const [,y,,h] = window.gallery.control_bounds({});
            return h > 0 && y >= 0 && y + h <= 480;
        }})()",
        js(id),
    ))
    .await?;
    let revealed = point(tab, id).await?;
    let image = Png::capture(env, tab, "dark-rtl-320-480-scrolled").await?;
    let revealed_ink = image.sample(Rect::from_point(&revealed), 38.4, 9.6)?;
    painted(
        revealed_ink,
        active,
        "wheel reveals the wavy ring in a short window",
    )?;
    Ok(
        json!({"linear_pixels":linear,"circular_pixels":ring,"wave_pixels":wave,
        "ax":roles,"text_scale":report["text_scale"],
        "linear_320_width":areas[0].width,"linear_260_width":narrow_flat.width,
        "linear_260_pixels":narrow_linear,
        "short_viewport_scroll":{"before":below.b,"revealed":revealed.b,"active":revealed_ink}}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, tab: &Tab) -> Result<()> {
    crate::smoke::ready(tab, "gallery").await?;
    tab.cdp("Accessibility.enable", json!({})).await?;
    let adapter = tab
        .eval(
            "(async () => {
            const adapter = await navigator.gpu.requestAdapter();
            if (!adapter) return null;
            const {vendor, architecture, isFallbackAdapter} = adapter.info;
            return {vendor, architecture, isFallbackAdapter};
        })()",
        )
        .await?;
    check!(!adapter.is_null(), "WebGPU adapter is available");
    let known = known(env, tab).await?;
    let unknown = unknown(env, tab).await?;
    let motion = motion(env, tab).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(errors.is_empty(), "progress page errors: {errors:?}");
    env.write_json(
        "browser-material-progress.json",
        &json!({
            "browser":chrome.version,"adapter":adapter,"known":known,"unknown":unknown,
            "motion":motion,"compact":compact,"errors":errors
        }),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 844, 2.).await?;
    tab.goto(url).await?;
    let result = journey(env, chrome, &tab).await;
    if let Err(error) = &result {
        let dump = tab
            .eval(
                "(() => ({state:JSON.parse(window.gallery.inspect()),
               dom:JSON.parse(window.gallery.export_dom())}))()",
            )
            .await
            .unwrap_or_else(|err| json!({"dump_error":err.to_string()}));
        let _ = env.write_json(
            "material-progress-failure.json",
            &json!({"error":error.to_string(),"dump":dump}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-progress-failure.png"))
            .await;
    }
    result
}
