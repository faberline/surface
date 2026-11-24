"""Run the targeted Chrome suite and watch its own processes for windows."""

import json
import os
from pathlib import Path
import runpy
import signal
import subprocess
import time

root = Path(__file__).resolve().parent
observer = runpy.run_path(str(root / "window-observer.py"))
window_list = observer["Windows"]()
existing = set(root.glob("run.*"))
owned = subprocess.Popen(["/bin/bash", root / "run-focused.sh"], start_new_session=True)
chrome_prefix = observer["CHROME"]
chrome_executable = observer["CHROME_EXECUTABLE"]
all_chrome = set()
visible = set()
samples = 0
try:
    while owned.poll() is None:
        candidates = set()
        for pid, command in observer["children_of"](owned.pid):
            if command.startswith(chrome_prefix):
                candidates.add(pid)
                if command.startswith(chrome_executable):
                    assert "--headless=new" in command, "Owned Chrome lacks the headless flag"
        if candidates:
            all_chrome.update(candidates)
            samples += len(candidates)
            visible.update(candidates & window_list.onscreen_owners())
        if visible:
            os.killpg(owned.pid, signal.SIGTERM)
            break
        time.sleep(.05)
finally:
    if owned.poll() is None:
        os.killpg(owned.pid, signal.SIGTERM)
    runner_exit = owned.wait(timeout=15)
    remaining = sorted(pid for pid, _, command in observer["processes"]()
                       if pid in all_chrome and command.startswith(chrome_prefix))
    [result] = set(root.glob("run.*")) - existing
    observation = {"samples": samples, "visible_windows": sorted(visible),
                   "remaining_chrome": remaining, "exit": runner_exit}
    (result / "headless-observation.json").write_text(json.dumps(observation, indent=2) + "\n")
assert samples > 0 and not visible and not remaining, observation
print(f"Observed {samples} Chrome process samples with zero onscreen windows.")
raise SystemExit(runner_exit)
