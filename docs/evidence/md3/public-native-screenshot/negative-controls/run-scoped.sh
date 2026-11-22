#!/bin/sh
set -eu
proof="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
results="$proof/scoped-evidence"
mkdir -p "$results"
cd "$proof/repo"
shasum -a 256 -c "$proof/scoped-runner-source.sha256"
shasum -a 256 -c "$proof/scoped-binaries.sha256"

for scenario in baseline swapped-red-blue stale-default-image ignored-max-side; do
    if test "$scenario" = baseline; then
        unset SURFACE_SCREENSHOT_NEGATIVE_CONTROL
    else
        export SURFACE_SCREENSHOT_NEGATIVE_CONTROL="$scenario"
    fi
    rc=0
    "$proof/bin/surface-e2e-scoped" --native-bin "$proof/bin/surface-material-gallery" \
      --devtools-bin "$proof/bin/surface-devtools" --artifacts "$results/$scenario" \
      devtools-native > "$results/$scenario.log" 2>&1 || rc=$?
    printf '%s\n' "$rc" > "$results/$scenario-exit.txt"
    grep -E 'DN25|^case devtools-native|^  last step|^e2e result' "$results/$scenario.log" || true
    if test "$scenario" = baseline && test "$rc" -ne 0; then
        printf 'The no-fault control failed. Stop before opening the fault windows.\n' >&2
        exit "$rc"
    fi
done
python3 "$proof/audit-controls.py" "$results"
