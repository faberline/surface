#!/usr/bin/env python3
"""Use the real Jet bridge CLI/MCP against owned native and WASM Surface apps."""
import argparse
import functools
import http.server
import json
import os
from pathlib import Path
import select
import signal
import socket
import stat
import subprocess
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
ARTIFACTS = ROOT / "artifacts"


class Client:
    def __init__(self, prefix, diagnostic, web=False):
        self.prefix, self.diagnostic = prefix, diagnostic
        self.process = subprocess.Popen(prefix + ["mcp"] + (["--ui"] if web else []),
                                        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=diagnostic, text=True, bufsize=1)
        self.sequence = 0
        hello = self.request("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                             "clientInfo": {"name": "surface-live-check", "version": "1"}})
        assert hello["protocolVersion"] == "2025-06-18", hello
        assert hello["serverInfo"]["name"] == "jet-bridge-ui", hello
        self.process.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
        self.process.stdin.flush()
        tools = self.request("tools/list", {})["tools"]
        assert {tool["name"] for tool in tools} == {
            "ui_capabilities", "ui_snapshot", "ui_query", "ui_activate", "ui_fill", "ui_set_value", "ui_key"}

    def request(self, method, params):
        self.sequence += 1
        self.process.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.sequence,
                                           "method": method, "params": params}) + "\n")
        self.process.stdin.flush()
        assert select.select([self.process.stdout], [], [], 20)[0], "MCP response timed out"
        response = json.loads(self.process.stdout.readline())
        assert response["id"] == self.sequence and "error" not in response, response
        return response["result"]

    def tool(self, name, args=None, error=None):
        result = self.request("tools/call", {"name": name, "arguments": args or {}})
        if error:
            assert result["isError"] and error in result["content"][0]["text"], result
            return result
        assert not result.get("isError"), result
        return json.loads(result["content"][0]["text"])

    def cli(self, *arguments):
        return command(self.prefix, self.diagnostic, *arguments)

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            self.process.wait(timeout=5)
        assert self.process.returncode == 0, "MCP server did not close cleanly"


def command(prefix, diagnostic, *arguments):
    result = subprocess.run(prefix + list(arguments), cwd=ROOT, text=True,
                            stdout=subprocess.PIPE, stderr=diagnostic, timeout=40, check=True)
    return json.loads(result.stdout) if result.stdout.strip() else None


def node(snapshot, stable_id):
    return next(item for item in snapshot["dom"]["nodes"] if item["stable_id"] == stable_id)


def fully_visible(snapshot, stable_id):
    index = next(i for i, item in enumerate(snapshot["dom"]["nodes"]) if item["stable_id"] == stable_id)
    geometry = snapshot["layout"]["nodes"][index]
    bounds, clip = geometry["bounds"], geometry["clip"]
    return (bounds["x"] >= clip["x"] - 0.5 and bounds["y"] >= clip["y"] - 0.5
            and bounds["x"] + bounds["width"] <= clip["x"] + clip["width"] + 0.5
            and bounds["y"] + bounds["height"] <= clip["y"] + clip["height"] + 0.5)


def journey(client, mode):
    caps = client.tool("ui_capabilities")
    assert caps["runtime"] == "surface" and caps["snapshot_schema"] == "surface.dom.v1", caps
    first = client.cli("ui", "snapshot")
    assert first["version"] == 1 and len(first["dom"]["nodes"]) > 10
    assert len(first["layout"]["nodes"]) == len(first["dom"]["nodes"]), "missing current geometry"
    assert first["layout"]["nodes"][0]["bounds"]["width"] > 0
    initially_clipped = not fully_visible(first, "volume")
    # A clipped control remains reachable by Tab. Read actual new geometry.
    visited = []
    for _ in range(20):
        client.cli("ui", "key", "tab")
        frame = client.tool("ui_snapshot")
        focused = frame["interaction"]["focused"]
        assert focused not in visited, f"Tab cycled before reaching volume: {visited}"
        visited.append(focused)
        if focused == "volume":
            assert fully_visible(frame, "volume"), "focused range is still clipped"
            break
    else:
        raise AssertionError("Tab did not reach the range control")
    client.tool("ui_set_value", {"id": "volume", "value": 67})
    assert node(client.cli("ui", "snapshot"), "volume")["range"]["value"] == 65
    client.cli("ui", "activate", "check")
    assert node(client.tool("ui_snapshot"), "check")["checked"] is True
    client.tool("ui_activate", {"id": "section-1"})
    client.tool("ui_set_value", {"id": "volume", "value": 25}, error="missing_node")
    client.tool("ui_activate", {"id": "filled"})
    assert node(client.cli("ui", "snapshot"), "click-count")["text"] == "已執行 1 次"
    client.tool("ui_activate", {"id": "disabled-action"}, error="disabled")
    assert node(client.tool("ui_snapshot"), "click-count")["text"] == "已執行 1 次"
    client.cli("ui", "activate", "section-2")
    client.tool("ui_activate", {"id": "open-dialog"})
    result = client.tool("ui_query", {"role": "TextInput", "name": "名稱"})
    assert len(result["matches"]) == 1 and result["matches"][0]["node"]["stable_id"] == "dialog-name", result
    client.cli("ui", "fill", "dialog-name", "中文橋接🦀")
    editing = client.tool("ui_snapshot")
    assert editing["text_inputs"]["dialog-name"]["value"] == "中文橋接🦀"
    assert editing["text_inputs"]["dialog-name"]["selection_cursor"] == len("中文橋接🦀".encode())
    client.tool("ui_activate", {"id": "open-menu"}, error="outside_modal")
    client.tool("ui_key", {"key": "tab"})
    assert client.cli("ui", "snapshot")["interaction"]["focused"] == "dialog-cancel"
    client.cli("ui", "key", "tab")
    assert client.tool("ui_snapshot")["interaction"]["focused"] == "dialog-save"
    client.tool("ui_key", {"key": "tab"})
    assert client.cli("ui", "snapshot")["interaction"]["focused"] == "dialog-name"
    client.tool("ui_activate", {"id": "dialog-save"})
    saved = client.cli("ui", "snapshot")
    assert node(saved, "saved-name")["text"] == "名稱：中文橋接🦀"
    assert saved["interaction"]["focused"] == "open-dialog"
    client.tool("ui_fill", {"id": "dialog-name", "value": "stale"}, error="missing_node")
    client.cli("ui", "activate", "open-dialog")
    client.tool("ui_fill", {"id": "dialog-name", "value": "取消的草稿"})
    client.cli("ui", "key", "escape")
    final = client.tool("ui_snapshot")
    assert node(final, "saved-name")["text"] == "名稱：中文橋接🦀"
    assert final["interaction"]["focused"] == "open-dialog"
    client.tool("ui_not_a_tool", error="invalid application tool arguments")
    (ARTIFACTS / f"bridge-{mode}-dom.json").write_text(json.dumps(final, ensure_ascii=False, indent=2))
    return {"mode": mode, "tools": 7, "saved_name": "中文橋接🦀", "focus_restored": True,
            "controlled_range": 65, "stale_node_rejected": True, "disabled_rejected": True,
            "modal_scope_enforced": True, "cli_mcp_share_state": True, "matching_layout": True,
            "tab_reaches_scrolled_controls": True, "tab_order": visited,
            "initial_range_clipped": initially_clipped}


def native(args, folder, diagnostic):
    session_file = Path(folder) / "app-session.json"
    env = dict(os.environ, SURFACE_BRIDGE_SESSION=str(session_file))
    app = subprocess.Popen([str(args.native_bin)], cwd=ROOT, env=env,
                           stdout=subprocess.DEVNULL, stderr=diagnostic)
    client = None
    try:
        deadline = time.monotonic() + 30
        session = None
        while time.monotonic() < deadline:
            assert app.poll() is None, "native app exited before bridge readiness"
            try:
                session = json.loads(session_file.read_text())
                break
            except (FileNotFoundError, json.JSONDecodeError):
                time.sleep(0.05)
        assert session and session["pid"] == app.pid, "wrong native app session"
        if os.name == "posix":
            assert stat.S_IMODE(session_file.stat().st_mode) == 0o600
        host, port = session["endpoint"].rsplit(":", 1)
        with socket.create_connection((host, int(port)), timeout=5) as stream:
            stream.sendall(json.dumps({"token": "wrong-token", "request": {
                "version": 1, "id": 91, "command": {"method": "activate", "id": "section-1"}}}).encode() + b"\n")
            response = json.loads(stream.makefile("rb").readline())
            assert response["id"] == 91 and response["code"] == "unauthorized", response
        prefix = [str(args.jet_bridge), "--app-session", str(session_file)]
        client = Client(prefix, diagnostic)
        evidence = journey(client, "native")
        client.close()
        client = None
        diagnostic.flush()
        assert session["token"] not in Path(diagnostic.name).read_text(), "session token was logged"
        evidence["private_descriptor"] = True
    finally:
        if client:
            client.close()
        app.terminate()
        app.wait(timeout=10)
    # A separate two-frame run exercises normal host cleanup, including its descriptor.
    smoke_file = Path(folder) / "smoke-session.json"
    completed = subprocess.run([str(args.native_bin), "--smoke"], cwd=ROOT,
                               env=dict(os.environ, SURFACE_BRIDGE_SESSION=str(smoke_file)),
                               stdout=subprocess.DEVNULL, stderr=diagnostic, timeout=30)
    assert completed.returncode == 0 and not smoke_file.exists(), "native shutdown left its session file"
    evidence["normal_shutdown_removes_descriptor"] = True
    return evidence


class QuietServer(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *_):
        pass


def web(args, folder, diagnostic):
    handler = functools.partial(QuietServer, directory=str(args.web_dir))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    prefix = [str(args.jet_bridge), "--root", folder]
    owned_pid = None
    client = None
    try:
        launched = command(prefix, diagnostic, "bb", "launch", f"http://127.0.0.1:{server.server_port}/")
        owned_pid = launched["pid"]
        deadline = time.monotonic() + 40
        status = ""
        while time.monotonic() < deadline:
            status = command(prefix, diagnostic, "bb", "eval", "document.querySelector('#status').textContent")
            if status.startswith("Ready"):
                break
            if status.startswith("Surface error:") or status.startswith("Error:"):
                raise AssertionError(status)
            time.sleep(0.1)
        assert status.startswith("Ready"), status
        assert command(prefix, diagnostic, "bb", "eval", "window.__jet_bridge.version") == 1
        unsupported = command(prefix, diagnostic, "bb", "eval", 'window.__jet_bridge.request({version:999,id:92,command:{method:"snapshot"}})')
        assert unsupported["id"] == 92 and unsupported["code"] == "unsupported_version", unsupported
        client = Client(prefix, diagnostic, web=True)
        evidence = journey(client, "web")
        command(prefix, diagnostic, "bb", "screenshot", "--out", str(ARTIFACTS / "bridge-web.png"))
        assert (ARTIFACTS / "bridge-web.png").read_bytes().startswith(b"\x89PNG\r\n\x1a\n")
        evidence["webgpu_ready"] = True
        client.close()
        client = None
        command(prefix, diagnostic, "bb", "shutdown")
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            try:
                os.kill(owned_pid, 0)
            except ProcessLookupError:
                owned_pid = None
                break
            time.sleep(0.05)
        assert owned_pid is None, "shutdown ACK did not prove owned browser exit"
        evidence["owned_browser_exited"] = True
        return evidence
    finally:
        if client:
            client.close()
        if owned_pid:
            try:
                os.kill(owned_pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jet-bridge", required=True, type=Path)
    parser.add_argument("--native-bin", type=Path, default=ROOT / "target/debug/surface-material-gallery")
    parser.add_argument("--web-dir", type=Path, default=ROOT / "target/web/devtools/material-gallery",
                        help="output of `cargo xtask web build material-gallery --features devtools --out <dir>`")
    parser.add_argument("--mode", choices=["native", "web", "both"], default="both")
    parser.add_argument("--require-overflow", action="store_true",
                        help="Require the range to start outside the visible viewport")
    args = parser.parse_args()
    args.jet_bridge = args.jet_bridge.resolve()
    ARTIFACTS.mkdir(exist_ok=True)
    evidence = []
    for mode in (["native", "web"] if args.mode == "both" else [args.mode]):
        with tempfile.TemporaryDirectory(prefix=f"surface-bridge-{mode}-") as folder, \
                (ARTIFACTS / f"bridge-{mode}-stderr.log").open("w") as diagnostic:
            result = (native if mode == "native" else web)(args, folder, diagnostic)
            if args.require_overflow:
                assert result["initial_range_clipped"], f"{mode} fixture did not exercise scrolling"
            evidence.append(result)
            (ARTIFACTS / f"bridge-{mode}-result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2))
            print(json.dumps(result, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
