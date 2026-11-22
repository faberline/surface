# Final transport replay

The final PR source is commit `8c0724fb3157e171040fe4f623bda91785d4c660`
and tree `99dc1a2a3e95c2e6ecfe31c0ceac78997e03bc05`.
The product code is the same as the initial evidence. The new commit
changes only `e2e/src/devtools_native.rs`.

The first hosted Linux test compared sequential requests while the
Gallery checkbox was animating. The earlier MCP capture and later
raw and CLI captures were at different points in that animation. The
replay now enables Reduce Motion for the equality comparison and writes
all three PNGs before the equality assertions. The test keeps the same
pixel, freshness, size, write and protocol checks. [First hosted result](https://github.com/faberline/surface/actions/runs/45019360780/job/137198603820)

All four runtime suites passed 471 checks on the Mac after the change.
The scaled raw, CLI and MCP files below have SHA-256
`9967e9d4308deda8da629f05a21d015fc29cb1baaadcf2e4af2baa9d9041423f`.
The three injected controls also failed at their own expected checks
with the final runner. [Final fault receipts](negative-controls/README.md)

[Transport log](final-transport.log) · [Raw PNG](devtools-native-screenshot-scaled.png) ·
[CLI PNG](devtools-native-screenshot-cli.png) · [MCP PNG](devtools-native-screenshot-mcp.png)
