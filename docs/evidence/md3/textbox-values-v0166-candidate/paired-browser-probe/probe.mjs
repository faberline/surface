import assert from "node:assert/strict";
import {spawn} from "node:child_process";
import {createHash} from "node:crypto";
import {once} from "node:events";
import {readFile, writeFile} from "node:fs/promises";
import {createServer} from "node:http";
import {extname, join} from "node:path";

const [root, out, guard, expectation] = process.argv.slice(2);
assert.ok(expectation === "missing" || expectation === "present");
const assetPins = JSON.parse(await readFile(join(out, "asset-pins.json"), "utf8"));
const mime = {
  ".html": "text/html; charset=utf-8", ".js": "text/javascript",
  ".wasm": "application/wasm", ".json": "application/json",
};
const requests = {};
const server = createServer(async (request, response) => {
  const match = /^\/([A-Za-z0-9_.-]+)?$/.exec(request.url);
  const name = match && (match[1] || "index.html");
  if (!name || !Object.hasOwn(assetPins, name)) {
    response.writeHead(404).end();
    return;
  }
  try {
    const bytes = await readFile(join(root, name));
    const hash = createHash("sha256").update(bytes).digest("hex");
    assert.equal(hash, assetPins[name]);
    requests[name] = hash;
    response.writeHead(200, {"Content-Type": mime[extname(name)], "Cache-Control": "no-store"}).end(bytes);
  } catch (error) {
    response.writeHead(500).end(error.message);
  }
});
server.listen(0, "127.0.0.1");
await once(server, "listening");

const profile = join(out, "tmp", "profile");
const child = spawn(guard, [
  "--headless=new", "--remote-debugging-address=127.0.0.1", "--remote-debugging-port=0",
  "--no-first-run", "--no-default-browser-check", "--disable-background-networking",
  "--disable-extensions", "--disable-sync", "--disable-default-apps", "--mute-audio",
  "--user-data-dir=" + profile, "about:blank",
], {stdio: ["ignore", "ignore", "inherit"]});
const exited = once(child, "exit");
const sleep = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));
let socket, session;
let serial = 0;
const pending = new Map();
const pageErrors = [];
const phases = {};

function send(method, params = {}, scoped = true) {
  const id = ++serial;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(Error(`CDP timed out in ${method}`));
    }, 8000);
    pending.set(id, {resolve, reject, timer});
    socket.send(JSON.stringify({id, method, params, ...(scoped && session ? {sessionId: session} : {})}));
  });
}

async function evaluate(expression) {
  const reply = await send("Runtime.evaluate", {expression, returnByValue: true, awaitPromise: true});
  if (reply.exceptionDetails) throw Error(reply.exceptionDetails.text);
  return reply.result.value;
}

async function waitFor(label, test) {
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    const value = await test();
    if (value) return value;
    await sleep(30);
  }
  throw Error(`timed out waiting for ${label}`);
}

const state = "JSON.parse(window.gallery.inspect())";
const layout = "JSON.parse(window.gallery.export_dom())";
const demo = `(${state}.multiline_text_field_demo)`;
const area = "document.getElementById('surface-text-input')";
const prefix = "multiline-text-field-demo-";
const show = prefix + "show";
const grow = prefix + "grow";
const fixed = prefix + "fixed";
const readonly = prefix + "read-only";
const disabled = prefix + "disabled";
const action = grow + "-trailing-action";
const parent = prefix + "scroll";
const initial = "第一行";
const updated = "  甲  \n乙 a\u0301\n";
const long = "第一行😀\n第二行 a\u0301\n第三行 ABC 中文文字\n第四行\n第五行\n第六行\n第七行\n第八行\n";
const stored = "已儲存的第一行\n已儲存的第二行\n";
const stopped = "停用的第一行\n停用的第二行";
const names = new Map([[grow, "多行備註"], [fixed, "詳細說明"], [readonly, "唯讀內容"], [disabled, "停用內容"]]);

const selector = id => `document.querySelector('[data-surface-id="${id}"]')`;
async function phase(name, ids) {
  const {nodes} = await send("Accessibility.getFullAXTree");
  const byId = new Map(nodes.map(node => [node.nodeId,node]));
  const dom = await evaluate(`(() => {
    const host = ${area};
    return {focus: ${state}.focused, active: document.activeElement?.id ?? null,
      host: {hidden: host.hidden, readOnly: host.readOnly,
        value: host.hidden ? null : host.value, label: host.getAttribute("aria-label")},
      proxies: Object.fromEntries(${JSON.stringify(ids)}.map(id => {
        const node = document.querySelector('[data-surface-id="' + id + '"]');
        return [id,node ? {text: node.textContent, whiteSpace: getComputedStyle(node).whiteSpace,
          hiddenCarrier: node.querySelector('span[aria-hidden="true"]')?.textContent ?? null} : null];
      }))};
  })()`);
  const fields = Object.fromEntries(ids.map(id => {
    const matches = nodes.filter(node => !node.ignored && node.role?.value === "textbox"
      && node.name?.value === names.get(id));
    assert.equal(matches.length, 1, `${name} should have one textbox named ${names.get(id)}`);
    const node = matches[0];
    const props = Object.fromEntries(node.properties.map(property => [property.name, property.value.value]));
    assert.equal(props.multiline, true);
    if (dom.proxies[id]) {
      const pending = [...(node.childIds ?? [])];
      while (pending.length) {
        const child = byId.get(pending.pop());
        if (!child) continue;
        assert.ok(child.ignored || !["StaticText", "InlineTextBox"].includes(child.role?.value),
          `${name} has a duplicate readable value child`);
        pending.push(...(child.childIds ?? []));
      }
    }
    return [id, {name: node.name.value, backend: node.backendDOMNodeId, value: node.value?.value ?? null,
      description: node.description?.value ?? null, disabled: props.disabled ?? false,
      readonly: props.readonly ?? false, multiline: props.multiline}];
  }));
  phases[name] = {dom, fields};
  return phases[name];
}

async function resize(height) {
  await send("Emulation.setDeviceMetricsOverride", {width:960, height:height + 44, deviceScaleFactor:2, mobile:false});
  await waitFor("Canvas resize", () => evaluate(`${layout}.layout?.viewport?.height === ${height}`));
}

async function focus(id, editor = false) {
  await waitFor(id, () => evaluate(`!!${selector(id)}`));
  await evaluate(`${selector(id)}.focus()`);
  await waitFor("host focus", () => evaluate(`${state}.focused === ${JSON.stringify(id)}
    && document.activeElement === ${editor ? area : selector(id)}`));
}

async function key(key, code, number, options = {}) {
  const data = {key, code, windowsVirtualKeyCode:number, ...options};
  await send("Input.dispatchKeyEvent", {type:options.text ? "keyDown" : "rawKeyDown", ...data});
  await send("Input.dispatchKeyEvent", {type:"keyUp", key, code, windowsVirtualKeyCode:number,
    modifiers:options.modifiers ?? 0});
}

async function selectAll() {
  const meta = {key:"Meta", code:"MetaLeft", windowsVirtualKeyCode:91, location:1};
  await send("Input.dispatchKeyEvent", {type:"rawKeyDown", modifiers:4, ...meta});
  await key("a", "KeyA", 65, {modifiers:4, commands:["selectAll"]});
  await send("Input.dispatchKeyEvent", {type:"keyUp", modifiers:0, ...meta});
}

async function revealLocked() {
  const [parentId, roId, disId] = [parent, readonly, disabled];
  const area = await evaluate(`(() => {
    const [x,y,_,height] = window.gallery.control_bounds(${JSON.stringify(parentId)});
    const canvas = document.getElementById("surface").getBoundingClientRect();
    return [canvas.x + x + 4, canvas.y + y + height / 2];
  })()`);
  const [x, y] = area;
  assert.ok(Number.isFinite(x) && Number.isFinite(y));
  await send("Input.dispatchMouseEvent", {type:"mouseMoved", x, y});
  await send("Input.dispatchMouseEvent", {type:"mouseWheel", x, y, deltaX:0, deltaY:720});
  await waitFor("locked textbox proxies", () => evaluate(`!!${selector(roId)} && !!${selector(disId)}`));
}

try {
  const connection = await waitFor("DevTools port", () => readFile(join(profile, "DevToolsActivePort"),"utf8").catch(() => null));
  const [port, browserPath] = connection.trim().split("\n");
  socket = new WebSocket(`ws://127.0.0.1:${port}${browserPath}`);
  await once(socket, "open");
  socket.addEventListener("message", ({data}) => {
    const message = JSON.parse(data);
    if (message.method === "Runtime.exceptionThrown")
      pageErrors.push(message.params.exceptionDetails.exception?.description ?? message.params.exceptionDetails.text);
    if (message.method === "Log.entryAdded" && message.params.entry.level === "error")
      pageErrors.push(message.params.entry.text);
    const task = pending.get(message.id);
    if (task) {
      pending.delete(message.id);
      clearTimeout(task.timer);
      if (message.error) task.reject(Error(message.error.message));
      else task.resolve(message.result);
    }
  });
  const version = await send("Browser.getVersion", {}, false);
  const {targetId} = await send("Target.createTarget", {url:"about:blank"}, false);
  ({sessionId:session} = await send("Target.attachToTarget", {targetId, flatten:true}, false));
  await send("Runtime.enable");
  await send("Page.enable");
  await send("Log.enable");
  await send("Accessibility.enable");
  await send("Emulation.setEmulatedMedia", {features:[{name:"prefers-reduced-motion",value:"reduce"}]});
  await send("Page.navigate", {url:`http://127.0.0.1:${server.address().port}/`});
  await waitFor("WebGPU Gallery", () => evaluate("!!window.gallery?.inspect"));
  await resize(2500);
  await evaluate(`window.trusted = [];
    for (const event of ["keydown", "beforeinput", "input", "wheel"]) window.addEventListener(event, e => {
      if (e.isTrusted) window.trusted.push({event, id:e.target?.id ?? null, key:e.key ?? null, input:e.inputType ?? null});
    }, {capture:true, passive:true})`);
  await focus(show);
  await key("Enter", "Enter", 13, {text:"\r", unmodifiedText:"\r"});
  await waitFor("Multiline Gallery", () => evaluate(`${demo}.visible && ${demo}.grow.value === ${JSON.stringify(initial)}`));
  await resize(840);
  await waitFor("initial proxy publication", () => evaluate(`!!${selector(grow)} && !!${selector(fixed)}`));
  const before = await phase("before", [grow,fixed]);
  assert.equal(before.dom.host.hidden, true);

  await focus(grow, true);
  await waitFor("proxy handoff", () => evaluate(`!${selector(grow)} && ${area}.value === ${JSON.stringify(initial)}`));
  const active = await phase("active", [grow,fixed]);
  assert.equal(active.dom.host.value, initial);
  assert.equal(active.fields[grow].value, initial);
  assert.notEqual(active.fields[grow].backend, before.fields[grow].backend);
  await selectAll();
  await waitFor("select all", () => evaluate(`${area}.selectionStart === 0 && ${area}.selectionEnd === ${initial.length}`));
  await send("Input.insertText", {text:updated});
  await waitFor("exact committed string", () => evaluate(`${area}.value === ${JSON.stringify(updated)}
    && ${demo}.grow.value === ${JSON.stringify(updated)} && ${demo}.grow.edits === 1 && ${demo}.submits === 0`));
  const edited = await phase("edited", [grow]);
  assert.equal(edited.fields[grow].value, updated);
  assert.equal(edited.fields[grow].backend, active.fields[grow].backend);
  await key("Tab", "Tab", 9);
  await waitFor("post-edit proxy and hidden host", () => evaluate(`${state}.focused === ${JSON.stringify(action)}
    && !!${selector(grow)} && ${area}.hidden`));
  const after = await phase("after", [grow,fixed]);
  assert.notEqual(after.fields[grow].backend, edited.fields[grow].backend);
  assert.ok(after.fields[grow].description.includes("井字號") && after.fields[grow].description.includes("請簡短"));

  await revealLocked();
  const locked = await phase("locked", [readonly,disabled]);
  assert.equal(locked.fields[readonly].readonly, true);
  assert.equal(locked.fields[disabled].disabled, true);
  assert.equal(locked.fields[readonly].description, "這段文字可以選取和複製。");
  await focus(readonly, true);
  await waitFor("read-only native owner", () => evaluate(`!${selector(readonly)}
    && ${area}.value === ${JSON.stringify(stored)} && ${area}.readOnly`));
  const read = await phase("read", [readonly]);
  assert.equal(read.dom.host.readOnly, true);
  assert.equal(read.fields[readonly].value, stored);
  assert.notEqual(read.fields[readonly].backend, locked.fields[readonly].backend);
  await key("Tab", "Tab", 9);
  await waitFor("disabled skipped", () => evaluate(`${state}.focused === ${JSON.stringify(prefix + "back")} && ${area}.hidden`));

  const pendingFields = [[before,grow,initial], [before,fixed,long], [after,grow,updated],
    [after,fixed,long], [locked,readonly,stored], [locked,disabled,stopped]];
  const inactive = pendingFields.map(([step, id, expected]) => {
    const node = step.fields[id];
    const proxy = step.dom.proxies[id];
    return {id, value:node.value, expected, proxyText:proxy.text, whiteSpace:proxy.whiteSpace,
      hiddenCarrier:proxy.hiddenCarrier};
  });
  const adapter = await evaluate(`(async()=>{const {info}=await navigator.gpu.requestAdapter();
    const {vendor,architecture,isFallbackAdapter}=info;return {vendor,architecture,isFallbackAdapter}})()`);
  assert.equal(adapter.vendor, "apple");
  assert.ok(adapter.architecture.startsWith("metal"));
  assert.equal(adapter.isFallbackAdapter, false);
  const trusted = await evaluate("window.trusted");
  assert.ok(trusted.some(event => event.event === "input" && event.id === "surface-text-input"));
  assert.ok(trusted.some(event => event.event === "keydown" && event.id === "surface-text-input" && event.key === "Tab"));
  assert.ok(trusted.some(event => event.event === "wheel" && event.id === "surface"));
  assert.deepEqual(pageErrors, []);
  const manifest = JSON.parse(await readFile(join(root, "surface-web.json"), "utf8"));
  assert.deepEqual(Object.keys(requests).sort(), manifest.files.map(file => file.path).sort());
  const report = {scope:"independent focused Gallery development pack", expectation,
    browser:version.product, adapter, actual_served_assets:requests, inactive, phases, trusted, pageErrors};
  await writeFile(join(out,"textbox-values.json"), JSON.stringify(report,null,2)+"\n");
  for (const field of inactive) {
    assert.equal(field.value, expectation === "present" ? field.expected : null, `AX value for ${field.id}`);
    assert.equal(field.proxyText, expectation === "present" ? field.expected : "", `proxy value for ${field.id}`);
    if (expectation === "present") {
      assert.equal(field.whiteSpace, "pre");
      assert.equal(field.hiddenCarrier, field.expected);
    }
  }
  console.log(JSON.stringify({browser:version.product, adapter, expectation,
    testedValues:inactive.length, handoff: ["activate","commit","blur","read-only","skip-disabled"]}));
} catch (error) {
  await writeFile(join(out,"failure.json"), JSON.stringify({error:String(error), phases, requests, pageErrors},null,2)+"\n");
  throw error;
} finally {
  if (socket?.readyState === WebSocket.OPEN) {
    await Promise.race([send("Browser.close", {}, false), sleep(1000)]).catch(() => {});
    socket.close();
  }
  if (!await Promise.race([exited.then(()=>true), sleep(2000).then(()=>false)])) child.kill();
  await exited;
  for (const task of pending.values()) clearTimeout(task.timer);
  server.close();
}
