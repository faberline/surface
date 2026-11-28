# Surface v0.1.76 choice-menu diagnosis

The recovered candidate is tree `b3e62f07a03b08f1741de8695fe845d3b771f11d`.
Its 14 targeted Rust tests passed. Two native replays captured eight PNGs
across light and narrow dark RTL. Page.screenshot, CLI and MCP returned
matching bytes for every capture. All six Gallery processes exited.
[Native results and replay](native-source-and-appkit/README.md)

Chrome WebGPU ran the full Material case on the same source. It failed at
M373 after the real mouse click on the disabled Owner row. The menu was
required to stay open with the same choice state. The runner attempted
30,633 checks. Its guard sampled 4,868 times with zero visible windows
and zero remaining processes. [Original browser log](browser-r3/runner.log)

The [three correction patches](native-source-and-appkit/r4/patches/) produce
combined tree `37e4454bc7d3f63f7111c8264d4e59e451c8ef9e`. The disabled-row
fix passed eight menu tests and 372 Runtime tests on the v0.1.75 base.
The AppKit fix passed a separate detached-view test. The full corrected
native and Chrome replays are pending.
