//! The docked picker uses a live field beside its calendar disclosure.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::{click, point, state, until};
use crate::browser::{js, Chrome, Tab};
use crate::check::step;
use crate::{check, check_eq, Env};

const PICKER: &str = "docked-date-picker-demo-picker";
const INPUT: &str = "docked-date-picker-demo-picker-input";
const OPENER: &str = "docked-date-picker-demo-picker-calendar-trigger";
const OUTSIDE: &str = "docked-date-picker-demo-outside";
const INITIAL: &str = "2024-02-28";
const TODAY: &str = "2024-02-29";

fn part(suffix: &str) -> String {
    format!("{PICKER}-{suffix}")
}

fn day(date: &str) -> String {
    part(&format!("day-{date}"))
}

fn projected(id: &str) -> String {
    format!(
        "document.querySelector({})",
        js(format!("#surface-accessibility [data-surface-id=\"{id}\"]"))
    )
}

fn overlaps(a: &[f64], b: &[f64]) -> bool {
    let x = (a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0]);
    let y = (a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1]);
    x > 0.75 && y > 0.75
}

async fn resize(tab: &Tab, width: u32, height: u32) -> Result<()> {
    tab.viewport(width, height + 44).await?;
    tab.wait_for(&format!(
        "(() => {{ const v = JSON.parse(window.gallery.export_dom()).layout?.viewport;
           return v?.width === {width} && v?.height === {height}; }})()"
    ))
    .await?;
    tab.settle().await
}

async fn picker_state(tab: &Tab, predicate: &str) -> Result<()> {
    until(tab, &format!("(d => {predicate})(s.docked_date_picker)")).await?;
    tab.settle().await
}

async fn reload(tab: &Tab, url: &str) -> Result<()> {
    tab.goto(url).await?;
    crate::smoke::ready(tab, "gallery").await?;
    resize(tab, 960, 980).await?;
    click(tab, "section-2").await?;
    until(tab, "s.section === 2").await?;
    click(tab, "docked-date-picker-demo-show").await?;
    picker_state(tab, "d.visible && d.mode === null && d.commits === 0").await?;
    resize(tab, 960, 800).await
}

async fn runtime_focused(tab: &Tab, id: &str) -> Result<()> {
    until(tab, &format!("s.focused === {}", js(id))).await
}

async fn focused(tab: &Tab, id: &str) -> Result<()> {
    runtime_focused(tab, id).await?;
    let expected = if id == INPUT {
        "document.getElementById('surface-text-input')".to_owned()
    } else {
        projected(id)
    };
    tab.wait_for(&format!("document.activeElement === {expected}"))
        .await?;
    tab.settle().await
}

async fn focus_projected(tab: &Tab, id: &str) -> Result<()> {
    let element = projected(id);
    tab.wait_for(&format!("{element} !== null")).await?;
    tab.eval(&format!("{element}.focus()")).await?;
    focused(tab, id).await
}

async fn open_with_key(tab: &Tab, key: &str) -> Result<()> {
    focus_projected(tab, OPENER).await?;
    tab.press(key).await?;
    picker_state(tab, "d.mode === 'Calendar'").await?;
    focused(tab, &day(INITIAL)).await
}

async fn closed(tab: &Tab, date: &str, commits: u32) -> Result<()> {
    picker_state(
        tab,
        &format!(
            "d.mode === null && d.committed === {} && d.commits === {commits}",
            js(date)
        ),
    )
    .await?;
    tab.wait_for(&format!(
        "{} === null && window.gallery.control_bounds({}).length === 0 &&
         {}?.getAttribute('aria-expanded') === 'false'",
        projected(&part("popup")),
        js(part("popup")),
        projected(OPENER),
    ))
    .await
}

async fn replace(tab: &Tab, raw: &str) -> Result<()> {
    tab.shortcut("a").await?;
    tab.insert_text(raw).await?;
    picker_state(tab, &format!("d.raw === {}", js(raw))).await
}

async fn capture(env: &Env, tab: &Tab, image: &str) -> Result<()> {
    tab.settle().await?;
    tab.screenshot(&env.artifact(&format!("material-docked-date-picker-{image}.png")))
        .await
}

async fn ax_tree(tab: &Tab) -> Result<Vec<Value>> {
    let mut tree = tab.cdp("Accessibility.getFullAXTree", json!({})).await?;
    match tree["nodes"].take() {
        Value::Array(nodes) => Ok(nodes),
        _ => anyhow::bail!("missing nodes in the Chrome accessibility tree"),
    }
}

fn ax<'a>(tree: &'a [Value], role: &str, label: &str) -> Result<&'a Value> {
    tree.iter()
        .find(|node| {
            node["ignored"] == false
                && node["role"]["value"] == role
                && node["name"]["value"]
                    .as_str()
                    .is_some_and(|name| name.contains(label))
        })
        .with_context(|| format!("Chrome accessibility lacks {role} named {label}"))
}

fn ax_property<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node["properties"]
        .as_array()?
        .iter()
        .find(|property| property["name"] == name)
        .map(|property| &property["value"]["value"])
}

async fn input_help(tab: &Tab, target: &str, words: &str) -> Result<Value> {
    tab.wait_for(&format!(
        "(() => {{
           const textbox = document.getElementById('surface-text-input');
           const ref = textbox?.getAttribute('aria-describedby');
           return ref && document.getElementById(ref) === {};
        }})()",
        projected(&part(target)),
    ))
    .await?;
    let nodes = ax_tree(tab).await?;
    let editor = ax(&nodes, "textbox", "日期")?;
    let description = editor["description"]["value"].as_str().unwrap_or_default();
    check!(
        description.contains(words) && description.contains("YYYY/MM/DD"),
        "Chrome exposes the editor hint or error and date format: {description}"
    );
    Ok(editor["description"].clone())
}

async fn typed_entry(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M116",
        "docked field and icon have separate Tab stops and typed dates commit on Enter or blur",
    );
    focus_projected(tab, OPENER).await?;
    closed(tab, INITIAL, 0).await?;
    tab.press("Shift+Tab").await?;
    focused(tab, INPUT).await?;
    tab.press("Tab").await?;
    focused(tab, OPENER).await?;
    tab.press("Tab").await?;
    focused(tab, OUTSIDE).await?;
    closed(tab, INITIAL, 0).await?;

    click(tab, INPUT).await?;
    focused(tab, INPUT).await?;
    closed(tab, INITIAL, 0).await?;
    let hint = input_help(tab, "hint", "YYYY/MM/DD").await?;
    replace(tab, "２０００／２／２９").await?;
    closed(tab, INITIAL, 0).await?;
    tab.press("Enter").await?;
    focused(tab, INPUT).await?;
    closed(tab, "2000-02-29", 1).await?;
    picker_state(tab, "d.raw === '2000/02/29' && d.error === null").await?;

    replace(tab, "2000/10/03").await?;
    closed(tab, "2000-02-29", 1).await?;
    tab.press("Tab").await?;
    runtime_focused(tab, OPENER).await?;
    closed(tab, "2000-10-03", 2).await?;

    tab.press("Shift+Tab").await?;
    focused(tab, INPUT).await?;
    replace(tab, "2100/2/29").await?;
    tab.press("Enter").await?;
    focused(tab, INPUT).await?;
    closed(tab, "2000-10-03", 2).await?;
    picker_state(tab, "d.error === 'InvalidDate' && d.raw === '2100/2/29'").await?;
    let error = input_help(tab, "error", "這個日期不存在").await?;
    capture(env, tab, "invalid-date").await?;

    replace(tab, "2000/10/04").await?;
    let trigger = point(tab, OPENER).await?;
    tab.mouse_move(trigger.x, trigger.y).await?;
    tab.mouse_down().await?;
    tab.settle().await?;
    runtime_focused(tab, INPUT).await?;
    closed(tab, "2000-10-03", 2).await?;
    tab.mouse_up().await?;
    picker_state(
        tab,
        "d.mode === 'Calendar' && d.committed === '2000-10-03' && d.commits === 2",
    )
    .await?;
    tab.press("Escape").await?;
    runtime_focused(tab, OPENER).await?;
    closed(tab, "2000-10-03", 2).await?;
    Ok(
        json!({"hint":hint,"error":error,"committed":"2000-10-03","commits":2,
        "held_opener_keeps_focus_and_does_not_commit":true}),
    )
}

async fn composing_entry(tab: &Tab) -> Result<Value> {
    step(
        "M117",
        "the first Enter belongs to IME composition and a later Enter commits once",
    );
    click(tab, INPUT).await?;
    focused(tab, INPUT).await?;
    replace(tab, "2024/02/").await?;
    tab.ime_compose("２９", 2, 2).await?;
    tab.wait_for(&format!(
        "JSON.parse(window.gallery.export_dom()).text_inputs[{}]?.composing === true &&
         document.getElementById('surface-text-input').value.includes('２９')",
        js(INPUT)
    ))
    .await?;
    tab.press("Enter").await?;
    focused(tab, INPUT).await?;
    closed(tab, INITIAL, 0).await?;
    let raw = state(tab).await?["docked_date_picker"]["raw"].clone();
    if raw == "2024/02/" {
        tab.insert_text("２９").await?;
    }
    picker_state(tab, "d.raw === '2024/02/２９' || d.raw === '2024/02/29'").await?;
    closed(tab, INITIAL, 0).await?;
    tab.press("Enter").await?;
    focused(tab, INPUT).await?;
    closed(tab, TODAY, 1).await?;
    picker_state(tab, "d.raw === '2024/02/29' && d.error === null").await?;
    Ok(json!({"ime_enter_commits":0,"separate_enter_commits":1,"committed":TODAY}))
}

async fn placement(tab: &Tab, above: bool) -> Result<Value> {
    let anchor = point(tab, &part("field")).await?.b;
    let popup = point(tab, &part("popup")).await?.b;
    if above {
        check!(
            popup[1] + popup[3] <= anchor[1] + 0.75,
            "popup clears the field above"
        );
    } else {
        check!(
            popup[1] >= anchor[1] + anchor[3] - 0.75,
            "popup clears the field below"
        );
    }
    check!(
        (popup[2] - 360.).abs() <= 0.75 && (popup[3] - 456.).abs() <= 0.75,
        "desktop popup matches the docked token bounds: {popup:?}"
    );
    Ok(json!({"anchor":anchor,"popup":popup,"above":above}))
}

async fn calendar(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M118",
        "docked calendar exposes a date grid and commits one keyboard choice",
    );
    open_with_key(tab, "Enter").await?;
    let position = placement(tab, false).await?;
    let disclosure = projected(OPENER);
    check_eq!(
        tab.eval(&format!("{disclosure}?.getAttribute('aria-expanded')"))
            .await?,
        json!("true"),
        "the calendar button reports its expanded state"
    );
    let nodes = ax_tree(tab).await?;
    let grid = ax(&nodes, "grid", "2024年2月")?;
    let header = ax(&nodes, "columnheader", "星期日")?;
    let selected = ax(&nodes, "gridcell", "2024年2月28日")?;
    let unavailable = ax(&nodes, "gridcell", "2024年2月27日")?;
    check_eq!(
        ax_property(selected, "selected"),
        Some(&json!(true)),
        "selected day"
    );
    check_eq!(
        ax_property(unavailable, "disabled"),
        Some(&json!(true)),
        "unavailable day"
    );
    ax(&nodes, "textbox", "日期")?;
    let today = projected(&day(TODAY));
    check_eq!(
        tab.eval(&format!(
            "({{current:{today}?.getAttribute('aria-current'), selected:{today}?.getAttribute('aria-selected')}})"
        ))
        .await?,
        json!({"current":"date","selected":"false"}),
        "today and selection are distinct"
    );
    capture(env, tab, "calendar-light-below").await?;

    tab.press("Tab").await?;
    focused(tab, INPUT).await?;
    picker_state(tab, "d.mode === 'Calendar'").await?;
    tab.press("Shift+Tab").await?;
    focused(tab, &day(INITIAL)).await?;
    tab.press("ArrowRight").await?;
    focused(tab, &day(TODAY)).await?;
    focus_projected(tab, &day(INITIAL)).await?;
    tab.press("ArrowRight").await?;
    focused(tab, &day(TODAY)).await?;
    tab.press("ArrowRight").await?;
    focused(tab, &day("2024-03-02")).await?;
    tab.press("Shift+PageDown").await?;
    focused(tab, &day("2025-03-02")).await?;
    picker_state(tab, "d.committed === '2024-02-28' && d.commits === 0").await?;
    tab.press("Enter").await?;
    focused(tab, OPENER).await?;
    closed(tab, "2025-03-02", 1).await?;
    picker_state(tab, "d.raw === '2025/03/02'").await?;
    Ok(
        json!({"placement":position,"grid":grid["role"],"weekday":header["name"],
        "selected":selected["properties"],"unavailable":unavailable["properties"],"committed":"2025-03-02"}),
    )
}

async fn lists(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M119",
        "month and bounded year lists browse by keyboard without committing a date",
    );
    open_with_key(tab, "Space").await?;
    focus_projected(tab, &part("month")).await?;
    tab.press("Enter").await?;
    picker_state(tab, "d.mode === 'Month'").await?;
    let month_list = point(tab, &part("month-list")).await?.b;
    focused(tab, &part("month-02")).await?;
    tab.press("ArrowDown").await?;
    focused(tab, &part("month-03")).await?;
    capture(env, tab, "month-list").await?;
    tab.press("Enter").await?;
    picker_state(
        tab,
        "d.mode === 'Calendar' && d.month[0] === 2024 && d.month[1] === 3",
    )
    .await?;
    focused(tab, &part("month")).await?;

    focus_projected(tab, &part("year")).await?;
    tab.press("Enter").await?;
    picker_state(tab, "d.mode === 'Year'").await?;
    let year_list = point(tab, &part("year-list")).await?.b;
    focused(tab, &part("year-2024")).await?;
    let year_count = tab.eval(&format!(
        "(() => {{const prefix = {}; return JSON.parse(window.gallery.export_dom()).dom.nodes
         .filter(n => n.stable_id?.startsWith(prefix) && /^\\d{{4}}$/.test(n.stable_id.slice(prefix.length))).length; }})()",
        js(part("year-")),
    )).await?;
    check_eq!(
        year_count,
        json!(72),
        "the current year page has a bounded row count"
    );
    tab.press("ArrowDown").await?;
    focused(tab, &part("year-2025")).await?;
    capture(env, tab, "year-list").await?;
    tab.press("Enter").await?;
    picker_state(
        tab,
        "d.mode === 'Calendar' && d.month[0] === 2025 && d.month[1] === 3",
    )
    .await?;
    focused(tab, &part("year")).await?;
    tab.press("Enter").await?;
    focused(tab, &part("year-2025")).await?;
    tab.press("Escape").await?;
    focused(tab, &part("year")).await?;
    picker_state(tab, "d.mode === 'Calendar' && d.commits === 0").await?;
    tab.press("Escape").await?;
    focused(tab, OPENER).await?;
    closed(tab, INITIAL, 0).await?;
    Ok(
        json!({"month_list":month_list,"year_list":year_list,"year_count":year_count,
        "browsed_to":[2025,3],"commits":0}),
    )
}

async fn dismissal(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M120",
        "the dark popup flips above and outside mouse or touch dismisses without click-through",
    );
    click(tab, "docked-date-picker-demo-theme").await?;
    click(tab, "docked-date-picker-demo-bottom").await?;
    click(tab, OPENER).await?;
    runtime_focused(tab, &day(INITIAL)).await?;
    let position = placement(tab, true).await?;
    capture(env, tab, "calendar-dark-above").await?;
    let popup = point(tab, &part("popup")).await?;
    tab.click(popup.canvas[0] + popup.b[0] + 3., popup.y)
        .await?;
    picker_state(tab, "d.mode === 'Calendar' && d.commits === 0").await?;
    click(tab, OUTSIDE).await?;
    runtime_focused(tab, OPENER).await?;
    closed(tab, INITIAL, 0).await?;
    picker_state(tab, "d.outside_actions === 0").await?;

    click(tab, OPENER).await?;
    runtime_focused(tab, &day(INITIAL)).await?;
    let outside = point(tab, OUTSIDE).await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchStart","touchPoints":[{"x":outside.x,"y":outside.y,"id":1}]}),
    )
    .await?;
    tab.cdp(
        "Input.dispatchTouchEvent",
        json!({"type":"touchEnd","touchPoints":[]}),
    )
    .await?;
    runtime_focused(tab, OPENER).await?;
    closed(tab, INITIAL, 0).await?;
    picker_state(tab, "d.outside_actions === 0").await?;

    click(tab, OPENER).await?;
    runtime_focused(tab, &day(INITIAL)).await?;
    click(tab, INPUT).await?;
    focused(tab, INPUT).await?;
    replace(tab, "2024/02/29").await?;
    click(tab, &part("next")).await?;
    picker_state(tab, "d.mode === 'Calendar' && d.month[1] === 3 && d.committed === '2024-02-28' && d.commits === 0")
        .await?;
    click(tab, INPUT).await?;
    focused(tab, INPUT).await?;
    replace(tab, "2024/03/02").await?;
    tab.press("Escape").await?;
    runtime_focused(tab, OPENER).await?;
    closed(tab, INITIAL, 0).await?;
    Ok(
        json!({"placement":position,"outside_mouse":true,"outside_touch":true,"outside_actions":0,"escape_commits":0}),
    )
}

async fn compact(env: &Env, tab: &Tab) -> Result<Value> {
    step(
        "M121",
        "320 dp dark RTL at 200 percent retains touch targets and date focus",
    );
    for setting in ["theme", "rtl", "large-text"] {
        click(tab, &format!("docked-date-picker-demo-{setting}")).await?;
    }
    resize(tab, 320, 720).await?;
    until(
        tab,
        "s.dark && s.docked_date_picker.rtl && s.docked_date_picker.text_scale === 2",
    )
    .await?;
    let field = point(tab, &part("field")).await?.b;
    let committed_label = point(tab, "docked-date-picker-demo-status").await?.b;
    let outside = point(tab, OUTSIDE).await?.b;
    check!(
        !overlaps(&field, &committed_label) && !overlaps(&field, &outside),
        "the field stays clear of the committed date and outside action at 200%: {field:?}, {committed_label:?}, {outside:?}"
    );
    open_with_key(tab, "Enter").await?;
    let popup = point(tab, &part("popup")).await?.b;
    check!(
        popup[0] >= -0.75
            && popup[0] + popup[2] <= 320.75
            && popup[1] >= -0.75
            && popup[1] + popup[3] <= 720.75,
        "the compact popup stays in the viewport: {popup:?}"
    );
    let mut targets = Vec::new();
    for id in [
        OPENER.to_owned(),
        part("prev"),
        part("month"),
        part("year"),
        part("next"),
    ] {
        let bounds = point(tab, &id).await?.b;
        check!(
            bounds[2] >= 47.5 && bounds[3] >= 47.5,
            "48 dp target for {id}: {bounds:?}"
        );
        targets.push((id, bounds));
    }
    for (i, (id, a)) in targets.iter().enumerate() {
        for (other, b) in targets.iter().skip(i + 1) {
            check!(
                !overlaps(a, b),
                "{id} and {other} have separate hit targets"
            );
        }
    }
    tab.press("ArrowLeft").await?;
    focused(tab, &day(TODAY)).await?;
    let cell = point(tab, &day(TODAY)).await?.b;
    check!(
        cell[2] >= 47.5 && cell[3] >= 47.5,
        "the day stays 48 dp: {cell:?}"
    );
    capture(env, tab, "compact-dark-rtl-200").await?;
    tab.press("Escape").await?;
    focused(tab, OPENER).await?;
    closed(tab, INITIAL, 0).await?;
    Ok(
        json!({"popup":popup,"day":cell,"controls":targets,"field":field,"committed_label":committed_label,"outside":outside}),
    )
}

async fn journey(env: &Env, chrome: &Chrome, url: &str, tab: &Tab) -> Result<()> {
    reload(tab, url).await?;
    let typed = typed_entry(env, tab).await?;
    reload(tab, url).await?;
    let composition = composing_entry(tab).await?;
    reload(tab, url).await?;
    let calendar = calendar(env, tab).await?;
    reload(tab, url).await?;
    let lists = lists(env, tab).await?;
    reload(tab, url).await?;
    let dismissal = dismissal(env, tab).await?;
    reload(tab, url).await?;
    let compact = compact(env, tab).await?;
    let errors = tab.errors();
    check!(
        errors.is_empty(),
        "docked date picker page errors: {errors:?}"
    );
    env.write_json(
        "browser-material-docked-date-picker.json",
        &json!({"browser":chrome.version,"typed":typed,"composition":composition,"calendar":calendar,
            "lists":lists,"dismissal":dismissal,"compact":compact,"errors":errors}),
    )
}

pub(super) async fn run(env: &Env, chrome: &Chrome, url: &str) -> Result<()> {
    let tab = chrome.page(960, 844, 2.).await?;
    let result = journey(env, chrome, url, &tab).await;
    if let Err(error) = &result {
        let _ = env.write_json(
            "material-docked-date-picker-failure.json",
            &json!({"error":format!("{error:#}"),"state":state(&tab).await.ok(),
                "dom":tab.eval_json("window.gallery.export_dom()").await.ok(),
                "active":tab.eval("document.activeElement?.outerHTML").await.ok(),"errors":tab.errors()}),
        );
        let _ = tab
            .screenshot(&env.artifact("material-docked-date-picker-failure.png"))
            .await;
    }
    result
}
