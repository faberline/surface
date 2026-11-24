"""Run the sealed stock suite and sample the owned native and Chrome windows."""

import ctypes
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent
CHROME = "/Applications/Google Chrome.app/Contents/"
CHROME_EXECUTABLE = CHROME + "MacOS/Google Chrome "
GALLERY = "surface-material-gallery"


class Windows:
    def __init__(self):
        cg = ctypes.CDLL("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics")
        cf = ctypes.CDLL("/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation")
        pointer = ctypes.c_void_p
        cg.CGWindowListCopyWindowInfo.argtypes = [ctypes.c_uint32, ctypes.c_uint32]
        cg.CGWindowListCopyWindowInfo.restype = pointer
        cf.CFArrayGetCount.argtypes = [pointer]
        cf.CFArrayGetCount.restype = ctypes.c_long
        cf.CFArrayGetValueAtIndex.argtypes = [pointer, ctypes.c_long]
        cf.CFArrayGetValueAtIndex.restype = pointer
        cf.CFDictionaryGetValue.argtypes = [pointer, pointer]
        cf.CFDictionaryGetValue.restype = pointer
        cf.CFNumberGetValue.argtypes = [pointer, ctypes.c_long, pointer]
        cf.CFNumberGetValue.restype = ctypes.c_bool
        cf.CFRelease.argtypes = [pointer]
        self.cg = cg
        self.cf = cf
        self.owner_key = pointer.in_dll(cg, "kCGWindowOwnerPID")

    def onscreen_owners(self):
        window_list = self.cg.CGWindowListCopyWindowInfo(1, 0)
        assert window_list, "CoreGraphics could not enumerate visible windows"
        try:
            owners = set()
            for index in range(self.cf.CFArrayGetCount(window_list)):
                item = self.cf.CFArrayGetValueAtIndex(window_list, index)
                owner = self.cf.CFDictionaryGetValue(item, self.owner_key)
                pid = ctypes.c_int32()
                if owner and self.cf.CFNumberGetValue(owner, 3, ctypes.byref(pid)):
                    owners.add(pid.value)
            return owners
        finally:
            self.cf.CFRelease(window_list)


def processes():
    info = subprocess.check_output(["/bin/ps", "-axww", "-o", "pid=,ppid=,command="], text=True)
    for row in info.splitlines():
        try:
            pid, ppid, command = row.strip().split(maxsplit=2)
        except ValueError:
            continue
        yield int(pid), int(ppid), command


def children_of(parent):
    children = {}
    for pid, ppid, command in processes():
        children.setdefault(ppid, []).append((pid, command))
    queue = list(children.get(parent, ()))
    for item in queue:
        yield item
        queue.extend(children.get(item[0], ()))


def observe(stage, run_name):
    stock_run = stage / run_name
    assert not stock_run.exists(), stock_run
    windows = Windows()
    counters = {"chrome": {"samples": 0, "pids": []}, "native_gallery": {"samples": 0, "pids": []}}
    visible = []
    owning_script = subprocess.Popen(
        ["/bin/bash", ROOT / "stock-run.sh", stage, run_name], start_new_session=True,
    )
    try:
        while owning_script.poll() is None:
            candidates = {}
            for pid, command in children_of(owning_script.pid):
                if command.startswith(CHROME):
                    candidates[pid] = "chrome"
                    if command.startswith(CHROME_EXECUTABLE):
                        assert "--headless=new" in command, "Owned Chrome lacks the headless flag"
                elif command.split(maxsplit=1)[0].endswith("/" + GALLERY):
                    candidates[pid] = "native_gallery"
                    assert " --headless" in command, "Owned Gallery lacks the headless flag"
            if candidates:
                owners = windows.onscreen_owners()
                for pid, label in candidates.items():
                    series = counters[label]
                    series["samples"] += 1
                    if pid not in series["pids"]:
                        series["pids"].append(pid)
                    if pid in owners:
                        visible.append({"pid": pid, "kind": label})
                if visible:
                    os.killpg(owning_script.pid, signal.SIGTERM)
                    break
            time.sleep(.05)
    finally:
        if owning_script.poll() is None:
            os.killpg(owning_script.pid, signal.SIGTERM)
        return_code = owning_script.wait(timeout=15)
        remaining = [pid for pid, _, command in processes()
                     if (pid in counters["chrome"]["pids"] and command.startswith(CHROME))
                     or (pid in counters["native_gallery"]["pids"] and "/" + GALLERY in command)]
        if stock_run.is_dir():
            (stock_run / "headless-window-observation.json").write_text(json.dumps({
                "method": "CoreGraphics on-screen window owner PIDs matched against the owned process tree",
                "sampling_interval_seconds": 0.05,
                "observations": counters,
                "visible_owned_windows": visible,
                "remaining_observed_processes": remaining,
                "stock_exit": return_code,
            }, indent=2) + "\n")
    if return_code:
        raise SystemExit(return_code)
    assert not visible
    assert not remaining, remaining
    assert all(series["samples"] > 0 for series in counters.values()), counters
    print(f"Owned headless windows stayed hidden: Chrome {counters['chrome']['samples']} samples and native Gallery {counters['native_gallery']['samples']} samples.")


if __name__ == "__main__":
    observe(Path(sys.argv[1]).resolve(strict=True), sys.argv[2] if len(sys.argv) > 2 else "run")
