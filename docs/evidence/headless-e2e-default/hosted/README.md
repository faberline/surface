# Hosted test runs for the headless default

Both runs tested PR #222 head
`2c0f0c3b6b6b9444f41230a4f166f9d51af45fc6` on the Divider base.
Each passed all eight suites and 18,922 checks. The workflow requests
`cargo xtask web e2e all` on a virtual display. The separate macOS run
checks the five-case default used on a physical desktop.
[PR run](https://github.com/faberline/surface/actions/runs/45489582963) · [Push run](https://github.com/faberline/surface/actions/runs/45489561619)

Both job logs and the verification receipts appear below. Use `gzip -dc`
to read the original log bytes. The receipts pin the event, source tree,
adapter and original artifact ZIP digest.
[Combined receipt](hosted-verified.json) · [Pull request receipt](pull_request/run-verified.json) · [Push receipt](push/run-verified.json)
