# Surface v0.1.43 first hosted Search result

This archive records the first hosted result for candidate `2a63788e80bba537a271f5ace2514f5a9db53756`. Both builds passed on tree `433cd5d4937d66154a323931932a17d70a863f7d`. The PR workflow used GitHub's merge checkout `e8c6138e836d5c23a55692d2aae9f7931c68f260`. The push workflow used the candidate commit.

- [PR run 45447002788](https://github.com/faberline/surface/actions/runs/45447002788) and [browser job 138633409256](https://github.com/faberline/surface/actions/runs/45447002788/job/138633409256)
- [Push run 45446974715](https://github.com/faberline/surface/actions/runs/45446974715) and [browser job 138633304581](https://github.com/faberline/surface/actions/runs/45446974715/job/138633304581)

Each browser job reported seven suite passes, one failure and 16,577 checks. IME passed 77 checks with both cancellation reports. Contrast M247 completed and decoded pixels matched the selected and unselected underline color evidence.

Both Material suites stopped at M248. The runner first checked its wide Search geometry and recorded three Canvas PNGs. Its next assertion expected `isFallbackAdapter` to be `false`. The workflow specifies Chrome SwiftShader, a software WebGPU adapter. Chrome reported `true` as expected on that host. The archived Showcase and Side Sheet reports record `google`, `swiftshader` and `isFallbackAdapter: true`. The Search report has both docked styles and zero page errors. M249 through M255 did not run on this head. [Failing test line](https://github.com/faberline/surface/blob/2a63788e80bba537a271f5ace2514f5a9db53756/e2e/src/material_search_overlay.rs#L569).

The raw JSON and PNG files retain the bytes downloaded from their named artifacts. The log excerpts contain the exact checkouts, build commands, test steps, failure and totals. Original complete logs and downloads are kept in the local run folder. Run `shasum -a 256 -c SHA256SUMS` here to check the archive.
