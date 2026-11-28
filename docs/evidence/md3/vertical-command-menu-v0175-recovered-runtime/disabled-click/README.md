# Disabled row pointer regression

The new regression replays pointer down and up at the center of a disabled
row. The Standard panel has separated groups and a transparent base. The
unmodified product source dismissed the open menu. The fix leaves the
menu and focus in place and keeps the row inactive. The same test checks
that a click outside and a click in the gap still dismiss the menu.

Run from the source snapshot with Git tree
`1a8f21971d18d2782882fc8e8e57424bdb9cf301`. Set `proof` to this directory.
Use the [build setup](../reproduce/README.md) with a new Cargo target.

```sh
git apply --check --whitespace=error-all "$proof/disabled-menu-regression.patch"
git apply --whitespace=error-all "$proof/disabled-menu-regression.patch"
cap cargo test --locked -p surface-material --test vertical_command_menu \
    disabled_click_stays_in_a_separated_menu_and_outside_click_dismisses
```

The expected result above is one assertion failure on the open state.
Then apply the fix and run the surrounding tests.

```sh
git apply --check --whitespace=error-all "$proof/disabled-menu-fix.patch"
git apply --whitespace=error-all "$proof/disabled-menu-fix.patch"
cap cargo fmt --all --check
cap cargo test --locked -p surface-material --test vertical_command_menu --test command_menu
cap cargo test --locked -p surface-app
```

Formatting passed. All eight menu tests and 372 Runtime tests passed. The
fixed source has its own two-file manifest. It still needs a new headless
browser and native run before it can replace the baseline evidence.
