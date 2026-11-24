#!/bin/sh
set -eu
headless=0
for arg in "$@"; do
    case "$arg" in
        --headless=new) headless=$((headless + 1)) ;;
        --headless|--headless=*) exit 64 ;;
    esac
done
test "$headless" -eq 1
printf '%s\n' 'Chrome confirmed --headless=new' >&2
exec "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" "$@"
