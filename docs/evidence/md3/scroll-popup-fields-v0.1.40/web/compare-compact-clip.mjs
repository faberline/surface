import {spawn} from "node:child_process";
import {once} from "node:events";
import {mkdtemp, readFile, writeFile} from "node:fs/promises";
import {createServer} from "node:http";
import {extname, join} from "node:path";
import assert from "node:assert/strict";

const [before, after, out] = process.argv.slice(2);
const profile = await mkdtemp(join(out, "compact-clip-chrome."));
const roots = {before, after};
const chrome = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const contents = {
  ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json"
};
const server = createServer(async (request, response) => {
  const match = /^\/(before|after)\/(?:([a-zA-Z0-9_.-]+))?$/.exec(request.url);
  if (!match) return response.writeHead(404).end();
  const [, root, file = "index.html"] = match;
  const type = contents[extname(file)];
  if (!type) return response.writeHead(404).end();
  try {
    response.writeHead(200, {"Content-Type": type}).end(await readFile(join(roots[root], file)));
  } catch {
    response.end();
  }
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const child = spawn(chrome, [
  "--headless=new", "--remote-debugging-port=0", "--no-first-run",
  "--disable-background-networking", "--no-default-browser-check",
  "--user-data-dir=" + profile, "about:blank",
], {stdio: "ignore"});
const exited = once(child, "exit");
let socket, sessionId;
let serial = 0;
const waiting = new Map();
let pageErrors = [];
const sleep = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));
async function poll(work, timeout = 8000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const val = await work();
    if (val) return val;
    await sleep(33);
  }
  throw Error("probe condition timed out");
}
function send(method, params = {}, scoped = true) {
  const id = ++serial;
  const options = (scoped && sessionId) ? {sessionId} : {};
  return new Promise((resolve, reject) => {
    waiting.set(id, {resolve, reject});
    socket.send(JSON.stringify({id, method, params, ...options}));
  });
}
async function evaluate(expression) {
  const reply = await send("Runtime.evaluate", {expression, returnByValue: true, awaitPromise: true});
  if (reply.exceptionDetails) throw Error(reply.exceptionDetails.text);
  return reply.result.value;
}
async function waitFor(expression) {
  return poll(() => evaluate(expression));
}
async function activate(id) {
  const lookup = `document.querySelector('[data-surface-id="${id}"]')`;
  try {
    await waitFor(`!!(${lookup})`);
    await evaluate(`${lookup}.focus()`);
    await waitFor(`document.activeElement === ${lookup} && JSON.parse(window.gallery.inspect()).focused === ${JSON.stringify(id)}`);
    const key = {key: "Enter", code: "Enter", windowsVirtualKeyCode: 13};
    await send("Input.dispatchKeyEvent", {type: "keyDown", text: "\r", unmodifiedText: "\r", ...key});
    await send("Input.dispatchKeyEvent", {type: "keyUp", ...key});
    await evaluate("new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))");
  } catch (error) {
    throw Error(`activating ${id}: ${error.message}`);
  }
}
function resize(width, height) {
  return send("Emulation.setDeviceMetricsOverride", {
    width, height: height + 44, deviceScaleFactor: 2, mobile: false
  });
}
try {
  const [port, browser] = (await poll(async () => (
    readFile(join(profile, "DevToolsActivePort"), "utf8").catch(() => null)
  ))).trim().split("\n");
  socket = new WebSocket(`ws://127.0.0.1:${port}${browser}`);
  await once(socket, "open");
  socket.addEventListener("message", ({data}) => {
    const message = JSON.parse(data);
    if (message.method === "Runtime.exceptionThrown")
      pageErrors.push(message.params.exceptionDetails.text);
    const task = waiting.get(message.id);
    if (task) {
      waiting.delete(message.id);
      if (message.error) task.reject(Error(message.error.message));
      else task.resolve(message.result);
    }
  });
  const version = (await send("Browser.getVersion")).product;
  const {targetId} = await send("Target.createTarget", {url: "about:blank"});
  ({sessionId} = await send("Target.attachToTarget", {targetId, flatten: true}));
  await send("Runtime.enable");
  await send("Page.enable");
  const samples = {};
  for (const variant of ["before", "after"]) {
    await resize(960, 2000);
    await send("Page.navigate", {url: `http://127.0.0.1:${server.address().port}/${variant}/`});
    await waitFor(`location.pathname === '/${variant}/' && !!window.gallery?.inspect`);
    await activate("section-0");
    await activate("scroll-popup-demo-show");
    for (const id of ["theme","rtl","large-text","bottom"]) await activate(`scroll-popup-demo-${id}`);
    await resize(320, 400);
    const state = "JSON.parse(window.gallery.inspect()).scroll_popup";
    await waitFor(`JSON.parse(window.gallery.export_dom()).layout?.viewport?.height === 400 && ${state}.anchor !== null`);
    await activate("scroll-popup-demo-opener");
    await waitFor(`${state}.open && ${state}.above`);
    const geometry = await evaluate(`(() => {
      const outside = window.gallery.control_bounds("scroll-popup-demo-outside");
      const label = document.querySelector('[data-surface-id="scroll-popup-demo-outside"]').getAttribute("aria-label");
      return {outside, label, anchor: ${state}.anchor, dialog: ${state}.bounds};
    })()`);
    geometry.clipped_height = Math.max(0, geometry.outside[1] + geometry.outside[3] - 400);
    samples[variant] = geometry;
  }
  assert.equal(samples.before.label, "外部操作 0 次");
  assert.ok(samples.before.clipped_height > 0, samples.before);
  assert.equal(samples.after.label, "外部 0 次");
  assert.equal(samples.after.clipped_height, 0);
  assert.deepEqual(pageErrors, []);
  const report = {version, headless: true, viewport: [320,400], samples, pageErrors};
  await writeFile(join(out, "compact-clip-geometry.json"), JSON.stringify(report, null, 2) + "\n");
  console.log(JSON.stringify(report));
} finally {
  if (socket?.readyState === WebSocket.OPEN) {
    await Promise.race([send("Browser.close", {}, false), sleep(1000)]).catch(() => {});
    socket.close();
  }
  if (child.exitCode === null) child.kill();
  await exited.catch(() => {});
  server.close();
}
