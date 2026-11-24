"""Recheck earlier component and IME artifacts from the exact Switch stock run."""

from hashlib import file_digest
import json
from pathlib import Path
import sys

from verify_choice_report import validate as choice
from verify_contrast_report import validate as contrast
from verify_dialog_report import validate as dialog
from verify_divider_report import validate as divider
from verify_ime_cancel_report import validate as ime_cancel
from verify_loop_report import validate as loop
from verify_menu_report import validate as menu
from verify_modal_report import validate as modal, verify_compact_pngs
from verify_navigation_bar import validate_navigation
from verify_numeric_report import validate as numeric
from verify_pair_report import validate as pair
from verify_popup_report import validate as popup
from verify_search_report import validate as search


def read(path):
    return json.loads(path.read_text())


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


run = Path(sys.argv[1]).resolve()
artifacts = run / "artifacts"
stock = read(run / "verified-switch-stock.json")
assert stock["cases"]["material"]["steps"] == 276 and stock["guard_and_window_checks"] == "passed"
browser, adapter = stock["browser"], stock["adapter"]
log_path = run / "runner.log"
log = log_path.read_text()
scroll = {
    "scroll-modal-fields": modal,
    "scroll-popup-fields": popup,
    "scroll-fields": numeric,
    "scroll-choice-fields": choice,
    "scroll-loop-fields": loop,
    "scroll-pair-fields": pair,
}
for name, verify in scroll.items():
    verify(read(artifacts / f"browser-material-{name}.json"))
text = read(artifacts / "browser-ime-line-endings.json")
assert text["authored"] == "ab\r\n你🦀\rZ"
assert text["browser_value"] == "ab\n你🦀\nZ"
assert [(s["key"], s["browser"], s["runtime"]) for s in text["steps"]] == [
    ("ArrowLeft", [1, 1], [1, 1]), ("Shift+ArrowLeft", [2, 3], [4, 2]), ("ArrowRight", [6, 6], [11, 11]),
]
assert text["steps"][1]["direction"] == "backward"
nav = validate_navigation(read(artifacts / "browser-material-flexible-navigation-bar.json"), artifacts)
switch = read(artifacts / "browser-material-switch.json")
assert switch["browser"] == browser and switch["adapter"] == adapter and switch["errors"] == []
label_focus = switch["compact"]["label_focus"]
assert label_focus == {"runtime": "switch-demo-both-icons", "browser": "surface"}
menu_output = menu(artifacts, platform="macos", scope="stock", log=log_path,
                   expected_metadata={"browser": browser, "adapter": adapter},
                   window_receipt=run / "headless-window-observation.json")
records = {
    "contrast": contrast(artifacts, browser, adapter),
    "search": search(artifacts, browser, adapter),
    "dialog": dialog(artifacts, browser, adapter, log=log),
    "divider": divider(artifacts, browser, adapter, log=log),
    "menu": menu_output,
    "navigation": nav,
}
assert [len(records[key]["pngs"]) for key in ("contrast", "search", "dialog", "menu", "navigation")] == [
    28, 46, 41, 3, 4,
]
assert len(records["divider"]["profiles"]) == 4
result = {"recorded_stock_sha256": sha(run / "verified-switch-stock.json"),
          "total_checks": stock["total_checks"], "material_steps": 276,
          "browser": browser, "adapter": adapter, "checked_scroll_reports": list(scroll),
          "compact_scroll_pngs": verify_compact_pngs(artifacts), "ime_line_endings": text,
          "ime_cancel": ime_cancel(artifacts), "switch_label_focus": label_focus, "components": records}
print(json.dumps(result, indent=2, ensure_ascii=False))
