# Surface v0.1.70 text-field boundary evidence

The exact 689-file tree `409a21f6023802f055371cdec4391976c55ffd17` applies the 12-path patch to the
v0.1.69 tree `b243564b27c4fb8d2856afc5ae19cd4cb2ad905a`. The scoped Web and native development
sources differ from the exact source only in the coverage document.

- Exact Rust passed 1,585 tests, provenance, lint and three Wasm checks.
  Actual Apple Metal passed 18 tests and produced 21 original PNGs.
- The exact native run repeated 18 original PNGs byte-for-byte through
  Page.screenshot, CLI and MCP. Its 48 observations showed no windows.
  All four Gallery processes exited.
- Scoped Chrome passed 5,228 checks in 32 steps. It records 33 new
  boundary PNGs plus 92 earlier PNGs on the same product. Its 547
  samples showed zero visible windows or surviving processes.
- Exact stock passed 28,706 checks in five
  suites. It includes 355 ordered Material steps through M356, 20
  IME steps, 2,478 independent boundary probes and zero windows or survivors.

Use Python 3.12 with Pillow. Keep the original packet seal from the handoff.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-boundary.py --verify-package . --package-sha "$reviewed_sha"
```

The verifier opens the sealed original files without launching a
browser or Gallery. It includes only fixture captures and input hashes.
Build caches and binaries stay outside the packet. Original receipts
include the temporary paths of their guarded processes. The native
evidence covers Reduce Motion endpoints. Browser composition
and platform semantics are separate from live speech or OS input.
