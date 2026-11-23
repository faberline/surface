#!/usr/bin/env python3
"""Capture production Surface Gallery frames using its public interfaces."""

import base64
import ipaddress
import json
import socket
import struct
import subprocess
import time
from urllib.parse import urlsplit
import zlib


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def read_png(data):
    require(data[:8] == PNG_SIGNATURE, "PNG signature failed")
    width, height = None, None
    compressed = []
    offset = 8
    closed = False
    while offset + 12 <= len(data):
        size, kind = struct.unpack_from(">I4s", data, offset)
        start, end = offset + 8, offset + 8 + size
        require(end + 4 <= len(data), "PNG chunk was truncated")
        content = data[start:end]
        expected_crc = struct.unpack_from(">I", data, end)[0]
        require(zlib.crc32(kind + content) == expected_crc, "PNG chunk CRC failed")
        offset = end + 4
        if kind == b"IHDR":
            width, height, depth, color, comp, filtering, interlace = struct.unpack(">IIBBBBB", content)
            require((depth, color, comp, filtering, interlace) == (8, 6, 0, 0, 0),
                    "Screenshot must use non-interlaced RGBA8")
        elif kind == b"IDAT":
            compressed.append(content)
        elif kind == b"IEND":
            closed = True
            break
    require(closed and offset == len(data) and compressed and width and height,
            "Incomplete PNG")
    decoded = zlib.decompress(b"".join(compressed))
    stride = width * 4
    require(len(decoded) == (stride + 1) * height, "PNG scanline sizes differ")
    previous = bytes(stride)
    rows = []
    for y in range(height):
        origin = y * (stride + 1)
        filter_type = decoded[origin]
        result = bytearray(decoded[origin + 1 : origin + 1 + stride])
        require(filter_type < 5, "Unknown PNG row filter")
        if filter_type:
            for x in range(stride):
                a = result[x - 4] if x >= 4 else 0
                b = previous[x]
                c = previous[x - 4] if x >= 4 else 0
                if filter_type == 1:
                    predictor = a
                elif filter_type == 2:
                    predictor = b
                elif filter_type == 3:
                    predictor = (a + b) // 2
                else:
                    p = a + b - c
                    distance = (abs(p - a), abs(p - b), abs(p - c))
                    predictor = (a, b, c)[distance.index(min(distance))]
                result[x] = (result[x] + predictor) & 255
        previous = result
        rows.append(bytes(result))
    return width, height, rows


def metadata(result):
    return {key: result[key] for key in ("format", "width", "height", "scale")}


def png_payload(result):
    require(result.get("format") == "png", "Wrong reply image format")
    data = base64.b64decode(result["data_base64"], validate=True)
    return data


class Gallery:
    def __init__(self, plan, session, proc):
        self.plan = plan
        self.session_path = session
        self.proc = proc

    def wire(self, method, params, expected_error=None):
        session = json.loads(self.session_path.read_text())
        require(session["pid"] == self.proc.pid and session["protocol"] == "surface-devtools"
                and session["version"] == 1, "Session does not match the launched Gallery")
        endpoint = urlsplit("//" + session["endpoint"])
        require(ipaddress.ip_address(endpoint.hostname).is_loopback,
                "The private session must use loopback")
        token = session["token"]
        request = {"v": 1, "id": 1, "method": method, "params": params, "token": token}
        # The credential is only sent over the application's localhost transport.
        with socket.create_connection((endpoint.hostname, endpoint.port), timeout=5) as connection:
            with connection.makefile("rwb") as stream:
                stream.write(json.dumps(request, allow_nan=False).encode("utf-8") + b"\n")
                stream.flush()
                reply = json.loads(stream.readline(16 * 1024 * 1024))
        require(reply.get("id") == 1, "Wire reply identifier mismatch")
        if expected_error:
            require(not reply.get("ok") and reply.get("error", {}).get("code") == expected_error,
                    f"Unexpected wire result for {method} limit check")
            return expected_error
        require(reply.get("ok"), f"Wire request {method} returned a protocol error")
        return reply.get("result")

    def cli(self, args, want_status=0):
        run = subprocess.run([self.plan.cli, "--session", self.session_path, *args],
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=15)
        if run.returncode != want_status:
            # Never expose raw connection messages, tokens, or private session paths.
            raise RuntimeError(f"CLI command {args[:2]} returned exit {run.returncode}")
        if want_status == 0:
            return json.loads(run.stdout)
        return want_status

    def dom(self):
        deadline = time.monotonic() + 3
        while True:
            snapshot = self.wire("DOM.snapshot", {})
            if snapshot.get("layout") is not None:
                return snapshot
            require(time.monotonic() < deadline, "Gallery did not lay out the current frame")
            time.sleep(0.025)

    def activate(self, stable_id):
        result = self.cli(["action", "activate", stable_id])
        require(result.get("changed"), f"Activation made no change for {stable_id}")

    def key(self, key):
        return self.cli(["action", "key", key])

    def mcp(self, params):
        requests = [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize",
             "params": {"protocolVersion": "2025-03-26", "capabilities": {},
                        "clientInfo": {"name": "surface-screenshot-evidence", "version": "1"}}},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
            {"jsonrpc": "2.0", "id": 3, "method": "tools/call",
             "params": {"name": "page_screenshot", "arguments": params}},
        ]
        payload = "".join(json.dumps(request) + "\n" for request in requests)
        result = subprocess.run([self.plan.cli, "--session", self.session_path, "mcp"],
                                input=payload.encode("utf-8"), stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, timeout=25)
        require(result.returncode == 0, "MCP process failed")
        replies = [json.loads(line) for line in result.stdout.splitlines()]
        require([value.get("id") for value in replies] == [1, 2, 3], "Invalid MCP reply sequence")
        tool = next(tool for tool in replies[1]["result"]["tools"] if tool["name"] == "page_screenshot")
        limits = tool["inputSchema"]["properties"]["max_side"]
        require((limits["minimum"], limits["maximum"], limits["default"]) == (1, 8192, 4096),
                "MCP screenshot limits do not match the schema")
        expected = replies[2]["result"]
        require(not expected.get("isError"), "MCP screenshot returned a tool error")
        image, text = expected["content"]
        require(image["type"] == "image" and image["mimeType"] == "image/png"
                and text["type"] == "text", "MCP content blocks are incorrect")
        return base64.b64decode(image["data"], validate=True), json.loads(text["text"]), tool
