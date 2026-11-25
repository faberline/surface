# Surface v0.1.69 R2 scoped replay

The R2 runner uses the 688-path root source seal
47642376bb5f97f07a542ba6c1fb159e1bdeda1d904cfb4c3dc39096d2c75354.
Its only difference from the R1 source is the awaited focused
paint barrier in the M350 test. The private scoped registry still
selects all 29 original steps M325–M353. The focused source
manifest is 783c7463a93b34a33db63e709c52dec7817d44af6f17579cb4e40fcc7a249c30.

The Gallery still uses the four measured R1 originals pinned by
700ba34b6e25aaf7406f0ecbdbae056274a6fba9725e7a3e120a6e099e81cd33.
The source and Gallery receipts bind their actual build origins and
prove the only difference is the earlier test file.

The failed original R1 is intact under
/tmp/surface-md3-field-placeholder-focused-v0169.612HMDhp/development-r1-authorized.
Its relative evidence paths are recorded in r1-failure.sha256.
It reached M350 and captured the idle color with a focused outline.
The R2 test retains the exact color and position assertions.

## Controller handoff

Wait until the authorized native run is terminal and the controller
authorizes the next real browser run. Recheck and use one fresh
guarded output directory. Preserve original failures untouched.

~~~sh
lane=/tmp/surface-md3-field-placeholder-focused-v0169-r2.8AArzw7l
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
"$py" -E -B "$lane/run-focused.py" --verify-inputs-only
cap "$py" -E -B "$lane/run-focused.py" --run --out "$lane/development-r2"
"$py" -E -B "$lane/verify-placeholder.py" --run "$lane/development-r2" \
  > "$lane/development-r2-verified.json"
~~~

The guard needs permission to read the process list and window
owners. Use the same approved headless observer environment as
the authorized R1. This is one scoped Material suite. It checks
the prior 34 label frames and 58 new paired placeholder frames.
