"""Check that the exact native replay left no owned processes or session files."""

import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
pins = str(root / "pins") + "/"
rows = subprocess.check_output(["/bin/ps", "-axww", "-o", "pid=,comm="], text=True)
owned = [int(pid) for pid, exe in (line.split(maxsplit=1) for line in rows.splitlines())
         if exe.startswith(pins) or exe.startswith(pins.removeprefix("/private"))]
assert not owned, owned
assert not list(root.glob("surface-loading-session-*"))
print(json.dumps({"remaining_owned_processes": owned, "remaining_owned_sessions": []}, indent=2))
