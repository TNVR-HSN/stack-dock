"""Create an editable test fixture using the app's real backup format."""
import argparse
import base64
import json
from pathlib import Path

ap = argparse.ArgumentParser()
ap.add_argument("out", type=Path)
app = ap.parse_args()
root = Path(__file__).resolve().parents[1]
layout = json.loads((root / "assets" / "starter.json").read_text(encoding="utf-8"))
first, second, third = layout["layout"]["folders"]
icon = base64.b64encode((root / "assets" / "folder-creative.png").read_bytes()).decode()
first["slots"] = [
    {"id": "qa-generated-file", "type": "item", "kind": "local", "localType": "file",
     "r": 0, "c": 1, "name": "Layout notes", "target": r"C:\Windows\win.ini", "icon": "file"},
    {"id": "qa-generated-subfolder", "type": "folder", "r": 1, "c": 3, "name": "References",
     "icon": "folder-web", "rows": 2, "cols": 3, "slots": [{"id": "qa-nested-item", "type": "item",
     "kind": "url", "r": 0, "c": 1, "name": "Project source", "icon": "web",
     "target": "https://github.com/TNVR-HSN/stack-dock"}], "style": dict(second["style"])},
    {"id": "qa-generated-custom", "type": "item", "kind": "url", "r": 2, "c": 0,
     "name": "StackDock releases", "target": "https://github.com/TNVR-HSN/stack-dock/releases",
     "iconMode": "custom", "image": f"data:image/png;base64,{icon}"},
]
first["entryMotion"] = "fly-in"
layout["layout"]["dockShortcuts"] = [
    {"id": "qa-dock-shortcut", "type": "item", "kind": "url", "name": "Source code", "icon": "web",
     "target": "https://github.com/TNVR-HSN/stack-dock"},
]
layout["layout"]["dockOrder"] = [first["id"], second["id"], "qa-dock-shortcut", third["id"]]
app.out.write_text(json.dumps(layout, indent=2) + "\n", encoding="utf-8")
