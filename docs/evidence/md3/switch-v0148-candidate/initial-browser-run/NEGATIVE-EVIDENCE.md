# v0.1.48 Switch v1 Browser test rejection

The five-suite macOS run reached M277. Four suites passed. Material failed
after a true click on the large RTL Switch label. The original test waited
for the accessibility proxy to become the browser's active element. The
Web host leaves browser focus on the canvas after a true canvas press.

The retained report shows the requested checked value, count 3 of effective
changes, and Runtime focus on `switch-demo-both-icons`. The Chrome
accessibility checks for the changed Switch ran before the mismatched focus
wait. Independent PNG samples confirm Primary on the selected track and
Secondary on the focus outline. All three guards accepted headless flags.
CoreGraphics found zero visible owned windows and zero survivors.

This is the original rejection. The v2 test uses the existing check for
Runtime focus on the Switch and browser focus on the canvas. It keeps the
separate keyboard and projected accessibility checks.

`failed-run.sha256` seals all 704 original output files, including the
original JSON report and four PNGs. Run `shasum -a 256 -c ../failed-run.sha256`
inside the original `run` folder. This package carries the M277 files.
The complete raw run remains in the original local evidence directory.

[Measured original failure](m277-negative-receipt.json) · [Raw report](run/artifacts/browser-material-switch.json) · [Original failure PNG](run/artifacts/material-switch-failure.png)
