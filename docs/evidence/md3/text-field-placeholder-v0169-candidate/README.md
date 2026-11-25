# Surface v0.1.69 placeholder candidate proof

This packet binds the local exact source tree `b243564b27c4fb8d2856afc5ae19cd4cb2ad905a` and all 688 source
paths to the 16-path patch. The [v0.1.68 evidence package](https://github.com/faberline/surface/tree/14b110f1c269c1127389be68b3e9c7cb40e69b5f/docs/evidence/md3/text-field-label-color-v0168-candidate) is the
recorded predecessor. This packet covers local acceptance.

- The exact workspace passed 1,583 Rust tests and three Wasm checks. Real
  Apple Metal ran 18 tests and produced 21 retained PNGs.
- Native exact headless testing ran two repeats and made 40 original
  endpoint PNGs. All match development R1. Page.screenshot, CLI and MCP
  returned equal bytes. Four Gallery processes were reaped and
  92 samples showed zero visible windows.
- The original R1 run stopped at M350. Its PNG, typed output and full
  raw receipt remain in `focused/r1`. R2 adds a paint barrier in that
  test. It keeps the original checks and Gallery bytes.
- Scoped R2 passed 2,373 checks in 29 steps. It retains 34 label motion and
  color PNGs and adds 58 paired placeholder PNGs. Its 443 process samples
  show zero visible windows or surviving processes.
- Exact stock passed 25,851 checks across five suites. It
  ran 352 Material steps through M353, 20 IME steps and the prior feature
  journey. It produced both sets of rendered PNGs, with zero visible
  windows or surviving processes.

Browser frames cover real WebGPU pixels and Chrome accessibility output.
The IME evidence records browser composition and controlled input events.
Native evidence covers reduced-motion endpoints and Runtime semantics.
Live speech and OS input composition need separate proof.

## Offline replay

Use Python 3.12 and Pillow with the reviewed package seal.

~~~sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-placeholder.py --verify-package . --package-sha "$reviewed_sha"
~~~

The readback opens the archived original images and process receipts. It
needs no browser, native executable or source checkout. Private temporary
paths in raw logs identify the original owned runs. The archive uses
manifests to pin the original executable bytes.
