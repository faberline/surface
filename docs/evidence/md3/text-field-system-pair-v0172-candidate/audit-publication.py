"""Read a sealed evidence packet and report candidate paths without raw matches."""

import argparse
from collections import Counter, defaultdict
import gzip
import ipaddress
import json
from pathlib import Path
import re
import stat
import struct
from urllib.parse import parse_qsl, urlsplit

from evidence_common import digest, require, seal


SECRET_KEYS = {
    "token", "access_token", "auth_token", "refresh_token", "github_token", "password",
    "secret", "client_secret", "api_key", "authorization", "cookie", "set_cookie",
    "private_key", "x_api_key", "session_token", "signature", "sig",
}
TRUSTED_HOSTS = {
    "github.com", "www.github.com", "m3.material.io", "fonts.google.com", "developers.google.com",
    "developer.apple.com", "developer.mozilla.org", "www.w3.org", "www.w3schools.com",
    "chromium.googlesource.com", "android.googlesource.com", "crates.io", "docs.rs", "example.com",
    "example.org", "example.net", "www.example.com", "registry.npmjs.org",
}
SECRET_PATTERNS = {
    "credential_token": (
        r"\b(?:gh[pousr]_[a-zA-Z0-9]{24,}|github_pat_[A-Za-z0-9_]{30,}|"
        r"xox[baprs]-[A-Za-z0-9-]{12,}|AKIA[A-Z0-9]{16}|ASIA[A-Z0-9]{16}|"
        r"sk_(?:live|test)_[A-Za-z0-9]{20,}|sk-(?:proj-)?[A-Za-z0-9_-]{30,}|"
        r"AIza[A-Za-z0-9_-]{30,}|hf_[A-Za-z0-9]{30,})\b"
    ),
    "private_key": r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |ENCRYPTED )?PRIVATE KEY-----",
    "bearer_value": r"(?i)\bauthorization[\s\"']*[:=][\s\"']*bearer\s+[A-Za-z0-9_.=+/\-]{12,}",
    "jwt": r"\beyJ[A-Za-z0-9_\-]{12,}\.[A-Za-z0-9_\-]{12,}\.[A-Za-z0-9_\-]{15,}\b",
    "mail_address": r"(?<![\w./])[A-Za-z0-9_.+\-]{2,}@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}(?![\w./])",
    "host_home_path": r"/(?:Users|home)/[^/\s\"'<>]+",
    "unix_tmp_path": r"/(?:private/)?tmp/",
    "os_session_directory": r"/(?:private/)?var/folders/",
}
URL = re.compile(r"https?://[^\s`<>\[\]\"'\\]+")
BINARY_MAGIC = (
    b"\x7fELF", b"MZ", b"\0asm", b"\xfe\xed\xfa\xce", b"\xfe\xed\xfa\xcf",
    b"\xce\xfa\xed\xfe", b"\xcf\xfa\xed\xfe", b"\xca\xfe\xba\xbe", b"\xbe\xba\xfe\xca",
    b"\xca\xfe\xba\xbf", b"PK\x03\x04", b"SQLite format 3\0",
)
PNG_MAGIC = b"\x89PNG\r\n\x1a\n"
TEXT_MAX = 12 * 1024 * 1024


def normalize_key(key):
    return re.sub(r"(?<=[a-z])(?=[A-Z])", "_", key).lower().replace("-", "_")


def fixture_host(host):
    return host.endswith((".invalid", ".test", ".example", ".localhost")) or host in TRUSTED_HOSTS


def scan_strings(value, name, observations):
    if isinstance(value, dict):
        for key, child in value.items():
            if (normalize_key(key) in SECRET_KEYS and isinstance(child, str) and child.strip()
                    and not re.fullmatch(r"(?i)(?:dummy|fixture|sample|example|redacted|invalid|-|none|null)", child)):
                observations["credential_like_json_field"].add(name)
            scan_strings(child, name, observations)
    elif isinstance(value, list):
        for item in value:
            scan_strings(item, name, observations)


def scan_text(text, name, observations):
    for family, pattern in SECRET_PATTERNS.items():
        for found in re.finditer(pattern, text):
            if family == "mail_address":
                host = found.group().lower().rsplit("@", 1)[-1]
                observations["fixture_mail_address" if fixture_host(host) else family].add(name)
            else:
                observations[family].add(name)
    for match in URL.finditer(text):
        suffix = match.group().rstrip(").,;")
        try:
            url = urlsplit(suffix)
            host = (url.hostname or "").lower()
        except ValueError:
            observations["unparsed_url"].add(name)
            continue
        if url.username or url.password or any(
            normalize_key(key) in SECRET_KEYS or key.lower().startswith("x-amz-")
            for key, value in parse_qsl(url.query, keep_blank_values=False) + parse_qsl(url.fragment)
            if value
        ):
            observations["url_auth"].add(name)
        try:
            address = ipaddress.ip_address(host)
        except ValueError:
            address = None
        if host == "localhost" or (address and address.is_loopback):
            observations["runtime_loopback_urls"].add(name)
        elif address and (address.is_private or address.is_link_local):
            observations["private_network_urls"].add(name)
        elif fixture_host(host) or host in {"developer.chrome.com", "www.rust-lang.org", "rust-lang.org"}:
            observations["known_reference_urls"].add(name)
        elif host:
            observations["other_url_origin"].add(name)


def scan_png(raw, name, observations):
    offset, ended, dimension = 8, False, None
    while offset + 12 <= len(raw):
        count, tag = struct.unpack_from(">I4s", raw, offset)
        head, tail = offset + 8, offset + 8 + count
        require(tail + 4 <= len(raw), f"Truncated PNG {name}")
        if tag == b"IHDR":
            dimension = struct.unpack_from(">II", raw, head)
        elif tag in {b"tEXt", b"iTXt", b"zTXt", b"eXIf", b"iCCP"}:
            observations["png_text_exif_or_icc_metadata"].add(name)
        offset = tail + 4
        if tag == b"IEND":
            ended = True
            break
    require(ended and dimension is not None and offset == len(raw), f"Wrong PNG structure or trailer {name}")
    return dimension


def inspect(root, expected):
    entries = seal(root, "SHA256SUMS", expected, exact=True)
    require(not any(p.is_symlink() for p in root.rglob("*")), "Package contains linked files")
    sizes, categories, observations, dimensions = [], Counter(), defaultdict(set), Counter()
    for name in entries:
        path = root / name
        size = path.stat().st_size
        sizes.append((size, name))
        if size >= 5 * 1024 * 1024:
            observations["large_file"].add(name)
        if path.stat().st_mode & (stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH):
            observations["executable_mode"].add(name)
        raw = path.read_bytes()
        if raw.startswith(PNG_MAGIC):
            categories["png"] += 1
            dimensions[scan_png(raw, name, observations)] += 1
            continue
        if raw.startswith(BINARY_MAGIC):
            categories["binary_executable_or_archive"] += 1
            observations["binary_signature"].add(name)
            continue
        if raw.startswith(b"\x1f\x8b"):
            categories["gzip"] += 1
            with gzip.open(path, "rb") as inner:
                raw = inner.read(TEXT_MAX + 1)
            require(len(raw) <= TEXT_MAX and name.endswith((".log.gz", ".patch.gz")), f"Unexpected gzip payload {name}")
        else:
            categories["other"] += 1
        if not raw:
            continue
        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError:
            observations["unrecognized_nontext"].add(name)
            continue
        if "\0" in text:
            observations["unrecognized_nontext"].add(name)
        scan_text(text, name, observations)
        if path.suffix == ".json":
            scan_strings(json.loads(text), name, observations)
    require(digest(root / "SHA256SUMS") == expected, "Package changed during scan")
    return {"scope": "offline file inventory, text and PNG metadata audit",
            "seal_sha256": expected, "files": len(entries), "total_bytes": sum(s for s, _ in sizes),
            "largest_files": [{"bytes": size, "path": name} for size, name in sorted(sizes, reverse=True)[:5]],
            "file_types": dict(categories),
            "png_dimensions": {f"{w}x{h}": count for (w, h), count in sorted(dimensions.items())},
            "observations": {key: {"count": len(paths), "paths": sorted(paths) if key != "unix_tmp_path" else []}
                             for key, paths in sorted(observations.items())}}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--seal", required=True)
    args = parser.parse_args()
    print(json.dumps(inspect(args.archive.resolve(strict=True), args.seal), ensure_ascii=False, indent=2))
