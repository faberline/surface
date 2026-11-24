# Surface v0.1.43 Search candidate before cancellation fix

This candidate has a 557-file source manifest with SHA-256 `4452974627bf7d8e9d54518a3b114b7c37dd7c94d9f1cd6e7f69ed62f3e1ffd9`. It remains on hold for the independently reproduced Showcase cancellation regression. [Exact source reconstruction](../integration/README.md#source-and-replay) · [Controlled regression](https://github.com/faberline/surface/tree/12a77def89134f8a3c420067ae65a33158dae29f/docs/evidence/md3/scroll-contrast-v0.1.42/web-cancel-regression)

The first Mac headless run failed Showcase I11 after a canceled input method composition. The other four suites passed. The unchanged retry passed all five suites and 17,275 checks. Material ran 254 steps and 17,014 checks, including Search M248–M255 and 46 verified Canvas captures. The retry had zero page errors and left no Chrome processes. The controlled regression still reproduces.

- [First run](browser/run.log) · [Retry](browser/run-retry.log)
- [Verified stock receipt](browser/verified-browser.json) · [Search result](browser/search.json) · [Browser binary manifest](browser/binaries.sha256)

The exact v0.1.43 native Gallery matched both settled frames of two fixed-clock diagnostic runs. The eight motion PNGs and report were identical across repeats. All captures agreed through Page.screenshot, CLI and MCP. All 26 checks saw zero visible windows. [Native report](native/exact-motion/motion-1/native-search-motion-report.json) · [Stock comparison](native/exact-stock/exact-stock-comparison.json) · [Native reproduction](native/README.md)

Run `shasum -a 256 -c SHA256SUMS` here to verify this record.
