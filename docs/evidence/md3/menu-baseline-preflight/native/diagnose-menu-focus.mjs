import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdir, mkdtemp, readFile, readdir, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const [web, output] = process.argv.slice(2).map(path => resolve(path));
const files = new Set(await readdir(web));
const types = {html:"text/html;charset=utf-8", js:"application/javascript", wasm:"application/wasm"};
await mkdir(output);
const server = createServer(async (request, response) => {
  const file = request.url === "/" ? "index.html" : request.url.slice(1);
  if (request.method !== "GET" || !files.has(file)) return response.writeHead(404).end();
  const bytes = await readFile(resolve(web, file));
  response.writeHead(200, {"content-type": types[file.split(".").pop()] ?? "text/plain"});
  response.end(bytes);
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const profile = await mkdtemp(resolve(output, "chrome-profile."));
const child = spawn("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", [
  "--headless=new", "--no-first-run", "--no-default-browser-check",
  "--disable-background-networking", "--remote-debugging-pipe",
  "--user-data-dir=" + profile, "about:blank",
], {stdio:["ignore", "ignore", "ignore", "pipe", "pipe"]});
const reportedExit = once(child, "exit");
const [input, replies] = child.stdio.slice(3);
const pending = new Map();
let sequence = 0;
let remainder = "";
let sessionId;
input.on("error", () => {});
replies.setEncoding("utf8").on("data", contents => {
  remainder += contents;
  const packets = remainder.split("\0");
  remainder = packets.pop();
  for (const packet of packets) {
    const reply = JSON.parse(packet);
    const awaiting = pending.get(reply.id);
    if (!awaiting) continue;
    clearTimeout(awaiting.timeout);
    pending.delete(reply.id);
    if (reply.error) awaiting.reject(new Error(reply.error.message));
    else awaiting.resolve(reply.result);
  }
});
const send = (method, params = {}) => new Promise((resolve, reject) => {
  const id = ++sequence;
  const timeout = setTimeout(() => {
    pending.delete(id);
    reject(new Error("CDP call timed out: " + method));
  }, 5000);
  pending.set(id, {resolve, reject, timeout});
  input.write(JSON.stringify({id, method, params, sessionId}) + "\0");
});
const evaluate = async expression => {
  const reply = await send("Runtime.evaluate", {expression, returnByValue:true});
  assert.ok(!reply.exceptionDetails, reply.exceptionDetails?.exception?.description);
  return reply.result.value;
};
const until = async expression => {
  for (let count = 0; count < 240; ++count) {
    const current = await evaluate(expression);
    if (current) return current;
    await delay(40);
  }
  throw new Error("Timed out waiting for " + expression);
};
const projected = id => `document.querySelector('#surface-accessibility [data-surface-id="${id}"]')`;
const state = (condition) => until(`window.gallery && (s => ${condition})(JSON.parse(window.gallery.inspect()))`);
const focus = async id => {
  const control = projected(id);
  await until(`Boolean(${control})`);
  await evaluate(`${control}.focus()`);
  await state(`s.focused === '${id}'`);
  await until(`document.activeElement === ${control}`);
};
const keyCodes = {Enter:13, ArrowDown:40, ArrowUp:38, " ":32};
const press = async key => {
  const code = key === " " ? "Space" : key;
  for (const type of ["keyDown", "keyUp"]) {
    await send("Input.dispatchKeyEvent", {type, key, code, windowsVirtualKeyCode:keyCodes[key]});
  }
};
const setViewport = height => send("Emulation.setDeviceMetricsOverride", {
  width:960, height:height + 44, deviceScaleFactor:2, mobile:false,
});
const snap = () => evaluate("window.menuFocusOracle.snapshot()");

try {
  const version = await send("Browser.getVersion");
  const {targetInfos} = await send("Target.getTargets");
  const {targetId} = targetInfos.find(target => target.type === "page");
  ({sessionId} = await send("Target.attachToTarget", {targetId, flatten:true}));
  await send("Emulation.setFocusEmulationEnabled", {enabled:true});
  await setViewport(2040);
  await send("Page.navigate", {url:`http://127.0.0.1:${server.address().port}/`});
  await state("typeof s.section === 'number'");
  await focus("section-2");
  await press("Enter");
  await state("s.section === 2");
  await focus("menu-demo-show");
  await press("Enter");
  await state("s.menu_demo.visible && s.focused === 'menu-demo-back'");
  await setViewport(800);
  await until("JSON.parse(window.gallery.export_dom()).layout?.viewport?.height === 800");
  await focus("menu-demo-opener");
  await press("Enter");
  await state("s.menu_demo.open && s.focused === 'menu-demo-inbox'");
  await until(`document.activeElement === ${projected("menu-demo-inbox")}`);
  await evaluate(`(() => {
    const events = [];
    const describe = target => ({id:target.id || null, tag:target.tagName,
      stableId:target.getAttribute('data-surface-id'),
      disabled:target.getAttribute('aria-disabled'), role:target.getAttribute('role'),
      action:target.getAttribute('data-surface-action')});
    for (const type of ['focusin', 'focusout', 'pointerdown', 'pointerup', 'click', 'keydown', 'keyup']) {
      document.addEventListener(type, event => {
        events.push({type, key:event.key ?? null, trusted:event.isTrusted,
          target:describe(event.target), active:describe(document.activeElement)});
      }, true);
    }
    window.menuFocusOracle = {events, snapshot:() => {
      const {focused, menu_demo} = JSON.parse(window.gallery.inspect());
      return {focused, menu:menu_demo, active:describe(document.activeElement),
        find:describe(${projected("menu-demo-find")}),
        unavailable:describe(${projected("menu-demo-archive")})};
    }};
  })()`);
  const report = {schema:"surface.test.command-menu-focus-ownership.v1", browser:version.product,
                  headless:true};
  await press("ArrowDown");
  await state("s.focused === 'menu-demo-archive'");
  await until(`document.activeElement === ${projected("menu-demo-archive")}`);
  report.projectedDisabled = await snap();
  assert.equal(report.projectedDisabled.active.disabled, "true");
  assert.equal(report.projectedDisabled.active.action, null);
  await press("Enter");
  await press(" ");
  await state("s.menu_demo.open && s.menu_demo.last === null && s.focused === 'menu-demo-archive'");
  await press("ArrowDown");
  await state("s.focused === 'menu-demo-find'");
  await until(`document.activeElement === ${projected("menu-demo-find")}`);
  report.projectedFind = await snap();
  await press("ArrowUp");
  await state("s.focused === 'menu-demo-archive'");
  await until(`document.activeElement === ${projected("menu-demo-archive")}`);

  const [x, y] = await evaluate(`(() => {
    const [x, y, width, height] = window.gallery.control_bounds('menu-demo-archive');
    const canvas = document.querySelector('canvas').getBoundingClientRect();
    return [canvas.left + x + 0.5 * width, canvas.top + y + 0.5 * height];
  })()`);
  for (const [type, button, buttons] of [["mouseMoved", "none", 0],
    ["mousePressed", "left", 1], ["mouseReleased", "left", 0]]) {
    await send("Input.dispatchMouseEvent", {type, x, y, button, buttons, clickCount:1});
  }
  await until("document.activeElement === document.querySelector('canvas')");
  await state("s.menu_demo.open && s.menu_demo.last === null && s.focused === 'menu-demo-archive'");
  report.afterDisabledCanvasClick = await snap();
  await press("ArrowDown");
  await state("s.menu_demo.open && s.menu_demo.last === null && s.focused === 'menu-demo-find'");
  report.afterCanvasArrowDown = await snap();
  assert.equal(report.afterCanvasArrowDown.active.id, "surface");
  assert.equal(report.afterCanvasArrowDown.find.action, "press");
  await focus("menu-demo-find");
  report.afterProjectedReentry = await snap();
  assert.equal(report.afterProjectedReentry.active.stableId, "menu-demo-find");
  assert.equal(report.afterProjectedReentry.focused, "menu-demo-find");
  report.events = await evaluate("window.menuFocusOracle.events");
  for (const type of ["pointerdown", "pointerup", "click"]) {
    assert.ok(report.events.some(event => event.type === type && event.trusted &&
       event.target.id === "surface"), type + " must reach Canvas as trusted input");
  }
  await writeFile(resolve(output, "dom-focus-diagnostic.json"), JSON.stringify(report, null, 2) + "\n");
  console.log("PASS projected arrows, inert disabled Canvas click, Canvas arrow, and projected reentry");
} finally {
  sessionId = undefined;
  try { await send("Browser.close"); }
  catch { child.kill(); }
  await reportedExit;
  server.close();
}
