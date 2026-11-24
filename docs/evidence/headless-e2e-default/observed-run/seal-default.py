"""Seal the completed default E2E proof without launching any process."""

import argparse
import importlib.util
import json
from pathlib import Path
from shutil import copyfile


ROOT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("observe_default", ROOT / "observe-default.py")
observer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(observer)


def checked_run(run):
    invocation = observer.read_json(run / "invocation.json")
    assert invocation["default_suite_arguments"] == invocation["injected_chrome_flags"] == []
    argv = invocation["argv"]
    assert argv[1::2] == ["--chrome", "--web-root", "--bridge-web", "--jet-bridge", "--devtools-bin",
                           "--native-bin", "--compact-bin", "--artifacts"]
    assert len(argv) == 17
    observed = observer.read_json(run / "headless-window-observation.json")
    measured = observer.verify_run(run, observed)
    assert observer.read_json(run / "verified-default.json") == json.loads(json.dumps(measured))
    events = [observer.read_json(path) for path in (run / "guard-ledger").glob("*.json")]
    assert sorted(events, key=lambda event: event["pid"]) == sorted(observed["guard_events"], key=lambda event: event["pid"])
    return measured


def package(run, out):
    measured = checked_run(run)
    assert out.is_relative_to(Path("/tmp").resolve())
    out.mkdir(mode=0o700)
    for name in ("observe-default.py", "seal-default.py", "verify_menu_report.py",
                 "verify_divider_report.py", "verify_ime_cancel_report.py", "README.md"):
        copyfile(ROOT / name, out / name)
    for name in ("runner.log", "invocation.json", "headless-window-observation.json", "verified-default.json"):
        copyfile(run / name, out / name)
    for dirname in ("guard-ledger", "artifacts"):
        (out / dirname).mkdir()
    for source in (run / "guard-ledger").glob("*.json"):
        copyfile(source, out / "guard-ledger" / source.name)
    reports = ("browser-smoke.json", "browser-ime.json", "browser-material-side-sheet.json",
               "browser-material-command-menu.json", "browser-material-divider.json",
               "browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json",
               "bridge-web-result.json", "devtools-headless-result.json")
    images = (*measured["headless_transport_pngs"],
              *(f"material-command-menu-{name}.png" for name in measured["menu"]["pngs"]),
              *(item["file"] for item in measured["divider"]["profiles"].values()))
    for name in (*reports, *images):
        copyfile(run / "artifacts" / name, out / "artifacts" / name)
    manifest = out / "SHA256SUMS"
    files = sorted(path for path in out.rglob("*") if path.is_file())
    manifest.write_text("".join(f"{observer.sha(path)}  {path.relative_to(out)}\n" for path in files))
    recheck(out)
    return {"path": str(out), "files": len(files), "manifest_sha256": observer.sha(manifest),
            "checks": measured["total_checks"], "pngs": len(images)}


def recheck(out):
    items = observer.checksums(out / "SHA256SUMS")
    assert set(items) == {str(path.relative_to(out)) for path in out.rglob("*")
                          if path.is_file() and path != out / "SHA256SUMS"}
    observer.verify_entries(out, items)
    return checked_run(out)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    parser.add_argument("--out", type=Path, help="copy the proof to a new private /tmp directory")
    args = parser.parse_args()
    if not __debug__:
        parser.error("run without -O")
    answer = package(args.run.resolve(), args.out.resolve()) if args.out else recheck(args.run.resolve())
    print(json.dumps(answer, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
