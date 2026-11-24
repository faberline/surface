"""Check the sealed native Divider replay, all PNG pixels, and owned process exit."""

from hashlib import sha256
import importlib.util
import json
from pathlib import Path
import subprocess
import sys


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def load_audit(batch):
    scripts = batch/"native-scripts"
    sys.path.insert(0, str(scripts))
    spec = importlib.util.spec_from_file_location("divider_audit", scripts/"replay-divider.py")
    audit = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(audit)
    return audit


def main():
    batch, run = (Path(argument).resolve() for argument in sys.argv[1:3])
    audit = load_audit(batch)
    source = batch/"source.sha256"
    binaries = batch/"pins/native-binaries.sha256"
    assert len(source.read_text().splitlines()) == 583
    assert 'version = "0.1.45"' in (batch/"source/Cargo.toml").read_text()
    assert (batch/"build-source-before.log").read_bytes()==(batch/"build-source-after.log").read_bytes()
    for name, count in (("source",583),("pins",2)):
        before = (run/f"{name}-before.log").read_bytes()
        after = (run/f"{name}-after.log").read_bytes()
        assert before == after and len(before.splitlines()) == count
        assert all(line.endswith(b": OK") for line in before.splitlines())
    assert (run/"repeat.log").read_bytes() == b""

    pins = {}
    owned_paths = []
    for item in binaries.read_text().splitlines():
        checksum, name = item.split()
        path = batch/"pins"/name
        key = {"surface-material-gallery":"gallery","surface-devtools":"cli"}[name]
        assert digest(path) == checksum
        pins[key] = checksum
        owned_paths.extend([str(path), str(path).removeprefix("/private")])
    assert set(pins) == {"gallery", "cli"}
    assert not list(run.glob("surface-loading-session-*"))
    programs = subprocess.check_output(["/bin/ps","-axww","-o","pid=,comm="], text=True)
    owned = [int(pid) for pid, exe in (line.split(maxsplit=1) for line in programs.splitlines())
             if exe in owned_paths]
    assert not owned, owned

    first, second = run/"replay-1", run/"replay-2"
    report_file = "native-divider-report.json"
    raw = (first/report_file).read_bytes()
    assert raw == (second/report_file).read_bytes()
    report = json.loads(raw)
    assert report["schema"] == "surface.test.material-divider-native.v2"
    assert report["transport"] == ["Page.screenshot","CLI","MCP"]
    assert pins == report["binaries"]
    assert [profile["viewport"] for profile in report["profiles"]] == [[960,800],[320,800]]
    scans = {}
    samples = []
    for profile in report["profiles"]:
        viewport = tuple(profile["viewport"])
        samples += profile["visible_windows"]
        assert profile["tab_path"] == ["divider-demo-"+name for name in
                                       ("back","theme","rtl","large-text","weight","back")]
        assert profile["restored_focus"]=="divider-demo-show"
        assert [weight["thickness_dp"] for weight in profile["weights"]] == [1,8]
        for mode, weight in zip(("regular","heavy"), profile["weights"]):
            assert weight["weight"]==mode and weight["selected"] is (mode=="heavy")
            assert weight["focused"]=="divider-demo-weight"
            shot=weight["capture"]
            name=shot["name"]
            assert name==f"{profile['profile']}-{mode}.png"
            path=first/name
            assert path.read_bytes()==(second/name).read_bytes()
            assert shot["sha256"]==digest(path)
            assert (shot["width"],shot["height"],shot["scale"])==(*viewport,1)
            png=audit.read_png(path.read_bytes())
            assert png[:2]==viewport
            thickness=weight["thickness_dp"]
            for suffix, actual in weight["ink"].items():
                checked=audit.stripe(actual["bounds"],thickness,suffix,
                                    bytes(actual["rgba"]),bytes(actual["background"]),png)
                assert checked==actual, (name,suffix)
            scans[name]={"sha256":shot["sha256"],"dimensions":list(viewport),
                         "full_depth_solid_pixels":sum(part["solid_pixels"] for part in weight["ink"].values()),
                         "clean_surround_pixels":sum(part["adjacent_pixels"] for part in weight["ink"].values()),
                         "thickness_px":thickness}
    assert len(scans)==4 and samples==[0]*8
    for scan in (first,second):
        assert set(scans)=={path.name for path in scan.glob("*.png")}
    result={"version":"0.1.45","headless":True,"replays":2,"pngs_per_replay":4,
            "source_manifest_sha256":digest(source),"binary_manifest_sha256":digest(binaries),
            "binaries":pins,"transports":report["transport"],"identical_replay_bytes":True,
            "window_samples_per_run":len(samples),"max_visible_windows":0,"remaining_owned_processes":owned,
            "pngs":scans}
    (run/"verified-native.json").write_text(json.dumps(result,indent=2)+"\n")
    print("Verified 2 × 4 native PNGs. Every measured stripe has its full depth and clean surrounding pixels.")


if __name__=="__main__":
    main()
