#!/bin/sh
set -eu
lane=/tmp/surface-md3-v0178-native-r2.axxgX14N
build=/tmp/surface-md3-v0178-menu-system-colors.hlLwvQTS/outputs-r2
source=/tmp/surface-md3-v0178-menu-system-colors.hlLwvQTS/surface
source_sha=e58979727fbe6a9adc0b8ba64f9141e11bb7a32e570063f328d2449a8d034a4e
tree=0de7289a17faab38a240c387a8534471fb9d8906
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
test "$#" -ge 1
mode=$1
shift
sha() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
verify_helpers() {
    test "$(sha "$lane/native-tools/SHA256SUMS")" = 75e20e6b556e458c2e9d07ae9e94393eb54129fe3ee11bf183b4fd620fa085b7
    (cd "$lane/native-tools" && shasum -a 256 -c SHA256SUMS > /dev/null)
}
verify_source() {
    "$py" -E -B "$lane/source-preflight.py"
    "$py" -E -B "$lane/native-tools/replay-vertical-choice-menu.py" --check-source \
      --source "$source" --source-manifest "$source/../source.sha256" --source-sha "$source_sha" > /dev/null
}
verify_inputs() {
    verify_helpers
    verify_source
    test "$(sha "$build/SHA256SUMS")" = "$build_sha"
    test "$(sha "$build/native/SHA256SUMS")" = "$native_sha"
    test "$(sed -n '1p' "$build/BUILD-COMPLETE")" = "$tree"
    test "$(sed -n '2p' "$build/BUILD-COMPLETE")" = "$source_sha"
    test "$(wc -l < "$build/BUILD-COMPLETE" | tr -d ' ')" = 2
    (cd "$build" && shasum -a 256 -c SHA256SUMS > /dev/null)
    (cd "$build/native" && shasum -a 256 -c SHA256SUMS > /dev/null)
}
case "$mode" in
  --check-source) test "$#" -eq 0; verify_helpers; verify_source; exit 0 ;;
  --check-inputs) test "$#" -eq 2 ;;
  --run) test "$#" -eq 3; case "$3" in /tmp/*) ;; *) exit 2 ;; esac ;;
  --verify) test "$#" -eq 4 ;;
  *) exit 2 ;;
esac
build_sha=$1
native_sha=$2
verify_inputs
case "$mode" in
  --check-inputs) set -- --check-inputs ;;
  --run) set -- --run --out "$3" ;;
  --verify) set -- --readback "$3" --evidence-sha "$4" ;;
esac
set +e
"$py" -E -B "$lane/native-tools/replay-vertical-choice-menu.py" "$@" \
  --source "$source" --source-manifest "$source/../source.sha256" --source-sha "$source_sha" \
  --binary-manifest "$build/native/SHA256SUMS" --binary-sha "$native_sha"
status=$?
set -e
verify_inputs
exit "$status"
