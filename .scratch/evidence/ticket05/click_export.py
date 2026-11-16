#!/usr/bin/env python3
"""Click the form's 'Ekspor PDF saat ini...' button by locating it with OCR.

The form scrolls itself as fields take focus, so a fixed y drifts. The Render
button sits immediately to its left; OCR its row and click past its right edge.
"""
import importlib.util
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("drive", HERE.parent / "ticket02" / "drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
drive.SHOTS = HERE
drive.DATA = pathlib.Path("/home/airzy/.hermes/cache/scratch/cvgen-ticket05")

shot = HERE / "export-click.png"
subprocess.run(["import", "-display", ":99", "-window", "root", str(shot)], check=True)
tsv = subprocess.run(["tesseract", str(shot), "stdout", "tsv"], capture_output=True, text=True).stdout

rows = []
for line in tsv.splitlines()[1:]:
    parts = line.split("\t")
    if len(parts) < 12 or parts[0] != "5":
        continue
    text = parts[11].replace(" ", "").lower()
    if "render" in text:
        x, y, w, h = int(parts[6]), int(parts[7]), int(parts[8]), int(parts[9])
        if x < 200 and h >= 8:
            rows.append((x, y, w, h))

if not rows:
    sys.exit("export button row not found; is the form scrolled to the actions?")

x, y, w, h = max(rows, key=lambda row: row[0])  # the Render button, left of Export
target = (x + w + 62, y + h // 2)
drive.display()
drive.click(*target)
print(f"clicked export at {target}")
