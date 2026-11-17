#!/usr/bin/env python3
"""Click the on-screen text found by OCR: click_text.py <phrase> [nth]. Prints what it clicked."""
import importlib.util, pathlib, subprocess, sys, collections
HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("drive", HERE.parent / "ticket02" / "drive.py")
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
phrase = sys.argv[1].lower(); nth = int(sys.argv[2]) if len(sys.argv) > 2 else 0
shot = HERE / "ocr.png"
subprocess.run(["import", "-display", ":99", "-window", "root", str(shot)], check=True)
tsv = subprocess.run(["tesseract", str(shot), "stdout", "tsv"], capture_output=True, text=True).stdout
lines = collections.defaultdict(list)
for row in tsv.splitlines()[1:]:
    p = row.split("\t")
    if len(p) >= 12 and p[0] == "5" and p[11].strip():
        lines[tuple(p[1:5])].append((p[11], int(p[6]), int(p[7]), int(p[8]), int(p[9])))
hits = []
for words in lines.values():
    text = " ".join(w[0] for w in words).lower()
    i = text.find(phrase)
    if i >= 0:
        # word span covering the phrase
        pos, first, last = 0, None, None
        for k, w in enumerate(words):
            if first is None and pos + len(w[0]) > i: first = k
            pos += len(w[0]) + 1
            if first is not None and pos - 1 >= i + len(phrase): last = k; break
        last = last if last is not None else len(words) - 1
        a, b = words[first], words[last]
        hits.append(((a[1] + b[1] + b[3]) // 2, a[2] + a[4] // 2))
if len(hits) <= nth: sys.exit(f"phrase {phrase!r} not found ({len(hits)} hits)")
x, y = hits[nth]
drive.display(); drive.click(x, y); print(f"clicked {phrase!r} at {(x, y)}")
